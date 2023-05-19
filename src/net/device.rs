use smoltcp::iface::{Interface, Config};
use smoltcp::phy::{Device, DeviceCapabilities, self, Medium};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpCidr};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use rand::prelude::*;
use crate::async_communication::{AsyncGateway, AsyncChannel};
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

impl Device for AsyncGateway<Vec<u8>> {
    type RxToken<'a> = STDRx;
    type TxToken<'a> = STDTx;

    fn capabilities(&self) -> DeviceCapabilities {
        let mut d = DeviceCapabilities::default();
        d.medium = Medium::Ip;
        
        d.max_transmission_unit = 65000;
        return d ;
    }

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        if let Some(v) = self.try_receive() {
            return Some((STDRx(v), STDTx(self.tx.clone())));
        }
        return None;
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(STDTx(self.tx.clone()))
    }
}
pub struct  NetifPair<D: smoltcp::phy::Device> {
    pub iface: Box<Interface>,
    pub device: Box<D>
}

pub fn setup_if<D: phy::Device>( ip_address: IpCidr, mut device: Box<D> ) -> NetifPair<D> {
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
