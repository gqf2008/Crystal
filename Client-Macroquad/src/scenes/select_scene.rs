// ============================================================================
// 角色选择场景 - 纯 Native 版本 (无 egui)
// ============================================================================
// 
// 【渲染架构说明】
// 本场景采用纯 macroquad 原生渲染，无 egui 依赖
//
// ============================================================================

use crate::game::GameResult;
use crate::resources::LibraryName;
use crate::scenes::{Scene, SceneTransition};
use crate::ui::text_renderer::{draw_text_cn, measure_text_cn};
use macroquad::prelude::*;

/// 角色信息
#[derive(Debug, Clone)]
pub struct CharacterInfo {
    pub index: i32,
    pub name: String,
    pub level: u16,
    pub class: u8,  // 0=Warrior, 1=Wizard, 2=Taoist
    pub gender: u8, // 0=Male, 1=Female
    pub last_access: String,
}

/// 角色选择场景
pub struct SelectScene {
    // 角色数据
    characters: Vec<CharacterInfo>,
    selected_index: Option<usize>,
    
    // 对话框状态
    show_new_character: bool,
    show_delete_character: bool,
    show_message_box: bool,
    message_text: String,
    
    // 新建角色表单
    new_char_name: String,
    new_char_class: u8,
    new_char_gender: u8,
    
    // 删除角色确认
    delete_char_name: String,
    delete_char_index: i32,
    delete_confirm_input: String,
    
    // 角色预览动画
    animation_frame: usize,
    animation_timer: f32,
    animation_delay: f32,
    
    // 光标闪烁
    cursor_visible: bool,
    cursor_timer: f32,
}

impl SelectScene {
    pub fn new(characters: Vec<CharacterInfo>) -> GameResult<Self> {
        let selected_index = if !characters.is_empty() { Some(0) } else { None };
        
        Ok(Self {
            characters,
            selected_index,
            
            show_new_character: false,
            show_delete_character: false,
            show_message_box: false,
            message_text: String::new(),
            
            new_char_name: String::new(),
            new_char_class: 0,
            new_char_gender: 0,
            
            delete_char_name: String::new(),
            delete_char_index: -1,
            delete_confirm_input: String::new(),
            
            animation_frame: 0,
            animation_timer: 0.0,
            animation_delay: 0.25,
            
            cursor_visible: true,
            cursor_timer: 0.0,
        })
    }
    
    /// 显示消息框
    fn show_message(&mut self, message: &str) {
        self.message_text = message.to_string();
        self.show_message_box = true;
    }
    
    /// 绘制背景
    fn draw_background(&self) {
        // 背景 Prguse[65]
        if let Some(info) = LibraryName::Prguse.get_texture(65) {
            if let Some(ref texture) = info.image {
                draw_texture(texture, info.offset_x as f32, info.offset_y as f32, WHITE);
            }
        }
        
        // 标题 Title[40] at (468, 20)
        if let Some(info) = LibraryName::Title.get_texture(40) {
            if let Some(ref texture) = info.image {
                draw_texture(texture, 468.0 + info.offset_x as f32, 20.0 + info.offset_y as f32, WHITE);
            }
        }
        
        // 服务器名称 at (432, 60)
        draw_text_cn("Legend of Mir 2", 460.0, 77.0, 17.0, WHITE);
    }
    
    /// 绘制角色预览动画
    fn draw_character_preview(&self) {
        if let Some(selected_idx) = self.selected_index {
            if selected_idx < self.characters.len() {
                let character = &self.characters[selected_idx];
                
                // 计算角色预览帧索引
                let base_index = if character.class == 4 {
                    if character.gender == 0 { 100 } else { 140 }
                } else {
                    20 + (character.class as usize * 20) + (character.gender as usize * 280)
                };
                let frame_index = base_index + self.animation_frame;
                
                if let Some(info) = LibraryName::ChrSel.get_texture(frame_index) {
                    if let Some(ref texture) = info.image {
                        let scale = 1.2;
                        let x = 260.0 + info.offset_x as f32 * scale;
                        let y = 420.0 + info.offset_y as f32 * scale;
                        let w = texture.width() * scale;
                        let h = texture.height() * scale;
                        draw_texture_ex(texture, x, y, WHITE, DrawTextureParams {
                            dest_size: Some(Vec2::new(w, h)),
                            ..Default::default()
                        });
                    }
                }
            }
        }
    }
    
    /// 绘制角色信息
    fn draw_character_info(&self) {
        if let Some(selected_idx) = self.selected_index {
            if selected_idx < self.characters.len() {
                let character = &self.characters[selected_idx];
                draw_text_cn("Last Online:", 200.0, 623.0, 14.0, WHITE);
                draw_text_cn(&character.last_access, 280.0, 623.0, 14.0, WHITE);
            }
        }
    }
    
    /// 绘制角色按钮
    fn draw_character_buttons(&mut self) {
        let positions = [(637.0, 194.0), (637.0, 298.0), (637.0, 402.0), (637.0, 506.0)];
        let (mx, my) = mouse_position();
        
        for (i, &(x, y)) in positions.iter().enumerate() {
            let has_character = i < self.characters.len();
            let is_selected = self.selected_index == Some(i);
            
            // 按钮尺寸
            let btn_w = 280.0;
            let btn_h = 90.0;
            let is_hovered = mx >= x && mx <= x + btn_w && my >= y && my <= y + btn_h;
            
            if has_character {
                let character = &self.characters[i];
                
                // 获取职业对应纹理索引
                let base_index = match character.class {
                    0 => 660, 1 => 661, 2 => 662, 3 => 663, 4 => 664, _ => 660,
                };
                let texture_index = if is_selected { base_index + 5 } else { base_index };
                
                if let Some(info) = LibraryName::Title.get_texture(texture_index) {
                    if let Some(ref texture) = info.image {
                        draw_texture(texture, x, y, WHITE);
                    }
                }
                
                // 绘制角色信息文字
                draw_text_cn(&character.name, x + 107.0, y + 18.0, 13.0, WHITE);
                draw_text_cn(&format!("Lv.{}", character.level), x + 107.0, y + 37.0, 11.0, LIGHTGRAY);
                
                let class_name = match character.class {
                    0 => "战士", 1 => "法师", 2 => "道士", 3 => "刺客", 4 => "弓手", _ => "未知",
                };
                draw_text_cn(class_name, x + 178.0, y + 37.0, 11.0, LIGHTGRAY);
                
                // 检测点击
                if !self.show_new_character && !self.show_delete_character && !self.show_message_box {
                    if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
                        self.selected_index = Some(i);
                    }
                }
            } else {
                // 空槽位 - Prguse[44]
                if let Some(info) = LibraryName::Prguse.get_texture(44) {
                    if let Some(ref texture) = info.image {
                        draw_texture(texture, x, y, WHITE);
                    }
                }
                
                // 检测点击空槽位
                if !self.show_new_character && !self.show_delete_character && !self.show_message_box {
                    if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
                        if self.characters.len() < 4 {
                            self.show_new_character = true;
                        }
                    }
                }
            }
        }
    }
    
    /// 绘制底部按钮
    fn draw_bottom_buttons(&mut self) {
        let screen_w = screen_width();
        let x_point = (screen_w - 200.0) / 5.0;
        let y = screen_height() - 32.0;
        
        // 开始游戏 Title[340-342]
        if self.selected_index.is_some() {
            if self.draw_button(100.0 + x_point - x_point / 2.0 - 50.0, y, 340, 341, 342) {
                println!("🎮 开始游戏");
            }
        }
        
        // 新建角色 Title[343-345]
        if self.draw_button(100.0 + x_point * 2.0 - x_point / 2.0 - 50.0, y, 343, 344, 345) {
            if self.characters.len() < 4 {
                self.show_new_character = true;
                self.new_char_name.clear();
            } else {
                self.show_message("最多只能创建4个角色！");
            }
        }
        
        // 删除角色 Title[346-348]
        if self.draw_button(100.0 + x_point * 3.0 - x_point / 2.0 - 50.0, y, 346, 347, 348) {
            if let Some(idx) = self.selected_index {
                if idx < self.characters.len() {
                    let character = &self.characters[idx];
                    self.delete_char_name = character.name.clone();
                    self.delete_char_index = character.index;
                    self.delete_confirm_input.clear();
                    self.show_delete_character = true;
                }
            }
        }
        
        // 退出 Title[352-354]
        if self.draw_button(100.0 + x_point * 5.0 - x_point / 2.0 - 50.0, y, 352, 353, 354) {
            std::process::exit(0);
        }
    }
    
    /// 绘制按钮
    fn draw_button(&self, x: f32, y: f32, normal_idx: usize, hover_idx: usize, pressed_idx: usize) -> bool {
        let (mx, my) = mouse_position();
        
        let btn_size = if let Some(info) = LibraryName::Title.get_texture(normal_idx) {
            (info.width as f32, info.height as f32)
        } else {
            (100.0, 30.0)
        };
        
        let is_hovered = mx >= x && mx <= x + btn_size.0 && my >= y && my <= y + btn_size.1;
        let is_pressed = is_hovered && is_mouse_button_down(MouseButton::Left);
        
        // 如果有对话框显示，禁用按钮交互
        if self.show_new_character || self.show_delete_character || self.show_message_box {
            if let Some(info) = LibraryName::Title.get_texture(normal_idx) {
                if let Some(ref texture) = info.image {
                    draw_texture(texture, x, y, Color::new(0.5, 0.5, 0.5, 1.0));
                }
            }
            return false;
        }
        
        let texture_idx = if is_pressed {
            pressed_idx
        } else if is_hovered {
            hover_idx
        } else {
            normal_idx
        };
        
        if let Some(info) = LibraryName::Title.get_texture(texture_idx) {
            if let Some(ref texture) = info.image {
                draw_texture(texture, x, y, WHITE);
            }
        }
        
        is_hovered && is_mouse_button_pressed(MouseButton::Left)
    }
    
    /// 绘制新建角色对话框
    fn draw_new_character_dialog(&mut self) {
        let screen_w = screen_width();
        let screen_h = screen_height();
        
        let dialog_w = 350.0;
        let dialog_h = 280.0;
        let dialog_x = (screen_w - dialog_w) / 2.0;
        let dialog_y = (screen_h - dialog_h) / 2.0;
        
        // 背景
        draw_rectangle(dialog_x, dialog_y, dialog_w, dialog_h, Color::from_rgba(40, 40, 50, 240));
        draw_rectangle_lines(dialog_x, dialog_y, dialog_w, dialog_h, 2.0, Color::from_rgba(100, 100, 120, 255));
        
        // 标题
        draw_text_cn("创建新角色", dialog_x + dialog_w / 2.0 - 45.0, dialog_y + 30.0, 18.0, WHITE);
        
        // 名称输入框
        draw_text_cn("角色名称:", dialog_x + 30.0, dialog_y + 70.0, 14.0, WHITE);
        let input_x = dialog_x + 100.0;
        let input_y = dialog_y + 55.0;
        let input_w = 200.0;
        let input_h = 25.0;
        draw_rectangle(input_x, input_y, input_w, input_h, Color::from_rgba(30, 30, 40, 255));
        draw_rectangle_lines(input_x, input_y, input_w, input_h, 1.0, Color::from_rgba(80, 80, 100, 255));
        draw_text_cn(&self.new_char_name, input_x + 5.0, input_y + 18.0, 14.0, WHITE);
        
        // 光标
        if self.cursor_visible {
            let text_width = measure_text_cn(&self.new_char_name, 14.0).width;
            draw_line(input_x + 5.0 + text_width, input_y + 3.0, input_x + 5.0 + text_width, input_y + input_h - 3.0, 1.0, WHITE);
        }
        
        // 性别选择
        draw_text_cn("性别:", dialog_x + 30.0, dialog_y + 110.0, 14.0, WHITE);
        let gender_names = ["男", "女"];
        for (i, name) in gender_names.iter().enumerate() {
            let btn_x = dialog_x + 100.0 + (i as f32) * 80.0;
            let btn_y = dialog_y + 95.0;
            let is_selected = self.new_char_gender == i as u8;
            let color = if is_selected { Color::from_rgba(80, 120, 180, 255) } else { Color::from_rgba(60, 60, 80, 255) };
            draw_rectangle(btn_x, btn_y, 60.0, 25.0, color);
            draw_rectangle_lines(btn_x, btn_y, 60.0, 25.0, 1.0, Color::from_rgba(100, 100, 120, 255));
            draw_text_cn(name, btn_x + 22.0, btn_y + 18.0, 14.0, WHITE);
            
            let (mx, my) = mouse_position();
            if mx >= btn_x && mx <= btn_x + 60.0 && my >= btn_y && my <= btn_y + 25.0 {
                if is_mouse_button_pressed(MouseButton::Left) {
                    self.new_char_gender = i as u8;
                }
            }
        }
        
        // 职业选择
        draw_text_cn("职业:", dialog_x + 30.0, dialog_y + 150.0, 14.0, WHITE);
        let class_names = ["战士", "法师", "道士", "刺客", "弓手"];
        for (i, name) in class_names.iter().enumerate() {
            let btn_x = dialog_x + 30.0 + (i as f32) * 60.0;
            let btn_y = dialog_y + 160.0;
            let is_selected = self.new_char_class == i as u8;
            let color = if is_selected { Color::from_rgba(80, 120, 180, 255) } else { Color::from_rgba(60, 60, 80, 255) };
            draw_rectangle(btn_x, btn_y, 55.0, 25.0, color);
            draw_rectangle_lines(btn_x, btn_y, 55.0, 25.0, 1.0, Color::from_rgba(100, 100, 120, 255));
            draw_text_cn(name, btn_x + 12.0, btn_y + 18.0, 14.0, WHITE);
            
            let (mx, my) = mouse_position();
            if mx >= btn_x && mx <= btn_x + 55.0 && my >= btn_y && my <= btn_y + 25.0 {
                if is_mouse_button_pressed(MouseButton::Left) {
                    self.new_char_class = i as u8;
                }
            }
        }
        
        // 确认/取消按钮
        let btn_y = dialog_y + dialog_h - 45.0;
        
        // 确认按钮
        let confirm_x = dialog_x + dialog_w / 2.0 - 100.0;
        let (mx, my) = mouse_position();
        let confirm_hovered = mx >= confirm_x && mx <= confirm_x + 80.0 && my >= btn_y && my <= btn_y + 30.0;
        let confirm_color = if confirm_hovered { Color::from_rgba(80, 120, 180, 255) } else { Color::from_rgba(60, 80, 120, 255) };
        draw_rectangle(confirm_x, btn_y, 80.0, 30.0, confirm_color);
        draw_rectangle_lines(confirm_x, btn_y, 80.0, 30.0, 1.0, Color::from_rgba(100, 120, 150, 255));
        draw_text_cn("确认", confirm_x + 22.0, btn_y + 22.0, 14.0, WHITE);
        
        if confirm_hovered && is_mouse_button_pressed(MouseButton::Left) {
            self.on_create_character();
        }
        
        // 取消按钮
        let cancel_x = dialog_x + dialog_w / 2.0 + 20.0;
        let cancel_hovered = mx >= cancel_x && mx <= cancel_x + 80.0 && my >= btn_y && my <= btn_y + 30.0;
        let cancel_color = if cancel_hovered { Color::from_rgba(120, 80, 80, 255) } else { Color::from_rgba(80, 60, 60, 255) };
        draw_rectangle(cancel_x, btn_y, 80.0, 30.0, cancel_color);
        draw_rectangle_lines(cancel_x, btn_y, 80.0, 30.0, 1.0, Color::from_rgba(150, 100, 100, 255));
        draw_text_cn("取消", cancel_x + 22.0, btn_y + 22.0, 14.0, WHITE);
        
        if cancel_hovered && is_mouse_button_pressed(MouseButton::Left) {
            self.show_new_character = false;
        }
    }
    
    /// 绘制删除角色对话框
    fn draw_delete_character_dialog(&mut self) {
        let screen_w = screen_width();
        let screen_h = screen_height();
        
        let dialog_w = 300.0;
        let dialog_h = 180.0;
        let dialog_x = (screen_w - dialog_w) / 2.0;
        let dialog_y = (screen_h - dialog_h) / 2.0;
        
        // 背景
        draw_rectangle(dialog_x, dialog_y, dialog_w, dialog_h, Color::from_rgba(50, 40, 40, 240));
        draw_rectangle_lines(dialog_x, dialog_y, dialog_w, dialog_h, 2.0, Color::from_rgba(150, 100, 100, 255));
        
        // 标题
        draw_text_cn("删除角色确认", dialog_x + dialog_w / 2.0 - 50.0, dialog_y + 30.0, 16.0, Color::from_rgba(255, 150, 150, 255));
        
        // 提示
        draw_text_cn(&format!("确定要删除角色 {} 吗?", self.delete_char_name), dialog_x + 30.0, dialog_y + 60.0, 13.0, WHITE);
        draw_text_cn("请输入角色名称确认:", dialog_x + 30.0, dialog_y + 85.0, 12.0, LIGHTGRAY);
        
        // 输入框
        let input_x = dialog_x + 30.0;
        let input_y = dialog_y + 95.0;
        let input_w = dialog_w - 60.0;
        let input_h = 25.0;
        draw_rectangle(input_x, input_y, input_w, input_h, Color::from_rgba(30, 30, 40, 255));
        draw_rectangle_lines(input_x, input_y, input_w, input_h, 1.0, Color::from_rgba(100, 80, 80, 255));
        draw_text_cn(&self.delete_confirm_input, input_x + 5.0, input_y + 18.0, 14.0, WHITE);
        
        // 按钮
        let btn_y = dialog_y + dialog_h - 45.0;
        let (mx, my) = mouse_position();
        
        // 确认按钮
        let confirm_x = dialog_x + dialog_w / 2.0 - 100.0;
        let can_confirm = self.delete_confirm_input == self.delete_char_name;
        let confirm_color = if can_confirm {
            if mx >= confirm_x && mx <= confirm_x + 80.0 && my >= btn_y && my <= btn_y + 30.0 {
                Color::from_rgba(180, 80, 80, 255)
            } else {
                Color::from_rgba(120, 60, 60, 255)
            }
        } else {
            Color::from_rgba(60, 60, 60, 255)
        };
        draw_rectangle(confirm_x, btn_y, 80.0, 30.0, confirm_color);
        draw_rectangle_lines(confirm_x, btn_y, 80.0, 30.0, 1.0, Color::from_rgba(150, 100, 100, 255));
        draw_text_cn("删除", confirm_x + 22.0, btn_y + 22.0, 14.0, if can_confirm { WHITE } else { GRAY });
        
        if can_confirm && mx >= confirm_x && mx <= confirm_x + 80.0 && my >= btn_y && my <= btn_y + 30.0 {
            if is_mouse_button_pressed(MouseButton::Left) {
                self.on_delete_character();
            }
        }
        
        // 取消按钮
        let cancel_x = dialog_x + dialog_w / 2.0 + 20.0;
        let cancel_hovered = mx >= cancel_x && mx <= cancel_x + 80.0 && my >= btn_y && my <= btn_y + 30.0;
        let cancel_color = if cancel_hovered { Color::from_rgba(80, 80, 100, 255) } else { Color::from_rgba(60, 60, 80, 255) };
        draw_rectangle(cancel_x, btn_y, 80.0, 30.0, cancel_color);
        draw_rectangle_lines(cancel_x, btn_y, 80.0, 30.0, 1.0, Color::from_rgba(100, 100, 120, 255));
        draw_text_cn("取消", cancel_x + 22.0, btn_y + 22.0, 14.0, WHITE);
        
        if cancel_hovered && is_mouse_button_pressed(MouseButton::Left) {
            self.show_delete_character = false;
        }
    }
    
    /// 绘制消息框
    fn draw_message_box(&self) {
        let screen_w = screen_width();
        let screen_h = screen_height();
        
        let dialog_w = 300.0;
        let dialog_h = 120.0;
        let dialog_x = (screen_w - dialog_w) / 2.0;
        let dialog_y = (screen_h - dialog_h) / 2.0;
        
        // 背景
        draw_rectangle(dialog_x, dialog_y, dialog_w, dialog_h, Color::from_rgba(40, 40, 50, 240));
        draw_rectangle_lines(dialog_x, dialog_y, dialog_w, dialog_h, 2.0, Color::from_rgba(100, 100, 120, 255));
        
        // 消息
        let text_width = measure_text_cn(&self.message_text, 14.0).width;
        draw_text_cn(&self.message_text, dialog_x + (dialog_w - text_width) / 2.0, dialog_y + 50.0, 14.0, WHITE);
        
        // 确定按钮
        let btn_x = dialog_x + dialog_w / 2.0 - 40.0;
        let btn_y = dialog_y + dialog_h - 45.0;
        let (mx, my) = mouse_position();
        let hovered = mx >= btn_x && mx <= btn_x + 80.0 && my >= btn_y && my <= btn_y + 30.0;
        let color = if hovered { Color::from_rgba(80, 120, 180, 255) } else { Color::from_rgba(60, 80, 120, 255) };
        draw_rectangle(btn_x, btn_y, 80.0, 30.0, color);
        draw_rectangle_lines(btn_x, btn_y, 80.0, 30.0, 1.0, Color::from_rgba(100, 120, 150, 255));
        draw_text_cn("确定", btn_x + 22.0, btn_y + 22.0, 14.0, WHITE);
    }
    
    /// 处理创建角色
    fn on_create_character(&mut self) {
        if self.new_char_name.trim().is_empty() {
            self.show_message("请输入角色名称！");
            return;
        }
        
        if self.new_char_name.chars().count() < 2 {
            self.show_message("角色名称至少需2个字符！");
            return;
        }
        
        if self.new_char_name.chars().count() > 16 {
            self.show_message("角色名称最多16个字符！");
            return;
        }
        
        let new_char = CharacterInfo {
            index: self.characters.len() as i32,
            name: self.new_char_name.clone(),
            level: 1,
            class: self.new_char_class,
            gender: self.new_char_gender,
            last_access: "刚刚".to_string(),
        };
        
        self.characters.push(new_char);
        println!("🎭 创建角色成功: {}", self.new_char_name);
        
        self.new_char_name.clear();
        self.show_new_character = false;
    }
    
    /// 处理删除角色
    fn on_delete_character(&mut self) {
        if let Some(pos) = self.characters.iter().position(|c| c.index == self.delete_char_index) {
            let name = self.characters[pos].name.clone();
            self.characters.remove(pos);
            println!("✅ 角色已删除: {}", name);
            
            if self.characters.is_empty() {
                self.selected_index = None;
            } else if let Some(idx) = self.selected_index {
                if idx >= self.characters.len() {
                    self.selected_index = Some(self.characters.len() - 1);
                }
            }
        }
        
        self.show_delete_character = false;
    }
    
    /// 处理文本输入
    fn handle_text_input(&mut self) {
        while let Some(ch) = get_char_pressed() {
            if ch.is_control() {
                continue;
            }
            
            if self.show_new_character {
                if self.new_char_name.chars().count() < 16 {
                    self.new_char_name.push(ch);
                }
            } else if self.show_delete_character {
                if self.delete_confirm_input.chars().count() < 16 {
                    self.delete_confirm_input.push(ch);
                }
            }
        }
        
        if is_key_pressed(KeyCode::Backspace) {
            if self.show_new_character {
                self.new_char_name.pop();
            } else if self.show_delete_character {
                self.delete_confirm_input.pop();
            }
        }
    }
}

impl Scene for SelectScene {
    fn name(&self) -> &str {
        "CharacterSelect"
    }

    fn on_enter(&mut self) -> GameResult {
        println!("🎬 进入角色选择界面");
        Ok(())
    }
    
    fn on_exit(&mut self) -> GameResult {
        println!("🎬 离开角色选择界面");
        Ok(())
    }
    
    fn update(&mut self, dt: f32) -> GameResult<SceneTransition> {
        // 更新角色预览动画
        self.animation_timer += dt;
        if self.animation_timer >= self.animation_delay {
            self.animation_timer = 0.0;
            self.animation_frame = (self.animation_frame + 1) % 16;
        }
        
        // 更新光标闪烁
        self.cursor_timer += dt;
        if self.cursor_timer >= 0.5 {
            self.cursor_timer = 0.0;
            self.cursor_visible = !self.cursor_visible;
        }
        
        Ok(SceneTransition::None)
    }
    
    fn render(&mut self) -> GameResult {
        clear_background(BLACK);
        
        // 绘制背景和角色
        self.draw_background();
        self.draw_character_preview();
        self.draw_character_info();
        
        // 绘制UI
        self.draw_character_buttons();
        self.draw_bottom_buttons();
        
        // 绘制对话框
        if self.show_new_character {
            self.draw_new_character_dialog();
        }
        
        if self.show_delete_character {
            self.draw_delete_character_dialog();
        }
        
        if self.show_message_box {
            self.draw_message_box();
        }
        
        Ok(())
    }

    fn handle_input(&mut self) -> GameResult {
        // 处理文本输入
        self.handle_text_input();
        
        // ESC 关闭对话框
        if is_key_pressed(KeyCode::Escape) {
            if self.show_message_box {
                self.show_message_box = false;
            } else if self.show_new_character {
                self.show_new_character = false;
            } else if self.show_delete_character {
                self.show_delete_character = false;
            }
        }
        
        // 点击关闭消息框
        if self.show_message_box && is_mouse_button_pressed(MouseButton::Left) {
            let screen_w = screen_width();
            let screen_h = screen_height();
            let dialog_w = 300.0;
            let dialog_h = 120.0;
            let dialog_x = (screen_w - dialog_w) / 2.0;
            let dialog_y = (screen_h - dialog_h) / 2.0;
            let btn_x = dialog_x + dialog_w / 2.0 - 40.0;
            let btn_y = dialog_y + dialog_h - 45.0;
            
            let (mx, my) = mouse_position();
            if mx >= btn_x && mx <= btn_x + 80.0 && my >= btn_y && my <= btn_y + 30.0 {
                self.show_message_box = false;
            }
        }
        
        // Enter 确认
        if is_key_pressed(KeyCode::Enter) {
            if self.show_new_character {
                self.on_create_character();
            } else if self.show_delete_character && self.delete_confirm_input == self.delete_char_name {
                self.on_delete_character();
            }
        }
        
        Ok(())
    }
}
