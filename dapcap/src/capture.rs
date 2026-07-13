

pub trait AsUrb {
    fn is_complete(&self) -> bool;
    fn urb_type(&self) -> u8;
    fn ep(&self) -> u8;
    fn tt(&self) -> UrbTransferType;
    fn rt(&self) -> u8;
    fn req(&self) -> u8;
    fn idx(&self) -> u16;
    fn val(&self) -> u16;
    fn len(&self) -> u16;
    fn dir(&self) -> Dir;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir { Rx, Tx }

#[derive(Debug)]
pub struct LinuxUrbData {
    pub data: [u8; 0x40]
}
impl LinuxUrbData {
    pub fn from_slice(slice: &[u8]) -> Self { 
        Self { data: slice.try_into().unwrap() }
    }
}
impl AsUrb for LinuxUrbData {
    fn is_complete(&self) -> bool { 
        self.urb_type() == 0x43 
    }
    fn urb_type(&self)  -> u8 { 
        self.data[0x08]
    }
    fn tt(&self) -> UrbTransferType { 
        UrbTransferType::from(self.data[0x09])
    }
    fn ep(&self) -> u8 { 
        self.data[0x0a] 
    }
    fn dir(&self) -> Dir { 
        if self.ep() & 0x80 != 0 { Dir::Rx } else { Dir::Tx }
    }
    fn rt(&self) -> u8 { 
        self.data[0x28] 
    }
    fn req(&self) -> u8 { 
        self.data[0x29]
    }
    fn val(&self) -> u16 { 
        u16::from_le_bytes(self.data[0x2a..=0x2b].try_into().unwrap()) 
    }
    fn idx(&self) -> u16 { 
        u16::from_le_bytes(self.data[0x2c..=0x2d].try_into().unwrap()) 
    }
    fn len(&self) -> u16 { 
        u16::from_le_bytes(self.data[0x2e..=0x2f].try_into().unwrap()) 
    }
}


#[derive(Debug, Eq, PartialEq)]
pub enum UrbTransferType {
    Intr,
    Ctrl,
    Bulk,
    Unk(u8),
}
impl From<u8> for UrbTransferType {
    fn from(x: u8) -> Self {
        match x {
            0x01 => Self::Intr,
            0x02 => Self::Ctrl,
            0x03 => Self::Bulk,
            _    => Self::Unk(x),
        }
    }
}


