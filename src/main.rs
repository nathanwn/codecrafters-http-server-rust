use clap::Parser;
use std::fs;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
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
struct RequestHeader {
    method: String,
    path: String,
    user_agent: Option<String>,
    content_length: Option<usize>,
}

fn parse_request_header(lines: &Vec<String>) -> RequestHeader {
    let first_line = lines[0].clone();
    let first_line_tokens: Vec<_> = first_line.split(" ").collect();
    let method = String::from(first_line_tokens[0]);
    let path = String::from(first_line_tokens[1]);

    let mut user_agent: Option<String> = None;
    let mut content_length: Option<usize> = None;

    for i in 1..lines.len() {
        let line = lines[i].clone();
        if let Some(val) = line.strip_prefix("User-Agent: ") {
            user_agent = Some(String::from(val));
        }
        if let Some(val) = line.strip_prefix("Content-Length: ") {
            content_length = Some(val.parse::<usize>().unwrap());
        }
    }

    return RequestHeader {
        method,
        path,
        user_agent,
        content_length,
    };
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
    let mut header_lines: Vec<String> = Vec::new();
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
            header_lines.push(line_text);
        }
    }
    let header = parse_request_header(&header_lines);
    if header.method == "POST" && header.content_length.is_some() && directory.is_some() {
        let filename = header.path.strip_prefix("/files/").unwrap();
        let filepath = Path::new(directory.unwrap().as_str()).join(filename);
        let mut buf: Vec<u8> = vec![0u8; header.content_length.unwrap()];
        reader.read_exact(&mut buf)?;
        let content = String::from_utf8(buf).unwrap();
        fs::write(filepath, content).expect("File should be written successfully.");
        respond(tcp_stream, 201, "OK", "text/plain", "")?
    } else if directory.is_some() && header.method == "GET" && header.path.starts_with("/files/") {
        let filename = header.path.strip_prefix("/files/").unwrap();
        let filepath = Path::new(directory.unwrap().as_str()).join(filename);
        if filepath.exists() {
            let content = fs::read_to_string(filepath).expect("File should be read successfully.");
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
    } else if header.path.starts_with("/echo/") {
        let payload = header.path.strip_prefix("/echo/").unwrap();
        respond(tcp_stream, 200, "OK", "text/plain", payload)?
    } else if header.path == "/" {
        respond(tcp_stream, 200, "OK", "text/plain", "")?
    } else if header.path == "/user-agent" {
        if let Some(user_agent) = header.user_agent {
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
