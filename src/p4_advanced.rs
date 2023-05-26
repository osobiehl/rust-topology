
use crate::{
    async_communication::{AsyncGateway},
    internal_bus,
    sysmodules::{com::*, common::*},
    utils::{spawn_test_sysmodule, new_module, new_netif, new_internal_module}, net::device::{setup_if, AsyncGatewayDevice},
};

use internal_bus::InternalBus;
use smoltcp::wire::IpCidr;


// basic p4
pub struct P4Advanced{
    pub pv: Option<(PV, TestingSender)>,
    pub hub_to_adv: Option<(Com, TestingSender)>,
    pub adv_to_basic: Option<(Com, TestingSender)>,
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
        if let Some(external_bus) = hub {
            let ( gateway_netif, ib_gateway) = new_netif(IpCidr::new(TRANSIENT_GATEWAY_ID, 24));
            bus.subscribe(ib_gateway);
            let net = setup_if(ComType::AdvUpstream.external_bus_ip(), Box::new(AsyncGatewayDevice::new(external_bus)) );
            let (com_mod, test_mod) = new_module( vec![gateway_netif, net]);
            hub2adv = Some((Com::new(com_mod, ComType::AdvUpstream), test_mod));
        }
        // TODO: add PI module

        if let Some(external_bus) = basic {
            let ( gateway_netif, ib_gateway) = new_netif(IpCidr::new(TRANSIENT_GATEWAY_ID, 24));
            bus.subscribe(ib_gateway);
            let net = setup_if(ComType::AdvDownstream.external_bus_ip(), Box::new(AsyncGatewayDevice::new(external_bus)) );
            let (com_mod, test_mod) = new_module( vec![gateway_netif, net]);
            basic2adv = Some((Com::new(com_mod, ComType::AdvDownstream), test_mod));

        }
        else {
            let ( gateway_netif, _ib_gateway) = new_netif(IpCidr::new(TRANSIENT_GATEWAY_ID, 24));
            let  (pv_, test) = new_module(vec![gateway_netif]);
            pv = Some((PV(pv_), test));
        }
        let (hmi, ib_hmi, hmi_test_tx) = new_internal_module(IpCidr::new(TRANSIENT_HMI_ID, 24));

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

        if let Some((com, _)) = self.adv_to_basic {
            futures.push(spawn_test_sysmodule(com))
        }
        if let Some((com, _)) = self.hub_to_adv {
            futures.push(spawn_test_sysmodule(com));
        }
        if let Some((pv, _)) = self.pv {
            futures.push(spawn_test_sysmodule(pv.0));
        }
        futures.push(spawn_test_sysmodule(self.hmi.0 .0));

        futures.push(tokio::spawn(async move {
            loop{
                self.bus.run_once().await;
            }
        }));

        for f in futures{
            let _ = f.await;
        }
       
    }
}
