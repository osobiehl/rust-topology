use bichannel::Channel;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use std::fmt::Debug;
use std::net::{Ipv4Addr};
use std::future::Future;

pub type IPMessage = ( Ipv4Addr, String);
type SysmoduleRPC = Box<dyn Fn(&mut dyn Sysmodule) -> Box<dyn Future<Output=()>> + Send>;

pub enum ChannelEvent{
    DeadChannel,
    RunProcedure( SysmoduleRPC ), // run a specific test command
    Message( IPMessage )
}

impl Debug for ChannelEvent{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x = match self {

            DeadChannel => f.write_str("DeadChannel"),
            Self::RunProcedure(_) =>  f.write_str("RunProcedure"),
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
// a sysmodule can either receive data on a channel, or it can also be prompted to run an algorithm
pub trait Sysmodule{
    // testing method to make sure correct things happen on receive
    fn on_next_receive( &mut self, when_received: Box<dyn FnOnce( &mut dyn Sysmodule)> );
    // send a message, the Sysmodule determines exactly how this is done
    fn send(&mut self, message: IPMessage);
}
