extern crate crypto;

pub mod transport;
pub mod server;
pub mod store;

use std::net::SocketAddr;
use std::sync::Arc;
use std::{thread, vec};
use std::time::Duration;

use server::file_server::{FileServer, FileServerOpts};
use transport::encoding::DefaultDecoder;
use transport::tcp::{self, TCPTransportOpts};

fn make_server(listen_addr: String, nodes: Vec<SocketAddr>) -> Arc<FileServer> {
    // create the transport layer
    let opts = TCPTransportOpts {
        listen_addr: listen_addr.clone(),
        shakehands: |_| Ok(()),
        decoder: Box::new(DefaultDecoder {}),
        // FIXME: implement on peer fn
        on_peer: |_peer| {
            Ok(())
        }
    };
    let tcp_transport = tcp::TCPTransport::new(opts);
    
    let file_server_opts = FileServerOpts {
        store_opts: store::store::StoreOpts {
            root_dir: format!("storage/{}", listen_addr),
            filename_transform: store::hashlib::filename_transform,
        },
        transport: tcp_transport.clone(),
        bootstrap_node: nodes,
    };
    
    FileServer::new(file_server_opts)
}

fn main() {
    let server = make_server(
        "127.0.0.1:3000".to_string(), 
        Vec::new(),
    );

    let p1 = make_server("127.0.0.1:4000".to_string(), vec![SocketAddr::from(([127, 0, 0, 1], 3000))]);

    thread::scope(|s| {
        s.spawn(|| {
            thread::sleep(Duration::from_secs(30));
            println!("Shutting down server...");
            server.clone().shutdown();
        });
        s.spawn(|| {
            server.clone().start().unwrap();
        });
        s.spawn(|| {
            thread::sleep(Duration::from_secs(5));
            p1.clone().start().unwrap();
        });
    });
}