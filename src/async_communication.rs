
use futures::future::BoxFuture;
use futures::{select, FutureExt};
use std::fmt::Debug;

use std::net::Ipv4Addr;

use std::time::Duration;

use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};


use crate::sysmodules::common::SysModule;

pub type IPMessage = (Ipv4Addr, String);
pub type SysmoduleRPC = Box<dyn FnOnce(&mut dyn SysModule) -> BoxFuture<()> + Send>;



pub struct AsyncGateway<T> {
    tx: UnboundedSender<T>,
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

#[async_trait::async_trait]
pub trait AsyncChannel<T>: Send {
    fn send(&mut self, msg: T);
    async fn receive(&mut self) -> T;

    async fn try_receive(&mut self, timeout: Duration) -> Option<T>;
}
#[async_trait::async_trait]
impl<T: std::marker::Send + Debug> AsyncChannel<T> for AsyncGateway<T> {
    fn send(&mut self, msg: T) {
        self.tx.send(msg).expect("channel closed unexpectedly");
    }
    async fn receive(&mut self) -> T {
        return self.rx.recv().await.expect("channel closed prematurely");
    }
    async fn try_receive(&mut self, timeout: Duration) -> Option<T> {
        let ans = select! {
            x = self.receive().fuse() => Some(x),
            _ = tokio::time::sleep(timeout).fuse() => None
        };
        return ans;
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
    async fn try_receive(&mut self, _timeout: Duration) -> Option<T> {
        None
    }
}
