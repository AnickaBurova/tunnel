/**
 * File: src/tunnel.rs
 * Author: Anicka Burova <anicka.burova@gmail.com>
 * Date: 08.09.2017
 * Last Modified Date: 11.10.2017
 * Last Modified By: Anicka Burova <anicka.burova@gmail.com>
 */

use messages::*;
use std::sync::mpsc::{Sender, Receiver};
use std::io::{self};

type NewConnection = (u64, String, u16);

pub enum WriterData {
    Connect(u64, String, u16),
    Disconnect(u64),
    Data(u64, Vec<u8>),
}

pub enum ReaderData {
    Disconnect,
    Data(Vec<u8>),
}

pub struct Tunnel {
    pub writer: Sender<WriterData>,
    pub reader: Receiver<(u64, ReaderData)>,
    pub connection: Option<Receiver<NewConnection>>,
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

fn read_tunnel(reader_pipe: &Receiver<ReadCommand>, reader_sync: &mut usize, writer_sync: &mut usize, connection_sender: &Option<Sender<NewConnection>>, reader_sender: &Sender<(u64, ReaderData)>) {
    for msg in reader_pipe.try_iter() {
        match msg {
            ReadCommand::NoFile => (),
            ReadCommand::Read(mut stream) => {
                for msg in stream.drain(..) {
                    if *reader_sync >= msg.id {
                        // if id of the message has already been processed
                        continue;
                    }
                    *reader_sync = msg.id;
                    match msg.payload {
                        Payload::Connect(id,ip,port) => {
                            // start a connection!
                            info!("[{}] Got connection request from the tunnel to {}:{}", id, ip, port);
                            match connection_sender.as_ref() {
                                Some(ref connection_sender) => connection_sender.send((id,ip,port)).unwrap(),
                                None => (),
                            }
                        }
                        Payload::Disconnect(id) => {
                            info!("[{}] got disconnection request from the tunnel", id);
                            reader_sender.send((id,ReaderData::Disconnect)).unwrap();
                        }
                        Payload::Data(id, data) => {
                            info!("[{}] Got data", id);
                            reader_sender.send((id,ReaderData::Data(data))).unwrap();
                        }
                        Payload::Sync(_id, last_msg) => {
                            info!("Got sync: {}", last_msg);
                            *writer_sync = last_msg;
                        }
                    }
                }
            }
        }
    }
}

fn tidy_up_msgs(last_writer_sync: usize, writer_sync: usize, all_msgs: &mut Vec<Message>) {
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
            *all_msgs = tmp;
        }
    }
}

fn resync_msg(last_reader_sync: usize, reader_sync: usize, remove_sync: &mut Option<usize>, msg_id: &mut usize, all_msgs: &mut Vec<Message>) -> bool {
    if last_reader_sync < reader_sync && remove_sync.is_none() {
        // We read another messages from the tunnel, let the other side know, we
        // have them and it doesn't need to send them anymore.
        let id = *msg_id;
        *msg_id += 1;
        all_msgs
            .push(Message {
                id,
                // For now, id of the connection is just 1, until multiple
                // connections are implemented.
                payload: Payload::Sync(1, reader_sync),
            });
        info!("Sending sync: {}", reader_sync);
        *remove_sync = Some(id);
        true
    } else {
        false
    }
}

fn add_msgs(is_change: bool,  writer_receiver: &Receiver<WriterData>, all_msgs: &mut Vec<Message>, remove_sync: &mut Option<usize>, msg_id: &mut usize) -> bool {
    let saved_len = all_msgs.len();
    all_msgs.extend(
        writer_receiver
        .try_iter()
        .map(|msg| {
            let id = *msg_id;
            *msg_id += 1;
            match msg {
                WriterData::Connect(connection, ip , port) =>
                    Message {
                     id,
                     payload: Payload::Connect(connection, ip, port),
                    },
                WriterData::Disconnect(connection) =>
                    Message {
                        id,
                        payload: Payload::Disconnect(connection),
                    },
                WriterData::Data(connection,data) =>
                    Message {
                     id,
                     payload: Payload::Data(connection, data),
                    },
            }
        }));
    if saved_len < all_msgs.len() {
        *remove_sync = None;
        true
    } else {
        is_change
    }
}

pub fn run(is_server: bool, pipes: TunnelPipes) -> io::Result<Tunnel> {
    use std::sync::mpsc::{channel};
    use std::thread;
    use std::str;
    use serde_yaml;

    let (writer_sender, writer_receiver) = channel::<WriterData>();
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
            read_tunnel(&reader_pipe, &mut reader_sync, &mut writer_sync, &connection_sender, &reader_sender);

            tidy_up_msgs(last_writer_sync, writer_sync, &mut all_msgs);

            let mut is_change = resync_msg(last_reader_sync, reader_sync, &mut remove_sync, &mut msg_id, &mut all_msgs);

            //let new_msgs = writer_receiver.try_iter().collect::<Vec<Vec<u8>>>();

            is_change = add_msgs(is_change, &writer_receiver, &mut all_msgs, &mut remove_sync, &mut msg_id);

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
            wait_little!();
        }
    });

    Ok(Tunnel {
        writer: writer_sender,
        reader: reader_receiver,
        connection: connection_receiver,
    })

}

