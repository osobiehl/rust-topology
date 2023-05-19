use crate::async_communication::AsyncGateway;

use std::net::Ipv4Addr;
use crate::{
    internal_bus,
    sysmodules::{common::*},
};

pub fn new_basic(
    initial_address: Ipv4Addr,
) -> (BasicModule, AsyncGateway<Vec<u8>>, TestingSender) {
    let (mod_to_ibus, ibus_to_mod) = AsyncGateway::new();
    let (mod_test_tx, mod_test_rx) = tokio::sync::mpsc::unbounded_channel();
    let module = BasicModule::new(mod_to_ibus, mod_test_rx, initial_address);
    return (module, ibus_to_mod, mod_test_tx);
}


use tokio::task::JoinHandle;

pub fn spawn_sysmodule<M: SysModuleStartup + Send + 'static>(mut module: M) -> JoinHandle<()> {
    let h = tokio::spawn(async move {
        module.on_start().await;
        loop {
            module.run_once().await;
        }
    });
    return h;
}
