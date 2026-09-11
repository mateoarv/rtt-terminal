use anyhow::{anyhow, Result};
use probe_rs::rtt::Rtt;
use probe_rs::Core;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ChannelInfo {
    pub number: usize,
    pub name: String,
    pub buffer_size: usize,
}

#[allow(dead_code)]
pub struct RttManager {
    rtt: Rtt,
}

#[allow(dead_code)]
impl RttManager {
    /// Attempts to attach to the RTT control block in target RAM, retrying if necessary.
    pub fn attach_with_retries(
        core: &mut Core,
        max_retries: usize,
        retry_interval: Duration,
    ) -> Result<Self> {
        let mut attempts = 0;
        loop {
            attempts += 1;
            match Rtt::attach(core) {
                Ok(rtt) => {
                    return Ok(Self { rtt });
                }
                Err(e) => {
                    if attempts >= max_retries {
                        return Err(anyhow!(
                            "Failed to locate _SEGGER_RTT control block in target RAM after {} attempts: {}. \
                            Please ensure the firmware is running and initialized SEGGER_RTT.",
                            attempts,
                            e
                        ));
                    }
                    thread::sleep(retry_interval);
                }
            }
        }
    }

    /// Returns metadata for all available Up channels.
    pub fn list_up_channels(&mut self) -> Vec<ChannelInfo> {
        self.rtt
            .up_channels()
            .iter()
            .map(|ch| ChannelInfo {
                number: ch.number(),
                name: ch.name().unwrap_or("unnamed").to_string(),
                buffer_size: ch.buffer_size(),
            })
            .collect()
    }

    /// Returns metadata for all available Down channels.
    pub fn list_down_channels(&mut self) -> Vec<ChannelInfo> {
        self.rtt
            .down_channels()
            .iter()
            .map(|ch| ChannelInfo {
                number: ch.number(),
                name: ch.name().unwrap_or("unnamed").to_string(),
                buffer_size: ch.buffer_size(),
            })
            .collect()
    }

    /// Reads available bytes from the specified Up channel.
    pub fn read_channel(&mut self, core: &mut Core, channel: usize, buf: &mut [u8]) -> Result<usize> {
        if let Some(up_ch) = self.rtt.up_channel(channel) {
            up_ch.read(core, buf).map_err(|e| anyhow!("RTT read error: {}", e))
        } else {
            Err(anyhow!("Up channel {} not found", channel))
        }
    }

    /// Writes bytes to the specified Down channel.
    pub fn write_channel(&mut self, core: &mut Core, channel: usize, data: &[u8]) -> Result<usize> {
        if let Some(down_ch) = self.rtt.down_channel(channel) {
            down_ch.write(core, data).map_err(|e| anyhow!("RTT write error: {}", e))
        } else {
            Err(anyhow!("Down channel {} not found", channel))
        }
    }
}
