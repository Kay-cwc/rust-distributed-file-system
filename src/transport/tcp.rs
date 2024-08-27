use std::collections::HashMap;
use std::fmt::Error;
use std::sync::{Arc, Mutex};
use std::{panic, thread};
use std::net::{SocketAddr, TcpListener, TcpStream};

use crate::transport::p2p::P2P;

// use super::handshake::{no_handshake, HandShakeFn};

// the peer struct is responsible for the connection between nodes
pub struct TCPPeer {
    // the underlying connection of the peer
    conn: TcpStream,
    // if dial and retrieve the connection => outbound = true
    // if accept and retrieve the connection => outbound = false
    outbound: bool,
}

// the transport layer is responsible for the communication between nodes
pub struct TcpTransport {
    pub listen_addr: String,
    listener: TcpListener,

    peers: Mutex<HashMap<SocketAddr, TCPPeer>>,
}

// section: implement the transport layer

impl TcpTransport {
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

    // handle_conn is responsible for handling the connection between nodes
    // it handles the handshake and store the peer in the peers list
    fn handle_conn(&self, conn: TcpStream) {
        match self.shakehands(&conn) {
            Ok(_) => println!("Handshake successful"),
            Err(_) => panic!("Handshake failed"),
        };

        let peer = new_tcp_peer(conn, true);
        self.peers.lock().unwrap().insert(peer.conn.peer_addr().unwrap(), peer);
    }

    fn shakehands(&self, _conn: &TcpStream) -> Result<(), Error> {
        Ok(())
    }
}

impl AsRef<TcpTransport> for TcpTransport {
    fn as_ref(&self) -> &TcpTransport {
        self
    }
}

impl P2P for TcpTransport {
    fn listen_and_accept(self: Arc<Self>) -> Result<(), Error> {
        thread::spawn(move || {
            self.start_accept();
        });

        Ok(())
    }
}

// section: all public function for the transport layer

pub fn new_tcp_transport(addr: &String) -> Arc<TcpTransport> {
    Arc::new(TcpTransport {
        listen_addr: addr.clone(),
        listener: TcpListener::bind(addr).unwrap(),
        peers: Mutex::new(HashMap::new()),
    })
}

// section: all private function for the transport layer

fn new_tcp_peer(conn: TcpStream, outbound: bool) -> TCPPeer {
    let peer = TCPPeer {
        conn,
        outbound,
    };

    return peer;
}

// section: tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tcp_transport() {
        let addr = String::from("localhost:3000");
        let transport = new_tcp_transport(&addr);
        assert_eq!(transport.listen_addr, addr);
    }

    #[test]
    fn test_listen_and_accept() {
        let addr = String::from("localhost:3000");
        let transport = new_tcp_transport(&addr);
        // test if the listen_and_accept function is working
        assert_eq!(transport.listen_and_accept().is_ok(), true);
    }

    // TODO: test if a peer is added to the peers list
}