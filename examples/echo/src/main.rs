use mws::{html::Status, utils, Request, WebServer};
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() {
    let server = WebServer::new("0.0.0.0", 80, true);

    server.listen(|mut request: Request| async move {
        println!("> {}", request.get_real_ip(None));

        let request_parsed = format!("{:#?}", request);
        request.stream.write_all(
            &utils::format_response_with_body(
                Status::OK,
                request_parsed,
            )
        ).await.unwrap();
    }).await;
}
