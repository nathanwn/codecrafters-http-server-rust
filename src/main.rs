use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;

#[allow(dead_code)]
#[derive(Debug)]
struct Request {
    method: String,
    path: String,
}

impl Request {
    fn new(method: String, path: String) -> Self {
        Request { method, path }
    }
}

fn parse_request(first_line: String) -> Request {
    let first_line_tokens: Vec<_> = first_line.split(" ").collect();
    return Request::new(
        String::from(first_line_tokens[0]),
        String::from(first_line_tokens[1]),
    );
}

fn respond(
    tcp_stream: &mut TcpStream,
    respond_code: usize,
    msg: &str,
    body: &str,
) -> io::Result<()> {
    let response = [
        format!("HTTP/1.1 {} {}", respond_code, msg),
        String::from("Content-Type: text/plain"),
        format!("Content-Length: {}", body.len()),
        String::new(),
        String::from(body),
    ]
    .join("\r\n");
    tcp_stream.write(response.as_bytes())?;
    tcp_stream.flush()?;
    Ok(())
}

fn handle_connection(tcp_stream: &mut TcpStream) -> io::Result<()> {
    let mut reader = BufReader::new(tcp_stream.try_clone()?);
    // let lines: Vec<String> = reader.lines().map(|line| line.unwrap()).collect();
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let request = parse_request(line);
    if let Some(payload) = request.path.strip_prefix("/echo/") {
        respond(tcp_stream, 200, "OK", payload)?
    } else if request.path == "/" {
        respond(tcp_stream, 200, "OK", "")?
    } else {
        respond(tcp_stream, 404, "Not Found", "")?
    }
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
