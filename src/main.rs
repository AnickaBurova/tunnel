/**
 * File: src/main.rs
 * Author: Anicka Burova <anicka.burova@gmail.com>
 * Date: 04.09.2017
 * Last Modified Date: 08.09.2017
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

#[macro_use]
mod tools;
mod config;
mod messages;
mod s3tunnel;
mod tunnel;
mod server;
mod client;

use config::*;
use clap::ArgMatches;

fn s3run(matches: &ArgMatches) -> io::Result<()> {
    let mode = &matches.value_of("mode").unwrap();
    let is_server = mode == &"server";
    let (writer_name , reader_name ) = if is_server {
        ("tunnel.in", "tunnel.out")
    } else {
        ("tunnel.out", "tunnel.in")
    };
    load_config()
        .and_then( |cfg| {
            s3tunnel::create_clients(is_server, cfg, writer_name, reader_name)
        })
        .and_then( |tunnel| {
            if is_server {
                server::run(matches, tunnel)
            } else {
                client::run(matches, tunnel)
            }
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
        .arg(Arg::with_name("server-port")
             .long("server-port")
             .short("p")
             .takes_value(true)
             .default_value("1234")
             .validator(|val| val.parse::<u16>().map(|_| ()).map_err(|_| format!("Cannot parse {} to u16", val))))
        .arg(Arg::with_name("client-address")
             .long("client-address")
             .help("Address where to connect on a new connection for the client mode")
             .takes_value(true)
             .default_value("127.0.0.1")
            )
        .arg(Arg::with_name("client-port")
             .long("client-port")
             .help("Port where to connect on a new connection for the client mode")
             .takes_value(true)
             .default_value("22")
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
