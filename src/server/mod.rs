pub mod file_server {
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::sync::mpsc::RecvTimeoutError;
    use std::sync::{mpsc::{Receiver, Sender}, Arc, Mutex};
    use std::{io, thread};

    use serde::{Deserialize, Serialize};

    use crate::{
        store::store::{Store, StoreOpts}, 
        transport::transport::{PeerLike, Transport},
    };

    pub struct FileServerOpts<T: Transport> {
        // storage options
        pub store_opts: StoreOpts,
        pub transport: Arc<T>,
        pub bootstrap_node: Vec<SocketAddr>,
    }

    // for future me: FileServer is generic since we need to make sure the size of the transport layer is known at compile time
    // the transport layer can be generic in coding level, but in runtime, we need to know the size of the transport layer
    pub struct FileServer<T: Transport> {
        transport: Arc<T>,
        store: Store,
        shutdown_chan: (Mutex<Sender<bool>>, Mutex<Receiver<bool>>),
        bootstrap_node: Vec<SocketAddr>,
        peers: Mutex<HashMap<SocketAddr, Arc<Mutex<dyn PeerLike + Sync + Send>>>>
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct Payload {
        key: String,
        data: Vec<u8>,
    }

    impl Payload {
        pub fn from_buffer(buf: Vec<u8>) -> Payload {
            bincode::deserialize(&buf).unwrap()
        }

        pub fn to_buffer(&self) -> Vec<u8> {
            bincode::serialize(&self).unwrap()
        }
    }

    impl<T: Transport> FileServer<T> {
        pub fn new(opts: FileServerOpts<T>) -> Arc<FileServer<T>> {
            let store_opts = opts.store_opts;
            let transport = opts.transport;
            let store = Store::new(store_opts);
            let shutdown_chan_ = std::sync::mpsc::channel();

            let server = Arc::new(FileServer {
                transport,
                store,
                shutdown_chan: (Mutex::new(shutdown_chan_.0), Mutex::new(shutdown_chan_.1)),
                bootstrap_node: opts.bootstrap_node,
                peers: Mutex::new(HashMap::new())
            });

            server.register_on_peer_cb();

            server
        }

        /// a blocking function to start the server
        pub fn start(self: Arc<Self>) -> Result<(), Box<dyn std::error::Error>> {
            // start the transport layer and listen for incoming connections
            let _ = self.transport.clone().listen_and_accept();
            println!("server running on {}", self.transport.clone().addr());

            // bootstrap the network
            self.bootstrap_network();

            self.run()
        }

        pub fn run(self: &Arc<Self>) -> Result<(), Box<dyn std::error::Error>> {
            loop {
                // break the loop if we receive a shutdown message
                if let Ok(true) = self.shutdown_chan.1.lock().unwrap().try_recv() {
                    self.transport.clone().close()?;
                    break;
                }

                let msg = match self.transport.clone().consume() {
                    Ok(msg) => msg,
                    Err(e) => {
                        // handle error
                        match e {
                            RecvTimeoutError::Timeout => continue,
                            RecvTimeoutError::Disconnected => break,
                        }
                    }
                };

                println!("Received data from {}: {:?}", msg.from, msg.payload);
                let paylod = Payload::from_buffer(msg.payload);
                println!("Received data from {}: {} -> {}", msg.from, paylod.key, String::from_utf8_lossy(&paylod.data));
                self.store.write(paylod.key, &mut paylod.data.as_slice()).unwrap();
            }

            Ok(())
        }

        pub fn shutdown(self: Arc<Self>) {
            self.shutdown_chan.0.lock().unwrap().send(true).unwrap();
        }

        /// read from a stream and store in the store  
        /// will also broadcast the data to all connected peers
        pub fn store_data(self: &Arc<Self>, key: String, r: &mut dyn io::Read) {
            let mut buf = vec![0; 1024];
            let n = r.read(&mut buf).unwrap();
            println!("[server] read {} bytes", n);
            buf = buf[..n].to_vec();
            // questionable design choice: we are reading the stream twice
            match self.store.write(key.clone(), &mut buf) {
                Ok(_) => {
                    let payload = Payload {
                        key: key.clone(),
                        data: buf,
                    };
                    self.broadcast(payload);
                },
                Err(e) => {
                    println!("Error writing to store: {}", e);
                }
            }
        }

        /// bootstrap the network by connecting to the bootstrap nodes
        /// each dial will be done in a separate thread
        fn bootstrap_network(self: &Arc<Self>) {
            let nodes = self.bootstrap_node.clone();
            // lesson for future me: iter() does not work here as we need to pass the node to the thread
            // this causes a lifetime issue
            // or maybe i am just not good enough to figure it out
            let len = nodes.len();
            for i in 0..len {
                let node = nodes[i].clone();
                let t = self.transport.clone();
                thread::spawn(move || {
                    t.dial(node).unwrap();
                });
            }
        }

        fn register_on_peer_cb(self: &Arc<Self>) {
            // callback fn when a new peer is connected
            let cb = {
                let cloned_self = self.clone();
                move |peer: Arc<Mutex<T::Peer>>| {
                    let p = peer.lock().unwrap();
                    println!("[server] {} on_peer: {}", if p.is_outbound() { "outbound" } else { "inbound" },  p.addr());
                    cloned_self.peers.lock().unwrap().insert(p.addr(), peer.clone());

                    true
                }
            };

            self.transport.clone().register_on_peer(Box::new(cb));
        }

        /// broadcast the payload to all connected peers  
        fn broadcast(self: &Arc<Self>, payload: Payload) {
            println!("Broadcasting data: {:?}", payload);
            let buf = Payload::to_buffer(&payload);
            let peers = self.peers.lock().unwrap();
            for (_, peer) in peers.iter() {
                let mut p = peer.lock().unwrap();
                println!("Sending data to {}: {:?}", p.addr(), buf);
                p.send(&buf).unwrap();
            }
        }
    }
}