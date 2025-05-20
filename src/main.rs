use log::info;
use server::handlers::HandlerRegistry;
use server::listener::{self, ListenerConfig};
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};

mod dog_crud_example;
use dog_crud_example::{self as dogstore, DogStoreGetHandler, DogStorePostHandler};
mod mime;
mod request;
mod server;

static IP: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
// TODO: increment if port is unavailable. Will require this to not be static
static PORT: u16 = 8080;

fn main() -> std::io::Result<()> {
    env_logger::init();

    info!(target: "listener", "Initialising handlers");
    let dog_store = Arc::new(Mutex::new(dogstore::DogStore::default()));
    let registry = HandlerRegistry::new(vec![
        Arc::new(DogStoreGetHandler::new(dog_store.clone())),
        Arc::new(DogStorePostHandler::new(dog_store.clone())),
    ]);

    info!(target: "listener", "Starting server on {IP}:{PORT}");
    listener::HTTPListener::new(IP, PORT, registry, ListenerConfig::default()).listen()
}
