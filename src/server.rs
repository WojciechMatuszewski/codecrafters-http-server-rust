use core::fmt;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
    thread,
};
#[allow(unused, dead_code)]
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
};

use flate2::{write::GzEncoder, Compression, GzBuilder};
use itertools::{izip, Itertools};

pub struct Server {
    address: String,
    routes: Vec<Route>,
}

impl Server {
    pub fn new(address: &str) -> Self {
        return Self {
            routes: vec![],
            address: address.to_string(),
        };
    }

    pub fn get(&mut self, path: &str, responder: fn(MatchedRequest) -> Response) -> &mut Self {
        self.routes.push(Route {
            path: path.to_string(),
            method: "get".to_string(),
            responder,
        });

        return self;
    }

    pub fn post(&mut self, path: &str, responder: fn(MatchedRequest) -> Response) -> &mut Self {
        self.routes.push(Route {
            path: path.to_string(),
            method: "post".to_string(),
            responder,
        });

        return self;
    }

    pub fn run(&self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(self.address.clone())?;

        for stream in listener.incoming() {
            let routes = Arc::new(Mutex::new(self.routes.clone()));

            thread::spawn(move || {
                let routes = routes.lock().unwrap();
                handle_connection(stream.unwrap(), routes).unwrap();
            });
        }

        return Ok(());
    }
}

fn handle_connection(mut stream: TcpStream, routes: MutexGuard<Vec<Route>>) -> anyhow::Result<()> {
    let request = Request::new(&mut stream)?;

    let matched_request = routes.iter().fold_while(None, |_, route| {
        if let Some(matched_request) = request.match_route(route) {
            return itertools::FoldWhile::Done(Some((route, matched_request)));
        }

        return itertools::FoldWhile::Continue(None);
    });

    if let Some((route, matched_request)) = matched_request.into_inner() {
        let mut response = (route.responder)(matched_request);
        response.send(&mut stream, &request)?;
    } else {
        let mut response = Response::new();
        response.send(&mut stream, &request)?;
    }

    return Ok(());
}

#[derive(Debug, Clone)]
pub struct Route {
    path: String,
    method: String,
    responder: fn(MatchedRequest) -> Response,
}

#[allow(unused, dead_code)]
#[derive(Debug)]
pub struct Request {
    method: String,
    headers: HashMap<String, String>,
    path: String,
    protocol: String,
    body: Option<String>,
}

pub struct MatchedRequest {
    pub path: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub parameters: HashMap<String, String>,
    pub body: Option<String>,
}

impl Request {
    pub fn new(stream: &mut TcpStream) -> anyhow::Result<Self> {
        let mut buffer = [0u8; 2048];

        let length = stream.read(&mut buffer)?;

        let raw_data = String::from_utf8_lossy(&buffer[..length]);

        let mut request_data = raw_data.lines();

        let request_info = request_data.next().unwrap();
        let request_data_parts = request_info.split(" ").take(3).collect::<Vec<_>>();
        let [method, path, protocol] = request_data_parts.as_slice() else {
            anyhow::bail!("Failed to extract data")
        };

        let mut headers: HashMap<String, String> = HashMap::new();
        loop {
            let request_header = request_data.next().unwrap();
            if request_header.is_empty() {
                break;
            }

            let (key, value) = request_header
                .split_once(":")
                .expect("Failed to parse header");

            headers.insert(key.to_string(), value.trim().to_string());
        }

        let request_body = request_data.next().map(|body| body.to_owned());

        return Ok(Self {
            method: method.to_string(),
            path: path.to_string(),
            protocol: protocol.to_string(),
            headers: headers,
            body: request_body,
        });
    }

    pub fn match_route(&self, route: &Route) -> Option<MatchedRequest> {
        if route.method.to_lowercase() != self.method.to_lowercase() {
            return None;
        }

        match route.path.contains("/:") {
            true => {
                if let Some(parameters) = get_parameters(&route.path, &self.path) {
                    return Some(MatchedRequest {
                        headers: self.headers.to_owned(),
                        method: self.method.to_owned(),
                        path: self.path.to_owned(),
                        parameters: parameters,
                        body: self.body.to_owned(),
                    });
                }

                return None;
            }
            false => {
                let exact_match = route.path == self.path;
                if exact_match {
                    return Some(MatchedRequest {
                        headers: self.headers.to_owned(),
                        method: self.method.to_owned(),
                        path: self.path.to_owned(),
                        body: self.body.to_owned(),
                        parameters: HashMap::new(),
                    });
                }

                return None;
            }
        }
    }
}

fn get_parameters(defined_path: &str, request_path: &str) -> Option<HashMap<String, String>> {
    let request_path_parts: Vec<&str> = request_path
        .split_terminator("/")
        .filter(|path_part| return !path_part.is_empty())
        .collect();

    let defined_path_parts: Vec<&str> = defined_path
        .split_terminator("/")
        .filter(|path_part| return !path_part.is_empty())
        .collect();

    if request_path_parts.len() != defined_path_parts.len() {
        return None;
    }

    let incoming = request_path_parts.iter();
    let defined = defined_path_parts.iter();

    let mut parameters: HashMap<String, String> = HashMap::new();

    for (incoming_part, defined_part) in izip!(incoming, defined) {
        match defined_part.starts_with(":") {
            true => {
                let defined_part = &defined_part[1..];
                parameters.insert(defined_part.to_string(), incoming_part.to_string());
            }
            false => {
                if incoming_part != defined_part {
                    return None;
                }
            }
        }
    }

    return Some(parameters);
}

pub struct Response {
    status_code: i32,
    status_verb: String,

    headers: HashMap<String, String>,
    body: Option<String>,
}

impl Response {
    pub fn new() -> Self {
        return Self {
            status_code: 404,
            status_verb: String::from("Not Found"),
            headers: HashMap::new(),
            body: None,
        };
    }

    pub fn status(&mut self, status: i32) -> &mut Self {
        match status {
            200 => {
                self.status_code = 200;
                self.status_verb = String::from("OK");
            }
            201 => {
                self.status_code = 201;
                self.status_verb = String::from("Created")
            }
            404 => {
                self.status_code = 404;
                self.status_verb = String::from("Not Found");
            }
            _ => {
                self.status_code = status;
                self.status_verb = String::from("Unknown");
            }
        }

        return self;
    }

    pub fn content_type(&mut self, content_type: &str) -> &mut Self {
        self.headers
            .insert("Content-Type".to_string(), content_type.to_string());

        return self;
    }

    pub fn body(&mut self, body: &str) -> &mut Self {
        self.body = Some(body.to_string());

        self.headers
            .insert("Content-Length".to_string(), format!("{}", body.len()));

        return self;
    }

    pub fn build(&self) -> Self {
        return Response {
            status_code: self.status_code,
            status_verb: self.status_verb.to_owned(),
            body: self.body.to_owned(),
            headers: self.headers.to_owned(),
        };
    }

    pub fn send(&mut self, stream: &mut TcpStream, request: &Request) -> anyhow::Result<()> {
        let is_gzip_encoding = request
            .headers
            .get("Accept-Encoding")
            .map(|header_value| return header_value.contains("gzip"))
            .is_some();

        if is_gzip_encoding {
            self.headers
                .insert("Content-Encoding".to_string(), "gzip".to_string());
        }

        let mut response = format!("").into_bytes();

        response.extend_from_slice(
            format!("HTTP/1.1 {} {}\r\n", self.status_code, self.status_verb).as_bytes(),
        );

        let parsed_body = self.body.as_ref().map(|body| match is_gzip_encoding {
            true => {
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(body.as_bytes()).unwrap();
                let compressed_data = encoder.finish().unwrap();
                return compressed_data;
            }
            false => return body.clone().into_bytes(),
        });

        if let Some(body) = parsed_body.clone() {
            self.headers
                .insert("Content-Length".to_string(), body.len().to_string());
        };

        if self.headers.len() > 0 {
            self.headers.iter().for_each(|header| {
                response.extend_from_slice(format!("{}: {}\r\n", header.0, header.1).as_bytes())
            });

            response.extend_from_slice("\r\n".as_bytes());
        }

        if let Some(body) = parsed_body {
            response.extend_from_slice(&body.clone())
        }

        response.extend_from_slice("\r\n".as_bytes());
        stream.write_all(&response)?;

        return Ok(());
    }
}
