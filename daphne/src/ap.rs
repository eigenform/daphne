
use num_enum::*;
use modular_bitfield::prelude::*;

pub mod mem_ap;
pub use mem_ap::*;


/// 8-bit offset of an AP register. 
pub struct ApRegOff(u8);
impl ApRegOff { 
    /// Create a new [`ApRegOff`] from the `A[3:2]` bits and some value of 
    /// `DP.SELECT.APBANKSEL`. 
    pub fn new(a: usize, apbanksel: usize) -> Self { 
        let a = (a & 0x0f) as u8;
        let apbanksel = (apbanksel & 0x0f) as u8;
        Self(apbanksel << 4 | a)
    }
    pub fn value(&self) -> usize { self.0 as _ }
}


/// AP Identification Register
#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Debug)]
pub struct ApIdr {
    pub ty: B4,
    pub variant: B4,
    pub res8: B5,
    pub class: B4,
    pub designer: B11,
    pub revision: B4,
}
impl ApIdr { 
    pub fn ap_class(&self) -> ApClass { 
        ApClass::from(self.class())
    }
    pub fn ap_type(&self) -> ApType { 
        ApType::from_type_class(self.ty(), self.ap_class())
    }
    pub fn jep106(&self) -> u8 { 
        (self.designer() & 0b000_0111_1111) as _
    }

}

/// ADIv5 AP class. 
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, FromPrimitive)]
pub enum ApClass { 
    Undefined = 0b0000,
    ComAp     = 0b0001,
    MemAp     = 0b1000,

    #[num_enum(catch_all)]
    Reserved(u8),
}


/// Bus/connection associated with an AP. 
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApType { 
    Jtag,
    ComAp,
    Ahb3,
    Apb2_Apb3,
    Axi3_Axi4,
    Ahb5,
    Apb4_Apb5,
    Axi5,
    Axi5Enh,

    Unknown(u8, ApClass),
}
impl ApType { 
    pub fn from_type_class(ty: u8, class: ApClass) -> Self { 
        match (ty, class) { 
            (0x0, ApClass::Undefined) => Self::Jtag,
            (0x0, ApClass::ComAp) => Self::ComAp,

            (0x1, ApClass::MemAp) => Self::Ahb3,
            (0x2, ApClass::MemAp) => Self::Apb2_Apb3,
            (0x4, ApClass::MemAp) => Self::Axi3_Axi4,
            (0x5, ApClass::MemAp) => Self::Ahb5,
            (0x6, ApClass::MemAp) => Self::Apb4_Apb5,
            (0x7, ApClass::MemAp) => Self::Axi5,
            (0x8, ApClass::MemAp) => Self::Axi5Enh,

            (_, _) => Self::Unknown(ty, class),
        }
    }
}


#[cfg(test)]
mod test { 
    use super::*;
    #[test]
    fn apidr_smoke() { 
        let x = ApIdr::from(0x24770002);
        assert!(x.ap_class() == ApClass::MemAp);
        assert!(x.ap_type() == ApType::Apb2_Apb3);
        assert!(x.jep106() == 0x3b);
    }
}

