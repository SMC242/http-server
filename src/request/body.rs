use std::str::Bytes;

use crate::mime::{MainMimeType, MimeType, SubMimeType};

use super::{content_type::ContentEncoding, headers::content_type::MimeParseInfo};

/// An arbitrary JSON
pub type Json = serde_json::Value;

pub fn decode_body(encoding: Vec<ContentEncoding>, body: Vec<u8>) -> Result<String, &'static str> {
    // TODO: Use flate2 and rust-brotli to decode the body
    String::from_utf8(body).or(Err("Failed to decode bytes as UTF-8"))
}

pub fn parse_body_json(parse_info: MimeParseInfo, body: &String) -> Result<Json, String> {
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
    let content: String = decode_body(parse_info.encoding, content_bytes)?;

    let actual_length = content.len();
    if actual_length != expected_length {
        return Err(format!("Content-Length ({expected_length}) is greater than the actual length ({actual_length})"));
    }

    serde_json::from_str::<Json>(content.as_str())
        .map_err(|reason| format!("Failed to decode JSON because: '{reason}'"))
}
