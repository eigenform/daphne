
use daphne::prelude::*;
use bitvec::prelude::*;

fn main() -> Result<()> { 
    //let mut ctx = rusb::Context::new()?;
    //let p = DebugProbe::init(&mut ctx)?;
    //let mut dap = Dap::new(p);

    //let dap_caps = dap.get_capabilities()?;
    //println!("[*] Capabilities: ({})", dap_caps.to_string());

    //dap.connect(ConnectCmdPort::SwdMode)?;

    println!("{} {:02x?}", 
        JTAG_TO_SWD_SEQ, 
        JTAG_TO_SWD_SEQ.data
    );


    Ok(())
}
