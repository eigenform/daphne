//! Watch for CMSIS-DAP packets over USB. 
//!
//! NOTE: If you're trying to use this, you should read it. 
//! There are some assumptions about my particular setup in here. 

use anyhow::{Result, anyhow};
use pcap::*;
use pretty_hex::*;
use daphne::prelude::*;

pub trait AsUrb {
    fn is_complete(&self) -> bool;
    fn urb_type(&self) -> u8;
    fn ep(&self) -> u8;
    fn tt(&self) -> UrbTransferType;
    fn rt(&self) -> u8;
    fn req(&self) -> u8;
    fn idx(&self) -> u16;
    fn val(&self) -> u16;
    fn len(&self) -> u16;
}

#[derive(Debug)]
pub struct LinuxUrbData {
    pub data: [u8; 0x40]
}
impl LinuxUrbData {
    pub fn from_slice(slice: &[u8]) -> Self { 
        Self { data: slice.try_into().unwrap() }
    }
}
impl AsUrb for LinuxUrbData {
    fn is_complete(&self) -> bool { 
        self.urb_type() == 0x43 
    }
    fn urb_type(&self)  -> u8 { 
        self.data[0x08]
    }
    fn tt(&self) -> UrbTransferType { 
        UrbTransferType::from(self.data[0x09])
    }
    fn ep(&self) -> u8 { 
        self.data[0x0a] 
    }
    fn rt(&self) -> u8 { 
        self.data[0x28] 
    }
    fn req(&self) -> u8 { 
        self.data[0x29]
    }
    fn val(&self) -> u16 { 
        u16::from_le_bytes(self.data[0x2a..=0x2b].try_into().unwrap()) 
    }
    fn idx(&self) -> u16 { 
        u16::from_le_bytes(self.data[0x2c..=0x2d].try_into().unwrap()) 
    }
    fn len(&self) -> u16 { 
        u16::from_le_bytes(self.data[0x2e..=0x2f].try_into().unwrap()) 
    }
}


#[derive(Debug, Eq, PartialEq)]
pub enum UrbTransferType {
    Intr,
    Ctrl,
    Bulk,
    Unk(u8),
}
impl From<u8> for UrbTransferType {
    fn from(x: u8) -> Self {
        match x {
            0x01 => Self::Intr,
            0x02 => Self::Ctrl,
            0x03 => Self::Bulk,
            _    => Self::Unk(x),
        }
    }
}


fn parse_usb_linux<T>(cap: &mut Capture<T>) -> Result<()> 
    where T: pcap::State + pcap::Activated
{
    while let Ok(p) = cap.next_packet() {

        let urb = LinuxUrbData::from_slice(&p.data[0x00..0x40]);
        // NOTE: These are the RX/TX endpoints for my CMSIS-DAP probe
        match urb.ep() { 
            0x04 | 0x85 => {},
            _ => continue,
        }

        let dir = if urb.ep() == 0x04 { "IN" } else { "OUT" };
        let data = &p.data[0x40..];

        let mpkt = if data.len() != 0 {
            let p = DapPacketBuf::new_from_slice(data)?;
            Some(p)
        } else {
            None
        };
        if let Some(pkt) = mpkt { 
            let cmd_id = DapCmdId::from_primitive(pkt.id());
            let cmd_s = format!("{:?}", cmd_id);
            println!("{:>3} cmd={:02x} ({:16}) data={:02x?}", 
                dir, pkt.id(), cmd_s, pkt.content()
            );
        }


    }
    Ok(())
}


fn main() -> Result<()> {
    let mut cap = Capture::from_device("usbmon1")
        .expect("usbmon not loaded")
        .immediate_mode(true)
        .open()
        .unwrap();

    parse_usb_linux(&mut cap)?;

    Ok(())
}
