use smoltcp::iface::{Interface, Config};
use smoltcp::phy::{Device, DeviceCapabilities, self, Medium};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpCidr, IpProtocol, Ipv4Packet, Ipv4Repr, Ipv4Cidr};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use rand::prelude::*;

struct TokioChannel {
    rx: UnboundedReceiver<Vec<u8>>,
    tx: UnboundedSender<Vec<u8>>,
}

impl TokioChannel {}
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
        let res = f(&mut bytes);
        self.0.send(bytes);
        return res;
    }
}

impl Device for TokioChannel {
    type RxToken<'a> = STDRx;
    type TxToken<'a> = STDTx;

    fn capabilities(&self) -> DeviceCapabilities {
        let mut d = DeviceCapabilities::default();
        d.medium = Medium::Ip;
        return d ;
    }

    fn receive(&mut self, timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        if let Ok(v) = self.rx.try_recv() {
            return Some((STDRx(v), STDTx(self.tx.clone())));
        }
        return None;
    }

    fn transmit(&mut self, timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(STDTx(self.tx.clone()))
    }
}
impl TokioChannel {
    pub fn new(rx: UnboundedReceiver<Vec<u8>>, tx: UnboundedSender<Vec<u8>>) -> Self {
        Self { tx, rx }
    }
}

fn setup_if<D: phy::Device>( ip_address: IpCidr, mac_address: EthernetAddress, device: &mut D ) -> Box<Interface> {
    let file_path = "output.txt";
    let mut config = Config::default();
    config.random_seed = random();
    config.hardware_addr = Some(smoltcp::wire::HardwareAddress::Ethernet(mac_address)); //TODO remove;
    
    let mut netif = Interface::new(config, device);
    netif.update_ip_addrs(|addrs| {
        let _ = addrs.push(ip_address).map_err( |_| {println!("could not add ip addr: {}", ip_address)});
    });

    return Box::new(netif)
}
