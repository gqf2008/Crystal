// ============================================================================
// 传奇2 - 正式启动入口（macroquad）
// ============================================================================
//
// 目标：提供一个稳定的可运行 bin，用于完整流程：登录 -> 选角 -> 进游戏。
//
// 运行：
//   cargo run --manifest-path Client-Macroquad/Cargo.toml --bin mir2
//
// 配置：
//   Client-Macroquad/config.ini
//     [Network] UseMock=true/false
//     [Network] ServerAddr=IP:PORT
//
// ============================================================================

// Windows: Release 模式不弹控制台（Debug 仍保留控制台便于调试）
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use client_macroquad::game::{GameResult, GameState};
use macroquad::miniquad::conf::Platform;
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "传奇2".to_string(),
        window_width: 1024,
        window_height: 768,
        window_resizable: true,
        high_dpi: false,
        fullscreen: false,
        platform: Platform {
            swap_interval: Some(1),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn init_tracing() {
    // 避免重复初始化（部分测试 bin 也会 init）。
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();
}

#[macroquad::main(window_conf)]
async fn main() {
    init_tracing();

    let result: GameResult = async {
        let game = GameState::new().await?;
        game.run().await?;
        Ok(())
    }
    .await;

    if let Err(e) = result {
        eprintln!("❌ 运行失败: {}", e);
    }
}
