/**
 * File: src/tunnel.rs
 * Author: Anicka Burova <anicka.burova@gmail.com>
 * Date: 08.09.2017
 * Last Modified Date: 06.10.2017
 * Last Modified By: Anicka Burova <anicka.burova@gmail.com>
 */

use messages::*;
use std::sync::mpsc::{Sender, Receiver};
use std::io::{self};

pub struct Tunnel {
    pub writer: Sender<Vec<u8>>,
    pub reader: Receiver<Vec<u8>>,
    pub connection: Option<Receiver<u64>>,
}

pub enum WriteCommand {
    /// Write these messages to the tunnel
    Write(Vec<u8>),
    /// Delete the tunnel, because there are no more messages.
    Delete,
}

pub enum ReadCommand {
    Read(Vec<Message>),
    NoFile,
}

pub struct TunnelPipes {
    pub writer: Sender<WriteCommand>,
    pub reader: Receiver<ReadCommand>,
}


pub fn run(is_server: bool, pipes: TunnelPipes) -> io::Result<Tunnel> {
    use std::sync::mpsc::{channel};
    use std::thread;
    use std::str;
    use serde_yaml;

    let (writer_sender, writer_receiver) = channel::<Vec<u8>>();
    let (reader_sender, reader_receiver) = channel();
    let (connection_sender, connection_receiver) = if !is_server {
        let (sender, receiver) = channel();
        (Some(sender), Some(receiver))
    } else {
        (None, None)
    };
    let writer_pipe = pipes.writer;
    let reader_pipe = pipes.reader;

    let mut writer_created = false;
    let _ = thread::spawn(move|| {
        info!("Tunnel thread started");
        let mut reader_sync = 0;
        let mut writer_sync = 0;
        // at the begining, remove tunnel files
        let mut all_msgs: Vec<Message> = Vec::new(); // all the messages which are writen to the tunnel writer.
        let mut msg_id = 1; // start messages from 1, so we can keep writer_sync from 0
        let mut remove_sync = None;
        loop {
            let last_writer_sync = writer_sync;
            let last_reader_sync = reader_sync;
            // Reading tunnel input
            for msg in reader_pipe.try_iter() {
                match msg {
                    ReadCommand::NoFile => (),
                    ReadCommand::Read(mut stream) => {
                        for msg in stream.drain(..) {
                            if reader_sync >= msg.id {
                                // if id of the message has already been processed
                                continue;
                            }
                            reader_sync = msg.id;
                            match msg.payload {
                                Payload::Connect(id) => {
                                    // start a connection!
                                    info!("Got connection");
                                    match connection_sender.as_ref() {
                                        Some(ref connection_sender) => connection_sender.send(id).unwrap(),
                                        None => (),
                                    }
                                }
                                Payload::Data(_, data) => {
                                    info!("Got data");
                                    reader_sender.send(data).unwrap();
                                }
                                Payload::Sync(_id, last_msg) => {
                                    info!("Got sync: {}", last_msg);
                                    writer_sync = last_msg;
                                }
                            }
                        }
                    }
                }
            }

            if last_writer_sync < writer_sync {
                // the other side has read messages up to writer_sync, we can
                // filter them out
                let remove_all = all_msgs
                    .last()
                    .map(|last| last.id == writer_sync )
                    .unwrap_or(false);
                // little performance improvement in case we can just clear all the
                // messages
                if remove_all {
                    all_msgs.clear();
                } else {
                    // otherwise filter all older then writer_sync
                    let tmp = all_msgs
                        .drain(..)
                        .filter(|msg| msg.id > writer_sync)
                        .collect();
                    all_msgs = tmp;
                }
            }

            let mut is_change = false;

            let mut new_msgs = writer_receiver.try_iter().collect::<Vec<Vec<u8>>>();

            if last_reader_sync < reader_sync && remove_sync.is_none() {
                // We read another messages from the tunnel, let the other side know, we
                // have them and it doesn't need to send them anymore.
                let id = msg_id;
                msg_id += 1;
                all_msgs
                    .push(Message {
                        id,
                        // For now, id of the connection is just 1, until multiple
                        // connections are implemented.
                        payload: Payload::Sync(1, reader_sync),
                    });
                info!("Sending sync: {}", reader_sync);
                remove_sync = Some(id);
                is_change = true;
            }

            if new_msgs.len() > 0 {

                // if this is a server, and this is the first message to send, send connect
                // first
                if msg_id == 1 && is_server {
                    let id = msg_id;
                    msg_id += 1;
                    all_msgs
                        .push(Message {
                            id,
                            // For now, id of the connection is just 1, until multiple
                            // connections are implemented.
                            payload: Payload::Connect(1),
                        });
                }
                // add the new messages to all
                all_msgs
                    .extend( new_msgs
                             .drain(..)
                             .filter(|data| data.len()>0)
                             .map(|data| {
                                 is_change = true;
                                 remove_sync = None;
                                 let id = msg_id;
                                 msg_id += 1;
                                 Message {
                                     id,
                                     payload: Payload::Data(1, data),
                                 }
                             }));
            }

            if is_change {
                let msg = serde_yaml::to_string(&all_msgs).unwrap();
                let msg = msg.bytes().collect::<Vec<u8>>();
                let _ = writer_pipe.send(WriteCommand::Write(msg)).unwrap();
                writer_created = true;
            } else {
                if all_msgs.len() == 0 && writer_created {
                    let _ = writer_pipe.send(WriteCommand::Delete).unwrap();
                    writer_created = false;
                }
            }
        }
    });

    Ok(Tunnel {
        writer: writer_sender,
        reader: reader_receiver,
        connection: connection_receiver,
    })

}

