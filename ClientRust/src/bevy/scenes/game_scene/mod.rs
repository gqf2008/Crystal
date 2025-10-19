// GameScene - 游戏场景主模块
// 处理游戏中玩家控制、交互、网络同步等
//
// 模块化重构计划 (2018行 -> 目标: 按功能拆分成子模块):
// 
// 当前结构分析:
// - mod.rs: 2018 行 (太大,需要拆分)
// - components.rs: 946 行 (可能也需要拆分)
//
// 建议重构方案:
// 1. constants.rs (~20行) - 所有常量定义
// 2. player_systems.rs (~200行) - Phase 1: 玩家实体管理 (553-628)
// 3. map_systems.rs (~250行) - Phase 2: 地图加载和渲染 (632-821)
// 4. interaction_systems.rs (~260行) - Phase 3: NPC和对象交互 (822-1081)
// 5. chat_systems.rs (~340行) - Phase 4: 聊天系统 (1082-1420)
// 6. network_systems.rs (~245行) - Phase 5: 网络同步 (1421-1665)
// 7. game_loop_systems.rs (~353行) - Phase 6: 完整事件循环 (1666-2018)
// 8. input_systems.rs (~150行) - 输入处理系统 (237-404)
// 9. hud_systems.rs (~100行) - HUD更新系统 (405-552)
//
// 预期效果: mod.rs减少到 ~300行 (setup/cleanup/基础系统)
// 总代码量: ~2200行 (增加模块化开销 ~200行)

// 子模块
mod components;
mod constants;
mod player_systems;
mod map_systems;
mod interaction_systems;
mod chat_systems;
mod network_systems;
mod game_loop_systems;

// 渲染层 (Bevy 特定)
pub mod rendering;

// 桥接层 (ECS ↔ 传统对象系统)
pub mod bridge;

// 重导出
pub use components::*;
pub use constants::*;
pub use player_systems::*;
pub use map_systems::*;
pub use interaction_systems::*;
pub use chat_systems::*;
pub use network_systems::*;
pub use game_loop_systems::*;

// 重导出渲染和桥接模块
pub use rendering::{
    MLibraryAssets, SpriteRenderer, MapRenderData, TileCache, TileEntity, TileLayer, DoorInfo, 
    GameCamera, MapLoadRequest, load_map_direct,
    render_map_system, update_animation_system, camera_follow_system_new, camera_zoom_system, load_map_system_new,
    setup_game_rendering, cleanup_game_rendering, setup_map_renderer,
    debug_transforms_system
};
pub use bridge::{MapObjectRef, NetworkBridge, ServerPacketEvent, ClientPacketEvent};

use bevy::prelude::*;
use std::collections::VecDeque;

// ============================================================================
// Setup and Cleanup
// ============================================================================

/// 设置游戏场景
pub fn setup_game_scene(
    mut commands: Commands,
) {
    info!("🎮 设置游戏场景");
    
    // 创建游戏场景状态资源
    commands.insert_resource(GameSceneState::default());
    info!("✅ GameSceneState 已创建");
    
    // 初始化 MapData 资源 (空地图,等待加载)
    commands.insert_resource(MapData::default());
    info!("✅ MapData 资源已初始化");
    
    // 创建游戏场景根节点 (全屏容器)
    let root = commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            ..default()
        },
        GameSceneRoot,
        Name::new("GameSceneRoot"),
        // 移除不透明背景，让下面的2D Sprite可见
        // BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 1.0)),
    )).id();
    
    info!("✅ 游戏场景根节点已创建");
    
    // 创建 HUD 层
    commands.entity(root).with_children(|parent| {
        // HUD 根节点
        parent.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            HudRoot,
            Name::new("HudRoot"),
        )).with_children(|parent| {
            // 顶部信息栏 (玩家信息)
            parent.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(60.0),
                    position_type: PositionType::Relative,
                    padding: UiRect::all(Val::Px(10.0)),
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(20.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
                PlayerInfoHud,
            )).with_children(|parent| {
                // 玩家等级和名称
                parent.spawn((
                    Text::new("Lv. 1 | 玩家"),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(HUD_TEXT_COLOR),
                ));
                
                // 血量条信息
                parent.spawn((
                    Text::new("HP: 100/100"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(HP_COLOR),
                ));
                
                // 蓝量条信息
                parent.spawn((
                    Text::new("MP: 50/50"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(MANA_COLOR),
                ));
            });
            
            // 右下角快捷栏
            parent.spawn((
                Node {
                    width: Val::Px(400.0),
                    height: Val::Px(60.0),
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(10.0),
                    right: Val::Px(10.0),
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(5.0),
                    padding: UiRect::all(Val::Px(5.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
                SkillBar,
            )).with_children(|parent| {
                // 生成 12 个快捷栏按钮
                for i in 0..QUICKSLOT_COUNT {
                    parent.spawn((
                        Button,
                        Node {
                            width: Val::Px(50.0),
                            height: Val::Px(50.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.3, 0.3, 0.3, 1.0)),
                        QuickSlotButton {
                            slot_index: i,
                        },
                    )).with_children(|parent| {
                        parent.spawn((
                            Text::new(format!("{}", (i + 1) % 10)),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(HUD_TEXT_COLOR),
                        ));
                    });
                }
            });
            
            // 右下角小地图
            parent.spawn((
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(200.0),
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(80.0),
                    right: Val::Px(10.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
                BorderColor::all(Color::srgba(0.8, 0.8, 0.8, 1.0)),
                MiniMap,
            ));
            
            // 左下角聊天面板
            parent.spawn((
                Node {
                    width: Val::Px(400.0),
                    height: Val::Px(150.0),
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(10.0),
                    left: Val::Px(10.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
                ChatPanel,
            )).with_children(|parent| {
                // 聊天消息列表
                parent.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(100.0),
                        overflow: Overflow::clip_y(),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(2.0),
                        padding: UiRect::all(Val::Px(5.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
                    ChatMessageList,
                ));
                
                // 聊天输入框
                parent.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(30.0),
                        padding: UiRect::all(Val::Px(5.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 1.0)),
                    ChatInput,
                )).with_children(|parent| {
                    parent.spawn((
                        Text::new("说点什么... (Enter 发送)"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.7, 0.7, 0.7, 1.0)),
                    ));
                });
            });
        });
    });
    
    info!("🎉 游戏场景设置完成!");
}

/// 清理游戏场景
pub fn cleanup_game_scene(
    mut commands: Commands,
    query: Query<Entity, With<GameSceneRoot>>,
) {
    info!("🧹 开始清理游戏场景");
    let mut count = 0;
    
    for entity in query.iter() {
        commands.entity(entity).despawn(); // Bevy会自动清理子节点
        count += 1;
    }
    
    commands.remove_resource::<GameSceneState>();
    info!("🧹 游戏场景已清理 ({} 个根实体)", count);
}

// ============================================================================
// Game Loop Systems - 基础游戏循环系统
// TODO: 考虑移到 game_loop_systems.rs (如果系统变得更复杂)
// ============================================================================

/// 更新游戏时间
pub fn update_game_time(
    mut state: ResMut<GameSceneState>,
    time: Res<Time>,
) {
    if !state.is_paused {
        state.game_time += time.delta_secs();
    }
}

/// 处理玩家输入
pub fn handle_player_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut events: Option<MessageWriter<PlayerMoveMessage>>,
) {
    // 处理移动输入 (WASD 或方向键)
    let mut moved = false;
    
    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        // 向上移动
        moved = true;
    }
    
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        // 向左移动
        moved = true;
    }
    
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        // 向下移动
        moved = true;
    }
    
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        // 向右移动
        moved = true;
    }
    
    // 处理聊天开启/关闭 (Enter)
    if keyboard.just_pressed(KeyCode::Enter) {
        info!("📝 打开聊天");
        // TODO: 发送 OpenChatMessage
    }
    
    // 处理快捷键 (1-0)
    let quickslot_keys = [
        KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4,
        KeyCode::Digit5, KeyCode::Digit6, KeyCode::Digit7, KeyCode::Digit8,
        KeyCode::Digit9, KeyCode::Digit0,
    ];
    for (i, &key) in quickslot_keys.iter().enumerate() {
        if keyboard.just_pressed(key) {
            info!("⚡ 快捷键 {} 被按下", i + 1);
            // TODO: 触发快捷技能
        }
    }
    
    // 处理暂停 (Esc)
    if keyboard.just_pressed(KeyCode::Escape) {
        info!("⏸️ 暂停游戏");
        // TODO: 发送 PauseGameMessage
    }
}

/// 处理玩家移动
pub fn handle_player_movement(
    mut player_query: Query<(&Player, &mut PlayerMovement)>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    for (_player, mut movement) in player_query.iter_mut() {
        // 获取移动方向
        let mut direction = Vec3::ZERO;
        
        if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
            direction.y += 1.0;
        }
        if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
            direction.y -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
            direction.x -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
            direction.x += 1.0;
        }
        
        // 更新移动状态
        if direction.length() > 0.0 {
            movement.direction = direction.normalize();
            movement.is_moving = true;
        } else {
            movement.is_moving = false;
            movement.direction = Vec3::ZERO;
        }
    }
}

/// 更新玩家位置 (基于移动速度和时间)
pub fn update_player_position(
    mut player_query: Query<(&PlayerMovement, &mut Transform)>,
    time: Res<Time>,
) {
    for (movement, mut transform) in player_query.iter_mut() {
        if movement.is_moving {
            let delta = movement.direction * movement.speed * time.delta_secs();
            transform.translation += delta;
        }
    }
}

// ============================================================================
// UI Update Systems
// ============================================================================

/// 更新 HUD 显示
pub fn update_hud_display(
    state: Res<GameSceneState>,
    mut query: Query<&mut Text, With<PlayerInfoHud>>,
) {
    for mut text in query.iter_mut() {
        if text.0.starts_with("Lv.") {
            text.0 = format!(
                "Lv. {} | 玩家 | EXP: {}",
                state.player_level, state.player_experience
            );
        } else if text.0.starts_with("HP:") {
            let health_percent = (state.player_health as f32 / state.player_max_health as f32 * 100.0) as i32;
            text.0 = format!(
                "HP: {}/{} ({}%)",
                state.player_health, state.player_max_health, health_percent
            );
        } else if text.0.starts_with("MP:") {
            let mana_percent = (state.player_mana as f32 / state.player_max_mana as f32 * 100.0) as i32;
            text.0 = format!(
                "MP: {}/{} ({}%)",
                state.player_mana, state.player_max_mana, mana_percent
            );
        }
    }
}

/// 处理快捷栏按钮悬停
pub fn handle_quickslot_hover(
    mut query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<QuickSlotButton>),
    >,
) {
    for (interaction, mut bg_color) in query.iter_mut() {
        match *interaction {
            Interaction::Hovered => {
                *bg_color = BackgroundColor(Color::srgba(0.5, 0.5, 0.5, 1.0));
            }
            Interaction::Pressed => {
                *bg_color = BackgroundColor(Color::srgba(0.7, 0.7, 0.7, 1.0));
            }
            Interaction::None => {
                *bg_color = BackgroundColor(Color::srgba(0.3, 0.3, 0.3, 1.0));
            }
        }
    }
}
