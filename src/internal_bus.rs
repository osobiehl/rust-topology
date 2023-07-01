use crate::async_communication::AsyncChannel;
use crate::async_communication::{AsyncGateway};
use futures::future::select_all;
use log::{trace, info};

pub struct InternalBus {
    subscribers: Vec<AsyncGateway<Vec<u8>>>,
}

impl InternalBus {
    pub fn new() -> Self {
        Self {
            subscribers: vec![],
        }
    }
    pub fn subscribe(&mut self, gateway: AsyncGateway<Vec<u8>>) {
        self.subscribers.push(gateway);
    }

    pub async fn run_once(&mut self) {
        let futures = self.subscribers.iter_mut().map(|x| Box::pin(x.receive()));
        let (event, index, remaining) = select_all(futures).await;
        drop(remaining);


        // println!("internal bus: received: {:?}", &event);
       

        let l = self.subscribers.len();

        for i in 0..l {
            if i != index {
                self.subscribers[i].send(event.clone());
            }
        }

        
    }
}
