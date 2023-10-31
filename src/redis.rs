use core::panic;
use std::{collections::HashMap, time::Instant};

use crate::{oneshot, resp::Resp};
use bytes::Bytes;
use oneshot::Sender;

pub type CommandMessage = (String, Sender<String>);

enum RedisValue {
    String(String),
}

pub struct Redis {
    store: HashMap<String, RedisValue>,
    expiry_table: HashMap<String, Instant>,
}

impl Redis {
    pub fn new() -> Redis {
        Redis {
            store: HashMap::new(),
            expiry_table: HashMap::new(),
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
            "set" => Self::parse_set_command(args),
            "get" => {
                let key = args[0].to_string();
                Command::Get { key }
            }
            _ => {
                panic!("Invalid command");
            }
        }
    }

    pub fn parse_set_command(args: Vec<Resp>) -> Command {
        let mut args = args.iter();
        let key = args.next().unwrap().to_string();
        let value = args.next().unwrap().to_string();

        let mut options = Vec::new();

        while let Some(arg) = args.next() {
            match arg.to_string().as_str() {
                "px" => {
                    let value = args.next().unwrap().to_string();
                    options.push(("px".to_string(), Some(value)));
                }
                _ => todo!("arg: {} not implemented", arg),
            }
        }

        Command::Set {
            key,
            value,
            options,
        }
    }

    pub fn handle_command(&mut self, command: Command) -> Resp {
        match command {
            Command::Ping => Resp::SimpleString("PONG".to_string()),
            Command::Echo { message } => Resp::BulkString(Bytes::from(message)),
            Command::Set {
                key,
                value,
                options,
            } => self.set(key, value, options),
            Command::Get { key } => self.get(key),
        }
    }

    fn set(&mut self, key: String, value: String, options: Vec<(String, Option<String>)>) -> Resp {
        let mut expiry = None;
        for (option, value) in options {
            match option.as_str() {
                "px" => {
                    expiry = Some(value.unwrap().parse::<u64>().unwrap());
                }
                _ => todo!("option: {} not implemented", option),
            }
        }

        if let Some(expiry) = expiry {
            let now = Instant::now();
            let expiry = now + std::time::Duration::from_millis(expiry);
            self.expiry_table.insert(key.clone(), expiry);
        } else {
            self.expiry_table.remove(&key);
        }

        self.store.insert(key, RedisValue::String(value));
        Resp::SimpleString("OK".to_string())
    }

    fn get(&self, key: String) -> Resp {
        if let Some(expiry) = self.expiry_table.get(&key) {
            if expiry < &Instant::now() {
                return Resp::Null;
            }
        }

        match self.store.get(&key) {
            Some(RedisValue::String(value)) => Resp::BulkString(Bytes::from(value.clone())),
            None => Resp::Null,
        }
    }
}

#[derive(Debug)]
pub enum Command {
    Ping,
    Echo {
        message: String,
    },
    Set {
        key: String,
        value: String,
        options: Vec<(String, Option<String>)>,
    },
    Get {
        key: String,
    },
}
