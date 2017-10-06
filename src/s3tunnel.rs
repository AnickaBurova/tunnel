/**
 * File: src/s3tunnel.rs
 * Author: Anicka Burova <anicka.burova@gmail.com>
 * Date: 07.09.2017
 * Last Modified Date: 06.10.2017
 * Last Modified By: Anicka Burova <anicka.burova@gmail.com>
 */

use std::io::{self};
use config::S3Config;
use serde_yaml;

use std::sync::mpsc::{channel};

use tunnel::*;
use messages::*;
use aws_sdk_rust::aws::s3::object::{ GetObjectRequest, PutObjectRequest};

macro_rules! delete_files {
    ($client: ident, $bucket_name: ident, $bucket_prefix: ident, $files: expr) => {
        use aws_sdk_rust::aws::s3::object::{DeleteObjectRequest};
        let mut del = DeleteObjectRequest::default();
        del.bucket = $bucket_name.clone();
        for file in $files.iter() {
            del.key = format!("{}/{}", $bucket_prefix, file);
            match $client.delete_object(&del, None) {
                Ok(_) => info!( "Tunnel file {} is removed", file),
                Err(e) => error!("{:#?}", e),
            }
        }
    }
}

pub fn create_clients(is_server: bool, cfg: S3Config, writer_name: &str, reader_name: &str)
    -> io::Result<TunnelPipes> {
    //-> io::Result<()> {
    // create connection to s3
    let access_key = cfg.access_key;
    let bucket_name = cfg.bucket_name;
    let bucket_prefix = cfg.bucket_prefix;
    let bucket_location = cfg.bucket_location;
    let secret_key = cfg.secret_key;
    use aws_sdk_rust::aws::common::credentials::{DefaultCredentialsProvider,ParametersProvider};
    // Create s3 connection
    ParametersProvider::with_parameters(
        access_key,
        secret_key,
        None)
        .and_then(|credentials| {
            // Credentials to connect to the s3 cloud
            info!("Creating credentials");
            DefaultCredentialsProvider::new(Some(credentials.clone()))
                //.and_then(|provider| {
                    //DefaultCredentialsProvider::new(Some(credentials))
                        //.and_then(|provider2| {
                            //Ok((provider, provider2))
                        //})
                //})
        })
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
        .and_then(|provider| {
            // Create the s3 client connection
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
                    Ok((     S3Client::new(provider, endpoint)
                            //,S3Client::new(provider2, endpoint)
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
            let (writer_sender, writer_receiver) = channel::<WriteCommand>();
            let (reader_sender, reader_receiver) = channel::<ReadCommand>();

            use std::thread;

            info!("Reading data from '{}'", reader_name);
            info!("Writing data to '{}'", writer_name);
            // create thread own variables to avoid lifetime errors
            let reader_name: String = reader_name.into();
            let writer_name: String = writer_name.into();

            thread::spawn(move|| {
                if is_server {
                    delete_files!(client, bucket_name, bucket_prefix, vec![&reader_name, &writer_name]);
                }
                let mut reader = GetObjectRequest::default();
                reader.bucket = bucket_name.clone();
                reader.key = format!("{}/{}", bucket_prefix, reader_name);
                loop {
                    for cmd in writer_receiver.try_iter() {
                        match cmd {
                            WriteCommand::Write(msg) => {
                                let mut stream_out = PutObjectRequest::default();
                                stream_out.bucket = bucket_name.clone();
                                stream_out.key = format!("{}/{}", bucket_prefix, writer_name);
                                stream_out.body = Some(&msg);
                                match client.put_object(&stream_out, None) {
                                    Ok(_) => {
                                        info!("Message writes, size = {}", msg.len());
                                    }
                                    Err(e) => error!("{:#?}", e),
                                }
                            }
                            WriteCommand::Delete => {
                                delete_files!(client, bucket_name, bucket_name, vec![&writer_name]);
                            }
                        }
                    }

                    match client.get_object(&reader, None) {
                        Ok(output) => {
                            use std::str;
                            let text = str::from_utf8(&output.body).unwrap();
                            let stream: Vec<Message> = serde_yaml::from_str(text).unwrap();

                            let _ = reader_sender.send(ReadCommand::Read(stream)).unwrap();
                            //sleep(Duration::from_millis(800));
                        }
                        Err(_) => {
                            let _ = reader_sender.send(ReadCommand::NoFile).unwrap();
                            // if the file is missing, just wait until it is created
                            //info!("{} not found", reader_name);
                            //sleep(Duration::from_millis(2000));
                        }
                    }
                }
            });


            Ok(TunnelPipes{
                writer: writer_sender,
                reader: reader_receiver,
            })
        })
}



