use std::net::IpAddr;

pub fn is_valid_ip(ip: &IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(ip) => {
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
