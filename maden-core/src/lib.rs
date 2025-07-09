use maden_config::Server as ServerConfig;
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

// --- HTTP Types --- //

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    // Add more methods as needed
    UNKNOWN,
}

impl From<&str> for HttpMethod {
    fn from(s: &str) -> Self {
        match s {
            "GET" => HttpMethod::GET,
            "POST" => HttpMethod::POST,
            "PUT" => HttpMethod::PUT,
            "DELETE" => HttpMethod::DELETE,
            _ => HttpMethod::UNKNOWN,
        }
    }
}

pub struct Request {
    pub method: HttpMethod,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub path_params: HashMap<String, String>,
}

pub struct Response {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status: u16) -> Self {
        Response {
            status,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.insert(name.to_string(), value.to_string());
        self
    }
}

// --- Handler and Route --- //

pub type Handler = Box<dyn Fn(Request) -> Response + Send + Sync + 'static>;

pub struct Route {
    pub method: HttpMethod,
    pub path: String,
    pub handler: Handler,
}

pub struct Router {
    routes: HashMap<HttpMethod, HashMap<String, Handler>>,
}

impl Router {
    pub fn new() -> Self {
        Router {
            routes: HashMap::new(),
        }
    }

    pub fn add_route(&mut self, method: HttpMethod, path: &str, handler: Handler) {
        self.routes
            .entry(method)
            .or_default()
            .insert(path.to_string(), handler);
    }

    pub fn get(&mut self, path: &str, handler: Handler) {
        self.add_route(HttpMethod::GET, path, handler);
    }

    pub fn post(&mut self, path: &str, handler: Handler) {
        self.add_route(HttpMethod::POST, path, handler);
    }

    pub fn match_route(&self, method: &HttpMethod, request_path: &str) -> Option<&Handler> {
        if let Some(method_routes) = self.routes.get(method) {
            // Try exact match first
            if let Some(handler) = method_routes.get(request_path) {
                return Some(handler);
            }

            // Try path parameter matching
            for (route_path, handler) in method_routes.iter() {
                if route_path.contains(":") {
                    let route_parts: Vec<&str> = route_path.split('/').collect();
                    let request_parts: Vec<&str> = request_path.split('/').collect();

                    if route_parts.len() == request_parts.len() {
                        let mut params = HashMap::new();
                        let mut is_match = true;
                        for i in 0..route_parts.len() {
                            if route_parts[i].starts_with(":") {
                                params.insert(route_parts[i][1..].to_string(), request_parts[i].to_string());
                            } else if route_parts[i] != request_parts[i] {
                                is_match = false;
                                break;
                            }
                        }
                        if is_match {
                            // For now, we just return the handler. Path params will be passed in Request.
                            return Some(handler);
                        }
                    }
                }
            }
        }
        None
    }
}

// --- Actor Trait and Message --- //

pub enum ActorMessage {
    HttpRequest(TcpStream),
    // Add other message types as needed
}

pub trait Actor {
    fn start(self, receiver: mpsc::Receiver<ActorMessage>);
}

// --- HttpActor Implementation --- //

pub struct HttpActor {
    config: ServerConfig,
    router: Router,
}

impl HttpActor {
    pub fn new(config: ServerConfig, router: Router) -> Self {
        HttpActor { config, router }
    }

    async fn handle_http_request(mut stream: TcpStream, router: &Router) {
        let mut buffer = vec![0; 4096]; // Increased buffer size
        let bytes_read = stream.read(&mut buffer).await.unwrap();
        let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);

        let mut lines = request_str.lines();
        let request_line = lines.next().unwrap_or("");

        // Parse request line: METHOD /path HTTP/1.1
        let parts: Vec<&str> = request_line.splitn(3, ' ').collect();
        if parts.len() < 3 {
            let response = Response::new(400).with_body("Bad Request: Invalid request line");
            send_response(&mut stream, response).await;
            return;
        }

        let method = HttpMethod::from(parts[0]);
        let path = parts[1].to_string();
        let http_version = parts[2]; // Not used for now

        let mut headers = HashMap::new();
        let mut content_length = 0;
        let mut body_start_index = 0;

        for (i, line) in lines.clone().enumerate() {
            if line.is_empty() {
                body_start_index = request_line.len() + lines.take(i).map(|l| l.len() + 2).sum::<usize>() + 4; // +2 for \r\n, +4 for \r\n\r\n
                // Adjust body_start_index to be relative to the start of the buffer
                let header_section_len = buffer[..bytes_read].iter().position(|&b| b == b'\r' && buffer[bytes_read - 1] == b'\n' && buffer[bytes_read - 2] == b'\r' && buffer[bytes_read - 3] == b'\n').unwrap_or(bytes_read);
                // This is a very rough estimate and needs proper HTTP parsing
                body_start_index = request_str.find("\r\n\r\n").map_or(bytes_read, |idx| idx + 4);
                break;
            }
            let header_parts: Vec<&str> = line.splitn(2, ':').collect();
            if header_parts.len() == 2 {
                let name = header_parts[0].trim().to_string();
                let value = header_parts[1].trim().to_string();
                if name.eq_ignore_ascii_case("Content-Length") {
                    content_length = value.parse::<usize>().unwrap_or(0);
                }
                headers.insert(name, value);
            }
        }

        let body = if content_length > 0 {
            // This is a very simplified body reading. It assumes the entire body is in the first buffer read.
            // In a real server, you'd need to read more from the stream if the body is larger than the buffer.
            buffer[body_start_index..bytes_read].to_vec()
        } else {
            Vec::new()
        };

        let mut request = Request { method: method.clone(), path: path.clone(), headers, body, path_params: HashMap::new() };

        // Path parameter matching and extraction
        let mut matched_handler: Option<&Handler> = None;
        let mut extracted_params = HashMap::new();

        if let Some(method_routes) = router.routes.get(&method) {
            // Try exact match first
            if let Some(handler) = method_routes.get(&path) {
                matched_handler = Some(handler);
            } else {
                // Try path parameter matching
                for (route_path_pattern, handler) in method_routes.iter() {
                    if route_path_pattern.contains(":") {
                        let route_parts: Vec<&str> = route_path_pattern.split('/').collect();
                        let request_parts: Vec<&str> = path.split('/').collect();

                        if route_parts.len() == request_parts.len() {
                            let mut current_params = HashMap::new();
                            let mut is_match = true;
                            for i in 0..route_parts.len() {
                                if route_parts[i].starts_with(":") {
                                    current_params.insert(route_parts[i][1..].to_string(), request_parts[i].to_string());
                                } else if route_parts[i] != request_parts[i] {
                                    is_match = false;
                                    break;
                                }
                            }
                            if is_match {
                                matched_handler = Some(handler);
                                extracted_params = current_params;
                                break;
                            }
                        }
                    }
                }
            }
        }

        request.path_params = extracted_params;

        let response = if let Some(handler) = matched_handler {
            handler(request)
        } else {
            Response::new(404).with_body("Not Found").with_header("Content-Type", "text/plain")
        };

        send_response(&mut stream, response).await;
    }
}

impl Actor for HttpActor {
    fn start(self, mut receiver: mpsc::Receiver<ActorMessage>) {
        let config = self.config;
        let router = self.router;
        tokio::spawn(async move {
            let addr: SocketAddr = SocketAddr::new(config.ip.parse().unwrap(), config.port);
            let listener = TcpListener::bind(addr).await.unwrap();

            println!("Maden server (HttpActor) running at http://{}", addr);

            loop {
                tokio::select! {
                    Ok((stream, _)) = listener.accept() => {
                        HttpActor::handle_http_request(stream, &router).await;
                    }
                    Some(message) = receiver.recv() => {
                        match message {
                            ActorMessage::HttpRequest(stream) => {
                                HttpActor::handle_http_request(stream, &router).await;
                            }
                        }
                    }
                    else => break, // Channel closed or listener error
                }
            }
        });
    }
}

async fn send_response(stream: &mut TcpStream, response: Response) {
    let status_line = format!("HTTP/1.1 {} {}\r\n", response.status, get_status_text(response.status));
    let mut headers_line = String::new();
    for (name, value) in response.headers {
        headers_line.push_str(&format!("{}: {}\r\n", name, value));
    }
    let content_length = response.body.len();
    headers_line.push_str(&format!("Content-Length: {}\r\n", content_length));
    headers_line.push_str("\r\n");

    let full_response = [status_line.as_bytes(), headers_line.as_bytes(), response.body.as_slice()].concat();

    stream.write_all(&full_response).await.unwrap();
    stream.flush().await.unwrap();
}

fn get_status_text(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Unknown Status",
    }
}

// --- Main Run Function --- //

pub async fn run(config: ServerConfig, router: Router) -> Result<(), Box<dyn std::error::Error>> {
    let (_sender, receiver) = mpsc::channel(32);

    let http_actor = HttpActor::new(config, router);
    http_actor.start(receiver);

    // Keep the main task alive indefinitely
    std::future::pending::<()>().await;

    Ok(())
}