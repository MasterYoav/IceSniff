use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use capture_engine::{temp_capture_path, ActiveCaptureSession, CaptureEngine};
pub use capture_engine::{CaptureBackend, CaptureError, CaptureInterface};
use file_io::{capture_file_size, read_capture, write_pcap};
use filter_engine::matches_filter;
use parser_core::{
    capture_stats, conversations, decode_captured_packet, inspect_metadata, inspect_packet,
    list_packets, packet_indexes_for_filter, stream_packet_indexes, streams, transactions,
};
use session_model::{
    CaptureReport, CaptureStatsReport, ConversationReport, PacketDetailReport, PacketListReport,
    SaveCaptureReport, StreamReport, TransactionReport,
};

#[derive(Debug)]
pub struct LiveCaptureSession {
    session: ActiveCaptureSession,
}

impl LiveCaptureSession {
    pub fn interface(&self) -> &str {
        self.session.interface()
    }

    pub fn path(&self) -> &Path {
        self.session.path()
    }

    pub fn is_running(&mut self) -> Result<bool, CaptureError> {
        self.session.is_running()
    }

    pub fn stop(self) -> Result<PathBuf, CaptureError> {
        self.session.stop()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartLiveCaptureInput {
    pub interface: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveCaptureCoordinator {
    engine: CaptureEngine,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureRuntimeInfo {
    pub tool_path: String,
    pub backend: CaptureBackend,
}

impl Default for LiveCaptureCoordinator {
    fn default() -> Self {
        Self {
            engine: CaptureEngine::default(),
        }
    }
}

impl LiveCaptureCoordinator {
    pub fn with_engine(engine: CaptureEngine) -> Self {
        Self { engine }
    }

    pub fn list_interfaces(&self) -> Result<Vec<CaptureInterface>, CaptureError> {
        self.engine.available_interfaces()
    }

    pub fn runtime_info(&self) -> CaptureRuntimeInfo {
        CaptureRuntimeInfo {
            tool_path: self.engine.tool_path().to_string(),
            backend: self.engine.backend(),
        }
    }

    pub fn start(&self, input: StartLiveCaptureInput) -> Result<LiveCaptureSession, CaptureError> {
        let interface = match input.interface {
            Some(interface) => interface,
            None => self.engine.default_interface()?.name,
        };
        let path = temp_capture_path("pcap");
        let session = self.engine.start_capture(&interface, path)?;
        Ok(LiveCaptureSession { session })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveCaptureInput {
    pub source_path: PathBuf,
    pub output_path: PathBuf,
    pub filter: Option<String>,
    pub stream_filter: Option<String>,
}

#[derive(Debug, Default)]
pub struct SaveCaptureService;

impl SaveCaptureService {
    pub fn save(&self, input: SaveCaptureInput) -> Result<SaveCaptureReport, String> {
        if input.source_path == input.output_path {
            return Err("source and output capture paths must be different".to_string());
        }

        let capture = read_capture(&input.source_path)?;
        let selected_packets = select_packets(
            &capture,
            input.filter.as_deref(),
            input.stream_filter.as_deref(),
        )?;
        write_pcap(&input.output_path, &selected_packets)?;

        Ok(SaveCaptureReport {
            source_path: input.source_path,
            output_path: input.output_path,
            format: session_model::CaptureFormat::Pcap,
            packets_written: selected_packets.len() as u64,
            filter: input.filter,
            stream_filter: input.stream_filter,
        })
    }
}

fn select_packets(
    capture: &session_model::LoadedCapture,
    filter: Option<&str>,
    stream_filter: Option<&str>,
) -> Result<Vec<session_model::CapturedPacket>, String> {
    if let Some(stream_expression) = stream_filter {
        let selected_indexes = stream_packet_indexes(capture, filter, stream_expression)?
            .into_iter()
            .collect::<BTreeSet<_>>();
        return Ok(capture
            .packets
            .iter()
            .filter(|packet| selected_indexes.contains(&packet.summary.index))
            .cloned()
            .collect());
    }

    let Some(expression) = filter else {
        return Ok(capture.packets.clone());
    };

    if let Some(selected_indexes) = packet_indexes_for_filter(capture, expression)? {
        return Ok(capture
            .packets
            .iter()
            .filter(|packet| selected_indexes.contains(&packet.summary.index))
            .cloned()
            .collect());
    }

    capture
        .packets
        .iter()
        .filter_map(|packet| {
            let decoded = decode_captured_packet(packet);
            match matches_filter(&decoded, expression) {
                Ok(true) => Some(Ok(packet.clone())),
                Ok(false) => None,
                Err(error) => Some(Err(error)),
            }
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectCaptureInput {
    pub path: PathBuf,
}

#[derive(Debug, Default)]
pub struct InspectCaptureService;

impl InspectCaptureService {
    pub fn inspect(&self, input: InspectCaptureInput) -> Result<CaptureReport, String> {
        let capture = read_capture(&input.path)?;
        let size_bytes = capture_file_size(&input.path)?;
        Ok(inspect_metadata(
            &input.path,
            capture.format,
            Some(capture.packets.len() as u64),
            size_bytes,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListPacketsInput {
    pub path: PathBuf,
    pub limit: Option<usize>,
    pub filter: Option<String>,
}

#[derive(Debug, Default)]
pub struct ListPacketsService;

impl ListPacketsService {
    pub fn list(&self, input: ListPacketsInput) -> Result<PacketListReport, String> {
        let capture = read_capture(&input.path)?;
        list_packets(&capture, input.limit, input.filter.as_deref())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectPacketInput {
    pub path: PathBuf,
    pub packet_index: u64,
}

#[derive(Debug, Default)]
pub struct InspectPacketService;

impl InspectPacketService {
    pub fn inspect(&self, input: InspectPacketInput) -> Result<PacketDetailReport, String> {
        let capture = read_capture(&input.path)?;
        inspect_packet(&capture, input.packet_index)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureStatsInput {
    pub path: PathBuf,
    pub filter: Option<String>,
}

#[derive(Debug, Default)]
pub struct CaptureStatsService;

impl CaptureStatsService {
    pub fn stats(&self, input: CaptureStatsInput) -> Result<CaptureStatsReport, String> {
        let capture = read_capture(&input.path)?;
        capture_stats(&capture, input.filter.as_deref())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationsInput {
    pub path: PathBuf,
    pub filter: Option<String>,
}

#[derive(Debug, Default)]
pub struct ConversationsService;

impl ConversationsService {
    pub fn list(&self, input: ConversationsInput) -> Result<ConversationReport, String> {
        let capture = read_capture(&input.path)?;
        conversations(&capture, input.filter.as_deref())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamsInput {
    pub path: PathBuf,
    pub filter: Option<String>,
    pub stream_filter: Option<String>,
}

#[derive(Debug, Default)]
pub struct StreamsService;

impl StreamsService {
    pub fn list(&self, input: StreamsInput) -> Result<StreamReport, String> {
        let capture = read_capture(&input.path)?;
        streams(
            &capture,
            input.filter.as_deref(),
            input.stream_filter.as_deref(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionsInput {
    pub path: PathBuf,
    pub filter: Option<String>,
    pub transaction_filter: Option<String>,
}

#[derive(Debug, Default)]
pub struct TransactionsService;

impl TransactionsService {
    pub fn list(&self, input: TransactionsInput) -> Result<TransactionReport, String> {
        let capture = read_capture(&input.path)?;
        transactions(
            &capture,
            input.filter.as_deref(),
            input.transaction_filter.as_deref(),
        )
    }
}
