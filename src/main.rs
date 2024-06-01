use anyhow::Result;
use http_server_starter_rust::config::Config;
use std::env;
use std::net::TcpListener;
use std::sync::Arc;

const ADDRESS: &str = "127.0.0.1:4221";

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind(ADDRESS)?;
    println!("Server started on: {}", ADDRESS);

    let config = Arc::new(Config::new(env::args()));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");

                let config = Arc::clone(&config);
                tokio::spawn(async move {
                    http_server_starter_rust::handle_connection(stream, config)
                        .await
                        .unwrap_or_else(|e| {
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
