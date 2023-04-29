
mod sysmodule;
mod communication;
use sysmodule::transmitters::{P4Advanced, P4Basic, P4Hub};
use communication::IdentityResolver;
use sysmodule::{BasicTransmitter, HubIndex,ModuleNeighborInfo};
use communication::BlockingExternalBus;
use bichannel::{Channel, channel};
use std::{thread::{self, JoinHandle}, time::Duration};

fn spawn_sysmodule( mut sysmodule: Box<dyn IdentityResolver + Send> ) -> JoinHandle<()>
{
    thread::spawn(move || 
    {
        sysmodule.discover_identity();
        thread::sleep(Duration::from_secs(1));
    })
}

fn main() {
    let (left, right ) = BlockingExternalBus::new();
    let basic= P4Basic::new(
        Box::new(left)
    );
    let (hub_to_adv, adv_to_hub) = BlockingExternalBus::new();



    let advanced = P4Advanced::new(
        Box::new(adv_to_hub), 
        Box::new(right)
    );

    
    let hub = P4Hub::new(
        [
            Some(Box::new( hub_to_adv)),
            None,
            None,
            None
        ]
    );


    let x = spawn_sysmodule(Box::new(basic));
    let y = spawn_sysmodule(Box::new(advanced));
    let z = spawn_sysmodule(Box::new(hub));

    let _ = x.join();
    let _ = y.join();
    let _ = z.join();

}
