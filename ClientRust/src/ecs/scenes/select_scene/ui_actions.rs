//! UI 交互逻辑模块
//! 负责处理用户界面操作，如按钮点击、对话框管理等
//! 
//! 遵循单一职责原则：所有UI相关的操作逻辑都在此模块

use tokio::sync::mpsc;
use crate::network::NetworkCommand;
use mir2_shared::enums::{MirClass, MirGender};
use super::{SelectScene, BottomButton, NewCharacterDialog, DeleteCharacterDialog, CreditsDialog};
use super::new_character_dialog::DialogButton;

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
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) {
        match button {
            BottomButton::StartGame => {
                self.start_game();
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
    pub(super) fn handle_dialog_button_click(&mut self, button: DialogButton) {
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
                    tracing::info!("📝 创建新角色: {} ({:?}, {:?})", 
                        dialog.name, dialog.selected_class, dialog.selected_gender);
                    
                    if let Some(tx) = &self.command_tx {
                        if let Err(e) = tx.send(crate::network::NetworkCommand::NewCharacter {
                            name: dialog.name.clone(),
                            class: dialog.selected_class as u8,
                            gender: dialog.selected_gender as u8,
                        }) {
                            tracing::error!("Failed to send NewCharacter command: {}", e);
                            dialog.error_message = Some("网络错误,请重试".to_string());
                            dialog.creating = false;
                            return;
                        }
                        tracing::info!("✅ 角色创建请求已发送,等待服务器响应...");
                    } else {
                        tracing::warn!("No network command channel available");
                        dialog.error_message = Some("网络未连接".to_string());
                        dialog.creating = false;
                    }
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
    pub(super) fn handle_new_character_button(&mut self, button: DialogButton) {
        self.handle_dialog_button_click(button);
    }

    /// 开始游戏
    /// 
    /// Mirrors C# StartGame():
    /// ```csharp
    /// public void StartGame()
    /// {
    ///     Network.Enqueue(new C.StartGame { CharacterIndex = Characters[_selected].Index });
    /// }
    /// ```
    pub(super) fn start_game(&mut self) {
        tracing::info!("🎮 start_game() called - selected_index={}, characters.len()={}", 
            self.selected_index, self.characters.len());
        
        if self.selected_index >= 0 && (self.selected_index as usize) < self.characters.len() {
            let character = &self.characters[self.selected_index as usize];
            tracing::info!("🎮 Starting game with character: {} (index={})", character.name, character.index);
            
            // Send StartGame command to network thread
            if let Some(command_tx) = &self.command_tx {
                use crate::network::NetworkCommand;
                
                tracing::info!("📡 Network command channel available, sending StartGame...");
                if command_tx.send(NetworkCommand::StartGame {
                    character_index: character.index,
                }).is_ok() {
                    tracing::info!("✅ Sent StartGame command for character index {}", character.index);
                } else {
                    tracing::error!("❌ Failed to send StartGame command (channel send error)");
                }
            } else {
                tracing::error!("❌ Network command channel not available (command_tx is None)");
            }
        } else {
            tracing::warn!("⚠️ Cannot start game: No character selected (selected_index={}, len={})", 
                self.selected_index, self.characters.len());
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
        if self.selected_index < 0 || (self.selected_index as usize) >= self.characters.len() {
            tracing::warn!("⚠️ 没有选中角色,无法删除");
            
            // 🆕 显示消息框提示用户
            let mut message_box = super::MessageBox::new(
                "Please select a character first.".to_string(),
                super::MessageBoxButtons::Ok,
                super::DESIGN_WIDTH,
                super::DESIGN_HEIGHT
            );
            message_box.show();
            self.message_box = Some(message_box);
            return;
        }
        
        let character = &self.characters[self.selected_index as usize];
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
    pub(super) fn submit_delete_character(&mut self) {
        if let Some(dialog) = &mut self.delete_character_dialog {
            if !dialog.can_submit() {
                tracing::warn!("⚠️ 无法提交删除请求: 名称不匹配或正在删除");
                return;
            }
            
            let character_index = dialog.character_index;
            dialog.deleting = true;
            
            tracing::info!("🗑️ 发送删除角色请求: index={}", character_index);
            
            // 发送删除命令到网络层
            if let Some(tx) = &self.command_tx {
                use crate::network::NetworkCommand;
                if let Err(e) = tx.send(NetworkCommand::DeleteCharacter { index: character_index }) {
                    tracing::error!("❌ 发送删除角色命令失败: {}", e);
                    dialog.deleting = false;
                    dialog.error_message = Some("网络错误,无法发送删除请求".to_string());
                }
            } else {
                tracing::error!("❌ 网络命令发送器未初始化");
                dialog.deleting = false;
                dialog.error_message = Some("网络未连接".to_string());
            }
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
