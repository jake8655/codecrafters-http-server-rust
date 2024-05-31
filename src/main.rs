use core::str::Lines;
use std::io::Read;
use std::io::{BufRead, BufReader};
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
                handle_connection(stream)?;
            }
            Err(e) => {
                println!("error: {}", e);
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
    fn from(lines: &mut Lines) -> Result<Self> {
        let mut headers = Vec::new();

        for line in lines {
            if line.is_empty() {
                break;
            }

            let Some(parts) = line.split_once(':') else {
                return Err(anyhow::anyhow!("invalid header line: {}", line))?;
            };
            headers.push((parts.0.to_string(), parts.1.to_string()));
        }

        Ok(Self(headers))
    }
}

#[derive(Debug)]
struct Body(Vec<String>);

impl Body {
    fn from(lines: &mut Lines) -> Result<Self> {
        let body = lines.map(|line| line.to_string()).collect();

        Ok(Self(body))
    }
}

fn handle_connection(mut stream: TcpStream) -> Result<()> {
    let mut request_string = String::new();
    stream.read_to_string(&mut request_string)?;
    let mut lines = request_string.lines();

    let first_line = lines.next().unwrap();
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

    dbg!(&response);

    stream.write_all(response.as_bytes())?;

    Ok(())
}
