use bevy::prelude::*;

// 引入 Bevy 模块
mod bevy_modules {
    pub use mir2_client::bevy::*;
}

use bevy_modules::{GameState, GameConfig, MLibraryAssets, MapAssets};
use bevy_modules::systems::{
    mouse_input_system, 
    keyboard_input_system,
    movement_system,
    render_offset_system,
    animation_system,
    camera_follow_system,
};

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "传奇客户端 - Bevy 0.17.2".to_string(),
                        resolution: (1024.0, 768.0).into(),
                        resizable: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()), // 像素风格游戏使用最近邻插值
        )
        // 初始化状态
        .init_state::<GameState>()
        
        // 初始化资源
        .insert_resource(GameConfig::default())
        .insert_resource(MLibraryAssets::new())
        .insert_resource(MapAssets::new())
        
        // 启动系统
        .add_systems(Startup, setup)
        
        // 通用更新系统 (所有状态都运行)
        .add_systems(Update, (
            keyboard_input_system,
            animation_system,
        ))
        
        // 游戏中的系统 (仅在 Game 状态运行)
        .add_systems(Update, (
            mouse_input_system,
            movement_system,
            render_offset_system,
            camera_follow_system,
        ).run_if(in_state(GameState::Game)))
        
        .run();
}

/// 启动系统 - 创建摄像机和基础设置
fn setup(mut commands: Commands, mut next_state: ResMut<NextState<GameState>>) {
    // 生成 2D 摄像机
    commands.spawn(Camera2d::default());
    
    println!("✅ Bevy 原型启动成功!");
    println!("🎮 窗口大小: 1024x768");
    println!("📦 插件: DefaultPlugins + 最近邻插值");
    println!("🏗️ ECS 架构初始化完成");
    println!("📊 状态机: Loading -> Login -> Select -> Game");
    
    // 暂时直接进入游戏状态 (后续会添加登录界面)
    next_state.set(GameState::Game);
    println!("🎮 进入游戏状态 (测试模式)");
}
