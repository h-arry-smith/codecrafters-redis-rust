use core::panic;
use std::collections::HashMap;

use crate::{oneshot, resp::Resp};
use bytes::Bytes;
use oneshot::Sender;

pub type CommandMessage = (String, Sender<String>);

enum RedisValue {
    String(String),
}

pub struct Redis {
    store: HashMap<String, RedisValue>,
}

impl Redis {
    pub fn new() -> Redis {
        Redis {
            store: HashMap::new(),
        }
    }

    pub async fn handle_message(&mut self, message: String, resp: Sender<String>) {
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

    pub fn parse_command(command: Resp, args: Vec<Resp>) -> Command {
        let command = command.to_string().to_lowercase();

        // TODO: Args might be empty/wrong, handle these cases
        match command.as_str() {
            "ping" => Command::Ping,
            "echo" => {
                let message = args[0].to_string();
                Command::Echo { message }
            }
            "set" => {
                let key = args[0].to_string();
                let value = args[1].to_string();
                Command::Set { key, value }
            }
            "get" => {
                let key = args[0].to_string();
                Command::Get { key }
            }
            _ => {
                panic!("Invalid command");
            }
        }
    }

    pub fn handle_command(&mut self, command: Command) -> Resp {
        match command {
            Command::Ping => Resp::SimpleString("PONG".to_string()),
            Command::Echo { message } => Resp::BulkString(Bytes::from(message)),
            Command::Set { key, value } => {
                self.store.insert(key, RedisValue::String(value));
                Resp::SimpleString("OK".to_string())
            }
            Command::Get { key } => {
                let value = self.store.get(&key).unwrap();
                match value {
                    RedisValue::String(value) => Resp::BulkString(Bytes::from(value.clone())),
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum Command {
    Ping,
    Echo { message: String },
    Set { key: String, value: String },
    Get { key: String },
}
