//! `daphne` - CMSIS-DAPv2 hackery
//!
//! This library is used to support experiments with the
//! "Raspberry Pi Debug Probe", a CMSIS-DAPv2 probe used to interact with
//! Arm debugging features exposed over SWD on the Raspberry Pi 5.

pub mod probe;
pub mod dap;

pub mod prelude {
    pub use rusb;
    pub use anyhow::{Result, anyhow};
    pub use crate::probe::*;
    pub use crate::dap::*;
}

