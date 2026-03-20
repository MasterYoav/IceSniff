mod filter_input;
mod tui;

use std::env;
use std::fmt;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::filter_input::normalize_filter_expression;
use app_services::{
    CaptureError, CaptureStatsInput, CaptureStatsService, ConversationsInput, ConversationsService,
    InspectCaptureInput, InspectCaptureService, InspectPacketInput, InspectPacketService,
    ListPacketsInput, ListPacketsService, LiveCaptureCoordinator, LiveCaptureSession,
    SaveCaptureInput, SaveCaptureService, StartLiveCaptureInput, StreamsInput, StreamsService,
    TransactionsInput, TransactionsService,
};
use output_formatters::{
    render_capture_report, render_capture_report_json, render_capture_stats_report,
    render_capture_stats_report_json, render_conversation_report, render_conversation_report_json,
    render_engine_info_report, render_engine_info_report_json, render_packet_detail_report,
    render_packet_detail_report_json, render_packet_list_report, render_packet_list_report_json,
    render_save_capture_report, render_save_capture_report_json, render_stream_report,
    render_stream_report_json, render_transaction_report, render_transaction_report_json,
};
use session_model::{
    EngineCapabilitiesReport, EngineCaptureSupport, EngineDissectorSupport, EngineExportSupport,
    EngineFilterSupport, EngineInfoReport,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(error.exit_status);
    }
}

fn run() -> Result<(), CliError> {
    let cli = parse_cli(env::args().skip(1)).map_err(CliError::usage)?;

    match cli.command {
        Command::Help => {
            println!("{}", help_text());
            Ok(())
        }
        Command::App { path } => tui::run_app(path).map_err(CliError::from),
        Command::EngineInfo => {
            let report = engine_info_report();
            match cli.output_mode {
                OutputMode::Text => println!("{}", render_engine_info_report(&report)),
                OutputMode::Json => println!("{}", render_engine_info_report_json(&report)),
            }
            Ok(())
        }
        Command::Shell { path } => run_shell(path, cli.output_mode).map_err(CliError::from),
        Command::Save {
            source_path,
            output_path,
            filter,
            stream_filter,
        } => {
            let report = SaveCaptureService::default().save(SaveCaptureInput {
                source_path,
                output_path,
                filter,
                stream_filter,
            })?;
            match cli.output_mode {
                OutputMode::Text => println!("{}", render_save_capture_report(&report)),
                OutputMode::Json => println!("{}", render_save_capture_report_json(&report)),
            }
            Ok(())
        }
        Command::Inspect { path } => {
            let report = InspectCaptureService::default().inspect(InspectCaptureInput { path })?;
            match cli.output_mode {
                OutputMode::Text => println!("{}", render_capture_report(&report)),
                OutputMode::Json => println!("{}", render_capture_report_json(&report)),
            }
            Ok(())
        }
        Command::List {
            path,
            limit,
            filter,
        } => {
            let report = ListPacketsService::default().list(ListPacketsInput {
                path,
                limit,
                filter,
            })?;
            match cli.output_mode {
                OutputMode::Text => println!("{}", render_packet_list_report(&report)),
                OutputMode::Json => println!("{}", render_packet_list_report_json(&report)),
            }
            Ok(())
        }
        Command::ShowPacket { path, packet_index } => {
            let report = InspectPacketService::default()
                .inspect(InspectPacketInput { path, packet_index })?;
            match cli.output_mode {
                OutputMode::Text => println!("{}", render_packet_detail_report(&report)),
                OutputMode::Json => println!("{}", render_packet_detail_report_json(&report)),
            }
            Ok(())
        }
        Command::Stats { path, filter } => {
            let report =
                CaptureStatsService::default().stats(CaptureStatsInput { path, filter })?;
            match cli.output_mode {
                OutputMode::Text => println!("{}", render_capture_stats_report(&report)),
                OutputMode::Json => println!("{}", render_capture_stats_report_json(&report)),
            }
            Ok(())
        }
        Command::Conversations { path, filter } => {
            let report =
                ConversationsService::default().list(ConversationsInput { path, filter })?;
            match cli.output_mode {
                OutputMode::Text => println!("{}", render_conversation_report(&report)),
                OutputMode::Json => println!("{}", render_conversation_report_json(&report)),
            }
            Ok(())
        }
        Command::Streams { path, filter } => {
            let report = StreamsService::default().list(StreamsInput {
                path,
                filter,
                stream_filter: None,
            })?;
            match cli.output_mode {
                OutputMode::Text => println!("{}", render_stream_report(&report)),
                OutputMode::Json => println!("{}", render_stream_report_json(&report)),
            }
            Ok(())
        }
        Command::StreamsWithAnalysisFilter {
            path,
            filter,
            stream_filter,
        } => {
            let report = StreamsService::default().list(StreamsInput {
                path,
                filter,
                stream_filter,
            })?;
            match cli.output_mode {
                OutputMode::Text => println!("{}", render_stream_report(&report)),
                OutputMode::Json => println!("{}", render_stream_report_json(&report)),
            }
            Ok(())
        }
        Command::Transactions { path, filter } => {
            let report = TransactionsService::default().list(TransactionsInput {
                path,
                filter,
                transaction_filter: None,
            })?;
            match cli.output_mode {
                OutputMode::Text => println!("{}", render_transaction_report(&report)),
                OutputMode::Json => println!("{}", render_transaction_report_json(&report)),
            }
            Ok(())
        }
        Command::TransactionsWithAnalysisFilter {
            path,
            filter,
            transaction_filter,
        } => {
            let report = TransactionsService::default().list(TransactionsInput {
                path,
                filter,
                transaction_filter,
            })?;
            match cli.output_mode {
                OutputMode::Text => println!("{}", render_transaction_report(&report)),
                OutputMode::Json => println!("{}", render_transaction_report_json(&report)),
            }
            Ok(())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliError {
    code: &'static str,
    exit_status: i32,
    message: String,
}

impl CliError {
    fn usage(message: impl Into<String>) -> Self {
        Self {
            code: "ISCLI_USAGE",
            exit_status: 2,
            message: message.into(),
        }
    }

    fn runtime(message: impl Into<String>) -> Self {
        Self {
            code: "ISCLI_RUNTIME",
            exit_status: 1,
            message: message.into(),
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl From<String> for CliError {
    fn from(value: String) -> Self {
        Self::runtime(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Cli {
    output_mode: OutputMode,
    command: Command,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OutputMode {
    Text,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Help,
    App {
        path: Option<PathBuf>,
    },
    EngineInfo,
    Shell {
        path: Option<PathBuf>,
    },
    Save {
        source_path: PathBuf,
        output_path: PathBuf,
        filter: Option<String>,
        stream_filter: Option<String>,
    },
    Inspect {
        path: PathBuf,
    },
    List {
        path: PathBuf,
        limit: Option<usize>,
        filter: Option<String>,
    },
    ShowPacket {
        path: PathBuf,
        packet_index: u64,
    },
    Stats {
        path: PathBuf,
        filter: Option<String>,
    },
    Conversations {
        path: PathBuf,
        filter: Option<String>,
    },
    Streams {
        path: PathBuf,
        filter: Option<String>,
    },
    StreamsWithAnalysisFilter {
        path: PathBuf,
        filter: Option<String>,
        stream_filter: Option<String>,
    },
    Transactions {
        path: PathBuf,
        filter: Option<String>,
    },
    TransactionsWithAnalysisFilter {
        path: PathBuf,
        filter: Option<String>,
        transaction_filter: Option<String>,
    },
}

fn parse_cli<I>(args: I) -> Result<Cli, String>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let mut output_mode = OutputMode::Text;
    let mut command_name = None;
    let mut remaining = Vec::new();

    for arg in args.by_ref() {
        match arg.as_str() {
            "--json" => output_mode = OutputMode::Json,
            "--help" | "-h" => {
                command_name = Some("help".to_string());
                break;
            }
            _ if arg.starts_with('-') => return Err(usage(&format!("unknown flag: {arg}"))),
            _ => {
                command_name = Some(arg);
                break;
            }
        }
    }

    remaining.extend(args);

    let command = match command_name.as_deref() {
        Some("help") => Command::Help,
        Some("app") => {
            if remaining.len() > 1 {
                return Err(usage("too many arguments for app"));
            }
            Command::App {
                path: remaining.first().map(PathBuf::from),
            }
        }
        Some("engine-info") => Command::EngineInfo,
        None => Command::App { path: None },
        Some("shell") => Command::Shell {
            path: remaining.first().map(PathBuf::from),
        },
        Some("save") => {
            let parsed = parse_analysis_filter_args(&remaining, "--stream-filter")?;
            let source_path = parsed
                .positional
                .first()
                .map(PathBuf::from)
                .ok_or_else(|| usage("missing source capture file path"))?;
            let output_path = parsed
                .positional
                .get(1)
                .map(PathBuf::from)
                .ok_or_else(|| usage("missing output capture file path"))?;
            if parsed.positional.len() > 2 {
                return Err(usage("too many arguments for save"));
            }
            Command::Save {
                source_path,
                output_path,
                filter: parsed.filter,
                stream_filter: parsed.analysis_filter,
            }
        }
        Some("inspect") => Command::Inspect {
            path: single_path_arg("inspect", &remaining)?,
        },
        Some("list") => {
            let (filter, positional) = parse_filter_args(&remaining)?;
            let path = positional
                .first()
                .map(PathBuf::from)
                .ok_or_else(|| usage("missing capture file path"))?;
            let limit = match positional.get(1) {
                Some(raw_limit) => {
                    let parsed = raw_limit
                        .parse::<usize>()
                        .map_err(|_| usage("limit must be a positive integer"))?;
                    if parsed == 0 {
                        return Err(usage("limit must be a positive integer"));
                    }
                    Some(parsed)
                }
                None => None,
            };

            if positional.len() > 2 {
                return Err(usage("too many arguments for list"));
            }

            Command::List {
                path,
                limit,
                filter,
            }
        }
        Some("show-packet") => {
            let path = remaining
                .first()
                .map(PathBuf::from)
                .ok_or_else(|| usage("missing capture file path"))?;
            let packet_index = remaining
                .get(1)
                .ok_or_else(|| usage("missing packet index"))?
                .parse::<u64>()
                .map_err(|_| usage("packet index must be a non-negative integer"))?;

            if remaining.len() > 2 {
                return Err(usage("too many arguments for show-packet"));
            }

            Command::ShowPacket { path, packet_index }
        }
        Some("stats") => {
            let (filter, positional) = parse_filter_args(&remaining)?;
            Command::Stats {
                path: single_path_arg("stats", &positional)?,
                filter,
            }
        }
        Some("conversations") => {
            let (filter, positional) = parse_filter_args(&remaining)?;
            Command::Conversations {
                path: single_path_arg("conversations", &positional)?,
                filter,
            }
        }
        Some("streams") => {
            let parsed = parse_analysis_filter_args(&remaining, "--stream-filter")?;
            if parsed.analysis_filter.is_some() {
                Command::StreamsWithAnalysisFilter {
                    path: single_path_arg("streams", &parsed.positional)?,
                    filter: parsed.filter,
                    stream_filter: parsed.analysis_filter,
                }
            } else {
                Command::Streams {
                    path: single_path_arg("streams", &parsed.positional)?,
                    filter: parsed.filter,
                }
            }
        }
        Some("transactions") => {
            let parsed = parse_analysis_filter_args(&remaining, "--transaction-filter")?;
            if parsed.analysis_filter.is_some() {
                Command::TransactionsWithAnalysisFilter {
                    path: single_path_arg("transactions", &parsed.positional)?,
                    filter: parsed.filter,
                    transaction_filter: parsed.analysis_filter,
                }
            } else {
                Command::Transactions {
                    path: single_path_arg("transactions", &parsed.positional)?,
                    filter: parsed.filter,
                }
            }
        }
        Some(command) => return Err(usage(&format!("unknown command: {command}"))),
    };

    Ok(Cli {
        output_mode,
        command,
    })
}

fn single_path_arg(command: &str, args: &[String]) -> Result<PathBuf, String> {
    let path = args
        .first()
        .map(PathBuf::from)
        .ok_or_else(|| usage("missing capture file path"))?;
    if args.len() > 1 {
        return Err(usage(&format!("too many arguments for {command}")));
    }
    Ok(path)
}

fn parse_filter_args(args: &[String]) -> Result<(Option<String>, Vec<String>), String> {
    let parsed = parse_analysis_filter_args(args, "")?;
    Ok((parsed.filter, parsed.positional))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedAnalysisFilters {
    filter: Option<String>,
    analysis_filter: Option<String>,
    positional: Vec<String>,
}

fn parse_analysis_filter_args(
    args: &[String],
    analysis_flag: &str,
) -> Result<ParsedAnalysisFilters, String> {
    let mut filter = None;
    let mut analysis_filter = None;
    let mut positional = Vec::new();
    let mut index = 0usize;

    while index < args.len() {
        if args[index] == "--filter" {
            let value = args
                .get(index + 1)
                .ok_or_else(|| usage("missing value for --filter"))?;
            filter = normalize_filter_expression(value);
            index += 2;
            continue;
        }
        if !analysis_flag.is_empty() && args[index] == analysis_flag {
            let value = args
                .get(index + 1)
                .ok_or_else(|| usage(&format!("missing value for {analysis_flag}")))?;
            analysis_filter = Some(value.clone());
            index += 2;
            continue;
        }
        positional.push(args[index].clone());
        index += 1;
    }

    Ok(ParsedAnalysisFilters {
        filter,
        analysis_filter,
        positional,
    })
}

fn help_text() -> &'static str {
    "\
IceSniff CLI

Usage:
  icesniff-cli help
  icesniff-cli [capture-file]
  icesniff-cli app [capture-file]
  icesniff-cli [--json] engine-info
  icesniff-cli [--json] shell [capture-file]
  icesniff-cli [--json] save <source-capture-file> <output-capture-file> [--filter <expr>] [--stream-filter <expr>]
  icesniff-cli [--json] inspect <capture-file>
  icesniff-cli [--json] list <capture-file> [limit] [--filter <expr>]
  icesniff-cli [--json] show-packet <capture-file> <packet-index>
  icesniff-cli [--json] stats <capture-file> [--filter <expr>]
  icesniff-cli [--json] conversations <capture-file> [--filter <expr>]
  icesniff-cli [--json] streams <capture-file> [--filter <expr>] [--stream-filter <expr>]
  icesniff-cli [--json] transactions <capture-file> [--filter <expr>] [--transaction-filter <expr>]

Commands:
  app          Launch the full-screen IceSniff terminal app.
  engine-info  Print versioned engine capabilities for app clients.
  shell        Start an interactive IceSniff session with a current capture context.
  save         Write packet/stream-filtered capture output into a new PCAP file.
  inspect      Read a capture file and print a shared-engine summary.
  list         Enumerate packets through shared services with derived columns.
  show-packet  Decode one packet through the shared service layer.
  stats        Summarize packet and protocol counts through shared services.
  conversations  Summarize bidirectional flows through shared services.
  streams      Summarize client/server streams and basic transactions.
  transactions  Enumerate parsed HTTP and TLS transactions from shared stream analysis.

Flags:
  --json       Emit machine-readable JSON (schema_version=v1) instead of text output.
  --filter     Apply packet filters like `protocol=dns && !port=443`, `http.status>=200 && http.status<300`, `http.reason~=content`, `dns.answer_count=0`, `tls.handshake_length>=60`, `tls.server_name~=example`, or `http.method=get && host=EXAMPLE.COM`.
  --stream-filter  For `save` and `streams`, apply row-level stream filters like `stream.state=reset`, `stream.has_alerts=true`, or `stream.is_pipelined=true && stream.has_timeline=true`.
  --transaction-filter  Apply row-level transaction filters like `tx.state=matched`, `tx.request.method=get`, or `tx.tls.alpn=h2 && tx.has_alerts=true`.

Error codes:
  [ISCLI_USAGE]   Exit status 2 for command/argument usage errors.
  [ISCLI_RUNTIME] Exit status 1 for runtime/service failures.

Notes:
  Running `icesniff-cli` with no command launches the interactive terminal app.
"
}

fn engine_info_report() -> EngineInfoReport {
    EngineInfoReport {
        schema_version: "v1".to_string(),
        engine_version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: EngineCapabilitiesReport {
            inspect: true,
            packet_list: true,
            packet_detail: true,
            stats: true,
            conversations: true,
            streams: true,
            transactions: true,
            save: true,
            live_capture: true,
        },
        capture: EngineCaptureSupport {
            bundled_backend: true,
            built_in_tcpdump: true,
            interface_discovery: true,
            requires_admin_for_live_capture: true,
        },
        filters: EngineFilterSupport {
            packet_filters: true,
            stream_filters: true,
            transaction_filters: true,
            shorthand_protocol_terms: true,
            shorthand_port_terms: true,
            case_insensitive_protocols: true,
            alternate_and_operators: vec!["&&".to_string(), "&".to_string(), "and".to_string()],
        },
        export: EngineExportSupport {
            save_capture: true,
            filtered_save: true,
            whole_capture_save: true,
        },
        dissectors: EngineDissectorSupport {
            protocols: vec![
                "arp".to_string(),
                "dns".to_string(),
                "ethernet".to_string(),
                "http".to_string(),
                "ipv4".to_string(),
                "tcp".to_string(),
                "tls".to_string(),
                "udp".to_string(),
            ],
        },
    }
}

fn shell_help_text() -> &'static str {
    "\
IceSniff shell commands

  help
  open <capture-file>
  save <output-capture-file> [--filter <expr>] [--stream-filter <expr>]
  capture interfaces
  capture start [interface]
  capture stop
  capture status
  close
  status
  mode <text|json>
  inspect
  list [limit] [--filter <expr>]
  show-packet <packet-index>
  stats [--filter <expr>]
  conversations [--filter <expr>]
  streams [--filter <expr>] [--stream-filter <expr>]
  transactions [--filter <expr>] [--transaction-filter <expr>]
  quit

Notes:
  - open a capture once, then run analysis commands against the current session
  - live capture writes to a temporary pcap and opens it when stopped
  - quote paths or filter expressions when they contain spaces
  - mode switches between text and json output inside the shell
"
}

fn usage(message: &str) -> String {
    format!("{message}\n\n{}", help_text())
}

#[derive(Debug)]
struct ShellState {
    current_path: Option<PathBuf>,
    output_mode: OutputMode,
    active_capture: Option<ActiveCapture>,
}

#[derive(Debug)]
struct ActiveCapture {
    session: LiveCaptureSession,
    stop_flag: Arc<AtomicBool>,
    header_printed: Arc<AtomicBool>,
    last_seen_index: Arc<std::sync::atomic::AtomicU64>,
    monitor_handle: Option<JoinHandle<()>>,
}

fn run_shell(path: Option<PathBuf>, output_mode: OutputMode) -> Result<(), String> {
    let mut state = ShellState {
        current_path: path,
        output_mode,
        active_capture: None,
    };

    println!("IceSniff shell");
    println!("type `help` for commands");
    if let Some(path) = &state.current_path {
        println!("current capture: {}", path.display());
    } else {
        println!("current capture: none");
    }

    let stdin = io::stdin();

    loop {
        print!("{}", shell_prompt(&state));
        io::stdout()
            .flush()
            .map_err(|error| format!("failed to flush prompt: {error}"))?;

        let mut line = String::new();
        let bytes_read = stdin
            .read_line(&mut line)
            .map_err(|error| format!("failed to read shell input: {error}"))?;
        if bytes_read == 0 {
            println!();
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let args = parse_shell_words(trimmed)?;
        if !execute_shell_command(&mut state, &args)? {
            stop_active_capture_if_needed(&mut state)?;
            break;
        }
    }

    Ok(())
}

fn shell_prompt(state: &ShellState) -> String {
    match &state.current_path {
        Some(path) => format!("icesniff:{}> ", path.display()),
        None => "icesniff> ".to_string(),
    }
}

fn execute_shell_command(state: &mut ShellState, args: &[String]) -> Result<bool, String> {
    let Some(command) = args.first().map(String::as_str) else {
        return Ok(true);
    };

    match command {
        "help" => {
            println!("{}", shell_help_text());
            Ok(true)
        }
        "quit" | "exit" => Ok(false),
        "capture" => {
            execute_capture_command(state, &args[1..])?;
            Ok(true)
        }
        "open" => {
            let path = args
                .get(1)
                .map(PathBuf::from)
                .ok_or_else(|| "shell open requires a capture file path".to_string())?;
            if args.len() > 2 {
                return Err("shell open accepts only one capture file path".to_string());
            }
            let report = InspectCaptureService::default()
                .inspect(InspectCaptureInput { path: path.clone() })?;
            state.current_path = Some(path);
            render_capture_summary(&state.output_mode, &report);
            Ok(true)
        }
        "save" => {
            let source_path = require_shell_capture_path(state)?;
            let parsed = parse_analysis_filter_args(&args[1..], "--stream-filter")?;
            let output_path = parsed
                .positional
                .first()
                .map(PathBuf::from)
                .ok_or_else(|| "shell save requires an output capture file path".to_string())?;
            if parsed.positional.len() > 1 {
                return Err("shell save accepts only one output capture file path".to_string());
            }
            let report = SaveCaptureService::default().save(SaveCaptureInput {
                source_path,
                output_path,
                filter: parsed.filter,
                stream_filter: parsed.analysis_filter,
            })?;
            render_save_capture(&state.output_mode, &report);
            Ok(true)
        }
        "close" => {
            state.current_path = None;
            println!("capture closed");
            Ok(true)
        }
        "status" => {
            let live_capture = match state.active_capture.as_mut() {
                Some(capture) => {
                    if capture.session.is_running().map_err(render_capture_error)? {
                        format!("running on {}", capture.session.interface())
                    } else {
                        format!("exited on {}", capture.session.interface())
                    }
                }
                None => "idle".to_string(),
            };
            println!(
                "mode: {}\ncurrent capture: {}\nlive capture: {}",
                output_mode_name(&state.output_mode),
                state
                    .current_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "none".to_string()),
                live_capture
            );
            Ok(true)
        }
        "mode" => {
            let value = args
                .get(1)
                .ok_or_else(|| "shell mode requires `text` or `json`".to_string())?;
            if args.len() > 2 {
                return Err("shell mode accepts only one argument".to_string());
            }
            state.output_mode = match value.as_str() {
                "text" => OutputMode::Text,
                "json" => OutputMode::Json,
                _ => return Err("shell mode must be `text` or `json`".to_string()),
            };
            println!("mode: {}", output_mode_name(&state.output_mode));
            Ok(true)
        }
        "inspect" => {
            let path = require_shell_capture_path(state)?;
            let report = InspectCaptureService::default().inspect(InspectCaptureInput { path })?;
            render_capture_summary(&state.output_mode, &report);
            Ok(true)
        }
        "list" => {
            let path = require_shell_capture_path(state)?;
            let parsed = parse_analysis_filter_args(&args[1..], "")?;
            let limit = match parsed.positional.first() {
                Some(raw_limit) => Some(
                    raw_limit
                        .parse::<usize>()
                        .map_err(|_| "shell list limit must be a positive integer".to_string())?,
                ),
                None => None,
            };
            if limit == Some(0) {
                return Err("shell list limit must be a positive integer".to_string());
            }
            if parsed.positional.len() > 1 {
                return Err("shell list accepts at most one limit argument".to_string());
            }
            let report = ListPacketsService::default().list(ListPacketsInput {
                path,
                limit,
                filter: parsed.filter,
            })?;
            render_packet_list(&state.output_mode, &report);
            Ok(true)
        }
        "show-packet" => {
            let path = require_shell_capture_path(state)?;
            let packet_index = args
                .get(1)
                .ok_or_else(|| "shell show-packet requires a packet index".to_string())?
                .parse::<u64>()
                .map_err(|_| {
                    "shell show-packet index must be a non-negative integer".to_string()
                })?;
            if args.len() > 2 {
                return Err("shell show-packet accepts only one packet index".to_string());
            }
            let report = InspectPacketService::default()
                .inspect(InspectPacketInput { path, packet_index })?;
            render_packet_detail(&state.output_mode, &report);
            Ok(true)
        }
        "stats" => {
            let path = require_shell_capture_path(state)?;
            let (filter, positional) = parse_filter_args(&args[1..])?;
            if !positional.is_empty() {
                return Err("shell stats does not accept positional arguments".to_string());
            }
            let report =
                CaptureStatsService::default().stats(CaptureStatsInput { path, filter })?;
            render_capture_stats(&state.output_mode, &report);
            Ok(true)
        }
        "conversations" => {
            let path = require_shell_capture_path(state)?;
            let (filter, positional) = parse_filter_args(&args[1..])?;
            if !positional.is_empty() {
                return Err("shell conversations does not accept positional arguments".to_string());
            }
            let report =
                ConversationsService::default().list(ConversationsInput { path, filter })?;
            render_conversations(&state.output_mode, &report);
            Ok(true)
        }
        "streams" => {
            let path = require_shell_capture_path(state)?;
            let parsed = parse_analysis_filter_args(&args[1..], "--stream-filter")?;
            if !parsed.positional.is_empty() {
                return Err("shell streams does not accept positional arguments".to_string());
            }
            let report = StreamsService::default().list(StreamsInput {
                path,
                filter: parsed.filter,
                stream_filter: parsed.analysis_filter,
            })?;
            render_streams(&state.output_mode, &report);
            Ok(true)
        }
        "transactions" => {
            let path = require_shell_capture_path(state)?;
            let parsed = parse_analysis_filter_args(&args[1..], "--transaction-filter")?;
            if !parsed.positional.is_empty() {
                return Err("shell transactions does not accept positional arguments".to_string());
            }
            let report = TransactionsService::default().list(TransactionsInput {
                path,
                filter: parsed.filter,
                transaction_filter: parsed.analysis_filter,
            })?;
            render_transactions(&state.output_mode, &report);
            Ok(true)
        }
        _ => Err(format!("unknown shell command: {command}")),
    }
}

fn execute_capture_command(state: &mut ShellState, args: &[String]) -> Result<(), String> {
    let Some(command) = args.first().map(String::as_str) else {
        return Err(
            "capture requires a subcommand: interfaces, start, stop, or status".to_string(),
        );
    };
    let coordinator = LiveCaptureCoordinator::default();

    match command {
        "interfaces" => {
            if args.len() > 1 {
                return Err("capture interfaces does not accept extra arguments".to_string());
            }
            let interfaces = coordinator
                .list_interfaces()
                .map_err(render_capture_error)?;
            if interfaces.is_empty() {
                println!("no capture interfaces detected");
            } else {
                println!("capture interfaces:");
                for interface in interfaces {
                    println!("  - {}", interface.name);
                }
            }
            Ok(())
        }
        "start" => {
            if state.active_capture.is_some() {
                return Err("a live capture is already running; stop it first".to_string());
            }
            if args.len() > 2 {
                return Err("capture start accepts at most one interface argument".to_string());
            }
            let interface = args.get(1).cloned();
            let session = coordinator
                .start(StartLiveCaptureInput {
                    interface: interface.clone(),
                })
                .map_err(render_capture_error)?;
            let interface = session.interface().to_string();
            let path = session.path().to_path_buf();
            let stop_flag = Arc::new(AtomicBool::new(false));
            let header_printed = Arc::new(AtomicBool::new(false));
            let last_seen_index = Arc::new(std::sync::atomic::AtomicU64::new(u64::MAX));
            let monitor_handle = Some(start_live_packet_monitor(
                path.clone(),
                stop_flag.clone(),
                header_printed.clone(),
                last_seen_index.clone(),
            ));

            state.active_capture = Some(ActiveCapture {
                session,
                stop_flag,
                header_printed,
                last_seen_index,
                monitor_handle,
            });
            println!(
                "live capture started on {} -> {}",
                interface,
                path.display()
            );
            Ok(())
        }
        "stop" => {
            if args.len() > 1 {
                return Err("capture stop does not accept extra arguments".to_string());
            }
            let Some(capture) = state.active_capture.take() else {
                return Err("no live capture is running".to_string());
            };
            let path = stop_capture(capture)?;
            let report = InspectCaptureService::default()
                .inspect(InspectCaptureInput { path: path.clone() })?;
            state.current_path = Some(path);
            render_capture_summary(&state.output_mode, &report);
            Ok(())
        }
        "status" => {
            if args.len() > 1 {
                return Err("capture status does not accept extra arguments".to_string());
            }
            let runtime = coordinator.runtime_info();
            match state.active_capture.as_mut() {
                Some(capture) => {
                    if capture.session.is_running().map_err(render_capture_error)? {
                        println!(
                            "live capture: running\ninterface: {}\npath: {}\nbackend: {}\ntool: {}",
                            capture.session.interface(),
                            capture.session.path().display(),
                            runtime.backend.as_str(),
                            runtime.tool_path
                        );
                    } else {
                        println!(
                            "live capture: exited\ninterface: {}\npath: {}\nbackend: {}\ntool: {}",
                            capture.session.interface(),
                            capture.session.path().display(),
                            runtime.backend.as_str(),
                            runtime.tool_path
                        );
                    }
                }
                None => println!(
                    "live capture: idle\nbackend: {}\ntool: {}",
                    runtime.backend.as_str(),
                    runtime.tool_path
                ),
            }
            Ok(())
        }
        _ => Err(format!("unknown capture command: {command}")),
    }
}

fn start_live_packet_monitor(
    path: PathBuf,
    stop_flag: Arc<AtomicBool>,
    header_printed: Arc<AtomicBool>,
    last_seen_index: Arc<std::sync::atomic::AtomicU64>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        print_live_packet_header_once(&header_printed);

        while !stop_flag.load(Ordering::Relaxed) {
            let _ = flush_live_packets(&path, &header_printed, &last_seen_index);

            thread::sleep(Duration::from_millis(400));
        }
    })
}

fn flush_live_packets(
    path: &PathBuf,
    header_printed: &Arc<AtomicBool>,
    last_seen_index: &Arc<std::sync::atomic::AtomicU64>,
) -> Result<(), String> {
    let report = ListPacketsService::default().list(ListPacketsInput {
        path: path.clone(),
        limit: None,
        filter: None,
    })?;
    print_live_packet_header_once(header_printed);

    for packet in &report.packets {
        let last_seen = last_seen_index.load(Ordering::Relaxed);
        if last_seen != u64::MAX && packet.summary.index <= last_seen {
            continue;
        }
        println!(
            "{:<6} | {:<16} | {:<20} | {:<20} | {:<10} | {}",
            packet.summary.index,
            live_packet_time(
                packet.summary.timestamp_seconds,
                packet.summary.timestamp_fraction
            ),
            truncate_column(&packet.source, 20),
            truncate_column(&packet.destination, 20),
            truncate_column(&packet.protocol, 10),
            packet.info
        );
        last_seen_index.store(packet.summary.index, Ordering::Relaxed);
    }

    Ok(())
}

fn print_live_packet_header_once(header_printed: &Arc<AtomicBool>) {
    if header_printed.swap(true, Ordering::Relaxed) {
        return;
    }

    println!();
    println!(
        "{:<6} | {:<16} | {:<20} | {:<20} | {:<10} | {}",
        "Id", "Time", "Source", "Destination", "Protocol", "Info"
    );
    println!("{}", "-".repeat(96));
}

fn live_packet_time(seconds: u32, fraction: u32) -> String {
    format!("{seconds}.{fraction:06}")
}

fn truncate_column(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        return value.to_string();
    }

    let truncated = value
        .chars()
        .take(width.saturating_sub(1))
        .collect::<String>();
    format!("{truncated}~")
}

fn stop_active_capture_if_needed(state: &mut ShellState) -> Result<(), String> {
    if let Some(capture) = state.active_capture.take() {
        let path = stop_capture(capture)?;
        state.current_path = Some(path);
    }
    Ok(())
}

fn stop_capture(mut capture: ActiveCapture) -> Result<PathBuf, String> {
    capture.stop_flag.store(true, Ordering::Relaxed);
    let path = capture.session.path().to_path_buf();
    let _ = flush_live_packets(&path, &capture.header_printed, &capture.last_seen_index);
    if let Some(handle) = capture.monitor_handle.take() {
        let _ = handle.join();
    }
    capture.session.stop().map_err(render_capture_error)
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
        CaptureError::NoInterfacesAvailable => {
            "no capture interfaces are available".to_string()
        }
        other => other.to_string(),
    }
}

fn require_shell_capture_path(state: &ShellState) -> Result<PathBuf, String> {
    state
        .current_path
        .clone()
        .ok_or_else(|| "no capture is open; use `open <capture-file>` first".to_string())
}

fn parse_shell_words(input: &str) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote = None::<char>;

    for ch in input.chars() {
        match quote {
            Some(active_quote) if ch == active_quote => {
                quote = None;
            }
            Some(_) => current.push(ch),
            None if ch == '"' || ch == '\'' => {
                quote = Some(ch);
            }
            None if ch.is_whitespace() => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            None => current.push(ch),
        }
    }

    if quote.is_some() {
        return Err("unterminated quote in shell input".to_string());
    }
    if !current.is_empty() {
        words.push(current);
    }

    Ok(words)
}

fn output_mode_name(output_mode: &OutputMode) -> &'static str {
    match output_mode {
        OutputMode::Text => "text",
        OutputMode::Json => "json",
    }
}

fn render_capture_summary(output_mode: &OutputMode, report: &session_model::CaptureReport) {
    match output_mode {
        OutputMode::Text => println!("{}", render_capture_report(report)),
        OutputMode::Json => println!("{}", render_capture_report_json(report)),
    }
}

fn render_save_capture(output_mode: &OutputMode, report: &session_model::SaveCaptureReport) {
    match output_mode {
        OutputMode::Text => println!("{}", render_save_capture_report(report)),
        OutputMode::Json => println!("{}", render_save_capture_report_json(report)),
    }
}

fn render_packet_list(output_mode: &OutputMode, report: &session_model::PacketListReport) {
    match output_mode {
        OutputMode::Text => println!("{}", render_packet_list_report(report)),
        OutputMode::Json => println!("{}", render_packet_list_report_json(report)),
    }
}

fn render_packet_detail(output_mode: &OutputMode, report: &session_model::PacketDetailReport) {
    match output_mode {
        OutputMode::Text => println!("{}", render_packet_detail_report(report)),
        OutputMode::Json => println!("{}", render_packet_detail_report_json(report)),
    }
}

fn render_capture_stats(output_mode: &OutputMode, report: &session_model::CaptureStatsReport) {
    match output_mode {
        OutputMode::Text => println!("{}", render_capture_stats_report(report)),
        OutputMode::Json => println!("{}", render_capture_stats_report_json(report)),
    }
}

fn render_conversations(output_mode: &OutputMode, report: &session_model::ConversationReport) {
    match output_mode {
        OutputMode::Text => println!("{}", render_conversation_report(report)),
        OutputMode::Json => println!("{}", render_conversation_report_json(report)),
    }
}

fn render_streams(output_mode: &OutputMode, report: &session_model::StreamReport) {
    match output_mode {
        OutputMode::Text => println!("{}", render_stream_report(report)),
        OutputMode::Json => println!("{}", render_stream_report_json(report)),
    }
}

fn render_transactions(output_mode: &OutputMode, report: &session_model::TransactionReport) {
    match output_mode {
        OutputMode::Text => println!("{}", render_transaction_report(report)),
        OutputMode::Json => println!("{}", render_transaction_report_json(report)),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_cli, parse_shell_words, Cli, Command, OutputMode};
    use std::path::PathBuf;

    #[test]
    fn defaults_to_app_when_no_command_is_given() {
        let cli = parse_cli(Vec::<String>::new()).expect("expected app command");

        assert_eq!(
            cli,
            Cli {
                output_mode: OutputMode::Text,
                command: Command::App { path: None },
            }
        );
    }

    #[test]
    fn parses_app_command_with_initial_capture() {
        let cli = parse_cli(["app".to_string(), "sample.pcap".to_string()])
            .expect("expected app command");

        assert_eq!(
            cli,
            Cli {
                output_mode: OutputMode::Text,
                command: Command::App {
                    path: Some(PathBuf::from("sample.pcap")),
                },
            }
        );
    }

    #[test]
    fn parses_shell_command_with_initial_capture() {
        let cli = parse_cli(["shell".to_string(), "sample.pcap".to_string()])
            .expect("expected shell command");

        assert_eq!(
            cli,
            Cli {
                output_mode: OutputMode::Text,
                command: Command::Shell {
                    path: Some(PathBuf::from("sample.pcap")),
                },
            }
        );
    }

    #[test]
    fn parses_save_command_with_filter() {
        let cli = parse_cli([
            "save".to_string(),
            "input.pcap".to_string(),
            "output.pcap".to_string(),
            "--filter".to_string(),
            "protocol=http".to_string(),
        ])
        .expect("expected save command to parse");

        assert_eq!(
            cli,
            Cli {
                output_mode: OutputMode::Text,
                command: Command::Save {
                    source_path: PathBuf::from("input.pcap"),
                    output_path: PathBuf::from("output.pcap"),
                    filter: Some("protocol=http".to_string()),
                    stream_filter: None,
                },
            }
        );
    }

    #[test]
    fn parses_save_command_with_stream_filter() {
        let cli = parse_cli([
            "save".to_string(),
            "input.pcap".to_string(),
            "output.pcap".to_string(),
            "--filter".to_string(),
            "protocol=tcp".to_string(),
            "--stream-filter".to_string(),
            "stream.service=http".to_string(),
        ])
        .expect("expected save command to parse");

        assert_eq!(
            cli,
            Cli {
                output_mode: OutputMode::Text,
                command: Command::Save {
                    source_path: PathBuf::from("input.pcap"),
                    output_path: PathBuf::from("output.pcap"),
                    filter: Some("protocol=tcp".to_string()),
                    stream_filter: Some("stream.service=http".to_string()),
                },
            }
        );
    }

    #[test]
    fn parses_json_stats_command() {
        let cli = parse_cli(vec![
            "--json".to_string(),
            "stats".to_string(),
            "sample.pcap".to_string(),
        ])
        .unwrap();

        assert_eq!(cli.output_mode, OutputMode::Json);
        assert_eq!(
            cli.command,
            Command::Stats {
                path: PathBuf::from("sample.pcap"),
                filter: None,
            }
        );
    }

    #[test]
    fn parses_filter_for_list_command() {
        let cli = parse_cli(vec![
            "list".to_string(),
            "sample.pcap".to_string(),
            "--filter".to_string(),
            "protocol=dns".to_string(),
        ])
        .unwrap();

        assert_eq!(
            cli.command,
            Command::List {
                path: PathBuf::from("sample.pcap"),
                limit: None,
                filter: Some("protocol=dns".to_string()),
            }
        );
    }

    #[test]
    fn parses_filter_for_conversations_command() {
        let cli = parse_cli([
            "conversations".to_string(),
            "capture.pcap".to_string(),
            "--filter".to_string(),
            "host=example.com".to_string(),
        ])
        .expect("expected command to parse");

        assert_eq!(
            cli,
            Cli {
                output_mode: OutputMode::Text,
                command: Command::Conversations {
                    path: PathBuf::from("capture.pcap"),
                    filter: Some("host=example.com".to_string()),
                },
            }
        );
    }

    #[test]
    fn parses_filter_for_streams_command() {
        let cli = parse_cli([
            "streams".to_string(),
            "capture.pcap".to_string(),
            "--filter".to_string(),
            "protocol=http".to_string(),
        ])
        .expect("expected command to parse");

        assert_eq!(
            cli,
            Cli {
                output_mode: OutputMode::Text,
                command: Command::Streams {
                    path: PathBuf::from("capture.pcap"),
                    filter: Some("protocol=http".to_string()),
                },
            }
        );
    }

    #[test]
    fn parses_stream_analysis_filter() {
        let cli = parse_cli([
            "streams".to_string(),
            "capture.pcap".to_string(),
            "--stream-filter".to_string(),
            "stream.state=reset".to_string(),
        ])
        .expect("expected command to parse");

        assert_eq!(
            cli,
            Cli {
                output_mode: OutputMode::Text,
                command: Command::StreamsWithAnalysisFilter {
                    path: PathBuf::from("capture.pcap"),
                    filter: None,
                    stream_filter: Some("stream.state=reset".to_string()),
                },
            }
        );
    }

    #[test]
    fn parses_filter_for_transactions_command() {
        let cli = parse_cli([
            "transactions".to_string(),
            "capture.pcap".to_string(),
            "--filter".to_string(),
            "protocol=tls".to_string(),
        ])
        .expect("expected command to parse");

        assert_eq!(
            cli,
            Cli {
                output_mode: OutputMode::Text,
                command: Command::Transactions {
                    path: PathBuf::from("capture.pcap"),
                    filter: Some("protocol=tls".to_string()),
                },
            }
        );
    }

    #[test]
    fn parses_transaction_analysis_filter() {
        let cli = parse_cli([
            "transactions".to_string(),
            "capture.pcap".to_string(),
            "--transaction-filter".to_string(),
            "tx.state=matched".to_string(),
        ])
        .expect("expected command to parse");

        assert_eq!(
            cli,
            Cli {
                output_mode: OutputMode::Text,
                command: Command::TransactionsWithAnalysisFilter {
                    path: PathBuf::from("capture.pcap"),
                    filter: None,
                    transaction_filter: Some("tx.state=matched".to_string()),
                },
            }
        );
    }

    #[test]
    fn parses_shell_words_with_quotes() {
        let words = parse_shell_words("open \"capture file.pcap\"").expect("expected shell words");
        assert_eq!(words, vec!["open", "capture file.pcap"]);
    }
}
