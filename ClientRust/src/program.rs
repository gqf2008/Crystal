use anyhow::{Context, Result};
use tokio::runtime::{Builder, Runtime};

use crate::key_bind_settings::KeyBindSettings;
use crate::settings::ClientSettings;
use crate::version;

// TODO: Implement these modules
// use crate::audio;  // Audio engine - not yet implemented
// use crate::ui;     // UI layer - not yet implemented
use crate::network as net;  // Use network module as 'net'

pub struct ClientRuntime {
    settings: ClientSettings,
    keybinds: KeyBindSettings,
    tokio: Runtime,
}

impl ClientRuntime {
    pub fn bootstrap(use_test_config: bool) -> Result<()> {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_target(false)
            .init();

        let settings =
            ClientSettings::load(use_test_config, None).context("loading client settings")?;
        let keybinds =
            KeyBindSettings::load(&settings.root_path).context("loading key bindings")?;

        let tokio = Builder::new_multi_thread()
            .enable_all()
            .thread_name("mir2-client")
            .build()
            .context("building tokio runtime")?;

        let runtime = Self {
            settings,
            keybinds,
            tokio,
        };

        runtime.run()
    }

    fn run(self) -> Result<()> {
        let Self {
            settings,
            keybinds,
            tokio,
        } = self;

        tokio.block_on(async move {
            // TODO: Initialize audio engine (not yet implemented)
            // let audio = audio::AudioEngine::new(&settings.sound).context("initializing audio")?;
            
            let mut net = net::NetworkStack::new(&settings.network);
            net.connect(&settings.network)
                .await
                .context("initializing network")?;

            let version_hash = match version::client_binary_hash() {
                Ok(hash) => {
                    tracing::info!(
                        hash = %version::hash_to_hex(&hash),
                        "computed client version hash"
                    );
                    hash
                }
                Err(err) => {
                    tracing::warn!(
                        error = %err,
                        "failed to compute client version hash, falling back to empty hash"
                    );
                    Vec::new()
                }
            };

            // Launch UI (Forms-based windows)
            let launch_result = crate::ui::launch(&settings, &keybinds)
                .await
                .context("running ui")?;
            
            tracing::info!("Client UI completed: {:?}", launch_result);
            
            // Save settings and keybinds
            settings.save().context("saving settings")?;
            keybinds.save().context("saving key bindings")?;
            
            // Handle restart request
            match launch_result {
                crate::ui::LaunchResult::Restart => {
                    tracing::info!("Restart requested, client will restart");
                    // TODO: Implement restart mechanism
                }
                crate::ui::LaunchResult::Exit => {
                    tracing::info!("Normal exit");
                }
            }
            
            Ok(())
        })
    }
}
