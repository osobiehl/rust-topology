use bichannel::Channel;
use futures::{select, FutureExt};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::time;
use std::fmt::Debug;
use std::net::{Ipv4Addr};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use futures::future::BoxFuture;

use crate::sysmodules::common::SysModule;

pub type IPMessage = ( Ipv4Addr, String);
pub type SysmoduleRPC = Box<dyn FnOnce(&mut dyn SysModule) -> BoxFuture<()> + Send>;

pub enum ChannelEvent{
    DeadChannel,
    Message( IPMessage )
}

impl Debug for ChannelEvent{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x = match self {

            DeadChannel => f.write_str("DeadChannel"),
            Self::Message(msg) => f.write_fmt(format_args!("IP: {}, message: {}",msg.0, msg.1 ))
        };
        return std::fmt::Result::Ok(());
    }
}
pub struct AsyncGateway<T>{
    tx: UnboundedSender<T>,
    rx: UnboundedReceiver<T>,
}

impl<T>  AsyncGateway<T> {
    pub fn new() -> (Self, Self){
        let (a_send,b_rx) = unbounded_channel();
        let (c_send, d_rx) = unbounded_channel();
        return (
            Self{
                tx: (a_send), rx: (d_rx)
            },
            Self{
                tx: (c_send), rx: (b_rx),
            }
        )
    }
}


#[async_trait::async_trait]
pub trait AsyncChannel<T>: Send{
    fn send(&mut self, msg: T);
    async fn receive(&mut self) ->T;

    async fn try_receive(&mut self, timeout: Duration)->Option<T>
    {
        let ans = select! {
            x = self.receive().fuse() => Some(x),
            _ = tokio::time::sleep(timeout).fuse() => None
        };
        return ans;
    }


}
#[async_trait::async_trait]
impl<T: std::marker::Send + Debug> AsyncChannel<T> for AsyncGateway<T>{
    fn send(&mut self, msg: T){
            self.tx.send(msg).expect("channel closed unexpectedly");
        
    }
    async fn receive(&mut self) -> T {
        return self.rx.recv().await.expect("channel closed prematurely");
    }



}

pub struct DeadExternalBus{}
#[async_trait::async_trait]
impl AsyncChannel<ChannelEvent> for DeadExternalBus{
    async fn receive(&mut self)-> ChannelEvent{
        return ChannelEvent::DeadChannel;
    }

    fn send(&mut self, _msg: ChannelEvent) {
        // no-op
    }
}

