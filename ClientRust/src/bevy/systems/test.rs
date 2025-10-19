// Test System - 测试精灵显示
use bevy::prelude::*;
use crate::bevy::{MLibraryResource, GridPosition, Movement, RenderOffset, LibraryName};
use crate::bevy::components::Player as LegacyPlayer;

/// 生成测试玩家精灵
pub fn spawn_test_player(
    mut commands: Commands,
    mlibrary: Res<MLibraryResource>,
    mut images: ResMut<Assets<Image>>,
) {
    info!("🎮 开始生成测试玩家...");
    
    // 检查核心库是否加载
    if !mlibrary.loaded {
        warn!("⚠️ 核心库未加载,跳过生成测试精灵");
        return;
    }
    
    // 🔧 临时方案：先用 ChrSel 测试，确保玩家能显示
    // TODO: 切换到 CArmours(0) 使用正确的游戏角色纹理
    let test_library = LibraryName::ChrSel;
    let test_image_index = 20; // ChrSel 中战士的站立姿态
    
    info!("📚 尝试加载玩家纹理: {:?}, 图像索引: {}", test_library, test_image_index);
    
    if let Some(bevy_image) = MLibraryResource::get_bevy_image(test_library.clone(), test_image_index) {
        let texture = images.add(bevy_image);
        
        info!("✅ 成功加载测试精灵 ({:?}, 图像:{}) - 玩家角色纹理", test_library, test_image_index);
        
        // 玩家初始网格位置
        let grid_x = 5;
        let grid_y = 5;
        
        // 计算世界坐标 (用于 Transform)
        let world_x = grid_x as f32 * 48.0; // CELL_WIDTH = 48
        let world_y = -(grid_y as f32 * 32.0); // CELL_HEIGHT = 32, Y轴翻转
        
        // 🔧 修复：设置正确的Z坐标
        // Z层级：Back=0.0, Middle=1.0, Player=1.5, Front=2.0
        let player_z = 1.5;
        
        // 生成玩家实体
        commands.spawn((
            LegacyPlayer,
            GridPosition::new(grid_x, grid_y),
            Movement::new(),
            RenderOffset::default(),
            Sprite {
                image: texture,
                ..default()
            },
            Transform::from_xyz(world_x, world_y, player_z), // 玩家在Middle层之上
        ));
        
        info!("🎮 测试玩家已生成 | 网格:({}, {}) | 世界:({:.1}, {:.1}, {}) | Z层级: Player", 
            grid_x, grid_y, world_x, world_y, player_z);
    } else {
        error!("❌ 无法加载测试精灵 ({:?} 库可能不包含图像或库文件不存在)", test_library);
        error!("   请检查库文件是否存在");
    }
}

/// 调试信息显示系统
pub fn debug_info_system(
    time: Res<Time>,
    player_query: Query<(&GridPosition, &Transform), With<LegacyPlayer>>,
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
