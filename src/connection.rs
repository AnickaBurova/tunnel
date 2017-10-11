/**
 * File: src/connection.rs
 * Author: Anicka Burova <anicka.burova@gmail.com>
 * Date: 09.10.2017
 * Last Modified Date: 11.10.2017
 * Last Modified By: Anicka Burova <anicka.burova@gmail.com>
 */


use std::sync::mpsc::{channel, Sender, Receiver};
use std::net::TcpStream;
use tunnel::{WriterData, ReaderData};
use std::thread;
use std::io::{self,Read,Write};

pub enum ClientState {
    NewClient(u64, Sender<ReaderData>),
    Disconnected(u64),
}

pub fn manage_clients(tunnel_reader: Receiver<(u64, ReaderData)>) -> Sender<ClientState> {
    let (client_state_sender, client_state_receiver) = channel();

    use std::thread;
    thread::spawn(move|| {
        // the data which come from the tunnel, needs to be redirected to the appropriate client. The
        // client is identified by the `id`
        use std::collections::HashMap;
        let mut clients = HashMap::new();
        loop {
            for state in client_state_receiver.try_iter() {
                match state {
                    ClientState::NewClient(id, sender) => clients.insert(id, sender),
                    ClientState::Disconnected(id)      => clients.remove(&id),
                };
            }
            for (id, data) in tunnel_reader.try_iter() {
                match clients.get(&id) {
                    Some(sender)   => {
                        match sender.send(data) {
                            Ok(_)  => (),
                            Err(e) => error!("Failed to send the data to appropriate client, this should never happen! {:?}", e),
                        }
                    }
                    None           => error!("Received a data from the tunnel for not existing client id: {}", id),
                }
            }
            wait_little!();
        }
    });

    client_state_sender
}


pub fn run_connection(tunnel_writer: Sender<WriterData>, id: u64, client_state_sender: Sender<ClientState>, socket: TcpStream) {
    let mut socket = socket;
    thread::spawn(move|| {
        debug!("[{}] Thread started.", id);
        let (data_sender, data_receiver) = channel();
        client_state_sender.send(ClientState::NewClient(id, data_sender)).unwrap();
        let mut buf = [0;2048];
        use std::time::Duration;
        socket
            .set_read_timeout(Some(Duration::from_millis(20)))
            .and_then(move |_| {
                debug!("[{}] Loop started", id);
                let mut disconnect = false;
                loop {
                    for data in data_receiver.try_iter() {
                        match data {
                            ReaderData::Disconnect => disconnect = true,
                            ReaderData::Data(data) => {
                                let _ = socket.write(&data);
                            }
                        }
                    }
                    if disconnect {
                        break;
                    }
                    match socket.read(&mut buf) {
                        Ok(0)   => break,
                        Ok(len) => {
                            let data  = buf[0..len].to_vec();
                            let msg   = WriterData::Data(id, data);
                            let _     = tunnel_writer.send(msg).unwrap();
                        }
                        Err(err)     => {
                            match err.kind() {
                                io::ErrorKind::WouldBlock => (),
                                _                         => break,
                            }
                        }
                    }
                    wait_little!();
                }
                let _ = tunnel_writer.send(WriterData::Disconnect(id)).unwrap();
                Ok(())
            })
            .unwrap();

        client_state_sender.send(ClientState::Disconnected(id)).unwrap();
        info!("[{}] Closing connection", id);
    });
}

