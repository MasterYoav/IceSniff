use std::fmt;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::{SystemTime, UNIX_EPOCH};

use pcap::{Capture, Device, Error as PcapError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureInterface {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureSessionInfo {
    pub interface: String,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct ActiveCaptureSession {
    info: CaptureSessionInfo,
    stop_flag: Arc<AtomicBool>,
    finished_flag: Arc<AtomicBool>,
    join_handle: Option<JoinHandle<Result<(), CaptureError>>>,
}

impl ActiveCaptureSession {
    pub fn interface(&self) -> &str {
        &self.info.interface
    }

    pub fn path(&self) -> &Path {
        &self.info.path
    }

    pub fn info(&self) -> CaptureSessionInfo {
        self.info.clone()
    }

    pub fn is_running(&mut self) -> Result<bool, CaptureError> {
        if self.finished_flag.load(Ordering::Relaxed) {
            self.join_finished_capture_thread()?;
            return Ok(false);
        }

        Ok(match &self.join_handle {
            Some(handle) => !handle.is_finished(),
            None => false,
        })
    }

    pub fn stop(mut self) -> Result<PathBuf, CaptureError> {
        self.stop_flag.store(true, Ordering::Relaxed);
        self.join_finished_capture_thread()?;
        Ok(self.info.path)
    }

    fn join_finished_capture_thread(&mut self) -> Result<(), CaptureError> {
        let Some(handle) = self.join_handle.take() else {
            return Ok(());
        };

        match handle.join() {
            Ok(result) => result,
            Err(_) => Err(CaptureError::StopFailed(
                "capture worker thread panicked".to_string(),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CaptureEngine;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureBackend {
    Libpcap,
}

impl CaptureEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn with_backend(_backend: CaptureBackend) -> Self {
        Self
    }

    pub fn tool_path(&self) -> &str {
        "libpcap"
    }

    pub fn backend(&self) -> CaptureBackend {
        CaptureBackend::Libpcap
    }

    pub fn available_interfaces(&self) -> Result<Vec<CaptureInterface>, CaptureError> {
        let interfaces = Device::list()
            .map_err(|error| map_pcap_error(error, CaptureAction::EnumerateInterfaces))?;

        Ok(interfaces
            .into_iter()
            .map(|device| CaptureInterface { name: device.name })
            .collect())
    }

    pub fn default_interface(&self) -> Result<CaptureInterface, CaptureError> {
        let interfaces = self.available_interfaces()?;
        interfaces
            .into_iter()
            .find(|interface| !interface.name.starts_with("lo"))
            .or_else(|| self.available_interfaces().ok()?.into_iter().next())
            .ok_or(CaptureError::NoInterfacesAvailable)
    }

    pub fn start_capture(
        &self,
        interface: &str,
        output_path: PathBuf,
    ) -> Result<ActiveCaptureSession, CaptureError> {
        prepare_output_path(&output_path)
            .map_err(|error| CaptureError::StartFailed(error.to_string()))?;

        let device_name = interface.to_string();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let finished_flag = Arc::new(AtomicBool::new(false));
        let worker_stop_flag = Arc::clone(&stop_flag);
        let worker_finished_flag = Arc::clone(&finished_flag);
        let worker_output_path = output_path.clone();
        let worker_interface = device_name.clone();

        let join_handle = thread::spawn(move || {
            let result = run_capture_loop(&worker_interface, &worker_output_path, worker_stop_flag);
            worker_finished_flag.store(true, Ordering::Relaxed);
            result
        });

        Ok(ActiveCaptureSession {
            info: CaptureSessionInfo {
                interface: device_name,
                path: output_path,
            },
            stop_flag,
            finished_flag,
            join_handle: Some(join_handle),
        })
    }
}

impl CaptureBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            CaptureBackend::Libpcap => "libpcap",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureError {
    ToolUnavailable(String),
    PermissionDenied(String),
    DriverUnavailable(String),
    InterfaceEnumerationFailed(String),
    NoInterfacesAvailable,
    StartFailed(String),
    StopFailed(String),
    ProcessPollFailed(String),
}

impl fmt::Display for CaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ToolUnavailable(message) => write!(f, "capture tool unavailable: {message}"),
            Self::PermissionDenied(message) => write!(f, "capture permission denied: {message}"),
            Self::DriverUnavailable(message) => {
                write!(f, "capture backend unavailable: {message}")
            }
            Self::InterfaceEnumerationFailed(message) => {
                write!(f, "capture interface enumeration failed: {message}")
            }
            Self::NoInterfacesAvailable => write!(f, "no capture interfaces are available"),
            Self::StartFailed(message) => write!(f, "capture start failed: {message}"),
            Self::StopFailed(message) => write!(f, "capture stop failed: {message}"),
            Self::ProcessPollFailed(message) => write!(f, "capture process poll failed: {message}"),
        }
    }
}

impl std::error::Error for CaptureError {}

pub fn temp_capture_path(extension: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("icesniff-live-{nanos}.{extension}"))
}

fn prepare_output_path(output_path: &Path) -> std::io::Result<()> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let _ = fs::remove_file(output_path);
    let _ = fs::File::create(output_path)?;

    #[cfg(unix)]
    fs::set_permissions(output_path, fs::Permissions::from_mode(0o644))?;

    Ok(())
}

fn run_capture_loop(
    interface: &str,
    output_path: &Path,
    stop_flag: Arc<AtomicBool>,
) -> Result<(), CaptureError> {
    let inactive = Capture::from_device(interface)
        .map_err(|error| map_pcap_error(error, CaptureAction::StartCapture))?;
    let mut capture = inactive
        .immediate_mode(true)
        .timeout(250)
        .open()
        .map_err(|error| map_pcap_error(error, CaptureAction::StartCapture))?;

    let mut savefile = capture
        .savefile(output_path)
        .map_err(|error| map_pcap_error(error, CaptureAction::StartCapture))?;
    savefile
        .flush()
        .map_err(|error| map_pcap_error(error, CaptureAction::StartCapture))?;

    #[cfg(unix)]
    let _ = fs::set_permissions(output_path, fs::Permissions::from_mode(0o644));

    while !stop_flag.load(Ordering::Relaxed) {
        match capture.next_packet() {
            Ok(packet) => {
                savefile.write(&packet);
                savefile
                    .flush()
                    .map_err(|error| map_pcap_error(error, CaptureAction::ReadPacket))?;
            }
            Err(PcapError::TimeoutExpired) => continue,
            Err(error) => return Err(map_pcap_error(error, CaptureAction::ReadPacket)),
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaptureAction {
    EnumerateInterfaces,
    StartCapture,
    ReadPacket,
}

fn map_pcap_error(error: PcapError, action: CaptureAction) -> CaptureError {
    let message = error.to_string();
    let normalized = message.to_ascii_lowercase();

    if normalized.contains("permission denied")
        || normalized.contains("operation not permitted")
        || normalized.contains("/dev/bpf")
    {
        return CaptureError::PermissionDenied(message);
    }

    if normalized.contains("interface") || normalized.contains("device") {
        return match action {
            CaptureAction::EnumerateInterfaces => CaptureError::InterfaceEnumerationFailed(message),
            _ => CaptureError::StartFailed(message),
        };
    }

    if normalized.contains("libpcap")
        || normalized.contains("pcap")
        || normalized.contains("datalink")
    {
        return CaptureError::DriverUnavailable(message);
    }

    match action {
        CaptureAction::EnumerateInterfaces => CaptureError::InterfaceEnumerationFailed(message),
        CaptureAction::StartCapture | CaptureAction::ReadPacket => {
            CaptureError::StartFailed(message)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CaptureBackend, CaptureEngine};

    #[test]
    fn reports_libpcap_backend() {
        let engine = CaptureEngine::default();
        assert_eq!(engine.backend(), CaptureBackend::Libpcap);
        assert_eq!(engine.tool_path(), "libpcap");
    }
}
