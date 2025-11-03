use crate::ecs::components::CharacterList;
use crate::ecs::scenes::select_scene::BottomButton;
use crate::ecs::GameContext; // ✅ 使用 GameContext 代替 WorldExt
use ggez::winit::event::MouseButton;
use ggez::winit::keyboard::KeyCode;
use ggez::{Context, GameResult};
use hecs::World;

use super::SceneType;
use super::SelectScene;

impl SelectScene {
    /// 基于 InputContext 的输入事件处理
    ///
    /// 使用 GameContext 提供的事件迭代器
    pub(crate) fn handle_input_event(&mut self, game_ctx: &mut GameContext) -> GameResult {
        let mouse_moves: Vec<_> = game_ctx.input().mouse_motion().collect();
        let mouse_downs: Vec<_> = if let Some((btn, x, y)) = game_ctx.input().mouse_button_pressed(MouseButton::Left) {
            vec![(btn, x, y)]
        } else {
            vec![]
        };
        let key_downs: Vec<_> = game_ctx
            .input()
            .pressed_keys()
            .map(|(k, t)| (k, t.map(|s| s.to_string())))
            .collect();
        let text_inputs: Vec<_> = game_ctx.input().text_input().collect();

        // 1️⃣ 处理鼠标移动事件
        for (x, y, _dx, _dy) in mouse_moves {
            self.on_mouse_move(game_ctx.ctx, game_ctx.world, x, y)?;
        }

        // 2️⃣ 处理鼠标按下事件
        for (button, x, y) in mouse_downs {
            self.on_mouse_down(game_ctx.ctx, game_ctx.world, &button, x, y)?;
        }

        // 3️⃣ 处理键盘按下事件
        for (keycode, text) in key_downs {
            self.on_key_down(game_ctx.ctx, game_ctx.world, &keycode, text.as_deref())?;
        }

        // 4️⃣ 处理文本输入事件
        for character in text_inputs {
            self.on_text_input(game_ctx.ctx, game_ctx.world, character.to_string())?;
        }

        Ok(())
    }

    fn on_mouse_down(
        &mut self,
        ctx: &mut Context,
        world: &mut World,
        button: &ggez::winit::event::MouseButton,
        x: f32,
        y: f32,
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
        if *button != MouseButton::Left {
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
                    self.handle_new_character_button(button, world);
                    return Ok(()); // 对话框消费了点击事件
                }
            }
        }

        if let Some(_dialog) = &mut self.delete_character_dialog {
            // TODO: 实现 DeleteCharacterDialog 的点击处理
        }

        // 2. 处理角色槽位点击 (右侧垂直布局) - 使用设计坐标
        if let Some(slot_index) = self
            .character_select_ui
            .check_slot_click(design_x, design_y)
        {
            self.select_character(slot_index as i32, world);
            if let Some(character) = self.character_select_ui.get_selected_character() {
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
                1 => self.handle_button_click(BottomButton::StartGame, world),
                2 => self.handle_button_click(BottomButton::NewCharacter, world),
                3 => self.handle_button_click(BottomButton::DeleteCharacter, world),
                4 => self.handle_button_click(BottomButton::Credits, world),
                5 => self.handle_button_click(BottomButton::ExitGame, world),
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
        world: &mut World,
        key: &KeyCode,
        text: Option<&str>,
    ) -> GameResult<Option<SceneType>> {
        // 🆕 优先处理新建角色对话框的文本输入
        if let Some(ref mut dialog) = self.new_character_dialog {
            if dialog.visible && dialog.input_focused {
                match key {
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
                input_box.on_key_down(key, ctx); // ✅ 传入 ctx
                                                 // 检查是否已确认或取消
                if input_box.confirmed || input_box.cancelled {
                    // 在update中处理结果
                }
                return Ok(None);
            }
        }

        match key {
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
                // Enter 键开始游戏 - 从World读取选中状态
                let selected_index =
                    if let Some((_, char_list)) = world.query::<&CharacterList>().iter().next() {
                        char_list.selected_index
                    } else {
                        -1
                    };

                if selected_index >= 0 {
                    self.handle_button_click(BottomButton::StartGame, world);
                }
            }
            KeyCode::Digit1 | KeyCode::Numpad1 => {
                if !self.character_select_ui.get_characters().is_empty() {
                    self.select_character(0, world);
                    if let Some(character) = self.character_select_ui.get_selected_character() {
                        tracing::info!("⌨️ 键盘选中角色1: {}", character.name);
                    }
                }
            }
            KeyCode::Digit2 | KeyCode::Numpad2 => {
                if self.character_select_ui.get_characters().len() > 1 {
                    self.select_character(1, world);
                    if let Some(character) = self.character_select_ui.get_selected_character() {
                        tracing::info!("⌨️ 键盘选中角色2: {}", character.name);
                    }
                }
            }
            KeyCode::Digit3 | KeyCode::Numpad3 => {
                if self.character_select_ui.get_characters().len() > 2 {
                    self.select_character(2, world);
                    if let Some(character) = self.character_select_ui.get_selected_character() {
                        tracing::info!("⌨️ 键盘选中角色3: {}", character.name);
                    }
                }
            }
            KeyCode::Digit4 | KeyCode::Numpad4 => {
                if self.character_select_ui.get_characters().len() > 3 {
                    self.select_character(3, world);
                    if let Some(character) = self.character_select_ui.get_selected_character() {
                        tracing::info!("⌨️ 键盘选中角色4: {}", character.name);
                    }
                }
            }
            _ => {}
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
