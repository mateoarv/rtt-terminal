use anyhow::Result;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::rtt::ChannelInfo;
use super::segger::SeggerJLink;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum RttCommand {
    Write(usize, Vec<u8>),
    Reset,
    Halt,
    Run,
    Stop,
}

#[derive(Debug, Clone)]
pub enum RttEvent {
    Connected {
        probe_name: String,
        target_name: String,
    },
    Attached {
        up_channels: Vec<ChannelInfo>,
        down_channels: Vec<ChannelInfo>,
    },
    Data {
        channel: usize,
        data: Vec<u8>,
    },
    Info(String),
    Warning(String),
    Error(String),
    Stopped,
}

pub struct RttWorkerHandle {
    pub cmd_tx: Sender<RttCommand>,
    pub event_rx: Receiver<RttEvent>,
    pub thread_handle: JoinHandle<()>,
}

/// Starts the RTT background worker thread using Segger J-Link DLL.
pub fn start_rtt_worker(
    _probe_selector: Option<String>,
    chip_name: String,
    speed_khz: u32,
    active_channel: usize,
) -> Result<RttWorkerHandle> {
    let (cmd_tx, cmd_rx) = unbounded::<RttCommand>();
    let (event_tx, event_rx) = unbounded::<RttEvent>();

    let thread_handle = thread::spawn(move || {
        let segger = match SeggerJLink::open(&chip_name, speed_khz) {
            Ok(s) => {
                let _ = event_tx.send(RttEvent::Info(format!(
                    "Loaded Segger J-Link DLL: {:?}",
                    s.dll_path
                )));
                let sn = s.get_serial_number();
                let _ = event_tx.send(RttEvent::Connected {
                    probe_name: format!("J-Link (Serial: {:08})", sn),
                    target_name: chip_name.clone(),
                });
                s
            }
            Err(e) => {
                let _ = event_tx.send(RttEvent::Error(format!("{:#}", e)));
                let _ = event_tx.send(RttEvent::Stopped);
                return;
            }
        };

        // Start Segger RTT terminal engine
        if let Err(e) = segger.start_rtt() {
            let _ = event_tx.send(RttEvent::Error(format!("Failed to start Segger RTT: {:#}", e)));
            let _ = event_tx.send(RttEvent::Stopped);
            return;
        }

        let up_channels = vec![ChannelInfo {
            number: 0,
            name: "Terminal".to_string(),
            buffer_size: 1024,
        }];
        let down_channels = vec![ChannelInfo {
            number: 0,
            name: "Terminal".to_string(),
            buffer_size: 64,
        }];

        let _ = event_tx.send(RttEvent::Attached {
            up_channels,
            down_channels,
        });

        let mut read_buf = [0u8; 1024];

        'worker_loop: loop {
            // 1. Process pending commands
            while let Ok(cmd) = cmd_rx.try_recv() {
                match cmd {
                    RttCommand::Write(channel, bytes) => {
                        if let Err(e) = segger.write_rtt(channel as u32, &bytes) {
                            let _ = event_tx.send(RttEvent::Warning(format!("Write error on ch {}: {}", channel, e)));
                        }
                    }
                    RttCommand::Reset => {
                        segger.reset_target();
                    }
                    RttCommand::Halt => {
                        let _ = segger.halt_target();
                    }
                    RttCommand::Run => {
                        segger.resume_target();
                    }
                    RttCommand::Stop => {
                        break 'worker_loop;
                    }
                }
            }

            // 2. Poll active Up channel
            match segger.read_rtt(active_channel as u32, &mut read_buf) {
                Ok(bytes_read) if bytes_read > 0 => {
                    let _ = event_tx.send(RttEvent::Data {
                        channel: active_channel,
                        data: read_buf[..bytes_read].to_vec(),
                    });
                    thread::yield_now();
                }
                Ok(_) => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(e) => {
                    let _ = event_tx.send(RttEvent::Warning(format!("Read error on ch {}: {}", active_channel, e)));
                    thread::sleep(Duration::from_millis(50));
                }
            }
        }

        let _ = event_tx.send(RttEvent::Stopped);
    });

    Ok(RttWorkerHandle {
        cmd_tx,
        event_rx,
        thread_handle,
    })
}
