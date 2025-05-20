use crate::mime::{MainMimeType, MimeType};
use crate::request::types::{HTTPHeaders, RequestParseError};
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub enum ContentEncoding {
    Gzip,
    Compress,
    Deflate,
    Br,
    Zstd,
}

#[derive(Debug)]
pub struct MimeParseInfo {
    pub length: u64,
    pub boundary: Option<String>,
    pub content_type: MimeType,
    pub charset: Option<String>, // TODO: Handle decoding downstream with encoding_rs
    pub encoding: Vec<ContentEncoding>,
}

struct ContentTypeInfo {
    content_type: MimeType,
    charset: Option<String>,
    boundary: Option<String>,
}

/// Use `parse_content_encoding` instead of calling this directly
/// because Content-Encoding headers can have multiple encodings
impl FromStr for ContentEncoding {
    type Err = RequestParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "gzip" => Ok(Self::Gzip),
            "compress" => Ok(Self::Compress),
            "deflate" => Ok(Self::Deflate),
            "br" => Ok(Self::Br),
            "zstd" => Ok(Self::Zstd),
            other => Err(Self::Err::BodyParseError(format!(
                "Invalid content encoding '{other}'"
            ))),
        }
    }
}

/// The Content-Encoding header may have a series of encodings,
/// representing a the order that encodings were applied.
/// See https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Content-Encoding
pub fn parse_content_encoding(s: &str) -> Result<Vec<ContentEncoding>, RequestParseError> {
    s.split(",")
        .map(str::trim)
        .map(ContentEncoding::from_str)
        .collect()
}

fn parse_content_type(content_type: &str) -> Result<ContentTypeInfo, RequestParseError> {
    let mut parts = content_type.split(';').peekable();
    let media_type = if let Some(mt) = parts.next() {
        mt
    } else {
        content_type
    };
    let mime_type = MimeType::from_str(media_type).map_err(|_| {
        RequestParseError::InvalidHeader(format!("Invalid or unsupported MIME type {media_type}"))
    })?;

    let (mut charset, mut boundary) = (None, None);
    for param in parts {
        let param_parts: Vec<&str> = param.split('=').collect();
        if param_parts.len() != 2 {
            return Err(RequestParseError::InvalidHeader(
                "Malformed parameter in Content-Type header".to_string(),
            ));
        }
        match param_parts[0].trim() {
            "boundaryString" => boundary = Some(param_parts[1].to_string()),
            "charset" => charset = Some(param_parts[1].to_string()),
            other_param => {
                return Err(RequestParseError::InvalidHeader(format!(
                    "Unexpected parameter: '{other_param}'"
                )))
            }
        }
    }

    if matches!(mime_type.main_type, MainMimeType::Multipart) && boundary.is_none() {
        return Err(RequestParseError::BodyParseError(format!(
            "boundaryString is required for multipart/* MIME types. MIME type: {0}",
            mime_type.original
        )));
    }

    Ok(ContentTypeInfo {
        content_type: mime_type,
        charset,
        boundary,
    })
}

pub fn parse_mime_info(headers: &HTTPHeaders) -> Result<MimeParseInfo, RequestParseError> {
    let content_length = headers
        .get("content-length")
        .ok_or(RequestParseError::BodyParseError(
            "Missing content-length".to_string(),
        ))
        .map(|len| {
            u64::from_str(len).or(Err(RequestParseError::InvalidHeader(format!(
                "{len} is not a valid integer"
            ))))
        })??;

    let encoding = headers
        .get("content-encoding")
        .map_or(Ok(vec![]), |enc| parse_content_encoding(enc))?;
    let content_type = headers
        .get("content-type")
        .ok_or(RequestParseError::BodyParseError(
            "Missing content-type".to_string(),
        ))?;

    let ContentTypeInfo {
        content_type: mime_type,
        charset,
        boundary,
    } = parse_content_type(content_type)?;

    Ok(MimeParseInfo {
        length: content_length,
        content_type: mime_type,
        boundary,
        charset,
        encoding,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_http_headers(pairs: &[(&str, &str)]) -> HTTPHeaders {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    use crate::{
        mime::SubMimeType,
        request::{content_type::parse_mime_info, HTTPHeaders},
    };

    #[test]
    fn empty_headers() {
        let headers: HTTPHeaders = new_http_headers(&[]);
        parse_mime_info(&headers)
            .expect_err("Attempting to parse the MIME info from empty headers should fail");
    }

    #[test]
    fn missing_headers() {
        parse_mime_info(&new_http_headers(&[("content-type", "application/json")]))
            .expect_err("Parsing MIME info without a Content-Length should fail");
        parse_mime_info(&new_http_headers(&[("content-length", "0")]))
            .expect_err("Parsing MIME info without a Content-Type should fail");
    }

    #[test]
    fn invalid_content_type() {
        parse_mime_info(&new_http_headers(&[
            ("content-type", ""),
            ("content-length", "0"),
        ]))
        .expect_err("Parsing MIME info with an empty Content-Type should fail");
        parse_mime_info(&new_http_headers(&[
            ("content-type", "application/fakesubtype"),
            ("content-length", "0"),
        ]))
        .expect_err("Parsing MIME info with a fake main MIME type should fail");
        parse_mime_info(&new_http_headers(&[
            ("content-type", "fakemaintype/html"),
            ("content-length", "0"),
        ]))
        .expect_err("Parsing MIME info with a fake MIME subtype should fail");
    }

    #[test]
    fn invalid_content_length() {
        parse_mime_info(&new_http_headers(&[
            ("content-type", "text/html"),
            ("content-length", "-1"),
        ]))
        .expect_err("Parsing negative Content-Lengths should fail");

        parse_mime_info(&new_http_headers(&[
            ("content-type", "text/html"),
            ("content-length", "afkajaffjej"),
        ]))
        .expect_err("Parsing string Content-Lengths should fail");

        parse_mime_info(&new_http_headers(&[
            ("content-type", "text/html"),
            ("content-length", "6.0"),
        ]))
        .expect_err("Parsing string decimal Content-Lengths should fail");
    }

    #[test]
    fn varying_content_lengths() {
        parse_mime_info(&new_http_headers(&[
            ("content-type", "text/html"),
            ("content-length", "1"),
        ]))
        .expect("Parsing Content-Length = 1 should succeed");

        parse_mime_info(&new_http_headers(&[
            ("content-type", "text/html"),
            ("content-length", "128"),
        ]))
        .expect("Parsing Content-Length = 128 should succeed");

        parse_mime_info(&new_http_headers(&[
            ("content-type", "text/html"),
            ("content-length", "4000000000"),
        ]))
        .expect("Parsing Content-Length = 4 billion should succeed");
    }

    #[test]
    fn normal_content_type_and_length() {
        let MimeParseInfo {
            content_type,
            length,
            encoding,
            ..
        } = parse_mime_info(&new_http_headers(&[
            ("content-type", "audio/ogg"),
            ("content-length", "1024"),
        ]))
        .expect("Parsing Content-Type = audio/ogg, Content-Length = 1024 should succeed");
        assert_eq!(
            content_type,
            MimeType {
                main_type: MainMimeType::Audio,
                sub_type: SubMimeType::OGA,
                original: "audio/ogg".to_string()
            },
            "Should be audio/ogg"
        );
        assert_eq!(length, 1024u64, "Should be 1024");
        assert_eq!(
            encoding,
            vec![],
            "The encodings should be set to the empty vector by default"
        )
    }

    #[test]
    fn with_boundary() {
        let MimeParseInfo {
            content_type,
            length,
            boundary,
            charset,
            ..
        } = parse_mime_info(&new_http_headers(&[
            ("content-type", "multipart/form-data;boundaryString=---------------------------1003363413119651595289485765"),
            ("content-length", "1024"),
        ]))
        .expect("Parsing Content-Type = multipart/form-data, Content-Length = 1024, with boundaryString should succeed");
        assert_eq!(
            content_type,
            MimeType {
                main_type: MainMimeType::Multipart,
                sub_type: SubMimeType::FormData,
                original: "multipart/form-data".to_string()
            },
            "Should be multipart/form-data"
        );
        assert_eq!(length, 1024u64);
        assert_eq!(
            boundary,
            Some("---------------------------1003363413119651595289485765".to_string())
        );
        assert!(
            charset.is_none(),
            "charset and boundaryString are mutually exclusive"
        );
    }

    #[test]
    fn with_charset() {
        let MimeParseInfo {
            content_type,
            length,
            charset,
            boundary,
            ..
        } = parse_mime_info(&new_http_headers(&[
            ("content-type", "text/html;charset=utf-8"),
            ("content-length", "1024"),
        ]))
        .expect(
            "Parsing Content-Type = text/html Content-Length = 1024, with charset should succeed",
        );
        assert_eq!(
            content_type,
            MimeType {
                main_type: MainMimeType::Text,
                sub_type: SubMimeType::HTM,
                original: "text/html".to_string()
            },
            "Should be text/html"
        );
        assert_eq!(length, 1024u64);
        assert_eq!(charset, Some("utf-8".to_string()));
        assert!(
            boundary.is_none(),
            "charset and boundaryString are mutually exclusive"
        );
    }

    #[test]
    fn with_encoding() {
        let MimeParseInfo {
            content_type,
            length,
            encoding,
            ..
        } = parse_mime_info(&new_http_headers(&[
            ("content-type", "video/mp4"),
            ("content-length", "1024"),
            ("content-encoding", "compress")
        ]))
        .expect("Parsing Content-Type = video/mp4, Content-Length = 1024, Content-Encoding = compress should succeed");
        assert_eq!(
            content_type,
            MimeType {
                main_type: MainMimeType::Video,
                sub_type: SubMimeType::MP4,
                original: "video/mp4".to_string()
            },
            "Should be video/mp4"
        );
        assert_eq!(length, 1024u64);
        assert_eq!(encoding, vec![ContentEncoding::Compress]);
    }

    #[test]
    fn with_multiple_encodings() {
        // NOTE: the inconsistent whitespace in Content-Encoding is to
        // check that the parser is whitespace-tolerant
        let MimeParseInfo {
            content_type,
            length,
            encoding,
            ..
        } = parse_mime_info(&new_http_headers(&[
            ("content-type", "video/mp4"),
            ("content-length", "1024"),
            ("content-encoding", "compress,deflate, gzip"),
        ]))
        .expect("Parsing with multiple Content-Encodings should succeed");
        assert_eq!(
            content_type,
            MimeType {
                main_type: MainMimeType::Video,
                sub_type: SubMimeType::MP4,
                original: "video/mp4".to_string()
            },
            "Should be video/mp4"
        );
        assert_eq!(length, 1024u64);
        assert_eq!(
            encoding,
            vec![
                ContentEncoding::Compress,
                ContentEncoding::Deflate,
                ContentEncoding::Gzip
            ]
        );
    }

    #[test]
    fn with_charset_and_encoding() {
        let MimeParseInfo {
            content_type,
            charset,
            boundary,
            ..
        } = parse_mime_info(&new_http_headers(&[
            (
                "content-type",
                "multipart/form-data; charset=UTF-8; boundaryString=aba",
            ),
            ("content-length", "1024"),
        ]))
        .expect("Parsing a Content-Type with a boundaryString and charset should succeed");
        assert_eq!(
            content_type,
            MimeType {
                main_type: MainMimeType::Multipart,
                sub_type: SubMimeType::FormData,
                original: "multipart/form-data".to_string()
            },
            "Should be multipart/form-data"
        );
        assert_eq!(charset, Some("UTF-8".to_string()));
        assert_eq!(boundary, Some("aba".to_string()));

        // test order
        let MimeParseInfo {
            content_type,
            charset,
            boundary,
            ..
        } = parse_mime_info(&new_http_headers(&[
            (
                "content-type",
                "multipart/form-data; boundaryString=aba; charset=UTF-8",
            ),
            ("content-length", "1024"),
        ]))
        .expect("Parsing a Content-Type with a boundaryString and charset should succeed");
        assert_eq!(
            content_type,
            MimeType {
                main_type: MainMimeType::Multipart,
                sub_type: SubMimeType::FormData,
                original: "multipart/form-data".to_string()
            },
            "Should be multipart/form-data"
        );
        assert_eq!(
            charset,
            Some("UTF-8".to_string()),
            "Parameter order should not matter"
        );
        assert_eq!(
            boundary,
            Some("aba".to_string()),
            "Parameter order should not matter"
        );
    }
}
