
use num_enum::*;
use crate::component::*;
use crate::dap::*;
use crate::adi::*;
use std::time::Duration;
use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use postcard;
use std::io::{Read, Write};


pub mod server;
pub mod client;

pub use server::*;
pub use client::*;

/// A request packet (from client to server).
#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub struct DaphnePacket { 
    pub msg: DaphneOp,
}

/// A response packet (from server to client).
#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub struct DaphneResp { 
    pub data: u32,
    pub sts: DaphneRespSts
}

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub enum DaphneRespSts {
    Ok,
    Err,
}
//impl From<DaphneServerErr> for DaphneRespErr { 
//    fn from(e: DaphneServerErr) -> Self { 
//        match e { 
//            DaphneServerErr::
//        }
//    }
//}

impl std::error::Error for DaphneRespSts {}
impl std::fmt::Display for DaphneRespSts { 
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { 
        write!(f, "{:?}", self)
    }
}


/// Interface exposed by a `daphne` server. 
#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub enum DaphneOp { 
    Ping,

    /// Set a session flag
    SetFlag(String, bool),
    /// Get a session flag
    GetFlag(String),
    
    /// CMSIS-DAP command
    Dap(DapOp),

    /// DP read/write
    Dp(DpOp),

    /// MEM-AP read/write
    MemAp(MemApOp),

    /// MEM-AP bus read/write
    MemApBus(MemApBusOp),
}



/// Primitive CMSIS-DAP operations.
#[derive(Clone, Copy, Debug)]
#[derive(Serialize, Deserialize)]
pub enum DapOp { 
    /// Connect to the target DAP
    Connect,
    /// Disconnect from the target DAP
    Disconnect,
}

/// Primitive DP operations. 
///
/// NOTE: These do not write DPBANKSEL when targeting unbanked registers.
/// ADIv5 does not *explicitly* say this, but it's implied that the bank 
/// is simply *ignored* when performing accesses on words other than 0x4. 
///
#[derive(Clone, Copy, Debug)]
#[derive(Serialize, Deserialize)]
pub enum DpOp { 
    /// Read a DP register
    Read { reg: DpRegister },
    /// Write a DP register
    Write { reg: DpRegister, data: u32 },
}
impl TransferSequence for DpOp { 
    fn as_transfer_seq(&self) -> Vec<Transfer> { 
        let mut res = Vec::new();
        match self { 
            Self::Read { reg } =>  { 
                if let Some(dpbanksel) = reg.dpbanksel() { 
                    res.push(Transfer::write_dpselect(
                DpSelect::new()
                            .with_dpbanksel(dpbanksel)
                    ));
                    res.push(Transfer::dp_read(*reg));
                } else { 
                    res.push(Transfer::dp_read(*reg));
                }
            },
            Self::Write { reg, data } => {
                if let Some(dpbanksel) = reg.dpbanksel() { 
                    res.push(Transfer::write_dpselect(
                DpSelect::new()
                            .with_dpbanksel(dpbanksel)
                    ));
                    res.push(Transfer::dp_write(*reg, *data));
                } else { 
                    res.push(Transfer::dp_write(*reg, *data));
                }
            },
        }
        res
    }
}

///// An abstract MEM-AP operation. 
#[derive(Clone, Copy, Debug)]
#[derive(Serialize, Deserialize)]
pub enum MemApOp { 
    /// Read from a MEM-AP register
    Read { ap: u8, reg: MemApRegister },
    /// Write to a MEM-AP register
    Write { ap: u8, reg: MemApRegister, data: u32 },
}
impl TransferSequence for MemApOp { 
    fn as_transfer_seq(&self) -> Vec<Transfer> { 
        let mut res = Vec::new();
        match self { 
            Self::Read { ap, reg } => {
                let offset = reg.ap_reg_off();
                // DP write (set the value of SELECT.APBANKSEL)
                res.push(Transfer::write_dpselect(
                    DpSelect::new()
                    .with_apsel(*ap)
                    .with_dpbanksel(0)
                    .with_apbanksel(offset.apbanksel())
                ));
                // AP read
                res.push(Transfer::mem_ap_read(offset.word_idx()));
            },

            Self::Write { ap, reg, data } => { 
                let offset = reg.ap_reg_off();
                // DP write (set the value of SELECT.APBANKSEL)
                res.push(Transfer::write_dpselect(
                    DpSelect::new()
                    .with_apsel(*ap)
                    .with_dpbanksel(0)
                    .with_apbanksel(offset.apbanksel())
                ));
                // AP write
                res.push(Transfer::mem_ap_write(offset.word_idx(), *data));
            },
        }
        res
    }
}

/// Perform an operation on a MEM-AP bus. 
#[derive(Clone, Copy, Debug)]
#[derive(Serialize, Deserialize)]
pub enum MemApBusOp {
    Read { ap: u8, addr: u32 },
    Write { ap: u8, addr: u32, data: u32 },
}
impl TransferSequence for MemApBusOp { 
    fn as_transfer_seq(&self) -> Vec<Transfer> { 
        let mut res = Vec::new();
        match self { 
            Self::Read { ap, addr } => {
                res.extend_from_slice(&MemApOp::Write { 
                    ap: *ap, reg: MemApRegister::TAR_LO, data: *addr
                }.as_transfer_seq());
                res.extend_from_slice(&MemApOp::Read { 
                    ap: *ap, reg: MemApRegister::DRW,
                }.as_transfer_seq());
            },
            Self::Write { ap, addr, data } => {
                res.extend_from_slice(&MemApOp::Write { 
                    ap: *ap, reg: MemApRegister::TAR_LO, data: *addr
                }.as_transfer_seq());
                res.extend_from_slice(&MemApOp::Write { 
                    ap: *ap, reg: MemApRegister::DRW, data: *data
                }.as_transfer_seq());
            },
        }
        res
    }
}

/// TODO: Abstract over MEM-AP ops
pub enum DbgOp { 
    Read { blk: DebugBlock, reg: Armv8ExtDbgReg },
    Write { blk: DebugBlock, reg: Armv8ExtDbgReg, data: u32 },
}
pub enum CtiOp { 
    Read { blk: CtiBlock, reg: CtiRegister },
    Write { blk: CtiBlock, reg: CtiRegister, data: u32 },
}


#[cfg(test)]
mod test { 
    use super::*;
    #[test]
    fn foo() { 
        let x = MemApBusOp::Read { ap: 0, addr: 0x80010000 }.as_transfer_seq();
        println!("{:#x?}", x);
    }
}



