
use crate::capture::*;
use std::collections::*;
use daphne::prelude::*;

pub struct TxQueue { 
    q: VecDeque<TransferCmd>
}
impl TxQueue { 
    pub fn new() -> Self { 
        Self { 
            q: VecDeque::new(),
        }
    }
    pub fn push(&mut self, xfer: TransferCmd) {
        self.q.push_back(xfer);
    }
    pub fn pop(&mut self) -> TransferCmd {
        self.q.pop_front().unwrap()
    }
}

/// State used for parsing transactions. 
pub struct State { 
    /// Queue of pending transfers
    pub txq: TxQueue,

    pub dp: DpState,
}
impl State { 
    pub fn new() -> Self { 
        Self { 
            txq: TxQueue::new(),
            dp: DpState::new(),
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
                self.handle_xfer_tx(pkt);
            },
            (DapCmdId::Transfer, Dir::Rx) => {
                self.handle_xfer_rx(pkt);
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
            let kind = req.kind()?;
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
        self.txq.push(cmd.clone());

        println!("  dap_idx={}, tx_cnt={}", cmd.dap_idx, cmd.xfers.len());

        for xfer in &cmd.xfers { 
            let kind = xfer.req.kind()?;
            let kind_s = format!("{:?}", kind);
            println!("    {:?} addr={:02x} data={:08x?} {}", 
                xfer.req.target(),
                xfer.req.address(),
                xfer.data,
                kind_s,
            );
        }
        self.apply_writes(&cmd)?;
        Ok(())
    }

    fn handle_xfer_rx(&mut self, pkt: &DapPacketBuf) -> Result<()> { 
        let cmd = self.txq.pop();
        let resp = TransferRespUnresolved::from_packet(pkt)?;
        let resp = resp.resolve(&cmd)?;

        println!("  ack={:?} value_mismatch={} protocol_err={}",
            resp.ctl.ack(), resp.ctl.value_mismatch(), resp.ctl.protocol_err()
        );
        for xfer in resp.data { 
            let tgt = xfer.req.target();

            let kind = xfer.req.kind()?;
            let kind_s = match kind { 
                TransferKind::Read | TransferKind::ReadMatch => "RD",
                TransferKind::Write | TransferKind::WriteMatchMask => "WR",
            };

            let reg_s = if tgt == TransferTarget::DP { 
                let reg = xfer.resolve_dp_register(
                    self.dp.select.dpbanksel() as _,
                ).unwrap();
                format!("{:?}", reg)
            } else { 
                let off = xfer.resolve_ap_register_offset(
                    self.dp.select.apbanksel() as _
                ).unwrap();
                format!("{:02x}", off.value())
            };

            let data_s = if let Some(val) = xfer.data { 
                format!("{:08x}", val)
            } else { 
                "".to_string()
            };

            println!("    {:?} {:12} {} {:8}", tgt, reg_s, kind_s, data_s);
        }

        Ok(())
    }


}

/// These are for managing DP/AP state.
impl State {
    pub fn apply_writes(&mut self, cmd: &TransferCmd) -> Result<()> { 
        for xfer in &cmd.xfers { 
            let kind = xfer.req.kind()?;
            match kind { 
                TransferKind::Read | TransferKind::ReadMatch => {},
                TransferKind::WriteMatchMask => { 
                    println!("[!] Unimplemented write to match mask");
                },
                TransferKind::Write => { 
                    if xfer.req.target() == TransferTarget::DP { 
                        self.apply_dp_write(xfer)?;
                    } 
                },
            }
        }

        Ok(())
    }

    pub fn apply_dp_write(&mut self, xfer: &Transfer) -> Result<()> { 
        let val = xfer.data.unwrap();
        let reg = DpRegister::from_address(
            xfer.req.address(), self.dp.select.dpbanksel() as _
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

}


