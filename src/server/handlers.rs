use std::collections::hash_map::Entry;
use std::collections::HashMap;

use crate::request::{HTTPMethod, Request};
use crate::server::response::Response;

static KEY_DELIMITER: &str = "[##]";

type HandlerCallback = Box<dyn FnMut(Request) -> Response>;

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

pub struct Handler {
    path: HandlerPath,
    method: HTTPMethod,
    // See https://stackoverflow.com/questions/27831944/how-do-i-store-a-closure-in-a-struct-in-rust/27832320#27832320
    pub callback: HandlerCallback,
}

/**
   A composite key from a handler. This is necessary because paths can be reused for
   different HTTP verbs
*/
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
struct HandlerRegistryKey(String);

impl From<&Handler> for HandlerRegistryKey {
    fn from(handler: &Handler) -> Self {
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
pub struct HandlerRegistry {
    // TODO: figure out how to efficiently discriminate between HTTP methods
    handlers: HashMap<HandlerRegistryKey, Handler>,
}

impl Handler {
    pub fn new(method: HTTPMethod, path: &str, callback: HandlerCallback) -> Self {
        Self {
            path: HandlerPath::new(path),
            method,
            callback,
        }
    }

    pub fn call(&mut self, request: Request) -> Response {
        (self.callback)(request)
    }
}

#[derive(Debug)]
pub enum HandlerRegistryAddError {
    DuplicateKey(HandlerRegistryKey),
    UnhandlableMethod(HTTPMethod),
}

impl HandlerRegistry {
    pub fn new(handlers: Vec<Handler>) -> Self {
        let mut registry = HashMap::new();
        handlers.into_iter().for_each(|h| {
            registry.entry(HandlerRegistryKey::from(&h)).or_insert(h);
        });
        HandlerRegistry { handlers: registry }
    }

    pub fn add(&mut self, handler: Handler) -> Result<(), HandlerRegistryAddError> {
        if matches!(
            handler.method,
            HTTPMethod::Trace | HTTPMethod::Connect | HTTPMethod::Options
        ) {
            return Err(HandlerRegistryAddError::UnhandlableMethod(handler.method));
        }

        let key = HandlerRegistryKey::from(&handler);

        if let Entry::Vacant(e) = self.handlers.entry(key.clone()) {
            e.insert(handler);
            Ok(())
        } else {
            Err(HandlerRegistryAddError::DuplicateKey(key))
        }
    }

    pub fn get(&self, method: HTTPMethod, path: HandlerPath) -> Option<&Handler> {
        self.handlers
            .get(&HandlerRegistryKey::from((method, path.0)))
    }
}

