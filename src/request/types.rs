use std::{collections::HashMap, str::FromStr};

#[derive(Debug, PartialEq)]
pub enum Path {
    OriginForm(String),
    AbsoluteForm(String),
    AuthorityForm(String, u16), // Used by the CONNECT method
    Asterisk,                   // Used by the OPTIONS method
}

#[derive(Debug, PartialEq)]
pub enum HTTPMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    // More obscure methods below
    Connect,
    Options,
    Trace,
    Head,
}

pub type HTTPHeaders = HashMap<String, String>;

#[derive(Debug)]
pub struct Request {
    pub method: HTTPMethod,
    pub path: Path,
    pub version: HTTPVersion;
    pub headers: HTTPHeaders,
    pub body: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum RequestParseError {
    InvalidStartLine(&'static str),
    InvalidHeader(String),
    MissingHostHeader, // HTTP 1.1 requires the Host header to be set
    InvalidBody(String),
    UnsupportedVersion(String),
}

#[derive(Debug, PartialEq, PartialOrd)]
pub enum HTTPVersion {
    V0_9,
    V1_0,
    V1_1,
    V2,
    V3,
}

impl FromStr for Path {
    type Err = ();
    fn from_str(path: &str) -> Result<Path, Self::Err> {
        if path.starts_with('/') {
            Ok(Path::OriginForm(path.to_string()))
        }
        // TODO: support HTTPS
        else if path.starts_with("http://") {
            Ok(Path::AbsoluteForm(path.to_string()))
        } else if path.contains(':') {
            // TODO: refactor this. It sucks!
            match path.split(':').take(2).collect::<Vec<_>>()[0..1] {
                [domain, port] => {
                    let parsed_port = u16::from_str(port).or(Err(()))?;
                    Ok(Path::AuthorityForm(domain.to_string(), parsed_port))
                }
                _ => Err(()),
            }
        } else if path == "*" {
            Ok(Path::Asterisk)
        } else {
            Err(())
        }
    }
}

impl FromStr for HTTPMethod {
    type Err = ();
    fn from_str(s: &str) -> Result<HTTPMethod, Self::Err> {
        match s {
            "GET" => Ok(HTTPMethod::Get),
            "POST" => Ok(HTTPMethod::Post),
            "PUT" => Ok(HTTPMethod::Put),
            "PATCH" => Ok(HTTPMethod::Patch),
            "DELETE" => Ok(HTTPMethod::Delete),
            "CONNECT" => Ok(HTTPMethod::Connect),
            "OPTIONS" => Ok(HTTPMethod::Options),
            "TRACE" => Ok(HTTPMethod::Trace),
            "HEAD" => Ok(HTTPMethod::Head),
            _ => Err(()),
        }
    }
}

impl FromStr for HTTPVersion {
    type Err = RequestParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "HTTP/0.9" => Ok(HTTPVersion::V0_9),
            "HTTP/1.0" => Ok(HTTPVersion::V1_0),
            "HTTP/1.1" => Ok(HTTPVersion::V1_1),
            // NOTE: HTTP 2 and 3 do not have start lines and therefore don't have a version string
            version => Err(RequestParseError::UnsupportedVersion(version.to_string())),
        }
    }
}

impl std::fmt::Display for RequestParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let prelude = "Failed to parse request.";
        let content = match self {
            Self::InvalidBody(body) => format!("Body is invalid: \"{body}\""),
            Self::InvalidStartLine(reason) => format!("Start line is invalid: {reason}"),
            Self::MissingHostHeader => {
                "The Host header must be passed in HTTP/1.1 requests".to_string()
            }
            Self::InvalidHeader(header_line) => {
                format!("The following header was invalid: \"{header_line}\"")
            }
            Self::UnsupportedVersion(version) => format!("Unsupported version \"{version}\""),
        };
        write!(f, "{prelude}\n=>{content}")
    }
}
