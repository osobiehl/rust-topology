use smoltcp::wire::{Ipv4Address};
// class B subnet borrowing 4 bits ??
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum HubIndex {
    One = 0,
    Two = 1,
    Three = 2,
    Four = 3,
}

impl HubIndex {
    pub fn to_ip_octet(&self) -> u8 {
        match self {
            HubIndex::One => 192,
            HubIndex::Two => 193,
            HubIndex::Three => 194,
            HubIndex::Four => 195,
        }
    }
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

impl ModuleNeighborInfo{
    pub fn into_vec(self)->Vec<u8>{
        self.into()
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum Sysmodule {
    COM = 1,
    PI,
    PV,
    HMI,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Transmitter {
    Basic,
    Advanced,
    Hub(HubIndex),
}

pub fn determine_ip(
    sysmodule: &Sysmodule,
    transmitter: &Transmitter,
    neighbor_info: &ModuleNeighborInfo,
) -> Ipv4Address {
    let sysmodule_octet = *sysmodule as u8;
    match (transmitter, neighbor_info) {
        (Transmitter::Basic, ModuleNeighborInfo::Advanced(None, _x)) => {
            Ipv4Address::new(HubIndex::Two.to_ip_octet(), 168, 0, sysmodule_octet)
        }
        (Transmitter::Basic, ModuleNeighborInfo::Advanced(Some(idx), _)) => {
            Ipv4Address::new(idx.to_ip_octet(), 168, 0, sysmodule_octet)
        }
        (Transmitter::Basic, ModuleNeighborInfo::Hub(idx)) => {
            Ipv4Address::new(idx.to_ip_octet(), 168, 0, sysmodule_octet)
        }

        //cases where module is alone
        (Transmitter::Advanced, ModuleNeighborInfo::Advanced(None, None))
        | (Transmitter::Advanced, ModuleNeighborInfo::NoNeighbor)
        | (Transmitter::Basic, ModuleNeighborInfo::NoNeighbor) => {
            Ipv4Address::new(HubIndex::One.to_ip_octet(), 168, 0, sysmodule_octet)
        }

        // case advanced + basic: advanced is channel one
        (Transmitter::Advanced, ModuleNeighborInfo::Advanced(None, Some(_trans))) => {
            Ipv4Address::new(HubIndex::One.to_ip_octet(), 168, 1, sysmodule_octet)
        }
        (Transmitter::Advanced, ModuleNeighborInfo::Advanced(Some(idx), Some(_trans)))
            if *sysmodule == Sysmodule::HMI =>
        {
            Ipv4Address::new(idx.to_ip_octet(), 168, 1, sysmodule_octet)
        }
        (Transmitter::Advanced, ModuleNeighborInfo::Advanced(Some(idx), Some(_trans))) => {
            Ipv4Address::new(idx.to_ip_octet(), 168, 1, sysmodule_octet)
        }

        (Transmitter::Hub(_idx), ModuleNeighborInfo::NoNeighbor) => panic!("empty hub"),
        // hub has identifier 3 so it is easy to route
        (Transmitter::Hub(idx), _other) => {
            Ipv4Address::new(idx.to_ip_octet(), 168, 3, sysmodule_octet)

        }
        (_, _) => panic!("setup not defined")
    }
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

fn parse_discovery_message(m: Vec<u8>) -> Result<ModuleNeighborInfo, ()> {
    return serde_json::from_slice(&m).map_err(|_| ());
}
