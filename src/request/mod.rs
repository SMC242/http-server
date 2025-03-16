use std::str::FromStr;

// Re-exports
mod headers;
pub use headers::*;
mod body;
pub use body::*;
pub mod http1_1;
mod types;
pub use types::*;

impl FromStr for RequestHead {
    type Err = RequestParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!("Implement a parser that can handle any HTTP version using the version-specific modules");
    }
}

impl RequestHead {
    pub fn should_read_body(&self) -> bool {
        matches!(
            self.method,
            HTTPMethod::Put | HTTPMethod::Post | HTTPMethod::Patch
        )
    }
}
