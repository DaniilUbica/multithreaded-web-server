use std::net::{TcpListener};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let pool = web_server::ThreadPool::new(5);

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        pool.execute(|| {
            web_server::handle_connection(stream);
        });
    }
}
