// Character Slot Systems
// 处理角色槽的点击选择和文本更新

use bevy::prelude::*;
use super::components::*;

// ============================================================================
// 角色槽点击选择系统
// ============================================================================

/// 处理角色槽点击，更新选中状态
pub fn handle_slot_clicks(
    mut interaction_query: Query<(&Interaction, &CharacterSlot), Changed<Interaction>>,
    mut state: ResMut<SelectSceneState>,
    mut dialog_state: ResMut<crate::bevy::scenes::select_scene::DialogState>,
    commands: Commands,
    mlibrary_assets: ResMut<crate::bevy::MLibraryAssets>,
    images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
    root_query: Query<Entity, With<SelectSceneRoot>>,
) {
    let mut should_open_dialog = false;
    
    for (interaction, slot) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            // 检查是否有角色数据
            if slot.slot_index < state.characters.len() {
                // 切换选中状态
                if state.selected_index == slot.slot_index as i32 {
                    // 取消选中
                    state.selected_index = -1;
                    info!("❌ 取消选中槽位 {}", slot.slot_index);
                } else {
                    // 选中新槽位
                    state.selected_index = slot.slot_index as i32;
                    let character = &state.characters[slot.slot_index];
                    info!("✅ 选中角色: {} (Lv.{})", character.name, character.level);
                }
            } else {
                // 空槽位，打开创建新角色对话框
                info!("➕ 点击空槽位 {} - 打开创建角色对话框", slot.slot_index);
                should_open_dialog = true;
            }
        }
    }
    
    // 在循环外打开对话框，避免借用冲突
    if should_open_dialog {
        dialog_state.active_dialog = Some(crate::bevy::scenes::select_scene::ActiveDialog::NewCharacter);
        dialog_state.new_character = Default::default();
        
        crate::bevy::scenes::select_scene::spawn_new_character_dialog(
            commands,
            mlibrary_assets,
            images,
            asset_server,
            root_query,
        );
    }
}

// ============================================================================
// 角色槽文本更新系统
// ============================================================================

/// 更新角色槽文本内容（名称、等级、职业）
pub fn update_slot_texts(
    state: Res<SelectSceneState>,
    mut text_query: Query<(&mut Text, &CharacterSlotText)>,
) {
    // 只在状态改变时更新
    if !state.is_changed() {
        return;
    }
    
    for (mut text, slot_text) in text_query.iter_mut() {
        // 检查槽位是否有角色
        if slot_text.slot_index < state.characters.len() {
            let character = &state.characters[slot_text.slot_index];
            
            // 根据文本类型更新内容
            **text = match slot_text.text_type {
                SlotTextType::Name => {
                    character.name.clone()
                }
                SlotTextType::Level => {
                    format!("等级: {}", character.level)
                }
                SlotTextType::Class => {
                    format!("{:?}", character.class)
                }
            };
        } else {
            // 空槽位，清空文本
            **text = match slot_text.text_type {
                SlotTextType::Name => "- 空槽位 -".to_string(),
                SlotTextType::Level => String::new(),
                SlotTextType::Class => String::new(),
            };
        }
    }
}

/// 更新角色槽文本颜色（高亮选中的槽位）
pub fn update_slot_text_colors(
    state: Res<SelectSceneState>,
    mut text_query: Query<(&mut TextColor, &CharacterSlotText)>,
) {
    // 只在选中状态改变时更新
    if !state.is_changed() {
        return;
    }
    
    for (mut color, slot_text) in text_query.iter_mut() {
        let is_selected = state.selected_index == slot_text.slot_index as i32;
        
        // 根据选中状态和文本类型设置颜色
        color.0 = if is_selected {
            // 选中状态 - 高亮颜色
            match slot_text.text_type {
                SlotTextType::Name => Color::srgb(1.0, 1.0, 0.3),  // 亮黄色
                SlotTextType::Level => Color::srgb(1.0, 1.0, 1.0), // 白色
                SlotTextType::Class => Color::srgb(0.5, 1.0, 1.0), // 青色
            }
        } else {
            // 未选中状态 - 普通颜色
            match slot_text.text_type {
                SlotTextType::Name => Color::srgb(1.0, 0.9, 0.6),  // 金黄色
                SlotTextType::Level => Color::srgb(0.8, 0.8, 0.8), // 浅灰色
                SlotTextType::Class => Color::srgb(0.6, 0.8, 1.0), // 浅蓝色
            }
        };
    }
}

// ============================================================================
// 角色槽悬停效果系统
// ============================================================================

/// 处理角色槽悬停效果（可选，增强用户体验）
pub fn handle_slot_hover(
    mut slot_query: Query<(&Interaction, &mut BackgroundColor, &CharacterSlot), Changed<Interaction>>,
    state: Res<SelectSceneState>,
) {
    for (interaction, mut bg_color, slot) in slot_query.iter_mut() {
        match *interaction {
            Interaction::Hovered => {
                // 悬停效果 - 半透明高亮
                *bg_color = BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.1));
            }
            Interaction::Pressed => {
                // 按下效果 - 更深的高亮
                *bg_color = BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.2));
            }
            Interaction::None => {
                // 检查是否是选中的槽位
                let is_selected = state.selected_index == slot.slot_index as i32;
                *bg_color = if is_selected {
                    BackgroundColor(Color::srgba(1.0, 1.0, 0.5, 0.15))  // 选中的淡黄色背景
                } else {
                    BackgroundColor(Color::NONE)  // 透明
                };
            }
        }
    }
}
