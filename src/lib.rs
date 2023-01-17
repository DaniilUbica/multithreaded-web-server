use std::fs;
use std::io::{prelude::*, BufReader};
use std::net::{TcpStream};
use std::thread;
use std::sync::{mpsc, Arc, Mutex};

type Job = Box<dyn FnOnce() + Send + 'static>;

struct Request {
    status_line: String,
    filename: String,
}
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Request {
    fn new(status_line: String, filename: String) -> Request {
        Request {
            status_line,
            filename, 
        }
    }
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender: Some(sender), }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        self.sender.as_ref().unwrap().send(job).unwrap();
    }
    
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv();

            match message {
                Ok(job) => {
                    println!("Worker {id} got a job; executing.");

                    job();
                }
                Err(_) => {
                    println!("Worker {id} disconnected; shutting down.");
                    break;
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}

pub fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);
    let request_line = buf_reader.lines().next().unwrap().unwrap();

    let get = "GET / HTTP/1.1";

    let mut request = Request::new(String::from(""), String::from(""));

    if request_line == get {
        request.status_line = "HTTP/1.1 200 OK".to_string();
        request.filename = "pages/hello.html".to_string();
    }    
    else {
        request.status_line = "HTTP/1.1 404 NOT FOUND".to_string();
        request.filename = "pages/404.html".to_string();
    }

    let contents = fs::read_to_string(&request.filename).unwrap();

    let response = format!("{}\r\n\r\n{contents}", &request.status_line);

    stream.write_all(response.as_bytes()).unwrap();
}
