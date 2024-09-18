use std::char::MAX;
use std::collections::HashMap;
use std::io::Write;
use std::sync::mpsc::{channel, Receiver, Sender, RecvTimeoutError};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use std::{io, thread};
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

    fn send(&mut self, buf: &[u8]) -> Result<(), io::Error> {
        println!("Sending data to {}: {}", self.addr(), String::from_utf8_lossy(buf));
        self.conn.write_all(buf)
    }

    fn is_outbound(&self) -> bool {
        self.outbound
    }
}

/// defines the configuration of the tcp transport layer
pub struct TcpTransportOpts {
    pub listen_addr: String,
    /// allow the handshake function to be passed from the constructor
    pub shakehands: Option<HandShakeFn<TcpPeer>>,
    pub decoder: Box<dyn Decoder>,
}

impl TcpTransportOpts {
    pub fn new(listen_addr: String, decoder: Box<dyn Decoder>) -> TcpTransportOpts {
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

    peers: RwLock<HashMap<SocketAddr, Arc<RwLock<TcpPeer>>>>,
    on_peer: Arc<Mutex<Option<Box<dyn Fn(Arc<RwLock<TcpPeer>>) -> bool + Send + Sync + 'static>>>>,
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
            peers: RwLock::new(HashMap::new()),
            on_peer: Arc::new(Mutex::new(Option::None)),
        })
    }

    /// create a blocking loop to accept incoming connections
    fn start_accept(self: &Arc<Self>) {
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    // received a new connection. handle the connection and unblock the thread
                    let self_clone = self.clone();
                    thread::spawn(move || {
                        println!("New connection from {}", stream.peer_addr().unwrap());
                        self_clone.clone().handle_conn(stream, false);
                    });
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
        let peer_addr = conn.peer_addr().unwrap();
        let peer = Arc::new(RwLock::new(
            TcpPeer::new(conn.try_clone().unwrap(), 
            outbound)
        )); // inbound connection

        // perform the handshake
        match self.opts.shakehands {
            Some(shakehands) => {
                match shakehands(&peer) {
                    Ok(_) => println!("Handshake with {} successful", peer.read().unwrap().addr()),
                    Err(_) => {
                        peer.write().unwrap().close().unwrap();
                        return;
                    },
                };
            },
            None => {
                println!("No handshake function provided");
            }
        }

        // call the on_peer function
        let on_peer = self.on_peer.lock().unwrap();
        if let Some(cb) = &*on_peer {
            match cb(peer.clone()) {
                true => {},
                false => {
                    println!("Peer {} failed to connect", peer.read().unwrap().addr());
                    // remove the peer from the peers list
                    self.peers.write().unwrap().remove(&peer.read().unwrap().addr());
                    // close the peer
                    peer.write().unwrap().close().unwrap();
                    return;
                },
            };
        }

        // add the peer to the peers list
        self.peers.write().unwrap().insert(peer_addr, peer.clone());

        // read from the connection
        println!("Starting to read from connection: {}", peer.read().unwrap().addr());
        loop {
            let mut msg = Message::new(peer_addr);
            match self.opts.decoder.decode(&mut conn.try_clone().unwrap(), &mut msg) {
                Ok(_) => {
                    println!("Received data from {}: {}", msg.from, String::from_utf8_lossy(&msg.payload));
                }
                Err(e) => {
                    println!("Error reading from connection: {}", e);
                    break;
                }
            }

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

    fn consume(self: Arc<Self>) -> Result<Message, RecvTimeoutError> {
        self.msg_chan.1.lock().unwrap().recv_timeout(Duration::from_secs(1))
    }

    fn close(self: Arc<Self>) -> Result<(), Box<dyn std::error::Error>> {
        // nothing to do here
        Ok(())
    }

    fn dial(self: &Arc<Self>, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        // dial to a remote address
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

    fn try_dial(self: &Arc<Self>, addr: SocketAddr, max_attemps: u8) -> Result<(), Box<dyn std::error::Error>> {
        // dial to a remote address with exponential backoff
        let mut backoff = Duration::from_secs(1);
        let mut attempts = 0;
        loop {
            match self.dial(addr) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    if attempts >= max_attemps {
                        // stop trying
                        println!("Error connecting to {}: {}", addr, e);
                        return Err(e)
                    } else {
                        // exponential backoff
                        println!("Error connecting to {}. Retrying in {} seconds", addr, backoff.as_secs());
                        attempts += 1;
                        thread::sleep(backoff);
                        backoff *= 2;
                    }
                }
            }
        }
    }

    fn register_on_peer(self: Arc<Self>, callback: Box<dyn Fn(Arc<RwLock<TcpPeer>>) -> bool + Sync + Send + 'static>) {
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