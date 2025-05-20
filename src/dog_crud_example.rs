use serde::Serialize;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{
    request::{HTTPMethod, Request},
    server::{
        handlers::{Handler, HandlerPath},
        response::{Response, ResponseStatus},
    },
};

#[derive(Default, Serialize)]
pub struct DogStore {
    pub names: Vec<String>,
}

impl DogStore {
    pub fn add(&mut self, name: &str) {
        self.names.push(name.to_string())
    }
}

pub struct DogStoreGetHandler {
    store: Arc<Mutex<DogStore>>,
    path: HandlerPath,
    method: HTTPMethod,
}

impl DogStoreGetHandler {
    pub fn new(store: Arc<Mutex<DogStore>>) -> Self {
        Self {
            store,
            path: HandlerPath::new("/dogs"),
            method: HTTPMethod::Get,
        }
    }
}

impl Handler for DogStoreGetHandler {
    fn get_path(&self) -> &HandlerPath {
        &self.path
    }

    fn get_method(&self) -> &HTTPMethod {
        &self.method
    }

    fn on_request(&self, _req: &Request) -> Response {
        let store = self.store.lock().unwrap();
        let jsonified = serde_json::to_string(&*store).expect("DogStore should be serialisable");

        Response::new(
            // FIXME: don't hardcode the HTTP version
            crate::request::HTTPVersion::V1_1,
            ResponseStatus::OK,
            HashMap::from([
                ("Content-Type".to_string(), "application/json".to_string()),
                ("Content-Length".to_string(), jsonified.len().to_string()),
            ]),
            jsonified,
        )
    }
}

pub struct DogStorePostHandler {
    store: Arc<Mutex<DogStore>>,
    path: HandlerPath,
    method: HTTPMethod,
}

impl DogStorePostHandler {
    pub fn new(store: Arc<Mutex<DogStore>>) -> Self {
        Self {
            store,
            path: HandlerPath::new("/dogs"),
            method: HTTPMethod::Post,
        }
    }
}

impl Handler for DogStorePostHandler {
    fn get_path(&self) -> &HandlerPath {
        &self.path
    }

    fn get_method(&self) -> &HTTPMethod {
        &self.method
    }

    fn on_request(&self, req: &Request) -> Response {
        let mut store = self.store.lock().unwrap();

        match req.read_body_json() {
            Ok(body) => {
                let dog_name = body["name"].to_string();
                if store.names.contains(&dog_name) {
                    Response::new(
                        crate::request::HTTPVersion::V1_1,
                        ResponseStatus::Conflict,
                        HashMap::default(),
                        "Not added".to_string(),
                    )
                } else {
                    store.add(&dog_name);
                    Response::new(
                        crate::request::HTTPVersion::V1_1,
                        ResponseStatus::OK,
                        HashMap::default(),
                        "Added".to_string(),
                    )
                }
            }
            Err(e) => {
                log::error!("{e}");
                Response::new(
                    crate::request::HTTPVersion::V1_1,
                    ResponseStatus::BadRequest,
                    HashMap::default(),
                    e.to_string(),
                )
            }
        }
    }
}
