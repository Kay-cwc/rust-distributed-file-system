use std::net::SocketAddr;

#[derive(Debug)]
pub struct Message {
    pub from: SocketAddr,
    pub payload: Vec<u8>,
}

impl Message {
    pub fn new(from: SocketAddr) -> Message {
        Message {
            from,
            payload: Vec::new(),
        }
    }
}