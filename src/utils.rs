use std::sync::Arc;

use crate::async_communication::AsyncGateway;
use crate::TestDevice;
use crate::net::device::{NetifPair, setup_if, AsyncGatewayDevice};
use crate::net::udp_state::UDPState;
use crate::{
    sysmodules::{common::*},
};
use smoltcp::wire::{IpCidr};


pub fn new_netif( addr: IpCidr ) -> (NetifPair<TestDevice>, AsyncGateway<Vec<u8>>){
    let ( dev1, ib_side1 ) = AsyncGateway::<Vec<u8>>::new();
    let stack1 = setup_if(addr, Box::new(AsyncGatewayDevice::new(dev1)));
    return (stack1, ib_side1)
}

pub fn external_bus_netif(addr1: IpCidr, addr2: IpCidr) -> (NetifPair<TestDevice>, NetifPair<TestDevice>){
    let ( dev1, dev2 ) = AsyncGateway::<Vec<u8>>::new_async_device();
    return (
        setup_if(addr1, Box::new(dev1))
        ,
        setup_if(addr2, Box::new(dev2))
    )

}


pub fn new_internal_module(
    initial_address: IpCidr,
) -> (BasicModule, AsyncGateway<Vec<u8>>, TestingSender) {
    let (netif, gateway) = new_netif(initial_address);
    let (module, mod_test_tx) = new_module(vec![netif]);
    return (module, gateway, mod_test_tx)

}

pub fn new_module(netifs: Vec<NetifPair<TestDevice>>)->(BasicModule, TestingSender){
    let udp_1: Arc<Mutex<UDPState<TestDevice>>> = Arc::new(Mutex::new(UDPState::new(netifs) ) );
    let (mod_test_tx, mod_test_rx) = tokio::sync::mpsc::unbounded_channel();
    let module = BasicModule::new(udp_1, mod_test_rx );
    return (module, mod_test_tx)


}


use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub fn spawn_test_sysmodule<M: SysModuleStartup + Send + 'static>(mut module: M) -> JoinHandle<()> {
    let h = tokio::spawn(async move {
        module.on_start().await;

        module.run_once().await;
        
    });
    return h;
}
