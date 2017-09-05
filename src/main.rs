/**
 * File: src/main.rs
 * Author: Anicka Burova <anicka.burova@gmail.com>
 * Date: 04.09.2017
 * Last Modified Date: 06.09.2017
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

#[derive (Debug, Serialize, Deserialize)]
enum Payload {
    /// Create connection with id
    Connect(u64),
    /// For connection id, pass data
    Data(u64, Vec<u8>),
    /// Connection with id is synchronised up to msg id
    Sync(u64, usize),
}

#[derive (Debug, Serialize, Deserialize)]
struct Message {
    pub id: usize,
    pub payload: Payload,
}

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

use std::net::SocketAddr;

impl UdpCodec for RawCodec {
    type In = (SocketAddr, Vec<u8>);
    type Out = (SocketAddr, Vec<u8>);
    fn decode(&mut self, src: &SocketAddr, buf: &[u8]) -> io::Result<Self::In> {
        let size = buf.len();
        let line = buf.to_vec();
        self.id += 1;
        use std::str;
        match str::from_utf8(&line) {
            Ok(s) => {
                info!("Udp-{}[{}]: {}", self.name, self.id, s);
            }
            Err(_) => {
                info!("Udp-{}[{}]: {}", self.name, self.id, line.len());
            }
        }
        Ok((*src, line))
    }
    fn encode(&mut self, (addr, msg): Self::Out, buf: &mut Vec<u8>) -> SocketAddr {
        buf.extend(msg);
        addr
    }
}

macro_rules! io_res {
    ($e: expr) => {
        $e.map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    };
    ($e: expr, $k: ident) => {
        $e.map_err(|e| io::Error::new(io::ErrorKind::$k, e))
    };
    (opt => $e: expr, $msg: expr) => {
        $e.ok_or(io::Error::new(io::ErrorKind::Other, $msg))
    };
    (opt => $e: expr, $k: ident, $msg: expr) => {
        $e.ok_or(io::Error::new(io::ErrorKind::$k, $msg))
    };
}

use tokio_io::{AsyncRead};
use futures::{Future};

fn s3run(matches: &ArgMatches) -> io::Result<()> {
    #[derive (Debug, Serialize, Deserialize)]
    struct S3Config {
        pub access_key: String,
        pub bucket_name: String,
        pub bucket_prefix: String,
        pub bucket_location: String,
        pub secret_key: String,
    }

    let mode = &matches.value_of("mode").unwrap();
    let port = &matches.value_of("port").unwrap();

    use std::collections::BTreeMap;
    use std::fs::File;
    use std::io::Read;
    File::open("tunnel.cfg")
        .and_then(|file| {
            io_res!(serde_yaml::from_reader::<_,S3Config>(file), InvalidData)
        })
        .and_then(|cfg| {
            // create connection to s3
            let access_key = cfg.access_key;
            let bucket_name = cfg.bucket_name;
            let bucket_prefix = cfg.bucket_prefix;
            let bucket_location = cfg.bucket_location;
            let secret_key = cfg.secret_key;
            use aws_sdk_rust::aws::common::credentials::{DefaultCredentialsProvider,ParametersProvider};
            ParametersProvider::with_parameters(
                access_key,
                secret_key,
                None)
                .and_then(|credentials| {
                    DefaultCredentialsProvider::new(Some(credentials.clone()))
                        .and_then(|provider| {
                            DefaultCredentialsProvider::new(Some(credentials.clone()))
                                .and_then(|provider2| Ok((provider, provider2)))
                        })
                })
                .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
                .and_then(|(provider, provider2)| {
                    use std::str::FromStr;
                    use aws_sdk_rust::aws::common::region::Region;
                    Region::from_str(&bucket_location)
                        .and_then(|region| {
                            use aws_sdk_rust::aws::s3::endpoint::{Endpoint, Signature};
                            Ok(Endpoint::new(region, Signature::V4, None, None, None, None))
                        })
                        .and_then(|endpoint| {
                            use aws_sdk_rust::aws::s3::s3client::S3Client;
                            Ok((     S3Client::new(provider, endpoint.clone())
                                    ,S3Client::new(provider2, endpoint)
                                    ,bucket_name
                                    ,bucket_prefix))
                        })
                        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
                })
        })
        .and_then(|(worker_client, client, bucket_name, bucket_prefix)| {
            #[derive(PartialEq)]
            enum Mode {
                Server,
                Client,
            }

            let mode = if mode == &"server" { Mode::Server } else { Mode::Client };
            let (mut stream_in, mut stream_out) = {
                let (name_in, name_out) = if mode == Mode::Server {
                    (format!("{}/tunnel.in", bucket_prefix), format!("{}/tunnel.out", bucket_prefix))
                } else {
                    (format!("{}/tunnel.out", bucket_prefix), format!("{}/tunnel.in", bucket_prefix))
                };
                use aws_sdk_rust::aws::s3::object::{PutObjectRequest, GetObjectRequest};
                let mut stream_in = GetObjectRequest::default();
                stream_in.bucket = bucket_name.clone();
                stream_in.key = name_in;
                let mut stream_out = PutObjectRequest::default();
                stream_out.bucket = bucket_name.clone();
                stream_out.key = name_out;
                (stream_in, stream_out)
            };
            use std::sync::atomic::AtomicBool;
            let allow_connection = if mode == Mode::Server {
                None
            } else {
                Some(AtomicBool::new(false))
            };
            // worker reads data and sends it throght channel data_in_sender,
            let (mut data_in_sender, data_in_receiver) = {
                use futures::sync::mpsc;
                mpsc::channel::<Result<Vec<u8>, io::Error>>(5)
            };

            use std::sync::Arc;
            use std::sync::atomic::{AtomicUsize, Ordering};
            // when sending data over s3 we will keep writing all data until we receive
            // confirmation what data was last read by the other side, then we dont need to send
            // that data anymore.
            let last_msg_in = Arc::new(AtomicUsize::new(0));
            let last_msg_out = Arc::new(AtomicUsize::new(0));

            use std::thread;
            // worker which reads periodically stream_in file and send any new changes towards the
            // tunnel
            let last_msg_in_clone = last_msg_in.clone();
            let last_msg_out_clone = last_msg_out.clone();
            let worker_in = thread::spawn(move || {
                let last_msg_in = last_msg_in_clone;
                let last_msg_out = last_msg_out_clone;
                let client = worker_client;
                // read s3 files
                loop {
                    use std::str;
                    use std::thread::sleep;
                    use std::time::Duration;
                    match client.get_object(&stream_in, None) {
                        Ok(output) => {
                            let text = str::from_utf8(&output.body).unwrap();
                            let mut stream: Vec<Message> = serde_yaml::from_str(text).unwrap();
                            let last_id = last_msg_in.load(Ordering::SeqCst);
                            let mut max_id = 0;
                            for msg in stream.drain(..) {
                                use std::cmp::max;
                                max_id = max(max_id, msg.id);
                                if last_id > msg.id {
                                    // if id of the message has already been processed
                                    continue;
                                }
                                match msg.payload {
                                    Payload::Connect(id) => {
                                        // start a connection!
                                        match allow_connection {
                                            Some(ref ac) => ac.store(true, Ordering::Relaxed),
                                            None => (),
                                        }
                                    }
                                    Payload::Data(id, data) => {
                                        // send data to the tunnel
                                        use futures::{Sink};
                                        match data_in_sender.send(Ok(data)).wait() {
                                            Err(_) => return,
                                            Ok(t) => data_in_sender = t,
                                        }
                                    }
                                    Payload::Sync(id, last_msg) => {
                                        // set last message to out to this value (anything before
                                        // that can be from now ignored)
                                        last_msg_out.store(last_msg, Ordering::SeqCst);
                                    }
                                }
                            }
                            sleep(Duration::from_millis(200));
                        }
                        Err(e) => {
                            // if the file is missing, just wait until it is created
                            sleep(Duration::from_millis(2000));
                        }
                    }
                }
            });

            // channels to send and receive data to send over s3
            use std::sync::mpsc::{Sender, Receiver};
            let (data_out_sender, data_out_receiver) = {
                use std::sync::mpsc;
                mpsc::channel()
            };

            // worker to write data stream out
            let worker_out = thread::spawn(move || {
                let mut all_msgs = Vec::new();
                loop {
                    // get new msgs
                    let msgs = data_out_receiver.try_iter().collect::<Vec<Vec<u8>>>();
                    if msgs.len() == 0 {
                        use std::thread::sleep;
                        use std::time::Duration;
                        sleep(Duration::from_millis(500));
                        continue;
                    }

                }
            });

            use tokio_core::reactor::Core;
            let mut core = Core::new().unwrap();
            let handle = core.handle();
            use futures::stream::Stream;
            use futures::Sink;
            use futures::future::{self};
            // if this is server, then create listener and wait for the connection
            if mode == Mode::Server {
                let address = format!("0.0.0.0:{}",port).parse().unwrap(); // waiting on specific port for incoming connections
                use tokio_core::net::TcpListener;
                let listener = TcpListener::bind(&address, &handle).unwrap();
                let server = listener
                    .incoming()
                    .for_each(move |(socket, _peer_addr)| {
                        info!("server: connected");
                        // we have a connection, something connected to the listener on the local
                        // pc or possible from somewhere else

                        // create writer and reader as frames of Vec<u8>
                        let (writer, reader) = socket.framed(RawCodec{id:0,name:"Server".to_owned()}).split();
                        // for each data read from s3 send the data using writer
                        let received_data = data_in_receiver.and_then(move|req| Box::new(future::ok(req)));
                        let writer = writer.send_all(received_data);
                        Ok(())
                    });
            } else {
                // if this is client, then wait until we are allowed to create connection
                //let address = format!("0.0.0.0:{}",22).parse().unwrap(); // create connection to the ssh
            }

            stream_out.body = Some(b"this is stream_in");
            match client.put_object(&stream_out, None) {
                Ok(output) => info!( "{:#?}", output),
                Err(e) => info!("{:#?}", e),
            }

            //use tokio_core::net::TcpListener;
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
            Ok(())
        })
}

use clap::ArgMatches;

fn tunnel(matches: &ArgMatches) {
}

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

