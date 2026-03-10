use std::collections::BTreeMap;
use std::path::Path;

use filter_engine::matches_filter;
use protocol_dissectors::decode_packet;
use session_model::{
    ApplicationLayerSummary, CaptureFormat, CaptureStatsReport, CapturedPacket, ConversationReport,
    ConversationRow, DecodedPacket, LinkLayerSummary, LoadedCapture, NamedCount,
    NetworkLayerSummary, PacketDetailReport, PacketListReport, PacketListRow, StreamReport,
    StreamRow, TransportLayerSummary,
};

pub fn list_packets(
    capture: &LoadedCapture,
    limit: Option<usize>,
    filter: Option<&str>,
) -> Result<PacketListReport, String> {
    let decoded = filtered_packets(capture, filter)?;
    let total_packets = decoded.len() as u64;
    let packets = match limit {
        Some(limit) => decoded
            .into_iter()
            .take(limit)
            .map(packet_list_row)
            .collect(),
        None => decoded.into_iter().map(packet_list_row).collect(),
    };

    Ok(PacketListReport {
        path: capture.path.clone(),
        format: capture.format.clone(),
        total_packets,
        packets,
    })
}

pub fn inspect_packet(
    capture: &LoadedCapture,
    packet_index: u64,
) -> Result<PacketDetailReport, String> {
    let packet = capture
        .packets
        .iter()
        .find(|packet| packet.summary.index == packet_index)
        .ok_or_else(|| format!("packet index {packet_index} does not exist"))?;

    Ok(PacketDetailReport {
        path: capture.path.clone(),
        format: capture.format.clone(),
        packet: decode_captured_packet(packet),
    })
}

pub fn capture_stats(
    capture: &LoadedCapture,
    filter: Option<&str>,
) -> Result<CaptureStatsReport, String> {
    let decoded = filtered_packets(capture, filter)?;
    let total_packets = decoded.len() as u64;
    let total_captured_bytes = decoded
        .iter()
        .map(|packet| u64::from(packet.summary.captured_length))
        .sum::<u64>();
    let average_captured_bytes = if total_packets == 0 {
        0
    } else {
        total_captured_bytes / total_packets
    };

    let mut link_counts = BTreeMap::new();
    let mut network_counts = BTreeMap::new();
    let mut transport_counts = BTreeMap::new();

    for decoded in &decoded {
        increment_count(&mut link_counts, link_layer_name(&decoded.link));
        increment_count(
            &mut network_counts,
            network_layer_name(decoded.network.as_ref()),
        );
        increment_count(
            &mut transport_counts,
            transport_layer_name(decoded.transport.as_ref()),
        );
    }

    Ok(CaptureStatsReport {
        path: capture.path.clone(),
        format: capture.format.clone(),
        total_packets,
        total_captured_bytes,
        average_captured_bytes,
        link_layer_counts: into_named_counts(link_counts),
        network_layer_counts: into_named_counts(network_counts),
        transport_layer_counts: into_named_counts(transport_counts),
    })
}

pub fn conversations(
    capture: &LoadedCapture,
    filter: Option<&str>,
) -> Result<ConversationReport, String> {
    let decoded = filtered_packets(capture, filter)?;
    let mut rows = BTreeMap::<(String, String, String), ConversationRow>::new();

    for packet in decoded {
        let protocol = packet_protocol(&packet);
        let service = packet_service(&packet);
        let (source_endpoint, destination_endpoint) = packet_directional_endpoints(&packet);
        let (endpoint_a, endpoint_b) =
            ordered_endpoints(source_endpoint.clone(), destination_endpoint.clone());
        let key = (protocol.clone(), endpoint_a.clone(), endpoint_b.clone());
        let source_is_a = source_endpoint == endpoint_a;
        let (request_count, response_count) = packet_request_response_counts(&packet);

        match rows.get_mut(&key) {
            Some(row) => {
                row.packets += 1;
                if source_is_a {
                    row.packets_a_to_b += 1;
                } else {
                    row.packets_b_to_a += 1;
                }
                row.request_count += request_count;
                row.response_count += response_count;
                row.total_captured_bytes += u64::from(packet.summary.captured_length);
                row.first_packet_index = row.first_packet_index.min(packet.summary.index);
                row.last_packet_index = row.last_packet_index.max(packet.summary.index);
            }
            None => {
                rows.insert(
                    key,
                    ConversationRow {
                        service,
                        protocol,
                        endpoint_a,
                        endpoint_b,
                        packets: 1,
                        packets_a_to_b: if source_is_a { 1 } else { 0 },
                        packets_b_to_a: if source_is_a { 0 } else { 1 },
                        request_count,
                        response_count,
                        total_captured_bytes: u64::from(packet.summary.captured_length),
                        first_packet_index: packet.summary.index,
                        last_packet_index: packet.summary.index,
                    },
                );
            }
        }
    }

    let conversations = rows.into_values().collect::<Vec<_>>();
    Ok(ConversationReport {
        path: capture.path.clone(),
        format: capture.format.clone(),
        total_conversations: conversations.len() as u64,
        conversations,
    })
}

pub fn streams(capture: &LoadedCapture, filter: Option<&str>) -> Result<StreamReport, String> {
    let decoded = filtered_packets(capture, filter)?;
    let mut rows = BTreeMap::<(String, String, String), StreamAccumulator>::new();

    for packet in decoded {
        let service = packet_service(&packet);
        let protocol = stream_protocol(&packet, &service);
        let (source_endpoint, destination_endpoint) = packet_directional_endpoints(&packet);
        let (client, server) = client_server_endpoints(&packet, &service);
        let key = (protocol.clone(), client.clone(), server.clone());
        let from_client = source_endpoint == client && destination_endpoint == server;
        let (request_count, response_count) = packet_request_response_counts(&packet);

        let row = rows.entry(key).or_insert_with(|| StreamAccumulator {
            service,
            protocol,
            client,
            server,
            packets: 0,
            client_to_server_packets: 0,
            server_to_client_packets: 0,
            request_count: 0,
            response_count: 0,
            total_captured_bytes: 0,
            first_packet_index: packet.summary.index,
            last_packet_index: packet.summary.index,
            notes: Vec::new(),
            client_segments: Vec::new(),
            server_segments: Vec::new(),
        });

        row.packets += 1;
        if from_client {
            row.client_to_server_packets += 1;
            if let Some(segment) = tcp_payload_segment(&packet) {
                row.client_segments.push(segment);
            }
        } else {
            row.server_to_client_packets += 1;
            if let Some(segment) = tcp_payload_segment(&packet) {
                row.server_segments.push(segment);
            }
        }
        row.request_count += request_count;
        row.response_count += response_count;
        row.total_captured_bytes += u64::from(packet.summary.captured_length);
        row.first_packet_index = row.first_packet_index.min(packet.summary.index);
        row.last_packet_index = row.last_packet_index.max(packet.summary.index);
    }

    let streams = rows
        .into_values()
        .map(|row| {
            let (request_count, response_count, notes) = analyze_stream_transactions(&row);
            let matched_transactions = request_count.min(response_count);
            StreamRow {
                service: row.service,
                protocol: row.protocol,
                client: row.client,
                server: row.server,
                packets: row.packets,
                client_to_server_packets: row.client_to_server_packets,
                server_to_client_packets: row.server_to_client_packets,
                request_count,
                response_count,
                matched_transactions,
                unmatched_requests: request_count.saturating_sub(matched_transactions),
                unmatched_responses: response_count.saturating_sub(matched_transactions),
                total_captured_bytes: row.total_captured_bytes,
                first_packet_index: row.first_packet_index,
                last_packet_index: row.last_packet_index,
                notes,
            }
        })
        .collect::<Vec<_>>();

    Ok(StreamReport {
        path: capture.path.clone(),
        format: capture.format.clone(),
        total_streams: streams.len() as u64,
        streams,
    })
}

pub fn decode_captured_packet(packet: &CapturedPacket) -> DecodedPacket {
    decode_packet(packet.summary.clone(), &packet.raw_bytes, packet.linktype)
}

pub fn inspect_metadata(
    path: &Path,
    format: CaptureFormat,
    packet_count_hint: Option<u64>,
    size_bytes: u64,
) -> session_model::CaptureReport {
    session_model::CaptureReport {
        path: path.to_path_buf(),
        size_bytes,
        format: format.clone(),
        packet_count_hint,
        notes: metadata_notes(&format, size_bytes),
    }
}

fn metadata_notes(format: &CaptureFormat, size_bytes: u64) -> Vec<String> {
    let mut notes = Vec::new();
    match format {
        CaptureFormat::Pcap => {
            notes.push("Detected legacy PCAP container from magic number.".to_string());
            if size_bytes < 24 {
                notes.push("File is shorter than a complete PCAP global header.".to_string());
            } else {
                notes.push(
                    "Packet listing is available through the shared PCAP reader.".to_string(),
                );
                notes.push("Packet detail inspection is available for PCAP with minimal protocol decoding.".to_string());
            }
        }
        CaptureFormat::PcapNg => {
            notes.push("Detected PCAPNG section header block magic number.".to_string());
            notes.push("Packet listing is available through the shared PCAPNG reader.".to_string());
            notes.push("Packet detail inspection is available for common PCAPNG interface and enhanced-packet blocks.".to_string());
        }
        CaptureFormat::Unknown => {
            notes.push(
                "Unknown capture container; packet decoding is not implemented yet.".to_string(),
            );
        }
    }
    if size_bytes == 0 {
        notes.push("File is empty.".to_string());
    }
    notes
}

fn filtered_packets(
    capture: &LoadedCapture,
    filter: Option<&str>,
) -> Result<Vec<DecodedPacket>, String> {
    let mut packets = Vec::new();
    for packet in &capture.packets {
        let decoded = decode_captured_packet(packet);
        if let Some(filter) = filter {
            if !matches_filter(&decoded, filter)? {
                continue;
            }
        }
        packets.push(decoded);
    }
    Ok(packets)
}

fn packet_list_row(packet: DecodedPacket) -> PacketListRow {
    let (source, destination) = packet_addresses(&packet);
    let protocol = packet_protocol(&packet);
    let info = packet_info(&packet);

    PacketListRow {
        summary: packet.summary,
        source,
        destination,
        protocol,
        info,
    }
}

fn packet_addresses(packet: &DecodedPacket) -> (String, String) {
    match &packet.network {
        Some(NetworkLayerSummary::Ipv4(ipv4)) => {
            (ipv4.source_ip.clone(), ipv4.destination_ip.clone())
        }
        Some(NetworkLayerSummary::Arp(arp)) => (
            arp.sender_protocol_address.clone(),
            arp.target_protocol_address.clone(),
        ),
        None => ("n/a".to_string(), "n/a".to_string()),
    }
}

fn packet_protocol(packet: &DecodedPacket) -> String {
    if let Some(app) = &packet.application {
        match app {
            ApplicationLayerSummary::Dns(_) => "dns".to_string(),
            ApplicationLayerSummary::Http(_) => "http".to_string(),
            ApplicationLayerSummary::TlsHandshake(_) => "tls".to_string(),
        }
    } else {
        match &packet.transport {
            Some(TransportLayerSummary::Tcp(_)) => "tcp".to_string(),
            Some(TransportLayerSummary::Udp(_)) => "udp".to_string(),
            Some(TransportLayerSummary::Icmp(_)) => "icmp".to_string(),
            None => match &packet.network {
                Some(NetworkLayerSummary::Arp(_)) => "arp".to_string(),
                Some(NetworkLayerSummary::Ipv4(_)) => "ipv4".to_string(),
                None => "unknown".to_string(),
            },
        }
    }
}

fn packet_info(packet: &DecodedPacket) -> String {
    match &packet.application {
        Some(ApplicationLayerSummary::Dns(dns)) => format!(
            "dns {}",
            dns.questions.first().map(String::as_str).unwrap_or("query")
        ),
        Some(ApplicationLayerSummary::Http(http)) => format!(
            "{} {}",
            http.method.as_deref().unwrap_or("http"),
            http.path.as_deref().unwrap_or("/")
        ),
        Some(ApplicationLayerSummary::TlsHandshake(tls)) => format!(
            "tls {} {}",
            tls.handshake_type,
            tls.server_name.as_deref().unwrap_or("")
        )
        .trim()
        .to_string(),
        None => match &packet.transport {
            Some(TransportLayerSummary::Tcp(tcp)) => {
                format!("{} -> {}", tcp.source_port, tcp.destination_port)
            }
            Some(TransportLayerSummary::Udp(udp)) => {
                format!("{} -> {}", udp.source_port, udp.destination_port)
            }
            Some(TransportLayerSummary::Icmp(icmp)) => {
                format!("type={} code={}", icmp.icmp_type, icmp.code)
            }
            None => "n/a".to_string(),
        },
    }
}

fn packet_directional_endpoints(packet: &DecodedPacket) -> (String, String) {
    let (source, destination) = packet_addresses(packet);
    match &packet.transport {
        Some(TransportLayerSummary::Tcp(tcp)) => (
            format!("{source}:{}", tcp.source_port),
            format!("{destination}:{}", tcp.destination_port),
        ),
        Some(TransportLayerSummary::Udp(udp)) => (
            format!("{source}:{}", udp.source_port),
            format!("{destination}:{}", udp.destination_port),
        ),
        _ => (source, destination),
    }
}

fn ordered_endpoints(left: String, right: String) -> (String, String) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

fn client_server_endpoints(packet: &DecodedPacket, service: &str) -> (String, String) {
    let (source_endpoint, destination_endpoint) = packet_directional_endpoints(packet);

    if let Some((client, server)) =
        role_from_application(packet, &source_endpoint, &destination_endpoint)
    {
        return (client, server);
    }

    if let Some((client, server)) =
        role_from_transport(packet, &source_endpoint, &destination_endpoint, service)
    {
        return (client, server);
    }

    (source_endpoint, destination_endpoint)
}

fn packet_request_response_counts(packet: &DecodedPacket) -> (u64, u64) {
    match &packet.application {
        Some(ApplicationLayerSummary::Dns(dns)) => {
            if dns.is_response {
                (0, 1)
            } else {
                (1, 0)
            }
        }
        Some(ApplicationLayerSummary::Http(http)) => {
            if http.kind == "response" {
                (0, 1)
            } else {
                (1, 0)
            }
        }
        Some(ApplicationLayerSummary::TlsHandshake(tls)) => match tls.handshake_type.as_str() {
            "client_hello" => (1, 0),
            "server_hello" => (0, 1),
            _ => (0, 0),
        },
        None => (0, 0),
    }
}

fn packet_service(packet: &DecodedPacket) -> String {
    match &packet.application {
        Some(ApplicationLayerSummary::Dns(_)) => "dns".to_string(),
        Some(ApplicationLayerSummary::Http(_)) => "http".to_string(),
        Some(ApplicationLayerSummary::TlsHandshake(_)) => "tls".to_string(),
        None => match &packet.transport {
            Some(TransportLayerSummary::Tcp(tcp)) => {
                guess_service_from_ports("tcp", tcp.source_port, tcp.destination_port)
            }
            Some(TransportLayerSummary::Udp(udp)) => {
                guess_service_from_ports("udp", udp.source_port, udp.destination_port)
            }
            Some(TransportLayerSummary::Icmp(_)) => "icmp".to_string(),
            None => packet_protocol(packet),
        },
    }
}

fn stream_protocol(packet: &DecodedPacket, service: &str) -> String {
    match service {
        "dns" | "http" | "tls" | "mdns" => service.to_string(),
        _ => packet_protocol(packet),
    }
}

fn role_from_application(
    packet: &DecodedPacket,
    source_endpoint: &str,
    destination_endpoint: &str,
) -> Option<(String, String)> {
    match &packet.application {
        Some(ApplicationLayerSummary::Dns(dns)) => {
            if dns.is_response {
                Some((
                    destination_endpoint.to_string(),
                    source_endpoint.to_string(),
                ))
            } else {
                Some((
                    source_endpoint.to_string(),
                    destination_endpoint.to_string(),
                ))
            }
        }
        Some(ApplicationLayerSummary::Http(http)) => {
            if http.kind == "response" {
                Some((
                    destination_endpoint.to_string(),
                    source_endpoint.to_string(),
                ))
            } else {
                Some((
                    source_endpoint.to_string(),
                    destination_endpoint.to_string(),
                ))
            }
        }
        Some(ApplicationLayerSummary::TlsHandshake(tls)) => match tls.handshake_type.as_str() {
            "client_hello" => Some((
                source_endpoint.to_string(),
                destination_endpoint.to_string(),
            )),
            "server_hello" => Some((
                destination_endpoint.to_string(),
                source_endpoint.to_string(),
            )),
            _ => None,
        },
        None => None,
    }
}

fn role_from_transport(
    packet: &DecodedPacket,
    source_endpoint: &str,
    destination_endpoint: &str,
    service: &str,
) -> Option<(String, String)> {
    match &packet.transport {
        Some(TransportLayerSummary::Tcp(tcp)) => {
            if is_server_port(service, tcp.source_port)
                && !is_server_port(service, tcp.destination_port)
            {
                Some((
                    destination_endpoint.to_string(),
                    source_endpoint.to_string(),
                ))
            } else if is_server_port(service, tcp.destination_port) {
                Some((
                    source_endpoint.to_string(),
                    destination_endpoint.to_string(),
                ))
            } else {
                None
            }
        }
        Some(TransportLayerSummary::Udp(udp)) => {
            if is_server_port(service, udp.source_port)
                && !is_server_port(service, udp.destination_port)
            {
                Some((
                    destination_endpoint.to_string(),
                    source_endpoint.to_string(),
                ))
            } else if is_server_port(service, udp.destination_port) {
                Some((
                    source_endpoint.to_string(),
                    destination_endpoint.to_string(),
                ))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn is_server_port(service: &str, port: u16) -> bool {
    match service {
        "dns" => port == 53,
        "mdns" => port == 5353,
        "http" => port == 80,
        "tls" => port == 443,
        _ => port < 1024,
    }
}

fn guess_service_from_ports(fallback: &str, source_port: u16, destination_port: u16) -> String {
    let low_port = source_port.min(destination_port);
    match low_port {
        53 => "dns".to_string(),
        80 => "http".to_string(),
        443 => "tls".to_string(),
        5353 => "mdns".to_string(),
        _ => fallback.to_string(),
    }
}

#[derive(Debug, Clone)]
struct StreamAccumulator {
    service: String,
    protocol: String,
    client: String,
    server: String,
    packets: u64,
    client_to_server_packets: u64,
    server_to_client_packets: u64,
    request_count: u64,
    response_count: u64,
    total_captured_bytes: u64,
    first_packet_index: u64,
    last_packet_index: u64,
    notes: Vec<String>,
    client_segments: Vec<PayloadSegment>,
    server_segments: Vec<PayloadSegment>,
}

#[derive(Debug, Clone)]
struct PayloadSegment {
    sequence_number: u32,
    bytes: Vec<u8>,
}

fn analyze_stream_transactions(row: &StreamAccumulator) -> (u64, u64, Vec<String>) {
    let mut notes = row.notes.clone();

    match row.service.as_str() {
        "http" => {
            let client_bytes = reassemble_tcp_segments(&row.client_segments, "client", &mut notes);
            let server_bytes = reassemble_tcp_segments(&row.server_segments, "server", &mut notes);
            let requests = count_http_messages(&client_bytes, true, "client", &mut notes);
            let responses = count_http_messages(&server_bytes, false, "server", &mut notes);
            (requests, responses, notes)
        }
        "tls" => {
            let client_bytes = reassemble_tcp_segments(&row.client_segments, "client", &mut notes);
            let server_bytes = reassemble_tcp_segments(&row.server_segments, "server", &mut notes);
            let requests = count_tls_handshake_messages(&client_bytes, true, "client", &mut notes);
            let responses =
                count_tls_handshake_messages(&server_bytes, false, "server", &mut notes);
            notes.push(
                "TLS transaction counts reflect reassembled handshake messages only.".to_string(),
            );
            (requests, responses, notes)
        }
        _ => (row.request_count, row.response_count, notes),
    }
}

fn tcp_payload_segment(packet: &DecodedPacket) -> Option<PayloadSegment> {
    let ipv4 = match &packet.network {
        Some(NetworkLayerSummary::Ipv4(ipv4)) => ipv4,
        _ => return None,
    };
    let tcp = match &packet.transport {
        Some(TransportLayerSummary::Tcp(tcp)) => tcp,
        _ => return None,
    };
    let link_header_len = match packet.link {
        LinkLayerSummary::Ethernet(_) => 14usize,
        LinkLayerSummary::Unknown => return None,
    };
    let ip_header_len = usize::from(ipv4.header_length);
    let tcp_start = link_header_len.checked_add(ip_header_len)?;
    if packet.raw_bytes.len() < tcp_start + 20 {
        return None;
    }
    let tcp_header_len = usize::from(packet.raw_bytes[tcp_start + 12] >> 4) * 4;
    if tcp_header_len < 20 || packet.raw_bytes.len() < tcp_start + tcp_header_len {
        return None;
    }
    let payload_start = tcp_start + tcp_header_len;
    Some(PayloadSegment {
        sequence_number: tcp.sequence_number,
        bytes: packet.raw_bytes[payload_start..].to_vec(),
    })
}

fn reassemble_tcp_segments(
    segments: &[PayloadSegment],
    direction: &str,
    notes: &mut Vec<String>,
) -> Vec<u8> {
    let mut segments = segments.to_vec();
    segments.sort_by_key(|segment| segment.sequence_number);

    let mut bytes = Vec::new();
    let mut next_sequence = None::<u64>;
    let mut noted_gap = false;

    for segment in segments {
        if segment.bytes.is_empty() {
            continue;
        }

        let sequence = u64::from(segment.sequence_number);
        let segment_end = sequence + segment.bytes.len() as u64;

        match next_sequence {
            None => {
                bytes.extend_from_slice(&segment.bytes);
                next_sequence = Some(segment_end);
            }
            Some(expected) if sequence > expected => {
                if !noted_gap {
                    notes.push(format!(
                        "{direction} tcp stream has sequence gaps; reassembly may be incomplete."
                    ));
                    noted_gap = true;
                }
                bytes.extend_from_slice(&segment.bytes);
                next_sequence = Some(segment_end);
            }
            Some(expected) if sequence < expected => {
                let overlap = (expected - sequence) as usize;
                if overlap < segment.bytes.len() {
                    bytes.extend_from_slice(&segment.bytes[overlap..]);
                    next_sequence = Some(segment_end.max(expected));
                }
            }
            Some(_) => {
                bytes.extend_from_slice(&segment.bytes);
                next_sequence = Some(segment_end);
            }
        }
    }

    bytes
}

fn count_http_messages(
    bytes: &[u8],
    expect_request: bool,
    direction: &str,
    notes: &mut Vec<String>,
) -> u64 {
    let mut count = 0u64;
    let mut offset = 0usize;

    while offset < bytes.len() {
        let header_end = match find_subslice(&bytes[offset..], b"\r\n\r\n") {
            Some(end) => end,
            None => {
                notes.push(format!(
                    "{direction} http stream ended with an incomplete header after reassembly."
                ));
                break;
            }
        };
        let header_slice = &bytes[offset..offset + header_end];
        let header_text = String::from_utf8_lossy(header_slice);
        let first_line = header_text.lines().next().unwrap_or_default();

        if expect_request && !looks_like_http_request(first_line) {
            notes.push(format!(
                "{direction} http stream did not start with a recognizable request line."
            ));
            break;
        }
        if !expect_request && !looks_like_http_response(first_line) {
            notes.push(format!(
                "{direction} http stream did not start with a recognizable response line."
            ));
            break;
        }

        let content_length = parse_http_content_length(&header_text).unwrap_or(0usize);
        let message_len = header_end + 4 + content_length;
        if offset + message_len > bytes.len() {
            notes.push(format!(
                "{direction} http stream ended with an incomplete body after reassembly."
            ));
            break;
        }

        count += 1;
        offset += message_len;
    }

    count
}

fn count_tls_handshake_messages(
    bytes: &[u8],
    expect_client: bool,
    direction: &str,
    notes: &mut Vec<String>,
) -> u64 {
    let mut count = 0u64;
    let mut offset = 0usize;

    while offset + 5 <= bytes.len() {
        let content_type = bytes[offset];
        let record_len = u16::from_be_bytes([bytes[offset + 3], bytes[offset + 4]]) as usize;
        if offset + 5 + record_len > bytes.len() {
            notes.push(format!(
                "{direction} tls stream ended with an incomplete record after reassembly."
            ));
            break;
        }

        if content_type == 22 {
            let mut handshake_offset = offset + 5;
            let handshake_end = offset + 5 + record_len;
            while handshake_offset + 4 <= handshake_end {
                let handshake_type = bytes[handshake_offset];
                let handshake_len = ((usize::from(bytes[handshake_offset + 1])) << 16)
                    | ((usize::from(bytes[handshake_offset + 2])) << 8)
                    | usize::from(bytes[handshake_offset + 3]);
                if handshake_offset + 4 + handshake_len > handshake_end {
                    notes.push(format!(
                        "{direction} tls stream ended with an incomplete handshake message after reassembly."
                    ));
                    break;
                }
                if expect_client && handshake_type == 1 {
                    count += 1;
                }
                if !expect_client && handshake_type == 2 {
                    count += 1;
                }
                handshake_offset += 4 + handshake_len;
            }
        }

        offset += 5 + record_len;
    }

    count
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn looks_like_http_request(first_line: &str) -> bool {
    [
        "GET ", "POST ", "PUT ", "DELETE ", "HEAD ", "OPTIONS ", "PATCH ",
    ]
    .iter()
    .any(|prefix| first_line.starts_with(prefix))
}

fn looks_like_http_response(first_line: &str) -> bool {
    first_line.starts_with("HTTP/1.")
}

fn parse_http_content_length(header_text: &str) -> Option<usize> {
    for line in header_text.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case("content-length") {
            return value.trim().parse::<usize>().ok();
        }
    }
    None
}

fn increment_count(counts: &mut BTreeMap<String, u64>, key: String) {
    *counts.entry(key).or_insert(0) += 1;
}

fn into_named_counts(counts: BTreeMap<String, u64>) -> Vec<NamedCount> {
    counts
        .into_iter()
        .map(|(name, count)| NamedCount { name, count })
        .collect()
}

fn link_layer_name(link: &LinkLayerSummary) -> String {
    match link {
        LinkLayerSummary::Ethernet(_) => "ethernet".to_string(),
        LinkLayerSummary::Unknown => "unknown".to_string(),
    }
}

fn network_layer_name(network: Option<&NetworkLayerSummary>) -> String {
    match network {
        Some(NetworkLayerSummary::Arp(_)) => "arp".to_string(),
        Some(NetworkLayerSummary::Ipv4(_)) => "ipv4".to_string(),
        None => "none".to_string(),
    }
}

fn transport_layer_name(transport: Option<&TransportLayerSummary>) -> String {
    match transport {
        Some(TransportLayerSummary::Tcp(_)) => "tcp".to_string(),
        Some(TransportLayerSummary::Udp(_)) => "udp".to_string(),
        Some(TransportLayerSummary::Icmp(_)) => "icmp".to_string(),
        None => "none".to_string(),
    }
}
