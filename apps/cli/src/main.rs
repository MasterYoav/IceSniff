use std::env;
use std::path::PathBuf;

use app_services::{
    CaptureStatsInput, CaptureStatsService, ConversationsInput, ConversationsService,
    InspectCaptureInput, InspectCaptureService, InspectPacketInput, InspectPacketService,
    ListPacketsInput, ListPacketsService,
};
use output_formatters::{
    render_capture_report, render_capture_report_json, render_capture_stats_report,
    render_capture_stats_report_json, render_conversation_report, render_conversation_report_json,
    render_packet_detail_report, render_packet_detail_report_json, render_packet_list_report,
    render_packet_list_report_json,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let cli = parse_cli(env::args().skip(1))?;

    match cli.command {
        Command::Help => {
            println!("{}", help_text());
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
        Some("help") | None => Command::Help,
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
    let mut filter = None;
    let mut positional = Vec::new();
    let mut index = 0usize;

    while index < args.len() {
        if args[index] == "--filter" {
            let value = args
                .get(index + 1)
                .ok_or_else(|| usage("missing value for --filter"))?;
            filter = Some(value.clone());
            index += 2;
            continue;
        }
        positional.push(args[index].clone());
        index += 1;
    }

    Ok((filter, positional))
}

fn help_text() -> &'static str {
    "\
IceSniff CLI

Usage:
  icesniff-cli help
  icesniff-cli [--json] inspect <capture-file>
  icesniff-cli [--json] list <capture-file> [limit] [--filter <expr>]
  icesniff-cli [--json] show-packet <capture-file> <packet-index>
  icesniff-cli [--json] stats <capture-file> [--filter <expr>]
  icesniff-cli [--json] conversations <capture-file> [--filter <expr>]

Commands:
  inspect      Read a capture file and print a shared-engine summary.
  list         Enumerate packets through shared services with derived columns.
  show-packet  Decode one packet through the shared service layer.
  stats        Summarize packet and protocol counts through shared services.
  conversations  Summarize bidirectional flows through shared services.

Flags:
  --json       Emit machine-readable JSON instead of text output.
  --filter     Apply filters like `protocol=dns`, `protocol=http`, `port=443`, or `host=example.com`.
"
}

fn usage(message: &str) -> String {
    format!("{message}\n\n{}", help_text())
}

#[cfg(test)]
mod tests {
    use super::{parse_cli, Cli, Command, OutputMode};
    use std::path::PathBuf;

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
}
