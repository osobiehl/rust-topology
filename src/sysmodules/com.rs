use std::time::Duration;

use super::common::{BasicModule, SysModuleStartup, TRANSIENT_GATEWAY_ID};
use crate::net::udp_state::{NetStack, UDP, IPEndpoint};
use crate::sysmodule::ModuleNeighborInfo::{Advanced, Basic, Hub, NoNeighbor};
use crate::sysmodule::{BasicTransmitter, HubIndex, ModuleNeighborInfo, determine_ip, Sysmodule, Transmitter};
use crate::sysmodules::common::{TRANSIENT_PI_ID, TRANSIENT_PV_ID, TRANSIENT_HMI_ID, ADDRESS_ASSIGNMENT_PORT};
use futures::future;
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr, IpEndpoint, Ipv4Address, Ipv6Address};
use crate::net::udp_state::AsyncSocket;

#[derive(Clone, Copy, Debug)]
pub enum ComType {
    HubCom(HubIndex),
    AdvUpstream,
    AdvDownstream,
    Basic,
}

impl Into<Transmitter> for ComType{
    fn into(self) -> Transmitter {
        match self {
            Self::HubCom(_) => Transmitter::Hub,
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
    initial_configuration: ComType,
    is_external_dead: bool,
}
const WAIT_DEFAULT: Duration = Duration::from_millis(5);
impl Com {
    pub fn new(base: BasicModule, initial_configuration: ComType) -> Self {
        Self {
            base,
            initial_configuration,
            is_external_dead: false,
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
        let mut parent_socket  = self.base.socket(EXTERNAL_BUS_TRANSIENT_PORT).await;
        let mut child_socket = self.base.socket(INTERNAL_BUS_TRANSIENT_COM_PORT).await;

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
        // let parent = self.external_bus.receive_with_timeout(WAIT_DEFAULT).await;
        // let parent = parent
        //     .map(|addr | {
        //         addr.try_into()
        //             .expect("did not receive module neighbor info!")
        //     })
        //     .unwrap_or(ModuleNeighborInfo::NoNeighbor);

        // let child = self.base.internal_bus.receive_with_timeout(WAIT_DEFAULT).await;

        // let child = child
        //     .map(|addr| {
        //         addr.try_into()
        //             .expect("did not receive module neighbor info!")
        //     })
        //     .unwrap_or(ModuleNeighborInfo::NoNeighbor);

        // let state = match (parent, child) {
        //     (NoNeighbor, NoNeighbor) => ModuleNeighborInfo::Advanced(None, None),
        //     (Hub(i), NoNeighbor) => Advanced(Some(i), None),
        //     (NoNeighbor, Basic) => Advanced(None, Some(BasicTransmitter())),
        //     (Hub(i), Basic) => Advanced(Some(i), Some(BasicTransmitter())),
        //     (_, _) => panic!("unknown configuration!"),
        // };

        // println!("state of advanced: {:?}", &state);
        // self.base.internal_bus.send(state.clone().into());
        // self.external_bus.send(state.clone().into());
    }

    async fn configure_downstream(&mut self) {
        let mut socket_external_bus: crate::net::udp_state::AsyncSocketHandle<
            crate::net::device::AsyncGatewayDevice<
                crate::async_communication::AsyncGateway<Vec<u8>>,
               
            >,
            UDP> = self.base.socket(EXTERNAL_BUS_TRANSIENT_PORT).await;
        let mut socket_internal_bus = self.base.socket(INTERNAL_BUS_TRANSIENT_COM_PORT).await;

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
                    self.assign_ips(state).await;
                }
            }
        }

        // let child = self.external_bus.receive_with_timeout(WAIT_DEFAULT).await;
        // match child {
        //     None => self
        //         .base
        //         .internal_bus
        //         .send(ModuleNeighborInfo::NoNeighbor.into()),
        //     Some(_) => self
        //         .base
        //         .internal_bus
        //         .send(ModuleNeighborInfo::Basic.into()),
        // };
        // let state = self.base.internal_bus.receive().await;
        // println!("sending state downstream ...");
        // self.external_bus.send(state);
    }

    async fn configure_basic(&mut self) {
        println!("configuring basic...");
        let mut socket_parent = self.base.socket(EXTERNAL_BUS_TRANSIENT_PORT).await;
        socket_parent
            .send(
                &ModuleNeighborInfo::Basic.into_vec(),
                IPEndpoint {
                    addr: EXTERNAL_BUS_TRANSIENT_ADDRESS,
                    port: EXTERNAL_BUS_TRANSIENT_PORT,
                },
            )
            .await;
        println!("sending p4 identity");
        let parent = socket_parent.receive_with_timeout(WAIT_DEFAULT).await;

        let mut parent_info: ModuleNeighborInfo = ModuleNeighborInfo::NoNeighbor;
        if let Ok((val, endpoint)) = parent {
            parent_info = val
                .try_into()
                .expect("did not receive module info on external bus");
        }
        println!("state from pov of basic: {:?}", &parent_info);
        self.assign_ips(parent_info).await;
        
    }


    async fn assign_ips(&mut self, module_info: ModuleNeighborInfo){
        let transmitter: Transmitter = self.initial_configuration.into();
        const SYSMODULES: [(IpAddress,  Sysmodule); 3] = [
            (TRANSIENT_PI_ID, Sysmodule::PI),
            (TRANSIENT_PV_ID, Sysmodule::PV),
            (TRANSIENT_HMI_ID, Sysmodule::HMI)
        ];

        for (addr, module) in SYSMODULES {
            let new_ip = determine_ip(&module, &transmitter, &module_info);
            let mut s = self.base.socket( ADDRESS_ASSIGNMENT_PORT).await;
            
            s.send(&Vec::from(new_ip.0), IPEndpoint {addr, port: ADDRESS_ASSIGNMENT_PORT }).await;
            
        }

    }
}

#[async_trait::async_trait]
impl SysModuleStartup for Com {
    async fn run_once(&mut self) {
        tokio::time::sleep(Duration::from_millis(1000)).await;
    }
    async fn on_start(&mut self) {
        println!("starting!!");
        match self.initial_configuration {
            ComType::HubCom(i) => self.configure_hub(i).await,
            ComType::AdvUpstream => self.configure_upstream().await,
            ComType::AdvDownstream => self.configure_downstream().await,
            ComType::Basic => self.configure_basic().await,
        }

        // no-op for now
    }
}
