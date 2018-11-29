/**
 * File: src/main.rs
 * Author: Anicka Burova <anicka.burova@gmail.com>
 * Date: 04.09.2017
 * Last Modified Date: 13.10.2017
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

extern crate rustyline;

#[macro_use]
extern crate clap;

#[macro_use]
extern crate log;
extern crate log4rs;

#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;

use std::io;

macro_rules! wait_little {
    () => {
        use std::thread::sleep;
        use std::time::Duration;
        sleep(Duration::from_millis(10));
    }
}


#[macro_use]
mod tools;
mod config;
mod messages;
mod s3tunnel;
mod s3tunnel_cmd;
mod tunnel;
mod server;
mod client;
mod connection;
mod master;

use clap::{ArgMatches, SubCommand};
use config::*;


pub enum Mode {
    Master,
    Server(u16, String, u16),
    Client
}


fn s3run(matches: &ArgMatches) -> io::Result<()> {
    let mode = match matches.subcommand() {
        ("master", _) => Mode::Master,
        ("server", Some(m)) => Mode::Server(value_t!(m, "server-port", u16).unwrap(), m.value_of("client-address").unwrap().to_owned(), value_t!(m, "client-port", u16).unwrap()),
        ("client", _) => Mode::Client,
        _ => {
            return Err(io::Error::new(io::ErrorKind::Other, "Choose mode to run: master, server, client"));
        }
    };
    let file_name = matches.value_of("tunnel-file-name").unwrap();
    let is_server = match mode {
        Mode::Client => false,
        _ => true,
    };
    let (writer_name, reader_name) = if is_server {
        (format!("{}.in", file_name), format!("{}.out", file_name))
    } else {
        (format!("{}.out", file_name), format!("{}.in", file_name))
    };
    load_config()
        .and_then(|cfg| {
            let tunnel_api = &matches.value_of("tunnel-api").unwrap();
            match tunnel_api {
                &"s3cmd" => {
                    s3tunnel_cmd::create_clients(is_server, cfg, &writer_name, &reader_name)
                }
                _ => s3tunnel::create_clients(is_server, cfg, &writer_name, &reader_name),
            }
        })
        .and_then(|tunnel_pipes| tunnel::run(is_server, tunnel_pipes))
        .and_then(|tunnel| match mode {
                      Mode::Master => master::run(tunnel),
                      Mode::Server(port, client, client_port) => server::run(port, client, client_port, tunnel),
                      Mode::Client => client::run( tunnel),
                  })
}


fn main() {
    use clap::{App, Arg};
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(crate_version!())
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(Arg::with_name("log-config")
                 .long("log-config")
                 .help("Log configuration file.")
                 .takes_value(true)
                 .default_value("log.yaml"))
        .arg(Arg::with_name("tunnel-api")
                 .long("tunnel-api")
                 .help("Aws or s3cmd tunnel api communication.")
                 .possible_values(&["aws", "s3cmd"])
                 .default_value("aws"))
        .arg(Arg::with_name("tunnel-file-name")
                 .help("Name of the files to use for transfer data: tunnel.in tunnel.out, \
                        this is just the name, not the extension.")
                 .long("tunnel-file-name")
                 .default_value("tunnel"))
        .arg(Arg::with_name("proxy")
                 .help("Proxy server")
                 .long("proxy"))
        .subcommand(SubCommand::with_name("master")
                        .about("Start the tunnel in interactive session to start multiple different servers")
                    )
        .subcommand(SubCommand::with_name("server")
                        .about("Start the tunnel as one listener listening on one specific port")
                        .arg(Arg::with_name("server-port")
                             .help("What port to listen on")
                             .long("server-port")
                             .help("Port to listen on.")
                             .short("p")
                             .takes_value(true)
                             .default_value("1234")
                             .validator(|val| {
                                            val.parse::<u16>()
                                                .map(|_| ())
                                                .map_err(|_| format!("Cannot parse {} to u16", val))
                                        }))
                        .arg(Arg::with_name("client-address")
                                 .long("client-address")
                                 .help("Address where to connect on a new connection for the client mode.")
                                 .takes_value(true)
                                 .default_value("127.0.0.1"))
                        .arg(Arg::with_name("client-port")
                                 .long("client-port")
                                 .help("Port where to connect on a new connection for the client mode.")
                                 .takes_value(true)
                                 .default_value("22")
                                 .validator(|val| {
                                                val.parse::<u16>()
                                                    .map(|_| ())
                                                    .map_err(|_| format!("Cannot parse {} to u16", val))
                                            }))
                        )
        .subcommand(SubCommand::with_name("client")
                        .about("Start the tunnel in client mode")
                    )
        .get_matches();
    let _ = log4rs::init_file(&matches.value_of("log-config").unwrap(), Default::default())
        .unwrap();
    let _ = s3run(&matches).unwrap();
}
