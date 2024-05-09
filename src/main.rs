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

fn handle_connection(mut stream: std::net::TcpStream) -> Result<()> {
    let _ = stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes())?;

    Ok(())
}
