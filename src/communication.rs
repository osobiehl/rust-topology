use std::sync::{Arc};
use crate::sysmodule::{HubIndex, ModuleNeighborInfo};
use bichannel::{channel, Channel};
use async_trait::async_trait;
pub trait ExternalBus: Send{
    // todo await identity
    fn receive_neighbor_identity(&mut self) -> Option<ModuleNeighborInfo>;
    fn send_identity(&mut self, id: ModuleNeighborInfo);


}
type IBChannel = Channel<ModuleNeighborInfo,ModuleNeighborInfo>;
pub struct BlockingExternalBus{
    line: Option<IBChannel>,
}

impl BlockingExternalBus{

    pub fn new() -> (Self, Self)
    {
        let (right, left) = channel();
        return (
            Self{ line: Some(right)},
            Self { line: Some( left) }
        )
    }

    pub fn new_empty() -> Self {
        return Self { line: None};
    }

}


impl ExternalBus for BlockingExternalBus{
    fn receive_neighbor_identity(&mut self) -> Option<ModuleNeighborInfo> {
        if self.line.is_none()
        {
            return None
        }
        else {
            {
                return Some(self.line.as_ref().unwrap().recv().expect("channel error"));
            }
        }
    }
    fn send_identity(&mut self, id: ModuleNeighborInfo) {
        self.line.as_ref().expect("sending identity to non-existing node!").send(id).unwrap();
    }
}

#[async_trait]
pub trait IdentityResolver{
     async fn discover_identity(&mut self);
}
