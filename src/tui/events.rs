use anyhow::Result;
use crossbeam_channel::Sender;
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};

use super::app::AppState;
use crate::probe::worker::RttCommand;

pub fn handle_event(event: Event, state: &mut AppState, cmd_tx: &Sender<RttCommand>) -> Result<()> {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            // Check for Ctrl+C
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                state.should_quit = true;
                return Ok(());
            }

            match key.code {
                KeyCode::Char(c) => {
                    state.insert_char(c);
                }
                KeyCode::Backspace => {
                    state.delete_char_before();
                }
                KeyCode::Delete => {
                    state.delete_char_at();
                }
                KeyCode::Left => {
                    state.move_cursor_left();
                }
                KeyCode::Right => {
                    state.move_cursor_right();
                }
                KeyCode::Home => {
                    state.move_cursor_home();
                }
                KeyCode::End => {
                    state.move_cursor_end();
                }
                KeyCode::Up => {
                    state.history_up();
                }
                KeyCode::Down => {
                    state.history_down();
                }
                KeyCode::PageUp => {
                    state.scroll_up(10);
                }
                KeyCode::PageDown => {
                    state.scroll_down(10);
                }
                KeyCode::Enter => {
                    let submitted = state.submit_input();
                    handle_submitted_line(&submitted, state, cmd_tx)?;
                }
                KeyCode::Esc => {
                    // Quick clear input or exit
                    state.input.clear();
                    state.cursor_position = 0;
                }
                _ => {}
            }
        }
        Event::Mouse(mouse) => match mouse.kind {
            MouseEventKind::ScrollUp => {
                state.scroll_up(3);
            }
            MouseEventKind::ScrollDown => {
                state.scroll_down(3);
            }
            _ => {}
        },
        _ => {}
    }

    Ok(())
}

fn handle_submitted_line(
    line: &str,
    state: &mut AppState,
    cmd_tx: &Sender<RttCommand>,
) -> Result<()> {
    if line.is_empty() {
        // Send empty line (EOL) to the target
        let mut bytes = Vec::new();
        append_eol(&mut bytes, &state.eol);
        let _ = cmd_tx.send(RttCommand::Write(state.active_channel, bytes));
        return Ok(());
    }

    if line.starts_with('/') {
        let parts: Vec<&str> = line[1..].split_whitespace().collect();
        let cmd = parts.first().copied().unwrap_or("");

        match cmd {
            "quit" | "exit" => {
                state.should_quit = true;
            }
            "clear" => {
                state.clear_logs();
            }
            "help" => {
                state.add_log_line("--- Available Commands ---".to_string());
                state.add_log_line("  /help              : Show this help summary".to_string());
                state.add_log_line("  /clear             : Clear the log viewport".to_string());
                state.add_log_line("  /channel <0-15>    : Switch active RTT channel".to_string());
                state.add_log_line("  /timestamps [on|off]: Toggle timestamps display".to_string());
                state.add_log_line("  /reset             : Reset target microcontroller".to_string());
                state.add_log_line("  /quit, /exit       : Disconnect and exit".to_string());
                state.add_log_line("--------------------------".to_string());
            }
            "timestamps" => {
                if let Some(&arg) = parts.get(1) {
                    state.timestamps_enabled = arg.eq_ignore_ascii_case("on");
                } else {
                    state.timestamps_enabled = !state.timestamps_enabled;
                }
                state.add_log_line(format!(
                    "[System] Timestamps: {}",
                    if state.timestamps_enabled { "ON" } else { "OFF" }
                ));
            }
            "channel" | "c" => {
                if let Some(num_str) = parts.get(1) {
                    if let Ok(ch) = num_str.parse::<usize>() {
                        state.active_channel = ch;
                        state.add_log_line(format!("[System] Active channel switched to CH{}", ch));
                    } else {
                        state.add_log_line("[Error] Invalid channel number".to_string());
                    }
                }
            }
            "reset" => {
                state.add_log_line("[System] Triggering hardware reset on target...".to_string());
                let _ = cmd_tx.send(RttCommand::Reset);
            }
            other => {
                state.add_log_line(format!(
                    "[Error] Unknown command '/{}'. Type /help for available commands.",
                    other
                ));
            }
        }
    } else {
        // Plain text: send down to the microcontroller
        let mut bytes = line.as_bytes().to_vec();
        append_eol(&mut bytes, &state.eol);
        let _ = cmd_tx.send(RttCommand::Write(state.active_channel, bytes));
    }

    Ok(())
}

fn append_eol(buf: &mut Vec<u8>, eol: &str) {
    match eol.to_lowercase().as_str() {
        "crlf" => buf.extend_from_slice(b"\r\n"),
        "lf" => buf.extend_from_slice(b"\n"),
        "cr" => buf.extend_from_slice(b"\r"),
        "none" => {}
        _ => buf.extend_from_slice(b"\r\n"),
    }
}
