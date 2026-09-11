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
            "[00:00:00.001] <info> boot: Device booted from flash\r\n",
            "[00:00:00.005] <info> clock: HFCLK running at 64 MHz\r\n",
            "[00:00:00.120] <info> app: System initialized. Firmware v2.1.0\r\n",
            "[00:00:00.125] <info> sensor: Calibrating IMU sensor...\r\n",
            "[00:00:00.350] <info> sensor: IMU OK (Offset: 0.02, -0.01, 1.00)\r\n",
            "[00:00:01.000] <warning> ble: Waiting for BLE connection...\r\n",
            "[00:00:02.450] <info> ble: Connected to peer 4A:21:88:9C:12:00\r\n",
            "[00:00:02.455] <info> net: MTU exchanged: 247 bytes\r\n",
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
                            "[mock-firmware] Received command on CH{}: \"{}\"\r\n",
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
                            data: b"[mock-firmware] *** Target Reset Triggered ***\r\n".to_vec(),
                        });
                    }
                    RttCommand::Halt => {
                        let _ = event_tx.send(RttEvent::Data {
                            channel: 0,
                            data: b"[mock-firmware] Target Halted.\r\n".to_vec(),
                        });
                    }
                    RttCommand::Run => {
                        let _ = event_tx.send(RttEvent::Data {
                            channel: 0,
                            data: b"[mock-firmware] Target Resumed.\r\n".to_vec(),
                        });
                    }
                    RttCommand::Stop => {
                        break 'mock_loop;
                    }
                }
            }

            // Periodic telemetry log simulation
            counter += 1;
            let temp = 22.0 + (counter % 10) as f32 * 0.4;
            let batt = 4.15 - (counter % 30) as f32 * 0.01;
            let telemetry = format!(
                "[{:02}:{:02}:{:02}.000] <debug> telemetry: Batt={:.2}V, Temp={:.1}C, Packet #{}\r\n",
                (counter / 3600) % 24,
                (counter / 60) % 60,
                counter % 60,
                batt,
                temp,
                counter
            );

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
