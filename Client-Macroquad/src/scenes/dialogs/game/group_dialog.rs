// ============================================================================
// GroupDialogHybrid - 组队面板（对齐 C# GroupDialog）
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/GroupDialog.cs (~227 行)
// - 背景：Prguse[120]
// - 成员列表：最多 MaxGroup 人（通常 10 人）
// - 2 列布局：(16, 33) 起始，列间距约 120px，行间距 20px
// - 操作按钮：邀请/踢出/切换允许组队
// - 仅队长可邀请/踢出
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::*;

// ============================================================================
// 常量
// ============================================================================

/// 最大组队人数
const MAX_GROUP: usize = 10;
/// 成员列表起始位置
const LIST_X: f32 = 16.0;
const LIST_Y: f32 = 33.0;
/// 行高
const ROW_HEIGHT: f32 = 20.0;
/// 列宽
const COL_WIDTH: f32 = 120.0;
/// 窗口尺寸
const DIALOG_WIDTH: f32 = 260.0;
const DIALOG_HEIGHT: f32 = 180.0;

// ============================================================================
// 类型定义
// ============================================================================

/// 组队成员
#[derive(Debug, Clone)]
pub struct GroupMember {
    pub name: String,
    /// 附加信息（等级、职业等）
    pub info: String,
}

impl GroupMember {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            info: String::new(),
        }
    }
}

/// 组队动作
#[derive(Debug, Clone, PartialEq)]
pub enum GroupAction {
    /// 邀请成员（需要后续输入名字）
    InviteMember,
    /// 踢出成员（需要后续输入名字）
    KickMember,
    /// 切换允许组队
    ToggleAllowGroup,
    /// 关闭
    Close,
}

/// 组队面板
pub struct GroupDialogHybrid {
    pub visible: bool,
    pub members: Vec<GroupMember>,
    /// 是否为队长
    pub is_leader: bool,
    /// 是否允许他人邀请
    pub allow_group: bool,
    position: Vec2,
    // UI
    bg_texture: BackgroundTexture,
    close_btn: CloseButton,
    drag_helper: DragHelper,
}

impl GroupDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            members: Vec::new(),
            is_leader: false,
            allow_group: true,
            position: Vec2::new(400.0, 150.0),
            bg_texture: BackgroundTexture::new(),
            close_btn: CloseButton::new(),
            drag_helper: DragHelper::new(),
        }
    }

    pub fn load_textures(&mut self) {
        self.bg_texture = BackgroundTexture::load(LibraryName::Prguse, 120, None);
        self.close_btn = CloseButton::load_prguse2();
    }

    /// 设置组队成员（超出上限截断）
    pub fn set_members(&mut self, members: Vec<GroupMember>) {
        self.members = if members.len() > MAX_GROUP {
            members.into_iter().take(MAX_GROUP).collect()
        } else {
            members
        };
    }

    /// 清除组队
    pub fn clear(&mut self) {
        self.members.clear();
        self.is_leader = false;
    }

    /// 是否在队伍中
    pub fn in_group(&self) -> bool {
        !self.members.is_empty()
    }

    /// 绘制并处理输入
    pub fn draw(&mut self) -> Option<GroupAction> {
        if !self.visible {
            return None;
        }

        let mouse = mouse_pos();
        let mut action = None;

        // --- 拖动 ---
        let title_rect = Rect::new(self.position.x, self.position.y, DIALOG_WIDTH, 22.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        // --- 背景 ---
        self.bg_texture.draw(vec2(x, y));

        // --- 标题 ---
        let title = if self.in_group() {
            format!("组队 ({}/{})", self.members.len(), MAX_GROUP)
        } else {
            "组队".to_string()
        };
        draw_text_cn(&title, x + 90.0, y + 6.0, 13.0, GOLD);

        // --- 成员列表 ---
        if self.members.is_empty() {
            draw_text_cn("未加入队伍", x + 80.0, y + 80.0, 12.0, GRAY);
        } else {
            for (i, member) in self.members.iter().enumerate().take(MAX_GROUP) {
                let col = i / 5;
                let row = i % 5;
                let mx = x + LIST_X + col as f32 * COL_WIDTH;
                let my = y + LIST_Y + row as f32 * ROW_HEIGHT;

                // 队长标记
                let name_text = if i == 0 {
                    format!("★ {}", member.name)
                } else {
                    member.name.clone()
                };

                let color = if i == 0 { GOLD } else { WHITE };
                draw_text_cn(&name_text, mx, my, 11.0, color);

                // 悬停提示
                let row_rect = Rect::new(mx, my, COL_WIDTH - 4.0, ROW_HEIGHT);
                if row_rect.contains(mouse) && !member.info.is_empty() {
                    draw_text_cn(&member.info, mouse.x + 10.0, mouse.y - 10.0, 11.0, LIGHTGRAY);
                }
            }
        }

        // --- 操作按钮 ---
        let btn_y = y + DIALOG_HEIGHT - 32.0;

        // 允许组队切换
        let switch_rect = Rect::new(x + 10.0, btn_y, 70.0, 20.0);
        let switch_label = if self.allow_group { "允许组队" } else { "禁止组队" };
        let switch_color = if self.allow_group { LIME } else { RED };
        let sw_state = ButtonState::from_mouse(switch_rect, mouse);
        let sw_text_color = if sw_state == ButtonState::Hover { WHITE } else { GRAY };
        draw_rectangle_lines(switch_rect.x, switch_rect.y, switch_rect.w, switch_rect.h, 1.0, switch_color);
        draw_text_cn(switch_label, switch_rect.x + 6.0, switch_rect.y + 3.0, 11.0, sw_text_color);
        if ButtonState::is_clicked(switch_rect, mouse) {
            self.allow_group = !self.allow_group;
            action = Some(GroupAction::ToggleAllowGroup);
        }

        // 邀请按钮（仅队长或无队伍时可用）
        if self.is_leader || !self.in_group() {
            let add_rect = Rect::new(x + 90.0, btn_y, 55.0, 20.0);
            let add_state = ButtonState::from_mouse(add_rect, mouse);
            let add_color = if add_state == ButtonState::Hover { WHITE } else { GRAY };
            draw_rectangle_lines(add_rect.x, add_rect.y, add_rect.w, add_rect.h, 1.0, Color::new(0.4, 0.4, 0.4, 0.6));
            draw_text_cn("邀请", add_rect.x + 12.0, add_rect.y + 3.0, 11.0, add_color);
            if ButtonState::is_clicked(add_rect, mouse) {
                action = Some(GroupAction::InviteMember);
            }
        }

        // 踢出按钮（仅队长可用）
        if self.is_leader && self.in_group() {
            let del_rect = Rect::new(x + 155.0, btn_y, 55.0, 20.0);
            let del_state = ButtonState::from_mouse(del_rect, mouse);
            let del_color = if del_state == ButtonState::Hover { WHITE } else { GRAY };
            draw_rectangle_lines(del_rect.x, del_rect.y, del_rect.w, del_rect.h, 1.0, Color::new(0.4, 0.4, 0.4, 0.6));
            draw_text_cn("踢出", del_rect.x + 12.0, del_rect.y + 3.0, 11.0, del_color);
            if ButtonState::is_clicked(del_rect, mouse) {
                action = Some(GroupAction::KickMember);
            }
        }

        // --- 关闭按钮 ---
        let win_size = vec2(DIALOG_WIDTH, DIALOG_HEIGHT);
        if self.close_btn.draw(self.position, win_size, mouse) {
            self.visible = false;
            action = Some(GroupAction::Close);
        }

        action
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_dialog_creation() {
        let dialog = GroupDialogHybrid::new();
        assert!(!dialog.visible);
        assert!(dialog.members.is_empty());
        assert!(!dialog.is_leader);
        assert!(dialog.allow_group);
        assert!(!dialog.in_group());
    }

    #[test]
    fn test_set_members() {
        let mut dialog = GroupDialogHybrid::new();
        let members = vec![
            GroupMember::new("Leader"),
            GroupMember::new("Player2"),
            GroupMember::new("Player3"),
        ];
        dialog.set_members(members);
        assert!(dialog.in_group());
        assert_eq!(dialog.members.len(), 3);
    }

    #[test]
    fn test_clear_group() {
        let mut dialog = GroupDialogHybrid::new();
        dialog.set_members(vec![GroupMember::new("Test")]);
        dialog.is_leader = true;
        dialog.clear();
        assert!(!dialog.in_group());
        assert!(!dialog.is_leader);
    }

    #[test]
    fn test_max_group_size() {
        assert_eq!(MAX_GROUP, 10);
    }
}
