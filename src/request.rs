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
    V2_2,
}

struct Request {
    method: HTTPMethod,
    path: String,
    http_version: HTTPVersion,
    headers: HTTPHeaders,
    // TODO: store the POST data. Maybe have a variant?
    //data: &'a String,
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
            "2.2" => Ok(HTTPVersion::V2_2),
            version => Err(RequestParseError::UnsupportedVersion(version.to_string())),
        }
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

impl FromStr for Request {
    type Err = RequestParseError;

    fn from_str(s: &str) -> Result<Request, RequestParseError> {
        let mut s_iter = s.chars();
        let method: String = take_until(|c| *c == ' ' && !is_newline(c), s_iter.by_ref());
        if method.is_empty() {
            return Err(RequestParseError::MissingMethod);
        }
        let parsed_method = HTTPMethod::from_str(method.as_str())?;

        let path: String = take_until(|c| *c == ' ' && !is_newline(c), s_iter.by_ref());
        if path.is_empty() {
            return Err(RequestParseError::MissingPath);
        }

        let http_version_string: String = s_iter.take_while(|c| !is_newline(c)).collect();
        if http_version_string.is_empty() {
            return Err(RequestParseError::MissingVersion);
        }
        if http_version_string.contains(' ') {
            return Err(RequestParseError::MalformedHeaders);
        }

        let http_version = HTTPVersion::from_str(&http_version_string)?;

        // TODO: parse headers
        let headers = HashMap::new();

        Ok(Request {
            method: parsed_method,
            path,
            http_version,
            headers,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::convert;

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
    fn http_version_parse_2_1() {
        let result = HTTPVersion::from_str("HTTP/2.2").expect("Parsing HTTP/2.2 should succeed");
        assert_eq!(
            HTTPVersion::V2_2,
            result,
            "Expected HTTP V2, parsed {result:?}"
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
    fn http_request_parse_no_headers() {
        let request = Request::from_str("GET / HTTP/1.1")
            .expect("Parsing request with no headers should succeed");
        assert_eq!(HTTPMethod::Get, request.method);
        assert_eq!("/", request.path);
        assert_eq!(HTTPVersion::V1_1, request.http_version);
    }
}
