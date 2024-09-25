use std::{collections::HashMap, io::Error, path::PathBuf};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}};

#[derive(Debug)]
struct Request {
    stream: TcpStream,

    method: String,
    path: PathBuf,
    version: String,

    headers: HashMap<String, String>,

    body: Vec<u8>,
}

#[tokio::main]
async fn main() {
    let yeet = TcpListener::bind("0.0.0.0:80").await.unwrap();

    loop {
        let client = yeet.accept().await;

        let request_handle = tokio::spawn(async move {
            let (mut stream, _) = client.unwrap();

            //println!("Connection from {}", addr);

            let inital_line = read_line(&mut stream).await.unwrap();
            let (method, right) = inital_line.split_once(" ").unwrap();
            let (path, version) = right.rsplit_once(" ").unwrap();
            //println!("{} {} {}", method, path, version);

            let mut headers = HashMap::new();
            loop {
                let current_header = read_line(&mut stream).await.unwrap();

                if current_header.len() == 0 {
                    break;
                }

                let (k, v) = current_header.split_once(": ").unwrap();
                headers.insert(k.to_string(), v.to_string());
            }
            //println!("{:#?}", headers);

            let mut body = Vec::new();
            if headers.contains_key("Content-Length") {
                let content_length = headers.get("Content-Length").unwrap().parse::<usize>().unwrap();
                let mut buffer = vec![0; content_length];
                stream.read_exact(&mut buffer).await.unwrap();
                body = buffer;
            }

            Request {
                stream,
                method: method.to_string(),
                path: PathBuf::from(path),
                version: version.to_string(),
                headers,
                body,
            }
        });

        let mut request = request_handle.await.unwrap();
        
        let mut response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", request.body.len()).as_bytes().to_vec();
        response.extend(&request.body);
        request.stream.write_all(&response).await.unwrap();
    }
}

async fn read_line(stream: &mut TcpStream) -> Result<String, Error> {
    let mut str: String = String::new();
    loop {
        let byte = stream.read_u8().await?;

        match byte{
            b'\r' => {
                stream.read_u8().await?;
                break;
            },
            _ => str.push(byte as char),
        }
    }
    return Ok(str);
}
