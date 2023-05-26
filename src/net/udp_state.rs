use crate::async_communication::AsyncChannel;

use futures::FutureExt;

use log::trace;

use smoltcp::iface::{SocketHandle, SocketSet};
use smoltcp::socket::{raw, udp, AnySocket};
use smoltcp::time::Instant;
use smoltcp::wire::{IpAddress, IpEndpoint, IpListenEndpoint, IpProtocol};
use std::marker::PhantomData;
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;
use tokio::sync::Mutex;

use async_trait::async_trait;
use std::future::Future;
#[async_trait]
pub trait NetStack<D: AsyncDevice> {
    async fn socket<T: Into<IpListenEndpoint> + Send>(&self, endpoint: T) -> AsyncSocketHandle<D, UDP>;
    async fn raw_socket(&self) -> AsyncSocketHandle<D, Raw>;
    async fn modify_netif<F>(&self, f: F)
    where
        F: FnOnce(&mut UDPState<D>) + Send;
}

pub trait SocketAdapter {
    type Output: Send ;
    type Destination: Send + Into<IpAddress> + Clone;
    type InputSocket<'a>: AnySocket<'a>;
    fn register_receive_waker<'a>(sock: &mut Self::InputSocket<'a>, waker: &std::task::Waker);
    fn receive<'a>(sock: &mut Self::InputSocket<'a>) -> Result<Self::Output, ()>;
    fn is_open<'a>(sock: &mut Self::InputSocket<'a>) -> bool;
    fn send<'a>(sock: &mut Self::InputSocket<'a>, data: &[u8], destination: Self::Destination);
    fn close<'a>(sock: &mut Self::InputSocket<'a>);

}

pub struct AsyncUDP<'a>(pub &'a mut udp::Socket<'a>);
pub struct AsyncRaw<'a>(pub &'a mut raw::Socket<'a>);
pub type UdpOutput = (Vec<u8>, IpEndpoint);

pub struct UDP {}
pub struct Raw {}

#[derive(Clone, Copy, Debug)]
pub struct IPEndpoint {
    pub addr: IpAddress,
    pub port: u16
}
impl Into<IpAddress> for IPEndpoint{
    fn into(self) -> IpAddress {
        return self.addr
    }
}

impl SocketAdapter for UDP {
    type Output = UdpOutput;
    type InputSocket<'a> = udp::Socket<'a>;
    type Destination = IPEndpoint;
    fn receive<'a>(sock: &mut Self::InputSocket<'a>) -> Result<Self::Output, ()> {
        sock.recv()
            .map(|(bytes, endpoint)| (Vec::<u8>::from(bytes), endpoint))
            .map_err(|_| ())
    }

    fn register_receive_waker<'a>(sock: &mut Self::InputSocket<'a>, waker: &std::task::Waker) {
        sock.register_recv_waker(waker);
    }

    fn is_open<'a>(sock: &mut Self::InputSocket<'a>) -> bool {
        sock.is_open()
    }
    fn send<'a>(sock: &mut Self::InputSocket<'a>, data: &[u8], destination: Self::Destination){
        let _ = sock.send_slice(data, IpEndpoint{addr: destination.addr, port: destination.port});
    }
    fn close<'a>(sock: &mut Self::InputSocket<'a>){
        sock.close();
    }

}

impl SocketAdapter for Raw {
    type Output = Vec<u8>;
    type Destination = IpAddress;
    type InputSocket<'a> = raw::Socket<'a>;
    fn receive<'a>(sock: &mut Self::InputSocket<'a>) -> Result<Self::Output, ()> {
        sock.recv().map(Vec::<u8>::from).map_err(|_| ())
    }

    fn register_receive_waker<'a>(sock: &mut Self::InputSocket<'a>, waker: &std::task::Waker) {
        sock.register_recv_waker(waker);
    }

    fn is_open<'a>(_sock: &mut Self::InputSocket<'a>) -> bool {
        true
    }
    fn send<'a>(sock: &mut Self::InputSocket<'a>, data: &[u8], _destination: Self::Destination) {
        

        let _ = sock.send_slice(&data);
    }
    fn close<'a>(sock: &mut Self::InputSocket<'a>){
        // raw sockets cannot be closed (?) 
    }
}

pub struct AsyncSocketHandle<D: AsyncDevice, SockType: SocketAdapter> {
    handle: SocketHandle,
    state: Arc<Mutex<UDPState<D>>>,
    socket: PhantomData<SockType>
}

pub type UDPSocket<D> = AsyncSocketHandle<D, UDP>;

#[async_trait::async_trait]
pub trait AsyncSocket<D, A> where 
D: AsyncDevice + AsyncChannel<Vec<u8>>,
A: SocketAdapter + Send {
    fn recv(&mut self) -> AsyncSocketRead<D, A>;
    async fn send(&mut self, data: &[u8], dest: A::Destination);
    async fn receive_with_timeout(&mut self, timeout: Duration) -> Result<A::Output, ()>{
        let ans: Result<A::Output, ()> = futures::select! {
            x = self.recv().fuse() => x,
            _ = tokio::time::sleep(timeout).fuse() => Err(())
        };
        return ans;
    }
}
#[async_trait::async_trait]
impl<D, A> AsyncSocket<D,A> for AsyncSocketHandle<D, A> where
D: AsyncDevice,
A: SocketAdapter + Send{
    fn recv(&mut self) -> AsyncSocketRead<D, A>{
        return AsyncSocketRead {
            handle: self.handle,
            socket: Default::default(),
            state: self.state.clone(),
        };
    }
    async fn send(&mut self, data: &[u8], dest: A::Destination){
        let mut state = self.state.lock().await;
        let s = state.sockets.get_mut::<A::InputSocket<'static> >(self.handle);
        A::send(s, data, dest.clone());
        drop(s);
        state.poll_for_send(&dest.into());
    }
}


pub type UdpResponse = (Vec<u8>, smoltcp::wire::IpEndpoint);
impl<D: AsyncDevice + AsyncChannel<Vec<u8>>> AsyncSocketHandle<D, UDP> {
    pub async fn new_udp<T: Into<IpListenEndpoint>>(
        src_port: T,
        state: Arc<Mutex<UDPState<D>>>,
    ) -> AsyncSocketHandle<D, UDP> {
        let state_2 = state.clone();
        let mut s = state.lock().await;
        return Self {
            handle: s.new_socket(src_port),
            state: state_2,
            socket: Default::default()
        };
    }
}

impl<D: AsyncDevice + AsyncChannel<Vec<u8>>> AsyncSocketHandle<D, Raw> {
    pub async fn new_raw(
        state: Arc<Mutex<UDPState<D>>>,
    ) -> AsyncSocketHandle<D, Raw> {
        let state_2 = state.clone();
        let mut s = state.lock().await;
        return Self {
            handle: s.new_raw_socket(),
            state: state_2,
            socket: Default::default()
        };
    }
}


// todo: phantomdata for type, just use socket
pub struct AsyncSocketRead<D: AsyncDevice, SockType: SocketAdapter> {
    pub(crate) handle: SocketHandle,
    pub(crate) state: Arc<Mutex<UDPState<D>>>,
    socket: PhantomData<SockType>,
}

impl<D: AsyncDevice, SockType: SocketAdapter> Drop for AsyncSocketHandle<D, SockType> {

    fn drop(&mut self) {
        let s = self.state.clone();
        let handle = self.handle.clone();
        tokio::task::block_in_place( move || {
            let mut state = s.blocking_lock();
            let sock = state.sockets.get_mut::<SockType::InputSocket<'static>>(handle);
            SockType::close(sock);

        });
    }
}

use crate::net::device::AsyncDevice;
use futures::pin_mut;

impl<D, SockType> std::future::Future for AsyncSocketRead<D, SockType>
where
    D: AsyncDevice,
    SockType: SocketAdapter,
{
    type Output = Result<SockType::Output, ()>;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        // get state mutex
        let fut = self.state.lock();
        pin_mut!(fut);
        let Poll::Ready(mut state) = fut.poll(cx) else {
            return Poll::Pending;
        };
        for netif in &mut state.netifs {
            let mut r = AsyncChannel::receive(netif.device.as_mut());
            let y = r.poll_unpin(cx);
            drop(r);
            if let Poll::Ready(x) = y {
                netif.device.inject_frame(x);
            }
        }
        state.poll();
        let handle = self.handle.clone();
        let socket = state
            .sockets
            .get_mut::<SockType::InputSocket<'static>>(handle);
        if !SockType::is_open(socket) {
            println!("err: socket not open");
            return Poll::Ready(Err(()));
        }
        if let Ok(out) = SockType::receive(socket) {
            return Poll::Ready(Ok(out));
        } else {
            SockType::register_receive_waker(socket, cx.waker());
        }
        return Poll::Pending;
    }
}

use crate::net::device::NetifPair;

pub struct UDPState<D: AsyncDevice> {
    pub sockets: SocketSet<'static>,
    pub handles: Vec<SocketHandle>,
    pub netifs: Vec<NetifPair<D>>,
}


impl<'a, D: AsyncDevice> UDPState<D> {
    pub fn new(netifs: Vec<NetifPair<D>>) -> Self {
        Self {
            sockets: SocketSet::new(vec![]),
            handles: vec![],
            netifs,
        }
    }

    pub fn new_raw_socket(&mut self) -> SocketHandle {
        let udp_rx_buffer1 = raw::PacketBuffer::new(
            vec![raw::PacketMetadata::EMPTY, raw::PacketMetadata::EMPTY],
            vec![0; 65535],
        );
        let udp_tx_buffer1 = raw::PacketBuffer::new(
            vec![raw::PacketMetadata::EMPTY, raw::PacketMetadata::EMPTY],
            vec![0; 65535],
        );
        let raw_socket = raw::Socket::new(
            smoltcp::wire::IpVersion::Ipv4,
            IpProtocol::Udp,
            udp_rx_buffer1,
            udp_tx_buffer1,
        );

        let raw_handle = self.sockets.add(raw_socket);
        self.handles.push(raw_handle.clone());
        return raw_handle
    }

    pub fn new_socket<T: Into<IpListenEndpoint>>(&mut self, endpoint: T) -> SocketHandle {
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

    pub fn poll_for_send(&mut self, endpoint: &IpAddress) {
        if endpoint.is_unicast() {
            let mut done = false;
            for iface in &mut self.netifs {
                let _found: bool = false;
                let found = iface
                    .iface
                    .ip_addrs()
                    .iter()
                    .find(|cidr| cidr.contains_addr(endpoint));
                if let Some(x) = found {
                    let timestamp = Instant::now();
                    trace!("using interface: {} for request {}", x, endpoint);
                    iface
                        .iface
                        .poll(timestamp, iface.device.as_mut(), &mut self.sockets);
                    done = true;
                    break;
                }
            }
            if !done {
                trace!("warn: no route found for {}", endpoint);
            }
        }
        if endpoint.is_broadcast() {
            trace!("sending broadcast through default netif");
            self.poll();
        } else if endpoint.is_multicast() {
            trace!("warn: multicast not supported yet");
            self.poll();
        } else {
            self.poll();
        }
    }

    pub fn poll(&mut self) -> bool {
        let timestamp = Instant::now();
        let mut worked = false;
        for x in self.netifs.iter_mut() {
            if x.iface
                .poll(timestamp, x.device.as_mut(), &mut self.sockets)
                == true
            {
                worked = true;
            }
        }
        return worked;
    }
}
