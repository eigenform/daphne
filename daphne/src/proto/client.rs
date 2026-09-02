
use num_enum::*;
use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use postcard;
use std::io::{Read, Write};
use std::net;
use super::*;

#[derive(Debug)]
pub enum DaphneClientErr { 
    Postcard(postcard::Error),
    Io(std::io::ErrorKind),
}
impl std::fmt::Display for DaphneClientErr { 
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { 
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for DaphneClientErr {}
impl From<postcard::Error> for DaphneClientErr { 
    fn from(e: postcard::Error) -> Self { 
        Self::Postcard(e)
    }
}
impl From<std::io::Error> for DaphneClientErr { 
    fn from(e: std::io::Error) -> Self { 
        Self::Io(e.kind())
    }
}

/// A connection to some [`DaphneServer`]. 
pub struct DaphneClient { 
    sock: net::TcpStream,
}
impl DaphneClient { 
    pub fn connect() -> Result<Self> {
        let sock = net::TcpStream::connect("127.0.0.1:4444")?;
        sock.set_nodelay(true)?;
        let res = Self { sock };
        Ok(res)
    }

    pub fn set_flag(&mut self, name: impl ToString, val: bool) 
        -> Result<()>
    { 
        self.send(DaphneOp::SetFlag(name.to_string(), val))?;
        Ok(())
    }

    pub fn get_flag(&mut self, name: impl ToString) -> Result<bool> { 
        let res = self.send(DaphneOp::GetFlag(name.to_string()))?;
        Ok(res.data != 0)
    }

    pub fn send(&mut self, msg: DaphneOp) -> Result<DaphneResp, DaphneClientErr> {
        let pkt = DaphnePacket { msg };
        let buf = postcard::to_vec::<DaphnePacket, 4096>(&pkt)
            .unwrap();
        self.sock.write(&buf).unwrap();

        let mut buf = vec![0u8; 4096];
        match self.sock.read(&mut buf) { 
            Ok(len) => { 
                let resp = postcard::from_bytes::<DaphneResp>(&buf)?;
                return Ok(resp);
            },
            Err(e) => Err(e.into()),
        }
    }
}
impl Drop for DaphneClient { 
    fn drop(&mut self) { 
        self.sock.shutdown(net::Shutdown::Both);
    }
}


