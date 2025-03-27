use std::{
    io::{BufReader, Cursor, Read},
    sync::{Arc, Mutex},
};

use crate::mime::{MainMimeType, MimeType, SubMimeType};
use crate::request::content_type::{ContentEncoding, MimeParseInfo};
use crate::request::types::{BodyReader, Json};

pub fn decode_body(encoding: &[ContentEncoding], body: Vec<u8>) -> Result<String, &'static str> {
    // TODO: Use flate2 and rust-brotli to decode the body
    String::from_utf8(body).or(Err("Failed to decode bytes as UTF-8"))
}

pub struct HTTP1_1BodyReader<R: Read> {
    stream: Arc<Mutex<BufReader<R>>>,
}

fn read_body<Stream: Read>(length: u64, reader: &mut BufReader<Stream>) -> Result<Vec<u8>, String> {
    let expected_length = length.try_into().expect("The server should be 64-bit");
    let mut bytes: Vec<u8> = vec![0; expected_length];

    reader
        .read_exact(&mut bytes)
        .or(Err("Could not read from stream"))?;

    let actual_length = bytes.len();
    if actual_length != expected_length {
        Err(format!("Content-Length ({expected_length}) is greater than the actual length ({actual_length})"))
    } else {
        Ok(bytes)
    }
}

impl<R: Read> HTTP1_1BodyReader<R> {
    pub fn new(reader: BufReader<R>) -> Self {
        Self {
            stream: Arc::new(Mutex::new(reader)),
        }
    }
}
impl<R: Read> BodyReader for HTTP1_1BodyReader<R> {
    fn text(&self, parse_info: &MimeParseInfo) -> Result<String, String> {
        if !matches!(
            parse_info.content_type,
            MimeType {
                main_type: MainMimeType::Text,
                ..
            },
        ) {
            return Err("Not a text document".to_string());
        }

        let mut reader = self.stream.lock().unwrap();
        let bytes = read_body(parse_info.length, &mut *reader)?;
        decode_body(&parse_info.encoding, bytes).map_err(|e| e.to_string())
    }

    fn json(&self, parse_info: &MimeParseInfo) -> Result<Json, String> {
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
        let mut reader = self.stream.lock().unwrap();
        let content_bytes = read_body(parse_info.length, &mut *reader)?;
        let content: String = decode_body(&parse_info.encoding, content_bytes)?;

        serde_json::from_str::<Json>(content.as_str())
            .map_err(|reason| format!("Failed to decode JSON because: '{reason}'"))
    }
}

// TODO: multipart parser
fn mock_stream(content: &'static str) -> Arc<Mutex<BufReader<Cursor<Vec<u8>>>>> {
    Arc::new(Mutex::new(BufReader::new(Cursor::new(content.into()))))
}

#[cfg(test)]
mod json_tests {
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

        HTTP1_1BodyReader {
            stream: mock_stream(r#"{"foo":"bar"}"#),
        }
        .json(&mime_info)
        .expect("Parsing the body should succeed");
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

        HTTP1_1BodyReader {
            stream: mock_stream(
                r#"{
  "foo": "bar",
  "baz": "qux"
}"#,
            ),
        }
        .json(&mime_info)
        .expect("Parsing a multiline JSON body should succeed");
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

        HTTP1_1BodyReader {
            stream: mock_stream(r#"{"foo":"bar"}"#),
        }
        .json(&mime_info)
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

        HTTP1_1BodyReader {
            stream: mock_stream("lol"),
        }
        .json(&incorrect_mime_info)
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

        HTTP1_1BodyReader {
            stream: mock_stream(r#"not a json"#),
        }
        .json(&correct_mime_info)
        .expect_err("Parsing a body that is not JSON as JSON should fail");
    }

    #[test]
    fn parse_empty_json() {
        let mime_info = MimeParseInfo {
            content_type: MimeType {
                main_type: MainMimeType::Application,
                sub_type: SubMimeType::JSON,
                original: "application/json".to_string(),
            },
            length: 0u64,
            boundary: None,
            charset: None,
            encoding: vec![],
        };

        HTTP1_1BodyReader {
            stream: mock_stream(r#""#),
        }
        .json(&mime_info)
        .expect_err("Parsing an empty body as JSON should fail");
    }
}

#[cfg(test)]
mod text_tests {
    use super::*;

    #[test]
    fn parse_html() {
        let mime_info = MimeParseInfo {
            content_type: MimeType {
                main_type: MainMimeType::Text,
                sub_type: SubMimeType::HTM,
                original: "text/html".to_string(),
            },
            length: 31u64,
            boundary: None,
            charset: None,
            encoding: vec![],
        };
        let result = HTTP1_1BodyReader {
            stream: mock_stream(r#"<!doctype html><title>a</title>"#),
        }
        .text(&mime_info)
        .expect("Parsing a basic HTML document should succeed");
        assert_eq!(result, "<!doctype html><title>a</title>".to_string());
    }

    #[test]
    fn parse_empty_text() {
        let mime_info = MimeParseInfo {
            content_type: MimeType {
                main_type: MainMimeType::Text,
                sub_type: SubMimeType::HTM,
                original: "text/html".to_string(),
            },
            length: 0u64,
            boundary: None,
            charset: None,
            encoding: vec![],
        };

        let result = HTTP1_1BodyReader {
            stream: mock_stream(r#""#),
        }
        .text(&mime_info)
        .expect("Parsing an empty HTML document should succeed");
        assert_eq!(result, "".to_string());
    }

    #[test]
    fn parse_nontext() {
        let mime_info = MimeParseInfo {
            content_type: MimeType {
                main_type: MainMimeType::Audio,
                sub_type: SubMimeType::MP3,
                original: "audio/mp3".to_string(),
            },
            length: 31u64,
            boundary: None,
            charset: None,
            encoding: vec![],
        };

        HTTP1_1BodyReader {
            stream: mock_stream(r#"IDK what an .mp3 file looks like"#),
        }
        .text(&mime_info)
        .expect_err("Parsing a non-text document should fail");
    }
    // TODO: add tests for encodings, charsets, and boundaries
}
