use std::time::Duration;

use super::common::{BasicModule, SysModuleStartup, TRANSIENT_GATEWAY_ID, Device};
use crate::async_communication::AsyncGateway;
use crate::net::device::AsyncGatewayDevice;
use crate::net::udp_state::{NetStack, UDP, IPEndpoint, AsyncSocketHandle, RawDirection, UDPState};
use crate::sysmodule::ModuleNeighborInfo::{Advanced, Basic, Hub, NoNeighbor};
use crate::sysmodule::{BasicTransmitter, HubIndex, ModuleNeighborInfo, determine_ip, Sysmodule, Transmitter};
use crate::sysmodules::common::{TRANSIENT_PI_ID, TRANSIENT_PV_ID, TRANSIENT_HMI_ID, ADDRESS_ASSIGNMENT_PORT};
use futures::future;
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr, IpEndpoint, Ipv4Address, Ipv6Address, Ipv4Cidr, Ipv4Packet, Ipv4Repr};
use crate::net::udp_state::AsyncSocket;
use log::{error, info};
use std::sync::Arc;
use tokio::sync::Mutex;


#[derive(Clone,Copy,Debug, PartialEq)]
pub enum Direction{
    Downstream,
    Upstream,
}



#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ComType {
    HubCom(HubIndex),
    AdvUpstream,
    AdvDownstream,
    Basic,
}

impl Into<Transmitter> for ComType{
    fn into(self) -> Transmitter {
        match self {
            Self::HubCom(idx) => Transmitter::Hub(idx),
            Self::AdvUpstream => Transmitter::Advanced,
            Self::AdvDownstream => Transmitter::Advanced,
            Self::Basic => Transmitter::Basic
        }
    }
}

const EXTERNAL_BUS_TRANSIENT_ADDRESS: IpAddress = IpAddress::v4(192, 168, 70, 1);
const EXTERNAL_BUS_TRANSIENT_PORT: u16 = 6969;
const INTERNAL_BUS_TRANSIENT_COM_PORT: u16 = 6968;

impl ComType {
    pub fn external_bus_ip(&self) -> IpCidr {
        return IpCidr::new(EXTERNAL_BUS_TRANSIENT_ADDRESS, 24);
    }
}

pub struct Com {

    netif: Arc<Mutex<UDPState<Device>>>,
    initial_configuration: ComType,
    assigned_ip: Ipv4Address,
    redirect_socket: Option<AsyncSocketHandle<AsyncGatewayDevice<AsyncGateway<Vec<u8>>>, RawDirection>>
}
const WAIT_DEFAULT: Duration = Duration::from_millis(3);
impl Com {
    pub fn new(base: Arc<Mutex<UDPState<Device>>>, initial_configuration: ComType) -> Self {
        Self {
            netif: base,
            initial_configuration,
            assigned_ip: Ipv4Address::UNSPECIFIED,
            redirect_socket: None
        }
    }

    async fn configure_hub(&mut self, _i: HubIndex) {
        todo!();
        // self.external_bus.send(ModuleNeighborInfo::Hub(i).into());
        // let neighbor_id: ModuleNeighborInfo = self
        //     .external_bus
        //     .receive()
        //     .await
        //     .try_into()
        //     .expect("could not figure out identity");
        // println!("neighbor from hub: {:?}: {:?}", i, neighbor_id);
    }

    async fn configure_upstream(&mut self) {
        let mut parent_socket  = self.netif.socket(EXTERNAL_BUS_TRANSIENT_PORT).await;
        let mut child_socket = self.netif.socket(INTERNAL_BUS_TRANSIENT_COM_PORT).await;

        let parent_res: Result<(Vec<u8>, IpEndpoint), ()> =
            parent_socket.receive_with_timeout(WAIT_DEFAULT).await;
        let parent: ModuleNeighborInfo = parent_res
            .map(|(v, _)| v.try_into().expect("did not receive neighbor info!"))
            .unwrap_or(ModuleNeighborInfo::NoNeighbor);

        let child_res = child_socket.receive_with_timeout(WAIT_DEFAULT).await;

        let child = child_res
            .map(|(addr, _from)| {
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
        let v: Vec<u8> = state.clone().into();

        parent_socket
            .send(
                &v,
                IPEndpoint {
                    addr: EXTERNAL_BUS_TRANSIENT_ADDRESS,
                    port: EXTERNAL_BUS_TRANSIENT_PORT,
                },
            )
            .await;
        child_socket
            .send(
                &v,
                IPEndpoint {
                    addr: TRANSIENT_GATEWAY_ID,
                    port: INTERNAL_BUS_TRANSIENT_COM_PORT,
                },
            )
            .await;
        self.assign_ips(state).await;
 
    }

    async fn configure_downstream(&mut self) {
        let mut socket_external_bus: crate::net::udp_state::AsyncSocketHandle<
            crate::net::device::AsyncGatewayDevice<
                crate::async_communication::AsyncGateway<Vec<u8>>,
               
            >,
            UDP> = self.netif.socket(EXTERNAL_BUS_TRANSIENT_PORT).await;
        let mut socket_internal_bus = self.netif.socket(INTERNAL_BUS_TRANSIENT_COM_PORT).await;

        let child = socket_external_bus.receive_with_timeout(WAIT_DEFAULT).await;
        println!("received from child! ");
        match child {
            Err(_) => {
                println!("warn: COM module with no child, todo deactivate")
            }
            Ok(_p) => {
                let data: Vec<u8> = ModuleNeighborInfo::Basic.into();
                socket_internal_bus
                    .send(
                        &data,
                        IPEndpoint {
                            addr: TRANSIENT_GATEWAY_ID,
                            port: INTERNAL_BUS_TRANSIENT_COM_PORT,
                        },
                    )
                    .await;
                let state: ModuleNeighborInfo = socket_internal_bus
                    .receive_with_timeout(Duration::from_millis(2))
                    .await
                    .map_or(
                        ModuleNeighborInfo::Advanced(None, Some(BasicTransmitter())),
                        |(v, _from)| v.try_into().expect("did not receivemodule neighbor info"),
                    );
                println!("sending downstream info to child!");
                socket_external_bus
                    .send(
                        & state.clone().into_vec(),
                        IPEndpoint {
                            addr: EXTERNAL_BUS_TRANSIENT_ADDRESS,
                            port: EXTERNAL_BUS_TRANSIENT_PORT,
                        },
                    )
                    .await;

                if let ModuleNeighborInfo::Advanced(None, _ ) = state.clone(){
                    println!("state from pov of advanced downstream: {:?}", &state);

                    self.assign_ips(state).await;
                }
            }
        }

    }

    async fn configure_basic(&mut self) {
        println!("configuring basic...");
        let mut socket_parent = self.netif.socket(EXTERNAL_BUS_TRANSIENT_PORT).await;
        socket_parent
            .send(
                &ModuleNeighborInfo::Basic.into_vec(),
                IPEndpoint {
                    addr: EXTERNAL_BUS_TRANSIENT_ADDRESS,
                    port: EXTERNAL_BUS_TRANSIENT_PORT,
                },
            )
            .await;
        // println!("sending p4 identity");
        let parent = socket_parent.receive_with_timeout(WAIT_DEFAULT).await;

        let mut parent_info: ModuleNeighborInfo = ModuleNeighborInfo::NoNeighbor;
        if let Ok((val, endpoint)) = parent {
            parent_info = val
                .try_into()
                .expect("did not receive module info on external bus");
        }
        // println!("state from pov of basic: {:?}", &parent_info);
        self.assign_ips(parent_info).await;
        
    }


    async fn assign_ips(&mut self, module_info: ModuleNeighborInfo){
        let transmitter: Transmitter = self.initial_configuration.clone().into();
        const SYSMODULES: [(IpAddress,  Sysmodule); 3] = [
            (TRANSIENT_PI_ID, Sysmodule::PI),
            (TRANSIENT_PV_ID, Sysmodule::PV),
            (TRANSIENT_HMI_ID, Sysmodule::HMI)
        ];

        for (addr, module) in SYSMODULES {
            let new_ip = determine_ip(&module, &transmitter, &module_info);
            let mut s = self.netif.socket( ADDRESS_ASSIGNMENT_PORT).await;
            
            s.send(&Vec::from(new_ip.0), IPEndpoint {addr, port: ADDRESS_ASSIGNMENT_PORT }).await;
            
        }
        self.assigned_ip = determine_ip(&Sysmodule::COM, &self.initial_configuration.clone().into(), &module_info);

    }


    fn device_index(address: &Ipv4Address)->u8{
        return address.0[2];
    }

    pub const BASIC_INDEX: u8 = 0;

    pub fn determine_direction_basic(sender_index: u8, destination_index: u8) -> Option<Direction>{
        if sender_index == destination_index{
            return None
        }
        else if destination_index == Self::BASIC_INDEX{
            return Some(Direction::Downstream)
        }
        else {
            return Some(Direction::Upstream)
        }
    }
    pub const ADVANCED_INDEX: u8 = 1;
    pub fn determine_direction_advanced_downstream(sender_index: u8, destination_index: u8) -> Option<Direction>{

        if sender_index == destination_index{
            return None
        }
        // if message comes from above, we do nothing
        else if destination_index < Self::ADVANCED_INDEX{
            return Some(Direction::Downstream)
        }
        else if destination_index >= Self::ADVANCED_INDEX &&  sender_index < Self::ADVANCED_INDEX {
            return Some(Direction::Upstream)
        }
        else {
            return None
        }

    }
    pub const HUB_INDEX: u8 = 2;
    pub fn determine_direction_advanced_upstream(sender_index: u8, destination_index: u8) -> Option<Direction>{

        if sender_index == destination_index{
            return None
        }
        // if message comes from above, we do nothing
        else if destination_index > Self::ADVANCED_INDEX{
            return Some(Direction::Upstream)
        }
        else if destination_index <= Self::ADVANCED_INDEX &&  sender_index > Self::ADVANCED_INDEX {
            return Some(Direction::Downstream)
        }
        else {
            return None
        }

    }

    pub fn determine_direction_hub(sender_index: u8, destination_index: u8) -> Option<Direction>{

        if sender_index == destination_index{
            return None
        }
        else if destination_index == Self::HUB_INDEX{
            return Some(Direction::Upstream)
        }
        else {
            return Some(Direction::Downstream)
        }

    }

    pub fn determine_direction(configuration: ComType, sender: &Ipv4Address, destination: &Ipv4Address) -> Option<Direction>{

        let sender_index = Self::device_index(sender);     
        let destination_index = Self::device_index(destination);

        let config = match configuration{
            ComType::AdvDownstream => Self::determine_direction_advanced_downstream(sender_index, destination_index),
            ComType::AdvUpstream => Self::determine_direction_advanced_upstream(sender_index, destination_index),
            ComType::Basic => Self::determine_direction_basic(sender_index, destination_index),
            // TODO: this needs to also have ip index
            ComType::HubCom(_) => Self::determine_direction_hub(sender_index, destination_index)
        };
        println!("got direction for config {:?} : {:?}, sender: {}, dest: {}", configuration, config,  sender, destination);
        return config
    }

    fn direction_to_netidx(configuration: ComType, direction: Direction)-> usize{
        match configuration{
            ComType::AdvDownstream | ComType::HubCom(_) =>  if direction as usize == 0 {1} else {0},
            _ => direction as usize
        }
    }

    async fn run_routing(&mut self){
        let com_type = self.initial_configuration;
        self.netif.modify_netif(|n| {
            info!("intial config: {:?}, netifs: {:?}",&self.initial_configuration, n.netifs.len());
        }).await;
        let socket = self.redirect_socket.as_mut().unwrap();
        let incoming = socket.recv().await.expect("nothing received from socket!");
        let ip_addr = Ipv4Packet::new_checked(&incoming);
        let Ok(x) = ip_addr else {
            error!("receive malformed IP packet on raw socket");
            return;
        };

        info!("initial config: {:?}, recv: {}", &self.initial_configuration, &x);

        if let Some(dir) = Self::determine_direction(com_type, & x.src_addr(), &x.dst_addr()) {
            socket.send(&incoming, Self::direction_to_netidx(com_type, dir)).await;
        };
        

        
    }


}

#[async_trait::async_trait]
impl SysModuleStartup for Com {
    async fn run_once(&mut self) {
        // tokio::time::sleep(Duration::from_millis(1000)).await;
        self.run_routing().await;
    }
    async fn on_start(&mut self) {
        match self.initial_configuration {
            ComType::HubCom(i) => self.configure_hub(i).await,
            ComType::AdvUpstream => self.configure_upstream().await,
            ComType::AdvDownstream => self.configure_downstream().await,
            ComType::Basic => self.configure_basic().await,
        }
        self.redirect_socket = Some( self.netif.raw_direction_socket().await);

        // no-op for now
    }
}
