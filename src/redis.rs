use core::panic;

use crate::{oneshot, resp::Resp};
use oneshot::Sender;

pub type CommandMessage = (String, Sender<String>);

pub struct Redis {}

impl Redis {
    pub fn new() -> Redis {
        Redis {}
    }

    pub async fn handle_message(&self, message: String, resp: Sender<String>) {
        let decoded_message = Resp::decode(&message.to_lowercase()).unwrap();
        let (command, args) = match decoded_message {
            Resp::Array(array) => {
                let mut iter = array.into_iter();
                let command = iter.next().unwrap();
                let args = iter.collect::<Vec<_>>();
                (command, args)
            }
            _ => {
                panic!("Invalid message");
            }
        };

        let command = Redis::parse_command(command, args);

        let response = self.handle_command(command);
        let encoded_response = response.encoded().unwrap();
        resp.send(encoded_response).unwrap();
    }

    pub fn parse_command(command: Resp, _args: Vec<Resp>) -> Command {
        let command = command.to_string().to_lowercase();

        match command.as_str() {
            "ping" => Command::Ping,
            _ => {
                panic!("Invalid command");
            }
        }
    }

    pub fn handle_command(&self, command: Command) -> Resp {
        match command {
            Command::Ping => {
                eprintln!("Received ping command");
                Resp::SimpleString("PONG".to_string())
            }
        }
    }
}

#[derive(Debug)]
pub enum Command {
    Ping,
}
