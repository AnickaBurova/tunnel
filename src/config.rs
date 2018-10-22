/**
 * File: src/config.rs
 * Author: Anicka Burova <anicka.burova@gmail.com>
 * Date: 07.09.2017
 * Last Modified Date: 08.09.2017
 * Last Modified By: Anicka Burova <anicka.burova@gmail.com>
 */

/// S3 configuration data
#[derive (Debug, Serialize, Deserialize)]
pub struct S3Config {
    /// User access key to s3.
    pub access_key: String,
    /// Bucket name.
    pub bucket_name: String,
    /// Bucket prefix.
    pub bucket_prefix: String,
    /// Bucket location.
    pub bucket_location: String,
    /// User secret key.
    pub secret_key: String,
}

use std::io::{self};

/// Load s3 config from file tunnel.cfg
pub fn load_config() -> io::Result<S3Config> {
    use std::fs::File;
    File::open("tunnel.cfg")
        .map_err(|err| {
            io::Error::new(err.kind(), format!("Failed to load tunnel.cfg: {}", err))
        })
        .and_then(|file| {
            info!("Loading");
            use serde_yaml;
            io_res!(serde_yaml::from_reader::<_,S3Config>(file), InvalidData)
        })
        .map(|r| {
            info!("Loaded");
            r
        })
}
