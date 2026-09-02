//! Parse CMSIS-DAP packets from USB packet captures.
//!
//! NOTE: If you're trying to use this, you should read it. 
//! There are some assumptions about my particular setup in here. 

use anyhow::{Result, anyhow};
use pretty_hex::*;
use daphne::prelude::*;
use std::collections::*;
use pcap;
use clap::Parser;

mod capture; 
mod parse;

use capture::*;
use parse::*;

fn parse_usb_linux<T>(cap: &mut pcap::Capture<T>) -> Result<()> 
    where T: pcap::State + pcap::Activated
{
    let mut state = State::new();
    while let Ok(p) = cap.next_packet() {
        let urb = LinuxUrbData::from_slice(&p.data[0x00..0x40]);
        // NOTE: These are the RX/TX endpoints for my CMSIS-DAP probe
        match urb.ep() { 
            0x04 | 0x85 => {},
            _ => continue,
        }

        let data = &p.data[0x40..];
        let mpkt = if data.len() != 0 {
            let p = DapPacketBuf::new_from_slice(data)?;
            Some(p)
        } else {
            None
        };
        if let Some(pkt) = mpkt { 
            state.parse_packet(&urb, &pkt)?;
        }
    }
    Ok(())
}

#[derive(Clone, Parser)]
pub enum Command { 
    Capture,
    File { path: String },
}

#[derive(Parser)]
pub struct Args { 
    #[clap(subcommand)]
    cmd: Command,
}

fn main() -> Result<()> {
    let arg = Args::parse();

    match arg.cmd { 
        Command::Capture => { 
            let mut cap = pcap::Capture::from_device("usbmon1")
                .expect("usbmon not loaded")
                .immediate_mode(true)
                .open()
                .unwrap();
            parse_usb_linux(&mut cap)?;
        },
        Command::File { path } => { 
            let mut cap = pcap::Capture::from_file(path).unwrap();
            parse_usb_linux(&mut cap)?;
        },
    };



    Ok(())
}
