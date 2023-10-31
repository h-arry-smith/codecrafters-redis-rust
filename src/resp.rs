use std::fmt::Display;

use bytes::{Buf, Bytes};

#[derive(Debug, PartialEq)]
pub enum Resp {
    SimpleString(String),
    SimpleError(String),
    Integer(i64),
    BulkString(Bytes),
    Array(Vec<Resp>),
    Null,
    Boolean(bool),
    Double(f64),
    // NOTE: BigNum not included because needs additional crates
    // TODO: Bulk Error, Verbatim Strings, Maps, Sets, Pushes
    //       I've done more than enough to get the idea :^)
}

// NOTE: Bytes may have been the wrong choice here, and a BufReader would have been less
// .     cludgy. Converting back to a string all the time is horrible.

impl Resp {
    pub fn encoded(&self) -> Result<String, ()> {
        match self {
            Resp::SimpleString(s) => Self::encode_simple_string(s),
            Resp::SimpleError(s) => Self::encode_simple_error(s),
            Resp::Integer(i) => Self::encode_integer(i),
            Resp::BulkString(bytes) => Self::encode_bulk_string(bytes),
            Resp::Null => Self::encode_null(),
            Resp::Array(arr) => Self::encode_array(arr),
            Resp::Boolean(bool) => Self::encode_bool(bool),
            Resp::Double(double) => Self::encode_double(double),
        }
    }

    fn encode_simple_string(s: &str) -> Result<String, ()> {
        // The string mustn't contain a CR (\r) or LF (\n) character and is terminated by CRLF (i.e., \r\n).
        if s.contains('\n') || s.contains('\r') {
            return Err(());
        }

        Ok(format!("+{}\r\n", s))
    }

    fn encode_simple_error(s: &str) -> Result<String, ()> {
        // The string mustn't contain a CR (\r) or LF (\n) character and is terminated by CRLF (i.e., \r\n).
        if s.contains('\n') || s.contains('\r') {
            return Err(());
        }

        Ok(format!("-{}\r\n", s))
    }

    fn encode_integer(int: &i64) -> Result<String, ()> {
        // The null bulk string represents a non-existing value.
        // It is encoded as a bulk string with the length of negative one (-1)
        Ok(format!(":{}\r\n", int))
    }

    fn encode_bulk_string(bytes: &Bytes) -> Result<String, ()> {
        let len = bytes.len();
        let string = String::from_utf8(bytes.to_vec()).map_err(|_| ())?;

        Ok(format!("${}\r\n{}\r\n", len, string))
    }

    fn encode_null() -> Result<String, ()> {
        Ok("$-1\r\n".to_string())
    }

    fn encode_array(arr: &[Resp]) -> Result<String, ()> {
        let mut encoded = String::new();
        encoded.push_str(&format!("*{}\r\n", arr.len()));

        for resp in arr {
            encoded.push_str(&resp.encoded()?);
        }

        Ok(encoded)
    }

    fn encode_bool(bool: &bool) -> Result<String, ()> {
        if *bool {
            Ok("#t\r\n".to_string())
        } else {
            Ok("#f\r\n".to_string())
        }
    }

    fn encode_double(double: &f64) -> Result<String, ()> {
        Ok(format!(",{}\r\n", double))
    }

    pub fn decode(s: &str) -> Result<Resp, ()> {
        // The \r\n (CRLF) is the protocol's terminator, which always separates its parts.
        if !s.ends_with("\r\n") {
            return Err(());
        }

        let mut bytes = Bytes::from(s.to_string());
        Self::decode_bytes(&mut bytes)
    }

    fn decode_bytes(bytes: &mut Bytes) -> Result<Resp, ()> {
        let first_char = *(bytes.first().unwrap()) as char;
        match first_char {
            '+' => Self::decode_simple_string(bytes),
            '-' => Self::decode_simple_error(bytes),
            ':' => Self::decode_integer(bytes),
            '$' => Self::decode_bulk_string(bytes),
            '*' => Self::decode_array(bytes),
            '#' => Self::decode_boolean(bytes),
            ',' => Self::decode_double(bytes),
            _ => Err(()),
        }
    }

    fn decode_simple_string(b: &mut Bytes) -> Result<Resp, ()> {
        b.advance(1);
        let (string, _) = b.split_at(b.len());
        let string = String::from_utf8_lossy(string).to_string();
        let (string, _) = string.split_once("\r\n").ok_or(())?;
        b.advance(string.len() + 2);

        Ok(Resp::SimpleString(string.to_string()))
    }

    fn decode_simple_error(b: &mut Bytes) -> Result<Resp, ()> {
        b.advance(1);
        let (string, _) = b.split_at(b.len());
        let string = String::from_utf8_lossy(string).to_string();
        let (string, _) = string.split_once("\r\n").ok_or(())?;
        b.advance(string.len() + 2);

        Ok(Resp::SimpleError(string.to_string()))
    }

    fn decode_integer(b: &mut Bytes) -> Result<Resp, ()> {
        b.advance(1);

        let (string, _) = b.split_at(b.len());
        let string = String::from_utf8_lossy(string);

        let (int_str, _) = string.split_once("\r\n").ok_or(())?;
        let int = int_str.parse::<i64>().map_err(|_| ())?;
        b.advance(int_str.len() + 2);

        Ok(Resp::Integer(int))
    }

    fn decode_bulk_string(b: &mut Bytes) -> Result<Resp, ()> {
        b.advance(1);
        let (string, _) = b.split_at(b.len() - 2);
        let string = String::from_utf8_lossy(string);

        if string == "-1" {
            b.advance(4);
            return Ok(Resp::Null);
        }

        let (len_str, remaining) = string.split_once("\r\n").ok_or(())?;
        let len = len_str.parse::<usize>().map_err(|_| ())?;

        if len == 0 {
            return Ok(Resp::BulkString(Bytes::new()));
        }

        let (bytes, _) = remaining.split_at(len);
        let bytes = Bytes::from(bytes.to_string());

        b.advance(len_str.len() + 2);
        b.advance(len + 2);
        Ok(Resp::BulkString(bytes))
    }

    fn decode_array(b: &mut Bytes) -> Result<Resp, ()> {
        b.advance(1);

        let string = String::from_utf8_lossy(b);
        dbg!(&string);

        let (len_str, _) = string.split_once("\r\n").ok_or(())?;
        let len = len_str.parse::<usize>().map_err(|_| ())?;

        dbg!(&len);
        b.advance(len_str.len() + 2);

        let mut arr = Vec::with_capacity(len);
        for _ in 0..len {
            dbg!(&b);
            let resp = Self::decode_bytes(b)?;
            arr.push(resp);
        }

        Ok(Resp::Array(arr))
    }

    fn decode_boolean(b: &mut Bytes) -> Result<Resp, ()> {
        b.advance(1);
        let (string, _) = b.split_at(b.len());
        let string = String::from_utf8_lossy(string).to_string();
        let (string, _) = string.split_once("\r\n").ok_or(())?;
        b.advance(string.len() + 2);

        if string == "t" {
            Ok(Resp::Boolean(true))
        } else if string == "f" {
            Ok(Resp::Boolean(false))
        } else {
            Err(())
        }
    }

    fn decode_double(b: &mut Bytes) -> Result<Resp, ()> {
        b.advance(1);
        let (string, _) = b.split_at(b.len());
        let string = String::from_utf8_lossy(string).to_string();
        let (string, _) = string.split_once("\r\n").ok_or(())?;
        b.advance(string.len() + 2);

        let double = string.parse::<f64>().map_err(|_| ())?;
        Ok(Resp::Double(double))
    }
}

impl Display for Resp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Resp::SimpleString(s) => write!(f, "{}", s),
            Resp::SimpleError(s) => write!(f, "{}", s),
            Resp::Integer(i) => write!(f, "{}", i),
            Resp::BulkString(b) => write!(f, "{}", String::from_utf8_lossy(b)),
            Resp::Array(b) => {
                let mut s = String::from("[");
                for resp in b {
                    s.push_str(&format!("{},", resp));
                }
                s.push(']');
                write!(f, "{}", s)
            }
            Resp::Null => write!(f, "null"),
            Resp::Boolean(b) => write!(f, "{}", b),
            Resp::Double(d) => write!(f, "{}", d),
        }
    }
}

mod test {
    #[allow(unused_imports)]
    use crate::resp::Resp;
    #[allow(unused_imports)]
    use bytes::Bytes;

    #[test]
    fn encode_simple_string() {
        let resp = Resp::SimpleString("PONG".to_string());
        assert_eq!(resp.encoded().unwrap(), "+PONG\r\n");
    }

    #[test]
    fn cant_encode_simple_string_with_newline() {
        let resp = Resp::encode_simple_string("PO\nN\rG");
        assert!(resp.is_err());
    }

    #[test]
    fn decode_simple_string() {
        let resp_str = "+PONG\r\n";
        let resp = Resp::decode(resp_str).unwrap();
        assert_eq!(resp, Resp::SimpleString("PONG".to_string()));
    }

    #[test]
    fn encode_simple_error() {
        let resp = Resp::SimpleError("ERR".to_string());
        assert_eq!(resp.encoded().unwrap(), "-ERR\r\n");
    }

    #[test]
    fn cant_encode_simple_error_with_newline() {
        let resp = Resp::encode_simple_error("ER\nR\r");
        assert!(resp.is_err());
    }

    #[test]
    fn decode_simple_error() {
        let resp_str = "-ERR\r\n";
        let resp = Resp::decode(resp_str).unwrap();
        assert_eq!(resp, Resp::SimpleError("ERR".to_string()));
    }

    #[test]
    fn encode_positive_integer() {
        let resp = Resp::Integer(42);
        assert_eq!(resp.encoded().unwrap(), ":42\r\n");
    }

    #[test]
    fn encode_negative_integer() {
        let resp = Resp::Integer(-42);
        assert_eq!(resp.encoded().unwrap(), ":-42\r\n");
    }

    #[test]
    fn decode_integer_with_no_sign() {
        let resp_str = ":42\r\n";
        let resp = Resp::decode(resp_str).unwrap();
        assert_eq!(resp, Resp::Integer(42));
    }

    #[test]
    fn decode_integer_with_positive_sign() {
        let resp_str = ":+42\r\n";
        let resp = Resp::decode(resp_str).unwrap();
        assert_eq!(resp, Resp::Integer(42));
    }

    #[test]
    fn decode_integer_with_negative_sign() {
        let resp_str = ":-42\r\n";
        let resp = Resp::decode(resp_str).unwrap();
        assert_eq!(resp, Resp::Integer(-42));
    }

    #[test]
    fn encode_empty_bulk_string() {
        let resp = Resp::BulkString(Bytes::new());
        assert_eq!(resp.encoded().unwrap(), "$0\r\n\r\n");
    }

    #[test]
    fn encode_hello_string() {
        let resp = Resp::BulkString(Bytes::from("hello"));
        assert_eq!(resp.encoded().unwrap(), "$5\r\nhello\r\n");
    }

    #[test]
    fn encode_null_bulk_string() {
        let resp = Resp::Null;
        assert_eq!(resp.encoded().unwrap(), "$-1\r\n");
    }

    #[test]
    fn decode_empty_bulk_string() {
        let resp_str = "$0\r\n\r\n";
        let resp = Resp::decode(resp_str).unwrap();
        assert_eq!(resp, Resp::BulkString(Bytes::new()));
    }

    #[test]
    fn decode_hello_bulk_string() {
        let resp_str = "$5\r\nhello\r\n";
        let resp = Resp::decode(resp_str).unwrap();
        assert_eq!(resp, Resp::BulkString(Bytes::from("hello")));
    }

    #[test]
    fn decode_null() {
        let resp_str = "$-1\r\n";
        let resp = Resp::decode(resp_str).unwrap();
        assert_eq!(resp, Resp::Null);
    }

    #[test]
    fn encode_empty_array() {
        let resp = Resp::Array(vec![]);
        assert_eq!(resp.encoded().unwrap(), "*0\r\n");
    }

    #[test]
    fn encode_array_of_strings() {
        let resp = Resp::Array(vec![
            Resp::SimpleString("foo".to_string()),
            Resp::SimpleString("bar".to_string()),
        ]);
        assert_eq!(resp.encoded().unwrap(), "*2\r\n+foo\r\n+bar\r\n");
    }

    #[test]
    fn encode_array_of_bulk_strings() {
        let resp = Resp::Array(vec![
            Resp::BulkString(Bytes::from("foo")),
            Resp::BulkString(Bytes::from("bar")),
        ]);
        assert_eq!(resp.encoded().unwrap(), "*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n");
    }

    #[test]
    fn encode_array_of_integers_and_bulk_strings() {
        let resp = Resp::Array(vec![
            Resp::Integer(42),
            Resp::BulkString(Bytes::from("foo")),
            Resp::Integer(-42),
        ]);
        assert_eq!(
            resp.encoded().unwrap(),
            "*3\r\n:42\r\n$3\r\nfoo\r\n:-42\r\n"
        );
    }

    #[test]
    fn decode_the_empty_array() {
        let resp_str = "*0\r\n";
        let resp = Resp::decode(resp_str).unwrap();
        assert_eq!(resp, Resp::Array(vec![]));
    }

    #[test]
    fn decode_the_hello_world_array() {
        let resp_str = "*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n";
        let resp = Resp::decode(resp_str).unwrap();
        assert_eq!(
            resp,
            Resp::Array(vec![
                Resp::BulkString(Bytes::from("hello")),
                Resp::BulkString(Bytes::from("world")),
            ])
        );
    }

    #[test]
    fn decode_array_with_bulk_string_and_simple_string_and_integers() {
        let resp_str = "*5\r\n$3\r\nfoo\r\n:1\r\n:-2\r\n:3\r\n$3\r\nbar\r\n";
        let resp = Resp::decode(resp_str).unwrap();
        assert_eq!(
            resp,
            Resp::Array(vec![
                Resp::BulkString(Bytes::from("foo")),
                Resp::Integer(1),
                Resp::Integer(-2),
                Resp::Integer(3),
                Resp::BulkString(Bytes::from("bar")),
            ])
        );
    }

    #[test]
    fn decode_array_of_simple_strings() {
        let resp_str = "*2\r\n+foo\r\n+bar\r\n";
        let resp = Resp::decode(resp_str).unwrap();
        assert_eq!(
            resp,
            Resp::Array(vec![
                Resp::SimpleString("foo".to_string()),
                Resp::SimpleString("bar".to_string()),
            ])
        );
    }

    #[test]
    fn decode_nested_arrays() {
        let resp_str = "*2\r\n*3\r\n:1\r\n:2\r\n:3\r\n*2\r\n+Foo\r\n-Bar\r\n";
        let resp = Resp::decode(resp_str).unwrap();
        assert_eq!(
            resp,
            Resp::Array(vec![
                Resp::Array(vec![Resp::Integer(1), Resp::Integer(2), Resp::Integer(3),]),
                Resp::Array(vec![
                    Resp::SimpleString("Foo".to_string()),
                    Resp::SimpleError("Bar".to_string()),
                ]),
            ])
        );
    }

    #[test]
    fn encode_true_boolean() {
        let resp = Resp::Boolean(true);
        assert_eq!(resp.encoded().unwrap(), "#t\r\n");
    }

    #[test]
    fn encode_false_boolean() {
        let resp = Resp::Boolean(false);
        assert_eq!(resp.encoded().unwrap(), "#f\r\n");
    }

    #[test]
    fn decode_true_boolean() {
        let resp_str = "#t\r\n";
        let resp = Resp::decode(resp_str).unwrap();
        assert_eq!(resp, Resp::Boolean(true));
    }

    #[test]
    fn decode_false_boolean() {
        let resp_str = "#f\r\n";
        let resp = Resp::decode(resp_str).unwrap();
        assert_eq!(resp, Resp::Boolean(false));
    }

    #[test]
    fn encode_simple_double() {
        let resp = Resp::Double(5.673);
        assert_eq!(resp.encoded().unwrap(), ",5.673\r\n");
    }

    #[test]
    fn encode_double_without_fraction() {
        let resp = Resp::Double(5.0);
        assert_eq!(resp.encoded().unwrap(), ",5\r\n");
    }

    #[test]
    fn encode_double_with_exponential_part() {
        let resp = Resp::Double(5.0e3);
        assert_eq!(resp.encoded().unwrap(), ",5000\r\n");
    }

    #[test]
    fn encode_double_with_neg_exponential_part() {
        let resp = Resp::Double(5.0e-3);
        assert_eq!(resp.encoded().unwrap(), ",0.005\r\n");
    }

    #[test]
    fn decode_double() {
        let resp_str = ",5.673\r\n";
        let resp = Resp::decode(resp_str).unwrap();
        assert_eq!(resp, Resp::Double(5.673));
    }

    #[test]
    fn decode_double_without_fraction() {
        let resp_str = ",5\r\n";
        let resp = Resp::decode(resp_str).unwrap();
        assert_eq!(resp, Resp::Double(5.0));
    }

    #[test]
    fn decode_dobule_with_exponential_part() {
        let resp_str = ",5.0e3\r\n";
        let resp = Resp::decode(resp_str).unwrap();
        assert_eq!(resp, Resp::Double(5.0e3));
    }

    #[test]
    fn decode_double_with_neg_exponential_part() {
        let resp_str = ",-5.0e-3\r\n";
        let resp = Resp::decode(resp_str).unwrap();
        assert_eq!(resp, Resp::Double(-5.0e-3));
    }

    #[test]
    fn encode_positive_infinity() {
        let resp = Resp::Double(f64::INFINITY);
        assert_eq!(resp.encoded().unwrap(), ",inf\r\n");
    }

    #[test]
    fn encode_negative_infinity() {
        let resp = Resp::Double(f64::NEG_INFINITY);
        assert_eq!(resp.encoded().unwrap(), ",-inf\r\n");
    }

    #[test]
    fn decode_positive_infinity() {
        let resp_str = ",inf\r\n";
        let resp = Resp::decode(resp_str).unwrap();
        assert_eq!(resp, Resp::Double(f64::INFINITY));
    }

    #[test]
    fn decode_negative_infinity() {
        let resp_str = ",-inf\r\n";
        let resp = Resp::decode(resp_str).unwrap();
        assert_eq!(resp, Resp::Double(f64::NEG_INFINITY));
    }
}
