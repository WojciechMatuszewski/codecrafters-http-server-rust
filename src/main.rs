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
        .run()?;

    return Ok(());
}
