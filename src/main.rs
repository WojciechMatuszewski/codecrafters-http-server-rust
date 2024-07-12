use std::env;
use std::fs::read_to_string;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use server::Response;
use server::Server;

mod server;

static ADDR: &str = "127.0.0.1:4221";

fn main() -> anyhow::Result<()> {
    Server::new(ADDR)
        .get("/", |_| {
            let response = Response::new().status(200).build();
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

            let args: Vec<String> = env::args().collect();
            let file_directory = args.get(2).unwrap();

            let path = PathBuf::from(format!("/{file_directory}/{filename}"));
            match read_to_string(path) {
                Ok(file_content) => {
                    let response = Response::new()
                        .status(200)
                        .content_type("application/octet-stream")
                        .body(&file_content)
                        .build();

                    return response;
                }
                Err(error) => {
                    println!("{error} {file_directory}");

                    let response = Response::new()
                        .status(404)
                        .content_type("text/plain")
                        .build();

                    return response;
                }
            }
        })
        .post("/files/:filename", |matched_request| {
            let filename = matched_request.parameters.get("filename").unwrap();
            let args: Vec<String> = env::args().collect();
            let file_directory = args.get(2).unwrap();
            let path = PathBuf::from(format!("/{file_directory}/{filename}"));

            let request_body = matched_request.body.unwrap();
            let mut file = File::create(path).unwrap();
            file.write_all(request_body.as_bytes()).unwrap();

            let response = Response::new().status(201).build();
            return response;
        })
        .run()?;

    return Ok(());
}
