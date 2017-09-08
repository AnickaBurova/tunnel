/**
 * File: src/messages.rs
 * Author: Anicka Burova <anicka.burova@gmail.com>
 * Date: 07.09.2017
 * Last Modified Date: 07.09.2017
 * Last Modified By: Anicka Burova <anicka.burova@gmail.com>
 */

/// Message payload
#[derive (Debug, Serialize, Deserialize)]
pub enum Payload {
    /// When there is a new connection to the server, it will send 'Connect' request with a new id.
    Connect(u64),
    /// Pass data for specific connection.
    Data(usize, Vec<u8>),
    /// Send information which last message the other side has received. Sender can stop sending
    /// that through the tunnel.
    Sync(u64, usize),
}

/// Message in the tunnel file
#[derive (Debug, Serialize, Deserialize)]
pub struct Message {
    // id of the message, after receiving Sync payload, there is no need to send any older than
    // sync id.
    pub id: usize,
    // Message payload.
    pub payload: Payload,
}

//pub struct DataMessage {
    //pub id: u64,
    //pub data: Vec<u8>,
//}
