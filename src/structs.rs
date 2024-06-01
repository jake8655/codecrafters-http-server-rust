#![allow(dead_code)]

use anyhow::Result;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::collections::HashMap;
use std::fmt;
use std::io::Write;
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

    pub fn is_gzip_encoding(&self) -> bool {
        self.headers
            .get_accept_encoding()
            .map(|encoding| encoding.contains("gzip"))
            .unwrap_or(false)
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
pub struct Body(pub Vec<u8>);

impl Body {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn from_str(s: &str) -> Self {
        let mut body = Self::new();
        body.push_str(s);

        body
    }

    pub fn from_lines(lines: Vec<String>) -> Self {
        let mut buffer = Vec::new();
        buffer.extend_from_slice(lines.join("\r\n").as_bytes());

        Self(buffer)
    }

    pub fn push_str(&mut self, s: &str) {
        self.0.extend_from_slice(s.as_bytes());
    }
}

impl fmt::Display for Body {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.0))
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
            body: Body(Vec::new()),
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

    pub fn apply_compression(&mut self, request: &Request) {
        if !request.is_gzip_encoding() {
            return;
        }

        self.headers.set_content_encoding(ContentEncoding::Gzip);

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&self.body.0).unwrap();
        let compressed_body = encoder.finish().unwrap();

        self.body = Body(compressed_body);
        self.headers.set_content_length(self.body.0.len());
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::new();

        buffer.extend_from_slice(format!("{} ", self.version).as_bytes());
        buffer.extend_from_slice(format!("{}\r\n", self.status).as_bytes());

        for (key, value) in &self.headers.0 {
            buffer.extend_from_slice(format!("{}: {}\r\n", key, value).as_bytes());
        }
        buffer.extend_from_slice(b"\r\n");

        buffer.extend_from_slice(&self.body.0);

        buffer
    }
}
