/**
 * File: src/s3tunnel.rs
 * Author: Anicka Burova <anicka.burova@gmail.com>
 * Date: 07.09.2017
 * Last Modified Date: 08.09.2017
 * Last Modified By: Anicka Burova <anicka.burova@gmail.com>
 */

use std::io::{self};
use config::S3Config;
use serde_yaml;

use std::sync::mpsc::{Sender, Receiver, channel};

use messages::*;

pub struct Tunnel {
    pub writer: Sender<Vec<u8>>,
    pub reader: Receiver<Vec<u8>>,
    pub connection: Option<Receiver<u64>>,
}

pub struct Pipes {
    pub writer: Sender<Message>,
    pub reader: Receiver<Message>,
}


pub fn create_clients(is_server: bool, cfg: S3Config, writer_name: &str, reader_name: &str) -> io::Result<Tunnel> {
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
            info!("Creating credentials");
            DefaultCredentialsProvider::new(Some(credentials))
        })
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
        .and_then(|provider| {
            info!("Creating s3 client");
            use std::str::FromStr;
            use aws_sdk_rust::aws::common::region::Region;
            Region::from_str(&bucket_location)
                .and_then(|region| {
                    use aws_sdk_rust::aws::s3::endpoint::{Endpoint, Signature};
                    // endpoint can be cloned so just one is enough to create
                    Ok(Endpoint::new(region, Signature::V4, None, None, None, None))
                })
                .and_then(|endpoint| {
                    use aws_sdk_rust::aws::s3::s3client::S3Client;
                    Ok((    S3Client::new(provider, endpoint)
                            ,bucket_name
                            ,bucket_prefix))
                })
                .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
        })
        .map(|a| {
            info!("S3 connection created");
            a
        })
        .and_then(|(client, bucket_name, bucket_prefix)| {
            let (writer_sender, writer_receiver) = channel::<Vec<u8>>();
            let (reader_sender, reader_receiver) = channel();
            let (connection_sender, connection_receiver) = if !is_server {
                let (sender, receiver) = channel();
                (Some(sender), Some(receiver))
            } else {
                (None, None)
            };
            info!("Reading data from '{}'", reader_name);
            info!("Writing data to '{}'", writer_name);
            let reader_name: String = reader_name.into();
            let writer_name: String = writer_name.into();
            use std::thread;
            use std::thread::sleep;
            use std::time::Duration;
            use std::str;
            let _ = thread::spawn(move|| {
                info!("Tunnel thread started");
                let mut reader_sync = 0;
                let mut writer_sync = 0;
                use aws_sdk_rust::aws::s3::object::{GetObjectRequest};
                let mut reader = GetObjectRequest::default();
                reader.bucket = bucket_name.clone();
                reader.key = format!("{}/{}", bucket_prefix, reader_name);
                // at the begining, remove tunnel files
                if is_server {
                    use aws_sdk_rust::aws::s3::object::{DeleteObjectRequest};
                    let mut del = DeleteObjectRequest::default();
                    del.bucket = bucket_name.clone();
                    del.key = format!("{}/{}", bucket_prefix, reader_name);
                    match client.delete_object(&del, None) {
                        Ok(_) => info!( "Tunnel file {} is removed", reader_name),
                        Err(e) => error!("{:#?}", e),
                    }
                    del.key = format!("{}/{}", bucket_prefix, writer_name);
                    match client.delete_object(&del, None) {
                        Ok(_) => info!( "Tunnel file {} is removed", writer_name),
                        Err(e) => error!("{:#?}", e),
                    }
                }
                let mut all_msgs: Vec<Message> = Vec::new(); // all the messages which are writen to the tunnel writer.
                let mut msg_id = 1; // start messages from 1, so we can keep writer_sync from 0
                loop {
                    let last_writer_sync = writer_sync;
                    let last_reader_sync = reader_sync;
                    // Reading tunnel input
                    info!("Reading tunnel input: {}", reader_name);
                    match client.get_object(&reader, None) {
                        Ok(output) => {
                            let text = str::from_utf8(&output.body).unwrap();
                            let mut stream: Vec<Message> = serde_yaml::from_str(text).unwrap();
                            for msg in stream.drain(..) {
                                if reader_sync >= msg.id {
                                    // if id of the message has already been processed
                                    continue;
                                }
                                reader_sync = msg.id;
                                match msg.payload {
                                    Payload::Connect(id) => {
                                        // start a connection!
                                        info!("Got connection");
                                        match connection_sender.as_ref() {
                                            Some(ref connection_sender) => connection_sender.send(id).unwrap(),
                                            None => (),
                                        }
                                    }
                                    Payload::Data(_, data) => {
                                        info!("Got data");
                                        reader_sender.send(data).unwrap();
                                    }
                                    Payload::Sync(_id, last_msg) => {
                                        info!("Got sync: {}", last_msg);
                                        writer_sync = last_msg;
                                    }
                                }
                            }

                            sleep(Duration::from_millis(200));
                        }
                        Err(_) => {
                            // if the file is missing, just wait until it is created
                            info!("{} not found", reader_name);
                            sleep(Duration::from_millis(2000));
                        }
                    }

                    if last_writer_sync < writer_sync {
                        // the other side has read messages up to writer_sync, we can
                        // filter them out
                        let remove_all = all_msgs
                            .last()
                            .map(|last| last.id == writer_sync )
                            .unwrap_or(false);
                        // little performance improvement in case we can just clear all the
                        // messages
                        if remove_all {
                            all_msgs.clear();
                        } else {
                            // otherwise filter all older then writer_sync
                            let tmp = all_msgs
                                .drain(..)
                                .filter(|msg| msg.id > writer_sync)
                                .collect();
                            all_msgs = tmp;
                        }
                    }

                    let mut is_change = false;

                    let mut new_msgs = writer_receiver.try_iter().collect::<Vec<Vec<u8>>>();

                    if new_msgs.len() > 0 {

                        if last_reader_sync < reader_sync {
                            // We read another messages from the tunnel, let the other side know, we
                            // have them and it doesn't need to send them anymore.
                            let id = msg_id;
                            msg_id += 1;
                            all_msgs
                                .push(Message {
                                    id,
                                    // For now, id of the connection is just 1, until multiple
                                    // connections are implemented.
                                    payload: Payload::Sync(1, reader_sync),
                                });
                        }
                        // if this is a server, and this is the first message to send, send connect
                        // first
                        if msg_id == 1 && is_server {
                            let id = msg_id;
                            msg_id += 1;
                            all_msgs
                                .push(Message {
                                    id,
                                    // For now, id of the connection is just 1, until multiple
                                    // connections are implemented.
                                    payload: Payload::Connect(1),
                                });
                        }
                        // add the new messages to all
                        all_msgs
                            .extend( new_msgs
                                     .drain(..)
                                     .filter(|data| data.len()>0)
                                     .map(|data| {
                                         is_change = true;
                                         let id = msg_id;
                                         msg_id += 1;
                                         Message {
                                             id,
                                             payload: Payload::Data(1, data),
                                         }
                                     }));
                    }

                    if is_change {
                        let msg = serde_yaml::to_string(&all_msgs).unwrap();
                        let msg = msg.bytes().collect::<Vec<u8>>();
                        use aws_sdk_rust::aws::s3::object::{PutObjectRequest};
                        let mut stream_out = PutObjectRequest::default();
                        stream_out.bucket = bucket_name.clone();
                        stream_out.key = format!("{}/{}", bucket_prefix, writer_name);
                        stream_out.body = Some(&msg);
                        match client.put_object(&stream_out, None) {
                            Ok(output) => info!( "{:#?}", output),
                            Err(e) => error!("{:#?}", e),
                        }
                    }
                }
            });

            Ok(Tunnel {
                writer: writer_sender,
                reader: reader_receiver,
                connection: connection_receiver,
            })
        })
}
