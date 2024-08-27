use std::io;

use super::message::Message;

pub trait Decoder {
    fn decode(&self, r: &mut dyn io::Read, msg: &mut Message) -> Result<(), io::Error>;
}

pub struct DefaultDecoder {}

impl Decoder for DefaultDecoder {
    fn decode(&self, r: &mut dyn io::Read, msg: &mut Message) -> Result<(), io::Error> {
        r.read(&mut msg.payload);
        Ok(())
    }
}