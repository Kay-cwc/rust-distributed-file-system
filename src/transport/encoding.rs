use std::io;

use rust_distributed_file::read_all_from_stream;

use super::message::Message;

pub trait Decoder: Send + Sync {
    fn decode(&self, r: &mut dyn io::Read, msg: &mut Message) -> Result<(), io::Error>;
}

pub struct DefaultDecoder {}

impl Decoder for DefaultDecoder {
    fn decode(&self, r: &mut dyn io::Read, msg: &mut Message) -> Result<(), io::Error> {
        // FIXME: it is not guaranteed that we will read all the bytes
        // let mut buf = vec![0; 1024];
        // let n = r.read(&mut buf).unwrap();
        // println!("[Decoder] Read {} b1ytes", n);
        // msg.payload = buf[..n].to_vec();
        let buf = read_all_from_stream(r).unwrap();
        msg.payload = buf;
        
        Ok(())
    }
}