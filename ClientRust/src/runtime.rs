use anyhow::{Context, Result};
use tokio::runtime::{Builder, Runtime};

use crate::keybinds::KeyBindSettings;
use crate::settings::ClientSettings;
use crate::{audio, net, ui};
use mir2_shared::ClientVersion;

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
            let audio = audio::AudioEngine::new(&settings.sound).context("initializing audio")?;
            let net = net::NetworkStack::connect(&settings.network)
                .await
                .context("initializing network")?;

            if let Err(err) = net
                .send_packet(&ClientVersion {
                    version_hash: Vec::new(),
                })
                .await
            {
                tracing::warn!(error = %err, "failed to send client version handshake");
            }
            ui::launch(&settings, &keybinds, audio, net)
                .await
                .context("running ui")?;
            keybinds.save().context("saving key bindings")
        })
    }
}
