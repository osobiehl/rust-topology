use crate::async_communication::{AsyncGateway, SysmoduleRPC};
use crate::net::device::AsyncGatewayDevice;
use crate::{async_communication::AsyncChannel};
use async_trait::async_trait;
use either::Either;
use smoltcp::wire::IpListenEndpoint;
use tokio::sync::Mutex;
use std::net::Ipv4Addr;

use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use crate::net::udp_state::{AsyncUDPSocket, NetStack, UDPState};
use futures::future::FutureExt;



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
    base: BasicModule,
    hart_interface: AsyncGateway<Vec<u8>>,
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
        // let test_future = Box::pin(self.testing_interface.recv());
        // TODO: change this
        let closure = self.testing_interface.recv().await.unwrap();
        closure(self).await;
    }
}

