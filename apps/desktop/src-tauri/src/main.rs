use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use app_services::{
    CaptureError, CaptureStatsInput, CaptureStatsService, ConversationsInput, ConversationsService,
    InspectCaptureInput, InspectCaptureService, InspectPacketInput, InspectPacketService,
    ListPacketsInput, ListPacketsService, LiveCaptureCoordinator, LiveCaptureSession,
    SaveCaptureInput, SaveCaptureService, StartLiveCaptureInput, StreamsInput, StreamsService,
    TransactionsInput, TransactionsService,
};
use output_formatters::{
    render_capture_report_json, render_capture_stats_report_json, render_conversation_report_json,
    render_packet_detail_report_json, render_packet_list_report_json,
    render_save_capture_report_json, render_stream_report_json, render_transaction_report_json,
};
use serde_json::json;

#[derive(Debug, Default)]
struct DesktopCaptureState {
    session: Mutex<Option<LiveCaptureSession>>,
}

#[tauri::command]
fn inspect_capture(path: String) -> Result<serde_json::Value, String> {
    let path = parse_capture_path(&path)?;
    let report = InspectCaptureService::default().inspect(InspectCaptureInput { path })?;
    parse_json_output(render_capture_report_json(&report))
}

#[tauri::command]
fn list_packets(
    path: String,
    limit: Option<u64>,
    filter: Option<String>,
) -> Result<serde_json::Value, String> {
    let path = parse_capture_path(&path)?;
    let limit = match limit {
        Some(value) => {
            Some(usize::try_from(value).map_err(|_| "packet limit is too large".to_string())?)
        }
        None => None,
    };
    let report = ListPacketsService::default().list(ListPacketsInput {
        path,
        limit,
        filter: normalize_optional(filter),
    })?;
    parse_json_output(render_packet_list_report_json(&report))
}

#[tauri::command]
fn inspect_packet(path: String, packet_index: u64) -> Result<serde_json::Value, String> {
    let path = parse_capture_path(&path)?;
    let report =
        InspectPacketService::default().inspect(InspectPacketInput { path, packet_index })?;
    parse_json_output(render_packet_detail_report_json(&report))
}

#[tauri::command]
fn capture_stats(path: String, filter: Option<String>) -> Result<serde_json::Value, String> {
    let path = parse_capture_path(&path)?;
    let report = CaptureStatsService::default().stats(CaptureStatsInput {
        path,
        filter: normalize_optional(filter),
    })?;
    parse_json_output(render_capture_stats_report_json(&report))
}

#[tauri::command]
fn list_conversations(path: String, filter: Option<String>) -> Result<serde_json::Value, String> {
    let path = parse_capture_path(&path)?;
    let report = ConversationsService::default().list(ConversationsInput {
        path,
        filter: normalize_optional(filter),
    })?;
    parse_json_output(render_conversation_report_json(&report))
}

#[tauri::command]
fn list_streams(
    path: String,
    filter: Option<String>,
    stream_filter: Option<String>,
) -> Result<serde_json::Value, String> {
    let path = parse_capture_path(&path)?;
    let report = StreamsService::default().list(StreamsInput {
        path,
        filter: normalize_optional(filter),
        stream_filter: normalize_optional(stream_filter),
    })?;
    parse_json_output(render_stream_report_json(&report))
}

#[tauri::command]
fn list_transactions(
    path: String,
    filter: Option<String>,
    transaction_filter: Option<String>,
) -> Result<serde_json::Value, String> {
    let path = parse_capture_path(&path)?;
    let report = TransactionsService::default().list(TransactionsInput {
        path,
        filter: normalize_optional(filter),
        transaction_filter: normalize_optional(transaction_filter),
    })?;
    parse_json_output(render_transaction_report_json(&report))
}

#[tauri::command]
fn save_capture(
    source_path: String,
    output_path: String,
    filter: Option<String>,
    stream_filter: Option<String>,
) -> Result<serde_json::Value, String> {
    let source_path = parse_capture_path(&source_path)?;
    let output_path = parse_capture_path(&output_path)?;
    let report = SaveCaptureService::default().save(SaveCaptureInput {
        source_path,
        output_path,
        filter: normalize_optional(filter),
        stream_filter: normalize_optional(stream_filter),
    })?;
    parse_json_output(render_save_capture_report_json(&report))
}

#[tauri::command]
fn export_conversations(
    path: String,
    output_path: String,
    filter: Option<String>,
    format: String,
) -> Result<String, String> {
    let path = parse_capture_path(&path)?;
    let output_path = parse_capture_path(&output_path)?;
    let format = parse_export_format(&format)?;
    let report = ConversationsService::default().list(ConversationsInput {
        path,
        filter: normalize_optional(filter),
    })?;

    let payload = match format {
        ExportFormat::Json => render_conversation_report_json(&report),
        ExportFormat::Csv => render_conversation_report_csv(&report),
    };
    fs::write(&output_path, payload).map_err(|error| {
        format!(
            "failed to write export file {}: {error}",
            output_path.display()
        )
    })?;
    Ok(format!(
        "exported {} conversation rows to {} ({})",
        report.conversations.len(),
        output_path.display(),
        format.as_name(),
    ))
}

#[tauri::command]
fn export_streams(
    path: String,
    output_path: String,
    filter: Option<String>,
    stream_filter: Option<String>,
    format: String,
) -> Result<String, String> {
    let path = parse_capture_path(&path)?;
    let output_path = parse_capture_path(&output_path)?;
    let format = parse_export_format(&format)?;
    let report = StreamsService::default().list(StreamsInput {
        path,
        filter: normalize_optional(filter),
        stream_filter: normalize_optional(stream_filter),
    })?;

    let payload = match format {
        ExportFormat::Json => render_stream_report_json(&report),
        ExportFormat::Csv => render_stream_report_csv(&report),
    };
    fs::write(&output_path, payload).map_err(|error| {
        format!(
            "failed to write export file {}: {error}",
            output_path.display()
        )
    })?;
    Ok(format!(
        "exported {} stream rows to {} ({})",
        report.streams.len(),
        output_path.display(),
        format.as_name(),
    ))
}

#[tauri::command]
fn export_transactions(
    path: String,
    output_path: String,
    filter: Option<String>,
    transaction_filter: Option<String>,
    format: String,
) -> Result<String, String> {
    let path = parse_capture_path(&path)?;
    let output_path = parse_capture_path(&output_path)?;
    let format = parse_export_format(&format)?;
    let report = TransactionsService::default().list(TransactionsInput {
        path,
        filter: normalize_optional(filter),
        transaction_filter: normalize_optional(transaction_filter),
    })?;

    let payload = match format {
        ExportFormat::Json => render_transaction_report_json(&report),
        ExportFormat::Csv => render_transaction_report_csv(&report),
    };
    fs::write(&output_path, payload).map_err(|error| {
        format!(
            "failed to write export file {}: {error}",
            output_path.display()
        )
    })?;
    Ok(format!(
        "exported {} transaction rows to {} ({})",
        report.transactions.len(),
        output_path.display(),
        format.as_name(),
    ))
}

#[tauri::command]
fn sample_capture_paths() -> Vec<String> {
    let candidates = [
        workspace_path("fixtures/golden/sample.pcap"),
        workspace_path("fixtures/golden/sample.pcapng"),
    ];

    candidates
        .into_iter()
        .filter(|path| path.exists())
        .map(|path| path.display().to_string())
        .collect()
}

#[tauri::command]
fn capture_runtime_info() -> serde_json::Value {
    let runtime = LiveCaptureCoordinator::default().runtime_info();
    json!({
        "backend": runtime.backend.as_str(),
        "tool_path": runtime.tool_path,
    })
}

#[tauri::command]
fn capture_interfaces() -> Result<serde_json::Value, String> {
    let interfaces = LiveCaptureCoordinator::default()
        .list_interfaces()
        .map_err(render_capture_error)?;
    Ok(json!({
        "interfaces": interfaces.into_iter().map(|entry| entry.name).collect::<Vec<_>>(),
    }))
}

#[tauri::command]
fn capture_start(
    interface: Option<String>,
    state: tauri::State<'_, DesktopCaptureState>,
) -> Result<serde_json::Value, String> {
    let mut guard = state
        .session
        .lock()
        .map_err(|_| "failed to lock live capture session state".to_string())?;
    if guard.is_some() {
        return Err("a live capture is already running; stop it first".to_string());
    }

    let session = LiveCaptureCoordinator::default()
        .start(StartLiveCaptureInput {
            interface: normalize_optional(interface),
        })
        .map_err(render_capture_error)?;
    let interface_name = session.interface().to_string();
    let path = session.path().display().to_string();
    *guard = Some(session);

    Ok(json!({
        "state": "running",
        "interface": interface_name,
        "path": path,
    }))
}

#[tauri::command]
fn capture_status(
    state: tauri::State<'_, DesktopCaptureState>,
) -> Result<serde_json::Value, String> {
    let runtime = LiveCaptureCoordinator::default().runtime_info();
    let mut guard = state
        .session
        .lock()
        .map_err(|_| "failed to lock live capture session state".to_string())?;

    if let Some(session) = guard.as_mut() {
        let running = session.is_running().map_err(render_capture_error)?;
        let status = if running { "running" } else { "exited" };
        return Ok(json!({
            "state": status,
            "interface": session.interface(),
            "path": session.path().display().to_string(),
            "backend": runtime.backend.as_str(),
            "tool_path": runtime.tool_path,
        }));
    }

    Ok(json!({
        "state": "idle",
        "backend": runtime.backend.as_str(),
        "tool_path": runtime.tool_path,
    }))
}

#[tauri::command]
fn capture_stop(state: tauri::State<'_, DesktopCaptureState>) -> Result<serde_json::Value, String> {
    let session = {
        let mut guard = state
            .session
            .lock()
            .map_err(|_| "failed to lock live capture session state".to_string())?;
        guard
            .take()
            .ok_or_else(|| "no live capture is running".to_string())?
    };

    let path = session.stop().map_err(render_capture_error)?;
    let report =
        InspectCaptureService::default().inspect(InspectCaptureInput { path: path.clone() })?;
    let report_json = parse_json_output(render_capture_report_json(&report))?;

    Ok(json!({
        "state": "stopped",
        "path": path.display().to_string(),
        "capture": report_json,
    }))
}

fn parse_capture_path(path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("capture path is required".to_string());
    }
    Ok(PathBuf::from(trimmed))
}

fn render_capture_error(error: CaptureError) -> String {
    match error {
        CaptureError::ToolUnavailable(message) => format!(
            "capture tool unavailable: {message}\n\nhint: install a pcap-compatible capture tool or set ICESNIFF_CAPTURE_TOOL (and optionally ICESNIFF_CAPTURE_BACKEND) to match your capture provider"
        ),
        CaptureError::PermissionDenied(message) => format!(
            "capture permission denied: {message}\n\nhint: grant packet-capture privileges (for example root/administrator, or Npcap/libpcap capture permissions)"
        ),
        CaptureError::DriverUnavailable(message) => format!(
            "capture backend unavailable: {message}\n\nhint: ensure libpcap/Npcap is installed and its capture driver service is running"
        ),
        CaptureError::NoInterfacesAvailable => "no capture interfaces are available".to_string(),
        other => other.to_string(),
    }
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    match value {
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        None => None,
    }
}

fn parse_json_output(payload: String) -> Result<serde_json::Value, String> {
    serde_json::from_str(&payload)
        .map_err(|error| format!("failed to parse generated report JSON payload: {error}"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExportFormat {
    Json,
    Csv,
}

impl ExportFormat {
    fn as_name(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Csv => "csv",
        }
    }
}

fn parse_export_format(value: &str) -> Result<ExportFormat, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "json" => Ok(ExportFormat::Json),
        "csv" => Ok(ExportFormat::Csv),
        other => Err(format!(
            "unsupported export format '{other}', expected 'json' or 'csv'"
        )),
    }
}

fn render_conversation_report_csv(report: &session_model::ConversationReport) -> String {
    let mut rows = vec![
        "service,protocol,endpoint_a,endpoint_b,packets,packets_a_to_b,packets_b_to_a,request_count,response_count,total_captured_bytes,first_packet_index,last_packet_index".to_string(),
    ];

    for row in &report.conversations {
        rows.push(format!(
            "{},{},{},{},{},{},{},{},{},{},{},{}",
            csv_escape(&row.service),
            csv_escape(&row.protocol),
            csv_escape(&row.endpoint_a),
            csv_escape(&row.endpoint_b),
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

    rows.join("\n")
}

fn render_stream_report_csv(report: &session_model::StreamReport) -> String {
    let mut rows = vec![
        "service,protocol,client,server,packets,session_state,request_count,response_count,matched_transactions,unmatched_requests,unmatched_responses,tls_handshake_state,tls_alert_count,first_packet_index,last_packet_index,notes,timeline".to_string(),
    ];

    for row in &report.streams {
        rows.push(format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            csv_escape(&row.service),
            csv_escape(&row.protocol),
            csv_escape(&row.client),
            csv_escape(&row.server),
            row.packets,
            csv_escape(&row.session_state),
            row.request_count,
            row.response_count,
            row.matched_transactions,
            row.unmatched_requests,
            row.unmatched_responses,
            csv_escape(&row.tls_handshake_state),
            row.tls_alert_count,
            row.first_packet_index,
            row.last_packet_index,
            csv_escape(&row.notes.join(" | ")),
            csv_escape(&row.transaction_timeline.join(" | ")),
        ));
    }

    rows.join("\n")
}

fn render_transaction_report_csv(report: &session_model::TransactionReport) -> String {
    let mut rows = vec![
        "service,protocol,client,server,sequence,request_summary,request_details,response_summary,response_details,state,notes".to_string(),
    ];

    for row in &report.transactions {
        let request_details = row
            .request_details
            .iter()
            .map(|detail| format!("{}={}", detail.key, detail.value))
            .collect::<Vec<_>>()
            .join(" | ");
        let response_details = row
            .response_details
            .iter()
            .map(|detail| format!("{}={}", detail.key, detail.value))
            .collect::<Vec<_>>()
            .join(" | ");

        rows.push(format!(
            "{},{},{},{},{},{},{},{},{},{},{}",
            csv_escape(&row.service),
            csv_escape(&row.protocol),
            csv_escape(&row.client),
            csv_escape(&row.server),
            row.sequence,
            csv_escape(&row.request_summary),
            csv_escape(&request_details),
            csv_escape(&row.response_summary),
            csv_escape(&response_details),
            csv_escape(&row.state),
            csv_escape(&row.notes.join(" | ")),
        ));
    }

    rows.join("\n")
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn workspace_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(relative)
}

fn main() {
    tauri::Builder::default()
        .manage(DesktopCaptureState::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            inspect_capture,
            list_packets,
            inspect_packet,
            capture_stats,
            capture_runtime_info,
            capture_interfaces,
            capture_start,
            capture_status,
            capture_stop,
            list_conversations,
            list_streams,
            list_transactions,
            save_capture,
            export_conversations,
            export_streams,
            export_transactions,
            sample_capture_paths
        ])
        .run(tauri::generate_context!())
        .expect("failed to run IceSniff desktop app");
}
