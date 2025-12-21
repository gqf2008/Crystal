// 角色选择场景测试程序

// Windows: Release 模式不弹控制台（Debug 仍保留控制台便于调试）
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use client_macroquad::game::GameResult;
use client_macroquad::scenes::{CharacterInfo, GameScene, Scene, SceneTransition, SelectScene};
use client_macroquad::ui::init_chinese_font;
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "Legend of Mir 2 - Select Scene Test".to_owned(),
        window_width: 1024,
        window_height: 768,
        high_dpi: false,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() -> GameResult {
    println!("🎮 传奇2 - 角色选择场景测试");
    println!("📐 窗口尺寸: {}x{}", screen_width(), screen_height());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    init_chinese_font().await;
    // 创建测试角色数据 - 只创建2个角色，留出空间测试创建功能
    let characters = vec![
        CharacterInfo {
            index: 0,
            name: "战神".to_string(),
            level: 35,
            class: 0,  // 战士
            gender: 0, // 男
            last_access: "2024-01-15 20:30".to_string(),
        },
        CharacterInfo {
            index: 1,
            name: "法神".to_string(),
            level: 40,
            class: 1,  // 法师
            gender: 1, // 女
            last_access: "2024-01-16 18:45".to_string(),
        },
    ];
    
    // 创建初始场景
    enum LocalScene {
        Select(SelectScene),
        Game(GameScene),
    }

    let mut scene = LocalScene::Select(SelectScene::new(characters)?);
    if let LocalScene::Select(s) = &mut scene {
        s.on_enter()?;
    }
    
    let mut last_time = get_time();
    
    loop {
        let current_time = get_time();
        let dt = (current_time - last_time) as f32;
        last_time = current_time;
        
        // 更新 + 切换
        match &mut scene {
            LocalScene::Select(s) => {
                let transition = s.update(dt)?;
                match transition {
                    SceneTransition::None => {}
                    SceneTransition::Game => {
                        println!("🎬 场景切换: Select -> Game");
                        s.on_exit().ok();
                        let mut g = GameScene::new();
                        g.load_textures();
                        g.on_enter()?;
                        scene = LocalScene::Game(g);
                    }
                    other => {
                        println!("🎬 场景切换请求: {:?}", other);
                        break;
                    }
                }
            }
            LocalScene::Game(g) => {
                let _ = g.update(dt)?;
            }
        }

        // 渲染
        match &mut scene {
            LocalScene::Select(s) => s.render()?,
            LocalScene::Game(g) => g.render()?,
        }
        
        next_frame().await;
    }
    
    match &mut scene {
        LocalScene::Select(s) => {
            s.on_exit()?;
        }
        LocalScene::Game(g) => {
            g.on_exit()?;
        }
    }
    
    Ok(())
}
