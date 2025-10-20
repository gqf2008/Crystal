// SelectScene Button Systems
// 负责按钮点击和交互处理

use bevy::prelude::*;
use super::components::*;

/// 处理底部按钮点击
pub fn handle_button_clicks(
    mut interaction_query: Query<(&Interaction, &BottomButton), Changed<Interaction>>,
    state: Res<SelectSceneState>,
    mut dialog_state: ResMut<crate::bevy::scenes::select_scene::DialogState>,
    mut next_state: ResMut<NextState<crate::bevy::GameState>>,
    commands: Commands,
    mlibrary_assets: ResMut<crate::bevy::MLibraryAssets>,
    images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
    root_query: Query<Entity, With<SelectSceneRoot>>,
) {
    let mut should_open_new_dialog = false;
    
    for (interaction, button) in interaction_query.iter_mut() {
        if *interaction == Interaction::Pressed {
            match button.button_type {
                BottomButtonType::StartGame => {
                    info!("🎮 开始游戏按钮被点击");
                    // 检查是否选中了角色
                    if state.selected_index >= 0 && (state.selected_index as usize) < state.characters.len() {
                        let character = &state.characters[state.selected_index as usize];
                        info!("✅ 准备开始游戏: {} (索引: {})", character.name, character.index);
                        
                        // 发送 StartGame 网络命令
                        if let Some(tx) = &state.command_tx {
                            let command = crate::network::NetworkCommand::StartGame {
                                character_index: character.index,
                            };
                            
                            match tx.send(command) {
                                Ok(_) => {
                                    info!("📤 已发送 StartGame 命令: character_index={}", character.index);
                                }
                                Err(e) => {
                                    error!("❌ 发送 StartGame 命令失败: {}", e);
                                }
                            }
                        } else {
                            // 测试模式: NetworkManager 未初始化,直接切换到游戏场景
                            warn!("⚠️ 网络命令通道未初始化 - 进入测试模式");
                            info!("🎮 [测试模式] 直接进入游戏场景: {}", character.name);
                            next_state.set(crate::bevy::GameState::Game);
                        }
                    } else {
                        warn!("⚠️ 请先选择一个角色！");
                    }
                }
                BottomButtonType::NewCharacter => {
                    info!("➕ 新建角色按钮被点击");
                    should_open_new_dialog = true;
                }
                BottomButtonType::DeleteCharacter => {
                    info!("🗑️ 删除角色按钮被点击");
                    // 检查是否选中了角色
                    if state.selected_index >= 0 && (state.selected_index as usize) < state.characters.len() {
                        let character = &state.characters[state.selected_index as usize];
                        info!("⚠️ 确认删除角色: {} (索引: {})", character.name, character.index);
                        
                        // 发送删除角色网络命令
                        if let Some(tx) = &state.command_tx {
                            let command = crate::network::NetworkCommand::DeleteCharacter {
                                index: character.index,
                            };
                            
                            match tx.send(command) {
                                Ok(_) => {
                                    info!("📤 已发送 DeleteCharacter 命令: index={}", character.index);
                                }
                                Err(e) => {
                                    error!("❌ 发送 DeleteCharacter 命令失败: {}", e);
                                }
                            }
                        } else {
                            warn!("⚠️ 网络命令通道未初始化");
                            info!("📤 [TESTING] 测试模式: 假装删除角色 - {}", character.name);
                        }
                        
                        // TODO: 打开删除确认对话框
                        // TODO: 等待服务器响应后再从列表中移除
                    } else {
                        warn!("⚠️ 请先选择要删除的角色！");
                    }
                }
                BottomButtonType::Credits => {
                    info!("📜 制作人员按钮被点击");
                    // TODO: 打开制作人员对话框
                }
                BottomButtonType::ExitGame => {
                    info!("🚪 退出游戏按钮被点击");
                    std::process::exit(0);
                }
            }
        }
    }
    
    // 在循环外打开对话框
    if should_open_new_dialog {
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
