// ============================================================================
// FriendDialogHybrid - 好友对话框
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/FriendDialog.cs (569 行)
// - 背景：Title[199]
// - 标题：Title[6] at (18, 9)
// - 关闭按钮：Title[193/194/195] at (200, 256)
// - 好友列表：每行显示名称、在线状态、私聊按钮
// - 底部按钮：添加好友、删除好友、刷新列表
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::DragHelper;

/// 好友信息
#[derive(Debug, Clone)]
pub struct FriendInfo {
    pub object_id: u32,
    pub name: String,
    pub memo: String,
    pub online: bool,
}

/// 好友对话框动作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FriendDialogAction {
    None,
    AddFriend,
    RemoveSelected,
    RefreshList,
    PrivateChatSelected,
}

pub struct FriendDialogHybrid {
    position: Vec2,
    visible: bool,
    size: Vec2,
    friends: Vec<FriendInfo>,
    selected_friend: Option<usize>,
    scroll_offset: f32,
    bg_texture: Option<Texture2D>,
    title_texture: Option<Texture2D>,
    close_button_textures: [Option<Texture2D>; 3],
    drag_helper: DragHelper,
    pending_action: FriendDialogAction,
    /// 双击检测（替代 static mut）
    last_click_time: f64,
    last_click_idx: Option<usize>,
}

impl Default for FriendDialogHybrid {
    fn default() -> Self {
        Self::new()
    }
}

impl FriendDialogHybrid {
    const FRIEND_START_Y: f32 = 40.0;
    const FRIEND_ITEM_H: f32 = 20.0;
    const BUTTON_Y: f32 = 210.0;

    pub fn new() -> Self {
        Self {
            position: vec2(300.0, 100.0),
            visible: false,
            size: vec2(260.0, 290.0),
            friends: Vec::new(),
            selected_friend: None,
            scroll_offset: 0.0,
            bg_texture: None,
            title_texture: None,
            close_button_textures: [None, None, None],
            drag_helper: DragHelper::new(),
            pending_action: FriendDialogAction::None,
            last_click_time: 0.0,
            last_click_idx: None,
        }
    }

    pub fn open(&mut self) {
        self.visible = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.selected_friend = None;
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

    /// 更新好友列表
    pub fn update_friends(&mut self, friends: Vec<FriendInfo>) {
        self.friends = friends;
    }

    /// 添加好友
    pub fn add_friend(&mut self, friend: FriendInfo) {
        self.friends.push(friend);
    }

    /// 移除好友
    pub fn remove_friend(&mut self, name: &str) {
        self.friends.retain(|f| f.name != name);
    }

    /// 获取待处理动作
    pub fn take_action(&mut self) -> FriendDialogAction {
        std::mem::replace(&mut self.pending_action, FriendDialogAction::None)
    }

    /// 获取当前选中好友的名称
    pub fn get_selected_friend_name(&self) -> Option<String> {
        self.selected_friend
            .and_then(|i| self.friends.get(i))
            .map(|f| f.name.clone())
    }

    /// 获取当前选中好友的 object_id
    pub fn get_selected_friend_object_id(&self) -> Option<u32> {
        self.selected_friend
            .and_then(|i| self.friends.get(i))
            .map(|f| f.object_id)
    }

    /// 加载纹理
    pub fn load_textures(&mut self) {
        // 背景纹理 - Title[199]
        if let Some(texture) = LibraryName::Title.get_texture(199) {
            self.size = vec2(texture.width as f32, texture.height as f32);
            if let Some(tex) = texture.image {
                self.bg_texture = Some(tex);
            }
        }

        // 标题纹理 - Title[6]
        if let Some(texture) = LibraryName::Title.get_texture(6) {
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

        // 绘制好友列表
        self.draw_friend_list(mouse_pos);

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

        // 好友数量统计
        let online_count = self.friends.iter().filter(|f| f.online).count();
        let status_text = format!("好友: {}/{} 在线", online_count, self.friends.len());
        draw_text_cn(
            &status_text,
            self.position.x + 15.0,
            self.position.y + self.size.y - 25.0,
            12.0,
            WHITE,
        );
    }

    fn draw_friend_list(&mut self, mouse_pos: Vec2) {
        let list_x = self.position.x + 10.0;
        let list_w = self.size.x - 20.0;
        let list_top = self.position.y + Self::FRIEND_START_Y;

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
        let mut clicked_friend: Option<usize> = None;
        let mut double_clicked: Option<usize> = None;

        for (i, friend) in self.friends.iter().enumerate() {
            let item_rect = Rect::new(list_x + 5.0, y, list_w - 10.0, Self::FRIEND_ITEM_H);
            let item_visible = item_rect.y + item_rect.h > list_top && item_rect.y < list_bottom;

            if item_visible {
                let is_selected = self.selected_friend == Some(i);
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

                // 在线状态指示
                let status_color = if friend.online {
                    Color::from_rgba(80, 200, 80, 255)
                } else {
                    Color::from_rgba(150, 150, 150, 255)
                };
                draw_circle(list_x + 15.0, y + 10.0, 4.0, status_color);

                // 显示名称（优先显示备注）
                let display_name = if friend.memo.is_empty() {
                    &friend.name
                } else {
                    &friend.memo
                };
                let name_color = if friend.online { WHITE } else { GRAY };
                draw_text_cn(
                    display_name,
                    list_x + 25.0,
                    item_rect.y + 14.0,
                    12.0,
                    name_color,
                );

                // 私聊提示（在线好友）
                if friend.online && is_hovered {
                    draw_text_cn(
                        "[私聊]",
                        item_rect.x + item_rect.w - 50.0,
                        item_rect.y + 14.0,
                        10.0,
                        Color::from_rgba(100, 200, 255, 255),
                    );
                }

                // 点击检测
                if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
                    let now = get_time();
                    if self.last_click_idx == Some(i) && (now - self.last_click_time) < 0.3 {
                        double_clicked = Some(i);
                    }
                    self.last_click_time = now;
                    self.last_click_idx = Some(i);
                    clicked_friend = Some(i);
                }
            }

            y += Self::FRIEND_ITEM_H;
        }

        if let Some(idx) = clicked_friend {
            self.selected_friend = Some(idx);
        }

        if double_clicked.is_some() {
            // 双击打开私聊
            self.pending_action = FriendDialogAction::PrivateChatSelected;
        }

        // 限制滚动偏移
        let content_h = self.friends.len() as f32 * Self::FRIEND_ITEM_H;
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
            ("添加好友", FriendDialogAction::AddFriend),
            ("删除好友", FriendDialogAction::RemoveSelected),
            ("刷新列表", FriendDialogAction::RefreshList),
        ];

        let total_w = buttons.len() as f32 * (btn_w + btn_spacing) - btn_spacing;
        let start_x = self.position.x + (self.size.x - total_w) / 2.0;

        for (i, (label, action)) in buttons.iter().enumerate() {
            let btn_x = start_x + i as f32 * (btn_w + btn_spacing);
            let btn_rect = Rect::new(btn_x, btn_y, btn_w, btn_h);

            let is_hovered = btn_rect.contains(mouse_pos);
            let is_pressed = is_hovered && is_mouse_button_down(MouseButton::Left);

            // 删除按钮特殊处理（仅当选中好友时可用）
            let is_remove = matches!(action, FriendDialogAction::RemoveSelected);
            let is_disabled = is_remove && self.selected_friend.is_none();

            let btn_color = if is_disabled {
                Color::from_rgba(40, 40, 50, 255)
            } else if is_pressed {
                Color::from_rgba(100, 120, 140, 255)
            } else if is_hovered {
                Color::from_rgba(80, 100, 120, 255)
            } else {
                Color::from_rgba(60, 70, 80, 255)
            };
            draw_rectangle(btn_x, btn_y, btn_w, btn_h, btn_color);
            draw_rectangle_lines(btn_x, btn_y, btn_w, btn_h, 1.0, Color::from_rgba(100, 100, 120, 255));

            let text_color = if is_disabled { GRAY } else { WHITE };
            draw_text_cn(label, btn_x + 10.0, btn_y + 16.0, 12.0, text_color);

            if is_hovered && is_mouse_button_pressed(MouseButton::Left) && !is_disabled {
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
