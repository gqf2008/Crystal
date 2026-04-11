// ============================================================================
// MailDialogHybrid - 邮件对话框
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/MailDialog.cs
// - 背景：Prguse[956]
// - 标题：Title[20] at (18, 9)
// - 关闭按钮：Title[193/194/195] at (200, 256)
// - 多标签页：收件箱、发件箱、写信
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use crate::ui::ui_state::MailEntry;
use super::native_ui_utils::DragHelper;

/// 标签页
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailTab {
    Inbox,
    Outbox,
    Compose,
}

/// 邮件对话框动作
#[derive(Debug, Clone, PartialEq)]
pub enum MailDialogAction {
    None,
    ReadMail { mail_id: u64 },
    CollectParcel { mail_id: u64 },
    DeleteMail { mail_id: u64 },
    SendMail { to: String, subject: String, body: String },
}

pub struct MailDialogHybrid {
    position: Vec2,
    visible: bool,
    size: Vec2,
    active_tab: MailTab,
    mails: Vec<MailEntry>,
    selected_mail: Option<usize>,
    scroll_offset: f32,
    bg_texture: Option<Texture2D>,
    title_texture: Option<Texture2D>,
    close_button_textures: [Option<Texture2D>; 3],
    drag_helper: DragHelper,
    pending_action: MailDialogAction,
}

impl Default for MailDialogHybrid {
    fn default() -> Self {
        Self::new()
    }
}

impl MailDialogHybrid {
    const TAB_Y: f32 = 35.0;
    const CONTENT_START_Y: f32 = 60.0;
    const ITEM_H: f32 = 22.0;
    const BUTTON_Y: f32 = 210.0;

    pub fn new() -> Self {
        Self {
            position: vec2(280.0, 80.0),
            visible: false,
            size: vec2(360.0, 290.0),
            active_tab: MailTab::Inbox,
            mails: Vec::new(),
            selected_mail: None,
            scroll_offset: 0.0,
            bg_texture: None,
            title_texture: None,
            close_button_textures: [None, None, None],
            drag_helper: DragHelper::new(),
            pending_action: MailDialogAction::None,
        }
    }

    pub fn open(&mut self) {
        self.visible = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.selected_mail = None;
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

    /// 同步邮件列表
    pub fn set_mails(&mut self, mails: Vec<MailEntry>) {
        self.mails = mails;
        self.selected_mail = None;
        self.scroll_offset = 0.0;
    }

    /// 追加一封邮件
    pub fn add_mail(&mut self, mail: MailEntry) {
        self.mails.push(mail);
    }

    /// 获取待处理动作
    pub fn take_action(&mut self) -> MailDialogAction {
        std::mem::replace(&mut self.pending_action, MailDialogAction::None)
    }

    /// 获取当前选中邮件的 ID
    pub fn get_selected_mail_id(&self) -> Option<u64> {
        self.selected_mail
            .and_then(|i| self.mails.get(i))
            .map(|m| m.mail_id)
    }

    /// 加载纹理
    pub fn load_textures(&mut self) {
        // 背景纹理 - Prguse[956]
        if let Some(texture) = LibraryName::Prguse.get_texture(956) {
            self.size = vec2(texture.width as f32, texture.height as f32);
            if let Some(tex) = texture.image {
                self.bg_texture = Some(tex);
            }
        }

        // 标题纹理 - Title[20]
        if let Some(texture) = LibraryName::Title.get_texture(20) {
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

        // 绘制标签页
        self.draw_tabs(mouse_pos);

        // 绘制内容
        match self.active_tab {
            MailTab::Inbox => self.draw_mail_list(mouse_pos),
            MailTab::Outbox => self.draw_outbox(),
            MailTab::Compose => self.draw_compose(mouse_pos),
        }

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

    fn draw_tabs(&mut self, mouse_pos: Vec2) {
        let tab_y = self.position.y + Self::TAB_Y;
        let tab_w = 80.0;
        let tab_h = 22.0;
        let tab_spacing = 2.0;
        let start_x = self.position.x + 15.0;

        let tabs = ["收件箱", "发件箱", "写信"];
        let tab_kinds = [MailTab::Inbox, MailTab::Outbox, MailTab::Compose];

        for (i, (label, kind)) in tabs.iter().zip(tab_kinds.iter()).enumerate() {
            let tab_x = start_x + i as f32 * (tab_w + tab_spacing);
            let tab_rect = Rect::new(tab_x, tab_y, tab_w, tab_h);
            let is_active = self.active_tab == *kind;
            let is_hovered = tab_rect.contains(mouse_pos);

            let tab_color = if is_active {
                Color::from_rgba(80, 100, 120, 255)
            } else if is_hovered {
                Color::from_rgba(60, 70, 80, 200)
            } else {
                Color::from_rgba(40, 45, 55, 200)
            };
            draw_rectangle(tab_x, tab_y, tab_w, tab_h, tab_color);
            draw_rectangle_lines(tab_x, tab_y, tab_w, tab_h, 1.0, Color::from_rgba(100, 100, 120, 255));

            draw_text_cn(label, tab_x + 12.0, tab_y + 15.0, 11.0, WHITE);

            if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
                self.active_tab = *kind;
                self.selected_mail = None;
            }
        }
    }

    fn draw_mail_list(&mut self, mouse_pos: Vec2) {
        let list_x = self.position.x + 10.0;
        let list_w = self.size.x - 20.0;
        let list_top = self.position.y + Self::CONTENT_START_Y;
        let list_bottom = self.position.y + Self::BUTTON_Y - 5.0;
        let list_h = (list_bottom - list_top).max(0.0);

        // 鼠标滚轮
        let list_rect = Rect::new(list_x, list_top, list_w, list_h);
        if list_rect.contains(mouse_pos) {
            let wheel = mouse_wheel().1;
            if wheel != 0.0 {
                self.scroll_offset = (self.scroll_offset - wheel * 20.0).max(0.0);
            }
        }

        let mut y = list_top - self.scroll_offset;
        let mut clicked: Option<usize> = None;

        if self.mails.is_empty() {
            draw_text_cn("收件箱为空", list_x + 15.0, list_top + 30.0, 12.0, GRAY);
            return;
        }

        for (i, mail) in self.mails.iter().enumerate() {
            let item_rect = Rect::new(list_x + 5.0, y, list_w - 10.0, Self::ITEM_H);
            let item_visible = item_rect.y + item_rect.h > list_top && item_rect.y < list_bottom;

            if item_visible {
                let is_selected = self.selected_mail == Some(i);
                let is_hovered = item_rect.contains(mouse_pos);

                if is_selected || is_hovered {
                    let color = if is_selected {
                        Color::from_rgba(60, 80, 100, 150)
                    } else {
                        Color::from_rgba(50, 50, 60, 100)
                    };
                    draw_rectangle(item_rect.x, item_rect.y, item_rect.w, item_rect.h, color);
                }

                // 未读标记
                let name_color = if mail.is_read { WHITE } else { Color::from_rgba(100, 200, 255, 255) };
                let prefix = if !mail.is_read { "[新] " } else { "" };

                draw_text_cn(
                    &format!("{}{}", prefix, mail.sender),
                    list_x + 10.0,
                    item_rect.y + 14.0,
                    11.0,
                    name_color,
                );

                // 主题
                draw_text_cn(&mail.subject, list_x + 80.0, item_rect.y + 14.0, 11.0, Color::from_rgba(200, 200, 200, 255));

                // 包裹标记
                if mail.has_parcel {
                    draw_text_cn("[包裹]", list_x + 250.0, item_rect.y + 14.0, 10.0, Color::from_rgba(255, 200, 50, 255));
                }

                if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
                    clicked = Some(i);
                }

                // 双击读取邮件
                if is_hovered && is_mouse_button_down(MouseButton::Left) {
                    self.pending_action = MailDialogAction::ReadMail { mail_id: mail.mail_id };
                }
            }

            y += Self::ITEM_H;
        }

        if let Some(idx) = clicked {
            self.selected_mail = Some(idx);
        }

        // 限制滚动
        let content_h = self.mails.len() as f32 * Self::ITEM_H;
        let max_scroll = (content_h - list_h).max(0.0);
        if self.scroll_offset > max_scroll {
            self.scroll_offset = max_scroll;
        }
    }

    fn draw_outbox(&self) {
        let content_y = self.position.y + Self::CONTENT_START_Y;
        let content_x = self.position.x + 15.0;
        draw_text_cn("发件箱为空", content_x, content_y + 30.0, 12.0, GRAY);
    }

    fn draw_compose(&self, _mouse_pos: Vec2) {
        let content_y = self.position.y + Self::CONTENT_START_Y;
        let content_x = self.position.x + 15.0;
        draw_text_cn("写信功能需要文本输入支持", content_x, content_y + 30.0, 12.0, GRAY);
    }

    fn draw_buttons(&mut self, mouse_pos: Vec2) {
        let btn_y = self.position.y + Self::BUTTON_Y;
        let btn_w = 80.0;
        let btn_h = 25.0;
        let btn_spacing = 10.0;

        // 根据当前选中邮件和标签页显示不同按钮
        let has_selection = self.selected_mail.is_some() && self.active_tab == MailTab::Inbox;
        let has_parcel = self.selected_mail
            .and_then(|i| self.mails.get(i))
            .map(|m| m.has_parcel)
            .unwrap_or(false);

        let buttons: Vec<(&str, MailDialogAction)> = if has_selection {
            let mut btns = vec![("删除邮件", MailDialogAction::DeleteMail {
                mail_id: self.get_selected_mail_id().unwrap_or(0),
            })];
            if has_parcel {
                btns.push(("领取包裹", MailDialogAction::CollectParcel {
                    mail_id: self.get_selected_mail_id().unwrap_or(0),
                }));
            }
            btns
        } else {
            vec![]
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
                self.pending_action = action.clone();
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
