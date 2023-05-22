use crate::async_communication::AsyncChannel;

use futures::FutureExt;

use log::{trace};

use smoltcp::wire::{IpAddress, IpEndpoint, IpListenEndpoint};
use smoltcp::socket::{udp, AnySocket,raw};
use smoltcp::iface::{SocketSet, SocketHandle};

use smoltcp::time::Instant;
use std::marker::PhantomData;
use std::sync::Arc;
use std::task::{Poll};
use std::time::Duration;
use tokio::sync::Mutex;

use std::future::Future;
use async_trait::async_trait;
#[async_trait]
pub trait NetStack<D: AsyncDevice>{
    async fn udp_socket<T:Into<IpListenEndpoint> + Send> (&self, endpoint: T) -> AsyncSocket<D, udp::Socket>;
    async fn modify_netif< F>(&self, f: F) where F: FnOnce( & mut NetworkCore<D>) + Send;
    async fn raw_socket() -> AsyncSocket<D, raw::Socket<'static>>;
}

pub struct AsyncSocket< D: AsyncDevice, Sock: AnySocket<'static> >  {
    socket_type: PhantomData<Sock>,
    handle: SocketHandle,
    state: Arc<Mutex<NetworkCore< D> >>
}
pub type UdpResponse = (Vec<u8>, smoltcp::wire::IpEndpoint);
impl <D: AsyncDevice + AsyncChannel<Vec<u8>>, Sock: AnySocket<'static>> AsyncSocket< D, Sock> {


    pub fn recv<'b, > (&'b mut self) -> SocketRead<'b , D, Sock > {
        return  SocketRead(self);
    }

    pub async fn receive_with_timeout(&mut self, timeout: Duration) -> UdpResult{
        let ans: Result<(Vec<u8>, IpEndpoint), ()> = futures::select! {
            x = self.recv().fuse() => x,
            _ = tokio::time::sleep(timeout).fuse() => Err(())
        };
        return ans;
    }
    
    pub async fn send( &mut self, mut payload: Vec<u8>, endpoint: IpEndpoint){
        let mut state =  self.state.lock().await;
        let s = state.sockets.get_mut::<udp::Socket>(self.handle);
        s.send_slice(&mut payload, endpoint).expect("could not send message!");
        drop(s);
        state.poll_for_send(&endpoint.addr);
    }

}



impl<D: AsyncDevice + AsyncChannel<Vec<u8>>> AsyncSocket< D, udp::Socket<'static>>{
    pub async fn new_udp<T: Into<IpListenEndpoint>>( src_port: T, state: Arc<Mutex<NetworkCore< D> >> ) -> AsyncSocket< D,udp::Socket<'static>>{
        let state_2 = state.clone();
        let mut s = state.lock().await;
        return Self{
            socket_type: Default::default(),
            handle: s.new_udp_socket(src_port),
            state: state_2
        };
    }
}

pub struct SocketRead<'b , D: AsyncDevice,  Sock: AnySocket<'static> >(
   pub(crate) &'b mut AsyncSocket< D, Sock>);


use futures::pin_mut;
use crate::net::device::{AsyncDevice};
pub type UdpResult = Result<UdpResponse, ()>;
impl< 'b , D: AsyncDevice, Sock: AnySocket<'static> > std::future::Future for SocketRead<'b, D, Sock>{
    type Output = UdpResult;
    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let fut = self.0.state.lock();
        pin_mut!(fut);
        let Poll::Ready(mut state) = fut.poll(cx) else {
            return Poll::Pending;
        };
        for netif in &mut state.netifs{
            let mut r = AsyncChannel::receive(netif.device.as_mut());
            let y = r.poll_unpin(cx);
            drop(r);
            if let Poll::Ready(x) = y{
                netif.device.inject_frame(x);
            }
        }
        state.poll();
        let socket = state.sockets.get_mut::<Sock>(self.0.handle.clone());
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

use crate::net::device::NetifPair;

pub struct NetworkCore< D: AsyncDevice>{
    pub sockets: SocketSet<'static>,
    pub handles: Vec<SocketHandle>,
    pub netifs: Vec<NetifPair<D>>,

}

impl<'a, D: AsyncDevice> NetworkCore< D>{
    pub fn new(netifs: Vec<NetifPair<D>>)->Self{
        Self { sockets: SocketSet::new(vec![]), handles: vec![], netifs}
    }

    pub fn new_udp_socket<T: Into<IpListenEndpoint>>(&mut self, endpoint:T)->SocketHandle{
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


    

    pub fn poll_for_send(&mut self, endpoint: &IpAddress)
    {
        if endpoint.is_unicast(){
  
            let mut done = false;
            for iface in &mut self.netifs{
                let _found: bool = false;
                let found = iface.iface.ip_addrs().iter().find( |cidr| cidr.contains_addr(endpoint) );
                if let Some(x) = found {
                    let timestamp = Instant::now();
                    trace!("using interface: {} for request {}", x, endpoint );
                    iface.iface.poll(timestamp, iface.device.as_mut(), &mut self.sockets);
                    done = true;
                    break;
                }
            }
            if !done {
                trace!("warn: no route found for {}", endpoint);
            }
        }
        if endpoint.is_broadcast(){
            trace!("sending broadcast through default netif");
            self.poll();
        }
        else if endpoint.is_multicast(){
            trace!("warn: multicast not supported yet");
            self.poll();
        }
        else {
            self.poll();
        }
    }


    pub fn poll(&mut self)-> bool{
        let timestamp = Instant::now();
        let mut worked = false;
        for x in self.netifs.iter_mut(){
            if x.iface.poll(timestamp, x.device.as_mut(), &mut self.sockets) == true {
                worked = true;
            }
        }
        return worked;
    }


 
}
