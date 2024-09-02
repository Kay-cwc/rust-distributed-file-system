extern crate crypto;

pub mod transport;
pub mod server;
pub mod store;

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use server::file_server::{FileServer, FileServerOpts};
use transport::encoding::DefaultDecoder;
use transport::tcp::{self, TCPTransportOpts};
// use transport::transport::{};

fn main() {
    // create the transport layer
    let opts = TCPTransportOpts {
        listen_addr: String::from("localhost:3000"),
        shakehands: |_| Ok(()),
        decoder: Box::new(DefaultDecoder {}),
        // FIXME: implement on peer fn
        on_peer: |_peer| {
            Ok(())
        }
    };
    let tcp_transport = tcp::TCPTransport::new(opts);

    let file_server_opts = FileServerOpts {
        listen_addr: String::from("localhost:3000"),
        store_opts: store::store::StoreOpts {
            root_dir: String::from("store"),
            filename_transform: store::hashlib::filename_transform,
        },
        transport: tcp_transport.clone(),
    };
    let server = FileServer::new(file_server_opts);

    thread::scope(|s| {
        s.spawn(|| {
            thread::sleep(Duration::from_secs(10));
            println!("Shutting down server...");
            server.clone().shutdown();
        });
        s.spawn(|| {
            server.clone().start().unwrap();
        }); 
    });

    // tcp_transport.clone().listen_and_accept().unwrap();

    // println!("[server] waiting for msg...");
    // loop {
    //     // keep the main thread alive
    //     let msg = tcp_transport.clone().consume().unwrap();
    //     println!("[server] received msg: {:?}", msg);
    // }
}