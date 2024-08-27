use std::fmt::Display;

#[derive(Debug)]
pub struct ErrInvalidHandshake;

impl Display for ErrInvalidHandshake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid Handshake")
    }
}