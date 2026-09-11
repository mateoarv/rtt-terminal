use anyhow::Result;
use crossbeam_channel::unbounded;
use std::thread;
use std::time::Duration;

use crate::probe::rtt::ChannelInfo;
use crate::probe::worker::{RttCommand, RttEvent, RttWorkerHandle};

pub fn start_mock_worker() -> Result<RttWorkerHandle> {
    let (cmd_tx, cmd_rx) = unbounded::<RttCommand>();
    let (event_tx, event_rx) = unbounded::<RttEvent>();

    let thread_handle = thread::spawn(move || {
        let _ = event_tx.send(RttEvent::Connected {
            probe_name: "Mock-JLink (Serial: 99999999)".to_string(),
            target_name: "nRF52840_xxAA [Simulated]".to_string(),
        });

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

        let initial_messages = [
            "\x1b[1;32m<info>\x1b[0m \x1b[1mboot:\x1b[0m Device booted from internal flash\r\n",
            "\x1b[1;32m<info>\x1b[0m \x1b[1mclock:\x1b[0m HFCLK running at 64 MHz, LFCLK calibrated (32.768 kHz)\r\n",
            "\x1b[1;32m<info>\x1b[0m \x1b[1mapp:\x1b[0m System initialized. Firmware v2.1.0-alpha [Zephyr RTOS]\r\n",
            "\x1b[1;32m<info>\x1b[0m \x1b[1msensor:\x1b[0m Calibrating 6-axis IMU (LSM6DSOX)...\r\n",
            "\x1b[1;32m<info>\x1b[0m \x1b[1msensor:\x1b[0m IMU OK (Accel: +0.02, -0.01, +0.98 g)\r\n",
            "\x1b[1;33m<warning>\x1b[0m \x1b[1mble:\x1b[0m Advertising on channel 37, 38, 39... Waiting for peer\r\n",
            "\x1b[1;32m<info>\x1b[0m \x1b[1mble:\x1b[0m Connected to central 4A:21:88:9C:12:00 (Interval: 15ms)\r\n",
            "\x1b[1;36m<debug>\x1b[0m \x1b[1mnet:\x1b[0m MTU exchange complete: 247 bytes\r\n",
        ];

        for msg in initial_messages {
            let _ = event_tx.send(RttEvent::Data {
                channel: 0,
                data: msg.as_bytes().to_vec(),
            });
            thread::sleep(Duration::from_millis(150));
        }

        let mut counter = 0u32;

        'mock_loop: loop {
            // Check for commands sent from TUI input
            while let Ok(cmd) = cmd_rx.try_recv() {
                match cmd {
                    RttCommand::Write(channel, bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        let response = format!(
                            "\x1b[36m[mock-firmware]\x1b[0m Received command on CH{}: \"\x1b[1m{}\x1b[0m\"\r\n",
                            channel,
                            text.trim_end()
                        );
                        let _ = event_tx.send(RttEvent::Data {
                            channel: 0,
                            data: response.into_bytes(),
                        });
                    }
                    RttCommand::Reset => {
                        let _ = event_tx.send(RttEvent::Data {
                            channel: 0,
                            data: b"\x1b[1;33m[mock-firmware] *** Target Reset Triggered ***\x1b[0m\r\n".to_vec(),
                        });
                    }
                    RttCommand::Halt => {
                        let _ = event_tx.send(RttEvent::Data {
                            channel: 0,
                            data: b"\x1b[1;33m[mock-firmware] Target Halted.\x1b[0m\r\n".to_vec(),
                        });
                    }
                    RttCommand::Run => {
                        let _ = event_tx.send(RttEvent::Data {
                            channel: 0,
                            data: b"\x1b[1;32m[mock-firmware] Target Resumed.\x1b[0m\r\n".to_vec(),
                        });
                    }
                    RttCommand::Stop => {
                        break 'mock_loop;
                    }
                }
            }

            // Periodic telemetry log simulation with ANSI colors
            counter += 1;
            let temp = 22.0 + (counter % 10) as f32 * 0.4;
            let batt = 4.15 - (counter % 30) as f32 * 0.01;

            let telemetry = if counter % 15 == 0 {
                format!(
                    "\x1b[1;33m<warning>\x1b[0m \x1b[1mpower:\x1b[0m Battery voltage threshold: \x1b[33m{:.2}V\x1b[0m\r\n",
                    batt
                )
            } else if counter % 28 == 0 {
                format!(
                    "\x1b[1;31m<error>\x1b[0m \x1b[1mble:\x1b[0m Packet CRC check failed on conn event #\x1b[31m{}\x1b[0m\r\n",
                    counter
                )
            } else {
                format!(
                    "\x1b[1;36m<debug>\x1b[0m \x1b[1mtelemetry:\x1b[0m Batt=\x1b[32m{:.2}V\x1b[0m, Temp=\x1b[33m{:.1}C\x1b[0m, Seq=#\x1b[35m{}\x1b[0m\r\n",
                    batt, temp, counter
                )
            };

            let _ = event_tx.send(RttEvent::Data {
                channel: 0,
                data: telemetry.into_bytes(),
            });

            thread::sleep(Duration::from_millis(400));
        }

        let _ = event_tx.send(RttEvent::Stopped);
    });

    Ok(RttWorkerHandle {
        cmd_tx,
        event_rx,
        thread_handle,
    })
}
