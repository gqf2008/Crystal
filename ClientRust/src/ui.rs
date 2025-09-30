use anyhow::Result;

use crate::{
    audio::AudioEngine,
    keybinds::KeyBindSettings,
    net::{NetworkEvent, NetworkStack},
    settings::ClientSettings,
};

/// Temporary text-mode loop that will be replaced with the real rendering stack.
pub async fn launch(
    settings: &ClientSettings,
    keybinds: &KeyBindSettings,
    _audio: AudioEngine,
    mut network: NetworkStack,
) -> Result<()> {
    let resolution = settings.resolution();
    let (server, port) = settings.server_address();

    tracing::info!(
        width = resolution.width,
        height = resolution.height,
        server = %server,
        port,
        keybinds = keybinds.len(),
        "starting Rust client placeholder UI"
    );

    while let Some(event) = network.next_event().await {
        match event {
            NetworkEvent::Connected => tracing::info!("connected to server"),
            NetworkEvent::Disconnected => {
                tracing::warn!("server disconnected");
                break;
            }
            NetworkEvent::Packet(data) => tracing::debug!(size = data.len(), "received packet"),
            NetworkEvent::Error(err) => {
                tracing::error!(error = %err, "network error");
                break;
            }
        }
    }

    Ok(())
}
