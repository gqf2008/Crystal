// NewCharacterDialog - Character creation dialog
// Mirrors Client/MirScenes/Dialogs/NewCharacterDialog.cs

use mir2_shared::enums::{MirClass, MirGender};

/// Button identifier for NewCharacterDialog
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogButton {
    Warrior,
    Wizard,
    Taoist,
    Assassin,
    Archer,
    Male,
    Female,
    OK,
    Cancel,
}

/// Character creation dialog
#[derive(Debug, Clone)]
pub struct NewCharacterDialog {
    /// 是否显示对话框
    pub visible: bool,

    /// 角色名称
    pub name: String,

    /// 选择的职业
    pub selected_class: MirClass,

    /// 选择的性别
    pub selected_gender: MirGender,

    /// 错误消息
    pub error_message: Option<String>,

    /// 是否正在创建（等待服务器响应）
    pub creating: bool,
    
    /// 对话框位置
    pub x: f32,
    pub y: f32,
    
    /// 当前鼠标悬停的按钮
    pub hovered_button: Option<DialogButton>,
    
    /// 当前按下的按钮
    pub pressed_button: Option<DialogButton>,
    
    /// 输入框是否获得焦点
    pub input_focused: bool,
    
    /// 角色预览动画帧
    pub animation_frame: usize,
    pub animation_timer: f32,
    
    /// 输入框光标位置
    pub cursor_position: usize,
    
    /// 输入框光标闪烁计时器
    pub cursor_blink_timer: f32,
    pub cursor_visible: bool,
    
    /// IME 拼音编辑中的文本
    pub ime_preedit: String,
}

impl Default for NewCharacterDialog {
    fn default() -> Self {
        // 对话框居中显示 (1024x768 屏幕)
        let dialog_width = 656.0;  // Prguse_73 的宽度
        let dialog_height = 537.0; // Prguse_73 的高度
        let x = (1024.0 - dialog_width) / 2.0;
        let y = (768.0 - dialog_height) / 2.0;
        
        Self {
            visible: false,
            name: String::new(),
            selected_class: MirClass::Warrior,
            selected_gender: MirGender::Male,
            error_message: None,
            creating: false,
            x,
            y,
            hovered_button: None,
            pressed_button: None,
            input_focused: false,
            animation_frame: 0,
            animation_timer: 0.0,
            cursor_position: 0,
            cursor_blink_timer: 0.0,
            cursor_visible: true,
            ime_preedit: String::new(),
        }
    }
}

impl NewCharacterDialog {
    /// Create new dialog
    pub fn new() -> Self {
        Self::default()
    }

    /// 显示对话框
    pub fn show(&mut self) {
        self.visible = true;
        self.name.clear();
        self.selected_class = MirClass::Warrior;
        self.selected_gender = MirGender::Male;
        self.error_message = None;
        self.creating = false;
        self.input_focused = true;  // 自动聚焦输入框
        self.animation_frame = 0;
        self.animation_timer = 0.0;
        self.cursor_position = 0;
        self.cursor_visible = true;
        self.cursor_blink_timer = 0.0;
        self.hovered_button = None;
        self.pressed_button = None;
        self.ime_preedit.clear();
    }

    /// 隐藏对话框
    pub fn hide(&mut self) {
        self.visible = false;
        self.input_focused = false;
    }
    
    /// 更新动画和计时器
    pub fn update(&mut self, delta_time: f32) {
        if !self.visible {
            return;
        }
        
        // 更新角色预览动画 (16帧, 每帧250ms)
        self.animation_timer += delta_time;
        if self.animation_timer >= 0.25 {
            self.animation_timer = 0.0;
            self.animation_frame = (self.animation_frame + 1) % 16;
        }
        
        // 更新光标闪烁 (每0.5秒切换)
        if self.input_focused {
            self.cursor_blink_timer += delta_time;
            if self.cursor_blink_timer >= 0.5 {
                self.cursor_blink_timer = 0.0;
                self.cursor_visible = !self.cursor_visible;
            }
        }
    }
    
    /// 获取按钮矩形区域
    pub fn get_button_rect(&self, button: DialogButton) -> (f32, f32, f32, f32) {
        let base_x = self.x;
        let base_y = self.y;
        
        match button {
            DialogButton::Warrior => (base_x + 323.0, base_y + 296.0, 40.0, 40.0),
            DialogButton::Wizard => (base_x + 373.0, base_y + 296.0, 40.0, 40.0),
            DialogButton::Taoist => (base_x + 423.0, base_y + 296.0, 40.0, 40.0),
            DialogButton::Assassin => (base_x + 473.0, base_y + 296.0, 40.0, 40.0),
            DialogButton::Archer => (base_x + 523.0, base_y + 296.0, 40.0, 40.0),
            DialogButton::Male => (base_x + 323.0, base_y + 343.0, 40.0, 40.0),
            DialogButton::Female => (base_x + 373.0, base_y + 343.0, 40.0, 40.0),
            DialogButton::OK => (base_x + 160.0, base_y + 425.0, 70.0, 40.0),
            DialogButton::Cancel => (base_x + 425.0, base_y + 425.0, 70.0, 40.0),
        }
    }
    
    /// 检查鼠标是否在按钮上
    pub fn is_mouse_over_button(&self, button: DialogButton, mouse_x: i32, mouse_y: i32) -> bool {
        let (x, y, w, h) = self.get_button_rect(button);
        let mx = mouse_x as f32;
        let my = mouse_y as f32;
        mx >= x && mx <= x + w && my >= y && my <= y + h
    }
    
    /// 检查鼠标是否在输入框上
    pub fn is_mouse_over_input(&self, mouse_x: i32, mouse_y: i32) -> bool {
        let input_x = self.x + 325.0;
        let input_y = self.y + 268.0;
        let input_w = 240.0;
        let input_h = 20.0;
        let mx = mouse_x as f32;
        let my = mouse_y as f32;
        mx >= input_x && mx <= input_x + input_w && my >= input_y && my <= input_y + input_h
    }
    
    /// 处理鼠标移动
    pub fn handle_mouse_move(&mut self, mouse_x: i32, mouse_y: i32) {
        if !self.visible {
            return;
        }
        
        // 检查所有按钮
        let buttons = [
            DialogButton::Warrior,
            DialogButton::Wizard,
            DialogButton::Taoist,
            DialogButton::Assassin,
            DialogButton::Archer,
            DialogButton::Male,
            DialogButton::Female,
            DialogButton::OK,
            DialogButton::Cancel,
        ];
        
        self.hovered_button = None;
        for button in &buttons {
            if self.is_mouse_over_button(*button, mouse_x, mouse_y) {
                self.hovered_button = Some(*button);
                break;
            }
        }
    }
    
    /// 处理鼠标按下
    pub fn handle_mouse_down(&mut self, mouse_x: i32, mouse_y: i32) -> Option<DialogButton> {
        if !self.visible {
            return None;
        }
        
        // 检查输入框
        if self.is_mouse_over_input(mouse_x, mouse_y) {
            self.input_focused = true;
            self.cursor_visible = true;
            self.cursor_blink_timer = 0.0;
            return None;
        } else {
            self.input_focused = false;
        }
        
        // 检查按钮
        if let Some(button) = self.hovered_button {
            self.pressed_button = Some(button);
            return Some(button);
        }
        
        None
    }
    
    /// 处理鼠标释放
    pub fn handle_mouse_up(&mut self) {
        self.pressed_button = None;
    }
    
    /// 处理文本输入
    pub fn handle_text_input(&mut self, ch: char) {
        if !self.visible || !self.input_focused {
            tracing::debug!("❌ 文本输入被忽略: visible={}, input_focused={}", self.visible, self.input_focused);
            return;
        }
        
        // 限制字符数量 (不是字节数)
        if self.name.chars().count() >= 16 {
            tracing::debug!("❌ 文本输入被忽略: 已达到最大长度16");
            return;
        }
        
        // 只允许字母、数字、中文、下划线
        // 注意：is_ascii_alphanumeric() 只检查ASCII字母和数字
        let is_valid = ch.is_ascii_alphabetic()  // a-z, A-Z
            || ch.is_ascii_digit()                // 0-9
            || ch == '_'                           // 下划线
            || ('\u{4e00}'..='\u{9fa5}').contains(&ch);  // 中文
        
        if is_valid {
            // 将字符索引转换为字节索引
            let byte_pos = self.name.chars().take(self.cursor_position).map(|c| c.len_utf8()).sum();
            self.name.insert(byte_pos, ch);
            self.cursor_position += 1;
            
            tracing::debug!("✅ 接受字符: '{}' (U+{:04X}), 当前名称: '{}'", ch, ch as u32, self.name);
            
            // 重置光标闪烁
            self.cursor_visible = true;
            self.cursor_blink_timer = 0.0;
            
            // 清除错误消息
            self.error_message = None;
        } else {
            tracing::debug!("❌ 拒绝无效字符: '{}' (U+{:04X})", ch, ch as u32);
        }
    }
    
    /// 处理退格键
    pub fn handle_backspace(&mut self) {
        if !self.visible || !self.input_focused || self.cursor_position == 0 {
            return;
        }
        
        // 将字符索引转换为字节索引
        let byte_pos = self.name.chars().take(self.cursor_position - 1).map(|c| c.len_utf8()).sum();
        self.name.remove(byte_pos);
        self.cursor_position -= 1;
        
        // 重置光标闪烁
        self.cursor_visible = true;
        self.cursor_blink_timer = 0.0;
        
        // 清除错误消息
        self.error_message = None;
    }
    
    /// 处理删除键
    pub fn handle_delete(&mut self) {
        let char_count = self.name.chars().count();
        if !self.visible || !self.input_focused || self.cursor_position >= char_count {
            return;
        }
        
        // 将字符索引转换为字节索引
        let byte_pos = self.name.chars().take(self.cursor_position).map(|c| c.len_utf8()).sum();
        self.name.remove(byte_pos);
        
        // 重置光标闪烁
        self.cursor_visible = true;
        self.cursor_blink_timer = 0.0;
    }
    
    /// 处理左箭头
    pub fn handle_left_arrow(&mut self) {
        if !self.visible || !self.input_focused || self.cursor_position == 0 {
            return;
        }
        
        self.cursor_position -= 1;
        self.cursor_visible = true;
        self.cursor_blink_timer = 0.0;
    }
    
    /// 处理右箭头
    pub fn handle_right_arrow(&mut self) {
        let char_count = self.name.chars().count();
        if !self.visible || !self.input_focused || self.cursor_position >= char_count {
            return;
        }
        
        self.cursor_position += 1;
        self.cursor_visible = true;
        self.cursor_blink_timer = 0.0;
    }
    
    /// 获取当前角色预览动画索引
    pub fn get_animation_index(&self) -> i32 {
        let base = match (self.selected_class, self.selected_gender) {
            (MirClass::Warrior, MirGender::Male) => 20,
            (MirClass::Warrior, MirGender::Female) => 300,
            (MirClass::Wizard, MirGender::Male) => 40,
            (MirClass::Wizard, MirGender::Female) => 320,
            (MirClass::Taoist, MirGender::Male) => 60,
            (MirClass::Taoist, MirGender::Female) => 340,
            (MirClass::Assassin, MirGender::Male) => 80,
            (MirClass::Assassin, MirGender::Female) => 360,
            (MirClass::Archer, MirGender::Male) => 100,
            (MirClass::Archer, MirGender::Female) => 140,
        };
        base + self.animation_frame as i32
    }

    /// 验证角色名称
    pub fn validate_name(&self) -> Result<(), String> {
        let name = self.name.trim();

        if name.is_empty() {
            return Err("角色名称不能为空".to_string());
        }

        if name.len() < 2 {
            return Err("角色名称至少需要2个字符".to_string());
        }

        if name.len() > 16 {
            return Err("角色名称最多16个字符".to_string());
        }

        // 检查字符是否合法（字母、数字、中文）
        let valid_chars = name.chars().all(|c| {
            c.is_ascii_alphanumeric() ||
            ('\u{4e00}'..='\u{9fa5}').contains(&c) // 中文字符
        });

        if !valid_chars {
            return Err("角色名称只能包含字母、数字和中文".to_string());
        }

        Ok(())
    }

    /// 获取职业描述
    pub fn get_class_description(&self) -> &'static str {
        match self.selected_class {
            MirClass::Warrior => WARRIOR_DESCRIPTION,
            MirClass::Wizard => WIZARD_DESCRIPTION,
            MirClass::Taoist => TAOIST_DESCRIPTION,
            MirClass::Assassin => ASSASSIN_DESCRIPTION,
            MirClass::Archer => ARCHER_DESCRIPTION,
        }
    }

    /// 获取职业图标emoji
    pub fn get_class_icon(&self) -> &'static str {
        match self.selected_class {
            MirClass::Warrior => "⚔️",
            MirClass::Wizard => "🔮",
            MirClass::Taoist => "☯️",
            MirClass::Assassin => "🗡️",
            MirClass::Archer => "🏹",
        }
    }

    /// 获取性别图标emoji
    pub fn get_gender_icon(&self) -> &'static str {
        match self.selected_gender {
            MirGender::Male => "♂️",
            MirGender::Female => "♀️",
        }
    }
    
    /// 绘制对话框
    pub fn draw(&mut self, ctx: &mut ggez::graphics::GraphicsContext, canvas: &mut ggez::graphics::Canvas) -> ggez::GameResult {
        use ggez::graphics::{Color, Text, PxScale, DrawParam};
        use crate::graphics::{LibraryName, draw_sprite_at};
        
        if !self.visible {
            return Ok(());
        }
        
        // 1. 绘制对话框背景 (Prguse_73) - 不使用偏移
        draw_sprite_at(ctx, canvas, &LibraryName::Prguse, 73, self.x, self.y)
            .map_err(|e| ggez::GameError::CustomError(format!("{:?}", e)))?;
        
        // 2. 绘制标题 (Title_20) - 不使用偏移
        let _ = draw_sprite_at(ctx, canvas, &LibraryName::Title, 20, self.x + 206.0, self.y + 11.0);
        
        // 3. 绘制角色预览动画 (ChrSel库) - 使用库系统绘制（与 CharacterSelect 保持一致）
        let character_index = self.get_character_display_index();
        let anim_index = (character_index + self.animation_frame) as usize;
        
        // 使用库系统绘制角色主体（use_offset=true 保持清晰）
        if let Some(lib_arc) = crate::graphics::libraries::get_library(LibraryName::ChrSel) {
            if let Ok(mut lib) = lib_arc.try_lock() {
                // 绘制角色主体动画
                let _ = lib.draw_with_color(
                    ctx, 
                    canvas, 
                    anim_index,
                    self.x + 120.0,
                    self.y + 250.0,
                    Color::WHITE,
                    true  // use_offset=true
                );
                
                // 如果是法师，叠加绘制光效（半透明白色，alpha=180）
                if self.selected_class == MirClass::Wizard {
                    let blend_index = anim_index + 560;
                    let _ = lib.draw_with_color(
                        ctx, 
                        canvas, 
                        blend_index,
                        self.x + 120.0,
                        self.y + 250.0,
                        Color::from_rgba(255, 255, 255, 180),  // 半透明白色
                        true  // use_offset=true
                    );
                }
            }
        }
        
        // 5. 绘制职业按钮 (Prguse库) - 根据状态显示不同索引
        // 战士按钮 (323, 296)
        let warrior_index = if self.selected_class == MirClass::Warrior { 
            2427 
        } else if self.hovered_button == Some(DialogButton::Warrior) {
            2427 // 悬停效果
        } else { 
            2426 
        };
        let _ = draw_sprite_at(ctx, canvas, &LibraryName::Prguse, warrior_index, self.x + 323.0, self.y + 296.0);
        
        // 法师按钮 (373, 296)
        let wizard_index = if self.selected_class == MirClass::Wizard { 
            2430 
        } else if self.hovered_button == Some(DialogButton::Wizard) {
            2430
        } else { 
            2429 
        };
        let _ = draw_sprite_at(ctx, canvas, &LibraryName::Prguse, wizard_index, self.x + 373.0, self.y + 296.0);
        
        // 道士按钮 (423, 296)
        let taoist_index = if self.selected_class == MirClass::Taoist { 
            2433 
        } else if self.hovered_button == Some(DialogButton::Taoist) {
            2433
        } else { 
            2432 
        };
        let _ = draw_sprite_at(ctx, canvas, &LibraryName::Prguse, taoist_index, self.x + 423.0, self.y + 296.0);
        
        // 刺客按钮 (473, 296)
        let assassin_index = if self.selected_class == MirClass::Assassin { 
            2436 
        } else if self.hovered_button == Some(DialogButton::Assassin) {
            2436
        } else { 
            2435 
        };
        let _ = draw_sprite_at(ctx, canvas, &LibraryName::Prguse, assassin_index, self.x + 473.0, self.y + 296.0);
        
        // 弓手按钮 (523, 296)
        let archer_index = if self.selected_class == MirClass::Archer { 
            2439 
        } else if self.hovered_button == Some(DialogButton::Archer) {
            2439
        } else { 
            2438 
        };
        let _ = draw_sprite_at(ctx, canvas, &LibraryName::Prguse, archer_index, self.x + 523.0, self.y + 296.0);
        
        // 6. 绘制性别按钮 (Prguse库) - 根据状态显示不同索引
        // 男性按钮 (323, 343)
        let male_index = if self.selected_gender == MirGender::Male { 
            2421 
        } else if self.hovered_button == Some(DialogButton::Male) {
            2421
        } else { 
            2420 
        };
        let _ = draw_sprite_at(ctx, canvas, &LibraryName::Prguse, male_index, self.x + 323.0, self.y + 343.0);
        
        // 女性按钮 (373, 343)
        let female_index = if self.selected_gender == MirGender::Female { 
            2424 
        } else if self.hovered_button == Some(DialogButton::Female) {
            2424
        } else { 
            2423 
        };
        let _ = draw_sprite_at(ctx, canvas, &LibraryName::Prguse, female_index, self.x + 373.0, self.y + 343.0);
        
        // 7. 绘制 OK 按钮 (Title库, 160, 425) - 根据状态显示不同索引
        let ok_enabled = !self.name.is_empty() && self.name.chars().count() >= 2 && self.name.chars().count() <= 15;
        let ok_index = if self.hovered_button == Some(DialogButton::OK) {
            361 // HoverIndex
        } else if self.pressed_button == Some(DialogButton::OK) {
            362 // PressedIndex
        } else {
            360 // 默认
        };
        let _ = draw_sprite_at(ctx, canvas, &LibraryName::Title, ok_index, self.x + 160.0, self.y + 425.0);
        
        // 8. 绘制 Cancel 按钮 (Title库, 425, 425) - 根据状态显示不同索引
        let cancel_index = if self.hovered_button == Some(DialogButton::Cancel) {
            281 // HoverIndex
        } else if self.pressed_button == Some(DialogButton::Cancel) {
            282 // PressedIndex
        } else {
            280 // 默认
        };
        let _ = draw_sprite_at(ctx, canvas, &LibraryName::Title, cancel_index, self.x + 425.0, self.y + 425.0);
        
        // 9. 绘制职业描述文本 - 动态计算最大宽度
        let description = self.get_class_description();
        
        // 对话框参数:
        // - 对话框宽度: 656像素 (Prguse_73)
        // - 文本起始X: 279像素 (相对于对话框左边缘)
        // - 右侧边距: 约30像素 (留出边距)
        // - 计算最大宽度: 656 - 279 - 30 = 347像素
        let text_start_x = 279.0;
        let right_margin = 30.0;
        let max_width = 656.0 - text_start_x - right_margin; // 约347像素
        let text_start_y = 70.0;
        
        let char_width = 12.0; // 中文字符平均宽度
        
        let mut lines = Vec::new();
        let mut current_line = String::new();
        let mut current_width = 0.0;
        
        for ch in description.chars() {
            let ch_width = if ch.is_ascii() { 6.0 } else { 12.0 }; // 英文字符约6像素
            
            if current_width + ch_width > max_width {
                // 当前行已满，开始新行
                if !current_line.is_empty() {
                    lines.push(current_line.clone());
                    current_line.clear();
                    current_width = 0.0;
                }
            }
            
            current_line.push(ch);
            current_width += ch_width;
        }
        
        // 添加最后一行
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        
        // 绘制每一行
        let line_height = 16.0;
        for (i, line) in lines.iter().enumerate() {
            let mut text = Text::new(line);
            text.set_scale(PxScale::from(12.0));
            text.set_font("AlibabaPuHuiTi"); // 使用中文字体
            canvas.draw(
                &text,
                DrawParam::default()
                    .dest([self.x + text_start_x, self.y + text_start_y + (i as f32 * line_height)])
                    .color(Color::WHITE),
            );
        }
        
        // 10. 绘制输入框文本 (325, 268)
        if !self.name.is_empty() {
            let mut text = Text::new(&self.name);
            text.set_scale(PxScale::from(14.0));
            text.set_font("AlibabaPuHuiTi"); // 使用中文字体
            canvas.draw(
                &text,
                DrawParam::default()
                    .dest([self.x + 325.0, self.y + 268.0])
                    .color(Color::WHITE),
            );
        }
        
        // 11. 绘制输入框光标 (精确测量位置)
        if self.input_focused && self.cursor_visible {
            // 精确测量光标位置：使用ggez的文本测量API
            let text_before_cursor: String = self.name.chars().take(self.cursor_position).collect();
            
            let text_width = if !text_before_cursor.is_empty() {
                let mut temp_text = Text::new(&text_before_cursor);
                temp_text.set_scale(PxScale::from(14.0));
                temp_text.set_font("AlibabaPuHuiTi"); // 使用中文字体
                temp_text.measure(ctx).map(|r| r.x).unwrap_or(0.0)
            } else {
                0.0
            };
            
            let cursor_x = self.x + 325.0 + text_width;
            let cursor_y = self.y + 268.0;
            
            // 绘制光标竖线
            let cursor_rect = ggez::graphics::Rect::new(cursor_x, cursor_y, 2.0, 16.0);
            let cursor_mesh = ggez::graphics::Mesh::new_rectangle(
                ctx,
                ggez::graphics::DrawMode::fill(),
                cursor_rect,
                Color::WHITE,
            )?;
            canvas.draw(&cursor_mesh, DrawParam::default());
        }
        
        // 12. 绘制错误消息 (已移至 MessageBox，这里不再显示内联错误)
        // 错误消息现在通过 MessageBox 显示，更符合原版UI体验
        
        Ok(())
    }
    
    /// 获取角色显示索引（基于职业和性别）
    fn get_character_display_index(&self) -> usize {
        match self.selected_class {
            MirClass::Warrior => if self.selected_gender == MirGender::Male { 20 } else { 300 },
            MirClass::Wizard => if self.selected_gender == MirGender::Male { 40 } else { 320 },
            MirClass::Taoist => if self.selected_gender == MirGender::Male { 60 } else { 340 },
            MirClass::Assassin => if self.selected_gender == MirGender::Male { 80 } else { 360 },
            MirClass::Archer => if self.selected_gender == MirGender::Male { 100 } else { 140 },
        }
    }
}

// 职业描述文本
const WARRIOR_DESCRIPTION: &str =
    "战士是力量和体力的化身。他们不容易在战斗中被杀死，并且能够使用各种重型武器和盔甲。\
    战士偏好基于近战物理伤害的攻击。他们的远程攻击较弱，但是专为战士开发的各种装备弥补了他们在远程战斗中的弱点。";

const WIZARD_DESCRIPTION: &str =
    "法师是力量和耐力较低的职业，但拥有使用强大法术的能力。他们的攻击性法术非常有效，\
    但由于施放这些法术需要时间，因此很容易让自己暴露在敌人的攻击之下。因此,身体虚弱的法师必须在安全距离攻击敌人。";

const TAOIST_DESCRIPTION: &str =
    "道士除了武功外，还精通天文学、医学等学科。他们的专长不在于直接与敌人交战，\
    而在于用辅助技能协助盟友。道士可以召唤强大的生物，对魔法有很高的抵抗力，是攻守兼备的职业。";

const ASSASSIN_DESCRIPTION: &str =
    "刺客是秘密组织的成员，他们的历史相对不为人知。他们能够隐藏自己，在别人看不见的情况下进行攻击，\
    这自然使他们擅长快速击杀。由于体力和力量较弱，他们需要避免与多个敌人作战。";

const ARCHER_DESCRIPTION: &str =
    "弓箭手是精准和力量兼备的职业，使用弓箭的强大技能从远处造成非凡的伤害。\
    就像法师一样，他们依靠敏锐的直觉来躲避迎面而来的攻击，因为他们往往会让自己暴露在正面攻击之下。\
    然而，他们的身体素质和致命的准确性使他们能够让任何被击中的人感到恐惧。";