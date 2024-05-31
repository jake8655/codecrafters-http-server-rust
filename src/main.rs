use std::io::BufRead;
use std::io::BufReader;
use std::net::TcpStream;
use std::{io::Write, net::TcpListener};

use anyhow::Result;

fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:4221")?;
    println!("Server started on: 127.0.0.1:4221");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                handle_connection(stream).unwrap_or_else(|e| {
                    eprintln!("error: {}", e);
                });
            }
            Err(e) => {
                eprintln!("error: {}", e);
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
struct Request {
    method: String,
    path: String,
    version: String,
    headers: Headers,
    body: Body,
}

impl Request {
    fn new(method: String, path: String, version: String, headers: Headers, body: Body) -> Self {
        Self {
            method,
            path,
            version,
            headers,
            body,
        }
    }
}

#[derive(Debug)]
struct Headers(Vec<(String, String)>);

impl Headers {
    fn from(lines: &mut [String]) -> Result<Self> {
        let headers = lines
            .iter()
            .take_while(|line| !line.is_empty())
            .filter_map(|line| {
                let Some(parts) = line.split_once(": ") else {
                    eprintln!("invalid header line: {}", line);
                    return None;
                };
                Some((parts.0.to_string(), parts.1.to_string()))
            })
            .collect::<Vec<_>>();

        Ok(Self(headers))
    }
}

#[derive(Debug)]
struct Body(Vec<String>);

impl Body {
    fn from(lines: &mut [String]) -> Result<Self> {
        let body = lines.iter().map(|line| line.to_string()).collect();

        Ok(Self(body))
    }
}

fn handle_connection(mut stream: TcpStream) -> Result<()> {
    let buf_reader = BufReader::new(&mut stream);
    let mut lines = buf_reader
        .lines()
        .map(|line| line.expect("error reading line"))
        .take_while(|line| !line.is_empty())
        .collect::<Vec<_>>();

    let first_line = lines.remove(0);
    let parts = first_line.split_whitespace().collect::<Vec<&str>>();
    let method = parts[0].to_string();
    let path = parts[1].to_string();
    let version = parts[2].to_string();

    let headers = Headers::from(&mut lines)?;
    let body = Body::from(&mut lines)?;

    let request = Request::new(method, path, version, headers, body);

    let mut response = format!("{} ", request.version);

    match request.path.as_str() {
        "/" => {
            response.push_str("200 OK\r\n\r\n");
        }
        _ => {
            response.push_str("404 Not Found\r\n\r\n");
        }
    }

    stream.write_all(response.as_bytes())?;

    Ok(())
}
