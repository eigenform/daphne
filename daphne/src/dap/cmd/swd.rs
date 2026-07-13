use anyhow::{Result, anyhow};
use num_enum::{IntoPrimitive, FromPrimitive, TryFromPrimitive};
use bitflags::bitflags;
use super::*;

/// SWD sequencing mode.
pub enum SwdSeqMode {
    /// Bits are being driven over SWDIO
    Output,
    /// Bits are being received over SWDIO
    Input,
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

