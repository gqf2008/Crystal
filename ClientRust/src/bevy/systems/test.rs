// Test System - 测试精灵显示
use bevy::prelude::*;
use crate::bevy::{MLibraryResource, Player, GridPosition, Movement, RenderOffset, LibraryName};

/// 生成测试玩家精灵
pub fn spawn_test_player(
    mut commands: Commands,
    mlibrary: Res<MLibraryResource>,
    mut images: ResMut<Assets<Image>>,
) {
    // 检查核心库是否加载
    if !mlibrary.loaded {
        println!("⚠️ 核心库未加载,跳过生成测试精灵");
        return;
    }
    
    // 从 ChrSel 库加载第一个图像作为测试
    let test_image_index = 0;
    if let Some(bevy_image) = MLibraryResource::get_bevy_image(LibraryName::ChrSel, test_image_index) {
        let texture = images.add(bevy_image);
        
        println!("✅ 成功加载测试精灵 (ChrSel, 图像:{})", test_image_index);
        
        // 生成玩家实体
        commands.spawn((
            Player,
            GridPosition::new(5, 5),
            Movement::new(),
            RenderOffset::default(),
            Sprite {
                image: texture,
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, 0.0),
        ));
        
        println!("🎮 测试玩家已生成在网格坐标 (5, 5)");
    } else {
        println!("❌ 无法加载测试精灵 (ChrSel 库可能不包含图像)");
    }
}

/// 调试信息显示系统
pub fn debug_info_system(
    time: Res<Time>,
    player_query: Query<(&GridPosition, &Transform), With<Player>>,
) {
    // 每秒打印一次调试信息
    if time.elapsed_secs() as u32 % 5 == 0 && time.delta_secs() < 0.1 {
        for (grid_pos, transform) in player_query.iter() {
            println!(
                "🐛 DEBUG | 网格:({}, {}) | 世界:({:.1}, {:.1})",
                grid_pos.x, grid_pos.y,
                transform.translation.x, transform.translation.y
            );
        }
    }
}
