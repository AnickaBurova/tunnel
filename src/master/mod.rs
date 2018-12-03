//! Master starts and manager individual servers
//! Run a prompt to start individual listeners, which will connect to one
//! specific address on the
//! other side of the tunnel.
//!
//!

use std::io;
use tunnel::{Input, Output, Message};
mod cmd;
mod prompt;

use self::cmd::Cmd;

pub fn create(input: Input, output: Output) -> io::Result<()> {
    for cmd in prompt::prompt() {
        match cmd {
            Cmd::Server(address, port) => {
                use std::thread;
                thread::spawn(move || start_server(address, port));
            }
        }
    }
    println!("Master finished");
    Ok(())
}

fn start_server(address: String, port: u16) {
    info!("Starting server on the port {} for connection to '{}'",
          port,
          address);

}
