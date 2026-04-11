// ============================================================================
// MentorDialogHybrid - 师徒对话框
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/MentorDialog.cs (314 行)
// - 背景：Prguse[962]
// - 标题：Title[17] at (18, 9)
// - 关闭按钮：Title[193/194/195] at (200, 256)
// - 师徒关系展示：师傅/徒弟名称、经验、允许拜师开关
// - 按钮：拜师、取消申请、允许/拒绝拜师
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::DragHelper;

/// 师徒信息
#[derive(Debug, Clone, Default)]
pub struct MentorInfo {
    pub mentor_name: String,
    pub mentor_level: u32,
    pub mentor_online: bool,
    pub apprentice_name: String,
    pub apprentice_level: u32,
    pub apprentice_online: bool,
    pub exp_points: u32,
    pub allow_request: bool,
    pub is_mentor: bool,    // 本地玩家是师傅
    pub is_apprentice: bool, // 本地玩家是徒弟
}

/// 师徒对话框动作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MentorDialogAction {
    None,
    AddMentor,
    CancelMentor,
    ToggleAllowRequest,
    AcceptMentor,
    DeclineMentor,
}

pub struct MentorDialogHybrid {
    position: Vec2,
    visible: bool,
    size: Vec2,
    mentor_info: MentorInfo,
    bg_texture: Option<Texture2D>,
    title_texture: Option<Texture2D>,
    close_button_textures: [Option<Texture2D>; 3],
    drag_helper: DragHelper,
    pending_action: MentorDialogAction,
}

impl Default for MentorDialogHybrid {
    fn default() -> Self {
        Self::new()
    }
}

impl MentorDialogHybrid {
    const CONTENT_START_Y: f32 = 50.0;
    const BUTTON_Y: f32 = 210.0;

    pub fn new() -> Self {
        Self {
            position: vec2(280.0, 100.0),
            visible: false,
            size: vec2(260.0, 290.0),
            mentor_info: MentorInfo::default(),
            bg_texture: None,
            title_texture: None,
            close_button_textures: [None, None, None],
            drag_helper: DragHelper::new(),
            pending_action: MentorDialogAction::None,
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

    /// 更新师徒信息
    pub fn update_mentor_info(&mut self, info: MentorInfo) {
        self.mentor_info = info;
    }

    /// 设置是否允许拜师请求
    pub fn set_allow_request(&mut self, allow: bool) {
        self.mentor_info.allow_request = allow;
    }

    /// 检查是否允许拜师请求
    pub fn allow_request(&self) -> bool {
        self.mentor_info.allow_request
    }

    /// 获取待处理动作
    pub fn take_action(&mut self) -> MentorDialogAction {
        std::mem::replace(&mut self.pending_action, MentorDialogAction::None)
    }

    /// 加载纹理
    pub fn load_textures(&mut self) {
        // 背景纹理 - Prguse[962]
        if let Some(texture) = LibraryName::Prguse.get_texture(962) {
            self.size = vec2(texture.width as f32, texture.height as f32);
            if let Some(tex) = texture.image {
                self.bg_texture = Some(tex);
            }
        }

        // 标题纹理 - Title[17]
        if let Some(texture) = LibraryName::Title.get_texture(17) {
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

        // 绘制师徒信息
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
        let line_h = 24.0;

        if self.mentor_info.is_mentor {
            // 我是师傅
            draw_text_cn("身份：师傅", content_x, content_y, 13.0, Color::from_rgba(255, 200, 50, 255));
            draw_text_cn("徒弟：", content_x, content_y + line_h, 12.0, Color::from_rgba(200, 200, 200, 255));

            let apprentice_text = if self.mentor_info.apprentice_name.is_empty() {
                "暂无徒弟"
            } else {
                &self.mentor_info.apprentice_name
            };
            let apprentice_color = if self.mentor_info.apprentice_online { WHITE } else { GRAY };
            draw_text_cn(
                apprentice_text,
                content_x + 50.0,
                content_y + line_h,
                12.0,
                apprentice_color,
            );

            let lv_text = format!("Lv.{}", self.mentor_info.apprentice_level);
            draw_text_cn(&lv_text, content_x + 160.0, content_y + line_h, 11.0, GRAY);
        } else if self.mentor_info.is_apprentice {
            // 我是徒弟
            draw_text_cn("身份：徒弟", content_x, content_y, 13.0, Color::from_rgba(100, 180, 255, 255));
            draw_text_cn("师傅：", content_x, content_y + line_h, 12.0, Color::from_rgba(200, 200, 200, 255));

            let mentor_text = if self.mentor_info.mentor_name.is_empty() {
                "暂无师傅"
            } else {
                &self.mentor_info.mentor_name
            };
            let mentor_color = if self.mentor_info.mentor_online { WHITE } else { GRAY };
            draw_text_cn(
                mentor_text,
                content_x + 50.0,
                content_y + line_h,
                12.0,
                mentor_color,
            );

            let lv_text = format!("Lv.{}", self.mentor_info.mentor_level);
            draw_text_cn(&lv_text, content_x + 160.0, content_y + line_h, 11.0, GRAY);
        } else {
            draw_text_cn("暂无师徒关系", content_x, content_y + line_h, 13.0, GRAY);
        }

        // 师徒经验
        draw_text_cn("师徒经验：", content_x, content_y + line_h * 2.0, 12.0, Color::from_rgba(200, 200, 200, 255));
        let exp_text = format!("{}", self.mentor_info.exp_points);
        draw_text_cn(&exp_text, content_x + 70.0, content_y + line_h * 2.0, 12.0, WHITE);

        // 允许拜师状态
        let status_text = if self.mentor_info.allow_request { "允许拜师" } else { "拒绝拜师" };
        let status_color = if self.mentor_info.allow_request { GREEN } else { RED };
        draw_text_cn(status_text, content_x, content_y + line_h * 3.0, 12.0, status_color);
    }

    fn draw_buttons(&mut self, mouse_pos: Vec2) {
        let btn_y = self.position.y + Self::BUTTON_Y;
        let btn_w = 70.0;
        let btn_h = 25.0;
        let btn_spacing = 10.0;

        // 根据当前关系动态决定按钮
        let buttons: Vec<(&str, MentorDialogAction)> = if !self.mentor_info.is_mentor && !self.mentor_info.is_apprentice {
            vec![
                ("拜师", MentorDialogAction::AddMentor),
                ("允许拜师", MentorDialogAction::ToggleAllowRequest),
            ]
        } else if self.mentor_info.is_apprentice {
            vec![
                ("取消申请", MentorDialogAction::CancelMentor),
                ("允许拜师", MentorDialogAction::ToggleAllowRequest),
            ]
        } else {
            vec![
                ("允许拜师", MentorDialogAction::ToggleAllowRequest),
            ]
        };

        let total_w = buttons.len() as f32 * (btn_w + btn_spacing) - btn_spacing;
        let start_x = self.position.x + (self.size.x - total_w) / 2.0;

        for (i, (label, action)) in buttons.iter().enumerate() {
            let btn_x = start_x + i as f32 * (btn_w + btn_spacing);
            let btn_rect = Rect::new(btn_x, btn_y, btn_w, btn_h);

            let is_hovered = btn_rect.contains(mouse_pos);
            let is_pressed = is_hovered && is_mouse_button_down(MouseButton::Left);

            let btn_color = if is_pressed {
                Color::from_rgba(100, 120, 140, 255)
            } else if is_hovered {
                Color::from_rgba(80, 100, 120, 255)
            } else {
                Color::from_rgba(60, 70, 80, 255)
            };
            draw_rectangle(btn_x, btn_y, btn_w, btn_h, btn_color);
            draw_rectangle_lines(btn_x, btn_y, btn_w, btn_h, 1.0, Color::from_rgba(100, 100, 120, 255));

            draw_text_cn(label, btn_x + 10.0, btn_y + 16.0, 12.0, WHITE);

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
