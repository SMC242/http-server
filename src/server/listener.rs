use std::net::{Ipv4Addr, TcpListener, TcpStream};

pub fn listen<E, F>(ip: Ipv4Addr, port: u16, mut on_stream: F) -> std::io::Result<()>
where
    F: FnMut(TcpStream) -> Result<(), E>,
    E: std::fmt::Debug,
{
    let listener = TcpListener::bind((ip, port))?;
    for stream in listener.incoming() {
        let _ = on_stream(stream?)
            .inspect_err(|err| println!("Error occurred in on_stream: {0:?}", err));
    }
    Ok(())
}
