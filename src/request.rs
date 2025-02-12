use fancy_regex::Regex;
use std::{collections::HashMap, str::FromStr};

type HTTPHeaders = HashMap<String, String>;

// TODO: add more methods
#[derive(Debug, PartialEq, Eq)]
enum HTTPMethod {
    Get,
}

#[derive(PartialEq, PartialOrd, Debug)]
enum HTTPVersion {
    V1_1,
    V2,
}

type Domain = String;
type Port = u16;

struct Request {
    method: HTTPMethod,
    path: String,
    http_version: HTTPVersion,
    host: Domain,
    port: Option<Port>,
    headers: HTTPHeaders,
    // TODO: store the POST data. Maybe have a variant?
    //data: &'a String,
}

#[derive(Debug, PartialEq, Eq)]
enum HeaderParseError {
    MissingHeaderName,
    MissingValue,
}

#[derive(Debug, PartialEq, Eq)]
enum RequestParseError {
    UnsupportedVersion(String),
    MissingVersion,
    NotHTTP,
    UnsupportedMethod(String),
    MissingMethod,
    MissingPath,
    MalformedHeaders,
    MalformedVersion,
    MissingHost,
    InvalidHostDomain,
    HostBadPort,
}

fn is_newline(c: &char) -> bool {
    matches!(c, '\r' | '\n')
}

fn take_until<F, T, Iter, Collection>(pred: F, iter: &mut Iter) -> Collection
where
    T: PartialEq,
    F: Fn(&T) -> bool,
    Iter: Iterator<Item = T>,
    Collection: std::iter::FromIterator<T>,
{
    iter.take_while(|x| !pred(x)).collect()
}

impl FromStr for HTTPVersion {
    type Err = RequestParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() {
            return Err(RequestParseError::MissingVersion);
        }

        let mut iter = value.chars();
        let protocol: String = take_until(|x| *x == '/' || is_newline(x), iter.by_ref());
        if protocol.is_empty() {
            return Err(RequestParseError::MissingVersion);
        }
        if protocol != "HTTP" {
            return Err(RequestParseError::NotHTTP);
        }

        match take_until::<_, _, _, String>(is_newline, &mut iter).as_str() {
            "" => Err(RequestParseError::MissingVersion),
            "1.1" => Ok(HTTPVersion::V1_1),
            "2.2" => Ok(HTTPVersion::V2),
            version => Err(RequestParseError::UnsupportedVersion(version.to_string())),
        }
    }
}

fn parse_host(
    stream: &mut impl Iterator<Item = char>,
) -> Result<(Domain, Option<Port>), RequestParseError> {
    if stream.take(5).collect::<String>() != "Host:" {
        return Err(RequestParseError::MissingHost);
    };

    let host = err_if_empty(
        RequestParseError::MissingHost,
        take_until(is_newline, stream),
    )?;

    // Separate port from domain because IDNA doesn't validate ports
    let (domain, port) = host
        .split_once(':')
        .map(|(left, right)| (left, Some(right)))
        .unwrap_or((&host, None));

    let port_num = port
        .map(|p| u16::from_str(p).map_err(|_| RequestParseError::HostBadPort))
        .transpose()?;

    // Handle internationalised domains. See https://stackoverflow.com/a/26987741
    // TODO: check for IPV6. See https://docs.rs/idna/1.0.3/idna/uts46/struct.AsciiDenyList.html
    let ascii_domain =
        idna::domain_to_ascii_cow(domain.trim().as_bytes(), idna::AsciiDenyList::URL)
            .map_err(|_| RequestParseError::InvalidHostDomain)?;

    // Regex from https://stackoverflow.com/a/26987741
    // adapted to support ports
    let domain_regex = Regex::new(
        r"^(((?!-))(xn--|_)?[a-z0-9-]{0,61}[a-z0-9]{1,1}\.)*(xn--)?([a-z0-9][a-z0-9\-]{0,60}|[a-z0-9-]{1,30}\.[a-z]{2,})(?::(\d{1,5}))?$",
    ).expect("The domain regex should compile");
    // fancy-regex returns errors when the regex times out. This mitigates DDoS attacks
    match Regex::is_match(&domain_regex, ascii_domain.trim()).unwrap_or(false) {
        true => Ok((ascii_domain.to_string(), port_num)),
        false => Err(RequestParseError::InvalidHostDomain),
    }
}

/*
* Check that an HTTP version string is valid. Supports HTTP 1.1
*/
fn validate_http_version(value: &str) -> Option<RequestParseError> {
    if value.is_empty() {
        return Some(RequestParseError::MissingVersion);
    }

    let mut iter = value.chars();
    let protocol: String = take_until(|x| *x == '/' || is_newline(x), iter.by_ref());
    if protocol.is_empty() {
        return Some(RequestParseError::MissingVersion);
    }
    if protocol != "HTTP" {
        return Some(RequestParseError::NotHTTP);
    }

    match take_until::<_, _, _, String>(is_newline, &mut iter).as_str() {
        "" => Some(RequestParseError::MissingVersion),
        "1.1" => None,
        // E.G 0.9
        version => Some(RequestParseError::UnsupportedVersion(version.to_string())),
    }
}

impl FromStr for HTTPMethod {
    type Err = RequestParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GET" => Ok(HTTPMethod::Get),
            method => Err(Self::Err::UnsupportedMethod(method.to_string())),
        }
    }
}

fn err_if_empty<E>(err: E, s: String) -> Result<String, E> {
    println!("input was {s}");
    (!s.is_empty()).then_some(s).ok_or(err)
}

fn parse_http1_1_headers(s: &str) -> Result<HTTPHeaders, HeaderParseError> {
    let mut headers: HTTPHeaders = HashMap::new();
    for line in s.lines() {
        let mut line_chars = line.chars();
        let header_name = err_if_empty(
            HeaderParseError::MissingHeaderName,
            take_until(|c| *c == ':', &mut line_chars),
        )?;

        let header_value = err_if_empty(HeaderParseError::MissingValue, line_chars.collect())?;
        headers.insert(header_name, header_value);
    }
    Ok(headers)
}

struct HTTP1_1RequestLine {
    method: HTTPMethod,
    path: String,
}

fn parse_http1_1_request_line(
    stream: &mut impl Iterator<Item = char>,
) -> Result<HTTP1_1RequestLine, RequestParseError> {
    let method: String = err_if_empty(
        RequestParseError::MissingMethod,
        take_until(|c| *c == ' ' || is_newline(c), stream.by_ref()),
    )?;
    let parsed_method = HTTPMethod::from_str(method.as_str())?;

    let path: String = err_if_empty(
        RequestParseError::MissingPath,
        take_until(|c| *c == ' ' || is_newline(c), stream.by_ref()),
    )?;

    let http_version_string: String = err_if_empty(
        RequestParseError::MissingVersion,
        stream.by_ref().take_while(|c| !is_newline(c)).collect(),
    )?;
    if http_version_string.contains(' ') {
        return Err(RequestParseError::MalformedHeaders);
    }

    // Ensure valid HTTP version
    match validate_http_version(http_version_string.as_str()) {
        None => Ok(HTTP1_1RequestLine {
            path,
            method: parsed_method,
        }),
        Some(err) => Err(err),
    }
}

impl FromStr for Request {
    type Err = RequestParseError;

    fn from_str(s: &str) -> Result<Request, RequestParseError> {
        let mut chars = s.chars();
        let request_line = parse_http1_1_request_line(&mut chars.by_ref())?;

        let (host, port) = parse_host(&mut chars.by_ref())?;
        // TODO: support upgrading to HTTP 2
        // See https://serverfault.com/questions/1060286/what-is-the-request-line-for-http-2
        match parse_http1_1_headers(chars.by_ref().collect::<String>().as_str()) {
            Ok(headers) => Ok(Request {
                method: request_line.method,
                path: request_line.path,
                host,
                port,
                http_version: HTTPVersion::V1_1,
                headers,
            }),
            Err(_) => Err(RequestParseError::MalformedHeaders),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_version_parse_1_1() {
        let result = HTTPVersion::from_str("HTTP/1.1").expect("Parsing HTTP/1.1 should succeed");
        assert_eq!(
            HTTPVersion::V1_1,
            result,
            "Expected HTTP V1, parsed {result:?}"
        );
    }

    #[test]
    fn http_version_parse_empty() {
        HTTPVersion::from_str("").expect_err("Parsing empty strings should fail");
    }

    #[test]
    fn http_version_parse_wrong_protocol() {
        let err = HTTPVersion::from_str("TCP/3").expect_err("Parsing other protocols should fail");
        assert!(
            matches!(err, RequestParseError::NotHTTP),
            "Parsing non-HTTP strings should fail"
        );
    }

    #[test]
    fn http_version_parse_bad_version() {
        let err = HTTPVersion::from_str("HTTP/1.0").expect_err("Parsing bad versions should fail");
        assert!(
            matches!(err, RequestParseError::UnsupportedVersion(_)),
            "Parsing unsupported methods should fail"
        );
    }

    #[test]
    fn http_version_parse_no_version() {
        HTTPVersion::from_str("HTTP/").expect_err("Parsing strings without versions should fail");
    }

    #[test]
    fn http_request_parse_no_headers_no_port() {
        let content = "GET / HTTP/1.1\nHost: cheese.com";
        let request = Request::from_str(content)
            .expect("Parsing request with no headers and a host with no port should succeed");
        assert_eq!(HTTPMethod::Get, request.method);
        assert_eq!("/", request.path);
        assert_eq!(HTTPVersion::V1_1, request.http_version);
    }

    #[test]
    fn http_request_parse_no_headers_with_port() {
        let content = "GET / HTTP/1.1\nHost: cheese.com:80";
        let request = Request::from_str(content)
            .expect("Parsing request with no headers and a host with no port should succeed");
        assert_eq!(HTTPMethod::Get, request.method);
        assert_eq!("/", request.path);
        assert_eq!(HTTPVersion::V1_1, request.http_version);
    }

    #[test]
    fn http_request_parse_carriage_returns() {
        // Carriage returns are preferred by the HTTP standard
        let request = Request::from_str(r"GET / HTTP/1.1\r\nHost: cheese.com")
            .expect("Parsing a request containing carriage returns should succeed");
        assert_eq!(HTTPMethod::Get, request.method);
        assert_eq!("/", request.path);
        assert_eq!(HTTPVersion::V1_1, request.http_version);
    }

    #[test]
    fn http_request_parse_mixed_newlines() {
        let request = Request::from_str(r"GET / HTTP/1.1\r\nHost: cheese.com\n")
            .expect("Parsing a request containing mixed LF and CRLF should succeed");
        assert_eq!(HTTPMethod::Get, request.method);
        assert_eq!("/", request.path);
        assert_eq!(HTTPVersion::V1_1, request.http_version);
    }
}
