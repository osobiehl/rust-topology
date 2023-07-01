use std::collections::VecDeque;
use std::time::Duration;

use log::trace;
use smoltcp::iface::{Interface, Config};
use smoltcp::phy::{Device, DeviceCapabilities, Medium};
use smoltcp::time::Instant;
use smoltcp::wire::{IpCidr};
use tokio::sync::mpsc::{UnboundedSender};
use rand::prelude::*;
use crate::async_communication::{AsyncChannel};
pub struct STDRx(pub(crate) Vec<u8>);
impl smoltcp::phy::RxToken for STDRx {
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut b = self.0;
        return f(&mut b);
    }
}

pub struct STDTx(pub(crate) UnboundedSender<Vec<u8>>);

impl smoltcp::phy::TxToken for STDTx {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut bytes = Vec::with_capacity(len);
        unsafe {bytes.set_len(len)};
        let res = f(&mut bytes[..]);
        self.0.send(bytes).expect("could not send Tx!");
        return res;
    }
}

pub struct AsyncGatewayDevice<T: AsyncChannel<Vec<u8>>>{
    pub cached: VecDeque<Vec<u8>>,
    pub gateway: T
}

impl<T: AsyncChannel<Vec<u8>>> AsyncGatewayDevice<T>{
    pub fn new( gateway:  T) -> Self{
        return Self { cached: VecDeque::new(), gateway };
    }
}
#[async_trait::async_trait]
impl<T: AsyncChannel<Vec<u8>>>  AsyncChannel<Vec<u8>> for AsyncGatewayDevice<T>{
    fn send(&mut self, msg: Vec<u8>){
        self.gateway.send(msg);
    }
    async fn receive(&mut self) -> Vec<u8>{
        self.gateway.receive().await
    }

    async fn receive_with_timeout(&mut self, timeout: Duration) -> Option<Vec<u8>>{
        self.gateway.receive_with_timeout(timeout).await
    }

    fn try_receive(&mut self) -> Option<Vec<u8>>{
        self.gateway.try_receive()
    }

    fn sender(&self) -> UnboundedSender<Vec<u8>>
    {
        self.gateway.sender()
    }

}

pub trait AsyncDevice: Device + AsyncChannel<Vec<u8>>{
    fn inject_frame(&mut self, frame: Vec<u8>);
}

impl<T: AsyncChannel<Vec<u8>>> AsyncDevice for AsyncGatewayDevice<T>{
    fn inject_frame(&mut self, frame: Vec<u8>) {
        self.cached.push_back(frame);
    }
}

impl<T: AsyncChannel<Vec<u8>>> Into<AsyncGatewayDevice<T>> for (T, ){
    fn into(self) -> AsyncGatewayDevice<T> {
        return AsyncGatewayDevice { cached: VecDeque::new(), gateway: self.0 }
    }
}

impl<T: AsyncChannel<Vec<u8>>>  Device for AsyncGatewayDevice<T> {
    type RxToken<'a> = STDRx where T: 'a; 
    type TxToken<'a> = STDTx where T: 'a;

    fn capabilities(&self) -> DeviceCapabilities {
        let mut d = DeviceCapabilities::default();
        d.medium = Medium::Ip;
        
        d.max_transmission_unit = 65000;
        return d ;
    }

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        if let Some(v) = self.cached.pop_front(){
            return Some((STDRx(v), STDTx(self.gateway.sender())));
        }
        else if let Some(v) = self.gateway.try_receive() {
            return Some((STDRx(v), STDTx(self.gateway.sender())));
        }
        return None;
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(STDTx(self.gateway.sender()))
    }
}



pub struct  NetifPair<D: AsyncDevice> {
    pub iface: Box<Interface>,
    pub device: Box<D>
}

pub fn setup_if<D: AsyncDevice>( ip_address: IpCidr, mut device: Box<D> ) -> NetifPair<D> {
    let mut config = Config::default();
    config.random_seed = random();
    config.hardware_addr = None;
    
    let mut netif = Interface::new(config, device.as_mut());
    netif.update_ip_addrs(|addrs| {
        let _ = addrs.push(ip_address).map_err( |_| {println!("could not add ip addr: {}", ip_address)});
    });
    netif.set_any_ip(false);

    return NetifPair{
        iface: Box::new(netif),
        device
    }
}
