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
use std::sync::mpsc::{channel, Sender, Receiver};
use connection::{manage_clients, run_connection};
use std::thread;

enum ClientState {
    NewClient(u64, Sender<Vec<u8>>),
    Disconnected(u64),
}

pub fn run(matches: &ArgMatches, tunnel: Tunnel) -> io::Result<()>{
    use std::net::{TcpListener};
    use std::io::{Write, Read};
    let port = &matches.value_of("server-port").unwrap();
    let address = format!("0.0.0.0:{}",port);
    info!("Creating server on: {}", address);
    let client_ip = &matches.value_of("client-address").unwrap();
    let client_port = value_t!(matches, "client-port", u16).unwrap();

    let tunnel_reader = tunnel.reader;
    let tunnel_writer = tunnel.writer;

    let client_state_sender = manage_clients(tunnel_reader);

    TcpListener::bind(address)
        .and_then(move|listener| {
            info!("Waiting for connection, going to sleep.");
            for (id, socket) in listener .incoming().enumerate() {
                info!("Got connection {}", id);
                let client_ip = client_ip.to_string();
                let id = (id + 1) as u64;
                let tunnel_writer = tunnel_writer.clone();
                let client_state_sender = client_state_sender.clone();
                tunnel_writer.send(WriterData::Connect(id, client_ip, client_port));
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

    info!("Server finished");

    Ok(())
}
