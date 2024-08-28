pub mod transport;

use transport::encoding::DefaultDecoder;
use transport::tcp::{self, TCPTransportOpts};
use transport::transport::{ErrConnClose, Peer, Transport};

fn main() {
    // FIXME: this is for testing only. should be updated later
    let opts = TCPTransportOpts {
        listen_addr: String::from("localhost:3000"),
        shakehands: |_| Ok(()),
        decoder: Box::new(DefaultDecoder {}),
        on_peer: |_peer| {
            return Err(ErrConnClose);
        }
    };
    let tcp_transport = tcp::TCPTransport::new(opts);

    tcp_transport.clone().listen_and_accept().unwrap();

    println!("[server] waiting for msg...");
    loop {
        // keep the main thread alive
        let msg = tcp_transport.clone().consume().unwrap();
        println!("[server] received msg: {:?}", msg);
    }
}