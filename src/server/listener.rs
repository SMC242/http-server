use crate::request::{self, http1_1, HTTPHeaders, HTTPVersion};
use std::{
    io::{BufRead, BufReader, Error as IoError, ErrorKind, Read, Write},
    net::{IpAddr, TcpListener, TcpStream},
};

use log::info;

use crate::request::{Request, RequestParseError};

use super::{
    handlers::HandlerRegistry,
    response::{Response, ResponseStatus},
};

static CARRIAGE_RETURN: &str = "\r\n";

/// A low-level function for receiving and operating on TCP connections.
/// Use `Listener` for a higher level interface
pub fn listen<E, F>(ip: IpAddr, port: u16, mut on_stream: F) -> std::io::Result<()>
where
    F: FnMut(TcpStream) -> Result<(), E>,
    E: std::fmt::Debug,
{
    let listener = TcpListener::bind((ip, port))?;
    for stream in listener.incoming() {
        let _ = on_stream(stream?)
            .inspect_err(|err| println!("Error occurred in on_stream: {0:?}", err));
    }
    Ok(())
}

#[derive(Debug)]
pub struct ListenerConfig {
    timeout: Option<std::time::Duration>,
}

impl Default for ListenerConfig {
    fn default() -> Self {
        Self {
            timeout: Some(std::time::Duration::new(10, 0)),
        }
    }
}

/// Parses incoming HTTP messages from TCP connections using
/// the given parse function before dispatching the request to handlers.
/// Will support middleware in the future
pub struct HTTPListener {
    ip: IpAddr,
    port: u16,
    handler_registry: HandlerRegistry,
    config: ListenerConfig,
}

impl HTTPListener {
    pub fn new(
        ip: IpAddr,
        port: u16,
        handler_registry: HandlerRegistry,
        config: ListenerConfig,
    ) -> Self {
        Self {
            ip,
            port,
            handler_registry,
            config,
        }
    }

    pub fn listen(&self) -> std::io::Result<()> {
        listen(self.ip, self.port, |mut conn| {
            self.handle_connection(&mut conn)
        })
    }

    fn handle_connection(&self, stream: &mut TcpStream) -> Result<(), IoError> {
        let client_ip: String = stream
            .peer_addr()
            .map(|addr| addr.to_string())
            .unwrap_or("IP address unknown".to_string());
        info!(target: "listener", "Connection received from {client_ip}");

        info!(target: "listener", "Configuring connection for {client_ip}");
        self.configure_connection(stream)?;

        let (request_content, reader) = self.read_message(stream)?;
        info!(target: "listener", "Parsing message from {client_ip} as HTTP request");

        let request_head = self.parse_message(request_content).map_err(|err| {
        info!(target: "listener", "Failed to parse request from {client_ip} due to the following error: {err}");
        IoError::new(
            ErrorKind::InvalidData,
            "Could not parse message as HTTP request",
        )
    })?;
        info!(target: "listener", "Request received from {client_ip}: {request_head:?}");

        let request = request::Request::new(request_head, reader);

        let response = self.dispatch(request);
        stream.write_all(response.to_string().as_bytes())
    }

    fn configure_connection(&self, conn: &TcpStream) -> Result<(), IoError> {
        conn.set_read_timeout(self.config.timeout)?;
        conn.set_write_timeout(self.config.timeout)?;
        Ok(())
    }

    fn read_message(&self, stream: &TcpStream) -> Result<(String, BufReader<TcpStream>), IoError> {
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
                unwrapped.push_str(CARRIAGE_RETURN);
                request_content += &unwrapped;
            }
        }

        Ok((request_content, reader))
    }

    fn parse_message(
        &self,
        message: String,
    ) -> Result<crate::request::RequestHead, RequestParseError> {
        // This iterator will be adavanced to the request body
        let req_lines = &mut message.lines();
        http1_1::parse_req_head(req_lines)
    }

    fn dispatch(&self, request: Request) -> Response {
        if let Ok(request_path) = request.head.path.clone().try_into() {
            match self.handler_registry.get(request.head.method, request_path) {
                Some(handler) => handler.on_request(&request),
                None => Response::new(
                    HTTPVersion::V1_1,
                    super::response::ResponseStatus::NotFound,
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
        }
    }
}
