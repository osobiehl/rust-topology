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


pub struct AsyncExternalBus{
    tx: UnboundedSender<ChannelEvent>,
    rx: UnboundedReceiver<ChannelEvent>,
    is_dead: bool,
}


impl AsyncExternalBus{
    pub fn send(&mut self, msg: IPMessage){
        if !self.is_dead{
            self.tx.send(ChannelEvent::Message(msg)).expect("channel closed unexpectedly");
        }
        
    }
    pub async fn receive(&mut self) -> ChannelEvent{
        if self.is_dead
        {
            return ChannelEvent::DeadChannel
        }
        return self.rx.recv().await.expect("channel closed prematurely");
    }

    pub fn new() -> (AsyncExternalBus, AsyncExternalBus){
        let (a_send,b_rx) = unbounded_channel();
        let (c_send, d_rx) = unbounded_channel();
        return (
            Self{
                tx: (a_send), rx: (d_rx), is_dead: false
            },
            Self{
                tx: (c_send), rx: (b_rx), is_dead: false
            }
        )
    }

    pub fn new_empty() -> Self {
        let (tx, rx ) = unbounded_channel();
        return Self{ tx: tx, rx: rx, is_dead: true}
    }

}

// a sysmodule can either receive data on a channel, or it can also be prompted to run an algorithm
pub trait Sysmodule{
    // testing method to make sure correct things happen on receive
    fn on_next_receive( &mut self, when_received: Box<dyn FnOnce( &mut dyn Sysmodule)> );
}
