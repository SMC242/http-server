use regex::Regex;
use std::char::ToUppercase;
use std::fmt::Write as _;
use std::io::{Error as IoError, Write};
use std::{borrow::Cow, fmt::Display};

use crate::request::{HTTPHeaders, HTTPVersion, Request, RequestHead, SyncableStream};

// See https://stackoverflow.com/a/36928678
// Generated from en.wikipedia.org/wiki/List_of_HTTP_status_codes
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ResponseStatus {
    Continue,
    SwitchingProtocols,
    Processing,
    EarlyHints,
    OK,
    Created,
    Accepted,
    NonAuthoritativeInformation,
    NoContent,
    ResetContent,
    PartialContent,
    MultiStatus,
    AlreadyReported,
    IMUsed,
    MultipleChoices,
    MovedPermanently,
    Found,
    SeeOther,
    NotModified,
    UseProxy,
    Unused,
    TemporaryRedirect,
    PermanentRedirect,
    BadRequest,
    Unauthorized,
    PaymentRequired,
    Forbidden,
    NotFound,
    MethodNotAllowed,
    NotAcceptable,
    ProxyAuthenticationRequired,
    RequestTimeout,
    Conflict,
    Gone,
    LengthRequired,
    PreconditionFailed,
    ContentTooLarge,
    URITooLong,
    UnsupportedMediaType,
    RangeNotSatisfiable,
    ExpectationFailed,
    Imateapot,
    MisdirectedRequest,
    UnprocessableContent,
    Locked,
    FailedDependency,
    TooEarly,
    UpgradeRequired,
    PreconditionRequired,
    TooManyRequests,
    RequestHeaderFieldsTooLarge,
    UnavailableForLegalReasons,
    InternalServerError,
    NotImplemented,
    BadGateway,
    ServiceUnavailable,
    GatewayTimeout,
    HTTPVersionNotSupported,
    VariantAlsoNegotiates,
    InsufficientStorage,
    LoopDetected,
    NotExtended,
    NetworkAuthenticationRequired,
    /// For non-standard status codes such as "521 Web Server Is Down"
    /// See https://en.wikipedia.org/wiki/List_of_HTTP_status_codes#Unofficial_codes
    NonStandard(u16, String),
}

/// Converts PascalCase to TitleCase
fn unpascal_case(s: &str) -> Cow<'_, str> {
    let regex = Regex::new("([a-z])([A-Z])").expect("The regex should compile");
    regex.replace_all(s, "$1 $2")
}

impl Display for ResponseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Most names can just be un-pascal-cased but there are exceptions (E.G hyphenated or
        // containing apostrophes)
        let s: String = match self {
            Self::NonAuthoritativeInformation => "Non-Authoritative Information".to_string(),
            Self::MultiStatus => "Mutli-Status".to_string(),
            Self::Imateapot => "I'm A Teapot".to_string(),
            Self::NonStandard(code, name) => format!("{code} {name}"),
            pascal_cased => unpascal_case(&format!("{pascal_cased:?}")).to_string(),
        };

        write!(f, "{s}")
    }
}

impl PartialOrd for ResponseStatus {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ResponseStatus {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let (c1, c2) = (self.to_code(), other.to_code());
        c1.cmp(&c2)
    }
}
impl ResponseStatus {
    // Use https://stackoverflow.com/a/28029279
    pub fn is_ok(&self) -> bool {
        let code = self.to_code();
        (200..=300).contains(&code)
    }

    pub fn to_code(&self) -> u16 {
        match self {
            Self::Continue => 100,
            Self::SwitchingProtocols => 101,
            Self::Processing => 102,
            Self::EarlyHints => 103,
            Self::OK => 200,
            Self::Created => 201,
            Self::Accepted => 202,
            Self::NonAuthoritativeInformation => 203,
            Self::NoContent => 204,
            Self::ResetContent => 205,
            Self::PartialContent => 206,
            Self::MultiStatus => 207,
            Self::AlreadyReported => 208,
            Self::IMUsed => 226,
            Self::MultipleChoices => 300,
            Self::MovedPermanently => 301,
            Self::Found => 302,
            Self::SeeOther => 303,
            Self::NotModified => 304,
            Self::UseProxy => 305,
            Self::Unused => 306,
            Self::TemporaryRedirect => 307,
            Self::PermanentRedirect => 308,
            Self::BadRequest => 400,
            Self::Unauthorized => 401,
            Self::PaymentRequired => 402,
            Self::Forbidden => 403,
            Self::NotFound => 404,
            Self::MethodNotAllowed => 405,
            Self::NotAcceptable => 406,
            Self::ProxyAuthenticationRequired => 407,
            Self::RequestTimeout => 408,
            Self::Conflict => 409,
            Self::Gone => 410,
            Self::LengthRequired => 411,
            Self::PreconditionFailed => 412,
            Self::ContentTooLarge => 413,
            Self::URITooLong => 414,
            Self::UnsupportedMediaType => 415,
            Self::RangeNotSatisfiable => 416,
            Self::ExpectationFailed => 417,
            Self::Imateapot => 418,
            Self::MisdirectedRequest => 421,
            Self::UnprocessableContent => 422,
            Self::Locked => 423,
            Self::FailedDependency => 424,
            Self::TooEarly => 425,
            Self::UpgradeRequired => 426,
            Self::PreconditionRequired => 428,
            Self::TooManyRequests => 429,
            Self::RequestHeaderFieldsTooLarge => 431,
            Self::UnavailableForLegalReasons => 451,
            Self::InternalServerError => 500,
            Self::NotImplemented => 501,
            Self::BadGateway => 502,
            Self::ServiceUnavailable => 503,
            Self::GatewayTimeout => 504,
            Self::HTTPVersionNotSupported => 505,
            Self::VariantAlsoNegotiates => 506,
            Self::InsufficientStorage => 507,
            Self::LoopDetected => 508,
            Self::NotExtended => 510,
            Self::NetworkAuthenticationRequired => 511,
            Self::NonStandard(code, _) => *code,
        }
    }
}

pub struct ResponseBuilder {
    version: Option<HTTPVersion>,
    status: Option<ResponseStatus>,
    headers: Option<HTTPHeaders>,
    body: Option<String>,
    stream: Option<Box<dyn SyncableStream>>,
}

impl std::fmt::Debug for ResponseBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResponseBuilder")
            .field("version", &self.version)
            .field("status", &self.status)
            .field("headers", &self.headers)
            .field("body", &self.body)
            .field("stream", &self.stream.as_ref().map(|s| s.get_type()))
            .finish()
    }
}

impl ResponseBuilder {
    pub fn version(mut self, version: HTTPVersion) -> Self {
        self.version = Some(version);
        self
    }

    pub fn status(mut self, status: ResponseStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn headers(mut self, headers: HTTPHeaders) -> Self {
        self.headers = Some(
            headers
                .into_iter()
                .map(|(k, v)| (k.to_lowercase(), v))
                .collect(),
        );
        self
    }

    pub fn body(mut self, body: String) -> Self {
        self.body = Some(body);
        self
    }

    pub fn stream(mut self, stream: Box<dyn SyncableStream>) -> Self {
        self.stream = Some(stream);
        self
    }

    pub fn build(self) -> Result<Response, &'static str> {
        Ok(Response::new(
            self.version
                .ok_or("Can't construct a Response without a version")?,
            self.status
                .ok_or("Can't construct a Response without a status")?,
            self.headers.unwrap_or_default(),
            self.body.unwrap_or_default(),
            self.stream
                .ok_or("Can't construct a Response without a stream")?,
        ))
    }

    /// Helper method to set a header
    /// NOTE: will overwrite headers
    pub fn header(mut self, key: &str, value: &str) -> Self {
        let h = self.headers.get_or_insert(HTTPHeaders::default());
        h.entry(key.to_lowercase()).insert_entry(value.to_string());
        self
    }

    /// A helper method to set the status to 200 OK
    pub fn ok(mut self) -> Self {
        self.status = Some(ResponseStatus::OK);
        self
    }

    /// A helper method to set the status to 400 Bad Request
    pub fn bad_request(mut self) -> Self {
        self.status = Some(ResponseStatus::BadRequest);
        self
    }

    /// A helper method to set the status to 403 Unauthorized
    pub fn unauthorised(mut self) -> Self {
        self.status = Some(ResponseStatus::Unauthorized);
        self
    }

    /// A helper method to set the status to 404 Not Found
    pub fn not_found(mut self) -> Self {
        self.status = Some(ResponseStatus::NotFound);
        self
    }

    /// A helper method to set the status to 503 Internal Server Error
    pub fn internal_error(mut self) -> Self {
        self.status = Some(ResponseStatus::InternalServerError);
        self
    }
}

impl Default for ResponseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Request> for ResponseBuilder {
    fn from(value: Request) -> Self {
        let Request {
            head: RequestHead { version, .. },
            ..
        } = value;
        let stream = value.into_stream();
        ResponseBuilder::default().version(version).stream(stream)
    }
}

pub struct Response {
    pub version: HTTPVersion,
    pub status: ResponseStatus,
    pub headers: HTTPHeaders,
    pub body: String,
    stream: Box<dyn SyncableStream>,
}

impl std::fmt::Debug for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Response")
            .field("version", &self.version)
            .field("status", &self.status)
            .field("headers", &self.headers)
            .field("body", &self.body)
            .field("stream", &self.stream.get_type())
            .finish()
    }
}

impl Response {
    pub fn new(
        version: HTTPVersion,
        status: ResponseStatus,
        headers: HTTPHeaders,
        body: String,
        stream: Box<dyn SyncableStream>,
    ) -> Self {
        let mut obj = Self {
            version,
            status,
            headers,
            body,
            stream,
        };
        ensure_headers(&mut obj);
        obj
    }

    pub fn version(&self) -> HTTPVersion {
        self.version
    }

    pub fn status(&self) -> &ResponseStatus {
        &self.status
    }

    pub fn headers(&self) -> &HTTPHeaders {
        &self.headers
    }

    pub fn body(&self) -> &str {
        &self.body
    }

    pub fn set_header(&mut self, k: String, v: String) -> Option<String> {
        self.headers.insert(k.to_lowercase(), v)
    }

    pub fn get_header(&self, k: String) -> Option<String> {
        self.headers.get(&k.to_lowercase()).cloned()
    }

    pub fn extend_headers(&mut self, headers: impl Iterator<Item = (String, String)>) {
        self.headers.extend(headers)
    }

    pub fn insert_if_absent(&mut self, k: String, v: String) {
        self.headers.entry(k.to_lowercase()).or_insert(v);
    }

    pub fn format(&self) -> String {
        match self.version {
            HTTPVersion::V0_9 => format_http0_9(self).to_owned(),
            HTTPVersion::V1_0 | HTTPVersion::V1_1 => format_http1_x(self),
            HTTPVersion::V2 => todo!("Implement formatting HTTP 2 responses"),
            HTTPVersion::V3 => todo!("Implement formatting HTTP 3 responses"),
        }
    }

    pub fn send(mut self) -> Result<(), IoError> {
        write!(self.stream, "{0}", self.format())
    }
}

impl Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self.version() {
                HTTPVersion::V0_9 => format_http0_9(self).to_owned(),
                HTTPVersion::V1_0 | HTTPVersion::V1_1 => format_http1_x(self),
                other =>
                    panic!("Formatting responses for HTTP version {other} is not yet supported"),
            }
        )
    }
}

pub fn ensure_headers(res: &mut Response) {
    if !res.body.is_empty() {
        res.insert_if_absent("Content-Length".to_string(), res.body.len().to_string());

        if let Some(ct) = res.get_header("Content-Type".to_string()) {
            if !ct.contains("charset") {
                res.set_header("Content-Type".to_string(), ct + "; charset=UTF-8");
            }
        };
    }
}

// Format for HTTP 1.1
pub fn format_http0_9(res: &Response) -> &String {
    &res.body
}

fn title_case_header(s: &str) -> String {
    let mut new_s = String::with_capacity(s.len());
    let words = s.split('-');

    for (i, word) in words.enumerate() {
        if i != 0 {
            new_s.push('-');
        }

        let mut word_chars = word.chars();
        if let Some(head) = word_chars.next() {
            head.to_uppercase().for_each(|c| new_s.push(c));
            word_chars.for_each(|c| new_s.push(c));
        }
    }
    new_s
}

pub fn format_http1_x(res: &Response) -> String {
    let stringified_headers: String =
        res.headers
            .iter()
            .fold(String::new(), |mut s, (key, value)| {
                let _ = write!(s, "{0}: {value}\r\n", title_case_header(key));
                s
            });

    format!(
        "{0} {1} {2}\n{3}\n\n{4}",
        res.version,
        res.status.to_code(),
        res.status,
        stringified_headers,
        res.body
    )
}
