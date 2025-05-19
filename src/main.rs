use log::{debug, info};
use request::{HTTPHeaders, HTTPVersion};
use server::handlers::{Handler, HandlerRegistry};
use server::response::{Response, ResponseStatus};
use std::io::{BufRead, BufReader, Error as IoError, ErrorKind, Read, Write};
use std::net::{Ipv4Addr, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod dog_crud_example;
use dog_crud_example as dogstore;
mod mime;
mod request;
mod server;

static IP: Ipv4Addr = Ipv4Addr::LOCALHOST;
// TODO: increment if port is unavailable. Will require this to not be static
static PORT: u16 = 8080;
static TIMEOUT: Option<Duration> = Some(Duration::new(10, 0));
static DEFAULT_RESPONSE: &str = "HTTP/1.1 200 OK\r\n";
static CARRIAGE_RETURN: &str = "\r\n";

fn handle_connection(mut stream: TcpStream) -> std::io::Result<()> {
    let client_ip: String = stream
        .peer_addr()
        .map(|addr| addr.to_string())
        .unwrap_or("IP address unknown".to_string());
    info!(target: "listener", "Connection received from {client_ip}");

    info!(target: "listener", "Configuring connection for {client_ip}");
    stream.set_read_timeout(TIMEOUT)?;
    stream.set_write_timeout(TIMEOUT)?;

    let mut request_content = String::new();
    // Read until end of request head (empty line).
    // NOTE: further reading will be required to get the request body
    let mut reader = stream.try_clone().map(BufReader::new)?;
    // This ultimately does 2 passes through the connection :( Would it be possible to cut out
    // the first pass? The main reason for it is to unwrap each line
    for line in reader.by_ref().lines() {
        let mut unwrapped = line?;
        if unwrapped.is_empty() {
            break;
        } else {
            let n_bytes = unwrapped.len();
            info!(target: "listener", "Read {n_bytes} from {client_ip}");
            unwrapped.push_str(CARRIAGE_RETURN);
            request_content += &unwrapped;
        }
    }

    info!(target: "listener", "Parsing message from {client_ip} as HTTP request");
    // This iterator will be adavanced to the request body
    //let readerfn: FnMut(usize) -> Box<[u8]> = |size| reader.read(&mut Box::new([0u8; size]));
    let req_lines = &mut request_content.lines();
    let req_head = request::http1_1::parse_req_head(req_lines).map_err(|err| {
        info!(target: "listener", "Failed to parse request from {client_ip} due to the following error: {err}");
        IoError::new(
            ErrorKind::InvalidData,
            "Could not parse message as HTTP request",
        )
    })?;
    info!(target: "listener", "Request received from {client_ip}: {req_head:?}");

    let request = request::Request::new(req_head, reader);

    let dog_store = Arc::new(Mutex::new(dogstore::DogStore::default()));
    let handlers: Vec<Arc<dyn Handler>> = vec![
        Arc::new(dogstore::DogStoreGetHandler::new(dog_store.clone())),
        Arc::new(dogstore::DogStorePostHandler::new(dog_store.clone())),
    ];

    let handler_registry = HandlerRegistry::new(handlers);

    let response = if let Ok(request_path) = request.head.path.clone().try_into() {
        match handler_registry.get(request.head.method, request_path) {
            Some(handler) => handler.on_request(&request),
            None => Response::new(
                HTTPVersion::V1_1,
                ResponseStatus::NotFound,
                HTTPHeaders::default(),
                "No matching handler found".to_string(),
            ),
        }
    } else {
        Response::new(
            HTTPVersion::V1_1,
            ResponseStatus::BadRequest,
            HTTPHeaders::default(),
            "Malformed URL path".to_string(),
        )
    };
    stream.write_all(response.to_string().as_bytes())
}

fn main() -> std::io::Result<()> {
    env_logger::init();
    info!(target: "listener", "Starting server on {IP}:{PORT}");
    server::listener::listen(IP, PORT, handle_connection)
}
