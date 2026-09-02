
use bitflags::bitflags;
use modular_bitfield::prelude::*;
use serde::{Serialize, Deserialize};
use num_enum::*;

use crate::dap::cmd::xfer::TransferWordIdx;

/// Debug port (DP) register name. 
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub enum DpRegister { 
    /// Offset 0x0
    DPIDR,
    /// Offset 0x4, DPBANKSEL=0x0
    CTRLSTAT,
    /// Offset 0x4, DPBANKSEL=0x1
    DLCR,
    /// Offset 0x4, DPBANKSEL=0x2
    TARGETID,
    /// Offset 0x4, DPBANKSEL=0x3
    DLPIDR,
    /// Offset 0x4, DPBANKSEL=0x4
    EVENTSTAT,
    /// Offset 0x8
    SELECT,
    /// Offset 0xc
    RDBUFF,

    Undef,
}
impl DpRegister { 
    /// Return the DPBANKSEL value used to access this register.
    pub fn dpbanksel(&self) -> Option<u8> { 
        match self { 
            Self::DPIDR     => None,
            Self::CTRLSTAT  => Some(0),
            Self::DLCR      => Some(1),
            Self::TARGETID  => Some(2),
            Self::DLPIDR    => Some(3),
            Self::EVENTSTAT => Some(4),
            Self::SELECT    => None,
            Self::RDBUFF    => None,
            Self::Undef     => None,
        }
    }

    /// Return the word index (`A[3:2]`) used to access this register.
    pub fn word_idx(&self) -> TransferWordIdx { 
        match self { 
            Self::DPIDR     => TransferWordIdx::new_from_offset(0x00),
            Self::CTRLSTAT  => TransferWordIdx::new_from_offset(0x04),
            Self::DLCR      => TransferWordIdx::new_from_offset(0x04),
            Self::TARGETID  => TransferWordIdx::new_from_offset(0x04),
            Self::DLPIDR    => TransferWordIdx::new_from_offset(0x04),
            Self::EVENTSTAT => TransferWordIdx::new_from_offset(0x04),
            Self::SELECT    => TransferWordIdx::new_from_offset(0x08),
            Self::RDBUFF    => TransferWordIdx::new_from_offset(0x0c),
            Self::Undef     => unreachable!(),
        }
    }

    pub fn from_address(word_idx: TransferWordIdx, dpbanksel: usize) -> Self { 
        match (word_idx.bits(), dpbanksel) { 
            (0b00, _)   => Self::DPIDR,
            (0b01, 0x0) => Self::CTRLSTAT,
            (0b01, 0x1) => Self::DLCR,
            (0b01, 0x2) => Self::TARGETID,
            (0b01, 0x3) => Self::DLPIDR,
            (0b01, 0x4) => Self::EVENTSTAT,
            (0b01, _)   => Self::Undef,
            (0b10, _)   => Self::SELECT,
            (0b11, _)   => Self::RDBUFF,
            (_, _)      => Self::Undef,
        }
    }
}


#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Debug)]
pub struct DpCtrlStat {
    pub orun_detect: B1,
    pub sticky_orun: B1,
    pub trn_mode: B2,
    pub sticky_cmp: B1,
    pub sticky_err: B1,
    pub read_ok: B1,
    pub wdata_err: B1,
    pub masklane: B4,
    pub trncnt: B12,
    pub res24: B2,
    pub cdbg_rst_req: B1,
    pub cdbg_rst_ack: B1,
    pub cdbg_pwrup_req: B1,
    pub cdbg_pwrup_ack: B1,
    pub csys_pwrup_req: B1,
    pub csys_pwrup_ack: B1,
}

/// Representing values for CTRLSTAT.TRNMODE
#[derive(Clone, Copy, Debug, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
// The response only contains the results of reads
#[repr(u8)]
pub enum DpApTransferMode { 
    Normal        = 0b00,
    PushedVerify  = 0b01,
    PushedCompare = 0b10,
    Reserved      = 0b11,
}

#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Debug)]
pub struct DpAbort { 
    pub dap_abort: B1,
    pub stk_cmp_clr: B1,
    pub stk_err_clr: B1,
    pub wd_err_clr: B1,
    pub orun_err_clr: B1,
    pub res5: B27,
}

#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Debug)]
pub struct DpDlcr {
    pub res0: B6,
    pub res6: B1,
    pub res7: B1,
    pub turnaround: B2,
    pub res10: B22,
}

#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Debug)]
pub struct DpDlpIdr {
    pub protsvc: B4,
    pub res4: B24,
    pub tinstance: B4,
}

#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Debug)]
pub struct DpIdr {
    pub rao: B1,
    pub designer: B11,
    pub version: B4,
    pub min: B1,
    pub res17: B3,
    pub partno: B8,
    pub revision: B4,
}


#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Debug)]
pub struct DpSelect {
    pub dpbanksel: B4,
    pub apbanksel: B4,
    pub res16: B16,
    pub apsel: B8,
}


/// Container for tracking the state of DP control registers. 
pub struct DpState { 
    pub ctrlstat: DpCtrlStat,
    pub select: DpSelect,
}
impl DpState { 
    pub fn new() -> Self { 
        Self { 
            ctrlstat: DpCtrlStat::from(0),
            select: DpSelect::from(0),
        }
    }
}

#[cfg(test)]
mod test { 
    use super::*;
    #[test]
    fn bitfield_smoke() { 
        let x = DpCtrlStat::from(0x50000022);
        assert!(x.sticky_orun() == 1);
        assert!(x.sticky_err() == 1);
        assert!(x.cdbg_pwrup_req() == 1);
        assert!(x.csys_pwrup_req() == 1);

        let x: u32 = DpCtrlStat::new()
            .with_sticky_orun(1)
            .with_sticky_err(1)
            .with_cdbg_pwrup_req(1)
            .with_csys_pwrup_req(1)
            .into();
        assert!(x == 0x50000022);

    }
}





