use anyhow::{Result, anyhow};
use num_enum::{IntoPrimitive, FromPrimitive, TryFromPrimitive};
use modular_bitfield::prelude::*;
use bitflags::bitflags;

use crate::dp::*;
use crate::ap::*;
use super::*;

/// A CMSIS-DAP transfer, representing some DP/AP read or write transaction.
#[derive(Clone)]
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
        Some(DpRegister::from_address(self.req.address(), dpbanksel))
    }

    /// Given some value of 'APBANKSEL' while this transfer is occurring, 
    /// return the offset of the AP register associated with this transfer
    /// [if any]. 
    pub fn resolve_ap_register_offset(&self, apbanksel: usize) 
        -> Option<ApRegOff> 
    { 
        if self.req.target() != TransferTarget::AP { 
            return None;
        }
        Some(ApRegOff::new(self.req.address(), apbanksel))
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

/// Representing the target of a CMSIS-DAP [`Transfer`]. 
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferTarget { DP, AP }



/// Transfer request [control] byte. 
#[derive(Clone, Copy)]
#[bitfield(bits = 8)]
#[repr(u8)]
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
    pub fn target(&self) -> TransferTarget { 
        if self.apndp() == 1 { 
            TransferTarget::AP
        } else { 
            TransferTarget::DP
        }
    }
    pub fn address(&self) -> usize { 
        (self.a() << 2) as _
    }
    pub fn kind(&self) -> Result<TransferKind> { 
        let read = self.rnw() != 0;
        let read_match = self.value_match() != 0;
        let write_mask = self.match_mask() != 0;

        match (read, read_match, write_mask) { 
            (true, true, false)   => Ok(TransferKind::ReadMatch),
            (true, false, false)  => Ok(TransferKind::Read),
            (false, false, true)  => Ok(TransferKind::WriteMatchMask),
            (false, false, false) => Ok(TransferKind::Write),
            (_, _, _) => { 
                Err(anyhow!("invalid TransferKind? {:02x}", 
                    u8::from(*self)
                ))
            }
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
        unimplemented!();
        //Ok(DapPacketBuf::new(Self::ID.into(),
        //))
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
    //pub tx_cnt: u8,
    pub xfers: Vec<Transfer>,
}
impl DapCommand for TransferCmd { 
    const ID: DapCmdId = DapCmdId::Transfer;
    type Resp = TransferRespUnresolved;
    fn to_packet(&self) -> Result<DapPacketBuf> {

        // FIXME: limits
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
    pub fn get_xfer(&self, idx: usize) -> Transfer {
        assert!(idx < self.xfers.len());
        self.xfers[idx].clone()
    }
}

/// Response to a [`TransferCmd`]. 
pub struct TransferRespUnresolved { 
    pkt: DapPacketBuf,
}
impl DapResponse for TransferRespUnresolved {
    fn from_packet(pkt: &DapPacketBuf) -> Result<Self> {
        Ok(Self { pkt: pkt.clone() })
    }
}
impl TransferRespUnresolved { 
    /// Use the given [`TransferCmd`] to "resolve" the structure of 
    /// this response. 
    pub fn resolve(&self, cmd: &TransferCmd) -> Result<TransferResp> { 
        let content = self.pkt.content();
        let rx_cnt = content[0];
        let ctl = TransferRespCtl::new(content[1]);

        let mut data = Vec::new();
        let mut cur = 2;
        for idx in 0..rx_cnt { 
            let xfer = cmd.get_xfer(idx as _);
            let kind = xfer.req.kind()?;
            match kind { 
                TransferKind::Read |
                TransferKind::ReadMatch => { 
                    let val = u32::from_le_bytes(content[cur..cur+4].try_into().unwrap());
                    data.push(Transfer { 
                        req: xfer.req,
                        data: Some(val),
                    });
                    cur += 4;
                },
                TransferKind::Write |
                TransferKind::WriteMatchMask => {
                    data.push(Transfer { 
                        req: xfer.req,
                        data: xfer.data,
                    });
                },
            }

        }

        Ok(TransferResp { ctl, data })
    }
}


pub struct TransferResp { 
    pub ctl: TransferRespCtl,
    pub data: Vec<Transfer>,
}

