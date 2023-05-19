

use std::net::Ipv4Addr;
// class B subnet borrowing 4 bits ??
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum HubIndex {
    One = 0,
    Two = 1,
    Three = 2,
    Four = 3,
}

impl TryInto<HubIndex> for usize {
    type Error = ();
    fn try_into(self) -> Result<HubIndex, Self::Error> {
        match self {
            0 => Ok(HubIndex::One),
            1 => Ok(HubIndex::Two),
            2 => Ok(HubIndex::Three),
            3 => Ok(HubIndex::Four),
            _ => Err(()),
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

impl TryInto<ModuleNeighborInfo> for Vec<u8> {
    type Error = ();
    fn try_into(self) -> Result<ModuleNeighborInfo, Self::Error> {
        // println!("message: {:?}", &self);
        return parse_discovery_message(self);
    }
}

impl Into<Vec<u8>> for ModuleNeighborInfo {
    fn into(self) -> Vec<u8> {
        return serde_json::to_vec(&self).unwrap();
    }
}
const DISCOVERY_ADDRESS: Ipv4Addr = Ipv4Addr::new(255, 255, 255, 255);
fn parse_discovery_message(m: Vec<u8>) -> Result<ModuleNeighborInfo, ()> {
    return serde_json::from_slice(&m).map_err(|_| () )
}


