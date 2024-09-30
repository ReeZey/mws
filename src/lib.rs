pub mod html;
pub mod utils;

use std::{
    collections::HashMap, future::Future, io::{Error, ErrorKind}, path::PathBuf, time::Duration
};
use html::Status;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use utils::{format_response, read_line};

pub struct WebServer {
    verbose: bool,
}

impl WebServer {
    pub fn new(verbose: bool) -> Self {
        WebServer {
            verbose,
        }
    }

    pub async fn listen<F, Fut>(&self, host: &str, port: u16, on_request: F)
    where
        F: Fn(Request) -> Fut + Send + Sync + Copy + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let listener = TcpListener::bind(format!("{}:{}", host, port))
            .await
            .unwrap();
        let verbose = self.verbose;
        loop {
            let (stream, _addr) = listener.accept().await.unwrap();

            let result = tokio::time::timeout(Duration::from_secs(60), tokio::spawn(async move {
                let request = match handle_client(stream).await {
                    Ok(request) => request,
                    Err(mut err) => {
                        if verbose {
                            eprintln!("could not handle request: {:?}", err.error);
                        }
                        err.stream.write_all(&format_response(Status::InternalServerError)).await.unwrap();
                        return;
                    }
                };
    
                on_request(request).await;
            }));

            if let Err(_) = result.await {
                if verbose {
                    eprintln!("connection took too long time to process and was terminated [60s]"); 
                }
            }
        }
    }
}

async fn handle_client(mut stream: TcpStream) -> Result<Request, RequestError> {
    let inital_line = match read_line(&mut stream).await {
        Ok(inital_line) => inital_line,
        Err(error) => {
            return Err(RequestError { 
                stream, 
                error 
            });
        }
    };

    let (method, right) = match inital_line.split_once(" ") {
        Some((method, right)) => (method, right),
        None => {
            return Err(RequestError {
                stream,
                error: Error::new(ErrorKind::InvalidData, "could not parse method"),
            });
        }
    };

    let (path, version) = match right.rsplit_once(" ") {
        Some((path, version)) => (path, version),
        None => {
            return Err(RequestError {
                stream,
                error: Error::new(ErrorKind::InvalidData, "could not parse path and version"),
            });
        }
    };

    let mut headers = HashMap::new();
    loop {
        let current_header = match read_line(&mut stream).await {
            Ok(current_header) => current_header,
            Err(error) => {
                return Err(RequestError { stream, error });
            }
        };

        if current_header.len() == 0 {
            break;
        }

        let (k, v) = match current_header.split_once(": ") {
            Some((k, v)) => (k, v),
            None => {
                return Err(RequestError {
                    stream,
                    error: Error::new(ErrorKind::InvalidData, format!("could not parse header [{}]", current_header)),
                });
            }
        };
        headers.insert(k.to_string(), v.to_string());
    }

    let mut body = Vec::new();
    if headers.contains_key("Content-Length") {
        let content_length = headers
            .get("Content-Length")
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let mut buffer = vec![0; content_length];
        match stream.read_exact(&mut buffer).await {
            Ok(_) => (),
            Err(error) => {
                return Err(RequestError { stream, error });
            }
        }
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
    
    pub body: Vec<u8>,

    pub headers: HashMap<String, String>,
}

pub struct RequestError {
    pub stream: TcpStream,
    pub error: Error,
}

impl Request {
    pub fn get_header(&self, key: &str) -> Option<String> {
        for (k, v) in &self.headers {
            if k.to_lowercase() == key.to_lowercase() {
                return Some(v.to_string());
            }
        }
        return None;
    }

    pub fn get_real_ip(&self, headers: Option<Vec<&str>>) -> String {
        if let Some(headers) = headers {
            for key in headers {
                if let Some(real_ip) = self.headers.get(key) {
                    return real_ip.to_string();
                }
            }
        }

        //Cloudflare
        if let Some(real_ip) = self.get_header("cf-connecting-ip")  {
            return real_ip;
        }
        
        if let Some(real_ip) = self.get_header("X-Real-IP")  {
            return real_ip;
        }

        self.stream.peer_addr().unwrap().ip().to_string()
    }
}
