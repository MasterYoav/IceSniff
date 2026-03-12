use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use filter_engine::{matches_filter, matches_stream_filter, matches_transaction_filter};
use protocol_dissectors::decode_packet;
use session_model::{
    ApplicationLayerSummary, CaptureFormat, CaptureStatsReport, CapturedPacket, ConversationReport,
    ConversationRow, DecodedPacket, LinkLayerSummary, LoadedCapture, NamedCount,
    NetworkLayerSummary, PacketDetailReport, PacketListReport, PacketListRow, StreamReport,
    StreamRow, TransactionDetail, TransactionReport, TransactionRow, TransportLayerSummary,
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

pub fn streams(
    capture: &LoadedCapture,
    filter: Option<&str>,
    stream_filter: Option<&str>,
) -> Result<StreamReport, String> {
    let streams = collect_stream_accumulators(capture, filter)?
        .into_values()
        .map(|row| stream_row_from_accumulator(&row))
        .collect::<Vec<_>>();
    let streams = apply_stream_filter(streams, stream_filter)?;

    Ok(StreamReport {
        path: capture.path.clone(),
        format: capture.format.clone(),
        total_streams: streams.len() as u64,
        streams,
    })
}

pub fn stream_packet_indexes(
    capture: &LoadedCapture,
    filter: Option<&str>,
    stream_filter: &str,
) -> Result<Vec<u64>, String> {
    let mut packet_indexes = BTreeSet::new();

    for row in collect_stream_accumulators(capture, filter)?.into_values() {
        let stream_row = stream_row_from_accumulator(&row);
        if matches_stream_filter(&stream_row, stream_filter)? {
            packet_indexes.extend(row.packet_indexes);
        }
    }

    Ok(packet_indexes.into_iter().collect())
}

pub fn transactions(
    capture: &LoadedCapture,
    filter: Option<&str>,
    transaction_filter: Option<&str>,
) -> Result<TransactionReport, String> {
    let mut transactions = Vec::new();

    for row in collect_stream_accumulators(capture, filter)?.into_values() {
        match row.service.as_str() {
            "http" => transactions.extend(http_transactions(&row)),
            "tls" => transactions.extend(tls_transactions(&row)),
            _ => {}
        }
    }

    let transactions = apply_transaction_filter(transactions, transaction_filter)?;

    Ok(TransactionReport {
        path: capture.path.clone(),
        format: capture.format.clone(),
        total_transactions: transactions.len() as u64,
        transactions,
    })
}

fn apply_stream_filter(
    rows: Vec<StreamRow>,
    filter: Option<&str>,
) -> Result<Vec<StreamRow>, String> {
    let Some(expression) = filter else {
        return Ok(rows);
    };

    rows.into_iter()
        .filter_map(|row| match matches_stream_filter(&row, expression) {
            Ok(true) => Some(Ok(row)),
            Ok(false) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

fn apply_transaction_filter(
    rows: Vec<TransactionRow>,
    filter: Option<&str>,
) -> Result<Vec<TransactionRow>, String> {
    let Some(expression) = filter else {
        return Ok(rows);
    };

    rows.into_iter()
        .filter_map(|row| match matches_transaction_filter(&row, expression) {
            Ok(true) => Some(Ok(row)),
            Ok(false) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

fn stream_row_from_accumulator(row: &StreamAccumulator) -> StreamRow {
    let analysis = analyze_stream_transactions(row);
    let matched_transactions = analysis.request_count.min(analysis.response_count);
    let session_state = stream_session_state(row);

    StreamRow {
        service: row.service.clone(),
        protocol: row.protocol.clone(),
        client: row.client.clone(),
        server: row.server.clone(),
        packets: row.packets,
        syn_packets: row.syn_packets,
        fin_packets: row.fin_packets,
        rst_packets: row.rst_packets,
        session_state,
        client_to_server_packets: row.client_to_server_packets,
        server_to_client_packets: row.server_to_client_packets,
        request_count: analysis.request_count,
        response_count: analysis.response_count,
        matched_transactions,
        unmatched_requests: analysis.request_count.saturating_sub(matched_transactions),
        unmatched_responses: analysis.response_count.saturating_sub(matched_transactions),
        tls_client_hellos: analysis.tls.client_hellos,
        tls_server_hellos: analysis.tls.server_hellos,
        tls_certificates: analysis.tls.certificates,
        tls_finished_messages: analysis.tls.finished_messages,
        tls_handshake_cycles: analysis.tls.handshake_cycles,
        tls_incomplete_handshakes: analysis.tls.incomplete_handshakes,
        tls_handshake_state: analysis.tls.handshake_state,
        tls_alert_count: analysis.tls.alert_count,
        tls_alerts: analysis.tls.alerts,
        total_captured_bytes: row.total_captured_bytes,
        first_packet_index: row.first_packet_index,
        last_packet_index: row.last_packet_index,
        transaction_timeline: analysis.transaction_timeline,
        notes: analysis.notes,
    }
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

fn tcp_flag_counts(packet: &DecodedPacket) -> Option<(u64, u64, u64)> {
    let tcp = match &packet.transport {
        Some(TransportLayerSummary::Tcp(tcp)) => tcp,
        _ => return None,
    };
    let syn = u64::from((tcp.flags & 0x002) != 0);
    let fin = u64::from((tcp.flags & 0x001) != 0);
    let rst = u64::from((tcp.flags & 0x004) != 0);
    Some((syn, fin, rst))
}

fn stream_session_state(row: &StreamAccumulator) -> String {
    if row.rst_packets > 0 {
        "reset".to_string()
    } else if row.syn_packets > 0 && row.fin_packets > 0 {
        "closed".to_string()
    } else if row.syn_packets > 0 {
        "open".to_string()
    } else {
        "midstream".to_string()
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
    syn_packets: u64,
    fin_packets: u64,
    rst_packets: u64,
    client_to_server_packets: u64,
    server_to_client_packets: u64,
    request_count: u64,
    response_count: u64,
    total_captured_bytes: u64,
    first_packet_index: u64,
    last_packet_index: u64,
    packet_indexes: Vec<u64>,
    notes: Vec<String>,
    client_segments: Vec<PayloadSegment>,
    server_segments: Vec<PayloadSegment>,
}

#[derive(Debug, Clone)]
struct PayloadSegment {
    packet_index: u64,
    sequence_number: u32,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
struct ReassembledStream {
    bytes: Vec<u8>,
    packet_ranges: Vec<ReassembledPacketRange>,
}

#[derive(Debug, Clone)]
struct ReassembledPacketRange {
    output_start: usize,
    output_end: usize,
    packet_index: u64,
}

fn collect_stream_accumulators(
    capture: &LoadedCapture,
    filter: Option<&str>,
) -> Result<BTreeMap<(String, String, String), StreamAccumulator>, String> {
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
            syn_packets: 0,
            fin_packets: 0,
            rst_packets: 0,
            client_to_server_packets: 0,
            server_to_client_packets: 0,
            request_count: 0,
            response_count: 0,
            total_captured_bytes: 0,
            first_packet_index: packet.summary.index,
            last_packet_index: packet.summary.index,
            packet_indexes: Vec::new(),
            notes: Vec::new(),
            client_segments: Vec::new(),
            server_segments: Vec::new(),
        });

        row.packets += 1;
        if let Some((syn, fin, rst)) = tcp_flag_counts(&packet) {
            row.syn_packets += syn;
            row.fin_packets += fin;
            row.rst_packets += rst;
        }
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
        row.packet_indexes.push(packet.summary.index);
    }

    Ok(rows)
}

fn analyze_stream_transactions(row: &StreamAccumulator) -> StreamAnalysis {
    let mut notes = row.notes.clone();

    match row.service.as_str() {
        "http" => {
            let client_stream = reassemble_tcp_stream(&row.client_segments, "client", &mut notes);
            let server_stream = reassemble_tcp_stream(&row.server_segments, "server", &mut notes);
            let requests = parse_http_messages(
                &client_stream.bytes,
                true,
                "client",
                &mut notes,
                &client_stream.packet_ranges,
            );
            let responses = parse_http_messages(
                &server_stream.bytes,
                false,
                "server",
                &mut notes,
                &server_stream.packet_ranges,
            );
            let transaction_timeline = http_event_timeline(&requests, &responses, &mut notes);
            StreamAnalysis {
                request_count: requests.len() as u64,
                response_count: responses.len() as u64,
                tls: TlsStreamAnalysis::not_applicable(),
                transaction_timeline,
                notes,
            }
        }
        "tls" => {
            let client_stream = reassemble_tcp_stream(&row.client_segments, "client", &mut notes);
            let server_stream = reassemble_tcp_stream(&row.server_segments, "server", &mut notes);
            let tls = analyze_tls_stream(
                &client_stream.bytes,
                &server_stream.bytes,
                row.rst_packets > 0,
                &mut notes,
            );
            let client_trace = parse_tls_handshake_trace(
                &client_stream.bytes,
                "client",
                &mut notes,
                &client_stream.packet_ranges,
            );
            let server_trace = parse_tls_handshake_trace(
                &server_stream.bytes,
                "server",
                &mut notes,
                &server_stream.packet_ranges,
            );
            let transaction_timeline =
                tls_event_timeline(&client_trace.events, &server_trace.events).collect();
            notes.push(
                "TLS transaction counts reflect reassembled handshake messages only.".to_string(),
            );
            StreamAnalysis {
                request_count: tls.client_hellos,
                response_count: tls.server_hellos,
                tls,
                transaction_timeline,
                notes,
            }
        }
        _ => StreamAnalysis {
            request_count: row.request_count,
            response_count: row.response_count,
            tls: TlsStreamAnalysis::not_applicable(),
            transaction_timeline: Vec::new(),
            notes,
        },
    }
}

fn http_transactions(row: &StreamAccumulator) -> Vec<TransactionRow> {
    let mut notes = row.notes.clone();
    let client_stream = reassemble_tcp_stream(&row.client_segments, "client", &mut notes);
    let server_stream = reassemble_tcp_stream(&row.server_segments, "server", &mut notes);
    let requests = parse_http_messages(
        &client_stream.bytes,
        true,
        "client",
        &mut notes,
        &client_stream.packet_ranges,
    );
    let responses = parse_http_messages(
        &server_stream.bytes,
        false,
        "server",
        &mut notes,
        &server_stream.packet_ranges,
    );
    let total = requests.len().max(responses.len());

    (0..total)
        .map(|index| TransactionRow {
            service: row.service.clone(),
            protocol: row.protocol.clone(),
            client: row.client.clone(),
            server: row.server.clone(),
            sequence: (index + 1) as u64,
            request_summary: requests
                .get(index)
                .map(|message| message.summary.clone())
                .unwrap_or_else(|| "none".to_string()),
            request_details: requests
                .get(index)
                .map(|message| message.details.clone())
                .unwrap_or_default(),
            response_summary: responses
                .get(index)
                .map(|message| message.summary.clone())
                .unwrap_or_else(|| "none".to_string()),
            response_details: responses
                .get(index)
                .map(|message| message.details.clone())
                .unwrap_or_default(),
            state: if index < requests.len() && index < responses.len() {
                "matched".to_string()
            } else if index < requests.len() {
                "request_only".to_string()
            } else {
                "response_only".to_string()
            },
            notes: notes.clone(),
        })
        .collect()
}

fn tls_transactions(row: &StreamAccumulator) -> Vec<TransactionRow> {
    let mut notes = row.notes.clone();
    let client_stream = reassemble_tcp_stream(&row.client_segments, "client", &mut notes);
    let server_stream = reassemble_tcp_stream(&row.server_segments, "server", &mut notes);
    let client = parse_tls_handshake_trace(
        &client_stream.bytes,
        "client",
        &mut notes,
        &client_stream.packet_ranges,
    );
    let server = parse_tls_handshake_trace(
        &server_stream.bytes,
        "server",
        &mut notes,
        &server_stream.packet_ranges,
    );
    let analysis = analyze_tls_stream(
        &client_stream.bytes,
        &server_stream.bytes,
        row.rst_packets > 0,
        &mut notes,
    );
    let client_cycles = tls_client_cycles(&client.events);
    let server_cycles = tls_server_cycles(&server.events);
    let total = client.client_hellos.max(server_cycles.len() as u64);

    (0..total)
        .map(|index| TransactionRow {
            service: row.service.clone(),
            protocol: row.protocol.clone(),
            client: row.client.clone(),
            server: row.server.clone(),
            sequence: (index + 1) as u64,
            request_summary: if (index as u64) < client.client_hellos {
                "client_hello".to_string()
            } else {
                "none".to_string()
            },
            request_details: client_cycles
                .get(index as usize)
                .map(tls_client_cycle_details)
                .unwrap_or_default(),
            response_summary: server_cycles
                .get(index as usize)
                .map(tls_cycle_summary)
                .unwrap_or_else(|| "none".to_string()),
            response_details: server_cycles
                .get(index as usize)
                .map(tls_server_cycle_details)
                .unwrap_or_default(),
            state: tls_transaction_state(
                index as u64,
                &analysis,
                server_cycles.get(index as usize),
            ),
            notes: tls_transaction_notes(
                index as u64,
                &analysis,
                server_cycles.get(index as usize),
                &notes,
            ),
        })
        .collect()
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
        packet_index: packet.summary.index,
        sequence_number: tcp.sequence_number,
        bytes: packet.raw_bytes[payload_start..].to_vec(),
    })
}

fn reassemble_tcp_stream(
    segments: &[PayloadSegment],
    direction: &str,
    notes: &mut Vec<String>,
) -> ReassembledStream {
    let mut segments = segments.to_vec();
    segments.sort_by_key(|segment| (segment.sequence_number, segment.packet_index));

    let mut bytes = Vec::new();
    let mut packet_ranges = Vec::new();
    let mut next_sequence = None::<u64>;
    let mut saw_gap = false;
    let mut saw_retransmission = false;
    let mut saw_overlap = false;
    let mut saw_out_of_order = false;

    for window in segments.windows(2) {
        if window[0].packet_index > window[1].packet_index {
            saw_out_of_order = true;
            break;
        }
    }

    for segment in segments {
        if segment.bytes.is_empty() {
            continue;
        }

        let sequence = u64::from(segment.sequence_number);
        let segment_end = sequence + segment.bytes.len() as u64;

        match next_sequence {
            None => {
                let start = bytes.len();
                bytes.extend_from_slice(&segment.bytes);
                packet_ranges.push(ReassembledPacketRange {
                    output_start: start,
                    output_end: bytes.len(),
                    packet_index: segment.packet_index,
                });
                next_sequence = Some(segment_end);
            }
            Some(expected) if sequence > expected => {
                saw_gap = true;
                let start = bytes.len();
                bytes.extend_from_slice(&segment.bytes);
                packet_ranges.push(ReassembledPacketRange {
                    output_start: start,
                    output_end: bytes.len(),
                    packet_index: segment.packet_index,
                });
                next_sequence = Some(segment_end);
            }
            Some(expected) if sequence < expected => {
                let overlap = (expected - sequence) as usize;
                if overlap < segment.bytes.len() {
                    saw_overlap = true;
                    let start = bytes.len();
                    bytes.extend_from_slice(&segment.bytes[overlap..]);
                    packet_ranges.push(ReassembledPacketRange {
                        output_start: start,
                        output_end: bytes.len(),
                        packet_index: segment.packet_index,
                    });
                    next_sequence = Some(segment_end.max(expected));
                } else {
                    saw_retransmission = true;
                }
            }
            Some(_) => {
                let start = bytes.len();
                bytes.extend_from_slice(&segment.bytes);
                packet_ranges.push(ReassembledPacketRange {
                    output_start: start,
                    output_end: bytes.len(),
                    packet_index: segment.packet_index,
                });
                next_sequence = Some(segment_end);
            }
        }
    }

    if saw_gap {
        push_note_once(
            notes,
            format!("{direction} tcp stream has sequence gaps; reassembly may be incomplete."),
        );
    }
    if saw_retransmission {
        push_note_once(
            notes,
            format!("{direction} tcp stream contained retransmitted segments that were ignored."),
        );
    }
    if saw_overlap {
        push_note_once(
            notes,
            format!(
                "{direction} tcp stream contained overlapping segments that were trimmed during reassembly."
            ),
        );
    }
    if saw_out_of_order {
        push_note_once(
            notes,
            format!(
                "{direction} tcp stream contained out-of-order segments that were reordered during reassembly."
            ),
        );
    }

    ReassembledStream {
        bytes,
        packet_ranges,
    }
}

fn parse_http_messages(
    bytes: &[u8],
    expect_request: bool,
    direction: &str,
    notes: &mut Vec<String>,
    packet_ranges: &[ReassembledPacketRange],
) -> Vec<HttpMessageRecord> {
    let mut offset = 0usize;
    let mut messages = Vec::new();

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

        let body_slice = &bytes[offset + header_end + 4..];
        let Some(body_measurements) = measure_http_body(&header_text, body_slice) else {
            notes.push(format!(
                "{direction} http stream ended with an incomplete body after reassembly."
            ));
            break;
        };
        let message_len = header_end + 4 + body_measurements.framed_len;

        messages.push(HttpMessageRecord {
            summary: summarize_http_message(first_line, expect_request),
            details: summarize_http_message_details(
                &header_text,
                first_line,
                expect_request,
                body_measurements.decoded_len,
                &body_measurements.transfer_semantics,
            ),
            packet_index: Some(packet_index_for_stream_offset(packet_ranges, offset)),
        });
        offset += message_len;
    }

    messages
}

fn analyze_tls_stream(
    client_bytes: &[u8],
    server_bytes: &[u8],
    reset_seen: bool,
    notes: &mut Vec<String>,
) -> TlsStreamAnalysis {
    let client = parse_tls_handshake_trace(client_bytes, "client", notes, &[]);
    let server = parse_tls_handshake_trace(server_bytes, "server", notes, &[]);
    let handshake_cycles = client.client_hellos.min(server.server_hellos);
    let incomplete_handshakes = client
        .client_hellos
        .max(server.server_hellos)
        .saturating_sub(handshake_cycles);
    let handshake_state = derive_tls_handshake_state(
        &client,
        &server,
        handshake_cycles,
        incomplete_handshakes,
        reset_seen,
    );

    TlsStreamAnalysis {
        client_hellos: client.client_hellos,
        server_hellos: server.server_hellos,
        certificates: client.certificates + server.certificates,
        finished_messages: client.finished_messages + server.finished_messages,
        handshake_cycles,
        incomplete_handshakes,
        handshake_state,
        alert_count: client.alerts + server.alerts,
        alerts: tls_alert_labels(&client.events, &server.events),
    }
}

fn parse_tls_handshake_trace(
    bytes: &[u8],
    direction: &str,
    notes: &mut Vec<String>,
    packet_ranges: &[ReassembledPacketRange],
) -> TlsHandshakeTrace {
    let mut trace = TlsHandshakeTrace::default();
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
                match handshake_type {
                    1 => {
                        let client_hello = extract_tls_client_hello_metadata(
                            &bytes[offset..offset + 5 + record_len],
                        );
                        trace.client_hellos += 1;
                        trace.events.push(TlsHandshakeEvent::ClientHello {
                            packet_index: packet_index_for_stream_offset(packet_ranges, offset),
                            record_version: format!("{}.{}", bytes[offset + 1], bytes[offset + 2]),
                            server_name: client_hello.server_name,
                            alpn: client_hello.alpn,
                        });
                        count += 1;
                    }
                    2 => {
                        trace.server_hellos += 1;
                        trace.events.push(TlsHandshakeEvent::ServerHello {
                            packet_index: packet_index_for_stream_offset(packet_ranges, offset),
                            record_version: format!("{}.{}", bytes[offset + 1], bytes[offset + 2]),
                        });
                        count += 1;
                    }
                    11 => {
                        trace.certificates += 1;
                        trace.events.push(TlsHandshakeEvent::Certificate {
                            packet_index: packet_index_for_stream_offset(packet_ranges, offset),
                        });
                    }
                    16 => trace.events.push(TlsHandshakeEvent::ClientKeyExchange {
                        packet_index: packet_index_for_stream_offset(packet_ranges, offset),
                    }),
                    20 => {
                        trace.finished_messages += 1;
                        trace.events.push(TlsHandshakeEvent::Finished {
                            packet_index: packet_index_for_stream_offset(packet_ranges, offset),
                        });
                    }
                    _ => {}
                }
                handshake_offset += 4 + handshake_len;
            }
        } else if content_type == 21 {
            if record_len < 2 {
                notes.push(format!(
                    "{direction} tls stream ended with an incomplete alert record after reassembly."
                ));
                break;
            }
            trace.alerts += 1;
            trace.events.push(TlsHandshakeEvent::Alert {
                packet_index: packet_index_for_stream_offset(packet_ranges, offset),
                level: tls_alert_level_name(bytes[offset + 5]).to_string(),
                description: tls_alert_description_name(bytes[offset + 6]).to_string(),
            });
        }

        offset += 5 + record_len;
    }

    if count == 0 && bytes.is_empty() {
        return trace;
    }
    trace
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
    first_line.starts_with("HTTP/")
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

fn parse_http_status_line_any(line: &str) -> Option<(u16, &str)> {
    let rest = line.strip_prefix("HTTP/")?;
    let (_version, remainder) = rest.split_once(' ')?;
    let mut parts = remainder.splitn(2, ' ');
    Some((parts.next()?.parse().ok()?, parts.next().unwrap_or("")))
}

fn push_note_once(notes: &mut Vec<String>, note: String) {
    if !notes.iter().any(|existing| existing == &note) {
        notes.push(note);
    }
}

fn summarize_http_message(first_line: &str, expect_request: bool) -> String {
    if expect_request {
        let mut parts = first_line.split_whitespace();
        let method = parts.next().unwrap_or("http");
        let path = parts.next().unwrap_or("/");
        return format!("{method} {path}");
    }

    let mut parts = first_line.split_whitespace();
    let _version = parts.next().unwrap_or("HTTP/1.1");
    let status = parts.next().unwrap_or("?");
    let reason = parts.collect::<Vec<_>>().join(" ");
    if reason.is_empty() {
        status.to_string()
    } else {
        format!("{status} {reason}")
    }
}

fn summarize_http_message_details(
    header_text: &str,
    first_line: &str,
    expect_request: bool,
    body_bytes: usize,
    transfer_semantics: &str,
) -> Vec<TransactionDetail> {
    let mut details = Vec::new();
    let headers = parse_http_headers(header_text);
    let transfer_encoding = headers.get("transfer-encoding").cloned();

    if expect_request {
        let mut parts = first_line.split_whitespace();
        if let Some(method) = parts.next() {
            details.push(transaction_detail("method", method));
        }
        if let Some(path) = parts.next() {
            details.push(transaction_detail("path", path));
        }
        if let Some(host) = headers.get("host") {
            details.push(transaction_detail("host", host));
        }
    } else if let Some((status_code, reason_phrase)) = parse_http_status_line_any(first_line) {
        details.push(transaction_detail("status_code", status_code.to_string()));
        details.push(transaction_detail("reason_phrase", reason_phrase));
    }

    details.push(transaction_detail(
        "header_count",
        headers.len().to_string(),
    ));
    details.push(transaction_detail("body_bytes", body_bytes.to_string()));
    details.push(transaction_detail("transfer_semantics", transfer_semantics));
    if let Some(transfer_encoding) = &transfer_encoding {
        details.push(transaction_detail("transfer_encoding", transfer_encoding));
    }
    if let Some(content_type) = headers.get("content-type") {
        details.push(transaction_detail("content_type", content_type));
    }

    details
}

fn parse_http_headers(header_text: &str) -> BTreeMap<String, String> {
    let mut headers = BTreeMap::new();
    for line in header_text.lines().skip(1) {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
    }
    headers
}

fn measure_http_body(header_text: &str, body: &[u8]) -> Option<HttpBodyMeasurements> {
    if let Some(transfer_encoding) = parse_http_header(header_text, "transfer-encoding") {
        if transfer_encoding
            .split(',')
            .any(|value| value.trim().eq_ignore_ascii_case("chunked"))
        {
            let (framed_len, decoded_len) = parse_chunked_body(body)?;
            return Some(HttpBodyMeasurements {
                framed_len,
                decoded_len,
                transfer_semantics: "chunked".to_string(),
            });
        }
        return Some(HttpBodyMeasurements {
            framed_len: 0,
            decoded_len: 0,
            transfer_semantics: "streaming".to_string(),
        });
    }

    let content_length = parse_http_content_length(header_text).unwrap_or(0usize);
    Some(HttpBodyMeasurements {
        framed_len: content_length,
        decoded_len: content_length,
        transfer_semantics: if content_length > 0 {
            "content-length".to_string()
        } else {
            "header-only".to_string()
        },
    })
}

fn parse_chunked_body(body: &[u8]) -> Option<(usize, usize)> {
    let mut offset = 0usize;
    let mut decoded_len = 0usize;

    loop {
        let size_line_end = find_subslice(&body[offset..], b"\r\n")?;
        let size_line = std::str::from_utf8(&body[offset..offset + size_line_end]).ok()?;
        let size = usize::from_str_radix(size_line.trim(), 16).ok()?;
        offset += size_line_end + 2;

        if size == 0 {
            if body.get(offset..offset + 2)? != b"\r\n" {
                return None;
            }
            offset += 2;
            return Some((offset, decoded_len));
        }

        if offset + size + 2 > body.len() {
            return None;
        }
        decoded_len += size;
        offset += size;
        if body.get(offset..offset + 2)? != b"\r\n" {
            return None;
        }
        offset += 2;
    }
}

fn parse_http_header(header_text: &str, header_name: &str) -> Option<String> {
    for line in header_text.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case(header_name) {
            return Some(value.trim().to_string());
        }
    }
    None
}

fn packet_index_for_stream_offset(packet_ranges: &[ReassembledPacketRange], offset: usize) -> u64 {
    packet_ranges
        .iter()
        .find(|range| offset >= range.output_start && offset < range.output_end)
        .map(|range| range.packet_index)
        .or_else(|| packet_ranges.last().map(|range| range.packet_index))
        .unwrap_or(0)
}

fn transaction_detail(key: impl Into<String>, value: impl Into<String>) -> TransactionDetail {
    TransactionDetail {
        key: key.into(),
        value: value.into(),
    }
}

fn http_event_timeline(
    requests: &[HttpMessageRecord],
    responses: &[HttpMessageRecord],
    notes: &mut Vec<String>,
) -> Vec<String> {
    let mut events = Vec::new();

    for request in requests {
        events.push((
            request.packet_index.unwrap_or(u64::MAX),
            0u8,
            format!(
                "#{} client request {}",
                request.packet_index.unwrap_or(0),
                request.summary
            ),
        ));
    }
    for response in responses {
        events.push((
            response.packet_index.unwrap_or(u64::MAX),
            1u8,
            format!(
                "#{} server response {}",
                response.packet_index.unwrap_or(0),
                response.summary
            ),
        ));
    }

    events.sort_by_key(|(packet_index, side_order, _)| (*packet_index, *side_order));

    let mut outstanding_requests = 0u64;
    let mut max_outstanding_requests = 0u64;
    let mut saw_response_without_request = false;
    for (_, _, entry) in &events {
        if entry.contains("client request") {
            outstanding_requests += 1;
            max_outstanding_requests = max_outstanding_requests.max(outstanding_requests);
        } else if outstanding_requests > 0 {
            outstanding_requests -= 1;
        } else {
            saw_response_without_request = true;
        }
    }

    if max_outstanding_requests > 1 {
        notes.push(format!(
            "http stream shows pipelined requests with max in-flight depth {}.",
            max_outstanding_requests
        ));
    }
    if saw_response_without_request {
        notes.push(
            "http stream includes responses before any pending request in the reconstructed timeline."
                .to_string(),
        );
    }

    events.into_iter().map(|(_, _, entry)| entry).collect()
}

fn tls_event_timeline(
    client_events: &[TlsHandshakeEvent],
    server_events: &[TlsHandshakeEvent],
) -> std::vec::IntoIter<String> {
    let mut entries = Vec::new();

    for event in client_events.iter().chain(server_events.iter()) {
        entries.push((
            tls_event_packet_index(event),
            tls_event_sort_key(event),
            tls_event_entry(event),
        ));
    }

    entries.sort_by_key(|(packet_index, sort_key, _)| (*packet_index, *sort_key));
    entries
        .into_iter()
        .map(|(_, _, entry)| entry)
        .collect::<Vec<_>>()
        .into_iter()
}

fn tls_transaction_state(
    index: u64,
    analysis: &TlsStreamAnalysis,
    cycle: Option<&TlsServerHandshakeCycle>,
) -> String {
    if let Some(cycle) = cycle {
        if !cycle.alerts.is_empty() {
            return "alert_seen".to_string();
        }
        if cycle.finished_seen {
            return "finished_seen".to_string();
        }
        if cycle.certificate_count > 0 {
            return "certificate_seen".to_string();
        }
        if cycle.server_hello_seen {
            return "server_hello_seen".to_string();
        }
    }
    if index < analysis.handshake_cycles {
        return "server_hello_seen".to_string();
    }
    if analysis.incomplete_handshakes > 0 {
        return "incomplete".to_string();
    }
    analysis.handshake_state.clone()
}

fn tls_transaction_notes(
    index: u64,
    analysis: &TlsStreamAnalysis,
    cycle: Option<&TlsServerHandshakeCycle>,
    notes: &[String],
) -> Vec<String> {
    let mut transaction_notes = notes.to_vec();
    if let Some(cycle) = cycle {
        if cycle.certificate_count > 0 {
            transaction_notes.push("certificate_seen".to_string());
        }
        if cycle.finished_seen {
            transaction_notes.push("finished_seen".to_string());
        }
        for alert in &cycle.alerts {
            transaction_notes.push(format!("alert_seen:{alert}"));
        }
    } else if index < analysis.handshake_cycles {
        if analysis.certificates > index {
            transaction_notes.push("certificate_seen".to_string());
        }
        if analysis.finished_messages > index {
            transaction_notes.push("finished_seen".to_string());
        }
    }
    dedupe_notes(&mut transaction_notes);
    transaction_notes
}

fn tls_server_cycles(events: &[TlsHandshakeEvent]) -> Vec<TlsServerHandshakeCycle> {
    let mut cycles = Vec::new();
    let mut current = None::<TlsServerHandshakeCycle>;

    for event in events {
        match event {
            TlsHandshakeEvent::ServerHello { record_version, .. } => {
                if let Some(cycle) = current.take() {
                    cycles.push(cycle);
                }
                current = Some(TlsServerHandshakeCycle {
                    server_hello_seen: true,
                    record_version: Some(record_version.clone()),
                    certificate_count: 0,
                    finished_seen: false,
                    alerts: Vec::new(),
                });
            }
            TlsHandshakeEvent::Certificate { .. } => match current.as_mut() {
                Some(cycle) => cycle.certificate_count += 1,
                None => {
                    current = Some(TlsServerHandshakeCycle {
                        server_hello_seen: false,
                        record_version: None,
                        certificate_count: 1,
                        finished_seen: false,
                        alerts: Vec::new(),
                    });
                }
            },
            TlsHandshakeEvent::Finished { .. } => match current.as_mut() {
                Some(cycle) => cycle.finished_seen = true,
                None => {
                    current = Some(TlsServerHandshakeCycle {
                        server_hello_seen: false,
                        record_version: None,
                        certificate_count: 0,
                        finished_seen: true,
                        alerts: Vec::new(),
                    });
                }
            },
            TlsHandshakeEvent::Alert {
                level, description, ..
            } => {
                if let Some(cycle) = current.as_mut() {
                    cycle.alerts.push(format!("{level}:{description}"));
                } else {
                    current = Some(TlsServerHandshakeCycle {
                        server_hello_seen: false,
                        record_version: None,
                        certificate_count: 0,
                        finished_seen: false,
                        alerts: vec![format!("{level}:{description}")],
                    });
                }
            }
            TlsHandshakeEvent::ClientHello { .. } | TlsHandshakeEvent::ClientKeyExchange { .. } => {
            }
        }
    }

    if let Some(cycle) = current {
        cycles.push(cycle);
    }

    cycles
}

fn tls_client_cycles(events: &[TlsHandshakeEvent]) -> Vec<TlsClientHandshakeCycle> {
    let mut cycles = Vec::new();
    let mut current = None::<TlsClientHandshakeCycle>;

    for event in events {
        match event {
            TlsHandshakeEvent::ClientHello {
                record_version,
                server_name,
                alpn,
                ..
            } => {
                if let Some(cycle) = current.take() {
                    cycles.push(cycle);
                }
                current = Some(TlsClientHandshakeCycle {
                    client_hello_seen: true,
                    record_version: Some(record_version.clone()),
                    server_name: server_name.clone(),
                    alpn: alpn.clone(),
                    client_key_exchange_seen: false,
                    finished_seen: false,
                    alerts: Vec::new(),
                });
            }
            TlsHandshakeEvent::ClientKeyExchange { .. } => match current.as_mut() {
                Some(cycle) => cycle.client_key_exchange_seen = true,
                None => {
                    current = Some(TlsClientHandshakeCycle {
                        client_hello_seen: false,
                        record_version: None,
                        server_name: None,
                        alpn: None,
                        client_key_exchange_seen: true,
                        finished_seen: false,
                        alerts: Vec::new(),
                    });
                }
            },
            TlsHandshakeEvent::Finished { .. } => {
                if let Some(cycle) = current.as_mut() {
                    cycle.finished_seen = true;
                }
            }
            TlsHandshakeEvent::Alert {
                level, description, ..
            } => {
                if let Some(cycle) = current.as_mut() {
                    cycle.alerts.push(format!("{level}:{description}"));
                } else {
                    current = Some(TlsClientHandshakeCycle {
                        client_hello_seen: false,
                        record_version: None,
                        server_name: None,
                        alpn: None,
                        client_key_exchange_seen: false,
                        finished_seen: false,
                        alerts: vec![format!("{level}:{description}")],
                    });
                }
            }
            TlsHandshakeEvent::ServerHello { .. } | TlsHandshakeEvent::Certificate { .. } => {}
        }
    }

    if let Some(cycle) = current {
        cycles.push(cycle);
    }

    cycles
}

fn tls_cycle_summary(cycle: &TlsServerHandshakeCycle) -> String {
    let mut parts = Vec::new();
    if cycle.server_hello_seen {
        parts.push("server_hello");
    }
    if cycle.certificate_count > 0 {
        parts.push("certificate");
    }
    if cycle.finished_seen {
        parts.push("finished");
    }
    if !cycle.alerts.is_empty() {
        parts.push("alert");
    }

    if parts.is_empty() {
        "none".to_string()
    } else {
        parts.join(" + ")
    }
}

fn tls_client_cycle_details(cycle: &TlsClientHandshakeCycle) -> Vec<TransactionDetail> {
    let mut details = Vec::new();
    if let Some(record_version) = &cycle.record_version {
        details.push(transaction_detail("record_version", record_version));
    }
    if let Some(server_name) = &cycle.server_name {
        details.push(transaction_detail("server_name", server_name));
    }
    if let Some(alpn) = &cycle.alpn {
        details.push(transaction_detail("alpn", alpn));
    }
    details.push(transaction_detail(
        "handshake_messages",
        tls_client_cycle_messages(cycle).join(","),
    ));
    if !cycle.alerts.is_empty() {
        details.push(transaction_detail("alerts", cycle.alerts.join(",")));
    }
    details
}

fn tls_server_cycle_details(cycle: &TlsServerHandshakeCycle) -> Vec<TransactionDetail> {
    let mut details = Vec::new();
    if let Some(record_version) = &cycle.record_version {
        details.push(transaction_detail("record_version", record_version));
    }
    details.push(transaction_detail(
        "certificate_messages",
        cycle.certificate_count.to_string(),
    ));
    details.push(transaction_detail(
        "handshake_messages",
        tls_server_cycle_messages(cycle).join(","),
    ));
    if !cycle.alerts.is_empty() {
        details.push(transaction_detail("alerts", cycle.alerts.join(",")));
    }
    details
}

fn tls_client_cycle_messages(cycle: &TlsClientHandshakeCycle) -> Vec<&'static str> {
    let mut messages = Vec::new();
    if cycle.client_hello_seen {
        messages.push("client_hello");
    }
    if cycle.client_key_exchange_seen {
        messages.push("client_key_exchange");
    }
    if cycle.finished_seen {
        messages.push("finished");
    }
    if !cycle.alerts.is_empty() {
        messages.push("alert");
    }
    messages
}

fn tls_server_cycle_messages(cycle: &TlsServerHandshakeCycle) -> Vec<&'static str> {
    let mut messages = Vec::new();
    if cycle.server_hello_seen {
        messages.push("server_hello");
    }
    if cycle.certificate_count > 0 {
        messages.push("certificate");
    }
    if cycle.finished_seen {
        messages.push("finished");
    }
    if !cycle.alerts.is_empty() {
        messages.push("alert");
    }
    messages
}

fn extract_tls_client_hello_metadata(payload: &[u8]) -> TlsClientHelloMetadata {
    let mut metadata = TlsClientHelloMetadata::default();
    if payload.len() < 43 || payload.get(0) != Some(&22) || payload.get(5) != Some(&1) {
        return metadata;
    }

    let Some(session_id_len) = payload.get(43).copied().map(usize::from) else {
        return metadata;
    };
    let cipher_suites_len_offset = 44 + session_id_len;
    let Some(cipher_suites_len_bytes) =
        payload.get(cipher_suites_len_offset..cipher_suites_len_offset + 2)
    else {
        return metadata;
    };
    let cipher_suites_len = usize::from(u16::from_be_bytes([
        cipher_suites_len_bytes[0],
        cipher_suites_len_bytes[1],
    ]));
    let compression_methods_len_offset = cipher_suites_len_offset + 2 + cipher_suites_len;
    let Some(compression_methods_len) = payload
        .get(compression_methods_len_offset)
        .copied()
        .map(usize::from)
    else {
        return metadata;
    };
    let extensions_len_offset = compression_methods_len_offset + 1 + compression_methods_len;
    let Some(extensions_len_bytes) = payload.get(extensions_len_offset..extensions_len_offset + 2)
    else {
        return metadata;
    };
    let extensions_len = usize::from(u16::from_be_bytes([
        extensions_len_bytes[0],
        extensions_len_bytes[1],
    ]));
    let mut offset = extensions_len_offset + 2;
    let end = offset + extensions_len;

    while offset + 4 <= end && offset + 4 <= payload.len() {
        let extension_type = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
        let extension_len = usize::from(u16::from_be_bytes([
            payload[offset + 2],
            payload[offset + 3],
        ]));
        let extension_data_start = offset + 4;
        let extension_data_end = extension_data_start + extension_len;
        if extension_data_end > payload.len() {
            break;
        }
        match extension_type {
            0 => {
                if let Some(server_name) =
                    parse_tls_server_name_extension(payload, extension_data_start)
                {
                    metadata.server_name = Some(server_name);
                }
            }
            16 => {
                if let Some(alpn) = parse_tls_alpn_extension(payload, extension_data_start) {
                    metadata.alpn = Some(alpn);
                }
            }
            _ => {}
        }
        offset = extension_data_end;
    }

    metadata
}

fn parse_tls_server_name_extension(payload: &[u8], extension_data_start: usize) -> Option<String> {
    let list_len = usize::from(u16::from_be_bytes([
        *payload.get(extension_data_start)?,
        *payload.get(extension_data_start + 1)?,
    ]));
    if extension_data_start + 2 + list_len > payload.len() {
        return None;
    }
    let name_type = *payload.get(extension_data_start + 2)?;
    if name_type != 0 {
        return None;
    }
    let name_len = usize::from(u16::from_be_bytes([
        *payload.get(extension_data_start + 3)?,
        *payload.get(extension_data_start + 4)?,
    ]));
    let name_start = extension_data_start + 5;
    let name_end = name_start + name_len;
    Some(String::from_utf8_lossy(payload.get(name_start..name_end)?).to_string())
}

fn parse_tls_alpn_extension(payload: &[u8], extension_data_start: usize) -> Option<String> {
    let protocols_len = usize::from(u16::from_be_bytes([
        *payload.get(extension_data_start)?,
        *payload.get(extension_data_start + 1)?,
    ]));
    if extension_data_start + 2 + protocols_len > payload.len() {
        return None;
    }
    let first_len = *payload.get(extension_data_start + 2)? as usize;
    let first_start = extension_data_start + 3;
    let first_end = first_start + first_len;
    Some(String::from_utf8_lossy(payload.get(first_start..first_end)?).to_string())
}

fn tls_alert_level_name(level: u8) -> &'static str {
    match level {
        1 => "warning",
        2 => "fatal",
        _ => "unknown",
    }
}

fn tls_alert_description_name(description: u8) -> &'static str {
    match description {
        0 => "close_notify",
        10 => "unexpected_message",
        20 => "bad_record_mac",
        40 => "handshake_failure",
        42 => "bad_certificate",
        70 => "protocol_version",
        80 => "internal_error",
        _ => "unknown",
    }
}

fn tls_alert_labels(
    client_events: &[TlsHandshakeEvent],
    server_events: &[TlsHandshakeEvent],
) -> Vec<String> {
    let mut alerts = Vec::new();

    for event in client_events.iter().chain(server_events.iter()) {
        if let TlsHandshakeEvent::Alert {
            level, description, ..
        } = event
        {
            alerts.push(format!("{level}:{description}"));
        }
    }

    dedupe_notes(&mut alerts);
    alerts
}

fn tls_event_packet_index(event: &TlsHandshakeEvent) -> u64 {
    match event {
        TlsHandshakeEvent::ClientHello { packet_index, .. }
        | TlsHandshakeEvent::ServerHello { packet_index, .. }
        | TlsHandshakeEvent::Certificate { packet_index }
        | TlsHandshakeEvent::ClientKeyExchange { packet_index }
        | TlsHandshakeEvent::Finished { packet_index }
        | TlsHandshakeEvent::Alert { packet_index, .. } => *packet_index,
    }
}

fn tls_event_sort_key(event: &TlsHandshakeEvent) -> u8 {
    match event {
        TlsHandshakeEvent::ClientHello { .. } => 0,
        TlsHandshakeEvent::ServerHello { .. } => 1,
        TlsHandshakeEvent::Certificate { .. } => 2,
        TlsHandshakeEvent::ClientKeyExchange { .. } => 3,
        TlsHandshakeEvent::Finished { .. } => 4,
        TlsHandshakeEvent::Alert { .. } => 5,
    }
}

fn tls_event_entry(event: &TlsHandshakeEvent) -> String {
    match event {
        TlsHandshakeEvent::ClientHello {
            packet_index,
            server_name,
            alpn,
            ..
        } => {
            let mut details = Vec::new();
            if let Some(server_name) = server_name {
                details.push(format!("sni={server_name}"));
            }
            if let Some(alpn) = alpn {
                details.push(format!("alpn={alpn}"));
            }
            if details.is_empty() {
                format!("#{packet_index} client client_hello")
            } else {
                format!("#{packet_index} client client_hello {}", details.join(" "))
            }
        }
        TlsHandshakeEvent::ServerHello { packet_index, .. } => {
            format!("#{packet_index} server server_hello")
        }
        TlsHandshakeEvent::Certificate { packet_index } => {
            format!("#{packet_index} server certificate")
        }
        TlsHandshakeEvent::ClientKeyExchange { packet_index } => {
            format!("#{packet_index} client client_key_exchange")
        }
        TlsHandshakeEvent::Finished { packet_index } => {
            format!("#{packet_index} finished")
        }
        TlsHandshakeEvent::Alert {
            packet_index,
            level,
            description,
        } => format!("#{packet_index} alert {level}:{description}"),
    }
}

fn dedupe_notes(notes: &mut Vec<String>) {
    let mut unique = Vec::new();
    for note in notes.drain(..) {
        if !unique.iter().any(|existing| existing == &note) {
            unique.push(note);
        }
    }
    *notes = unique;
}

fn derive_tls_handshake_state(
    client: &TlsHandshakeTrace,
    server: &TlsHandshakeTrace,
    handshake_cycles: u64,
    incomplete_handshakes: u64,
    reset_seen: bool,
) -> String {
    let client_hello_seen = client.client_hellos > 0;
    let server_hello_seen = server.server_hellos > 0;
    let certificate_seen = client.certificates + server.certificates > 0;
    let finished_seen = client.finished_messages + server.finished_messages > 0;

    if !client_hello_seen && !server_hello_seen && !certificate_seen && !finished_seen {
        return "none".to_string();
    }
    if reset_seen {
        if handshake_cycles > 1 {
            return "reset_after_multiple_handshakes".to_string();
        }
        if server_hello_seen || certificate_seen || finished_seen {
            return "reset_after_handshake_progress".to_string();
        }
        if client_hello_seen {
            return "reset_after_client_hello".to_string();
        }
        return "reset".to_string();
    }
    if handshake_cycles > 1 {
        if incomplete_handshakes > 0 {
            return "multiple_handshakes_with_incomplete_tail".to_string();
        }
        return "multiple_handshakes_seen".to_string();
    }
    if incomplete_handshakes > 0 && server_hello_seen {
        return "incomplete_handshake_progress".to_string();
    }
    if incomplete_handshakes > 0 && client_hello_seen {
        return "client_hello_only".to_string();
    }
    if finished_seen {
        return "finished_seen".to_string();
    }
    if certificate_seen {
        return "certificate_seen".to_string();
    }
    if server_hello_seen {
        return "server_hello_seen".to_string();
    }
    if client_hello_seen {
        return "client_hello_only".to_string();
    }
    "none".to_string()
}

#[derive(Debug, Clone)]
struct StreamAnalysis {
    request_count: u64,
    response_count: u64,
    tls: TlsStreamAnalysis,
    transaction_timeline: Vec<String>,
    notes: Vec<String>,
}

#[derive(Debug, Clone)]
struct HttpMessageRecord {
    summary: String,
    details: Vec<TransactionDetail>,
    packet_index: Option<u64>,
}

#[derive(Debug, Clone)]
struct HttpBodyMeasurements {
    framed_len: usize,
    decoded_len: usize,
    transfer_semantics: String,
}

#[derive(Debug, Clone)]
struct TlsStreamAnalysis {
    client_hellos: u64,
    server_hellos: u64,
    certificates: u64,
    finished_messages: u64,
    handshake_cycles: u64,
    incomplete_handshakes: u64,
    handshake_state: String,
    alert_count: u64,
    alerts: Vec<String>,
}

impl TlsStreamAnalysis {
    fn not_applicable() -> Self {
        Self {
            client_hellos: 0,
            server_hellos: 0,
            certificates: 0,
            finished_messages: 0,
            handshake_cycles: 0,
            incomplete_handshakes: 0,
            handshake_state: "n/a".to_string(),
            alert_count: 0,
            alerts: Vec::new(),
        }
    }
}

#[derive(Debug, Default, Clone)]
struct TlsHandshakeTrace {
    client_hellos: u64,
    server_hellos: u64,
    certificates: u64,
    finished_messages: u64,
    alerts: u64,
    events: Vec<TlsHandshakeEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TlsHandshakeEvent {
    ClientHello {
        packet_index: u64,
        record_version: String,
        server_name: Option<String>,
        alpn: Option<String>,
    },
    ServerHello {
        packet_index: u64,
        record_version: String,
    },
    Certificate {
        packet_index: u64,
    },
    ClientKeyExchange {
        packet_index: u64,
    },
    Finished {
        packet_index: u64,
    },
    Alert {
        packet_index: u64,
        level: String,
        description: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TlsServerHandshakeCycle {
    server_hello_seen: bool,
    record_version: Option<String>,
    certificate_count: u64,
    finished_seen: bool,
    alerts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TlsClientHandshakeCycle {
    client_hello_seen: bool,
    record_version: Option<String>,
    server_name: Option<String>,
    alpn: Option<String>,
    client_key_exchange_seen: bool,
    finished_seen: bool,
    alerts: Vec<String>,
}

#[derive(Debug, Default, Clone)]
struct TlsClientHelloMetadata {
    server_name: Option<String>,
    alpn: Option<String>,
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
