
use daphne::prelude::*;

fn main() -> Result<()> { 
    let mut ctx = rusb::Context::new()?;
    let p = DebugProbe::init(&mut ctx)?;
    let dap = Dap::new(p);

    let dap_caps = dap.get_capabilities()?;
    println!("[*] Capabilities: ({})", dap_caps.to_string());

    Ok(())
}
