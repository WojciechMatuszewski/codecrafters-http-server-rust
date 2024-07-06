use std::{
    io::Write,
    net::{TcpListener, TcpStream},
};

static ADDR: &str = "127.0.0.1:4221";

fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind(ADDR)?;

    for stream in listener.incoming() {
        handle_client(stream?)?;
    }

    return Ok(());
}

fn handle_client(mut stream: TcpStream) -> anyhow::Result<()> {
    let response = "HTTP/1.1 200 OK\r\n\r\n";
    stream.write_all(response.as_bytes())?;

    return Ok(());
}
