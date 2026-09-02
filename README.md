# `daphne` 

CMSIS-DAPv2 probe hackery, specifically for playing with my
[Raspberry Pi Debug Probe](https://www.raspberrypi.com/documentation/microcontrollers/debug-probe.html).

```
.
├── dapcap/        # Utility for sniffing CMSIS-DAPv2 packets over USB
├── daphne/        # Library crate
├── daphne-server/ # Server binary crate
└── README.md      # (You are here)
```

> [!CAUTION]
> This is not a "complete" implementation in any sense, and important details
> from various specifications (CMSIS-DAP, ADIv5, CoreSight, JTAG, SWD, etc.)
> may be only partially represented or otherwise totally absent from this 
> library. 
>
> There are no guarantees about the correctness/soundness of this library.
> In general, you should probably not use this. 

This library has two parts: 

- A `daphne-server` binary crate, which manages communication between the 
  target and some client program on the host machine

- A `daphne` library crate, used to write client programs 

## Server Usage

When running client programs, `daphne-server` must be running in the background 
in order to actually communicate with the target device.

```
$ git clone https://github.com/eigenform/daphne && cd daphne
...

$ cargo build --release --bin daphne-server
...

# Start the server
$ ./target/release/daphne-server
...
```

## Client Usage

> [!CAUTION]
> This library crate might make assumptions about my target device 
> (the Raspberry Pi 5). For instance, currently all DAP transactions are 
> hard-coded to use DAP index 0. 

The library crate includes a `DaphneClient` type that exposes methods for 
connecting to the DAP on a target machine and performing DP/MEM-AP accesses.
CMSIS-DAP transactions are handled transparently by the server.
For example:

```rust
use daphne::prelude::*;

// Connect to the server
let mut cli = DaphneClient::connect()?;

// Connect to the DAP 
cli.send(DaphneOp::Dap(DapOp::Connect))?;

// Perform a DP read operation
let resp = cli.send(DaphneOp::Dp(DpOp::Read { reg: DpRegister::DPIDR }))?;
println!("[*] DPIDR: {:08x?}", resp);

// Perform a MEM-AP read operation
let dbg = DebugBlock { base: 0x8001_0000 };
let resp = cli.send(DaphneOp::MemApBus(MemApBusOp::Read { 
    ap: 0, addr: dbg.addr_of(Armv8ExtDbgReg::MIDR),
}))?;
println!("[*] MIDR: {:08x?}", resp);
```


## License

This project is MIT licensed (see [`LICENSE-MIT`](./LICENSE-MIT)), and was 
originally based on [adamgreig/jtagdap](https://github.com/adamgreig/jtagdap), 
which is dual-licensed under either APLv2 or MIT. The original MIT license text
is preserved here in [`LICENSE-MIT.jtagdap`](./LICENSE-MIT.jtagdap).

