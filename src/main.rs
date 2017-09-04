/**
 * File: src/main.rs
 * Author: Anicka Burova <anicka.burova@gmail.com>
 * Date: 04.09.2017
 * Last Modified Date: 04.09.2017
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

use std::io::{self};

pub struct RawCodec {
    pub id: u64,
    pub name: String,
}

use tokio_io::codec::{Encoder, Decoder};
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
                    info!("{}[{}]: {}", self.name, self.id, s);
                }
                Err(_) => {
                    info!("{}[{}]: {}", self.name, self.id, line.len());
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

use tokio_io::{AsyncRead};
use futures::{Future};

fn s3run() -> io::Result<()> {
    use ini::Ini;
    Ini::load_from_file("/home/anca/.s3cfg")
        .map_err(|err| {
            io::Error::new(io::ErrorKind::InvalidData, err)
        })
        .and_then(|cfg| {
            // read s3 configuration from config file.
            cfg.section(Some("default".to_owned()))
                .ok_or(io::Error::new(io::ErrorKind::InvalidData, "Cannot read default section"))
                .and_then(|section| {
                    section.get("access_key")
                        .ok_or(io::Error::new(io::ErrorKind::InvalidData, "Cannot read access_key value"))
                        .and_then(|access_key| {
                            section.get("secret_key")
                                .ok_or(io::Error::new(io::ErrorKind::InvalidData, "Cannot read secret_key value"))
                                .and_then(|secret_key| {
                                    Ok((access_key, secret_key))
                                })
                        })
                        .and_then(|(access_key, secret_key)| {
                            section.get("bucket_location")
                                .ok_or(io::Error::new(io::ErrorKind::InvalidData, "Cannot read bucket_location value"))
                                .and_then(|region| {
                                    Ok((access_key, secret_key, region))
                                })
                        })
                })
            .and_then(|(access_key, secret_key, region)| {
                // create connection to s3
                info!("{}\n{}\n{}", access_key, secret_key, region);
                use aws_sdk_rust::aws::common::credentials::{DefaultCredentialsProvider,ParametersProvider};
                ParametersProvider::with_parameters(
                    access_key.to_owned(),
                    secret_key.to_owned(),
                    None)
                    .and_then(|credentials| {
                        DefaultCredentialsProvider::new(Some(credentials))
                    })
                    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
                    .and_then(|provider| {
                        use std::str::FromStr;
                        use aws_sdk_rust::aws::common::region::Region;
                        Region::from_str(region)
                            .and_then(|region| {
                                use aws_sdk_rust::aws::s3::endpoint::{Endpoint, Signature};
                                Ok(Endpoint::new(region, Signature::V4, None, None, None, None))
                            })
                            .and_then(|endpoint| {
                                use aws_sdk_rust::aws::s3::s3client::S3Client;
                                Ok(S3Client::new(provider, endpoint))
                            })
                            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
                    })
            })
            .and_then(|client| {
                let bucket_name = "bucket_name";
                use aws_sdk_rust::aws::s3::object::PutObjectRequest;
                let mut object = PutObjectRequest::default();
                object.bucket = bucket_name.to_string();
                object.key = "exchange/tunnel.in".to_string();
                object.body = Some(b"this is a test.");
                match client.put_object(&object, None) {
                    Ok(output) => info!( "{:#?}", output),
                    Err(e) => info!("{:#?}", e),
                }
                // read s3 files
                use aws_sdk_rust::aws::s3::object::GetObjectRequest;
                let mut object = GetObjectRequest::default();
                object.bucket = bucket_name.to_string();
                object.key = "exchange/tunnel.out".to_string();
                use std::str;
                match client.get_object(&object, None) {
                    Ok(output) => info!( "\n\n{:#?}\n\n", str::from_utf8(&output.body).unwrap()),
                    Err(e) => info!( "{:#?}", e),
                }
                Ok(())
            })
        })
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
        .get_matches();
    let _ = log4rs::init_file(&matches.value_of("log-config").unwrap(), Default::default()).unwrap();
    //let _ = s3run().unwrap();
    let address = format!("0.0.0.0:{}", &matches.value_of("port").unwrap()).parse().unwrap();
    use tokio_core::reactor::Core;
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    use tokio_core::net::TcpListener;
    let listener = TcpListener::bind(&address, &handle).unwrap();
    use futures::stream::Stream;
    let server = listener
        .incoming()
        .for_each(move |(socket, _peer_addr)| {
            info!("server: connected");
            let (writer, reader) = socket.framed(RawCodec{id:0,name:"Server".to_owned()}).split();
            //let address = "192.168.1.10:22".parse().unwrap();
            //
            let address = "10.10.101.146:22".parse().unwrap();
            use tokio_core::net::TcpStream;
            let client = TcpStream::connect(&address, &handle);
            let handle = handle.clone();
            client.and_then(move |socket| {
                let (writer2, reader2) = socket.framed(RawCodec{id:0,name:"Client".to_owned()}).split();
                info!("client: connected");
                use futures::Sink;
                let server = writer.send_all(reader2).then(|_|Ok(()));
                let client = writer2.send_all(reader).then(|_|Ok(()));
                handle.spawn(server);
                handle.spawn(client);
                Ok(())
            })
        });
    let _ = core.run(server);
}
