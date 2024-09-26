use mws::{ Request, WebServer };
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() {
    let server = WebServer::new("0.0.0.0", 80, |mut request: Request| {
        tokio::spawn(async move {
            println!("> {}", mws::get_real_ip(&request, None));

            let request_parsed = format!("{:#?}", request);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                request_parsed.len(),
                request_parsed
            );

            request.stream.write_all(response.as_bytes()).await.unwrap();
        });
    }).await;

    server.run().await;
}