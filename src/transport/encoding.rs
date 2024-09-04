use std::io;

use super::message::Message;

pub trait Decoder: Send + Sync {
    fn decode(&self, r: &mut dyn io::Read, msg: &mut Message) -> Result<(), io::Error>;
}

pub struct DefaultDecoder {}

impl Decoder for DefaultDecoder {
    fn decode(&self, r: &mut dyn io::Read, msg: &mut Message) -> Result<(), io::Error> {
        // FIXME: it is not guaranteed that we will read all the bytes
        let mut buf = vec![0; 1024];
        let n = r.read(&mut buf).unwrap();
        println!("[Decoder] Read {} bytes", n);
        msg.payload = buf[..n].to_vec();
        Ok(())
    }
}