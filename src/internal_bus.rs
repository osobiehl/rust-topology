use crate::async_communication::{AsyncGateway, Sysmodule, IPMessage};
use crate::async_communication::AsyncChannel;
use futures::future::select_all;
use crate::async_communication::ChannelEvent::Message;

pub struct InternalBus{
    subscribers: Vec<AsyncGateway<IPMessage>>,
}

impl InternalBus{
    pub fn subscribe(&mut self, gateway: AsyncGateway<IPMessage>){
        self.subscribers.push(gateway);
    }

    pub async fn run_once(&mut self){
        let futures = self.subscribers.iter_mut().map( |x|Box::pin(x.receive() ));
        let (event, index, remaining) = select_all(futures).await;
        drop(remaining);
        let l = self.subscribers.len();
        for i in 0..l{
            if i != index
            {
                self.subscribers[i].send(event.clone());
            }
        }
    }
}