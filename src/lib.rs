use std::io::Read;
use std::net::SocketAddr;
use std::net::TcpStream;
use std::str;
use std::io::{ BufRead, BufReader };

pub mod resp;

use resp::*;

pub struct RedisClient {
    pub connection: TcpStream,
    pub commands_handled: u32
}

fn head_equals(v: &[u8], b: u8) -> bool {
    match v.get(0) {
        Some(&head) if head == b => true,
        _ => false,
    }
}

/// The BulkString type deals with pure binary data (Non UTF-8)
fn await_response_bulkstr(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let mut buf_reader = BufReader::new(stream);
    let mut len_buf: Vec<u8> = Vec::new();
    buf_reader.read_until(CR, &mut len_buf)?;
    if len_buf.is_empty() {
        return Err(Error::BadBulkString);
    }
    let (prefix, size_bytes) = len_buf.split_at(1);

    if !head_equals(prefix, RespPrefix::BulkString.byte_repr()) {
        return Err(Error::BadBulkString);
    }
    let head = prefix.get(0).map_or(Err(Error::BadBulkString), |h| Ok(h))?;
    if *head != RespPrefix::BulkString.byte_repr() {
        return Err(Error::BadBulkString);
    }
    let as_str = str::from_utf8(size_bytes).map_err(|_| Error::BadBulkString)?;
    let size: usize = as_str.parse().map_err(|_| Error::BadBulkString)?;
    let mut content_buf: Vec<u8> = Vec::with_capacity(size);
    buf_reader.read_exact(&mut content_buf)?;
    // len_buf.iter().
    // let len = u64::from len_buf[1..];
    Ok(content_buf)
}

/// The SimpleString type deals with data that we can assume is valid UTF-8
fn await_response_simplestr(stream: &mut TcpStream) -> Result<String> {
    let mut content_buf = String::new();
    let mut buf_reader = BufReader::new(stream);
    buf_reader.read_line(&mut content_buf)?;
    let (prefix, content) = content_buf.split_at(1);
    let head = prefix.chars().next().map_or(Err(Error::BadSimpleString), |h| Ok(h))?;
    if head != RespPrefix::SimpleString.char_repr() {
        return Err(Error::BadSimpleString);
    }
    Ok(content.to_string())
}
/// This type is just a CRLF terminated string representing an integer, prefixed by a ":" byte.
/// For example ":0\r\n", or ":1000\r\n" are integer replies.
/// Many Redis commands return RESP Integers, like INCR, LLEN and LASTSAVE.
/// There is no special meaning for the returned integer, it is just an incremental number for INCR,
/// a UNIX time for LASTSAVE and so forth. However, the returned integer is guaranteed to be in
/// the range of a signed 64 bit integer.
fn await_response_integer(stream: &mut TcpStream) -> Result<i64> {
    let mut _content_buf: Vec<u8> = Vec::new();
    let mut _buf_reader = BufReader::new(stream);
    Err(Error::UnknownError)
}

impl RedisClient {
    pub fn ping(&mut self) -> Result<()> {
        self.commands_handled = self.commands_handled + 1;
        send(&mut self.connection, Command::Ping)?;
        let response = await_response_simplestr(&mut self.connection)?;
        println!("Response from PING: {}", response);
        // FIXME: Trim is required
        if response.trim() == "PONG" {
            Ok(())
        } else {
            Err(Error::UnknownError)
        }
    }
    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        self.commands_handled = self.commands_handled + 1;
        send(&mut self.connection, Command::Set(key.to_string(), value.to_string()))?; 
        let response = await_response_simplestr(&mut self.connection)?;
        // FIXME: Trim is required
        if response.trim() == "OK" {
            Ok(())
        } else {
            Err(Error::UnknownError)
        }
    }
    /// Returns a rust string, not sure if this is strictly correct, we should
    /// probably not require valid UTF-8
    pub fn get(&mut self, key: &str) -> Result<RedisData> {
        self.commands_handled = self.commands_handled + 1;
        send(&mut self.connection, Command::Get(key.to_string()))?;        
        let response = await_response_bulkstr(&mut self.connection)?;
        Ok(RedisData::BulkString(response))
    }
}
