#![feature(async_closure)]
mod async_communication;
mod communication;
mod internal_bus;
mod p4_advanced;
mod p4_basic;
mod sysmodule;
mod sysmodules;
mod utils;

use std::net::Ipv4Addr;

use async_communication::{AsyncGateway, DeadExternalBus, SysmoduleRPC};
use communication::IdentityResolver;
use futures::future::{BoxFuture, FutureExt};
use p4_advanced::P4Advanced;
use p4_basic::P4Basic;
use sysmodule::{HubIndex, ModuleNeighborInfo};
use sysmodules::common::SysModule;
use tokio::task;
use tokio::task::JoinHandle;
fn spawn_sysmodule(mut sysmodule: Box<dyn IdentityResolver + Send>) -> JoinHandle<()> {
    tokio::task::spawn(async move {
        sysmodule.discover_identity().await;
    })
}
#[tokio::main]
async fn main() {
    
    let (basic, adv) = AsyncGateway::new();
    let mut basic = P4Basic::new(Box::new(basic));

    let dead = DeadExternalBus {};
    let advanced = P4Advanced::new(Some(Box::new(dead)), Some(Box::new(adv)));
    let hmi_send = advanced.hmi.1.clone();

    let end_adv = tokio::spawn(async move {
        advanced.start().await;
    });
    let end = tokio::spawn(async move {
        basic.start().await;
    });

    // let mut f = async move |sys: &mut dyn SysModule| {sys.send((Ipv4Addr::new(0,0,0,0), "hello".to_string()))};
    // let func: SysmoduleRPC = Box::new( move |sys: &mut dyn SysModule|
    // {
    //     return async
    //     {
    //         sys.send((Ipv4Addr::new(0,0,0,0), "hello from hmi".to_string()));

    //     }.boxed()
    // });

    // hmi_send.send(
    // func);

    end.await;
    end_adv.await;
}
