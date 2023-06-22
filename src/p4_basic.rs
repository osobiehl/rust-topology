
use crate::async_communication::AsyncGateway;
use crate::utils::{new_internal_module, new_netif, new_module, new_com};
use crate::{
    internal_bus,
    sysmodules::{com::*, common::*},
    utils::spawn_test_sysmodule,
};

use internal_bus::InternalBus;

use crate::net::device::{setup_if, AsyncGatewayDevice};
use smoltcp::wire::{IpCidr};
use crate::sysmodules::common::{TRANSIENT_HMI_ID, TRANSIENT_PV_ID, TRANSIENT_GATEWAY_ID};
// basic p4
pub struct P4Basic {
    pub pv: (PV, TestingSender),
    pub com: Com,
    pub hmi: (HMI, TestingSender),
    pub pi: Option<(PI, TestingSender)>,
    pub bus: InternalBus,
}

impl P4Basic {
    pub fn new(parent: Option<AsyncGateway<Vec<u8>>>) -> Self {
        let mut bus = InternalBus::new();

        let ( pv, ib_pv, pv_test_tx) = new_internal_module(  IpCidr::new(TRANSIENT_PV_ID, 24), BasicModuleType::PV);

        let (hmi, ib_hmi, hmi_test_tx) = new_internal_module(IpCidr::new(TRANSIENT_HMI_ID, 24), BasicModuleType::HMI);
        
        bus.subscribe(ib_hmi);
        bus.subscribe(ib_pv);
        let ( gateway_netif, ib_gateway) = new_netif(IpCidr::new(TRANSIENT_GATEWAY_ID, 24));
        bus.subscribe(ib_gateway);

        // this should be an enum but oh well
        let mut com: Com;
        let mut Pi: Option<(PI, TestingSender)> = None;
        let mut com_vec = vec![gateway_netif];
        if let Some(external_bus) = parent{
            let net: crate::net::device::NetifPair<AsyncGatewayDevice<AsyncGateway<Vec<u8>>>> = setup_if(ComType::Basic.external_bus_ip(), Box::new(AsyncGatewayDevice::new(external_bus)) );
            com_vec.push(net);
        }
        // else{
        //     let  (pi, test) = new_module(vec![gateway_netif], BasicModuleType::PI);
        //     Pi = Some((PI{base: pi}, test));
            
        // }
        com = new_com(com_vec, ComType::Basic);


        

        return Self {
            pv: (PV(pv), pv_test_tx),
            com,
            hmi: (HMI(hmi), hmi_test_tx),
            pi: Pi,
            bus,
        };
    }

    ///
    ///
    ///
    ///
    /// starts a P4 simulation
    pub async fn start(mut self) {
        
        let pv = spawn_test_sysmodule(self.pv.0 .0);
        let hmi = spawn_test_sysmodule(self.hmi.0 .0);
        let mut futures = vec![pv, hmi];

        futures.push(spawn_test_sysmodule(self.com));

        if let Some(c) = self.pi{
            futures.push(spawn_test_sysmodule(c.0.base));
        }

        let bus = tokio::spawn(async move {
            loop {
                self.bus.run_once().await;
            }
        });
        bus.await;
        for f in futures{
            let _ = f.await;
        }


    }
}
