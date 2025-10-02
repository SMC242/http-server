use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};

use crate::request::{HTTPMethod, HTTPVersion, Path, Request, RequestHead, SyncableStream};
use crate::server::response::Response;

use super::response::{ResponseBuilder, ResponseStatus};

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

/// Handlers will return a `Done` if finished (I.E a response has been generated)
/// or a `Continue` containing the potentially-modified `Request`
/// if the next handler should continue processing the request.
/// All endpoints must return a `Done` while middleware may return either
pub enum HandlerResult {
    Done(Response),
    Continue(Request),
}

pub trait Handler {
    fn get_path(&self) -> &HandlerPath;
    fn get_method(&self) -> &HTTPMethod;
    fn on_request(&self, req: Request) -> HandlerResult;
}

type SyncableHandler = dyn Handler + Send + Sync;

/**
   A composite key from a handler. This is necessary because paths can be reused for
   different HTTP verbs
*/
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
struct HandlerRegistryKey(String);

impl From<&SyncableHandler> for HandlerRegistryKey {
    fn from(handler: &SyncableHandler) -> Self {
        Self(format!(
            "{0}{KEY_DELIMITER}{1}",
            handler.get_method(),
            handler.get_path().0
        ))
    }
}

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
    handlers: HashMap<HandlerRegistryKey, Arc<SyncableHandler>>,
}

#[derive(Debug)]
pub enum HandlerRegistryAddError {
    DuplicateKey(HandlerRegistryKey),
    UnhandlableMethod(HTTPMethod),
}

#[derive(Debug)]
pub enum HandlerCallErrorReason {
    /// Paths that can't be converted to origin form.
    /// The server needs to know where to route to
    UnhandlablePath(Path),
    NoCompatibleHandler(HTTPMethod, Path),
}

pub struct HandlerCallError {
    pub reason: HandlerCallErrorReason,
    stream: Box<dyn SyncableStream>,
    pub http_version: HTTPVersion,
    pub path: Path,
}

impl std::fmt::Debug for HandlerCallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandlerCallError")
            .field("reason", &self.reason)
            .field("http_version", &self.http_version)
            .field("path", &self.path)
            .field("stream", &self.stream.get_type())
            .finish()
    }
}

pub trait DispatcherError {
    fn as_status_code(&self) -> ResponseStatus;
    fn into_response(self) -> ResponseBuilder;
}

pub trait RequestDispatcher {
    type Error: DispatcherError;

    fn add(&mut self, handler: Arc<SyncableHandler>) -> Result<(), HandlerRegistryAddError>;
    fn dispatch(&self, request: Request) -> Result<Response, Self::Error>;
}

impl DispatcherError for HandlerCallError {
    fn as_status_code(&self) -> ResponseStatus {
        match self.reason {
            HandlerCallErrorReason::UnhandlablePath(_)
            | HandlerCallErrorReason::NoCompatibleHandler(_, _) => ResponseStatus::NotFound,
        }
    }

    fn into_response(self) -> ResponseBuilder {
        let builder = ResponseBuilder::default()
            .version(self.http_version)
            .stream(self.stream);

        match self.reason {
            HandlerCallErrorReason::UnhandlablePath(path) => builder
                .bad_request()
                .body(format!("Malformed URL path {path}")),
            HandlerCallErrorReason::NoCompatibleHandler(httpmethod, ref path) => builder
                .not_found()
                .body(format!("No matching handler found for {httpmethod} {path}")),
        }
    }
}

impl HandlerCallError {
    pub fn new(reason: HandlerCallErrorReason, req: Request) -> Self {
        Self {
            reason,
            http_version: req.head.version,
            path: req.head.path.clone(),
            stream: req.into_stream(),
        }
    }
}

impl HandlerRegistry {
    pub fn new(handlers: Vec<Arc<SyncableHandler>>) -> Self {
        let mut registry = HashMap::new();
        handlers.into_iter().for_each(|h| {
            let key = { HandlerRegistryKey::from(h.as_ref()) };
            registry.entry(key).or_insert(h);
        });
        HandlerRegistry { handlers: registry }
    }

    pub fn get(&self, method: HTTPMethod, path: HandlerPath) -> Option<&Arc<SyncableHandler>> {
        self.handlers
            .get(&HandlerRegistryKey::from((method, path.0)))
    }
}

impl RequestDispatcher for HandlerRegistry {
    type Error = HandlerCallError;

    fn add(&mut self, handler: Arc<SyncableHandler>) -> Result<(), HandlerRegistryAddError> {
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

    fn dispatch(&self, req: Request) -> Result<Response, HandlerCallError> {
        let RequestHead {
            method, ref path, ..
        } = req.head;
        let owned_path = path.clone();
        let mut lazy_req = Some(req);

        let handler_path = owned_path.clone().try_into().or_else(|_| {
            Err(HandlerCallError::new(
                HandlerCallErrorReason::UnhandlablePath(owned_path.clone()),
                lazy_req.take().unwrap(),
            ))
        })?;
        let handler = self.get(method, handler_path).ok_or_else(|| {
            HandlerCallError::new(
                HandlerCallErrorReason::NoCompatibleHandler(method, owned_path),
                lazy_req.take().unwrap(),
            )
        })?;

        match handler.on_request(lazy_req.take().unwrap()) {
            HandlerResult::Done(res) => Ok(res),
            HandlerResult::Continue(_) => {
                todo!("Pass the request onto the next Handler")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{request::HTTPVersion, server::response::ResponseBuilder};

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

        fn on_request(&self, req: Request) -> HandlerResult {
            HandlerResult::Done(
                ResponseBuilder::from(req)
                    .version(HTTPVersion::V1_1)
                    .ok()
                    .body("Hello, world!".to_string())
                    .build()
                    .expect("A valid hello world response will be constructed"),
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

        fn on_request(&self, _req: Request) -> HandlerResult {
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

        fn on_request(&self, _req: Request) -> HandlerResult {
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

        fn on_request(&self, _req: Request) -> HandlerResult {
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
