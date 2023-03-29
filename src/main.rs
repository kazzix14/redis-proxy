use std::{
    env,
    fs::OpenOptions,
    io::{self, BufReader, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc::{channel, SendError},
    thread, //time,
};

use resp::{encode, Decoder, Value};

fn main() {
    println!("Hello, world!");
    let listener = TcpListener::bind(format!("0.0.0.0:{}", env::args().nth(1).unwrap())).unwrap();
    for streams in listener.incoming() {
        match streams {
            Err(e) => {
                eprintln!("error: {}", e)
            }
            Ok(stream) => {
                thread::spawn(move || {
                    handler(stream).unwrap_or_else(|error| eprintln!("{:?}", error));
                });
            }
        }
    }
}

fn handler(client_stream: TcpStream) -> Result<(), io::Error> {
    //println!("Connection from {}", client_stream.peer_addr()?);
    let mut file = OpenOptions::new().append(true).open("inspect.txt").unwrap();
    file.write_all(
        format!(
            "got new connection<{}>\n",
            client_stream.peer_addr().unwrap()
        )
        .as_ref(),
    );

    let redis_addr = "redis:6379";
    let rx_client_stream = client_stream;
    let mut tx_client_stream = rx_client_stream.try_clone().unwrap();
    let mut tx_redis_stream = TcpStream::connect(&redis_addr).unwrap();
    let rx_redis_stream = tx_redis_stream.try_clone().unwrap();

    let (to_client_tx, to_client_rx) = channel();
    let (to_redis_tx, to_redis_rx) = channel();

    // tcp rx client
    thread::spawn(move || {
        let addr = rx_client_stream.peer_addr().unwrap();
        let mut decoder = Decoder::new(BufReader::new(rx_client_stream));
        loop {
            //if let Ok(value) = decoder.decode().map_err(|_e| println!("closed: {:}", addr)) {
            if let Ok(value) = decoder.decode() {
                //println!("received from client: {:?}", value);
                let mut file = OpenOptions::new().append(true).open("inspect.txt").unwrap();
                file.write_all(format!("got from client<{}>: {:?}\n", addr, value).as_ref());
                to_redis_tx.send(value).unwrap();
            } else {
            }
        }
    });

    // tcp tx client
    thread::spawn(move || loop {
        if let Ok(value) = to_client_rx.recv() {
            /*
            let unsub_reply = Value::Array(
                [
                    Value::Bulk(String::from("unsubscribe")),
                    Value::Bulk(String::from("my-channel")),
                    Value::Integer(0),
                ]
                .to_vec(),
            );
            if value == unsub_reply {
                //println!("unsub reply");
                let neko_message = Value::Array(
                    [
                        Value::Bulk(String::from("message")),
                        Value::Bulk(String::from("my-channel")),
                        Value::Bulk(String::from("neko")),
                    ]
                    .to_vec(),
                );
                for _ in 0..3 {
                    tx_client_stream.write(&encode(&neko_message)).unwrap();
                    println!("sent to client: {:?}", neko_message);
                }
                tx_client_stream.flush().unwrap();
            }
            */

            //thread::sleep(time::Duration::from_millis(300));
            if let Ok(addr) = tx_client_stream.peer_addr() {
                let mut file = OpenOptions::new().append(true).open("inspect.txt").unwrap();
                file.write_all(format!("sent to client<{}>: {:?}\n", addr, value).as_ref());
                tx_client_stream.write(&encode(&value)).unwrap();
                tx_client_stream.flush().unwrap();
            }
        }
    });

    // tcp rx redis
    thread::spawn(move || {
        let addr = rx_redis_stream.peer_addr().unwrap();
        let mut decoder = Decoder::new(BufReader::new(rx_redis_stream));
        loop {
            //if let Ok(value) = decoder.decode() {
            if let Ok(value) = decoder.decode() {
                //println!("received from redis: {:?}", value);
                to_client_tx
                    .send(value)
                    .or_else::<SendError<Value>, _>(|e| {
                        let mut file = OpenOptions::new().append(true).open("inspect.txt").unwrap();
                        file.write_all(format!("{}\n", e).as_ref());
                        Ok(())
                    })
                    .unwrap();
            }
        }
    });

    // tcp tx redis
    thread::spawn(move || loop {
        if let Ok(value) = to_redis_rx.recv() {
            //thread::sleep(time::Duration::from_millis(300));
            tx_redis_stream.write(&encode(&value)).unwrap();
            tx_redis_stream.flush().unwrap();
        }
    });

    Ok(())
}
