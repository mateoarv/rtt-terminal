pub mod app;
pub mod events;
pub mod ui;

use anyhow::Result;
use crossterm::event::EnableMouseCapture;
use crossterm::event::DisableMouseCapture;
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{stdout, Stdout};
use std::time::Duration;

use self::app::AppState;
use crate::probe::worker::{RttCommand, RttEvent, RttWorkerHandle};

pub fn init_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;

    // Set panic hook to cleanly restore terminal if an unhandled panic occurs
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal();
        original_hook(panic_info);
    }));

    Ok(terminal)
}

pub fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

pub fn run_tui(
    mut terminal: Terminal<CrosstermBackend<Stdout>>,
    mut state: AppState,
    worker: RttWorkerHandle,
) -> Result<()> {
    while !state.should_quit {
        // 1. Render frame
        terminal.draw(|f| ui::render(f, &mut state))?;

        // 2. Process all pending events from RTT background worker
        while let Ok(event) = worker.event_rx.try_recv() {
            match event {
                RttEvent::Connected { probe_name, target_name } => {
                    state.probe_name = probe_name;
                    state.target_name = target_name;
                    state.add_log_line("[System] Probe connected successfully.".to_string());
                }
                RttEvent::Attached { up_channels, down_channels } => {
                    state.add_log_line(format!(
                        "[System] RTT initialized. Up: {} channel(s), Down: {} channel(s).",
                        up_channels.len(),
                        down_channels.len()
                    ));
                }
                RttEvent::Data { channel, data } => {
                    if channel == state.active_channel {
                        state.add_log_bytes(&data);
                    }
                }
                RttEvent::Warning(w) => {
                    state.add_log_line(format!("[Warning] {}", w));
                }
                RttEvent::Error(e) => {
                    state.add_log_line(format!("[Error] {}", e));
                }
                RttEvent::Stopped => {
                    state.core_status = "STOPPED".to_string();
                    state.add_log_line("[System] Session ended.".to_string());
                }
            }
        }

        // 3. Poll crossterm events (keyboard/mouse) with 16ms timeout (~60 FPS target)
        if crossterm::event::poll(Duration::from_millis(16))? {
            let event = crossterm::event::read()?;
            events::handle_event(event, &mut state, &worker.cmd_tx)?;
        }
    }

    // Stop worker thread cleanly
    let _ = worker.cmd_tx.send(RttCommand::Stop);
    let _ = worker.thread_handle.join();

    Ok(())
}
