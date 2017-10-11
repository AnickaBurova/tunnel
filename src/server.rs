/**
 * File: src/server.rs
 * Author: Anicka Burova <anicka.burova@gmail.com>
 * Date: 07.09.2017
 * Last Modified Date: 11.10.2017
 * Last Modified By: Anicka Burova <anicka.burova@gmail.com>
 */


use tunnel::{Tunnel, WriterData};
use std::io::{self};
use clap::ArgMatches;
use connection::{manage_clients, run_connection};
use std::thread;

use rustyline::error::ReadlineError;
use rustyline::Editor;

use std::sync::{Arc, Mutex};

pub fn run(matches: &ArgMatches, tunnel: Tunnel) -> io::Result<()>{
    use std::net::{TcpListener};
    let port = &matches.value_of("server-port").unwrap();
    let address = format!("0.0.0.0:{}",port);
    info!("Creating server on: {}", address);
    let client_ip = {
        let client_ip = &matches.value_of("client-address").unwrap();
        Arc::new(Mutex::new(client_ip.to_string()))
    };
    let client_port = {
        let client_port = value_t!(matches, "client-port", u16).unwrap();
        Arc::new(Mutex::new(client_port))
    };

    let tunnel_reader = tunnel.reader;
    let tunnel_writer = tunnel.writer;

    let client_state_sender = manage_clients(tunnel_reader);

    let client_ip_thread = client_ip.clone();
    let client_port_thread = client_port.clone();

    thread::spawn(move || {
        let client_ip = client_ip_thread;
        let client_port = client_port_thread;
        TcpListener::bind(address)
            .and_then(move|listener| {
                info!("Waiting for connection, going to sleep.");
                for (id, socket) in listener .incoming().enumerate() {
                    info!("Got connection {}", id);
                    let id = (id + 1) as u64;
                    let tunnel_writer = tunnel_writer.clone();
                    let client_state_sender = client_state_sender.clone();
                    {
                        let client_ip = client_ip.lock().unwrap();
                        let client_port = client_port.lock().unwrap();

                        let _ = tunnel_writer.send(WriterData::Connect(id, (*client_ip).clone(), *client_port)).unwrap();
                    }
                    socket
                        .and_then(|socket| {
                            run_connection( tunnel_writer, id, client_state_sender, socket);
                            Ok(())
                        })
                        .unwrap();

                    info!("Closing connection {}", id);

                }
                Ok(())
            })
            .unwrap();
    });


    info!("Server finished");

    Ok(())
}
