use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::{boxed, io, thread};
use std::net::{SocketAddr, TcpListener, TcpStream, Shutdown};

use crate::transport::message::Message;
use crate::transport::transport::Transport;

use super::encoding::Decoder;
use super::transport::{HandShakeFn, OnPeerFn, PeerLike};

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

impl PeerLike for TCPPeer {
    fn addr(&self) -> SocketAddr {
        self.conn.peer_addr().unwrap()
    }
    fn close(&self) -> Result<(), io::Error> {
        self.conn.shutdown(Shutdown::Both)
    }
}

/**
 * defines the configuration of the tcp transport layer
 */
pub struct TCPTransportOpts {
    pub listen_addr: String,
    /// allow the handshake function to be passed from the constructor
    pub shakehands: Option<HandShakeFn>,
    pub decoder: Box<dyn Decoder + Send + Sync>,
    on_peer: Option<OnPeerFn>,
}

impl TCPTransportOpts {
    pub fn new(listen_addr: String, decoder: Box<dyn Decoder + Send + Sync>) -> TCPTransportOpts {
        TCPTransportOpts {
            listen_addr,
            shakehands: Option::None,
            decoder,
            on_peer: Option::None,
        }
    }
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

    pub fn on_peer(&mut self, on_peer: OnPeerFn) {
        self.opts.on_peer = Some(on_peer);
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
        let peer = TCPPeer::new(conn.try_clone().unwrap(), outbound); // inbound connection
        let peerlike: Box<dyn PeerLike> = Box::new(TCPPeer::new(conn.try_clone().unwrap(), outbound));

        // perform the handshake
        match self.opts.shakehands {
            Some(shakehands) => {
                match shakehands(&peerlike) {
                    Ok(_) => println!("Handshake with {} successful", peerlike.addr()),
                    Err(_) => {
                        peerlike.close().unwrap();
                        return;
                    },
                };
            },
            None => {
                println!("No handshake function provided");
            }
        }

        // call the on_peer function
        match self.opts.on_peer {
            Some(on_peer) => {
                match on_peer(&peerlike) {
                    Ok(_) => {
                        println!("Peer {} connected", peerlike.addr());
                        // self.peers.lock().unwrap().insert(peerlike.addr(), peer);
                    },
                    Err(_) => {
                        println!("[Error] Peer {} failed to connect", peerlike.addr());
                        peerlike.close().unwrap();
                        return;
                    },
                }
            },
            None => {
                println!("No on_peer function provided");
            }
        }

        // add the peer to the peers list
        self.peers.lock().unwrap().insert(peerlike.addr(), peer);

        // read from the connection
        loop {
            let mut msg = Message::new(peerlike.addr());
            match self.opts.decoder.decode(&mut conn.try_clone().unwrap(), &mut msg) {
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
            shakehands: Option::None,
            decoder: Box::new(DefaultDecoder {}),
            on_peer: Option::None
        };
        let transport = TCPTransport::new(opts);
        assert_eq!(transport.opts.listen_addr, addr);
    }

    #[test]
    fn test_listen_and_accept() {
        let addr = String::from("localhost:3000");
        let opts = TCPTransportOpts {
            listen_addr: addr.clone(),
            shakehands: Option::None,
            decoder: Box::new(DefaultDecoder {}),
            on_peer: Option::None,
        };

        let transport = TCPTransport::new(opts);
        // test if the listen_and_accept function is working
        assert_eq!(transport.listen_and_accept().is_ok(), true);
    }

    // TODO: test if a peer is added to the peers list
}