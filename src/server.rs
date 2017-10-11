/**
 * File: src/server.rs
 * Author: Anicka Burova <anicka.burova@gmail.com>
 * Date: 07.09.2017
 * Last Modified Date: 11.10.2017
 * Last Modified By: Anicka Burova <anicka.burova@gmail.com>
 */


use tunnel::{Tunnel, WriterData};
use std::io::{self};
use clap::ArgMatches;
use connection::{manage_clients, run_connection};
use std::thread;

use rustyline::error::ReadlineError;
use rustyline::Editor;

use std::sync::{Arc, Mutex};

pub fn run(matches: &ArgMatches, tunnel: Tunnel) -> io::Result<()>{
    use std::net::{TcpListener};
    let port = &matches.value_of("server-port").unwrap();
    let address = format!("0.0.0.0:{}",port);
    info!("Creating server on: {}", address);
    let client_ip = {
        let client_ip = &matches.value_of("client-address").unwrap();
        Arc::new(Mutex::new(client_ip.to_string()))
    };
    let client_port = {
        let client_port = value_t!(matches, "client-port", u16).unwrap();
        Arc::new(Mutex::new(client_port))
    };

    let tunnel_reader = tunnel.reader;
    let tunnel_writer = tunnel.writer;

    let client_state_sender = manage_clients(tunnel_reader);

    let client_ip_thread = client_ip.clone();
    let client_port_thread = client_port.clone();

    thread::spawn(move || {
        let client_ip = client_ip_thread;
        let client_port = client_port_thread;
        TcpListener::bind(address)
            .and_then(move|listener| {
                info!("Waiting for connection, going to sleep.");
                for (id, socket) in listener .incoming().enumerate() {
                    info!("Got connection {}", id);
                    let id = (id + 1) as u64;
                    let tunnel_writer = tunnel_writer.clone();
                    let client_state_sender = client_state_sender.clone();
                    {
                        let client_ip = client_ip.lock().unwrap();
                        let client_port = client_port.lock().unwrap();

                        let _ = tunnel_writer.send(WriterData::Connect(id, (*client_ip).clone(), *client_port)).unwrap();
                    }
                    socket
                        .and_then(|socket| {
                            run_connection( tunnel_writer, id, client_state_sender, socket);
                            Ok(())
                        })
                        .unwrap();

                    info!("Closing connection {}", id);

                }
                Ok(())
            })
            .unwrap();
    });

    prompt(client_ip, client_port);

    info!("Server finished");

    Ok(())
}

fn prompt(client_ip: Arc<Mutex<String>>, client_port: Arc<Mutex<u16>>) {
    let mut rl = Editor::<()>::new();
    if let Err(_) = rl.load_history(".history.txt") {
        warn!("No previous history");
    }

    loop {
        match rl.readline(">> ") {
            Ok(line) => {
                let args: Vec<&str> = line.split(char::is_whitespace).collect();
                let parser = create_parser();
                match parser.get_matches_from_safe(args) {
                    Ok(matches) =>
                        match matches.subcommand_name() {
                            Some("connect") => {
                                let matches = matches.subcommand_matches("connect").unwrap();
                                let mut client_ip = client_ip.lock().unwrap();
                                let ip = matches.value_of("ip").unwrap();
                                *client_ip = ip.to_owned();
                                let mut client_port = client_port.lock().unwrap();
                                let port = value_t!(matches, "port", u16).unwrap();
                                *client_port = port;
                            },
                            _ => {
                                println!("Unknown command");
                            }
                        },
                    Err(e) => {
                        println!("{}", e.message);
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                info!("CTRL-C");
                break
            },
            Err(ReadlineError::Eof) => {
                info!("CTRL-D");
                break
            },
            Err(err) => {
                error!("Error: {:?}", err);
                break
            }
        }
    }

    rl.save_history(".history.txt").unwrap();

}

use std::str::{self};

fn is_val<T: str::FromStr>(x: String) -> Result<(), String> {
     x.parse::<T>()
         .and_then(|_| Ok(()))
         .or_else(|_| Err(format!("Cannot parse {}", x)))
}

use clap::{App};

fn create_parser() -> App<'static, 'static> {
    use clap::{Arg, AppSettings, SubCommand};
    App::new(env!("CARGO_PKG_NAME"))
        .version(crate_version!())
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .setting(AppSettings::NoBinaryName)
        .subcommand(SubCommand::with_name("connect")
                    .about("Connect to the addres.")
                    .arg(Arg::with_name("ip")
                         .help("ip address to connect to")
                         .index(1)
                         .required(true)
                         )
                    .arg(Arg::with_name("port")
                         .help("port to connect to")
                         .index(2)
                         .required(true)
                         .validator(is_val::<u16>)
                         )
                    )
        //.arg(Arg::with_name("tunnel-file-name")
            //.help("Name of the files to use for transfer data: tunnel.in tunnel.out, this is just the name, not the extension.")
            //.long("tunnel-file-name")
            //.default_value("tunnel"))
}
