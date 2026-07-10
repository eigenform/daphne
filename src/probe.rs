//! Types representing a USB interface to a CMSIS-DAPv2 probe

use anyhow::{Result, anyhow};
use std::time::Duration;
use rusb::{
    Context, UsbContext, DeviceHandle, Direction, TransferType,
};

/// USB interface to a CMSIS-DAPv2 probe.
///
/// NOTE: This is hardcoded for my "Raspberry Pi Debug Probe".
/// This makes no attempt to discover other kinds of probes.
pub struct DebugProbe {
    pub handle: DeviceHandle<Context>,
    pub tx_ep: u8,
    pub rx_ep: u8,
    pub max_pkt_sz: usize,
}
impl DebugProbe {
    const VID: u16 = 0x2e8a;
    const PID: u16 = 0x000c;

    pub fn init(ctx: &mut Context) -> Result<Self, rusb::Error> {

        let handle = ctx.open_device_with_vid_pid(
            Self::VID,
            Self::PID
        ).ok_or(rusb::Error::NoDevice)?;

        // (Quickly verify that this is the right device)
        let lang = handle.read_languages(Duration::from_millis(100))?
            .get(0).cloned().unwrap();
        let cdesc = handle.device().active_config_descriptor()?;
        for interface in cdesc.interfaces() {
            for idesc in interface.descriptors() {
                let res = handle.read_interface_string(
                    lang, &idesc, Duration::from_millis(100)
                );
                match res {
                    Ok(s) if !s.contains("CMSIS-DAP") => continue,
                    Err(_) => continue,
                    Ok(_) => {},
                }

                let eps: Vec<_> = idesc.endpoint_descriptors().collect();
                if eps.len() != 2 { continue; }
                if eps[0].transfer_type() != TransferType::Bulk ||
                   eps[0].direction() != Direction::Out {
                       continue;
                }
                if eps[1].transfer_type() != TransferType::Bulk ||
                   eps[1].direction() != Direction::In {
                       continue;
                }

                match handle.claim_interface(interface.number()) {
                    Ok(_) => {
                        let tx_ep = eps[0].address();
                        let rx_ep = eps[1].address();
                        let max_pkt_sz = eps[1].max_packet_size() as usize;

                        return Ok(Self {
                            handle,
                            tx_ep,
                            rx_ep,
                            max_pkt_sz,
                        });
                    },
                    Err(_) => continue,
                }
            }
        }

        Err(rusb::Error::NoDevice)
    }

    /// Read data from the RX endpoint.
    pub fn read(&self) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; self.max_pkt_sz];
        let n = self.handle.read_bulk(
            self.rx_ep, &mut buf, Duration::from_millis(100)
        )?;
        buf.truncate(n);
        Ok(buf)
    }

    /// Write data to the TX endpoint.
    pub fn write(&self, buf: &[u8]) -> Result<usize> {
        let n = self.handle.write_bulk(
            self.tx_ep, buf, Duration::from_millis(10)
        )?;
        Ok(n)
    }

    /// Drain messages from the RX endpoint.
    pub fn drain(&self) -> Result<()> {
        let mut buf = vec![0u8; 1024];
        loop {
            match self.handle.read_bulk(self.rx_ep, &mut buf, Duration::from_millis(1)) {
                Ok(n) if n > 0 => continue,
                Ok(_) => break,
                Err(rusb::Error::Timeout) => break,
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }


}


