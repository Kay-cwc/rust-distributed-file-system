pub mod transport;

use transport::tcp;
use transport::p2p::P2P;

fn main() {
    // FIXME: this is for testing only. should be updated later
    let tcp_transport = tcp::new_tcp_transport(&String::from("localhost:3000"));

    println!("Listening on {}", tcp_transport.listen_addr);
    tcp_transport.listen_and_accept().unwrap();

    loop {
        // keep the main thread alive
    }
}