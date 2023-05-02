use std::net::Ipv4Addr;
use std::time::Duration;

use super::common::{BasicModule, SysModule, SysModuleStartup};
use crate::async_communication::{AsyncChannel, IPMessage};
use crate::sysmodule::{
    BasicTransmitter, HubIndex,
    ModuleNeighborInfo::{self, Advanced, Basic, Hub, NoNeighbor},
};
use tokio::select;

#[derive(Clone, Copy, Debug)]
pub enum ComType {
    HubCom(HubIndex),
    AdvUpstream,
    AdvDownstream,
    Basic,
}

// impl Into<u8> for ComType{
//     fn into(self) -> u8 {
//          match self{
//             ComType::HubCom(i) => i as u8,
//             Self::AdvUpstream => 1,
//             Self::AdvDownstream => 2,
//             Self::Basic => 3,
//          }
//     }
// }
// const HUB_IDENTIFIER: u8 = 2;
// const ADV_IDENTIFIER: u8 = 1;
// const BASIC_IDENTIFIER: u8 = 0;
// impl Into<Ipv4Addr> for ComType{
//     fn into(self) -> Ipv4Addr {

//     }
// }

pub struct Com {
    base: BasicModule,
    external_bus: Box<dyn AsyncChannel<IPMessage>>,
    initial_configuration: ComType,
    is_external_dead: bool,
}
const WAIT_DEFAULT: Duration = Duration::from_millis(5);
impl Com {
    pub fn new(
        external_bus: Box<dyn AsyncChannel<IPMessage>>,
        base: BasicModule,
        initial_configuration: ComType,
    ) -> Self {
        Self {
            external_bus,
            base,
            initial_configuration,
            is_external_dead: false,
        }
    }

    async fn configure_hub(&mut self, i: HubIndex) {
        self.external_bus.send(ModuleNeighborInfo::Hub(i).into());
        let neighbor_id: ModuleNeighborInfo = self
            .external_bus
            .receive()
            .await
            .try_into()
            .expect("could not figure out identity");
        println!("neighbor from hub: {:?}: {:?}", i, neighbor_id);
    }

    async fn configure_upstream(&mut self) {
        let parent = self.external_bus.try_receive(WAIT_DEFAULT).await;
        let parent = parent
            .map(|addr: (Ipv4Addr, String)| {
                addr.try_into()
                    .expect("did not receive module neighbor info!")
            })
            .unwrap_or(ModuleNeighborInfo::NoNeighbor);

        let child = self.base.internal_bus.try_receive(WAIT_DEFAULT).await;

        let child = child
            .map(|addr| {
                addr.try_into()
                    .expect("did not receive module neighbor info!")
            })
            .unwrap_or(ModuleNeighborInfo::NoNeighbor);

        let state = match (parent, child) {
            (NoNeighbor, NoNeighbor) => ModuleNeighborInfo::Advanced(None, None),
            (Hub(i), NoNeighbor) => Advanced(Some(i), None),
            (NoNeighbor, Basic) => Advanced(None, Some(BasicTransmitter())),
            (Hub(i), Basic) => Advanced(Some(i), Some(BasicTransmitter())),
            (_, _) => panic!("unknown configuration!"),
        };

        println!("state of advanced: {:?}", &state);
        self.base.internal_bus.send(state.clone().into());
        self.external_bus.send(state.clone().into());
    }

    async fn configure_downstream(&mut self) {
        let child = self.external_bus.try_receive(WAIT_DEFAULT).await;
        match child {
            None => self
                .base
                .internal_bus
                .send(ModuleNeighborInfo::NoNeighbor.into()),
            Some(_) => self
                .base
                .internal_bus
                .send(ModuleNeighborInfo::Basic.into()),
        };
        let state = self.base.internal_bus.receive().await;
        println!("sending state downstream ...");
        self.external_bus.send(state);
    }

    async fn configure_basic(&mut self) {
        self.external_bus.send(ModuleNeighborInfo::Basic.into());
        println!("sending p4 identity");
        let parent = self.external_bus.try_receive(WAIT_DEFAULT).await;
        let parent = parent
            .map(|addr: (Ipv4Addr, String)| {
                addr.try_into()
                    .expect("did not receive module neighbor info!")
            })
            .unwrap_or(ModuleNeighborInfo::NoNeighbor);
        println!("state from pov of basic: {:?}", &parent);
    }
}

#[async_trait::async_trait]
impl SysModule for Com {
    async fn receive(&mut self) -> IPMessage {
        if !self.is_external_dead {
            return select! {
               ib_msg =  self.base.receive() => ib_msg,
               ext_msg =  self.external_bus.receive() => ext_msg
            };
        } else {
            return self.base.receive().await;
        }
    }

    fn send(&mut self, msg: IPMessage) {
        //
        todo!("decide routing for sysmodule")
        // no-op
    }
}

#[async_trait::async_trait]
impl SysModuleStartup for Com {
    async fn run_once(&mut self) {
        loop {
            let msg = self.base.internal_bus.receive().await;
            println!("COM module: recv {:?}", msg);
        }
    }
    async fn on_start(&mut self) {
        match self.initial_configuration {
            ComType::HubCom(i) => self.configure_hub(i).await,
            ComType::AdvUpstream => self.configure_upstream().await,
            ComType::AdvDownstream => self.configure_downstream().await,
            ComType::Basic => self.configure_basic().await,
        }

        // no-op for now
    }
}