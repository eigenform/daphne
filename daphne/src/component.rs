use num_enum::*;

pub mod dbg;
pub mod cti;

pub use dbg::*;
pub use cti::*;

/// Marker trait for enums that map register names to register offsets.
pub trait ComponentReg: Sized + FromPrimitive<Primitive = u16> + From<u16> {}
impl ComponentReg for CtiRegister {}
impl ComponentReg for Armv8ExtDbgReg {}

/// Implemented on bitfield representations of some register. 
pub trait ComponentRegBits<T: ComponentReg> {
    /// The name of this register. 
    const REG: T;
}

/// Implemented on types representing debug components. 
pub trait Component { 
    /// The type of registers belonging to this component. 
    type Reg: ComponentReg;

    /// The 32-bit base address of this component on the debug bus. 
    fn base(&self) -> u32;

    /// Returns the address of a register belonging to this component. 
    fn addr_of(&self, reg: Self::Reg) -> u32 
        where u16: From<Self::Reg> 
    { 
        let x: u16 = u16::from(reg);
        self.base() + (x as u32)
    }
}


pub struct Armv8CoreBlock { 
    pub dbg: DebugBlock,
    pub cti: CtiBlock,
}

/// An Arm external debug component. 
pub struct DebugBlock { 
    pub base: u32,
}
impl Component for DebugBlock { 
    type Reg = Armv8ExtDbgReg;
    fn base(&self) -> u32 { 
        self.base
    }
}

/// An Arm cross-trigger interface (CTI) component. 
pub struct CtiBlock { 
    pub base: u32,
}
impl Component for CtiBlock { 
    type Reg = CtiRegister;
    fn base(&self) -> u32 { 
        self.base
    }
}



