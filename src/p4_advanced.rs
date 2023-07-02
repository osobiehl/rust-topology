
use crate::{
    channel::async_communication::{AsyncGateway},
    channel::internal_bus::InternalBus,
    sysmodules::{com::*, common::*},
    utils::{spawn_test_sysmodule, new_module, new_netif, new_internal_module, new_com}, net::device::{setup_if, AsyncGatewayDevice},
};

use futures::future::join_all;
use smoltcp::wire::IpCidr;


pub struct P4Advanced{
    pub pv: Option<(PV, TestingSender)>,
    pub hub_to_adv: Option<Com>,
    pub adv_to_basic: Option<Com>,
    pub hmi: (HMI, TestingSender),
    pub bus: InternalBus,
}

impl P4Advanced {
    pub fn new(
        hub: Option<AsyncGateway<Vec<u8>>>,
        basic: Option<AsyncGateway<Vec<u8>>>,
    ) -> Self {
        let mut bus = InternalBus::new();
        let mut pv = None;
        let mut hub2adv = None;
        let mut basic2adv = None;

        // TODO: add PI module

        if let Some(external_bus) = basic {
            let ( gateway_netif, ib_gateway) = new_netif(IpCidr::new(TRANSIENT_GATEWAY_ID, 24));
            bus.subscribe(ib_gateway);
            let net = setup_if(ComType::AdvDownstream.external_bus_ip(), Box::new(AsyncGatewayDevice::new(external_bus)) );            
            
            basic2adv = Some(new_com(vec![gateway_netif, net], ComType::AdvDownstream));

        }
        if let Some(external_bus) = hub {
            let ( gateway_netif, ib_gateway) = new_netif(IpCidr::new(TRANSIENT_GATEWAY_ID, 24));
            bus.subscribe(ib_gateway);
            let net = setup_if(ComType::AdvUpstream.external_bus_ip(), Box::new(AsyncGatewayDevice::new(external_bus)) );     
            hub2adv = Some(new_com(vec![gateway_netif, net], ComType::AdvUpstream));
        }
        else {
            let ( gateway_netif, ib_gateway) = new_netif(IpCidr::new(TRANSIENT_GATEWAY_ID, 24));
            let  (pv_, test) = new_module(vec![gateway_netif], BasicModuleType::PV);
            pv = Some((PV(pv_), test));
            bus.subscribe(ib_gateway);
        }
        let (hmi, ib_hmi, hmi_test_tx) = new_internal_module(IpCidr::new(TRANSIENT_HMI_ID, 24), BasicModuleType::HMI);

        bus.subscribe(ib_hmi);

        return Self {
            pv,
            hub_to_adv: hub2adv,
            adv_to_basic: basic2adv,
            hmi: (HMI(hmi), hmi_test_tx),
            bus,
        };
    }

    ///
    /// starts a P4 simulation using an advanced transmitter
    pub async fn start(mut self) {
        let mut futures = vec![];

 
        if let Some((pv, _)) = self.pv {
            futures.push(spawn_test_sysmodule(pv.0));
        }
        futures.push(spawn_test_sysmodule(self.hmi.0 .0));

        futures.push(tokio::spawn(async move {
            loop{
                self.bus.run_once().await;
            }
        }));
        if let Some(com) = self.adv_to_basic {
            futures.push(spawn_test_sysmodule(com))
        }
        if let Some(com) = self.hub_to_adv {
            futures.push(spawn_test_sysmodule(com));
        }

        let _ = join_all(futures).await;
        
       
    }
}
