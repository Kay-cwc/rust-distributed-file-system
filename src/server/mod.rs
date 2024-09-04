pub mod file_server {
    use std::{collections::HashMap, net::SocketAddr, sync::{mpsc::{Receiver, Sender, TryRecvError}, Arc, Mutex}, thread};

    use crate::{
        store::store::{Store, StoreOpts}, 
        transport::transport::{PeerLike, Transport},
    };

    pub struct FileServerOpts {
        // storage options
        pub store_opts: StoreOpts,
        pub transport: Arc<dyn Transport + Send + Sync>,
        pub bootstrap_node: Vec<SocketAddr>,
    }

    pub struct FileServer {
        transport: Arc<dyn Transport + Send + Sync>,
        store: Store,
        shutdown_chan: (Mutex<Sender<bool>>, Mutex<Receiver<bool>>),
        bootstrap_node: Vec<SocketAddr>,
        peers: Mutex<HashMap<SocketAddr, Arc<dyn PeerLike + Sync + Send>>>
    }

    impl FileServer {
        pub fn new(opts: FileServerOpts) -> Arc<FileServer> {
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

        fn register_on_peer_cb(self: &Arc<Self>) {
            let cb = Box::new(move |addr: SocketAddr| {
                println!("[server] on_peer: {}", addr);
            });

            self.transport.clone().register_on_peer(cb);
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
    }
}