use crate::io;
use crate::sys::common::small_c_string::run_with_cstr;
use crate::sys::cvt_nz;
use crate::net::SocketAddr;

use libc::dnsr_hostinfo_t;

#[path = "./unsupported.rs"]
mod unsupported;
pub use unsupported::{
    UdpSocket, TcpListener, TcpStream
};

pub struct LookupHost {
    hostinfo: *mut dnsr_hostinfo_t,
    port: u16,
    index: usize,
}

impl Drop for LookupHost {
    fn drop(&mut self) {
        unsafe {
            libc::dnsr_hostinfo_destroy(self.hostinfo);
        }
    }
}

impl LookupHost {
    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Iterator for LookupHost {
    type Item = SocketAddr;
    fn next(&mut self) -> Option<SocketAddr> {
        if self.index > 0 {
            return None;
        }
        self.index += 1;
        unimplemented!();
    }
}


impl TryFrom<&str> for LookupHost {
    type Error = io::Error;

    fn try_from(s: &str) -> io::Result<LookupHost> {
        macro_rules! try_opt {
            ($e:expr, $msg:expr) => {
                match $e {
                    Some(r) => r,
                    None => return Err(io::const_error!(io::ErrorKind::InvalidInput, $msg)),
                }
            };
        }

        // split the string by ':' and convert the second part to u16
        let (host, port_str) = try_opt!(s.rsplit_once(':'), "invalid socket address");
        let port: u16 = try_opt!(port_str.parse().ok(), "invalid port value");
        (host, port).try_into()
    }
}

impl<'a> TryFrom<(&'a str, u16)> for LookupHost {
    type Error = io::Error;

    fn try_from((host, port): (&'a str, u16)) -> io::Result<LookupHost> {
        let hostinfo = run_with_cstr(host.as_bytes(), &|c_host| {
            let mut hostinfo: *mut dnsr_hostinfo_t = crate::ptr::null_mut();
            cvt_nz(unsafe { libc::dnsr_name2host(c_host.as_ptr(), &mut hostinfo, libc::ip_ver_t::ip_any) })?;
            Ok(hostinfo)
        })?;
        Ok(LookupHost {
            hostinfo,
            port,
            index: 0,
        })
    }
}
