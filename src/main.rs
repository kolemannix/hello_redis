
use std::net::{ TcpStream, TcpListener, ToSocketAddrs };
use std::io::{BufReader};
use std::convert::TryFrom;
use hello_redis::resp::*;

fn send_ping(stream: &mut TcpStream) -> std::io::Result<String> {
    send(stream, Command::Ping)?;
    // stream.flush()?;
    await_response(stream)
}



fn send_command(stream: &mut TcpStream, command: Command) -> std::io::Result<String> {
    // let setabcfoo = b"*3\r\n$3\r\nSET\r\n$3\r\nabc\r\n$3\r\nfoo\r\n";
    send(stream, command)?;
    println!("Sent set");
    await_response(stream)
}

fn parse_response(response: &String) -> () {
    if response.is_empty() {
        return ();
    }
    let first = response.chars().nth(0).unwrap();
    match RespPrefix::try_from(first) {
        Ok(prefix) => println!("Parsed {:?}", prefix),
        Err(_) => println!("Unknown prefix {}", first),
    }
}

fn main() -> Result<(), std::io::Error> {
    let mut tcp = TcpStream::connect("127.0.0.1:6379").expect("Can't connect");
    // send_ping(&mut tcp)?;
    let set_abc123 = Command::Set("abc".to_string(), "123".to_string());
    let response = send_command(&mut tcp, set_abc123)?;
    println!("Set response: {:?}", response);
    Ok(())
}
