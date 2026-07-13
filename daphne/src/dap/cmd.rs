
use anyhow::{Result, anyhow};
use num_enum::{IntoPrimitive, FromPrimitive, TryFromPrimitive};
use bitflags::bitflags;

pub mod general;
pub mod xfer;
pub mod swd;
pub mod swj;

pub use general::*;
pub use xfer::*;
pub use swd::*;
pub use swj::*;

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
#[derive(Clone)]
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

    pub fn cmd(&self) -> DapCmdId { 
        DapCmdId::from_primitive(self.id())
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

    TransferConfigure = 0x04,
    Transfer     = 0x05,

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

