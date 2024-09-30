use std::io::Error;
use tokio::{io::AsyncReadExt, net::TcpStream};
use crate::html::Status;

pub(crate) async fn read_line(stream: &mut TcpStream) -> Result<String, Error> {
    let mut str: String = String::new();
    loop {
        let byte = stream.read_u8().await?;

        match byte {
            b'\r' => {
                stream.read_u8().await?;
                break;
            }
            _ => str.push(byte as char),
        }
    }
    return Ok(str);
}

pub fn format_response(status: Status) -> Vec<u8> {
    format!("HTTP/1.1 {}\r\nContent-Length: 0\r\n\r\n", status).into_bytes()
}

pub fn format_response_with_body(status: Status, body: impl Into<Vec<u8>>) -> Vec<u8> {
    let body = body.into();
    let mut response = format!(
        "HTTP/1.1 {}\r\nContent-Length: {}\r\n\r\n",
        status,
        body.len()
    ).into_bytes();
    response.extend(body);

    response
}