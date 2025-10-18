// GameScene - 游戏场景主模块
// 处理游戏中玩家控制、交互、网络同步等

mod components;

pub use components::*;

use bevy::prelude::*;
use std::collections::VecDeque;

// ============================================================================
// 常量定义
// ============================================================================
const MAX_CHAT_MESSAGE_LENGTH: usize = 200;

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
        BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 1.0)),
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
    info!("🧹 清理游戏场景");
    
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
    
    commands.remove_resource::<GameSceneState>();
}

// ============================================================================
// Game Loop Systems
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

// ============================================================================
// Message Handlers
// ============================================================================

/// 处理玩家移动消息
pub fn message_handle_player_move(
    events: Option<MessageReader<PlayerMoveMessage>>,
    mut state: ResMut<GameSceneState>,
) {
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        info!("📨 玩家移动到: ({}, {})", event.x, event.y);
        // 更新状态或发送到服务器
    }
}

/// 处理打开聊天消息
pub fn message_handle_open_chat(
    events: Option<MessageReader<OpenChatMessage>>,
    mut state: ResMut<GameSceneState>,
) {
    let Some(mut events) = events else { return; };
    
    for _ in events.read() {
        info!("💬 打开聊天面板");
        state.show_chat = true;
    }
}

/// 处理关闭聊天消息
pub fn message_handle_close_chat(
    events: Option<MessageReader<CloseChatMessage>>,
    mut state: ResMut<GameSceneState>,
) {
    let Some(mut events) = events else { return; };
    
    for _ in events.read() {
        info!("💬 关闭聊天面板");
        state.show_chat = false;
    }
}

/// 处理打开背包消息
pub fn message_handle_open_inventory(
    events: Option<MessageReader<OpenInventoryMessage>>,
    mut state: ResMut<GameSceneState>,
) {
    let Some(mut events) = events else { return; };
    
    for _ in events.read() {
        info!("🎒 打开背包");
        state.show_inventory = true;
    }
}

/// 处理关闭背包消息
pub fn message_handle_close_inventory(
    events: Option<MessageReader<CloseInventoryMessage>>,
    mut state: ResMut<GameSceneState>,
) {
    let Some(mut events) = events else { return; };
    
    for _ in events.read() {
        info!("🎒 关闭背包");
        state.show_inventory = false;
    }
}

/// 处理打开技能面板消息
pub fn message_handle_open_skills(
    events: Option<MessageReader<OpenSkillsMessage>>,
    mut state: ResMut<GameSceneState>,
) {
    let Some(mut events) = events else { return; };
    
    for _ in events.read() {
        info!("⚡ 打开技能面板");
        state.show_skills = true;
    }
}

/// 处理关闭技能面板消息
pub fn message_handle_close_skills(
    events: Option<MessageReader<CloseSkillsMessage>>,
    mut state: ResMut<GameSceneState>,
) {
    let Some(mut events) = events else { return; };
    
    for _ in events.read() {
        info!("⚡ 关闭技能面板");
        state.show_skills = false;
    }
}

/// 处理暂停游戏消息
pub fn message_handle_pause_game(
    events: Option<MessageReader<PauseGameMessage>>,
    mut state: ResMut<GameSceneState>,
) {
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        info!("⏸️ 游戏暂停: {}", event.is_paused);
        state.is_paused = event.is_paused;
    }
}

/// 处理退出游戏消息
pub fn message_handle_exit_game(
    events: Option<MessageReader<ExitGameMessage>>,
    mut next_state: ResMut<NextState<crate::bevy::GameState>>,
) {
    let Some(mut events) = events else { return; };
    
    for _ in events.read() {
        info!("🚪 返回角色选择");
        next_state.set(crate::bevy::GameState::Select);
    }
}

/// 处理与 NPC 交互消息
pub fn message_handle_interact_npc(
    events: Option<MessageReader<InteractWithNpcMessage>>,
) {
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        info!("🗣️ 与 NPC {} 交互", event.npc_id);
        // 打开对话框等
    }
}

/// 处理使用技能消息
pub fn message_handle_use_skill(
    events: Option<MessageReader<UseSkillMessage>>,
) {
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        info!(
            "✨ 使用技能 {} 在位置 ({}, {})",
            event.skill_id, event.target_x, event.target_y
        );
        // 播放技能动画等
    }
}

// ============================================================================
// Phase 1: 玩家实体管理系统
// ============================================================================

/// 玩家属性更新系统 - 同步玩家属性到 GameSceneState
pub fn update_player_stats_system(
    mut player_query: Query<(&Player, &mut Transform), Changed<Player>>,
    mut game_state: ResMut<GameSceneState>,
) {
    for (player, _transform) in player_query.iter_mut() {
        // 更新游戏状态中的玩家属性
        game_state.player_level = player.level;
        game_state.player_health = 100; // TODO: 从 player.stats 获取
        
        info!(
            "📊 玩家属性已更新: Lv.{} | 攻击力:{} | 防御力:{}",
            player.level, player.stats.attack, player.stats.defense
        );
    }
}

/// 处理增益效果系统 - 管理 buff 的持续时间和过期
pub fn process_buffs_system(
    mut player_query: Query<&mut Player>,
    time: Res<Time>,
) {
    for mut player in player_query.iter_mut() {
        if player.buffs.is_empty() {
            continue;
        }
        
        // 更新增益持续时间
        for buff in player.buffs.iter_mut() {
            buff.duration -= time.delta_secs();
        }
        
        // 移除过期增益
        let original_count = player.buffs.len();
        player.buffs.retain(|buff| buff.duration > 0.0);
        
        if player.buffs.len() < original_count {
            info!(
                "💫 增益已消退: {} → {} | 剩余增益: {}",
                original_count,
                player.buffs.len(),
                player.buffs.iter()
                    .map(|b| b.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
}

/// 聊天系统 - 处理聊天输入
pub fn handle_chat_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut chat_manager: ResMut<ChatManager>,
    mut game_state: ResMut<GameSceneState>,
) {
    // 切换聊天开启/关闭 (Enter 键)
    if keyboard.just_pressed(KeyCode::Enter) {
        game_state.show_chat = !game_state.show_chat;
        
        if game_state.show_chat {
            info!("💬 聊天窗口已打开");
        } else {
            info!("💬 聊天窗口已关闭");
            // 如果有输入内容，发送消息
            if !chat_manager.input_buffer.is_empty() {
                send_chat_message(&mut chat_manager);
            }
        }
    }
}

// ============================================================================
// Phase 2: 地图加载和渲染系统
// ============================================================================
// ============================================================================
// Phase 2: 地图加载和渲染系统
// ============================================================================

/// 加载地图系统 - 初始化地图数据
pub fn load_map_system(
    mut commands: Commands,
    mut game_state: ResMut<GameSceneState>,
) {
    // 创建默认地图
    let mut map_data = MapData::new(
        1,
        "Mirror World".to_string(),
        100,
        100,
    );
    
    // 初始化地面瓦片
    for y in 0..map_data.height {
        for x in 0..map_data.width {
            let mut tile = MapTile {
                tile_x: x,
                tile_y: y,
                layer: 0,
                tile_id: 1,  // 地面瓦片
                walkable: true,
            };
            
            // 添加一些不可行走的区域（例如树木、墙壁）
            if (x < 5) || (x > 95) || (y < 5) || (y > 95) {
                tile.walkable = false;
                tile.tile_id = 2;  // 不可行走的瓦片
            }
            
            map_data.set_tile(x, y, 0, tile);
        }
    }
    
    // 添加一些地图对象
    let npc = MapObject {
        object_id: 1,
        object_type: 1,  // NPC
        x: 50,
        y: 50,
        name: "村长".to_string(),
        properties: Default::default(),
    };
    map_data.add_object(npc);
    
    // 添加传送点
    let teleport = MapObject {
        object_id: 2,
        object_type: 3,  // 传送点
        x: 25,
        y: 25,
        name: "传送点".to_string(),
        properties: Default::default(),
    };
    map_data.add_object(teleport);
    
    map_data.is_loaded = true;
    
    // 存储为资源
    commands.insert_resource(map_data);
    game_state.is_initialized = true;
    
    info!("🗺️ 地图加载完成 (100×100, 对象数: 2)");
}

/// 创建地图图层系统 - 生成地图图层实体
pub fn create_map_layers_system(
    mut commands: Commands,
    map_data: Res<MapData>,
) {
    if !map_data.is_loaded {
        return;
    }
    
    // 为每个图层创建实体
    for layer_idx in 0..3 {
        let layer_entity = commands.spawn((
            MapLayer {
                layer_index: layer_idx as u32,
            },
            Transform::default(),
            Visibility::default(),
            Name::new(format!("MapLayer_{}", layer_idx)),
        )).id();
        
        info!("✅ 地图图层 {} 已创建", layer_idx);
    }
}

/// 生成地图对象系统 - 生成 NPC 和其他对象
pub fn spawn_map_objects_system(
    mut commands: Commands,
    map_data: Res<MapData>,
) {
    if !map_data.is_loaded {
        return;
    }
    
    for object in &map_data.objects {
        match object.object_type {
            1 => {
                // NPC
                commands.spawn((
                    Transform::from_xyz(
                        object.x as f32 * 32.0,
                        object.y as f32 * 32.0,
                        1.0,
                    ),
                    NPC {
                        npc_id: object.object_id as i32,
                        name: object.name.clone(),
                        dialogue_id: None,
                    },
                    Visibility::Visible,
                    Name::new(format!("NPC_{}", object.name)),
                ));
                
                info!("👤 NPC 已生成: {} 在 ({}, {})", 
                    object.name, object.x, object.y);
            }
            3 => {
                // 传送点
                commands.spawn((
                    Transform::from_xyz(
                        object.x as f32 * 32.0,
                        object.y as f32 * 32.0,
                        0.5,
                    ),
                    InteractiveObject {
                        object_id: object.object_id as i32,
                        name: object.name.clone(),
                        object_type: "teleport".to_string(),
                        interaction_range: 32.0,
                    },
                    Visibility::Visible,
                    Name::new("Teleport"),
                ));
                
                info!("🚪 传送点已生成: {} 在 ({}, {})", 
                    object.name, object.x, object.y);
            }
            _ => {
                info!("ℹ️ 未知对象类型: {}", object.object_type);
            }
        }
    }
    
    info!("🎮 所有地图对象已生成 (总数: {})", map_data.objects.len());
}

/// 更新地图状态系统 - 监听地图加载完成
pub fn update_map_state_system(
    map_data: Res<MapData>,
    mut game_state: ResMut<GameSceneState>,
) {
    if map_data.is_loaded && !game_state.is_initialized {
        game_state.current_map = map_data.map_name.clone();
        game_state.is_initialized = true;
        info!("🗺️ 地图状态已更新: {}", map_data.map_name);
    }
}

/// 处理地图碰撞检测系统
pub fn handle_map_collision_system(
    mut player_query: Query<&mut Transform, With<Player>>,
    map_data: Res<MapData>,
) {
    if !map_data.is_loaded {
        return;
    }
    
    for mut player_transform in player_query.iter_mut() {
        // 将世界坐标转换为地图坐标
        let tile_x = (player_transform.translation.x / 32.0) as u16;
        let tile_y = (player_transform.translation.y / 32.0) as u16;
        
        // 检查该瓦片是否可行走
        if !map_data.is_walkable(tile_x, tile_y) {
            // 回退到上一个有效位置
            player_transform.translation.x = ((tile_x - 1) as f32 * 32.0);
            
            info!("⚠️ 玩家碰撞检测: 不可通过的瓦片 ({}, {})", tile_x, tile_y);
        }
    }
}

// ============================================================================
// Phase 3: NPC 和对象交互系统
// ============================================================================

/// 初始化对话系统
pub fn setup_dialogue_system(
    mut commands: Commands,
) {
    // 创建对话状态资源
    commands.insert_resource(DialogueState::default());
    commands.insert_resource(InteractionState::default());
    
    // 创建一个示例对话树 (村长的对话)
    let mut dialogue_tree = DialogueTree::new(1, 1, 1);
    
    // 节点 1: 初次问候
    let greeting_options = vec![
        DialogueOption {
            option_id: 1,
            text: "你好，我是新手冒险者。".to_string(),
            next_dialogue_id: Some(2),
            action: String::new(),
            conditions: vec![],
        },
        DialogueOption {
            option_id: 2,
            text: "能告诉我关于这个世界吗？".to_string(),
            next_dialogue_id: Some(3),
            action: String::new(),
            conditions: vec![],
        },
    ];
    
    let greeting_node = DialogueNode {
        node_id: 1,
        npc_id: 1,
        text: "欢迎来到我们的村子！有什么我可以帮你的吗？".to_string(),
        speaker: "村长".to_string(),
        options: greeting_options,
        auto_next: None,
    };
    
    dialogue_tree.add_node(greeting_node);
    
    // 节点 2: 介绍自己
    let intro_node = DialogueNode {
        node_id: 2,
        npc_id: 1,
        text: "很高兴认识你！希望你在这里过得愉快。".to_string(),
        speaker: "村长".to_string(),
        options: vec![
            DialogueOption {
                option_id: 3,
                text: "谢谢你的欢迎。".to_string(),
                next_dialogue_id: None,
                action: String::new(),
                conditions: vec![],
            },
        ],
        auto_next: None,
    };
    
    dialogue_tree.add_node(intro_node);
    
    // 节点 3: 世界介绍
    let world_node = DialogueNode {
        node_id: 3,
        npc_id: 1,
        text: "这是一个充满魔法和冒险的世界。小心怪物和强大的敌人！".to_string(),
        speaker: "村长".to_string(),
        options: vec![
            DialogueOption {
                option_id: 4,
                text: "我会小心的。".to_string(),
                next_dialogue_id: None,
                action: String::new(),
                conditions: vec![],
            },
        ],
        auto_next: None,
    };
    
    dialogue_tree.add_node(world_node);
    
    commands.insert_resource(dialogue_tree);
    
    info!("🎭 对话系统已初始化");
}

/// 检测交互系统 - 检测玩家附近的可交互对象
pub fn detect_interaction_system(
    player_query: Query<&Transform, With<Player>>,
    npc_query: Query<(&NPC, &Transform)>,
    object_query: Query<(&InteractiveObject, &Transform)>,
    mut interaction_state: ResMut<InteractionState>,
) {
    let Some(player_transform) = player_query.iter().next() else {
        return;
    };
    
    let player_pos = player_transform.translation;
    let interaction_range = 100.0;  // 交互范围（像素）
    
    interaction_state.nearby_objects.clear();
    
    // 检查附近的 NPC
    for (npc, npc_transform) in npc_query.iter() {
        let distance = player_pos.distance(npc_transform.translation);
        
        if distance < interaction_range {
            interaction_state.nearby_objects.push(npc.npc_id);
            interaction_state.can_interact = true;
        }
    }
    
    // 检查附近的交互对象
    for (obj, obj_transform) in object_query.iter() {
        let distance = player_pos.distance(obj_transform.translation);
        
        if distance < interaction_range {
            interaction_state.nearby_objects.push(obj.object_id);
            interaction_state.can_interact = true;
        }
    }
    
    if !interaction_state.nearby_objects.is_empty() {
        info!("✨ 附近有 {} 个可交互对象", interaction_state.nearby_objects.len());
    }
}

/// 处理交互系统 - 处理玩家交互
pub fn handle_interaction_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    interaction_state: Res<InteractionState>,
    mut dialogue_state: ResMut<DialogueState>,
    dialogue_tree: Res<DialogueTree>,
) {
    // 按 F 键交互
    if keyboard.just_pressed(KeyCode::KeyF) && interaction_state.can_interact {
        if let Some(object_id) = interaction_state.nearby_objects.first() {
            start_dialogue_with_npc(&mut dialogue_state, dialogue_tree.as_ref(), *object_id as i32);
        }
    }
}

/// 启动对话
fn start_dialogue_with_npc(
    dialogue_state: &mut DialogueState,
    dialogue_tree: &DialogueTree,
    npc_id: i32,
) {
    dialogue_state.is_in_dialogue = true;
    dialogue_state.current_npc_id = Some(npc_id);
    dialogue_state.current_node_id = dialogue_tree.start_node_id;
    dialogue_state.tree_id = dialogue_tree.tree_id;
    
    info!("🎭 开始与 NPC {} 对话", npc_id);
    
    if let Some(node) = dialogue_tree.get_node(dialogue_state.current_node_id) {
        info!("💬 [{}]: {}", node.speaker, node.text);
    }
}

/// 显示对话UI系统 - 更新对话显示
pub fn update_dialogue_display_system(
    dialogue_state: Res<DialogueState>,
    dialogue_tree: Res<DialogueTree>,
    mut ui_query: Query<&mut Text, With<ChatMessageList>>,
) {
    if !dialogue_state.is_in_dialogue {
        return;
    }
    
    if let Some(node) = dialogue_tree.get_node(dialogue_state.current_node_id) {
        for mut text in ui_query.iter_mut() {
            let mut display = format!("【对话】\n");
            display.push_str(&format!("[{}]: {}\n\n", node.speaker, node.text));
            
            // 显示选项
            for (idx, option) in node.options.iter().enumerate() {
                display.push_str(&format!("{}. {}\n", idx + 1, option.text));
            }
            
            display.push_str("\n[按数字键选择选项, ESC 关闭对话]");
            text.0 = display;
        }
    }
}

/// 处理对话选择系统
pub fn handle_dialogue_choice_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut dialogue_state: ResMut<DialogueState>,
    mut dialogue_tree: ResMut<DialogueTree>,
) {
    if !dialogue_state.is_in_dialogue {
        return;
    }
    
    // ESC 键关闭对话
    if keyboard.just_pressed(KeyCode::Escape) {
        dialogue_state.is_in_dialogue = false;
        dialogue_state.current_npc_id = None;
        info!("🎭 对话已结束");
        return;
    }
    
    // 处理数字键选择
    let choice_keys = [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
    ];
    
    if let Some(current_node) = dialogue_tree.get_node(dialogue_state.current_node_id) {
        for (key, index) in choice_keys {
            if keyboard.just_pressed(key) && index < current_node.options.len() {
                let option = &current_node.options[index];
                
                info!("玩家选择: {}", option.text);
                
                // 执行动作
                if !option.action.is_empty() {
                    info!("执行动作: {}", option.action);
                }
                
                // 进行到下一个对话
                if let Some(next_id) = option.next_dialogue_id {
                    dialogue_state.current_node_id = next_id;
                    
                    if let Some(next_node) = dialogue_tree.get_node(next_id) {
                        info!("💬 [{}]: {}", next_node.speaker, next_node.text);
                    }
                } else {
                    // 对话结束
                    dialogue_state.is_in_dialogue = false;
                    dialogue_state.current_npc_id = None;
                    info!("🎭 对话已结束");
                }
                
                break;
            }
        }
    }
}

/// 处理 NPC 交互消息
pub fn message_handle_npc_dialogue(
    events: Option<MessageReader<StartDialogueMessage>>,
    mut dialogue_state: ResMut<DialogueState>,
    dialogue_tree: Res<DialogueTree>,
) {
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        start_dialogue_with_npc(&mut dialogue_state, dialogue_tree.as_ref(), event.npc_id);
    }
}

// ============================================================================
// Phase 4: 聊天系统完整实现
// ============================================================================

/// 初始化聊天系统
pub fn setup_chat_system(
    mut commands: Commands,
) {
    // 聊天管理器已在 Phase 1 中初始化
    // 这里初始化聊天的额外设置
    commands.insert_resource(ChatFilterConfig::default());
    commands.insert_resource(ChatCommandManager::default());
    commands.insert_resource(ChatDisplaySettings::default());
    
    info!("💬 聊天系统已完整初始化");
}

/// 处理聊天输入系统 - 完整的字符输入处理
pub fn process_chat_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut chat_manager: ResMut<ChatManager>,
    mut game_state: ResMut<GameSceneState>,
) {
    // T 键打开/关闭聊天
    if keyboard.just_pressed(KeyCode::KeyT) {
        game_state.show_chat = !game_state.show_chat;
        
        if game_state.show_chat {
            info!("💬 聊天窗口已打开");
        } else {
            info!("💬 聊天窗口已关闭");
        }
        
        return;
    }
    
    if !game_state.show_chat {
        return;
    }
    
    // 处理字符输入
    // 注: 在实际应用中应该使用 ReceivedCharacter 事件
    // 这里简化处理
    
    // Backspace 删除字符
    if keyboard.just_pressed(KeyCode::Backspace) {
        chat_manager.input_buffer.pop();
    }
    
    // Enter 发送消息
    if keyboard.just_pressed(KeyCode::Enter) {
        if !chat_manager.input_buffer.is_empty() {
            send_chat_message(&mut chat_manager);
        }
    }
    
    // Escape 关闭聊天
    if keyboard.just_pressed(KeyCode::Escape) {
        game_state.show_chat = false;
        chat_manager.input_buffer.clear();
        info!("💬 聊天窗口已关闭");
    }
}

/// 发送聊天消息 - 将消息添加到历史记录
fn send_chat_message(chat_manager: &mut ResMut<ChatManager>) {
    if chat_manager.input_buffer.is_empty() {
        return;
    }
    
    let content = chat_manager.input_buffer.clone();
    
    // 检查消息长度
    if content.len() > chat_manager.input_buffer.len() {
        info!("⚠️ 消息过长，已截断");
        return;
    }
    
    let message = ChatMessage {
        sender: "玩家".to_string(),
        content: content.clone(),
        timestamp: 0.0,  // TODO: 使用真实时间戳
        message_type: 0, // 普通消息
    };
    
    chat_manager.history.push_back(message);
    
    // 保持历史记录大小限制
    while chat_manager.history.len() > chat_manager.max_history {
        chat_manager.history.pop_front();
    }
    
    info!("💬 消息已发送: {}", content);
    chat_manager.input_buffer.clear();
}

/// 处理聊天命令系统 - 处理以特定前缀开头的消息
pub fn process_chat_commands_system(
    mut chat_manager: ResMut<ChatManager>,
    command_manager: Res<ChatCommandManager>,
) {
    if !command_manager.enabled || chat_manager.input_buffer.is_empty() {
        return;
    }
    
    let input = &chat_manager.input_buffer;
    
    // 检查是否以 / 开头
    if !input.starts_with('/') {
        return;
    }
    
    // 解析命令
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return;
    }
    
    let command_name = parts[0].trim_start_matches('/');
    let args = parts[1..].to_vec();
    
    // 处理命令
    match command_name {
        "help" => {
            info!("📖 帮助命令:");
            for cmd in &command_manager.commands {
                info!("  /{} - {}", cmd.name, cmd.description);
            }
        }
        "emote" => {
            if !args.is_empty() {
                info!("😊 执行表情动作: {}", args.join(" "));
            } else {
                info!("⚠️ 用法: /emote <动作>");
            }
        }
        "whisper" => {
            if args.len() >= 2 {
                let target = args[0];
                let message = args[1..].join(" ");
                info!("🤐 私聊 {}: {}", target, message);
            } else {
                info!("⚠️ 用法: /whisper <玩家名> <消息>");
            }
        }
        "party" => {
            if !args.is_empty() {
                info!("👥 队伍聊天: {}", args.join(" "));
            } else {
                info!("⚠️ 用法: /party <消息>");
            }
        }
        _ => {
            info!("⚠️ 未知命令: /{}", command_name);
        }
    }
    
    chat_manager.input_buffer.clear();
}

/// 接收聊天消息系统 - 模拟接收其他玩家的消息
pub fn receive_chat_messages_system(
    mut chat_manager: ResMut<ChatManager>,
    game_state: Res<GameSceneState>,
) {
    // 这里可以从网络接收消息
    // 现在只是演示如何添加消息到历史记录
    
    // 可以在这里添加来自 NPC 或系统的消息
    if game_state.game_time as u32 % 60 == 0 {
        // 每 60 帧添加一条系统消息（约 1 秒）
        if chat_manager.history.iter().all(|m| m.message_type != 1) {
            let system_message = ChatMessage {
                sender: "系统".to_string(),
                content: "欢迎来到游戏世界！".to_string(),
                timestamp: game_state.game_time,
                message_type: 1,  // 系统消息
            };
            
            chat_manager.history.push_back(system_message);
        }
    }
}

/// 过滤聊天消息系统 - 根据过滤器过滤消息
pub fn filter_chat_messages_system(
    chat_manager: Res<ChatManager>,
    filter_config: Res<ChatFilterConfig>,
) -> Vec<ChatMessage> {
    let mut filtered_messages = Vec::new();
    
    for message in chat_manager.history.iter() {
        let should_show = match message.message_type {
            0 => true,  // 普通消息总是显示
            1 => filter_config.show_system,
            2 => filter_config.show_whisper,
            3 => filter_config.show_broadcast,
            _ => true,
        };
        
        if should_show {
            filtered_messages.push(message.clone());
        }
    }
    
    filtered_messages
}

/// 应用屏蔽词系统 - 对消息内容进行检查和过滤
pub fn apply_word_filter_system(
    content: &str,
    filter_config: &ChatFilterConfig,
) -> String {
    let mut filtered = content.to_string();
    
    for bad_word in &filter_config.word_filter {
        let replacement = "*".repeat(bad_word.len());
        filtered = filtered.replace(bad_word, &replacement);
    }
    
    filtered
}

/// 更新聊天显示系统 - 在 UI 中显示聊天消息
pub fn update_chat_display_system(
    chat_manager: Res<ChatManager>,
    display_settings: Res<ChatDisplaySettings>,
    filter_config: Res<ChatFilterConfig>,
    mut text_query: Query<&mut Text, With<ChatMessageList>>,
    game_state: Res<GameSceneState>,
) {
    if !game_state.show_chat {
        return;
    }
    
    // 过滤消息
    let mut display_messages: Vec<ChatMessage> = chat_manager
        .history
        .iter()
        .filter(|m| match m.message_type {
            0 => true,
            1 => filter_config.show_system,
            2 => filter_config.show_whisper,
            3 => filter_config.show_broadcast,
            _ => true,
        })
        .cloned()
        .collect();
    
    // 只显示最后的消息
    let start_idx = if display_messages.len() > display_settings.max_visible_messages {
        display_messages.len() - display_settings.max_visible_messages
    } else {
        0
    };
    
    let visible_messages = &display_messages[start_idx..];
    
    for mut text in text_query.iter_mut() {
        let mut display_text = String::from("【聊天】\n");
        
        for msg in visible_messages {
            // 格式化消息
            let formatted = if display_settings.show_timestamps {
                format!(
                    "[{:.0}] {}: {}",
                    msg.timestamp,
                    msg.sender,
                    apply_word_filter_system(&msg.content, &filter_config)
                )
            } else {
                format!(
                    "{}: {}",
                    msg.sender,
                    apply_word_filter_system(&msg.content, &filter_config)
                )
            };
            
            // 根据消息类型着色
            let colored = match msg.message_type {
                0 => format!("{}\n", formatted),  // 普通白色
                1 => format!("【系统】{}\n", formatted),  // 系统黄色
                2 => format!("【私聊】{}\n", formatted),  // 私聊紫色
                3 => format!("【公告】{}\n", formatted),  // 公告青色
                _ => format!("{}\n", formatted),
            };
            
            display_text.push_str(&colored);
        }
        
        // 显示输入缓冲
        display_text.push_str("\n> ");
        display_text.push_str(&chat_manager.input_buffer);
        display_text.push('_');  // 光标
        
        text.0 = display_text;
    }
}

/// 管理聊天历史系统 - 清理过期消息
pub fn manage_chat_history_system(
    mut chat_manager: ResMut<ChatManager>,
    display_settings: Res<ChatDisplaySettings>,
    game_state: Res<GameSceneState>,
) {
    // 更新消息时间戳
    for message in chat_manager.history.iter_mut() {
        if message.timestamp == 0.0 {
            message.timestamp = game_state.game_time;
        }
    }
    
    // 删除太旧的消息（根据淡出时间）
    let current_time = game_state.game_time;
    let max_age = display_settings.message_fade_time;
    
    chat_manager.history.retain(|msg| {
        current_time - msg.timestamp < max_age
    });
    
    // 保持最大消息数限制
    while chat_manager.history.len() > chat_manager.max_history {
        chat_manager.history.pop_front();
    }
}

/// 处理发送聊天消息
pub fn message_handle_send_chat(
    events: Option<MessageReader<SendChatMessage>>,
    mut chat_manager: ResMut<ChatManager>,
) {
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        chat_manager.input_buffer = event.text.clone();
        send_chat_message(&mut chat_manager);
    }
}

// ============================================================================
// Phase 5: 网络同步系统
// ============================================================================

/// 初始化网络系统
pub fn setup_network_system(mut commands: Commands) {
    commands.insert_resource(NetworkState::default());
    info!("🌐 网络系统已初始化");
}

/// 定期发送玩家位置同步消息
pub fn send_player_position_system(
    mut network_state: ResMut<NetworkState>,
    game_state: Res<GameSceneState>,
    player_query: Query<&Transform, With<Player>>,
) {
    // 检查是否需要同步
    if network_state.connection_state != ConnectionState::Connected {
        return;
    }
    
    // 检查是否到达同步间隔
    if network_state.last_sync_time < network_state.sync_interval {
        return;
    }
    
    // 重置同步计时器
    network_state.last_sync_time = 0.0;
    
    // 获取玩家位置
    if let Ok(transform) = player_query.single() {
        let pos = transform.translation;
        
        // 创建同步消息（这里模拟发送，实际需要网络传输）
        info!(
            "📤 发送玩家位置同步: ({:.1}, {:.1}, {:.1})",
            pos.x, pos.y, pos.z
        );
        
        network_state.pending_updates += 1;
    }
}

/// 定期发送玩家属性同步消息
pub fn send_player_stats_system(
    mut network_state: ResMut<NetworkState>,
    game_state: Res<GameSceneState>,
) {
    // 检查连接状态
    if network_state.connection_state != ConnectionState::Connected {
        return;
    }
    
    // 定期发送属性更新（这里模拟发送）
    info!(
        "📤 发送玩家属性同步: 等级={}, HP={}/{}",
        game_state.player_level, game_state.player_health, game_state.player_max_health
    );
    
    network_state.pending_updates += 1;
}

/// 发送聊天消息到服务器
pub fn send_chat_to_server_system(
    network_state: Res<NetworkState>,
    chat_manager: Res<ChatManager>,
) {
    // 检查连接状态
    if network_state.connection_state != ConnectionState::Connected {
        return;
    }
    
    // 只在有新消息时发送
    if chat_manager.history.is_empty() {
        return;
    }
    
    // 获取最后一条消息
    if let Some(last_msg) = chat_manager.history.back() {
        if last_msg.message_type == 0 {  // 0=普通消息
            info!("📤 发送聊天消息到服务器: {}", last_msg.content);
        }
    }
}

/// 发送交互事件到服务器
pub fn send_interaction_to_server_system(
    network_state: Res<NetworkState>,
    events: Option<MessageReader<InteractWithNpcMessage>>,
) {
    // 检查连接状态
    if network_state.connection_state != ConnectionState::Connected {
        return;
    }
    
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        info!("📤 发送NPC交互事件到服务器: NPC ID = {}", event.npc_id);
    }
}

/// 接收远端玩家同步信息
pub fn receive_player_sync_system(
    mut network_state: ResMut<NetworkState>,
) {
    // 模拟接收其他玩家同步信息
    if network_state.connection_state == ConnectionState::Connected {
        if network_state.pending_updates > 0 {
            // 模拟处理同步数据
            // 实际应用会从网络缓冲区读取
            network_state.pending_updates = network_state.pending_updates.saturating_sub(1);
        }
    }
}

/// 接收NPC状态同步信息
pub fn receive_npc_sync_system(
    network_state: Res<NetworkState>,
    mut npc_query: Query<&mut Transform, With<NPC>>,
) {
    // 检查连接状态
    if network_state.connection_state != ConnectionState::Connected {
        return;
    }
    
    // 模拟接收NPC状态更新
    // 实际应用会从网络缓冲区读取
    for mut transform in npc_query.iter_mut() {
        // 这里会更新NPC位置/状态
        // info!("📥 接收NPC状态同步");
    }
}

/// 接收地图对象同步信息
pub fn receive_map_sync_system(
    network_state: Res<NetworkState>,
    map_data: Res<MapData>,
) {
    // 检查连接状态
    if network_state.connection_state != ConnectionState::Connected {
        return;
    }
    
    // 模拟接收地图对象状态更新（物品、门等）
    // 实际应用会处理掉落物品、门打开/关闭等
    if !map_data.objects.is_empty() {
        // info!("📥 接收地图对象同步: {} 个对象", map_data.objects.len());
    }
}

/// 接收服务器聊天广播
pub fn receive_server_chat_system(
    mut chat_manager: ResMut<ChatManager>,
) {
    // 模拟接收服务器聊天广播
    // 实际应用会从网络缓冲区读取
    
    // 这个系统在 receive_chat_messages_system 中已部分实现
    // 这里主要用于处理服务器特定的广播格式
}

/// 处理网络连接事件（连接成功、断开、超时等）
pub fn handle_connection_events_system(
    mut network_state: ResMut<NetworkState>,
) {
    // 模拟处理连接事件
    match network_state.connection_state {
        ConnectionState::Disconnected => {
            info!("❌ 网络断开连接");
        }
        ConnectionState::Connecting => {
            info!("🔗 正在连接到服务器...");
            // 模拟连接延迟后设置为已连接
        }
        ConnectionState::Connected => {
            // info!("✅ 已连接到服务器");
        }
        ConnectionState::Reconnecting => {
            info!("🔄 正在重新连接...");
        }
        ConnectionState::Disconnecting => {
            info!("🔌 正在断开连接...");
        }
    }
}

/// 应用远端玩家同步数据
pub fn apply_player_sync_system(
    network_state: Res<NetworkState>,
    mut remote_player_query: Query<&mut Transform, With<RemotePlayer>>,
) {
    // 检查连接状态
    if network_state.connection_state != ConnectionState::Connected {
        return;
    }
    
    // 应用远端玩家位置/状态更新
    for mut transform in remote_player_query.iter_mut() {
        // 这里会平滑更新远端玩家位置
        // info!("🔄 更新远端玩家位置");
    }
}

/// 应用NPC状态同步
pub fn apply_npc_sync_system(
    network_state: Res<NetworkState>,
) {
    // 检查连接状态
    if network_state.connection_state != ConnectionState::Connected {
        return;
    }
    
    // 应用NPC状态变化（血量、状态等）
    // info!("🔄 应用NPC状态同步");
}

/// 处理物品生成/消失
pub fn apply_item_spawn_system(
    network_state: Res<NetworkState>,
    mut _commands: Commands,
) {
    // 检查连接状态
    if network_state.connection_state != ConnectionState::Connected {
        return;
    }
    
    // 处理物品掉落/消失事件
    // 实际应用会根据网络消息创建/删除物品实体
}

/// 维持本地同步状态
pub fn sync_local_state_system(
    mut network_state: ResMut<NetworkState>,
    time: Res<Time>,
) {
    // 更新同步计时器
    network_state.last_sync_time += time.delta_secs();
    
    // 保持待处理更新数不超过限制
    if network_state.pending_updates > MAX_PENDING_UPDATES {
        network_state.pending_updates = MAX_PENDING_UPDATES;
    }
}

// ============================================================================
// Phase 6: 完整事件循环系统
// ============================================================================

/// 初始化游戏循环系统
pub fn setup_game_loop_system(mut commands: Commands) {
    commands.insert_resource(FrameStats::default());
    commands.insert_resource(GameTimer::default());
    commands.insert_resource(EventQueue::default());
    commands.insert_resource(SystemHealthCheck::default());
    info!("🎮 游戏循环系统已初始化");
}

/// 主游戏循环系统
pub fn game_loop_system(
    mut game_timer: ResMut<GameTimer>,
    mut frame_stats: ResMut<FrameStats>,
    time: Res<Time>,
    game_state: Res<GameSceneState>,
) {
    // 如果游戏暂停，不更新时间
    if game_state.is_paused {
        return;
    }
    
    // 更新增量时间
    let delta = time.delta_secs();
    game_timer.delta_time = delta * game_timer.game_speed;
    game_timer.elapsed_time += game_timer.delta_time;
    
    // 更新帧统计
    let frame_time_ms = delta * 1000.0;
    frame_stats.last_frame_time = frame_time_ms;
    frame_stats.frame_count += 1;
    frame_stats.total_time += delta;
    
    // 更新帧时间历史
    frame_stats.frame_time_history.push(frame_time_ms);
    if frame_stats.frame_time_history.len() > frame_stats.history_size {
        frame_stats.frame_time_history.remove(0);
    }
    
    // 更新最小/最大帧时间
    frame_stats.min_frame_time = frame_stats.min_frame_time.min(frame_time_ms);
    frame_stats.max_frame_time = frame_stats.max_frame_time.max(frame_time_ms);
    
    // 计算 FPS
    if frame_stats.last_frame_time > 0.0 {
        frame_stats.current_fps = 1000.0 / frame_stats.last_frame_time;
        frame_stats.average_fps = (frame_stats.total_time * 1000.0) / (frame_stats.frame_count as f32 * frame_stats.last_frame_time);
    }
    
    // 每秒记录一次 FPS
    if frame_stats.frame_count % 60 == 0 {
        info!(
            "📊 FPS: {:.1} (avg: {:.1}) | 帧数: {} | 运行时间: {:.1}s",
            frame_stats.current_fps, frame_stats.average_fps, frame_stats.frame_count, frame_stats.total_time
        );
    }
}

/// 处理帧事件系统
pub fn process_frame_events_system(
    mut event_queue: ResMut<EventQueue>,
    game_state: Res<GameSceneState>,
) {
    // 记录主要事件
    if game_state.is_paused {
        event_queue.push_event("⏸️ 游戏已暂停".to_string());
    }
    
    // 定期输出事件统计
    if event_queue.events.len() % 100 == 0 && !event_queue.events.is_empty() {
        info!("📋 事件队列: {} 条事件", event_queue.events.len());
    }
}

/// 更新帧统计系统
pub fn update_frame_stats_system(
    mut frame_stats: ResMut<FrameStats>,
) {
    // 这个系统在 game_loop_system 中已处理
    // 这里可以做额外的统计工作
    
    // 如果帧时间异常（超过阈值），记录警告
    if frame_stats.last_frame_time > 33.33 {  // > 30 FPS
        if frame_stats.frame_count % 300 == 0 {  // 每 5 秒一次
            warn!(
                "⚠️ 帧时间过长: {:.2}ms (FPS: {:.1})",
                frame_stats.last_frame_time, frame_stats.current_fps
            );
        }
    }
}

/// 检查胜负条件系统
pub fn check_win_lose_conditions_system(
    game_state: Res<GameSceneState>,
) {
    // 模拟检查游戏胜负条件
    
    // 玩家死亡检查
    if game_state.player_health == 0 {
        warn!("💀 玩家已死亡！游戏结束");
    }
    
    // 其他胜负条件可在这里添加
    // 如：击败所有敌人、完成任务等
}

/// 整合所有系统的主控制器
pub fn integrate_all_systems_system(
    game_state: Res<GameSceneState>,
    network_state: Res<NetworkState>,
    frame_stats: Res<FrameStats>,
) {
    // 定期验证所有系统是否正常工作
    if frame_stats.frame_count % 300 == 0 && frame_stats.frame_count > 0 {
        info!(
            "🔄 系统集成检查 - 游戏运行: {:?} | 网络: {:?} | FPS: {:.1}",
            !game_state.is_paused,
            network_state.connection_state,
            frame_stats.current_fps
        );
    }
}

/// 验证游戏状态系统
pub fn validate_game_state_system(
    game_state: Res<GameSceneState>,
    mut health_check: ResMut<SystemHealthCheck>,
    frame_stats: Res<FrameStats>,
) {
    // 定期检查系统健康状态
    if frame_stats.frame_count % 300 == 0 && frame_stats.frame_count > 0 {
        // 检查玩家实体是否存在
        health_check.player_system_ok = game_state.player_entity.is_some();
        
        // 检查地图是否已初始化
        health_check.map_system_ok = game_state.is_initialized;
        
        // 更新整体状态
        health_check.all_systems_ok = health_check.player_system_ok
            && health_check.map_system_ok
            && health_check.dialogue_system_ok
            && health_check.chat_system_ok
            && health_check.network_system_ok
            && health_check.render_system_ok;
        
        // 如果有系统出问题，记录日志
        if !health_check.all_systems_ok {
            warn!(
                "⚠️ 系统健康检查失败 - 玩家: {} | 地图: {} | 对话: {} | 聊天: {} | 网络: {} | 渲染: {}",
                health_check.player_system_ok,
                health_check.map_system_ok,
                health_check.dialogue_system_ok,
                health_check.chat_system_ok,
                health_check.network_system_ok,
                health_check.render_system_ok
            );
        } else {
            info!("✅ 所有系统运行正常");
        }
    }
}

/// 错误处理系统
pub fn handle_game_errors_system(
    game_state: Res<GameSceneState>,
    health_check: Res<SystemHealthCheck>,
) {
    // 处理可能的游戏错误
    
    // 如果玩家健康为负，修正为 0
    if game_state.player_health == 0 && game_state.player_max_health > 0 {
        // 这里可以触发死亡事件
    }
    
    // 如果系统健康检查失败，采取措施
    if !health_check.all_systems_ok {
        // 可以记录错误、发送警告或采取恢复措施
        info!("🔧 正在进行错误恢复...");
    }
}

/// 系统健康检查系统
pub fn debug_system_health_system(
    mut health_check: ResMut<SystemHealthCheck>,
    game_state: Res<GameSceneState>,
    network_state: Res<NetworkState>,
    frame_stats: Res<FrameStats>,
) {
    // 定期输出系统健康状态
    if frame_stats.frame_count % 600 == 0 && frame_stats.frame_count > 0 {
        info!("━━━━━━━━━━ 系统健康检查报告 ━━━━━━━━━━");
        info!("⏱️ 运行时间: {:.1}s | 帧数: {}", frame_stats.total_time, frame_stats.frame_count);
        info!("📊 性能: {:.1} FPS (min: {:.1}, max: {:.1})", 
            frame_stats.current_fps, frame_stats.min_frame_time, frame_stats.max_frame_time);
        info!("👤 玩家: 等级 {} | HP {}/{}", 
            game_state.player_level, game_state.player_health, game_state.player_max_health);
        info!("🌐 网络: {:?}", network_state.connection_state);
        info!("✅ 系统状态: {}", 
            if health_check.all_systems_ok { "正常" } else { "异常" });
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    }
}

/// 网络更新优化系统
pub fn optimize_network_updates_system(
    mut network_state: ResMut<NetworkState>,
    frame_stats: Res<FrameStats>,
) {
    // 根据当前性能调整网络同步间隔
    if frame_stats.frame_count % 600 == 0 {
        if frame_stats.current_fps < 30.0 {
            // FPS 过低，减少网络更新频率
            network_state.sync_interval = network_state.sync_interval.max(0.2);
            warn!("🔻 降低网络同步频率 -> {:.2}s", network_state.sync_interval);
        } else if frame_stats.current_fps > 100.0 && network_state.sync_interval > 0.1 {
            // FPS 足够高，可以增加网络更新频率
            network_state.sync_interval = 0.1;
            info!("🔼 恢复网络同步频率 -> {:.2}s", network_state.sync_interval);
        }
    }
}

/// 渲染优化系统
pub fn optimize_render_system(
    frame_stats: Res<FrameStats>,
) {
    // 根据FPS动态调整渲染质量
    if frame_stats.frame_count % 600 == 0 {
        let quality = if frame_stats.current_fps < 30.0 {
            "低"
        } else if frame_stats.current_fps < 60.0 {
            "中"
        } else {
            "高"
        };
        
        info!("🎨 渲染质量: {} ({:.1} FPS)", quality, frame_stats.current_fps);
    }
}

/// 性能分析系统
pub fn profile_system_performance_system(
    frame_stats: Res<FrameStats>,
) {
    // 定期输出性能分析报告
    if frame_stats.frame_count == 3600 {  // 60 秒后（@60FPS）
        info!("╔═══════════════════════════════════════╗");
        info!("║       GameScene 性能分析报告          ║");
        info!("╠═══════════════════════════════════════╣");
        info!("║ 总帧数: {}", frame_stats.frame_count);
        info!("║ 总运行时间: {:.1}s", frame_stats.total_time);
        info!("║ 平均FPS: {:.1}", frame_stats.average_fps);
        info!("║ 最小FPS: {:.1}", 1000.0 / frame_stats.max_frame_time.max(0.1));
        info!("║ 最大FPS: {:.1}", 1000.0 / frame_stats.min_frame_time.max(0.1));
        info!("║ 当前FPS: {:.1}", frame_stats.current_fps);
        info!("╚═══════════════════════════════════════╝");
    }
}

/// 消息处理器 - 游戏循环消息
pub fn message_handle_game_loop(
    events: Option<MessageReader<GameLoopMessage>>,
    mut game_state: ResMut<GameSceneState>,
) {
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        match event.loop_state {
            1 => {
                game_state.is_paused = true;
                info!("⏸️ 游戏已暂停");
            }
            2 => {
                game_state.is_paused = false;
                info!("▶️ 游戏已恢复");
            }
            _ => {}
        }
    }
}

/// 消息处理器 - 帧统计请求
pub fn message_handle_frame_stats_request(
    events: Option<MessageReader<RequestFrameStatsMessage>>,
    frame_stats: Res<FrameStats>,
) {
    let Some(mut events) = events else { return; };
    
    for _ in events.read() {
        info!(
            "📊 帧统计 - FPS: {:.1} | 帧数: {} | 平均: {:.1}",
            frame_stats.current_fps, frame_stats.frame_count, frame_stats.average_fps
        );
    }
}

/// 消息处理器 - 系统健康检查请求
pub fn message_handle_system_health_request(
    events: Option<MessageReader<RequestSystemHealthMessage>>,
    health_check: Res<SystemHealthCheck>,
) {
    let Some(mut events) = events else { return; };
    
    for _ in events.read() {
        info!(
            "🏥 系统健康: 玩家{} | 地图{} | 对话{} | 聊天{} | 网络{} | 渲染{} | 总体{}",
            if health_check.player_system_ok { "✅" } else { "❌" },
            if health_check.map_system_ok { "✅" } else { "❌" },
            if health_check.dialogue_system_ok { "✅" } else { "❌" },
            if health_check.chat_system_ok { "✅" } else { "❌" },
            if health_check.network_system_ok { "✅" } else { "❌" },
            if health_check.render_system_ok { "✅" } else { "❌" },
            if health_check.all_systems_ok { "✅ 正常" } else { "❌ 异常" }
        );
    }
}

/// 消息处理器 - 性能报告请求
pub fn message_handle_performance_report(
    events: Option<MessageReader<PerformanceReportMessage>>,
    frame_stats: Res<FrameStats>,
    game_timer: Res<GameTimer>,
) {
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        match event.report_type {
            0 => {  // FPS 报告
                info!(
                    "📈 FPS报告 - 当前: {:.1} | 平均: {:.1} | 最小: {:.1} | 最大: {:.1}",
                    frame_stats.current_fps,
                    frame_stats.average_fps,
                    1000.0 / frame_stats.max_frame_time.max(0.1),
                    1000.0 / frame_stats.min_frame_time.max(0.1)
                );
            }
            1 => {  // 内存报告
                info!("💾 内存报告 - 事件队列: 准备就绪");
            }
            2 => {  // 网络报告
                info!("🌐 网络报告 - 游戏速度: {:.2}x", game_timer.game_speed);
            }
            3 => {  // 完整报告
                info!("📋 完整性能报告已生成");
            }
            _ => {}
        }
    }
}
