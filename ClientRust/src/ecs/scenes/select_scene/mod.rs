// SelectScene - Character selection scene
// Mirrors Client/MirScenes/SelectScene.cs

// UI 组件子模块（每个组件负责自己的绘制和事件处理）
mod character_select;
pub mod credits_dialog;
pub mod delete_character_dialog;
mod message_box; // 🆕 SelectScene 专用消息框
pub mod new_character_dialog; // 🆕 角色选择主界面绘制

// 业务逻辑子模块（按照单一职责原则分离）
mod network_handler;
mod ui_actions; // UI 交互逻辑（按钮点击、对话框打开、游戏启动等） // 网络事件处理（服务器响应处理）

use character_select::CharacterSelect;

pub use credits_dialog::CreditsDialog;
pub use delete_character_dialog::DeleteCharacterDialog;
pub use message_box::{MessageBox, MessageBoxButtons, MessageBoxResult};
pub use new_character_dialog::NewCharacterDialog; // 🆕 导出消息框

use ggez::graphics::Canvas;
use ggez::{Context, GameResult};
use hecs::World;
use tokio::sync::mpsc;

use super::ui::InputBox; // 🆕 只导入 InputBox
use super::{Scene, SceneType};
use crate::ecs::ui::{ButtonGroup, ButtonWidget}; // ButtonGroup 从原路径导入
use crate::network::NetworkCommand;
use mir2_shared::SelectInfo;

// 设计分辨率常量 (与 LoginScene 保持一致)
const DESIGN_WIDTH: f32 = 1024.0;
const DESIGN_HEIGHT: f32 = 768.0;


pub struct SelectScene {
    // 🆕 角色选择主界面组件（封装角色列表、选中状态、动画等）
    character_select: CharacterSelect,

    // Dialogs
    /// Mirrors C# `private NewCharacterDialog _character`
    pub new_character_dialog: Option<NewCharacterDialog>,
    /// Delete character dialog
    pub delete_character_dialog: Option<DeleteCharacterDialog>,
    /// Credits dialog
    pub credits_dialog: Option<CreditsDialog>,

    // 🆕 消息框和输入框
    pub message_box: Option<MessageBox>,
    pub input_box: Option<InputBox>,

    // Network
    command_tx: Option<tokio::sync::mpsc::UnboundedSender<crate::network::NetworkCommand>>,

    // Scene transition
    pub pending_scene_change: Option<SceneType>,

    // 🆕 按钮组管理器(替代手动状态管理)
    bottom_buttons: ButtonGroup,

    // UI state (保留用于对话框)
    hovered_button: Option<BottomButton>, // TODO: 可以删除,由 ButtonGroup 管理
    pressed_button: Option<BottomButton>, // TODO: 可以删除,由 ButtonGroup 管理
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
        // 创建底部按钮组
        let mut bottom_buttons = ButtonGroup::new();

        // 按钮布局常量
        const BUTTON_Y: f32 = 736.0;
        const BUTTON_WIDTH: f32 = 96.0;
        const BUTTON_HEIGHT: f32 = 32.0;
        const BUTTON_SPACING: f32 = 150.0;
        const BUTTON_START_X: f32 = 100.0;

        // 添加5个底部按钮 (使用 Builder 模式添加工具提示)
        bottom_buttons.add(
            ButtonWidget::new(
                1,
                BUTTON_START_X,
                BUTTON_Y,
                BUTTON_WIDTH,
                BUTTON_HEIGHT,
                340,
            )
            .with_tooltip("开始游戏 (Enter)"),
        );
        bottom_buttons.add(
            ButtonWidget::new(
                2,
                BUTTON_START_X + BUTTON_SPACING,
                BUTTON_Y,
                BUTTON_WIDTH,
                BUTTON_HEIGHT,
                343,
            )
            .with_tooltip("新建角色"),
        );
        bottom_buttons.add(
            ButtonWidget::new(
                3,
                BUTTON_START_X + BUTTON_SPACING * 2.0,
                BUTTON_Y,
                BUTTON_WIDTH,
                BUTTON_HEIGHT,
                346,
            )
            .with_tooltip("删除角色 (Delete)"),
        );
        bottom_buttons.add(
            ButtonWidget::new(
                4,
                BUTTON_START_X + BUTTON_SPACING * 3.0,
                BUTTON_Y,
                BUTTON_WIDTH,
                BUTTON_HEIGHT,
                349,
            )
            .with_tooltip("制作人员"),
        );
        bottom_buttons.add(
            ButtonWidget::new(
                5,
                BUTTON_START_X + BUTTON_SPACING * 4.0,
                BUTTON_Y,
                BUTTON_WIDTH,
                BUTTON_HEIGHT,
                352,
            )
            .with_tooltip("退出游戏 (ESC)"),
        );

        // 排序角色列表
        let mut sorted_characters = characters;
        sorted_characters.sort_by(|a, b| b.last_access.cmp(&a.last_access));

        // 创建角色选择组件
        let character_select = CharacterSelect::new(sorted_characters);

        Self {
            character_select,
            new_character_dialog: None,
            delete_character_dialog: None,
            credits_dialog: None,
            message_box: None,
            input_box: None,
            command_tx: None,
            pending_scene_change: None,
            bottom_buttons, // 🆕 使用 ButtonGroup
            hovered_button: None,
            pressed_button: None,
        }
    }

    /// 设置网络命令发送器
    pub fn set_command_sender(
        &mut self,
        tx: tokio::sync::mpsc::UnboundedSender<crate::network::NetworkCommand>,
    ) {
        self.command_tx = Some(tx);
    }

    /// Select character by index
    ///
    /// Mirrors C# CharacterButton click handler:
    /// ```csharp
    /// _selected = index;
    /// UpdateInterface();
    /// ```
    pub fn select_character(&mut self, index: i32) {
        self.character_select.select_character(index);
        if let Some(character) = self.character_select.get_selected_character() {
            println!("Selected character: {}", character.name);
        }
    }

    /// 获取当前选中的角色索引
    pub fn get_selected_character_index(&self) -> i32 {
        self.character_select.get_selected_index()
    }

    fn window_to_design_coords(&self, ctx: &Context, window_x: f32, window_y: f32) -> (f32, f32) {
        let (window_width,window_height) = ctx.gfx.drawable_size();
        // 计算4:3视口
        let aspect_ratio = 4.0 / 3.0;
        let current_ratio = window_width / window_height;

        let (viewport_width, viewport_height) = if current_ratio > aspect_ratio {
            (window_height * aspect_ratio, window_height)
        } else {
            (window_width, window_width / aspect_ratio)
        };

        let offset_x = (window_width - viewport_width) / 2.0;
        let offset_y = (window_height - viewport_height) / 2.0;

        // 转换：窗口坐标 -> 视口坐标 -> 设计坐标
        let viewport_x = window_x - offset_x;
        let viewport_y = window_y - offset_y;

        let design_x = (viewport_x / viewport_width) * DESIGN_WIDTH;
        let design_y = (viewport_y / viewport_height) * DESIGN_HEIGHT;

        (design_x, design_y)
    }
}

impl Default for SelectScene {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl Scene for SelectScene {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(
        &mut self,
        ctx: &mut Context,
        _world: &mut World,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult<Option<SceneType>> {
        let delta = ctx.time.delta().as_secs_f32();

        // 更新 NewCharacterDialog 动画和计时器
        if let Some(dialog) = &mut self.new_character_dialog {
            dialog.update(delta);
        }

        // ✅ 更新 InputBox (光标闪烁)
        if let Some(ref mut input_box) = self.input_box {
            input_box.update(delta);
        }

        // 🆕 更新角色选择组件（动画）
        self.character_select.update(delta);

        // 🆕 处理消息框结果
        if let Some(ref mut message_box) = self.message_box {
            if message_box.has_result() {
                let result = message_box.result;
                self.message_box = None; // 关闭消息框

                // 根据结果执行相应操作
                match result {
                    MessageBoxResult::Ok => {
                        tracing::debug!("✅ 消息框: 用户点击OK");
                    }
                    MessageBoxResult::Yes => {
                        tracing::debug!("✅ 消息框: 用户点击Yes - 显示输入框");
                        // 删除角色确认后，显示输入框验证
                        if self.character_select.get_selected_character().is_some() {
                            let mut input_box =
                                InputBox::new("Please enter the character's name.".to_string());
                            input_box.show(ctx); // ✅ 传入 ctx 启用 IME
                            self.input_box = Some(input_box);
                        }
                    }
                    MessageBoxResult::No => {
                        tracing::debug!("❌ 消息框: 用户点击No");
                    }
                    _ => {}
                }
            }
        }

        // 🆕 处理输入框结果
        if let Some(ref mut input_box) = self.input_box {
            if input_box.confirmed {
                let input_text = input_box.get_input().to_string();
                tracing::info!("✅ 输入框确认: {}", input_text);

                // 验证角色名并发送删除请求
                if let Some(character) = self.character_select.get_selected_character() {
                    if input_text == character.name {
                        tracing::info!("🗑️ 发送删除角色请求: index={}", character.index);
                        // 发送 DeleteCharacter 包
                        let _ = network_tx.send(NetworkCommand::DeleteCharacter {
                            index: character.index,
                        });
                    } else {
                        // 名称不匹配，显示错误消息
                        let mut msg = MessageBox::new(
                            "Incorrect Entry.".to_string(),
                            MessageBoxButtons::Ok,
                            DESIGN_WIDTH,
                            DESIGN_HEIGHT,
                        );
                        msg.show();
                        self.message_box = Some(msg);
                    }
                }

                // ✅ 关闭输入框时禁用 IME
                input_box.hide(ctx);
                self.input_box = None;
            } else if input_box.cancelled {
                tracing::info!("❌ 输入框取消");
                // ✅ 关闭输入框时禁用 IME
                input_box.hide(ctx);
                self.input_box = None;
            }
        }

        // 检查场景切换
        if let Some(scene_type) = self.pending_scene_change.take() {
            return Ok(Some(scene_type));
        }

        Ok(None)
    }

    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, _world: &World) -> GameResult {
        // 设置画布使用设计分辨率坐标系（1024x768）
        // ggez会自动缩放到窗口大小，保持4:3比例
        canvas.set_screen_coordinates(ggez::graphics::Rect::new(
            0.0,
            0.0,
            DESIGN_WIDTH,
            DESIGN_HEIGHT,
        ));

        // 🆕 使用 CharacterSelect 组件绘制主界面
        self.character_select
            .draw(ctx, canvas, &self.bottom_buttons)?;

        // TODO: 8. 绘制 NewCharacterDialog (最上层)
        if let Some(ref mut dialog) = self.new_character_dialog {
            if dialog.visible {
                dialog.draw(ctx, canvas)?;
            }
        }

        // TODO: 9. 绘制 DeleteCharacterDialog (最上层)
        // 暂时禁用对话框绘制

        // 10. 🆕 绘制 CreditsDialog (最上层)
        if let Some(ref dialog) = self.credits_dialog {
            if dialog.visible {
                let _ = dialog.draw(ctx, canvas);
            }
        }

        // 11. 🆕 绘制消息框 (最上层)
        if let Some(ref message_box) = self.message_box {
            let _ = message_box.draw(ctx, canvas); // 使用正确的 MessageBox
        }

        // 12. 🆕 绘制输入框 (最上层)
        if let Some(ref mut input_box) = self.input_box {
            input_box.draw(ctx, canvas)?;
        }

        Ok(())
    }

    fn on_mouse_down(
        &mut self,
        ctx: &mut Context,
        _world: &mut World,
        button: ggez::winit::event::MouseButton,
        x: f32,
        y: f32,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult {
        use ggez::winit::event::MouseButton;

        // 🔧 转换窗口坐标为设计坐标
        let (design_x, design_y) = self.window_to_design_coords(ctx, x, y);

        // 调试：输出转换后的坐标
        tracing::debug!(
            "🖱️ SelectScene 鼠标点击: 窗口({:.0}, {:.0}) -> 设计({:.0}, {:.0}), 按钮: {:?}",
            x,
            y,
            design_x,
            design_y,
            button
        );

        // 只处理左键点击
        if button != MouseButton::Left {
            return Ok(());
        }

        // 🆕 0. 优先处理输入框（最上层）
        if let Some(ref mut input_box) = self.input_box {
            if input_box.visible {
                input_box.on_mouse_down(design_x, design_y, ctx); // ✅ 传入 ctx
                return Ok(()); // 输入框消费了事件
            }
        }

        // 🆕 0.3. 处理 CreditsDialog
        if let Some(ref mut dialog) = self.credits_dialog {
            if dialog.visible {
                dialog.hide(); // 点击任意位置关闭
                return Ok(());
            }
        }

        // 🆕 0.5. 处理消息框
        if let Some(ref mut message_box) = self.message_box {
            if message_box.visible {
                message_box.on_mouse_down(design_x, design_y);
                return Ok(()); // 消息框消费了事件
            }
        }

        // 1. 处理对话框点击（优先级最高）
        if let Some(ref mut dialog) = self.new_character_dialog {
            if dialog.visible {
                if let Some(button) = dialog.handle_mouse_down(design_x as i32, design_y as i32) {
                    self.handle_new_character_button(button);
                    return Ok(()); // 对话框消费了点击事件
                }
            }
        }

        if let Some(_dialog) = &mut self.delete_character_dialog {
            // TODO: 实现 DeleteCharacterDialog 的点击处理
        }

        // 2. 处理角色槽位点击 (右侧垂直布局) - 使用设计坐标
        if let Some(slot_index) = self.character_select.check_slot_click(design_x, design_y) {
            self.select_character(slot_index as i32);
            if let Some(character) = self.character_select.get_selected_character() {
                tracing::info!("🖱️ 选中角色: {}", character.name);
            }
            return Ok(());
        }

        // 3. 处理底部按钮点击 (使用 ButtonGroup) - 使用设计坐标
        if let Some(button_id) = self.bottom_buttons.on_mouse_down(design_x, design_y) {
            tracing::info!(
                "🖱️ 点击按钮 ID: {} at 设计({:.0}, {:.0})",
                button_id,
                design_x,
                design_y
            );

            // 根据按钮ID分发事件
            match button_id {
                1 => self.handle_button_click(BottomButton::StartGame, network_tx),
                2 => self.handle_button_click(BottomButton::NewCharacter, network_tx),
                3 => self.handle_button_click(BottomButton::DeleteCharacter, network_tx),
                4 => self.handle_button_click(BottomButton::Credits, network_tx),
                5 => self.handle_button_click(BottomButton::ExitGame, network_tx),
                _ => {}
            }
            return Ok(());
        }

        Ok(())
    }

    fn on_mouse_up(
        &mut self,
        ctx: &mut Context,
        _world: &mut World,
        _button: ggez::winit::event::MouseButton,
        x: f32,
        y: f32,
        _network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult {
        // 🔧 转换窗口坐标为设计坐标
        let (design_x, design_y) = self.window_to_design_coords(ctx, x, y);

        // 🆕 处理新建角色对话框鼠标释放
        if let Some(ref mut dialog) = self.new_character_dialog {
            if dialog.visible {
                dialog.handle_mouse_up();
            }
        }

        // 使用 ButtonGroup 处理释放 (如果需要触发点击) - 使用设计坐标
        if let Some(button_id) = self.bottom_buttons.on_mouse_up(design_x, design_y) {
            tracing::info!("✅ 按钮点击完成: {}", button_id);
            // 按下和释放都在同一按钮内才触发(已在 on_mouse_down 处理)
        }

        // 清除旧的按下状态 (兼容性)
        self.pressed_button = None;
        Ok(())
    }

    fn on_mouse_move(
        &mut self,
        ctx: &mut Context,
        _world: &mut World,
        x: f32,
        y: f32,
    ) -> GameResult {
        // 🔧 转换窗口坐标为设计坐标
        let (design_x, design_y) = self.window_to_design_coords(ctx, x, y);

        // 🆕 0. 优先处理输入框鼠标移动
        if let Some(ref mut input_box) = self.input_box {
            if input_box.visible {
                input_box.on_mouse_move(design_x, design_y);
                return Ok(());
            }
        }

        // 🆕 0.5. 处理消息框鼠标移动
        if let Some(ref mut message_box) = self.message_box {
            if message_box.visible {
                message_box.on_mouse_move(design_x, design_y);
                return Ok(());
            }
        }

        // 🆕 0.6. 处理新建角色对话框鼠标移动
        if let Some(ref mut dialog) = self.new_character_dialog {
            if dialog.visible {
                dialog.handle_mouse_move(design_x as i32, design_y as i32);
                return Ok(());
            }
        }

        // 处理对话框鼠标移动 - 使用设计坐标
        if let Some(_dialog) = &mut self.delete_character_dialog {
            // TODO: 实现对话框鼠标移动
        }

        // 使用 ButtonGroup 自动更新悬停状态 - 使用设计坐标
        self.bottom_buttons.update_hover(design_x, design_y);

        // 兼容旧代码:同步 hovered_button 字段 (TODO: 删除)
        self.hovered_button = None;
        for (i, button) in self.bottom_buttons.buttons.iter().enumerate() {
            if button.state == crate::ecs::ui::ButtonState::Hovered
                || button.state == crate::ecs::ui::ButtonState::Pressed
            {
                self.hovered_button = Some(match i {
                    0 => BottomButton::StartGame,
                    1 => BottomButton::NewCharacter,
                    2 => BottomButton::DeleteCharacter,
                    3 => BottomButton::Credits,
                    4 => BottomButton::ExitGame,
                    _ => continue,
                });
                break;
            }
        }

        Ok(())
    }

    fn on_key_down(
        &mut self,
        ctx: &mut Context, // ✅ 去掉下划线，需要传给InputBox
        _world: &mut World,
        input: ggez::input::keyboard::KeyInput,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult<Option<SceneType>> {
        use ggez::winit::keyboard::KeyCode;

        // 检查物理键码
        if let ggez::winit::event::KeyEvent {
            physical_key: ggez::winit::keyboard::PhysicalKey::Code(keycode),
            ..
        } = &input.event
        {
            // 🆕 优先处理新建角色对话框的文本输入
            if let Some(ref mut dialog) = self.new_character_dialog {
                if dialog.visible && dialog.input_focused {
                    match keycode {
                        KeyCode::Backspace => {
                            dialog.handle_backspace();
                            return Ok(None);
                        }
                        KeyCode::Delete => {
                            dialog.handle_delete();
                            return Ok(None);
                        }
                        KeyCode::Escape => {
                            dialog.hide();
                            tracing::info!("❌ ESC 关闭新建角色对话框");
                            return Ok(None);
                        }
                        _ => {}
                    }
                }
            }

            // 处理输入框按键
            if let Some(ref mut input_box) = self.input_box {
                if input_box.visible {
                    input_box.on_key_down(*keycode, ctx); // ✅ 传入 ctx
                                                          // 检查是否已确认或取消
                    if input_box.confirmed || input_box.cancelled {
                        // 在update中处理结果
                    }
                    return Ok(None);
                }
            }

            match keycode {
                KeyCode::Escape => {
                    // ESC 键关闭对话框或退出
                    if self.new_character_dialog.is_some() {
                        self.new_character_dialog = None;
                        tracing::info!("❌ 关闭新建角色对话框");
                    } else if self.delete_character_dialog.is_some() {
                        self.delete_character_dialog = None;
                        tracing::info!("❌ 关闭删除角色对话框");
                    } else if self.credits_dialog.is_some() {
                        self.credits_dialog = None;
                        tracing::info!("❌ 关闭制作人员对话框");
                    } else {
                        // TODO: 返回登录场景或退出游戏
                        tracing::info!("❌ ESC 键: 退出角色选择");
                    }
                }
                KeyCode::Enter | KeyCode::NumpadEnter => {
                    // Enter 键开始游戏
                    if self.character_select.get_selected_index() >= 0 {
                        self.handle_button_click(BottomButton::StartGame, network_tx);
                    }
                }
                KeyCode::Digit1 | KeyCode::Numpad1 => {
                    if !self.character_select.get_characters().is_empty() {
                        self.select_character(0);
                        if let Some(character) = self.character_select.get_selected_character() {
                            tracing::info!("⌨️ 键盘选中角色1: {}", character.name);
                        }
                    }
                }
                KeyCode::Digit2 | KeyCode::Numpad2 => {
                    if self.character_select.get_characters().len() > 1 {
                        self.select_character(1);
                        if let Some(character) = self.character_select.get_selected_character() {
                            tracing::info!("⌨️ 键盘选中角色2: {}", character.name);
                        }
                    }
                }
                KeyCode::Digit3 | KeyCode::Numpad3 => {
                    if self.character_select.get_characters().len() > 2 {
                        self.select_character(2);
                        if let Some(character) = self.character_select.get_selected_character() {
                            tracing::info!("⌨️ 键盘选中角色3: {}", character.name);
                        }
                    }
                }
                KeyCode::Digit4 | KeyCode::Numpad4 => {
                    if self.character_select.get_characters().len() > 3 {
                        self.select_character(3);
                        if let Some(character) = self.character_select.get_selected_character() {
                            tracing::info!("⌨️ 键盘选中角色4: {}", character.name);
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(None)
    }

    /// 🆕 文本输入事件 (IME 支持)
    fn on_text_input(
        &mut self,
        _ctx: &mut Context,
        _world: &mut World,
        character: String,
    ) -> GameResult {
        tracing::debug!("📝 SelectScene::on_text_input: '{}'", character);

        // 🆕 优先处理新建角色对话框的文本输入
        if let Some(ref mut dialog) = self.new_character_dialog {
            if dialog.visible && dialog.input_focused {
                tracing::debug!("📝 转发到新建角色对话框");
                // 处理每个字符
                for ch in character.chars() {
                    dialog.handle_text_input(ch);
                }
                return Ok(());
            }
        }

        // 转发给输入框
        if let Some(ref mut input_box) = self.input_box {
            if input_box.visible {
                input_box.on_text_input(&character);
            }
        }

        Ok(())
    }
}
