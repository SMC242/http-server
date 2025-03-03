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

    match segments.len() {
        0 => Err(RequestParseError::InvalidStartLine("")),
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

fn parse_req(req: &str) -> Result<Request, RequestParseError> {
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

    let body = match method {
        HTTPMethod::Post | HTTPMethod::Put | HTTPMethod::Patch => Some(lines.collect()),
        _ => None,
    };

    Ok(Request {
        method,
        path,
        headers,
        body,
    })
}
