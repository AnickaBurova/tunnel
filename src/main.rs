/**
 * File: src/main.rs
 * Author: Anicka Burova <anicka.burova@gmail.com>
 * Date: 04.09.2017
 * Last Modified Date: 07.09.2017
 * Last Modified By: Anicka Burova <anicka.burova@gmail.com>
 */
extern crate aws_sdk_rust;
extern crate ini;

extern crate bytes;
extern crate futures;

extern crate tokio_io;
extern crate tokio_core;
extern crate tokio_proto;
extern crate tokio_service;

#[macro_use]
extern crate clap;

#[macro_use]
extern crate log;
extern crate log4rs;

#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;

use std::io::{self};

#[macro_use]
mod tools;
mod config;
mod messages;
mod s3tunnel;
mod server;
mod client;


pub struct RawCodec {
    pub id: u64,
    pub name: String,
}

use tokio_io::codec::{Encoder, Decoder};
use tokio_core::net::UdpCodec;
use bytes::BytesMut;

impl Decoder for RawCodec {
    type Item = Vec<u8>;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<Self::Item>> {
        if buf.len() > 0 {
            let size = buf.len();
            let line = buf.split_to(size).to_vec();
            self.id += 1;
            use std::str;
            match str::from_utf8(&line) {
                Ok(s) => {
                    info!("Tcp-{}[{}]: {}", self.name, self.id, s);
                }
                Err(_) => {
                    info!("Tcp-{}[{}]: {}", self.name, self.id, line.len());
                }
            }
            Ok(Some(line))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for RawCodec {
    type Item = Vec<u8>;
    type Error = io::Error;

    fn encode(&mut self, msg: Self::Item, buf: &mut BytesMut) -> io::Result<()> {
        buf.extend(msg);
        Ok(())
    }
}

//use std::net::SocketAddr;

//impl UdpCodec for RawCodec {
    //type In = (SocketAddr, Vec<u8>);
    //type Out = (SocketAddr, Vec<u8>);
    //fn decode(&mut self, src: &SocketAddr, buf: &[u8]) -> io::Result<Self::In> {
        //let size = buf.len();
        //let line = buf.to_vec();
        //self.id += 1;
        //use std::str;
        //match str::from_utf8(&line) {
            //Ok(s) => {
                //info!("Udp-{}[{}]: {}", self.name, self.id, s);
            //}
            //Err(_) => {
                //info!("Udp-{}[{}]: {}", self.name, self.id, line.len());
            //}
        //}
        //Ok((*src, line))
    //}
    //fn encode(&mut self, (addr, msg): Self::Out, buf: &mut Vec<u8>) -> SocketAddr {
        //buf.extend(msg);
        //addr
    //}
//}

use tokio_io::{AsyncRead};
use futures::{Future};

use config::*;

fn s3run(matches: &ArgMatches) -> io::Result<()> {
    let mode = &matches.value_of("mode").unwrap();
    let is_server = mode == &"server";
    let (writer_name , reader_name ) = if is_server {
        ("tunnel.in", "tunnel.out")
    } else {
        ("tunnel.out", "tunnel.in")
    };
    let port = &matches.value_of("port").unwrap();
    load_config()
        .and_then( |cfg| {
            s3tunnel::create_clients(is_server, cfg, writer_name, reader_name)
        })
        .and_then( |tunnel| {
            if is_server {
                server::run(tunnel)
            } else {
                client::run(tunnel)
            }
            //use tokio_core::reactor::Core;
            //let mut core = Core::new().unwrap();
            //let handle = core.handle();
            //use futures::stream::Stream;
            //use futures::Sink;
            //use futures::future::{self};
            //// if this is server, then create listener and wait for the connection
            //if mode == Mode::Server {
                //info!("Creating tcp listener");
                //let address = format!("0.0.0.0:{}",port).parse().unwrap(); // waiting on specific port for incoming connections
                //use tokio_core::net::TcpListener;
                //let listener = TcpListener::bind(&address, &handle).unwrap();
                //let runner = listener
                    //.incoming()
                    //.for_each(move |(socket, _peer_addr)| {
                        //info!("Got a new connection");
                        //if let Some((data_in_receiver, data_out_sender)) = data_in_out.take() {
                            //info!("server: connected");
                            //// we have a connection, something connected to the listener on the local
                            //// pc or possible from somewhere else

                            //// create writer and reader as frames of Vec<u8>
                            //let (writer, reader) = socket.framed(RawCodec{id:0,name:"Server".to_owned()}).split();
                            //// for each data read from s3 send the data using writer
                            ////use futures::stream::{Stream};
                            //use futures::future::*;
                            //let reader = reader
                                //.then(move |req| {
                                    //info!("got message from TCP");
                                    //let _ = data_out_sender.send(req.unwrap()).unwrap();
                                    //Ok::<Vec<u8>,io::Error>(vec![])
                                //})
                            //;
                                ////.into_future()
                                ////.then(|_| Ok(()));

                            ////handle.spawn(reader);
                            //let received_data = data_in_receiver.then(|r| r.unwrap()).select(reader);
                            //let writer = writer
                                //.send_all(received_data)
                                //.then(|_| Ok(()));
                            //handle.spawn(writer);
                        //}
                        //Ok(())
                    //});
                //let _ = core.run(runner);
            //} else {
                //// if this is client, then wait until we are allowed to create connection
                //use std::net::TcpStream;
                //use std::io::{Write, Read};
                //let address = format!("0.0.0.0:{}",4000); // create connection to the ssh
                //use std::sync::atomic::{Ordering};
                //let allow_connection = allow_connection.unwrap();
                //while !allow_connection.load(Ordering::SeqCst) {
                    //use std::thread::sleep;
                    //use std::time::Duration;
                    //info!("No connection, waiting");
                    //sleep(Duration::from_secs(1));
                //}
                //info!("Connection...");
                //let client = TcpStream::connect(&address)
                    //.and_then(|socket| {
                        ////if let Some((data_in_receiver, data_out_sender)) = data_in_out.take() {
                            ////loop {
                                //////for 
                            ////}
                        ////}
                        //Ok(())
                    //});
                ////let runner = client
                    ////.and_then(move |socket| {
                        ////if let Some((data_in_receiver, data_out_sender)) = data_in_out.take() {
                            ////info!("client: connected");
                            ////// we have a connection, something connected to the listener on the local
                            ////// pc or possible from somewhere else

                            ////// create writer and reader as frames of Vec<u8>
                            ////let (writer, reader) = socket.framed(RawCodec{id:0,name:"client".to_owned()}).split();
                            ////// for each data read from s3 send the data using writer
                            //////let reader = reader
                                //////.then(move |req| {
                                    //////let _ = data_out_sender.send(req.unwrap()).unwrap();
                                    //////Ok::<Vec<u8>,io::Error>(vec![])
                                //////});
                            ////use futures::future::{loop_fn, ok, Future, FutureResult, Loop};
                            ////let received_data = data_in_receiver
                                ////.then(|value| {
                                    ////value.unwrap()
                                ////})
                                //////.select(reader)
                            ////;
                            ////let writer = writer
                                ////.send_all(reader)
                                ////.then(|_| {
                                    ////info!("writer end");
                                    ////Ok(())
                                ////});
                            ////handle.spawn(writer);
                            ////info!("client: ...");
                            //////use futures::stream::{Stream};
                            //////use futures::future::*;
                            //////let reader = reader
                                //////.then(move |req| {
                                    //////let _ = data_out_sender.send(req.unwrap()).unwrap();
                                    //////Ok::<Vec<u8>,io::Error>(vec![])
                                //////})
                                //////.into_future()
                                //////.then(|_| Ok(()));

                            //////handle.spawn(reader);
                        ////}
                        ////Ok(())
                    ////});
                ////let _ = core.run(runner);
            //};
            ////use tokio_core::net::TcpListener;
            //let listener = TcpListener::bind(&address, &handle).unwrap();
            //let server = listener
                //.incoming()
                //.for_each(move |(socket, _peer_addr)| {
                    //info!("server: connected");
                    //let (writer, reader) = socket.framed(RawCodec{id:0,name:"Server".to_owned()}).split();
                    ////let address = "192.168.1.10:22".parse().unwrap();
                    ////
                    //let address = "10.10.101.146:22".parse().unwrap();
                    //use tokio_core::net::TcpStream;
                    //let client = TcpStream::connect(&address, &handle);
                    //let handle = handle.clone();
                    //client.and_then(move |socket| {
                        //let (writer2, reader2) = socket.framed(RawCodec{id:0,name:"Client".to_owned()}).split();
                        //info!("client: connected");
                        //use futures::Sink;
                        //let server = writer.send_all(reader2).then(|_|Ok(()));
                        //let client = writer2.send_all(reader).then(|_|Ok(()));
                        //handle.spawn(server);
                        //handle.spawn(client);
                        //use tokio_core::net::UdpSocket;
                        //let address = "0.0.0.0:48000".parse().unwrap();
                        //let server = UdpSocket::bind(&address, &handle).unwrap();
                        //info!("server");
                        //let (writer, reader) = server.framed(RawCodec{id:0,name:"Server".to_owned()}).split();
                        //let address = "0.0.0.0:60000".parse().unwrap();
                        //let client = UdpSocket::bind(&address, &handle).unwrap();
                        //let (writer2, reader2) = client.framed(RawCodec{id:0,name:"Client".to_owned()}).split();
                        //let server = writer2
                            //.send_all(reader
                                      //.map(|(_,msg)| {
                                          //let address = "10.10.101.146:60000".parse().unwrap();
                                          //(address, msg)
                                      //}));
                        ////let client = writer
                            ////.send_all(reader2
                                    ////.map(|(_,msg)| {
                                          ////let address = "0.0.0.0:47999".parse().unwrap();
                                          ////(address, msg)
                                    ////}));
                        ////let writer = writer.send_all(reader);
                        //handle.spawn(server.then(|_| Ok(())));
                        ////handle.spawn(client.then(|_| Ok(())));
                        //Ok(())
                    //})
                //});
            //let _ = core.run(server);
        })
}

use clap::ArgMatches;

fn main() {
    use clap::{App,Arg};
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(crate_version!())
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(Arg::with_name("log-config")
             .long("log-config")
             .help("Log configuration")
             .takes_value(true)
             .default_value("log.yaml")
            )
        .arg(Arg::with_name("port")
             .long("port")
             .short("p")
             .takes_value(true)
             .default_value("1234")
             .validator(|val| val.parse::<u16>().map(|_| ()).map_err(|_| format!("Cannot parse {} to u16", val))))
        .arg(Arg::with_name("mode")
             .help("What mode to run the tunnel in")
             .index(1)
             .possible_values(&["server", "client"])
             .required(true))
        .get_matches();
    let _ = log4rs::init_file(&matches.value_of("log-config").unwrap(), Default::default()).unwrap();
    let _ = s3run(&matches).unwrap();
}


#[test]
fn serde_loading() {
    let mut stream = Vec::new();
    stream.push(Message {
        id: 3,
        payload: Payload::Connect(1),
    });

    stream.push(Message {
        id: 4,
        payload: Payload::Data(1, vec![0,1,2,3,4]),
    });

    stream.push(Message {
        id: 5,
        payload: Payload::Data(1, vec![0xff,1,2,3,4]),
    });
    stream.push(Message {
        id: 6,
        payload: Payload::Sync(1, 5),
    });
    let s = serde_yaml::to_string(&stream).unwrap();
    println!("{}", s);

    let stream: Vec<Message> = serde_yaml::from_str(&s).unwrap();

    println!("{:#?}", stream);
}

