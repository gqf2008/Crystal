// Guild Dialog - 公会面板
// C# reference: Client/MirScenes/Dialogs/GuildDialog.cs

use macroquad::prelude::*;
use super::native_ui_utils::*;

/// Guild rank options (bitflags-style)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GuildRank {
    Leader,
    Officer,
    Member,
}

/// Guild member info
#[derive(Debug, Clone)]
pub struct GuildMember {
    pub name: String,
    pub rank: GuildRank,
    pub online: bool,
    pub level: u16,
    pub class_id: u8,
}

/// Guild buff info
#[derive(Debug, Clone)]
pub struct GuildBuff {
    pub id: u32,
    pub name: String,
    pub icon_index: i32,
    pub active: bool,
    pub info: String,
}

/// Guild tab pages
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GuildTab {
    Notice,
    Members,
    Storage,
    Rank,
    Buffs,
    Status,
}

/// Actions the guild dialog can produce
#[derive(Debug, Clone)]
pub enum GuildAction {
    Close,
    InviteMember(String),
    KickMember(String),
    PromoteMember { name: String, new_rank: GuildRank },
    UpdateNotice(String),
    SwitchTab(GuildTab),
    DonateGold(u64),
}

pub struct GuildDialogHybrid {
    pub visible: bool,
    pub tab: GuildTab,
    pub guild_name: String,
    pub guild_level: u16,
    pub experience: u64,
    pub max_experience: u64,
    pub gold: u64,
    pub members: Vec<GuildMember>,
    pub notice: String,
    pub buffs: Vec<GuildBuff>,
    pub my_rank: GuildRank,
    member_scroll_index: usize,
    selected_member: Option<usize>,
    position: Vec2,
    bg_texture: BackgroundTexture,
    close_btn: CloseButton,
    drag_helper: DragHelper,
}

const GUILD_WIDTH: f32 = 600.0;
const GUILD_HEIGHT: f32 = 400.0;
const MEMBERS_PER_PAGE: usize = 18;

impl GuildDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            tab: GuildTab::Notice,
            guild_name: String::new(),
            guild_level: 0,
            experience: 0,
            max_experience: 0,
            gold: 0,
            members: Vec::new(),
            notice: String::new(),
            buffs: Vec::new(),
            my_rank: GuildRank::Member,
            member_scroll_index: 0,
            selected_member: None,
            position: vec2(100.0, 60.0),
            bg_texture: BackgroundTexture::empty(),
            close_btn: CloseButton::empty(),
            drag_helper: DragHelper::new(),
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn is_leader(&self) -> bool {
        self.my_rank == GuildRank::Leader
    }

    pub fn is_officer_or_above(&self) -> bool {
        matches!(self.my_rank, GuildRank::Leader | GuildRank::Officer)
    }

    pub fn online_count(&self) -> usize {
        self.members.iter().filter(|m| m.online).count()
    }

    pub fn visible_members(&self) -> &[GuildMember] {
        let start = self.member_scroll_index;
        let end = (start + MEMBERS_PER_PAGE).min(self.members.len());
        if start < self.members.len() {
            &self.members[start..end]
        } else {
            &[]
        }
    }

    pub fn draw(&mut self) -> Option<GuildAction> {
        if !self.visible {
            return None;
        }

        let pos = self.position;
        let rect = Rect::new(pos.x, pos.y, GUILD_WIDTH, GUILD_HEIGHT);
        let mouse = mouse_position();
        let mouse_pos = vec2(mouse.0, mouse.1);
        let mut action = None;

        // Background
        draw_rectangle(pos.x, pos.y, GUILD_WIDTH, GUILD_HEIGHT, Color::new(0.1, 0.1, 0.15, 0.95));
        draw_rectangle_lines(pos.x, pos.y, GUILD_WIDTH, GUILD_HEIGHT, 2.0, GRAY);

        // Title bar
        let title = format!("公会 - {} (Lv.{})", self.guild_name, self.guild_level);
        draw_text(&title, pos.x + 18.0, pos.y + 22.0, 16.0, WHITE);

        // Close button
        let close_rect = Rect::new(pos.x + GUILD_WIDTH - 30.0, pos.y + 4.0, 24.0, 24.0);
        draw_text("✕", close_rect.x + 6.0, close_rect.y + 17.0, 16.0, WHITE);
        if is_mouse_button_pressed(MouseButton::Left) && close_rect.contains(mouse_pos) {
            self.visible = false;
            return Some(GuildAction::Close);
        }

        // Tab buttons
        let tabs = [
            (GuildTab::Notice, "公告"),
            (GuildTab::Members, "成员"),
            (GuildTab::Storage, "仓库"),
            (GuildTab::Rank, "职位"),
            (GuildTab::Buffs, "增益"),
            (GuildTab::Status, "状态"),
        ];

        let tab_y = pos.y + 35.0;
        for (i, (tab, label)) in tabs.iter().enumerate() {
            let tx = pos.x + 10.0 + (i as f32) * 95.0;
            let tab_rect = Rect::new(tx, tab_y, 88.0, 22.0);
            let color = if self.tab == *tab { Color::new(0.3, 0.4, 0.6, 1.0) } else { Color::new(0.2, 0.2, 0.3, 1.0) };
            draw_rectangle(tx, tab_y, 88.0, 22.0, color);
            draw_text(label, tx + 30.0, tab_y + 16.0, 14.0, WHITE);

            if is_mouse_button_pressed(MouseButton::Left) && tab_rect.contains(mouse_pos) {
                self.tab = *tab;
                action = Some(GuildAction::SwitchTab(*tab));
            }
        }

        // Content area
        let content_y = tab_y + 30.0;
        let content_h = GUILD_HEIGHT - 70.0;

        match self.tab {
            GuildTab::Notice => {
                draw_text("公会公告:", pos.x + 15.0, content_y + 20.0, 14.0, YELLOW);
                // Draw notice text (scrollable area)
                let lines: Vec<&str> = self.notice.lines().collect();
                for (i, line) in lines.iter().enumerate().take(20) {
                    let ly = content_y + 40.0 + (i as f32) * 16.0;
                    if ly < pos.y + GUILD_HEIGHT - 10.0 {
                        draw_text(line, pos.x + 15.0, ly, 13.0, LIGHTGRAY);
                    }
                }
            }
            GuildTab::Members => {
                // Header
                draw_text("名称", pos.x + 15.0, content_y + 16.0, 13.0, YELLOW);
                draw_text("职位", pos.x + 200.0, content_y + 16.0, 13.0, YELLOW);
                draw_text("等级", pos.x + 320.0, content_y + 16.0, 13.0, YELLOW);
                draw_text("状态", pos.x + 420.0, content_y + 16.0, 13.0, YELLOW);

                let info = format!("在线: {}/{}", self.online_count(), self.members.len());
                draw_text(&info, pos.x + 480.0, content_y + 16.0, 13.0, GRAY);

                // Member rows
                let visible = self.visible_members();
                for (i, member) in visible.iter().enumerate() {
                    let ry = content_y + 24.0 + ((i + 1) as f32) * 19.0;
                    let row_rect = Rect::new(pos.x + 10.0, ry - 14.0, GUILD_WIDTH - 20.0, 18.0);

                    // Selection highlight
                    if Some(self.member_scroll_index + i) == self.selected_member {
                        draw_rectangle(row_rect.x, row_rect.y, row_rect.w, row_rect.h,
                            Color::new(0.3, 0.3, 0.5, 0.5));
                    }

                    let name_color = if member.online { GREEN } else { GRAY };
                    draw_text(&member.name, pos.x + 15.0, ry, 13.0, name_color);

                    let rank_str = match member.rank {
                        GuildRank::Leader => "会长",
                        GuildRank::Officer => "长老",
                        GuildRank::Member => "成员",
                    };
                    draw_text(rank_str, pos.x + 200.0, ry, 13.0, LIGHTGRAY);
                    draw_text(&format!("{}", member.level), pos.x + 320.0, ry, 13.0, LIGHTGRAY);

                    let status = if member.online { "在线" } else { "离线" };
                    let status_color = if member.online { GREEN } else { DARKGRAY };
                    draw_text(status, pos.x + 420.0, ry, 13.0, status_color);

                    if is_mouse_button_pressed(MouseButton::Left) && row_rect.contains(mouse_pos) {
                        self.selected_member = Some(self.member_scroll_index + i);
                    }
                }

                // Action buttons (only for leader/officer)
                if self.is_officer_or_above() {
                    let btn_y = pos.y + GUILD_HEIGHT - 35.0;
                    let invite_rect = Rect::new(pos.x + 15.0, btn_y, 80.0, 24.0);
                    draw_rectangle(invite_rect.x, invite_rect.y, invite_rect.w, invite_rect.h,
                        Color::new(0.2, 0.4, 0.2, 1.0));
                    draw_text("邀请", invite_rect.x + 22.0, invite_rect.y + 17.0, 14.0, WHITE);

                    if self.is_leader() {
                        let kick_rect = Rect::new(pos.x + 105.0, btn_y, 80.0, 24.0);
                        draw_rectangle(kick_rect.x, kick_rect.y, kick_rect.w, kick_rect.h,
                            Color::new(0.4, 0.2, 0.2, 1.0));
                        draw_text("踢出", kick_rect.x + 22.0, kick_rect.y + 17.0, 14.0, WHITE);

                        if is_mouse_button_pressed(MouseButton::Left) && kick_rect.contains(mouse_pos) {
                            if let Some(idx) = self.selected_member {
                                if let Some(member) = self.members.get(idx) {
                                    action = Some(GuildAction::KickMember(member.name.clone()));
                                }
                            }
                        }
                    }
                }

                // Scroll buttons
                if self.members.len() > MEMBERS_PER_PAGE {
                    let up_rect = Rect::new(pos.x + GUILD_WIDTH - 30.0, content_y + 20.0, 20.0, 20.0);
                    let down_rect = Rect::new(pos.x + GUILD_WIDTH - 30.0, pos.y + GUILD_HEIGHT - 60.0, 20.0, 20.0);
                    draw_text("▲", up_rect.x + 2.0, up_rect.y + 15.0, 14.0, WHITE);
                    draw_text("▼", down_rect.x + 2.0, down_rect.y + 15.0, 14.0, WHITE);

                    if is_mouse_button_pressed(MouseButton::Left) {
                        if up_rect.contains(mouse_pos) && self.member_scroll_index > 0 {
                            self.member_scroll_index = self.member_scroll_index.saturating_sub(1);
                        }
                        if down_rect.contains(mouse_pos) && self.member_scroll_index + MEMBERS_PER_PAGE < self.members.len() {
                            self.member_scroll_index += 1;
                        }
                    }
                }
            }
            GuildTab::Storage => {
                draw_text("公会仓库", pos.x + 15.0, content_y + 20.0, 14.0, YELLOW);
                draw_text(&format!("公会资金: {} 金", self.gold), pos.x + 15.0, content_y + 40.0, 13.0, Color::new(1.0, 0.84, 0.0, 1.0));
                draw_text("(仓库物品显示区域)", pos.x + 15.0, content_y + 70.0, 13.0, GRAY);
            }
            GuildTab::Rank => {
                draw_text("职位管理", pos.x + 15.0, content_y + 20.0, 14.0, YELLOW);
                if !self.is_leader() {
                    draw_text("仅会长可管理职位", pos.x + 15.0, content_y + 50.0, 13.0, GRAY);
                }
            }
            GuildTab::Buffs => {
                draw_text("公会增益", pos.x + 15.0, content_y + 20.0, 14.0, YELLOW);
                for (i, buff) in self.buffs.iter().enumerate() {
                    let by = content_y + 40.0 + (i as f32) * 35.0;
                    let status_color = if buff.active { GREEN } else { GRAY };
                    draw_text(&buff.name, pos.x + 50.0, by + 14.0, 13.0, status_color);
                    draw_text(&buff.info, pos.x + 200.0, by + 14.0, 12.0, LIGHTGRAY);
                }
            }
            GuildTab::Status => {
                draw_text("公会信息", pos.x + 15.0, content_y + 20.0, 14.0, YELLOW);
                let status_lines = [
                    format!("等级: {}", self.guild_level),
                    format!("经验: {} / {}", self.experience, self.max_experience),
                    format!("成员: {} / 100", self.members.len()),
                    format!("资金: {} 金", self.gold),
                ];
                for (i, line) in status_lines.iter().enumerate() {
                    draw_text(line, pos.x + 15.0, content_y + 45.0 + (i as f32) * 20.0, 13.0, LIGHTGRAY);
                }
            }
        }

        // Dragging
        if let Some(new_pos) = self.drag_helper.update(rect, mouse_pos) {
            self.position = new_pos;
        }

        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guild_dialog_new() {
        let dialog = GuildDialogHybrid::new();
        assert!(!dialog.visible);
        assert_eq!(dialog.tab, GuildTab::Notice);
        assert!(dialog.members.is_empty());
        assert!(dialog.guild_name.is_empty());
    }

    #[test]
    fn test_guild_toggle() {
        let mut dialog = GuildDialogHybrid::new();
        assert!(!dialog.visible);
        dialog.toggle();
        assert!(dialog.visible);
        dialog.toggle();
        assert!(!dialog.visible);
    }

    #[test]
    fn test_guild_permissions() {
        let mut dialog = GuildDialogHybrid::new();
        dialog.my_rank = GuildRank::Member;
        assert!(!dialog.is_leader());
        assert!(!dialog.is_officer_or_above());

        dialog.my_rank = GuildRank::Officer;
        assert!(!dialog.is_leader());
        assert!(dialog.is_officer_or_above());

        dialog.my_rank = GuildRank::Leader;
        assert!(dialog.is_leader());
        assert!(dialog.is_officer_or_above());
    }

    #[test]
    fn test_guild_online_count() {
        let mut dialog = GuildDialogHybrid::new();
        dialog.members = vec![
            GuildMember { name: "Player1".into(), rank: GuildRank::Leader, online: true, level: 50, class_id: 0 },
            GuildMember { name: "Player2".into(), rank: GuildRank::Member, online: false, level: 30, class_id: 1 },
            GuildMember { name: "Player3".into(), rank: GuildRank::Officer, online: true, level: 45, class_id: 2 },
        ];
        assert_eq!(dialog.online_count(), 2);
    }

    #[test]
    fn test_guild_visible_members() {
        let mut dialog = GuildDialogHybrid::new();
        for i in 0..25 {
            dialog.members.push(GuildMember {
                name: format!("Player{}", i),
                rank: GuildRank::Member,
                online: i % 2 == 0,
                level: 10 + i as u16,
                class_id: 0,
            });
        }
        assert_eq!(dialog.visible_members().len(), MEMBERS_PER_PAGE);
        dialog.member_scroll_index = 20;
        assert_eq!(dialog.visible_members().len(), 5);
    }
}
