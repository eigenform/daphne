
use modular_bitfield::prelude::*;
use num_enum::*;

pub struct Aarch64Enc;
impl Aarch64Enc { 
    pub const DSB_SY: u32 = 0xd5033f9f;
    pub const ISB: u32    = 0xd5033fdf;
    pub const IC_IALLU: u32 = 0xd508751f;
    pub const HLT_0: u32 = 0xd4400000;
    pub const HLT_DEAD: u32 = 0xd45bd5a0;
}

pub enum Aarch64Reg { 
    Gpr(Gpr),
    Sp,
    Pc,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, FromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum ExceptionClass { 
    Unknown      = 0b000_000,
    TrapWfiWfe   = 0b000_001,
    TrapMcrMrc   = 0b000_011,
    TrapMcrrMrrc = 0b000_100,

    IllExecState = 0b001_110,

    DataAbort    = 0b100_101,
    SpAlignFault = 0b100_110,
    SErrorInt    = 0b101_111,

    #[num_enum(catch_all)]
    Undef(u8),
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, FromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum Gpr { 
     x0 = 00,
     x1 = 01,
     x2 = 02,
     x3 = 03,
     x4 = 04,
     x5 = 05,
     x6 = 06,
     x7 = 07,
     x8 = 08,
     x9 = 09,
    x10 = 10,
    x11 = 11,
    x12 = 12,
    x13 = 13,
    x14 = 14,
    x15 = 15,
    x16 = 16,
    x17 = 17,
    x18 = 18,
    x19 = 19,
    x20 = 20,
    x21 = 21,
    x22 = 22,
    x23 = 23,
    x24 = 24,
    x25 = 25,
    x26 = 26,
    x27 = 27,
    x28 = 28,
    x29 = 29,
    x30 = 30,

    #[num_enum(catch_all)]
    Undef(u8),
}


#[bitfield(bits = 16)]
#[repr(u16)]
#[derive(Clone, Copy, Debug)]
pub struct SysregEnc { 
    pub op2: B3,
    pub crm: B4,
    pub crn: B4,
    pub op1: B3,
    pub op0: B2,
}
impl SysregEnc { 
    pub const fn enc(op0: u8, op1: u8, crn: u8, crm: u8, op2: u8) -> u16 { 
        let mut x: u16 = 0;
        x |= ((op2 & 0b111) as u16);
        x |= ((crm & 0b1111) as u16) << 3;
        x |= ((crn & 0b1111) as u16) << 7;
        x |= ((op1 & 0b111) as u16) << 11;
        x |= ((op0 & 0b11) as u16) << 14;
        x
    }
    pub const fn from_enc(op0: u8, op1: u8, crn: u8, crm: u8, op2: u8) -> Self { 
        Self::from_bytes(Self::enc(op0, op1, crn, crm, op2).to_le_bytes())
    }

    pub fn as_sysreg(self) -> Sysreg { 
        let t: u16 = self.into();
        Sysreg::from(t)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, FromPrimitive, IntoPrimitive)]
#[repr(u16)]
pub enum Sysreg { 


    // 64-bit half-duplex channel. 
    //
    // When read (from the PE), this maps to DBG_DTR_TX.
    // When written (from the PE), this maps to DBG_DTR_RX.
    DBG_DTR_EL0     = SysregEnc::enc(0b10, 0b011, 0b0000, 0b0100, 0b000),

    // 32-bit full-duplex channel. 
    //
    // Note that DBG_DTR_RX and DBG_DTR_TX are the same register encoding, and 
    // only distinguished by whether or not the operation is a load or store. 
    //
    // DBG_DTR_TX is a transfer from the PE to the debugger.
    // DBG_DTR_RX is a transfer from the debugger to the PE.
    DBG_DTR_RXTX_EL0= SysregEnc::enc(0b10, 0b011, 0b0000, 0b0101, 0b000),

    // Debug feature registers
    ID_AA64DFR0_EL1 = SysregEnc::enc(0b11, 0b000, 0b0000, 0b0101, 0b000),
    ID_AA64DFR1_EL1 = SysregEnc::enc(0b11, 0b000, 0b0000, 0b0101, 0b001),

    DLR_EL0         = SysregEnc::enc(0b11, 0b011, 0b0100, 0b0101, 0b001),

    // PSTATE information is written here when entering the debug state
    DS_PSR_EL0      = SysregEnc::enc(0b11, 0b011, 0b0100, 0b0101, 0b000),


    SCTLR_EL3  = SysregEnc::enc(0b11, 0b110, 0b0001, 0b0000, 0b000),
    SCR_EL3    = SysregEnc::enc(0b11, 0b110, 0b0001, 0b0001, 0b000),

    TTBR0_EL3  = SysregEnc::enc(0b11, 0b110, 0b0010, 0b0000, 0b000),
    TCR_EL3    = SysregEnc::enc(0b11, 0b110, 0b0010, 0b0000, 0b010),

    ESR_EL3    = SysregEnc::enc(0b11, 0b110, 0b0101, 0b0010, 0b000),

    FAR_EL3    = SysregEnc::enc(0b11, 0b110, 0b0110, 0b0000, 0b000),

    MAIR_EL3   = SysregEnc::enc(0b11, 0b110, 0b1010, 0b0010, 0b000),

    VBAR_EL3   = SysregEnc::enc(0b11, 0b110, 0b1100, 0b0000, 0b000),
    RVBAR_EL3  = SysregEnc::enc(0b11, 0b110, 0b1100, 0b0000, 0b001),
    RMR_EL3    = SysregEnc::enc(0b11, 0b110, 0b1100, 0b0000, 0b010),


    IDATA0_EL3 = SysregEnc::enc(0b11, 0b110, 0b1111, 0b0000, 0b000),
    IDATA1_EL3 = SysregEnc::enc(0b11, 0b110, 0b1111, 0b0000, 0b001),
    IDATA2_EL3 = SysregEnc::enc(0b11, 0b110, 0b1111, 0b0000, 0b010),
    DDATA0_EL3 = SysregEnc::enc(0b11, 0b110, 0b1111, 0b0001, 0b000),
    DDATA1_EL3 = SysregEnc::enc(0b11, 0b110, 0b1111, 0b0001, 0b001),
    DDATA2_EL3 = SysregEnc::enc(0b11, 0b110, 0b1111, 0b0001, 0b010),

    // According to the Cortex-A76 TRM, you're supposed to perform writes 
    // with 'SYS #6, c15, c0, #0, xt'
    RAMINDEX   = SysregEnc::enc(0b01, 0b110, 0b1111, 0b0000, 0b000),


    #[num_enum(catch_all)]
    Undefined(u16),
}
impl Sysreg { 
    pub fn enc(self) -> SysregEnc { 
        let x: u16 = self.into();
        SysregEnc::from(x)
    }
}

#[bitfield(bits = 32)]
#[derive(Debug)]
#[repr(u32)]
pub struct SysregOp {
    pub rt: B5,
    pub op2: B3,
    pub crm: B4,
    pub crn: B4,
    pub op1: B3,
    pub op0: B2,
    pub l: B1,
    pub opcd: B10,
}
impl SysregOp { 
    const OPCD: u16 = 0b1101_0101_00;
    pub fn to_string(&self) -> String { 
        let enc = self.sysreg_enc();
        let sysreg = enc.as_sysreg();
        let sysreg_s = if matches!(sysreg, Sysreg::Undefined(_)) { 
            format!("S{}_{}_c{}_c{}_{}", enc.op0(), enc.op1(), enc.crn(), enc.crm(), enc.op2())
        } else { 
            format!("{:?}", sysreg)
        };


        if self.l() != 0 { 
            format!("mrs x{}, {}", self.rt(), sysreg_s)
        } else { 
            format!("msr {}, x{}", sysreg_s, self.rt())
        }
    }

    pub fn sysreg_enc(&self) -> SysregEnc { 
        SysregEnc::new()
            .with_op2(self.op2())
            .with_crm(self.crm())
            .with_crn(self.crn())
            .with_op1(self.op1())
            .with_op0(self.op0())
    }

    pub fn new_mrs(rt: u8, sysreg: SysregEnc) -> Self { 
        Self::new()
            .with_rt(rt)
            .with_op2(sysreg.op2())
            .with_crm(sysreg.crm())
            .with_crn(sysreg.crn())
            .with_op1(sysreg.op1())
            .with_op0(sysreg.op0())
            .with_l(1)
            .with_opcd(Self::OPCD)
    }
    pub fn new_msr(sysreg: SysregEnc, rt: u8) -> Self { 
        Self::new()
            .with_rt(rt)
            .with_op2(sysreg.op2())
            .with_crm(sysreg.crm())
            .with_crn(sysreg.crn())
            .with_op1(sysreg.op1())
            .with_op0(sysreg.op0())
            .with_l(0)
            .with_opcd(Self::OPCD)
    }

}


#[cfg(test)]
mod test { 
    use super::*;
    #[test]
    fn sysreg_enc_smoke() { 
        const ENCS: &[u32] = &[
            0xd53b_0000,
            0xd539_0020,

            0xd53b_4500,
            0xd53b_4520,
            0xd53e_1000,

            0xd513_0500,

            0xd513_0400,

            0xd50e_f400,

        ];
        for enc in ENCS { 
            let op = SysregOp::from(*enc);
            println!("{:08x} {} {:x?}", enc, op.to_string(), op);
        }
    }
}
