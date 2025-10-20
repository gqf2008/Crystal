// SelectScene Update Systems
// 负责动画更新和纹理状态更新

use bevy::prelude::*;
use crate::bevy::MLibraryAssets;
use super::components::*;

/// 更新角色预览动画
pub fn update_character_animation(
    time: Res<Time>,
    mut state: ResMut<SelectSceneState>,
    preview_query: Query<Entity, With<CharacterPreview>>,
    mut mlibrary_assets: ResMut<MLibraryAssets>,
    mut images: ResMut<Assets<Image>>,
    mut commands: Commands,
) {
    // 更新动画计时器
    state.character_animation_timer += time.delta_secs();
    
    if state.character_animation_timer >= ANIMATION_DELAY {
        state.character_animation_timer = 0.0;
        
        // 前进到下一帧
        state.character_animation_frame = (state.character_animation_frame + 1) % ANIMATION_FRAME_COUNT;
        
        // 如果有选中的角色，更新预览纹理
        if state.selected_index >= 0 && (state.selected_index as usize) < state.characters.len() {
            let character = &state.characters[state.selected_index as usize];
            
            // 获取动画基础索引
            let base_index = get_character_animation_base(character.class, character.gender);
            let anim_index = base_index + state.character_animation_frame as i32;
            
            // 加载新的动画帧
            if let Some(anim_tex) = mlibrary_assets.get_texture("ChrSel", anim_index, &mut images) {
                // 更新预览实体的纹理
                for entity in preview_query.iter() {
                    commands.entity(entity).insert(ImageNode {
                        image: anim_tex.clone(),
                        ..default()
                    });
                }
            }
        }
    }
}

/// 更新角色槽位纹理
pub fn update_character_slots(
    state: Res<SelectSceneState>,
    mut slot_query: Query<(Entity, &CharacterSlot)>,
    mut mlibrary_assets: ResMut<MLibraryAssets>,
    mut images: ResMut<Assets<Image>>,
    mut commands: Commands,
) {
    // 只在状态改变时更新
    if !state.is_changed() {
        return;
    }
    
    for (entity, slot) in slot_query.iter_mut() {
        let slot_index = slot.slot_index;
        
        // 确定槽位纹理索引
        let texture_index = if slot_index < state.characters.len() {
            // 有角色：根据选中状态和职业选择纹理
            let character = &state.characters[slot_index];
            if state.selected_index == slot_index as i32 {
                665 + (character.class as i32)  // 选中状态: 665-669
            } else {
                660 + (character.class as i32)  // 未选中状态: 660-664
            }
        } else {
            // 空槽位：Prguse_44
            if let Some(empty_tex) = mlibrary_assets.get_texture("Prguse", 44, &mut images) {
                commands.entity(entity).insert(ImageNode {
                    image: empty_tex,
                    ..default()
                });
                continue;
            } else {
                continue;
            }
        };
        
        // 加载槽位纹理
        if let Some(slot_tex) = mlibrary_assets.get_texture("Title", texture_index, &mut images) {
            commands.entity(entity).insert(ImageNode {
                image: slot_tex,
                ..default()
            });
        }
    }
}

/// 更新底部按钮纹理（悬停/按下状态）
pub fn update_button_textures(
    _state: Res<SelectSceneState>,
    mut button_query: Query<(Entity, &BottomButton, &Interaction), Changed<Interaction>>,
    mut mlibrary_assets: ResMut<MLibraryAssets>,
    mut images: ResMut<Assets<Image>>,
    mut commands: Commands,
) {
    for (entity, button, interaction) in button_query.iter_mut() {
        // 获取按钮基础索引
        let base_index = BOTTOM_BUTTONS.iter()
            .find(|(btn_type, _)| *btn_type == button.button_type)
            .map(|(_, idx)| *idx)
            .unwrap_or(340);
        
        // 根据状态确定纹理索引
        let texture_index = match interaction {
            Interaction::Pressed => base_index + 2,  // 按下状态
            Interaction::Hovered => base_index + 1,  // 悬停状态
            Interaction::None => base_index,          // 正常状态
        };
        
        // 加载纹理
        if let Some(button_tex) = mlibrary_assets.get_texture("Title", texture_index, &mut images) {
            commands.entity(entity).insert(ImageNode {
                image: button_tex,
                ..default()
            });
        }
    }
}
