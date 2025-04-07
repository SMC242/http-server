use std::collections::hash_map::Entry;
use std::collections::HashMap;

use crate::request::{HTTPMethod, Request};
use crate::server::response::Response;

static KEY_DELIMITER: &str = "[##]";

type HandlerCallback<'a> = &'a mut dyn FnMut(Request) -> Response;

#[derive(PartialEq, Debug)]
pub struct HandlerPath(String);

/// A relative path to match against
impl HandlerPath {
    pub fn new(path: &str) -> Self {
        if !path.starts_with('/') {
            panic!("Invalid path {path}. Must be a relative path")
        }
        Self(path.strip_suffix('/').unwrap_or(path).to_string())
    }
}

pub struct Handler<'a> {
    path: HandlerPath,
    method: HTTPMethod,
    // See https://stackoverflow.com/questions/27831944/how-do-i-store-a-closure-in-a-struct-in-rust/27832320#27832320
    pub callback: HandlerCallback<'a>,
}

/**
   A composite key from a handler. This is necessary because paths can be reused for
   different HTTP verbs
*/
#[derive(Hash, PartialEq, Eq, Clone)]
struct HandlerRegistryKey(String);

impl<'a> From<&Handler<'a>> for HandlerRegistryKey {
    fn from(handler: &Handler<'a>) -> Self {
        Self(format!(
            "{0}{KEY_DELIMITER}{1}",
            handler.method, handler.path.0
        ))
    }
}

impl From<(HTTPMethod, String)> for HandlerRegistryKey {
    fn from((method, path): (HTTPMethod, String)) -> Self {
        Self(format!("{0}{KEY_DELIMITER}{1}", method, path))
    }
}

#[derive(Default)]
pub struct HandlerRegistry<'a> {
    // TODO: figure out how to efficiently discriminate between HTTP methods
    handlers: HashMap<HandlerRegistryKey, Handler<'a>>,
}

impl<'a> Handler<'a> {
    pub fn new(path: &str, method: HTTPMethod, callback: HandlerCallback<'a>) -> Self {
        Self {
            path: HandlerPath(path.to_string()),
            method,
            callback,
        }
    }

    pub fn call(&mut self, request: Request) -> Response {
        (self.callback)(request)
    }
}

struct DuplicateKey(HandlerRegistryKey);

impl<'a> HandlerRegistry<'a> {
    pub fn new(handlers: Vec<Handler<'a>>) -> Self {
        let mut registry = HashMap::new();
        handlers.into_iter().for_each(|h| {
            registry.entry(HandlerRegistryKey::from(&h)).or_insert(h);
        });
        HandlerRegistry { handlers: registry }
    }

    pub fn add(&mut self, handler: Handler<'a>) -> Result<(), DuplicateKey> {
        let key = HandlerRegistryKey::from(&handler);

        if let Entry::Vacant(e) = self.handlers.entry(key.clone()) {
            e.insert(handler);
            Ok(())
        } else {
            Err(DuplicateKey(key))
        }
    }

    pub fn get(&self, method: HTTPMethod, path: String) -> Option<&Handler<'a>> {
        self.handlers.get(&HandlerRegistryKey::from((method, path)))
    }
}
