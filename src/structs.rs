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

            let lowercase_key = parts.0.to_lowercase();

            headers.insert(lowercase_key, parts.1.to_string());
        }

        Ok(Self(headers))
    }

    pub fn set(&mut self, key: String, value: String) {
        self.0.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.0.get(key)
    }

    pub fn set_content_type(&mut self, content_type: ContentType) {
        self.set("content-type".to_string(), content_type.to_string());
    }

    pub fn set_content_length(&mut self, length: usize) {
        self.set("content-length".to_string(), length.to_string());
    }

    pub fn get_content_length(&self) -> Option<usize> {
        self.get("content-length")
            .and_then(|length| length.parse::<usize>().ok())
    }

    pub fn set_content_encoding(&mut self, encoding: ContentEncoding) {
        self.set("content-encoding".to_string(), encoding.to_string());
    }

    pub fn get_accept_encoding(&self) -> Option<&String> {
        self.get("accept-encoding")
    }

    pub fn get_user_agent(&self) -> Option<&String> {
        self.get("user-agent")
    }
}

pub enum ContentType {
    PlainText,
    OctetStream,
}

impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContentType::PlainText => write!(f, "text/plain"),
            ContentType::OctetStream => write!(f, "application/octet-stream"),
        }
    }
}

pub enum ContentEncoding {
    Gzip,
}

impl fmt::Display for ContentEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContentEncoding::Gzip => write!(f, "gzip"),
        }
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
        self.headers.set_content_type(ContentType::PlainText);
        self.headers.set_content_length(body.0.len());
        self.body = body;
    }

    pub fn set_file_body(&mut self, body: Body) {
        self.headers.set_content_type(ContentType::OctetStream);
        self.headers.set_content_length(body.0.len());
        self.body = body;
    }

    pub fn apply_compression(&mut self, accept_encoding: Option<&String>) {
        if accept_encoding.is_none() {
            return;
        }

        let accept_encoding = accept_encoding.unwrap();

        if accept_encoding.contains("gzip") {
            self.headers.set_content_encoding(ContentEncoding::Gzip);
        }
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
