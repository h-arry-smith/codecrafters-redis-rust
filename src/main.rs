use anyhow::Result;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{
        mpsc::{self, Sender},
        oneshot,
    },
};

#[derive(Debug)]
enum Command {
    Ping { resp: oneshot::Sender<String> },
}

async fn handle_connection(stream: &mut TcpStream, tx: Sender<Command>) {
    loop {
        let mut buffer = [0; 1024];
        let read_amount = stream.read(&mut buffer).await.unwrap();

        eprintln!(
            "Read {} bytes for stream {}",
            read_amount,
            stream.local_addr().unwrap()
        );

        if read_amount == 0 {
            break;
        }

        let (resp_tx, resp_rx) = oneshot::channel();

        tx.send(Command::Ping { resp: resp_tx }).await.unwrap();
        eprintln!("send ping command over oneshot");

        let response = resp_rx.await.unwrap();
        eprintln!("received response over oneshot");
        stream.write_all(response.as_bytes()).await.unwrap();
        eprintln!("wrote response to stream {}", stream.peer_addr().unwrap());
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let (tx, mut rx) = mpsc::channel(32);

    let server = tokio::spawn(async move {
        let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
        eprintln!("Listening on {}", listener.local_addr().unwrap());

        loop {
            let mut handles = Vec::new();
            let (mut stream, _) = listener.accept().await.unwrap();
            eprintln!("Accepted connection from {}", stream.peer_addr().unwrap());

            let task_tx = tx.clone();
            handles.push(tokio::spawn(async move {
                handle_connection(&mut stream, task_tx).await;
            }));
        }
    });

    let message_handler = tokio::spawn(async move {
        while let Some(cmd) = rx.recv().await {
            println!("Received command over mpsc: {:?}", cmd);
            match cmd {
                Command::Ping { resp } => {
                    println!("Sending pong over oneshot");
                    resp.send("+PONG\r\n".to_string()).unwrap();
                }
            }
        }
    });

    server.await.unwrap();
    message_handler.await.unwrap();

    Ok(())
}
