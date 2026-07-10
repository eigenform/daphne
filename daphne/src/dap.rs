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

pub mod cmd;
pub use cmd::*;

/// Interface to a CMSIS-DAPv2 device.
pub struct Dap {
    /// USB interface
    probe: DebugProbe,

    connected: Option<ConnectCmdPort>,
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
        Self { probe, connected: None }
    }

    /// Returns 'true' if this [`Dap`] is connected to some debug port.
    pub fn is_connected(&self) -> bool {
        self.connected.is_some()
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

        if resp.port == ConnectRespPort::Failed {
            return Err(anyhow!("Failed to connect to debug port?"));
        }

        self.connected = Some(port);
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

impl Drop for Dap {
    fn drop(&mut self) {
        if self.is_connected() {
            let _ = self.disconnect();
        }
    }
}

