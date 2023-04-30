use crate::async_communication::{ AsyncExternalBus, ChannelEvent::{DeadChannel, Message, RunProcedure}};
use std::net::{Ipv4Addr};
use ipnet::{Ipv4AddrRange, Ipv4Net, Ipv4Subnets};
use async_trait::async_trait;
// class B subnet borrowing 4 bits ??
use serde::{Serialize, Deserialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum HubIndex {
    One = 0,
    Two = 1,
    Three = 2,
    Four = 3,
}

impl TryInto<HubIndex> for usize{
    type Error = ();
fn try_into(self) -> Result<HubIndex, Self::Error> {
    match self {
        0 => Ok(HubIndex::One),
        1 => Ok(HubIndex::Two),
        2 => Ok(HubIndex::Three),
        3 => Ok(HubIndex::Four),
        _ => Err(())
    }
}
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BasicTransmitter();

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ModuleNeighborInfo {
    NoNeighbor,
    Basic,                                                // basic transmitter, is end of leaf
    Hub(HubIndex),                                        // hub
    Advanced(Option<HubIndex>, Option<BasicTransmitter>), // advanced: can be root, middle component, or leaf
}

pub mod transmitters {
    use crate::{communication::IdentityResolver, async_communication::{ChannelEvent, IPMessage}};

    pub use super::*;
    pub struct P4Basic {
        parent: AsyncExternalBus,
        address_mask: Ipv4Addr,
    }
    const DISCOVERY_ADDRESS: Ipv4Addr = Ipv4Addr::new(255,255,255,255);
    fn parse_discovery_message(m: IPMessage)-> Result<ModuleNeighborInfo, ()>{
        if m.0.cmp(&DISCOVERY_ADDRESS) == std::cmp::Ordering::Equal{
            let msg = serde_json::from_str::<ModuleNeighborInfo>(&m.1);
            return msg.map_err( |_| ());
        }
        return Err(());
        
    }

    impl TryInto<ModuleNeighborInfo> for ChannelEvent {
        type Error = ();
        fn try_into(self) -> Result<ModuleNeighborInfo, Self::Error> {
            return match self{
                DeadChannel => Ok(ModuleNeighborInfo::NoNeighbor),
                RunProcedure(_) => Err(()),
                Message(m) =>  parse_discovery_message(m)
            }
        }
    }

    impl Into<IPMessage> for ModuleNeighborInfo {
        fn into(self) -> IPMessage {
            return (
                Ipv4Addr::new(255,255,255,255),
                serde_json::to_string(&self).unwrap()
            );
        }
    }


    impl P4Basic {
        pub fn new(parent: AsyncExternalBus) -> Self {
            Self { parent, address_mask: Ipv4Addr::UNSPECIFIED }
        }

        pub fn handle_state(&mut self, state: ModuleNeighborInfo){

            println!("state from p4 basic: {:?}",state);
        }
    }
    #[async_trait]
    impl IdentityResolver for P4Basic{
        async fn discover_identity(&mut self) {
            self.parent.send( ModuleNeighborInfo::Basic.into());
            println!("sending p4 identity");
            let incoming_event = self.parent.receive().await;
            let state: ModuleNeighborInfo = incoming_event.try_into().expect("could not receive in P4 basic");
            self.handle_state(state);
        }
    }

    pub struct P4Advanced {
        parent: AsyncExternalBus,
        child: AsyncExternalBus,
    }
    impl P4Advanced {
        pub fn new(parent: AsyncExternalBus, child: AsyncExternalBus) -> Self {
            Self { parent, child }
        }
    }

    use ModuleNeighborInfo::*;

    #[async_trait]
    impl IdentityResolver for P4Advanced{
        async fn discover_identity(&mut self) {
            let child: ModuleNeighborInfo = self.child.receive().await.try_into().expect("child module did not send discovery");
            let parent:ModuleNeighborInfo = self.parent.receive().await.try_into().expect("parent module did not send discovery");
            let state = match (parent, child)
            {
                (NoNeighbor, NoNeighbor) => ModuleNeighborInfo::Advanced(None, None),
                (Hub(i), NoNeighbor) => Advanced(Some(i), None),
                (NoNeighbor, Basic) => Advanced(None, Some(BasicTransmitter())),
                (Hub(i), Basic ) => Advanced(Some(i), Some(BasicTransmitter())),
                (_,_) => panic!("unknown configuration!"),
            };

            println!("state of advanced: {:?}", &state);
            self.child.send( state.clone().into());
            self.parent.send(state.clone().into());
        }

    }

    const HUB_NUM_CHILDREN: usize = 4;

    pub struct P4Hub {
        children: [Option<AsyncExternalBus>; HUB_NUM_CHILDREN],
    }
    impl P4Hub {
        pub fn new(children: [Option<AsyncExternalBus>; HUB_NUM_CHILDREN]) -> Self {
            Self { children }
        }
    }
    #[async_trait]
    impl IdentityResolver for P4Hub {
        async fn discover_identity(&mut self) {
            for elem in 0..HUB_NUM_CHILDREN{
                let idx: HubIndex = elem.try_into().unwrap();
                if let Some(bus) = &mut self.children[elem] {
                    bus.send(Hub(idx).into());
                    let neighbor_id: ModuleNeighborInfo = bus.receive().await.try_into().expect("could not figure out identity");
                    println!("neighbor identity for channel {:?}: {:?}", idx, neighbor_id );
                }
            }
        }
    }



}
