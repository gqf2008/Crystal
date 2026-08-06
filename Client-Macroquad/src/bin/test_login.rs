// ============================================================================
// 登录场景测试程序
// ============================================================================
//
// 说明：
// - 测试登录场景 UI 和动画
// - 使用 macroquad::ui 构建界面
// - 加载游戏资源（ChrSel, Title, Prguse）
//
// 运行方式：
// cargo run --bin test_login --release
// ============================================================================

// Windows: Release 模式不弹控制台（Debug 仍保留控制台便于调试）
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use macroquad::miniquad::conf::Platform;
use macroquad::prelude::*;

// 引用库模块
use client_macroquad::{
    scenes::{
        CharacterInfo, GameScene, LoadingScene, LoginScene, Scene, SceneKind, SceneTransition,
        SelectScene,
    },
    ui::init_chinese_font,
};

// ============================================================================
// 常量配置
// ============================================================================

const WINDOW_WIDTH: i32 = 1024;
const WINDOW_HEIGHT: i32 = 768;

// ============================================================================
// 窗口配置
// ============================================================================

fn window_conf() -> Conf {
    Conf {
        window_title: "传奇2 - 登录测试".to_string(),
        window_width: WINDOW_WIDTH,
        window_height: WINDOW_HEIGHT,
        window_resizable: true,
        high_dpi: false,
        fullscreen: false,
        platform: Platform {
            swap_interval: Some(1), // VSync
            ..Default::default()
        },
        ..Default::default()
    }
}

// ============================================================================
// 主程序
// ============================================================================

#[macroquad::main(window_conf)]
async fn main() {
    println!("🎮 传奇2 - 登录场景测试");
    println!("📐 窗口尺寸: {}x{}", WINDOW_WIDTH, WINDOW_HEIGHT);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    init_chinese_font().await;
    // 创建初始场景 (登录场景)
    let mut current_scene = SceneKind::Login(LoginScene::new());

    if let Err(e) = current_scene.on_enter() {
        eprintln!("❌ 场景初始化失败: {}", e);
        return;
    }

    println!("✅ {} 初始化成功", current_scene.name());
    println!("\n💡 提示:");
    println!("  - 输入账号和密码");
    println!("  - 点击 OK 按钮或按 Enter 登录");
    println!("  - ESC 退出程序");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 主循环
    loop {
        let dt = get_frame_time();

        // 处理输入
        if let Err(e) = current_scene.handle_input() {
            eprintln!("❌ 输入处理错误: {}", e);
        }

        // 更新场景
        match current_scene.update(dt) {
            Ok(SceneTransition::None) => {}
            Ok(SceneTransition::Login) => {
                println!("🎬 切换场景: {} -> 登录", current_scene.name());
                current_scene.on_exit().ok();
                let mut new_scene = SceneKind::Login(LoginScene::new());
                new_scene.on_enter().expect("场景初始化失败");
                current_scene = new_scene;
            }
            Ok(SceneTransition::CharacterSelect) => {
                println!("🎬 切换场景: {} -> 角色选择", current_scene.name());
                current_scene.on_exit().ok();
                // 临时：LoginScene 还未接入真实网络拉取角色列表，这里提供测试角色，打通闭环。
                let characters = vec![CharacterInfo {
                    index: 0,
                    name: "测试角色".to_string(),
                    level: 1,
                    class: 0,
                    gender: 0,
                    last_access: "刚刚".to_string(),
                }];
                let mut new_scene = SceneKind::CharacterSelect(
                    SelectScene::new(characters).expect("SelectScene 创建失败"),
                );
                new_scene.on_enter().expect("场景初始化失败");
                current_scene = new_scene;
            }
            Ok(SceneTransition::Game) => {
                println!("🎬 切换场景: {} -> 游戏", current_scene.name());
                current_scene.on_exit().ok();
                let mut scene = GameScene::new();
                scene.load_textures();
                let mut new_scene = SceneKind::Game(scene);
                new_scene.on_enter().expect("场景初始化失败");
                current_scene = new_scene;
            }
            Ok(SceneTransition::Loading) => {
                println!("🎬 切换场景: {} -> 加载中", current_scene.name());
                current_scene.on_exit().ok();
                let mut new_scene = SceneKind::Loading(LoadingScene::new());
                new_scene.on_enter().expect("场景初始化失败");
                current_scene = new_scene;
            }
            Ok(SceneTransition::Exit) => {
                println!("👋 退出程序");
                break;
            }
            Err(e) => {
                eprintln!("❌ 场景更新错误: {}", e);
            }
        }

        // 渲染场景
        if let Err(e) = current_scene.render() {
            eprintln!("❌ 场景渲染错误: {}", e);
        }

        // ESC退出
        if is_key_pressed(KeyCode::Escape) {
            println!("👋 退出程序 (ESC)");
            break;
        }

        next_frame().await;
    }

    println!("\n✅ 程序正常退出");
}
