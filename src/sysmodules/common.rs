use crate::channel::async_communication::{AsyncGateway, SysmoduleRPC};
use crate::net::device::AsyncGatewayDevice;


use async_trait::async_trait;

use log::info;
use smoltcp::wire::{IpListenEndpoint, IpCidr, Ipv4Cidr};
use tokio::sync::Mutex;



use std::sync::Arc;
use std::time::Duration;


use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use crate::net::udp_state::{AsyncSocketHandle, NetStack, UDPState, UDP, Raw, RawDirection};
use smoltcp::wire::{IpAddress, Ipv4Address};

pub const TRANSIENT_HMI_ID: IpAddress = IpAddress::v4(192, 168, 69, 1);
pub const TRANSIENT_PV_ID: IpAddress = IpAddress::v4(192, 168, 69, 2);
pub const TRANSIENT_PI_ID: IpAddress = IpAddress::v4(192, 168, 69, 3);
pub const TRANSIENT_GATEWAY_ID: IpAddress = IpAddress::v4(192, 168, 69, 4);
pub const ADDRESS_ASSIGNMENT_PORT: u16 = 6967;

use crate::net::udp_state::AsyncSocket;

pub type TestingReceiver= UnboundedReceiver<SysmoduleRPC>;
pub type TestingSender = UnboundedSender<SysmoduleRPC>;

pub type Device = AsyncGatewayDevice<AsyncGateway<Vec<u8>>>;
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BasicModuleType{
    HMI,
    PV,
    PI,
}
pub struct BasicModule {
    pub(super) testing_interface: TestingReceiver,
    netif: Arc<Mutex<UDPState<Device>>>,
    mod_type: BasicModuleType
}


#[async_trait::async_trait]
impl NetStack<Device> for Arc<Mutex<UDPState<Device>>>{
    async fn socket<T:Into<IpListenEndpoint> + Send> (&self, endpoint: T) -> AsyncSocketHandle<Device, UDP>{
        AsyncSocketHandle::<Device,UDP>::new_udp(endpoint,self.clone()).await
    }
    async fn raw_socket(&self) -> AsyncSocketHandle<Device, Raw>{
        AsyncSocketHandle::<Device, Raw>::new_raw(self.clone()).await
    }
    async fn raw_direction_socket (&self)-> AsyncSocketHandle<Device, RawDirection>{
        AsyncSocketHandle::<Device, RawDirection>::new(self.clone()).await
    }

    async fn modify_netif<F>(&self, f: F) where F: FnOnce( & mut UDPState< Device>) + Send {
        let mut netif = self.lock().await;
        f( &mut *netif);
    }
}



impl BasicModule {
    pub fn new(
        netif: Arc<Mutex<UDPState<Device>>>,
        testing_interface: TestingReceiver,
        mod_type: BasicModuleType,
    ) -> Self {
        Self {
            testing_interface,
            netif,
            mod_type
        }
    }
    pub async fn socket<T:Into<IpListenEndpoint> + Send> (&self, endpoint: T) -> AsyncSocketHandle<Device, UDP>{
        AsyncSocketHandle::<Device,UDP>::new_udp(endpoint,self.netif.clone()).await
    }
    pub async fn raw_socket(&self) -> AsyncSocketHandle<Device, Raw>{
        AsyncSocketHandle::<Device, Raw>::new_raw(self.netif.clone()).await
    }

}

pub struct PI {
    pub base: BasicModule,
    // hart_interface: AsyncGateway<Vec<u8>>, TODO Change
}

pub struct PV(pub BasicModule);

pub struct HMI(pub BasicModule);

#[async_trait]
pub trait SysModuleStartup {
    async fn on_start(&mut self);
    async fn run_once(&mut self);
}


#[async_trait]
impl SysModuleStartup for BasicModule {
    async fn on_start(&mut self) {
        let mut socket_internal_bus = self.socket(ADDRESS_ASSIGNMENT_PORT).await;
        if let Ok((val,_req)) = socket_internal_bus.receive_with_timeout(Duration::from_millis(30)).await{
        assert!(val.len() == 4, "non-ipv4 message received!");
        let new_ip: Ipv4Address = Ipv4Address::new(val[0], val[1], val[2], val[3]);
        println!("RECV new ip adddr: {}, module {:?}", new_ip, &self.mod_type);
        self.netif.modify_netif( move |state| {
            state.netifs[0].iface.update_ip_addrs( |addrs| {
                addrs.clear();
                addrs.push(IpCidr::Ipv4(Ipv4Cidr::new(new_ip, 24)));
            })
        }).await
    }
    else{
        info!("module did not receive message on startup, module: {:?}", &self.mod_type);
    }
    //wait for all other modules to finish address assignment
    tokio::time::sleep(Duration::from_millis(100)).await;


    }
    async fn run_once(&mut self) {
        
        let timeout = tokio::time::timeout(Duration::from_millis(5000), self.testing_interface.recv()).await;
        if let Ok(f) = timeout{
            let closure = f.unwrap();
            closure(self).await;
        }

    }
}

