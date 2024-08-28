use std::{fmt::Error, sync::Arc};

// a top level interface for the transport layer.
// should be implemented by all transport layer
pub trait Transport {
    // fn Addr() -> String;
    // fn Dial(v: String) -> void;
    fn listen_and_accept(self: Arc<Self>) -> Result<(), Error>;
    // fn Consume() -> TcpStream;
}