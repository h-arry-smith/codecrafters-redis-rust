use anyhow::{Context, Result};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

fn handle_connection(stream: &mut TcpStream) -> Result<()> {
    loop {
        let mut buffer = [0; 1024];
        let read_amount = stream
            .read(&mut buffer)
            .context("Failed to read from stream")?;

        if read_amount == 0 {
            break;
        }

        stream
            .write_all(b"+PONG\r\n")
            .context("Failed to write to stream")?;
    }

    Ok(())
}

fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                handle_connection(&mut stream)?;
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }

    Ok(())
}
