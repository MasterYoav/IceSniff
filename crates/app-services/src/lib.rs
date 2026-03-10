use std::path::PathBuf;

use file_io::{capture_file_size, read_capture};
use parser_core::{capture_stats, inspect_metadata, inspect_packet, list_packets};
use session_model::{CaptureReport, CaptureStatsReport, PacketDetailReport, PacketListReport};

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
