// 最简单的Bevy测试 - 只显示窗口
use bevy::prelude::*;
use bevy::window::WindowResolution;

fn main() {
    println!("🚀 启动最简Bevy测试...");
    
    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Bevy 测试窗口".to_string(),
                    resolution: WindowResolution::new(800, 600),
                    ..default()
                }),
                ..default()
            })
        )
        .add_systems(Startup, setup)
        .run();
        
    println!("程序退出");
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());
    println!("✅ 窗口已创建!");
}
