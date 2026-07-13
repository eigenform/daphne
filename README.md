# `daphne` 

CMSIS-DAPv2 probe hackery, specifically for playing with my
[Raspberry Pi Debug Probe](https://www.raspberrypi.com/documentation/microcontrollers/debug-probe.html).

```
.
├── dapcap/        # Helper for watching/parsing USB CMSIS-DAP packets
├── daphne/        # Library crate
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

## License

This project is MIT licensed (see [`LICENSE-MIT`](./LICENSE-MIT)), and was 
originally based on [adamgreig/jtagdap](https://github.com/adamgreig/jtagdap), 
which is dual-licensed under either APLv2 or MIT. The original MIT license text
is preserved here in [`LICENSE-MIT.jtagdap`](./LICENSE-MIT.jtagdap).

