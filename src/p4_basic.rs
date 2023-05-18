use crate::async_communication::{IPMessage};

use crate::utils::new_basic;
use crate::{
    async_communication::AsyncChannel,
    internal_bus,
    sysmodules::{com::*, common::*},
    utils::spawn_sysmodule,
};
use internal_bus::InternalBus;
use std::net::Ipv4Addr;

// basic p4
pub struct P4Basic {
    pub pv: (PV, TestingSender),
    pub com: (Com, TestingSender),
    pub hmi: (HMI, TestingSender),
    pub bus: InternalBus,
}

impl P4Basic {
    pub fn new(parent: Box<dyn AsyncChannel<IPMessage>>) -> Self {
        let mut bus = InternalBus::new();

        let ( pv, ib_pv, pv_test_tx) = new_basic(Ipv4Addr::UNSPECIFIED);
        let (com, ib_com, com_test_tx) = new_basic(Ipv4Addr::UNSPECIFIED);
        let (hmi, ib_hmi, hmi_test_tx) = new_basic(Ipv4Addr::UNSPECIFIED);
        
        bus.subscribe(ib_com);
        bus.subscribe(ib_hmi);
        bus.subscribe(ib_pv);

        let com_module = Com::new(parent, com, ComType::Basic);

        return Self {
            pv: (PV(pv), pv_test_tx),
            com: (com_module, com_test_tx),
            hmi: (HMI(hmi), hmi_test_tx),
            bus,
        };
    }

    ///
    ///
    ///
    ///
    /// starts a P4 simulation
    pub async fn start(mut self) {
        let pv = spawn_sysmodule(self.pv.0 .0);
        let hmi = spawn_sysmodule(self.hmi.0 .0);
        let com = spawn_sysmodule(self.com.0);

        let bus = tokio::spawn(async move {
            loop {
                self.bus.run_once().await;
            }
        });
        let futures = vec![pv, hmi, com, bus];
        for i in futures {
            _ = i.await;
        }
    }
}
