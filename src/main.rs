/**
 * File: src/main.rs
 * Author: Anicka Burova <anicka.burova@gmail.com>
 * Date: 04.09.2017
 * Last Modified Date: 04.09.2017
 * Last Modified By: Anicka Burova <anicka.burova@gmail.com>
 */
extern crate aws_sdk_rust;
extern crate ini;

use std::io::{self};


fn main() {
    use ini::Ini;
    let _ = Ini::load_from_file("/home/milan/.s3cfg")
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
                let bucket_name = "imsdistributionfiles";
                use aws_sdk_rust::aws::s3::object::PutObjectRequest;
                let mut object = PutObjectRequest::default();
                object.bucket = bucket_name.to_string();
                object.key = "exchange/wal-com.in".to_string();
                object.body = Some(b"this is a test.");
                match client.put_object(&object, None) {
                    Ok(output) => println!( "{:#?}", output),
                    Err(e) => println!("{:#?}", e),
                }
                // read s3 files
                use aws_sdk_rust::aws::s3::object::GetObjectRequest;
                let mut object = GetObjectRequest::default();
                object.bucket = bucket_name.to_string();
                object.key = "exchange/wal-com.out".to_string();
                use std::str;
                match client.get_object(&object, None) {
                    Ok(output) => println!( "\n\n{:#?}\n\n", str::from_utf8(&output.body).unwrap()),
                    Err(e) => println!( "{:#?}", e),
                }
                Ok(())
            })
        })
        .unwrap();
}
