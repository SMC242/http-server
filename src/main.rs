use log::{error, info};
use std::io::{ErrorKind, Read, Write};
use std::net::Shutdown;
use std::{
    net::{Ipv4Addr, TcpStream},
    time::Duration,
};

mod request;
mod server;

static IP: Ipv4Addr = Ipv4Addr::LOCALHOST;
// TODO: increment if port is unavailable. Will require this to not be static
static PORT: u16 = 8080;
static TIMEOUT: Option<Duration> = Some(Duration::new(10, 0));
static DEFAULT_RESPONSE: &str = "HTTP/1.1 200 OK\r\n";

fn handle_connection(mut stream: TcpStream) -> std::io::Result<()> {
    let client_ip: String = stream
        .peer_addr()
        .map(|addr| addr.to_string())
        .unwrap_or("IP address unknown".to_string());
    info!(target: "listener", "Connection received from {client_ip}");

    info!(target: "listener", "Configuring connection for {client_ip}");
    stream.set_read_timeout(TIMEOUT)?;
    stream.set_write_timeout(TIMEOUT)?;

    info!(target: "listener", "Reading request content from {client_ip}");
    let mut request_content = String::new();
    match stream.read_to_string(&mut request_content) {
        Ok(n_bytes) => info!(target: "listener", "Read {n_bytes} from {client_ip}"),
        Err(err) => {
            error!(target: "listener", "Failed to read message from {client_ip} due to {err:?}");
            return Err(err);
        }
    };

    info!(target: "listener", "Parsing message from {client_ip} as HTTP request");
    let request = request::http1_1::parse_req(request_content.as_str()).map_err(|err| {
        info!(target: "listener", "Failed to parse request from {client_ip} due to the following error: {err}");
        std::io::Error::new(
            ErrorKind::InvalidData,
            "Could not parse message as HTTP request",
        )
    })?;

    info!(target: "listener", "Request received from {client_ip}: {request:?}");
    stream.write_all(DEFAULT_RESPONSE.as_bytes())
}

fn main() -> std::io::Result<()> {
    env_logger::init();
    info!(target: "listener", "Starting server on {IP}:{PORT}");
    server::listener::listen(IP, PORT, handle_connection)
}
