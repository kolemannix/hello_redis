use hello_redis::*;

fn main() -> Result<()> {
    // let mut redis = RedisClient::connect("127.0.0.1:6379");
    let mut redis = RedisClient::connect("127.0.0.1:6379")?;
    println!("Sending PING...");
    let pong = redis.ping()?;
    assert_eq!((), pong);
    println!("Got PONG!");
    println!("Sending SET...");
    let ok = redis.set("abc", "123")?;
    println!("Sending GET...");
    let get = redis.get("abc")?;
    assert_eq!(get, RedisData::BulkString("123".as_bytes().to_vec()));
    assert_eq!((), ok);
    println!("Result matched!");
    Ok(())

}
