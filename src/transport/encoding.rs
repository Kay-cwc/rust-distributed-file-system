use std::io;

use super::message::Message;

pub trait Decoder {
    fn decode(&self, r: &mut dyn io::Read, msg: &mut Message) -> Result<(), io::Error>;
}

pub struct DefaultDecoder {}

impl Decoder for DefaultDecoder {
    fn decode(&self, r: &mut dyn io::Read, msg: &mut Message) -> Result<(), io::Error> {
        let mut buf = Vec::new();
        let n = r.read(&mut buf).unwrap();
        msg.payload = buf[..n].to_vec();
        Ok(())
    }
}