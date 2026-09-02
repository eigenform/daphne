
use num_enum::*;

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[derive(FromPrimitive, IntoPrimitive)]
#[repr(u16)]
pub enum CtiRegister { 
    CTI_CONTROL         = 0x000,

    CTI_INT_ACK         = 0x010,
    CTI_APP_SET         = 0x014,
    CTI_APP_CLEAR       = 0x018,
    CTI_APP_PULSE       = 0x01c,

    CTI_IN_EN0          = 0x020,

    CTI_OUT_EN0         = 0x0a0,
    CTI_OUT_EN1         = 0x0a4,

    CTI_TRIG_IN_STATUS  = 0x130,
    CTI_TRIG_OUT_STATUS = 0x134,
    CTI_CH_IN_STATUS    = 0x138,
    CTI_CH_OUT_STATUS   = 0x13c,
    CTI_GATE            = 0x140,
    ASIC_CTL            = 0x144,
    CTI_DEV_CTL         = 0x150,

    CTI_IT_CTRL         = 0xf00,
    CTI_CLAIM_SET       = 0xfa0,
    CTI_CLAIM_CLR       = 0xfa4,
    CTI_DEV_AFF0        = 0xfa8,
    CTI_DEV_AFF1        = 0xfac,

    CTI_LAR             = 0xfb0,
    CTI_LSR             = 0xfb4,
    CTI_AUTH_STATUS     = 0xfb8,
    CTI_DEV_ARCH        = 0xfbc,

    CTI_DEV_ID2         = 0xfc0,
    CTI_DEV_ID1         = 0xfc4,
    CTI_DEV_ID          = 0xfc8,
    CTI_DEV_TYPE        = 0xfcc,

    CTI_PIDR4           = 0xfd0,

    CTI_PIDR0           = 0xfe0,
    CTI_PIDR1           = 0xfe4,
    CTI_PIDR2           = 0xfe8,
    CTI_PIDR3           = 0xfec,
    CTI_CIDR0           = 0xff0,
    CTI_CIDR1           = 0xff4,
    CTI_CIDR2           = 0xff8,
    CTI_CIDR3           = 0xffc,

    #[num_enum(catch_all)]
    Undefined(u16),
}

pub enum CtiOutputTrigger { 
    /// Force PE into debug state
    DebugRequest   = 0,
    /// Request PE to exit debug state
    RestartRequest = 1,
}

pub enum CtiInputTrigger { 
    /// Asserted by PE when entering debug state
    CrossHalt = 0,
}


