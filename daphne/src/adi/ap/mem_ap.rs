
use num_enum::*;
use modular_bitfield::prelude::*;
use crate::adi::ap::ApRegOff;
use serde::{Serialize, Deserialize};

pub struct MemApState { 
    pub tar_lo: u32,
}
impl MemApState { 
    pub fn new() -> Self { 
        Self { 
            tar_lo: 0,
        } 
    }
}

/// The name of a 32-bit MEM-AP register. 
#[derive(TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub enum MemApRegister { 
    CSW     = 0x00,
    TAR_LO  = 0x04,
    TAR_HI  = 0x08,
    DRW     = 0x0c,
    BD0     = 0x10,
    BD1     = 0x14,
    BD2     = 0x18,
    BD3     = 0x1c,
    MBT     = 0x20,
    T0TR    = 0x30,
    CFG1    = 0xe0,
    BASE_HI = 0xf0,
    CFG     = 0xf4,
    BASE_LO = 0xf8,
    IDR     = 0xfc,
}

#[repr(u8)]
#[derive(FromPrimitive)]
pub enum MemApSize { 
    Byte   = 0b000,
    Half   = 0b001,
    Word   = 0b010,
    Double = 0b011,

    #[num_enum(catch_all)]
    Reserved(u8),
}


#[repr(u8)]
#[derive(FromPrimitive)]
pub enum MemApAddrInc { 
    Disabled         = 0b00,
    IncrementSingle  = 0b01,
    IncrementPacked  = 0b10,

    #[num_enum(catch_all)]
    Reserved(u8),
}

#[repr(u8)]
#[derive(FromPrimitive)]
pub enum MemApMode { 
    Basic           = 0b0000,
    BarrierSupport  = 0b0001,

    #[num_enum(catch_all)]
    Reserved(u8),
}


/// MEM-AP Control/Status Word Register
#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Debug)]
pub struct MemApCsw {
    pub size: B3,
    pub res3: B1,
    pub addr_inc: B2,
    pub device_en: B1,
    pub tr_in_prog: B1,
    pub mode: B4,
    pub ty: B3,
    pub mte: B1,
    pub res16: B7,
    pub spiden: B1,
    pub prot: B7,
    pub dbg_sw_enable: B1,
}

#[cfg(test)]
mod test { 
    use super::*;
    #[test]
    fn bitfield_smoke() { 
        let x = MemApCsw::from(0xa200_0002);
        println!("{:#?}", x);
    }
}

