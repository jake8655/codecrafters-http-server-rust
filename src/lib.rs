use anyhow::Result;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::str::FromStr;
use std::{
    io::{BufRead, BufReader},
    net::TcpStream,
    sync::Arc,
};

mod structs;
use structs::*;

pub mod config;
use config::Config;

pub async fn handle_connection(mut stream: TcpStream, config: Arc<Config>) -> Result<()> {
    let mut reader = BufReader::new(&mut stream);

    let mut first_line = String::new();
    reader.read_line(&mut first_line)?;

    let parts = first_line.split_whitespace().collect::<Vec<&str>>();
    let method = Method::from_str(parts[0])?;
    let path = parts[1].to_string();
    let version = parts[2].to_string();

    let mut header_lines = Vec::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        if line == "\r\n" {
            break;
        }
        header_lines.push(line.trim_end().to_string());
    }
    let headers = Headers::from(header_lines)?;

    let mut body_lines = Vec::new();
    if let Some(length) = headers.get_content_length() {
        if length != 0 {
            let mut lines = vec![0; length];
            reader.read_exact(&mut lines)?;
            body_lines = String::from_utf8_lossy(&lines[..])
                .lines()
                .map(|line| line.to_string())
                .collect();
        }
    }

    let body = Body::from_lines(body_lines);

    let request = Request::new(method, path, version, headers, body);

    let mut response = Response::new();

    match request.path.as_str() {
        "/" => {
            response.set_status(Status::Ok);
        }
        x if x.starts_with("/echo/") => {
            let text = request.path.split_at(6).1;
            response.set_plain_text_body(Body::from_str(text));

            response.apply_compression(&request);
        }
        "/user-agent" => {
            let default_user_agent = String::from("None");
            let user_agent = request
                .headers
                .get_user_agent()
                .unwrap_or(&default_user_agent);

            response.set_plain_text_body(Body::from_str(user_agent));
        }
        x if x.starts_with("/files/") => {
            handle_files(&request, &mut response, &config);
        }
        _ => {
            response.set_status(Status::NotFound);
        }
    }

    stream.write_all(&response.to_bytes())?;

    Ok(())
}

fn handle_files(request: &Request, response: &mut Response, config: &Config) {
    let path = config.directory.join(request.path.split_at(7).1);

    match request.method {
        Method::Get => {
            let readable = fs::read_to_string(path);

            match readable {
                Ok(contents) => {
                    response.set_file_body(Body::from_str(&contents));
                }
                Err(e) => {
                    eprintln!("error opening file: {}", e);
                    response.set_status(Status::NotFound);
                }
            }
        }

        Method::Post => {
            let contents = request.body.to_string();
            let result = fs::write(path, contents);

            match result {
                Ok(_) => {
                    response.set_status(Status::Created);
                }
                Err(e) => {
                    eprintln!("error writing file: {}", e);
                    response.set_status(Status::InternalServerError);
                }
            }
        }
    }
}
