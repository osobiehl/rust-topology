use crate::communication::{ ExternalBus};
#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Copy, Debug)]
pub struct BasicTransmitter ();

#[derive(Clone, Debug)]
pub enum ModuleNeighborInfo {
    Basic,                                                // basic transmitter, is end of leaf
    Hub(HubIndex),                                        // hub
    Advanced(Option<HubIndex>, Option<BasicTransmitter>), // advanced: can be root, middle component, or leaf
}

pub mod transmitters {
    use crate::communication::IdentityResolver;

    pub use super::*;
    pub struct P4Basic {
        parent: Box<dyn ExternalBus>,
    }

    impl P4Basic {
        pub fn new(parent: Box<dyn ExternalBus>) -> Self {
            Self { parent }
        }
    }

    impl IdentityResolver for P4Basic{
        fn discover_identity(&mut self) {
            self.parent.send_identity(ModuleNeighborInfo::Basic);
            let parent_id = self.parent.receive_neighbor_identity();
            println!("identity for basic {:?}",parent_id );

        }
    }

    pub struct P4Advanced {
        parent: Box<dyn ExternalBus>,
        child: Box<dyn ExternalBus>,
    }
    impl P4Advanced {
        pub fn new(parent: Box<dyn ExternalBus>, child: Box<dyn ExternalBus>) -> Self {
            Self { parent, child }
        }
    }

    use ModuleNeighborInfo::*;
    impl IdentityResolver for P4Advanced{
        fn discover_identity(&mut self) {
            let child = self.child.receive_neighbor_identity();
            let parent = self.parent.receive_neighbor_identity();
            let state = match (parent, child)
            {
                (None, None) => ModuleNeighborInfo::Advanced(None, None),
                (Some(Hub(i)), None) => Advanced(Some(i), None),
                (None, Some(ModuleNeighborInfo::Basic)) => Advanced(None, Some(BasicTransmitter())),
                (Some(Hub(i)), Some(Basic)) => Advanced(Some(i), Some(BasicTransmitter())),
                (_,_) => panic!("unknown configuration!"),
            };

            println!("state of advanced: {:?}", &state);
            self.child.send_identity( state.clone());
            self.parent.send_identity(state);
        }

    }

    const HUB_NUM_CHILDREN: usize = 4;

    pub struct P4Hub {
        children: [Option<Box<dyn ExternalBus>>; HUB_NUM_CHILDREN],
    }
    impl P4Hub {
        pub fn new(children: [Option<Box<dyn ExternalBus>>; HUB_NUM_CHILDREN]) -> Self {
            Self { children }
        }
    }

    impl IdentityResolver for P4Hub {
        fn discover_identity(&mut self) {
            for elem in 0..HUB_NUM_CHILDREN{
                let idx: HubIndex = elem.try_into().unwrap();
                if let Some(bus) = &mut self.children[elem] {
                    bus.send_identity(Hub(idx));
                    let neighbor_id = bus.receive_neighbor_identity();
                    println!("neighbor identity for channel {:?}: {:?}", idx, neighbor_id.unwrap() );
                }
            }
        }
    }



}

