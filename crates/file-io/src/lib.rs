use std::fs;
use std::path::Path;

use session_model::{
    CaptureFormat, CapturedPacket, LoadedCapture, PacketSummary, TimestampPrecision,
};

const PCAP_MAGIC_BE: [u8; 4] = [0xa1, 0xb2, 0xc3, 0xd4];
const PCAP_MAGIC_LE: [u8; 4] = [0xd4, 0xc3, 0xb2, 0xa1];
const PCAP_NANO_BE: [u8; 4] = [0xa1, 0xb2, 0x3c, 0x4d];
const PCAP_NANO_LE: [u8; 4] = [0x4d, 0x3c, 0xb2, 0xa1];
const PCAPNG_MAGIC: [u8; 4] = [0x0a, 0x0d, 0x0d, 0x0a];
const PCAPNG_BYTE_ORDER_MAGIC_BE: [u8; 4] = [0x1a, 0x2b, 0x3c, 0x4d];
const PCAPNG_BYTE_ORDER_MAGIC_LE: [u8; 4] = [0x4d, 0x3c, 0x2b, 0x1a];

pub fn capture_file_size(path: &Path) -> Result<u64, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("failed to read metadata for {}: {error}", path.display()))?;
    Ok(metadata.len())
}

pub fn read_capture(path: &Path) -> Result<LoadedCapture, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("failed to read capture file {}: {error}", path.display()))?;
    let format = detect_capture_format(&bytes);

    let packets = match format {
        CaptureFormat::Pcap => parse_pcap_records(&bytes)?,
        CaptureFormat::PcapNg => parse_pcapng_records(&bytes)?,
        CaptureFormat::Unknown => {
            return Err("cannot read an unknown capture container".to_string())
        }
    };

    Ok(LoadedCapture {
        path: path.to_path_buf(),
        format,
        packets,
    })
}

fn detect_capture_format(bytes: &[u8]) -> CaptureFormat {
    match bytes.get(..4) {
        Some(header)
            if header == PCAP_MAGIC_BE
                || header == PCAP_MAGIC_LE
                || header == PCAP_NANO_BE
                || header == PCAP_NANO_LE =>
        {
            CaptureFormat::Pcap
        }
        Some(header) if header == PCAPNG_MAGIC => CaptureFormat::PcapNg,
        _ => CaptureFormat::Unknown,
    }
}

fn parse_pcap_records(bytes: &[u8]) -> Result<Vec<CapturedPacket>, String> {
    if bytes.len() < 24 {
        return Err("pcap file is shorter than the global header".to_string());
    }

    let header = parse_pcap_header(bytes)?;
    let mut offset = 24usize;
    let mut index = 0u64;
    let mut packets = Vec::new();

    while offset < bytes.len() {
        if offset + 16 > bytes.len() {
            return Err(format!(
                "pcap packet header at offset {offset} is truncated"
            ));
        }

        let timestamp_seconds = read_u32(header.endianness, &bytes[offset..offset + 4])?;
        let timestamp_fraction = read_u32(header.endianness, &bytes[offset + 4..offset + 8])?;
        let captured_length = read_u32(header.endianness, &bytes[offset + 8..offset + 12])?;
        let original_length = read_u32(header.endianness, &bytes[offset + 12..offset + 16])?;
        offset += 16;

        let packet_end = offset
            .checked_add(captured_length as usize)
            .ok_or_else(|| "pcap packet length overflows address space".to_string())?;
        if packet_end > bytes.len() {
            return Err(format!("pcap packet {index} payload exceeds file length"));
        }

        packets.push(CapturedPacket {
            summary: PacketSummary {
                index,
                timestamp_seconds,
                timestamp_fraction,
                timestamp_precision: header.timestamp_precision.clone(),
                captured_length,
                original_length,
            },
            raw_bytes: bytes[offset..packet_end].to_vec(),
            linktype: header.linktype,
        });

        offset = packet_end;
        index += 1;
    }

    Ok(packets)
}

fn parse_pcapng_records(bytes: &[u8]) -> Result<Vec<CapturedPacket>, String> {
    if bytes.len() < 28 {
        return Err("pcapng file is shorter than the minimum section header block".to_string());
    }

    let mut offset = 0usize;
    let mut section_endianness = None;
    let mut interfaces = Vec::new();
    let mut index = 0u64;
    let mut packets = Vec::new();

    while offset + 12 <= bytes.len() {
        let raw_block_type = &bytes[offset..offset + 4];
        let endianness = if raw_block_type == PCAPNG_MAGIC {
            match bytes.get(offset + 8..offset + 12) {
                Some(raw) if raw == PCAPNG_BYTE_ORDER_MAGIC_BE => Endianness::Big,
                Some(raw) if raw == PCAPNG_BYTE_ORDER_MAGIC_LE => Endianness::Little,
                _ => {
                    return Err("pcapng section header has an invalid byte-order magic".to_string())
                }
            }
        } else {
            section_endianness.ok_or_else(|| {
                "pcapng encountered a non-section block before the section header".to_string()
            })?
        };

        let block_type = read_u32(endianness, &bytes[offset..offset + 4])?;
        let block_total_length = read_u32(endianness, &bytes[offset + 4..offset + 8])? as usize;
        if block_total_length < 12 || offset + block_total_length > bytes.len() {
            return Err(format!(
                "pcapng block at offset {offset} has an invalid length"
            ));
        }
        let trailing_length = read_u32(
            endianness,
            &bytes[offset + block_total_length - 4..offset + block_total_length],
        )? as usize;
        if trailing_length != block_total_length {
            return Err(format!(
                "pcapng block at offset {offset} has mismatched leading and trailing lengths"
            ));
        }

        let body = &bytes[offset + 8..offset + block_total_length - 4];
        match block_type {
            0x0a0d0d0a => {
                section_endianness = Some(endianness);
                interfaces.clear();
            }
            0x0000_0001 => interfaces.push(parse_pcapng_interface(body, endianness)?),
            0x0000_0006 => {
                packets.push(parse_pcapng_enhanced_packet(
                    body,
                    endianness,
                    &interfaces,
                    index,
                )?);
                index += 1;
            }
            _ => {}
        }

        offset += block_total_length;
    }

    Ok(packets)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Endianness {
    Big,
    Little,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PcapHeader {
    endianness: Endianness,
    timestamp_precision: TimestampPrecision,
    linktype: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PcapngInterface {
    linktype: u32,
    timestamp_precision: TimestampPrecision,
    timestamp_denominator: u64,
}

fn parse_pcap_header(bytes: &[u8]) -> Result<PcapHeader, String> {
    match bytes.get(..4) {
        Some(header) if header == PCAP_MAGIC_BE => Ok(PcapHeader {
            endianness: Endianness::Big,
            timestamp_precision: TimestampPrecision::Microseconds,
            linktype: read_u32(Endianness::Big, &bytes[20..24])?,
        }),
        Some(header) if header == PCAP_MAGIC_LE => Ok(PcapHeader {
            endianness: Endianness::Little,
            timestamp_precision: TimestampPrecision::Microseconds,
            linktype: read_u32(Endianness::Little, &bytes[20..24])?,
        }),
        Some(header) if header == PCAP_NANO_BE => Ok(PcapHeader {
            endianness: Endianness::Big,
            timestamp_precision: TimestampPrecision::Nanoseconds,
            linktype: read_u32(Endianness::Big, &bytes[20..24])?,
        }),
        Some(header) if header == PCAP_NANO_LE => Ok(PcapHeader {
            endianness: Endianness::Little,
            timestamp_precision: TimestampPrecision::Nanoseconds,
            linktype: read_u32(Endianness::Little, &bytes[20..24])?,
        }),
        _ => Err("unsupported or invalid pcap header".to_string()),
    }
}

fn parse_pcapng_interface(body: &[u8], endianness: Endianness) -> Result<PcapngInterface, String> {
    if body.len() < 8 {
        return Err("pcapng interface description block is truncated".to_string());
    }

    let linktype = u32::from(read_u16(endianness, &body[0..2])?);
    let mut timestamp_precision = TimestampPrecision::Microseconds;
    let mut timestamp_denominator = 1_000_000u64;
    let mut offset = 8usize;

    while offset + 4 <= body.len() {
        let option_code = read_u16(endianness, &body[offset..offset + 2])?;
        let option_length = usize::from(read_u16(endianness, &body[offset + 2..offset + 4])?);
        offset += 4;

        if option_code == 0 {
            break;
        }
        if offset + option_length > body.len() {
            return Err("pcapng interface option exceeds block length".to_string());
        }

        let value = &body[offset..offset + option_length];
        if option_code == 9 && !value.is_empty() {
            let resolution = value[0];
            if resolution & 0x80 == 0 {
                let power = u32::from(resolution);
                if power <= 9 {
                    timestamp_denominator = 10u64.pow(power);
                    timestamp_precision = if power <= 6 {
                        TimestampPrecision::Microseconds
                    } else {
                        TimestampPrecision::Nanoseconds
                    };
                }
            }
        }

        offset += padded_length(option_length);
    }

    Ok(PcapngInterface {
        linktype,
        timestamp_precision,
        timestamp_denominator,
    })
}

fn parse_pcapng_enhanced_packet(
    body: &[u8],
    endianness: Endianness,
    interfaces: &[PcapngInterface],
    index: u64,
) -> Result<CapturedPacket, String> {
    if body.len() < 20 {
        return Err("pcapng enhanced packet block is truncated".to_string());
    }

    let interface_id = read_u32(endianness, &body[0..4])? as usize;
    let interface = interfaces
        .get(interface_id)
        .ok_or_else(|| format!("pcapng packet references missing interface {interface_id}"))?;

    let timestamp_high = u64::from(read_u32(endianness, &body[4..8])?);
    let timestamp_low = u64::from(read_u32(endianness, &body[8..12])?);
    let captured_length = read_u32(endianness, &body[12..16])?;
    let original_length = read_u32(endianness, &body[16..20])?;
    let payload_start = 20usize;
    let payload_end = payload_start
        .checked_add(captured_length as usize)
        .ok_or_else(|| "pcapng packet length overflows address space".to_string())?;
    if payload_end > body.len() {
        return Err("pcapng packet payload exceeds block length".to_string());
    }

    let timestamp_units = (timestamp_high << 32) | timestamp_low;
    Ok(CapturedPacket {
        summary: PacketSummary {
            index,
            timestamp_seconds: (timestamp_units / interface.timestamp_denominator) as u32,
            timestamp_fraction: (timestamp_units % interface.timestamp_denominator) as u32,
            timestamp_precision: interface.timestamp_precision.clone(),
            captured_length,
            original_length,
        },
        raw_bytes: body[payload_start..payload_end].to_vec(),
        linktype: interface.linktype,
    })
}

fn padded_length(length: usize) -> usize {
    (4 - (length % 4)) % 4
}

fn read_u16(endianness: Endianness, bytes: &[u8]) -> Result<u16, String> {
    let bytes: [u8; 2] = bytes
        .try_into()
        .map_err(|_| "expected a 2-byte integer slice".to_string())?;
    Ok(match endianness {
        Endianness::Big => u16::from_be_bytes(bytes),
        Endianness::Little => u16::from_le_bytes(bytes),
    })
}

fn read_u32(endianness: Endianness, bytes: &[u8]) -> Result<u32, String> {
    let bytes: [u8; 4] = bytes
        .try_into()
        .map_err(|_| "expected a 4-byte integer slice".to_string())?;
    Ok(match endianness {
        Endianness::Big => u32::from_be_bytes(bytes),
        Endianness::Little => u32::from_le_bytes(bytes),
    })
}

#[cfg(test)]
mod tests {
    use super::{detect_capture_format, parse_pcap_records, parse_pcapng_records, read_capture};
    use session_model::{CaptureFormat, TimestampPrecision};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn detects_pcap() {
        assert_eq!(
            detect_capture_format(&[0xd4, 0xc3, 0xb2, 0xa1]),
            CaptureFormat::Pcap
        );
    }

    #[test]
    fn detects_pcapng() {
        assert_eq!(
            detect_capture_format(&[0x0a, 0x0d, 0x0d, 0x0a]),
            CaptureFormat::PcapNg
        );
    }

    #[test]
    fn parses_little_endian_pcap_packets() {
        let bytes = sample_pcap_bytes();
        let packets = parse_pcap_records(&bytes).unwrap();
        assert_eq!(packets.len(), 2);
        assert_eq!(packets[0].summary.index, 0);
        assert_eq!(packets[0].summary.timestamp_seconds, 1);
        assert_eq!(
            packets[0].summary.timestamp_precision,
            TimestampPrecision::Microseconds
        );
    }

    #[test]
    fn parses_pcapng_packets() {
        let bytes = sample_pcapng_bytes();
        let packets = parse_pcapng_records(&bytes).unwrap();
        assert_eq!(packets.len(), 2);
        assert_eq!(packets[0].summary.index, 0);
        assert_eq!(packets[0].summary.timestamp_seconds, 1);
    }

    #[test]
    fn reads_capture_file() {
        let path = write_temp_file("pcapng", &sample_pcapng_bytes());
        let capture = read_capture(&path).unwrap();
        fs::remove_file(&path).unwrap();
        assert_eq!(capture.format, CaptureFormat::PcapNg);
        assert_eq!(capture.packets.len(), 2);
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

    fn sample_pcapng_bytes() -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[
            0x0a, 0x0d, 0x0d, 0x0a, 0x1c, 0x00, 0x00, 0x00, 0x4d, 0x3c, 0x2b, 0x1a, 0x01, 0x00,
            0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x1c, 0x00, 0x00, 0x00,
        ]);
        bytes.extend_from_slice(&[
            0x01, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xff, 0xff,
            0x00, 0x00, 0x14, 0x00, 0x00, 0x00,
        ]);
        let first_packet = sample_udp_frame();
        bytes.extend_from_slice(&[0x06, 0x00, 0x00, 0x00]);
        bytes.extend_from_slice(&80u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&1_000_002u32.to_le_bytes());
        bytes.extend_from_slice(&(first_packet.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&(first_packet.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&first_packet);
        bytes.extend_from_slice(&[0x00, 0x00]);
        bytes.extend_from_slice(&80u32.to_le_bytes());
        bytes.extend_from_slice(&[0x06, 0x00, 0x00, 0x00]);
        bytes.extend_from_slice(&36u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&3_000_004u32.to_le_bytes());
        bytes.extend_from_slice(&3u32.to_le_bytes());
        bytes.extend_from_slice(&5u32.to_le_bytes());
        bytes.extend_from_slice(&[0x01, 0x02, 0x03, 0x00]);
        bytes.extend_from_slice(&36u32.to_le_bytes());
        bytes
    }

    fn write_temp_file(extension: &str, bytes: &[u8]) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("icesniff-test-{unique}.{extension}"));
        fs::write(&path, bytes).unwrap();
        path
    }

    fn sample_udp_frame() -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
        bytes.extend_from_slice(&[0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb]);
        bytes.extend_from_slice(&[0x08, 0x00]);
        bytes.extend_from_slice(&[
            0x45, 0x00, 0x00, 0x20, 0x12, 0x34, 0x00, 0x00, 0x40, 0x11, 0x00, 0x00, 192, 168, 1,
            10, 8, 8, 8, 8,
        ]);
        bytes.extend_from_slice(&[
            0x14, 0xe9, 0x00, 0x35, 0x00, 0x0c, 0x00, 0x00, 0xde, 0xad, 0xbe, 0xef,
        ]);
        bytes
    }
}
