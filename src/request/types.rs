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
pub struct RequestHead {
    pub method: HTTPMethod,
    pub path: Path,
    pub version: HTTPVersion,
    pub headers: HTTPHeaders,
}

pub type RequestBody = Option<String>;

pub struct Request {
    head: RequestHead,
    body: RequestBody,
}

#[derive(Debug, PartialEq)]
pub enum RequestParseError {
    InvalidStartLine(&'static str),
    InvalidHeader(String),
    MissingHostHeader, // HTTP 1.1 requires the Host header to be set
    BodyParseError(String),
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
        println!("Path provided was: {0}", path);
        if path.starts_with('/') {
            Ok(Path::OriginForm(path.to_string()))
        }
        // TODO: support HTTPS
        else if path.starts_with("http://") {
            Ok(Path::AbsoluteForm(path.to_string()))
        } else if path.contains(':') {
            let mut parts = path.splitn(2, ':');
            match (parts.next(), parts.next()) {
                (Some(domain), Some(port)) => {
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
            Self::BodyParseError(reason) => format!("Could not parse body: {reason}"),
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
        HTTPVersion::from_str("TCP/1.1").expect_err("Parsing other protocols should fail");
    }

    #[test]
    fn http_version_parse_future_version() {
        let err =
            HTTPVersion::from_str("HTTP/4.0").expect_err("Parsing future versions should fail");
        assert!(
            matches!(err, RequestParseError::UnsupportedVersion(_)),
            "Parsing unsupported methods should fail"
        );
    }
    #[test]
    fn http_version_parse_bad_version() {
        HTTPVersion::from_str("HTTP/0.0").expect_err("Parsing 0.0 should fail");
        HTTPVersion::from_str("HTTP/1.5").expect_err("Parsing 1.5 should fail");
        HTTPVersion::from_str("HTTP/-420.0").expect_err("Parsing -420 should fail");
    }

    #[test]
    fn http_version_parse_no_version() {
        HTTPVersion::from_str("HTTP/").expect_err("Parsing strings without versions should fail");
    }

    #[test]
    fn path_parse_origin_form() {
        assert_eq!(
            Path::OriginForm("/".to_string()),
            Path::from_str("/").expect("Parsing / should succeed")
        );

        assert_eq!(
            Path::OriginForm("/echo/falls/spring".to_string()),
            Path::from_str("/echo/falls/spring")
                .expect("Parsing a nested origin-form path should succeed")
        );
    }

    #[test]
    fn path_parse_origin_form_file_extension() {
        assert_eq!(
            Path::OriginForm("/spring.html".to_string()),
            Path::from_str("/spring.html")
                .expect("Parsing an origin-form path with a file extension should succeed")
        );
    }

    #[test]
    fn path_parse_empty() {
        Path::from_str("").expect_err("Parsing an empty string should fail");
    }

    #[test]
    fn path_parse_absolute_form() {
        assert_eq!(
            Path::AbsoluteForm("http://example.com".to_string()),
            Path::from_str("http://example.com")
                .expect("Parsing http://example.com should succeed")
        );
    }

    #[test]
    fn path_parse_absolute_form_with_path() {
        assert_eq!(
            Path::AbsoluteForm("http://example.com/about".to_string()),
            Path::from_str("http://example.com/about")
                .expect("Parsing an absolute-form host with a path should succeed")
        );

        assert_eq!(
            Path::AbsoluteForm("http://example.com/about/my/entire/life-story".to_string()),
            Path::from_str("http://example.com/about/my/entire/life-story")
                .expect("Parsing an absolute-form host with a deeply-nested path should succeed")
        );
    }

    #[test]
    fn path_parse_authority_form() {
        assert_eq!(
            Path::AuthorityForm("mozilla.org".to_string(), 80),
            Path::from_str("mozilla.org:80")
                .expect("Parsing mozilla.org:80 as the authority should succeed")
        );
    }

    #[test]
    fn path_parse_authority_form_with_subdomain() {
        assert_eq!(
            Path::AuthorityForm("developer.mozilla.org".to_string(), 80),
            Path::from_str("developer.mozilla.org:80")
                .expect("Parsing developer.mozilla.org:80 as the authority should succeed")
        );

        assert_eq!(
            Path::AuthorityForm("highly.specific.place.in.a.domain.org".to_string(), 80),
            Path::from_str("highly.specific.place.in.a.domain.org:80")
                .expect("Parsing a deeply-nested domain as the authority should succeed")
        );
    }

    #[test]
    fn path_parse_asterisk_form() {
        assert_eq!(
            Path::Asterisk,
            Path::from_str("*").expect("Parsing * should succeed")
        );
    }

    #[test]
    fn path_parse_garbage() {
        Path::from_str("aghajgaajagkajakaj").expect_err("Parsing garbage strings should fail");
    }

    #[test]
    fn method_parse() {
        assert_eq!(
            HTTPMethod::Get,
            HTTPMethod::from_str("GET").expect("Parsing GET should succeed")
        );

        assert_eq!(
            HTTPMethod::Post,
            HTTPMethod::from_str("POST").expect("Parsing POST should succeed")
        );

        assert_eq!(
            HTTPMethod::Put,
            HTTPMethod::from_str("PUT").expect("Parsing PUT should succeed")
        );

        assert_eq!(
            HTTPMethod::Patch,
            HTTPMethod::from_str("PATCH").expect("Parsing PATCH should succeed")
        );

        assert_eq!(
            HTTPMethod::Delete,
            HTTPMethod::from_str("DELETE").expect("Parsing DELETE should succeed")
        );

        assert_eq!(
            HTTPMethod::Connect,
            HTTPMethod::from_str("CONNECT").expect("Parsing CONNECT should succeed")
        );

        assert_eq!(
            HTTPMethod::Options,
            HTTPMethod::from_str("OPTIONS").expect("Parsing OPTIONS should succeed")
        );

        assert_eq!(
            HTTPMethod::Trace,
            HTTPMethod::from_str("TRACE").expect("Parsing TRACE should succeed")
        );

        assert_eq!(
            HTTPMethod::Head,
            HTTPMethod::from_str("HEAD").expect("Parsing HEAD should succeed")
        );
    }
}
