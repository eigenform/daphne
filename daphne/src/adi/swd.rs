//! Types for Serial Wire Debug (SWD) interactions.
//!
//! NOTE: Some comments in this module refer to sections in
//! the Arm Debug Interface Architecture Specification 
//! (ADIv5.0 to ADIv5.2, document IHI0031H)[^adi].
//!
//! Safety: Bit-Ordering
//! ====================
//!
//! Although the documentation defines sequences in MSB-first order, 
//! the JTAG/SWD CMSIS-DAP sequencing commands expect a sequence of bits 
//! *in LSB-first order*. 
//!
//! To avoid confusing yourself, it's probably best to represent these as 
//! *a slice of bytes where the bits are in LSB-first order*. 
//! **All of the sequences here are defined and stored in LSB-first order**.
//!
//! [^adi]: [https://developer.arm.com/documentation/ihi0031/h/?lang=en]

use bitvec::prelude::*;

/// SWD line reset sequence. 
///
/// See section B4.3.3 ("Connection and line reset sequence"):
///
/// > A line reset is achieved by holding the data signal HIGH for at 
/// > least 50 clock cycles, followed by at least two idle cycles.
///
/// NOTE: For convenience, this sequence is rounded up to 56 bits in length
/// so that it can be represented with exactly 7 bytes. 
/// 
pub const LINE_RESET_BITARR: BitArr!(for 56, in u8, Lsb0) = { 
    bitarr!(u8, Lsb0; 1; 56)
};

/// JTAG-to-SWD select sequence. 
///
/// See section B5.2.2 ("Switching from JTAG to SWD operation").
///
/// This corresponds to the pair of bytes `[0x9e, 0xe7]`. 
///
pub const JTAG_TO_SWD_BITARR: BitArr!(for 16, in u8, Lsb0) = {
    let res = bitarr!(const u8, Lsb0; 
        0, 1, 1, 1, // bit 0, bit 1, bit 2, bit 3,
        1, 0, 0, 1, // ...
        1, 1, 1, 0, 
        0, 1, 1, 1, 
    );
    assert!(res.data[0] == 0x9e);
    assert!(res.data[1] == 0xe7);
    res
};

/// SWD-to-JTAG select sequence. 
///
/// See section B5.2.3 ("Switching from SWD to JTAG operation").
///
/// This corresponds to the pair of bytes `[0x3c, 0xe7]`. 
///
pub const SWD_TO_JTAG_BITARR: BitArr!(for 16, in u8, Lsb0) = {
    let res = bitarr!(const u8, Lsb0; 
        0, 0, 1, 1, // bit 0, bit 1, bit 2, bit 3,
        1, 1, 0, 0, // ...
        1, 1, 1, 0, 
        0, 1, 1, 1, 
    );
    assert!(res.data[0] == 0x3c);
    assert!(res.data[1] == 0xe7);
    res
};


pub const JTAG_TO_SWD_BYTES: [u8; 17] = [ 
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, // Line reset
    0x9e, 0xe7,                               // Switch sequence
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, // Line reset
    0x00                                      // Idle cycles
];

pub const SWD_TO_JTAG_BYTES: [u8; 10] = [ 
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, // Line reset
    0x3c, 0xe7,                               // Switch sequence
    0xff,                                     // Idle cycles
];


