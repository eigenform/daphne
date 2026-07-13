
use anyhow::{Result, anyhow};
use num_enum::{IntoPrimitive, FromPrimitive, TryFromPrimitive};
use bitflags::bitflags;
use super::*;

/// Subcommands for the DAP info command
#[derive(Clone, Copy, IntoPrimitive)]
#[repr(u8)]
pub enum InfoReqId {
    VendorName   = 0x01,
    ProductName  = 0x02,
    SerialNo     = 0x03,
    ProtocolVer  = 0x04,
    TgtDevVendor = 0x05,
    TgtDevName   = 0x06,
    TgtBrdVendor = 0x07,
    TgtBrdName   = 0x08,
    ProductFwVer = 0x09,
    Capabilities = 0xf0,
    MaxPktCnt    = 0xfe,
    MaxPktSize   = 0xff,
}

#[repr(transparent)]
#[derive(Debug)]
pub struct DapCapabilityInfo0(pub u8);
bitflags! {
    impl DapCapabilityInfo0: u8 {
        const SWD                 = 1 << 0;
        const JTAG                = 1 << 1;
        const SWO_UART            = 1 << 2;
        const SWO_MANCHESTER      = 1 << 3;
        const ATOMIC              = 1 << 4;
        const TEST_DOMAIN_TIMER   = 1 << 5;
        const SWO_STREAMING_TRACE = 1 << 6;
        const UART                = 1 << 7;
    }
}
impl ToString for DapCapabilityInfo0 {
    fn to_string(&self) -> String {
        use bitflags::parser::*;
        let mut s = String::new();
        to_writer::<Self>(self, &mut s).unwrap();
        s
    }
}


#[derive(Clone, Copy, IntoPrimitive, PartialEq, Eq)]
#[repr(u8)]
pub enum HostStatusType {
    Connect = 0x00,
    Running = 0x01,
}

#[derive(Clone, Copy, IntoPrimitive, PartialEq, Eq)]
#[repr(u8)]
pub enum ConnectCmdPort {
    Default  = 0x00,
    SwdMode  = 0x01,
    JtagMode = 0x02,
}

#[derive(Clone, Copy, TryFromPrimitive, IntoPrimitive, PartialEq, Eq)]
#[repr(u8)]
pub enum ConnectRespPort {
    Failed = 0x00,
    Swd    = 0x01,
    Jtag   = 0x02,
}



/// DAP "info" command
pub struct InfoCmd {
    pub req_id: InfoReqId,
}
impl DapCommand for InfoCmd {
    const ID: DapCmdId = DapCmdId::Info;
    type Resp = InfoResp;
    fn to_packet(&self) -> Result<DapPacketBuf> {
        Ok(DapPacketBuf::new(
            Self::ID.into(), &[self.req_id.into()]
        ))
    }
}
pub struct InfoResp {
    pub data: Vec<u8>,
}
impl DapResponse for InfoResp {
    fn from_packet(pkt: &DapPacketBuf) -> Result<Self> {
        let content = pkt.content();
        let sz = content[0] as usize;
        assert!(sz <= DapPacketBuf::MAX_DAT_SZ - 2, "uhhh?");
        let data = &content[1..];
        assert!(data.len() == sz,
            "expected {} bytes, got {}?", sz, data.len()
        );
        Ok(Self { data: data.to_vec() })
    }
}


/// DAP "host status" command
pub struct HostStatusCmd {
    pub ty: HostStatusType,
    pub sts: u8,
}
impl DapCommand for HostStatusCmd {
    const ID: DapCmdId = DapCmdId::HostStatus;
    type Resp = HostStatusResp;
    fn to_packet(&self) -> Result<DapPacketBuf> {
        Ok(DapPacketBuf::new(Self::ID.into(), &[
            self.ty.into(), self.sts
        ]))
    }
}

pub struct HostStatusResp {
    pub _zero: u8,
}
impl DapResponse for HostStatusResp {
    fn from_packet(pkt: &DapPacketBuf) -> Result<Self> {
        assert!(pkt.content().len() == 1);
        let res = Self { _zero: pkt.content()[0], };
        Ok(res)
    }
}

/// DAP "connect" command
pub struct ConnectCmd {
    pub port: ConnectCmdPort,
}
impl DapCommand for ConnectCmd {
    const ID: DapCmdId = DapCmdId::Connect;
    type Resp = ConnectResp;
    fn to_packet(&self) -> Result<DapPacketBuf> {
        Ok(DapPacketBuf::new(Self::ID.into(), &[
            self.port.into()
        ]))
    }
}

pub struct ConnectResp {
    pub port: ConnectRespPort,
}
impl DapResponse for ConnectResp {
    fn from_packet(pkt: &DapPacketBuf) -> Result<Self> {
        assert!(pkt.content().len() == 1);
        let port = ConnectRespPort::try_from_primitive(
            pkt.content()[0]
        )?;
        Ok(Self { port })
    }
}



/// DAP "disconnect" command
pub struct DisconnectCmd;
impl DapCommand for DisconnectCmd {
    const ID: DapCmdId = DapCmdId::Disconnect;
    type Resp = DisconnectResp;
    fn to_packet(&self) -> Result<DapPacketBuf> {
        Ok(DapPacketBuf::new(Self::ID.into(), &[]))
    }
}
pub struct DisconnectResp {
    pub sts: DapResponseStatus,
}
impl DapResponse for DisconnectResp {
    fn from_packet(pkt: &DapPacketBuf) -> Result<Self> {
        assert!(pkt.content().len() == 1);
        let sts = DapResponseStatus::try_from_primitive(
            pkt.content()[0]
        )?;
        Ok(Self { sts })
    }
}


