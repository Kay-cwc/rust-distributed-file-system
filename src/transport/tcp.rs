use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::{boxed, io, thread};
use std::net::{SocketAddr, TcpListener, TcpStream, Shutdown};

use crate::transport::message::Message;
use crate::transport::transport::Transport;

use super::encoding::Decoder;
use super::transport::{HandShakeFn, PeerLike};

/// the peer struct is responsible for the connection between nodes
pub struct TcpPeer {
    /// the underlying connection of the peer
    conn: TcpStream,
    /// if dial and retrieve the connection => outbound = true  
    /// if accept and retrieve the connection => outbound = false
    outbound: bool,
}

impl TcpPeer {
    pub fn new(conn: TcpStream, outbound: bool) -> TcpPeer {
        TcpPeer {
            conn,
            outbound,
        }
    }

}

impl PeerLike for TcpPeer {
    fn addr(&self) -> SocketAddr {
        self.conn.peer_addr().unwrap()
    }
    fn close(&self) -> Result<(), io::Error> {
        self.conn.shutdown(Shutdown::Both)
    }
}

/// defines the configuration of the tcp transport layer
pub struct TcpTransportOpts {
    pub listen_addr: String,
    /// allow the handshake function to be passed from the constructor
    pub shakehands: Option<HandShakeFn<TcpPeer>>,
    pub decoder: Box<dyn Decoder + Send + Sync>,
}

impl TcpTransportOpts {
    pub fn new(listen_addr: String, decoder: Box<dyn Decoder + Send + Sync>) -> TcpTransportOpts {
        TcpTransportOpts {
            listen_addr,
            shakehands: Option::None,
            decoder,
        }
    }
}

/// TCPTransport maintains the tcp transport layer and connection with other peer nodes
pub struct TcpTransport {
    pub opts: TcpTransportOpts,
    listener: TcpListener,
    msg_chan: (Mutex<Sender<Message>>, Mutex<Receiver<Message>>),

    peers: Mutex<HashMap<SocketAddr, Arc<TcpPeer>>>,
    on_peer: Arc<Mutex<Option<Box<dyn Fn(Arc<TcpPeer>) + Send + Sync + 'static>>>>,
}

// section: implement the transport layer

impl TcpTransport {
    /// create a new tcp transport layer
    pub fn new(opts: TcpTransportOpts) -> Arc<TcpTransport> {
        let listener = TcpListener::bind(&opts.listen_addr).unwrap();
        let channel: (Sender<Message>, Receiver<Message>) = channel();
        Arc::new(TcpTransport {
            opts,
            listener,
            msg_chan: (Mutex::new(channel.0), Mutex::new(channel.1)),
            peers: Mutex::new(HashMap::new()),
            on_peer: Arc::new(Mutex::new(Option::None)),
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
        let peer = Arc::new(TcpPeer::new(conn.try_clone().unwrap(), outbound)); // inbound connection

        // perform the handshake
        match self.opts.shakehands {
            Some(shakehands) => {
                match shakehands(&peer.clone()) {
                    Ok(_) => println!("Handshake with {} successful", peer.clone().addr()),
                    Err(_) => {
                        peer.clone().close().unwrap();
                        return;
                    },
                };
            },
            None => {
                println!("No handshake function provided");
            }
        }

        // call the on_peer function
        // TODO: move the mutex to the top level of this fn. other usage of the peer should ref the mutex
        let on_peer = self.on_peer.lock().unwrap();
        if let Some(cb) = &*on_peer {
            cb(peer.clone());
        }
        // on_peer(peerlike.addr()); // should callback should return a result so that when it fails we can close the connection
        //     Ok(_) => {
        //         println!("Peer {} connected", peerlike.addr());
        //         // self.peers.lock().unwrap().insert(peerlike.addr(), peer);
        //     },
        //     Err(_) => {
        //         println!("[Error] Peer {} failed to connect", peerlike.addr());
        //         peerlike.close().unwrap();
        //         return;
        //     },
        // }
        // self.on_peer {
        //         match on_peer(&peerlike) {
        //         }
        //     },
        // }

        // add the peer to the peers list
        self.peers.lock().unwrap().insert(peer.clone().addr(), peer.clone());

        // read from the connection
        loop {
            let mut msg = Message::new(peer.clone().addr());
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


impl Transport for TcpTransport {
    type Peer = TcpPeer;
    
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

    fn register_on_peer(self: Arc<Self>, callback: Box<dyn Fn(Arc<TcpPeer>) + Sync + Send + 'static>) {
        let mut cb = self.on_peer.lock().unwrap();
        *cb = Some(callback);
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
        let opts = TcpTransportOpts {
            listen_addr: addr.clone(),
            shakehands: Option::None,
            decoder: Box::new(DefaultDecoder {}),
        };
        let transport = TcpTransport::new(opts);
        assert_eq!(transport.opts.listen_addr, addr);
    }

    #[test]
    fn test_listen_and_accept() {
        let addr = String::from("localhost:3000");
        let opts = TcpTransportOpts {
            listen_addr: addr.clone(),
            shakehands: Option::None,
            decoder: Box::new(DefaultDecoder {}),
        };

        let transport = TcpTransport::new(opts);
        // test if the listen_and_accept function is working
        assert_eq!(transport.listen_and_accept().is_ok(), true);
    }

    // TODO: test if a peer is added to the peers list
}