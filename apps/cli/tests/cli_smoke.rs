use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
#[cfg(unix)]
use std::{os::unix::fs::PermissionsExt, path::Path};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

fn write_sample_pcap() -> String {
    let unique = unique_temp_suffix();
    let path = std::env::temp_dir().join(format!("icesniff-cli-{unique}.pcap"));
    fs::write(&path, sample_pcap_bytes()).expect("failed to write sample pcap");
    path.display().to_string()
}

fn write_sample_pcapng() -> String {
    let unique = unique_temp_suffix();
    let path = std::env::temp_dir().join(format!("icesniff-cli-{unique}.pcapng"));
    fs::write(&path, sample_pcapng_bytes()).expect("failed to write sample pcapng");
    path.display().to_string()
}

fn write_temp_capture(extension: &str, bytes: &[u8]) -> String {
    let unique = unique_temp_suffix();
    let path = std::env::temp_dir().join(format!("icesniff-cli-{unique}.{extension}"));
    fs::write(&path, bytes).expect("failed to write sample capture");
    path.display().to_string()
}

fn unique_temp_suffix() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_nanos();
    let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{}-{counter}", std::process::id(), timestamp)
}

#[cfg(unix)]
fn write_fake_capture_tool() -> String {
    let unique = unique_temp_suffix();
    let path = std::env::temp_dir().join(format!("icesniff-fake-capture-{unique}.sh"));
    let script = r#"#!/bin/sh
if [ "$1" = "-D" ]; then
  echo "1.fake0 [Up, Running]"
  exit 0
fi

OUTPUT=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "-w" ]; then
    shift
    OUTPUT="$1"
  fi
  shift
done

cp "$ICESNIFF_FAKE_CAPTURE_SOURCE" "$OUTPUT.tmp"
mv "$OUTPUT.tmp" "$OUTPUT"
while true; do
  sleep 1
done
"#;
    fs::write(&path, script).expect("failed to write fake capture tool");
    let mut permissions = fs::metadata(&path)
        .expect("failed to stat fake capture tool")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("failed to chmod fake capture tool");
    path_to_string(&path)
}

#[cfg(unix)]
fn write_fake_capture_tool_that_exits() -> String {
    let unique = unique_temp_suffix();
    let path = std::env::temp_dir().join(format!("icesniff-fake-capture-exit-{unique}.sh"));
    let script = r#"#!/bin/sh
if [ "$1" = "-D" ]; then
  echo "1.fake0 [Up, Running]"
  exit 0
fi

OUTPUT=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "-w" ]; then
    shift
    OUTPUT="$1"
  fi
  shift
done

cp "$ICESNIFF_FAKE_CAPTURE_SOURCE" "$OUTPUT.tmp"
mv "$OUTPUT.tmp" "$OUTPUT"
sleep 1
exit 0
"#;
    fs::write(&path, script).expect("failed to write fake exiting capture tool");
    let mut permissions = fs::metadata(&path)
        .expect("failed to stat fake exiting capture tool")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("failed to chmod fake exiting capture tool");
    path_to_string(&path)
}

#[cfg(unix)]
fn path_to_string(path: &Path) -> String {
    path.display().to_string()
}

fn extract_text_u64(output: &str, key: &str) -> u64 {
    let prefix = format!("{key}: ");
    output
        .lines()
        .find_map(|line| line.trim().strip_prefix(&prefix))
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or_else(|| panic!("failed to parse text field `{key}` from output:\n{output}"))
}

fn extract_json_u64(output: &str, key: &str) -> u64 {
    let pattern = format!("\"{key}\":");
    let start = output
        .find(&pattern)
        .map(|index| index + pattern.len())
        .unwrap_or_else(|| panic!("failed to find json field `{key}` in output:\n{output}"));
    let digits = output[start..]
        .chars()
        .skip_while(|ch| ch.is_whitespace())
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    digits
        .parse::<u64>()
        .unwrap_or_else(|_| panic!("failed to parse json field `{key}` from output:\n{output}"))
}

#[test]
fn inspect_command_outputs_json() {
    let path = write_sample_pcap();
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "inspect", &path])
        .output()
        .expect("failed to run inspect command");
    fs::remove_file(&path).expect("failed to remove sample pcap");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"format\":\"pcap\""));
    assert!(stdout.contains("\"packet_count_hint\":2"));
}

#[test]
fn json_outputs_include_schema_version_v1() {
    let path = write_sample_pcap();
    let save_output_path = write_temp_capture("pcap", b"");

    let inspect_output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "inspect", &path])
        .output()
        .expect("failed to run inspect command");
    let save_output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "save", &path, &save_output_path])
        .output()
        .expect("failed to run save command");
    let list_output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "list", &path])
        .output()
        .expect("failed to run list command");
    let show_packet_output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "show-packet", &path, "0"])
        .output()
        .expect("failed to run show-packet command");
    let stats_output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "stats", &path])
        .output()
        .expect("failed to run stats command");
    let conversations_output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "conversations", &path])
        .output()
        .expect("failed to run conversations command");
    let streams_output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "streams", &path])
        .output()
        .expect("failed to run streams command");
    let transactions_output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "transactions", &path])
        .output()
        .expect("failed to run transactions command");

    fs::remove_file(&path).expect("failed to remove sample pcap");
    fs::remove_file(&save_output_path).expect("failed to remove save output capture");

    let outputs = [
        inspect_output,
        save_output,
        list_output,
        show_packet_output,
        stats_output,
        conversations_output,
        streams_output,
        transactions_output,
    ];

    for output in outputs {
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
        assert!(stdout.contains("\"schema_version\":\"v1\""));
    }
}

#[test]
fn save_command_writes_filtered_capture_file() {
    let source = write_temp_capture(
        "pcap",
        &wrap_pcap_packets(&[sample_http_frame(), sample_dns_frame()]),
    );
    let output = write_temp_capture("pcap", b"");

    let save_output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["save", &source, &output, "--filter", "protocol=http"])
        .output()
        .expect("failed to run save command");

    assert!(save_output.status.success());
    let save_stdout = String::from_utf8(save_output.stdout).expect("stdout was not utf-8");
    assert!(save_stdout.contains("Capture saved"));
    assert!(save_stdout.contains("packets_written: 1"));

    let inspect_output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "inspect", &output])
        .output()
        .expect("failed to inspect saved capture");

    fs::remove_file(&source).expect("failed to remove source capture");
    fs::remove_file(&output).expect("failed to remove output capture");

    assert!(inspect_output.status.success());
    let inspect_stdout = String::from_utf8(inspect_output.stdout).expect("stdout was not utf-8");
    assert!(inspect_stdout.contains("\"format\":\"pcap\""));
    assert!(inspect_stdout.contains("\"packet_count_hint\":1"));
}

#[test]
fn save_command_writes_stream_filtered_capture_file() {
    let source = write_temp_capture(
        "pcap",
        &wrap_pcap_packets(&[sample_http_frame(), sample_dns_frame()]),
    );
    let output = write_temp_capture("pcap", b"");

    let save_output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args([
            "save",
            &source,
            &output,
            "--stream-filter",
            "stream.service=http",
        ])
        .output()
        .expect("failed to run save command with stream filter");

    assert!(save_output.status.success());
    let save_stdout = String::from_utf8(save_output.stdout).expect("stdout was not utf-8");
    assert!(save_stdout.contains("Capture saved"));
    assert!(save_stdout.contains("packets_written: 1"));
    assert!(save_stdout.contains("stream_filter: stream.service=http"));

    let inspect_output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "inspect", &output])
        .output()
        .expect("failed to inspect stream-filtered capture");

    fs::remove_file(&source).expect("failed to remove source capture");
    fs::remove_file(&output).expect("failed to remove output capture");

    assert!(inspect_output.status.success());
    let inspect_stdout = String::from_utf8(inspect_output.stdout).expect("stdout was not utf-8");
    assert!(inspect_stdout.contains("\"format\":\"pcap\""));
    assert!(inspect_stdout.contains("\"packet_count_hint\":1"));
}

#[test]
fn save_command_text_and_json_packets_written_match() {
    let source = write_temp_capture(
        "pcap",
        &wrap_pcap_packets(&[sample_http_frame(), sample_dns_frame()]),
    );
    let output_text = write_temp_capture("pcap", b"");
    let output_json = write_temp_capture("pcap", b"");

    let text_output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["save", &source, &output_text, "--filter", "protocol=http"])
        .output()
        .expect("failed to run text save command");
    let json_output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args([
            "--json",
            "save",
            &source,
            &output_json,
            "--filter",
            "protocol=http",
        ])
        .output()
        .expect("failed to run json save command");

    fs::remove_file(&source).expect("failed to remove source capture");
    fs::remove_file(&output_text).expect("failed to remove text output capture");
    fs::remove_file(&output_json).expect("failed to remove json output capture");

    assert!(text_output.status.success());
    assert!(json_output.status.success());
    let text_stdout = String::from_utf8(text_output.stdout).expect("text stdout was not utf-8");
    let json_stdout = String::from_utf8(json_output.stdout).expect("json stdout was not utf-8");
    assert_eq!(
        extract_text_u64(&text_stdout, "packets_written"),
        extract_json_u64(&json_stdout, "packets_written")
    );
}

#[test]
fn shell_command_runs_interactive_session() {
    let path = write_temp_capture("pcap", &wrap_pcap_packet(&sample_http_frame()));
    let mut child = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to launch shell");

    {
        let stdin = child.stdin.as_mut().expect("missing shell stdin");
        writeln!(stdin, "open \"{path}\"").expect("failed to write open command");
        writeln!(stdin, "status").expect("failed to write status command");
        writeln!(stdin, "list 1 --filter protocol=http").expect("failed to write list command");
        writeln!(stdin, "quit").expect("failed to write quit command");
    }

    let output = child.wait_with_output().expect("failed to wait for shell");
    fs::remove_file(&path).expect("failed to remove shell sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("IceSniff shell"));
    assert!(stdout.contains("current capture:"));
    assert!(stdout.contains("mode: text"));
    assert!(stdout.contains("Packet list"));
    assert!(stdout.contains("proto=http"));
}

#[test]
#[cfg(unix)]
fn shell_command_can_run_live_capture_without_open_file() {
    let capture_source = write_temp_capture("pcap", &wrap_pcap_packet(&sample_http_frame()));
    let tool_path = write_fake_capture_tool();
    let mut child = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .env("ICESNIFF_CAPTURE_TOOL", &tool_path)
        .env("ICESNIFF_FAKE_CAPTURE_SOURCE", &capture_source)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to launch shell");

    {
        let stdin = child.stdin.as_mut().expect("missing shell stdin");
        writeln!(stdin, "capture interfaces").expect("failed to write interfaces command");
        writeln!(stdin, "capture start fake0").expect("failed to write capture start");
        writeln!(stdin, "capture status").expect("failed to write capture status");
        thread::sleep(Duration::from_millis(700));
        writeln!(stdin, "capture stop").expect("failed to write capture stop");
        writeln!(stdin, "status").expect("failed to write status");
        writeln!(stdin, "quit").expect("failed to write quit");
    }

    let output = child.wait_with_output().expect("failed to wait for shell");
    fs::remove_file(&capture_source).expect("failed to remove fake capture source");
    fs::remove_file(&tool_path).expect("failed to remove fake capture tool");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("capture interfaces:"));
    assert!(stdout.contains("fake0"));
    assert!(stdout.contains("live capture started on fake0"));
    assert!(stdout.contains("Id"));
    assert!(stdout.contains("Source"));
    assert!(stdout.contains("Destination"));
    assert!(stdout.contains("Protocol"));
    assert!(stdout.contains("Info"));
    assert!(stdout.contains("10.0.0.1"));
    assert!(stdout.contains("93.184.216.34"));
    assert!(stdout.contains("http"));
    assert!(stdout.contains("live capture: running"));
    assert!(stdout.contains("backend: tcpdump"));
    assert!(stdout.contains("tool:"));
    assert!(stdout.contains("Capture summary"));
    assert!(stdout.contains("packet_count_hint: 1"));
}

#[test]
#[cfg(unix)]
fn shell_capture_status_reports_exited_when_capture_process_stops() {
    let capture_source = write_temp_capture("pcap", &wrap_pcap_packet(&sample_http_frame()));
    let tool_path = write_fake_capture_tool_that_exits();
    let mut child = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .env("ICESNIFF_CAPTURE_TOOL", &tool_path)
        .env("ICESNIFF_FAKE_CAPTURE_SOURCE", &capture_source)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to launch shell");

    {
        let stdin = child.stdin.as_mut().expect("missing shell stdin");
        writeln!(stdin, "capture start fake0").expect("failed to write capture start");
        thread::sleep(Duration::from_millis(2200));
        writeln!(stdin, "capture status").expect("failed to write capture status");
        writeln!(stdin, "status").expect("failed to write status");
        writeln!(stdin, "quit").expect("failed to write quit");
    }

    let output = child.wait_with_output().expect("failed to wait for shell");
    fs::remove_file(&capture_source).expect("failed to remove fake capture source");
    fs::remove_file(&tool_path).expect("failed to remove fake capture tool");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("live capture started on fake0"));
    assert!(stdout.contains("live capture: exited"));
    assert!(stdout.contains("backend: tcpdump"));
    assert!(stdout.contains("tool:"));
    assert!(stdout.contains("current capture:"));
}

#[test]
fn stats_command_reports_udp_packet() {
    let path = write_sample_pcap();
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["stats", &path])
        .output()
        .expect("failed to run stats command");
    fs::remove_file(&path).expect("failed to remove sample pcap");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("total_packets: 2"));
    assert!(stdout.contains("transport: none=1, udp=1"));
}

#[test]
fn stats_command_text_and_json_total_packets_match() {
    let path = write_sample_pcap();
    let text_output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["stats", &path])
        .output()
        .expect("failed to run text stats command");
    let json_output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "stats", &path])
        .output()
        .expect("failed to run json stats command");
    fs::remove_file(&path).expect("failed to remove sample pcap");

    assert!(text_output.status.success());
    assert!(json_output.status.success());
    let text_stdout = String::from_utf8(text_output.stdout).expect("text stdout was not utf-8");
    let json_stdout = String::from_utf8(json_output.stdout).expect("json stdout was not utf-8");
    assert_eq!(
        extract_text_u64(&text_stdout, "total_packets"),
        extract_json_u64(&json_stdout, "total_packets")
    );
}

#[test]
fn show_packet_json_contains_udp_ports() {
    let path = write_sample_pcap();
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "show-packet", &path, "0"])
        .output()
        .expect("failed to run show-packet command");
    fs::remove_file(&path).expect("failed to remove sample pcap");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"kind\":\"udp\""));
    assert!(stdout.contains("\"source_port\":5353"));
    assert!(stdout.contains("\"destination_port\":53"));
    assert!(stdout.contains("\"name\":\"ethernet\""));
    assert!(stdout.contains("\"name\":\"udp\""));
    assert!(stdout.contains("\"byte_range\":{\"start\":0,\"end\":14}"));
    assert!(stdout.contains("\"byte_range\":{\"start\":34,\"end\":42}"));
}

#[test]
fn pcapng_inspect_reports_packet_count() {
    let path = write_sample_pcapng();
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "inspect", &path])
        .output()
        .expect("failed to run inspect command");
    fs::remove_file(&path).expect("failed to remove sample pcapng");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"format\":\"pcapng\""));
    assert!(stdout.contains("\"packet_count_hint\":2"));
}

#[test]
fn pcapng_show_packet_contains_udp_ports() {
    let path = write_sample_pcapng();
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "show-packet", &path, "0"])
        .output()
        .expect("failed to run show-packet command");
    fs::remove_file(&path).expect("failed to remove sample pcapng");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"format\":\"pcapng\""));
    assert!(stdout.contains("\"kind\":\"udp\""));
    assert!(stdout.contains("\"destination_port\":53"));
    assert!(stdout.contains("\"fields\":["));
    assert!(stdout.contains("\"byte_range\":{\"start\":14,\"end\":34}"));
}

#[test]
fn dns_packet_reports_application_metadata() {
    let path = write_temp_capture("pcap", &wrap_pcap_packet(&sample_dns_frame()));
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "show-packet", &path, "0"])
        .output()
        .expect("failed to run dns show-packet command");
    fs::remove_file(&path).expect("failed to remove dns sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"kind\":\"dns\""));
    assert!(stdout.contains("\"questions\":[\"example.com\"]"));
    assert!(stdout.contains("\"name\":\"dns\""));
}

#[test]
fn tls_packet_reports_handshake_metadata() {
    let path = write_temp_capture("pcap", &wrap_pcap_packet(&sample_tls_frame()));
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "show-packet", &path, "0"])
        .output()
        .expect("failed to run tls show-packet command");
    fs::remove_file(&path).expect("failed to remove tls sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"kind\":\"tls_handshake\""));
    assert!(stdout.contains("\"handshake_type\":\"client_hello\""));
    assert!(stdout.contains("\"server_name\":\"example.com\""));
    assert!(stdout.contains("\"name\":\"tls\""));
}

#[test]
fn http_packet_reports_request_metadata() {
    let path = write_temp_capture("pcap", &wrap_pcap_packet(&sample_http_frame()));
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "show-packet", &path, "0"])
        .output()
        .expect("failed to run http show-packet command");
    fs::remove_file(&path).expect("failed to remove http sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"kind\":\"http\""));
    assert!(stdout.contains("\"message_kind\":\"request\""));
    assert!(stdout.contains("\"method\":\"GET\""));
    assert!(stdout.contains("\"path\":\"/hello\""));
    assert!(stdout.contains("\"host\":\"example.com\""));
}

#[test]
fn list_command_filters_dns_and_shows_derived_columns() {
    let path = write_temp_capture("pcap", &wrap_pcap_packet(&sample_dns_frame()));
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "list", &path, "--filter", "protocol=dns"])
        .output()
        .expect("failed to run filtered list command");
    fs::remove_file(&path).expect("failed to remove dns sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"packets_shown\":1"));
    assert!(stdout.contains("\"source\":\"192.168.1.10\""));
    assert!(stdout.contains("\"destination\":\"8.8.8.8\""));
    assert!(stdout.contains("\"protocol\":\"dns\""));
    assert!(stdout.contains("\"info\":\"dns example.com\""));
}

#[test]
fn stats_command_filters_by_port() {
    let path = write_temp_capture("pcap", &wrap_pcap_packet(&sample_tls_frame()));
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["stats", &path, "--filter", "port=443"])
        .output()
        .expect("failed to run filtered stats command");
    fs::remove_file(&path).expect("failed to remove tls sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("total_packets: 1"));
    assert!(stdout.contains("transport: tcp=1"));
}

#[test]
fn list_command_filters_by_ip_address() {
    let path = write_temp_capture("pcap", &wrap_pcap_packet(&sample_http_frame()));
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "list", &path, "--filter", "ip=10.0.0.1"])
        .output()
        .expect("failed to run ip-filtered list command");
    fs::remove_file(&path).expect("failed to remove ip-filter sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"packets_shown\":1"));
    assert!(stdout.contains("\"source\":\"10.0.0.1\""));
}

#[test]
fn list_command_filters_by_endpoint() {
    let path = write_temp_capture("pcap", &wrap_pcap_packet(&sample_http_frame()));
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args([
            "--json",
            "list",
            &path,
            "--filter",
            "endpoint=10.0.0.1:50000",
        ])
        .output()
        .expect("failed to run endpoint-filtered list command");
    fs::remove_file(&path).expect("failed to remove endpoint-filter sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"packets_shown\":1"));
    assert!(stdout.contains("\"protocol\":\"http\""));
}

#[test]
fn list_command_supports_compound_boolean_filters() {
    let path = write_temp_capture("pcap", &wrap_pcap_packet(&sample_http_frame()));
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args([
            "--json",
            "list",
            &path,
            "--filter",
            "(service=http || service=tls) && ip=10.0.0.1 && !port=443",
        ])
        .output()
        .expect("failed to run compound-filter list command");
    fs::remove_file(&path).expect("failed to remove compound-filter sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"packets_shown\":1"));
    assert!(stdout.contains("\"protocol\":\"http\""));
}

#[test]
fn list_command_text_and_json_counts_match() {
    let path = write_sample_pcap();
    let text_output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["list", &path])
        .output()
        .expect("failed to run text list command");
    let json_output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "list", &path])
        .output()
        .expect("failed to run json list command");
    fs::remove_file(&path).expect("failed to remove sample pcap");

    assert!(text_output.status.success());
    assert!(json_output.status.success());
    let text_stdout = String::from_utf8(text_output.stdout).expect("text stdout was not utf-8");
    let json_stdout = String::from_utf8(json_output.stdout).expect("json stdout was not utf-8");
    assert_eq!(
        extract_text_u64(&text_stdout, "packets_shown"),
        extract_json_u64(&json_stdout, "packets_shown")
    );
    assert_eq!(
        extract_text_u64(&text_stdout, "total_packets"),
        extract_json_u64(&json_stdout, "total_packets")
    );
}

#[test]
fn list_command_rejects_invalid_boolean_filter_expression() {
    let path = write_temp_capture("pcap", &wrap_pcap_packet(&sample_http_frame()));
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["list", &path, "--filter", "protocol=http &&"])
        .output()
        .expect("failed to run invalid-filter list command");
    fs::remove_file(&path).expect("failed to remove invalid-filter sample");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr was not utf-8");
    assert!(stderr.contains("[ISCLI_RUNTIME]"));
    assert!(stderr.contains("unexpected end of filter expression"));
}

#[test]
fn unknown_command_reports_usage_error_code_and_exit_status() {
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["definitely-not-a-command"])
        .output()
        .expect("failed to run unknown command");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("stderr was not utf-8");
    assert!(stderr.contains("[ISCLI_USAGE]"));
    assert!(stderr.contains("unknown command: definitely-not-a-command"));
}

#[test]
fn list_command_filters_by_http_fields() {
    let path = write_temp_capture("pcap", &wrap_pcap_packet(&sample_http_frame()));
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args([
            "--json",
            "list",
            &path,
            "--filter",
            "http.method=GET && http.path=/hello && http.host=example.com",
        ])
        .output()
        .expect("failed to run http-field-filtered list command");
    fs::remove_file(&path).expect("failed to remove http-field-filter sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"packets_shown\":1"));
    assert!(stdout.contains("\"protocol\":\"http\""));
}

#[test]
fn list_command_filters_by_tls_fields() {
    let path = write_temp_capture("pcap", &wrap_pcap_packet(&sample_tls_frame()));
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args([
            "--json",
            "list",
            &path,
            "--filter",
            "tls.handshake_type=client_hello && tls.server_name=example.com && tls.record_version=3.3",
        ])
        .output()
        .expect("failed to run tls-field-filtered list command");
    fs::remove_file(&path).expect("failed to remove tls-field-filter sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"packets_shown\":1"));
    assert!(stdout.contains("\"protocol\":\"tls\""));
}

#[test]
fn list_command_filters_by_dns_question() {
    let path = write_temp_capture("pcap", &wrap_pcap_packet(&sample_dns_frame()));
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args([
            "--json",
            "list",
            &path,
            "--filter",
            "dns.question=example.com && protocol=dns",
        ])
        .output()
        .expect("failed to run dns-question-filtered list command");
    fs::remove_file(&path).expect("failed to remove dns-question-filter sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"packets_shown\":1"));
    assert!(stdout.contains("\"info\":\"dns example.com\""));
}

#[test]
fn list_command_supports_status_range_filter() {
    let path = write_temp_capture(
        "pcap",
        &wrap_pcap_packet(&sample_http_response_data_frame()),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args([
            "--json",
            "list",
            &path,
            "--filter",
            "http.status>=200 && http.status<300",
        ])
        .output()
        .expect("failed to run status-range-filtered list command");
    fs::remove_file(&path).expect("failed to remove status-range-filter sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"packets_shown\":1"));
    assert!(stdout.contains("\"protocol\":\"http\""));
}

#[test]
fn list_command_supports_contains_filter() {
    let path = write_temp_capture("pcap", &wrap_pcap_packet(&sample_tls_frame()));
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args([
            "--json",
            "list",
            &path,
            "--filter",
            "tls.server_name~=example && host~=example",
        ])
        .output()
        .expect("failed to run contains-filtered list command");
    fs::remove_file(&path).expect("failed to remove contains-filter sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"packets_shown\":1"));
    assert!(stdout.contains("\"protocol\":\"tls\""));
}

#[test]
fn list_command_supports_case_insensitive_text_filter() {
    let path = write_temp_capture("pcap", &wrap_pcap_packet(&sample_http_frame()));
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args([
            "--json",
            "list",
            &path,
            "--filter",
            "http.method=get && host=EXAMPLE.COM",
        ])
        .output()
        .expect("failed to run case-insensitive text filter command");
    fs::remove_file(&path).expect("failed to remove case-insensitive filter sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"packets_shown\":1"));
    assert!(stdout.contains("\"protocol\":\"http\""));
}

#[test]
fn list_command_supports_additional_dns_fields() {
    let path = write_temp_capture("pcap", &wrap_pcap_packet(&sample_dns_frame()));
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args([
            "--json",
            "list",
            &path,
            "--filter",
            "dns.question_count=1 && dns.answer_count=0 && dns.is_response=false",
        ])
        .output()
        .expect("failed to run dns field filter command");
    fs::remove_file(&path).expect("failed to remove dns field filter sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"packets_shown\":1"));
    assert!(stdout.contains("\"protocol\":\"dns\""));
}

#[test]
fn list_command_supports_tls_handshake_length_range() {
    let path = write_temp_capture("pcap", &wrap_pcap_packet(&sample_tls_frame()));
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args([
            "--json",
            "list",
            &path,
            "--filter",
            "tls.handshake_length>=60 && tls.handshake_length<70",
        ])
        .output()
        .expect("failed to run tls handshake length filter command");
    fs::remove_file(&path).expect("failed to remove tls handshake length sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"packets_shown\":1"));
    assert!(stdout.contains("\"protocol\":\"tls\""));
}

#[test]
fn conversations_command_groups_bidirectional_dns_flow() {
    let capture = wrap_pcap_packets(&[sample_dns_frame(), sample_dns_response_frame()]);
    let path = write_temp_capture("pcap", &capture);
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "conversations", &path, "--filter", "protocol=dns"])
        .output()
        .expect("failed to run conversations command");
    fs::remove_file(&path).expect("failed to remove dns conversation sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"total_conversations\":1"));
    assert!(stdout.contains("\"service\":\"dns\""));
    assert!(stdout.contains("\"protocol\":\"dns\""));
    assert!(stdout.contains("\"packets\":2"));
    assert!(stdout.contains("\"packets_a_to_b\":1"));
    assert!(stdout.contains("\"packets_b_to_a\":1"));
    assert!(stdout.contains("\"request_count\":1"));
    assert!(stdout.contains("\"response_count\":1"));
}

#[test]
fn conversations_command_text_and_json_total_match() {
    let capture = wrap_pcap_packets(&[sample_dns_frame(), sample_dns_response_frame()]);
    let path = write_temp_capture("pcap", &capture);
    let text_output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["conversations", &path, "--filter", "protocol=dns"])
        .output()
        .expect("failed to run text conversations command");
    let json_output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "conversations", &path, "--filter", "protocol=dns"])
        .output()
        .expect("failed to run json conversations command");
    fs::remove_file(&path).expect("failed to remove dns conversation sample");

    assert!(text_output.status.success());
    assert!(json_output.status.success());
    let text_stdout = String::from_utf8(text_output.stdout).expect("text stdout was not utf-8");
    let json_stdout = String::from_utf8(json_output.stdout).expect("json stdout was not utf-8");
    assert_eq!(
        extract_text_u64(&text_stdout, "total_conversations"),
        extract_json_u64(&json_stdout, "total_conversations")
    );
}

#[test]
fn streams_command_reports_http_transaction_summary() {
    let capture = wrap_pcap_packets(&[
        sample_http_request_fragment_one(),
        sample_http_request_fragment_two(),
        sample_http_response_fragment_one(),
        sample_http_response_fragment_two(),
    ]);
    let path = write_temp_capture("pcap", &capture);
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "streams", &path, "--filter", "protocol=http"])
        .output()
        .expect("failed to run streams command");
    fs::remove_file(&path).expect("failed to remove http stream sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"total_streams\":1"));
    assert!(stdout.contains("\"service\":\"http\""));
    assert!(stdout.contains("\"request_count\":1"));
    assert!(stdout.contains("\"response_count\":1"));
    assert!(stdout.contains("\"matched_transactions\":1"));
    assert!(stdout.contains("\"unmatched_requests\":0"));
    assert!(stdout.contains("\"unmatched_responses\":0"));
    assert!(stdout.contains("\"notes\":[]"));
}

#[test]
fn streams_command_counts_multiple_http_transactions() {
    let capture = wrap_pcap_packets(&[
        sample_http_double_request_frame(),
        sample_http_double_response_frame(),
    ]);
    let path = write_temp_capture("pcap", &capture);
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "streams", &path, "--filter", "protocol=http"])
        .output()
        .expect("failed to run streams command");
    fs::remove_file(&path).expect("failed to remove double http stream sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"total_streams\":1"));
    assert!(stdout.contains("\"request_count\":2"));
    assert!(stdout.contains("\"response_count\":2"));
    assert!(stdout.contains("\"matched_transactions\":2"));
    assert!(stdout.contains("\"transaction_timeline\":["));
    assert!(stdout.contains("http stream shows pipelined requests"));
}

#[test]
fn streams_command_notes_retransmitted_segments() {
    let capture = wrap_pcap_packets(&[
        sample_http_request_fragment_one(),
        sample_http_request_fragment_one(),
        sample_http_request_fragment_two(),
        sample_http_response_fragment_one(),
        sample_http_response_fragment_two(),
    ]);
    let path = write_temp_capture("pcap", &capture);
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "streams", &path, "--filter", "protocol=http"])
        .output()
        .expect("failed to run streams command");
    fs::remove_file(&path).expect("failed to remove retransmit stream sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"request_count\":1"));
    assert!(stdout.contains("\"response_count\":1"));
    assert!(stdout.contains("retransmitted segments that were ignored"));
}

#[test]
fn streams_command_reorders_out_of_order_http_segments() {
    let capture = wrap_pcap_packets(&[
        sample_http_request_fragment_two(),
        sample_http_request_fragment_one(),
        sample_http_response_fragment_two(),
        sample_http_response_fragment_one(),
    ]);
    let path = write_temp_capture("pcap", &capture);
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "streams", &path, "--filter", "protocol=http"])
        .output()
        .expect("failed to run streams command");
    fs::remove_file(&path).expect("failed to remove out-of-order stream sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"request_count\":1"));
    assert!(stdout.contains("\"response_count\":1"));
    assert!(stdout.contains("out-of-order segments that were reordered"));
}

#[test]
fn streams_command_reports_closed_tcp_session_state() {
    let capture = wrap_pcap_packets(&[
        sample_http_syn_frame(),
        sample_http_syn_ack_frame(),
        sample_http_frame(),
        sample_http_response_data_frame(),
        sample_http_fin_frame(),
        sample_http_fin_ack_frame(),
    ]);
    let path = write_temp_capture("pcap", &capture);
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "streams", &path, "--filter", "protocol=http"])
        .output()
        .expect("failed to run streams command");
    fs::remove_file(&path).expect("failed to remove closed-session sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"session_state\":\"closed\""));
    assert!(stdout.contains("\"syn_packets\":2"));
    assert!(stdout.contains("\"fin_packets\":2"));
    assert!(stdout.contains("\"rst_packets\":0"));
    assert!(stdout.contains("\"request_count\":1"));
    assert!(stdout.contains("\"response_count\":1"));
}

#[test]
fn streams_command_reports_tls_handshake_progress() {
    let capture = wrap_pcap_packets(&[
        sample_tls_frame(),
        sample_tls_server_hello_frame(),
        sample_tls_certificate_frame(),
        sample_tls_finished_frame(),
    ]);
    let path = write_temp_capture("pcap", &capture);
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "streams", &path, "--filter", "protocol=tls"])
        .output()
        .expect("failed to run tls streams command");
    fs::remove_file(&path).expect("failed to remove tls stream sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"service\":\"tls\""));
    assert!(stdout.contains("\"request_count\":1"));
    assert!(stdout.contains("\"response_count\":1"));
    assert!(stdout.contains("\"tls_client_hellos\":1"));
    assert!(stdout.contains("\"tls_server_hellos\":1"));
    assert!(stdout.contains("\"tls_certificates\":1"));
    assert!(stdout.contains("\"tls_finished_messages\":1"));
    assert!(stdout.contains("\"tls_handshake_state\":\"finished_seen\""));
}

#[test]
fn streams_command_reports_tls_reset_state() {
    let capture = wrap_pcap_packets(&[sample_tls_frame(), sample_tls_reset_frame()]);
    let path = write_temp_capture("pcap", &capture);
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "streams", &path, "--filter", "protocol=tls"])
        .output()
        .expect("failed to run tls reset streams command");
    fs::remove_file(&path).expect("failed to remove tls reset sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"session_state\":\"reset\""));
    assert!(stdout.contains("\"rst_packets\":1"));
    assert!(stdout.contains("\"tls_handshake_state\":\"reset_after_client_hello\""));
}

#[test]
fn streams_command_supports_stream_level_filtering() {
    let capture = wrap_pcap_packets(&[sample_tls_frame(), sample_tls_reset_frame()]);
    let path = write_temp_capture("pcap", &capture);
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args([
            "--json",
            "streams",
            &path,
            "--filter",
            "protocol=tls",
            "--stream-filter",
            "stream.state=reset && stream.rst>=1",
        ])
        .output()
        .expect("failed to run stream-filtered streams command");
    fs::remove_file(&path).expect("failed to remove stream-filter sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"total_streams\":1"));
    assert!(stdout.contains("\"session_state\":\"reset\""));
}

#[test]
fn streams_command_text_and_json_total_match() {
    let capture = wrap_pcap_packets(&[
        sample_http_request_fragment_one(),
        sample_http_request_fragment_two(),
        sample_http_response_fragment_one(),
        sample_http_response_fragment_two(),
    ]);
    let path = write_temp_capture("pcap", &capture);
    let text_output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["streams", &path, "--filter", "protocol=http"])
        .output()
        .expect("failed to run text streams command");
    let json_output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "streams", &path, "--filter", "protocol=http"])
        .output()
        .expect("failed to run json streams command");
    fs::remove_file(&path).expect("failed to remove stream sample");

    assert!(text_output.status.success());
    assert!(json_output.status.success());
    let text_stdout = String::from_utf8(text_output.stdout).expect("text stdout was not utf-8");
    let json_stdout = String::from_utf8(json_output.stdout).expect("json stdout was not utf-8");
    assert_eq!(
        extract_text_u64(&text_stdout, "total_streams"),
        extract_json_u64(&json_stdout, "total_streams")
    );
}

#[test]
fn streams_command_supports_derived_session_filters() {
    let capture = wrap_pcap_packets(&[
        sample_http_double_request_frame(),
        sample_http_double_response_frame(),
    ]);
    let path = write_temp_capture("pcap", &capture);
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args([
            "--json",
            "streams",
            &path,
            "--filter",
            "protocol=http",
            "--stream-filter",
            "stream.is_pipelined=true && stream.client_packets=1 && stream.server_packets=1 && stream.has_timeline=true",
        ])
        .output()
        .expect("failed to run derived stream-filter command");
    fs::remove_file(&path).expect("failed to remove derived stream-filter sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"total_streams\":1"));
    assert!(stdout.contains("\"matched_transactions\":2"));
}

#[test]
fn streams_command_reports_tls_alert_timeline() {
    let capture = wrap_pcap_packets(&[
        sample_tls_alpn_frame(),
        sample_tls_server_hello_frame(),
        sample_tls_alert_frame(),
    ]);
    let path = write_temp_capture("pcap", &capture);
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "streams", &path, "--filter", "service=tls"])
        .output()
        .expect("failed to run tls alert streams command");
    fs::remove_file(&path).expect("failed to remove tls alert stream sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"tls_alert_count\":1"));
    assert!(stdout.contains("\"tls_alerts\":[\"fatal:handshake_failure\"]"));
    assert!(stdout.contains("\"transaction_timeline\":["));
    assert!(stdout.contains("client_hello"));
    assert!(stdout.contains("server_hello"));
    assert!(stdout.contains("handshake_failure"));
}

#[test]
fn streams_command_reports_multiple_tls_handshakes() {
    let capture = wrap_pcap_packets(&[
        sample_tls_frame(),
        sample_tls_server_hello_frame(),
        sample_tls_second_client_hello_frame(),
        sample_tls_second_server_hello_frame(),
    ]);
    let path = write_temp_capture("pcap", &capture);
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "streams", &path, "--filter", "protocol=tls"])
        .output()
        .expect("failed to run repeated tls streams command");
    fs::remove_file(&path).expect("failed to remove repeated tls sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"tls_client_hellos\":2"));
    assert!(stdout.contains("\"tls_server_hellos\":2"));
    assert!(stdout.contains("\"tls_handshake_cycles\":2"));
    assert!(stdout.contains("\"tls_incomplete_handshakes\":0"));
    assert!(stdout.contains("\"tls_handshake_state\":\"multiple_handshakes_seen\""));
}

#[test]
fn transactions_command_reports_multiple_http_transactions() {
    let capture = wrap_pcap_packets(&[
        sample_http_double_request_frame(),
        sample_http_double_response_frame(),
    ]);
    let path = write_temp_capture("pcap", &capture);
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "transactions", &path, "--filter", "protocol=http"])
        .output()
        .expect("failed to run http transactions command");
    fs::remove_file(&path).expect("failed to remove http transactions sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"total_transactions\":2"));
    assert!(stdout.contains("\"service\":\"http\""));
    assert!(stdout.contains("\"request_summary\":\"GET /one\""));
    assert!(stdout.contains("\"request_summary\":\"GET /two\""));
    assert!(stdout.contains("\"request_details\":["));
    assert!(stdout.contains("\"key\":\"method\",\"value\":\"GET\""));
    assert!(stdout.contains("\"key\":\"path\",\"value\":\"/one\""));
    assert!(stdout.contains("\"key\":\"host\",\"value\":\"example.com\""));
    assert!(stdout.contains("\"response_summary\":\"200 OK\""));
    assert!(stdout.contains("\"response_summary\":\"204 No Content\""));
    assert!(stdout.contains("\"key\":\"status_code\",\"value\":\"200\""));
    assert!(stdout.contains("\"key\":\"reason_phrase\",\"value\":\"No Content\""));
    assert!(stdout.contains("\"key\":\"body_bytes\",\"value\":\"2\""));
    assert!(stdout.contains("\"key\":\"transfer_semantics\",\"value\":\"content-length\""));
    assert!(stdout.contains("\"state\":\"matched\""));
}

#[test]
fn transactions_command_reports_chunked_http_response_details() {
    let capture = wrap_pcap_packets(&[sample_http_frame(), sample_http_chunked_response_frame()]);
    let path = write_temp_capture("pcap", &capture);
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "transactions", &path, "--filter", "service=http"])
        .output()
        .expect("failed to run chunked http transactions command");
    fs::remove_file(&path).expect("failed to remove chunked http transactions sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"total_transactions\":1"));
    assert!(stdout.contains("\"response_summary\":\"200 OK\""));
    assert!(stdout.contains("\"key\":\"transfer_semantics\",\"value\":\"chunked\""));
    assert!(stdout.contains("\"key\":\"transfer_encoding\",\"value\":\"chunked\""));
    assert!(stdout.contains("\"key\":\"body_bytes\",\"value\":\"5\""));
}

#[test]
fn transactions_command_reports_multiple_tls_handshakes() {
    let capture = wrap_pcap_packets(&[
        sample_tls_frame(),
        sample_tls_server_hello_frame(),
        sample_tls_second_client_hello_frame(),
        sample_tls_second_server_hello_frame(),
    ]);
    let path = write_temp_capture("pcap", &capture);
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "transactions", &path, "--filter", "protocol=tls"])
        .output()
        .expect("failed to run tls transactions command");
    fs::remove_file(&path).expect("failed to remove tls transactions sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"total_transactions\":2"));
    assert!(stdout.contains("\"service\":\"tls\""));
    assert!(stdout.contains("\"request_summary\":\"client_hello\""));
    assert!(stdout.contains("\"response_summary\":\"server_hello\""));
    assert!(stdout.contains("\"state\":\"server_hello_seen\""));
}

#[test]
fn transactions_command_reports_tls_handshake_progress_per_row() {
    let capture = wrap_pcap_packets(&[
        sample_tls_frame(),
        sample_tls_server_hello_frame(),
        sample_tls_certificate_frame(),
        sample_tls_finished_frame(),
    ]);
    let path = write_temp_capture("pcap", &capture);
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "transactions", &path, "--filter", "service=tls"])
        .output()
        .expect("failed to run progressed tls transactions command");
    fs::remove_file(&path).expect("failed to remove progressed tls transactions sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"total_transactions\":1"));
    assert!(stdout.contains("\"request_summary\":\"client_hello\""));
    assert!(stdout.contains("\"key\":\"record_version\",\"value\":\"3.3\""));
    assert!(stdout.contains("\"key\":\"server_name\",\"value\":\"example.com\""));
    assert!(stdout.contains("\"key\":\"handshake_messages\",\"value\":\"client_hello\""));
    assert!(stdout.contains("\"response_summary\":\"server_hello + certificate + finished\""));
    assert!(stdout.contains("\"response_details\":["));
    assert!(stdout.contains(
        "\"key\":\"handshake_messages\",\"value\":\"server_hello,certificate,finished\""
    ));
    assert!(stdout.contains("\"state\":\"finished_seen\""));
    assert!(stdout.contains("\"notes\":[\"certificate_seen\",\"finished_seen\"]"));
}

#[test]
fn transactions_command_reports_tls_alpn_and_alert_details() {
    let capture = wrap_pcap_packets(&[
        sample_tls_alpn_frame(),
        sample_tls_server_hello_frame(),
        sample_tls_alert_frame(),
    ]);
    let path = write_temp_capture("pcap", &capture);
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "transactions", &path, "--filter", "service=tls"])
        .output()
        .expect("failed to run tls alpn transactions command");
    fs::remove_file(&path).expect("failed to remove tls alpn transactions sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"total_transactions\":1"));
    assert!(stdout.contains("\"key\":\"alpn\",\"value\":\"h2\""));
    assert!(stdout.contains("\"response_summary\":\"server_hello + alert\""));
    assert!(stdout.contains("\"key\":\"alerts\",\"value\":\"fatal:handshake_failure\""));
    assert!(stdout.contains("\"state\":\"alert_seen\""));
}

#[test]
fn transactions_command_supports_transaction_level_filtering() {
    let capture = wrap_pcap_packets(&[
        sample_http_double_request_frame(),
        sample_http_double_response_frame(),
    ]);
    let path = write_temp_capture("pcap", &capture);
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args([
            "--json",
            "transactions",
            &path,
            "--filter",
            "protocol=http",
            "--transaction-filter",
            "tx.request.path=/two && tx.response.status_code=204 && tx.state=matched",
        ])
        .output()
        .expect("failed to run transaction-filtered transactions command");
    fs::remove_file(&path).expect("failed to remove transaction-filter sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"total_transactions\":1"));
    assert!(stdout.contains("\"request_summary\":\"GET /two\""));
    assert!(stdout.contains("\"response_summary\":\"204 No Content\""));
}

#[test]
fn transactions_command_text_and_json_total_match() {
    let capture = wrap_pcap_packets(&[
        sample_http_request_fragment_one(),
        sample_http_request_fragment_two(),
        sample_http_response_fragment_one(),
        sample_http_response_fragment_two(),
    ]);
    let path = write_temp_capture("pcap", &capture);
    let text_output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["transactions", &path, "--filter", "protocol=http"])
        .output()
        .expect("failed to run text transactions command");
    let json_output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args(["--json", "transactions", &path, "--filter", "protocol=http"])
        .output()
        .expect("failed to run json transactions command");
    fs::remove_file(&path).expect("failed to remove transaction sample");

    assert!(text_output.status.success());
    assert!(json_output.status.success());
    let text_stdout = String::from_utf8(text_output.stdout).expect("text stdout was not utf-8");
    let json_stdout = String::from_utf8(json_output.stdout).expect("json stdout was not utf-8");
    assert_eq!(
        extract_text_u64(&text_stdout, "total_transactions"),
        extract_json_u64(&json_stdout, "total_transactions")
    );
}

#[test]
fn transactions_command_supports_protocol_alias_filters() {
    let capture = wrap_pcap_packets(&[
        sample_tls_alpn_frame(),
        sample_tls_server_hello_frame(),
        sample_tls_alert_frame(),
    ]);
    let path = write_temp_capture("pcap", &capture);
    let output = Command::new(env!("CARGO_BIN_EXE_icesniff-cli"))
        .args([
            "--json",
            "transactions",
            &path,
            "--filter",
            "service=tls",
            "--transaction-filter",
            "tx.has_alerts=true && tx.tls.alpn=h2 && tx.tls.alerts~=handshake_failure",
        ])
        .output()
        .expect("failed to run protocol-alias transaction filter command");
    fs::remove_file(&path).expect("failed to remove protocol-alias transaction sample");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout was not utf-8");
    assert!(stdout.contains("\"total_transactions\":1"));
    assert!(stdout.contains("\"state\":\"alert_seen\""));
}

fn sample_pcap_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();

    bytes.extend_from_slice(&[0xd4, 0xc3, 0xb2, 0xa1]);
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&4u16.to_le_bytes());
    bytes.extend_from_slice(&0i32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&65535u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());

    let first_packet = sample_udp_frame();
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&(first_packet.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(first_packet.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&first_packet);

    bytes.extend_from_slice(&3u32.to_le_bytes());
    bytes.extend_from_slice(&4u32.to_le_bytes());
    bytes.extend_from_slice(&3u32.to_le_bytes());
    bytes.extend_from_slice(&5u32.to_le_bytes());
    bytes.extend_from_slice(&[0x01, 0x02, 0x03]);

    bytes
}

fn wrap_pcap_packet(packet: &[u8]) -> Vec<u8> {
    wrap_pcap_packets(&[packet.to_vec()])
}

fn wrap_pcap_packets(packets: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&[0xd4, 0xc3, 0xb2, 0xa1]);
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&4u16.to_le_bytes());
    bytes.extend_from_slice(&0i32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&65535u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());

    for (index, packet) in packets.iter().enumerate() {
        bytes.extend_from_slice(&((index as u32) + 1).to_le_bytes());
        bytes.extend_from_slice(&((index as u32) + 2).to_le_bytes());
        bytes.extend_from_slice(&(packet.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&(packet.len() as u32).to_le_bytes());
        bytes.extend_from_slice(packet);
    }

    bytes
}

fn sample_pcapng_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();

    bytes.extend_from_slice(&[
        0x0a, 0x0d, 0x0d, 0x0a, 0x1c, 0x00, 0x00, 0x00, 0x4d, 0x3c, 0x2b, 0x1a, 0x01, 0x00, 0x00,
        0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x1c, 0x00, 0x00, 0x00,
    ]);
    bytes.extend_from_slice(&[
        0x01, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00,
        0x00, 0x14, 0x00, 0x00, 0x00,
    ]);

    let first_packet = sample_udp_frame();
    bytes.extend_from_slice(&[0x06, 0x00, 0x00, 0x00]);
    bytes.extend_from_slice(&(80u32.to_le_bytes()));
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&1_000_002u32.to_le_bytes());
    bytes.extend_from_slice(&(first_packet.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(first_packet.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&first_packet);
    bytes.extend_from_slice(&[0x00, 0x00]);
    bytes.extend_from_slice(&(80u32.to_le_bytes()));

    bytes.extend_from_slice(&[0x06, 0x00, 0x00, 0x00]);
    bytes.extend_from_slice(&(36u32.to_le_bytes()));
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&3_000_004u32.to_le_bytes());
    bytes.extend_from_slice(&3u32.to_le_bytes());
    bytes.extend_from_slice(&5u32.to_le_bytes());
    bytes.extend_from_slice(&[0x01, 0x02, 0x03, 0x00]);
    bytes.extend_from_slice(&(36u32.to_le_bytes()));

    bytes
}

fn sample_udp_frame() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    bytes.extend_from_slice(&[0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb]);
    bytes.extend_from_slice(&[0x08, 0x00]);
    bytes.extend_from_slice(&[
        0x45, 0x00, 0x00, 0x20, 0x12, 0x34, 0x00, 0x00, 0x40, 0x11, 0x00, 0x00, 192, 168, 1, 10, 8,
        8, 8, 8,
    ]);
    bytes.extend_from_slice(&[
        0x14, 0xe9, 0x00, 0x35, 0x00, 0x0c, 0x00, 0x00, 0xde, 0xad, 0xbe, 0xef,
    ]);
    bytes
}

fn sample_dns_frame() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    bytes.extend_from_slice(&[0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb]);
    bytes.extend_from_slice(&[0x08, 0x00]);
    let dns_payload: [u8; 29] = [
        0x12, 0x34, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, b'e', b'x',
        b'a', b'm', b'p', b'l', b'e', 0x03, b'c', b'o', b'm', 0x00, 0x00, 0x01, 0x00, 0x01,
    ];
    let total_length = 20 + 8 + dns_payload.len() as u16;
    bytes.extend_from_slice(&[
        0x45,
        0x00,
        (total_length >> 8) as u8,
        total_length as u8,
        0x12,
        0x34,
        0x00,
        0x00,
        0x40,
        0x11,
        0x00,
        0x00,
        192,
        168,
        1,
        10,
        8,
        8,
        8,
        8,
    ]);
    let udp_length = 8 + dns_payload.len() as u16;
    bytes.extend_from_slice(&[
        0xd4,
        0x31,
        0x00,
        0x35,
        (udp_length >> 8) as u8,
        udp_length as u8,
        0x00,
        0x00,
    ]);
    bytes.extend_from_slice(&dns_payload);
    bytes
}

fn sample_tls_frame() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    bytes.extend_from_slice(&[0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb]);
    bytes.extend_from_slice(&[0x08, 0x00]);
    let tls_payload: Vec<u8> = vec![
        22, 3, 3, 0, 67, 1, 0, 0, 63, 3, 3, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
        16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 0, 0, 2, 0, 47, 1, 0, 0,
        20, 0, 0, 0, 16, 0, 14, 0, 0, 11, b'e', b'x', b'a', b'm', b'p', b'l', b'e', b'.', b'c',
        b'o', b'm',
    ];
    let tcp_length = 20 + tls_payload.len() as u16;
    let total_length = 20 + tcp_length;
    bytes.extend_from_slice(&[
        0x45,
        0x00,
        (total_length >> 8) as u8,
        total_length as u8,
        0xab,
        0xcd,
        0x00,
        0x00,
        0x40,
        0x06,
        0x00,
        0x00,
        10,
        0,
        0,
        1,
        93,
        184,
        216,
        34,
    ]);
    bytes.extend_from_slice(&[
        0xc3, 0x50, 0x01, 0xbb, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x50, 0x18, 0x04,
        0x00, 0x00, 0x00, 0x00, 0x00,
    ]);
    bytes.extend_from_slice(&tls_payload);
    bytes
}

fn sample_tls_alpn_frame() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    bytes.extend_from_slice(&[0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb]);
    bytes.extend_from_slice(&[0x08, 0x00]);
    let tls_payload: Vec<u8> = vec![
        22, 3, 3, 0, 76, 1, 0, 0, 72, 3, 3, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
        16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 0, 0, 2, 0, 47, 1, 0, 0,
        29, 0, 0, 0, 16, 0, 14, 0, 0, 11, b'e', b'x', b'a', b'm', b'p', b'l', b'e', b'.', b'c',
        b'o', b'm', 0, 16, 0, 5, 0, 3, 2, b'h', b'2',
    ];
    let tcp_length = 20 + tls_payload.len() as u16;
    let total_length = 20 + tcp_length;
    bytes.extend_from_slice(&[
        0x45,
        0x00,
        (total_length >> 8) as u8,
        total_length as u8,
        0xab,
        0xce,
        0x00,
        0x00,
        0x40,
        0x06,
        0x00,
        0x00,
        10,
        0,
        0,
        1,
        93,
        184,
        216,
        34,
    ]);
    bytes.extend_from_slice(&[
        0xc3, 0x50, 0x01, 0xbb, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x50, 0x18, 0x04,
        0x00, 0x00, 0x00, 0x00, 0x00,
    ]);
    bytes.extend_from_slice(&tls_payload);
    bytes
}

fn sample_tls_server_hello_frame() -> Vec<u8> {
    build_tcp_ipv4_frame(
        [0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb],
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
        [93, 184, 216, 34],
        [10, 0, 0, 1],
        443,
        50000,
        2,
        72,
        0xabdb,
        &[22, 3, 3, 0, 8, 2, 0, 0, 4, 3, 3, 0, 0],
    )
}

fn sample_tls_certificate_frame() -> Vec<u8> {
    build_tcp_ipv4_frame(
        [0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb],
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
        [93, 184, 216, 34],
        [10, 0, 0, 1],
        443,
        50000,
        15,
        72,
        0xabdc,
        &[22, 3, 3, 0, 8, 11, 0, 0, 4, 0, 0, 0, 0],
    )
}

fn sample_tls_finished_frame() -> Vec<u8> {
    build_tcp_ipv4_frame(
        [0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb],
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
        [93, 184, 216, 34],
        [10, 0, 0, 1],
        443,
        50000,
        28,
        72,
        0xabdd,
        &[22, 3, 3, 0, 8, 20, 0, 0, 4, 0, 0, 0, 0],
    )
}

fn sample_tls_reset_frame() -> Vec<u8> {
    build_tcp_ipv4_frame_with_flags(
        [0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb],
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
        [93, 184, 216, 34],
        [10, 0, 0, 1],
        443,
        50000,
        2,
        72,
        0xabde,
        0x14,
        &[],
    )
}

fn sample_tls_second_client_hello_frame() -> Vec<u8> {
    build_tcp_ipv4_frame(
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
        [0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb],
        [10, 0, 0, 1],
        [93, 184, 216, 34],
        50000,
        443,
        73,
        28,
        0xabdf,
        &[22, 3, 3, 0, 8, 1, 0, 0, 4, 3, 3, 0, 1],
    )
}

fn sample_tls_second_server_hello_frame() -> Vec<u8> {
    build_tcp_ipv4_frame(
        [0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb],
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
        [93, 184, 216, 34],
        [10, 0, 0, 1],
        443,
        50000,
        15,
        85,
        0xabe0,
        &[22, 3, 3, 0, 8, 2, 0, 0, 4, 3, 3, 0, 1],
    )
}

fn sample_tls_alert_frame() -> Vec<u8> {
    build_tcp_ipv4_frame(
        [0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb],
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
        [93, 184, 216, 34],
        [10, 0, 0, 1],
        443,
        50000,
        15,
        81,
        0xabe1,
        &[21, 3, 3, 0, 2, 2, 40],
    )
}

fn sample_dns_response_frame() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&[0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb]);
    bytes.extend_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    bytes.extend_from_slice(&[0x08, 0x00]);
    let dns_payload: [u8; 45] = [
        0x12, 0x34, 0x81, 0x80, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x07, b'e', b'x',
        b'a', b'm', b'p', b'l', b'e', 0x03, b'c', b'o', b'm', 0x00, 0x00, 0x01, 0x00, 0x01, 0xc0,
        0x0c, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x3c, 0x00, 0x04, 93, 184, 216, 34,
    ];
    let total_length = 20 + 8 + dns_payload.len() as u16;
    bytes.extend_from_slice(&[
        0x45,
        0x00,
        (total_length >> 8) as u8,
        total_length as u8,
        0x12,
        0x35,
        0x00,
        0x00,
        0x40,
        0x11,
        0x00,
        0x00,
        8,
        8,
        8,
        8,
        192,
        168,
        1,
        10,
    ]);
    let udp_length = 8 + dns_payload.len() as u16;
    bytes.extend_from_slice(&[
        0x00,
        0x35,
        0xd4,
        0x31,
        (udp_length >> 8) as u8,
        udp_length as u8,
        0x00,
        0x00,
    ]);
    bytes.extend_from_slice(&dns_payload);
    bytes
}

fn sample_http_frame() -> Vec<u8> {
    build_tcp_ipv4_frame(
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
        [0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb],
        [10, 0, 0, 1],
        [93, 184, 216, 34],
        50000,
        80,
        1,
        0,
        0xabce,
        b"GET /hello HTTP/1.1\r\nHost: example.com\r\n\r\n",
    )
}

fn sample_http_response_data_frame() -> Vec<u8> {
    build_tcp_ipv4_frame(
        [0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb],
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
        [93, 184, 216, 34],
        [10, 0, 0, 1],
        80,
        50000,
        2,
        42,
        0xabd6,
        b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello",
    )
}

fn sample_http_syn_frame() -> Vec<u8> {
    build_tcp_ipv4_frame_with_flags(
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
        [0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb],
        [10, 0, 0, 1],
        [93, 184, 216, 34],
        50000,
        80,
        0,
        0,
        0xabd7,
        0x02,
        &[],
    )
}

fn sample_http_syn_ack_frame() -> Vec<u8> {
    build_tcp_ipv4_frame_with_flags(
        [0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb],
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
        [93, 184, 216, 34],
        [10, 0, 0, 1],
        80,
        50000,
        0,
        1,
        0xabd8,
        0x12,
        &[],
    )
}

fn sample_http_fin_frame() -> Vec<u8> {
    build_tcp_ipv4_frame_with_flags(
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
        [0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb],
        [10, 0, 0, 1],
        [93, 184, 216, 34],
        50000,
        80,
        42,
        47,
        0xabd9,
        0x11,
        &[],
    )
}

fn sample_http_fin_ack_frame() -> Vec<u8> {
    build_tcp_ipv4_frame_with_flags(
        [0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb],
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
        [93, 184, 216, 34],
        [10, 0, 0, 1],
        80,
        50000,
        47,
        43,
        0xabda,
        0x11,
        &[],
    )
}

fn sample_http_request_fragment_one() -> Vec<u8> {
    build_tcp_ipv4_frame(
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
        [0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb],
        [10, 0, 0, 1],
        [93, 184, 216, 34],
        50000,
        80,
        1,
        0,
        0xabd0,
        b"GET /hello HTTP/1.1\r\nHo",
    )
}

fn sample_http_double_request_frame() -> Vec<u8> {
    build_tcp_ipv4_frame(
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
        [0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb],
        [10, 0, 0, 1],
        [93, 184, 216, 34],
        50000,
        80,
        1,
        0,
        0xabd4,
        b"GET /one HTTP/1.1\r\nHost: example.com\r\n\r\nGET /two HTTP/1.1\r\nHost: example.com\r\n\r\n",
    )
}

fn sample_http_double_response_frame() -> Vec<u8> {
    build_tcp_ipv4_frame(
        [0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb],
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
        [93, 184, 216, 34],
        [10, 0, 0, 1],
        80,
        50000,
        2,
        83,
        0xabd5,
        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nokHTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n",
    )
}

fn sample_http_chunked_response_frame() -> Vec<u8> {
    build_tcp_ipv4_frame(
        [0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb],
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
        [93, 184, 216, 34],
        [10, 0, 0, 1],
        80,
        50000,
        2,
        42,
        0xabd6,
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Type: text/plain\r\n\r\n5\r\nhello\r\n0\r\n\r\n",
    )
}

fn sample_http_request_fragment_two() -> Vec<u8> {
    build_tcp_ipv4_frame(
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
        [0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb],
        [10, 0, 0, 1],
        [93, 184, 216, 34],
        50000,
        80,
        24,
        0,
        0xabd1,
        b"st: example.com\r\n\r\n",
    )
}

fn sample_http_response_fragment_one() -> Vec<u8> {
    build_tcp_ipv4_frame(
        [0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb],
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
        [93, 184, 216, 34],
        [10, 0, 0, 1],
        80,
        50000,
        2,
        45,
        0xabd2,
        b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhe",
    )
}

fn sample_http_response_fragment_two() -> Vec<u8> {
    build_tcp_ipv4_frame(
        [0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb],
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
        [93, 184, 216, 34],
        [10, 0, 0, 1],
        80,
        50000,
        42,
        45,
        0xabd3,
        b"llo",
    )
}

fn build_tcp_ipv4_frame(
    destination_mac: [u8; 6],
    source_mac: [u8; 6],
    source_ip: [u8; 4],
    destination_ip: [u8; 4],
    source_port: u16,
    destination_port: u16,
    sequence_number: u32,
    acknowledgement_number: u32,
    identification: u16,
    payload: &[u8],
) -> Vec<u8> {
    build_tcp_ipv4_frame_with_flags(
        destination_mac,
        source_mac,
        source_ip,
        destination_ip,
        source_port,
        destination_port,
        sequence_number,
        acknowledgement_number,
        identification,
        0x18,
        payload,
    )
}

fn build_tcp_ipv4_frame_with_flags(
    destination_mac: [u8; 6],
    source_mac: [u8; 6],
    source_ip: [u8; 4],
    destination_ip: [u8; 4],
    source_port: u16,
    destination_port: u16,
    sequence_number: u32,
    acknowledgement_number: u32,
    identification: u16,
    tcp_flags: u8,
    payload: &[u8],
) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&destination_mac);
    bytes.extend_from_slice(&source_mac);
    bytes.extend_from_slice(&[0x08, 0x00]);
    let tcp_length = 20 + payload.len() as u16;
    let total_length = 20 + tcp_length;
    bytes.extend_from_slice(&[
        0x45,
        0x00,
        (total_length >> 8) as u8,
        total_length as u8,
        (identification >> 8) as u8,
        identification as u8,
        0x00,
        0x00,
        0x40,
        0x06,
        0x00,
        0x00,
        source_ip[0],
        source_ip[1],
        source_ip[2],
        source_ip[3],
        destination_ip[0],
        destination_ip[1],
        destination_ip[2],
        destination_ip[3],
    ]);
    bytes.extend_from_slice(&[
        (source_port >> 8) as u8,
        source_port as u8,
        (destination_port >> 8) as u8,
        destination_port as u8,
        (sequence_number >> 24) as u8,
        (sequence_number >> 16) as u8,
        (sequence_number >> 8) as u8,
        sequence_number as u8,
        (acknowledgement_number >> 24) as u8,
        (acknowledgement_number >> 16) as u8,
        (acknowledgement_number >> 8) as u8,
        acknowledgement_number as u8,
        0x50,
        tcp_flags,
        0x04,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
    ]);
    bytes.extend_from_slice(payload);
    bytes
}
