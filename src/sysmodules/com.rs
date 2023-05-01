use super::common::{BasicModule, SysModule, SysModuleStartup};
use crate::async_communication::{ChannelEvent,AsyncChannel};
use crate::sysmodule::HubIndex;

pub enum ComType{
    HubCom(HubIndex),
    AdvUpstream,
    AdvDownstream,
    Basic
}

pub struct Com{
    base: BasicModule,
    external_bus: Box<dyn AsyncChannel<ChannelEvent>>,
    initial_configuration: ComType
}

impl Com {
    pub fn new( external_bus: Box<dyn AsyncChannel<ChannelEvent>>, base: BasicModule, initial_configuration: ComType) -> Self
    {
        Self{ external_bus, base, initial_configuration}
    }
}

#[async_trait::async_trait]
impl SysModuleStartup for Com{
    async fn run_once(&mut self){
        loop{
            let msg = self.base.internal_bus.receive().await;
            println!("COM module: recv {:?}", msg);
        }      
    }
    async fn on_start(&mut self) {
        // no-op for now
    }
}