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
> library. This works for my particular use case, but there are no hard 
> guarantees in other cases. 
>
> Additionally, note that the server and library currently make assumptions 
> based on my own environment with the Raspberry Pi 5. For instance:
>
> - There is no JTAG support 
> - The USB backend assumes the PID/VID of the Raspberry Pi Debug Probe
> - Some SWD primitives are hardcoded to target DAP index 0
> - SWD DAP initialization may not be generalizable to other devices

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

