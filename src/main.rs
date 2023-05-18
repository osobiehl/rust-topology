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



use std::net::Ipv4Addr;

use async_communication::{AsyncGateway, DeadExternalBus};
use communication::IdentityResolver;

use futures::FutureExt;
use p4_advanced::P4Advanced;
use p4_basic::P4Basic;



use tokio::task::JoinHandle;

use crate::{async_communication::SysmoduleRPC, sysmodules::common::SysModule};
fn spawn_sysmodule(mut sysmodule: Box<dyn IdentityResolver + Send>) -> JoinHandle<()> {
    tokio::task::spawn(async move {
        sysmodule.discover_identity().await;
    })
}
#[tokio::main]
async fn main() {

    let (basic, adv) = AsyncGateway::new();
    let basic = P4Basic::new(Box::new(basic));

    let dead = DeadExternalBus {};
    let advanced = P4Advanced::new(Some(Box::new(dead)), Some(Box::new(adv)));
    let _hmi_send = advanced.hmi.1.clone();

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

mod test{
pub use super::*;
use crate::net::device::{STDRx,STDTx,TokioChannel, setup_if};
pub use net::udp_state::UDPState;
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address, Ipv6Address};
use smoltcp::socket::{tcp, udp};
use std::sync::Arc;
use tokio::sync::Mutex;


#[tokio::test(flavor = "multi_thread")]
async fn test_netif_setup(){
    let (tx1, rx1) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
    let (tx2, rx2) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

    let mut dev1 = TokioChannel::new(rx1, tx2.clone());
    let mut dev2 = TokioChannel::new(rx2, tx1.clone());

    let ip_1 = IpCidr::new(IpAddress::v4(192, 168, 69, 1), 24);
    let eth1 = EthernetAddress([0x02, 0x00, 0x00, 0x00, 0x00, 0x01]);


    let mut stack1 = setup_if(ip_1, &mut dev1);
    
    let ip_2 = IpCidr::new(IpAddress::v4(192, 168, 69, 2), 24);
    let eth2 = EthernetAddress([0x02, 0x00, 0x00, 0x00, 0x00, 0x02]);

    let stack2 = setup_if(ip_2, &mut dev2);

    let mut udp_1 = UDPState::new(stack1, dev1);
    let mut udp_2 = UDPState::new(stack2, dev2);


    let handle_send = udp_1.new_socket(6969);
    let handle_receive = udp_2.new_socket(6969);
    let hello = "hello_world!";
    {    let _ = udp_1.poll();
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
    let (tx1, rx1) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
    let (tx2, rx2) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

    let mut dev1 = TokioChannel::new(rx1, tx2.clone());
    let mut dev2 = TokioChannel::new(rx2, tx1.clone());

    let ip_1 = IpCidr::new(IpAddress::v4(192, 168, 69, 1), 24);
    let eth1 = EthernetAddress([0x02, 0x00, 0x00, 0x00, 0x00, 0x01]);


    let mut stack1 = setup_if(ip_1, &mut dev1);
    
    let ip_2 = IpCidr::new(IpAddress::v4(192, 168, 69, 2), 24);
    let eth2 = EthernetAddress([0x02, 0x00, 0x00, 0x00, 0x00, 0x02]);

    let stack2 = setup_if(ip_2, &mut dev2);

    let mut udp_1 = Arc::new(Mutex::new(UDPState::new(stack1, dev1) ) );
    let mut udp_2 = Arc::new(Mutex::new(UDPState::new(stack2, dev2)));

    let mut socket1 = AsyncUDPSocket::new(6969, udp_1.clone()).await;
    let mut socket2 = AsyncUDPSocket::new(6969, udp_2.clone()).await;
    let hello = "hello_world!";
    socket1.send(Vec::from(hello.as_bytes()), smoltcp::wire::IpEndpoint { addr: IpAddress::v4(192, 168, 69, 2), port: 6969 }).await;

    let (v, _) = socket2.recv().await.expect("socket is empty");

    assert_eq!(&v[..], hello.as_bytes(), "receive not equal to send!");
    
    
}


#[tokio::test(flavor = "multi_thread")]
async fn test_advanced_basic() {
    let (basic, adv) = AsyncGateway::new();
    let mut basic = P4Basic::new(Box::new(basic));

    let dead = DeadExternalBus {};
    let advanced = P4Advanced::new(Some(Box::new(dead)), Some(Box::new(adv)));
    let com_send = basic.com.1.clone();

    let end_adv = tokio::spawn(async move {
        advanced.start().await;
    });
    let end = tokio::spawn(async move {
        basic.start().await;
    });
    let mut f = async move |sys: &mut dyn SysModule| {sys.send((Ipv4Addr::new(0,0,0,0), "hello".to_string()))};
    let func: SysmoduleRPC = Box::new( move |sys: &mut dyn SysModule|
    {
        return async
        {
            sys.send((Ipv4Addr::new(0,0,0,0), "hello from com".to_string()));

        }.boxed()
    });
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    com_send.send(
    func);
    // end_adv.await;
    // end.await;
}

}