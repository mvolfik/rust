use libc::{c_void, dnsr_hostinfo_t, tcp_conn_t, tcp_t};

use crate::io::{self, BorrowedCursor, IoSlice, IoSliceMut};
use crate::net::{Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, SocketAddrV4, SocketAddrV6};
use crate::ptr;
use crate::sys::common::small_c_string::run_with_cstr;
use crate::sys::{cvt_nz, unsupported};
use crate::time::Duration;

#[path = "./unsupported.rs"]
mod unsupported;
pub use unsupported::UdpSocket;

pub struct LookupHost {
    hostinfo: *mut dnsr_hostinfo_t,
    port: u16,
    index: usize,
}

unsafe impl Send for LookupHost {}

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
        let _index = self.index;
        self.index += 1;
        let hostinfo: &dnsr_hostinfo_t = unsafe { &*self.hostinfo };
        let addr_union = &hostinfo.addr.addr;
        Some(match hostinfo.addr.version {
            libc::ip_ver_t::ip_v4 => {
                let addr = unsafe { addr_union.addr };
                SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::from_bits(addr), self.port))
            }
            libc::ip_ver_t::ip_v6 => {
                let addr = unsafe { addr_union.addr6 };
                SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::from_octets(addr), self.port, 0, 0))
            }
            invalid => panic!("invalid ip version returned by libc: {:?}", invalid as u32),
        })
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
            let mut hostinfo: *mut dnsr_hostinfo_t = ptr::null_mut();
            cvt_nz(unsafe {
                libc::dnsr_name2host(c_host.as_ptr(), &mut hostinfo, libc::ip_ver_t::ip_any)
            })?;
            Ok(hostinfo)
        })?;
        Ok(LookupHost { hostinfo, port, index: 0 })
    }
}

#[derive(Debug)]
pub struct TcpStream {
    client: *mut tcp_t,
    conn: *mut tcp_conn_t,
}

unsafe impl Send for TcpStream {}

impl Drop for TcpStream {
    fn drop(&mut self) {
        unsafe {
            libc::tcp_conn_destroy(self.conn);
            libc::tcp_destroy(self.client);
        }
    }
}

fn socket_addr_to_c(a: &SocketAddr) -> libc::inet_ep_t {
    let mut ep: libc::inet_ep_t = unsafe { crate::mem::zeroed() };
    unsafe { libc::inet_ep_init(&mut ep) };
    ep.port = a.port();
    match a {
        SocketAddr::V4(addr) => {
            ep.addr.version = libc::ip_ver_t::ip_v4;
            ep.addr.addr.addr = addr.ip().to_bits();
        }
        SocketAddr::V6(addr) => {
            ep.addr.version = libc::ip_ver_t::ip_v6;
            ep.addr.addr.addr6 = addr.ip().octets();
        }
    }
    ep
}

impl TcpStream {
    pub fn connect(addr: io::Result<&SocketAddr>) -> io::Result<TcpStream> {
        let addr = addr?;
        let mut client: *mut tcp_t = ptr::null_mut();
        cvt_nz(unsafe { libc::tcp_create(&mut client) })?;

        let mut epp: libc::inet_ep2_t = unsafe { crate::mem::zeroed() };
        unsafe { libc::inet_ep2_init(&mut epp) };
        epp.remote = socket_addr_to_c(addr);

        let mut conn: *mut tcp_conn_t = ptr::null_mut();
        let conn_res = cvt_nz(unsafe {
            libc::tcp_conn_create(client, &mut epp, ptr::null_mut(), ptr::null_mut(), &mut conn)
        });
        if let Err(e) = conn_res {
            unsafe { libc::tcp_destroy(client) };
            return Err(e);
        }
        let res = TcpStream { client, conn }; // drop on error will destroy the client and conn
        cvt_nz(unsafe { libc::tcp_conn_wait_connected(conn) })?;
        Ok(res)
    }

    pub fn connect_timeout(_: &SocketAddr, _: Duration) -> io::Result<TcpStream> {
        unsupported()
    }

    pub fn set_read_timeout(&self, _: Option<Duration>) -> io::Result<()> {
        unsupported()
    }

    pub fn set_write_timeout(&self, _: Option<Duration>) -> io::Result<()> {
        unsupported()
    }

    pub fn read_timeout(&self) -> io::Result<Option<Duration>> {
        unsupported()
    }

    pub fn write_timeout(&self) -> io::Result<Option<Duration>> {
        unsupported()
    }

    pub fn peek(&self, _: &mut [u8]) -> io::Result<usize> {
        unsupported()
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let mut read: usize = 0;
        cvt_nz(unsafe {
            libc::tcp_conn_recv_wait(
                self.conn,
                buf.as_mut_ptr() as *mut c_void,
                buf.len(),
                &mut read,
            )
        })?;
        Ok(read)
    }

    pub fn read_buf(&self, mut cursor: BorrowedCursor<'_>) -> io::Result<()> {
        unsafe {
            let mut read: usize = 0;
            cvt_nz(libc::tcp_conn_recv_wait(
                self.conn,
                cursor.as_mut().as_mut_ptr() as *mut c_void,
                cursor.capacity(),
                &mut read,
            ))?;
            cursor.advance_unchecked(read);
        }
        Ok(())
    }

    pub fn read_vectored(&self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        crate::io::default_read_vectored(|b| self.read(b), bufs)
    }

    pub fn is_read_vectored(&self) -> bool {
        false
    }

    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        cvt_nz(unsafe { libc::tcp_conn_send(self.conn, buf.as_ptr() as *mut c_void, buf.len()) })?;
        Ok(buf.len())
    }

    pub fn write_vectored(&self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        crate::io::default_write_vectored(|b| self.write(b), bufs)
    }

    pub fn is_write_vectored(&self) -> bool {
        false
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        unsupported()
    }

    pub fn socket_addr(&self) -> io::Result<SocketAddr> {
        unsupported()
    }

    pub fn shutdown(&self, _: Shutdown) -> io::Result<()> {
        unsupported()
    }

    pub fn duplicate(&self) -> io::Result<TcpStream> {
        unsupported()
    }

    pub fn set_linger(&self, _: Option<Duration>) -> io::Result<()> {
        unsupported()
    }

    pub fn linger(&self) -> io::Result<Option<Duration>> {
        unsupported()
    }

    pub fn set_nodelay(&self, _: bool) -> io::Result<()> {
        unsupported()
    }

    pub fn nodelay(&self) -> io::Result<bool> {
        unsupported()
    }

    pub fn set_ttl(&self, _: u32) -> io::Result<()> {
        unsupported()
    }

    pub fn ttl(&self) -> io::Result<u32> {
        unsupported()
    }

    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        unsupported()
    }

    pub fn set_nonblocking(&self, _: bool) -> io::Result<()> {
        unsupported()
    }
}

#[derive(Debug)]
pub struct TcpListener(!);

impl TcpListener {
    pub fn bind(_: io::Result<&SocketAddr>) -> io::Result<TcpListener> {
        unsupported()
    }

    pub fn socket_addr(&self) -> io::Result<SocketAddr> {
        self.0
    }

    pub fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        self.0
    }

    pub fn duplicate(&self) -> io::Result<TcpListener> {
        self.0
    }

    pub fn set_ttl(&self, _: u32) -> io::Result<()> {
        self.0
    }

    pub fn ttl(&self) -> io::Result<u32> {
        self.0
    }

    pub fn set_only_v6(&self, _: bool) -> io::Result<()> {
        self.0
    }

    pub fn only_v6(&self) -> io::Result<bool> {
        self.0
    }

    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        self.0
    }

    pub fn set_nonblocking(&self, _: bool) -> io::Result<()> {
        self.0
    }
}
