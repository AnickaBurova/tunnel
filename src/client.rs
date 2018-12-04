/**
 * File: src/client.rs
 * Author: Anicka Burova <anicka.burova@gmail.com>
 * Date: 07.09.2017
 * Last Modified Date: 11.10.2017
 * Last Modified By: Anicka Burova <anicka.burova@gmail.com>
 */

use tunnel::{Input, Output, Message};
use std::io::{self};
use connection::{manage_clients, run_connection};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, channel};


pub fn run(input: Input, output: Output) -> io::Result<()>{
    use std::net::TcpStream;

    let (closed_sender, closed_receiver) = channel();

    let mut connections = HashMap::new();

    loop {
        for closed_id in closed_receiver.try_iter() {
            connections.remove(&closed_id);
        }
        for msg in input.try_iter() {
            if let Message::Connect(connection_id, address) = msg {
                let (sender, receiver) = channel();
                let output_client = output.clone();
                match create_connection(connection_id, address, receiver, output_client) {
                    Ok(()) => connections.insert(connection_id, sender),
                    Err(err) => {
                        output.send(Message::Error(connection_id, err))
                    }
                }
            } else {
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
        wait_little!();
    }
    for (id, ip, port) in tunnel_connection.iter() {
        info!("Got connection [{}] to {}:{}", id, ip, port);
        let address = format!("{}:{}",ip, port); // create connection to the ssh
        let tunnel_writer = tunnel_writer.clone();
        let client_state_sender = client_state_sender.clone();
        TcpStream::connect(&address)
            .and_then(move |socket| {
                run_connection( tunnel_writer, id, client_state_sender, socket);
                Ok(())
            })
            .unwrap();
    }
    Ok(())
}
