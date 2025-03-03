use std::str::FromStr;

// Re-exports
pub mod http1_1;
mod types;
pub use types::*;

impl FromStr for Request {
    type Err = RequestParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!("Implement a parser that can handle any HTTP version using the version-specific modules");
    }
}
