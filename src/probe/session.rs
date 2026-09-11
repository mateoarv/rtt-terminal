use anyhow::{anyhow, Result};
use probe_rs::config::Registry;
use probe_rs::probe::list::Lister;
use probe_rs::probe::DebugProbeSelector;
use probe_rs::{Permissions, Session};

/// Opens a debug probe and establishes a session with the specified target chip.
#[allow(dead_code)]
pub fn create_session(
    probe_selector_str: Option<&str>,
    chip_name: &str,
    speed_khz: u32,
) -> Result<Session> {
    let lister = Lister::new();
    let probes = lister.list_all();

    if probes.is_empty() {
        return Err(anyhow!(
            "No debug probes found. Please connect your Segger J-Link or debug probe via USB."
        ));
    }

    let mut probe = match probe_selector_str {
        Some(sel) => {
            if let Ok(idx) = sel.parse::<usize>() {
                if let Some(probe_info) = probes.get(idx) {
                    lister.open(probe_info)?
                } else {
                    return Err(anyhow!(
                        "Probe index {} out of range ({} probe(s) detected)",
                        idx,
                        probes.len()
                    ));
                }
            } else if let Ok(parsed_sel) = sel.parse::<DebugProbeSelector>() {
                lister.open(parsed_sel)?
            } else {
                let found = probes.iter().find(|p| {
                    p.serial_number
                        .as_deref()
                        .map_or(false, |s| s.contains(sel))
                });
                if let Some(probe_info) = found {
                    lister.open(probe_info)?
                } else {
                    return Err(anyhow!("No probe matching selector '{}' found", sel));
                }
            }
        }
        None => {
            let probe_info = &probes[0];
            lister.open(probe_info).map_err(|e| {
                anyhow!(
                    "Failed to open probe '{}': {:#}\nNote: If another application (such as nRF Connect for Desktop, J-Link GDB Server, or Ozone) is open, please close it to release the J-Link USB interface.",
                    probe_info.identifier,
                    e
                )
            })?
        }
    };

    if speed_khz > 0 {
        if let Ok(actual_speed) = probe.set_speed(speed_khz) {
            println!("SWD clock speed configured: {} kHz (actual: {} kHz)", speed_khz, actual_speed);
        }
    }

    let registry = Registry::from_builtin_families();
    let target = registry.get_target_by_name(chip_name)?;

    let session = probe.attach(target, Permissions::default()).map_err(|e| {
        anyhow!(
            "Failed to attach to target '{}': {:#}\nNote: If another application (such as nRF Connect for Desktop, J-Link GDB Server, or Ozone) is open, please close it to release the J-Link USB interface.",
            chip_name,
            e
        )
    })?;
    Ok(session)
}
