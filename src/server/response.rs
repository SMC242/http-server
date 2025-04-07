use regex::Regex;
use std::{borrow::Cow, fmt::Display};

use crate::request::HTTPHeaders;

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
            pascal_cased => unpascal_case(&pascal_cased.to_string()).to_string(),
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
    status: ResponseStatus,
    headers: HTTPHeaders,
    body: String,
}

impl Response {
    pub fn new(status: ResponseStatus, headers: HTTPHeaders, body: String) -> Self {
        Self {
            status,
            headers,
            body,
        }
    }
}

impl Default for Response {
    fn default() -> Self {
        Self {
            status: ResponseStatus::OK,
            headers: std::collections::HashMap::new(),
            body: String::new(),
        }
    }
}
