#[allow(unused, dead_code)]
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
};

pub struct Server {
    address: String,
}

impl Server {
    pub fn new(address: String) -> Self {
        return Self { address };
    }

    pub fn run(self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(self.address)?;

        for stream in listener.incoming() {
            handle_client(stream?)?;
        }

        return Ok(());
    }
}

fn handle_client(mut stream: TcpStream) -> anyhow::Result<()> {
    let request = Request::new(&stream)?;

    if request.path != "/" {
        let response = "HTTP/1.1 404 Not Found\r\n\r\n";
        stream.write_all(response.as_bytes())?;
    } else {
        let response = "HTTP/1.1 200 OK\r\n\r\n";
        stream.write_all(response.as_bytes())?;
    }

    return Ok(());
}

#[allow(unused, dead_code)]
struct Request {
    method: String,
    headers: Vec<String>,
    path: String,
    protocol: String,
}

impl Request {
    fn new(stream: &TcpStream) -> anyhow::Result<Self> {
        let reader = BufReader::new(stream);

        let mut raw_data = reader
            .lines()
            .map(|result| result.unwrap())
            .take_while(|line| !line.is_empty());

        let request = raw_data.next().unwrap();
        let headers: Vec<String> = raw_data.collect();

        if let [method, path, protocol] = request.split(" ").take(3).collect::<Vec<_>>().as_slice()
        {
            return Ok(Self {
                method: method.to_string(),
                path: path.to_string(),
                protocol: protocol.to_string(),
                headers,
            });
        }

        anyhow::bail!("Failed to extract");
    }
}
