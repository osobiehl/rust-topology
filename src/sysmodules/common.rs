use crate::async_communication::{AsyncGateway, SysmoduleRPC};
use crate::{async_communication::AsyncChannel};
use async_trait::async_trait;
use either::Either;
use tokio::sync::Mutex;
use std::net::Ipv4Addr;

use std::time::Duration;
use tokio::select;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use crate::net::udp_state::{AsyncUDPSocket, NetStack, UDPState};
use futures::future::FutureExt;

pub type TestingReceiver = UnboundedReceiver<SysmoduleRPC>;
pub type TestingSender = UnboundedSender<SysmoduleRPC>;

pub type Device = AsyncGateway<Vec<u8>>;
pub struct BasicModule {
    pub(super) internal_bus: AsyncGateway<Vec<u8>>,
    pub(super) testing_interface: TestingReceiver,
    pub(super) address: Ipv4Addr,
    // netif: Arc<Mutex<UDPState<'_, Device>>>
}

impl BasicModule {
    pub fn new(
        internal_bus: AsyncGateway<Vec<u8>>,
        testing_interface: TestingReceiver,
        address: Ipv4Addr,
    ) -> Self {
        Self {
            internal_bus,
            testing_interface,
            address,
        }
    }
}

pub struct PI {
    base: BasicModule,
    hart_interface: AsyncGateway<Vec<u8>>,
    internal_bus: AsyncGateway<Vec<u8>>,
}

pub struct PV(pub BasicModule);

pub struct HMI(pub BasicModule);

#[async_trait]
pub trait SysModuleStartup {
    async fn on_start(&mut self);
    async fn run_once(&mut self);
}

///
/// TODO: refactor this trait
#[async_trait]
pub trait SysModule: Send  + {
    async fn receive(&mut self) -> Vec<u8>;
    async fn try_receive(&mut self, timeout: Duration) -> Option<Vec<u8>> {
        let ans = select! {
            x = self.receive().fuse() => Some(x),
            _ = tokio::time::sleep(timeout).fuse() => None
        };
        return ans;
    }
    fn send(&mut self, msg: Vec<u8>);
}

#[async_trait]
impl SysModuleStartup for BasicModule {
    async fn on_start(&mut self) {
        // no-op TODO: get address from COM
    }
    async fn run_once(&mut self) {
        // let test_future = Box::pin(self.testing_interface.recv());

        let test_cmd = self.testing_interface.recv();
        let incoming = select! {
            ip_msg = self.internal_bus.receive() =>
                     Either::Left(ip_msg),
            test_closure = test_cmd => {
                let test = test_closure.unwrap();
                Either::Right(test)
            }
        };
        match incoming {
            Either::Left(ip_msg) => println!("recv: {:?} for address {}", &ip_msg, &self.address),
            Either::Right(closure) => closure(self).await,
        };
    }
}

#[async_trait]
impl SysModule for BasicModule {
    async fn receive(&mut self) -> Vec<u8> {
        self.internal_bus.receive().await
    }
    fn send(&mut self, msg: Vec<u8>) {
        self.internal_bus.send(msg);
    }
}
