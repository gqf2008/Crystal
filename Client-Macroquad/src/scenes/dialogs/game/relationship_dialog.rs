// ============================================================================
// RelationshipDialogHybrid - 婚姻对话框
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/RelationshipDialog.cs (247 行)
// - 背景：Prguse[963]
// - 标题：Title[18] at (18, 9)
// - 关闭按钮：Title[193/194/195] at (200, 256)
// - 配偶信息展示、亲密度、离婚按钮
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::DragHelper;

/// 婚姻/伴侣信息
#[derive(Debug, Clone, Default)]
pub struct RelationshipInfo {
    pub partner_name: String,
    pub partner_level: u32,
    pub partner_online: bool,
    pub intimacy: u32,
    pub max_intimacy: u32,
    pub married: bool,
    pub wedding_date: String,
    /// 待处理的求婚请求者（非空时显示接受/拒绝按钮）
    pub pending_marriage_requester: String,
}

/// 婚姻对话框动作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipDialogAction {
    None,
    RequestMarriage,
    RequestDivorce,
    AcceptMarriage,
    DeclineMarriage,
}

pub struct RelationshipDialogHybrid {
    position: Vec2,
    visible: bool,
    size: Vec2,
    relationship: RelationshipInfo,
    bg_texture: Option<Texture2D>,
    title_texture: Option<Texture2D>,
    close_button_textures: [Option<Texture2D>; 3],
    drag_helper: DragHelper,
    pending_action: RelationshipDialogAction,
}

impl Default for RelationshipDialogHybrid {
    fn default() -> Self {
        Self::new()
    }
}

impl RelationshipDialogHybrid {
    const CONTENT_START_Y: f32 = 50.0;
    const BUTTON_Y: f32 = 210.0;

    pub fn new() -> Self {
        Self {
            position: vec2(300.0, 100.0),
            visible: false,
            size: vec2(260.0, 290.0),
            relationship: RelationshipInfo::default(),
            bg_texture: None,
            title_texture: None,
            close_button_textures: [None, None, None],
            drag_helper: DragHelper::new(),
            pending_action: RelationshipDialogAction::None,
        }
    }

    pub fn open(&mut self) {
        self.visible = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn set_position(&mut self, pos: Vec2) {
        self.position = pos;
    }

    pub fn get_position(&self) -> Vec2 {
        self.position
    }

    pub fn contains(&self, point: Vec2) -> bool {
        if !self.visible {
            return false;
        }
        Rect::new(self.position.x, self.position.y, self.size.x, self.size.y).contains(point)
    }

    /// 更新婚姻信息
    pub fn update_relationship(&mut self, info: RelationshipInfo) {
        self.relationship = info;
    }

    /// 获取待处理动作
    pub fn take_action(&mut self) -> RelationshipDialogAction {
        std::mem::replace(&mut self.pending_action, RelationshipDialogAction::None)
    }

    /// 设置待处理求婚请求者
    pub fn set_marriage_requester(&mut self, name: String) {
        self.relationship.pending_marriage_requester = name;
    }

    /// 清除待处理求婚请求
    pub fn clear_marriage_requester(&mut self) {
        self.relationship.pending_marriage_requester.clear();
    }

    /// 更新伴侣信息（增量合并，不清空师徒数据）
    pub fn set_lover_info(&mut self, name: String, date: i64) {
        self.relationship.partner_name = name;
        self.relationship.married = true;
        self.relationship.partner_online = true;
        self.relationship.wedding_date = format!("{}", date);
    }

    /// 更新师徒信息（增量合并，不清空伴侣数据）
    pub fn set_mentor_info(&mut self, name: String, level: u32, online: bool) {
        self.relationship.partner_name = name;
        self.relationship.partner_level = level;
        self.relationship.partner_online = online;
    }

    /// 加载纹理
    pub fn load_textures(&mut self) {
        // 背景纹理 - Prguse[963]
        if let Some(texture) = LibraryName::Prguse.get_texture(963) {
            self.size = vec2(texture.width as f32, texture.height as f32);
            if let Some(tex) = texture.image {
                self.bg_texture = Some(tex);
            }
        }

        // 标题纹理 - Title[18]
        if let Some(texture) = LibraryName::Title.get_texture(18) {
            if let Some(tex) = texture.image {
                self.title_texture = Some(tex);
            }
        }

        // 关闭按钮 - Title[193/194/195]
        for (i, idx) in [193, 194, 195].iter().enumerate() {
            if let Some(texture) = LibraryName::Title.get_texture(*idx) {
                if let Some(tex) = texture.image {
                    self.close_button_textures[i] = Some(tex);
                }
            }
        }
    }

    pub fn update_and_draw(&mut self) {
        if !self.visible {
            return;
        }

        let mouse_pos = vec2(mouse_position().0, mouse_position().1);

        // 拖拽
        let drag_area = Rect::new(self.position.x, self.position.y, self.size.x, 30.0);
        self.drag_helper.apply(drag_area, &mut self.position);

        // 绘制背景
        self.draw_background();

        // 绘制关系信息
        self.draw_relationship_info();

        // 绘制按钮
        self.draw_buttons(mouse_pos);

        // 绘制关闭按钮
        self.draw_close_button(mouse_pos);
    }

    fn draw_background(&self) {
        if let Some(texture) = &self.bg_texture {
            draw_texture_ex(
                texture,
                self.position.x,
                self.position.y,
                WHITE,
                DrawTextureParams::default(),
            );
        }

        if let Some(title_tex) = &self.title_texture {
            draw_texture_ex(
                title_tex,
                self.position.x + 18.0,
                self.position.y + 9.0,
                WHITE,
                DrawTextureParams::default(),
            );
        }
    }

    fn draw_relationship_info(&self) {
        let content_y = self.position.y + Self::CONTENT_START_Y;
        let content_x = self.position.x + 15.0;
        let line_h = 24.0_f32;

        if self.relationship.married && !self.relationship.partner_name.is_empty() {
            // 已婚
            draw_text_cn("已婚", content_x, content_y, 14.0, Color::from_rgba(255, 150, 150, 255));

            draw_text_cn("配偶：", content_x, content_y + line_h, 12.0, Color::from_rgba(200, 200, 200, 255));
            let partner_color = if self.relationship.partner_online { WHITE } else { GRAY };
            draw_text_cn(
                &self.relationship.partner_name,
                content_x + 50.0,
                content_y + line_h,
                12.0,
                partner_color,
            );

            let lv_text = format!("Lv.{}", self.relationship.partner_level);
            draw_text_cn(&lv_text, content_x + 160.0, content_y + line_h, 11.0, GRAY);

            // 亲密度（仅当服务器提供数据时显示）
            if self.relationship.intimacy > 0 || self.relationship.max_intimacy > 0 {
                draw_text_cn("亲密度：", content_x, content_y + line_h * 2.0, 12.0, Color::from_rgba(200, 200, 200, 255));
                let intimacy_text = format!("{}/{}", self.relationship.intimacy, self.relationship.max_intimacy);
                draw_text_cn(&intimacy_text, content_x + 60.0, content_y + line_h * 2.0, 12.0, Color::from_rgba(255, 180, 200, 255));

                // 亲密度条
                let bar_x = content_x;
                let bar_y = content_y + line_h * 3.0 - 4.0;
                let bar_w = self.size.x - 30.0;
                let bar_h = 8.0;
                let ratio = if self.relationship.max_intimacy > 0 {
                    (self.relationship.intimacy as f32 / self.relationship.max_intimacy as f32).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                draw_rectangle(bar_x, bar_y, bar_w, bar_h, Color::from_rgba(40, 40, 50, 255));
                draw_rectangle(bar_x, bar_y, bar_w * ratio, bar_h, Color::from_rgba(255, 130, 170, 200));

                if !self.relationship.wedding_date.is_empty() {
                    draw_text_cn("结婚日期：", content_x, content_y + line_h * 4.0, 11.0, Color::from_rgba(180, 180, 180, 255));
                    draw_text_cn(&self.relationship.wedding_date, content_x + 70.0, content_y + line_h * 4.0, 11.0, Color::from_rgba(180, 180, 180, 255));
                }
            } else {
                // 协议未提供亲密度数据，跳过显示
            }
        } else {
            draw_text_cn("未婚", content_x, content_y + line_h, 14.0, GRAY);
            draw_text_cn("当前没有婚姻关系", content_x, content_y + line_h * 2.0, 12.0, Color::from_rgba(150, 150, 150, 200));
        }
    }

    fn draw_buttons(&mut self, mouse_pos: Vec2) {
        let btn_y = self.position.y + Self::BUTTON_Y;
        let btn_w = 80.0;
        let btn_h = 25.0;
        let btn_spacing = 10.0;

        let buttons: Vec<(&str, RelationshipDialogAction)> = if !self.relationship.married {
            if self.relationship.pending_marriage_requester.is_empty() {
                vec![
                    ("求婚", RelationshipDialogAction::RequestMarriage),
                ]
            } else {
                vec![
                    ("接受", RelationshipDialogAction::AcceptMarriage),
                    ("拒绝", RelationshipDialogAction::DeclineMarriage),
                ]
            }
        } else {
            vec![
                ("离婚", RelationshipDialogAction::RequestDivorce),
            ]
        };

        if buttons.is_empty() {
            return;
        }

        let total_w = buttons.len() as f32 * (btn_w + btn_spacing) - btn_spacing;
        let start_x = self.position.x + (self.size.x - total_w) / 2.0;

        for (i, (label, action)) in buttons.iter().enumerate() {
            let btn_x = start_x + i as f32 * (btn_w + btn_spacing);
            let btn_rect = Rect::new(btn_x, btn_y, btn_w, btn_h);

            let is_hovered = btn_rect.contains(mouse_pos);
            let is_pressed = is_hovered && is_mouse_button_down(MouseButton::Left);

            let btn_color = if is_pressed {
                Color::from_rgba(180, 60, 60, 255)
            } else if is_hovered {
                Color::from_rgba(150, 50, 50, 255)
            } else {
                Color::from_rgba(120, 40, 40, 255)
            };
            draw_rectangle(btn_x, btn_y, btn_w, btn_h, btn_color);
            draw_rectangle_lines(btn_x, btn_y, btn_w, btn_h, 1.0, Color::from_rgba(200, 80, 80, 255));

            draw_text_cn(label, btn_x + 15.0, btn_y + 16.0, 12.0, WHITE);

            if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
                self.pending_action = *action;
            }
        }
    }

    fn draw_close_button(&mut self, mouse_pos: Vec2) {
        let btn_x = self.position.x + 200.0;
        let btn_y = self.position.y + 256.0;

        if let Some(normal) = &self.close_button_textures[0] {
            let btn_rect = Rect::new(btn_x, btn_y, normal.width(), normal.height());
            let is_hovered = btn_rect.contains(mouse_pos);
            let is_pressed = is_hovered && is_mouse_button_down(MouseButton::Left);

            let texture = if is_pressed {
                self.close_button_textures[2].as_ref().unwrap_or(normal)
            } else if is_hovered {
                self.close_button_textures[1].as_ref().unwrap_or(normal)
            } else {
                normal
            };

            draw_texture_ex(texture, btn_x, btn_y, WHITE, DrawTextureParams::default());

            if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
                self.close();
            }
        }
    }
}
