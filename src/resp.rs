use std::io::{BufRead};
use anyhow::{Result, anyhow};
#[derive(Debug, Clone)]
pub(crate) enum RespValue {
    SimpleString(String),
    Error(String),
    Integer(i64),
    BulkString(Vec<u8>),
    Array(Vec<RespValue>),
}

#[derive(Debug)]
pub(crate) enum RedisCommand {
    PING,
    ECHO(String),
    UNKNOWN,
}

impl RedisCommand {
    pub(crate) fn encode(&self) -> String {
        match self {
            RedisCommand::PING => {
                "+PONG\r\n".to_string()
            }
            RedisCommand::ECHO(string) => {
                let str_len = string.len();
                format!("${}\r\n{}\r\n", str_len, string)
            }
            RedisCommand::UNKNOWN => {
                "-ERROR_UNKNOWN_COMMAND\r\n".to_string()
            }
        }
    }
}

pub (crate) fn decode_value(reader: &mut impl BufRead) -> Result<RespValue> {
    let mut type_byte = [0u8];
    reader.read_exact(&mut type_byte)?;

    match type_byte[0] {
        b'+' => {
            let mut string = String::new();
            reader.read_line(&mut string)?;

            Ok(RespValue::SimpleString(string.trim_end().to_string()))
        }
        b'-' => {
            let mut string = String::new();
            reader.read_line(&mut string)?;

            Ok(RespValue::Error(string.trim_end().to_string()))
        }
        b':' => {
            let mut integer_vec = Vec::new();
            reader.read_until(b'\n', &mut integer_vec)?;
            let integer = String::from_utf8_lossy(&integer_vec[0..integer_vec.len() - 2])
                .parse::<i64>()?;

            Ok(RespValue::Integer(integer))
        }
        b'$' => {
            let mut length_vec = Vec::new();
            reader.read_until(b'\n', &mut length_vec)?;
            let length = String::from_utf8_lossy(&length_vec[0..length_vec.len() - 2])
                .parse::<usize>()?;

            let mut bulk_string = vec![0; length];
            reader.read_exact(&mut bulk_string)?;
            reader.read_exact(&mut [0u8; 2])?;

            Ok(RespValue::BulkString(bulk_string))
        }
        b'*' => {
            let mut array_len_vec = Vec::new();
            reader.read_until(b'\n', &mut array_len_vec)?;
            let array_len = String::from_utf8_lossy(&array_len_vec[0..array_len_vec.len() - 2])
                .parse::<usize>()?;

            let mut array = Vec::with_capacity(array_len);
            for _ in 0..array_len {
                array.push(decode_value(reader)?);
            }

            Ok(RespValue::Array(array))
        }
        _ => {
            let bad_byte = type_byte[0] as char;
            let mut dump = String::new();
            reader.read_line(&mut dump).expect("PANIC: Can't dump buffer");
            Err(anyhow!("Invalid RESP type byte: {}\nBuffer: {}", bad_byte, dump))
            }
    }
}

pub(crate) fn interpret_redis_command(value: &RespValue) -> Result<RedisCommand> {
    match value {

        RespValue::Array(array) => {
            match process_redis_array(array) {
                Ok(redis_command) => Ok(redis_command),
                Err(error) => Err(anyhow!("Error when interpreting command: {}", error))
            }
        }

        RespValue::Error(error_string) => {
            eprintln!("Got an error during interpretation of redis command: {}", error_string);
            Ok(RedisCommand::UNKNOWN)
        }

        _ => Err(anyhow!("Couldn't interpret redis command: {:?}", value))
    }
}

fn process_redis_array(array: &Vec<RespValue>) -> Result<RedisCommand> {
    if let RespValue::BulkString(command_bytes) = array[0].clone() {
        let command = String::from_utf8_lossy(&command_bytes);
        match command.as_ref().to_lowercase().as_str() {
            "ping" => {
                Ok(RedisCommand::PING)
            }
            "echo" => {
                if array.len() < 2 {
                    Err(anyhow!("ECHO with no string"))
                } else if array.len() > 2 {
                    Err(anyhow!("ECHO with more than 1 string, must implement"))
                } else {
                    if let RespValue::BulkString(echo_string_bytes) = array[1].clone() {
                        let echo_string = String::from_utf8_lossy(&echo_string_bytes);
                        Ok(RedisCommand::ECHO(echo_string.to_string()))
                    } else {
                        Err(anyhow!("Array element wasn't a bulkstring"))
                    }
                }
            }
            _ => Err(anyhow!("Unknown command type!"))
        }
    } else {
        Err(anyhow!("Array did not contain bulk string"))
    }
}