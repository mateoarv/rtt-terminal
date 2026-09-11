use anyhow::Result;
use probe_rs::probe::list::Lister;
use probe_rs::probe::DebugProbeInfo;

/// Lists all connected debug probes detected on the system.
pub fn get_connected_probes() -> Result<Vec<DebugProbeInfo>> {
    let lister = Lister::new();
    let probes = lister.list_all();
    Ok(probes)
}

/// Prints a user-friendly summary of connected probes to stdout.
pub fn print_probes(probes: &[DebugProbeInfo]) {
    if probes.is_empty() {
        println!("No debug probes detected.");
        println!("Please ensure your Segger J-Link or debug probe is connected via USB.");
        return;
    }

    println!("Connected debug probes ({} found):", probes.len());
    println!("{:-<75}", "");
    for (idx, probe) in probes.iter().enumerate() {
        let serial = probe
            .serial_number
            .as_deref()
            .unwrap_or("<No Serial>");
        println!(
            "  [{}] {} (Serial: {})",
            idx, probe.identifier, serial
        );
        println!(
            "      VID:PID: {:04x}:{:04x} | Probe Type: {:?}",
            probe.vendor_id, probe.product_id, probe.probe_type()
        );
    }
    println!("{:-<75}", "");
}
