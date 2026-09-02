//! Types representing a CMSIS-DAP interface.
//!
//! CMSIS-DAP is an interface for communicating with a "debug port" (DP) on
//! the target Arm device (either an SWD-DP, JTAG-DP, or SWJ-DP).
//! The debug port is connected to one or more "access ports" (APs)
//! that provide access to debug components on the target.
//!
//! See (the documentation)[https://arm-software.github.io/CMSIS-DAP/latest/]
//! for more details.
//!
//! Overview
//! ========
//!
//! - [`DapCommand`] is implemented for types that represent commands.
//! - [`DapResponse`] is implemented for types that represent responses.
//! - [`DapPacketBuf`] is a packet sent to the probe.
//!

use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use crate::probe::*;
use crate::adi::*;
use crate::proto::{ DpOp, MemApOp, MemApBusOp };
use std::collections::*;

pub mod cmd;
pub mod cfg;
pub use cmd::*;
pub use cfg::*;

/// The underlying transport protocol associated with some [`Dap`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DapTransportMode { Undefined, Jtag, Swd }


/// Structure for tracking pending transfers.
pub struct XferQueue { 
    /// FIFO queue of pending transfer commands
    pub q: VecDeque<TransferCmd>,
}
impl XferQueue { 
    pub fn new() -> Self { 
        Self { q: VecDeque::new(), }
    }
    pub fn is_empty(&self) -> bool { 
        self.q.is_empty()
    }
    pub fn push(&mut self, cmd: &TransferCmd) { 
        self.q.push_back(cmd.clone());
    }
    pub fn pop(&mut self) -> Option<TransferCmd> { 
        self.q.pop_front()
    }
    pub fn peek(&self, idx: usize) -> Option<&TransferCmd> { 
        self.q.get(idx)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum DapErr { 
    Probe(ProbeErr),
    Xfer(XferErr),
    CommandFailed,
    PacketErr,
    Unimpl,
}
impl From<ProbeErr> for DapErr { 
    fn from(e: ProbeErr) -> Self { Self::Probe(e) }
}
impl From<XferErr> for DapErr { 
    fn from(e: XferErr) -> Self { Self::Xfer(e) }
}
impl std::error::Error for DapErr {}
impl std::fmt::Display for DapErr { 
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { 
        write!(f, "{:?}", self)
    }
}


/// Interface to a CMSIS-DAPv2 device [over USB].
pub struct Dap {
    /// USB interface
    probe: DebugProbe,

    /// Current transport mode
    connection_mode: DapTransportMode,
}
impl Dap {
    /// Read a packet from the probe.
    fn read_pkt(&self) -> Result<DapPacketBuf, DapErr> {
        let data = self.probe.read()?;
        let pkt = DapPacketBuf::new_from_slice(&data)
            .map_err(|_| DapErr::PacketErr)?;
        //println!("[*] Read: {:02x?}", pkt.data());
        Ok(pkt)
    }

    /// Send a packet to the probe.
    fn send_pkt(&self, pkt: &DapPacketBuf) -> Result<(), DapErr> {
        let sz = self.probe.write(pkt.data())?;
        //println!("[*] Sent: {:02x?}", pkt.data());
        if sz != pkt.len() {
            return Err(DapErr::PacketErr);
        }
        Ok(())
    }
}

impl Dap {
    pub fn new(probe: DebugProbe) -> Self {
        Self { 
            probe, 
            connection_mode: DapTransportMode::Undefined,
        }
    }

    pub fn connection_mode(&self) -> DapTransportMode {
        self.connection_mode
    }

    /// Send a [`DapCommand`], then read the response.
    pub fn send_cmd<T: DapCommand>(&self, msg: T) -> Result<T::Resp, DapErr> {
        let cmd_id: u8 = T::ID.into();

        let pkt = msg.to_packet().map_err(|_| DapErr::PacketErr)?;
        self.send_pkt(&pkt)?;
        let rpkt = self.read_pkt()?;

        if rpkt.id() != cmd_id {
            return Err(DapErr::PacketErr);
        }

        T::Resp::from_packet(&rpkt).map_err(|_| DapErr::PacketErr)
    }

    /// Connect to a debug port.
    ///
    /// NOTE: This automatically issues various DAP commands after connecting.
    ///
    pub fn connect(&mut self, port: ConnectCmdPort) -> Result<(), DapErr> {
        if self.connection_mode != DapTransportMode::Undefined { 
            return Ok(());
        }

        let resp = self.send_cmd(ConnectCmd {
            port: port,
        })?;
        match resp.port { 
            ConnectRespPort::Failed => {
                return Err(DapErr::CommandFailed);
            },
            ConnectRespPort::Swd => { 
                println!("[*] Connecting via SWD ...");
                self.connection_mode = DapTransportMode::Swd;

                self.send_cmd(SwjPinsCmd {
                    pin_out: SwjPinBits(0),
                    pin_sel: SwjPinBits(0),
                    pin_wait: 0,
                })?;

                self.set_swj_clock(4_000_000)?;

                // NOTE: I guess openocd does this, which is fine for now
                self.xfer_configure(0, 0x40, 0)?;

                // NOTE: I guess openocd does this, which is fine for now
                self.swd_configure(SwdConfigurationBits::from(0))?;

                // The JTAG-to-SWD sequence leaves us in line reset
                // until a read from DPIDR occurs. 
                self.swj_switch_jtag_to_swd()?;

                self.set_swj_clock(4_000_000)?;

                let resp = self.send_xfer_seq(0, 
                    DpOp::Read { reg: DpRegister::DPIDR }
                )?;
                println!("[*] Read DPIDR: {:08x?}", resp);

                // Clear pending errors
                let resp = self.send_xfer_seq(0, 
                    DpOp::Write { reg: DpRegister::DPIDR, 
                        data: DpAbort::new()
                            .with_dap_abort(0)
                            .with_stk_cmp_clr(1)
                            .with_stk_err_clr(1)
                            .with_wd_err_clr(1)
                            .with_orun_err_clr(1)
                            .into()
                    }
                )?;

                let resp = self.send_xfer_seq(0, 
                    DpOp::Read { reg: DpRegister::CTRLSTAT }
                )?;
                let bits = DpCtrlStat::from(resp);
                println!("[*] Read CTRLSTAT: {:08x?} {:?}", resp, bits);


                println!("[*] Sending power-up request ..");

                let mut xfers = { SequenceBuilder::new()
                    // Clear SELECT
                    .add(DpOp::Write { reg: DpRegister::SELECT, data: 0 })

                    // CSYS/CDBG powerup
                    .add(DpOp::Write { reg: DpRegister::CTRLSTAT, 
                        data: DpCtrlStat::new()
                            .with_sticky_orun(1)
                            .with_sticky_err(1)
                            .with_csys_pwrup_req(1)
                            .with_cdbg_pwrup_req(1)
                            .into()
                    })

                    .add(DpOp::Read { reg: DpRegister::CTRLSTAT })

                    .add(DpOp::Write { reg: DpRegister::CTRLSTAT, 
                        data: DpCtrlStat::new()
                            .with_sticky_orun(0)
                            .with_sticky_err(0)
                            .with_csys_pwrup_req(1)
                            .with_cdbg_pwrup_req(1)
                            .into()
                    })

                    .add(DpOp::Read { reg: DpRegister::CTRLSTAT })

                    .finish()
                };

                let resp = self.send_xfer_raw(0, &xfers)?;
                let ctrlstat = resp.last_result().unwrap();
                let bits = DpCtrlStat::from(ctrlstat);
                println!("[*] got CTRLSTAT {:08x} {:#?}", ctrlstat, DpCtrlStat::from(ctrlstat));

                let mut xfers = { SequenceBuilder::new()
                    .add(DpOp::Read { reg: DpRegister::CTRLSTAT })

                    .add(DpOp::Write { reg: DpRegister::CTRLSTAT, 
                        data: DpCtrlStat::new()
                            .with_sticky_orun(0)
                            .with_sticky_err(0)
                            .with_csys_pwrup_req(1)
                            .with_cdbg_pwrup_req(1)
                            .into()
                    })

                    .add(DpOp::Read { reg: DpRegister::CTRLSTAT })
                    .finish()
                };

                let resp = self.send_xfer_raw(0, &xfers)?;
                let ctrlstat = resp.last_result().unwrap();
                let bits = DpCtrlStat::from(ctrlstat);
                println!("[*] got CTRLSTAT {:08x} {:#?}", ctrlstat, DpCtrlStat::from(ctrlstat));


                if bits.cdbg_pwrup_ack() == 0 || bits.csys_pwrup_ack() == 0 {
                    println!("[!] Powerup sequence failed?");
                    return Err(DapErr::CommandFailed);
                }
            },
            ConnectRespPort::Jtag => {
                return Err(DapErr::Unimpl);
            },
        }


        Ok(())
    }

    /// Disconnect from a debug port.
    pub fn disconnect(&mut self) -> Result<(), DapErr> {
        if self.connection_mode == DapTransportMode::Undefined { 
            return Ok(());
        }

        let resp = self.send_cmd(DisconnectCmd)?;
        if resp.sts != DapResponseStatus::DapOk {
            return Err(DapErr::CommandFailed);
        }
        self.connection_mode = DapTransportMode::Undefined;
        Ok(())
    }

    /// Read CMSIS-DAP probe capabilities.
    pub fn get_capabilities(&self) -> Result<DapCapabilityInfo0> {
        let resp = self.send_cmd(InfoCmd {
            req_id: InfoReqId::Capabilities
        })?;

        let caps = DapCapabilityInfo0(resp.data[0]);
        Ok(caps)
    }

}

impl Dap {


    /// Send some operation implementing [`TransferSequence`], returning the 
    /// data associated with the last transfer in the sequence. 
    pub fn send_xfer_seq<T: TransferSequence>(&self, dap_idx: u8, op: T)
        -> Result<u32, DapErr>
    {
        let xfers = op.as_transfer_seq();
        let resp = self.send_xfer_raw(dap_idx, &xfers)?;

        if let Some(xfer) = resp.data.last() { 
            if let Some(data) = xfer.data { 
                Ok(data)
            } else { 
                Ok(0)
            }
        } else { 
            Err(DapErr::PacketErr)
        }
    }

    /// Create and send a [`TransferCmd`] from the provided [`Transfer`]s. 
    fn send_xfer_raw(&self, dap_idx: u8, xfers: &[Transfer]) 
        -> Result<TransferResp, DapErr> 
    {
        //println!("[*] Sending transfers:");
        //for xfer in xfers { 
        //    println!("    {:x?}", xfer);
        //}

        let cmd = TransferCmd::new_from_slice(dap_idx, xfers)
            .map_err(|_| DapErr::PacketErr)?;
        let resp = self.send_cmd(cmd.clone())?;
        let rresp = resp.resolve(&cmd)?;
        if rresp.ctl.ack() != TransferAck::Ok {
            return Err(DapErr::Xfer(XferErr::Ack(rresp.ctl.ack())));
        }
        if rresp.ctl.protocol_err() { 
            return Err(DapErr::Xfer(XferErr::ProtocolErr));
        }
        if rresp.ctl.value_mismatch() {
            return Err(DapErr::Xfer(XferErr::ValueMismatch));
        }
        Ok(rresp)
    }

    /// Send the SWD configure command
    fn swd_configure(&self, cfg: SwdConfigurationBits) -> Result<(), DapErr> { 
        let resp = self.send_cmd(SwdConfigureCmd { cfg })?;

        if resp.sts != DapResponseStatus::DapOk {
            return Err(DapErr::CommandFailed);
        }
        Ok(())
    }

    /// Send the DAP transfer configure command
    fn xfer_configure(&self, idle_cycles: u8, wait_retry: u16, match_retry: u16) 
        -> Result<(), DapErr> 
    { 
        let resp = self.send_cmd(TransferConfigureCmd {
            idle_cycles, wait_retry, match_retry,
        })?;

        if resp.sts != DapResponseStatus::DapOk {
            return Err(DapErr::CommandFailed);
        }
        Ok(())
    }

    /// Set the SWD/JTAG clock frequency
    fn set_swj_clock(&self, hz: u32) -> Result<(), DapErr> { 
        // NOTE: 4Mhz works fine in openocd for me. 
        // (This is just to make sure nothing explodes)
        const LIM: u32 = 4_000_000;
        if hz > LIM { 
            //return Err(anyhow!("clock speed limited to <= 4Mhz"));
            return Err(DapErr::CommandFailed);
        }

        let resp = self.send_cmd(SwjClockCmd {
            clock: hz,
        })?;
        if resp.sts != DapResponseStatus::DapOk {
            return Err(DapErr::CommandFailed);
        }
        Ok(())
    }

    fn swj_switch_jtag_to_swd(&mut self) -> Result<(), DapErr> { 
        let seq_bitcnt: u8 = (JTAG_TO_SWD_BYTES.len() * 8) as u8;
        let seq_bitdata = JTAG_TO_SWD_BYTES.to_vec();

        let resp = self.send_cmd(SwjSequenceCmd { 
            seq_bitcnt,
            seq_bitdata,
        })?;
        if resp.sts != DapResponseStatus::DapOk {
            return Err(DapErr::CommandFailed);
        }
        Ok(())
    }

    fn swj_switch_swd_to_jtag(&mut self) -> Result<(), DapErr> { 
        let seq_bitcnt: u8 = (SWD_TO_JTAG_BYTES.len() * 8) as u8;
        let seq_bitdata = SWD_TO_JTAG_BYTES.to_vec();

        let resp = self.send_cmd(SwjSequenceCmd { 
            seq_bitcnt,
            seq_bitdata,
        })?;
        if resp.sts != DapResponseStatus::DapOk {
            return Err(DapErr::CommandFailed);
        }
        Ok(())
    }
}

impl Drop for Dap {
    fn drop(&mut self) {
        match self.connection_mode() { 
            DapTransportMode::Undefined => {},
            DapTransportMode::Jtag => { 
                let _ = self.disconnect();
            },
            DapTransportMode::Swd => {
                let _ = self.disconnect();
            },
        }
    }
}

