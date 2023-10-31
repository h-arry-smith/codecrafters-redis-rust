use std::{collections::HashMap, path::PathBuf};

use crate::redis::RedisValue;

pub struct Rdb {}

impl Rdb {
    pub fn load_from_path(path: PathBuf) -> HashMap<String, RedisValue> {
        dbg!(&path);
        let store = HashMap::new();

        let file_contents = std::fs::read(path).unwrap();
        let slice = file_contents.as_slice();
        let mut seek = 0;

        // The file starts off with the magic string “REDIS”
        assert!(slice[seek..].starts_with(b"REDIS"));
        seek += 5;

        // The next 4 bytes store the version number of the rdb format.
        // The 4 bytes are interpreted as ASCII characters and then converted to an integer using string to integer conversion.
        let version = std::str::from_utf8(&slice[seek..seek + 4]).unwrap();
        eprintln!("version: {}", version);
        seek += 4;

        loop {
            if seek >= slice.len() {
                break;
            }

            // Each part after the initial header is introduced by a special op code.
            let opcode = slice[seek];
            seek += 1;

            match opcode {
                0xFF => {
                    // End of the RDB file.
                    break;
                }
                _ => todo!("opcode: 0x{:X} not implemented", opcode),
            }
        }

        store
    }
}
