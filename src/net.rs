use std::io::{Error, Result};
use std::net::{IpAddr, ToSocketAddrs, UdpSocket};

pub fn is_valid_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            !(ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_unspecified())
        }
        _ => false,
    }
}

pub fn is_valid_port(port: u32) -> bool {
    (port > 0) && (port < 0xffff)
}

#[cfg(not(windows))]
#[inline(always)]
pub fn bind_udp_socket(addr: impl ToSocketAddrs) -> Result<UdpSocket> {
    UdpSocket::bind(addr)
}

// Suppress WSAECONNRESET on UDP sockets, see the link below:
// https://stackoverflow.com/questions/34242622/windows-udp-sockets-recvfrom-fails-with-error-10054
#[cfg(windows)]
pub fn bind_udp_socket(addr: impl ToSocketAddrs) -> Result<UdpSocket> {
    use std::os::windows::prelude::AsRawSocket;
    use winapi::shared::minwindef::{BOOL, DWORD, FALSE, LPDWORD, LPVOID};
    use winapi::um::mswsock::SIO_UDP_CONNRESET;
    use winapi::um::winsock2::{WSAGetLastError, WSAIoctl, SOCKET, SOCKET_ERROR};

    let socket = UdpSocket::bind(addr)?;
    let handle = socket.as_raw_socket() as SOCKET;

    let ret = unsafe {
        let mut bytes_returned: DWORD = 0;
        let mut enable: BOOL = FALSE;
        WSAIoctl(
            handle,
            SIO_UDP_CONNRESET,
            &mut enable as *mut _ as LPVOID,
            std::mem::size_of_val(&enable) as DWORD,
            std::ptr::null_mut(),
            0,
            &mut bytes_returned as *mut _ as LPDWORD,
            std::ptr::null_mut(),
            None,
        )
    };

    if ret != SOCKET_ERROR {
        Ok(socket)
    } else {
        let code = unsafe { WSAGetLastError() };
        Err(Error::from_raw_os_error(code))
    }
}
