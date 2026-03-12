use session_model::{
    ApplicationLayerSummary, ArpPacketSummary, ByteRange, DecodedPacket, DnsMessageSummary,
    EthernetFrameSummary, FieldNode, HttpMessageSummary, IcmpPacketSummary, Ipv4PacketSummary,
    Ipv6PacketSummary, LinkLayerSummary, NetworkLayerSummary, PacketSummary, TcpSegmentSummary,
    TlsHandshakeSummary, TransportLayerSummary, UdpDatagramSummary,
};

const LINKTYPE_ETHERNET: u32 = 1;

pub fn decode_packet(summary: PacketSummary, payload: &[u8], linktype: u32) -> DecodedPacket {
    let mut notes = Vec::new();
    let mut fields = Vec::new();
    let mut network = None;
    let mut transport = None;
    let mut application = None;

    let link = if linktype == LINKTYPE_ETHERNET && payload.len() >= 14 {
        let ether_type = u16::from_be_bytes([payload[12], payload[13]]);
        let ethernet = EthernetFrameSummary {
            source_mac: format_mac(&payload[6..12]),
            destination_mac: format_mac(&payload[0..6]),
            ether_type,
        };

        fields.push(node(
            "ethernet",
            "Ethernet II",
            Some(range(0, payload.len().min(14))),
            vec![
                field(
                    "destination",
                    ethernet.destination_mac.clone(),
                    Some(range(0, 6)),
                ),
                field("source", ethernet.source_mac.clone(), Some(range(6, 12))),
                field(
                    "ether_type",
                    format!("0x{ether_type:04x}"),
                    Some(range(12, 14)),
                ),
            ],
        ));

        match ether_type {
            0x0800 => match decode_ipv4(&payload[14..], 14) {
                Ok((ipv4, next_payload, next_offset, ipv4_fields)) => {
                    let (transport_summary, application_summary) = decode_transport(
                        ipv4.protocol,
                        next_payload,
                        next_offset,
                        &mut notes,
                        &mut fields,
                    );
                    network = Some(NetworkLayerSummary::Ipv4(ipv4));
                    transport = transport_summary;
                    application = application_summary;
                    fields.push(node(
                        "ipv4",
                        "Internet Protocol Version 4",
                        Some(range(14, next_offset)),
                        ipv4_fields,
                    ));
                }
                Err(error) => notes.push(error),
            },
            0x86dd => match decode_ipv6(&payload[14..], 14) {
                Ok((ipv6, next_payload, next_offset, ipv6_fields)) => {
                    let (transport_summary, application_summary) = decode_transport(
                        ipv6.next_header,
                        next_payload,
                        next_offset,
                        &mut notes,
                        &mut fields,
                    );
                    network = Some(NetworkLayerSummary::Ipv6(ipv6));
                    transport = transport_summary;
                    application = application_summary;
                    fields.push(node(
                        "ipv6",
                        "Internet Protocol Version 6",
                        Some(range(14, next_offset)),
                        ipv6_fields,
                    ));
                }
                Err(error) => notes.push(error),
            },
            0x0806 => match decode_arp(&payload[14..], 14) {
                Ok((arp, arp_fields)) => {
                    network = Some(NetworkLayerSummary::Arp(arp));
                    fields.push(node(
                        "arp",
                        "Address Resolution Protocol",
                        Some(range(14, 42)),
                        arp_fields,
                    ));
                }
                Err(error) => notes.push(error),
            },
            _ => notes.push(format!("unsupported ether type 0x{ether_type:04x}")),
        }

        LinkLayerSummary::Ethernet(ethernet)
    } else if linktype == LINKTYPE_ETHERNET {
        notes.push("packet is shorter than an ethernet header".to_string());
        LinkLayerSummary::Unknown
    } else {
        notes.push(format!("unsupported linktype {linktype}"));
        LinkLayerSummary::Unknown
    };

    DecodedPacket {
        summary,
        raw_bytes: payload.to_vec(),
        link,
        network,
        transport,
        application,
        fields,
        notes,
    }
}

fn decode_ipv4(
    payload: &[u8],
    base_offset: usize,
) -> Result<(Ipv4PacketSummary, &[u8], usize, Vec<FieldNode>), String> {
    if payload.len() < 20 {
        return Err("ipv4 packet is shorter than the minimum header size".to_string());
    }
    let version = payload[0] >> 4;
    if version != 4 {
        return Err(format!("expected ipv4 version 4 but found {version}"));
    }
    let header_length = (payload[0] & 0x0f) * 4;
    if header_length < 20 || payload.len() < header_length as usize {
        return Err("ipv4 header is truncated".to_string());
    }
    let total_length = u16::from_be_bytes([payload[2], payload[3]]);
    let total_length_usize = usize::from(total_length);
    if total_length_usize < header_length as usize || payload.len() < total_length_usize {
        return Err("ipv4 payload is truncated".to_string());
    }

    let summary = Ipv4PacketSummary {
        source_ip: format_ipv4(&payload[12..16]),
        destination_ip: format_ipv4(&payload[16..20]),
        protocol: payload[9],
        ttl: payload[8],
        header_length,
        total_length,
    };
    let fields = vec![
        field(
            "version",
            version.to_string(),
            Some(range(base_offset, base_offset + 1)),
        ),
        field(
            "header_length",
            header_length.to_string(),
            Some(range(base_offset, base_offset + 1)),
        ),
        field(
            "total_length",
            total_length.to_string(),
            Some(range(base_offset + 2, base_offset + 4)),
        ),
        field(
            "ttl",
            payload[8].to_string(),
            Some(range(base_offset + 8, base_offset + 9)),
        ),
        field(
            "protocol",
            payload[9].to_string(),
            Some(range(base_offset + 9, base_offset + 10)),
        ),
        field(
            "source",
            summary.source_ip.clone(),
            Some(range(base_offset + 12, base_offset + 16)),
        ),
        field(
            "destination",
            summary.destination_ip.clone(),
            Some(range(base_offset + 16, base_offset + 20)),
        ),
    ];

    Ok((
        summary,
        &payload[header_length as usize..total_length_usize],
        base_offset + header_length as usize,
        fields,
    ))
}

fn decode_ipv6(
    payload: &[u8],
    base_offset: usize,
) -> Result<(Ipv6PacketSummary, &[u8], usize, Vec<FieldNode>), String> {
    if payload.len() < 40 {
        return Err("ipv6 packet is shorter than the fixed header size".to_string());
    }
    let version = payload[0] >> 4;
    if version != 6 {
        return Err(format!("expected ipv6 version 6 but found {version}"));
    }
    let payload_length = u16::from_be_bytes([payload[4], payload[5]]);
    let payload_end = 40usize.saturating_add(usize::from(payload_length));
    if payload.len() < payload_end {
        return Err("ipv6 payload is truncated".to_string());
    }

    let summary = Ipv6PacketSummary {
        source_ip: format_ipv6(&payload[8..24]),
        destination_ip: format_ipv6(&payload[24..40]),
        next_header: payload[6],
        hop_limit: payload[7],
        payload_length,
    };
    let fields = vec![
        field(
            "version",
            version.to_string(),
            Some(range(base_offset, base_offset + 1)),
        ),
        field(
            "payload_length",
            payload_length.to_string(),
            Some(range(base_offset + 4, base_offset + 6)),
        ),
        field(
            "next_header",
            summary.next_header.to_string(),
            Some(range(base_offset + 6, base_offset + 7)),
        ),
        field(
            "hop_limit",
            summary.hop_limit.to_string(),
            Some(range(base_offset + 7, base_offset + 8)),
        ),
        field(
            "source",
            summary.source_ip.clone(),
            Some(range(base_offset + 8, base_offset + 24)),
        ),
        field(
            "destination",
            summary.destination_ip.clone(),
            Some(range(base_offset + 24, base_offset + 40)),
        ),
    ];

    Ok((summary, &payload[40..payload_end], base_offset + 40, fields))
}

fn decode_arp(
    payload: &[u8],
    base_offset: usize,
) -> Result<(ArpPacketSummary, Vec<FieldNode>), String> {
    if payload.len() < 28 {
        return Err("arp payload is shorter than the ethernet/ipv4 arp packet size".to_string());
    }
    let summary = ArpPacketSummary {
        operation: u16::from_be_bytes([payload[6], payload[7]]),
        sender_hardware_address: format_mac(&payload[8..14]),
        sender_protocol_address: format_ipv4(&payload[14..18]),
        target_hardware_address: format_mac(&payload[18..24]),
        target_protocol_address: format_ipv4(&payload[24..28]),
    };
    Ok((
        summary.clone(),
        vec![
            field(
                "operation",
                summary.operation.to_string(),
                Some(range(base_offset + 6, base_offset + 8)),
            ),
            field(
                "sender_hardware_address",
                summary.sender_hardware_address,
                Some(range(base_offset + 8, base_offset + 14)),
            ),
            field(
                "sender_protocol_address",
                summary.sender_protocol_address,
                Some(range(base_offset + 14, base_offset + 18)),
            ),
            field(
                "target_hardware_address",
                summary.target_hardware_address,
                Some(range(base_offset + 18, base_offset + 24)),
            ),
            field(
                "target_protocol_address",
                summary.target_protocol_address,
                Some(range(base_offset + 24, base_offset + 28)),
            ),
        ],
    ))
}

fn decode_transport(
    protocol: u8,
    payload: &[u8],
    base_offset: usize,
    notes: &mut Vec<String>,
    fields: &mut Vec<FieldNode>,
) -> (
    Option<TransportLayerSummary>,
    Option<ApplicationLayerSummary>,
) {
    match protocol {
        1 => match decode_icmp(payload, base_offset) {
            Ok((icmp, icmp_fields)) => {
                fields.push(node(
                    "icmp",
                    "Internet Control Message Protocol",
                    Some(range(base_offset, base_offset + payload.len().min(4))),
                    icmp_fields,
                ));
                (Some(TransportLayerSummary::Icmp(icmp)), None)
            }
            Err(error) => {
                notes.push(error);
                (None, None)
            }
        },
        58 => match decode_icmp(payload, base_offset) {
            Ok((icmp, icmp_fields)) => {
                fields.push(node(
                    "icmpv6",
                    "Internet Control Message Protocol Version 6",
                    Some(range(base_offset, base_offset + payload.len().min(4))),
                    icmp_fields,
                ));
                (Some(TransportLayerSummary::Icmp(icmp)), None)
            }
            Err(error) => {
                notes.push(error);
                (None, None)
            }
        },
        6 => match decode_tcp(payload, base_offset) {
            Ok((tcp, header_len, tcp_fields)) => {
                fields.push(node(
                    "tcp",
                    "Transmission Control Protocol",
                    Some(range(base_offset, base_offset + header_len)),
                    tcp_fields,
                ));
                let application = decode_tcp_application(
                    &tcp,
                    &payload[header_len..],
                    base_offset + header_len,
                    fields,
                );
                (Some(TransportLayerSummary::Tcp(tcp)), application)
            }
            Err(error) => {
                notes.push(error);
                (None, None)
            }
        },
        17 => match decode_udp(payload, base_offset) {
            Ok((udp, udp_fields)) => {
                fields.push(node(
                    "udp",
                    "User Datagram Protocol",
                    Some(range(base_offset, base_offset + payload.len().min(8))),
                    udp_fields,
                ));
                let application =
                    decode_udp_application(&udp, &payload[8..], base_offset + 8, fields);
                (Some(TransportLayerSummary::Udp(udp)), application)
            }
            Err(error) => {
                notes.push(error);
                (None, None)
            }
        },
        _ => {
            notes.push(format!("unsupported ip transport protocol {protocol}"));
            (None, None)
        }
    }
}

fn decode_tcp(
    payload: &[u8],
    base_offset: usize,
) -> Result<(TcpSegmentSummary, usize, Vec<FieldNode>), String> {
    if payload.len() < 20 {
        return Err("tcp segment is shorter than the minimum header size".to_string());
    }
    let header_len = usize::from(payload[12] >> 4) * 4;
    if header_len < 20 || payload.len() < header_len {
        return Err("tcp header is truncated".to_string());
    }
    let summary = TcpSegmentSummary {
        source_port: u16::from_be_bytes([payload[0], payload[1]]),
        destination_port: u16::from_be_bytes([payload[2], payload[3]]),
        sequence_number: u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]),
        acknowledgement_number: u32::from_be_bytes([
            payload[8],
            payload[9],
            payload[10],
            payload[11],
        ]),
        flags: u16::from_be_bytes([payload[12] & 0x1f, payload[13]]),
    };
    Ok((
        summary.clone(),
        header_len,
        vec![
            field(
                "source_port",
                summary.source_port.to_string(),
                Some(range(base_offset, base_offset + 2)),
            ),
            field(
                "destination_port",
                summary.destination_port.to_string(),
                Some(range(base_offset + 2, base_offset + 4)),
            ),
            field(
                "sequence_number",
                summary.sequence_number.to_string(),
                Some(range(base_offset + 4, base_offset + 8)),
            ),
            field(
                "acknowledgement_number",
                summary.acknowledgement_number.to_string(),
                Some(range(base_offset + 8, base_offset + 12)),
            ),
            field(
                "flags",
                format!("0x{:03x}", summary.flags),
                Some(range(base_offset + 12, base_offset + 14)),
            ),
        ],
    ))
}

fn decode_udp(
    payload: &[u8],
    base_offset: usize,
) -> Result<(UdpDatagramSummary, Vec<FieldNode>), String> {
    if payload.len() < 8 {
        return Err("udp datagram is shorter than the header size".to_string());
    }
    let summary = UdpDatagramSummary {
        source_port: u16::from_be_bytes([payload[0], payload[1]]),
        destination_port: u16::from_be_bytes([payload[2], payload[3]]),
        length: u16::from_be_bytes([payload[4], payload[5]]),
    };
    Ok((
        summary.clone(),
        vec![
            field(
                "source_port",
                summary.source_port.to_string(),
                Some(range(base_offset, base_offset + 2)),
            ),
            field(
                "destination_port",
                summary.destination_port.to_string(),
                Some(range(base_offset + 2, base_offset + 4)),
            ),
            field(
                "length",
                summary.length.to_string(),
                Some(range(base_offset + 4, base_offset + 6)),
            ),
        ],
    ))
}

fn decode_icmp(
    payload: &[u8],
    base_offset: usize,
) -> Result<(IcmpPacketSummary, Vec<FieldNode>), String> {
    if payload.len() < 4 {
        return Err("icmp packet is shorter than the minimum header size".to_string());
    }
    let summary = IcmpPacketSummary {
        icmp_type: payload[0],
        code: payload[1],
    };
    Ok((
        summary.clone(),
        vec![
            field(
                "type",
                summary.icmp_type.to_string(),
                Some(range(base_offset, base_offset + 1)),
            ),
            field(
                "code",
                summary.code.to_string(),
                Some(range(base_offset + 1, base_offset + 2)),
            ),
        ],
    ))
}

fn decode_udp_application(
    udp: &UdpDatagramSummary,
    payload: &[u8],
    base_offset: usize,
    fields: &mut Vec<FieldNode>,
) -> Option<ApplicationLayerSummary> {
    if [53, 5353].contains(&udp.source_port) || [53, 5353].contains(&udp.destination_port) {
        if let Ok((dns, dns_fields)) = decode_dns(payload, base_offset) {
            fields.push(node(
                "dns",
                "Domain Name System",
                Some(range(base_offset, base_offset + payload.len())),
                dns_fields,
            ));
            return Some(ApplicationLayerSummary::Dns(dns));
        }
    }
    None
}

fn decode_tcp_application(
    tcp: &TcpSegmentSummary,
    payload: &[u8],
    base_offset: usize,
    fields: &mut Vec<FieldNode>,
) -> Option<ApplicationLayerSummary> {
    if [80].contains(&tcp.source_port) || [80].contains(&tcp.destination_port) {
        if let Ok((http, http_fields)) = decode_http(payload, base_offset) {
            fields.push(node(
                "http",
                "Hypertext Transfer Protocol",
                Some(range(base_offset, base_offset + payload.len())),
                http_fields,
            ));
            return Some(ApplicationLayerSummary::Http(http));
        }
    }
    if [443].contains(&tcp.source_port) || [443].contains(&tcp.destination_port) {
        if let Ok((tls, tls_fields)) = decode_tls_handshake(payload, base_offset) {
            fields.push(node(
                "tls",
                "Transport Layer Security",
                Some(range(base_offset, base_offset + payload.len())),
                tls_fields,
            ));
            return Some(ApplicationLayerSummary::TlsHandshake(tls));
        }
    }
    None
}

fn decode_dns(
    payload: &[u8],
    base_offset: usize,
) -> Result<(DnsMessageSummary, Vec<FieldNode>), String> {
    if payload.len() < 12 {
        return Err("dns message is shorter than the header size".to_string());
    }
    let id = u16::from_be_bytes([payload[0], payload[1]]);
    let flags = u16::from_be_bytes([payload[2], payload[3]]);
    let question_count = u16::from_be_bytes([payload[4], payload[5]]);
    let answer_count = u16::from_be_bytes([payload[6], payload[7]]);
    let questions = parse_dns_questions(payload, question_count)?;
    let summary = DnsMessageSummary {
        id,
        is_response: (flags & 0x8000) != 0,
        opcode: ((flags >> 11) & 0x0f) as u8,
        question_count,
        answer_count,
        questions: questions
            .iter()
            .map(|(name, _, _, _)| name.clone())
            .collect(),
    };

    let mut fields = vec![
        field(
            "id",
            id.to_string(),
            Some(range(base_offset, base_offset + 2)),
        ),
        field(
            "is_response",
            summary.is_response.to_string(),
            Some(range(base_offset + 2, base_offset + 4)),
        ),
        field(
            "opcode",
            summary.opcode.to_string(),
            Some(range(base_offset + 2, base_offset + 4)),
        ),
        field(
            "question_count",
            question_count.to_string(),
            Some(range(base_offset + 4, base_offset + 6)),
        ),
        field(
            "answer_count",
            answer_count.to_string(),
            Some(range(base_offset + 6, base_offset + 8)),
        ),
    ];

    for (name, offset, qtype, qclass) in questions {
        let wire_len = name_wire_len(payload, offset)?;
        fields.push(node(
            "question",
            name.clone(),
            Some(range(
                base_offset + offset,
                base_offset + offset + wire_len + 4,
            )),
            vec![
                field(
                    "name",
                    name,
                    Some(range(base_offset + offset, base_offset + offset + wire_len)),
                ),
                field(
                    "type",
                    qtype.to_string(),
                    Some(range(
                        base_offset + offset + wire_len,
                        base_offset + offset + wire_len + 2,
                    )),
                ),
                field(
                    "class",
                    qclass.to_string(),
                    Some(range(
                        base_offset + offset + wire_len + 2,
                        base_offset + offset + wire_len + 4,
                    )),
                ),
            ],
        ));
    }

    Ok((summary, fields))
}

fn decode_http(
    payload: &[u8],
    base_offset: usize,
) -> Result<(HttpMessageSummary, Vec<FieldNode>), String> {
    let text = std::str::from_utf8(payload).map_err(|_| "http payload is not utf-8".to_string())?;
    let first_line = text
        .lines()
        .next()
        .ok_or_else(|| "http payload is empty".to_string())?;
    let host = text
        .lines()
        .find_map(|line| line.strip_prefix("Host: ").map(|v| v.trim().to_string()));

    if let Some((method, path)) = parse_http_request_line(first_line) {
        let mut fields = vec![
            field(
                "kind",
                "request",
                Some(range(base_offset, base_offset + first_line.len())),
            ),
            field(
                "method",
                method.to_string(),
                Some(range(base_offset, base_offset + method.len())),
            ),
            field("path", path.to_string(), None),
        ];
        if let Some(host_value) = &host {
            fields.push(field("host", host_value.clone(), None));
        }
        return Ok((
            HttpMessageSummary {
                kind: "request".to_string(),
                method: Some(method.to_string()),
                path: Some(path.to_string()),
                status_code: None,
                reason_phrase: None,
                host,
            },
            fields,
        ));
    }

    if let Some((status_code, reason_phrase)) = parse_http_status_line(first_line) {
        return Ok((
            HttpMessageSummary {
                kind: "response".to_string(),
                method: None,
                path: None,
                status_code: Some(status_code),
                reason_phrase: Some(reason_phrase.to_string()),
                host,
            },
            vec![
                field(
                    "kind",
                    "response",
                    Some(range(base_offset, base_offset + first_line.len())),
                ),
                field("status_code", status_code.to_string(), None),
                field("reason_phrase", reason_phrase.to_string(), None),
            ],
        ));
    }

    Err("http payload did not match request or response".to_string())
}

fn decode_tls_handshake(
    payload: &[u8],
    base_offset: usize,
) -> Result<(TlsHandshakeSummary, Vec<FieldNode>), String> {
    if payload.len() < 9 || payload[0] != 22 {
        return Err("tls record is not a handshake".to_string());
    }
    let handshake_type = match payload[5] {
        1 => "client_hello",
        2 => "server_hello",
        11 => "certificate",
        16 => "client_key_exchange",
        20 => "finished",
        _ => "unknown",
    }
    .to_string();
    let server_name = extract_tls_sni(payload).ok();
    let summary = TlsHandshakeSummary {
        record_version: format!("{}.{}", payload[1], payload[2]),
        handshake_type,
        handshake_length: (u32::from(payload[6]) << 16)
            | (u32::from(payload[7]) << 8)
            | u32::from(payload[8]),
        server_name,
    };
    let mut fields = vec![
        field(
            "content_type",
            "handshake",
            Some(range(base_offset, base_offset + 1)),
        ),
        field(
            "record_version",
            summary.record_version.clone(),
            Some(range(base_offset + 1, base_offset + 3)),
        ),
        field(
            "handshake_type",
            summary.handshake_type.clone(),
            Some(range(base_offset + 5, base_offset + 6)),
        ),
        field(
            "handshake_length",
            summary.handshake_length.to_string(),
            Some(range(base_offset + 6, base_offset + 9)),
        ),
    ];
    if let Some(server_name) = &summary.server_name {
        fields.push(field("server_name", server_name.clone(), None));
    }
    Ok((summary, fields))
}

fn parse_http_request_line(line: &str) -> Option<(&str, &str)> {
    let mut parts = line.split_whitespace();
    let method = parts.next()?;
    let path = parts.next()?;
    let version = parts.next()?;
    if version.starts_with("HTTP/") {
        Some((method, path))
    } else {
        None
    }
}

fn parse_http_status_line(line: &str) -> Option<(u16, &str)> {
    let rest = line.strip_prefix("HTTP/1.1 ")?;
    let mut parts = rest.splitn(2, ' ');
    Some((parts.next()?.parse().ok()?, parts.next().unwrap_or("")))
}

fn parse_dns_questions(
    payload: &[u8],
    question_count: u16,
) -> Result<Vec<(String, usize, u16, u16)>, String> {
    let mut offset = 12usize;
    let mut questions = Vec::new();
    for _ in 0..question_count {
        let name = parse_dns_name(payload, offset)?;
        let wire_len = name_wire_len(payload, offset)?;
        let type_offset = offset + wire_len;
        let qtype = u16::from_be_bytes([
            *payload
                .get(type_offset)
                .ok_or_else(|| "dns question type truncated".to_string())?,
            *payload
                .get(type_offset + 1)
                .ok_or_else(|| "dns question type truncated".to_string())?,
        ]);
        let qclass = u16::from_be_bytes([
            *payload
                .get(type_offset + 2)
                .ok_or_else(|| "dns question class truncated".to_string())?,
            *payload
                .get(type_offset + 3)
                .ok_or_else(|| "dns question class truncated".to_string())?,
        ]);
        questions.push((name, offset, qtype, qclass));
        offset = type_offset + 4;
    }
    Ok(questions)
}

fn parse_dns_name(payload: &[u8], mut offset: usize) -> Result<String, String> {
    let mut labels = Vec::new();
    loop {
        let len = *payload
            .get(offset)
            .ok_or_else(|| "dns name is truncated".to_string())? as usize;
        if len == 0 {
            break;
        }
        offset += 1;
        let end = offset + len;
        let label = payload
            .get(offset..end)
            .ok_or_else(|| "dns label exceeds payload".to_string())?;
        labels.push(String::from_utf8_lossy(label).to_string());
        offset = end;
    }
    Ok(labels.join("."))
}

fn name_wire_len(payload: &[u8], mut offset: usize) -> Result<usize, String> {
    let start = offset;
    loop {
        let len = *payload
            .get(offset)
            .ok_or_else(|| "dns name is truncated".to_string())? as usize;
        offset += 1;
        if len == 0 {
            break;
        }
        offset += len;
    }
    Ok(offset - start)
}

fn extract_tls_sni(payload: &[u8]) -> Result<String, String> {
    if payload.len() < 43 || payload[5] != 1 {
        return Err("tls payload is not a client hello".to_string());
    }

    fn read_u8(payload: &[u8], offset: usize, label: &str) -> Result<u8, String> {
        payload
            .get(offset)
            .copied()
            .ok_or_else(|| format!("{label} truncated"))
    }

    fn read_u16(payload: &[u8], offset: usize, label: &str) -> Result<u16, String> {
        let first = payload
            .get(offset)
            .copied()
            .ok_or_else(|| format!("{label} truncated"))?;
        let second = payload
            .get(offset + 1)
            .copied()
            .ok_or_else(|| format!("{label} truncated"))?;
        Ok(u16::from_be_bytes([first, second]))
    }

    fn advance(offset: usize, amount: usize, label: &str) -> Result<usize, String> {
        offset
            .checked_add(amount)
            .ok_or_else(|| format!("{label} overflow"))
    }

    let mut offset = 9usize + 2 + 32;

    let session_len = usize::from(read_u8(payload, offset, "tls session id length")?);
    offset = advance(offset, 1, "tls session id")?;
    let session_end = advance(offset, session_len, "tls session id")?;
    if session_end > payload.len() {
        return Err("tls session id truncated".to_string());
    }
    offset = session_end;

    let cipher_len = usize::from(read_u16(payload, offset, "tls cipher suites length")?);
    offset = advance(offset, 2, "tls cipher suites")?;
    let cipher_end = advance(offset, cipher_len, "tls cipher suites")?;
    if cipher_end > payload.len() {
        return Err("tls cipher suites truncated".to_string());
    }
    offset = cipher_end;

    let compression_len = usize::from(read_u8(payload, offset, "tls compression methods length")?);
    offset = advance(offset, 1, "tls compression methods")?;
    let compression_end = advance(offset, compression_len, "tls compression methods")?;
    if compression_end > payload.len() {
        return Err("tls compression truncated".to_string());
    }
    offset = compression_end;

    let extensions_len = usize::from(read_u16(payload, offset, "tls extensions length")?);
    offset = advance(offset, 2, "tls extensions")?;
    let extensions_end = advance(offset, extensions_len, "tls extensions")?;
    if extensions_end > payload.len() {
        return Err("tls extensions truncated".to_string());
    }

    while offset + 4 <= extensions_end {
        let ext_type = read_u16(payload, offset, "tls extension header")?;
        let ext_len = usize::from(read_u16(payload, offset + 2, "tls extension header")?);
        offset = advance(offset, 4, "tls extension payload")?;
        let ext_end = advance(offset, ext_len, "tls extension payload")?;
        if ext_end > extensions_end {
            return Err("tls extension payload truncated".to_string());
        }

        if ext_type == 0 {
            let list_len = usize::from(read_u16(payload, offset, "tls server-name list length")?);
            let mut name_offset = advance(offset, 2, "tls server-name list")?;
            let list_end = advance(name_offset, list_len, "tls server-name list")?;
            if list_end > ext_end {
                return Err("tls server-name list truncated".to_string());
            }

            while name_offset + 3 <= list_end {
                let name_type = read_u8(payload, name_offset, "tls server-name type")?;
                let name_len = usize::from(read_u16(
                    payload,
                    name_offset + 1,
                    "tls server-name length",
                )?);
                name_offset = advance(name_offset, 3, "tls server-name")?;
                let name_end = advance(name_offset, name_len, "tls server-name")?;
                if name_end > list_end {
                    return Err("tls sni truncated".to_string());
                }
                if name_type == 0 {
                    let name = payload
                        .get(name_offset..name_end)
                        .ok_or_else(|| "tls sni truncated".to_string())?;
                    return Ok(String::from_utf8_lossy(name).to_string());
                }
                name_offset = name_end;
            }
        }

        offset = ext_end;
    }

    Err("tls server name extension not found".to_string())
}

fn node(
    name: impl Into<String>,
    value: impl Into<String>,
    byte_range: Option<ByteRange>,
    children: Vec<FieldNode>,
) -> FieldNode {
    FieldNode {
        name: name.into(),
        value: value.into(),
        byte_range,
        children,
    }
}

fn field(
    name: impl Into<String>,
    value: impl Into<String>,
    byte_range: Option<ByteRange>,
) -> FieldNode {
    FieldNode {
        name: name.into(),
        value: value.into(),
        byte_range,
        children: Vec::new(),
    }
}

fn range(start: usize, end: usize) -> ByteRange {
    ByteRange { start, end }
}

fn format_mac(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(":")
}

fn format_ipv4(bytes: &[u8]) -> String {
    format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
}

fn format_ipv6(bytes: &[u8]) -> String {
    bytes
        .chunks_exact(2)
        .map(|chunk| format!("{:x}", u16::from_be_bytes([chunk[0], chunk[1]])))
        .collect::<Vec<_>>()
        .join(":")
}

#[cfg(test)]
mod tests {
    use super::extract_tls_sni;

    #[test]
    fn extract_tls_sni_handles_truncated_client_hello_without_panicking() {
        let mut payload = vec![0u8; 45];
        payload[0] = 22;
        payload[5] = 1;
        payload[43] = 104;

        let result = extract_tls_sni(&payload);
        assert!(result.is_err());
    }
}
