// SelectScene - 角色选择场景主模块
// 处理从登录场景到游戏场景的角色选择过程

mod components;

pub use components::*;

use bevy::prelude::*;

// ============================================================================
// Setup and Cleanup
// ============================================================================

/// 设置选择场景
pub fn setup_select_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    info!("🎮 Setting up SelectScene");
    
    // 插入 SelectSceneState 资源
    commands.insert_resource(SelectSceneState::default());
    info!("✅ SelectSceneState 已创建");
    
    // 创建根实体 (全屏容器)
    let root = commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        SelectSceneRoot,
        Name::new("SelectSceneRoot"),
        BackgroundColor(BACKGROUND_COLOR),
    )).id();
    
    info!("✅ Root 实体已创建");
    
    // 生成所有 UI 元素
    commands.entity(root).with_children(|parent| {
        // 生成标题
        parent.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(60.0),
                position_type: PositionType::Relative,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
        )).with_children(|parent| {
            parent.spawn((
                Text::new("选择角色"),
                TextFont {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 40.0,
                    ..default()
                },
                TextColor(TEXT_COLOR),
            ));
        });
        
        // 生成角色列表
        parent.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Auto,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(20.0)),
                row_gap: Val::Px(10.0),
                ..default()
            },
            CharacterListContainer,
        ));
        
        // 生成按钮面板
        parent.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Auto,
                position_type: PositionType::Relative,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(10.0),
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
        )).with_children(|parent| {
            // 开始游戏按钮
            parent.spawn((
                Button,
                Node {
                    width: Val::Px(150.0),
                    height: Val::Px(50.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(BUTTON_COLOR),
                StartGameButton,
            )).with_children(|parent| {
                parent.spawn((
                    Text::new("开始游戏"),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ));
            });
            
            // 创建角色按钮
            parent.spawn((
                Button,
                Node {
                    width: Val::Px(150.0),
                    height: Val::Px(50.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(BUTTON_COLOR),
                CreateCharacterButton,
            )).with_children(|parent| {
                parent.spawn((
                    Text::new("创建角色"),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ));
            });
            
            // 返回登录按钮
            parent.spawn((
                Button,
                Node {
                    width: Val::Px(150.0),
                    height: Val::Px(50.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(BUTTON_COLOR),
                BackToLoginButton,
            )).with_children(|parent| {
                parent.spawn((
                    Text::new("返回登录"),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ));
            });
        });
    });
    
    info!("🎉 SelectScene 设置完成!");
}

/// 清理选择场景
pub fn cleanup_select_scene(
    mut commands: Commands,
    query: Query<Entity, With<SelectSceneRoot>>,
) {
    info!("🧹 Cleaning up SelectScene");
    
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
    
    commands.remove_resource::<SelectSceneState>();
}

// ============================================================================
// System Implementations
// ============================================================================


/// 更新角色列表显示
pub fn update_character_list(
    select_state: Res<SelectSceneState>,
) {
    if select_state.is_changed() {
        info!("📋 角色列表已更新: {} 个角色", select_state.characters.len());
    }
}

// ============================================================================
// Button Interactions
// ============================================================================

/// 处理按钮悬停
pub fn handle_button_hover(
    mut query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut bg_color) in query.iter_mut() {
        match *interaction {
            Interaction::Hovered => {
                *bg_color = BackgroundColor(BUTTON_HOVER_COLOR);
            }
            Interaction::Pressed => {
                *bg_color = BackgroundColor(BUTTON_PRESSED_COLOR);
            }
            Interaction::None => {
                *bg_color = BackgroundColor(BUTTON_COLOR);
            }
        }
    }
}

/// 处理角色选择
pub fn handle_character_select(
    mut events: Option<MessageWriter<SelectCharacterMessage>>,
    mut select_state: ResMut<SelectSceneState>,
    query: Query<(&Interaction, &SelectButton), Changed<Interaction>>,
) {
    for (interaction, button) in query.iter() {
        if *interaction == Interaction::Pressed {
            info!("🎯 选择角色: 索引 {}", button.character_index);
            select_state.selected_index = Some(button.character_index);
            
            if let Some(ref mut events) = events {
                events.write(SelectCharacterMessage {
                    index: button.character_index,
                });
            }
        }
    }
}

/// 处理角色删除
pub fn handle_character_delete(
    mut events: Option<MessageWriter<DeleteCharacterMessage>>,
    mut select_state: ResMut<SelectSceneState>,
    query: Query<(&Interaction, &DeleteButton), Changed<Interaction>>,
) {
    for (interaction, button) in query.iter() {
        if *interaction == Interaction::Pressed {
            info!("🗑️ 删除确认对话框: 角色索引 {}", button.character_index);
            select_state.show_delete_dialog = true;
            select_state.delete_confirm_index = Some(button.character_index);
            
            if let Some(ref mut events) = events {
                events.write(DeleteCharacterMessage {
                    index: button.character_index,
                });
            }
        }
    }
}

/// 处理创建角色
pub fn handle_create_character(
    mut events: Option<MessageWriter<CreateCharacterMessage>>,
    mut select_state: ResMut<SelectSceneState>,
    query: Query<&Interaction, (Changed<Interaction>, With<CreateCharacterButton>)>,
) {
    for interaction in query.iter() {
        if *interaction == Interaction::Pressed {
            info!("➕ 显示创建角色对话框");
            select_state.show_create_dialog = true;
            
            if let Some(ref mut events) = events {
                events.write(CreateCharacterMessage::default());
            }
        }
    }
}

/// 处理开始游戏
pub fn handle_start_game(
    mut events: Option<MessageWriter<StartGameMessage>>,
    select_state: Res<SelectSceneState>,
    query: Query<&Interaction, (Changed<Interaction>, With<StartGameButton>)>,
    mut next_state: ResMut<NextState<crate::bevy::GameState>>,
) {
    for interaction in query.iter() {
        if *interaction == Interaction::Pressed {
            if let Some(idx) = select_state.selected_index {
                if idx < select_state.characters.len() {
                    let character_index = select_state.characters[idx].index;
                    info!("🎮 开始游戏: 角色索引 {}", character_index);
                    
                    if let Some(ref mut events) = events {
                        events.write(StartGameMessage {
                            character_index,
                        });
                    }
                    
                    // 转移到游戏状态
                    next_state.set(crate::bevy::GameState::Game);
                } else {
                    warn!("⚠️ 选中的角色索引无效");
                }
            } else {
                warn!("⚠️ 没有选择角色");
            }
        }
    }
}

/// 处理返回登录
pub fn handle_back_to_login(
    mut events: Option<MessageWriter<BackToLoginMessage>>,
    query: Query<&Interaction, (Changed<Interaction>, With<BackToLoginButton>)>,
    mut next_state: ResMut<NextState<crate::bevy::GameState>>,
) {
    for interaction in query.iter() {
        if *interaction == Interaction::Pressed {
            info!("🔙 返回登录场景");
            
            if let Some(ref mut events) = events {
                events.write(BackToLoginMessage);
            }
            
            // 转移回登录状态
            next_state.set(crate::bevy::GameState::Login);
        }
    }
}

// ============================================================================
// Message Handlers
// ============================================================================

/// 处理选择角色消息
pub fn message_handle_select_character(
    events: Option<MessageReader<SelectCharacterMessage>>,
) {
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        info!("📨 收到选择角色消息: 索引 {}", event.index);
    }
}

/// 处理删除角色消息
pub fn message_handle_delete_character(
    events: Option<MessageReader<DeleteCharacterMessage>>,
) {
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        info!("📨 收到删除角色消息: 索引 {}", event.index);
    }
}

/// 处理创建角色消息
pub fn message_handle_create_character(
    events: Option<MessageReader<CreateCharacterMessage>>,
) {
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        info!("📨 收到创建角色消息: 名称 '{}'", event.name);
    }
}

/// 处理开始游戏消息
pub fn message_handle_start_game(
    events: Option<MessageReader<StartGameMessage>>,
) {
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        info!("📨 收到开始游戏消息: 角色索引 {}", event.character_index);
    }
}

/// 处理返回登录消息
pub fn message_handle_back_to_login(
    events: Option<MessageReader<BackToLoginMessage>>,
) {
    let Some(mut events) = events else { return; };
    
    for _ in events.read() {
        info!("📨 收到返回登录消息");
    }
}
