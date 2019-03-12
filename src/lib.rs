use std::net::SocketAddr;
use std::net::TcpStream;
use std::io::{ BufRead, BufReader };
use std::convert::From;

pub mod resp;

use resp::*;

struct RedisClient {
    connection: TcpStream,
    host: SocketAddr,
    commands_handled: u32
}

enum ClientError {
    IO(std::io::Error),
    UnknownError
}

impl From<std::io::Error> for ClientError {
    fn from(io: std::io::Error) -> ClientError {
        ClientError::IO(io)
    }
}

type ClientResult<T> = Result<T, ClientError>;

enum RedisData {
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

/// Takes in a slice and returns owned data.
/// Will probably need to do some copying
fn parse_redis_data(buf: &[u8]) -> RedisData {
    RedisData()
}

fn await_response(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut buf_reader = BufReader::new(stream);
    // let mut buf: [u8; 128] = [0u8; 128];
    let mut response: Vec<u8> = Vec::new();
    buf_reader.read_line(&mut response)?;
    Ok(response)
}

impl RedisClient {
    fn ping(&mut self) -> ClientResult<()> {
        self.commands_handled = self.commands_handled + 1;
        send(&mut self.connection, Command::Ping)?;
        let response = await_response(&mut self.connection)?;


    }
    fn set(&mut self, key: &str, value: &str) -> ClientResult<()> {
        self.commands_handled = self.commands_handled + 1;
        match send(&mut self.connection, Command::Set(key.to_string(), value.to_string())) {
            Ok(_) => Ok(()),
            Err(ioerr) => Err(ClientError::IO(ioerr)),
        }
    }
    /// Returns a rust string, not sure if this is strictly correct, we should
    /// probably not require valid UTF-8
    fn get(&mut self, key: &str) -> ClientResult<String> {
        self.commands_handled = self.commands_handled + 1;
        match send(&mut self.connection, Command::Get(key.to_string())) {
            Ok(_) => ()
        }
        
    }
}
