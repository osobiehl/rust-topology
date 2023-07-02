
use futures::future::BoxFuture;
use futures::{select, FutureExt};
use std::fmt::Debug;



use std::time::Duration;

use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::net::device::AsyncGatewayDevice;
use crate::sysmodules::common::BasicModule;



pub type SysmoduleRPC = Box<dyn FnOnce(&mut BasicModule) -> BoxFuture<()> + Send>;



pub struct AsyncGateway<T> {
    pub tx: UnboundedSender<T>,
    rx: UnboundedReceiver<T>,
}

impl<T> AsyncGateway<T> {
    pub fn new() -> (Self, Self) {
        let (a_send, b_rx) = unbounded_channel();
        let (c_send, d_rx) = unbounded_channel();
        return (
            Self {
                tx: (a_send),
                rx: (d_rx),
            },
            Self {
                tx: (c_send),
                rx: (b_rx),
            },
        );
    }


}


impl AsyncGateway<Vec<u8>>{
    pub fn new_async_device() -> (AsyncGatewayDevice<Self>, AsyncGatewayDevice<Self>){
        let (g_a, g_b ) = Self::new();
        return (AsyncGatewayDevice::new(g_a), AsyncGatewayDevice::new(g_b))
    }
}

#[async_trait::async_trait]
pub trait AsyncChannel<T>: Send {
    fn send(&mut self, msg: T);
    async fn receive(&mut self) -> T;

    async fn receive_with_timeout(&mut self, timeout: Duration) -> Option<T>;

    fn try_receive(&mut self) -> Option<T>;

    fn sender(&self) -> UnboundedSender<T>;
}

#[async_trait::async_trait]
impl<T: std::marker::Send + Debug> AsyncChannel<T> for AsyncGateway<T> {
    fn send(&mut self, msg: T) {
        self.tx.send(msg).expect("channel closed unexpectedly");
    }
    async fn receive(&mut self) -> T {
        return self.rx.recv().await.expect("channel closed prematurely");
    }
    async fn receive_with_timeout(&mut self, timeout: Duration) -> Option<T> {

        let x = tokio::time::timeout(timeout, self.receive()).await.ok();


        return x;
    }
    fn try_receive(&mut self) -> Option<T>{
        self.rx.try_recv().ok()
    }
    fn sender(&self) -> UnboundedSender<T>{
        self.tx.clone()
    }
}

pub struct DeadExternalBus {}
#[async_trait::async_trait]
impl<T: std::marker::Send + Debug> AsyncChannel<T> for DeadExternalBus {
    async fn receive(&mut self) -> T {
        return std::future::pending().await;
    }

    fn send(&mut self, _msg: T) {
        // no-op
    }
    async fn receive_with_timeout(&mut self, _timeout: Duration) -> Option<T> {
        None
    }
    fn try_receive(&mut self) -> Option<T>{
        return None;
    }
    fn sender(&self) -> tokio::sync::mpsc::UnboundedSender<T> {
        todo!();
    }
}
