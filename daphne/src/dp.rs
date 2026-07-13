
use bitflags::bitflags;
use modular_bitfield::prelude::*;


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    pub fn from_address(addr: usize, dpbanksel: usize) -> Self { 
        match (addr, dpbanksel) { 
            (0x0, _)   => Self::DPIDR,

            (0x4, 0x0) => Self::CTRLSTAT,
            (0x4, 0x1) => Self::DLCR,
            (0x4, 0x2) => Self::TARGETID,
            (0x4, 0x3) => Self::DLPIDR,
            (0x4, 0x4) => Self::EVENTSTAT,
            (0x4, _)   => Self::Undef,

            (0x8, _)   => Self::SELECT,
            (0xc, _)   => Self::RDBUFF,

            (_, _)     => Self::Undef,
        }
    }

}

#[bitfield(bits = 32)]
#[repr(u32)]
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

#[bitfield(bits = 32)]
#[repr(u32)]
pub struct DpDlcr {
    pub res0: B6,
    pub res6: B1,
    pub res7: B1,
    pub turnaround: B2,
    pub res10: B22,
}

#[bitfield(bits = 32)]
#[repr(u32)]
pub struct DpDlpIdr {
    pub protsvc: B4,
    pub res4: B24,
    pub tinstance: B4,
}

#[bitfield(bits = 32)]
#[repr(u32)]
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
