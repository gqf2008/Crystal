// ============================================================================
// 测试 GameScene - 直接进入游戏主场景（含地图渲染）
// ============================================================================

use macroquad::miniquad::conf::Platform;
use macroquad::prelude::*;

use client_macroquad::scenes::{GameScene, Scene, SceneTransition};
use client_macroquad::ui::text_renderer::init_chinese_font;

const WINDOW_WIDTH: i32 = 1024;
const WINDOW_HEIGHT: i32 = 768;

fn window_conf() -> Conf {
    Conf {
        window_title: "传奇2 - GameScene 测试".to_string(),
        window_width: WINDOW_WIDTH,
        window_height: WINDOW_HEIGHT,
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

#[macroquad::main(window_conf)]
async fn main() {
    println!("🎮 传奇2 - GameScene 测试（直接进入主场景）");

    // 初始化中文字体（MainDialog/各对话框会用到）
    init_chinese_font().await;

    let mut scene = GameScene::new();
    scene.load_textures().await;
    scene.on_enter().ok();

    loop {
        let dt = get_frame_time();

        if let Err(e) = scene.handle_input() {
            eprintln!("❌ 输入处理错误: {}", e);
        }

        let transition = match scene.update(dt) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("❌ 场景更新错误: {}", e);
                SceneTransition::None
            }
        };

        if let Err(e) = scene.render() {
            eprintln!("❌ 场景渲染错误: {}", e);
        }

        if matches!(transition, SceneTransition::Exit) || is_key_pressed(KeyCode::Escape) {
            break;
        }

        next_frame().await;
    }
}
