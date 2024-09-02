pub mod file_server {
    use std::sync::{mpsc::{Receiver, Sender}, Arc};

    use crate::{
        store::store::{Store, StoreOpts}, 
        transport::transport::Transport,
    };

    pub struct FileServerOpts {
        // transport layer options
        pub listen_addr: String,
        // storage options
        pub store_opts: StoreOpts,
        pub transport: Arc<dyn Transport + 'static>,
    }

    pub struct FileServer {
        transport: Arc<dyn Transport>,
        store: Store,
        shutdown_chan: (Sender<bool>, Receiver<bool>)
    }

    impl FileServer {
        pub fn new(opts: FileServerOpts) -> Arc<FileServer> {
            let store_opts = opts.store_opts;
            let transport = opts.transport;
            let store = Store::new(store_opts);
            Arc::new(FileServer {
                transport,
                store,
                shutdown_chan: std::sync::mpsc::channel(),
            })
        }

        pub fn start(self: Arc<Self>) -> Result<(), Box<dyn std::error::Error>> {
            let _ = self.transport.clone().listen_and_accept();
            
            self.run()
        }

        pub fn run(self: Arc<Self>) -> Result<(), Box<dyn std::error::Error>> {
            loop {
                // break the loop if we receive a shutdown message
                if let Ok(true) = self.shutdown_chan.1.try_recv() {
                    break;
                }

                let msg = self.transport.clone().consume().unwrap();
                println!("Received message: {:?}", msg);   
            }

            Ok(())
        }
    }
}