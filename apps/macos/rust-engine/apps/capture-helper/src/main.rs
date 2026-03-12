use std::env;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

use capture_engine::CaptureEngine;

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
            let interfaces = engine.available_interfaces().map_err(|error| error.to_string())?;
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
                if let Some(stop_file) = &stop_file {
                    if stop_file.is_file() {
                        should_stop.store(true, Ordering::Relaxed);
                        break;
                    }
                }
                if !session.is_running().map_err(|error| error.to_string())? {
                    break;
                }
                thread::sleep(Duration::from_millis(100));
            }

            session.stop().map_err(|error| error.to_string())?;
            Ok(())
        }
        _ => Err(format!("unknown command: {command}")),
    }
}
