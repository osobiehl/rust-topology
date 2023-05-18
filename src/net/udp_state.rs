use crate::net::device::{STDRx,STDTx,TokioChannel, setup_if};
use futures::FutureExt;
use smoltcp::socket::udp::PacketBuffer;
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address, Ipv6Address, IpEndpoint, IpListenEndpoint};
use smoltcp::socket::{tcp, udp};
use smoltcp::iface::{Config, Interface, SocketSet, SocketHandle};
use smoltcp::time::Instant;
use std::sync::Arc;
use std::task::{Poll, Context};
use tokio::sync::Mutex;
use std::pin::Pin;
use std::future::Future;



pub struct AsyncUDPSocket<'a, D: smoltcp::phy::Device> {
    handle: SocketHandle,
    state: Arc<Mutex<UDPState<'a, D> >>
}
pub type UdpResponse = (Vec<u8>, smoltcp::wire::IpEndpoint);
impl <'a, D: smoltcp::phy::Device> AsyncUDPSocket<'a, D> {
    pub async fn new<T: Into<IpListenEndpoint>>( src_port: T, state: Arc<Mutex<UDPState<'a, D> >> ) -> AsyncUDPSocket<'a, D>{
        let state_2 = state.clone();
        let mut s = state.lock().await;
        return Self{
            handle: s.new_socket(src_port),
            state: state_2
        };
    }

    pub fn recv<'b> (&'b mut self) -> UDPSocketRead<'a, 'b , D > {
        return  UDPSocketRead(self);
    }


    pub async fn send( &mut self, mut payload: Vec<u8>, endpoint: IpEndpoint){
        let mut state =  self.state.lock().await;
        let s = state.sockets.get_mut::<udp::Socket>(self.handle);
        s.send_slice(&mut payload, endpoint).expect("could not send message!");
        drop(s);
        state.poll();
    }

}

pub struct UDPSocketRead<'a, 'b , D: smoltcp::phy::Device>(
   pub(crate) &'b mut AsyncUDPSocket<'a, D>);


use futures::pin_mut;
pub type UdpResult = Result<UdpResponse, ()>;
impl<'a, 'b , D: smoltcp::phy::Device> std::future::Future for UDPSocketRead<'a,'b, D>{
    type Output = UdpResult;
    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let fut = self.0.state.lock();
        pin_mut!(fut);
        let Poll::Ready(mut state) = fut.poll(cx) else {
            return Poll::Pending;
        };
        state.poll();
        let socket = state.sockets.get_mut::<udp::Socket>(self.0.handle.clone());
        if !socket.is_open(){
            println!("err: socket not open");
            return Poll::Ready(Err(()));
        }
        if let Ok( (bytes, endpoint)) = socket.recv() {
            return Poll::Ready(Ok( (Vec::from(bytes), endpoint) ))
        }
        else {
            socket.register_recv_waker(cx.waker());
        }
        return Poll::Pending;
        
    }
}



pub struct UDPState<'a, D: smoltcp::phy::Device>{
    pub sockets: SocketSet<'a>,
    pub handles: Vec<SocketHandle>,
    pub iface: Box<Interface>,
    pub device: D
}

impl<'a, D: smoltcp::phy::Device> UDPState<'a, D>{
    pub fn new(iface: Box<Interface>, device: D)->Self{
        Self { sockets: SocketSet::new(vec![]), handles: vec![], iface, device}
    }

    pub fn new_socket<T: Into<IpListenEndpoint>>(&mut self, endpoint:T)->SocketHandle{
        let udp_rx_buffer1 = udp::PacketBuffer::new(
            vec![udp::PacketMetadata::EMPTY, udp::PacketMetadata::EMPTY],
            vec![0; 65535],
        );
        let udp_tx_buffer1 = udp::PacketBuffer::new(
            vec![udp::PacketMetadata::EMPTY, udp::PacketMetadata::EMPTY],
            vec![0; 65535],
        );
        let mut udp_socket = udp::Socket::new(udp_rx_buffer1, udp_tx_buffer1);
        udp_socket.bind(endpoint).expect("could not bind to addr");

        let udp_handle = self.sockets.add(udp_socket);
        self.handles.push(udp_handle.clone());
        
        return udp_handle;
    }
    pub fn poll(&mut self)-> bool{
        let timestamp = Instant::now();
        self.iface.poll(timestamp, &mut self.device, &mut self.sockets)
    }


 
}
