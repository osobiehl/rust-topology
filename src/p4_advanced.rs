use crate::async_communication::{AsyncGateway, IPMessage};

use crate::{
    async_communication::AsyncChannel,
    internal_bus,
    sysmodules::{com::*, common::*},
    utils::spawn_sysmodule,
};
use internal_bus::InternalBus;
use std::net::Ipv4Addr;
use tokio::task::JoinHandle;
// basic p4
pub struct P4Advanced {
    pub pv: Option<(PV, TestingSender)>,
    pub hub_to_adv: Option<(Com, TestingSender)>,
    pub adv_to_basic: Option<(Com, TestingSender)>,
    pub hmi: (HMI, TestingSender),
    pub bus: InternalBus,
}
pub fn new_basic(
    initial_address: Ipv4Addr,
) -> (BasicModule, AsyncGateway<IPMessage>, TestingSender) {
    let (mod_to_ibus, ibus_to_mod) = AsyncGateway::new();
    let (mod_test_tx, mod_test_rx) = tokio::sync::mpsc::unbounded_channel();
    let module = BasicModule::new(mod_to_ibus, mod_test_rx, initial_address);
    return (module, ibus_to_mod, mod_test_tx);
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
