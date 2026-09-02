//! Armv8-A External Debug
//!
//! See the [Arm ARM](https://developer.arm.com/documentation/ddi0487/mc) 
//! (document ARM DDI 0487, version M.c) for more information, specifically
//! chapter H8 ("About the External Debug Registers"). 
//!

use num_enum::*;
use modular_bitfield::prelude::*;
use super::*;

#[allow(non_camel_case_types)]
#[derive(FromPrimitive, IntoPrimitive)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum Armv8ExtDbgReg { 
    /// EDESR, External Debug Event Status Register
    ED_ESR      = 0x020,
    /// EDECR, External Debug Execution Control Register
    ED_ECR      = 0x024,

    /// EDWAR, External Debug Watchpoint Address Register
    ED_WAR_LO   = 0x030,
    ED_WAR_HI   = 0x034,

    /// Debug Data Transfer Register, Receive
    DBG_DTR_RX  = 0x080,

    /// EDITR, External Debug Instruction Transfer Register
    ED_ITR      = 0x084,
    /// EDSCR, External Debug Status and Control Register
    ED_SCR      = 0x088,

    /// Debug Data Transfer Register, Transmit
    DBG_DTR_TX  = 0x08c,

    /// EDRCR, External Debug Reserve Control Register
    ED_RCR      = 0x090,
    /// EDACR, External Debug Auxiliary Control Register
    ED_ACR      = 0x094,
    /// EDECCR, External Debug Exception Catch Control Register
    ED_ECCR     = 0x098,

    /// EDPCSR, External Debug Program Counter Sample Register
    ED_PCSR_LO  = 0x0a0,
    /// EDCIDSR, External Debug Context ID Sample Register
    ED_CIDSR    = 0x0a4,
    /// EDVIDSR, External Debug Virtual Context Sample Register
    ED_VIDSR    = 0x0a8,
    ED_PCSR_HI  = 0x0ac,

    /// OSLAR, OS Lock Access Register
    OSLAR       = 0x300,
    /// EDPRCR, External Debug Power/Reset Control Register
    ED_PRCR     = 0x310,
    /// EDPRSR, External Debug Processor Status Register
    ED_PRSR     = 0x314,

    /// MIDR, Main ID register
    MIDR        = 0xd00,

    /// External Debug Processor Feature Register 0
    ED_PFR_LO   = 0xd20,
    ED_PFR_HI   = 0xd24,
    /// External Debug Feature Register 0
    ED_DFR_LO   = 0xd28,
    ED_DFR_HI   = 0xd2c,


    /// External Debug AArch32 Processor Feature Register
    ED_AA32_PFR_LO = 0xd60,
    ED_AA32_PFR_HI = 0xd64,

    /// External Debug Integration mode Control register
    ED_IT_CTRL  = 0xf00,

    /// Debug CLAIM Tag Set Register
    DBG_CLAIM_SET = 0xfa0,
    /// Debug CLAIM Tag Clear Register
    DBG_CLAIM_CLR = 0xfa4,

    /// External Debug Device Affinity register 0
    ED_DEV_AFF0 = 0xfa8,
    ED_DEV_AFF1 = 0xfac,

    /// External Debug Lock Access Register
    ED_LAR      = 0xfb0,

    /// External Debug Lock Status Register
    ED_LSR      = 0xfb4,

    /// Debug Authentication Status Register
    DBG_AUTH_STATUS = 0xfb8,

    /// External Debug Device Architecture Register
    ED_DEV_ARCH = 0xfbc,

    /// External Debug Device ID register
    ED_DEV_ID2  = 0xfc0,
    ED_DEV_ID1  = 0xfc4,
    ED_DEV_ID   = 0xfc8,

    /// External Debug Device Type register
    ED_DEV_TYPE = 0xfcc,

    /// External Debug Peripheral Identification Register
    ED_PIDR4    = 0xfd0,
    ED_PIDR0    = 0xfe0,
    ED_PIDR1    = 0xfe4,
    ED_PIDR2    = 0xfe8,
    ED_PIDR3    = 0xfec,

    /// External Debug Component Identification Register
    ED_CIDR0    = 0xff0,
    ED_CIDR1    = 0xff4,
    ED_CIDR2    = 0xff8,
    ED_CIDR3    = 0xffc,

    #[num_enum(catch_all)]
    Undefined(u16),
}
impl Armv8ExtDbgReg { 
    pub fn as_u16(self) -> u16 { 
        self.into()
    }
}

/// External Debug Processor Status Register
#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Debug)]
pub struct EdPrsr { 
    pub pu: B1,
    pub spd: B1,
    pub r: B1,
    pub sr: B1,
    pub halted: B1,
    pub oslk: B1,
    pub dlk: B1,
    pub edad: B1,
    pub sdad: B1,
    pub epmad: B1,
    pub spmad: B1,
    pub sdr: B1,
    pub etad: B1,
    pub stad: B1,
    pub edade: B1,
    pub etade: B1,
    pub epmade: B1,
    pub res17: B15
}

#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Debug)]
pub struct EdRcr { 
    pub res0: B2,
    pub cse: B1, 
    pub cspa: B1, 
    pub cbrrq: B1, 
    pub res5: B27,
}


/// External Debug Status/Control Register
#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Debug)]
pub struct EdScr { 
    pub status_bits: B6,
    pub err: B1,
    pub a:   B1,
    pub el:  B2,
    pub rw:  B4,
    pub hde: B1,
    pub nse: B1,
    pub sdd: B1,
    pub res17: B1,
    pub ns:  B1,
    pub sc2: B1,
    pub ma:  B1,
    pub tda: B1,
    pub int_dis: B2,
    // "The PE is ready to accept an instruction to the ITR"
    pub ite: B1,
    pub pipe_adv: B1,
    pub txu: B1,
    pub rxo: B1,
    pub ito: B1,
    // "DBG_DTR_TX contains a value that has been written by software running
    // on the target and not-yet-read by the debugger"
    pub tx_full: B1,
    // "DBG_DTR_RX contains a value that has been written by the debugger
    // and not-yet-read by software running on the target"
    pub rx_full: B1,
    pub tfo: B1,
}
impl EdScr { 
    pub fn status(&self) -> ScrStatus { 
        ScrStatus::from(self.status_bits())
    }
}
impl ComponentRegBits<Armv8ExtDbgReg> for EdScr { 
    const REG: Armv8ExtDbgReg = Armv8ExtDbgReg::ED_SCR;
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[derive(FromPrimitive, IntoPrimitive)]
pub enum ScrStatus { 
    Restarting           = 0b000_001,
    NonDebug             = 0b000_010,
    Breakpoint           = 0b000_111,
    ExternalDbgReq       = 0b010_011,
    HaltingStepNormal    = 0b011_011,
    HaltingStepExclusive = 0b011_111,

    OsUnlockCatch        = 0b100_011,
    ResetCatch           = 0b100_111,
    Watchpoint           = 0b101_011,
    Hlt                  = 0b101_111,
    SwAccDbgReg          = 0b110_011,
    ExceptionCatch       = 0b110_111,
    HaltingStepNoSynd    = 0b111_011,

    #[num_enum(catch_all)]
    Undefined(u8),
}


#[cfg(test)]
mod test { 
    use super::*;
    #[test]
    fn ed_scr_smoke() { 
        let x = EdScr::from(0x0300_3c02);
        println!("{:x?}", x);
        let x = EdScr::from(0x0300_7c02);
        println!("{:x?}", x);
        let x = EdScr::from(0x0300_7f13);
        println!("{:x?}", x);
        let x = EdScr::from(0x2300_7f13);
        println!("{:x?}", x);
    }
}


