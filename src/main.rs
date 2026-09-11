use anyhow::Result;
use clap::Parser;

mod cli;
mod probe;

use cli::Cli;
use probe::detector::{get_connected_probes, print_probes};
use probe::targets::{print_search_results, search_targets, validate_target};

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

    if let Some(chip_name) = args.chip {
        println!("Validating target microcontroller '{}'...", chip_name);
        match validate_target(&chip_name) {
            Ok(target) => {
                println!("Target '{}' is supported by probe-rs!", target.name);
                println!("Cores detected in target description: {}", target.cores.len());
                for (idx, core) in target.cores.iter().enumerate() {
                    println!("  Core [{}]: {} (Type: {:?})", idx, core.name, core.core_type);
                }

                // Check connected probes
                println!();
                let probes = get_connected_probes()?;
                if probes.is_empty() {
                    println!("Warning: No debug probe detected. Connect your Segger J-Link via USB.");
                } else {
                    println!("Ready to connect with {} detected probe(s):", probes.len());
                    for (i, p) in probes.iter().enumerate() {
                        let serial = p.serial_number.as_deref().unwrap_or("<No Serial>");
                        println!("  [{}] {} (Serial: {})", i, p.identifier, serial);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                println!();
                println!("Searching for similar chip names...");
                if let Ok(suggestions) = search_targets(&chip_name, 5) {
                    if !suggestions.is_empty() {
                        println!("Did you mean:");
                        for s in suggestions {
                            println!("  - {}", s.name);
                        }
                    }
                }
            }
        }
        return Ok(());
    }

    // Default behavior when no arguments are provided
    println!("rtt-terminal v{}", env!("CARGO_PKG_VERSION"));
    println!("High-performance interactive RTT terminal for microcontrollers.");
    println!();

    let probes = get_connected_probes()?;
    print_probes(&probes);

    println!();
    println!("Usage examples:");
    println!("  rtt-terminal --list-probes               # List connected probes");
    println!("  rtt-terminal --search-chip <query>       # Search target chips (fuzzy match)");
    println!("  rtt-terminal --chip <CHIP_NAME>          # Validate and test chip configuration");
    println!("  rtt-terminal --help                      # Show all CLI options");

    Ok(())
}
