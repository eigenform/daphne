
use std::net;
use daphne::prelude::*;
use std::thread;
use std::time::Duration;
use std::io::{Read, Write};
use anyhow::{Result, anyhow};
use postcard;
use serde::{Serialize, Deserialize};
use clap::Parser;

use ctrlc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() -> Result<(), String> {
    let running = Arc::new(AtomicBool::new(false));
    let r = running.clone();

    ctrlc::set_handler(move || { 
        r.store(true, Ordering::SeqCst);
    });

    let mut server = DaphneServer::init(running.clone()).unwrap();
    server.run().unwrap();
    Ok(())
}
