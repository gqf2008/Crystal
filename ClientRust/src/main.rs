// Core modules - organized to match C# Client structure
mod error;
mod version;
mod settings;
mod key_bind_settings; // Renamed from keybinds
mod program;           // Renamed from runtime
// mod app;            // 暂时注释 - 依赖 eframe，待重构为 winit + wgpu

// Main functional modules (matching C# Client directory structure)
// 阶段 1: 专注于 MirGraphics 移植，暂时注释其他依赖 egui 的模块
// mod forms;       // ← Client/Forms/ (依赖 egui，暂时注释)
// mod controls;    // ← Client/MirControls/ (依赖 egui，暂时注释)
mod graphics;    // ← Client/MirGraphics/ ✅ 当前移植目标
// mod map;      // ← 已废弃，功能移至 objects/map_code.rs
mod network;     // ← Client/MirNetwork/ (protocol, network moved here)
// mod objects;     // ← Client/MirObjects/ (包含 map_code.rs) (依赖其他模块)
// mod scenes;      // ← Client/MirScenes/ (依赖 egui，暂时注释)
// mod sounds;      // ← Client/MirSounds/ (依赖 rodio API 变化，暂时注释)
// mod resolution;  // ← Client/Resolution/
mod utils;       // ← Client/Utils/

use anyhow::Result;
use settings::ClientSettings;
// use app::MirClientApp;  // 暂时注释，等待重构
use network::{NetworkManager, network_task};
use std::sync::Arc;
use parking_lot::RwLock;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::event::{Event, WindowEvent};

fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    // Parse command line arguments
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

    // Load settings
    let use_test_config = cli_flag || env_flag;
    let settings = ClientSettings::load(use_test_config, None)?;

    tracing::info!("Starting Legend of Mir 2 - Rust Edition");
    tracing::info!("Using test config: {}", use_test_config);
    
    // Create event channel for game events (network -> UI)
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
    
    // Create command channel for network commands (UI -> network)
    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    
    // Create network manager
    let settings_arc = Arc::new(RwLock::new(settings.clone()));
    let network_manager = NetworkManager::new(settings_arc.clone(), event_tx, command_rx);
    let _game_client = network_manager.game_client();
    
    // Spawn network task in background
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(network_task(network_manager));
    });
    
    tracing::info!("Network task started in background");

    // TODO: 使用 winit + wgpu 27 实现窗口和渲染
    // C# equivalent: Program.cs - Main() + Form creation
    //
    // 简单的 winit 窗口示例 (完整实现待后续添加)
    let event_loop = EventLoop::new()?;
    let window = event_loop.create_window(
        winit::window::WindowAttributes::default()
            .with_title("Legend of Mir 2")
            .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 768.0))
    )?;
    
    // 创建 DXManager (wgpu)
    let window_arc = Arc::new(window);
    let dx_manager = pollster::block_on(graphics::DXManager::new(window_arc.clone()));
    
    tracing::info!("Graphics initialized (wgpu 27.0)");
    
    // 运行事件循环
    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);
        
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                tracing::info!("Window close requested, exiting");
                elwt.exit();
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                ..
            } => {
                dx_manager.resize(new_size.width, new_size.height);
            }
            Event::AboutToWait => {
                // TODO: 游戏主循环逻辑
                // 1. 处理网络事件
                // 2. 更新游戏状态
                // 3. 渲染帧
                window_arc.request_redraw();
            }
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                // TODO: 渲染逻辑
                // 类似 C# 的 CMain.Draw()
            }
            _ => {}
        }
    })?;
    
    Ok(())
}
