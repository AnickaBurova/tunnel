//! Low level implementation of the tunnel communication
//! The communication here is just passing stream of data back and forth

use std::sync::mpsc::{Sender, Receiver};

// Generic message is just a stream of data
pub type Message = Vec<u8>;

// Input from the tunnel
pub type Input = Receiver<Message>;

// Output from the tunnel
pub type Output = Sender<Message>;
