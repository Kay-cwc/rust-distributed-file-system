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
pub trait PeerLike: Send + Sync {
    fn addr(&self) -> SocketAddr;
    fn close(&self) -> Result<(), io::Error>;
    fn send(&mut self, buf: &[u8]) -> Result<(), io::Error>;
}

pub type HandShakeFn<P> = fn(peer: &Arc<P>) -> Result<(), ErrInvalidHandshake>;

/// a top level interface for the transport layer  
/// should be implemented by all transport layer
pub trait Transport: Send + Sync + 'static {
    // this associated type is used to define the type of peer for the transport layer.
    // this helps to ensure the generic impl of the transport layer to have a definite size at compile time
    type Peer: PeerLike;

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
    /// register a callback function to be called when a new peer is connected
    fn register_on_peer(self: Arc<Self>, callback: Box<dyn Fn(Arc<Self::Peer>) + Sync + Send + 'static>);
}