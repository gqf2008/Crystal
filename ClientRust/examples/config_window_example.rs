// Config Window Example
// Demonstrates the configuration dialog

use mir2_client::{
    forms::ConfigWindow,
    settings::ClientSettings,
};
use std::sync::Arc;
use anyhow::Result;
use winit::{
    event_loop::{EventLoop, ControlFlow},
    window::Window,
    dpi::LogicalSize,
    event::{Event, WindowEvent},
};

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .with_target(false)
        .init();

    println!("=== Config Window Example ===\n");

    // Load settings
    let settings = ClientSettings::load(true, None)?;
    println!("Current settings:");
    println!("  Resolution: {}x{}",
        settings.graphics.dimensions().width,
        settings.graphics.dimensions().height
    );
    println!("  Fullscreen: {}", settings.graphics.full_screen);
    println!("  Sound: {}%, Music: {}%",
        settings.sound.volume,
        settings.sound.music
    );
    println!("  FPS Cap: {}\n", settings.graphics.fps_cap);

    // Create event loop and window
    let event_loop = EventLoop::new()?;
    let window_attrs = Window::default_attributes()
        .with_title("Mir2 Settings")
        .with_inner_size(LogicalSize::new(600, 400))
        .with_resizable(false);
    
    let window = event_loop.create_window(window_attrs)?;
    
    // Create config window
    let mut config = ConfigWindow::new(Arc::new(window), settings);

    println!("Config window created. Try modifying settings:");
    println!("  Press 1-4: Change resolution");
    println!("  Press F: Toggle fullscreen");
    println!("  Press +/-: Adjust sound volume");
    println!("  Press S: Save settings");
    println!("  Press R: Reset to defaults");
    println!("  Press ESC: Close window\n");

    // Run event loop
    event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => {
                        println!("\nWindow closed");
                        if config.is_dirty() {
                            println!("⚠ Warning: Unsaved changes!");
                        }
                        elwt.exit();
                    }
                    
                    WindowEvent::KeyboardInput { event: key_event, .. } => {
                        if key_event.state == winit::event::ElementState::Pressed {
                            if let winit::keyboard::PhysicalKey::Code(key_code) = key_event.physical_key {
                                use winit::keyboard::KeyCode;
                                
                                match key_code {
                                    KeyCode::Digit1 => {
                                        config.set_resolution(0);
                                        println!("Resolution set to: {:?}", config.get_resolutions()[0]);
                                    }
                                    KeyCode::Digit2 => {
                                        config.set_resolution(1);
                                        println!("Resolution set to: {:?}", config.get_resolutions()[1]);
                                    }
                                    KeyCode::Digit3 => {
                                        config.set_resolution(2);
                                        println!("Resolution set to: {:?}", config.get_resolutions()[2]);
                                    }
                                    KeyCode::Digit4 => {
                                        config.set_resolution(3);
                                        println!("Resolution set to: {:?}", config.get_resolutions()[3]);
                                    }
                                    KeyCode::KeyF => {
                                        let current = config.settings().graphics.full_screen;
                                        config.set_fullscreen(!current);
                                        println!("Fullscreen: {}", !current);
                                    }
                                    KeyCode::Equal | KeyCode::NumpadAdd => {
                                        let vol = config.settings().sound.volume.saturating_add(10).min(100);
                                        config.set_sound_volume(vol);
                                        println!("Sound volume: {}%", vol);
                                    }
                                    KeyCode::Minus | KeyCode::NumpadSubtract => {
                                        let vol = config.settings().sound.volume.saturating_sub(10);
                                        config.set_sound_volume(vol);
                                        println!("Sound volume: {}%", vol);
                                    }
                                    KeyCode::KeyS => {
                                        if config.is_dirty() {
                                            match config.save() {
                                                Ok(_) => println!("✓ Settings saved!"),
                                                Err(e) => println!("✗ Failed to save: {}", e),
                                            }
                                        } else {
                                            println!("No changes to save");
                                        }
                                    }
                                    KeyCode::KeyR => {
                                        config.reset_to_defaults();
                                        println!("✓ Reset to default settings");
                                    }
                                    KeyCode::Escape => {
                                        println!("\nEscape pressed");
                                        if config.is_dirty() {
                                            println!("⚠ Warning: Unsaved changes!");
                                        }
                                        elwt.exit();
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    
                    _ => {
                        if config.handle_event(&event) {
                            elwt.exit();
                        }
                    }
                }
            }
            
            Event::AboutToWait => {
                // TODO: Render config UI here
                // config.render();
            }
            
            _ => {}
        }
        
        elwt.set_control_flow(ControlFlow::Poll);
    })?;

    println!("\n=== Example Complete ===");
    Ok(())
}
