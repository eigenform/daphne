
use anyhow::{Result, anyhow};
use num_enum::{IntoPrimitive, FromPrimitive, TryFromPrimitive};
use bitflags::bitflags;

/// Implemented on types that represent a CMSIS-DAPv2 command.
pub trait DapCommand: Sized {
    /// Associated Command ID
    const ID: DapCmdId;

    /// Associated response type.
    type Resp: DapResponse;

    fn to_packet(&self) -> Result<DapPacketBuf>;
}
/// Implemented on types that represent a CMSIS-DAPv2 command response.
pub trait DapResponse: Sized {
    fn from_packet(buf: &DapPacketBuf) -> Result<Self>;
}

/// Representing a CMSIS-DAPv2 packet.
pub struct DapPacketBuf {
    data: [u8; Self::MAX_PKT_SZ],
    len: usize,
}
impl DapPacketBuf {
    /// The maximum size of a CMSIS-DAPv2 packet (in bytes);
    const MAX_PKT_SZ: usize  = 64;
    /// The maximum size of CMSIS-DAPv2 packet contents (in bytes);
    const MAX_DAT_SZ: usize = Self::MAX_PKT_SZ - 1;


    pub fn new_empty(len: usize) -> Self {
        assert!(len <= Self::MAX_PKT_SZ);
        Self { data: [0; Self::MAX_PKT_SZ], len }
    }

    pub fn new(cmd: u8, content: &[u8]) -> Self {
        assert!(content.len() + 1 <= Self::MAX_PKT_SZ);
        let mut data = [0; Self::MAX_PKT_SZ];
        data[0] = cmd;
        data[1..content.len()+1].copy_from_slice(&content[0..content.len()]);
        Self { data, len: content.len() + 1 }
    }

    pub fn new_from_slice(slice: &[u8]) -> Result<Self> {
        if slice.len() < 1 {
            return Err(anyhow!("packet must be at least 1 bytes"));
        }
        if slice.len() > Self::MAX_PKT_SZ {
            return Err(
                anyhow!("packet must be less than MAX_PKT_SZ ({}B)",
                    Self::MAX_PKT_SZ)
            );
        }

        let mut data = [0; Self::MAX_PKT_SZ];
        data[..slice.len()].copy_from_slice(slice);
        Ok(Self { data, len: slice.len() })
    }

    /// Return the 8-bit command ID header byte.
    pub fn id(&self) -> u8 {
        self.data[0]
    }

    /// The length of the entire message (header byte plus content bytes).
    pub fn len(&self) -> usize {
        self.len
    }

    /// Return a slice of bytes containing the "content" of the packet
    /// (without the header byte).
    pub fn content(&self) -> &[u8] {
        &self.data[1..self.len]
    }

    /// Return a slice of bytes containing the entire packet.
    pub fn data(&self) -> &[u8] {
        &self.data[0..self.len]
    }
}


/// 8-bit DAP command ID
#[derive(Clone, Copy, Debug, IntoPrimitive, FromPrimitive, PartialEq, Eq)]
#[repr(u8)]
pub enum DapCmdId {
    // General Commands
    Info         = 0x00,
    HostStatus   = 0x01,
    Connect      = 0x02,
    Disconnect   = 0x03,
    WriteAbort   = 0x08,
    Delay        = 0x09,
    ResetTarget  = 0x0a,

    // Common SWD/JTAG Commands
    SwjPins      = 0x10,
    SwjClock     = 0x11,
    SwjSequence  = 0x12,

    // SWD Commands
    SwdConfigure = 0x13,
    SwdSequence  = 0x1d,

    #[num_enum(catch_all)]
    Undef(u8),
}

#[derive(Clone, Copy, TryFromPrimitive, IntoPrimitive, PartialEq, Eq)]
#[repr(u8)]
pub enum DapResponseStatus {
    DapOk  = 0x00,
    DapErr = 0xff,
}

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

/// DAP "SWD/JTAG pins" command
pub struct SwjPinsCmd {
    pub pin_out: SwjPinBits,
    pub pin_sel: SwjPinBits,
    pub pin_wait: u32,
}
impl DapCommand for SwjPinsCmd {
    const ID: DapCmdId = DapCmdId::SwjPins;
    type Resp = SwjPinsResp;
    fn to_packet(&self) -> Result<DapPacketBuf> {

        let buf = &[
            [self.pin_out.bits(), self.pin_sel.bits()].as_slice(),
            self.pin_wait.to_le_bytes().as_slice()
        ].concat();

        Ok(DapPacketBuf::new(Self::ID.into(), buf))
    }
}


pub struct SwjPinsResp {
    pub pin_inp: SwjPinBits,
}
impl DapResponse for SwjPinsResp {
    fn from_packet(pkt: &DapPacketBuf) -> Result<Self> {
        assert!(pkt.content().len() == 1);
        let pin_inp = SwjPinBits::from_bits_retain(
            pkt.content()[0]
        );
        Ok(Self { pin_inp })
    }
}


/// DAP "SWD/JTAG clock" command
pub struct SwjClockCmd {
    /// In hertz
    pub clock: u32,
}
impl DapCommand for SwjClockCmd {
    const ID: DapCmdId = DapCmdId::SwjClock;
    type Resp = SwjClockResp;
    fn to_packet(&self) -> Result<DapPacketBuf> {
        Ok(DapPacketBuf::new(Self::ID.into(),
            &self.clock.to_le_bytes()
        ))
    }
}


pub struct SwjClockResp {
    pub sts: DapResponseStatus,
}
impl DapResponse for SwjClockResp {
    fn from_packet(pkt: &DapPacketBuf) -> Result<Self> {
        assert!(pkt.content().len() == 1);
        let sts = DapResponseStatus::try_from_primitive(
            pkt.content()[0]
        )?;
        Ok(Self { sts })
    }
}


/// DAP "SWD/JTAG sequence" command
pub struct SwjSequenceCmd {
    pub seq_bitcnt: u8,
    pub seq_bitdata: Vec<u8>,
}
impl DapCommand for SwjSequenceCmd {
    const ID: DapCmdId = DapCmdId::SwjSequence;
    type Resp = SwjSequenceResp;
    fn to_packet(&self) -> Result<DapPacketBuf> {

        let data = self.seq_bitdata.as_slice();
        let buf = &[
            [self.seq_bitcnt].as_slice(), data
        ].concat();

        Ok(DapPacketBuf::new(Self::ID.into(), buf))
    }
}


pub struct SwjSequenceResp {
    pub sts: DapResponseStatus,
}
impl DapResponse for SwjSequenceResp {
    fn from_packet(pkt: &DapPacketBuf) -> Result<Self> {
        assert!(pkt.content().len() == 1);
        let sts = DapResponseStatus::try_from_primitive(
            pkt.content()[0]
        )?;
        Ok(Self { sts })
    }
}


/// DAP "SWD configure" command
pub struct SwdConfigureCmd {
    pub cfg: SwdConfigurationBits,
}
impl DapCommand for SwdConfigureCmd {
    const ID: DapCmdId = DapCmdId::SwdConfigure;
    type Resp = SwdConfigureResp;
    fn to_packet(&self) -> Result<DapPacketBuf> {
        Ok(DapPacketBuf::new(Self::ID.into(), &[self.cfg.bits()]))
    }
}

pub struct SwdConfigureResp {
    pub sts: DapResponseStatus,
}
impl DapResponse for SwdConfigureResp {
    fn from_packet(pkt: &DapPacketBuf) -> Result<Self> {
        assert!(pkt.content().len() == 1);
        let sts = DapResponseStatus::try_from_primitive(
            pkt.content()[0]
        )?;
        Ok(Self { sts })
    }
}

/// DAP "SWD sequence" command
pub struct SwdSequenceCmd {
    seq_cnt: usize,
    seq_data: Vec<SwdSeq>,
}
impl SwdSequenceCmd {
    const MAX_CNT: usize = (
        (DapPacketBuf::MAX_PKT_SZ - 2) / 2
    );
}
impl DapCommand for SwdSequenceCmd {
    const ID: DapCmdId = DapCmdId::SwdSequence;
    type Resp = SwdSequenceResp;
    fn to_packet(&self) -> Result<DapPacketBuf> {
        if self.seq_cnt == 0 {
            return Err(anyhow!("sequence count must be > 0"));
        }
        if self.seq_cnt > Self::MAX_CNT {
            return Err(anyhow!("sequence count must be <= MAX_CNT"));
        }

        let cnt = self.seq_cnt as u8;
        let data: Vec<u8> = self.seq_data.iter()
            .map(|x| x.as_bytes())
            .flatten()
            .collect();

        let buf = &[
            &[cnt], data.as_slice()
        ].concat();

        Ok(DapPacketBuf::new(Self::ID.into(), buf))
    }
}
pub struct SwdSequenceResp {
    sts: DapResponseStatus,
    data: Vec<u8>,
}
impl DapResponse for SwdSequenceResp {
    fn from_packet(pkt: &DapPacketBuf) -> Result<Self> {
        let sts = DapResponseStatus::try_from_primitive(
            pkt.content()[0]
        )?;
        let data = pkt.content()[1..].to_vec();
        Ok(Self { sts, data })
    }
}

/// SWD sequencing mode.
pub enum SwdSeqMode {
    /// Bits are being driven over SWDIO
    Output,
    /// Bits are being received over SWDIO
    Input,
}

/// SWD sequence information.
pub struct SwdSeqInfo(u8);
impl SwdSeqInfo {
    const VALID_CNT: std::ops::RangeInclusive<usize> = 1..=64;
    const CYCLE_MASK: u8 = 0b0011_1111;

    pub fn new(cycles: usize, mode: SwdSeqMode) -> Result<Self> {
        if !Self::VALID_CNT.contains(&cycles) {
            return Err(anyhow!(
                "sequence must be between 1 and 64 TCK cycles (got {})",
                cycles
            ));
        }

        let cyc = (cycles as u8) & Self::CYCLE_MASK;
        let mde: u8 = match mode {
            SwdSeqMode::Output => 0 << 7,
            SwdSeqMode::Input  => 1 << 7,
        };

        Ok(Self(mde | cyc))
    }

    pub fn cycles(&self) -> usize {
        let res = self.0 & Self::CYCLE_MASK;
        if res == 0 { 64 } else { res as _ }
    }

    pub fn mode(&self) -> SwdSeqMode {
        match self.0 & 0b1000_0000 != 0 {
            false => SwdSeqMode::Output,
            true  => SwdSeqMode::Input,
        }
    }

    pub fn bits(&self) -> u8 {
        self.0
    }
}

/// Describes an SWD sequence.
pub struct SwdSeq {
    /// SWCLK cycles and SWDIO mode
    info: SwdSeqInfo,
    /// SWDIO data
    data: u8,
}
impl SwdSeq {
    pub fn as_bytes(&self) -> [u8; 2] {
        [self.info.bits(), self.data]
    }
}

#[repr(transparent)]
pub struct SwdConfigurationBits(u8);
bitflags! {
    impl SwdConfigurationBits: u8 {
        const TURNAROUND_1CYC = 0b00 << 0;
        const TURNAROUND_2CYC = 0b01 << 0;
        const TURNAROUND_3CYC = 0b10 << 0;
        const TURNAROUND_4CYC = 0b11 << 0;
        const DATA_PHASE      = 0b01 << 2;
    }
}

#[repr(transparent)]
pub struct SwjPinBits(u8);
bitflags! {
    impl SwjPinBits: u8 {
        const SWCLK_TCK = 1 << 0;
        const SWDIO_TMS = 1 << 1;
        const TDI       = 1 << 2;
        const TDO       = 1 << 3;
        const _UNK4     = 1 << 4;
        const nRTST     = 1 << 5;
        const _UNK6     = 1 << 6;
        const nRESET    = 1 << 7;
    }
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


