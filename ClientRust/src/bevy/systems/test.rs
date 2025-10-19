// Test System - 测试精灵显示
use bevy::prelude::*;
use crate::bevy::{MLibraryResource, GridPosition, Movement, RenderOffset, LibraryName};
use crate::bevy::components::Player as LegacyPlayer;
use crate::bevy::GameState; // ← 添加 GameState 导入

/// 🔥🔥🔥 诊断系统：无论如何都要运行，显示当前游戏状态
pub fn diagnostic_state_system(state: Res<State<GameState>>) {
    static mut COUNTER: u32 = 0;
    unsafe {
        COUNTER += 1;
        if COUNTER % 60 == 0 { // 每秒打印一次
            error!("🔥🔥🔥 当前游戏状态: {:?} (帧{})", state.get(), COUNTER);
        }
    }
}

/// 生成测试玩家精灵（直接创建，无条件运行一次）
pub fn spawn_test_player_once(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    error!("🔥🔥🔥 spawn_test_player_once 开始创建玩家！🔥🔥🔥");
    
    // 玩家初始网格位置 - 放在地图中心 (700x700地图的中心是350, 350)
    let grid_x = 350;
    let grid_y = 350;
    
    // 计算世界坐标 (用于 Transform)
    let world_x = grid_x as f32 * 48.0; // CELL_WIDTH = 48 → 16800.0
    let world_y = -(grid_y as f32 * 32.0); // CELL_HEIGHT = 32, Y轴翻转 → -11200.0
    
    error!("🎯 玩家将生成在地图中心: 网格({}, {}) → 世界({:.1}, {:.1})", 
        grid_x, grid_y, world_x, world_y);
    
    // Z层级顺序（从下到上）：
    // Back=0.0 (最底层地面)
    // Middle=1.0 (中间层，如建筑底部、树木底部)
    // Player=1.5 (玩家和角色，在Middle和Front之间)
    // Front=2.0 (前景层，如建筑顶部、树冠，需要遮挡玩家)
    let player_z = 1.5;
    
    // 创建一个64x64的红色方块纹理
    let size = 64;
    let mut data = Vec::with_capacity(size * size * 4);
    for _ in 0..(size * size) {
        data.push(255); // R
        data.push(0);   // G
        data.push(0);   // B
        data.push(255); // A
    }
    
    let red_square = Image::new(
        bevy::render::render_resource::Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        Default::default(), // 使用默认的RenderAssetUsages
    );
    
    let texture = images.add(red_square);
    
    error!("✅ 生成64x64红色方块作为测试玩家");
    
    // 生成玩家实体 - LegacyPlayer就是完整的Player组件，不需要再添加
    commands.spawn((
        LegacyPlayer, // 这个就是 crate::bevy::components::Player，摄像机会找到它
        GridPosition::new(grid_x, grid_y),
        Movement::new(),
        RenderOffset::default(),
        Sprite {
            image: texture,
            custom_size: Some(Vec2::new(64.0, 64.0)), // 明确设置大小
            ..default()
        },
        Transform::from_xyz(world_x, world_y, player_z),
        Name::new("TestPlayer_RedSquare"),
    ));
    
    error!("🎮 测试玩家已生成 (红色方块64x64) | 网格:({}, {}) | 世界:({:.1}, {:.1}, {}) | Z={}", 
        grid_x, grid_y, world_x, world_y, player_z, player_z);
}

/// 验证玩家坐标系统 (在玩家生成后立即运行)
pub fn verify_player_coords(
    player_query: Query<(&Transform, &GridPosition), With<LegacyPlayer>>,
) {
    if let Ok((transform, grid_pos)) = player_query.single() {
        error!("🔍 verify_player_coords: 玩家Transform=({:.1}, {:.1}, {:.1}) | GridPos=({}, {})", 
            transform.translation.x, transform.translation.y, transform.translation.z,
            grid_pos.x, grid_pos.y);
    } else {
        error!("❌ verify_player_coords: 找不到玩家！");
    }
}


/// 生成测试玩家精灵（每帧检查，如果没有玩家就创建）
pub fn spawn_test_player(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    player_query: Query<Entity, With<LegacyPlayer>>,
) {
    // 🔥🔥🔥 无论如何都打印这条日志，确认系统被调用了
    let player_count = player_query.iter().count();
    if player_count % 60 == 0 { // 每60帧（约1秒）打印一次
        error!("🔥 spawn_test_player 正在运行！玩家数量: {}", player_count);
    }
    
    // 检查是否已经有玩家了
    if player_count > 0 {
        return; // 已经有玩家了，不需要重复创建
    }
    
    error!("🔥🔥🔥 spawn_test_player 检测到没有玩家，开始创建！🔥🔥🔥");
    
    // 调用一次性创建函数
    spawn_test_player_once(commands, images);
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
