pub mod file_server {
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::sync::{mpsc::{Receiver, Sender, TryRecvError}, Arc, Mutex};
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

    #[derive(Serialize, Deserialize)]
    struct Payload {
        key: String,
        data: Vec<u8>,
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
                            TryRecvError::Empty => continue,
                            TryRecvError::Disconnected => break,
                        }
                    }
                };
                
                println!("Received message: {:?}", msg);   
            }

            Ok(())
        }

        pub fn shutdown(self: Arc<Self>) {
            self.shutdown_chan.0.lock().unwrap().send(true).unwrap();
        }

        /// read from a stream and store in the store  
        /// will also broadcast the data to all connected peers
        pub fn store_data(self: &Arc<Self>, key: String, r: &mut dyn io::Read) {
            // self.store.write(key, data.as_slice()).unwrap();
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
            let buf = bincode::serialize(&payload).unwrap();
            let peers = self.peers.lock().unwrap();
            for (_, peer) in peers.iter() {
                let mut p = peer.lock().unwrap();
                p.send(&buf).unwrap();
            }   
        }
    }
}