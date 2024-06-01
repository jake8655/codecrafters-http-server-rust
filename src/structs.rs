#![allow(dead_code)]

use anyhow::Result;
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

#[derive(Debug)]
pub struct Request {
    pub method: Method,
    pub path: String,
    pub version: String,
    pub headers: Headers,
    pub body: Body,
}

#[derive(Debug)]
pub enum Method {
    Get,
    Post,
}

impl FromStr for Method {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GET" => Ok(Method::Get),
            "POST" => Ok(Method::Post),
            _ => Err(anyhow::anyhow!("invalid method: {}", s)),
        }
    }
}

impl Request {
    pub fn new(
        method: Method,
        path: String,
        version: String,
        headers: Headers,
        body: Body,
    ) -> Self {
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
pub struct Headers(pub HashMap<String, String>);

impl Headers {
    pub fn from(lines: Vec<String>) -> Result<Self> {
        let mut headers = HashMap::new();

        for line in lines {
            if line.is_empty() {
                break;
            }

            let Some(parts) = line.split_once(": ") else {
                eprintln!("invalid header line: {}", line);
                return Err(anyhow::anyhow!("invalid header line: {}", line));
            };

            headers.insert(parts.0.to_string(), parts.1.to_string());
        }

        Ok(Self(headers))
    }
}

#[derive(Debug)]
pub struct Body(pub String);

impl Body {
    pub fn from(lines: Vec<String>) -> Result<Self> {
        let body = lines.iter().map(|line| line.to_string()).collect();

        Ok(Self(body))
    }
}

#[derive(Debug)]
pub struct Response {
    pub version: String,
    pub status: Status,
    pub headers: Headers,
    pub body: Body,
}

#[derive(Debug)]
pub enum Status {
    Ok,
    NotFound,
    Created,
    InternalServerError,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = match self {
            Status::Ok => "200 OK",
            Status::NotFound => "404 Not Found",
            Status::Created => "201 Created",
            Status::InternalServerError => "500 Internal Server Error",
        };

        write!(f, "{}", status)
    }
}

impl Response {
    pub fn new() -> Self {
        Self {
            version: "HTTP/1.1".to_string(),
            status: Status::Ok,
            headers: Headers(HashMap::new()),
            body: Body("".to_string()),
        }
    }

    pub fn set_status(&mut self, status: Status) {
        self.status = status;
    }

    pub fn set_header(&mut self, key: String, value: String) {
        self.headers.0.insert(key, value);
    }

    pub fn set_plain_text_body(&mut self, body: Body) {
        self.headers
            .0
            .insert("Content-Length".to_string(), body.0.len().to_string());
        self.headers
            .0
            .insert("Content-Type".to_string(), "text/plain".to_string());
        self.body = body;
    }

    pub fn set_file_body(&mut self, body: Body) {
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

        response.push_str(&self.status.to_string());
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
