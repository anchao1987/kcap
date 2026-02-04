use crate::cli::Protocol;

pub fn build_filter(port: Option<u16>, protocol: Protocol, extra: Option<&str>) -> Option<String> {
    let base = port.map(|p| match protocol {
        Protocol::All => format!("port {p}"),
        Protocol::Tcp => format!("tcp port {p}"),
        Protocol::Udp => format!("udp port {p}"),
    });

    match (base, extra) {
        (Some(b), Some(e)) => Some(format!("({b}) and ({e})")),
        (Some(b), None) => Some(b),
        (None, Some(e)) => Some(e.to_string()),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_port_protocol() {
        assert_eq!(build_filter(Some(443), Protocol::Tcp, None).unwrap(), "tcp port 443");
        assert_eq!(build_filter(Some(53), Protocol::Udp, None).unwrap(), "udp port 53");
        assert_eq!(build_filter(Some(80), Protocol::All, None).unwrap(), "port 80");
    }

    #[test]
    fn filter_with_extra() {
        let f = build_filter(Some(443), Protocol::Tcp, Some("host 10.0.0.1")).unwrap();
        assert_eq!(f, "(tcp port 443) and (host 10.0.0.1)");
    }

    #[test]
    fn filter_extra_only() {
        let f = build_filter(None, Protocol::All, Some("host 10.0.0.1")).unwrap();
        assert_eq!(f, "host 10.0.0.1");
    }
}
