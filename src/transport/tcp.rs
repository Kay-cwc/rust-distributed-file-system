use std::collections::HashMap;
use std::fmt::Error;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::thread;
use std::net::{SocketAddr, TcpListener, TcpStream, Shutdown};

use crate::transport::message::Message;
use crate::transport::transport::Transport;

use super::encoding::Decoder;
use super::handshake::ErrInvalidHandshake;

/**
 * the peer struct is responsible for the connection between nodes
 */
pub struct TCPPeer {
    /**
     * the underlying connection of the peer
     */
    conn: TcpStream,
    /**
     * if dial and retrieve the connection => outbound = true
     * if accept and retrieve the connection => outbound = false
     */
    outbound: bool,
}

impl TCPPeer {
    pub fn new(conn: TcpStream, outbound: bool) -> TCPPeer {
        TCPPeer {
            conn,
            outbound,
        }
    }
}

pub type HandShakeFn = fn(peer: &TCPPeer) -> Result<(), ErrInvalidHandshake>;

/**
 * defines the configuration of the tcp transport layer
 */
pub struct TCPTransportOpts {
    pub listen_addr: String,
    /**
     * allow the handshake function to be passed from the constructor
     */
    pub shakehands: HandShakeFn,
    pub decoder: Box<dyn Decoder + Send + Sync>,
}

/**
 * TCPTransport maintains the tcp transport layer and connection with other peer nodes
 */
pub struct TCPTransport {
    pub opts: TCPTransportOpts,
    listener: TcpListener,

    peers: Mutex<HashMap<SocketAddr, TCPPeer>>,
}

// section: implement the transport layer

impl TCPTransport {
    /**
     * create a new tcp transport layer
     */
    pub fn new(opts: TCPTransportOpts) -> Arc<TCPTransport> {
        let listener = TcpListener::bind(&opts.listen_addr).unwrap();
        Arc::new(TCPTransport {
            opts,
            listener,
            peers: Mutex::new(HashMap::new()),
        })
    }

    /**
     * create a loop to accept incoming connections
     */
    pub fn start_accept(&self) {
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    println!("New connection: {}", stream.peer_addr().unwrap());
                    self.handle_conn(stream);
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        }
    }

    /**
     handle_conn is responsible for handling the connection between nodes
     it handles the handshake and store the peer in the peers list
     */
    fn handle_conn(&self, conn: TcpStream) {
        let mut peer = TCPPeer::new(conn, true);

        match (self.opts.shakehands)(&peer) {
            Ok(_) => println!("Handshake successful"),
            Err(_) => {
                peer.conn.shutdown(Shutdown::Both).unwrap();
                return;
            },
        };

        // self.peers.lock().unwrap().insert(peer.conn.peer_addr().unwrap(), peer);

        // read from the connection
        loop {
            let mut msg = Message::new(peer.conn.peer_addr().unwrap());
            match self.opts.decoder.decode(&mut peer.conn, &mut msg) {
                Ok(_) => {
                    println!("Received data from {}: {}", msg.from, String::from_utf8_lossy(&msg.payload));
                }
                Err(e) => {
                    println!("Error reading from connection: {}", e);
                    break;
                }
            }
        }
    }
}

impl AsRef<TCPTransport> for TCPTransport {
    fn as_ref(&self) -> &TCPTransport {
        self
    }
}

impl Transport for TCPTransport {
    fn listen_and_accept(self: Arc<Self>) -> Result<(), Error> {
        thread::spawn(move || {
            self.start_accept();
        });

        Ok(())
    }
}

// section: tests

#[cfg(test)]
mod tests {
    use crate::transport::encoding::DefaultDecoder;

    use super::*;

    #[test]
    fn test_new_tcp_transport() {
        let addr = String::from("localhost:3000");
        let opts = TCPTransportOpts {
            listen_addr: addr.clone(),
            shakehands: |_| Ok(()),
            decoder: Box::new(DefaultDecoder {})
        };
        let transport = TCPTransport::new(opts);
        assert_eq!(transport.opts.listen_addr, addr);
    }

    #[test]
    fn test_listen_and_accept() {
        let addr = String::from("localhost:3000");
        let opts = TCPTransportOpts {
            listen_addr: addr.clone(),
            shakehands: |_| Ok(()),
            decoder: Box::new(DefaultDecoder {})
        };

        let transport = TCPTransport::new(opts);
        // test if the listen_and_accept function is working
        assert_eq!(transport.listen_and_accept().is_ok(), true);
    }

    // TODO: test if a peer is added to the peers list
}