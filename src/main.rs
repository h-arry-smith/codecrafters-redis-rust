use anyhow::Result;
use redis::CommandMessage;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{
        mpsc::{self, Sender},
        oneshot,
    },
};

mod rdb;
mod redis;
mod resp;

async fn handle_connection(stream: &mut TcpStream, tx: Sender<CommandMessage>) {
    loop {
        let mut buffer = [0; 1024];
        let read_amount = stream.read(&mut buffer).await.unwrap();

        if read_amount == 0 {
            break;
        }

        let (resp_tx, resp_rx) = oneshot::channel();
        let received_string = String::from_utf8_lossy(&buffer[..read_amount]).to_string();
        tx.send((received_string, resp_tx)).await.unwrap();

        let response = resp_rx.await.unwrap();
        stream.write_all(response.as_bytes()).await.unwrap();
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let (tx, mut rx) = mpsc::channel(32);

    let server_task = tokio::spawn(async move {
        let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();

        loop {
            let mut handles = Vec::new();
            let (mut stream, _) = listener.accept().await.unwrap();

            let task_tx = tx.clone();
            handles.push(tokio::spawn(async move {
                handle_connection(&mut stream, task_tx).await;
            }));
        }
    });

    let redis_task = tokio::spawn(async move {
        let mut redis = redis::Redis::new(args);

        while let Some((message, resp)) = rx.recv().await {
            println!("Received command over mpsc: {:?}", message);
            redis.handle_message(message, resp).await;
        }
    });

    server_task.await.unwrap();
    redis_task.await.unwrap();

    Ok(())
}
