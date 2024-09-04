use std::io;

use super::message::Message;

pub trait Decoder {
    fn decode(&self, r: &mut dyn io::Read, msg: &mut Message) -> Result<(), io::Error>;
}

pub struct DefaultDecoder {}

impl Decoder for DefaultDecoder {
    fn decode(&self, r: &mut dyn io::Read, msg: &mut Message) -> Result<(), io::Error> {
        let mut buf = vec![0; 1024];
        println!("Reading from stream");
        let n = r.read(&mut buf).unwrap();
        println!("Read {} bytes", n);
        msg.payload = buf[..n].to_vec();
        Ok(())
    }
}