
use std::net::{ TcpStream, TcpListener, ToSocketAddrs };
use std::io::{BufReader};
use std::convert::TryFrom;
use hello_redis::*;
use hello_redis::resp::*;

fn main() -> Result<()> {
    // let mut redis = RedisClient::connect("127.0.0.1:6379");
    let mut tcp = TcpStream::connect("127.0.0.1:6379").expect("Can't connect");
    let mut redis = RedisClient {
        connection: tcp,
        commands_handled: 0
    };
    // send_ping(&mut tcp)?;
    let _pong = redis.ping()?;
    let _ok = redis.set("abc", "123")?;
    Ok(())
}
