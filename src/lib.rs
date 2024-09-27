use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
};

pub struct WebServer {
    listener: TcpListener,
    on_request: Arc<Mutex<dyn Fn(Request) + Send + Sync>>,
}

impl WebServer {
    pub async fn new<F>(host: &str, port: u16, on_request: F) -> WebServer
    where
        F: Fn(Request) + Send + Sync + 'static,
    {
        let listener = TcpListener::bind(format!("{}:{}", host, port))
            .await
            .unwrap();

        WebServer {
            listener,
            on_request: Arc::new(Mutex::new(on_request)),
        }
    }

    pub async fn run(&self) {
        loop {
            let client = self.listener.accept().await;

            let request_handle = tokio::spawn(async move {
                let (stream, _addr) = client.unwrap();

                handle_client(stream)
            });

            match request_handle.await.unwrap().await {
                Ok(request) => {
                    let on_request = self.on_request.lock().unwrap();
                    on_request(request);
                }
                Err(e) => {
                    println!("Error: {:?}", e);
                }
            }
        }
    }
}

async fn handle_client(mut stream: TcpStream) -> Result<Request, Error> {
    let inital_line = read_line(&mut stream).await?;
    let (method, right) = inital_line
        .split_once(" ")
        .ok_or(Error::new(ErrorKind::InvalidData, "Could not parse method"))?;
    let (path, version) = right.rsplit_once(" ").ok_or(Error::new(
        ErrorKind::InvalidData,
        "Could not parse path and version",
    ))?;
    //println!("{} {} {}", method, path, version);

    let mut headers = HashMap::new();
    loop {
        let current_header = read_line(&mut stream).await?;

        if current_header.len() == 0 {
            break;
        }

        let (k, v) = current_header.split_once(": ").ok_or(Error::new(
            ErrorKind::InvalidData,
            "Could not parse headers",
        ))?;
        headers.insert(k.to_string(), v.to_string());
    }
    //println!("{:#?}", headers);

    let mut body = Vec::new();
    if headers.contains_key("Content-Length") {
        let content_length = headers
            .get("Content-Length")
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let mut buffer = vec![0; content_length];
        stream.read_exact(&mut buffer).await?;
        body = buffer;
    }

    Ok(Request {
        stream,
        method: method.to_string(),
        path: PathBuf::from(path),
        version: version.to_string(),
        headers,
        body,
    })
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Request {
    pub stream: TcpStream,

    pub method: String,
    pub path: PathBuf,
    pub version: String,

    pub headers: HashMap<String, String>,

    pub body: Vec<u8>,
}

async fn read_line(stream: &mut TcpStream) -> Result<String, Error> {
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
    format_response("405 Method Not Allowed");
    return Ok(str);
}

pub fn get_real_ip<'a>(request: &Request, headers: Option<Vec<&'a str>>) -> String {
    if let Some(headers) = headers {
        for key in headers {
            if let Some(real_ip) = request.headers.get(key) {
                return real_ip.to_string();
            }
        }
    }
    if let Some(real_ip) = request.headers.get("X-Real-IP")  {
        return real_ip.to_string();
    }
    request.stream.peer_addr().unwrap().ip().to_string()
}

pub fn format_response(status: impl Into<String>) -> Vec<u8> {
    format!("HTTP/1.1 {}\r\nContent-Length: 0\r\n\r\n", status.into()).into_bytes()
}

pub fn format_response_with_body(status: impl Into<String>, body: Vec<u8>) -> Vec<u8> {
    let mut response = format!(
        "HTTP/1.1 {}\r\nContent-Length: {}\r\n\r\n",
        status.into(),
        body.len()
    ).into_bytes();
    response.extend(body);

    response
}
