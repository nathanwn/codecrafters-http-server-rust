use std::io;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;

fn handle_connection(tcp_stream: &mut TcpStream) -> io::Result<()> {
    tcp_stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes())?;
    tcp_stream.flush()?;
    Ok(())
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    for stream in listener.incoming() {
        match stream {
            Ok(mut tcp_stream) => {
                println!("accepted new connection");
                handle_connection(&mut tcp_stream).expect("Responded successfully");
                println!("done");
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
