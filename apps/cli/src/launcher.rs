use std::env;
use std::ffi::OsStr;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, size, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};

use crate::logo::render_icesniff_logo;

const MENU_OPTIONS: [&str; 3] = [
    "Start IceSniff Live  - web app",
    "Start IceSniff CLI tool",
    "Uninstall IceSniff  - fully remove the installed app",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LauncherOutcome {
    StartCli,
    Exit,
}

pub fn run_launcher() -> Result<LauncherOutcome, String> {
    let mut stdout = io::stdout();
    let _guard = LauncherTerminalGuard::enter(&mut stdout)?;
    let mut state = LauncherState::default();

    loop {
        draw(&mut stdout, &state)?;
        if let Event::Key(key) =
            event::read().map_err(|error| format!("failed to read launcher input: {error}"))?
        {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    if state.confirm_uninstall {
                        state.confirm_uninstall = false;
                        state.status = Some(StatusLine::info("Uninstall cancelled."));
                    } else {
                        return Ok(LauncherOutcome::Exit);
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    state.confirm_uninstall = false;
                    state.selected = state.selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    state.confirm_uninstall = false;
                    state.selected = (state.selected + 1).min(MENU_OPTIONS.len() - 1);
                }
                KeyCode::Char('1') => {
                    state.selected = 0;
                    match start_live_web() {
                        Ok(message) => {
                            state.status = Some(StatusLine::info(message));
                            return Ok(LauncherOutcome::Exit);
                        }
                        Err(message) => state.status = Some(StatusLine::error(message)),
                    }
                }
                KeyCode::Char('2') => return Ok(LauncherOutcome::StartCli),
                KeyCode::Char('3') => {
                    if state.confirm_uninstall {
                        perform_uninstall()?;
                        return Ok(LauncherOutcome::Exit);
                    }
                    state.selected = 2;
                    state.confirm_uninstall = true;
                    state.status = Some(StatusLine::error(
                        "Press Enter or 3 again to uninstall IceSniff, or Esc to cancel.",
                    ));
                }
                KeyCode::Enter => match state.selected {
                    0 => match start_live_web() {
                        Ok(message) => {
                            state.status = Some(StatusLine::info(message));
                            return Ok(LauncherOutcome::Exit);
                        }
                        Err(message) => state.status = Some(StatusLine::error(message)),
                    },
                    1 => return Ok(LauncherOutcome::StartCli),
                    2 => {
                        if state.confirm_uninstall {
                            perform_uninstall()?;
                            return Ok(LauncherOutcome::Exit);
                        }
                        state.confirm_uninstall = true;
                        state.status = Some(StatusLine::error(
                            "Press Enter again to uninstall IceSniff, or Esc to cancel.",
                        ));
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

#[derive(Default)]
struct LauncherState {
    selected: usize,
    confirm_uninstall: bool,
    status: Option<StatusLine>,
}

struct StatusLine {
    message: String,
    is_error: bool,
}

impl StatusLine {
    fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            is_error: false,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            is_error: true,
        }
    }
}

struct LauncherTerminalGuard;

impl LauncherTerminalGuard {
    fn enter(stdout: &mut io::Stdout) -> Result<Self, String> {
        enable_raw_mode().map_err(|error| format!("failed to enable raw mode: {error}"))?;
        execute!(stdout, EnterAlternateScreen, Hide)
            .map_err(|error| format!("failed to enter launcher screen: {error}"))?;
        Ok(Self)
    }
}

impl Drop for LauncherTerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, Show, LeaveAlternateScreen);
    }
}

fn draw(stdout: &mut io::Stdout, state: &LauncherState) -> Result<(), String> {
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))
        .map_err(|error| format!("failed to clear launcher screen: {error}"))?;

    let (width, _) = size().unwrap_or((120, 40));
    let content_width = width.saturating_sub(8) as usize;

    for line in render_icesniff_logo() {
        writeln!(stdout, "    {line}")
            .map_err(|error| format!("failed to draw launcher logo: {error}"))?;
    }

    writeln!(
        stdout,
        "    \x1b[38;5;250mChoose how you want to run IceSniff on this machine.\x1b[0m"
    )
    .map_err(|error| format!("failed to draw launcher text: {error}"))?;
    writeln!(stdout).map_err(|error| format!("failed to draw launcher spacer: {error}"))?;

    for (index, label) in MENU_OPTIONS.iter().enumerate() {
        let marker = if state.selected == index { ">" } else { " " };
        let color = if state.selected == index {
            "\x1b[38;2;132;245;220m"
        } else {
            "\x1b[38;5;252m"
        };
        writeln!(stdout, "    {marker} {}. {color}{label}\x1b[0m", index + 1)
            .map_err(|error| format!("failed to draw launcher option: {error}"))?;
    }

    writeln!(stdout).map_err(|error| format!("failed to draw launcher spacer: {error}"))?;
    writeln!(
        stdout,
        "    \x1b[38;5;246mUse ↑/↓ or j/k to move, Enter to select, q to quit.\x1b[0m"
    )
    .map_err(|error| format!("failed to draw launcher hint: {error}"))?;

    if let Some(status) = &state.status {
        writeln!(stdout).map_err(|error| format!("failed to draw launcher spacer: {error}"))?;
        let color = if status.is_error {
            "\x1b[38;2;255;129;129m"
        } else {
            "\x1b[38;2;132;245;220m"
        };
        for line in wrap_text(&status.message, content_width) {
            writeln!(stdout, "    {color}{line}\x1b[0m")
                .map_err(|error| format!("failed to draw launcher status: {error}"))?;
        }
    }

    stdout
        .flush()
        .map_err(|error| format!("failed to flush launcher screen: {error}"))?;
    Ok(())
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width < 8 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
            continue;
        }

        if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn resolve_bundle_root() -> Result<PathBuf, String> {
    let current_exe = env::current_exe()
        .map_err(|error| format!("failed to resolve executable path: {error}"))?;
    let parent = current_exe
        .parent()
        .ok_or_else(|| "failed to resolve launcher directory".to_string())?;

    if parent.file_name() == Some(OsStr::new("libexec")) {
        return parent
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| "failed to resolve installed bundle root".to_string());
    }

    if parent.file_name() == Some(OsStr::new("bin")) {
        return parent
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| "failed to resolve installed bundle root".to_string());
    }

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "failed to resolve repository root".to_string())?
        .to_path_buf();
    Ok(repo_root)
}

fn resolve_live_app_root(bundle_root: &Path) -> PathBuf {
    let bundled = bundle_root.join("live-app");
    if bundled.join("server.mjs").is_file() {
        return bundled;
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root")
        .join("apps/live")
}

fn resolve_cli_binary(bundle_root: &Path) -> PathBuf {
    let candidate = bundle_root.join("libexec").join(if cfg!(windows) {
        "icesniff-cli.exe"
    } else {
        "icesniff-cli"
    });
    if candidate.is_file() {
        return candidate;
    }

    env::current_exe().unwrap_or(candidate)
}

fn resolve_tshark_binary(bundle_root: &Path) -> Option<PathBuf> {
    let mac = bundle_root
        .join("runtime")
        .join("Wireshark.app")
        .join("Contents")
        .join("MacOS")
        .join(if cfg!(windows) {
            "tshark.exe"
        } else {
            "tshark"
        });
    if mac.is_file() {
        return Some(mac);
    }

    let unix = bundle_root
        .join("runtime")
        .join("wireshark")
        .join("bin")
        .join(if cfg!(windows) {
            "tshark.exe"
        } else {
            "tshark"
        });
    if unix.is_file() {
        return Some(unix);
    }

    None
}

fn resolve_node_binary() -> Option<PathBuf> {
    let executable = if cfg!(windows) { "node.exe" } else { "node" };
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths).find_map(|entry| {
            let candidate = entry.join(executable);
            candidate.is_file().then_some(candidate)
        })
    })
}

fn start_live_web() -> Result<String, String> {
    let bundle_root = resolve_bundle_root()?;
    let live_root = resolve_live_app_root(&bundle_root);
    let server_script = live_root.join("server.mjs");
    if !server_script.is_file() {
        return Err(format!(
            "IceSniff Live assets were not found at {}.",
            live_root.display()
        ));
    }

    let Some(node) = resolve_node_binary() else {
        return Err(
            "Node.js was not found in PATH, so IceSniff Live cannot start on this machine yet."
                .to_string(),
        );
    };

    let cli_binary = resolve_cli_binary(&bundle_root);
    let mut command = Command::new(node);
    command
        .arg(server_script)
        .current_dir(&live_root)
        .env("PORT", "4318")
        .env("ICESNIFF_CLI_BIN", cli_binary)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if let Some(tshark) = resolve_tshark_binary(&bundle_root) {
        command.env("ICESNIFF_TSHARK_BIN", tshark);
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }

    command
        .spawn()
        .map_err(|error| format!("failed to start IceSniff Live: {error}"))?;

    thread::sleep(Duration::from_millis(500));
    open_browser("http://127.0.0.1:4318")?;
    Ok("IceSniff Live is starting on http://127.0.0.1:4318".to_string())
}

fn open_browser(url: &str) -> Result<(), String> {
    let mut command = if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(url);
        command
    } else if cfg!(windows) {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", "", url]);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(url);
        command
    };

    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open the browser: {error}"))
}

fn perform_uninstall() -> Result<(), String> {
    if cfg!(windows) {
        perform_windows_uninstall()
    } else {
        perform_unix_uninstall()
    }
}

fn perform_unix_uninstall() -> Result<(), String> {
    let install_root = env::var("ICESNIFF_INSTALL_ROOT").map_err(|_| {
        "Uninstall is only available from an installed IceSniff launcher.".to_string()
    })?;
    let bin_root = env::var("ICESNIFF_INSTALL_BIN").map_err(|_| {
        "Uninstall is only available from an installed IceSniff launcher.".to_string()
    })?;

    let uninstall = format!(
        "sleep 1; rm -f '{bin}/icesniff' '{bin}/icesniff-cli'; rm -rf '{install}'; rmdir '{bin}' >/dev/null 2>&1 || true",
        bin = shell_single_quote(&bin_root),
        install = shell_single_quote(&install_root),
    );

    Command::new("sh")
        .arg("-c")
        .arg(uninstall)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("failed to start uninstall: {error}"))?;

    Ok(())
}

fn perform_windows_uninstall() -> Result<(), String> {
    let install_root = env::var("ICESNIFF_INSTALL_ROOT").map_err(|_| {
        "Uninstall is only available from an installed IceSniff launcher.".to_string()
    })?;
    let bin_root = env::var("ICESNIFF_INSTALL_BIN").map_err(|_| {
        "Uninstall is only available from an installed IceSniff launcher.".to_string()
    })?;
    let program_root = env::var("ICESNIFF_PROGRAM_ROOT").map_err(|_| {
        "Uninstall is only available from an installed IceSniff launcher.".to_string()
    })?;

    let ps = format!(
        "Start-Sleep -Seconds 1; $bin = '{bin}'; $install = '{install}'; $program = '{program}'; \
         $userPath = [Environment]::GetEnvironmentVariable('Path','User'); \
         if ($userPath) {{ $parts = $userPath.Split(';') | Where-Object {{ $_ -and $_ -ne $bin }}; [Environment]::SetEnvironmentVariable('Path', ($parts -join ';'), 'User') }}; \
         Remove-Item -Force -ErrorAction SilentlyContinue \"$bin\\icesniff.cmd\",\"$bin\\icesniff-cli.cmd\"; \
         Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $install; \
         Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $bin; \
         Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $program",
        bin = powershell_single_quote(&bin_root),
        install = powershell_single_quote(&install_root),
        program = powershell_single_quote(&program_root),
    );

    Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("failed to start uninstall: {error}"))?;

    Ok(())
}

fn shell_single_quote(value: &str) -> String {
    value.replace('\'', "'\"'\"'")
}

fn powershell_single_quote(value: &str) -> String {
    value.replace('\'', "''")
}
