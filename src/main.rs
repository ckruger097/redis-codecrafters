mod resp;

use std::io::{BufReader, Cursor};
use std::net::{TcpStream, TcpListener};
use std::io::prelude::*;
use std::thread;
use crate::resp::{decode_value, interpret_redis_command};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").expect("Unable to bind to address");

    for stream_result in listener.incoming() {
        match stream_result {
            Ok(mut stream) => {
                println!("accepted new connection");
                thread::spawn(move || {
                    handle_connection(&mut stream);
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_connection(stream: &mut TcpStream) {
    loop {
        let mut buffer = [0; 1024];
        match stream.read(&mut buffer) {
            Ok(_size) => {
                let cursor = Cursor::new(buffer);
                let mut reader = BufReader::new(cursor);
                let redis_value_result = decode_value(&mut reader);
                match redis_value_result {
                    Ok(value) => {
                        let response_result = interpret_redis_command(&value);
                        match response_result {
                            Ok(response) => {
                                stream.write(response.encode().as_bytes()).expect("Can't write to stream");
                                stream.flush().expect("Can't flush stream");
                            }
                            Err(_) => ()
                        }
                    }
                    Err(_) => ()
                }
            }
            Err(e) => {
                eprintln!("Error occurred: {}", e);
                break;
            }
        }
    }
}