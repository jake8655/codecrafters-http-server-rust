#![allow(dead_code)]

use anyhow::Result;
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs;
use std::io::BufRead;
use std::io::BufReader;
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::{io::Write, net::TcpListener};

struct Config {
    directory: Box<PathBuf>,
}

impl Config {
    fn new(mut args: impl Iterator<Item = String>) -> Self {
        let mut directory = PathBuf::from(format!(
            "{}/public",
            env::current_dir().unwrap().to_str().unwrap()
        ));

        while let Some(arg) = args.next() {
            if arg.as_str() == "--directory" {
                directory = PathBuf::from(args.next().expect("invalid args"));
            }
        }

        Self {
            directory: Box::new(directory),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:4221")?;
    println!("Server started on: 127.0.0.1:4221");

    let config = Arc::new(Config::new(env::args()));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");

                let config = Arc::clone(&config);
                tokio::spawn(async move {
                    handle_connection(stream, config).await.unwrap_or_else(|e| {
                        eprintln!("error: {}", e);
                    });
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
struct Headers(HashMap<String, String>);

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
            .collect::<HashMap<_, _>>();

        Ok(Self(headers))
    }
}

#[derive(Debug)]
struct Body(String);

impl Body {
    fn from(lines: &mut [String]) -> Result<Self> {
        let body = lines.iter().map(|line| line.to_string()).collect();

        Ok(Self(body))
    }
}

#[derive(Debug)]
struct Response {
    version: String,
    status: Status,
    headers: Headers,
    body: Body,
}

#[derive(Debug)]
enum Status {
    Ok,
    NotFound,
}

impl Response {
    fn new() -> Self {
        Self {
            version: "HTTP/1.1".to_string(),
            status: Status::Ok,
            headers: Headers(HashMap::new()),
            body: Body("".to_string()),
        }
    }

    fn set_status(&mut self, status: Status) {
        self.status = status;
    }

    fn set_header(&mut self, key: String, value: String) {
        self.headers.0.insert(key, value);
    }

    fn set_plain_text_body(&mut self, body: Body) {
        self.headers
            .0
            .insert("Content-Length".to_string(), body.0.len().to_string());
        self.headers
            .0
            .insert("Content-Type".to_string(), "text/plain".to_string());
        self.body = body;
    }

    fn set_file_body(&mut self, body: Body) {
        self.headers.0.insert(
            "Content-Type".to_string(),
            "application/octet-stream".to_string(),
        );
        self.headers
            .0
            .insert("Content-Length".to_string(), body.0.len().to_string());
        self.body = body;
    }
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut response = format!("{} ", self.version);

        match self.status {
            Status::Ok => {
                response.push_str("200 OK");
            }
            Status::NotFound => {
                response.push_str("404 Not Found");
            }
        }
        response.push_str("\r\n");

        for (key, value) in &self.headers.0 {
            response.push_str(&format!("{}: {}\r\n", key, value));
        }
        response.push_str("\r\n");

        self.body.0.lines().for_each(|line| {
            response.push_str(line);
            response.push_str("\r\n");
        });

        write!(f, "{}", response)
    }
}

async fn handle_connection(mut stream: TcpStream, config: Arc<Config>) -> Result<()> {
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

    let mut response = Response::new();

    match request.path.as_str() {
        "/" => {
            response.set_status(Status::Ok);
        }
        x if x.starts_with("/echo/") => {
            let text = request.path.split_at(6).1;
            response.set_plain_text_body(Body(text.to_string()));
        }
        "/user-agent" => {
            let default_user_agent = String::from("None");
            let user_agent = request
                .headers
                .0
                .get("User-Agent")
                .unwrap_or(&default_user_agent);
            response.set_plain_text_body(Body(user_agent.to_string()));
        }
        x if x.starts_with("/files/") => {
            let path = config.directory.join(request.path.split_at(7).1);
            let readable = fs::read_to_string(path);

            match readable {
                Ok(contents) => {
                    response.set_file_body(Body(contents));
                }
                Err(e) => {
                    eprintln!("error opening file: {}", e);
                    response.set_status(Status::NotFound);
                }
            }
        }
        _ => {
            response.set_status(Status::NotFound);
        }
    }

    stream.write_all(response.to_string().as_bytes())?;

    Ok(())
}
