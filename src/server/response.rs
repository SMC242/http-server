use regex::Regex;
use std::fmt::Write;
use std::{borrow::Cow, fmt::Display};

use crate::request::{HTTPHeaders, HTTPVersion};

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

#[derive(Debug)]
pub struct Response {
    version: HTTPVersion,
    status: ResponseStatus,
    headers: HTTPHeaders,
    body: String,
}

impl Response {
    pub fn new(
        version: HTTPVersion,
        status: ResponseStatus,
        headers: HTTPHeaders,
        body: String,
    ) -> Self {
        Self {
            version,
            status,
            headers,
            body,
        }
    }

    /// Convenience constructor that sets the version from the given request
    pub fn from_request(
        req: crate::request::Request,
        status: ResponseStatus,
        headers: HTTPHeaders,
        body: String,
    ) -> Self {
        Response::new(req.head.version, status, headers, body)
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

    pub fn upsert_header(&mut self, k: String, v: String) {
        self.headers.entry(k.to_lowercase()).or_insert(v);
    }
}

impl Default for Response {
    fn default() -> Self {
        Self {
            version: HTTPVersion::V1_1,
            status: ResponseStatus::OK,
            headers: std::collections::HashMap::new(),
            body: String::new(),
        }
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
    res.upsert_header("Content-Length".to_string(), res.body.len().to_string());

    // Set character encoding if required
    if !res.body.is_empty() {
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

pub fn format_http1_x(res: &Response) -> String {
    let stringified_headers: String =
        res.headers
            .iter()
            .fold(String::new(), |mut s, (key, value)| {
                let _ = write!(s, "{key}: {value}");
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
