use std::collections::HashMap;

type HTTPHeaders = HashMap<String, String>;

// TODO: add more methods
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

#[derive(Debug)]
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

impl TryFrom<String> for HTTPVersion {
    type Error = RequestParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(RequestParseError::MissingVersion);
        }

        let mut iter = value.chars();
        let protocol: String = iter
            .by_ref()
            .take_while(|c| *c != '/' && *c != '\r')
            .collect();
        if protocol.is_empty() {
            return Err(RequestParseError::MissingVersion);
        }
        if protocol != "HTTP" {
            return Err(RequestParseError::NotHTTP);
        }

        match iter
            .take_while(|c| !is_newline(c))
            .collect::<String>()
            .as_str()
        {
            "" => Err(RequestParseError::MissingVersion),
            "1.1" => Ok(HTTPVersion::V1_1),
            "2.2" => Ok(HTTPVersion::V2_2),
            version => Err(RequestParseError::UnsupportedVersion(version.to_string())),
        }
    }
}

//impl TryFrom<String> for Request {
//    type Error = RequestParseError;
//
//    fn try_from(s: String) -> Result<Request, RequestParseError> {
//        let s_iter = s.chars();
//        let method: String = s_iter.take_while(|c| *c != ' ' && !is_newline(c)).collect();
//        if method.is_empty() {
//            return Err(RequestParseError::MissingMethod);
//        }
//        s_iter.next();
//
//        let path: String = s_iter.take_while(|c| *c != ' ' && !is_newline(c)).collect();
//        if path.is_empty() {
//            return Err(RequestParseError::MissingPath);
//        }
//        s_iter.next();
//
//        let http_version_string: String = s_iter.take_while(|c| !is_newline(c)).collect();
//        if http_version_string.is_empty() {
//            return Err(RequestParseError::MissingVersion);
//        }
//        if http_version_string.contains(' ') {
//            return Err(RequestParseError::MalformedHeaders);
//        }
//
//        let http_version: HTTPVersion = http_version_string.try_into()?;
//
//        // TODO: parse headers
//        let headers = HashMap::new();
//
//        Ok(Request {
//            method: method,
//            path: path,
//            http_version: http_version,
//            headers: headers,
//        })
//    }
//}

#[cfg(test)]
mod tests {
    use std::convert;

    use super::*;

    #[test]
    fn test_http_version_parse_1_1() {
        let result = convert::TryInto::try_into("HTTP/1.1".to_string())
            .expect("Parsing HTTP/1.1 should succeed");
        assert_eq!(
            HTTPVersion::V1_1,
            result,
            "Expected HTTP V1, parsed {result:?}"
        );
    }

    #[test]
    fn test_http_version_parse_2_1() {
        let result = convert::TryInto::try_into("HTTP/2.2".to_string())
            .expect("Parsing HTTP/2.2 should succeed");
        assert_eq!(
            HTTPVersion::V2_2,
            result,
            "Expected HTTP V2, parsed {result:?}"
        );
    }

    #[test]
    fn test_http_version_parse_empty() {
        convert::TryInto::<HTTPVersion>::try_into("".to_string())
            .expect_err("Parsing empty strings should fail");
    }

    #[test]
    fn test_http_version_parse_wrong_protocol() {
        let err = convert::TryInto::<HTTPVersion>::try_into("TCP/3".to_string())
            .expect_err("Parsing other protocols should fail");
        assert!(
            matches!(err, RequestParseError::NotHTTP),
            "Parsing non-HTTP strings should fail"
        );
    }

    #[test]
    fn test_http_version_parse_bad_version() {
        let err = convert::TryInto::<HTTPVersion>::try_into("HTTP/1.0".to_string())
            .expect_err("Parsing bad versions should fail");
        assert!(
            matches!(err, RequestParseError::UnsupportedVersion(_)),
            "Parsing unsupported methods should fail"
        );
    }

    #[test]
    fn test_http_version_parse_no_version() {
        convert::TryInto::<HTTPVersion>::try_into("HTTP/".to_string())
            .expect_err("Parsing strings without versions should fail");
    }
}
