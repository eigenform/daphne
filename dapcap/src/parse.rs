
use crate::capture::*;
use std::collections::*;
use daphne::prelude::*;

#[derive(Debug)]
pub enum Component { 
    ExtDbg,
    Cti,
    Pmu,
    Etm,
    Unk(u32),
}
pub struct MemoryMap;
impl MemoryMap { 
    pub fn resolve_component(addr: u32) -> Component { 
        match addr & 0xffff_0000 { 
            0x80010000 => Component::ExtDbg,
            0x80020000 => Component::Cti,
            0x80030000 => Component::Pmu,
            0x80040000 => Component::Etm,

            0x80110000 => Component::ExtDbg,
            0x80120000 => Component::Cti,
            0x80130000 => Component::Pmu,
            0x80140000 => Component::Etm,

            0x80210000 => Component::ExtDbg,
            0x80220000 => Component::Cti,
            0x80230000 => Component::Pmu,
            0x80240000 => Component::Etm,

            0x80310000 => Component::ExtDbg,
            0x80320000 => Component::Cti,
            0x80330000 => Component::Pmu,
            0x80340000 => Component::Etm,
            _ => Component::Unk(addr),
        }
    }
}


/// State used for parsing transactions. 
pub struct State { 
    /// Queue of pending transfers
    pub txq: XferQueue,

    pub dp: DpState,

    pub memap: MemApState,
}
impl State { 
    pub fn new() -> Self { 
        Self { 
            txq: XferQueue::new(),
            dp: DpState::new(),
            memap: MemApState::new(),
        }
    }

    pub fn parse_packet(&mut self, urb: &LinuxUrbData, pkt: &DapPacketBuf) 
        -> Result<()> 
    {
        let dir = urb.dir();
        let cmd = pkt.cmd();
        let content = pkt.content();

        println!("[{:?}] DapCmdId::{:?}", dir, cmd);

        match (cmd, dir) { 
            (DapCmdId::Transfer, Dir::Tx) => {
                self.handle_xfer_tx(pkt)?;
            },
            (DapCmdId::Transfer, Dir::Rx) => {
                self.handle_xfer_rx(pkt)?;
            },
            (_, _) => { 
                println!("  content={:02x?}", content);
            },
        }

        if dir == Dir::Rx { println!(); }
        Ok(())
    }

}

/// These are for parsing types implementing [`DapCommand`]. 
impl State { 
    fn parse_transfercmd(pkt: &DapPacketBuf) -> Result<TransferCmd> { 
        let content = pkt.content();
        let dap_idx = content[0];
        let tx_cnt = content[1];
        let mut cur = 2;

        let mut xfers = Vec::new();
        for _ in 0..tx_cnt { 
            let req = TransferReqCtl::from(content[cur]);
            let kind = req.kind().ok_or(anyhow!("uhhh"))?;
            let next_cur = match kind {
                TransferKind::Read | 
                TransferKind::ReadMatch => {
                    xfers.push(Transfer::new(req));
                    cur+1
                },
                TransferKind::Write | 
                TransferKind::WriteMatchMask => {
                    let val = u32::from_le_bytes(
                        content[cur+1..cur+1+4].try_into().unwrap()
                    );
                    xfers.push(Transfer::new_with_data(req, val));
                    cur + 5
                },
            };
            cur = next_cur;
        }

        Ok(TransferCmd { dap_idx, xfers })
    }
}

/// These are the top-level handlers
impl State {
    fn handle_xfer_tx(&mut self, pkt: &DapPacketBuf) -> Result<()> { 
        let cmd = Self::parse_transfercmd(pkt)?;
        println!("  dap_idx={}, tx_cnt={}", cmd.dap_idx, cmd.xfers.len());
        for xfer in &cmd.xfers { 
            let kind = xfer.req.kind().ok_or(anyhow!("uhhhh"))?;
            let kind_s = format!("{:?}", kind);
            println!("    {:?} addr={:02x} data={:08x?} {}", 
                xfer.req.target(),
                xfer.req.word_idx().bits() << 2,
                xfer.data,
                kind_s,
            );
        }

        self.txq.push(&cmd);
        Ok(())
    }

    fn make_component_reg_string(addr: u32) -> String { 
        let mut res = String::new();
        let component = MemoryMap::resolve_component(addr);
        match component { 
            Component::ExtDbg => {
                let reg = Armv8ExtDbgReg::from_primitive(
                    (addr & 0xffff) as u16
                );
                res = format!("{:x?}", reg);
            },
            Component::Cti => {
                let reg = CtiRegister::from_primitive(
                    (addr & 0xffff) as u16
                );
                res = format!("{:x?}", reg);
            },
            Component::Pmu => {
                res = format!("PMU:{:04x}", addr & 0xffff);
            },
            Component::Etm => {
                res = format!("ETM:{:04x}", addr & 0xffff);
            },
            Component::Unk(_) => {
                res = format!("UNK:{:08x}", addr);
            },
        }
        res
    }

    fn make_xfer_ctx_string(&self, xfer: &Transfer) -> Result<String> { 
        let tgt = xfer.req.target();
        let kind = xfer.req.kind().ok_or(anyhow!("uhhh"))?;
        let mut res = String::new();

        if tgt != TransferTarget::AP { return Ok(res); }
        //if kind != TransferKind::Write { return Ok(res); }

        // FIXME: We're just assuming the AP is a MEM-AP
        let off = xfer.resolve_ap_register_offset(
            self.dp.select.apbanksel() as _
        ).unwrap();
        let reg = MemApRegister::try_from_primitive(
            off.into()
        )?;

        let addr = match reg { 
            MemApRegister::TAR_LO => Some(xfer.data.unwrap()),
            MemApRegister::BD0 => Some(self.memap.tar_lo + 0x0),
            MemApRegister::BD1 => Some(self.memap.tar_lo + 0x4),
            MemApRegister::BD2 => Some(self.memap.tar_lo + 0x8),
            MemApRegister::BD3 => Some(self.memap.tar_lo + 0xc),
            _ => None,
        };

        if let Some(addr) = addr { 
            res = Self::make_component_reg_string(addr);
        }

        Ok(res)
    }

    fn handle_xfer_rx(&mut self, pkt: &DapPacketBuf) -> Result<()> { 
        let uresp = TransferRespUnresolved::from_packet(pkt)?;
        let cmd   = self.txq.pop().unwrap();
        let resp  = uresp.resolve(&cmd)?;

        println!("  ack={:?} cnt={} value_mismatch={} protocol_err={}",
            resp.ctl.ack(), 
            resp.data.len(),
            resp.ctl.value_mismatch(), 
            resp.ctl.protocol_err()
        );

        // Print info about each of the transfers
        for xfer in resp.data { 
            //println!("{:x?}", xfer);

            // Apply this transfer to our view of the DP/AP state 
            self.apply_write(&xfer)?;

            let tgt = xfer.req.target();
            let tgt_s = match tgt { 
                TransferTarget::DP => format!("DP"),
                TransferTarget::AP => format!("AP:{:02x}", 
                    self.dp.select.apsel()),
            };

            let kind = xfer.req.kind().ok_or(anyhow!("uhhh"))?;
            let kind_s = match kind { 
                TransferKind::Read | TransferKind::ReadMatch => "R",
                TransferKind::Write | TransferKind::WriteMatchMask => "W",
            };

            // Resolve the name of the associated DP/AP register
            let reg_s = match tgt { 
                TransferTarget::DP => {
                    let reg = xfer.resolve_dp_register(
                        self.dp.select.dpbanksel() as _,
                    ).unwrap();
                    format!("{:?}", reg)
                },
                TransferTarget::AP => { 
                    let off = xfer.resolve_ap_register_offset(
                        self.dp.select.apbanksel() as _
                    ).unwrap();

                    // FIXME: We're just assuming the AP is a MEM-AP
                    let reg = MemApRegister::try_from_primitive(
                        off.into()
                    )?;
                    format!("{:?}", reg)
                },
            };

            let data_s = if let Some(val) = xfer.data { 
                format!("{:08x}", val)
            } else { 
                "".to_string()
            };

            let ctx_s = self.make_xfer_ctx_string(&xfer)?;

            println!("    {:6} {:12} {} {:8} {}", 
                tgt_s, reg_s, kind_s, data_s, ctx_s
            );
        }

        Ok(())
    }


}

/// These are for managing DP/AP state.
impl State {
    pub fn apply_write(&mut self, xfer: &Transfer) -> Result<()> { 
        let kind = xfer.req.kind().ok_or(anyhow!("uhhh"))?;
        match kind { 
            TransferKind::Read | TransferKind::ReadMatch => {},
            TransferKind::WriteMatchMask => { 
                println!("[!] Unimplemented write to match mask");
            },
            TransferKind::Write => { 
                if xfer.req.target() == TransferTarget::DP { 
                    self.apply_dp_write(xfer)?;
                } 
                if xfer.req.target() == TransferTarget::AP { 
                    self.apply_ap_write(xfer)?;
                } 

            },
        }

        Ok(())
    }

    pub fn apply_dp_write(&mut self, xfer: &Transfer) -> Result<()> { 
        let val = xfer.data.unwrap();
        let reg = DpRegister::from_address(
            xfer.req.word_idx(), self.dp.select.dpbanksel() as _
        );
        match reg { 
            DpRegister::SELECT => { 
                self.dp.select = DpSelect::from(val);
            },
            _ => {
                println!("[!] Unimplemented DP write {:08x} to {:?}", val, reg);
            },
        }
            
        Ok(())
    }

    pub fn apply_ap_write(&mut self, xfer: &Transfer) -> Result<()> { 
        let val = xfer.data.unwrap();

        // FIXME: For now, just assume all AP accesses are to a MEM-AP
        let off = xfer.resolve_ap_register_offset(
            self.dp.select.apbanksel() as _
        ).unwrap();
        let reg = MemApRegister::try_from_primitive(off.into())?;

        match reg { 
            MemApRegister::TAR_LO => { 
                self.memap.tar_lo = val;
            },
            _ => {
                println!("[!] Unimplemented AP write {:08x} to {:?}", val, reg);
            },
        }

        Ok(())
    }

}


