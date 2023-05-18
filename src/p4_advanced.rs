use crate::async_communication::{IPMessage};

use crate::{
    async_communication::AsyncChannel,
    internal_bus,
    sysmodules::{com::*, common::*},
    utils::{spawn_sysmodule, new_basic},
};
use internal_bus::InternalBus;
use std::net::Ipv4Addr;

// basic p4
pub struct P4Advanced {
    pub pv: Option<(PV, TestingSender)>,
    pub hub_to_adv: Option<(Com, TestingSender)>,
    pub adv_to_basic: Option<(Com, TestingSender)>,
    pub hmi: (HMI, TestingSender),
    pub bus: InternalBus,
}

impl P4Advanced {
    pub fn new(
        hub: Option<Box<dyn AsyncChannel<IPMessage>>>,
        basic: Option<Box<dyn AsyncChannel<IPMessage>>>,
    ) -> Self {
        let mut bus = InternalBus::new();
        let mut pv = None;
        let mut hub2adv = None;
        let mut basic2adv = None;
        if let Some(channel) = hub {
            let (com_basic, com_ib, com_test) = new_basic(Ipv4Addr::UNSPECIFIED);
            let com = Com::new(channel, com_basic, ComType::AdvUpstream);
            bus.subscribe(com_ib);
            hub2adv = Some((com, com_test));
        }
        if let Some(channel) = basic {
            let (com_basic, com_ib, com_test) = new_basic(Ipv4Addr::UNSPECIFIED);
            let com = Com::new(channel, com_basic, ComType::AdvDownstream);
            bus.subscribe(com_ib);
            basic2adv = Some((com, com_test));
        } else {
            let (pv_mod, ib_pv, pv_test_tx) = new_basic(Ipv4Addr::UNSPECIFIED);
            bus.subscribe(ib_pv);
            pv = Some((PV(pv_mod), pv_test_tx));
        }
        let (hmi, ib_hmi, hmi_test_tx) = new_basic(Ipv4Addr::UNSPECIFIED);

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
            futures.push(spawn_sysmodule(com))
        }
        if let Some((com, _)) = self.hub_to_adv {
            futures.push(spawn_sysmodule(com));
        }
        if let Some((pv, _)) = self.pv {
            futures.push(spawn_sysmodule(pv.0));
        }
        futures.push(spawn_sysmodule(self.hmi.0 .0));

        futures.push(tokio::spawn(async move {
            loop {
                self.bus.run_once().await;
            }
        }));
        for i in futures {
            _ = i.await;
        }
    }
}
