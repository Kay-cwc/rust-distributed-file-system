use std::{
    fmt::{self, Display, Formatter}, 
    io, net::SocketAddr, 
    sync::{
        mpsc::TryRecvError, Arc
    }
};

use super::{handshake::ErrInvalidHandshake, message::Message};

/** an error type for connection close */
#[derive(Debug)]
pub struct ErrConnClose;

impl Display for ErrConnClose {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "connection closed")
    }
}

impl std::error::Error for ErrConnClose {}

/// a generic interface for peer
pub trait PeerLike {
    fn addr(&self) -> SocketAddr;
    fn close(&self) -> Result<(), io::Error>;
    // fn send(&self, msg: ) -> Result<(), io::Error>;
}

pub type HandShakeFn = fn(peer: &Box<dyn PeerLike>) -> Result<(), ErrInvalidHandshake>;
pub type OnPeerFn = fn(peer: &Box<dyn PeerLike>) -> Result<(), ErrConnClose>;

/// a top level interface for the transport layer  
/// should be implemented by all transport layer
pub trait Transport {
    /// return the local address of the listener
    fn addr(self: Arc<Self>) -> String;
    /// clean up
    fn close(self: Arc<Self>) -> Result<(), Box<dyn std::error::Error>>; 
    /// to receive a message from the transport layer
    fn consume(self: Arc<Self>) -> Result<Message, TryRecvError>;
    /// start listening and accepting incoming connections
    fn listen_and_accept(self: Arc<Self>) -> Result<(), Box<dyn std::error::Error>>;
    /// dial a remote address
    fn dial(self: Arc<Self>, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>>;
}