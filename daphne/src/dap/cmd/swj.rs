use anyhow::{Result, anyhow};
use num_enum::{IntoPrimitive, FromPrimitive, TryFromPrimitive};
use bitflags::bitflags;
use super::*;

#[repr(transparent)]
pub struct SwjPinBits(pub u8);
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


