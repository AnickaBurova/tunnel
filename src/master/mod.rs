//! Master starts and manager individual servers
//! Run a prompt to start individual listeners, which will connect to one
//! specific address on the
//! other side of the tunnel.
//!
//!

use std::io;
use tunnel::{Input, Output, Message};
mod cmd;
mod prompt;

use self::cmd::Cmd;

pub fn create(input: Input, output: Output) -> io::Result<()> {
    let mut server_id = 1;
    for cmd in prompt::prompt() {
        match cmd {
            Cmd::Server(address, port) => {
                use std::thread;
                thread::spawn(move || start_server(address, port, server_id));
                server_id += 1;
            }
        }
    }
    println!("Master finished");
    Ok(())
}

use std::net::{TcpListener, TcpStream};
fn start_server(address: String, port: u16, server_id: u32) {
    info!("Starting server-{} on the port {} for connection to '{}'",
          server_id,
          port,
          address);

    match TcpListener::bind(format!("0.0.0.0:{}", port)) {
        Ok(listener) => {
            execute_listener(listener, server_id);
        }
        Err(err) => {
            error!("Failed to create server-{} listener: {}", server_id, err);
        }
    }
}


fn execute_listener(listener: TcpListener, server_id: u32) {
    info!("Starting the server-{} listener", server_id);
    let mut connection_id = 1;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_connection(stream, server_id, connection_id);
                connection_id += 1;
            }
            Err(err) => error!("Failed to accept connection for server-{}", server_id),
        }
    }
}

fn handle_connection(stream: TcpStream, server_id: u32, connection_id: u32) {}
