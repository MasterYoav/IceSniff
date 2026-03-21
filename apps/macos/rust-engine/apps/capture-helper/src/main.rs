use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;
#[cfg(target_os = "macos")]
use std::time::Instant;

use capture_engine::CaptureEngine;

#[cfg(target_os = "macos")]
const PRIVILEGED_CHILD_ENV: &str = "ICESNIFF_CAPTURE_HELPER_PRIVILEGED_CHILD";

fn main() {
    if let Err(message) = run() {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        return Err("usage: icesniff-capture-helper <list-interfaces|start>".to_string());
    };

    match command.as_str() {
        "list-interfaces" => {
            if args.next().is_some() {
                return Err("list-interfaces does not accept extra arguments".to_string());
            }
            let engine = CaptureEngine::default();
            let interfaces = engine
                .available_interfaces()
                .map_err(|error| error.to_string())?;
            for interface in interfaces {
                println!("{}", interface.name);
            }
            Ok(())
        }
        "start" => {
            let mut interface: Option<String> = None;
            let mut output: Option<PathBuf> = None;
            let mut stop_file: Option<PathBuf> = None;

            while let Some(flag) = args.next() {
                match flag.as_str() {
                    "--interface" => interface = args.next(),
                    "--output" => output = args.next().map(PathBuf::from),
                    "--stop-file" => stop_file = args.next().map(PathBuf::from),
                    _ => return Err(format!("unknown argument: {flag}")),
                }
            }

            let interface =
                interface.ok_or_else(|| "start requires --interface <name>".to_string())?;
            let output = output.ok_or_else(|| "start requires --output <path>".to_string())?;
            let stop_file =
                stop_file.ok_or_else(|| "start requires --stop-file <path>".to_string())?;

            #[cfg(target_os = "macos")]
            {
                if env::var_os(PRIVILEGED_CHILD_ENV).is_none() {
                    return run_privileged_parent(interface, output, stop_file);
                }
            }

            run_capture_loop(interface, output, stop_file)
        }
        _ => Err(format!("unknown command: {command}")),
    }
}

fn run_capture_loop(interface: String, output: PathBuf, stop_file: PathBuf) -> Result<(), String> {
    let should_stop = Arc::new(AtomicBool::new(false));
    let signal_stop = Arc::clone(&should_stop);
    ctrlc::set_handler(move || {
        signal_stop.store(true, Ordering::Relaxed);
    })
    .map_err(|error| format!("failed to install signal handler: {error}"))?;

    let engine = CaptureEngine::default();
    let mut session = engine
        .start_capture(&interface, output.clone())
        .map_err(|error| error.to_string())?;

    println!("ready {}", output.display());

    while !should_stop.load(Ordering::Relaxed) {
        if stop_file.is_file() {
            should_stop.store(true, Ordering::Relaxed);
            break;
        }
        if !session.is_running().map_err(|error| error.to_string())? {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    session.stop().map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn run_privileged_parent(
    interface: String,
    output: PathBuf,
    stop_file: PathBuf,
) -> Result<(), String> {
    let helper = env::current_exe().map_err(|error| error.to_string())?;
    let pid_file = temp_sidecar_path(&output, "pid");
    let error_file = temp_sidecar_path(&output, "err");

    let command = format!(
        "/bin/rm -f {pid_file} {error_file}\n\
         /usr/bin/env {env_key}=1 {helper} start --interface {interface} --output {output} --stop-file {stop_file} </dev/null >/dev/null 2>{error_file} &\n\
         /bin/echo $! > {pid_file}",
        pid_file = shell_quoted(pid_file.to_string_lossy().as_ref()),
        error_file = shell_quoted(error_file.to_string_lossy().as_ref()),
        env_key = PRIVILEGED_CHILD_ENV,
        helper = shell_quoted(helper.to_string_lossy().as_ref()),
        interface = shell_quoted(&interface),
        output = shell_quoted(output.to_string_lossy().as_ref()),
        stop_file = shell_quoted(stop_file.to_string_lossy().as_ref()),
    );

    let output_result = Command::new("/usr/bin/osascript")
        .args([
            "-e",
            &format!(
                r#"do shell script "{}" with administrator privileges"#,
                apple_script_escaped(&command)
            ),
        ])
        .output()
        .map_err(|error| format!("failed to launch macOS privilege prompt: {error}"))?;

    if !output_result.status.success() {
        let stderr = String::from_utf8_lossy(&output_result.stderr)
            .trim()
            .to_string();
        let stdout = String::from_utf8_lossy(&output_result.stdout)
            .trim()
            .to_string();
        return Err(if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            "administrator approval was not granted".to_string()
        });
    }

    let deadline = Instant::now() + Duration::from_secs(3);
    let mut child_pid = None;
    while Instant::now() < deadline {
        if let Some(message) = read_non_empty_file(&error_file) {
            return Err(message);
        }
        if let Some(pid) = read_pid_file(&pid_file)? {
            child_pid = Some(pid);
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    let Some(child_pid) = child_pid else {
        return Err(read_non_empty_file(&error_file)
            .unwrap_or_else(|| "privileged capture helper failed to launch".to_string()));
    };

    let ready_deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < ready_deadline {
        if capture_file_is_ready(&output)? {
            println!("ready {}", output.display());
            break;
        }

        if let Some(message) = read_non_empty_file(&error_file) {
            return Err(message);
        }

        if !is_pid_running(child_pid)? {
            return Err(read_non_empty_file(&error_file).unwrap_or_else(|| {
                "privileged capture exited before the capture file became readable".to_string()
            }));
        }

        thread::sleep(Duration::from_millis(100));
    }

    if !capture_file_is_ready(&output)? {
        return Err(read_non_empty_file(&error_file).unwrap_or_else(|| {
            "privileged capture did not produce a readable capture file in time".to_string()
        }));
    }

    loop {
        if stop_file.is_file() {
            while is_pid_running(child_pid)? {
                thread::sleep(Duration::from_millis(100));
            }
            break;
        }

        if !is_pid_running(child_pid)? {
            break;
        }

        thread::sleep(Duration::from_millis(100));
    }

    let _ = fs::remove_file(pid_file);
    let _ = fs::remove_file(error_file);
    Ok(())
}

#[cfg(target_os = "macos")]
fn capture_file_is_ready(path: &Path) -> Result<bool, String> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(format!("failed to read capture file metadata: {error}")),
    };
    Ok(metadata.len() > 0)
}

#[cfg(target_os = "macos")]
fn temp_sidecar_path(output: &Path, extension: &str) -> PathBuf {
    let file_name = output
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("icesniff-live");
    env::temp_dir().join(format!("{file_name}.{extension}"))
}

#[cfg(target_os = "macos")]
fn read_pid_file(path: &Path) -> Result<Option<u32>, String> {
    let Ok(contents) = fs::read_to_string(path) else {
        return Ok(None);
    };
    let trimmed = contents.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed
        .parse::<u32>()
        .map(Some)
        .map_err(|error| format!("failed to parse helper pid: {error}"))
}

#[cfg(target_os = "macos")]
fn read_non_empty_file(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

#[cfg(target_os = "macos")]
fn is_pid_running(pid: u32) -> Result<bool, String> {
    let output = Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .output()
        .map_err(|error| format!("failed to probe capture pid: {error}"))?;
    if output.status.success() {
        return Ok(true);
    }

    let stderr = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
    if stderr.contains("operation not permitted") {
        return Ok(true);
    }

    Ok(false)
}

#[cfg(target_os = "macos")]
fn shell_quoted(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(target_os = "macos")]
fn apple_script_escaped(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
