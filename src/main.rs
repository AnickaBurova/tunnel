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

use std::io::{self};

pub struct LineCodec;

use tokio_io::codec::{Encoder, Decoder};
use bytes::BytesMut;

impl Decoder for LineCodec {
    type Item = String;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<String>> {
        if let Some(i) = buf.iter().position(|&b| b == b'\n') {
            // remove the serialized frame from the buffer.
            let line = buf.split_to(i);

            // Also remove the '\n'
            buf.split_to(1);

            // Turn this data into a UTF string and return it in a Frame.
            use std::str;
            match str::from_utf8(&line) {
                Ok(s) => Ok(Some(s.to_string())),
                Err(_) => Err(io::Error::new(io::ErrorKind::Other,
                                             "invalid UTF-8")),
            }
        } else {
            Ok(None)
        }
    }
}

impl Encoder for LineCodec {
    type Item = String;
    type Error = io::Error;

    fn encode(&mut self, msg: String, buf: &mut BytesMut) -> io::Result<()> {
        buf.extend(msg.as_bytes());
        buf.extend(b"\n");
        Ok(())
    }
}

pub struct LineProto;

use tokio_proto::pipeline::ServerProto;
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::codec::Framed;

impl<T: AsyncRead + AsyncWrite + 'static> ServerProto<T> for LineProto {
    /// For this protocol style, `Request` matches the `Item` type of the codec's `Decoder`
    type Request = String;

    /// For this protocol style, `Response` matches the `Item` type of the codec's `Encoder`
    type Response = String;

    /// A bit of boilerplate to hook in the codec:
    type Transport = Framed<T, LineCodec>;
    type BindTransport = Result<Self::Transport, io::Error>;
    fn bind_transport(&self, io: T) -> Self::BindTransport {
        Ok(io.framed(LineCodec))
    }
}

use tokio_service::Service;
pub struct Echo;
use futures::{future, Future};

impl Service for Echo {
    // These types must match the corresponding protocol types:
    type Request = String;
    type Response = String;

    // For non-streaming protocols, service errors are always io::Error
    type Error = io::Error;

    // The future for computing the response; box it for simplicity.
    type Future = Box<Future<Item = Self::Response, Error =  Self::Error>>;

    // Produce a future for computing a response from a request.
    fn call(&self, req: Self::Request) -> Self::Future {
        // In this case, the response is immediate.
        Box::new(future::ok(req))
    }
}

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
                println!("{}\n{}\n{}", access_key, secret_key, region);
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
                    Ok(output) => println!( "{:#?}", output),
                    Err(e) => println!("{:#?}", e),
                }
                // read s3 files
                use aws_sdk_rust::aws::s3::object::GetObjectRequest;
                let mut object = GetObjectRequest::default();
                object.bucket = bucket_name.to_string();
                object.key = "exchange/tunnel.out".to_string();
                use std::str;
                match client.get_object(&object, None) {
                    Ok(output) => println!( "\n\n{:#?}\n\n", str::from_utf8(&output.body).unwrap()),
                    Err(e) => println!( "{:#?}", e),
                }
                Ok(())
            })
        })
}

fn main() {
    //let _ = s3run().unwrap();
    let address = format!("0.0.0.0:{}", Some("1234").unwrap()).parse().unwrap();
    use tokio_core::reactor::Core;
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    use tokio_core::net::TcpListener;
    let listener = TcpListener::bind(&address, &handle).unwrap();
    use futures::stream::Stream;
    let server = listener
        .incoming()
        .for_each(move |(socket, _peer_addr)| {
            println!("server: connected");
            let (writer, reader) = socket.framed(LineCodec).split();
            use std::net::{IpAddr, Ipv4Addr, SocketAddr};
            let address = "79.70.25.97:22".parse().unwrap();
            use tokio_core::net::TcpStream;
            let client = TcpStream::connect(&address, &handle);
            client.and_then(|socket| {
                let (writer2, reader2) = socket.framed(LineCodec).split();
                println!("client: connected");
                use futures::Sink;
                let server = writer.send_all(reader2).then(|_|Ok(()));
                let client = writer2.send_all(reader).then(|_|Ok(()));
                //let server_reads = reader
                    //.and_then(move |req| {

                    //})

                handle.spawn(server);
                handle.spawn(client);
                Ok(())
            })
        });
    core.run(server);
}
