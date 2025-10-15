// SelectScene - Character selection scene
// Mirrors Client/MirScenes/SelectScene.cs
pub mod new_character_dialog;
pub mod delete_character_dialog;
pub mod credits_dialog;

pub use new_character_dialog::NewCharacterDialog;
pub use delete_character_dialog::DeleteCharacterDialog;
pub use credits_dialog::CreditsDialog;

use crate::scenes::{Scene, SceneType};
use crate::network::game_client::GameEvent;
use mir2_shared::SelectInfo;

/// Character selection scene
/// 
/// Mirrors C# SelectScene:
/// ```csharp
/// public class SelectScene : MirScene
/// {
///     public List<SelectInfo> Characters = new List<SelectInfo>();
///     private int _selected;
///     private NewCharacterDialog _character;
///     // ... UI controls
/// }
/// ```
pub struct SelectScene {
    // Character list (mirrors C# Characters)
    /// Mirrors C# `public List<SelectInfo> Characters`
    pub characters: Vec<SelectInfo>,
    
    /// Mirrors C# `private int _selected`
    pub selected_index: i32,
    
    // Dialogs
    /// Mirrors C# `private NewCharacterDialog _character`
    pub new_character_dialog: Option<NewCharacterDialog>,
    /// Delete character dialog
    pub delete_character_dialog: Option<DeleteCharacterDialog>,
    /// Credits dialog
    pub credits_dialog: Option<CreditsDialog>,
    
    // Network
    command_tx: Option<tokio::sync::mpsc::UnboundedSender<crate::network::NetworkCommand>>,
    
    // Scene transition
    pub pending_scene_change: Option<SceneType>,
    
    // UI state
    hovered_button: Option<BottomButton>,
    pressed_button: Option<BottomButton>,
    
    // Character preview animation
    character_animation_frame: usize,
    character_animation_timer: f32,
    
    // Window dimensions
    window_width: f32,
    window_height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BottomButton {
    StartGame,
    NewCharacter,
    DeleteCharacter,
    Credits,
    ExitGame,
}

impl SelectScene {
    /// Create new select scene with character list
    /// 
    /// Mirrors C# constructor:
    /// ```csharp
    /// public SelectScene(List<SelectInfo> characters)
    /// {
    ///     Characters = characters;
    ///     SortList();
    ///     // ... initialize UI
    /// }
    /// ```
    pub fn new(characters: Vec<SelectInfo>) -> Self {
        let mut scene = Self {
            characters,
            selected_index: 0,
            new_character_dialog: None,
            delete_character_dialog: None,
            credits_dialog: None,
            command_tx: None,
            pending_scene_change: None,
            hovered_button: None,
            pressed_button: None,
            character_animation_frame: 0,
            character_animation_timer: 0.0,
            window_width: 1024.0,
            window_height: 768.0,
        };
        scene.sort_list();
        scene
    }
    
    /// 设置网络命令发送器
    pub fn set_command_sender(&mut self, tx: tokio::sync::mpsc::UnboundedSender<crate::network::NetworkCommand>) {
        self.command_tx = Some(tx);
    }
    
    /// Sort character list by last access time
    /// 
    /// Mirrors C# SortList():
    /// ```csharp
    /// public void SortList()
    /// {
    ///     if (Characters != null)
    ///         Characters.Sort((c1, c2) => c2.LastAccess.CompareTo(c1.LastAccess));
    /// }
    /// ```
    fn sort_list(&mut self) {
        self.characters.sort_by(|a, b| b.last_access.cmp(&a.last_access));
    }
    
    /// Select character by index
    /// 
    /// Mirrors C# CharacterButton click handler:
    /// ```csharp
    /// _selected = index;
    /// UpdateInterface();
    /// ```
    pub fn select_character(&mut self, index: i32) {
        if index >= 0 && (index as usize) < self.characters.len() {
            self.selected_index = index;
            println!("Selected character: {}", self.characters[index as usize].name);
            // TODO: UpdateInterface() - update UI display
        }
    }
    
    /// Start game with selected character
    /// 
    /// Mirrors C# StartGame():
    /// ```csharp
    /// private void StartGame()
    /// {
    ///     // Send StartGame packet
    ///     Network.Enqueue(new C.StartGame { CharacterIndex = Characters[_selected].Index });
    /// }
    /// ```
    pub fn start_game(&mut self) {
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
    
    /// Open new character creation dialog
    /// 
    /// Mirrors C# OpenNewCharacterDialog():
    /// ```csharp
    /// private void OpenNewCharacterDialog()
    /// {
    ///     if (_character == null || _character.IsDisposed)
    ///     {
    ///         _character = new NewCharacterDialog { Parent = this };
    ///         // ...
    ///     }
    /// }
    /// ```
    pub fn open_new_character_dialog(&mut self) {
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
    
    /// Open delete character dialog
    /// 
    /// Mirrors C# DeleteCharacter():
    /// ```csharp
    /// private void DeleteCharacter()
    /// {
    ///     MirMessageBox message = new MirMessageBox(...);
    ///     message.YesButton.Click += ...
    /// }
    /// ```
    pub fn open_delete_character_dialog(&mut self) {
        if self.selected_index < 0 || (self.selected_index as usize) >= self.characters.len() {
            tracing::warn!("⚠️ 没有选中角色,无法删除");
            return;
        }
        
        let character = &self.characters[self.selected_index as usize];
        tracing::info!("🗑️ 打开删除角色对话框: {}", character.name);
        
        let dialog = DeleteCharacterDialog::new(
            character.name.clone(),
            character.index
        );
        
        self.delete_character_dialog = Some(dialog);
    }
    
    /// Open credits dialog
    pub fn open_credits_dialog(&mut self) {
        tracing::info!("📜 打开Credits对话框");
        let dialog = CreditsDialog::new();
        self.credits_dialog = Some(dialog);
        if let Some(d) = &mut self.credits_dialog {
            d.show();
        }
    }
    
    /// Submit delete character request
    /// 
    /// Mirrors C# DeleteCharacter() sending packet:
    /// ```csharp
    /// Network.Enqueue(new C.DeleteCharacter { CharacterIndex = index });
    /// ```
    fn submit_delete_character(&mut self) {
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
    
    /// 处理对话框按钮点击
    fn handle_dialog_button_click(&mut self, button: crate::scenes::select_scene::new_character_dialog::DialogButton) {
        use crate::scenes::select_scene::new_character_dialog::DialogButton;
        
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
                    dialog.selected_class = mir2_shared::enums::MirClass::Warrior;
                    tracing::debug!("选择职业: 战士");
                }
                DialogButton::Wizard => {
                    dialog.selected_class = mir2_shared::enums::MirClass::Wizard;
                    tracing::debug!("选择职业: 法师");
                }
                DialogButton::Taoist => {
                    dialog.selected_class = mir2_shared::enums::MirClass::Taoist;
                    tracing::debug!("选择职业: 道士");
                }
                DialogButton::Assassin => {
                    dialog.selected_class = mir2_shared::enums::MirClass::Assassin;
                    tracing::debug!("选择职业: 刺客");
                }
                DialogButton::Archer => {
                    dialog.selected_class = mir2_shared::enums::MirClass::Archer;
                    tracing::debug!("选择职业: 弓箭手");
                }
                DialogButton::Male => {
                    dialog.selected_gender = mir2_shared::enums::MirGender::Male;
                    tracing::debug!("选择性别: 男");
                }
                DialogButton::Female => {
                    dialog.selected_gender = mir2_shared::enums::MirGender::Female;
                    tracing::debug!("选择性别: 女");
                }
            }
        }
    }
    
    /// 绘制NewCharacterDialog
    fn draw_new_character_dialog(&self, ctx: &mut ggez::Context, canvas: &mut crate::graphics::Canvas, ggez_manager: &crate::graphics::GgezManager, dialog: &NewCharacterDialog) {
        use ggez::graphics::{Text, DrawParam, Color, Rect, Mesh, DrawMode};
        use crate::scenes::select_scene::new_character_dialog::DialogButton;
        
        // 1. 绘制半透明遮罩
        let overlay_rect = Rect::new(0.0, 0.0, 1024.0, 768.0);
        if let Ok(mesh) = Mesh::new_rectangle(ctx, DrawMode::fill(), overlay_rect, Color::from_rgba(0, 0, 0, 180)) {
            canvas.draw(&mesh, DrawParam::default());
        }
        
        // 2. 绘制对话框背景 (Prguse_73)
        if let Some(texture) = ggez_manager.get_texture("Prguse_73") {
            canvas.draw(texture, DrawParam::default().dest([dialog.x, dialog.y]));
        }
        
        // 3. 绘制标题 (Title_20, 位置: dialog_x + 206, dialog_y + 11)
        if let Some(texture) = ggez_manager.get_texture("Title_20") {
            canvas.draw(texture, DrawParam::default().dest([dialog.x + 206.0, dialog.y + 11.0]));
        }
        
        // 4. 绘制角色预览动画 (位置: dialog_x + 120, dialog_y + 250)
        let anim_index = dialog.get_animation_index();
        let anim_key = format!("ChrSel_{}", anim_index);
        
        // 获取纹理偏移量（从MLibrary）
        use crate::graphics::libraries::{get_library, LibraryName};
        let (offset_x, offset_y) = if let Some(lib_arc) = get_library(LibraryName::ChrSel) {
            let mut lib = lib_arc.lock().unwrap();
            if let Ok(info) = lib.get_image_info(anim_index as usize) {
                (info.x as f32, info.y as f32)
            } else {
                (0.0, 0.0)
            }
        } else {
            (0.0, 0.0)
        };
        
        if let Some(texture) = ggez_manager.get_texture(&anim_key) {
            // 应用偏移量（C# UseOffSet = true）
            canvas.draw(texture, DrawParam::default().dest([dialog.x + 120.0 + offset_x, dialog.y + 250.0 + offset_y]));
            
            // 法师需要绘制混合效果
            if dialog.selected_class == mir2_shared::enums::MirClass::Wizard {
                let blend_key = format!("ChrSel_{}", anim_index + 560);
                if let Some(blend_texture) = ggez_manager.get_texture(&blend_key) {
                    canvas.draw(blend_texture, DrawParam::default()
                        .dest([dialog.x + 120.0 + offset_x, dialog.y + 250.0 + offset_y])
                        .color(Color::from_rgba(255, 255, 255, 180)));
                }
            }
        }
        
        // 5. 绘制职业按钮 (Prguse_2426-2440)
        let class_buttons = [
            (DialogButton::Warrior, 2426, 2427, 2428),
            (DialogButton::Wizard, 2429, 2430, 2431),
            (DialogButton::Taoist, 2432, 2433, 2434),
            (DialogButton::Assassin, 2435, 2436, 2437),
            (DialogButton::Archer, 2438, 2439, 2440),
        ];
        
        for (btn_id, normal_idx, hover_idx, pressed_idx) in class_buttons {
            let is_selected = match (btn_id, dialog.selected_class) {
                (DialogButton::Warrior, mir2_shared::enums::MirClass::Warrior) => true,
                (DialogButton::Wizard, mir2_shared::enums::MirClass::Wizard) => true,
                (DialogButton::Taoist, mir2_shared::enums::MirClass::Taoist) => true,
                (DialogButton::Assassin, mir2_shared::enums::MirClass::Assassin) => true,
                (DialogButton::Archer, mir2_shared::enums::MirClass::Archer) => true,
                _ => false,
            };
            
            let idx = if dialog.pressed_button == Some(btn_id) {
                pressed_idx
            } else if is_selected || dialog.hovered_button == Some(btn_id) {
                hover_idx
            } else {
                normal_idx
            };
            
            let (bx, by, _, _) = dialog.get_button_rect(btn_id);
            let texture_key = format!("Prguse_{}", idx);
            if let Some(texture) = ggez_manager.get_texture(&texture_key) {
                canvas.draw(texture, DrawParam::default().dest([bx, by]));
            }
        }
        
        // 6. 绘制性别按钮 (Prguse_2420-2425)
        let gender_buttons = [
            (DialogButton::Male, 2420, 2421, 2422),
            (DialogButton::Female, 2423, 2424, 2425),
        ];
        
        for (btn_id, normal_idx, hover_idx, pressed_idx) in gender_buttons {
            let is_selected = match (btn_id, dialog.selected_gender) {
                (DialogButton::Male, mir2_shared::enums::MirGender::Male) => true,
                (DialogButton::Female, mir2_shared::enums::MirGender::Female) => true,
                _ => false,
            };
            
            let idx = if dialog.pressed_button == Some(btn_id) {
                pressed_idx
            } else if is_selected || dialog.hovered_button == Some(btn_id) {
                hover_idx
            } else {
                normal_idx
            };
            
            let (bx, by, _, _) = dialog.get_button_rect(btn_id);
            let texture_key = format!("Prguse_{}", idx);
            if let Some(texture) = ggez_manager.get_texture(&texture_key) {
                canvas.draw(texture, DrawParam::default().dest([bx, by]));
            }
        }
        
        // 7. 绘制输入框 (位置: dialog_x + 325, dialog_y + 268, 大小: 240x20)
        let input_x = dialog.x + 325.0;
        let input_y = dialog.y + 268.0;
        let input_w = 240.0;
        let input_h = 20.0;
        
        // 输入框边框
        let border_color = if let Some(_err) = &dialog.error_message {
            Color::from_rgb(255, 0, 0)  // 红色表示错误
        } else if !dialog.name.is_empty() {
            Color::from_rgb(0, 255, 0)  // 绿色表示有效
        } else {
            Color::from_rgb(128, 128, 128)  // 灰色表示空
        };
        
        let input_rect = Rect::new(input_x, input_y, input_w, input_h);
        if let Ok(mesh) = Mesh::new_rectangle(ctx, DrawMode::fill(), input_rect, Color::from_rgb(0, 0, 0)) {
            canvas.draw(&mesh, DrawParam::default());
        }
        if let Ok(mesh) = Mesh::new_rectangle(ctx, DrawMode::stroke(1.0), input_rect, border_color) {
            canvas.draw(&mesh, DrawParam::default());
        }
        
        // 输入框文字
        if !dialog.name.is_empty() {
            let mut input_text = Text::new(&dialog.name);
            input_text.set_font("AlibabaPuHuiTi");
            input_text.set_scale(14.0);
            canvas.draw(&input_text, DrawParam::default().dest([input_x + 5.0, input_y + 3.0]).color(Color::WHITE));
        }
        
        // 光标
        if dialog.input_focused && dialog.cursor_visible {
            let text_width = if dialog.cursor_position == 0 { 
                0.0 
            } else {
                let cursor_text = &dialog.name.chars().take(dialog.cursor_position).collect::<String>();
                let mut temp_text = Text::new(cursor_text);
                temp_text.set_font("AlibabaPuHuiTi");
                temp_text.set_scale(14.0);
                temp_text.measure(ctx).map(|r| r.x).unwrap_or(0.0)
            };
            let cursor_rect = Rect::new(input_x + 5.0 + text_width, input_y + 3.0, 1.0, 14.0);
            if let Ok(mesh) = Mesh::new_rectangle(ctx, DrawMode::fill(), cursor_rect, Color::WHITE) {
                canvas.draw(&mesh, DrawParam::default());
            }
        }
        
        // 8. 绘制职业描述 (位置: dialog_x + 279, dialog_y + 70, 大小: 278x170)
        let desc_text = dialog.get_class_description();
        let mut desc_label = Text::new(desc_text);
        desc_label.set_font("AlibabaPuHuiTi");
        desc_label.set_scale(12.0);
        desc_label.set_wrap(true);
        desc_label.set_bounds([278.0, 170.0]);
        canvas.draw(&desc_label, DrawParam::default().dest([dialog.x + 279.0, dialog.y + 70.0]).color(Color::WHITE));
        
        // 9. 绘制确认按钮 (Title_360/361/362, 位置: dialog_x + 160, dialog_y + 425)
        let ok_enabled = dialog.validate_name().is_ok() && !dialog.creating;
        let ok_idx = if !ok_enabled {
            360  // 禁用状态
        } else if dialog.pressed_button == Some(DialogButton::OK) {
            362  // 按下
        } else if dialog.hovered_button == Some(DialogButton::OK) {
            361  // 悬停
        } else {
            360  // 正常
        };
        
        let ok_key = format!("Title_{}", ok_idx);
        if let Some(texture) = ggez_manager.get_texture(&ok_key) {
            let color = if ok_enabled { Color::WHITE } else { Color::from_rgba(128, 128, 128, 128) };
            canvas.draw(texture, DrawParam::default().dest([dialog.x + 160.0, dialog.y + 425.0]).color(color));
        }
        
        // 10. 绘制取消按钮 (Title_280/281/282, 位置: dialog_x + 425, dialog_y + 425)
        let cancel_idx = if dialog.pressed_button == Some(DialogButton::Cancel) {
            282  // 按下
        } else if dialog.hovered_button == Some(DialogButton::Cancel) {
            281  // 悬停
        } else {
            280  // 正常
        };
        
        let cancel_key = format!("Title_{}", cancel_idx);
        if let Some(texture) = ggez_manager.get_texture(&cancel_key) {
            canvas.draw(texture, DrawParam::default().dest([dialog.x + 425.0, dialog.y + 425.0]));
        }
        
        // 11. 绘制错误消息
        if let Some(err_msg) = &dialog.error_message {
            let mut err_label = Text::new(err_msg);
            err_label.set_font("AlibabaPuHuiTi");
            err_label.set_scale(14.0);
            canvas.draw(&err_label, DrawParam::default().dest([dialog.x + 325.0, dialog.y + 290.0]).color(Color::from_rgb(255, 0, 0)));
        }
        
        // 12. 绘制创建中提示
        if dialog.creating {
            let creating_text = "正在创建角色,请稍候...";
            let mut creating_label = Text::new(creating_text);
            creating_label.set_font("AlibabaPuHuiTi");
            creating_label.set_scale(16.0);
            canvas.draw(&creating_label, DrawParam::default().dest([dialog.x + 200.0, dialog.y + 500.0]).color(Color::from_rgb(255, 255, 0)));
        }
    }
    
    /// 绘制删除角色对话框
    /// 
    /// Mirrors C# MirMessageBox (Confirmation) + MirInputBox (Name Input)
    /// - MessageBox背景: Prguse_360
    /// - InputBox背景: Prguse_660
    /// - 按钮: Title库 (Yes: 206/207/208, No: 210/211/212, OK: 200/201/202, Cancel: 203/204/205)
    fn draw_delete_character_dialog(&self, ctx: &mut ggez::Context, canvas: &mut crate::graphics::Canvas, ggez_manager: &crate::graphics::GgezManager, dialog: &DeleteCharacterDialog) {
        use ggez::graphics::{Text, Color, DrawParam};
        
        // 根据对话框状态选择不同的背景
        let (bg_key, dialog_width, dialog_height) = match dialog.state {
            delete_character_dialog::DialogState::Confirmation => {
                // MessageBox: Prguse_360
                ("Prguse_360", 464.0, 260.0)  // C# Size from MirMessageBox
            }
            delete_character_dialog::DialogState::NameInput => {
                // InputBox: Prguse_660
                ("Prguse_660", 290.0, 188.0)  // C# Size from MirInputBox
            }
        };
        
        let dialog_x = (self.window_width - dialog_width) / 2.0;
        let dialog_y = (self.window_height - dialog_height) / 2.0;
        
        // 1. 绘制对话框背景
        if let Some(bg_texture) = ggez_manager.get_texture(bg_key) {
            canvas.draw(bg_texture, DrawParam::default().dest([dialog_x, dialog_y]));
        } else {
            // 如果纹理未加载,使用简单矩形作为后备
            if let Ok(rect) = ggez::graphics::Mesh::new_rectangle(
                ctx,
                ggez::graphics::DrawMode::fill(),
                ggez::graphics::Rect::new(dialog_x, dialog_y, dialog_width, dialog_height),
                Color::from_rgba(30, 30, 40, 220),
            ) {
                canvas.draw(&rect, DrawParam::default());
            }
        }
        
        // 2. 根据状态渲染内容
        match dialog.state {
            delete_character_dialog::DialogState::Confirmation => {
                self.draw_delete_confirmation(ctx, canvas, ggez_manager, dialog, dialog_x, dialog_y);
            }
            delete_character_dialog::DialogState::NameInput => {
                self.draw_delete_name_input(ctx, canvas, ggez_manager, dialog, dialog_x, dialog_y);
            }
        }
    }
    
    /// 绘制删除确认对话框 (第一阶段: Yes/No)
    fn draw_delete_confirmation(&self, ctx: &mut ggez::Context, canvas: &mut crate::graphics::Canvas, ggez_manager: &crate::graphics::GgezManager, dialog: &DeleteCharacterDialog, dialog_x: f32, dialog_y: f32) {
        use ggez::graphics::{Text, Color, DrawParam};
        
        // 消息文本 (C# Location: 35, 35, Size: 390, 110)
        let message = format!("您确定要删除角色 {} 吗？\n\n此操作无法撤销！", dialog.character_name);
        let mut text = Text::new(&message);
        text.set_font("AlibabaPuHuiTi");
        text.set_scale(16.0);
        canvas.draw(&text, DrawParam::default()
            .dest([dialog_x + 35.0, dialog_y + 35.0])
            .color(Color::WHITE));
        
        // Yes按钮 (C# Location: 260, 157)
        let yes_key = "Title_206";  // 正常状态
        if let Some(texture) = ggez_manager.get_texture(yes_key) {
            canvas.draw(texture, DrawParam::default().dest([dialog_x + 260.0, dialog_y + 157.0]));
        }
        
        // No按钮 (C# Location: 360, 157)
        let no_key = "Title_210";  // 正常状态
        if let Some(texture) = ggez_manager.get_texture(no_key) {
            canvas.draw(texture, DrawParam::default().dest([dialog_x + 360.0, dialog_y + 157.0]));
        }
    }
    
    /// 绘制删除名称输入对话框 (第二阶段: 输入角色名)
    fn draw_delete_name_input(&self, ctx: &mut ggez::Context, canvas: &mut crate::graphics::Canvas, ggez_manager: &crate::graphics::GgezManager, dialog: &DeleteCharacterDialog, dialog_x: f32, dialog_y: f32) {
        use ggez::graphics::{Text, Color, DrawParam, Rect};
        
        // 提示文本 (C# Location: 25, 25, Size: 235, 40)
        let caption = "请输入角色名称以确认删除:";
        let mut caption_text = Text::new(caption);
        caption_text.set_font("AlibabaPuHuiTi");
        caption_text.set_scale(14.0);
        canvas.draw(&caption_text, DrawParam::default()
            .dest([dialog_x + 25.0, dialog_y + 25.0])
            .color(Color::WHITE));
        
        // 角色名提示
        let name_hint = format!("角色名: {}", dialog.character_name);
        let mut hint_text = Text::new(&name_hint);
        hint_text.set_font("AlibabaPuHuiTi");
        hint_text.set_scale(12.0);
        canvas.draw(&hint_text, DrawParam::default()
            .dest([dialog_x + 25.0, dialog_y + 55.0])
            .color(Color::from_rgb(200, 200, 200)));
        
        // 输入框背景 (C# Location: 23, 86, Size: 240, 19)
        if let Ok(input_bg) = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::fill(),
            Rect::new(dialog_x + 23.0, dialog_y + 86.0, 240.0, 19.0),
            Color::from_rgb(20, 20, 30),
        ) {
            canvas.draw(&input_bg, DrawParam::default());
        }
        
        // 输入框边框
        if let Ok(input_border) = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::stroke(1.0),
            Rect::new(dialog_x + 23.0, dialog_y + 86.0, 240.0, 19.0),
            Color::from_rgb(0, 255, 0),  // C# BorderColour = Color.Lime
        ) {
            canvas.draw(&input_border, DrawParam::default());
        }
        
        // 输入文本 + IME拼音
        let display_text = if !dialog.ime_preedit.is_empty() {
            format!("{}|{}", dialog.input_text, dialog.ime_preedit)
        } else {
            dialog.input_text.clone()
        };
        
        let mut input_text = Text::new(&display_text);
        input_text.set_font("AlibabaPuHuiTi");
        input_text.set_scale(14.0);
        canvas.draw(&input_text, DrawParam::default()
            .dest([dialog_x + 28.0, dialog_y + 88.0])
            .color(Color::WHITE));
        
        // 错误消息或状态提示
        if let Some(error) = &dialog.error_message {
            let mut error_text = Text::new(error);
            error_text.set_font("AlibabaPuHuiTi");
            error_text.set_scale(12.0);
            canvas.draw(&error_text, DrawParam::default()
                .dest([dialog_x + 25.0, dialog_y + 110.0])
                .color(Color::from_rgb(255, 100, 100)));
        } else if dialog.deleting {
            let status = "正在删除角色...";
            let mut status_text = Text::new(status);
            status_text.set_font("AlibabaPuHuiTi");
            status_text.set_scale(12.0);
            canvas.draw(&status_text, DrawParam::default()
                .dest([dialog_x + 25.0, dialog_y + 110.0])
                .color(Color::from_rgb(255, 200, 100)));
        }
        
        // OK按钮 (C# Location: 60, 123)
        let ok_key = if dialog.can_submit() {
            "Title_200"  // 正常可用
        } else {
            "Title_200"  // TODO: 需要一个灰色/禁用状态的按钮
        };
        if let Some(texture) = ggez_manager.get_texture(ok_key) {
            let alpha = if dialog.can_submit() { 1.0 } else { 0.5 };
            canvas.draw(texture, DrawParam::default()
                .dest([dialog_x + 60.0, dialog_y + 123.0])
                .color(Color::from_rgba(255, 255, 255, (alpha * 255.0) as u8)));
        }
        
        // Cancel按钮 (C# Location: 160, 123)
        let cancel_key = "Title_203";  // 正常状态
        if let Some(texture) = ggez_manager.get_texture(cancel_key) {
            canvas.draw(texture, DrawParam::default().dest([dialog_x + 160.0, dialog_y + 123.0]));
        }
    }
}

impl Default for SelectScene {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl Scene for SelectScene {
    fn scene_type(&self) -> SceneType {
        SceneType::Select
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn initialize(&mut self) {
        println!("SelectScene::initialize");
        // 窗口尺寸将在draw方法中更新
        // TODO: Load character selection UI
        // TODO: Request character list from server
    }
    
    fn update(&mut self, ctx: &mut ggez::Context, delta_time: f32) {
        // 更新 NewCharacterDialog 动画和计时器
        if let Some(dialog) = &mut self.new_character_dialog {
            dialog.update(delta_time);
        }
        
        // 更新角色预览动画 (16帧, 250ms/帧 = 4 FPS)
        self.character_animation_timer += delta_time;
        if self.character_animation_timer >= 0.25 {
            self.character_animation_timer -= 0.25;
            let old_frame = self.character_animation_frame;
            self.character_animation_frame = (self.character_animation_frame + 1) % 16;
            
            // 调试：监控帧15→0的循环重启
            if old_frame == 15 && self.character_animation_frame == 0 {
                tracing::debug!("Animation loop restart: frame 15 -> 0");
            }
        }
    }
    
    fn draw(&mut self, ctx: &mut ggez::Context, canvas: &mut crate::graphics::Canvas) {
        use ggez::graphics::{DrawParam, Color, PxScale, Text};
        use crate::graphics::libraries::{get_library, LibraryName};
        
        // 🔧 清除Canvas,防止之前场景的残留
        use ggez::graphics::{Rect, DrawMode, Mesh};
        let (screen_width, screen_height) = ctx.gfx.drawable_size();
        let clear_color = Color::from_rgb(0, 0, 0); // 黑色背景
        let clear_rect = Rect::new(0.0, 0.0, screen_width, screen_height);
        if let Ok(clear_mesh) = Mesh::new_rectangle(ctx, DrawMode::fill(), clear_rect, clear_color) {
            canvas.draw(&clear_mesh, DrawParam::default());
        }
        
        // 1. 绘制背景 Prguse_65
        if let Some(lib_arc) = get_library(LibraryName::Prguse) {
            if let Ok(mut lib) = lib_arc.try_lock() {
                let _ = lib.draw_with_color(ctx, canvas, 65, 0.0, 0.0, Color::WHITE, false);
            }
        }
        
        // 2. 绘制标题 Title_40 (C#位置: 468, 20)
        if let Some(lib_arc) = get_library(LibraryName::Title) {
            if let Ok(mut lib) = lib_arc.try_lock() {
                let _ = lib.draw_with_color(ctx, canvas, 40, 468.0, 20.0, Color::WHITE, false);
            }
        }
        
        // 2.5 绘制服务器标签 (C#位置: 432, 60, Size: 155x17, 水平居中)
        // C#: DrawFormat = TextFormatFlags.HorizontalCenter | TextFormatFlags.VerticalCenter
        // 文本在155像素宽区域内居中，中心点 = 432 + 155/2 = 509.5
        let mut server_label = Text::new("Legend of Mir 2");
        server_label.set_font("AlibabaPuHuiTi")
            .set_scale(PxScale::from(14.0));
        
        // 测量文本宽度以实现居中对齐
        let text_width = server_label.measure(ctx).map(|r| r.x).unwrap_or(120.0);
        let center_x = 432.0 + 155.0 / 2.0;  // 区域中心点
        let x = center_x - text_width / 2.0;  // 文本起始点
        
        canvas.draw(&server_label, DrawParam::default()
            .dest([x, 60.0])  // Y坐标使用C#原始值60
            .color(Color::from_rgb(200, 200, 200)));  // 浅灰色
        
        // 3. 绘制角色槽位和角色信息 (右侧垂直布局)
        // C# 代码中的原始位置: (637, 194), (637, 298), (637, 402), (637, 506)
        let character_button_positions = [
            (637.0, 194.0),
            (637.0, 298.0),
            (637.0, 402.0),
            (637.0, 506.0),
        ];
        
        for (i, character) in self.characters.iter().enumerate() {
            if i >= 4 { break; }  // 最多4个角色
            
            let (slot_x, slot_y) = character_button_positions[i];
            
            // 绘制槽位背景 (选中/未选中)
            let slot_index = if self.selected_index == i as i32 {
                665 + (character.class as i32)  // 选中状态: 665-669
            } else {
                660 + (character.class as i32)  // 未选中状态: 660-664
            };
            
            if let Some(lib_arc) = get_library(LibraryName::Title) {
                if let Ok(mut lib) = lib_arc.try_lock() {
                    let _ = lib.draw_with_color(ctx, canvas, slot_index as usize, slot_x, slot_y, Color::WHITE, false);
                }
            }
            
            // 绘制角色名称和等级信息 (使用文本)
            // C# NameLabel: Location = (107, 9), Size = (170, 18)
            let mut name_text = Text::new(&character.name);
            name_text.set_font("AlibabaPuHuiTi")
                .set_scale(PxScale::from(14.0));
            canvas.draw(&name_text, DrawParam::default()
                .dest([slot_x + 107.0, slot_y + 9.0])  // C#原始相对位置
                .color(Color::WHITE));
            
            // C# LevelLabel: Location = (107, 28), Size = (30, 18)
            let mut level_text = Text::new(format!("Lv.{}", character.level));
            level_text.set_font("AlibabaPuHuiTi")
                .set_scale(PxScale::from(13.0));
            canvas.draw(&level_text, DrawParam::default()
                .dest([slot_x + 107.0, slot_y + 28.0])  // C#原始相对位置
                .color(Color::from_rgb(200, 200, 200)));
            
            // C# ClassLabel: Location = (178, 28), Size = (100, 18)
            let class_name = match character.class {
                mir2_shared::enums::MirClass::Warrior => "Warrior",
                mir2_shared::enums::MirClass::Wizard => "Wizard",
                mir2_shared::enums::MirClass::Taoist => "Taoist",
                mir2_shared::enums::MirClass::Assassin => "Assassin",
                mir2_shared::enums::MirClass::Archer => "Archer",
            };
            let mut class_text = Text::new(class_name);
            class_text.set_font("AlibabaPuHuiTi")
                .set_scale(PxScale::from(13.0));
            canvas.draw(&class_text, DrawParam::default()
                .dest([slot_x + 178.0, slot_y + 28.0])  // C#原始相对位置
                .color(Color::from_rgb(200, 200, 200)));
        }
        
        // 4. 绘制空槽位 (如果角色少于4个)
        for i in self.characters.len()..4 {
            let (slot_x, slot_y) = character_button_positions[i];
            
            if let Some(lib_arc) = get_library(LibraryName::Prguse) {
                if let Ok(mut lib) = lib_arc.try_lock() {
                    let _ = lib.draw_with_color(ctx, canvas, 44, slot_x, slot_y, Color::WHITE, false);
                }
            }
        }
        
        // 5. 绘制选中角色的预览动画 (左侧中央)
        // C# CharacterDisplay: Location = new Point(260, 420)
        if self.selected_index >= 0 && (self.selected_index as usize) < self.characters.len() {
            let character = &self.characters[self.selected_index as usize];
            
            // 获取角色动画基础索引
            let base_index = match (character.class, character.gender) {
                (mir2_shared::enums::MirClass::Warrior, mir2_shared::enums::MirGender::Male) => 20,
                (mir2_shared::enums::MirClass::Warrior, mir2_shared::enums::MirGender::Female) => 300,
                (mir2_shared::enums::MirClass::Wizard, mir2_shared::enums::MirGender::Male) => 40,
                (mir2_shared::enums::MirClass::Wizard, mir2_shared::enums::MirGender::Female) => 320,
                (mir2_shared::enums::MirClass::Taoist, mir2_shared::enums::MirGender::Male) => 60,
                (mir2_shared::enums::MirClass::Taoist, mir2_shared::enums::MirGender::Female) => 340,
                (mir2_shared::enums::MirClass::Assassin, mir2_shared::enums::MirGender::Male) => 80,
                (mir2_shared::enums::MirClass::Assassin, mir2_shared::enums::MirGender::Female) => 360,
                (mir2_shared::enums::MirClass::Archer, mir2_shared::enums::MirGender::Male) => 100,
                (mir2_shared::enums::MirClass::Archer, mir2_shared::enums::MirGender::Female) => 140,  // C# 确认使用 140
            };
            
            let anim_index = base_index + self.character_animation_frame as i32;
            let anim_key = format!("ChrSel_{}", anim_index);
            
            // 角色预览位置（左侧中央，C#原始坐标）
            let preview_x = 260.0;
            let preview_y = 420.0;
            
            // 使用新的库系统绘制角色预览
            if let Some(lib_arc) = get_library(LibraryName::ChrSel) {
                if let Ok(mut lib) = lib_arc.try_lock() {
                    // draw_with_color 最后一个参数 use_offset=true 会自动应用偏移量
                    let _ = lib.draw_with_color(ctx, canvas, anim_index as usize, preview_x, preview_y, Color::WHITE, true);
                    
                    // 法师需要绘制混合效果（C# AfterDraw事件）
                    if character.class == mir2_shared::enums::MirClass::Wizard {
                        let blend_index = (anim_index + 560) as usize;
                        let _ = lib.draw_with_color(ctx, canvas, blend_index, preview_x, preview_y, Color::from_rgba(255, 255, 255, 180), true);
                    }
                }
            }
            
            // C# LastAccessLabel: Location = (265, 609), Size = (180, 21)
            // C# LastAccessLabelLabel: Location = (-65, 0), Parent = LastAccessLabel, Text = "Last Online:"
            // 绘制"Last Online:"标签
            let mut last_online_label = Text::new("Last Online:");
            last_online_label.set_font("AlibabaPuHuiTi")
                .set_scale(PxScale::from(13.0));
            
            // 测量标签宽度以准确计算时间位置
            let label_width = last_online_label.measure(ctx).map(|r| r.x).unwrap_or(85.0);
            
            canvas.draw(&last_online_label, DrawParam::default()
                .dest([200.0, 615.0])  // C#原始Y位置609向上调整到615
                .color(Color::from_rgb(200, 200, 200)));
            
            // 绘制最后登录时间值（紧跟在标签之后，间距5像素）
            // C#: LastAccessLabel.Text = Characters[_selected].LastAccess == DateTime.MinValue ? "Never" : Characters[_selected].LastAccess.ToString();
            // 格式化时间为短格式 (例如: "2025-10-07 14:30" 而不是完整ISO格式)
            let last_access = character.last_access.format("%Y-%m-%d %H:%M").to_string();
            let mut last_access_text = Text::new(&last_access);
            last_access_text.set_font("AlibabaPuHuiTi")
                .set_scale(PxScale::from(13.0));
            canvas.draw(&last_access_text, DrawParam::default()
                .dest([200.0 + label_width + 5.0, 615.0])  // 标签宽度 + 5像素间距
                .color(Color::WHITE));
        }
        
        // 6. 绘制底部按钮 (水平布局，带悬停效果)
        // C# 代码: Location = new Point(100 + (xPoint * n) - (xPoint / 2) - 50, Settings.ScreenHeight - 32)
        let button_y = 736.0;  // 768 - 32
        let button_spacing = 150.0;
        let button_start_x = 100.0;
        
        // 辅助函数：根据按钮状态获取纹理索引
        let get_button_index = |base: i32, button_type: BottomButton| -> i32 {
            if self.pressed_button == Some(button_type) {
                base + 2  // Pressed
            } else if self.hovered_button == Some(button_type) {
                base + 1  // Hover
            } else {
                base  // Normal
            }
        };
        
        // 绘制所有底部按钮
        if let Some(lib_arc) = get_library(LibraryName::Title) {
            if let Ok(mut lib) = lib_arc.try_lock() {
                // 开始游戏按钮 (Title_340, 341, 342)
                let start_btn_index = get_button_index(340, BottomButton::StartGame);
                let _ = lib.draw_with_color(ctx, canvas, start_btn_index as usize, button_start_x + button_spacing * 0.0, button_y, Color::WHITE, false);
                
                // 新建角色按钮 (Title_343, 344, 345)
                let new_btn_index = get_button_index(343, BottomButton::NewCharacter);
                let _ = lib.draw_with_color(ctx, canvas, new_btn_index as usize, button_start_x + button_spacing * 1.0, button_y, Color::WHITE, false);
                
                // 删除角色按钮 (Title_346, 347, 348)
                let delete_btn_index = get_button_index(346, BottomButton::DeleteCharacter);
                let _ = lib.draw_with_color(ctx, canvas, delete_btn_index as usize, button_start_x + button_spacing * 2.0, button_y, Color::WHITE, false);
                
                // 制作人员按钮 (Title_349, 350, 351)
                let credits_btn_index = get_button_index(349, BottomButton::Credits);
                let _ = lib.draw_with_color(ctx, canvas, credits_btn_index as usize, button_start_x + button_spacing * 3.0, button_y, Color::WHITE, false);
                
                // 退出游戏按钮 (Title_352, 353, 354)
                let exit_btn_index = get_button_index(352, BottomButton::ExitGame);
                let _ = lib.draw_with_color(ctx, canvas, exit_btn_index as usize, button_start_x + button_spacing * 4.0, button_y, Color::WHITE, false);
            }
        }
        
        // TODO: 6. 绘制 NewCharacterDialog (最上层)
        // 暂时禁用对话框绘制，等待完整迁移到新的库系统
        
        // TODO: 7. 绘制 DeleteCharacterDialog (最上层)
        // 暂时禁用对话框绘制
        
        // TODO: 8. 绘制 CreditsDialog (最上层)
        // 暂时禁用对话框绘制
    }
    
    fn process_event(&mut self, event: &GameEvent) {
        match event {
            GameEvent::SystemMessage { message } => {
                println!("System message: {}", message);
                // TODO: Display in UI
            }
            GameEvent::Disconnected { reason } => {
                println!("Disconnected: {}", reason);
                // TODO: Return to login
            }
            GameEvent::DeleteCharacterSuccess { character_index } => {
                tracing::info!("✅ 角色删除成功: index={}", character_index);
                
                // 1. 关闭删除对话框
                self.delete_character_dialog = None;
                
                // 2. 从角色列表移除已删除的角色
                if let Some(pos) = self.characters.iter().position(|c| c.index == *character_index) {
                    self.characters.remove(pos);
                    tracing::info!("📋 已从列表移除角色 (index={}), 剩余角色数: {}", 
                        character_index, self.characters.len());
                    
                    // 3. 更新选中索引
                    if self.selected_index >= self.characters.len() as i32 {
                        self.selected_index = if self.characters.is_empty() {
                            -1
                        } else {
                            (self.characters.len() - 1) as i32
                        };
                    }
                }
                
                // TODO: 显示成功消息框 "Your character was deleted successfully."
            }
            GameEvent::DeleteCharacterResponse { result } => {
                tracing::info!("⚠️ 删除角色响应: result={}", result);
                if let Some(dialog) = &mut self.delete_character_dialog {
                    dialog.deleting = false;
                    if *result != 0 {
                        // 删除失败
                        dialog.error_message = Some(format!("删除失败 (错误代码: {})", result));
                    }
                }
            }
            GameEvent::NewCharacterResponse { result } => {
                tracing::info!("📝 创建角色响应: result={}", result);
                if let Some(dialog) = &mut self.new_character_dialog {
                    dialog.creating = false;
                    
                    // C# SelectScene.NewCharacter(S.NewCharacter p)
                    let error_msg = match *result {
                        0 => Some("Creating new characters is currently disabled.".to_string()),
                        1 => Some("Your Character Name is not acceptable.".to_string()),
                        2 => Some("The gender you selected does not exist.\nContact a GM for assistance.".to_string()),
                        3 => Some("The class you selected does not exist.\nContact a GM for assistance.".to_string()),
                        4 => Some("You cannot make more than 4 Characters.".to_string()),
                        5 => Some("A Character with this name already exists.".to_string()),
                        _ => Some(format!("Unknown error (code: {})", result)),
                    };
                    
                    if let Some(msg) = error_msg {
                        tracing::warn!("❌ 创建角色失败: {}", msg);
                        dialog.error_message = Some(msg);
                    }
                }
            }
            GameEvent::NewCharacterSuccess { character } => {
                tracing::info!("✅ 角色创建成功: {}", character.name);
                
                // 1. 关闭新建角色对话框
                self.new_character_dialog = None;
                
                // 2. 将新角色添加到列表开头
                self.characters.insert(0, character.clone());
                
                // 3. 选中新创建的角色
                self.selected_index = 0;
                
                tracing::info!("📋 新角色已添加到列表, 总角色数: {}", self.characters.len());
                
                // TODO: 显示成功消息框 "Your character was created successfully."
            }
            GameEvent::PlayerSpawned { player } => {
                // 🎉 玩家已生成,切换到游戏场景!
                // 注意: 某些服务器实现不发送 StartGameResponse,而是直接发送 PlayerSpawned
                tracing::info!("🎮 玩家已生成: {} (Lv.{}, HP:{}/{}, MP:{}/{})", 
                    player.name, player.level, player.health, player.max_health, player.mana, player.max_mana);
                tracing::info!("📍 位置: ({}, {})", 
                    player.location.x, player.location.y);
                tracing::info!("✅ 切换到游戏场景...");
                self.pending_scene_change = Some(SceneType::Game);
            }
            GameEvent::StartGameResponse { result } => {
                tracing::info!("🎮 进入游戏响应: result={}", result);
                // Result codes from Server\MirObjects\PlayerObject.cs:
                // 0: AllowStartGame disabled but connection allowed (special case)
                // 1: Not logged in
                // 2: Character not found
                // 3: Failed to start game (validation error)
                // 4: Success! (normal case - see StartGameSuccess())
                if *result == 4 || *result == 0 {
                    // Success - queue scene transition to game
                    tracing::info!("✅ 进入游戏成功! (result={}) 切换到游戏场景...", result);
                    self.pending_scene_change = Some(SceneType::Game);
                } else {
                    // Error
                    let error_msg = match *result {
                        1 => "You are not logged in.",
                        2 => "Character not found.",
                        3 => "Failed to start game.",
                        _ => &format!("Unknown error occurred (result code: {})", result),
                    };
                    tracing::error!("❌ 进入游戏失败: {}", error_msg);
                    // TODO: 显示错误消息框
                }
            }
            GameEvent::StartGameBanned { reason, expiry_date } => {
                tracing::warn!("🚫 进入游戏被禁止: reason={}, expiry={}", reason, expiry_date);
                // TODO: 显示封禁消息框
            }
            GameEvent::StartGameDelay { milliseconds } => {
                tracing::info!("⏱️ 进入游戏延迟: {}ms", milliseconds);
                // TODO: 显示延迟提示
            }
            _ => {
                // TODO: Handle other events
            }
        }
    }
    
    fn handle_mouse_move(&mut self, x: i32, y: i32) {
        // 如果对话框可见,优先处理对话框事件
        if let Some(dialog) = &mut self.new_character_dialog {
            if dialog.visible {
                dialog.handle_mouse_move(x, y);
                return;  // 对话框捕获事件,不再传递给下层
            }
        }
        
        // 检查鼠标是否悬停在底部按钮上
        let button_y = 736.0;
        let button_spacing = 150.0;
        let button_start_x = 100.0;
        let button_height = 32.0;
        let button_width = 120.0;
        
        self.hovered_button = None;
        
        // 检查每个按钮
        let buttons = [
            (BottomButton::StartGame, 0.0),
            (BottomButton::NewCharacter, 1.0),
            (BottomButton::DeleteCharacter, 2.0),
            (BottomButton::Credits, 3.0),
            (BottomButton::ExitGame, 4.0),
        ];
        
        for (button_type, offset) in buttons {
            let btn_x = button_start_x + button_spacing * offset;
            if x >= btn_x as i32 && x <= (btn_x + button_width) as i32 &&
               y >= button_y as i32 && y <= (button_y + button_height) as i32 {
                self.hovered_button = Some(button_type);
                break;
            }
        }
    }
    
    fn handle_mouse_button(&mut self, button: crate::scenes::MouseButton, pressed: bool, x: i32, y: i32) {
        // 只处理左键
        if button != crate::scenes::MouseButton::Left {
            return;
        }
        
        // 0. 如果Credits对话框可见,点击任意位置关闭 (最上层)
        if let Some(dialog) = &mut self.credits_dialog {
            if dialog.is_visible() && pressed {
                dialog.handle_click(x as f32, y as f32, self.window_width, self.window_height);
                return;
            }
        }
        
        // 1. 如果删除对话框可见,优先处理
        if let Some(dialog) = &mut self.delete_character_dialog {
            if dialog.is_visible() && pressed {
                let (confirm, cancel, submit) = dialog.handle_click(
                    x as f32, 
                    y as f32, 
                    self.window_width, 
                    self.window_height
                );
                
                if confirm {
                    // 用户点击了"是"按钮,进入名称输入状态
                    dialog.confirm();
                    tracing::info!("✅ 用户确认删除角色");
                    return;
                } else if cancel {
                    // 用户点击了"否"或"取消"按钮
                    tracing::info!("❌ 取消删除角色");
                    self.delete_character_dialog = None;
                    return;
                } else if submit {
                    // 用户点击了"确定"按钮,发送删除请求
                    self.submit_delete_character();
                    return;
                }
                return;  // 对话框捕获事件,不再传递给下层
            }
        }
        
        // 2. 如果新建角色对话框可见,处理对话框事件
        if let Some(dialog) = &mut self.new_character_dialog {
            if dialog.visible {
                if pressed {
                    if let Some(clicked_btn) = dialog.handle_mouse_down(x, y) {
                        // 处理对话框按钮点击
                        self.handle_dialog_button_click(clicked_btn);
                    }
                } else {
                    dialog.handle_mouse_up();
                }
                return;  // 对话框捕获事件,不再传递给下层
            }
        }
        
        if pressed {
            // 设置按下状态
            self.pressed_button = self.hovered_button;
            
            tracing::debug!("SelectScene click at ({}, {}) with {:?}", x, y, button);
            
            // 检查角色槽位点击 (637, 194), (637, 298), (637, 402), (637, 506)
            let character_button_positions = [
                (637.0, 194.0, 160.0, 80.0),  // x, y, width, height (估算)
                (637.0, 298.0, 160.0, 80.0),
                (637.0, 402.0, 160.0, 80.0),
                (637.0, 506.0, 160.0, 80.0),
            ];
            
            for (i, (btn_x, btn_y, btn_w, btn_h)) in character_button_positions.iter().enumerate() {
                if x >= *btn_x as i32 && x <= (*btn_x + *btn_w) as i32 &&
                   y >= *btn_y as i32 && y <= (*btn_y + *btn_h) as i32 {
                    // 点击了角色槽位
                    if i < self.characters.len() {
                        self.selected_index = i as i32;
                        tracing::info!("Selected character {}: {}", i, self.characters[i].name);
                    }
                    return;
                }
            }
            
            // 检查底部按钮点击
            tracing::debug!("Checking bottom button click - hovered_button={:?}", self.hovered_button);
            if let Some(clicked_button) = self.hovered_button {
                tracing::info!("✅ Bottom button clicked: {:?}", clicked_button);
                match clicked_button {
                    BottomButton::StartGame => {
                        tracing::info!("🎮 Start Game clicked");
                        self.start_game();
                    }
                    BottomButton::NewCharacter => {
                        tracing::info!("New Character clicked");
                        self.open_new_character_dialog();
                    }
                    BottomButton::DeleteCharacter => {
                        tracing::info!("Delete Character clicked");
                        self.open_delete_character_dialog();
                    }
                    BottomButton::Credits => {
                        tracing::info!("📜 Credits clicked - 显示制作人员名单");
                        self.open_credits_dialog();
                    }
                    BottomButton::ExitGame => {
                        tracing::info!("🚪 Exit Game clicked - 退出游戏");
                        // C#: Program.Form.Close()
                        std::process::exit(0);
                    }
                }
            }
        } else {
            // 释放按下状态
            self.pressed_button = None;
        }
    }
    
    fn handle_text_input(&mut self, ch: char) {
        // 1. 删除对话框优先
        if let Some(dialog) = &mut self.delete_character_dialog {
            if dialog.is_visible() {
                dialog.handle_char(ch);
                return;
            }
        }
        
        // 2. 转发文本输入到新建角色对话框
        if let Some(dialog) = &mut self.new_character_dialog {
            if dialog.visible && dialog.input_focused {
                dialog.handle_text_input(ch);
            }
        }
    }
    
    fn handle_ime_preedit(&mut self, text: String) {
        // 1. 删除对话框优先
        if let Some(dialog) = &mut self.delete_character_dialog {
            if dialog.is_visible() {
                dialog.handle_ime_preedit(text);
                return;
            }
        }
        
        // 2. IME 拼音编辑中 (例如: "ni hao" 还未选择汉字)
        if let Some(dialog) = &mut self.new_character_dialog {
            if dialog.visible && dialog.input_focused {
                dialog.ime_preedit = text;
                tracing::debug!("IME 拼音编辑: {}", dialog.ime_preedit);
            }
        }
    }
    
    fn handle_ime_commit(&mut self, text: String) {
        // 1. 删除对话框优先
        if let Some(dialog) = &mut self.delete_character_dialog {
            if dialog.is_visible() {
                dialog.handle_ime_commit(text);
                return;
            }
        }
        
        // 2. IME 确认输入 (例如: 选择了"你好"这个汉字)
        if let Some(dialog) = &mut self.new_character_dialog {
            if dialog.visible && dialog.input_focused {
                dialog.ime_preedit.clear();
                // 逐字符插入到名字中
                for ch in text.chars() {
                    // 检查字符数量限制 (不是字节数)
                    if dialog.name.chars().count() >= 16 {
                        break;
                    }
                    if ch.is_ascii_alphanumeric() || ('\u{4e00}'..='\u{9fa5}').contains(&ch) {
                        // 将字符索引转换为字节索引
                        let byte_pos = dialog.name.chars().take(dialog.cursor_position).map(|c| c.len_utf8()).sum();
                        dialog.name.insert(byte_pos, ch);
                        dialog.cursor_position += 1;
                    }
                }
                tracing::info!("✓ IME 确认中文输入: {} -> 当前名字: {}", text, dialog.name);
                dialog.cursor_visible = true;
                dialog.cursor_blink_timer = 0.0;
                dialog.error_message = None;
            }
        }
    }
    
    fn handle_key_press(&mut self, key: crate::scenes::KeyCode, _modifiers: crate::scenes::ModifiersState) -> bool {
        use crate::scenes::KeyCode;
        
        // 0. Credits对话框优先处理
        if let Some(dialog) = &mut self.credits_dialog {
            if dialog.is_visible() {
                if key == KeyCode::Escape {
                    dialog.hide();
                    return true;
                }
            }
        }
        
        // 1. 删除对话框优先处理
        if let Some(dialog) = &mut self.delete_character_dialog {
            if dialog.is_visible() {
                match key {
                    KeyCode::Backspace => {
                        dialog.handle_backspace();
                        return true;
                    }
                    KeyCode::Enter => {
                        // Enter键提交删除请求
                        if dialog.can_submit() {
                            self.submit_delete_character();
                        }
                        return true;
                    }
                    KeyCode::Escape => {
                        // Escape关闭对话框
                        tracing::info!("❌ 按ESC取消删除角色");
                        self.delete_character_dialog = None;
                        return true;
                    }
                    _ => {}
                }
            }
        }
        
        // 2. 如果新建角色对话框可见且输入框有焦点,处理文本输入相关按键
        if let Some(dialog) = &mut self.new_character_dialog {
            if dialog.visible && dialog.input_focused {
                match key {
                    KeyCode::Backspace => {
                        dialog.handle_backspace();
                        return true;
                    }
                    // KeyCode::Delete => {
                    //     dialog.handle_delete();
                    //     return true;
                    // }
                    KeyCode::ArrowLeft => {
                        dialog.handle_left_arrow();
                        return true;
                    }
                    KeyCode::ArrowRight => {
                        dialog.handle_right_arrow();
                        return true;
                    }
                    KeyCode::Enter => {
                        // Enter键提交角色创建
                        if dialog.validate_name().is_ok() && !dialog.creating {
                            self.handle_dialog_button_click(crate::scenes::select_scene::new_character_dialog::DialogButton::OK);
                        }
                        return true;
                    }
                    KeyCode::Escape => {
                        // Escape关闭对话框
                        dialog.hide();
                        return true;
                    }
                    _ => {}
                }
            } else if dialog.visible {
                // 对话框可见但输入框没焦点
                match key {
                    KeyCode::Escape => {
                        dialog.hide();
                        return true;
                    }
                    _ => {}
                }
            }
        }
        
        // 场景级别的按键处理
        match key {
            KeyCode::Enter => {
                self.start_game();
                true
            }
            KeyCode::Escape => {
                // TODO: Return to login scene
                tracing::info!("Escape pressed - would return to login");
                true
            }
            _ => false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mir2_shared::enums::{MirClass, MirGender};

    fn create_test_character(index: i32, name: &str, timestamp: i64) -> SelectInfo {
        use std::io::Cursor;
        use byteorder::{LittleEndian, WriteBytesExt};
        
        // Create a minimal binary representation
        let mut buffer = Vec::new();
        buffer.write_i32::<LittleEndian>(index).unwrap();
        // Write string length + string
        let name_bytes = name.as_bytes();
        buffer.push(name_bytes.len() as u8);
        buffer.extend_from_slice(name_bytes);
        buffer.write_u16::<LittleEndian>(1).unwrap(); // level
        buffer.write_u8(MirClass::Warrior as u8).unwrap();
        buffer.write_u8(MirGender::Male as u8).unwrap();
        buffer.write_i64::<LittleEndian>(timestamp).unwrap();
        
        let mut cursor = Cursor::new(buffer);
        SelectInfo::read_from(&mut cursor).unwrap()
    }

    #[test]
    fn test_select_scene_creation() {
        let now = 638000000000000000i64; // Some .NET DateTime ticks
        let characters = vec![
            create_test_character(0, "TestChar1", now),
            create_test_character(1, "TestChar2", now),
        ];
        let scene = SelectScene::new(characters);
        assert_eq!(scene.scene_type(), SceneType::Select);
        assert_eq!(scene.characters.len(), 2);
        assert_eq!(scene.selected_index, 0);
    }

    #[test]
    fn test_character_selection() {
        let now = 638000000000000000i64;
        let characters = vec![
            create_test_character(0, "TestChar1", now),
            create_test_character(1, "TestChar2", now),
            create_test_character(2, "TestChar3", now),
        ];
        let mut scene = SelectScene::new(characters);
        
        scene.select_character(1);
        assert_eq!(scene.selected_index, 1);
        
        // Out of bounds should be ignored
        scene.select_character(10);
        assert_eq!(scene.selected_index, 1);
        
        // Negative index should be ignored
        scene.select_character(-1);
        assert_eq!(scene.selected_index, 1);
    }
    
    #[test]
    fn test_sort_characters_by_last_access() {
        let old_time = 638000000000000000i64;
        let new_time = 638000100000000000i64; // Later time
        
        let characters = vec![
            create_test_character(0, "Old", old_time),
            create_test_character(1, "Recent", new_time),
        ];
        
        let scene = SelectScene::new(characters);
        // Should be sorted by last_access descending (most recent first)
        assert_eq!(scene.characters[0].name, "Recent");
        assert_eq!(scene.characters[1].name, "Old");
    }
}
