
#![feature(async_closure)]
mod sysmodule;
mod communication;
mod async_communication;
mod internal_bus;
mod com;
use std::borrow::Borrow;

use sysmodule::transmitters::{P4Advanced, P4Basic, P4Hub};
use communication::IdentityResolver;
use sysmodule::{HubIndex,ModuleNeighborInfo};
use tokio::task;
use async_communication::{AsyncGateway, DeadExternalBus};
use tokio::task::JoinHandle;
fn spawn_sysmodule( mut sysmodule: Box<dyn IdentityResolver + Send> ) -> JoinHandle<()>
{
        tokio::task::spawn(async move {
                sysmodule.discover_identity().await;
                
        })
}
#[tokio::main]
async fn main() {
    let (left, right ) = AsyncGateway::new();
    let basic= P4Basic::new(
        Box::new(left)
    );
    let (hub_to_adv, adv_to_hub) = AsyncGateway::new();

    let advanced = P4Advanced::new(
        Box::new(adv_to_hub),
        Box::new(right),
    );

    
    let hub = P4Hub::new(
        [
            Some(Box::new(hub_to_adv)),
            None,
            None,
            None
        ]
    );
    let a = spawn_sysmodule(Box::new(basic));
    let b = spawn_sysmodule(Box::new(advanced));
    let c = spawn_sysmodule(Box::new(hub));

    _ = a.await;
    _ = b.await;
    _ = c.await;    

}
