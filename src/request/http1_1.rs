use super::types::*;
use std::{collections::HashMap, str::FromStr};

struct StartLine {
    method: HTTPMethod,
    path: Path,
    version: HTTPVersion,
}

fn parse_start_line(line: &str) -> Result<StartLine, RequestParseError> {
    let segments: Vec<&str> = line.split(' ').take(3).collect();
    let parse_method = |m| {
        HTTPMethod::from_str(m).map_err(|_| RequestParseError::InvalidStartLine("Invalid method"))
    };
    let parse_path =
        |p| Path::from_str(p).or(Err(RequestParseError::InvalidStartLine("Invalid path")));

    println!("Segments: {0:?}", segments);
    match segments.len() {
        0 => Err(RequestParseError::InvalidStartLine("Empty")),
        1 => Err(RequestParseError::InvalidStartLine("Too few segments")),
        // The HTTP/{version} segment was introduced in HTTP 1.0
        2 => Ok(StartLine {
            method: parse_method(segments[0])?,
            path: parse_path(segments[1])?,
            version: HTTPVersion::V0_9,
        }),
        3 => {
            let version = HTTPVersion::from_str(segments[2]).or(Err(
                RequestParseError::InvalidStartLine("Invalid HTTP version"),
            ))?;

            Ok(StartLine {
                method: parse_method(segments[0])?,
                path: parse_path(segments[1])?,
                version,
            })
        }
        4.. => Err(RequestParseError::InvalidStartLine("Too many segments")),
    }
}

fn parse_headers<'a, I: Iterator<Item = &'a str>>(
    lines: &mut I,
) -> Result<HTTPHeaders, RequestParseError> {
    let mut headers = HashMap::new();
    for (line_no, line) in lines.enumerate() {
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(RequestParseError::InvalidHeader(line_no.to_string()));
        }

        // Headers must be case-insensitive
        headers.insert(parts[0].to_lowercase().to_string(), parts[1].to_string());
    }

    Ok(headers)
}

pub fn parse_req(req: &str) -> Result<Request, RequestParseError> {
    let mut lines = req.lines();

    let StartLine {
        method,
        path,
        version,
    } = lines
        .next()
        .map(parse_start_line)
        .ok_or(RequestParseError::InvalidStartLine("Missing start line"))??;

    let mut header_lines = lines.by_ref().take_while(|line| !line.is_empty());
    let headers: HTTPHeaders = parse_headers(&mut header_lines)?;

    // HTTP/1.1 requires a Host header
    if version == HTTPVersion::V1_1 {
        headers
            .get("host")
            .ok_or(RequestParseError::MissingHostHeader)?;
    }
    // TODO: validate host

    let body = match method {
        HTTPMethod::Post | HTTPMethod::Put | HTTPMethod::Patch => Some(lines.collect()),
        _ => None,
    };

    Ok(Request {
        method,
        path,
        version,
        headers,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_request_v0_9() {
        let request = parse_req("GET /\n").expect("Parsing an HTTP/0.9 request should succeed");
        assert_eq!(HTTPMethod::Get, request.method);
        assert_eq!(Path::OriginForm("/".to_string()), request.path,);
        assert_eq!(HTTPVersion::V0_9, request.version);
    }

    #[test]
    fn http_request_v1_0() {
        let request =
            parse_req("GET / HTTP/1.0\n").expect("Parsing an HTTP/1.0 request should succeed");
        assert_eq!(HTTPMethod::Get, request.method);
        assert_eq!(Path::OriginForm("/".to_string()), request.path,);
        assert_eq!(HTTPVersion::V1_0, request.version);
    }

    #[test]
    fn http_request_with_host() {
        let request = parse_req("GET / HTTP/1.1\nHost: example.com\n")
            .expect("Parsing a request with an origin-form path should succeed");
        assert_eq!(HTTPMethod::Get, request.method);
        assert_eq!(
            Path::OriginForm("/".to_string()),
            request.path,
            "Should be origin form, got {0:?}",
            request.path
        );
        assert_eq!(HTTPVersion::V1_1, request.version);

        let request2 = parse_req("CONNECT cheese.com:80 HTTP/1.1\nHost: example.com\n")
            .expect("Parsing a CONNECT request with an authority-form path should succeed");
        assert_eq!(HTTPMethod::Connect, request2.method);
        assert_eq!(
            Path::AuthorityForm("cheese.com".to_string(), 80),
            request2.path,
            "Should be authority form, got {0:?}",
            request2.path
        );
        assert_eq!(HTTPVersion::V1_1, request2.version);

        let request3 = parse_req("GET http://example.com HTTP/1.1\nHost: example.com\n")
            .expect("Parsing a request with an absolute-form path should succeed");
        assert_eq!(HTTPMethod::Get, request3.method);
        assert_eq!(
            Path::AbsoluteForm("http://example.com".to_string()),
            request3.path,
            "Should be absolute form, got {0:?}",
            request3.path
        );
        assert_eq!(HTTPVersion::V1_1, request.version);

        let request4 = parse_req("OPTIONS * HTTP/1.1\nHost: example.com\n")
            .expect("Parsing an OPTIONS request with an asterisk path should succeed");
        assert_eq!(HTTPMethod::Options, request4.method);
        assert_eq!(
            Path::Asterisk,
            request4.path,
            "Should be asterisk form, got {0:?}",
            request4.path
        );
        assert_eq!(HTTPVersion::V1_1, request4.version);
    }

    #[test]
    fn http_request_parse_newlines() {
        // Carriage returns are preferred by the HTTP standard but newlines are OK
        let request = parse_req("GET / HTTP/1.1\nHost: cheese.com\n")
            .expect("Parsing a request containing LFs should succeed");
        assert_eq!(HTTPMethod::Get, request.method);
        assert_eq!(Path::OriginForm("/".to_string()), request.path);
        assert_eq!(HTTPVersion::V1_1, request.version);
    }

    #[test]
    fn http_request_parse_carriage_returns() {
        // Carriage returns are preferred by the HTTP standard
        let request = parse_req("GET / HTTP/1.1\r\nHost: cheese.com\n")
            .expect("Parsing a request containing carriage returns should succeed");
        assert_eq!(HTTPMethod::Get, request.method);
        assert_eq!(Path::OriginForm("/".to_string()), request.path);
        assert_eq!(HTTPVersion::V1_1, request.version);
    }
}
