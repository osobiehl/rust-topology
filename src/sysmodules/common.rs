use crate::async_communication::{AsyncGateway, SysmoduleRPC};
use crate::net::device::AsyncGatewayDevice;

use async_trait::async_trait;

use smoltcp::wire::IpListenEndpoint;
use tokio::sync::Mutex;


use std::future;
use std::sync::Arc;
use std::time::Duration;


use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use crate::net::udp_state::{AsyncUDPSocket, NetStack, UDPState};
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address, Ipv6Address};

pub const TRANSIENT_HMI_ID: IpAddress = IpAddress::v4(192, 168, 69, 1);
pub const TRANSIENT_PV_ID: IpAddress = IpAddress::v4(192, 168, 69, 2);
pub const TRANSIENT_PI_ID: IpAddress = IpAddress::v4(192, 168, 69, 3);
pub const TRANSIENT_GATEWAY_ID: IpAddress = IpAddress::v4(192, 168, 69, 4);




pub type TestingReceiver= UnboundedReceiver<SysmoduleRPC>;
pub type TestingSender = UnboundedSender<SysmoduleRPC>;

pub type Device = AsyncGatewayDevice<AsyncGateway<Vec<u8>>>;
pub struct BasicModule {
    pub(super) testing_interface: TestingReceiver,
    netif: Arc<Mutex<UDPState<Device>>>
}
#[async_trait::async_trait]
impl NetStack<Device> for BasicModule{
    async fn socket<T:Into<IpListenEndpoint> + Send> (&self, endpoint: T) -> AsyncUDPSocket<Device>{
        AsyncUDPSocket::new(endpoint,self.netif.clone()).await
    }
    async fn modify_netif<F>(&self, f: F) where F: FnOnce( & mut UDPState< Device>) + Send {
        let mut netif = self.netif.lock().await;
        f( &mut *netif);
    }
}

impl BasicModule {
    pub fn new(
        netif: Arc<Mutex<UDPState<Device>>>,
        testing_interface: TestingReceiver,
    ) -> Self {
        Self {
            testing_interface,
            netif,
        }
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
        // no-op TODO: get address from COM
    }
    async fn run_once(&mut self) {
        tokio::time::sleep(Duration::from_millis(1000)).await;
        // let test_future = Box::pin(self.testing_interface.recv());
        // TODO: change this
        // let closure = self.testing_interface.recv().await.unwrap();
        // closure(self).await;
    }
}

