use std::io::Error as IoError;
use std::net::{IpAddr, Ipv4Addr};
use std::panic;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use http_server::_crud_example as rest_api;
use http_server::server::handlers::{Handler, HandlerRegistry};
use http_server::server::listener::{self, ListenerConfig};
use serde::Serialize;
use ureq::Agent;

static IP: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

struct TestDeps {
    agent: Agent,
    base_url: String,
    port: u16,
}

fn setup() -> TestDeps {
    let _ = env_logger::builder().is_test(true).try_init();

    let port = rand::random_range(8000..9000);
    let base_url = format!("http://{IP}:{port}");
    log::debug!("Generated {base_url} as the base URL");

    TestDeps {
        agent: Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(5)))
            .build()
            .into(),
        base_url,
        port,
    }
}

fn run_listener(
    port: u16,
    handlers: Vec<Arc<dyn Handler + Send + Sync>>,
) -> std::thread::JoinHandle<Result<(), IoError>> {
    log::info!(target: "listener", "Initialising handlers");
    let registry = HandlerRegistry::new(handlers);

    log::info!(target: "listener", "Starting server on {IP}:{port}");
    thread::spawn(move || {
        listener::HTTPListener::new(IP, port, registry, ListenerConfig::default()).listen()
    })
}

fn qualify(base_url: &str, segment: &str) -> String {
    let url = format!("{base_url}/{segment}");
    log::debug!("Qualified URL: {url}");
    url
}

fn assert_ok<T>(response: &http::Response<T>) {
    assert_eq!(
        response.status(),
        http::StatusCode::OK,
        "The request should return 200 OK"
    )
}

#[test]
fn test_get_endpoint() {
    let TestDeps {
        agent,
        base_url,
        port,
    } = setup();
    let dog_store = Arc::new(Mutex::new(rest_api::DogStore::default()));
    let _ = run_listener(
        port,
        vec![Arc::new(rest_api::DogStoreGetHandler::new(dog_store))],
    );
    thread::sleep(Duration::from_millis(50));

    let mut response = agent
        .get(qualify(&base_url, "dogs"))
        .call()
        .expect("Calling the /dogs endpoint should succeed");
    assert_ok(&response);

    let raw_body = response
        .body_mut()
        .read_to_string()
        .expect("Reading the body should succeed");
    log::debug!("Received raw body: {raw_body}");

    let dog_names: rest_api::DogStore =
        serde_json::from_str(&raw_body).expect("GET /dogs should return valid JSON");

    let empty_vec: Vec<String> = vec![];
    assert_eq!(
        dog_names.names, empty_vec,
        "The list of dog names should be empty"
    );
}

#[derive(Debug, Serialize)]
struct NewDogName {
    name: String,
}

#[test]
fn test_post_endpoint() {
    let TestDeps {
        agent,
        base_url,
        port,
    } = setup();
    let dog_store = Arc::new(Mutex::new(rest_api::DogStore::default()));
    let _ = run_listener(
        port,
        vec![
            Arc::new(rest_api::DogStoreGetHandler::new(dog_store.clone())),
            Arc::new(rest_api::DogStorePostHandler::new(dog_store.clone())),
        ],
    );
    thread::sleep(Duration::from_millis(50));

    let new_name = NewDogName {
        name: "Alfred".to_string(),
    };
    let response = agent
        .post(qualify(&base_url, "dogs"))
        .header("Content-Type", "application/json")
        .send_json(&new_name)
        .expect("POSTing to the endpoint should succeed");
    assert_eq!(response.status(), http::StatusCode::CREATED);

    let dog_names = agent
        .get(qualify(&base_url, "dogs"))
        .call()
        .expect("Calling the endpoint should succeed")
        .body_mut()
        .read_json::<rest_api::DogStore>()
        .expect("GET /dogs should return valid JSON");
    assert_eq!(
        dog_names.names,
        vec!["Alfred"],
        "Alfred should have been added to the store"
    );

    // Check for idempotency
    let response = agent
        .post(qualify(&base_url, "dogs"))
        .header("Content-Type", "application/json")
        .send_json(&new_name)
        .expect_err("POSTing a dog that already exists should fail");
    assert!(
        matches!(response, ureq::Error::StatusCode(409)),
        "The POST request should fail with status 409 Conflict"
    );

    let raw_body = agent
        .get(qualify(&base_url, "dogs"))
        .call()
        .expect("GET /dogs should succeed")
        .body_mut()
        .read_to_string()
        .expect("Reading the body should succeed");
    log::debug!("Received raw body: {raw_body}");

    let dog_names: rest_api::DogStore =
        serde_json::from_str(&raw_body).expect("GET /dogs should return valid JSON");

    let expected = vec!["Alfred"];
    log::debug!("Structure: {dog_names:?}, expected: {expected:?}");
    assert_eq!(
        dog_names.names, expected,
        "Alfred should still be in the store"
    );
}
