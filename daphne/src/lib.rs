//! `daphne` - CMSIS-DAPv2 hackery
//!
//! This library is used to support experiments with the
//! "Raspberry Pi Debug Probe", a CMSIS-DAPv2 probe used to interact with
//! Arm debugging features exposed over SWD on the Raspberry Pi 5.
//!
//! Organization
//! ============
//!
//! - [`DebugProbe`]: primitive USB communication with a probe
//! - [`Dap`]: CMSIS-DAP commands [over USB]
//!

pub mod probe;
pub mod dap;
pub mod adi;
pub mod proto;
pub mod component;
pub mod aarch64;

pub mod prelude {
    pub use rusb;
    pub use anyhow::{Result, anyhow};
    pub use num_enum::{FromPrimitive, IntoPrimitive, TryFromPrimitive};
    pub use modular_bitfield::prelude::*;

    pub use crate::probe::*;
    pub use crate::dap::*;
    pub use crate::adi::*;
    pub use crate::component::*;
    pub use crate::proto::*;
    pub use crate::aarch64::*;
}


