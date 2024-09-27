use mws::{Request, WebServer};
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() {
    let server = WebServer::new("0.0.0.0", 80, |mut request: Request| {
        tokio::spawn(async move {
            println!("> {}", mws::get_real_ip(&request, None));

            let request_parsed = format!("{:#?}", request);
            request.stream.write_all(
                format_response_with_body(
                    "200 OK",
                    request_parsed.into_bytes(),
                )
            ).await.unwrap();
        });
    })
    .await;

    server.run().await;
}
