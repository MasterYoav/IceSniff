use session_model::{
    ApplicationLayerSummary, CaptureFormat, CaptureReport, CaptureStatsReport, ConversationReport,
    FieldNode, LinkLayerSummary, NamedCount, NetworkLayerSummary, PacketDetailReport,
    PacketListReport, TimestampPrecision, TransportLayerSummary,
};

pub fn render_capture_report(report: &CaptureReport) -> String {
    let format = match report.format {
        CaptureFormat::Pcap => "pcap",
        CaptureFormat::PcapNg => "pcapng",
        CaptureFormat::Unknown => "unknown",
    };

    let packet_count = report
        .packet_count_hint
        .map(|count| count.to_string())
        .unwrap_or_else(|| "n/a".to_string());

    let notes = if report.notes.is_empty() {
        "  - none".to_string()
    } else {
        report
            .notes
            .iter()
            .map(|note| format!("  - {note}"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "\
Capture summary
  path: {}
  format: {format}
  size_bytes: {}
  packet_count_hint: {packet_count}
notes:
{notes}",
        report.path.display(),
        report.size_bytes,
    )
}

pub fn render_packet_list_report(report: &PacketListReport) -> String {
    let format = match report.format {
        CaptureFormat::Pcap => "pcap",
        CaptureFormat::PcapNg => "pcapng",
        CaptureFormat::Unknown => "unknown",
    };

    let mut lines = vec![
        "Packet list".to_string(),
        format!("  path: {}", report.path.display()),
        format!("  format: {format}"),
        format!("  packets_shown: {}", report.packets.len()),
        format!("  total_packets: {}", report.total_packets),
        "packets:".to_string(),
    ];

    if report.packets.is_empty() {
        lines.push("  - none".to_string());
        return lines.join("\n");
    }

    for packet in &report.packets {
        let (precision, fraction_width) = match packet.summary.timestamp_precision {
            TimestampPrecision::Microseconds => ("us", 6usize),
            TimestampPrecision::Nanoseconds => ("ns", 9usize),
        };

        lines.push(format!(
            "  - #{:04} ts={}.{:0width$}{} captured_len={} original_len={} src={} dst={} proto={} info={}",
            packet.summary.index,
            packet.summary.timestamp_seconds,
            packet.summary.timestamp_fraction,
            precision,
            packet.summary.captured_length,
            packet.summary.original_length,
            packet.source,
            packet.destination,
            packet.protocol,
            packet.info,
            width = fraction_width,
        ));
    }

    lines.join("\n")
}

pub fn render_packet_detail_report(report: &PacketDetailReport) -> String {
    let format = match report.format {
        CaptureFormat::Pcap => "pcap",
        CaptureFormat::PcapNg => "pcapng",
        CaptureFormat::Unknown => "unknown",
    };

    let packet = &report.packet;
    let (precision, fraction_width) = match packet.summary.timestamp_precision {
        TimestampPrecision::Microseconds => ("us", 6usize),
        TimestampPrecision::Nanoseconds => ("ns", 9usize),
    };

    let mut lines = vec![
        "Packet detail".to_string(),
        format!("  path: {}", report.path.display()),
        format!("  format: {format}"),
        format!("  index: {}", packet.summary.index),
        format!(
            "  timestamp: {}.{:0width$}{}",
            packet.summary.timestamp_seconds,
            packet.summary.timestamp_fraction,
            precision,
            width = fraction_width
        ),
        format!("  captured_length: {}", packet.summary.captured_length),
        format!("  original_length: {}", packet.summary.original_length),
        format!("  raw_bytes: {}", render_hex_bytes(&packet.raw_bytes)),
        "layers:".to_string(),
        format!("  link: {}", render_link_layer(&packet.link)),
        format!(
            "  network: {}",
            render_network_layer(packet.network.as_ref())
        ),
        format!(
            "  transport: {}",
            render_transport_layer(packet.transport.as_ref())
        ),
        format!(
            "  application: {}",
            render_application_layer(packet.application.as_ref())
        ),
        "fields:".to_string(),
    ];

    if packet.fields.is_empty() {
        lines.push("  - none".to_string());
    } else {
        render_field_lines(&mut lines, &packet.fields, 1);
    }

    lines.push("notes:".to_string());

    if packet.notes.is_empty() {
        lines.push("  - none".to_string());
    } else {
        for note in &packet.notes {
            lines.push(format!("  - {note}"));
        }
    }

    lines.join("\n")
}

pub fn render_capture_stats_report(report: &CaptureStatsReport) -> String {
    let format = match report.format {
        CaptureFormat::Pcap => "pcap",
        CaptureFormat::PcapNg => "pcapng",
        CaptureFormat::Unknown => "unknown",
    };

    vec![
        "Capture stats".to_string(),
        format!("  path: {}", report.path.display()),
        format!("  format: {format}"),
        format!("  total_packets: {}", report.total_packets),
        format!("  total_captured_bytes: {}", report.total_captured_bytes),
        format!(
            "  average_captured_bytes: {}",
            report.average_captured_bytes
        ),
        "layers:".to_string(),
        format!("  link: {}", render_named_counts(&report.link_layer_counts)),
        format!(
            "  network: {}",
            render_named_counts(&report.network_layer_counts)
        ),
        format!(
            "  transport: {}",
            render_named_counts(&report.transport_layer_counts)
        ),
    ]
    .join("\n")
}

pub fn render_conversation_report(report: &ConversationReport) -> String {
    let format = match report.format {
        CaptureFormat::Pcap => "pcap",
        CaptureFormat::PcapNg => "pcapng",
        CaptureFormat::Unknown => "unknown",
    };

    let mut lines = vec![
        "Conversations".to_string(),
        format!("  path: {}", report.path.display()),
        format!("  format: {format}"),
        format!("  total_conversations: {}", report.total_conversations),
        "items:".to_string(),
    ];

    if report.conversations.is_empty() {
        lines.push("  - none".to_string());
        return lines.join("\n");
    }

    for row in &report.conversations {
        lines.push(format!(
            "  - service={} proto={} endpoints={} <-> {} packets={} a_to_b={} b_to_a={} requests={} responses={} bytes={} first_packet={} last_packet={}",
            row.service,
            row.protocol,
            row.endpoint_a,
            row.endpoint_b,
            row.packets,
            row.packets_a_to_b,
            row.packets_b_to_a,
            row.request_count,
            row.response_count,
            row.total_captured_bytes,
            row.first_packet_index,
            row.last_packet_index,
        ));
    }

    lines.join("\n")
}

pub fn render_capture_report_json(report: &CaptureReport) -> String {
    format!(
        "{{\"path\":\"{}\",\"format\":\"{}\",\"size_bytes\":{},\"packet_count_hint\":{},\"notes\":[{}]}}",
        json_escape(&report.path.display().to_string()),
        capture_format_name(&report.format),
        report.size_bytes,
        report
            .packet_count_hint
            .map(|count| count.to_string())
            .unwrap_or_else(|| "null".to_string()),
        report
            .notes
            .iter()
            .map(|note| format!("\"{}\"", json_escape(note)))
            .collect::<Vec<_>>()
            .join(","),
    )
}

pub fn render_packet_list_report_json(report: &PacketListReport) -> String {
    format!(
        "{{\"path\":\"{}\",\"format\":\"{}\",\"packets_shown\":{},\"total_packets\":{},\"packets\":[{}]}}",
        json_escape(&report.path.display().to_string()),
        capture_format_name(&report.format),
        report.packets.len(),
        report.total_packets,
        report
            .packets
            .iter()
            .map(|packet| {
                format!(
                    "{{\"index\":{},\"timestamp_seconds\":{},\"timestamp_fraction\":{},\"timestamp_precision\":\"{}\",\"captured_length\":{},\"original_length\":{},\"source\":\"{}\",\"destination\":\"{}\",\"protocol\":\"{}\",\"info\":\"{}\"}}",
                    packet.summary.index,
                    packet.summary.timestamp_seconds,
                    packet.summary.timestamp_fraction,
                    timestamp_precision_name(&packet.summary.timestamp_precision),
                    packet.summary.captured_length,
                    packet.summary.original_length,
                    json_escape(&packet.source),
                    json_escape(&packet.destination),
                    json_escape(&packet.protocol),
                    json_escape(&packet.info),
                )
            })
            .collect::<Vec<_>>()
            .join(","),
    )
}

pub fn render_packet_detail_report_json(report: &PacketDetailReport) -> String {
    format!(
        "{{\"path\":\"{}\",\"format\":\"{}\",\"packet\":{{\"index\":{},\"timestamp_seconds\":{},\"timestamp_fraction\":{},\"timestamp_precision\":\"{}\",\"captured_length\":{},\"original_length\":{},\"raw_bytes\":[{}],\"link\":{},\"network\":{},\"transport\":{},\"application\":{},\"fields\":[{}],\"notes\":[{}]}}}}",
        json_escape(&report.path.display().to_string()),
        capture_format_name(&report.format),
        report.packet.summary.index,
        report.packet.summary.timestamp_seconds,
        report.packet.summary.timestamp_fraction,
        timestamp_precision_name(&report.packet.summary.timestamp_precision),
        report.packet.summary.captured_length,
        report.packet.summary.original_length,
        report
            .packet
            .raw_bytes
            .iter()
            .map(|byte| byte.to_string())
            .collect::<Vec<_>>()
            .join(","),
        render_link_layer_json(&report.packet.link),
        render_network_layer_json(report.packet.network.as_ref()),
        render_transport_layer_json(report.packet.transport.as_ref()),
        render_application_layer_json(report.packet.application.as_ref()),
        render_field_nodes_json(&report.packet.fields),
        report
            .packet
            .notes
            .iter()
            .map(|note| format!("\"{}\"", json_escape(note)))
            .collect::<Vec<_>>()
            .join(","),
    )
}

pub fn render_conversation_report_json(report: &ConversationReport) -> String {
    format!(
        "{{\"path\":\"{}\",\"format\":\"{}\",\"total_conversations\":{},\"conversations\":[{}]}}",
        json_escape(&report.path.display().to_string()),
        capture_format_name(&report.format),
        report.total_conversations,
        report
            .conversations
            .iter()
            .map(|row| {
                format!(
                    "{{\"service\":\"{}\",\"protocol\":\"{}\",\"endpoint_a\":\"{}\",\"endpoint_b\":\"{}\",\"packets\":{},\"packets_a_to_b\":{},\"packets_b_to_a\":{},\"request_count\":{},\"response_count\":{},\"total_captured_bytes\":{},\"first_packet_index\":{},\"last_packet_index\":{}}}",
                    json_escape(&row.service),
                    json_escape(&row.protocol),
                    json_escape(&row.endpoint_a),
                    json_escape(&row.endpoint_b),
                    row.packets,
                    row.packets_a_to_b,
                    row.packets_b_to_a,
                    row.request_count,
                    row.response_count,
                    row.total_captured_bytes,
                    row.first_packet_index,
                    row.last_packet_index,
                )
            })
            .collect::<Vec<_>>()
            .join(","),
    )
}

pub fn render_capture_stats_report_json(report: &CaptureStatsReport) -> String {
    format!(
        "{{\"path\":\"{}\",\"format\":\"{}\",\"total_packets\":{},\"total_captured_bytes\":{},\"average_captured_bytes\":{},\"link_layer_counts\":[{}],\"network_layer_counts\":[{}],\"transport_layer_counts\":[{}]}}",
        json_escape(&report.path.display().to_string()),
        capture_format_name(&report.format),
        report.total_packets,
        report.total_captured_bytes,
        report.average_captured_bytes,
        render_named_counts_json(&report.link_layer_counts),
        render_named_counts_json(&report.network_layer_counts),
        render_named_counts_json(&report.transport_layer_counts),
    )
}

fn render_link_layer(link: &LinkLayerSummary) -> String {
    match link {
        LinkLayerSummary::Ethernet(ethernet) => format!(
            "ethernet src={} dst={} ether_type=0x{:04x}",
            ethernet.source_mac, ethernet.destination_mac, ethernet.ether_type
        ),
        LinkLayerSummary::Unknown => "unknown".to_string(),
    }
}

fn render_link_layer_json(link: &LinkLayerSummary) -> String {
    match link {
        LinkLayerSummary::Ethernet(ethernet) => format!(
            "{{\"kind\":\"ethernet\",\"source_mac\":\"{}\",\"destination_mac\":\"{}\",\"ether_type\":{}}}",
            json_escape(&ethernet.source_mac),
            json_escape(&ethernet.destination_mac),
            ethernet.ether_type
        ),
        LinkLayerSummary::Unknown => "{\"kind\":\"unknown\"}".to_string(),
    }
}

fn render_network_layer(network: Option<&NetworkLayerSummary>) -> String {
    match network {
        Some(NetworkLayerSummary::Arp(arp)) => format!(
            "arp op={} sender={}/{} target={}/{}",
            arp.operation,
            arp.sender_hardware_address,
            arp.sender_protocol_address,
            arp.target_hardware_address,
            arp.target_protocol_address
        ),
        Some(NetworkLayerSummary::Ipv4(ipv4)) => format!(
            "ipv4 src={} dst={} proto={} ttl={} ihl={} total_len={}",
            ipv4.source_ip,
            ipv4.destination_ip,
            ipv4.protocol,
            ipv4.ttl,
            ipv4.header_length,
            ipv4.total_length
        ),
        None => "none".to_string(),
    }
}

fn render_network_layer_json(network: Option<&NetworkLayerSummary>) -> String {
    match network {
        Some(NetworkLayerSummary::Arp(arp)) => format!(
            "{{\"kind\":\"arp\",\"operation\":{},\"sender_hardware_address\":\"{}\",\"sender_protocol_address\":\"{}\",\"target_hardware_address\":\"{}\",\"target_protocol_address\":\"{}\"}}",
            arp.operation,
            json_escape(&arp.sender_hardware_address),
            json_escape(&arp.sender_protocol_address),
            json_escape(&arp.target_hardware_address),
            json_escape(&arp.target_protocol_address)
        ),
        Some(NetworkLayerSummary::Ipv4(ipv4)) => format!(
            "{{\"kind\":\"ipv4\",\"source_ip\":\"{}\",\"destination_ip\":\"{}\",\"protocol\":{},\"ttl\":{},\"header_length\":{},\"total_length\":{}}}",
            json_escape(&ipv4.source_ip),
            json_escape(&ipv4.destination_ip),
            ipv4.protocol,
            ipv4.ttl,
            ipv4.header_length,
            ipv4.total_length
        ),
        None => "null".to_string(),
    }
}

fn render_transport_layer(transport: Option<&TransportLayerSummary>) -> String {
    match transport {
        Some(TransportLayerSummary::Tcp(tcp)) => format!(
            "tcp src_port={} dst_port={} seq={} ack={} flags=0x{:03x}",
            tcp.source_port,
            tcp.destination_port,
            tcp.sequence_number,
            tcp.acknowledgement_number,
            tcp.flags
        ),
        Some(TransportLayerSummary::Udp(udp)) => format!(
            "udp src_port={} dst_port={} length={}",
            udp.source_port, udp.destination_port, udp.length
        ),
        Some(TransportLayerSummary::Icmp(icmp)) => {
            format!("icmp type={} code={}", icmp.icmp_type, icmp.code)
        }
        None => "none".to_string(),
    }
}

fn render_application_layer(application: Option<&ApplicationLayerSummary>) -> String {
    match application {
        Some(ApplicationLayerSummary::Dns(dns)) => format!(
            "dns id={} response={} questions={} answers={} first_question={}",
            dns.id,
            dns.is_response,
            dns.question_count,
            dns.answer_count,
            dns.questions.first().map(String::as_str).unwrap_or("n/a")
        ),
        Some(ApplicationLayerSummary::Http(http)) => format!(
            "http kind={} method={} path={} status={} host={}",
            http.kind,
            http.method.as_deref().unwrap_or("n/a"),
            http.path.as_deref().unwrap_or("n/a"),
            http.status_code
                .map(|v| v.to_string())
                .unwrap_or_else(|| "n/a".to_string()),
            http.host.as_deref().unwrap_or("n/a")
        ),
        Some(ApplicationLayerSummary::TlsHandshake(tls)) => format!(
            "tls handshake_type={} version={} length={} sni={}",
            tls.handshake_type,
            tls.record_version,
            tls.handshake_length,
            tls.server_name.as_deref().unwrap_or("n/a")
        ),
        None => "none".to_string(),
    }
}

fn render_application_layer_json(application: Option<&ApplicationLayerSummary>) -> String {
    match application {
        Some(ApplicationLayerSummary::Dns(dns)) => format!(
            "{{\"kind\":\"dns\",\"id\":{},\"is_response\":{},\"opcode\":{},\"question_count\":{},\"answer_count\":{},\"questions\":[{}]}}",
            dns.id,
            dns.is_response,
            dns.opcode,
            dns.question_count,
            dns.answer_count,
            dns.questions
                .iter()
                .map(|name| format!("\"{}\"", json_escape(name)))
                .collect::<Vec<_>>()
                .join(",")
        ),
        Some(ApplicationLayerSummary::Http(http)) => format!(
            "{{\"kind\":\"http\",\"message_kind\":\"{}\",\"method\":{},\"path\":{},\"status_code\":{},\"reason_phrase\":{},\"host\":{}}}",
            json_escape(&http.kind),
            http.method.as_ref().map(|v| format!("\"{}\"", json_escape(v))).unwrap_or_else(|| "null".to_string()),
            http.path.as_ref().map(|v| format!("\"{}\"", json_escape(v))).unwrap_or_else(|| "null".to_string()),
            http.status_code.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
            http.reason_phrase.as_ref().map(|v| format!("\"{}\"", json_escape(v))).unwrap_or_else(|| "null".to_string()),
            http.host.as_ref().map(|v| format!("\"{}\"", json_escape(v))).unwrap_or_else(|| "null".to_string())
        ),
        Some(ApplicationLayerSummary::TlsHandshake(tls)) => format!(
            "{{\"kind\":\"tls_handshake\",\"record_version\":\"{}\",\"handshake_type\":\"{}\",\"handshake_length\":{},\"server_name\":{}}}",
            json_escape(&tls.record_version),
            json_escape(&tls.handshake_type),
            tls.handshake_length,
            tls.server_name.as_ref().map(|v| format!("\"{}\"", json_escape(v))).unwrap_or_else(|| "null".to_string())
        ),
        None => "null".to_string(),
    }
}

fn render_transport_layer_json(transport: Option<&TransportLayerSummary>) -> String {
    match transport {
        Some(TransportLayerSummary::Tcp(tcp)) => format!(
            "{{\"kind\":\"tcp\",\"source_port\":{},\"destination_port\":{},\"sequence_number\":{},\"acknowledgement_number\":{},\"flags\":{}}}",
            tcp.source_port,
            tcp.destination_port,
            tcp.sequence_number,
            tcp.acknowledgement_number,
            tcp.flags
        ),
        Some(TransportLayerSummary::Udp(udp)) => format!(
            "{{\"kind\":\"udp\",\"source_port\":{},\"destination_port\":{},\"length\":{}}}",
            udp.source_port,
            udp.destination_port,
            udp.length
        ),
        Some(TransportLayerSummary::Icmp(icmp)) => format!(
            "{{\"kind\":\"icmp\",\"icmp_type\":{},\"code\":{}}}",
            icmp.icmp_type, icmp.code
        ),
        None => "null".to_string(),
    }
}

fn render_hex_bytes(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "none".to_string();
    }

    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_field_lines(lines: &mut Vec<String>, fields: &[FieldNode], depth: usize) {
    for field in fields {
        let indent = "  ".repeat(depth);
        let range = field
            .byte_range
            .as_ref()
            .map(|range| format!(" [{}..{}]", range.start, range.end))
            .unwrap_or_default();
        lines.push(format!(
            "{indent}- {}: {}{}",
            field.name, field.value, range
        ));
        render_field_lines(lines, &field.children, depth + 1);
    }
}

fn render_field_nodes_json(fields: &[FieldNode]) -> String {
    fields
        .iter()
        .map(render_field_node_json)
        .collect::<Vec<_>>()
        .join(",")
}

fn render_field_node_json(field: &FieldNode) -> String {
    format!(
        "{{\"name\":\"{}\",\"value\":\"{}\",\"byte_range\":{},\"children\":[{}]}}",
        json_escape(&field.name),
        json_escape(&field.value),
        match &field.byte_range {
            Some(range) => format!("{{\"start\":{},\"end\":{}}}", range.start, range.end),
            None => "null".to_string(),
        },
        render_field_nodes_json(&field.children)
    )
}

fn render_named_counts(counts: &[NamedCount]) -> String {
    if counts.is_empty() {
        return "none".to_string();
    }

    counts
        .iter()
        .map(|entry| format!("{}={}", entry.name, entry.count))
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_named_counts_json(counts: &[NamedCount]) -> String {
    counts
        .iter()
        .map(|entry| {
            format!(
                "{{\"name\":\"{}\",\"count\":{}}}",
                json_escape(&entry.name),
                entry.count
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn capture_format_name(format: &CaptureFormat) -> &'static str {
    match format {
        CaptureFormat::Pcap => "pcap",
        CaptureFormat::PcapNg => "pcapng",
        CaptureFormat::Unknown => "unknown",
    }
}

fn timestamp_precision_name(precision: &TimestampPrecision) -> &'static str {
    match precision {
        TimestampPrecision::Microseconds => "microseconds",
        TimestampPrecision::Nanoseconds => "nanoseconds",
    }
}

fn json_escape(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| match ch {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            _ => vec![ch],
        })
        .collect()
}
