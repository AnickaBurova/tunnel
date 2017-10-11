/**
 * File: src/client.rs
 * Author: Anicka Burova <anicka.burova@gmail.com>
 * Date: 07.09.2017
 * Last Modified Date: 11.10.2017
 * Last Modified By: Anicka Burova <anicka.burova@gmail.com>
 */

use tunnel::Tunnel;
use std::io::{self};
use clap::ArgMatches;
use connection::{manage_clients, run_connection};


pub fn run(_matches: &ArgMatches, tunnel: Tunnel) -> io::Result<()>{
    use std::net::TcpStream;
    let tunnel_reader = tunnel.reader;
    let tunnel_writer = tunnel.writer;
    let tunnel_connection = tunnel.connection.unwrap();

    let client_state_sender = manage_clients(tunnel_reader);


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
