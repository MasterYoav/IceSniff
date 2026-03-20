use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
    child: Child,
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
        self.child
            .try_wait()
            .map(|status| status.is_none())
            .map_err(|error| CaptureError::ProcessPollFailed(error.to_string()))
    }

    pub fn stop(mut self) -> Result<PathBuf, CaptureError> {
        stop_capture_process(&mut self.child, &self.info.path)?;
        Ok(self.info.path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureEngine {
    tool_path: String,
    backend: CaptureBackend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureBackend {
    TcpdumpStyle,
    Dumpcap,
}

impl Default for CaptureEngine {
    fn default() -> Self {
        let tool_path = resolve_capture_tool_path();
        Self {
            backend: resolve_capture_backend(&tool_path),
            tool_path,
        }
    }
}

impl CaptureEngine {
    pub fn new(tool_path: impl Into<String>) -> Self {
        let tool_path = tool_path.into();
        Self {
            backend: resolve_capture_backend(&tool_path),
            tool_path,
        }
    }

    pub fn with_backend(tool_path: impl Into<String>, backend: CaptureBackend) -> Self {
        Self {
            tool_path: tool_path.into(),
            backend,
        }
    }

    pub fn tool_path(&self) -> &str {
        &self.tool_path
    }

    pub fn backend(&self) -> CaptureBackend {
        self.backend
    }

    pub fn available_interfaces(&self) -> Result<Vec<CaptureInterface>, CaptureError> {
        let output = Command::new(&self.tool_path)
            .args(self.backend.interface_list_args())
            .output()
            .map_err(|error| {
                map_spawn_error(&self.tool_path, error, SpawnContext::InterfaceEnumeration)
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(map_process_failure(
                "failed to enumerate capture interfaces".to_string(),
                stderr.trim().to_string(),
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(parse_capture_interface_line)
            .collect())
    }

    pub fn default_interface(&self) -> Result<CaptureInterface, CaptureError> {
        let interfaces = self.available_interfaces()?;
        preferred_capture_interface(&interfaces)
            .cloned()
            .ok_or(CaptureError::NoInterfacesAvailable)
    }

    pub fn start_capture(
        &self,
        interface: &str,
        output_path: PathBuf,
    ) -> Result<ActiveCaptureSession, CaptureError> {
        let args = self.backend.start_capture_args(interface, &output_path);
        let mut child = Command::new(&self.tool_path)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| map_spawn_error(&self.tool_path, error, SpawnContext::StartCapture))?;

        thread::sleep(Duration::from_millis(120));
        if child
            .try_wait()
            .map_err(|error| CaptureError::ProcessPollFailed(error.to_string()))?
            .is_some()
        {
            let output = child
                .wait_with_output()
                .map_err(|error| CaptureError::StopFailed(error.to_string()))?;
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(map_process_failure(
                format!("failed to start capture on {interface}"),
                stderr,
            ));
        }

        Ok(ActiveCaptureSession {
            info: CaptureSessionInfo {
                interface: interface.to_string(),
                path: output_path,
            },
            child,
        })
    }
}

impl CaptureBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            CaptureBackend::TcpdumpStyle => "tcpdump",
            CaptureBackend::Dumpcap => "dumpcap",
        }
    }

    fn interface_list_args(self) -> [&'static str; 1] {
        match self {
            CaptureBackend::TcpdumpStyle | CaptureBackend::Dumpcap => ["-D"],
        }
    }

    fn start_capture_args(self, interface: &str, output_path: &Path) -> Vec<String> {
        match self {
            CaptureBackend::TcpdumpStyle => vec![
                "-i".to_string(),
                interface.to_string(),
                "-U".to_string(),
                "-w".to_string(),
                output_path.display().to_string(),
            ],
            CaptureBackend::Dumpcap => vec![
                "-i".to_string(),
                interface.to_string(),
                "-P".to_string(),
                "-w".to_string(),
                output_path.display().to_string(),
            ],
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
    env::temp_dir().join(format!("icesniff-live-{nanos}.{extension}"))
}

pub fn parse_capture_interface_line(line: &str) -> Option<CaptureInterface> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(dot_index) = trimmed.find('.') {
        let prefix = trimmed.get(..dot_index)?.trim();
        if prefix.chars().all(|ch| ch.is_ascii_digit()) {
            let remainder = trimmed.get(dot_index + 1..)?.trim();
            let name = remainder
                .split_whitespace()
                .next()
                .filter(|value| !value.is_empty())?;
            return Some(CaptureInterface {
                name: name.to_string(),
            });
        }
    }

    trimmed
        .split_whitespace()
        .next()
        .filter(|value| !value.is_empty())
        .map(|name| CaptureInterface {
            name: name.to_string(),
        })
}

fn resolve_capture_tool_path() -> String {
    if let Ok(path) = env::var("ICESNIFF_CAPTURE_TOOL") {
        if !path.trim().is_empty() {
            return path;
        }
    }

    for candidate in bundled_capture_tool_candidates() {
        if tool_candidate_exists(&candidate) {
            return candidate;
        }
    }

    for candidate in default_capture_tool_candidates() {
        if tool_candidate_exists(candidate) {
            return (*candidate).to_string();
        }
    }

    default_capture_tool_candidates()[0].to_string()
}

fn bundled_capture_tool_candidates() -> Vec<String> {
    let mut candidates = Vec::new();
    let mut seen = std::collections::BTreeSet::new();

    if let Ok(runtime_root) = env::var("ICESNIFF_RUNTIME_ROOT") {
        push_runtime_candidates(Path::new(runtime_root.trim()), &mut candidates, &mut seen);
    }

    if let Ok(executable) = env::current_exe() {
        if let Some(executable_dir) = executable.parent() {
            push_runtime_candidates(executable_dir, &mut candidates, &mut seen);
            push_runtime_candidates(&executable_dir.join("runtime"), &mut candidates, &mut seen);
            push_runtime_candidates(
                &executable_dir.join("..").join("runtime"),
                &mut candidates,
                &mut seen,
            );
        }
    }

    if let Ok(app_path) = env::var("ICESNIFF_WIRESHARK_APP") {
        let trimmed = app_path.trim();
        if !trimmed.is_empty() {
            push_runtime_candidates(Path::new(trimmed), &mut candidates, &mut seen);
        }
    }

    if let Ok(repo_root) = env::var("ICESNIFF_REPO_ROOT") {
        let bundled_root = PathBuf::from(repo_root)
            .join("apps")
            .join("macos")
            .join("Sources")
            .join("IceSniffMac")
            .join("Resources")
            .join("BundledTShark")
            .join("Wireshark.app");
        push_runtime_candidates(&bundled_root, &mut candidates, &mut seen);
    }

    candidates
}

fn push_runtime_candidates(
    root: &Path,
    candidates: &mut Vec<String>,
    seen: &mut std::collections::BTreeSet<String>,
) {
    let runtime_roots = [
        root.to_path_buf(),
        root.join("bin"),
        root.join("wireshark"),
        root.join("wireshark").join("bin"),
        root.join("Wireshark.app"),
        root.join("Contents").join("MacOS"),
        root.join("Wireshark.app").join("Contents").join("MacOS"),
    ];

    for runtime_root in runtime_roots {
        for tool in ["dumpcap", "dumpcap.exe", "tcpdump", "tcpdump.exe"] {
            let candidate = runtime_root.join(tool).display().to_string();
            if seen.insert(candidate.clone()) {
                candidates.push(candidate);
            }
        }
    }
}

fn resolve_capture_backend(tool_path: &str) -> CaptureBackend {
    if let Ok(value) = env::var("ICESNIFF_CAPTURE_BACKEND") {
        if let Some(parsed) = parse_capture_backend_name(&value) {
            return parsed;
        }
    }
    infer_capture_backend_from_tool_path(tool_path)
}

fn parse_capture_backend_name(value: &str) -> Option<CaptureBackend> {
    match value.trim().to_ascii_lowercase().as_str() {
        "tcpdump" | "tcpdump-style" => Some(CaptureBackend::TcpdumpStyle),
        "dumpcap" => Some(CaptureBackend::Dumpcap),
        _ => None,
    }
}

fn preferred_capture_interface(interfaces: &[CaptureInterface]) -> Option<&CaptureInterface> {
    for preferred in ["en0", "eth0", "wlan0", "wlp0s20f3", "wlp2s0"] {
        if let Some(interface) = interfaces
            .iter()
            .find(|interface| interface.name == preferred)
        {
            return Some(interface);
        }
    }

    if let Some(interface) = interfaces.iter().find(|interface| {
        !matches!(
            interface.name.as_str(),
            "any" | "lo" | "lo0" | "Loopback" | "Npcap Loopback Adapter"
        )
    }) {
        return Some(interface);
    }

    interfaces
        .iter()
        .find(|interface| interface.name == "en0")
        .or_else(|| interfaces.first())
}

fn infer_capture_backend_from_tool_path(tool_path: &str) -> CaptureBackend {
    let normalized = Path::new(tool_path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(tool_path)
        .to_ascii_lowercase();
    if normalized.contains("dumpcap") {
        CaptureBackend::Dumpcap
    } else {
        CaptureBackend::TcpdumpStyle
    }
}

fn stop_capture_process(child: &mut Child, output_path: &Path) -> Result<(), CaptureError> {
    for _ in 0..5 {
        if child
            .try_wait()
            .map_err(|error| CaptureError::ProcessPollFailed(error.to_string()))?
            .is_some()
        {
            if fs::metadata(output_path).is_ok() {
                return Ok(());
            }
            return Err(CaptureError::StopFailed(
                "capture ended without a readable capture file".to_string(),
            ));
        }
        if fs::metadata(output_path).is_ok() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(50));
    }

    interrupt_capture_process(child)?;
    let status = child
        .wait()
        .map_err(|error| CaptureError::StopFailed(error.to_string()))?;
    if !status.success() && fs::metadata(output_path).is_err() {
        return Err(CaptureError::StopFailed(
            "capture ended without a readable capture file".to_string(),
        ));
    }

    Ok(())
}

#[cfg(unix)]
fn interrupt_capture_process(child: &mut Child) -> Result<(), CaptureError> {
    let status = Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .map_err(|error| CaptureError::StopFailed(error.to_string()))?;
    if !status.success() {
        return Err(CaptureError::StopFailed(
            "failed to send interrupt to capture process".to_string(),
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn interrupt_capture_process(child: &mut Child) -> Result<(), CaptureError> {
    child
        .kill()
        .map_err(|error| CaptureError::StopFailed(error.to_string()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpawnContext {
    InterfaceEnumeration,
    StartCapture,
}

fn map_spawn_error(tool_path: &str, error: std::io::Error, context: SpawnContext) -> CaptureError {
    match error.kind() {
        std::io::ErrorKind::NotFound => CaptureError::ToolUnavailable(format!(
            "tool `{tool_path}` was not found (set ICESNIFF_CAPTURE_TOOL to override)"
        )),
        std::io::ErrorKind::PermissionDenied => CaptureError::PermissionDenied(error.to_string()),
        _ => match context {
            SpawnContext::InterfaceEnumeration => {
                CaptureError::InterfaceEnumerationFailed(error.to_string())
            }
            SpawnContext::StartCapture => CaptureError::StartFailed(error.to_string()),
        },
    }
}

fn map_process_failure(context: String, stderr: String) -> CaptureError {
    if is_permission_error(&stderr) {
        return CaptureError::PermissionDenied(stderr);
    }

    if is_driver_error(&stderr) {
        return CaptureError::DriverUnavailable(stderr);
    }

    if context.contains("enumerate") {
        return CaptureError::InterfaceEnumerationFailed(if stderr.is_empty() {
            context
        } else {
            format!("{context}: {stderr}")
        });
    }

    CaptureError::StartFailed(if stderr.is_empty() {
        context
    } else {
        format!("{context}: {stderr}")
    })
}

fn default_capture_tool_candidates() -> &'static [&'static str] {
    #[cfg(windows)]
    {
        &[
            "dumpcap.exe",
            "dumpcap",
            "windump.exe",
            "windump",
            "tcpdump.exe",
            "tcpdump",
        ]
    }
    #[cfg(not(windows))]
    {
        &[
            "/Applications/Wireshark.app/Contents/MacOS/dumpcap",
            "/opt/homebrew/bin/dumpcap",
            "/usr/local/bin/dumpcap",
            "/usr/bin/dumpcap",
            "dumpcap",
            "/usr/sbin/tcpdump",
            "tcpdump",
        ]
    }
}

fn tool_candidate_exists(candidate: &str) -> bool {
    if candidate.contains(std::path::MAIN_SEPARATOR) {
        return Path::new(candidate).is_file();
    }

    env::var_os("PATH")
        .map(|paths| {
            env::split_paths(&paths).any(|directory| {
                let direct = directory.join(candidate);
                if direct.is_file() {
                    return true;
                }

                #[cfg(windows)]
                {
                    if Path::new(candidate).extension().is_none() {
                        for extension in ["exe", "bat", "cmd"] {
                            if directory.join(format!("{candidate}.{extension}")).is_file() {
                                return true;
                            }
                        }
                    }
                }

                false
            })
        })
        .unwrap_or(false)
}

fn is_permission_error(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    normalized.contains("permission denied")
        || normalized.contains("operation not permitted")
        || normalized.contains("you don't have permission")
        || normalized.contains("access is denied")
}

fn is_driver_error(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    normalized.contains("npcap")
        || normalized.contains("npf driver")
        || normalized.contains("packet.dll")
        || normalized.contains("wpcap.dll")
        || (normalized.contains("no such file or directory") && normalized.contains("libpcap"))
        || (normalized.contains("could not load") && normalized.contains("libpcap"))
        || (normalized.contains("loading shared libraries") && normalized.contains("libpcap"))
        || (normalized.contains("cannot open shared object file") && normalized.contains("libpcap"))
}

#[cfg(test)]
mod tests {
    use super::{
        infer_capture_backend_from_tool_path, is_driver_error, is_permission_error,
        parse_capture_backend_name, parse_capture_interface_line, preferred_capture_interface,
        CaptureBackend, CaptureInterface,
    };

    #[test]
    fn parses_numeric_interface_lines() {
        let line = "1. en0 [Up, Running]";
        let parsed = parse_capture_interface_line(line).expect("expected interface");
        assert_eq!(parsed.name, "en0");
    }

    #[test]
    fn parses_numeric_interface_lines_without_space_after_dot() {
        let line = "2.eth0 [Up, Running]";
        let parsed = parse_capture_interface_line(line).expect("expected interface");
        assert_eq!(parsed.name, "eth0");
    }

    #[test]
    fn parses_fallback_interface_lines() {
        let line = "wlan0 up running";
        let parsed = parse_capture_interface_line(line).expect("expected interface");
        assert_eq!(parsed.name, "wlan0");
    }

    #[test]
    fn parses_windows_style_interface_lines() {
        let line = "1. \\Device\\NPF_{E1E92BD7-ABCD-0123-4567-1234567890AB} (Intel Ethernet)";
        let parsed = parse_capture_interface_line(line).expect("expected interface");
        assert_eq!(
            parsed.name,
            "\\Device\\NPF_{E1E92BD7-ABCD-0123-4567-1234567890AB}"
        );
    }

    #[test]
    fn detects_permission_errors() {
        assert!(is_permission_error(
            "You don't have permission to capture on that device"
        ));
        assert!(is_permission_error("Operation not permitted"));
        assert!(is_permission_error("Access is denied."));
        assert!(!is_permission_error("unsupported argument"));
    }

    #[test]
    fn detects_driver_errors() {
        assert!(is_driver_error("The NPF driver isn't running."));
        assert!(is_driver_error("Npcap not installed"));
        assert!(is_driver_error(
            "error while loading shared libraries: libpcap.so: cannot open shared object file"
        ));
        assert!(is_driver_error("wpcap.dll is missing"));
        assert!(!is_driver_error("permission denied"));
    }

    #[test]
    fn parses_backend_names() {
        assert_eq!(
            parse_capture_backend_name("tcpdump-style"),
            Some(CaptureBackend::TcpdumpStyle)
        );
        assert_eq!(
            parse_capture_backend_name("dumpcap"),
            Some(CaptureBackend::Dumpcap)
        );
        assert_eq!(parse_capture_backend_name("unknown"), None);
    }

    #[test]
    fn infers_backend_from_tool_name() {
        assert_eq!(
            infer_capture_backend_from_tool_path("/usr/bin/dumpcap"),
            CaptureBackend::Dumpcap
        );
        assert_eq!(
            infer_capture_backend_from_tool_path("tcpdump"),
            CaptureBackend::TcpdumpStyle
        );
    }

    #[test]
    fn prefers_en0_as_default_interface() {
        let interfaces = vec![
            CaptureInterface {
                name: "lo0".to_string(),
            },
            CaptureInterface {
                name: "en0".to_string(),
            },
            CaptureInterface {
                name: "bridge0".to_string(),
            },
        ];
        let preferred = preferred_capture_interface(&interfaces).expect("expected interface");
        assert_eq!(preferred.name, "en0");
    }
}
