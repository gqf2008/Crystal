// SelectScene - 角色选择场景主模块
// 参考 ggez 版本 src/scenes/select_scene.rs 实现
// 使用 Bevy ECS 架构 + MLibrary 纹理系统

// 子模块声明
mod components;
mod setup;
mod update_systems;
mod button_systems;
mod slot_systems;
mod dialog_systems;

// 导出组件和常量
pub use components::*;

// 导出 dialog 组件和系统
pub use dialog_systems::{
    DialogState,
    ActiveDialog,
    spawn_new_character_dialog,
    handle_dialog_button_clicks,
    handle_dialog_button_hover,
    update_dialog_character_preview,
    open_new_character_dialog,
};

// 导出 setup 函数
use setup::{
    spawn_background,
    spawn_title,
    spawn_server_label,
    spawn_character_slots,
    spawn_character_preview,
    spawn_bottom_buttons,
};

// 导出 update 系统
pub use update_systems::{
    update_character_animation,
    update_character_slots,
    update_button_textures,
};

// 导出 button 系统
pub use button_systems::handle_button_clicks;

// 导出 slot 系统
pub use slot_systems::{
    handle_slot_clicks,
    update_slot_texts,
    update_slot_text_colors,
    handle_slot_hover,
};

use bevy::prelude::*;

// ============================================================================
// Network Initialization
// ============================================================================

/// 初始化 SelectScene 的网络命令通道
/// 这会设置 Bevy UI 线程和网络线程之间的通信通道
pub fn init_network_channel(
    mut state: Option<ResMut<SelectSceneState>>,
    network_sender: Option<Res<crate::bevy::NetworkCommandSender>>,
) {
    // 只在 SelectSceneState 存在时处理（仅在 Select 状态）
    if state.is_none() {
        return;
    }
    
    if let Some(mut select_state) = state {
        if let Some(sender) = network_sender {
            // 使用全局 NetworkCommandSender
            select_state.command_tx = Some(sender.tx.clone());
            info!("📡 SelectScene 网络通道已连接到全局 NetworkManager");
        } else {
            // 如果 NetworkManager 未初始化，则回退到测试模式
            warn!("⚠️ 全局 NetworkManager 不可用，保持测试模式");
        }
    }
}

// ============================================================================
// Setup and Cleanup
// ============================================================================

/// 设置选择场景（参考 ggez 版本的 draw 方法）
pub fn setup_select_scene(
    mut commands: Commands,
    mut mlibrary_assets: ResMut<crate::bevy::MLibraryAssets>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    info!("🎮 Setting up SelectScene (模块化架构)");
    
    // 插入 SelectSceneState 资源（使用测试数据）
    commands.insert_resource(SelectSceneState::with_test_data());
    info!("✅ SelectSceneState 已创建（包含 3 个测试角色）");
    
    // 插入 DialogState 资源
    commands.insert_resource(crate::bevy::scenes::select_scene::DialogState::default());
    info!("✅ DialogState 已创建");
    
    // 创建根实体 (全屏容器)
    let root = commands.spawn((
        Node {
            width: Val::Px(1024.0),
            height: Val::Px(768.0),
            position_type: PositionType::Absolute,
            ..default()
        },
        SelectSceneRoot,
        Name::new("SelectSceneRoot"),
    )).id();
    
    info!("✅ Root 实体已创建");
    
    // 1. 加载背景纹理 (Prguse_65)
    spawn_background(&mut commands, root, &mut mlibrary_assets, &mut images);
    
    // 2. 加载标题纹理 (Title_40 at 468, 20)
    spawn_title(&mut commands, root, &mut mlibrary_assets, &mut images);
    
    // 3. 加载服务器标签 (文本: "Legend of Mir 2" at 432+77.5, 60)
    spawn_server_label(&mut commands, root, &asset_server);
    
    // 4. 创建角色槽位（4个，位置：637, 194/298/402/506）
    spawn_character_slots(&mut commands, root, &mut mlibrary_assets, &mut images, &asset_server);
    
    // 5. 创建角色预览动画区域（位置：260, 420）
    spawn_character_preview(&mut commands, root);
    
    // 6. 创建底部按钮（5个）
    spawn_bottom_buttons(&mut commands, root, &mut mlibrary_assets, &mut images);
    
    info!("🎉 SelectScene 设置完成（模块化架构）！");
}

/// 清理选择场景
pub fn cleanup_select_scene(
    mut commands: Commands,
    query: Query<Entity, With<SelectSceneRoot>>,
) {
    for entity in query.iter() {
        // 简单移除实体（子实体会自动清理）
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<SelectSceneState>();
    commands.remove_resource::<crate::bevy::scenes::select_scene::DialogState>();
    info!("🧹 SelectScene 已清理");
}
