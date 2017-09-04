/**
 * File: src/main.rs
 * Author: Anicka Burova <anicka.burova@gmail.com>
 * Date: 04.09.2017
 * Last Modified Date: 04.09.2017
 * Last Modified By: Anicka Burova <anicka.burova@gmail.com>
 */
extern crate s3;
extern crate ini;

use s3::bucket::Bucket;
use s3::credentials::Credentials;
use std::io::{self};


fn load_credentials() -> io::Result<(Credentials,String)> {
    use ini::Ini;
    Ini::load_from_file("/home/milan/.s3cfg")
        .map_err(|err| {
            io::Error::new(io::ErrorKind::InvalidData, err)
        })
        .and_then(|cfg| {
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
                println!("{}\n{}\n{}", access_key, secret_key, region);
                Ok((Credentials::new(&access_key, &secret_key, None), region.to_owned()))
            })
        })
}

fn main() {
    let _ = load_credentials()
        .and_then(|(credentials, region)| {
            region
                .parse()
                .map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "Failed to parse region")
                })
                .and_then(|region| {
                    Ok(Bucket::new("imsdistributionfiles", region, credentials))
                })
        })
        .and_then(|bucket| {
            bucket
                .list("/exchange", None)
                .and_then(|(list, code)| {
                    println!("code = {}\nlist = {:?}", code, list);
                    Ok(())
                })
                .map_err(|err| {
                    println!("failed: {}", err.description());
                    io::Error::new(io::ErrorKind::InvalidData, err.description().to_owned())
                })
        })
        .unwrap();
    println!("Hello, world!");
}
