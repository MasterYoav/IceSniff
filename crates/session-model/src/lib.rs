use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureFormat {
    Pcap,
    PcapNg,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureReport {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub format: CaptureFormat,
    pub packet_count_hint: Option<u64>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimestampPrecision {
    Microseconds,
    Nanoseconds,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketSummary {
    pub index: u64,
    pub timestamp_seconds: u32,
    pub timestamp_fraction: u32,
    pub timestamp_precision: TimestampPrecision,
    pub captured_length: u32,
    pub original_length: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketListReport {
    pub path: PathBuf,
    pub format: CaptureFormat,
    pub packets: Vec<PacketListRow>,
    pub total_packets: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketListRow {
    pub summary: PacketSummary,
    pub source: String,
    pub destination: String,
    pub protocol: String,
    pub info: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationReport {
    pub path: PathBuf,
    pub format: CaptureFormat,
    pub total_conversations: u64,
    pub conversations: Vec<ConversationRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationRow {
    pub protocol: String,
    pub endpoint_a: String,
    pub endpoint_b: String,
    pub packets: u64,
    pub total_captured_bytes: u64,
    pub first_packet_index: u64,
    pub last_packet_index: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedCapture {
    pub path: PathBuf,
    pub format: CaptureFormat,
    pub packets: Vec<CapturedPacket>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedPacket {
    pub summary: PacketSummary,
    pub raw_bytes: Vec<u8>,
    pub linktype: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketDetailReport {
    pub path: PathBuf,
    pub format: CaptureFormat,
    pub packet: DecodedPacket,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureStatsReport {
    pub path: PathBuf,
    pub format: CaptureFormat,
    pub total_packets: u64,
    pub total_captured_bytes: u64,
    pub average_captured_bytes: u64,
    pub link_layer_counts: Vec<NamedCount>,
    pub network_layer_counts: Vec<NamedCount>,
    pub transport_layer_counts: Vec<NamedCount>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedCount {
    pub name: String,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedPacket {
    pub summary: PacketSummary,
    pub raw_bytes: Vec<u8>,
    pub link: LinkLayerSummary,
    pub network: Option<NetworkLayerSummary>,
    pub transport: Option<TransportLayerSummary>,
    pub application: Option<ApplicationLayerSummary>,
    pub fields: Vec<FieldNode>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldNode {
    pub name: String,
    pub value: String,
    pub byte_range: Option<ByteRange>,
    pub children: Vec<FieldNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByteRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkLayerSummary {
    Ethernet(EthernetFrameSummary),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EthernetFrameSummary {
    pub source_mac: String,
    pub destination_mac: String,
    pub ether_type: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkLayerSummary {
    Arp(ArpPacketSummary),
    Ipv4(Ipv4PacketSummary),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArpPacketSummary {
    pub operation: u16,
    pub sender_hardware_address: String,
    pub sender_protocol_address: String,
    pub target_hardware_address: String,
    pub target_protocol_address: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ipv4PacketSummary {
    pub source_ip: String,
    pub destination_ip: String,
    pub protocol: u8,
    pub ttl: u8,
    pub header_length: u8,
    pub total_length: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportLayerSummary {
    Tcp(TcpSegmentSummary),
    Udp(UdpDatagramSummary),
    Icmp(IcmpPacketSummary),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TcpSegmentSummary {
    pub source_port: u16,
    pub destination_port: u16,
    pub sequence_number: u32,
    pub acknowledgement_number: u32,
    pub flags: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdpDatagramSummary {
    pub source_port: u16,
    pub destination_port: u16,
    pub length: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcmpPacketSummary {
    pub icmp_type: u8,
    pub code: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicationLayerSummary {
    Dns(DnsMessageSummary),
    Http(HttpMessageSummary),
    TlsHandshake(TlsHandshakeSummary),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsMessageSummary {
    pub id: u16,
    pub is_response: bool,
    pub opcode: u8,
    pub question_count: u16,
    pub answer_count: u16,
    pub questions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsHandshakeSummary {
    pub record_version: String,
    pub handshake_type: String,
    pub handshake_length: u32,
    pub server_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpMessageSummary {
    pub kind: String,
    pub method: Option<String>,
    pub path: Option<String>,
    pub status_code: Option<u16>,
    pub reason_phrase: Option<String>,
    pub host: Option<String>,
}
