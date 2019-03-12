use std::convert::{TryFrom};
use std::io::prelude::*;
use std::str;

/// Implementation of the REdiS Protocol (RESP)

const CR: u8 = 0x0d;
const LF: u8 = 0x0a;
const CRLF: [u8; 2] = [0x0d, 0x0a];

pub enum Command {
    Ping,
    Set(String, String),
    Get(String)
}

use Command::{Ping, Set, Get};

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
    fn byte_repr(&self) -> u8 {
        let c = match self {
            SimpleString => '+',
            Error => '-',
            Integer => ':',
            BulkString => '$', 
            Array => '*',
        };
        c as u8
    }
}

impl TryFrom<char> for RespPrefix {
    type Error = ();
    fn try_from(c: char) -> Result<Self, ()> {
        RespPrefix::try_from(c as u8)
    }
}

impl TryFrom<u8> for RespPrefix {
    type Error = ();
    fn try_from(byte: u8) -> Result<Self, ()> {
        match byte {
            b'+' => Ok(RespPrefix::SimpleString),
            b'-' => Ok(RespPrefix::Error),
            b':' => Ok(RespPrefix::Integer),
            b'$' => Ok(RespPrefix::BulkString),
            b'*' => Ok(RespPrefix::Array),
            _ => Err(())
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
            resp_bulk_string(writer, "SET")?;
            resp_bulk_string(writer, &key)?;
            resp_bulk_string(writer, &value)?;
        },
        Get(key) => {
            resp_array(writer, 2)?;
            resp_bulk_string(writer, "GET")?;
            resp_bulk_string(writer, &key)?;
        },
    };
    writer.flush()
}

pub fn resp_bulk_string<T : Write>(writer: &mut T, value: &str) -> std::io::Result<()> {
    writer.write(&[RespPrefix::BulkString.byte_repr()])?;
    let count_bytes = value.len().to_string().into_bytes();
    writer.write(&count_bytes)?;
    writer.write(&CRLF)?;
    writer.write(value.as_bytes())?;
    writer.write(&CRLF)?;
    Ok(())
}

pub fn resp_array<T : Write>(writer: &mut T, count: u32) -> std::io::Result<()> {
    let prefix: u8 = Array.byte_repr();
    // FIXME: I want to get these bytes without allocating a String, is it possible?
    let count_str = count.to_string();
    let count_bytes = count_str.as_bytes();
    
    writer.write(&[prefix])?;
    writer.write(count_bytes)?;
    writer.write(&CRLF)?;
    Ok(())
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
