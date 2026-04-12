// ============================================================================
// GroupDialogHybrid - 组队对话框
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/GroupDialog.cs (226 行)
// - 背景：Prguse[964]
// - 标题：Title[16] at (18, 9)
// - 关闭按钮：Title[193/194/195] at (200, 256)
// - 成员列表：最多 10 人，每行 20px
// - 按钮：允许加入、邀请、踢出、退出
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::DragHelper;

/// 队伍成员
#[derive(Debug, Clone)]
pub struct GroupMember {
    pub name: String,
    pub hp_percent: f32,   // 0.0 - 1.0
    pub online: bool,
    pub is_leader: bool,
    pub map_name: String,   // 成员所在地图名称
}

/// 组队对话框动作
#[derive(Debug, Clone, PartialEq)]
pub enum GroupDialogAction {
    None,
    AllowJoinToggle,
    Invite,
    Leave,
    KickSelected,
    ViewMemberDetail { name: String, hp_percent: f32, is_leader: bool },
}

pub struct GroupDialogHybrid {
    position: Vec2,
    visible: bool,
    size: Vec2,
    members: Vec<GroupMember>,
    allow_join: bool,
    selected_member: Option<usize>,
    scroll_offset: f32,
    bg_texture: Option<Texture2D>,
    title_texture: Option<Texture2D>,
    close_button_textures: [Option<Texture2D>; 3],
    drag_helper: DragHelper,
    pending_action: GroupDialogAction,
    /// 双击检测（替代 static mut）
    last_click_time: f64,
    last_click_idx: Option<usize>,
    /// 本地玩家名称（用于 is_leader 判断）
    local_player_name: String,
}

impl Default for GroupDialogHybrid {
    fn default() -> Self {
        Self::new()
    }
}

impl GroupDialogHybrid {
    const MEMBER_START_Y: f32 = 40.0;
    const MEMBER_ITEM_H: f32 = 20.0;
    const BUTTON_Y: f32 = 210.0;

    pub fn new() -> Self {
        Self {
            position: vec2(250.0, 100.0),
            visible: false,
            size: vec2(260.0, 290.0), // 默认值，会被纹理覆盖
            members: Vec::new(),
            allow_join: true,
            selected_member: None,
            scroll_offset: 0.0,
            bg_texture: None,
            title_texture: None,
            close_button_textures: [None, None, None],
            drag_helper: DragHelper::new(),
            pending_action: GroupDialogAction::None,
            last_click_time: 0.0,
            last_click_idx: None,
            local_player_name: String::new(),
        }
    }

    pub fn open(&mut self) {
        self.visible = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.selected_member = None;
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

    /// 更新队伍成员列表
    pub fn update_members(&mut self, members: Vec<GroupMember>) {
        self.members = members;
    }

    /// 添加成员
    pub fn add_member(&mut self, member: GroupMember) {
        self.members.push(member);
    }

    /// 移除成员
    pub fn remove_member(&mut self, name: &str) {
        self.members.retain(|m| m.name != name);
    }

    /// 更新成员所在地图
    pub fn update_member_map(&mut self, name: &str, map_name: String) {
        if let Some(member) = self.members.iter_mut().find(|m| m.name == name) {
            member.map_name = map_name;
        }
    }

    /// 检查是否允许加入
    pub fn allow_join(&self) -> bool {
        self.allow_join
    }

    /// 设置允许加入
    pub fn set_allow_join(&mut self, allow: bool) {
        self.allow_join = allow;
    }

    /// 设置是否队长
    pub fn set_leader(&mut self, name: &str, is_leader: bool) {
        if let Some(member) = self.members.iter_mut().find(|m| m.name == name) {
            member.is_leader = is_leader;
        }
    }

    /// 设置本地玩家名称（用于 is_leader 判断）
    pub fn set_local_player_name(&mut self, name: String) {
        self.local_player_name = name;
    }

    /// 检查本地玩家是否是队长
    pub fn is_leader(&self) -> bool {
        if self.local_player_name.is_empty() {
            return false;
        }
        self.members.iter().any(|m| m.is_leader && m.name == self.local_player_name)
    }

    /// 获取待处理动作
    pub fn take_action(&mut self) -> GroupDialogAction {
        std::mem::replace(&mut self.pending_action, GroupDialogAction::None)
    }

    /// 获取当前选中成员的名称
    pub fn get_selected_member_name(&self) -> Option<String> {
        self.selected_member
            .and_then(|i| self.members.get(i))
            .map(|m| m.name.clone())
    }

    /// 获取本地玩家名称
    pub fn get_local_player_name(&self) -> String {
        self.local_player_name.clone()
    }

    /// 加载纹理
    pub fn load_textures(&mut self) {
        // 背景纹理 - Prguse[964]
        if let Some(texture) = LibraryName::Prguse.get_texture(964) {
            self.size = vec2(texture.width as f32, texture.height as f32);
            if let Some(tex) = texture.image {
                self.bg_texture = Some(tex);
            }
        }

        // 标题纹理 - Title[16]
        if let Some(texture) = LibraryName::Title.get_texture(16) {
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

        // 绘制成员列表
        self.draw_member_list(mouse_pos);

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

        // 允许加入状态指示
        let status_text = if self.allow_join { "允许加入" } else { "拒绝加入" };
        let status_color = if self.allow_join { GREEN } else { RED };
        draw_text_cn(
            status_text,
            self.position.x + 15.0,
            self.position.y + self.size.y - 25.0,
            12.0,
            status_color,
        );
    }

    fn draw_member_list(&mut self, mouse_pos: Vec2) {
        let list_x = self.position.x + 10.0;
        let list_w = self.size.x - 20.0;
        let list_top = self.position.y + Self::MEMBER_START_Y;

        // 鼠标滚轮
        let list_bottom = self.position.y + Self::BUTTON_Y - 5.0;
        let list_h = (list_bottom - list_top).max(0.0);
        let list_rect = Rect::new(list_x, list_top, list_w, list_h);

        if list_rect.contains(mouse_pos) {
            let wheel = mouse_wheel().1;
            if wheel != 0.0 {
                self.scroll_offset = (self.scroll_offset - wheel * 20.0).max(0.0);
            }
        }

        let mut y = list_top - self.scroll_offset;
        let mut clicked_member: Option<usize> = None;
        let mut double_clicked: Option<usize> = None;

        for (i, member) in self.members.iter().enumerate() {
            let item_rect = Rect::new(list_x + 5.0, y, list_w - 10.0, Self::MEMBER_ITEM_H);
            let item_visible = item_rect.y + item_rect.h > list_top && item_rect.y < list_bottom;

            if item_visible {
                let is_selected = self.selected_member == Some(i);
                let is_hovered = item_rect.contains(mouse_pos);

                // 背景
                if is_selected || is_hovered {
                    let color = if is_selected {
                        Color::from_rgba(60, 80, 100, 150)
                    } else {
                        Color::from_rgba(50, 50, 60, 100)
                    };
                    draw_rectangle(item_rect.x, item_rect.y, item_rect.w, item_rect.h, color);
                }

                // 队长标记
                if member.is_leader {
                    draw_text_cn(
                        "[队长]",
                        item_rect.x,
                        item_rect.y + 14.0,
                        10.0,
                        Color::from_rgba(255, 200, 50, 255),
                    );
                }

                // 名称
                let name_x = if member.is_leader { item_rect.x + 40.0 } else { item_rect.x };
                let name_color = if member.online { WHITE } else { GRAY };
                draw_text_cn(
                    &member.name,
                    name_x,
                    item_rect.y + 14.0,
                    12.0,
                    name_color,
                );

                // 血条
                let hp_bar_x = item_rect.x + 150.0;
                let hp_bar_y = item_rect.y + 8.0;
                let hp_bar_w = 80.0;
                let hp_bar_h = 8.0;
                draw_rectangle(hp_bar_x, hp_bar_y, hp_bar_w, hp_bar_h, Color::from_rgba(40, 40, 50, 255));
                let hp_fill_w = (hp_bar_w * member.hp_percent).clamp(0.0, hp_bar_w);
                let hp_color = if member.hp_percent > 0.6 {
                    Color::from_rgba(80, 200, 80, 200)
                } else if member.hp_percent > 0.3 {
                    Color::from_rgba(255, 200, 50, 200)
                } else {
                    Color::from_rgba(255, 80, 80, 200)
                };
                draw_rectangle(hp_bar_x, hp_bar_y, hp_fill_w, hp_bar_h, hp_color);

                // 点击检测
                if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
                    let now = get_time();
                    if self.last_click_idx == Some(i) && (now - self.last_click_time) < 0.3 {
                        double_clicked = Some(i);
                    }
                    self.last_click_time = now;
                    self.last_click_idx = Some(i);
                    clicked_member = Some(i);
                }
            }

            y += Self::MEMBER_ITEM_H;
        }

        if let Some(idx) = clicked_member {
            self.selected_member = Some(idx);
        }

        if let Some(idx) = double_clicked {
            if let Some(m) = self.members.get(idx) {
                self.pending_action = GroupDialogAction::ViewMemberDetail {
                    name: m.name.clone(),
                    hp_percent: m.hp_percent,
                    is_leader: m.is_leader,
                };
            }
        }

        // 限制滚动偏移
        let content_h = self.members.len() as f32 * Self::MEMBER_ITEM_H;
        let max_scroll = (content_h - list_h).max(0.0);
        if self.scroll_offset > max_scroll {
            self.scroll_offset = max_scroll;
        }
    }

    fn draw_buttons(&mut self, mouse_pos: Vec2) {
        let btn_y = self.position.y + Self::BUTTON_Y;
        let btn_w = 70.0;
        let btn_h = 25.0;
        let btn_spacing = 10.0;

        let buttons = [
            ("允许加入", GroupDialogAction::AllowJoinToggle),
            ("邀请", GroupDialogAction::Invite),
            ("退出", GroupDialogAction::Leave),
        ];

        let total_w = buttons.len() as f32 * (btn_w + btn_spacing) - btn_spacing;
        let start_x = self.position.x + (self.size.x - total_w) / 2.0;

        for (i, (label, action)) in buttons.iter().enumerate() {
            let btn_x = start_x + i as f32 * (btn_w + btn_spacing);
            let btn_rect = Rect::new(btn_x, btn_y, btn_w, btn_h);

            let is_hovered = btn_rect.contains(mouse_pos);
            let is_pressed = is_hovered && is_mouse_button_down(MouseButton::Left);

            // 按钮背景
            let btn_color = if is_pressed {
                Color::from_rgba(100, 120, 140, 255)
            } else if is_hovered {
                Color::from_rgba(80, 100, 120, 255)
            } else {
                Color::from_rgba(60, 70, 80, 255)
            };
            draw_rectangle(btn_x, btn_y, btn_w, btn_h, btn_color);
            draw_rectangle_lines(btn_x, btn_y, btn_w, btn_h, 1.0, Color::from_rgba(100, 100, 120, 255));

            // 按钮文字
            draw_text_cn(label, btn_x + 10.0, btn_y + 16.0, 12.0, WHITE);

            if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
                self.pending_action = action.clone();
            }
        }

        // 踢出按钮（仅当选中成员时显示）
        if let Some(selected) = self.selected_member {
            if let Some(member) = self.members.get(selected) {
                if !member.is_leader {
                    let kick_btn_x = start_x + buttons.len() as f32 * (btn_w + btn_spacing);
                    let kick_rect = Rect::new(kick_btn_x, btn_y, btn_w, btn_h);
                    let is_hovered = kick_rect.contains(mouse_pos);
                    let is_pressed = is_hovered && is_mouse_button_down(MouseButton::Left);

                    let kick_color = if is_pressed {
                        Color::from_rgba(180, 60, 60, 255)
                    } else if is_hovered {
                        Color::from_rgba(150, 50, 50, 255)
                    } else {
                        Color::from_rgba(120, 40, 40, 255)
                    };
                    draw_rectangle(kick_btn_x, btn_y, btn_w, btn_h, kick_color);
                    draw_rectangle_lines(kick_btn_x, btn_y, btn_w, btn_h, 1.0, Color::from_rgba(200, 80, 80, 255));

                    draw_text_cn("踢出", kick_btn_x + 15.0, btn_y + 16.0, 12.0, WHITE);

                    if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
                        self.pending_action = GroupDialogAction::KickSelected;
                    }
                }
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
