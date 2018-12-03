use std::io;
use std::sync::mpsc::channel;
use tunnel_interface::{Input, Output};

pub fn create(file_in: String, file_out: String) -> io::Result<(Input, Output)> {
    // let tcp = TcpStream::connect("localhost:4444")?;
    let (_data_in_sender, input) = channel();
    let (output, _data_out_receiver) = channel();

    Ok((input, output))
}
