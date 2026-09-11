use anyhow::Result;
use clap::Parser;
use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

mod cli;
mod mock;
mod probe;
mod tui;

use cli::Cli;
use probe::detector::{get_connected_probes, print_probes};
use probe::targets::{print_search_results, search_targets, validate_target};
use probe::worker::{start_rtt_worker, RttCommand, RttEvent};
use tui::app::AppState;

fn main() -> Result<()> {
    let args = Cli::parse();

    if args.list_probes {
        let probes = get_connected_probes()?;
        print_probes(&probes);
        return Ok(());
    }

    if let Some(query) = args.search_chip {
        let results = search_targets(&query, 15)?;
        print_search_results(&query, &results);
        return Ok(());
    }

    // 1. Mock Mode (Simulated TUI)
    if args.mock {
        let state = AppState::new(
            "Mock-JLink (SN: 99999999)".to_string(),
            "nRF52840_xxAA [Simulated]".to_string(),
            args.channel,
            args.eol,
        );
        let worker = mock::start_mock_worker()?;
        let terminal = tui::init_terminal()?;
        let res = tui::run_tui(terminal, state, worker);
        tui::restore_terminal()?;
        return res;
    }

    // 2. Hardware Connected Mode
    if let Some(ref chip_name) = args.chip {
        let target = validate_target(chip_name)?;

        // Headless Mode (Console only)
        if args.headless {
            return run_headless(args.clone(), chip_name.clone(), target.cores.len());
        }

        // Default: Full Interactive TUI
        let state = AppState::new(
            "J-Link".to_string(),
            target.name.clone(),
            args.channel,
            args.eol,
        );

        let worker = start_rtt_worker(
            args.probe.clone(),
            chip_name.clone(),
            args.speed,
            args.channel,
        )?;

        let terminal = tui::init_terminal()?;
        let res = tui::run_tui(terminal, state, worker);
        tui::restore_terminal()?;
        return res;
    }

    // Default welcome message when no arguments are passed
    println!("rtt-terminal v{}", env!("CARGO_PKG_VERSION"));
    println!("High-performance interactive RTT terminal for microcontrollers.");
    println!();

    let probes = get_connected_probes()?;
    print_probes(&probes);

    println!();
    println!("Usage examples:");
    println!("  rtt-terminal --mock                      # Launch interactive TUI in simulated mock mode");
    println!("  rtt-terminal --chip nRF52840_xxAA        # Launch interactive TUI connected to target");
    println!("  rtt-terminal --chip nRF52840_xxAA --headless # Stream directly in console without TUI");
    println!("  rtt-terminal --list-probes               # List connected probes");
    println!("  rtt-terminal --search-chip <query>       # Search target chips (fuzzy match)");
    println!("  rtt-terminal --help                      # Show all CLI options");

    Ok(())
}

fn run_headless(args: Cli, chip_name: String, core_count: usize) -> Result<()> {
    println!("Target confirmed: {} ({} core(s))", chip_name, core_count);

    let eol_bytes = match args.eol.to_lowercase().as_str() {
        "crlf" => b"\r\n".as_slice(),
        "lf" => b"\n".as_slice(),
        "cr" => b"\r".as_slice(),
        "none" => b"".as_slice(),
        other => {
            eprintln!("Warning: unknown EOL '{}', defaulting to CRLF", other);
            b"\r\n".as_slice()
        }
    };

    println!("Starting RTT session for '{}' at {} kHz...", chip_name, args.speed);
    println!("Connecting via probe {:?}...", args.probe.as_deref().unwrap_or("auto"));

    let worker = start_rtt_worker(
        args.probe.clone(),
        chip_name.clone(),
        args.speed,
        args.channel,
    )?;

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    let cmd_tx_ctrlc = worker.cmd_tx.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
        let _ = cmd_tx_ctrlc.send(RttCommand::Stop);
    }).expect("Error setting Ctrl-C handler");

    // Stdin reader thread
    let cmd_tx_stdin = worker.cmd_tx.clone();
    let running_stdin = running.clone();
    let channel = args.channel;
    let eol_vec = eol_bytes.to_vec();

    thread::spawn(move || {
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        let mut line = String::new();

        while running_stdin.load(Ordering::SeqCst) {
            line.clear();
            match handle.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    let trimmed = line.trim_end_matches(&['\r', '\n'][..]);
                    let mut bytes_to_send = trimmed.as_bytes().to_vec();
                    bytes_to_send.extend_from_slice(&eol_vec);
                    let _ = cmd_tx_stdin.send(RttCommand::Write(channel, bytes_to_send));
                }
                Err(_) => break,
            }
        }
    });

    let mut attached = false;
    while running.load(Ordering::SeqCst) {
        match worker.event_rx.recv() {
            Ok(RttEvent::Connected { probe_name, target_name }) => {
                println!("Debug probe connected: {} -> {}", probe_name, target_name);
                println!("Searching for _SEGGER_RTT control block in RAM...");
            }
            Ok(RttEvent::Attached { up_channels, down_channels }) => {
                attached = true;
                println!("RTT Control Block successfully located in RAM!");
                println!("  Up Channels ({}):", up_channels.len());
                for ch in &up_channels {
                    println!("    [{}] \"{}\" (buffer: {} B)", ch.number, ch.name, ch.buffer_size);
                }
                println!("  Down Channels ({}):", down_channels.len());
                for ch in &down_channels {
                    println!("    [{}] \"{}\" (buffer: {} B)", ch.number, ch.name, ch.buffer_size);
                }
                println!("{:-<75}", "");
                println!("RTT Streaming started on Channel {}. Type commands + Enter to send.", args.channel);
                println!("Press Ctrl+C to terminate session.");
                println!("{:-<75}", "");
            }
            Ok(RttEvent::Data { channel, data }) => {
                if channel == args.channel || !attached {
                    let mut stdout = io::stdout().lock();
                    let _ = stdout.write_all(&data);
                    let _ = stdout.flush();
                }
            }
            Ok(RttEvent::Warning(w)) => {
                eprintln!("\n[RTT Warning] {}", w);
            }
            Ok(RttEvent::Error(e)) => {
                eprintln!("\n[RTT Error] {}", e);
                break;
            }
            Ok(RttEvent::Stopped) => {
                break;
            }
            Err(_) => {
                break;
            }
        }
    }

    println!("\nSession terminated. Cleaning up probe connection...");
    let _ = worker.cmd_tx.send(RttCommand::Stop);
    let _ = worker.thread_handle.join();
    println!("Done. Disconnected cleanly.");
    Ok(())
}
