use session_model::{
    ApplicationLayerSummary, DecodedPacket, NetworkLayerSummary, TransportLayerSummary,
};

pub fn matches_filter(packet: &DecodedPacket, expression: &str) -> Result<bool, String> {
    for clause in expression
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let (key, value) = clause
            .split_once('=')
            .ok_or_else(|| format!("invalid filter clause: {clause}"))?;
        if !matches_clause(packet, key.trim(), value.trim())? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn matches_clause(packet: &DecodedPacket, key: &str, value: &str) -> Result<bool, String> {
    match key {
        "protocol" => Ok(matches_protocol(packet, value)),
        "port" => {
            let port = value
                .parse::<u16>()
                .map_err(|_| format!("invalid port value: {value}"))?;
            Ok(matches_port(packet, port))
        }
        "host" => Ok(matches_host(packet, value)),
        _ => Err(format!("unsupported filter key: {key}")),
    }
}

fn matches_protocol(packet: &DecodedPacket, value: &str) -> bool {
    match value {
        "arp" => matches!(packet.network, Some(NetworkLayerSummary::Arp(_))),
        "ipv4" => matches!(packet.network, Some(NetworkLayerSummary::Ipv4(_))),
        "tcp" => matches!(packet.transport, Some(TransportLayerSummary::Tcp(_))),
        "udp" => matches!(packet.transport, Some(TransportLayerSummary::Udp(_))),
        "icmp" => matches!(packet.transport, Some(TransportLayerSummary::Icmp(_))),
        "dns" => matches!(packet.application, Some(ApplicationLayerSummary::Dns(_))),
        "http" => matches!(packet.application, Some(ApplicationLayerSummary::Http(_))),
        "tls" => matches!(
            packet.application,
            Some(ApplicationLayerSummary::TlsHandshake(_))
        ),
        _ => false,
    }
}

fn matches_port(packet: &DecodedPacket, port: u16) -> bool {
    match &packet.transport {
        Some(TransportLayerSummary::Tcp(tcp)) => {
            tcp.source_port == port || tcp.destination_port == port
        }
        Some(TransportLayerSummary::Udp(udp)) => {
            udp.source_port == port || udp.destination_port == port
        }
        _ => false,
    }
}

fn matches_host(packet: &DecodedPacket, host: &str) -> bool {
    match &packet.application {
        Some(ApplicationLayerSummary::Dns(dns)) => dns.questions.iter().any(|name| name == host),
        Some(ApplicationLayerSummary::Http(http)) => http.host.as_deref() == Some(host),
        Some(ApplicationLayerSummary::TlsHandshake(tls)) => {
            tls.server_name.as_deref() == Some(host)
        }
        _ => false,
    }
}
