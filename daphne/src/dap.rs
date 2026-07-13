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
use crate::probe::*;
use crate::swd::{
    JTAG_TO_SWD_BYTES,
    SWD_TO_JTAG_BYTES,
};

pub mod cmd;
pub use cmd::*;
use crate::dp::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DapTransportMode { Undefined, Jtag, Swd }

/// Friendlier representation for a [`Transfer`]. 
pub enum AbstractTransfer { 
    DpRead(DpRegister),
    DpWrite(DpRegister, u32),
}
impl AbstractTransfer { 
    pub fn as_transfer(&self) -> Transfer { 
        match self { 
            Self::DpRead(reg) => {
                let idx = reg.word_idx();
                Transfer::new(TransferReqCtl::new_dp_read(idx))
            },
            Self::DpWrite(reg, val) => { 
                let idx = reg.word_idx();
                Transfer::new_with_data(
                    TransferReqCtl::new_dp_write(idx), *val
                )
            },
        }
    }
}


/// Interface to a CMSIS-DAPv2 device.
pub struct Dap {
    /// USB interface
    probe: DebugProbe,

    connection_mode: DapTransportMode,
}
impl Dap {
    /// Read a packet from the probe.
    fn read_pkt(&self) -> Result<DapPacketBuf> {
        let data = self.probe.read()?;
        let pkt = DapPacketBuf::new_from_slice(&data)?;
        println!("[*] Read: {:02x?}", pkt.data());
        Ok(pkt)
    }

    /// Send a packet to the probe.
    fn send_pkt(&self, pkt: &DapPacketBuf) -> Result<()> {
        let sz = self.probe.write(pkt.data())?;
        println!("[*] Sent: {:02x?}", pkt.data());
        if sz != pkt.len() {
            return Err(
                anyhow!("only sent {} bytes, expected {}?", sz, pkt.len())
            );
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
    pub fn send_cmd<T: DapCommand>(&self, msg: T) -> Result<T::Resp> {
        let cmd_id: u8 = T::ID.into();

        let pkt = msg.to_packet()?;
        self.send_pkt(&pkt)?;
        let rpkt = self.read_pkt()?;

        if rpkt.id() != cmd_id {
            return Err(anyhow!(
                "mismatched command ID header? expected {:02x}, got {:02x}?",
                cmd_id, rpkt.id()
            ));
        }

        T::Resp::from_packet(&rpkt)
    }

    /// Connect to a debug port.
    pub fn connect(&mut self, port: ConnectCmdPort) -> Result<()> {
        let resp = self.send_cmd(ConnectCmd {
            port: port,
        })?;
        match resp.port { 
            ConnectRespPort::Failed => {
                return Err(anyhow!("Failed to connect to debug port?"));
            },
            ConnectRespPort::Swd => { 
                self.connection_mode = DapTransportMode::Swd;
                self.set_swj_clock(4_000_000)?;
                self.xfer_configure()?;
                self.swd_configure()?;

                // The JTAG-to-SWD sequence leaves us in line reset
                // until a read from DPIDR occurs. 
                self.swj_switch_jtag_to_swd()?;
                let res = self.send_xfer(0, &[
                    AbstractTransfer::DpRead(DpRegister::DPIDR).as_transfer()
                ])?;
                println!("[*] Read DPIDR: {:08x?}", res.data[0].data);


            },
            ConnectRespPort::Jtag => {
                //self.connection_mode = DapTransportMode::Jtag;
                return Err(anyhow!("unimplemented"));
            },
        }


        Ok(())
    }

    /// Disconnect from a debug port.
    pub fn disconnect(&mut self) -> Result<()> {
        let resp = self.send_cmd(DisconnectCmd)?;
        if resp.sts != DapResponseStatus::DapOk {
            return Err(anyhow!("failed to disconnect?"));
        }
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

    fn send_xfer(&self, dap_idx: u8, xfers: &[Transfer]) 
        -> Result<TransferResp> 
    {
        let cmd = TransferCmd::new_from_slice(dap_idx, xfers)?;

        let resp = self.send_cmd(cmd.clone())?;
        let rresp = resp.resolve(&cmd)?;
        if rresp.ctl.ack() != TransferAck::Ok {
            return Err(anyhow!("transfer ack {:?} (???)", rresp.ctl.ack()));
        }
        if rresp.ctl.protocol_err() { 
            return Err(anyhow!("protocol error?"));
        }
        if rresp.ctl.value_mismatch() {
            return Err(anyhow!("value mismatch?"));
        }

        Ok(rresp)
    }

    /// Send the SWD configure command
    fn swd_configure(&self) -> Result<()> { 
        // NOTE: I guess openocd does this, which is fine for now
        let resp = self.send_cmd(SwdConfigureCmd {
            cfg: SwdConfigurationBits::from(0b0000_0000)
        })?;

        if resp.sts != DapResponseStatus::DapOk {
            return Err(anyhow!("failed to configure SWD?"));
        }
        Ok(())
    }

    /// Send the DAP transfer configure command
    fn xfer_configure(&self) -> Result<()> { 
        // NOTE: I guess openocd does this, which is fine for now
        let resp = self.send_cmd(TransferConfigureCmd {
            idle_cycles: 0x00, 
            wait_retry: 0x0040, 
            match_retry: 0x0000,
        })?;

        if resp.sts != DapResponseStatus::DapOk {
            return Err(anyhow!("failed to configure DAP transfers?"));
        }
        Ok(())
    }

    /// Set the SWD/JTAG clock frequency
    fn set_swj_clock(&self, hz: u32) -> Result<()> { 
        // NOTE: 4Mhz works fine in openocd for me. 
        // (This is just to make sure nothing explodes)
        const LIM: u32 = 4_000_000;
        if hz > LIM { 
            return Err(anyhow!("clock speed limited to <= 4Mhz"));
        }

        let resp = self.send_cmd(SwjClockCmd {
            clock: hz,
        })?;
        if resp.sts != DapResponseStatus::DapOk {
            return Err(anyhow!("failed to set clock?"));
        }
        Ok(())
    }

    fn swj_switch_jtag_to_swd(&mut self) -> Result<()> { 
        let seq_bitcnt: u8 = (JTAG_TO_SWD_BYTES.len() * 8) as u8;
        let seq_bitdata = JTAG_TO_SWD_BYTES.to_vec();

        let resp = self.send_cmd(SwjSequenceCmd { 
            seq_bitcnt,
            seq_bitdata,
        })?;
        if resp.sts != DapResponseStatus::DapOk {
            return Err(anyhow!("failed JTAG-to-SWD sequence?"));
        }
        Ok(())
    }

    fn swj_switch_swd_to_jtag(&mut self) -> Result<()> { 
        let seq_bitcnt: u8 = (SWD_TO_JTAG_BYTES.len() * 8) as u8;
        let seq_bitdata = SWD_TO_JTAG_BYTES.to_vec();

        let resp = self.send_cmd(SwjSequenceCmd { 
            seq_bitcnt,
            seq_bitdata,
        })?;
        if resp.sts != DapResponseStatus::DapOk {
            return Err(anyhow!("failed SWD-to-JTAG sequence?"));
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

