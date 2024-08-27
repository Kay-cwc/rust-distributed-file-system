#[derive(Debug)]
pub struct Message {
    pub payload: [u8; 1024],
}

impl Message {
    pub fn new() -> Message {
        Message {
            payload: [0; 1024],
        }
    }
}