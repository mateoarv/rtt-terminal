use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "rtt-terminal",
    about = "Interactive RTT terminal for microcontrollers via debug probes (Segger J-Link, CMSIS-DAP, etc.)",
    version
)]
pub struct Cli {
    /// List all connected debug probes (J-Link, CMSIS-DAP, ST-Link, etc.)
    #[arg(short = 'l', long = "list-probes")]
    pub list_probes: bool,

    /// Target microcontroller chip name (e.g. nRF52840_xxAA, STM32F401RE)
    #[arg(short = 'c', long = "chip")]
    pub chip: Option<String>,

    /// Search for a chip in the probe-rs target database using fuzzy matching
    #[arg(short = 'S', long = "search-chip")]
    pub search_chip: Option<String>,

    /// Probe selector: index (e.g. 0), serial number, or VID:PID
    #[arg(short = 'p', long = "probe")]
    pub probe: Option<String>,

    /// Communication clock speed in kHz (default: 4000)
    #[arg(short = 's', long = "speed", default_value_t = 4000)]
    pub speed: u32,

    /// RTT channel to connect to initially (default: 0)
    #[arg(long = "channel", default_value_t = 0)]
    pub channel: usize,

    /// End of line sequence to send on Enter (crlf, lf, cr, none)
    #[arg(long = "eol", default_value = "crlf")]
    pub eol: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_list_probes_flag() {
        let args = vec!["rtt-terminal", "--list-probes"];
        let cli = Cli::parse_from(args);
        assert!(cli.list_probes);
    }

    #[test]
    fn test_cli_chip_flag() {
        let args = vec!["rtt-terminal", "-c", "STM32F401RE", "-s", "2000"];
        let cli = Cli::parse_from(args);
        assert_eq!(cli.chip.as_deref(), Some("STM32F401RE"));
        assert_eq!(cli.speed, 2000);
        assert_eq!(cli.channel, 0);
    }

    #[test]
    fn test_cli_search_chip() {
        let args = vec!["rtt-terminal", "-S", "nrf52840"];
        let cli = Cli::parse_from(args);
        assert_eq!(cli.search_chip.as_deref(), Some("nrf52840"));
    }
}
