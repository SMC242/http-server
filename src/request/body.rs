use crate::mime::{MainMimeType, MimeType, SubMimeType};

use super::{content_type::ContentEncoding, headers::content_type::MimeParseInfo};

/// An arbitrary JSON
pub type Json = serde_json::Value;

pub fn decode_body(encoding: &[ContentEncoding], body: Vec<u8>) -> Result<String, &'static str> {
    // TODO: Use flate2 and rust-brotli to decode the body
    String::from_utf8(body).or(Err("Failed to decode bytes as UTF-8"))
}

pub fn parse_body_json(parse_info: &MimeParseInfo, body: &str) -> Result<Json, String> {
    if !matches!(
        parse_info.content_type,
        MimeType {
            main_type: MainMimeType::Application,
            sub_type: SubMimeType::JSON,
            ..
        },
    ) {
        return Err("Not JSON".to_string());
    }

    // FIXME: this assumes that the charset is UTF-8. Use encoding_rs to decode first
    let expected_length = parse_info
        .length
        .try_into()
        .expect("The server should be 64-bit");
    let content_bytes: Vec<u8> = body.bytes().take(expected_length).collect();
    let content: String = decode_body(&parse_info.encoding, content_bytes)?;

    let actual_length = content.len();
    if actual_length != expected_length {
        return Err(format!("Content-Length ({expected_length}) is greater than the actual length ({actual_length})"));
    }

    serde_json::from_str::<Json>(content.as_str())
        .map_err(|reason| format!("Failed to decode JSON because: '{reason}'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_json_plaintext() {
        let mime_info = MimeParseInfo {
            content_type: MimeType {
                main_type: MainMimeType::Application,
                sub_type: SubMimeType::JSON,
                original: "application/json".to_string(),
            },
            length: 13u64,
            boundary: None,
            charset: None,
            encoding: vec![],
        };

        let body = r#"{"foo":"bar"}"#;

        parse_body_json(&mime_info, body).expect("Parsing the body should succeed");
    }

    #[test]
    fn parse_multiline_json() {
        let mime_info = MimeParseInfo {
            content_type: MimeType {
                main_type: MainMimeType::Application,
                sub_type: SubMimeType::JSON,
                original: "application/json".to_string(),
            },
            length: 34u64,
            boundary: None,
            charset: None,
            encoding: vec![],
        };

        let body = r#"{
  "foo": "bar",
  "baz": "qux"
}"#;

        parse_body_json(&mime_info, body).expect("Parsing a multiline JSON body should succeed");
    }

    #[test]
    fn parse_json_incorrect_length() {
        let mime_info = MimeParseInfo {
            content_type: MimeType {
                main_type: MainMimeType::Application,
                sub_type: SubMimeType::JSON,
                original: "application/json".to_string(),
            },
            length: 10u64,
            boundary: None,
            charset: None,
            encoding: vec![],
        };

        let body = r#"{"foo":"bar"}"#;

        parse_body_json(&mime_info, body)
            .expect_err("An error should be thrown when the Content-Length is wrong");
    }

    #[test]
    fn parse_json_not_json() {
        let incorrect_mime_info = MimeParseInfo {
            content_type: MimeType {
                main_type: MainMimeType::Font,
                sub_type: SubMimeType::TTF,
                original: "font/ttf".to_string(),
            },
            length: 3u64,
            boundary: None,
            charset: None,
            encoding: vec![],
        };

        parse_body_json(&incorrect_mime_info, "lol")
            .expect_err("Calling parse_body_json when the MIME type is not JSON should fail");

        let correct_mime_info = MimeParseInfo {
            content_type: MimeType {
                main_type: MainMimeType::Application,
                sub_type: SubMimeType::JSON,
                original: "application/json".to_string(),
            },
            length: 10u64,
            boundary: None,
            charset: None,
            encoding: vec![],
        };

        parse_body_json(&correct_mime_info, r#"not a json"#)
            .expect_err("Parsing a body that is not JSON as JSON should fail");
    }

    // TODO: add tests for encodings, charsets, and boundaries
}
