/**
 * File: src/client.rs
 * Author: Anicka Burova <anicka.burova@gmail.com>
 * Date: 07.09.2017
 * Last Modified Date: 09.10.2017
 * Last Modified By: Anicka Burova <anicka.burova@gmail.com>
 */

use tunnel::Tunnel;
use std::io::{self};
use clap::ArgMatches;
use std::thread;
use std::sync::mpsc::{channel, Sender, Receiver};
use connection::{manage_clients, run_connection};


pub fn run(matches: &ArgMatches, tunnel: Tunnel) -> io::Result<()>{
    use std::net::TcpStream;
    use std::io::{Write, Read};
    let port = &matches.value_of("client-port").unwrap();
    let ip = &matches.value_of("client-address").unwrap();
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

        //let tunnel_writer = tunnel_writer.clone();
        //let client_state_sender = client_state_sender.clone();
        //thread::spawn(move || {
            //let (data_sender, data_receiver) = channel();
            //client_state_sender.write(ClientState::NewClient(id, data_sender)).unwrap();
            //let address = format!("{}:{}",ip, port); // create connection to the ssh
            //info!("Connecting to {} ...", address);
            //TcpStream::connect(&address)
                //.and_then(move |mut socket| {
                    //use std::time::Duration;
                    //let mut buf = [0;2048];
                    //socket
                        //.set_read_timeout(Some(Duration::from_millis(20)))
                        //.and_then(move |_| {
                            //loop {
                                //for data in tunnel_reader.try_iter() {
                                    //let _ = socket.write(&data);
                                //}
                                //match socket.read(&mut buf) {
                                    //Ok(len) => {
                                        //let data = buf[0..len].to_vec();
                                        //let _ = tunnel_writer.send(data).unwrap();
                                    //}
                                    //Err(_) => break,
                                //}
                            //}
                            //Ok(())
                        //});
                    //client_state_sender.write(ClientState::Disconnected(id)).unwrap();
                //})
        //});
    }
    Ok(())
}
