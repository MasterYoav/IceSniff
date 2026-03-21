use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::{Child, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use capture_engine::{
    parse_capture_interface_line, temp_capture_path, ActiveCaptureSession, CaptureEngine,
};
pub use capture_engine::{CaptureBackend, CaptureError, CaptureInterface};
use file_io::{capture_file_size, read_capture, write_pcap};
use filter_engine::matches_filter;
use parser_core::{
    capture_stats, conversations, decode_captured_packet, inspect_metadata, inspect_packet,
    list_packets, stream_packet_indexes, streams, transactions,
};
use session_model::{
    CaptureReport, CaptureStatsReport, ConversationReport, PacketDetailReport, PacketListReport,
    SaveCaptureReport, StreamReport, TransactionReport,
};

#[derive(Debug)]
pub struct LiveCaptureSession {
    session: LiveCaptureSessionKind,
}

#[derive(Debug)]
enum LiveCaptureSessionKind {
    Engine(ActiveCaptureSession),
    Helper(HelperCaptureSession),
}

#[derive(Debug)]
struct HelperCaptureSession {
    interface: String,
    path: PathBuf,
    stop_file: PathBuf,
    child: Child,
}

impl LiveCaptureSession {
    pub fn interface(&self) -> &str {
        match &self.session {
            LiveCaptureSessionKind::Engine(session) => session.interface(),
            LiveCaptureSessionKind::Helper(session) => &session.interface,
        }
    }

    pub fn path(&self) -> &Path {
        match &self.session {
            LiveCaptureSessionKind::Engine(session) => session.path(),
            LiveCaptureSessionKind::Helper(session) => &session.path,
        }
    }

    pub fn is_running(&mut self) -> Result<bool, CaptureError> {
        match &mut self.session {
            LiveCaptureSessionKind::Engine(session) => session.is_running(),
            LiveCaptureSessionKind::Helper(session) => session
                .child
                .try_wait()
                .map(|status| status.is_none())
                .map_err(|error| CaptureError::ProcessPollFailed(error.to_string())),
        }
    }

    pub fn stop(self) -> Result<PathBuf, CaptureError> {
        match self.session {
            LiveCaptureSessionKind::Engine(session) => session.stop(),
            LiveCaptureSessionKind::Helper(session) => stop_helper_capture(session),
        }
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
        if let Some(helper) = resolve_capture_helper() {
            return helper_list_interfaces(&helper);
        }
        self.engine.available_interfaces()
    }

    pub fn runtime_info(&self) -> CaptureRuntimeInfo {
        CaptureRuntimeInfo {
            tool_path: self.engine.tool_path().to_string(),
            backend: self.engine.backend(),
        }
    }

    pub fn start(&self, input: StartLiveCaptureInput) -> Result<LiveCaptureSession, CaptureError> {
        if let Some(helper) = resolve_capture_helper() {
            return start_helper_capture(&helper, input.interface);
        }

        let interface = match input.interface {
            Some(interface) => interface,
            None => self.engine.default_interface()?.name,
        };
        let path = temp_capture_path("pcap");
        let session = self.engine.start_capture(&interface, path)?;
        Ok(LiveCaptureSession {
            session: LiveCaptureSessionKind::Engine(session),
        })
    }
}

fn resolve_capture_helper() -> Option<PathBuf> {
    if let Ok(helper) = env::var("ICESNIFF_CAPTURE_HELPER_BIN") {
        let helper = PathBuf::from(helper);
        if helper.is_file() {
            return Some(helper);
        }
    }

    if let Ok(current_exe) = env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            for candidate in [
                parent.join("icesniff-capture-helper"),
                parent.join("icesniff-capture-helper.exe"),
                parent
                    .parent()
                    .map(|value| value.join("libexec").join("icesniff-capture-helper"))
                    .unwrap_or_default(),
                parent
                    .parent()
                    .map(|value| value.join("libexec").join("icesniff-capture-helper.exe"))
                    .unwrap_or_default(),
            ] {
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../..")
        .canonicalize()
        .ok()?;

    let candidates = [
        repo_root
            .join("apps")
            .join("macos")
            .join("rust-engine")
            .join("target")
            .join("debug")
            .join("icesniff-capture-helper"),
        repo_root
            .join("apps")
            .join("macos")
            .join("rust-engine")
            .join("target")
            .join("release")
            .join("icesniff-capture-helper"),
        repo_root
            .join("apps")
            .join("macos")
            .join("Sources")
            .join("IceSniffMac")
            .join("Resources")
            .join("BundledCLI")
            .join("icesniff-capture-helper"),
    ];

    candidates.into_iter().find(|candidate| candidate.is_file())
}

fn helper_list_interfaces(helper: &Path) -> Result<Vec<CaptureInterface>, CaptureError> {
    let output = Command::new(helper)
        .arg("list-interfaces")
        .output()
        .map_err(|error| CaptureError::InterfaceEnumerationFailed(error.to_string()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(CaptureError::InterfaceEnumerationFailed(stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(parse_capture_interface_line)
        .collect())
}

fn start_helper_capture(
    helper: &Path,
    interface: Option<String>,
) -> Result<LiveCaptureSession, CaptureError> {
    let interfaces = helper_list_interfaces(helper)?;
    let interface = interface
        .or_else(|| {
            interfaces
                .iter()
                .find(|item| item.name == "en0")
                .map(|item| item.name.clone())
        })
        .or_else(|| interfaces.first().map(|item| item.name.clone()))
        .ok_or(CaptureError::NoInterfacesAvailable)?;
    let path = temp_capture_path("pcap");
    let stop_file = temp_capture_path("stop");
    let mut child = Command::new(helper)
        .args([
            "start",
            "--interface",
            interface.as_str(),
            "--output",
            path.to_string_lossy().as_ref(),
            "--stop-file",
            stop_file.to_string_lossy().as_ref(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| CaptureError::StartFailed(error.to_string()))?;

    thread::sleep(Duration::from_millis(350));
    if child
        .try_wait()
        .map_err(|error| CaptureError::ProcessPollFailed(error.to_string()))?
        .is_some()
    {
        let output = child
            .wait_with_output()
            .map_err(|error| CaptureError::StopFailed(error.to_string()))?;
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(CaptureError::StartFailed(stderr));
    }

    let deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < deadline {
        if capture_file_is_ready(&path) {
            return Ok(LiveCaptureSession {
                session: LiveCaptureSessionKind::Helper(HelperCaptureSession {
                    interface,
                    path,
                    stop_file,
                    child,
                }),
            });
        }

        if child
            .try_wait()
            .map_err(|error| CaptureError::ProcessPollFailed(error.to_string()))?
            .is_some()
        {
            let output = child
                .wait_with_output()
                .map_err(|error| CaptureError::StopFailed(error.to_string()))?;
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(CaptureError::StartFailed(if stderr.is_empty() {
                "capture helper exited before creating a readable capture file".to_string()
            } else {
                stderr
            }));
        }

        thread::sleep(Duration::from_millis(100));
    }

    Err(CaptureError::StartFailed(
        "capture helper did not create a readable capture file in time".to_string(),
    ))
}

fn stop_helper_capture(mut session: HelperCaptureSession) -> Result<PathBuf, CaptureError> {
    let _ = fs::write(&session.stop_file, b"stop");
    for _ in 0..15 {
        if session
            .child
            .try_wait()
            .map_err(|error| CaptureError::ProcessPollFailed(error.to_string()))?
            .is_some()
        {
            let _ = fs::remove_file(&session.stop_file);
            if session.path.is_file() {
                return Ok(session.path);
            }
            return Err(CaptureError::StopFailed(
                "capture ended without a readable capture file".to_string(),
            ));
        }
        thread::sleep(Duration::from_millis(100));
    }

    let _ = session.child.kill();
    let _ = session.child.wait();
    let _ = fs::remove_file(&session.stop_file);
    if session.path.is_file() {
        return Ok(session.path);
    }
    Err(CaptureError::StopFailed(
        "capture helper did not stop cleanly".to_string(),
    ))
}

fn capture_file_is_ready(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.len() > 0)
        .unwrap_or(false)
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
