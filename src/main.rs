#![feature(async_closure)]
mod async_communication;
mod communication;
mod internal_bus;
mod p4_advanced;
mod p4_basic;
mod sysmodule;
mod sysmodules;
mod utils;
mod net;

use crate::net::device::{setup_if, AsyncGatewayDevice};
pub type TestDevice = AsyncGatewayDevice<AsyncGateway<Vec<u8>>>;



use std::net::Ipv4Addr;

use async_communication::{AsyncGateway, DeadExternalBus};
use communication::IdentityResolver;

use futures::FutureExt;
use p4_advanced::P4Advanced;
use p4_basic::P4Basic;



use tokio::task::JoinHandle;

use crate::{async_communication::SysmoduleRPC};
fn spawn_sysmodule(mut sysmodule: Box<dyn IdentityResolver + Send>) -> JoinHandle<()> {
    tokio::task::spawn(async move {
        sysmodule.discover_identity().await;
    })
}
#[tokio::main]
async fn main() {

    // let (basic, adv) = AsyncGateway::new();
    // let basic = P4Basic::new(Box::new(basic));

    // let dead = DeadExternalBus {};
    // let advanced = P4Advanced::new(Some(Box::new(dead)), Some(Box::new(adv)));
    // let _hmi_send = advanced.hmi.1.clone();

    // let end_adv = tokio::spawn(async move {
    //     advanced.start().await;
    // });
    // let end = tokio::spawn(async move {
    //     basic.start().await;
    // });

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

    // end.await;
    // end_adv.await;
}

mod test{
use crate::p4_basic::P4Basic;

pub use super::*;

pub use net::udp_state::UDPState;
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address, Ipv6Address};
use smoltcp::socket::{tcp, udp};
use std::sync::Arc;
use tokio::sync::Mutex;
use async_communication::{AsyncChannel};


#[tokio::test(flavor = "multi_thread")]
async fn test_netif_setup(){

    let (mut dev1, mut dev2 ) = AsyncGateway::<Vec<u8>>::new_async_device();


    let ip_1 = IpCidr::new(IpAddress::v4(192, 168, 69, 1), 24);


    let mut stack1 = setup_if(ip_1, Box::new(dev1));
    
    let ip_2 = IpCidr::new(IpAddress::v4(192, 168, 69, 2), 24);

    let stack2 = setup_if(ip_2, Box::new(dev2));

    let mut udp_1 = UDPState::new(vec![stack1]);
    let mut udp_2 = UDPState::new(vec![stack2]);


    let handle_send = udp_1.new_socket(6969);
    let handle_receive = udp_2.new_socket(6969);
    let hello = "hello_world!";
    {   
        let _ = udp_1.poll();
        let _ = udp_2.poll();

        let send_sock = udp_1.sockets.get_mut::<udp::Socket>(handle_send);
        let receive_sock = udp_2.sockets.get_mut::<udp::Socket>(handle_receive);
        let _ = udp_2.poll();

        
        send_sock.send_slice(hello.as_bytes(), smoltcp::wire::IpEndpoint { addr: IpAddress::v4(192, 168, 69, 2), port: 6969 });
    }
    let _ = udp_1.poll();
    let _ = udp_2.poll();
    {
        let receive_sock = udp_2.sockets.get_mut::<udp::Socket>(handle_receive);

        let (data, endpoint) = receive_sock.recv()
        .expect("did not receive anything!");
        let s = std::str::from_utf8(data).unwrap();
        assert_eq!(s, hello, "receive not equal to send!");
    
    }
    
}

use net::udp_state::{AsyncUDPSocket, UDPSocketRead};
#[tokio::test(flavor = "multi_thread")]
async fn test_async_netif(){
    let (mut dev1, mut dev2 ) = AsyncGateway::<Vec<u8>>::new();


    let ip_1 = IpCidr::new(IpAddress::v4(192, 168, 69, 1), 24);


    let mut stack1 = setup_if(ip_1, Box::new(AsyncGatewayDevice::new(dev1)));
    
    let ip_2 = IpCidr::new(IpAddress::v4(192, 168, 69, 2), 24);

    let stack2 = setup_if(ip_2, Box::new(AsyncGatewayDevice::new(dev2)));


    let mut udp_1 = Arc::new(Mutex::new(UDPState::new(vec![stack1]) ) );
    let mut udp_2 = Arc::new(Mutex::new(UDPState::new(vec![stack2])));

    let mut socket1 = AsyncUDPSocket::new(6969, udp_1.clone()).await;
    let mut socket2 = AsyncUDPSocket::new(6969, udp_2.clone()).await;
    let hello = "hello_world!";
    socket1.send(Vec::from(hello.as_bytes()), smoltcp::wire::IpEndpoint { addr: IpAddress::v4(192, 168, 69, 2), port: 6969 }).await;

    let (v, _) = socket2.recv().await.expect("socket is empty");

    assert_eq!(&v[..], hello.as_bytes(), "receive not equal to send!");
    
    
}

#[tokio::test(flavor = "multi_thread")]
async fn test_concurrent_wait(){
    let (mut dev1, mut dev2 ) = AsyncGateway::<Vec<u8>>::new();


    let ip_1 = IpCidr::new(IpAddress::v4(192, 168, 69, 1), 24);


    let mut stack1 = setup_if(ip_1, Box::new(AsyncGatewayDevice::new(dev1)));
    
    let ip_2 = IpCidr::new(IpAddress::v4(192, 168, 69, 2), 24);

    let stack2 = setup_if(ip_2, Box::new(AsyncGatewayDevice::new(dev2)));


    let mut udp_1 = Arc::new(Mutex::new(UDPState::new(vec![stack1]) ) );
    let mut udp_2 = Arc::new(Mutex::new(UDPState::new(vec![stack2])));

    let mut socket1 = AsyncUDPSocket::new(6969, udp_1.clone()).await;
    let mut socket2 = AsyncUDPSocket::new(6969, udp_2.clone()).await;
    let mut socket_dummy = AsyncUDPSocket::new(6968, udp_2.clone()).await;

    let hello = "hello_world!";
    socket1.send(Vec::from(hello.as_bytes()), smoltcp::wire::IpEndpoint { addr: IpAddress::v4(192, 168, 69, 2), port: 6969 }).await;
    
    let incoming = futures::select! {
        x = socket2.recv().fuse() => Some(x),
        _ = socket_dummy.recv().fuse() => None
    };
    let incoming = incoming.expect("expect to receive something!");
    let (v, _ ) = incoming.unwrap();

    assert_eq!(&v[..], hello.as_bytes(), "receive not equal to send!");
    
    
}


#[tokio::test(flavor = "multi_thread")]
async fn test_second_netif_ingress(){

    let ( dev1,  mut stub1) = AsyncGateway::<Vec<u8>>::new();
    let  ( dev2, mut  stub2 ) = AsyncGateway::<Vec<u8>>::new(); 

    let ip_1 = IpCidr::new(IpAddress::v4(192, 168, 69, 1), 24);

    let ip_2 = IpCidr::new(IpAddress::v4(192, 169, 0, 1), 24);


    let stack1 = setup_if(ip_1, Box::new( AsyncGatewayDevice::new(dev1)));
    let stack2 = setup_if(ip_2, Box::new(AsyncGatewayDevice::new(dev2)));

    let mut udp_1 = Arc::new(Mutex::new(UDPState::new(vec![stack1, stack2]) ) );


    let hello = "hello_world!";
    let mut socket1 = AsyncUDPSocket::new(6969, udp_1.clone()).await;
    let mut socket2 = AsyncUDPSocket::new(smoltcp::wire::IpEndpoint { addr: IpAddress::v4(192, 169, 0, 1), port: 6968 }, udp_1.clone()).await;

    socket1.send(Vec::from(hello.as_bytes()), smoltcp::wire::IpEndpoint { addr: IpAddress::v4(192, 169, 0, 1), port: 6968 }).await;
    let r = stub2.try_receive().expect("stub should have received data");
    //loop back to itself :)
    stub2.send(r);

    let r = socket2.recv().await.expect("received nothing!");


        
}
use simple_logger::SimpleLogger;
use log::{LevelFilter, trace, logger};
//     simple_logger::init_with_level(log::Level::Trace);

#[tokio::test(flavor = "multi_thread")]
async fn test_broadcast_ability(){
    let ( dev1,   dev2) = AsyncGateway::<Vec<u8>>::new_async_device();

    let ip_1 = IpCidr::new(IpAddress::v4(192, 168, 69, 1), 24);

    let ip_2 = IpCidr::new(IpAddress::v4(192, 169, 0, 1), 24);


    let stack1 = setup_if(ip_1, Box::new(dev1));
    let stack2 = setup_if(ip_2, Box::new(dev2));

    let mut udp_1 = Arc::new(Mutex::new(UDPState::new(vec![stack1, stack2]) ) );


    let hello = "hello_world!";
    let mut socket1 = AsyncUDPSocket::new(6969, udp_1.clone()).await;
    let mut socket2 = AsyncUDPSocket::new(smoltcp::wire::IpEndpoint { addr: IpAddress::v4(192, 169, 0, 1), port: 6968 }, udp_1.clone()).await;

    socket1.send(Vec::from(hello.as_bytes()), smoltcp::wire::IpEndpoint { addr: IpAddress::v4(255, 255, 255, 255), port: 6968 }).await;

    let r = socket2.recv().await.expect("received nothing!");



        
}


#[tokio::test(flavor = "multi_thread")]
async fn test_two_netif_response(){

    let (mut dev1, mut test_netif_1 ) = AsyncGateway::<Vec<u8>>::new_async_device();
    let (mut dev2, mut test_netif_2 ) = AsyncGateway::<Vec<u8>>::new_async_device();

    let ip_1 = IpCidr::new(IpAddress::v4(192, 168, 69, 1), 24);

    let ip_2 = IpCidr::new(IpAddress::v4(192, 169, 0, 1), 24);


    let stack1 = setup_if(ip_1, Box::new(dev1));
    let stack2 = setup_if(ip_2, Box::new(dev2));

    let mut udp_1 = Arc::new(Mutex::new(UDPState::new(vec![stack1, stack2]) ) );


    let hello = "hello_world!";
    let mut socket1 = AsyncUDPSocket::new(6969, udp_1.clone()).await;

    socket1.send(Vec::from(hello.as_bytes()), smoltcp::wire::IpEndpoint { addr: IpAddress::v4(192, 168, 69, 2), port: 6969 }).await;
    assert! (test_netif_1.try_receive().is_some());
    assert!( test_netif_2.try_receive().is_none());

    let mut socket2 = AsyncUDPSocket::new(smoltcp::wire::IpEndpoint { addr: IpAddress::v4(192, 169, 0, 1), port: 6968 }, udp_1.clone()).await;

    socket2.send(Vec::from(hello.as_bytes()), smoltcp::wire::IpEndpoint { addr: IpAddress::v4(192, 169, 0, 2), port: 6962 }).await;

    assert! (test_netif_1.try_receive().is_none());
    assert!( test_netif_2.try_receive().is_some());
        
}

#[tokio::test(flavor = "multi_thread")]
async fn test_internal_bus_communication(){

    let ( dev1, ib_side1 ) = AsyncGateway::<Vec<u8>>::new_async_device();
    let ( dev2, ib_side2 ) = AsyncGateway::<Vec<u8>>::new_async_device();
    let ( dev3, ib_side3 ) = AsyncGateway::<Vec<u8>>::new_async_device();

    let mut ib = internal_bus::InternalBus::new();
    ib.subscribe(ib_side1.gateway);
    ib.subscribe(ib_side2.gateway);
    ib.subscribe(ib_side3.gateway);


    let addr_1 = IpAddress::v4(192, 168, 69, 1);
    let addr_2 = IpAddress::v4(192, 168, 69, 2);
    let addr_3 = IpAddress::v4(192, 168, 69, 3);

    let ip_1 = IpCidr::new(addr_1.clone(), 24);
    let ip_2 = IpCidr::new(addr_2.clone(), 24);
    let ip_3 = IpCidr::new(addr_3.clone(), 24);

    let stack1 = setup_if(ip_1, Box::new(dev1));
    let stack2 = setup_if(ip_2, Box::new(dev2));
    let stack3 = setup_if(ip_3, Box::new(dev3));


    let mut udp_1: Arc<Mutex<UDPState<TestDevice>>> = Arc::new(Mutex::new(UDPState::new(vec![stack1]) ) );
    let mut udp_2: Arc<Mutex<UDPState<TestDevice>>> = Arc::new(Mutex::new(UDPState::new(vec![stack2]) ) );
    let mut udp_3: Arc<Mutex<UDPState<TestDevice>>> = Arc::new(Mutex::new(UDPState::new(vec![stack3]) ) );

    let a = tokio::spawn(async move {
        loop{ib.run_once().await}
    });

    let  task_2  =tokio::spawn(async move {
        let mut socket2 = AsyncUDPSocket::new( 6969, udp_2.clone()).await;
        let a = socket2.receive_with_timeout(std::time::Duration::from_millis(500)).await.expect("did not receive in time");
        let a =socket2.receive_with_timeout(std::time::Duration::from_millis(500)).await.expect("did not receive in time");
    });

    let task_1 = tokio::spawn(async move {
        let hello = "hello_world!";
        let mut socket1 = AsyncUDPSocket::new(6969, udp_1.clone()).await;
        socket1.send(Vec::from(hello.as_bytes()), smoltcp::wire::IpEndpoint { addr: addr_2, port: 6969 }).await;
        task_2.await.expect("panic occured in read task!");

    });

    let task_3 = tokio::spawn(async move {
        let hello = "hello_world!";
        let mut socket1 = AsyncUDPSocket::new(6969, udp_3.clone()).await;
        socket1.send(Vec::from(hello.as_bytes()), smoltcp::wire::IpEndpoint { addr: addr_2, port: 6969 }).await;
        task_1.await.expect("panic occured in first send task!");
    });


    
    task_3.await;
    let _ = a.await; // cleanup: OK to panic
        
}

#[tokio::test(flavor = "multi_thread")]
async fn test_advanced_basic() {
    let mut basic = P4Basic::new(None);
    basic.start();
    
    // let dead = DeadExternalBus {};
    // let advanced = P4Advanced::new(Some(Box::new(dead)), Some(Box::new(adv)));
    // let com_send = basic.com.1.clone();

    // let end_adv = tokio::spawn(async move {
    //     advanced.start().await;
    // });
    // let end = tokio::spawn(async move {
    //     basic.start().await;
    // });
    // let mut f = async move |sys: &mut dyn SysModule| {sys.send("hello".as_bytes().to_vec())};
    // let func: SysmoduleRPC = Box::new( move |sys: &mut dyn SysModule|
    // {
    //     return async
    //     {
    //         sys.send( "hello from com".as_bytes().to_vec());

    //     }.boxed()
    // });
    // tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    // com_send.send(
    // func);
    // end_adv.await;
    // end.await;
}

}