
//! Intermediate representation for DAP operations.
//!
//! A [`DapOp`] is some sequence of related DAP transfers that performs a 
//! single operation (ie. reading or writing some register/memory).
//!
//! (in the sense that they perform a single read or write operation at ).
//! 
//! - [`DpOp`]: reads/writes to DP control registers
//! - [`MemApOp`]: reads/writes to MEM-AP control registers
//!

use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use postcard;
use std::io::{Read, Write};
use std::net;
use std::thread;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::collections::*;
use super::*;

use crate::probe::DebugProbe;

#[derive(Debug, Serialize, Deserialize)]
pub enum SessionVar { 
    Bool(bool),
}

/// Simple container for persistent user data. 
pub struct DaphneSessionData { 
    map: HashMap<String, SessionVar>,
}
impl DaphneSessionData { 
    pub fn new() -> Self { 
        Self { map: HashMap::new() }
    }
    pub fn insert_bool(&mut self, name: impl ToString, val: bool) { 
        self.map.insert(name.to_string(), SessionVar::Bool(val));
    }
    pub fn get_bool(&mut self, name: &str) -> Option<bool>
    { 
        if let Some(var) = self.map.get(name) {
            match var { 
                SessionVar::Bool(res) => Some(*res),
                _ => None,
            }
        } else { 
            None
        }
    }

}


#[derive(Debug)]
pub enum DaphneServerErr { 
    /// Generic I/O error
    Io(std::io::ErrorKind),

    /// Serialization error
    Postcard(postcard::Error),

    /// CMSIS-DAP error
    Dap(DapErr),
}
impl std::error::Error for DaphneServerErr {}
impl std::fmt::Display for DaphneServerErr { 
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { 
        write!(f, "{:?}", self)
    }
}
impl From<DapErr> for DaphneServerErr { 
    fn from(e: DapErr) -> Self { Self::Dap(e) }
}
impl From<DaphneServerTransportErr> for DaphneServerErr { 
    fn from(e: DaphneServerTransportErr) -> Self { 
        match e { 
            DaphneServerTransportErr::Io(e) => Self::Io(e),
            DaphneServerTransportErr::Postcard(e) => Self::Postcard(e),

            // FIXME: Maybe TryInto is better than doing this
            DaphneServerTransportErr::NoData => unreachable!(),
        }
    }
}

impl From<postcard::Error> for DaphneServerErr { 
    fn from(e: postcard::Error) -> Self { Self::Postcard(e) }
}
impl From<std::io::Error> for DaphneServerErr { 
    fn from(e: std::io::Error) -> Self { Self::Io(e.kind()) }
}



#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DaphneServerTransportErr { 
    /// No client connected?
    NoData,

    /// Serialization error
    Postcard(postcard::Error),

    /// I/O error
    Io(std::io::ErrorKind),
}
impl From<postcard::Error> for DaphneServerTransportErr { 
    fn from(e: postcard::Error) -> Self { Self::Postcard(e) }
}
impl From<std::io::Error> for DaphneServerTransportErr { 
    fn from(e: std::io::Error) -> Self { Self::Io(e.kind()) }
}


/// A server for managing interactions with a CMSIS-DAPv2 probe. 
///
/// NOTE: This server will block when running, and is intended to be run
/// in a process separate from client code. When using [`DaphneClient`], 
/// it is assumed that you are running in a different thread.
///
pub struct DaphneServer { 
    /// USB context
    ctx: rusb::Context,
    /// CMSIS-DAP interface
    dap: Dap,
    /// TCP listener
    sock: net::TcpListener,
    /// Active TCP stream
    stream: Option<net::TcpStream>,

    signal: Arc<AtomicBool>,

    /// Arbitrary user data [persistent across TCP connections]
    session: DaphneSessionData,

    rxbuf: Vec<u8>,
    txbuf: Vec<u8>,
}
impl DaphneServer { 
    pub fn init(signal: Arc<AtomicBool>) -> Result<Self> { 
        let sock = net::TcpListener::bind("127.0.0.1:4444")?;
        sock.set_nonblocking(true)?;
        let mut ctx = rusb::Context::new().map_err(|_| { 
            anyhow!("Couldn't create USB context (?)")
        })?;
        let probe = DebugProbe::init(&mut ctx).map_err(|_| {
            anyhow!("Couldn't connect to USB probe (?)")
        })?;
        let dap = Dap::new(probe);

        Ok(Self { 
            ctx,
            dap,
            sock,
            signal,
            session: DaphneSessionData::new(),
            stream: None,
            rxbuf: vec![0u8; 0x1000],
            txbuf: vec![0u8; 0x1000],
        })
    }

    /// Wrapper around [non-blocking] `accept()`. 
    fn try_accept_client(&mut self) 
        -> Result<Option<net::TcpStream>, DaphneServerErr> 
    {
        match self.sock.accept() { 
            Ok((stream, addr)) => { 
                println!("[!] Established connection to {:?}", addr);
                return Ok(Some(stream));
            },
            Err(e) => { 
                match e.kind() { 
                    std::io::ErrorKind::WouldBlock => {
                        return Ok(None);
                    },
                    _ => return Err(e.into()),
                }
            },
        }
    }

    /// Shutdown and clear the active connection. 
    fn hup_client(&mut self) { 
        if let Some(stream) = self.stream.as_mut() {
            println!("[!] Closing connection");
            stream.shutdown(net::Shutdown::Both);
            self.stream = None;
        }
    }


    /// Perform some operation on behalf of the client. 
    ///
    /// FIXME: Distinguish between messages that return a [`DapErr`] and other 
    /// kinds of messages that do not involve interacting with the DAP. 
    pub fn handle_msg(&mut self, msg: &DaphneOp) 
        -> Result<u32, DapErr> 
    { 
        match msg { 
            DaphneOp::Ping => { 
                Ok(0x504f4e47)
            },
            DaphneOp::GetFlag(name) => { 
                let res = self.session.get_bool(name);
                let val = res.unwrap_or(false);
                Ok(val as _)
            },
            DaphneOp::SetFlag(name, data) => { 
                self.session.insert_bool(name.to_string(), *data);
                Ok(0)
            },
            DaphneOp::Dap(DapOp::Connect) => { 
                if self.dap.connection_mode() != DapTransportMode::Undefined { 
                    println!("[*] DAP transport already connected");
                    return Ok(0);
                }
                self.dap.connect(ConnectCmdPort::SwdMode)?;

                // Wait around for a bit, I guess
                thread::sleep(Duration::from_millis(100));

                Ok(0)
            },
            DaphneOp::Dap(DapOp::Disconnect) => { 
                self.dap.disconnect()?;
                Ok(0)
            },
            DaphneOp::Dp(op) => {
                //println!("[*] Handling {:x?}", op);
                let data = self.dap.send_xfer_seq(0, *op)?;
                Ok(data)
            },
            DaphneOp::MemAp(op) => {
                //println!("[*] Handling {:x?}", op);
                let data = self.dap.send_xfer_seq(0, *op)?;
                Ok(data)
            },
            DaphneOp::MemApBus(op) => {
                //println!("[*] Handling {:x?}", op);
                let data = self.dap.send_xfer_seq(0, *op)?;
                Ok(data)
            },
        }
    }

    /// Wait for a connection with some client. 
    pub fn wait_for_client(&mut self) -> Result<bool, DaphneServerErr> { 
        assert!(self.stream.is_none());

        while self.stream.is_none() {
            let res = self.try_accept_client()?;
            if let Some(stream) = res { 
                stream.set_nodelay(true)?;
                self.stream = Some(stream);
            }

            if self.signal.load(Ordering::SeqCst) { 
                println!("[!] Caught signal, exiting");
                return Ok(false);
            }

            thread::sleep(Duration::from_millis(10));
        }
        return Ok(true);
    }

    /// Run the server [indefinitely]
    pub fn run(&mut self) -> Result<(), DaphneServerErr> { 
        loop {
            println!("[*] Waiting for connection on 127.0.0.1:4444 ...");

            // Wait [indefinitely] for a client to connect
            match self.wait_for_client() { 
                Ok(run) => { if !run { break; } },
                Err(e) => { 
                    println!("[!] Server terminating with {:?}", e);
                    return Err(e);
                },
            }

            // Interact with the client until the connection terminates
            // or some other fatal error occurs
            match self.main_loop() { 
                Ok(run) => { if !run { break; } },
                Err(e) => { 
                    println!("[!] Server terminating with {:?}", e);
                    return Err(e);
                },
            }
            thread::yield_now();
        }
        Ok(())
    }
    

    /// Handle the current active connection.
    pub fn main_loop(&mut self) -> Result<bool, DaphneServerErr> { 
        assert!(self.stream.is_some());

        loop { 

            if self.signal.load(Ordering::SeqCst) { 
                println!("[!] Caught signal, exiting");
                return Ok(false);
            }

            // Try to read a packet from the client
            let pkt = match self.read_packet() { 
                Ok(pkt) => pkt, 
                Err(DaphneServerTransportErr::Io(std::io::ErrorKind::WouldBlock)) => {
                    thread::yield_now();
                    continue;
                },

                Err(DaphneServerTransportErr::Io(std::io::ErrorKind::BrokenPipe)) |
                Err(DaphneServerTransportErr::NoData) => { 
                    println!("[!] Client disconnected?");
                    self.hup_client();
                    return Ok(true);
                },

                Err(e) => {
                    return Err(e.into());
                },
            };

            // Process a message from the client
            let msg_res = self.handle_msg(&pkt.msg);
            let resp = if let Err(e) = &msg_res { 
                println!("[!] Error handling message {:x?}", pkt.msg);
                println!("[!] {:?}", e);
                DaphneResp { data: 0, sts: DaphneRespSts::Err }
            } else { 
                DaphneResp { data: msg_res.unwrap(), sts: DaphneRespSts::Ok }
            };

            // Try to respond to the client
            match self.send_response(&resp) { 
                Ok(_) => {},
                Err(DaphneServerTransportErr::Io(std::io::ErrorKind::BrokenPipe)) => {
                    println!("[!] Client disconnected?");
                    self.hup_client();
                    return Ok(true);
                },
                Err(e) => { 
                    return Err(e.into());
                },
            }

            // If there was some error during message handling, deal with it 
            // here *after* we've responded to the client. 
            if let Err(e) = &msg_res  {
                return Err(DaphneServerErr::Dap(*e));
            }

            thread::yield_now();
        }

    }

    /// Read a message from the client. 
    fn read_packet(&mut self) 
        -> Result<DaphnePacket, DaphneServerTransportErr>
    {
        let stream = self.stream.as_mut().unwrap();

        let len = stream.read(&mut self.rxbuf)?;
        if len == 0 { 
            return Err(DaphneServerTransportErr::NoData);
        }
        let pkt = postcard::from_bytes::<DaphnePacket>(&self.rxbuf)?;
        return Ok(pkt)
    }

    /// Send a response to the client.
    fn send_response(&mut self, resp: &DaphneResp) 
        -> Result<(), DaphneServerTransportErr>
    { 
        let stream = self.stream.as_mut().unwrap();
        let slice = postcard::to_slice::<DaphneResp>(
            resp, 
            &mut self.txbuf
        )?;
        let len = stream.write(&slice)?;
        Ok(())
    }


}

// NOTE: When a [`DaphneServer`] is dropped, we disconnect from the DAP.
impl Drop for DaphneServer { 
    fn drop(&mut self) { 
        self.dap.disconnect();
    }
}


