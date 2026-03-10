use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn write_sample_pcap() -> String {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("icesniff-cli-{unique}.pcap"));
    fs::write(&path, sample_pcap_bytes()).expect("failed to write sample pcap");
    path.display().to_string()
}

fn write_sample_pcapng() -> String {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("icesniff-cli-{unique}.pcapng"));
    fs::write(&path, sample_pcapng_bytes()).expect("failed to write sample pcapng");
    path.display().to_string()
}

fn write_temp_capture(extension: &str, bytes: &[u8]) -> String {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("icesniff-cli-{unique}.{extension}"));
    fs::write(&path, bytes).expect("failed to write sample capture");
    path.display().to_string()
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
fn streams_command_reports_http_transaction_summary() {
    let capture = wrap_pcap_packets(&[sample_http_frame(), sample_http_response_frame()]);
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
    assert!(stdout.contains("\"notes\":[\"Transaction counts reflect per-packet HTTP messages without TCP reassembly.\"]"));
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
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    bytes.extend_from_slice(&[0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb]);
    bytes.extend_from_slice(&[0x08, 0x00]);
    let http_payload = b"GET /hello HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let tcp_length = 20 + http_payload.len() as u16;
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
        0xc3, 0x50, 0x00, 0x50, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x50, 0x18, 0x04,
        0x00, 0x00, 0x00, 0x00, 0x00,
    ]);
    bytes.extend_from_slice(http_payload);
    bytes
}

fn sample_http_response_frame() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&[0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb]);
    bytes.extend_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    bytes.extend_from_slice(&[0x08, 0x00]);
    let http_payload = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello";
    let tcp_length = 20 + http_payload.len() as u16;
    let total_length = 20 + tcp_length;
    bytes.extend_from_slice(&[
        0x45,
        0x00,
        (total_length >> 8) as u8,
        total_length as u8,
        0xab,
        0xcf,
        0x00,
        0x00,
        0x40,
        0x06,
        0x00,
        0x00,
        93,
        184,
        216,
        34,
        10,
        0,
        0,
        1,
    ]);
    bytes.extend_from_slice(&[
        0x00, 0x50, 0xc3, 0x50, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x50, 0x18, 0x04,
        0x00, 0x00, 0x00, 0x00, 0x00,
    ]);
    bytes.extend_from_slice(http_payload);
    bytes
}
