use crate::oneshot;
use oneshot::Sender;

pub type CommandMessage = (Command, Sender<String>);

pub struct Redis {}

impl Redis {
    pub fn new() -> Redis {
        Redis {}
    }

    pub fn handle_command(&self, command: Command, resp: Sender<String>) {
        match command {
            Command::Ping => {
                resp.send("+PONG\r\n".to_string()).unwrap();
            }
        }
    }
}

#[derive(Debug)]
pub enum Command {
    Ping,
}
