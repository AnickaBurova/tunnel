/**
 * File: src/server.rs
 * Author: Anicka Burova <anicka.burova@gmail.com>
 * Date: 07.09.2017
 * Last Modified Date: 08.09.2017
 * Last Modified By: Anicka Burova <anicka.burova@gmail.com>
 */


use s3tunnel::Tunnel;
use std::io::{self};

pub fn run(tunnel: Tunnel) -> io::Result<()>{
    use std::net::{TcpListener, TcpStream};
    use std::io::{Write, Read};
    let address = format!("0.0.0.0:{}",1234); // create connection to the ssh
    TcpListener::bind(address)
        .and_then(move|listener| {
            listener
                .incoming()
                .next()
                .unwrap()
                .and_then(move|mut socket| {
                    info!("Got connection");
                    let tunnel_reader = tunnel.reader;
                    let tunnel_writer = tunnel.writer;
                    use std::time::Duration;
                    let mut buf = [0;2048];
                    socket
                        .set_read_timeout(Some(Duration::from_millis(250)))
                        .and_then(move |_| {
                            loop {
                                for data in tunnel_reader.try_iter() {
                                    socket.write(&data);
                                }
                                match socket.read(&mut buf) {
                                    Ok(len) => {
                                        let data = buf[0..len].to_vec();
                                        let _ = io_res!(tunnel_writer.send(data))?;
                                    }
                                    Err(_) => {
                                    }
                                }
                            }
                            Ok(())
                        })
                })
        })
}
