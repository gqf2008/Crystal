// ============================================================================
// RelationshipDialogHybrid - 关系系统对话框（对齐 C# RelationshipDialog.cs + MentorDialog.cs）
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/RelationshipDialog.cs, MentorDialog.cs
// - RelationshipDialog: 伴侣信息（名字/职业/等级），戒指槽位，离婚/私聊按钮
// - MentorDialog: 师徒信息，可用导师列表，申请/接受/离开按钮
//
// ============================================================================

use macroquad::prelude::*;
use super::native_ui_utils::*;

// ============================================================================
// 常量
// ============================================================================

const REL_WIDTH: f32 = 264.0;
const REL_HEIGHT: f32 = 200.0;
const MENTOR_WIDTH: f32 = 280.0;
const MENTOR_HEIGHT: f32 = 300.0;
const MENTOR_LIST_ROWS: usize = 8;
const ROW_HEIGHT: f32 = 20.0;
const CELL_SIZE: f32 = 34.0;

// ============================================================================
// 类型定义
// ============================================================================

/// 关系操作事件
#[derive(Debug, Clone, PartialEq)]
pub enum RelationshipAction {
    /// 私聊伴侣/师傅
    Whisper,
    /// 离婚
    Divorce,
    /// 制作戒指
    MakeRing,
    /// 请求拜师
    RequestMentor(String),
    /// 接受徒弟
    AcceptMentee(String),
    /// 离开师门
    LeaveMentor,
    /// 关闭
    Close,
}

// ============================================================================
// RelationshipDialogHybrid
// ============================================================================

/// 伴侣关系对话框
pub struct RelationshipDialogHybrid {
    pub visible: bool,
    pub partner_name: String,
    pub partner_class: String,
    pub partner_level: u16,
    pub partner_online: bool,
    pub has_ring: bool,
    position: Vec2,
    drag_helper: DragHelper,
}

impl RelationshipDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            partner_name: String::new(),
            partner_class: String::new(),
            partner_level: 0,
            partner_online: false,
            has_ring: false,
            position: Vec2::new(300.0, 200.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 是否有伴侣
    pub fn has_partner(&self) -> bool {
        !self.partner_name.is_empty()
    }

    /// 设置伴侣信息
    pub fn set_partner(&mut self, name: &str, class: &str, level: u16, online: bool) {
        self.partner_name = name.to_string();
        self.partner_class = class.to_string();
        self.partner_level = level;
        self.partner_online = online;
    }

    /// 绘制并处理输入
    pub fn draw(&mut self) -> Option<RelationshipAction> {
        if !self.visible {
            return None;
        }

        let mouse = mouse_pos();
        let mut action = None;

        let title_rect = Rect::new(self.position.x, self.position.y, REL_WIDTH, 22.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        // 背景
        draw_rectangle(x, y, REL_WIDTH, REL_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.85));
        draw_rectangle_lines(x, y, REL_WIDTH, REL_HEIGHT, 1.0, DARKGRAY);
        draw_text("伴侣关系", x + 10.0, y + 16.0, 14.0, GOLD);

        if self.has_partner() {
            // 伴侣信息
            let online_color = if self.partner_online { LIME } else { GRAY };
            let status = if self.partner_online { "在线" } else { "离线" };
            draw_text(&format!("伴侣: {} ({})", self.partner_name, status), x + 10.0, y + 44.0, 12.0, online_color);
            draw_text(&format!("职业: {}  等级: {}", self.partner_class, self.partner_level), x + 10.0, y + 62.0, 11.0, WHITE);

            // 戒指槽位
            let ring_rect = Rect::new(x + 10.0, y + 80.0, CELL_SIZE, CELL_SIZE);
            let ring_highlight = if ring_rect.contains(mouse) { CellHighlight::Hovered } else { CellHighlight::None };
            draw_cell_frame(ring_rect, ring_highlight, &CellStyle::default());
            if self.has_ring {
                draw_rectangle(ring_rect.x + 6.0, ring_rect.y + 6.0, CELL_SIZE - 12.0, CELL_SIZE - 12.0, GOLD);
            }
            draw_text("戒指", x + 50.0, y + 100.0, 11.0, GRAY);

            // 按钮
            let btn_y = y + REL_HEIGHT - 34.0;

            let whisper_rect = Rect::new(x + 10.0, btn_y, 60.0, 20.0);
            draw_rectangle_lines(whisper_rect.x, whisper_rect.y, whisper_rect.w, whisper_rect.h, 1.0, GRAY);
            draw_text("私聊", whisper_rect.x + 16.0, whisper_rect.y + 14.0, 11.0, GRAY);
            if is_mouse_over(whisper_rect) && is_mouse_button_pressed(MouseButton::Left) {
                action = Some(RelationshipAction::Whisper);
            }

            let ring_btn_rect = Rect::new(x + 80.0, btn_y, 70.0, 20.0);
            draw_rectangle_lines(ring_btn_rect.x, ring_btn_rect.y, ring_btn_rect.w, ring_btn_rect.h, 1.0, GRAY);
            draw_text("制作戒指", ring_btn_rect.x + 8.0, ring_btn_rect.y + 14.0, 11.0, GRAY);
            if is_mouse_over(ring_btn_rect) && is_mouse_button_pressed(MouseButton::Left) {
                action = Some(RelationshipAction::MakeRing);
            }

            let divorce_rect = Rect::new(x + 160.0, btn_y, 60.0, 20.0);
            draw_rectangle_lines(divorce_rect.x, divorce_rect.y, divorce_rect.w, divorce_rect.h, 1.0, GRAY);
            draw_text("离婚", divorce_rect.x + 16.0, divorce_rect.y + 14.0, 11.0, RED);
            if is_mouse_over(divorce_rect) && is_mouse_button_pressed(MouseButton::Left) {
                action = Some(RelationshipAction::Divorce);
            }
        } else {
            draw_text("未建立伴侣关系", x + 70.0, y + 90.0, 12.0, GRAY);
        }

        // 关闭
        let close_rect = Rect::new(x + REL_WIDTH - 20.0, y + 2.0, 16.0, 16.0);
        draw_text("X", close_rect.x + 3.0, close_rect.y + 12.0, 12.0, GRAY);
        if is_mouse_over(close_rect) && is_mouse_button_pressed(MouseButton::Left) {
            self.visible = false;
            action = Some(RelationshipAction::Close);
        }

        action
    }
}

// ============================================================================
// MentorDialogHybrid
// ============================================================================

/// 师徒对话框
pub struct MentorDialogHybrid {
    pub visible: bool,
    pub mentor_name: String,
    pub mentor_level: u16,
    pub mentor_online: bool,
    pub is_mentor: bool,
    pub available_mentors: Vec<String>,
    pub selected_index: Option<usize>,
    page: usize,
    position: Vec2,
    drag_helper: DragHelper,
}

impl MentorDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            mentor_name: String::new(),
            mentor_level: 0,
            mentor_online: false,
            is_mentor: false,
            available_mentors: Vec::new(),
            selected_index: None,
            page: 0,
            position: Vec2::new(320.0, 150.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 是否有师傅
    pub fn has_mentor(&self) -> bool {
        !self.mentor_name.is_empty()
    }

    /// 当前页导师列表
    pub fn current_page_mentors(&self) -> &[String] {
        let start = self.page * MENTOR_LIST_ROWS;
        let end = (start + MENTOR_LIST_ROWS).min(self.available_mentors.len());
        if start >= self.available_mentors.len() {
            &[]
        } else {
            &self.available_mentors[start..end]
        }
    }

    /// 绘制并处理输入
    pub fn draw(&mut self) -> Option<RelationshipAction> {
        if !self.visible {
            return None;
        }

        let mouse = mouse_pos();
        let mut action = None;

        let title_rect = Rect::new(self.position.x, self.position.y, MENTOR_WIDTH, 22.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        // 背景
        draw_rectangle(x, y, MENTOR_WIDTH, MENTOR_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.85));
        draw_rectangle_lines(x, y, MENTOR_WIDTH, MENTOR_HEIGHT, 1.0, DARKGRAY);
        draw_text("师徒系统", x + 10.0, y + 16.0, 14.0, GOLD);

        // 当前师傅信息
        if self.has_mentor() {
            let online_color = if self.mentor_online { LIME } else { GRAY };
            let role = if self.is_mentor { "徒弟" } else { "师傅" };
            draw_text(&format!("{}: {} Lv.{}", role, self.mentor_name, self.mentor_level), x + 10.0, y + 40.0, 12.0, online_color);

            let leave_rect = Rect::new(x + 10.0, y + 56.0, 70.0, 20.0);
            draw_rectangle_lines(leave_rect.x, leave_rect.y, leave_rect.w, leave_rect.h, 1.0, GRAY);
            draw_text("离开师门", leave_rect.x + 8.0, leave_rect.y + 14.0, 11.0, RED);
            if is_mouse_over(leave_rect) && is_mouse_button_pressed(MouseButton::Left) {
                action = Some(RelationshipAction::LeaveMentor);
            }

            let whisper_rect = Rect::new(x + 90.0, y + 56.0, 60.0, 20.0);
            draw_rectangle_lines(whisper_rect.x, whisper_rect.y, whisper_rect.w, whisper_rect.h, 1.0, GRAY);
            draw_text("私聊", whisper_rect.x + 16.0, whisper_rect.y + 14.0, 11.0, GRAY);
            if is_mouse_over(whisper_rect) && is_mouse_button_pressed(MouseButton::Left) {
                action = Some(RelationshipAction::Whisper);
            }
        } else {
            draw_text("未拜师", x + 10.0, y + 40.0, 12.0, GRAY);
        }

        // 可用导师列表
        draw_text("可用导师:", x + 10.0, y + 92.0, 12.0, WHITE);
        let list_y = y + 100.0;
        let start = self.page * MENTOR_LIST_ROWS;
        let end = (start + MENTOR_LIST_ROWS).min(self.available_mentors.len());
        let page_range = if start < self.available_mentors.len() { start..end } else { 0..0 };
        for (i, idx) in page_range.enumerate() {
            let name = &self.available_mentors[idx];
            let row_y = list_y + i as f32 * ROW_HEIGHT;
            let row_rect = Rect::new(x + 10.0, row_y, MENTOR_WIDTH - 20.0, ROW_HEIGHT);

            let bg_color = if self.selected_index == Some(idx) {
                Color::new(0.3, 0.3, 0.5, 0.6)
            } else if row_rect.contains(mouse) {
                Color::new(0.2, 0.2, 0.3, 0.4)
            } else {
                Color::new(0.0, 0.0, 0.0, 0.0)
            };
            draw_rectangle(row_rect.x, row_rect.y, row_rect.w, row_rect.h, bg_color);
            draw_text(name, x + 14.0, row_y + 14.0, 11.0, WHITE);

            if is_mouse_over(row_rect) && is_mouse_button_pressed(MouseButton::Left) {
                self.selected_index = Some(idx);
            }
        }

        // 操作按钮
        let btn_y = y + MENTOR_HEIGHT - 32.0;

        let req_rect = Rect::new(x + 10.0, btn_y, 70.0, 20.0);
        draw_rectangle_lines(req_rect.x, req_rect.y, req_rect.w, req_rect.h, 1.0, GRAY);
        draw_text("请求拜师", req_rect.x + 8.0, req_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(req_rect) && is_mouse_button_pressed(MouseButton::Left) {
            if let Some(idx) = self.selected_index {
                if let Some(name) = self.available_mentors.get(idx) {
                    action = Some(RelationshipAction::RequestMentor(name.clone()));
                }
            }
        }

        let accept_rect = Rect::new(x + 90.0, btn_y, 70.0, 20.0);
        draw_rectangle_lines(accept_rect.x, accept_rect.y, accept_rect.w, accept_rect.h, 1.0, GRAY);
        draw_text("接受徒弟", accept_rect.x + 8.0, accept_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(accept_rect) && is_mouse_button_pressed(MouseButton::Left) {
            if let Some(idx) = self.selected_index {
                if let Some(name) = self.available_mentors.get(idx) {
                    action = Some(RelationshipAction::AcceptMentee(name.clone()));
                }
            }
        }

        // 关闭
        let close_rect = Rect::new(x + MENTOR_WIDTH - 20.0, y + 2.0, 16.0, 16.0);
        draw_text("X", close_rect.x + 3.0, close_rect.y + 12.0, 12.0, GRAY);
        if is_mouse_over(close_rect) && is_mouse_button_pressed(MouseButton::Left) {
            self.visible = false;
            action = Some(RelationshipAction::Close);
        }

        action
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relationship_dialog_creation() {
        let dialog = RelationshipDialogHybrid::new();
        assert!(!dialog.visible);
        assert!(!dialog.has_partner());
        assert!(dialog.partner_name.is_empty());
    }

    #[test]
    fn test_relationship_set_partner() {
        let mut dialog = RelationshipDialogHybrid::new();
        dialog.set_partner("Partner1", "Warrior", 50, true);
        assert!(dialog.has_partner());
        assert_eq!(dialog.partner_name, "Partner1");
        assert_eq!(dialog.partner_level, 50);
        assert!(dialog.partner_online);
    }

    #[test]
    fn test_mentor_dialog_creation() {
        let mut dialog = MentorDialogHybrid::new();
        assert!(!dialog.has_mentor());
        dialog.mentor_name = "MasterZhang".to_string();
        assert!(dialog.has_mentor());
        dialog.available_mentors = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        assert_eq!(dialog.current_page_mentors().len(), 3);
    }
}
