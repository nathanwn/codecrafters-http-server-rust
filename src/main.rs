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

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    directory: Option<String>,
}

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    user_agent: Option<String>,
    body: Option<String>,
}

#[derive(Debug)]
struct HttpResponse {
    respond_code: usize,
    msg: String,
    content_type: String,
    body: Option<String>,
}

impl HttpResponse {
    fn new_ok_response(respond_code: usize, content_type: &str, body: Option<String>) -> Self {
        HttpResponse {
            respond_code,
            msg: String::from("OK"),
            content_type: String::from(content_type),
            body,
        }
    }

    fn new_not_found_response() -> Self {
        HttpResponse {
            respond_code: 404,
            msg: String::from("Not Found"),
            content_type: String::from("text/plain"),
            body: None,
        }
    }

    fn to_string(self) -> String {
        let real_body: String = self.body.unwrap_or(String::new());
        [
            format!("HTTP/1.1 {} {}", self.respond_code, self.msg),
            format!("Content-Type: {}", self.content_type),
            format!("Content-Length: {}", real_body.len()),
            String::new(),
            real_body,
        ]
        .join("\r\n")
    }
}

trait HttpStream {
    fn read_request(&mut self) -> io::Result<HttpRequest>;
    fn write_response(&mut self, response: HttpResponse) -> io::Result<()>;
}

impl HttpStream for TcpStream {
    fn read_request(&mut self) -> io::Result<HttpRequest> {
        let mut reader = BufReader::new(self.try_clone()?);
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
        let first_line = header_lines[0].clone();
        let first_line_tokens: Vec<_> = first_line.split(" ").collect();
        let method = String::from(first_line_tokens[0]);
        let path = String::from(first_line_tokens[1]);

        let mut user_agent: Option<String> = None;
        let mut content_length: Option<usize> = None;

        for i in 1..header_lines.len() {
            let line = header_lines[i].clone();
            if let Some(val) = line.strip_prefix("User-Agent: ") {
                user_agent = Some(String::from(val));
            }
            if let Some(val) = line.strip_prefix("Content-Length: ") {
                content_length = Some(val.parse::<usize>().unwrap());
            }
        }

        let mut body: Option<String> = None;

        if let Some(body_len) = content_length {
            let mut body_buf: Vec<u8> = vec![0u8; body_len];
            reader.read_exact(&mut body_buf)?;
            body = Some(String::from_utf8(body_buf).unwrap());
        }

        Ok(HttpRequest {
            method,
            path,
            user_agent,
            body,
        })
    }

    fn write_response(&mut self, response: HttpResponse) -> io::Result<()> {
        self.write(response.to_string().as_bytes())?;
        self.flush()?;
        Ok(())
    }
}

fn process_request(request: HttpRequest, directory: Option<String>) -> HttpResponse {
    if let Some(dir) = directory {
        if request.method == "POST" && request.body.is_some() {
            let filename = request.path.strip_prefix("/files/").unwrap();
            let filepath = Path::new(dir.as_str()).join(filename);
            fs::write(filepath, request.body.unwrap()).expect("File should be written successfully.");
            HttpResponse::new_ok_response(201, "text/plain", None)
        } else if request.method == "GET" && request.path.starts_with("/files/") {
            let filename = request.path.strip_prefix("/files/").unwrap();
            let filepath = Path::new(dir.as_str()).join(filename);
            if filepath.exists() {
                let body = fs::read_to_string(filepath).ok();
                HttpResponse::new_ok_response(200, "application/octet-stream", body)
            } else {
                HttpResponse::new_not_found_response()
            }
        } else {
            HttpResponse::new_not_found_response()
        }
    } else if request.path.starts_with("/echo/") {
        let body = request.path.strip_prefix("/echo/").map(str::to_string);
        HttpResponse::new_ok_response(200, "text/plain", body)
    } else if request.path == "/user-agent" {
        HttpResponse::new_ok_response(200, "text/plain", request.user_agent)
    } else if request.path == "/" {
        HttpResponse::new_ok_response(200, "text/plain", None)
    } else {
        HttpResponse::new_not_found_response()
    }
}

fn handle_connection(tcp_stream: &mut TcpStream, directory: Option<String>) -> io::Result<()> {
    let request = tcp_stream.read_request().unwrap();
    let response = process_request(request, directory);
    tcp_stream.write_response(response)
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
