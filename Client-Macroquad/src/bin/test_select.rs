// 角色选择场景测试程序

use client_macroquad::game::GameResult;
use client_macroquad::scenes::select_scene::{SelectScene, CharacterInfo, Scene, SceneTransition};
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "Legend of Mir 2 - Select Scene Test".to_owned(),
        window_width: 1024,
        window_height: 768,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() -> GameResult {
    println!("🎮 传奇2 - 角色选择场景测试");
    println!("📐 窗口尺寸: {}x{}", screen_width(), screen_height());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // 创建测试角色数据
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
    
    // 创建场景
    let mut scene = SelectScene::new(characters)?;
    scene.on_enter()?;
    
    let mut last_time = get_time();
    
    loop {
        let current_time = get_time();
        let dt = (current_time - last_time) as f32;
        last_time = current_time;
        
        // 更新
        let transition = scene.update(dt)?;
        if let SceneTransition::None = transition {
            // 继续
        } else {
            println!("🎬 场景切换请求: {:?}", transition);
            break;
        }
        
        // 渲染
        scene.render()?;
        
        next_frame().await;
    }
    
    scene.on_exit()?;
    
    Ok(())
}
