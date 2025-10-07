// SelectScene - Character selection scene
// Mirrors Client/MirScenes/SelectScene.cs
pub mod new_character_dialog;
pub mod delete_character_dialog;

pub use new_character_dialog::NewCharacterDialog;
pub use delete_character_dialog::DeleteCharacterDialog;

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
    
    // Network
    command_tx: Option<tokio::sync::mpsc::UnboundedSender<crate::network::NetworkCommand>>,
    
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
            command_tx: None,
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
        if self.selected_index >= 0 && (self.selected_index as usize) < self.characters.len() {
            let character = &self.characters[self.selected_index as usize];
            println!("Starting game with character: {}", character.name);
            // TODO: Send StartGame packet
            // TODO: Switch to game scene
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
    
    fn update(&mut self, delta_time: f32) {
        // 更新 NewCharacterDialog 动画和计时器
        if let Some(dialog) = &mut self.new_character_dialog {
            dialog.update(delta_time);
        }
        
        // 更新角色预览动画 (16帧, 250ms/帧 = 4 FPS)
        self.character_animation_timer += delta_time;
        if self.character_animation_timer >= 0.25 {
            self.character_animation_timer -= 0.25;
            self.character_animation_frame = (self.character_animation_frame + 1) % 16;
        }
    }
    
    fn draw(&self, ctx: &mut ggez::Context, canvas: &mut crate::graphics::Canvas, ggez_manager: &crate::graphics::GgezManager) {
        use ggez::graphics::{DrawParam, Color, PxScale, Text};
        
        // 1. 绘制背景 Prguse_65
        if let Some(bg_texture) = ggez_manager.get_texture("Prguse_65") {
            let draw_param = DrawParam::default()
                .dest([0.0, 0.0])
                .color(Color::WHITE);
            canvas.draw(bg_texture, draw_param);
        } else {
            tracing::warn!("背景纹理未找到: Prguse_65");
        }
        
        // 2. 绘制标题 Title_40 (C#位置: 468, 20)
        if let Some(title_texture) = ggez_manager.get_texture("Title_40") {
            let draw_param = DrawParam::default()
                .dest([468.0, 20.0])  // C#原始位置
                .color(Color::WHITE);
            canvas.draw(title_texture, draw_param);
        } else {
            // 如果Title_40不存在，显示文本标题作为后备
            let mut title_text = Text::new("SELECT CHARACTER");
            title_text.set_font("AlibabaPuHuiTi")
                .set_scale(PxScale::from(32.0));
            canvas.draw(&title_text, DrawParam::default()
                .dest([400.0, 30.0])
                .color(Color::from_rgb(255, 215, 0)));  // 金色
            tracing::warn!("标题纹理未找到: Title_40");
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
            
            if let Some(slot_texture) = ggez_manager.get_texture(&format!("Title_{}", slot_index)) {
                let draw_param = DrawParam::default()
                    .dest([slot_x, slot_y])
                    .color(Color::WHITE);
                canvas.draw(slot_texture, draw_param);
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
            
            if let Some(empty_texture) = ggez_manager.get_texture("Prguse_44") {
                let draw_param = DrawParam::default()
                    .dest([slot_x, slot_y])
                    .color(Color::WHITE);
                canvas.draw(empty_texture, draw_param);
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
            
            // 调试：记录纹理获取尝试（只在帧0记录，避免刷屏）
            if self.character_animation_frame == 0 {
                tracing::debug!("Character animation loop restart: frame={}, base_index={}, anim_index={}, key={}", 
                    self.character_animation_frame, base_index, anim_index, anim_key);
            }
            
            // 角色预览位置（左侧中央，C#原始坐标）
            let preview_x = 260.0;
            let preview_y = 420.0;
            
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
                canvas.draw(texture, DrawParam::default().dest([preview_x + offset_x, preview_y + offset_y]));
                
                // 法师需要绘制混合效果
                if character.class == mir2_shared::enums::MirClass::Wizard {
                    let blend_key = format!("ChrSel_{}", anim_index + 560);
                    if let Some(blend_texture) = ggez_manager.get_texture(&blend_key) {
                        canvas.draw(blend_texture, DrawParam::default()
                            .dest([preview_x + offset_x, preview_y + offset_y])
                            .color(Color::from_rgba(255, 255, 255, 180)));
                    } else if self.character_animation_frame == 0 {
                        tracing::warn!("Wizard blend texture not found: {}", blend_key);
                    }
                }
            } else {
                // 纹理缺失！这会导致角色不显示（闪烁）
                if self.character_animation_frame == 0 {
                    tracing::error!("Character texture not found at loop restart: {}", anim_key);
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
        
        // 开始游戏按钮 (Title_340, 341, 342)
        let start_btn_index = get_button_index(340, BottomButton::StartGame);
        if let Some(start_btn) = ggez_manager.get_texture(&format!("Title_{}", start_btn_index)) {
            canvas.draw(start_btn, DrawParam::default().dest([button_start_x + button_spacing * 0.0, button_y]));
        }
        
        // 新建角色按钮 (Title_343, 344, 345)
        let new_btn_index = get_button_index(343, BottomButton::NewCharacter);
        if let Some(new_btn) = ggez_manager.get_texture(&format!("Title_{}", new_btn_index)) {
            canvas.draw(new_btn, DrawParam::default().dest([button_start_x + button_spacing * 1.0, button_y]));
        }
        
        // 删除角色按钮 (Title_346, 347, 348)
        let delete_btn_index = get_button_index(346, BottomButton::DeleteCharacter);
        if let Some(delete_btn) = ggez_manager.get_texture(&format!("Title_{}", delete_btn_index)) {
            canvas.draw(delete_btn, DrawParam::default().dest([button_start_x + button_spacing * 2.0, button_y]));
        }
        
        // 制作人员按钮 (Title_349, 350, 351)
        let credits_btn_index = get_button_index(349, BottomButton::Credits);
        if let Some(credits_btn) = ggez_manager.get_texture(&format!("Title_{}", credits_btn_index)) {
            canvas.draw(credits_btn, DrawParam::default().dest([button_start_x + button_spacing * 3.0, button_y]));
        }
        
        // 退出游戏按钮 (Title_352, 353, 354)
        let exit_btn_index = get_button_index(352, BottomButton::ExitGame);
        if let Some(exit_btn) = ggez_manager.get_texture(&format!("Title_{}", exit_btn_index)) {
            canvas.draw(exit_btn, DrawParam::default().dest([button_start_x + button_spacing * 4.0, button_y]));
        }
        
        // 6. 绘制 NewCharacterDialog (最上层)
        if let Some(dialog) = &self.new_character_dialog {
            if dialog.visible {
                self.draw_new_character_dialog(ctx, canvas, ggez_manager, dialog);
            }
        }
        
        // 7. 绘制 DeleteCharacterDialog (最上层)
        if let Some(dialog) = &self.delete_character_dialog {
            if dialog.is_visible() {
                self.draw_delete_character_dialog(ctx, canvas, ggez_manager, dialog);
            }
        }
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
            _ => {
                // TODO: Handle character creation/deletion events when added to GameEvent
                // For now, ignore other events
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
        
        // 1. 如果删除对话框可见,优先处理 (最上层)
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
            if let Some(clicked_button) = self.hovered_button {
                match clicked_button {
                    BottomButton::StartGame => {
                        tracing::info!("Start Game clicked");
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
                        // TODO: 显示制作人员对话框
                        tracing::info!("制作人员:\n  原作: Wemade Entertainment\n  Rust版本: Crystal Project Team");
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
