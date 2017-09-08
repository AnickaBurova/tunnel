/**
 * File: src/s3init.rs
 * Author: Anicka Burova <anicka.burova@gmail.com>
 * Date: 07.09.2017
 * Last Modified Date: 07.09.2017
 * Last Modified By: Anicka Burova <anicka.burova@gmail.com>
 */

use std::io::{self};
use self::config::S3Config;
use aws_sdk_rust::aws::s3::s3client::S3Client;

pub fn inits3(cfg: S3Config) -> io::Result<(S3Client, S3Client, String, String)> {
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
            DefaultCredentialsProvider::new(Some(credentials.clone()))
                .and_then(|provider| {
                    DefaultCredentialsProvider::new(Some(credentials.clone()))
                        .and_then(|provider2| Ok((provider, provider2)))
                })
        })
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
        .and_then(|(provider, provider2)| {
            info!("Creating clients");
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
        .map(|a| {
            info!("S3 connections created");
            a
        })
}
