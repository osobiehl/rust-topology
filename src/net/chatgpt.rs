use std::io;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use tokio::net::{UdpSocket, ToSocketAddrs};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task;

use smolltcp::{TcpStack, TcpStackBuilder};

// Structure representing the driver
struct SmollTcpDriver {
    tcp_stack: Arc<TcpStack>,
    tx: Sender<Vec<u8>>,
    rx: Receiver<Vec<u8>>,
}

impl SmollTcpDriver {
    // Creates a new instance of the driver
    async fn new<A: ToSocketAddrs>(local_addr: A) -> io::Result<Self> {
        let local_addr = local_addr.to_socket_addrs()?.next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid local address provided",
            )
        })?;

        let udp_socket = UdpSocket::bind(local_addr).await?;
        let tcp_stack = TcpStackBuilder::new().with_udp_socket(udp_socket).build();

        let (tx, rx) = mpsc::channel(100);

        Ok(Self {
            tcp_stack: Arc::new(tcp_stack),
            tx,
            rx,
        })
    }

    // Starts the driver's event loop
    async fn run(&mut self) -> io::Result<()> {
        loop {
            tokio::select! {
                Some(data) = self.rx.recv() => {
                    self.handle_outgoing_data(data).await?;
                }
                event = self.tcp_stack.next_event().await => {
                    if let Some(event) = event {
                        self.handle_incoming_event(event).await?;
                    }
                }
            }
        }
    }

    // Handles outgoing data from the Tokio channel
    async fn handle_outgoing_data(&mut self, data: Vec<u8>) -> io::Result<()> {
        // Perform necessary processing on the outgoing data
        // and send it through the TCP stack
        self.tcp_stack.send_data(data).await?;

        Ok(())
    }

    // Handles incoming events from the TCP stack
    async fn handle_incoming_event(&mut self, event: smolltcp::Event) -> io::Result<()> {
        match event {
            smolltcp::Event::DataReceived(data) => {
                // Perform necessary processing on the incoming data
                // and handle it accordingly
                // Here, we simply print the received data
                println!("Received data: {:?}", data);
            }
            smolltcp::Event::TcpConnectionClosed(_) => {
                // Handle TCP connection closure event
                // Here, we simply print that the connection was closed
                println!("TCP connection closed");
            }
            _ => {
                // Handle other events if necessary
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // Create the driver
    let mut driver = SmollTcpDriver::new("127.0.0.1:8080").await?;

    // Start the driver's event loop
    driver.run().await
}
