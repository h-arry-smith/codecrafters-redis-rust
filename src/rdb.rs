use std::{collections::HashMap, path::PathBuf};

use crate::redis::RedisValue;

pub struct Rdb {}

impl Rdb {
    pub fn load_from_path(path: PathBuf) -> HashMap<String, RedisValue> {
        let mut store = HashMap::new();

        if !path.exists() {
            return store;
        }

        let file_contents = std::fs::read(path).unwrap();
        let slice = file_contents.as_slice();
        let mut seek = 0;

        // The file starts off with the magic string “REDIS”
        assert!(slice[seek..].starts_with(b"REDIS"));
        seek += 5;

        // The next 4 bytes store the version number of the rdb format.
        // The 4 bytes are interpreted as ASCII characters and then converted to an integer using string to integer conversion.
        let _version = std::str::from_utf8(&slice[seek..seek + 4]).unwrap();
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
                0xFA => {
                    let length_encoding = Rdb::read_length_encoding(slice, &mut seek);
                    let _first_string = length_encoding.decode_from(slice, &mut seek);
                    let length_encoding = Rdb::read_length_encoding(slice, &mut seek);
                    let _second_string = length_encoding.decode_from(slice, &mut seek);
                }
                0xFE => {
                    let length_encoding = Rdb::read_length_encoding(slice, &mut seek);
                    let _key = length_encoding.decode_from(slice, &mut seek);
                }
                0xFB => {
                    // TODO: Docs are poor for this opcode, this might not be correct when size exceeds a byte
                    let _db_hash_table_size = slice[seek];
                    let _expiry_hash_table_size = slice[seek + 1];
                    seek += 2;
                }
                0x0 => {
                    let key =
                        Rdb::read_length_encoding(slice, &mut seek).decode_from(slice, &mut seek);
                    let value =
                        Rdb::read_length_encoding(slice, &mut seek).decode_from(slice, &mut seek);

                    store.insert(key, RedisValue::String(value));
                }
                _ => todo!("opcode: 0x{:X} not implemented", opcode),
            }
        }

        store
    }

    fn read_length_encoding(slice: &[u8], seek: &mut usize) -> LengthEncoding {
        let first_byte = slice[*seek];
        *seek += 1;

        let ms_bits = first_byte >> 6;
        match ms_bits {
            0b00 => {
                // 6 bit encoding
                LengthEncoding::SixBit(first_byte & 0b0011_1111)
            }
            0b01 => {
                // 14 bit encoding
                let second_byte = slice[*seek];
                *seek += 1;
                let masked_first_byte = first_byte & 0b0011_1111;

                LengthEncoding::FourteenBit((masked_first_byte as u16) << 8 | (second_byte as u16))
            }
            0b10 => {
                // 32 bit encoding
                let first_byte = slice[*seek];
                *seek += 1;
                let second_byte = slice[*seek];
                *seek += 1;
                let third_byte = slice[*seek];
                *seek += 1;
                let fourth_byte = slice[*seek];
                *seek += 1;
                LengthEncoding::ThirtyTwoBit(
                    first_byte as u32
                        | ((second_byte as u32) << 8)
                        | ((third_byte as u32) << 16)
                        | ((fourth_byte as u32) << 24),
                )
            }
            0b11 => {
                // encoded length
                LengthEncoding::Encoded(first_byte & 0b0011_1111)
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
enum LengthEncoding {
    SixBit(u8),
    FourteenBit(u16),
    ThirtyTwoBit(u32),
    Encoded(u8),
}

impl LengthEncoding {
    fn decode_from(&self, slice: &[u8], seek: &mut usize) -> String {
        match self {
            LengthEncoding::SixBit(length) => {
                let str =
                    String::from_utf8_lossy(&slice[*seek..*seek + *length as usize]).to_string();
                *seek += *length as usize;
                str
            }
            LengthEncoding::FourteenBit(length) => {
                let str =
                    String::from_utf8_lossy(&slice[*seek..*seek + *length as usize]).to_string();
                *seek += *length as usize;
                str
            }
            LengthEncoding::ThirtyTwoBit(length) => {
                let str =
                    String::from_utf8_lossy(&slice[*seek..*seek + *length as usize]).to_string();
                *seek += *length as usize;
                str
            }
            LengthEncoding::Encoded(format) => {
                match format {
                    0 => {
                        // string encoded as 8 bit integer
                        let integer = slice[0];
                        *seek += 1;
                        format!("{}", integer)
                    }
                    1 => {
                        // string encoded as 16 bit integer
                        let integer = (slice[0] as u16) << 8 | (slice[1] as u16);
                        *seek += 2;
                        format!("{}", integer)
                    }
                    2 => {
                        // string encoded as 32 bit integer
                        let integer = (slice[0] as u32)
                            | ((slice[1] as u32) << 8)
                            | ((slice[2] as u32) << 16)
                            | ((slice[3] as u32) << 24);
                        *seek += 4;
                        format!("{}", integer)
                    }
                    3 => todo!("compressed string decoding not implemented yet"),
                    _ => unreachable!(),
                }
            }
        }
    }
}
