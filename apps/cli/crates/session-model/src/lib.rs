use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineInfoReport {
    pub schema_version: String,
    pub engine_version: String,
    pub capabilities: EngineCapabilitiesReport,
    pub capture: EngineCaptureSupport,
    pub filters: EngineFilterSupport,
    pub export: EngineExportSupport,
    pub dissectors: EngineDissectorSupport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineCapabilitiesReport {
    pub inspect: bool,
    pub packet_list: bool,
    pub packet_detail: bool,
    pub stats: bool,
    pub conversations: bool,
    pub streams: bool,
    pub transactions: bool,
    pub save: bool,
    pub live_capture: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineCaptureSupport {
    pub bundled_backend: bool,
    pub built_in_tcpdump: bool,
    pub interface_discovery: bool,
    pub requires_admin_for_live_capture: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineFilterSupport {
    pub packet_filters: bool,
    pub stream_filters: bool,
    pub transaction_filters: bool,
    pub shorthand_protocol_terms: bool,
    pub shorthand_port_terms: bool,
    pub case_insensitive_protocols: bool,
    pub alternate_and_operators: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineExportSupport {
    pub save_capture: bool,
    pub filtered_save: bool,
    pub whole_capture_save: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineDissectorSupport {
    pub protocols: Vec<String>,
}

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
pub struct SaveCaptureReport {
    pub source_path: PathBuf,
    pub output_path: PathBuf,
    pub format: CaptureFormat,
    pub packets_written: u64,
    pub filter: Option<String>,
    pub stream_filter: Option<String>,
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
    pub service: String,
    pub protocol: String,
    pub endpoint_a: String,
    pub endpoint_b: String,
    pub packets: u64,
    pub packets_a_to_b: u64,
    pub packets_b_to_a: u64,
    pub request_count: u64,
    pub response_count: u64,
    pub total_captured_bytes: u64,
    pub first_packet_index: u64,
    pub last_packet_index: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamReport {
    pub path: PathBuf,
    pub format: CaptureFormat,
    pub total_streams: u64,
    pub streams: Vec<StreamRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionReport {
    pub path: PathBuf,
    pub format: CaptureFormat,
    pub total_transactions: u64,
    pub transactions: Vec<TransactionRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionRow {
    pub service: String,
    pub protocol: String,
    pub client: String,
    pub server: String,
    pub sequence: u64,
    pub request_summary: String,
    pub request_details: Vec<TransactionDetail>,
    pub response_summary: String,
    pub response_details: Vec<TransactionDetail>,
    pub state: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionDetail {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamRow {
    pub service: String,
    pub protocol: String,
    pub client: String,
    pub server: String,
    pub packets: u64,
    pub syn_packets: u64,
    pub fin_packets: u64,
    pub rst_packets: u64,
    pub session_state: String,
    pub client_to_server_packets: u64,
    pub server_to_client_packets: u64,
    pub request_count: u64,
    pub response_count: u64,
    pub matched_transactions: u64,
    pub unmatched_requests: u64,
    pub unmatched_responses: u64,
    pub tls_client_hellos: u64,
    pub tls_server_hellos: u64,
    pub tls_certificates: u64,
    pub tls_finished_messages: u64,
    pub tls_handshake_cycles: u64,
    pub tls_incomplete_handshakes: u64,
    pub tls_handshake_state: String,
    pub tls_alert_count: u64,
    pub tls_alerts: Vec<String>,
    pub total_captured_bytes: u64,
    pub first_packet_index: u64,
    pub last_packet_index: u64,
    pub transaction_timeline: Vec<String>,
    pub notes: Vec<String>,
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
