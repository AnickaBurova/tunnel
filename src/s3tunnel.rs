/**
 * File: src/s3tunnel.rs
 * Author: Anicka Burova <anicka.burova@gmail.com>
 * Date: 07.09.2017
 * Last Modified Date: 07.09.2017
 * Last Modified By: Anicka Burova <anicka.burova@gmail.com>
 */

use std::io::{self};
use config::S3Config;
use aws_sdk_rust::aws::s3::s3client::S3Client;
use serde_yaml;

use std::sync::mpsc::{Sender, Receiver, channel};

use messages::*;

pub struct Tunnel {
    pub writer: Sender<Vec<u8>>,
    pub reader: Receiver<Vec<u8>>,
    pub connection: Option<Receiver<u64>>,
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

            use aws_sdk_rust::aws::s3::object::{GetObjectRequest};
            let mut reader = GetObjectRequest::default();
            reader.bucket = bucket_name.clone();
            reader.key = reader_name.into();
            use std::thread;
            use std::thread::sleep;
            use std::time::Duration;
            use std::str;
            let _ = thread::spawn(move|| {
                let mut reader_sync = 0;
                let mut writer_sync = 0;
                loop {
                    let mut last_reader_sync = reader_sync;
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
                                        let len = data.len();
                                        reader_sender.send(data).unwrap();
                                    }
                                    Payload::Sync(id, last_msg) => {
                                        info!("Got sync: {}", last_msg);
                                        writer_sync = last_msg;
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
                    for msg in writer_receiver.try_iter() {
                        use std::str;
                        match str::from_utf8(&msg) {
                            Ok(text) => {
                                info!("Getting '{}'", text);
                            }
                            Err(err) => {
                                error!("Failed to convert to text: {}", err);
                            }
                        }
                    }
                    sleep(Duration::from_secs(1));
                }
            });
            //use std::sync::Arc;
            //use std::sync::atomic::{AtomicUsize, Ordering};
            //// When sending data over s3, keep writing all the data until we receive
            //// confirmation what data was last read by the other side, then we dont need to send
            //// that data anymore.
            //let last_msg_in = Arc::new(AtomicUsize::new(0));
            //let last_msg_out = Arc::new(AtomicUsize::new(0));

            //let (stream_in, stream_out) = {
                //info!("Using ({}, {}) for s3 files", name_in, name_out);
                //use aws_sdk_rust::aws::s3::object::{GetObjectRequest};
                //// get object ('stream_in') can be created just once here and then reused in the
                //// worker thread.
                //let mut stream_in = GetObjectRequest::default();
                //stream_in.bucket = bucket_name.clone();
                //stream_in.key = name_in;
                //// put object has to be always created when it is going to be used, because output
                //// data has lifetime of the put object
                //(stream_in, (bucket_name, name_out))
            //};

            Ok(Tunnel {
                writer: writer_sender,
                reader: reader_receiver,
                connection: connection_receiver,
            })
        })
}


// reader
            //let last_msg_in_clone = last_msg_in.clone();
            //let last_msg_out_clone = last_msg_out.clone();
            //let allow_connection_clone = allow_connection.as_ref().and_then(|a| Some(a.clone()));
            //let s3_reader = thread::spawn(move || {
                //info!("Starting s3 reader");
                //let last_msg_in = last_msg_in_clone;
                //let last_msg_out = last_msg_out_clone;
                //let allow_connection = allow_connection_clone;
                //let client = worker_client;
                //let mut data_in_sender = data_in_sender;
                //// read s3 files
                //loop {
                    //use std::str;
                    //use std::thread::sleep;
                    //use std::time::Duration;
                    //match client.get_object(&stream_in, None) {
                        //Ok(output) => {
                            //let text = str::from_utf8(&output.body).unwrap();
                            //let mut stream: Vec<Message> = serde_yaml::from_str(text).unwrap();
                            //let mut last_id = last_msg_in.load(Ordering::SeqCst);
                            //let mut max_id = 0;
                            //for msg in stream.drain(..) {
                                //use std::cmp::max;
                                //max_id = max(max_id, msg.id);
                                //if last_id >= msg.id {
                                    //// if id of the message has already been processed
                                    //continue;
                                //}
                                //last_id = msg.id;
                                //match msg.payload {
                                    //Payload::Connect(id) => {
                                        //// start a connection!
                                        //info!("Got connection");
                                        //match allow_connection {
                                            //Some(ref ac) => ac.store(true, Ordering::Relaxed),
                                            //None => (),
                                        //}
                                    //}
                                    //Payload::Data(id, data) => {
                                        //// send data to the tunnel
                                        //use futures::{Sink};
                                        //info!("Got data");
                                        //let len = data.len();
                                        //match data_in_sender.send(Ok(data)).wait() {
                                            //Ok(t) => {
                                                //info!("Data send to tcp receiver: {}", len);
                                                //data_in_sender = t;
                                            //}
                                            //Err(e) => {
                                                //error!("Error: {}", e);
                                                //return;
                                            //}
                                        //}
                                    //}
                                    //Payload::Sync(id, last_msg) => {
                                        //info!("Got sync: {}", last_msg);
                                        //// set last message to out to this value (anything before
                                        //// that can be from now ignored)
                                        //last_msg_out.store(last_msg, Ordering::SeqCst);
                                    //}
                                //}
                            //}
                            //last_msg_in.store(last_id, Ordering::SeqCst);
                            //sleep(Duration::from_millis(200));
                        //}
                        //Err(e) => {
                            //// if the file is missing, just wait until it is created
                            //sleep(Duration::from_millis(2000));
                        //}
                    //}
                //}
            //});

// writer
             //worker to write data stream out
            //let worker_out = thread::spawn(move || {
                //info!("Started worker_out");
                //let mut all_msgs: Vec<Message> = Vec::new();
                //let mut msg_id = 1;
                //let mut last_msg_in_stored = 0;
                //let (bucket_name, name_out) = stream_out;
                //loop {
                     //get new msgs
                    //let mut msgs = data_out_receiver.try_iter().collect::<Vec<Vec<u8>>>();
                    //if msgs.len() == 0 {
                        //use std::thread::sleep;
                        //use std::time::Duration;
                        //sleep(Duration::from_millis(500));
                        //continue;
                    //}
                    //if msg_id == 1 {  send connect
                        //let id = msg_id;
                        //msg_id += 1;
                        //all_msgs.push(Message {
                            //id,
                            //payload: Payload::Connect(1),
                        //});
                    //}
                    //info!("There are {} new messages in worker_out", msgs.len());
                     //remove any msg, which has been already received by the other end
                    //let last_msg_out = last_msg_out.load(Ordering::SeqCst);
                    //let mut tmp_all_msgs = all_msgs;
                    //all_msgs = tmp_all_msgs
                        //.drain(..)
                        //.filter(|msg| msg.id > last_msg_out)
                        //.collect();
                    //all_msgs
                        //.extend(msgs.drain(..).map(|data| {
                            //let id = msg_id;
                            //msg_id += 1;
                            //Message {
                                //id,
                                //payload: Payload::Data(1, data)
                            //}
                        //}));
                    //let last_msg_in_sync = last_msg_in.load(Ordering::SeqCst);
                    //if last_msg_in_sync > last_msg_in_stored {
                        //last_msg_in_stored = last_msg_in_sync;
                        //let id = msg_id;
                        //msg_id += 1;
                        //all_msgs.push(Message {
                            //id,
                            //payload: Payload::Sync(1, last_msg_in_stored),
                        //});
                    //}
                    //let data_msg = serde_yaml::to_string(&all_msgs).unwrap();
                    //let buf = data_msg.bytes().collect::<Vec<u8>>();
                    //use aws_sdk_rust::aws::s3::object::{PutObjectRequest};
                    //let mut stream_out = PutObjectRequest::default();
                    //stream_out.bucket = bucket_name.clone();
                    //stream_out.key = name_out.clone();
                    //stream_out.body = Some(&buf);
                    //match client.put_object(&stream_out, None) {
                        //Ok(output) => info!( "{:#?}", output),
                        //Err(e) => error!("{:#?}", e),
                    //}
                //}
            //});

