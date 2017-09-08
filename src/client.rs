/**
 * File: src/client.rs
 * Author: Anicka Burova <anicka.burova@gmail.com>
 * Date: 07.09.2017
 * Last Modified Date: 07.09.2017
 * Last Modified By: Anicka Burova <anicka.burova@gmail.com>
 */

use s3tunnel::Tunnel;
use std::io::{self};
//use std::iter::Iterator;

pub fn run(tunnel: Tunnel) -> io::Result<()>{
    use std::net::TcpStream;
    use std::io::{Write, Read};
    let tunnel_reader = tunnel.reader;
    let tunnel_writer = tunnel.writer;
    let tunnel_connection = tunnel.connection.unwrap();
    let connection_id = tunnel_connection.recv().unwrap();

    let address = format!("0.0.0.0:{}",4000); // create connection to the ssh
    info!("Connecting...");
    TcpStream::connect(&address)
        .and_then(move |mut socket| {
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
                                info!("Received {} data", len);
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
}
