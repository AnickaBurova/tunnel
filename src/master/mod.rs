//! Master starts and manager individual servers
//! Run a prompt to start individual listeners, which will connect to one
//! specific address on the
//! other side of the tunnel.
//!
//!

use std::io;
use tunnel::{Input, Output, Message};
use std::sync::mpsc::{Receiver, Sender, channel};
mod cmd;
mod prompt;
use unique_id::*;
use std::thread;
use std::io::{Read, Write};
use self::cmd::Cmd;

static CONNECTION_ID: Generator = GENERATOR_INIT;

enum Mgr {
    Create(ID, Output),
    Closed(ID),
}

pub fn create(input: Input, output: Output) -> io::Result<()> {
    let (mgr_sender, mgr_receiver) = channel();
    thread::spawn(move|| {
        use std::collections::HashMap;
        // map of current connections
        let mut connections = HashMap::new();
        loop {
            // read any management messages
            for mgr in mgr_receiver.try_iter() {
                match mgr {
                    Mgr::Create(id, sender) => {
                        connections.insert(id, sender);
                    }
                    Mgr::Closed(id) => {
                        connections.remove(&id);
                    }
                }
            }

            for msg in input.try_iter() {
                let connection_id = msg.get_id();
                match connections.get(&connection_id) {
                    Some(ref sender) => {
                        let _ = sender.send(msg);
                    }
                    None => {
                        error!("Received message from the tunnel for not existing connection: {}", connection_id);
                    }
                }
            }
        }
    });
    let mut server_id = 1;
    // run readline to manage new servers
    for cmd in prompt::prompt() {
        match cmd {
            Cmd::Server(address, port) => {
                let mgr_sender = mgr_sender.clone();
                let output = output.clone();
                thread::spawn(move || start_server(address, port, server_id, mgr_sender, output));
                server_id += 1;
            }
        }
    }
    println!("Master finished");
    Ok(())
}

use std::net::{TcpListener, TcpStream};
fn start_server(address: String, port: u16, server_id: u32, mgr_sender: Sender<Mgr>, output: Output) {
    info!("Starting server-{} on the port {} for connection to '{}'",
          server_id,
          port,
          address);

    match TcpListener::bind(format!("0.0.0.0:{}", port)) {
        Ok(listener) => {
            execute_listener(listener, server_id, mgr_sender, output);
        }
        Err(err) => {
            error!("Failed to create server-{} listener: {}", server_id, err);
        }
    }
}


fn execute_listener(listener: TcpListener, server_id: u32, mgr_sender: Sender<Mgr>, output: Output) {
    info!("Starting the server-{} listener", server_id);
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let mgr_sender = mgr_sender.clone();
                let output = output.clone();
                let connection_id = next_id!(CONNECTION_ID);
                use std::thread;
                thread::spawn(move|| handle_connection(stream, server_id, connection_id, mgr_sender, output));
            }
            Err(err) => error!("Failed to accept connection for server-{}", server_id),
        }
    }
}

fn handle_connection(stream: TcpStream, server_id: u32, connection_id: ID, mgr_sender: Sender<Mgr>, output: Output) {
    let mut stream = stream;
    let (from_tunnel_sender, input) = channel();
    use std::time::Duration;
    match stream.set_read_timeout(Some(Duration::from_millis(20))) {
        Ok(_) => (),
        Err(err) => {
            error!("Connection-{}-{} has failed to setup timeout", server_id, connection_id);
            return;
        }
    }
    mgr_sender.send(Mgr::Create(connection_id, from_tunnel_sender));
    macro_rules! check_id {
        ($server: ident, $my_id: ident, $id: ident, $msg: expr) => (
            if $id != $my_id {
                error!("Connection-{}-{} has received {} for connection-{}", $server, $my_id, $msg, $id);
                false
            } else {
                true
            }
        )
    }
    let mut buf = [0;2048];
    'main: loop {
        for msg in input.try_iter() {
            match msg {
                Message::Disconnect(id) => {
                    if check_id!(server_id, connection_id, id, "disconnect") {
                        break 'main;
                    }
                }
                Message::Data(id, data) => {
                    if check_id!(server_id, connection_id, id, "data") {
                        let _ = stream.write(&data[..]);
                    }
                }
                _ => (), // irrelevant messages
            }
        }
        match stream.read(&mut buf) {
            Ok(0)   => break, // end of stream, we can quit
            Ok(len) => {
                let data  = buf[0..len].to_vec();
                let msg   = Message::Data(connection_id, data);
                match output.send(msg) {
                    Ok(()) => (),
                    Err(err) => {
                        error!("Connection-{}-{} failed to send over channel: {}", server_id, connection_id, err);
                        break; // quit the connection
                    }
                }
            }
            Err(err)     => {
                match err.kind() {
                    io::ErrorKind::WouldBlock => (), // timeout on reading, that is ok
                    _                         => {
                        error!("Connection-{}-{} failed to receive data from tcp stream: {}", server_id, connection_id, err);
                        break;
                    }
                }
            }
        }
        wait_little!();
    }
    mgr_sender.send(Mgr::Closed(connection_id));
}
