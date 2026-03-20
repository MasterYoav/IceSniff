use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use session_model::{
    ApplicationLayerSummary, ArpPacketSummary, ByteRange, CapturedPacket, DecodedPacket,
    DnsMessageSummary, EthernetFrameSummary, FieldNode, HttpMessageSummary, IcmpPacketSummary,
    Ipv4PacketSummary, Ipv6PacketSummary, LinkLayerSummary, NetworkLayerSummary, TcpSegmentSummary,
    TlsHandshakeSummary, TransportLayerSummary, UdpDatagramSummary,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketListMetadata {
    pub source: String,
    pub destination: String,
    pub protocol: String,
    pub info: String,
    pub protocols: Vec<String>,
}

pub fn filter_packet_indexes(
    capture_path: &Path,
    filter_expression: &str,
) -> Result<Option<BTreeSet<u64>>, String> {
    let tshark_path = match resolve_tshark_path() {
        Some(path) => path,
        None => return Ok(None),
    };
    let display_filter =
        match filter_engine::translate_filter_to_tshark_display_filter(filter_expression) {
            Ok(filter) => filter,
            Err(_) => return Ok(None),
        };

    let output = Command::new(&tshark_path)
        .args([
            "-n",
            "-r",
            capture_path
                .to_str()
                .ok_or_else(|| "capture path is not valid UTF-8".to_string())?,
            "-Y",
            &display_filter,
            "-T",
            "fields",
            "-e",
            "frame.number",
        ])
        .output()
        .map_err(|error| format!("failed to launch tshark for filtering: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let first_line = stderr.lines().next().unwrap_or("unknown tshark failure");
        return Err(format!(
            "tshark failed to evaluate the filter: {first_line}"
        ));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("tshark returned invalid UTF-8 field output: {error}"))?;
    let indexes = stdout
        .lines()
        .filter_map(|line| line.trim().parse::<u64>().ok())
        .filter_map(|frame_number| frame_number.checked_sub(1))
        .collect::<BTreeSet<_>>();

    Ok(Some(indexes))
}

pub fn packet_list_metadata(
    capture_path: &Path,
) -> Result<Option<BTreeMap<u64, PacketListMetadata>>, String> {
    let tshark_path = match resolve_tshark_path() {
        Some(path) => path,
        None => return Ok(None),
    };

    let output = Command::new(&tshark_path)
        .args([
            "-n",
            "-r",
            capture_path
                .to_str()
                .ok_or_else(|| "capture path is not valid UTF-8".to_string())?,
            "-T",
            "fields",
            "-e",
            "frame.number",
            "-e",
            "_ws.col.Source",
            "-e",
            "_ws.col.Destination",
            "-e",
            "_ws.col.Protocol",
            "-e",
            "frame.protocols",
            "-e",
            "_ws.col.Info",
            "-E",
            "separator=\t",
            "-E",
            "quote=n",
            "-E",
            "header=n",
            "-E",
            "occurrence=f",
        ])
        .output()
        .map_err(|error| format!("failed to launch tshark for packet list metadata: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let first_line = stderr.lines().next().unwrap_or("unknown tshark failure");
        return Err(format!(
            "tshark failed to read packet list metadata: {first_line}"
        ));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("tshark returned invalid UTF-8 field output: {error}"))?;
    let mut metadata = BTreeMap::new();

    for line in stdout.lines() {
        let mut parts = line.splitn(6, '\t');
        let Some(frame_number) = parts
            .next()
            .and_then(|value| value.trim().parse::<u64>().ok())
        else {
            continue;
        };
        let index = match frame_number.checked_sub(1) {
            Some(value) => value,
            None => continue,
        };

        metadata.insert(
            index,
            PacketListMetadata {
                source: parts.next().unwrap_or("").trim().to_string(),
                destination: parts.next().unwrap_or("").trim().to_string(),
                protocol: parts.next().unwrap_or("").trim().to_string(),
                protocols: parse_protocol_chain(parts.next().unwrap_or("")),
                info: parts.next().unwrap_or("").trim().to_string(),
            },
        );
    }

    Ok(Some(metadata))
}

fn parse_protocol_chain(value: &str) -> Vec<String> {
    value
        .split(':')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.to_ascii_lowercase())
        .collect()
}

pub fn inspect_packet_with_tshark(
    capture_path: &Path,
    packet: &CapturedPacket,
) -> Result<DecodedPacket, String> {
    let tshark_path = resolve_tshark_path()
        .ok_or_else(|| "tshark is not available; falling back to the native decoder".to_string())?;
    let frame_number = packet.summary.index + 1;
    let output = Command::new(&tshark_path)
        .args([
            "-n",
            "-r",
            capture_path
                .to_str()
                .ok_or_else(|| "capture path is not valid UTF-8".to_string())?,
            "-Y",
            &format!("frame.number == {frame_number}"),
            "-c",
            "1",
            "-T",
            "pdml",
        ])
        .output()
        .map_err(|error| format!("failed to launch tshark: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let first_line = stderr.lines().next().unwrap_or("unknown tshark failure");
        return Err(format!("tshark failed to decode the packet: {first_line}"));
    }

    let xml = String::from_utf8(output.stdout)
        .map_err(|error| format!("tshark returned invalid UTF-8 PDML: {error}"))?;
    parse_pdml_packet(&xml, packet)
}

fn resolve_tshark_path() -> Option<PathBuf> {
    let explicit = env::var("ICESNIFF_TSHARK_BIN").ok().map(PathBuf::from);
    if let Some(candidate) = explicit.filter(|path| path.is_file()) {
        return Some(candidate);
    }

    let path_values = env::var_os("PATH")
        .map(|value| env::split_paths(&value).collect::<Vec<_>>())
        .unwrap_or_default();

    for directory in path_values {
        let candidate = directory.join("tshark");
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    [
        "/Applications/Wireshark.app/Contents/MacOS/tshark",
        "/opt/homebrew/bin/tshark",
        "/usr/local/bin/tshark",
        "/usr/bin/tshark",
    ]
    .into_iter()
    .map(PathBuf::from)
    .find(|path| path.is_file())
}

fn parse_pdml_packet(xml: &str, packet: &CapturedPacket) -> Result<DecodedPacket, String> {
    let pdml_fields = parse_pdml_fields(xml)?;

    let mut lookup = FieldLookup::default();
    for field in &pdml_fields {
        lookup.collect(field);
    }

    Ok(DecodedPacket {
        summary: packet.summary.clone(),
        raw_bytes: packet.raw_bytes.clone(),
        link: infer_link_layer(&lookup),
        network: infer_network_layer(&lookup),
        transport: infer_transport_layer(&lookup),
        application: infer_application_layer(&lookup),
        fields: pdml_fields
            .into_iter()
            .map(PdmlField::into_public)
            .collect(),
        notes: vec!["Decoded with bundled tshark for extended protocol coverage.".to_string()],
    })
}

#[derive(Debug, Clone)]
struct PdmlField {
    field_name: String,
    name: String,
    value: String,
    raw_value: String,
    showname: String,
    byte_range: Option<ByteRange>,
    children: Vec<PdmlField>,
}

impl PdmlField {
    fn into_public(self) -> FieldNode {
        FieldNode {
            name: self.name,
            value: self.value,
            byte_range: self.byte_range,
            children: self
                .children
                .into_iter()
                .map(PdmlField::into_public)
                .collect(),
        }
    }
}

fn parse_pdml_fields(xml: &str) -> Result<Vec<PdmlField>, String> {
    let mut in_packet = false;
    let mut stack: Vec<PdmlField> = Vec::new();
    let mut roots = Vec::new();

    for raw_line in xml.lines() {
        let line = raw_line.trim();
        if line.starts_with("<packet") {
            in_packet = true;
            continue;
        }
        if !in_packet {
            continue;
        }
        if line.starts_with("</packet") {
            break;
        }
        if line.starts_with("</proto") || line.starts_with("</field") {
            let Some(node) = stack.pop() else {
                return Err("invalid PDML nesting while closing a node".to_string());
            };
            attach_pdml_node(node, &mut stack, &mut roots);
            continue;
        }
        if !(line.starts_with("<proto ") || line.starts_with("<field ")) {
            continue;
        }

        let tag_name = if line.starts_with("<proto ") {
            "proto"
        } else {
            "field"
        };
        let attributes = parse_xml_attributes(line);
        let field_name = attributes.get("name").cloned().unwrap_or_default();
        if tag_name == "proto" && field_name == "geninfo" {
            continue;
        }

        let show = attributes.get("show").map(String::as_str).unwrap_or("");
        let showname = attributes.get("showname").map(String::as_str).unwrap_or("");
        let value = attributes.get("value").map(String::as_str).unwrap_or("");
        let (name, display_value) = display_parts(&field_name, showname, show, value, tag_name);
        let raw_value = if !show.is_empty() {
            show.to_string()
        } else if !value.is_empty() {
            value.to_string()
        } else {
            display_value.clone()
        };
        let byte_range = match (attributes.get("pos"), attributes.get("size")) {
            (Some(pos), Some(size)) => match (pos.parse::<usize>(), size.parse::<usize>()) {
                (Ok(start), Ok(length)) => Some(ByteRange {
                    start,
                    end: start.saturating_add(length),
                }),
                _ => None,
            },
            _ => None,
        };

        let node = PdmlField {
            field_name,
            name,
            value: display_value,
            raw_value,
            showname: showname.to_string(),
            byte_range,
            children: Vec::new(),
        };

        if line.ends_with("/>") {
            attach_pdml_node(node, &mut stack, &mut roots);
        } else {
            stack.push(node);
        }
    }

    while let Some(node) = stack.pop() {
        attach_pdml_node(node, &mut stack, &mut roots);
    }

    if roots.is_empty() {
        return Err("tshark PDML did not contain decodable protocol fields".to_string());
    }

    Ok(roots)
}

fn attach_pdml_node(node: PdmlField, stack: &mut [PdmlField], roots: &mut Vec<PdmlField>) {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
    } else {
        roots.push(node);
    }
}

fn parse_xml_attributes(line: &str) -> BTreeMap<String, String> {
    let mut attributes = BTreeMap::new();
    let bytes = line.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() || matches!(bytes[index], b'<' | b'/' | b'>') {
            index += 1;
            continue;
        }

        let key_start = index;
        while index < bytes.len()
            && !matches!(bytes[index], b'=' | b'>' | b'/')
            && !bytes[index].is_ascii_whitespace()
        {
            index += 1;
        }
        let key = &line[key_start..index];

        while index < bytes.len() && (bytes[index].is_ascii_whitespace() || bytes[index] == b'=') {
            index += 1;
        }
        if index >= bytes.len() || bytes[index] != b'"' {
            continue;
        }
        index += 1;
        let value_start = index;
        while index < bytes.len() && bytes[index] != b'"' {
            index += 1;
        }
        if index > bytes.len() {
            break;
        }

        let value = &line[value_start..index];
        attributes.insert(key.to_string(), decode_xml_entities(value));
        if index < bytes.len() {
            index += 1;
        }
    }

    attributes
}

fn decode_xml_entities(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

fn display_parts(
    field_name: &str,
    showname: &str,
    show: &str,
    value: &str,
    tag_name: &str,
) -> (String, String) {
    if tag_name == "proto" {
        let label = if showname.is_empty() {
            prettify_field_name(field_name)
        } else {
            showname.to_string()
        };
        return (label.clone(), label);
    }

    if let Some((name, detail)) = showname.split_once(": ") {
        return (name.to_string(), detail.to_string());
    }

    if !showname.is_empty() {
        return (prettify_field_name(field_name), showname.to_string());
    }

    if !show.is_empty() {
        return (prettify_field_name(field_name), show.to_string());
    }

    if !value.is_empty() {
        return (prettify_field_name(field_name), value.to_string());
    }

    (prettify_field_name(field_name), String::new())
}

fn prettify_field_name(field_name: &str) -> String {
    field_name
        .split('.')
        .next_back()
        .unwrap_or(field_name)
        .replace('_', " ")
        .split_whitespace()
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Default)]
struct FieldLookup {
    values: BTreeMap<String, Vec<String>>,
}

impl FieldLookup {
    fn collect(&mut self, field: &PdmlField) {
        let mut value = field.raw_value.trim().to_string();
        if value.is_empty() {
            value = field.showname.trim().to_string();
        }
        if !field.field_name.is_empty() && !value.is_empty() {
            self.values
                .entry(field.field_name.clone())
                .or_default()
                .push(value);
        }
        for child in &field.children {
            self.collect(child);
        }
    }

    fn first(&self, key: &str) -> Option<&str> {
        self.values.get(key)?.first().map(String::as_str)
    }

    fn any(&self, keys: &[&str]) -> Option<&str> {
        keys.iter().find_map(|key| self.first(key))
    }

    fn all(&self, key: &str) -> Vec<String> {
        self.values.get(key).cloned().unwrap_or_default()
    }
}

fn infer_link_layer(lookup: &FieldLookup) -> LinkLayerSummary {
    match (
        lookup.first("eth.src"),
        lookup.first("eth.dst"),
        parse_u16(lookup.first("eth.type")),
    ) {
        (Some(source), Some(destination), Some(ether_type)) => {
            LinkLayerSummary::Ethernet(EthernetFrameSummary {
                source_mac: source.to_string(),
                destination_mac: destination.to_string(),
                ether_type,
            })
        }
        _ => LinkLayerSummary::Unknown,
    }
}

fn infer_network_layer(lookup: &FieldLookup) -> Option<NetworkLayerSummary> {
    if let (Some(source), Some(destination)) = (lookup.first("ip.src"), lookup.first("ip.dst")) {
        return Some(NetworkLayerSummary::Ipv4(Ipv4PacketSummary {
            source_ip: source.to_string(),
            destination_ip: destination.to_string(),
            protocol: parse_u8(lookup.first("ip.proto")).unwrap_or_default(),
            ttl: parse_u8(lookup.first("ip.ttl")).unwrap_or_default(),
            header_length: parse_u8(lookup.first("ip.hdr_len")).unwrap_or_default(),
            total_length: parse_u16(lookup.first("ip.len")).unwrap_or_default(),
        }));
    }

    if let (Some(source), Some(destination)) = (lookup.first("ipv6.src"), lookup.first("ipv6.dst"))
    {
        return Some(NetworkLayerSummary::Ipv6(Ipv6PacketSummary {
            source_ip: source.to_string(),
            destination_ip: destination.to_string(),
            next_header: parse_u8(lookup.first("ipv6.nxt")).unwrap_or_default(),
            hop_limit: parse_u8(lookup.first("ipv6.hlim")).unwrap_or_default(),
            payload_length: parse_u16(lookup.first("ipv6.plen")).unwrap_or_default(),
        }));
    }

    if lookup
        .any(&["arp.src.hw_mac", "arp.src.proto_ipv4"])
        .is_some()
    {
        return Some(NetworkLayerSummary::Arp(ArpPacketSummary {
            operation: parse_u16(lookup.first("arp.opcode")).unwrap_or_default(),
            sender_hardware_address: lookup.first("arp.src.hw_mac").unwrap_or("").to_string(),
            sender_protocol_address: lookup.first("arp.src.proto_ipv4").unwrap_or("").to_string(),
            target_hardware_address: lookup.first("arp.dst.hw_mac").unwrap_or("").to_string(),
            target_protocol_address: lookup.first("arp.dst.proto_ipv4").unwrap_or("").to_string(),
        }));
    }

    None
}

fn infer_transport_layer(lookup: &FieldLookup) -> Option<TransportLayerSummary> {
    if lookup.first("tcp.srcport").is_some() {
        return Some(TransportLayerSummary::Tcp(TcpSegmentSummary {
            source_port: parse_u16(lookup.first("tcp.srcport")).unwrap_or_default(),
            destination_port: parse_u16(lookup.first("tcp.dstport")).unwrap_or_default(),
            sequence_number: parse_u32(lookup.any(&["tcp.seq", "tcp.seq_raw"])).unwrap_or_default(),
            acknowledgement_number: parse_u32(lookup.any(&["tcp.ack", "tcp.ack_raw"]))
                .unwrap_or_default(),
            flags: parse_u16(lookup.any(&["tcp.flags", "tcp.flags.str"])).unwrap_or_default(),
        }));
    }

    if lookup.first("udp.srcport").is_some() {
        return Some(TransportLayerSummary::Udp(UdpDatagramSummary {
            source_port: parse_u16(lookup.first("udp.srcport")).unwrap_or_default(),
            destination_port: parse_u16(lookup.first("udp.dstport")).unwrap_or_default(),
            length: parse_u16(lookup.first("udp.length")).unwrap_or_default(),
        }));
    }

    if lookup.any(&["icmp.type", "icmpv6.type"]).is_some() {
        return Some(TransportLayerSummary::Icmp(IcmpPacketSummary {
            icmp_type: parse_u8(lookup.any(&["icmp.type", "icmpv6.type"])).unwrap_or_default(),
            code: parse_u8(lookup.any(&["icmp.code", "icmpv6.code"])).unwrap_or_default(),
        }));
    }

    None
}

fn infer_application_layer(lookup: &FieldLookup) -> Option<ApplicationLayerSummary> {
    if lookup
        .any(&["http.request.method", "http.response.code"])
        .is_some()
    {
        let status_code = parse_u16(lookup.first("http.response.code"));
        return Some(ApplicationLayerSummary::Http(HttpMessageSummary {
            kind: if status_code.is_some() {
                "response".to_string()
            } else {
                "request".to_string()
            },
            method: lookup.first("http.request.method").map(ToString::to_string),
            path: lookup
                .any(&["http.request.uri", "http.request.full_uri"])
                .map(ToString::to_string),
            status_code,
            reason_phrase: lookup
                .first("http.response.phrase")
                .map(ToString::to_string),
            host: lookup.first("http.host").map(ToString::to_string),
        }));
    }

    if lookup.first("dns.id").is_some() {
        return Some(ApplicationLayerSummary::Dns(DnsMessageSummary {
            id: parse_u16(lookup.first("dns.id")).unwrap_or_default(),
            is_response: parse_bool(lookup.first("dns.flags.response")).unwrap_or(false),
            opcode: parse_u8(lookup.first("dns.flags.opcode")).unwrap_or_default(),
            question_count: parse_u16(lookup.first("dns.count.queries")).unwrap_or_default(),
            answer_count: parse_u16(lookup.first("dns.count.answers")).unwrap_or_default(),
            questions: lookup
                .all("dns.qry.name")
                .into_iter()
                .filter(|value| !value.is_empty())
                .collect(),
        }));
    }

    if lookup
        .any(&["tls.handshake.type", "ssl.handshake.type"])
        .is_some()
    {
        let handshake_code = lookup
            .any(&["tls.handshake.type", "ssl.handshake.type"])
            .unwrap_or_default();
        return Some(ApplicationLayerSummary::TlsHandshake(TlsHandshakeSummary {
            record_version: lookup
                .any(&["tls.record.version", "ssl.record.version"])
                .unwrap_or("unknown")
                .to_string(),
            handshake_type: tls_handshake_name(handshake_code).to_string(),
            handshake_length: parse_u32(
                lookup.any(&["tls.handshake.length", "ssl.handshake.length"]),
            )
            .unwrap_or_default(),
            server_name: lookup
                .any(&[
                    "tls.handshake.extensions_server_name",
                    "ssl.handshake.extensions_server_name",
                ])
                .map(ToString::to_string),
        }));
    }

    None
}

fn parse_u8(value: Option<&str>) -> Option<u8> {
    parse_integer(value).and_then(|parsed| u8::try_from(parsed).ok())
}

fn parse_u16(value: Option<&str>) -> Option<u16> {
    parse_integer(value).and_then(|parsed| u16::try_from(parsed).ok())
}

fn parse_u32(value: Option<&str>) -> Option<u32> {
    parse_integer(value).and_then(|parsed| u32::try_from(parsed).ok())
}

fn parse_integer(value: Option<&str>) -> Option<u64> {
    let trimmed = value?.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(hex) = trimmed.strip_prefix("0x") {
        return u64::from_str_radix(hex, 16).ok();
    }

    trimmed.parse::<u64>().ok()
}

fn parse_bool(value: Option<&str>) -> Option<bool> {
    match value?.trim() {
        "1" | "true" | "True" => Some(true),
        "0" | "false" | "False" => Some(false),
        _ => None,
    }
}

fn tls_handshake_name(code: &str) -> &'static str {
    match parse_u8(Some(code)) {
        Some(1) => "ClientHello",
        Some(2) => "ServerHello",
        Some(11) => "Certificate",
        Some(20) => "Finished",
        Some(8) => "EncryptedExtensions",
        Some(13) => "CertificateRequest",
        Some(15) => "CertificateVerify",
        _ => "Handshake",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use session_model::{PacketSummary, TimestampPrecision};

    #[test]
    fn parses_pdml_into_field_tree_and_summaries() {
        let xml = r#"<?xml version="1.0"?>
        <pdml>
          <packet>
            <proto name="geninfo" showname="General information"/>
            <proto name="eth" showname="Ethernet II">
              <field name="eth.dst" showname="Destination: aa:bb:cc:dd:ee:ff" pos="0" size="6"/>
              <field name="eth.src" showname="Source: 11:22:33:44:55:66" pos="6" size="6"/>
              <field name="eth.type" show="0x0800" showname="Type: IPv4 (0x0800)" pos="12" size="2"/>
            </proto>
            <proto name="ip" showname="Internet Protocol Version 4">
              <field name="ip.src" show="192.168.0.10" showname="Source Address: 192.168.0.10"/>
              <field name="ip.dst" show="1.1.1.1" showname="Destination Address: 1.1.1.1"/>
              <field name="ip.proto" show="6"/>
              <field name="ip.ttl" show="64"/>
              <field name="ip.hdr_len" show="20"/>
              <field name="ip.len" show="60"/>
            </proto>
            <proto name="tcp" showname="Transmission Control Protocol">
              <field name="tcp.srcport" show="55234"/>
              <field name="tcp.dstport" show="443"/>
              <field name="tcp.seq" show="1"/>
              <field name="tcp.ack" show="2"/>
              <field name="tcp.flags" show="0x0018"/>
            </proto>
            <proto name="http" showname="Hypertext Transfer Protocol">
              <field name="http.request.method" show="GET"/>
              <field name="http.request.uri" show="/"/>
              <field name="http.host" show="example.com"/>
            </proto>
          </packet>
        </pdml>"#;

        let packet = CapturedPacket {
            summary: PacketSummary {
                index: 0,
                timestamp_seconds: 0,
                timestamp_fraction: 0,
                timestamp_precision: TimestampPrecision::Microseconds,
                captured_length: 60,
                original_length: 60,
            },
            raw_bytes: vec![0; 60],
            linktype: 1,
        };

        let decoded = parse_pdml_packet(xml, &packet).expect("pdml should parse");
        assert!(matches!(decoded.link, LinkLayerSummary::Ethernet(_)));
        assert!(matches!(
            decoded.network,
            Some(NetworkLayerSummary::Ipv4(_))
        ));
        assert!(matches!(
            decoded.transport,
            Some(TransportLayerSummary::Tcp(_))
        ));
        assert!(matches!(
            decoded.application,
            Some(ApplicationLayerSummary::Http(_))
        ));
        assert_eq!(decoded.fields.len(), 4);
    }

    #[test]
    fn returns_none_for_unsupported_filter_translation() {
        let result = filter_packet_indexes(Path::new("/tmp/unused.pcap"), "endpoint=10.0.0.1:443")
            .expect("unsupported shorthand should not error");
        assert!(result.is_none());
    }
}
