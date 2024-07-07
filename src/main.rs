use std::fs::read_to_string;
use std::path::PathBuf;

use server::Response;
use server::Server;

mod server;

static ADDR: &str = "127.0.0.1:4221";

fn main() -> anyhow::Result<()> {
    Server::new(ADDR)
        .get("/", |_| {
            let response = Response::new()
                .status(200)
                .content_type("text/plain")
                .build();
            return response;
        })
        .get("/echo/:str", |matched_request| {
            let param = matched_request.parameters.get("str").unwrap();

            let response = Response::new()
                .status(200)
                .content_type("text/plain")
                .body(param)
                .build();
            return response;
        })
        .get("/user-agent", |matched_request| {
            let user_agent = matched_request.headers.get("User-Agent").unwrap();

            let response = Response::new()
                .status(200)
                .content_type("text/plain")
                .body(&user_agent)
                .build();

            return response;
        })
        .get("/files/:filename", |matched_request| {
            let filename = matched_request.parameters.get("filename").unwrap();
            let path = PathBuf::from(format!("/tmp/{filename}"));

            if let Ok(file_contents) = read_to_string(path) {
                let response = Response::new()
                    .status(200)
                    .content_type("application/octet-stream")
                    .body(&file_contents)
                    .build();

                return response;
            } else {
                let response = Response::new()
                    .status(404)
                    .content_type("application/octet-stream")
                    .build();
                return response;
            }
        })
        .run()?;

    return Ok(());
}
