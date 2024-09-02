pub mod file_server {
    use std::sync::{mpsc::{Receiver, Sender, TryRecvError}, Arc, Mutex};

    use crate::{
        store::store::{Store, StoreOpts}, 
        transport::transport::Transport,
    };

    pub struct FileServerOpts {
        // transport layer options
        pub listen_addr: String,
        // storage options
        pub store_opts: StoreOpts,
        pub transport: Arc<dyn Transport + Send + Sync>,
    }

    pub struct FileServer {
        transport: Arc<dyn Transport + Send + Sync>,
        store: Store,
        shutdown_chan: (Mutex<Sender<bool>>, Mutex<Receiver<bool>>)
    }

    impl FileServer {
        pub fn new(opts: FileServerOpts) -> Arc<FileServer> {
            let store_opts = opts.store_opts;
            let transport = opts.transport;
            let store = Store::new(store_opts);
            let shutdown_chan_ = std::sync::mpsc::channel();
            Arc::new(FileServer {
                transport,
                store,
                shutdown_chan: (Mutex::new(shutdown_chan_.0), Mutex::new(shutdown_chan_.1)),
            })
        }

        pub fn start(self: Arc<Self>) -> Result<(), Box<dyn std::error::Error>> {
            let _ = self.transport.clone().listen_and_accept();
            println!("servere running on {}", self.transport.clone().addr());
            self.run()
        }

        pub fn run(self: Arc<Self>) -> Result<(), Box<dyn std::error::Error>> {
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
    }
}