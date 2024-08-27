pub mod transport;

use transport::tcp::{self, TCPTransportOpts};
use transport::p2p::P2P;

fn main() {
    // FIXME: this is for testing only. should be updated later
    let opts = TCPTransportOpts {
        listen_addr: String::from("localhost:3000"),
        shakehands: |_| Ok(()),
    };
    let tcp_transport = tcp::new_tcp_transport(opts);

    tcp_transport.listen_and_accept().unwrap();

    loop {
        // keep the main thread alive
    }
}