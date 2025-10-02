mod app;
mod config;
mod game;
mod platform;
mod protocol_packets; // Modularized protocol packets
mod support;

pub use app::{runtime, ui};
pub use config::{keybinds, settings, version};
pub use game::{objects, protocol, state};
pub use platform::{audio, net};
pub use support::error;

use anyhow::Result;
use runtime::ClientRuntime;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let cli_flag = args
        .iter()
        .skip(1)
        .any(|arg| matches!(arg.to_ascii_lowercase().as_str(), "-tc" | "--test-config"));

    let env_flag = std::env::var("MIR2_CLIENT_USE_TEST_CONFIG")
        .ok()
        .map(|value| {
            let normalized = value.trim().to_ascii_lowercase();
            matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
        })
        .unwrap_or(false);

    ClientRuntime::bootstrap(cli_flag || env_flag)
}
