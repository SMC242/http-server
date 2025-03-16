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
struct MimeParseInfo {
    pub length: u64,
    pub boundary: Option<String>,
    pub content_type: MimeType,
    pub charset: Option<String>, // TODO: Handle decoding downstream with encoding_rs
    pub encoding: Option<ContentEncoding>,
}

struct ContentTypeInfo {
    content_type: MimeType,
    charset: Option<String>,
    boundary: Option<String>,
}

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

pub fn parse_content_type(content_type: &String) -> Result<ContentTypeInfo, RequestParseError> {
    let mut chars = content_type.chars().peekable();
    let media_type: String = chars.by_ref().take_while(|c| ';' != *c).collect();
    let mime_type = MimeType::from_str(media_type.as_str()).map_err(|_| {
        RequestParseError::InvalidHeader(format!("Invalid or unsupported MIME type {media_type}"))
    })?;

    // Parse parameters if they exist
    if chars.peek().is_none() {
        return Ok(ContentTypeInfo {
            content_type: mime_type,
            charset: None,
            boundary: None,
        });
    }

    let param_name: String = chars.by_ref().take_while(|c| '=' != *c).collect();
    // `boundaryString` and `charset` are mutually exclusive
    let (boundary, charset): (Option<String>, Option<String>) = match param_name.as_str() {
            "" => return Err(RequestParseError::InvalidHeader("Unexpected ';' in Content-Type header. ';' must be followed by either charset=... or boundaryString=...".to_string())),
            "boundarystring" => {
                if !matches!(mime_type.main_type, MainMimeType::Multipart) {return Err(RequestParseError::BodyParseError(format!(
                "boundaryString is required for multipart/* MIME types. MIME type: {0}",
                mime_type.original
            )));}
                (Some(chars.collect()), None)
            }
            "charset" => (None, Some(chars.collect())),
        other_param => return Err(RequestParseError::InvalidHeader(format!("Unexpected parameter: '{other_param}'")))
    };

    Ok(ContentTypeInfo {
        content_type: mime_type,
        charset,
        boundary,
    })
}

pub fn parse_mime_info(headers: HTTPHeaders) -> Result<MimeParseInfo, RequestParseError> {
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
        .map(|enc| ContentEncoding::from_str(enc))
        .transpose()?;
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
