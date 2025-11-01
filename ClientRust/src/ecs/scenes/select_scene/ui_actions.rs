//! UI 交互逻辑模块
//! 负责处理用户界面操作，如按钮点击、对话框管理等
//!
//! 遵循单一职责原则：所有UI相关的操作逻辑都在此模块

use super::new_character_dialog::DialogButton;
use super::{BottomButton, CreditsDialog, NewCharacterDialog, SelectScene};
use crate::ecs::WorldExt;
use crate::network::NetContext;
use mir2_shared::enums::{MirClass, MirGender};
use std::sync::Arc;

impl SelectScene {
    /// 处理底部按钮点击
    ///
    /// 根据按钮类型执行相应的操作：
    /// - StartGame: 发送开始游戏命令
    /// - NewCharacter: 打开新建角色对话框
    /// - DeleteCharacter: 打开删除角色对话框
    /// - Credits: 打开制作人员对话框
    /// - ExitGame: 退出游戏
    pub(super) fn handle_button_click(
        &mut self,
        button: BottomButton,
        world: &mut hecs::World,
      
    ) {
        match button {
            BottomButton::StartGame => {
                self.start_game(world);
            }
            BottomButton::NewCharacter => {
                self.open_new_character_dialog();
            }
            BottomButton::DeleteCharacter => {
                self.open_delete_character_dialog();
            }
            BottomButton::Credits => {
                self.open_credits_dialog();
            }
            BottomButton::ExitGame => {
                self.exit_game();
            }
        }
    }

    /// 处理对话框按钮点击
    ///
    /// 处理新建角色对话框中的各种按钮操作
    pub(super) fn handle_dialog_button_click(
        &mut self,
        button: DialogButton,
        world: &mut hecs::World,
    ) {
        if let Some(dialog) = &mut self.new_character_dialog {
            match button {
                DialogButton::OK => {
                    // 验证角色名称
                    if let Err(err_msg) = dialog.validate_name() {
                        tracing::warn!("角色名称验证失败: {}", err_msg);
                        dialog.error_message = Some(err_msg);
                        return;
                    }

                    // 标记为正在创建
                    dialog.creating = true;

                    // 发送 NewCharacter 网络包
                    tracing::info!(
                        "📝 创建新角色: {} ({:?}, {:?})",
                        dialog.name,
                        dialog.selected_class,
                        dialog.selected_gender
                    );

                    world
                        .network()
                        .send(crate::network::handlers::GameEvent::NewCharacterRequest {
                            name: dialog.name.clone(),
                            class: dialog.selected_class as u8,
                            gender: dialog.selected_gender as u8,
                        })
                        .ok();
                    tracing::info!("✅ 角色创建请求已发送,等待服务器响应...");
                }
                DialogButton::Cancel => {
                    tracing::info!("❌ 取消角色创建");
                    dialog.hide();
                }
                DialogButton::Warrior => {
                    dialog.selected_class = MirClass::Warrior;
                    tracing::debug!("选择职业: 战士");
                }
                DialogButton::Wizard => {
                    dialog.selected_class = MirClass::Wizard;
                    tracing::debug!("选择职业: 法师");
                }
                DialogButton::Taoist => {
                    dialog.selected_class = MirClass::Taoist;
                    tracing::debug!("选择职业: 道士");
                }
                DialogButton::Assassin => {
                    dialog.selected_class = MirClass::Assassin;
                    tracing::debug!("选择职业: 刺客");
                }
                DialogButton::Archer => {
                    dialog.selected_class = MirClass::Archer;
                    tracing::debug!("选择职业: 弓箭手");
                }
                DialogButton::Male => {
                    dialog.selected_gender = MirGender::Male;
                    tracing::debug!("选择性别: 男");
                }
                DialogButton::Female => {
                    dialog.selected_gender = MirGender::Female;
                    tracing::debug!("选择性别: 女");
                }
            }
        }
    }

    /// 处理新建角色对话框按钮（别名方法）
    pub(super) fn handle_new_character_button(
        &mut self,
        button: DialogButton,
        world: &mut hecs::World,
    ) {
        self.handle_dialog_button_click(button, world);
    }

    /// 开始游戏 - 从World读取选中的角色
    ///
    /// Mirrors C# StartGame():
    /// ```csharp
    /// public void StartGame()
    /// {
    ///     Network.Enqueue(new C.StartGame { CharacterIndex = Characters[_selected].Index });
    /// }
    /// ```
    pub(super) fn start_game(
        &mut self,
        world: &mut hecs::World,
       
    ) {
        use crate::ecs::components::CharacterList;

        // � Step 1: 从UI获取当前选中的角色索引
        let ui_selected_index = self.character_select_ui.get_selected_index();

        // 🎯 Step 2: 更新World中CharacterList的selected_index
        if let Some((_, mut char_list)) = world.query::<&mut CharacterList>().iter().next() {
            char_list.selected_index = ui_selected_index;
            tracing::info!(
                "📝 Updated CharacterList.selected_index to {}",
                ui_selected_index
            );
        }

        // 🎯 Step 3: 读取选中的角色信息
        let (selected_index, character) =
            if let Some((_, char_list)) = world.query::<&CharacterList>().iter().next() {
                let idx = char_list.selected_index;
                let char = char_list.get_selected().cloned();
                (idx, char)
            } else {
                (-1, None)
            };

        let character_count = self.character_select_ui.get_characters().len();
        tracing::info!(
            "🎮 start_game() called - selected_index={}, characters.len()={}",
            selected_index,
            character_count
        );

        if let Some(character) = character {
            tracing::info!(
                "🎮 Starting game with character: {} (index={})",
                character.name,
                character.index
            );

            // 🎯 Step 4: 发送StartGameRequest到服务器
            // 📡 SelectScene不是ECS架构，直接使用NetContext发送命令
            world.network()
                .send(crate::network::handlers::GameEvent::StartGameRequest {
                    character_index: character.index,
                })
                .ok();
            tracing::info!(
                "✅ Sent StartGameRequest via NetContext (character_index={})",
                character.index
            );

            // 🎯 Step 5: 等待服务器返回UserInformation，然后在network_handler中切换场景
            tracing::info!("⏳ Waiting for server UserInformation to switch to GameScene...");
        } else {
            tracing::warn!(
                "⚠️ Cannot start game: No character selected (selected_index={}, len={})",
                selected_index,
                character_count
            );
        }
    }

    /// 打开新建角色对话框
    ///
    /// Mirrors C# OpenNewCharacterDialog():
    /// ```csharp
    /// private void OpenNewCharacterDialog()
    /// {
    ///     if (_character == null || _character.IsDisposed)
    ///     {
    ///         _character = new NewCharacterDialog { Parent = this };
    ///     }
    /// }
    /// ```
    pub(super) fn open_new_character_dialog(&mut self) {
        tracing::info!("➕ 打开角色创建对话框");

        if self.new_character_dialog.is_none() {
            let mut dialog = NewCharacterDialog::new();
            dialog.show();
            self.new_character_dialog = Some(dialog);
            tracing::info!("✅ 角色创建对话框已打开");
        } else {
            // 如果对话框已存在,则显示它
            if let Some(dialog) = &mut self.new_character_dialog {
                dialog.show();
            }
            tracing::info!("ℹ️ 角色创建对话框已显示");
        }
    }

    /// 打开删除角色对话框
    ///
    /// Mirrors C# DeleteCharacter():
    /// ```csharp
    /// private void DeleteCharacter()
    /// {
    ///     MirMessageBox message = new MirMessageBox(...);
    ///     message.YesButton.Click += ...
    /// }
    /// ```
    pub(super) fn open_delete_character_dialog(&mut self) {
        let Some(character) = self.character_select_ui.get_selected_character() else {
            tracing::warn!("⚠️ 没有选中角色,无法删除");

            // 🆕 显示消息框提示用户
            let mut message_box = super::MessageBox::new(
                "Please select a character first.".to_string(),
                super::MessageBoxButtons::Ok,
                super::DESIGN_WIDTH,
                super::DESIGN_HEIGHT,
            );
            message_box.show();
            self.message_box = Some(message_box);
            return;
        };

        tracing::info!("🗑️ 打开删除角色对话框: {}", character.name);

        // 🆕 使用正确的 MessageBox 显示确认对话框
        let mut message_box = super::MessageBox::new(
            format!("Are you sure you want to Delete\nthe character {}?\n\nThis action cannot be undone!", character.name),
            super::MessageBoxButtons::YesNo,
            super::DESIGN_WIDTH,
            super::DESIGN_HEIGHT
        );
        message_box.show();
        self.message_box = Some(message_box);
    }

    /// 打开制作人员对话框
    ///
    /// Mirrors C# CreditsButton.Click event
    pub(super) fn open_credits_dialog(&mut self) {
        tracing::info!("📜 打开Credits对话框");
        let dialog = CreditsDialog::new();
        self.credits_dialog = Some(dialog);
        if let Some(d) = &mut self.credits_dialog {
            d.show();
        }
    }

    /// 提交删除角色请求
    ///
    /// Mirrors C# DeleteCharacter() sending packet:
    /// ```csharp
    /// Network.Enqueue(new C.DeleteCharacter { CharacterIndex = index });
    /// ```
    pub(super) fn submit_delete_character(&mut self, net_ctx: &Arc<NetContext>) {
        if let Some(dialog) = &mut self.delete_character_dialog {
            if !dialog.can_submit() {
                tracing::warn!("⚠️ 无法提交删除请求: 名称不匹配或正在删除");
                return;
            }

            let character_index = dialog.character_index;
            dialog.deleting = true;

            tracing::info!("🗑️ 发送删除角色请求: index={}", character_index);

            // 📡 SelectScene直接使用NetContext发送命令
            net_ctx.send(
                crate::network::handlers::GameEvent::DeleteCharacterRequest {
                    index: character_index,
                },
            );
            tracing::info!(
                "✅ Sent DeleteCharacterRequest via NetContext (index={})",
                character_index
            );
        }
    }

    /// 退出到登录场景
    ///
    /// Mirrors C# ExitGame.Click event:
    /// ```csharp
    /// ExitGame.Click += (o, e) => Program.Form.Close();
    /// ```
    ///
    /// 注意: C#原版是完全退出程序，但在现代游戏中
    /// 更好的做法是返回到登录界面，而不是直接关闭窗口
    fn exit_game(&mut self) {
        tracing::info!("� 返回登录场景");
        self.pending_scene_change = Some(crate::ecs::scenes::SceneType::Login);
    }
}
