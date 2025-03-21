use log::{error, info};
use std::io::{BufRead, BufReader, Error as IoError, ErrorKind, Read, Write};
use std::{
    net::{Ipv4Addr, TcpStream},
    time::Duration,
};

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
    let request = request::http1_1::parse_req_head(req_lines).map_err(|err| {
        info!(target: "listener", "Failed to parse request from {client_ip} due to the following error: {err}");
        IoError::new(
            ErrorKind::InvalidData,
            "Could not parse message as HTTP request",
        )
    })?;

    // TODO: read body if required
    // Somehow convert the lines back into bytes, lazily
    //let body: request::RequestBody = reader.bytes();
    let mut bodybuf: Vec<u8> = Vec::new();
    reader.read(&mut bodybuf).unwrap();
    println!("Body: {0}", String::from_utf8(bodybuf).unwrap());
    info!(target: "listener", "Request received from {client_ip}: {request:?}");
    stream.write_all(DEFAULT_RESPONSE.as_bytes())
}

fn main() -> std::io::Result<()> {
    env_logger::init();
    info!(target: "listener", "Starting server on {IP}:{PORT}");
    server::listener::listen(IP, PORT, handle_connection)
}
