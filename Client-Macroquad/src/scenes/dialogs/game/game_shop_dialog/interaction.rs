// ============================================================================
// GameShopDialogHybrid - 交互处理模块
// ============================================================================
// 
// 所有交互相关方法（包含点击、拖拽、搜索、预览等）
// ============================================================================

use macroquad::prelude::*;
use macroquad::ui::{self, hash};
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::dialog::GameShopDialogHybrid;
use super::super::native_ui_utils::{draw_library_button_with_offset, mouse_pos as ui_mouse_pos};

impl GameShopDialogHybrid {
    /// 绘制搜索框
    pub(super) fn draw_search_box(&mut self, pos: Vec2) {
        let search_x = pos.x + 540.0;
        let search_y = pos.y + 69.0;
        let search_w = 140.0;
        let search_h = 16.0;
        
        let mouse_pos = ui_mouse_pos();
        let hovered = mouse_pos.x >= search_x && mouse_pos.x <= search_x + search_w
            && mouse_pos.y >= search_y && mouse_pos.y <= search_y + search_h;
        
        // 背景
        let bg_color = if self.search_active {
            Color::from_rgba(20, 20, 30, 255)
        } else if hovered {
            Color::from_rgba(15, 15, 25, 255)
        } else {
            Color::from_rgba(4, 4, 4, 255)
        };
        draw_rectangle(search_x, search_y, search_w, search_h, bg_color);
        draw_rectangle_lines(search_x, search_y, search_w, search_h,
            1.0, if self.search_active { 
                Color::from_rgba(100, 150, 200, 255)
            } else {
                Color::from_rgba(80, 80, 100, 255)
            });
        
        // 点击激活搜索框
        if hovered && is_mouse_button_pressed(MouseButton::Left) {
            self.search_active = true;
        } else if !hovered && is_mouse_button_pressed(MouseButton::Left) {
            self.search_active = false;
        }
        
        // 显示搜索文本或占位符
        if self.search_text.is_empty() && !self.search_active {
            draw_text_cn("搜索...", search_x + 4.0, search_y + 12.0, 9.0, GRAY);
        } else {
            draw_text_cn(&self.search_text, search_x + 4.0, search_y + 12.0, 9.0, WHITE);
            // 光标
            if self.search_active {
                let cursor_x = search_x + 4.0 + self.search_text.chars().count() as f32 * 6.0;
                if (get_time() * 2.0) as i32 % 2 == 0 {
                    draw_line(cursor_x, search_y + 3.0, cursor_x, search_y + 13.0, 1.0, WHITE);
                }
            }
        }
        
        // 处理键盘输入
        if self.search_active {
            // 退格删除
            if is_key_pressed(KeyCode::Backspace) && !self.search_text.is_empty() {
                self.search_text.pop();
                self.refresh_categories_and_items();
            }
            // ESC取消
            if is_key_pressed(KeyCode::Escape) {
                self.search_active = false;
            }
            // 获取输入字符 (简化处理，只支持基本ASCII)
            for key in get_keys_pressed() {
                if let Some(c) = key_to_char(key) {
                    if self.search_text.len() < 23 {
                        self.search_text.push(c);
                        self.refresh_categories_and_items();
                    }
                }
            }
        }
    }
    
    /// 绘制预览窗口 (使用 Title[785] 纹理)
    pub(super) fn draw_preview_window(&mut self, pos: Vec2) {
        if let Some(idx) = self.preview_item {
            if idx >= self.filtered_items.len() {
                self.preview_item = None;
                return;
            }
            
            let item = &self.filtered_items[idx];
            let preview_w = 260.0;
            let preview_h = 300.0;
            let preview_x = pos.x + Self::DIALOG_WIDTH - preview_w - 30.0;
            let preview_y = pos.y + 120.0;
            
            // 半透明遮罩
            draw_rectangle(pos.x, pos.y, Self::DIALOG_WIDTH, Self::DIALOG_HEIGHT,
                Color::from_rgba(0, 0, 0, 80));
            
            // 预览窗口背景 Title[785]
            if let Some(ref tex) = self.viewer_bg_texture {
                draw_texture_ex(tex, preview_x, preview_y, WHITE, DrawTextureParams::default());
            } else {
                draw_rectangle(preview_x, preview_y, preview_w, preview_h,
                    Color::from_rgba(40, 40, 50, 250));
                draw_rectangle_lines(preview_x, preview_y, preview_w, preview_h,
                    2.0, Color::from_rgba(150, 150, 170, 255));
            }

            // 预览区域 (原版位置: 105, 160 居中)
            let preview_area_x = preview_x + 80.0;
            let preview_area_y = preview_y + 100.0;
            draw_rectangle(preview_area_x, preview_area_y, 100.0, 80.0,
                Color::from_rgba(20, 20, 30, 180));
            
            // 图标（大）
            let icon_x = preview_area_x + 18.0;
            let icon_y = preview_area_y + 8.0;
            if let Some(info) = LibraryName::Items.get_texture(item.icon_index) {
                if let Some(ref tex) = info.image {
                    draw_texture_ex(tex, icon_x, icon_y, WHITE, DrawTextureParams {
                        dest_size: Some(vec2(64.0, 64.0)),
                        ..Default::default()
                    });
                }
            } else {
                draw_rectangle(icon_x, icon_y, 64.0, 64.0, Color::from_rgba(60, 60, 70, 255));
            }
            
            // 价格
            if item.price_gold > 0 {
                draw_text_cn(&format!("金币: {}", item.price_gold), preview_x + 20.0, preview_y + 210.0,
                    12.0, Color::from_rgba(255, 215, 0, 255));
            }
            if item.price_ingot > 0 {
                draw_text_cn(&format!("元宝: {}", item.price_ingot), preview_x + 20.0, preview_y + 230.0,
                    12.0, Color::from_rgba(0, 255, 255, 255));
            }
            
            let mouse_pos = ui_mouse_pos();
            
            // 方向控制按钮 (原版位置: LeftDirection 81,282  RightDirection 160,282)
            let dir_y = preview_y + 252.0;

            if draw_library_button_with_offset(
                LibraryName::Prguse2,
                [240, 241, 242],
                vec2(preview_x + 81.0, dir_y),
                mouse_pos,
            ) {
                self.preview_direction = if self.preview_direction == 1 { 8 } else { self.preview_direction - 1 };
                println!("🔄 预览方向: {}", self.preview_direction);
            }

            if draw_library_button_with_offset(
                LibraryName::Prguse2,
                [243, 244, 245],
                vec2(preview_x + 160.0, dir_y),
                mouse_pos,
            ) {
                self.preview_direction = if self.preview_direction == 8 { 1 } else { self.preview_direction + 1 };
                println!("🔄 预览方向: {}", self.preview_direction);
            }
            
            // 方向显示
            draw_text_cn(&format!("方向: {}/8", self.preview_direction), preview_x + 105.0, dir_y + 14.0, 10.0,
                Color::from_rgba(150, 150, 150, 255));
            
            // 关闭按钮 (原版位置: 230, 8)

            if draw_library_button_with_offset(
                LibraryName::Prguse2,
                [360, 361, 362],
                vec2(preview_x + 230.0, preview_y + 8.0),
                mouse_pos,
            ) {
                self.preview_item = None;
            }
            
            // ESC 关闭
            if is_key_pressed(KeyCode::Escape) {
                self.preview_item = None;
            }
        }
    }
    
    /// 处理拖拽
    pub(super) fn handle_dragging(&mut self, pos: Vec2) {
        if let Some(skin) = &self.transparent_skin {
            ui::root_ui().push_skin(skin);
        }
        
        // 标题栏拖拽区域
        let drag_id = hash!("shop_drag");
        let title_rect = Rect::new(pos.x, pos.y, Self::DIALOG_WIDTH - 30.0, Self::TITLE_HEIGHT);
        
        let drag_result = ui::widgets::Group::new(drag_id, vec2(title_rect.w, title_rect.h))
            .position(vec2(title_rect.x, title_rect.y))
            .draggable(true)
            .ui(&mut ui::root_ui(), |_| {});
        
        // Drag 是枚举：Dragging(Vec2, Vec2), Hovered, Clicked, None
        match drag_result {
            ui::Drag::Dragging(_, _) => {
                let mouse_pos = ui_mouse_pos();
                if !self.dragging {
                    self.dragging = true;
                    self.drag_offset = mouse_pos - pos;
                }
                self.position = mouse_pos - self.drag_offset;
            }
            _ => {
                self.dragging = false;
            }
        }
        
        if let Some(_) = &self.transparent_skin {
            ui::root_ui().pop_skin();
        }
    }
    
    /// 处理关闭按钮 (使用纹理 Prguse2[360-362])
    pub(super) fn handle_close_button(&mut self, pos: Vec2) {
        // C# 原版:
        // - Dialog 背景 Title[749] 实际尺寸: 696x476 (AutoSize)
        // - CloseButton.Location = (671, 4)
        // - CloseButton 纹理 Prguse2[360] 实际尺寸: 24x21
        //   => 右边距 = 696 - 671 - 24 = 1px

        let close_pos = vec2(pos.x + 671.0, pos.y + 4.0);
        let mouse_pos = ui_mouse_pos();

        if draw_library_button_with_offset(
            LibraryName::Prguse2,
            [360, 361, 362],
            close_pos,
            mouse_pos,
        ) {
            self.close();
            println!("❌ 关闭商城");
        }
    }
    
    /// 绘制物品悬停提示框
    pub(super) fn draw_item_tooltip(&self) {
        if let Some(idx) = self.hover_item {
            if idx >= self.filtered_items.len() {
                return;
            }
            
            let item = &self.filtered_items[idx];
            let mouse = ui_mouse_pos();
            
            // 提示框内容
            let lines = vec![
                item.name.clone(),
                item.description.clone(),
                String::new(),  // 空行
                format!("分类: {:?}", item.category),
                if item.price_gold > 0 {
                    format!("金币: {}", item.price_gold)
                } else {
                    String::new()
                },
                if item.price_ingot > 0 {
                    format!("元宝: {}", item.price_ingot)
                } else {
                    String::new()
                },
                if item.count > 1 {
                    format!("数量: {}", item.count)
                } else {
                    String::new()
                },
                if item.stock > 0 {
                    format!("库存: {}", item.stock)
                } else if item.stock == 0 {
                    "库存: ∞".to_string()
                } else {
                    String::new()
                },
            ];
            
            // 过滤空行并计算尺寸
            let valid_lines: Vec<&String> = lines.iter().filter(|s| !s.is_empty()).collect();
            let line_height = 16.0;
            let padding = 8.0;
            let max_width = valid_lines.iter()
                .map(|s| s.chars().count() as f32 * 8.0)
                .fold(150.0f32, |a, b| a.max(b));
            let tooltip_w = max_width + padding * 2.0;
            let tooltip_h = valid_lines.len() as f32 * line_height + padding * 2.0;
            
            // 计算位置（在鼠标右下方，避免超出屏幕）
            let screen_w = screen_width();
            let screen_h = screen_height();
            let offset_x = 15.0;
            let offset_y = 10.0;
            
            let mut tooltip_x = mouse.x + offset_x;
            let mut tooltip_y = mouse.y + offset_y;
            
            // 边界检查
            if tooltip_x + tooltip_w > screen_w {
                tooltip_x = mouse.x - tooltip_w - 5.0;
            }
            if tooltip_y + tooltip_h > screen_h {
                tooltip_y = mouse.y - tooltip_h - 5.0;
            }
            
            // 绘制背景
            draw_rectangle(
                tooltip_x,
                tooltip_y,
                tooltip_w,
                tooltip_h,
                Color::from_rgba(20, 20, 30, 240)
            );
            
            // 绘制边框
            draw_rectangle_lines(
                tooltip_x,
                tooltip_y,
                tooltip_w,
                tooltip_h,
                2.0,
                Color::from_rgba(100, 100, 130, 255)
            );
            
            // 绘制物品名称（金色，第一行）
            let mut y_offset = tooltip_y + padding + 12.0;
            if !item.name.is_empty() {
                let name_color = if item.in_stock {
                    Color::from_rgba(255, 215, 0, 255)  // 金色
                } else {
                    GRAY
                };
                draw_text_cn(&item.name, tooltip_x + padding, y_offset, 12.0, name_color);
                y_offset += line_height;
            }
            
            // 绘制描述（白色）
            if !item.description.is_empty() {
                draw_text_cn(&item.description, tooltip_x + padding, y_offset, 11.0, WHITE);
                y_offset += line_height;
            }
            
            // 绘制其他信息（灰色）
            for line in &valid_lines[2..] {
                draw_text_cn(line, tooltip_x + padding, y_offset, 10.0, 
                    Color::from_rgba(180, 180, 180, 255));
                y_offset += line_height;
            }
            
            // 如果缺货，显示红色提示
            if !item.in_stock {
                draw_text_cn("[已售罄]", tooltip_x + padding, y_offset, 10.0, RED);
            }
        }
    }
}

/// 将KeyCode转换为字符 (简化版，只支持基本ASCII)
pub(super) fn key_to_char(key: KeyCode) -> Option<char> {
    match key {
        KeyCode::A => Some('a'),
        KeyCode::B => Some('b'),
        KeyCode::C => Some('c'),
        KeyCode::D => Some('d'),
        KeyCode::E => Some('e'),
        KeyCode::F => Some('f'),
        KeyCode::G => Some('g'),
        KeyCode::H => Some('h'),
        KeyCode::I => Some('i'),
        KeyCode::J => Some('j'),
        KeyCode::K => Some('k'),
        KeyCode::L => Some('l'),
        KeyCode::M => Some('m'),
        KeyCode::N => Some('n'),
        KeyCode::O => Some('o'),
        KeyCode::P => Some('p'),
        KeyCode::Q => Some('q'),
        KeyCode::R => Some('r'),
        KeyCode::S => Some('s'),
        KeyCode::T => Some('t'),
        KeyCode::U => Some('u'),
        KeyCode::V => Some('v'),
        KeyCode::W => Some('w'),
        KeyCode::X => Some('x'),
        KeyCode::Y => Some('y'),
        KeyCode::Z => Some('z'),
        KeyCode::Key0 => Some('0'),
        KeyCode::Key1 => Some('1'),
        KeyCode::Key2 => Some('2'),
        KeyCode::Key3 => Some('3'),
        KeyCode::Key4 => Some('4'),
        KeyCode::Key5 => Some('5'),
        KeyCode::Key6 => Some('6'),
        KeyCode::Key7 => Some('7'),
        KeyCode::Key8 => Some('8'),
        KeyCode::Key9 => Some('9'),
        KeyCode::Space => Some(' '),
        KeyCode::Minus => Some('-'),
        _ => None,
    }
}
