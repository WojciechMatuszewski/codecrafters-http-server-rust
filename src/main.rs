mod server;

static ADDR: &str = "127.0.0.1:4221";

fn main() -> anyhow::Result<()> {
    let server = server::Server::new(ADDR.to_string());
    server.run()?;

    return Ok(());
}
