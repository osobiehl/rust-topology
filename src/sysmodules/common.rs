use crate::async_communication::{AsyncGateway, ChannelEvent, IPMessage, SysmoduleRPC};
use std::net::Ipv4Addr;
use std::pin::Pin;
use async_trait::async_trait;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::select;
use tokio::select_variant;
use crate::{async_communication::AsyncChannel, sysmodule::HubIndex};
use either::Either;

use super::*;
pub type TestingReceiver = UnboundedReceiver<SysmoduleRPC>;
pub type TestingSender = UnboundedSender<SysmoduleRPC>;
pub struct BasicModule {
    pub(super) internal_bus: AsyncGateway<IPMessage>,
    pub(super) testing_interface: TestingReceiver,
    pub(super) address: Ipv4Addr,
}

impl BasicModule {
    pub fn new(
        internal_bus: AsyncGateway<IPMessage>,
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
    hart_interface: AsyncGateway<ChannelEvent>,
    internal_bus: AsyncGateway<IPMessage>,

}

pub struct PV(pub BasicModule);

pub struct HMI(pub BasicModule);

#[async_trait]
pub trait SysModuleStartup{

    async fn on_start(&mut self);
    async fn run_once(&mut self);
}

///
/// This trait can be used when sending commands to a module to
/// override its behaviour and make it act differently
#[async_trait]
pub trait SysModule: Send{
    async fn receive(&mut self) -> IPMessage;
    fn send(&mut self, msg: IPMessage);
}

#[async_trait]
impl SysModuleStartup for  BasicModule{

    async fn on_start(&mut self){
        // no-op TODO: get address from COM
    }
    async fn run_once(&mut self){

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
            Either::Right(closure) => {closure(self).await}
        };
        
    }
     
}

#[async_trait]
impl SysModule for BasicModule{
    async fn receive(&mut self){
        
    }
    fn send(&mut self, msg: IPMessage){
        self.internal_bus.send(msg);
    }
}