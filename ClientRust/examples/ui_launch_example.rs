// UI Launch Example
// Demonstrates the complete client startup flow

use mir2_client::{
    ui,
    settings::ClientSettings,
    key_bind_settings::KeyBindSettings,
};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .with_target(false)
        .init();

    println!("=== Mir2 Client UI Launch Example ===\n");

    // Load settings (use test config for this example)
    println!("Loading settings...");
    let settings = ClientSettings::load(true, None)?;
    println!("✓ Settings loaded");
    println!("  - Graphics: {}x{} (fullscreen: {})",
        settings.graphics.dimensions().width,
        settings.graphics.dimensions().height,
        settings.graphics.full_screen
    );
    println!("  - Sound: {}%, Music: {}%",
        settings.sound.volume,
        settings.sound.music
    );
    println!("  - Launcher enabled: {}\n",
        settings.launcher.enabled
    );

    // Load key bindings
    println!("Loading key bindings...");
    let keybinds = KeyBindSettings::load(&settings.root_path)?;
    println!("✓ Key bindings loaded\n");

    // Launch UI
    println!("Launching client UI...");
    println!("(Close the window to exit)\n");
    
    let result = ui::launch(&settings, &keybinds).await?;
    
    // Report result
    match result {
        ui::LaunchResult::Exit => {
            println!("\n✓ Client exited normally");
        }
        ui::LaunchResult::Restart => {
            println!("\n⟳ Client requested restart");
            println!("  (In production, would restart the process)");
        }
    }

    println!("\n=== Example Complete ===");
    Ok(())
}
