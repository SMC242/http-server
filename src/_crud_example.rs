use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{
    request::{HTTPMethod, Request},
    server::{
        self,
        handlers::{Handler, HandlerPath, HandlerResult},
        response::{Response, ResponseBuilder, ResponseStatus},
    },
};

#[derive(Debug, Default, Serialize, Deserialize)]
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

    fn on_request(&self, req: Request) -> HandlerResult {
        let store = self.store.lock().unwrap();
        let jsonified = serde_json::to_string(&*store).expect("DogStore should be serialisable");

        HandlerResult::Done(
            ResponseBuilder::from(req)
                .ok()
                .headers(HashMap::from([(
                    "Content-Type".to_string(),
                    "application/json".to_string(),
                )]))
                .body(jsonified)
                .build()
                .expect("A valid response should be created"),
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

    fn on_request(&self, mut req: Request) -> HandlerResult {
        let mut store = self.store.lock().unwrap();

        match req.read_body_json() {
            Ok(body) => {
                let dog_name = body["name"].to_string();
                if store.names.contains(&dog_name) {
                    HandlerResult::Done(
                        ResponseBuilder::from(req)
                            .status(ResponseStatus::Conflict)
                            .body("Not added".to_string())
                            .build()
                            .expect("A valid 409 response should be produced"),
                    )
                } else {
                    store.add(&dog_name);
                    HandlerResult::Done(
                        ResponseBuilder::from(req)
                            .status(ResponseStatus::Created)
                            .body("Added".to_string())
                            .build()
                            .expect("A valid 201 response should be produced"),
                    )
                }
            }
            Err(e) => {
                log::error!("{e}");
                HandlerResult::Done(
                    ResponseBuilder::from(req)
                        .bad_request()
                        .body(e.to_string())
                        .build()
                        .expect("A valid 400 response should be produced"),
                )
            }
        }
    }
}
