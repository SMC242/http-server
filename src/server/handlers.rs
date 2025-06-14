use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::request::{HTTPMethod, Path, Request, RequestHead};
use crate::server::response::Response;

static KEY_DELIMITER: &str = "[##]";

pub type HandlerCallback = Box<dyn FnMut(Request) -> Response>;

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

impl TryFrom<Path> for HandlerPath {
    type Error = &'static str;

    fn try_from(value: Path) -> Result<HandlerPath, Self::Error> {
        match value {
            Path::Asterisk => Err("Can't convert from asterisk form: it's only used for OPTIONS"),
            Path::AuthorityForm(..) => {
                Err("Can't convert from authority form: it's only used for CONNECT")
            }
            Path::OriginForm(path) => Ok(HandlerPath(path)),
            Path::AbsoluteForm(path) => {
                if path
                    .splitn(2, '/')
                    // Skip the host portion
                    .skip(1)
                    .take(1)
                    .collect::<String>()
                    .is_empty()
                {
                    // Index page (E.G example.com/). Corrects example.com to example.com/
                    Ok(HandlerPath("/".to_string()))
                } else {
                    Ok(HandlerPath(path.to_string()))
                }
            }
        }
    }
}

pub trait Handler {
    fn get_path(&self) -> &HandlerPath;
    fn get_method(&self) -> &HTTPMethod;
    fn on_request(&self, req: &Request) -> Response;
}

/**
   A composite key from a handler. This is necessary because paths can be reused for
   different HTTP verbs
*/
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
struct HandlerRegistryKey(String);

impl From<&dyn Handler> for HandlerRegistryKey {
    fn from(handler: &dyn Handler) -> Self {
        Self(format!(
            "{0}{KEY_DELIMITER}{1}",
            handler.get_method(),
            handler.get_path().0
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
    handlers: HashMap<HandlerRegistryKey, Arc<dyn Handler>>,
}

#[derive(Debug)]
pub enum HandlerRegistryAddError {
    DuplicateKey(HandlerRegistryKey),
    UnhandlableMethod(HTTPMethod),
}

#[derive(Debug)]
pub enum HandlerCallError {
    /// Paths that can't be converted to origin form.
    /// The server needs to know where to route to
    UnhandlablePath(Path),
    NoCompatibleHandler(HTTPMethod, Path),
}

impl HandlerRegistry {
    pub fn new(handlers: Vec<Arc<dyn Handler>>) -> Self {
        let mut registry = HashMap::new();
        handlers.into_iter().for_each(|h| {
            let key = { HandlerRegistryKey::from(h.as_ref()) };
            registry.entry(key).or_insert(h);
        });
        HandlerRegistry { handlers: registry }
    }

    pub fn add(&mut self, handler: Arc<dyn Handler>) -> Result<(), HandlerRegistryAddError> {
        if matches!(
            handler.get_method(),
            HTTPMethod::Trace | HTTPMethod::Connect | HTTPMethod::Options
        ) {
            return Err(HandlerRegistryAddError::UnhandlableMethod(
                handler.get_method().to_owned(),
            ));
        }

        let key = HandlerRegistryKey::from(handler.as_ref());

        if let Entry::Vacant(e) = self.handlers.entry(key.clone()) {
            e.insert(handler);
            Ok(())
        } else {
            Err(HandlerRegistryAddError::DuplicateKey(key))
        }
    }

    pub fn get(&self, method: HTTPMethod, path: HandlerPath) -> Option<&Arc<dyn Handler>> {
        self.handlers
            .get(&HandlerRegistryKey::from((method, path.0)))
    }

    pub fn dispatch(&self, req: &Request) -> Result<Response, HandlerCallError> {
        let RequestHead {
            method, ref path, ..
        } = req.head;

        let handler_path = path
            .clone()
            .try_into()
            .or(Err(HandlerCallError::UnhandlablePath(path.clone())))?;
        self.get(method, handler_path)
            .map(|h| h.on_request(req))
            .ok_or(HandlerCallError::NoCompatibleHandler(method, path.clone()))
    }
}

#[cfg(test)]
mod tests {
    use crate::request::HTTPVersion;

    use super::*;

    struct HelloWorldHandler {
        path: HandlerPath,
        method: HTTPMethod,
    }

    impl HelloWorldHandler {
        pub fn new() -> Self {
            Self {
                path: HandlerPath::new("/"),
                method: HTTPMethod::Get,
            }
        }
    }

    impl Handler for HelloWorldHandler {
        fn get_path(&self) -> &HandlerPath {
            &self.path
        }

        fn get_method(&self) -> &HTTPMethod {
            &self.method
        }

        fn on_request(&self, _req: &Request) -> Response {
            Response::new(
                HTTPVersion::V1_1,
                crate::server::response::ResponseStatus::OK,
                HashMap::default(),
                "Hello, world!".to_string(),
            )
        }
    }

    struct ConnectHandler {}

    impl Handler for ConnectHandler {
        fn get_path(&self) -> &HandlerPath {
            todo!("No path")
        }

        fn get_method(&self) -> &HTTPMethod {
            &HTTPMethod::Connect
        }

        fn on_request(&self, _req: &Request) -> Response {
            todo!("No handler")
        }
    }

    struct TraceHandler {}

    impl Handler for TraceHandler {
        fn get_path(&self) -> &HandlerPath {
            todo!("No path")
        }

        fn get_method(&self) -> &HTTPMethod {
            &HTTPMethod::Trace
        }

        fn on_request(&self, _req: &Request) -> Response {
            todo!("No handler")
        }
    }

    struct OptionsHandler {}

    impl Handler for OptionsHandler {
        fn get_path(&self) -> &HandlerPath {
            todo!("No path")
        }

        fn get_method(&self) -> &HTTPMethod {
            &HTTPMethod::Options
        }

        fn on_request(&self, _req: &Request) -> Response {
            todo!("No handler")
        }
    }

    #[test]
    fn add_handler() {
        println!("Startting");
        let handler = HelloWorldHandler::new();
        let mut registry: HandlerRegistry = HandlerRegistry::default();

        registry
            .add(Arc::new(handler))
            .expect("Adding a GET handler for / should succeed");

        let handler = registry
            .get(HTTPMethod::Get, HandlerPath::new("/"))
            .expect("A GET handler for / should be found");
        assert_eq!(*handler.get_method(), HTTPMethod::Get);
        assert_eq!(*handler.get_path(), HandlerPath::new("/"))
    }

    #[test]
    fn add_unhandlable() {
        let mut registry = HandlerRegistry::default();
        registry
            .add(Arc::new(ConnectHandler {}))
            .expect_err("Adding a handler for CONNECT should fail");

        registry
            .add(Arc::new(TraceHandler {}))
            .expect_err("Adding a handler for TRACE should fail");

        registry
            .add(Arc::new(OptionsHandler {}))
            .expect_err("Adding a handler for OPTIONS should fail");
    }
}
