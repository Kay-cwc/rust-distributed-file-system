use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, RecvError, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::{io, thread};
use std::net::{SocketAddr, TcpListener, TcpStream, Shutdown};

use crate::transport::message::Message;
use crate::transport::transport::Transport;

use super::encoding::Decoder;
use super::handshake::ErrInvalidHandshake;
use super::transport::{ErrConnClose, Peer};

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

impl Peer for TCPPeer {
    fn close(&self) -> Result<(), io::Error> {
        self.conn.shutdown(Shutdown::Both)
    }
}

// need to determine whether should put it here on in the transport.rs
pub type HandShakeFn = fn(peer: &TCPPeer) -> Result<(), ErrInvalidHandshake>;
pub type OnPeerFn = fn(peer: &TCPPeer) -> Result<(), ErrConnClose>;

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
    pub on_peer: OnPeerFn,
}

/**
 * TCPTransport maintains the tcp transport layer and connection with other peer nodes
 */
pub struct TCPTransport {
    pub opts: TCPTransportOpts,
    listener: TcpListener,
    msg_chan: (Mutex<Sender<Message>>, Mutex<Receiver<Message>>),

    peers: Mutex<HashMap<SocketAddr, TCPPeer>>,
}

// section: implement the transport layer

impl TCPTransport {
    /// create a new tcp transport layer
    pub fn new(opts: TCPTransportOpts) -> Arc<TCPTransport> {
        let listener = TcpListener::bind(&opts.listen_addr).unwrap();
        let channel: (Sender<Message>, Receiver<Message>) = channel();
        Arc::new(TCPTransport {
            opts,
            listener,
            msg_chan: (Mutex::new(channel.0), Mutex::new(channel.1)),
            peers: Mutex::new(HashMap::new()),
        })
    }

    /// create a blocking loop to accept incoming connections
    fn start_accept(&self) {
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    println!("New connection: {}", stream.peer_addr().unwrap());
                    self.handle_conn(stream, false);
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        }
    }

    /// tcp layer for handling after the connection is established between nodes
    /// it handles the handshake and store the peer in the peers list
    fn handle_conn(&self, conn: TcpStream, outbound: bool) {
        let mut peer = TCPPeer::new(conn, outbound); // inbound connection

        match (self.opts.shakehands)(&peer) {
            Ok(_) => println!("Handshake successful"),
            Err(_) => {
                peer.conn.shutdown(Shutdown::Both).unwrap();
                return;
            },
        };

        // call the on_peer function
        match (self.opts.on_peer)(&peer) {
            Ok(_) => println!("Peer connected"),
            Err(_) => {
                println!("Error on peer");
                peer.conn.shutdown(Shutdown::Both).unwrap();
                return;
            },
        }

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

            println!("Sending message to channel");

            // send the message to the channel
            let sender = self.msg_chan.0.lock().unwrap().clone();
            sender.send(msg).unwrap(); // FIXME: handle error
        }
    }
}


impl Transport for TCPTransport {
    fn addr(self: Arc<Self>) -> String {
        self.opts.listen_addr.clone()
    }

    fn listen_and_accept(self: Arc<Self>) -> Result<(), Box<dyn std::error::Error>> {
        thread::spawn(move || {
            self.start_accept();
        });

        Ok(())
    }

    fn consume(self: Arc<Self>) -> Result<Message, TryRecvError> {
        self.msg_chan.1.lock().unwrap().try_recv()
    }

    fn close(self: Arc<Self>) -> Result<(), Box<dyn std::error::Error>> {
        // nothing to do here
        Ok(())
    }

    fn dial(self: Arc<Self>, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        match TcpStream::connect(addr) {
            Ok(conn) => {
                self.handle_conn(conn, true);
                Ok(())
            },
            Err(e) => {
                println!("Error connecting to {}: {}", addr, e);
                Err(Box::new(e))
            }
        }
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
            decoder: Box::new(DefaultDecoder {}),
            on_peer: |_| { Ok(())}
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
            decoder: Box::new(DefaultDecoder {}),
            on_peer: |_| { Ok(()) }
        };

        let transport = TCPTransport::new(opts);
        // test if the listen_and_accept function is working
        assert_eq!(transport.listen_and_accept().is_ok(), true);
    }

    // TODO: test if a peer is added to the peers list
}