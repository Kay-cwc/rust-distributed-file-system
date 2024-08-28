use std::{
    fmt::{self, Display, Error, Formatter}, io, sync::{
        mpsc::RecvError, Arc
    }
};

use super::message::Message;

#[derive(Debug)]
pub struct ErrConnClose;

impl Display for ErrConnClose {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "connection closed")
    }
}

impl std::error::Error for ErrConnClose {}

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