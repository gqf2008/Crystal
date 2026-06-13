// ============================================================================
// GuildDialogHybrid - 行会对话框
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/GuildDialog.cs (2,232 行)
// - 背景：Prguse[956]
// - 标题：Title[15] at (18, 9)
// - 关闭按钮：Title[193/194/195] at (200, 256)
// - 多标签页：行会信息、成员列表、行会公告、行会仓库
// - 按钮：退出行会、编辑公告、管理成员
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::DragHelper;

/// 行会成员
#[derive(Debug, Clone)]
pub struct GuildMember {
    pub name: String,
    pub rank: String,
    pub online: bool,
}

/// 行会信息
#[derive(Debug, Clone, Default)]
pub struct GuildInfo {
    pub name: String,
    pub rank_name: String,
    pub level: u8,
    pub experience: i64,
    pub max_experience: i64,
    pub gold: u32,
    pub spare_points: u8,
    pub notice: String,
    pub members: Vec<GuildMember>,
    pub member_count: u32,
    pub max_members: u32,
    pub storage_gold: u32,
    pub storage_items: Vec<GuildStorageItem>,
    pub my_rank_id: i32,
}

/// 行会仓库物品
#[derive(Debug, Clone)]
pub struct GuildStorageItem {
    pub name: String,
    pub quantity: i32,
    pub slot: i32,
}

/// 标签页
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildTab {
    Info,
    Members,
    Notice,
    Storage,
    War,
}

/// 行会对话框动作
#[derive(Debug, Clone, PartialEq)]
pub enum GuildDialogAction {
    None,
    LeaveGuild,
    EditNotice(String),
    EditMemberRank { name: String, rank: String },
    RequestGuildInfo,
    ViewMemberDetail { name: String, rank: String, online: bool },
    RequestGuildWar,
}

pub struct GuildDialogHybrid {
    position: Vec2,
    visible: bool,
    size: Vec2,
    guild_info: GuildInfo,
    active_tab: GuildTab,
    selected_member: Option<usize>,
    scroll_offset: f32,
    bg_texture: Option<Texture2D>,
    title_texture: Option<Texture2D>,
    close_button_textures: [Option<Texture2D>; 3],
    drag_helper: DragHelper,
    pending_action: GuildDialogAction,
    /// 双击检测（替代 static mut）
    last_click_time: f64,
    last_click_idx: Option<usize>,
}

impl Default for GuildDialogHybrid {
    fn default() -> Self {
        Self::new()
    }
}

impl GuildDialogHybrid {
    const TAB_Y: f32 = 35.0;
    const CONTENT_START_Y: f32 = 60.0;
    const ITEM_H: f32 = 20.0;
    const BUTTON_Y: f32 = 210.0;

    pub fn new() -> Self {
        Self {
            position: vec2(280.0, 80.0),
            visible: false,
            size: vec2(320.0, 290.0),
            guild_info: GuildInfo::default(),
            active_tab: GuildTab::Info,
            selected_member: None,
            scroll_offset: 0.0,
            bg_texture: None,
            title_texture: None,
            close_button_textures: [None, None, None],
            drag_helper: DragHelper::new(),
            pending_action: GuildDialogAction::None,
            last_click_time: 0.0,
            last_click_idx: None,
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

    /// 更新行会信息
    pub fn update_guild_info(&mut self, info: GuildInfo) {
        // PR #1147: 实时刷新 member_count,避免依赖服务器发的 cache 值
        // (master C# 也在 NewMembersList 时重算,而不是用增量)
        self.guild_info = info;
        self.guild_info.member_count = self.count_members(&self.guild_info.members);
    }

    /// PR #1147: 计算实际成员数 (主 + 在线 + 离线都算)
    fn count_members(&self, members: &[GuildMember]) -> u32 {
        members.len() as u32
    }

    /// 更新行会公告
    pub fn update_notice(&mut self, notice: String) {
        self.guild_info.notice = notice;
    }

    /// 更新成员
    pub fn update_member(&mut self, name: String, rank: String, online: bool) {
        if let Some(m) = self.guild_info.members.iter_mut().find(|m| m.name == name) {
            m.rank = rank;
            m.online = online;
        } else {
            self.guild_info.members.push(GuildMember { name, rank, online });
        }
        // PR #1147: 增量更新时也同步 count (added/removed/updated)
        self.guild_info.member_count = self.count_members(&self.guild_info.members);
    }

    /// 更新行会仓库金币
    pub fn update_storage_gold(&mut self, gold: u32) {
        self.guild_info.storage_gold = gold;
    }

    /// 更新行会仓库物品
    pub fn update_storage_item(&mut self, name: String, quantity: i32, slot: i32) {
        if let Some(item) = self.guild_info.storage_items.iter_mut().find(|i| i.slot == slot) {
            item.name = name;
            item.quantity = quantity;
        } else {
            self.guild_info.storage_items.push(GuildStorageItem { name, quantity, slot });
        }
    }

    /// 清空仓库物品列表
    pub fn clear_storage_items(&mut self) {
        self.guild_info.storage_items.clear();
    }

    /// 获取待处理动作
    pub fn take_action(&mut self) -> GuildDialogAction {
        std::mem::replace(&mut self.pending_action, GuildDialogAction::None)
    }

    /// 获取当前选中成员的名称
    pub fn get_selected_member_name(&self) -> Option<String> {
        self.selected_member
            .and_then(|i| self.guild_info.members.get(i))
            .map(|m| m.name.clone())
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

        // 标题纹理 - Title[15]
        if let Some(texture) = LibraryName::Title.get_texture(15) {
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
            GuildTab::Info => self.draw_info_tab(),
            GuildTab::Members => self.draw_members_list(mouse_pos),
            GuildTab::Notice => self.draw_notice_tab(),
            GuildTab::Storage => self.draw_storage_tab(),
            GuildTab::War => self.draw_war_tab(mouse_pos),
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

        let tabs = ["行会信息", "成员列表", "行会公告", "行会仓库", "行会战"];
        let tab_kinds = [GuildTab::Info, GuildTab::Members, GuildTab::Notice, GuildTab::Storage, GuildTab::War];

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
            }
        }
    }

    fn draw_info_tab(&self) {
        let content_y = self.position.y + Self::CONTENT_START_Y;
        let content_x = self.position.x + 15.0;
        let line_h = 22.0;

        draw_text_cn("行会名称", content_x, content_y, 12.0, Color::from_rgba(200, 200, 200, 255));
        draw_text_cn(&self.guild_info.name, content_x + 80.0, content_y, 12.0, WHITE);

        let y2 = content_y + line_h;
        draw_text_cn("成员数量", content_x, y2, 12.0, Color::from_rgba(200, 200, 200, 255));
        let count_text = format!("{}/{}", self.guild_info.member_count, self.guild_info.max_members);
        draw_text_cn(&count_text, content_x + 80.0, y2, 12.0, WHITE);
    }

    fn draw_members_list(&mut self, mouse_pos: Vec2) {
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

        for (i, member) in self.guild_info.members.iter().enumerate() {
            let item_rect = Rect::new(list_x + 5.0, y, list_w - 10.0, Self::ITEM_H);
            let item_visible = item_rect.y + item_rect.h > list_top && item_rect.y < list_bottom;

            if item_visible {
                let is_selected = self.selected_member == Some(i);
                let is_hovered = item_rect.contains(mouse_pos);

                if is_selected || is_hovered {
                    let color = if is_selected {
                        Color::from_rgba(60, 80, 100, 150)
                    } else {
                        Color::from_rgba(50, 50, 60, 100)
                    };
                    draw_rectangle(item_rect.x, item_rect.y, item_rect.w, item_rect.h, color);
                }

                // 在线状态
                let status_color = if member.online {
                    Color::from_rgba(80, 200, 80, 255)
                } else {
                    Color::from_rgba(150, 150, 150, 255)
                };
                draw_circle(list_x + 15.0, y + 10.0, 4.0, status_color);

                // 名称
                let name_color = if member.online { WHITE } else { GRAY };
                draw_text_cn(&member.name, list_x + 25.0, item_rect.y + 14.0, 12.0, name_color);

                // 职位
                draw_text_cn(&member.rank, list_x + 150.0, item_rect.y + 14.0, 11.0, Color::from_rgba(200, 180, 100, 255));

                if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
                    let now = get_time();
                    if self.last_click_idx == Some(i) && (now - self.last_click_time) < 0.3 {
                        if let Some(m) = self.guild_info.members.get(i) {
                            self.pending_action = GuildDialogAction::ViewMemberDetail {
                                name: m.name.clone(),
                                rank: m.rank.clone(),
                                online: m.online,
                            };
                        }
                    }
                    self.last_click_time = now;
                    self.last_click_idx = Some(i);
                    clicked = Some(i);
                }
            }

            y += Self::ITEM_H;
        }

        if let Some(idx) = clicked {
            self.selected_member = Some(idx);
        }

        // 限制滚动
        let content_h = self.guild_info.members.len() as f32 * Self::ITEM_H;
        let max_scroll = (content_h - list_h).max(0.0);
        if self.scroll_offset > max_scroll {
            self.scroll_offset = max_scroll;
        }
    }

    fn draw_notice_tab(&self) {
        let content_y = self.position.y + Self::CONTENT_START_Y;
        let content_x = self.position.x + 15.0;
        let _line_w = self.size.x - 30.0;
        let line_h = 16.0;

        if self.guild_info.notice.is_empty() {
            draw_text_cn("暂无行会公告", content_x, content_y + 20.0, 12.0, GRAY);
            return;
        }

        // 简单多行文本渲染（按换行符分割）
        for (i, line) in self.guild_info.notice.lines().enumerate() {
            let y = content_y + i as f32 * line_h;
            let max_lines = ((self.position.y + Self::BUTTON_Y - 10.0 - content_y) / line_h) as usize;
            if i >= max_lines {
                break;
            }
            draw_text_cn(line, content_x, y, 11.0, WHITE);
        }
    }

    fn draw_storage_tab(&self) {
        let content_y = self.position.y + Self::CONTENT_START_Y;
        let content_x = self.position.x + 15.0;
        let line_h = 22.0;

        // 行会金币
        draw_text_cn("行会金币:", content_x, content_y, 12.0, Color::from_rgba(200, 200, 200, 255));
        draw_text_cn(&self.guild_info.storage_gold.to_string(), content_x + 80.0, content_y, 12.0, Color::from_rgba(255, 215, 0, 255));

        // 仓库物品列表
        let list_top = content_y + line_h + 5.0;
        let list_bottom = self.position.y + Self::BUTTON_Y - 10.0;

        draw_text_cn("仓库物品", content_x, list_top, 12.0, Color::from_rgba(200, 200, 200, 255));

        if self.guild_info.storage_items.is_empty() {
            draw_text_cn("暂无物品", content_x + 10.0, list_top + 25.0, 12.0, GRAY);
            return;
        }

        for (i, item) in self.guild_info.storage_items.iter().enumerate() {
            let y = list_top + 25.0 + i as f32 * (Self::ITEM_H + 2.0);
            if y > list_bottom {
                break;
            }
            let name_text = format!("{} x{}", item.name, item.quantity);
            draw_text_cn(&name_text, content_x + 10.0, y, 11.0, WHITE);
        }
    }

    fn draw_war_tab(&mut self, mouse_pos: Vec2) {
        let content_y = self.position.y + Self::CONTENT_START_Y;
        let content_x = self.position.x + 15.0;
        let line_h = 22.0;

        draw_text_cn("行会战设置", content_x, content_y, 13.0, Color::from_rgba(255, 200, 100, 255));

        let info_y = content_y + line_h;
        draw_text_cn("向其他行会发起战争", content_x, info_y, 11.0, GRAY);

        // 请求行会战按钮
        let btn_w = 120.0;
        let btn_h = 28.0;
        let btn_x = content_x + (self.size.x - 30.0 - btn_w) / 2.0;
        let btn_y = info_y + 40.0;
        let btn_rect = Rect::new(btn_x, btn_y, btn_w, btn_h);

        let is_hovered = btn_rect.contains(mouse_pos);
        let is_pressed = is_hovered && is_mouse_button_down(MouseButton::Left);

        let btn_color = if is_pressed {
            Color::from_rgba(180, 60, 60, 255)
        } else if is_hovered {
            Color::from_rgba(160, 50, 50, 255)
        } else {
            Color::from_rgba(140, 40, 40, 255)
        };
        draw_rectangle(btn_x, btn_y, btn_w, btn_h, btn_color);
        draw_rectangle_lines(btn_x, btn_y, btn_w, btn_h, 1.0, Color::from_rgba(200, 100, 100, 255));
        draw_text_cn("请求行会战", btn_x + 15.0, btn_y + 18.0, 12.0, WHITE);

        if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
            self.pending_action = GuildDialogAction::RequestGuildWar;
        }
    }

    fn draw_buttons(&mut self, mouse_pos: Vec2) {
        let btn_y = self.position.y + Self::BUTTON_Y;
        let btn_w = 80.0;
        let btn_h = 25.0;
        let btn_spacing = 10.0;

        let total_w = 3.0 * (btn_w + btn_spacing) - btn_spacing;
        let start_x = self.position.x + (self.size.x - total_w) / 2.0;

        let buttons: [(&str, GuildDialogAction); 3] = [
            ("退出行会", GuildDialogAction::LeaveGuild),
            ("刷新信息", GuildDialogAction::RequestGuildInfo),
            ("编辑公告", GuildDialogAction::EditNotice(String::new())),
        ];

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
