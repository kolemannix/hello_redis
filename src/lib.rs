use std::string::FromUtf8Error;
use std::str::Utf8Error;
use std::convert::TryFrom;
use std::io::Write;
use std::net::ToSocketAddrs;
use std::borrow::Cow;
use std::io::Read;
use std::net::TcpStream;
use std::borrow::Cow::{Borrowed};
use std::str;
use std::io::{ BufRead, BufReader };

pub struct RedisClient {
    pub connection: TcpStream,
    pub commands_handled: u32
}

impl RedisClient {
    pub fn connect<A : ToSocketAddrs>(addr: A) -> Result<RedisClient> {
        let tcp = std::net::TcpStream::connect(addr)?;
        let client = RedisClient {
            connection: tcp,
            commands_handled: 0
        };
        Ok(client)
    }

    fn respond<T>(&mut self, response: Result<T>) -> Result<T> {
        self.commands_handled = self.commands_handled + 1;
        response
    }

    pub fn ping(&mut self) -> Result<()> {
        self.commands_handled = self.commands_handled + 1;
        send(&mut self.connection, Command::Ping)?;
        let response = await_response_simplestr(&mut self.connection)?;
        let response = if response == "PONG" {
            Ok(())
        } else {
            Err(Error::UnknownError("Response to PING was not PONG".into()))
        };
        self.respond(response)
    }
    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        send(&mut self.connection, Command::Set(key.to_string(), value.to_string()))?; 
        let response = await_response_simplestr(&mut self.connection)?;
        // FIXME: Trim is required
        let response = if response.trim() == "OK" {
            Ok(())
        } else {
            Err(Error::UnknownError("Response to SET was not OK".into()))
        };
        self.respond(response)
    }
    /// Returns a rust string, not sure if this is strictly correct, we should
    /// probably not require valid UTF-8
    pub fn get(&mut self, key: &str) -> Result<RedisData> {
        send(&mut self.connection, Command::Get(key.to_string()))?;        
        let response = await_response_bulkstr(&mut self.connection)?;
        let response = Ok(RedisData::BulkString(response));
        self.respond(response)
    }
}

fn check_error(actual: &[u8], expected: u8) -> Result<()> {
    match actual.get(0) {
        Some(head) => {
            if *head == ERROR as u8 {
                let error_str = String::from_utf8(actual[1..].to_vec())?;
                Err(Error::ServerError(error_str.into()))
            } else {
                if *head == expected {
                    Ok(())
                } else {
                    let error_message = format!("Head was {}, not expected {}", *head as char, expected as char);
                    Err(Error::UnknownError(error_message.into()))
                }
            }
        },
        None => Err(Error::UnknownError("Tried checking prefix but buffer is empty".into()))
    }
}

/// The BulkString type deals with pure binary data (Non UTF-8)
fn await_response_bulkstr(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let mut buf_reader = BufReader::new(stream);
    let mut len_buf: Vec<u8> = Vec::new();
    buf_reader.read_until(LF, &mut len_buf)?;
    check_error(&len_buf, BULK_STRING as u8)?;
    // Pop CR
    len_buf.pop();
    // Pop LF
    len_buf.pop();
    let as_str = str::from_utf8(&len_buf[1..])?;
    let size: usize = as_str.parse()
        .map_err(|_| Error::BadBulkString("Length header was not a number".into()))?;
    println!("Parsed length to be {}", size);

    let mut content_buf: Vec<u8> = Vec::with_capacity(size + 1);
    buf_reader.read_until(CR, &mut content_buf)?;
    content_buf.pop();
    Ok(content_buf)
}

/// The SimpleString type deals with data that we can assume is valid UTF-8
fn await_response_simplestr(stream: &mut TcpStream) -> Result<String> {
    let mut content_buf = Vec::new();
    let mut buf_reader = BufReader::new(stream);
    buf_reader.read_until(CR, &mut content_buf)?;


    check_error(&content_buf, SIMPLE_STRING as u8)?;
    content_buf.pop();
    content_buf.remove(0);
    let as_string = String::from_utf8(content_buf)?;
    Ok(as_string)
}
/// This type is just a CRLF terminated string representing an integer, prefixed by a ":" byte.
/// For example ":0\r\n", or ":1000\r\n" are integer replies.
/// Many Redis commands return RESP Integers, like INCR, LLEN and LASTSAVE.
/// There is no special meaning for the returned integer, it is just an incremental number for INCR,
/// a UNIX time for LASTSAVE and so forth. However, the returned integer is guaranteed to be in
/// the range of a signed 64 bit integer.
fn await_response_integer(stream: &mut TcpStream) -> Result<i64> {
    let mut content_buf = Vec::new();
    let mut buf_reader = BufReader::new(stream);
    buf_reader.read_until(CR, &mut content_buf)?;
    check_error(&content_buf, INTEGER as u8)?;
    content_buf.remove(0);
    content_buf.pop();
    let as_str = str::from_utf8(&content_buf)?;
    let result: i64 = as_str.parse()?;
    Ok(result)
}

pub const SIMPLE_STRING: char = '+';
pub const ERROR: char = '-';
pub const INTEGER: char = ':';
pub const BULK_STRING: char = '$';
pub const ARRAY: char = '*';
pub const CR: u8 = 0x0d;
pub const LF: u8 = 0x0a;
pub const CRLF: [u8; 2] = [0x0d, 0x0a];

pub struct RedisKey {
    value: Vec<u8>
}
impl From<&str> for RedisKey {
    fn from(s: &str) -> RedisKey {
        RedisKey { value: s.into() }
    }
}

impl From<Vec<u8>> for RedisKey {
    fn from(v: Vec<u8>) -> RedisKey {
        RedisKey { value: v }
    }
}
pub enum Command {
    Ping,
    Set(String, String),
    Get(String)
}

use Command::{Ping, Set, Get};

#[derive(Debug)]
pub enum Error {
    IOError(std::io::Error),
    InvalidUtf8(Utf8Error),
    InvalidInt(std::num::ParseIntError),
    InvalidPrefix,
    BadBulkString(Cow<'static, str>),
    BadSimpleString(Cow<'static, str>),
    BadInteger(Cow<'static, str>),
    ServerError(Cow<'static, str>),
    UnknownError(Cow<'static, str>)
}

impl From<FromUtf8Error> for Error {
    fn from(e: FromUtf8Error) -> Error { Error::InvalidUtf8(e.utf8_error()) }
}

impl From<Utf8Error> for Error {
    fn from(e: Utf8Error) -> Error { Error::InvalidUtf8(e) }
}

impl From<std::num::ParseIntError> for Error {
    fn from(e: std::num::ParseIntError) -> Error { Error::InvalidInt(e) }
}

impl From<std::io::Error> for Error {
    fn from(io: std::io::Error) -> Error { Error::IOError(io) }
}
impl From<()> for Error {
    fn from(_: ()) -> Error { Error::UnknownError("Unit error".into()) }
}

pub type Result<T> = std::result::Result<T, Error>;

// #[repr(u8)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RespPrefix {
    SimpleString,
    Error,
    Integer,
    BulkString, 
    Array, 
}

use RespPrefix::*;

impl RespPrefix {
    pub fn byte_repr(&self) -> u8 {
        let c = self.char_repr();
        c as u8
    }
    pub fn char_repr(&self) -> char {
        match self {
            SimpleString => '+',
            Error => '-',
            Integer => ':',
            BulkString => '$', 
            Array => '*',
        }
    }
}

impl TryFrom<char> for RespPrefix {
    type Error = Error;
    fn try_from(c: char) -> Result<Self> {
        RespPrefix::try_from(c as u8)
    }
}

impl TryFrom<u8> for RespPrefix {
    type Error = Error;
    fn try_from(byte: u8) -> Result<Self> {
        match byte {
            b'+' => Ok(RespPrefix::SimpleString),
            b'-' => Ok(RespPrefix::Error),
            b':' => Ok(RespPrefix::Integer),
            b'$' => Ok(RespPrefix::BulkString),
            b'*' => Ok(RespPrefix::Array),
            _ => Err(Error::InvalidPrefix)
        }
    }
}

pub fn send<T : Write>(writer: &mut T, command: Command) -> std::io::Result<()> {
    match command {
        Ping => {
            writer.write(b"*1\r\n$4\r\nPING\r\n")?;
        },
        Set(key, value) => {
            resp_array(writer, 3)?;
            resp_bulk_string(writer, b"SET")?;
            resp_bulk_string(writer, key.as_bytes())?;
            resp_bulk_string(writer, value.as_bytes())?;
        },
        Get(key) => {
            resp_array(writer, 2)?;
            resp_bulk_string(writer, b"GET")?;
            resp_bulk_string(writer, key.as_bytes())?;
        },
    };
    writer.flush()
}

pub fn resp_bulk_string<T : Write>(writer: &mut T, binary_data: &[u8]) -> std::io::Result<()> {
    writer.write(&[RespPrefix::BulkString.byte_repr()])?;
    write!(writer, "{}", binary_data.len())?;
    writer.write(&CRLF)?;
    writer.write(binary_data)?;
    writer.write(&CRLF)?;
    Ok(())
}

pub fn resp_array<T : Write>(writer: &mut T, count: u32) -> std::io::Result<()> {
    let prefix: char = Array.char_repr();
    // let mut test_buffer = String::new();
    // use std::fmt::Write;
    // write!(&mut test_buffer, "{}", prefix).unwrap();
    // write!(&mut test_buffer, "{}\r\n", count).unwrap();
    // println!("Going to write: {}", test_buffer);
    
    // Q: I want to get these bytes without allocating a String, is it possible?
    // A: Yes! https://stackoverflow.com/questions/55151575
    write!(writer, "{}", prefix)?;
    write!(writer, "{}\r\n", count)?;
    Ok(())
}

fn write_number<T : Write>(writer: &mut T, num: u32) -> std::io::Result<()> {
    write!(writer, "{}", num)
    // let num_as_str: String = num.to_string();
    // let num_as_bytes = num_as_str.as_bytes();
    // writer.write(num_as_bytes)
}

#[cfg(test)]
mod tests {

    use super::*;
    
    #[test]
    fn set_test() -> () {
        let mut writer: Vec<u8> = Vec::new();
        send(&mut writer, Set("abc".to_string(), "123".to_string())).unwrap();
        let result_string = String::from_utf8(writer).unwrap();
        assert_eq!(result_string, "*3\r\n$3\r\nSET\r\n$3\r\nabc\r\n$3\r\n123\r\n");
        println!("{}", result_string);
    }
    
}


// Receiving


#[derive(Debug, PartialEq)]
pub enum RedisData {
    /// Redis 'SimpleString's to Rust's String type.
    /// These cannot contain CRLFs
    SimpleString(String),
    /// Like 'SimpleString', but signifies an error case
    Error(String),
    /// FIXME: Not sure about the size of this
    RedisInteger(i128), 
    /// Arbitrary binary data with a known length
    BulkString(Vec<u8>),
    /// An array of any of the above types
    Array(Vec<RedisData>)
}

// Takes in a slice and returns owned data.
// Will probably need to do some copying
// fn parse_redis_data(buf: &[u8]) -> Result<RedisData> {
//     let prefix = RespPrefix::try_from(buf[0])?;
//     match prefix {
//         RespPrefix::SimpleString => {
//             let ss = buf.iter().take_while(|x| x != CR);
//         }
//     }

//     Err(Error::UnknownError)
// }

