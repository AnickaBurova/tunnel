
// Commands used by the master
pub enum Cmd {
    // Create a server to connect to a specified address and listening to a specific port
    Server(String, u16),
}


impl Cmd {
    pub fn parse(line: String) -> Result<Cmd, String> {
        let args: Vec<&str> = line.split(char::is_whitespace).collect();
        let parser = create_parser();
        match parser.get_matches_from_safe(args) {
            Ok(matches) => {
                match matches.subcommand() {
                    ("server", Some(args)) => {
                        Ok(Cmd::Server(args.value_of("ip").unwrap().to_owned(),
                                       value_t!(args, "port", u16).unwrap()))
                    }
                    _ => Err("Unknown command".to_owned()),
                }
            }
            Err(e) => Err(format!("{}", e)),
        }
    }
}

use clap::App;
use validator::is_val;

fn create_parser() -> App<'static, 'static> {
    use clap::{Arg, AppSettings, SubCommand};
    App::new(env!("CARGO_PKG_NAME"))
        .version(crate_version!())
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .setting(AppSettings::NoBinaryName)
        .subcommand(SubCommand::with_name("server")
                        .about("Connect to the address.")
                        .arg(Arg::with_name("ip")
                                 .help("ip address to connect to")
                                 .index(1)
                                 .required(true))
                        .arg(Arg::with_name("port")
                                 .help("port to listen on this machine")
                                 .index(2)
                                 .required(true)
                                 .validator(is_val::<u16>)))
    // arg(Arg::with_name("tunnel-file-name")
    // help("Name of the files to use for transfer data: tunnel.in tunnel.out, this
    // is just the name, not the extension.")
    // long("tunnel-file-name")
    // default_value("tunnel"))
}
