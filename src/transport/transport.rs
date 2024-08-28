use std::{
    fmt::Error, io, sync::{
        mpsc::RecvError, Arc
    }
};

use super::message::Message;

pub trait Peer {
    fn close(&self) -> Result<(), io::Error>;
}

// a top level interface for the transport lar.
// should be implemented by all transport layer
pub trait Transport {
    // fn Addr() -> String;
    // fn Dial(v: String) -> void;
    fn listen_and_accept(self: Arc<Self>) -> Result<(), Error>;
    fn consume(self: Arc<Self>) -> Result<Message, RecvError>;
}