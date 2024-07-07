use core::fmt;
use std::collections::HashMap;
#[allow(unused, dead_code)]
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
};

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
    pub fn run(&self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(self.address.clone())?;
        for stream in listener.incoming() {
            let mut stream = stream?;

            let request = Request::new(&stream)?;

            let matched_request = self.routes.iter().fold_while(None, |_, route| {
                if let Some(matched_request) = request.match_route(route) {
                    return itertools::FoldWhile::Done(Some((route, matched_request)));
                }

                return itertools::FoldWhile::Continue(None);
            });

            if let Some((route, matched_request)) = matched_request.into_inner() {
                let response = (route.responder)(matched_request);
                stream.write_all(format!("{response}").as_bytes())?
            } else {
                let response = Response::new();
                stream.write_all(format!("{response}").as_bytes())?
            }
        }

        return Ok(());
    }
}

#[derive(Debug)]
struct Route {
    path: String,
    method: String,
    responder: fn(MatchedRequest) -> Response,
}

#[allow(unused, dead_code)]
#[derive(Debug)]
struct Request {
    method: String,
    headers: HashMap<String, String>,
    path: String,
    protocol: String,
}

pub struct MatchedRequest {
    pub path: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub parameters: HashMap<String, String>,
}

impl Request {
    pub fn new(stream: &TcpStream) -> anyhow::Result<Self> {
        let reader = BufReader::new(stream);

        let mut raw_data = reader
            .lines()
            .map(|result| result.unwrap())
            .take_while(|line| !line.is_empty());

        let request_data = raw_data.next().unwrap();
        let headers_data: Vec<String> = raw_data.collect();

        let request_data_parts = request_data.split(" ").take(3).collect::<Vec<_>>();
        let [method, path, protocol] = request_data_parts.as_slice() else {
            anyhow::bail!("Failed to extract data")
        };

        let headers: HashMap<String, String> =
            headers_data
                .iter()
                .fold(HashMap::new(), |mut headers, bar| {
                    if let Some((key, value)) = bar.split_once(":") {
                        headers.insert(key.to_string(), value.to_string());
                    }

                    return headers;
                });

        return Ok(Self {
            method: method.to_string(),
            path: path.to_string(),
            protocol: protocol.to_string(),
            headers,
        });
    }

    pub fn match_route(&self, route: &Route) -> Option<MatchedRequest> {
        if route.method.to_lowercase() != self.method.to_lowercase() {
            return None;
        }

        let contains_parameters = route.path.contains(":");
        if !contains_parameters {
            let exact_match = route.path.to_lowercase() == self.path.to_lowercase();
            if exact_match {
                return Some(MatchedRequest {
                    headers: self.headers.to_owned(),
                    method: self.method.to_owned(),
                    path: self.path.to_owned(),
                    parameters: HashMap::new(),
                });
            }

            return None;
        }

        let parameters = get_parameters(&route.path, &self.path);
        if let Some(parameters) = parameters {
            return Some(MatchedRequest {
                headers: self.headers.to_owned(),
                method: self.method.to_owned(),
                path: self.path.to_owned(),
                parameters: parameters,
            });
        }

        return None;
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

    let expected_num_parameters = defined_path_parts
        .iter()
        .filter(|defined_part| return defined_part.starts_with(":"))
        .count();

    let incoming = request_path_parts.iter();
    let defined = defined_path_parts.iter();

    let mut parameters: HashMap<String, String> = HashMap::new();

    for (incoming_part, defined_part) in izip!(incoming, defined) {
        if defined_part.starts_with(":") {
            let defined_part = &defined_part[1..];

            parameters.insert(defined_part.to_string(), incoming_part.to_string());
        }
    }

    if parameters.len() != expected_num_parameters {
        return None;
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
            404 => {
                self.status_code = 404;
                self.status_verb = String::from("Not found");
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
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut response = String::new();

        response
            .push_str(format!("HTTP/1.1 {} {}\r\n", self.status_code, self.status_verb).as_str());

        if self.headers.len() > 0 {
            self.headers.iter().for_each(|header| {
                response.push_str(format!("{}:{}\r\n", header.0, header.1).as_str())
            });

            response.push_str("\r\n");
        }

        if let Some(body) = &self.body {
            response.push_str(body.as_str());
        }

        response.push_str("\r\n");

        return f.write_str(response.as_str());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let response = Response::new();
        // let parsed_response = format!("{}", response);
        // let x = format!("{response}");
        // for (index, ch) in x.chars().enumerate() {
        //     if index == 1 {
        //         println!("{}", ch);
        //         break;
        //     }
        // }
        print!("{:?}", format!("{response}"));
    }
}
