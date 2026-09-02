use anyhow::{Result, anyhow};
use num_enum::*;
use modular_bitfield::prelude::*;
use bitflags::bitflags;
use serde::{Serialize, Deserialize};

use crate::adi::*;
use super::*;

/// Implemented on types that can be translated into one or more DAP transfers.
pub trait TransferSequence {
    fn as_transfer_seq(&self) -> Vec<Transfer>;
}


#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum XferErr { 
    ParseErr,
    ProtocolErr,
    ValueMismatch,
    Ack(TransferAck),
}
impl std::error::Error for XferErr {}
impl std::fmt::Display for XferErr { 
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { 
        write!(f, "{:?}", self)
    }
}



/// A CMSIS-DAP transfer, representing some DP/AP read or write transaction.
#[derive(Clone, Debug)]
pub struct Transfer { 
    pub req: TransferReqCtl,
    pub data: Option<u32>,
}
impl Transfer { 
    pub fn new(req: TransferReqCtl) -> Self { 
        Self { req, data: None }
    }
    pub fn new_with_data(req: TransferReqCtl, val: u32) -> Self { 
        Self { req, data: Some(val) }
    }

    /// Create a write to some [`DpRegister`].
    ///
    /// NOTE: This is a single transfer, and only encodes the word index 
    /// associated with the target DP register. This assumes that 
    /// DPBANKSEL has been set to the appropriate value for the target 
    /// DP register.
    pub fn dp_write(reg: DpRegister, data: u32) -> Self { 
        let req = TransferReqCtl::new_dp_write(reg.word_idx());
        Self { req, data: Some(data) }
    }

    /// Create a read to some [`DpRegister`].
    ///
    /// NOTE: This is a single transfer, and only encodes the word index 
    /// associated with the target DP register. This assumes that 
    /// DPBANKSEL has been set to the appropriate value for the target 
    /// DP register.
    pub fn dp_read(reg: DpRegister) -> Self { 
        let req = TransferReqCtl::new_dp_read(reg.word_idx());
        Self { req, data: None }
    }

    /// Create a read to some MEM-AP register. 
    ///
    /// NOTE: This is a single transfer, and only encodes the word index 
    /// associated with the target MEM-AP register. This assumes that 
    /// APBANKSEL has been set to the appropriate value for the target 
    /// MEM-AP register. 
    pub fn mem_ap_read(word_idx: TransferWordIdx) -> Self { 
        let req = TransferReqCtl::new_ap_read(word_idx);
        Self { req, data: None }
    }

    /// Create a write to some MEM-AP register.
    ///
    /// NOTE: This is a single transfer, and only encodes the word index 
    /// associated with the target MEM-AP register. This assumes that 
    /// APBANKSEL has been set to the appropriate value for the target 
    /// MEM-AP register. 
    pub fn mem_ap_write(word_idx: TransferWordIdx, data: u32) -> Self { 
        let req = TransferReqCtl::new_ap_write(word_idx);
        Self { req, data: Some(data) }
    }

    /// Create a write to the DP ABORT register.
    pub fn write_abort(abort: DpAbort) -> Self { 
        Self::dp_write(DpRegister::DPIDR, abort.into())
    }

    /// Create a write to the DP SELECT register.
    pub fn write_dpselect(select: DpSelect) -> Self { 
        Self::dp_write(DpRegister::SELECT, select.into())
    }


}

impl Transfer { 
    /// The size of this transfer (in bytes).
    pub fn size(&self) -> usize { 
        if self.data.is_none() { 1 } else { 5 }
    }


    /// Given some value of 'DPBANKSEL' while this transfer is occurring, 
    /// return the [`DpRegister`] associated with this transfer [if any].
    pub fn resolve_dp_register(&self, dpbanksel: usize) -> Option<DpRegister> { 
        if self.req.target() != TransferTarget::DP { 
            return None;
        }
        Some(DpRegister::from_address(self.req.word_idx(), dpbanksel))
    }

    /// Given some value of 'APBANKSEL' while this transfer is occurring, 
    /// return the offset of the AP register associated with this transfer
    /// [if any]. 
    pub fn resolve_ap_register_offset(&self, apbanksel: u8) 
        -> Option<ApRegOff> 
    { 
        if self.req.target() != TransferTarget::AP { 
            return None;
        }
        Some(ApRegOff::from_word_bank(self.req.word_idx(), apbanksel))
    }

}



/// Representing different types of CMSIS-DAP [`Transfer`]s. 
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferKind { 
    Read,
    ReadMatch,
    Write, 
    WriteMatchMask,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TransferAccessKind { 
    W = 0,
    R = 1,
}

/// Representing the target of a CMSIS-DAP [`Transfer`]. 
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TransferTarget { 
    DP = 0,
    AP = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct TransferWordIdx(u8);
impl TransferWordIdx { 
    pub fn new_idx(val: usize) -> Self { 
        Self((val & 0b11) as _)
    }
    pub fn new_from_offset(val: u8) -> Self { 
        Self((val & 0b0000_1100) >> 2)
    }
    pub fn bits(&self) -> u8 { 
        self.0
    }
}


/// Transfer request [control] byte. 
#[bitfield(bits = 8)]
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub struct TransferReqCtl {
    pub apndp: B1,
    pub rnw: B1,
    pub a: B2,
    pub value_match: B1,
    pub match_mask: B1,
    pub unk6: B1,
    pub td_timestamp: B1,
}
impl TransferReqCtl { 
    pub fn create(
        target: TransferTarget, 
        access: TransferAccessKind, 
        addr: TransferWordIdx,
    ) -> Self { 
        Self::new()
            .with_apndp(target as _)
            .with_rnw(access as _)
            .with_a(addr.bits())
    }

    pub fn new_dp_read(idx: TransferWordIdx) -> Self { 
        Self::create(
            TransferTarget::DP,
            TransferAccessKind::R,
            idx
        )
    }
    pub fn new_dp_write(idx: TransferWordIdx) -> Self { 
        Self::create(
            TransferTarget::DP,
            TransferAccessKind::W,
            idx
        )
    }
    pub fn new_ap_read(idx: TransferWordIdx) -> Self { 
        Self::create(
            TransferTarget::AP,
            TransferAccessKind::R,
            idx
        )
    }
    pub fn new_ap_write(idx: TransferWordIdx) -> Self { 
        Self::create(
            TransferTarget::AP,
            TransferAccessKind::W,
            idx
        )
    }

    /// Return the word index (`A[3:2]`) associated with this transfer.
    pub fn word_idx(&self) -> TransferWordIdx { 
        TransferWordIdx::new_idx(self.a() as _)
    }

    /// Return the associated [`TransferTarget`]
    pub fn target(&self) -> TransferTarget { 
        if self.apndp() == 1 { 
            TransferTarget::AP
        } else { 
            TransferTarget::DP
        }
    }

    /// Return the associated [`TransferKind`]
    pub fn kind(&self) -> Option<TransferKind> { 
        let read = self.rnw() != 0;
        let read_match = self.value_match() != 0;
        let write_mask = self.match_mask() != 0;

        match (read, read_match, write_mask) { 
            (true, true, false)   => Some(TransferKind::ReadMatch),
            (true, false, false)  => Some(TransferKind::Read),
            (false, false, true)  => Some(TransferKind::WriteMatchMask),
            (false, false, false) => Some(TransferKind::Write),
            (_, _, _) => None,
        }
    }

}

/// Transfer response [control] byte. 
#[repr(transparent)]
pub struct TransferRespCtl(u8);
impl TransferRespCtl { 
    pub fn new(val: u8) -> Self {
        Self(val)
    }
    pub fn ack(&self) -> TransferAck { 
        TransferAck::from_primitive(self.0 & 0b0000_0111)
    }
    pub fn protocol_err(&self) -> bool { 
        (self.0 & 0b0000_1000) != 0
    }
    pub fn value_mismatch(&self) -> bool { 
        (self.0 & 0b0001_0000) != 0
    }
}

/// Transfer acknowledgement bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, FromPrimitive)]
#[derive(Serialize, Deserialize)]
#[repr(u8)]
pub enum TransferAck { 
    Ok     = 0b001,
    Wait   = 0b010,
    Fault  = 0b100,
    NoAck  = 0b111,

    #[num_enum(catch_all)]
    Undef(u8),
}


pub struct TransferConfigureCmd { 
    /// "Number of extra idle cycles after each transfer"
    pub idle_cycles: u8,
    /// "Number of transfer retries after WAIT response"
    pub wait_retry: u16,
    /// "Number of retries on reads with Value Match"
    pub match_retry: u16,
}
impl DapCommand for TransferConfigureCmd { 
    const ID: DapCmdId = DapCmdId::TransferConfigure;
    type Resp = TransferConfigureResp;
    fn to_packet(&self) -> Result<DapPacketBuf> {
        let wait_retry = self.wait_retry.to_le_bytes();
        let match_retry = self.match_retry.to_le_bytes();
        Ok(DapPacketBuf::new(Self::ID.into(),
            &[ 
                &[self.idle_cycles], 
                wait_retry.as_slice(), 
                match_retry.as_slice() 
            ].concat()
        ))
    }
}
pub struct TransferConfigureResp { 
    pub sts: DapResponseStatus
}
impl DapResponse for TransferConfigureResp {
    fn from_packet(pkt: &DapPacketBuf) -> Result<Self> {
        assert!(pkt.content().len() == 1);
        let sts = DapResponseStatus::try_from_primitive(
            pkt.content()[0]
        )?;
        Ok(Self { sts })
    }
}

#[derive(Clone)]
pub struct TransferCmd { 
    pub dap_idx: u8,
    pub xfers: Vec<Transfer>,
}
impl DapCommand for TransferCmd { 
    const ID: DapCmdId = DapCmdId::Transfer;
    type Resp = TransferRespUnresolved;
    fn to_packet(&self) -> Result<DapPacketBuf> {
        if self.xfers.len() == 0 { 
            return Err(anyhow!("cannot serialize empty TransferCmd"));
        }

        let total_sz: usize = self.xfers.iter().map(|x| x.size()).sum();
        if total_sz + 2 > DapPacketBuf::MAX_PKT_SZ { 
            return Err(anyhow!("TransferCmd would exceed MAX_PKT_SZ (64)"));
        }

        let tx_cnt = self.xfers.len() as u8;
        let hdr = [ self.dap_idx, tx_cnt ];

        let mut buf: Vec<u8> = vec![];
        for xfer in &self.xfers { 
            buf.push(xfer.req.into());
            if let Some(val) = xfer.data { 
                buf.extend_from_slice(val.to_le_bytes().as_slice());
            }
        }
        let content = &[ &hdr, buf.as_slice() ].concat();
        Ok(DapPacketBuf::new(Self::ID.into(), content))
    }
}
impl TransferCmd { 
    pub fn new_from_slice(dap_idx: u8, slice: &[Transfer]) -> Result<Self> { 
        let total_sz: usize = slice.iter().map(|x| x.size()).sum();
        if total_sz + 2 > DapPacketBuf::MAX_PKT_SZ { 
            return Err(anyhow!("TransferCmd would exceed MAX_PKT_SZ (64)"));
        }
        let xfers = slice.to_vec();
        Ok(Self { dap_idx, xfers })
    }
    pub fn xfer_cnt(&self) -> u8 { 
        self.xfers.len() as _
    }
    pub fn get_xfer(&self, idx: usize) -> Transfer {
        assert!(idx < self.xfers.len());
        self.xfers[idx].clone()
    }
}

/// Response to a [`TransferCmd`]. 
pub struct TransferRespUnresolved { 
    pub pkt: DapPacketBuf,
}
impl DapResponse for TransferRespUnresolved {
    fn from_packet(pkt: &DapPacketBuf) -> Result<Self> {
        Ok(Self { pkt: pkt.clone() })
    }
}
impl TransferRespUnresolved { 
    pub fn xfer_cnt(&self) -> u8 { 
        self.pkt.content()[0]
    }
    pub fn xfer_respctl(&self) -> TransferRespCtl { 
        TransferRespCtl::new(self.pkt.content()[1])
    }
    pub fn xfer_data(&self) -> &[u8] {
        &self.pkt.content()[2..]
    }

    /// Use the given [`TransferCmd`] to "resolve" this response. 
    pub fn resolve(&self, cmd: &TransferCmd) -> Result<TransferResp, XferErr> { 
        if self.xfer_cnt() != cmd.xfer_cnt() { 
            return Err(XferErr::ParseErr);
        }

        let xfer_cnt = self.xfer_cnt();
        let ctl = self.xfer_respctl();
        let xfer_data = self.xfer_data();

        if self.xfer_data().len() % 4 != 0 { 
            return Err(XferErr::ParseErr);
        }

        // The response only contains the results of reads. 
        // Just clone the transfers from the command, and then fill in 
        // the results for read transactions. 
        let mut data = cmd.xfers.clone();
        let mut cur = 0;
        for idx in 0..xfer_cnt as usize { 
            let kind = data[idx].req.kind();
            match kind { 
                Some(TransferKind::Write) | 
                Some(TransferKind::WriteMatchMask) => {},
                Some(TransferKind::Read) | 
                Some(TransferKind::ReadMatch) => { 
                    let val = u32::from_le_bytes(
                        xfer_data[cur..cur + 4].try_into().unwrap()
                    );
                    data[idx].data = Some(val);
                    cur += 4;
                },
                None => { 
                    return Err(XferErr::ParseErr);
                },
            }
        }

        Ok(TransferResp { ctl, data })
    }
}


/// A response corresponding to some [`TransferCmd`]. 
pub struct TransferResp { 
    pub ctl: TransferRespCtl,
    pub data: Vec<Transfer>,
}
impl TransferResp { 
    pub fn last_result(&self) -> Option<u32> { 
        self.data.last().unwrap().data
    }
}

pub struct SequenceBuilder { 
    xfers: Vec<Transfer>,
}
impl SequenceBuilder { 
    pub fn new() -> Self { 
        Self { xfers: Vec::new() }
    }
    pub fn add(mut self, op: impl TransferSequence) -> Self { 
        self.xfers.extend_from_slice(&op.as_transfer_seq());
        self
    }
    pub fn finish(self) -> Vec<Transfer> { 
        self.xfers
    }
}

