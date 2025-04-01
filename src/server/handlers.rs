use std::collections::HashMap;

use crate::request::{HTTPMethod, Path, Request};
use crate::server::response::Response;

pub struct Handler<'a> {
    path: Path,
    method: HTTPMethod,
    // See https://stackoverflow.com/questions/27831944/how-do-i-store-a-closure-in-a-struct-in-rust/27832320#27832320
    pub callback: &'a mut dyn FnMut(Request) -> Response,
}

#[derive(Default)]
pub struct HandlerRegistry<'a> {
    // TODO: figure out how to efficiently discriminate between HTTP methods
    handlers: HashMap<String, Handler<'a>>,
}

impl<'a> Handler<'a> {
    pub fn new(
        path: Path,
        method: HTTPMethod,
        callback: &'a mut dyn FnMut(Request) -> Response,
    ) -> Self {
        Self {
            path,
            method,
            callback,
        }
    }
}
