use clap::Parser;
use std::fs;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::Path;
use std::thread;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    directory: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug)]
struct Request {
    method: String,
    path: String,
    user_agent: Option<String>,
}

impl Request {
    fn new(method: String, path: String, user_agent: Option<String>) -> Self {
        Request {
            method,
            path,
            user_agent,
        }
    }
}

fn parse_request(lines: &Vec<String>) -> Request {
    let first_line = lines[0].clone();
    let first_line_tokens: Vec<_> = first_line.split(" ").collect();
    let mut user_agent: Option<String> = None;
    for i in 1..lines.len() {
        let line = lines[i].clone();
        if let Some(val) = line.strip_prefix("User-Agent: ") {
            user_agent = Some(String::from(val));
            break;
        }
    }
    println!("{:?}", lines);
    return Request::new(
        String::from(first_line_tokens[0]),
        String::from(first_line_tokens[1]),
        user_agent,
    );
}

fn respond(
    tcp_stream: &mut TcpStream,
    respond_code: usize,
    msg: &str,
    content_type: &str,
    body: &str,
) -> io::Result<()> {
    let response = [
        format!("HTTP/1.1 {} {}", respond_code, msg),
        format!("Content-Type: {}", content_type),
        format!("Content-Length: {}", body.len()),
        String::new(),
        String::from(body),
    ]
    .join("\r\n");
    tcp_stream.write(response.as_bytes())?;
    tcp_stream.flush()?;
    Ok(())
}

fn handle_connection(tcp_stream: &mut TcpStream, directory: Option<String>) -> io::Result<()> {
    let mut reader = BufReader::new(tcp_stream.try_clone()?);
    let mut lines: Vec<String> = Vec::new();
    loop {
        let mut chars: Vec<u8> = Vec::new();
        let mut line: Option<String> = None;
        match reader.read_until('\n' as u8, &mut chars) {
            Ok(_) => {
                (0..2).for_each(|_| {
                    chars.pop();
                });
                line = Some(String::from_utf8(chars).unwrap());
            }
            _ => {}
        }
        if let Some(line_text) = line {
            if line_text.len() == 0 {
                break;
            }
            lines.push(line_text);
        }
    }
    let request = parse_request(&lines);
    if directory.is_some() && request.method == "GET" && request.path.starts_with("/files/") {
        let filename = request.path.strip_prefix("/files/").unwrap();
        let filepath = Path::new(directory.unwrap().as_str()).join(filename);
        if filepath.exists() {
            let content = fs::read_to_string(filepath).expect("File should be read.");
            respond(
                tcp_stream,
                200,
                "OK",
                "application/octet-stream",
                content.as_str(),
            )?
        } else {
            respond(tcp_stream, 404, "Not Found", "text/plain", "")?
        }
    } else if request.path.starts_with("/echo/") {
        let payload = request.path.strip_prefix("/echo/").unwrap();
        respond(tcp_stream, 200, "OK", "text/plain", payload)?
    } else if request.path == "/" {
        respond(tcp_stream, 200, "OK", "text/plain", "")?
    } else if request.path == "/user-agent" {
        if let Some(user_agent) = request.user_agent {
            respond(tcp_stream, 200, "OK", "text/plain", user_agent.as_str())?
        } else {
            panic!()
        }
    } else {
        respond(tcp_stream, 404, "Not Found", "text/plain", "")?
    }
    Ok(())
}

fn main() {
    let args = Args::parse();

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    for stream in listener.incoming() {
        match stream {
            Ok(mut tcp_stream) => {
                println!("accepted new connection");
                let directory = args.directory.clone();
                thread::spawn(move || {
                    handle_connection(&mut tcp_stream, directory).expect("Responded successfully");
                });
                println!("done");
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
