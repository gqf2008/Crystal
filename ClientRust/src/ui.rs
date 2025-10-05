// UI Launcher - Integrates Forms to launch the client
// Corresponds to: Client/Program.cs Main() method

use anyhow::{Context, Result};
use std::sync::Arc;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;
use winit::dpi::LogicalSize;

use crate::settings::ClientSettings;
use crate::key_bind_settings::KeyBindSettings;
use crate::forms::{LauncherWindow, MainWindow};

/// UI launch result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchResult {
    /// Normal exit
    Exit,
    /// Request restart
    Restart,
}

/// Launch the client UI
/// 
/// This is the main entry point for the client UI, corresponding to
/// the C# Program.Main() flow:
/// 1. Optional launcher/patcher window
/// 2. Main game window
/// 3. Handle restart requests
pub async fn launch(
    settings: &ClientSettings,
    keybinds: &KeyBindSettings,
) -> Result<LaunchResult> {
    tracing::info!("Launching client UI");
    
    // Step 1: Run launcher/patcher if enabled
    if settings.launcher.enabled {
        tracing::info!("Running launcher/patcher");
        
        match run_launcher(settings).await {
            Ok(true) => {
                tracing::info!("Launcher completed successfully");
            }
            Ok(false) => {
                tracing::info!("Launcher cancelled by user");
                return Ok(LaunchResult::Exit);
            }
            Err(e) => {
                tracing::error!("Launcher failed: {}", e);
                return Err(e).context("launcher failed");
            }
        }
    } else {
        tracing::info!("Patcher disabled, skipping launcher");
    }
    
    // Step 2: Run main game window
    tracing::info!("Running main game window");
    let restart = run_game(settings, keybinds).await?;
    
    if restart {
        tracing::info!("Restart requested");
        Ok(LaunchResult::Restart)
    } else {
        tracing::info!("Normal exit");
        Ok(LaunchResult::Exit)
    }
}

/// Run the launcher/patcher window
/// 
/// Returns:
/// - Ok(true) if patching completed successfully
/// - Ok(false) if user cancelled
/// - Err if an error occurred
async fn run_launcher(settings: &ClientSettings) -> Result<bool> {
    tracing::debug!("Creating launcher window");
    
    let event_loop = EventLoop::new()
        .context("creating event loop")?;
    
    let window_attrs = Window::default_attributes()
        .with_title("Legend of Mir 2 - Launcher")
        .with_inner_size(LogicalSize::new(800, 600))
        .with_resizable(false)
        .with_decorations(true);
    
    let window = event_loop.create_window(window_attrs)
        .context("creating launcher window")?;
    
    let mut launcher = LauncherWindow::new(Arc::new(window), settings.clone());
    
    // Start the patching process
    launcher.start().context("starting launcher")?;
    
    // Track completion and cancellation
    let mut completed = false;
    let mut cancelled = false;
    
    // Run event loop
    event_loop.run(move |event, elwt| {
        use winit::event::{Event, WindowEvent};
        
        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => {
                        tracing::info!("Launcher window close requested");
                        cancelled = true;
                        elwt.exit();
                    }
                    _ => {
                        if launcher.handle_event(&event) {
                            elwt.exit();
                        }
                    }
                }
            }
            Event::AboutToWait => {
                // Check completion
                if launcher.completed {
                    if launcher.error_found {
                        tracing::error!("Launcher completed with errors");
                        cancelled = true;
                    } else {
                        tracing::info!("Launcher completed successfully");
                        completed = true;
                    }
                    elwt.exit();
                }
                
                // TODO: Render launcher UI
                // launcher.render();
            }
            _ => {}
        }
        
        elwt.set_control_flow(ControlFlow::Poll);
    })?;
    
    Ok(completed && !cancelled)
}

/// Run the main game window
/// 
/// Returns true if restart is requested
async fn run_game(
    settings: &ClientSettings,
    _keybinds: &KeyBindSettings,
) -> Result<bool> {
    tracing::debug!("Creating game window");
    
    let event_loop = EventLoop::new()
        .context("creating event loop")?;
    
    let dims = settings.graphics.dimensions();
    
    let mut window_attrs = Window::default_attributes()
        .with_title("Legend of Mir 2")
        .with_inner_size(LogicalSize::new(
            dims.width as u32,
            dims.height as u32,
        ));
    
    // Apply fullscreen if enabled
    if settings.graphics.full_screen {
        use winit::window::Fullscreen;
        window_attrs = window_attrs.with_fullscreen(Some(Fullscreen::Borderless(None)));
    }
    
    let window = event_loop.create_window(window_attrs)
        .context("creating game window")?;
    
    let mut game = MainWindow::new(Arc::new(window), settings.clone());
    
    // Initialize game
    game.initialize().context("initializing game")?;
    
    // Track restart request
    let restart_requested = false;
    
    // Game loop
    use std::time::Instant;
    let mut last_update = Instant::now();
    
    event_loop.run(move |event, elwt| {
        use winit::event::{Event, WindowEvent};
        
        match event {
            Event::WindowEvent { event, .. } => {
                match &event {
                    WindowEvent::CloseRequested => {
                        tracing::info!("Game window close requested");
                        game.shutdown();
                        elwt.exit();
                    }
                    _ => {
                        if game.handle_event(&event) {
                            game.shutdown();
                            elwt.exit();
                        }
                    }
                }
            }
            Event::AboutToWait => {
                // Calculate delta time
                let now = Instant::now();
                let delta = now.duration_since(last_update);
                last_update = now;
                
                // Update game logic
                game.update(delta.as_secs_f32());
                
                // Render
                game.render();
                
                // Request redraw
                // game.window.request_redraw(); // TODO: expose window
            }
            _ => {}
        }
        
        elwt.set_control_flow(ControlFlow::Poll);
    })?;
    
    Ok(restart_requested)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_launch_result() {
        assert_eq!(LaunchResult::Exit, LaunchResult::Exit);
        assert_ne!(LaunchResult::Exit, LaunchResult::Restart);
    }
}
