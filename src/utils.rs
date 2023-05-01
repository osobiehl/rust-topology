use crate::async_communication::AsyncGateway;

use crate::{
    async_communication::AsyncChannel,
    internal_bus,
    sysmodules::{com::*, common::*},
};
use internal_bus::InternalBus;
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
