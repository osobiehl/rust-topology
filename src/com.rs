use crate::async_communication::{AsyncGateway, IPMessage,ChannelEvent};
pub mod sysmodules{

    use super::*;
    pub struct BasicModule{
        internal_bus: AsyncGateway<IPMessage>
    }
    pub struct Com{
        external_bus: AsyncGateway<ChannelEvent>,
        internal_bus: AsyncGateway<IPMessage>
    }

    pub struct PI{
        hart_interface: AsyncGateway<ChannelEvent>,
        internal_bus: AsyncGateway<IPMessage>
    }

    pub type PV = BasicModule;
    pub type HMI = BasicModule;



}