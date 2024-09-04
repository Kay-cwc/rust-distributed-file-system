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
use transport::tcp::{self, TcpTransport, TcpTransportOpts};

fn make_server(listen_addr: String, nodes: Vec<SocketAddr>) -> Arc<FileServer<TcpTransport>> {
    // create the transport layer
    let opts = TcpTransportOpts::new(listen_addr.clone(), Box::new(DefaultDecoder {}));
    let tcp_transport = tcp::TcpTransport::new(opts);
    
    let file_server_opts = FileServerOpts {
        store_opts: store::store::StoreOpts {
            root_dir: format!("storage/{}", listen_addr),
            filename_transform: store::hashlib::filename_transform,
        },
        transport: tcp_transport.clone(),
        bootstrap_node: nodes,
    };

    let server = FileServer::new(file_server_opts);
    
    server
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