// ============================================================================
// MountDialogHybrid / FishingDialogHybrid / RankingDialogHybrid
// （对齐 C# MountDialog.cs + FishingDialog.cs + RankingDialog.cs）
// ============================================================================
//
// C# 参考：
// - Client/MirScenes/Dialogs/MountDialog.cs: 坐骑显示/喂养/骑乘
// - Client/MirScenes/Dialogs/FishingDialog.cs: 钓鱼竿/鱼饵/抛竿/收竿
// - Client/MirScenes/Dialogs/RankingDialog.cs: 排行榜（等级/PK/公会）
//
// ============================================================================

use macroquad::prelude::*;
use super::native_ui_utils::*;

// ============================================================================
// 常量
// ============================================================================

const MOUNT_WIDTH: f32 = 264.0;
const MOUNT_HEIGHT: f32 = 240.0;
const FISHING_WIDTH: f32 = 220.0;
const FISHING_HEIGHT: f32 = 180.0;
const FISHING_STATUS_WIDTH: f32 = 140.0;
const FISHING_STATUS_HEIGHT: f32 = 40.0;
const RANKING_WIDTH: f32 = 300.0;
const RANKING_HEIGHT: f32 = 380.0;
const CELL_SIZE: f32 = 34.0;
const RANKING_ROWS: usize = 15;
const ROW_HEIGHT: f32 = 20.0;

// ============================================================================
// 类型定义
// ============================================================================

/// 坐骑操作事件
#[derive(Debug, Clone, PartialEq)]
pub enum MountAction {
    /// 骑乘
    Ride,
    /// 下马
    Dismount,
    /// 喂养
    Feed,
    /// 关闭
    Close,
}

/// 钓鱼操作事件
#[derive(Debug, Clone, PartialEq)]
pub enum FishingAction {
    /// 抛竿
    Cast,
    /// 收竿
    Reel,
    /// 关闭
    Close,
}

/// 排行榜操作事件
#[derive(Debug, Clone, PartialEq)]
pub enum RankingAction {
    /// 切换页签
    SwitchTab(RankingTab),
    /// 上一页
    PrevPage,
    /// 下一页
    NextPage,
    /// 关闭
    Close,
}

/// 排行榜页签
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankingTab {
    Level,
    PK,
    Guild,
}

/// 排行榜条目
#[derive(Debug, Clone)]
pub struct RankingEntry {
    pub rank: u32,
    pub name: String,
    pub value: String,
}

impl RankingEntry {
    pub fn new(rank: u32, name: &str, value: &str) -> Self {
        Self {
            rank,
            name: name.to_string(),
            value: value.to_string(),
        }
    }
}

// ============================================================================
// MountDialogHybrid
// ============================================================================

/// 坐骑对话框
pub struct MountDialogHybrid {
    pub visible: bool,
    pub mount_name: String,
    pub mount_level: u16,
    pub exp: u64,
    pub max_exp: u64,
    pub is_riding: bool,
    position: Vec2,
    drag_helper: DragHelper,
}

impl MountDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            mount_name: String::new(),
            mount_level: 0,
            exp: 0,
            max_exp: 100,
            is_riding: false,
            position: Vec2::new(300.0, 150.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 经验值百分比
    pub fn exp_percent(&self) -> f32 {
        if self.max_exp == 0 {
            0.0
        } else {
            self.exp as f32 / self.max_exp as f32
        }
    }

    /// 是否有坐骑
    pub fn has_mount(&self) -> bool {
        !self.mount_name.is_empty()
    }

    /// 绘制并处理输入
    pub fn draw(&mut self) -> Option<MountAction> {
        if !self.visible {
            return None;
        }

        let mouse = mouse_pos();
        let mut action = None;

        let title_rect = Rect::new(self.position.x, self.position.y, MOUNT_WIDTH, 22.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        // 背景
        draw_rectangle(x, y, MOUNT_WIDTH, MOUNT_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.85));
        draw_rectangle_lines(x, y, MOUNT_WIDTH, MOUNT_HEIGHT, 1.0, DARKGRAY);
        draw_text("坐骑", x + 10.0, y + 16.0, 14.0, GOLD);

        if self.has_mount() {
            // 坐骑信息
            draw_text(&format!("{} Lv.{}", self.mount_name, self.mount_level), x + 10.0, y + 44.0, 12.0, WHITE);

            // 经验条
            let bar_x = x + 10.0;
            let bar_y = y + 56.0;
            let bar_w = MOUNT_WIDTH - 20.0;
            draw_rectangle(bar_x, bar_y, bar_w, 10.0, Color::new(0.2, 0.2, 0.2, 1.0));
            draw_rectangle(bar_x, bar_y, bar_w * self.exp_percent(), 10.0, Color::new(0.2, 0.6, 1.0, 1.0));
            draw_text(
                &format!("EXP: {}/{}", self.exp, self.max_exp),
                bar_x + 4.0,
                bar_y + 9.0,
                9.0,
                WHITE,
            );

            // 坐骑显示区域
            let display_rect = Rect::new(x + 40.0, y + 76.0, MOUNT_WIDTH - 80.0, 100.0);
            draw_rectangle_lines(display_rect.x, display_rect.y, display_rect.w, display_rect.h, 1.0, Color::new(0.3, 0.3, 0.3, 0.5));

            let status = if self.is_riding { "骑乘中" } else { "待命" };
            draw_text(status, x + 100.0, y + 130.0, 12.0, if self.is_riding { LIME } else { GRAY });

            // 操作按钮
            let btn_y = y + MOUNT_HEIGHT - 32.0;

            let ride_label = if self.is_riding { "下马" } else { "骑乘" };
            let ride_rect = Rect::new(x + 20.0, btn_y, 60.0, 20.0);
            draw_rectangle_lines(ride_rect.x, ride_rect.y, ride_rect.w, ride_rect.h, 1.0, GRAY);
            draw_text(ride_label, ride_rect.x + 14.0, ride_rect.y + 14.0, 11.0, GRAY);
            if is_mouse_over(ride_rect) && is_mouse_button_pressed(MouseButton::Left) {
                action = Some(if self.is_riding { MountAction::Dismount } else { MountAction::Ride });
            }

            let feed_rect = Rect::new(x + 100.0, btn_y, 60.0, 20.0);
            draw_rectangle_lines(feed_rect.x, feed_rect.y, feed_rect.w, feed_rect.h, 1.0, GRAY);
            draw_text("喂养", feed_rect.x + 16.0, feed_rect.y + 14.0, 11.0, GRAY);
            if is_mouse_over(feed_rect) && is_mouse_button_pressed(MouseButton::Left) {
                action = Some(MountAction::Feed);
            }
        } else {
            draw_text("没有坐骑", x + 90.0, y + 110.0, 12.0, GRAY);
        }

        // 关闭
        let close_rect = Rect::new(x + MOUNT_WIDTH - 20.0, y + 2.0, 16.0, 16.0);
        draw_text("X", close_rect.x + 3.0, close_rect.y + 12.0, 12.0, GRAY);
        if is_mouse_over(close_rect) && is_mouse_button_pressed(MouseButton::Left) {
            self.visible = false;
            action = Some(MountAction::Close);
        }

        action
    }
}

// ============================================================================
// FishingDialogHybrid
// ============================================================================

/// 钓鱼对话框
pub struct FishingDialogHybrid {
    pub visible: bool,
    pub rod_slot: Option<usize>,
    pub bait_slot: Option<usize>,
    pub is_casting: bool,
    pub progress: f32,
    position: Vec2,
    drag_helper: DragHelper,
}

impl FishingDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            rod_slot: None,
            bait_slot: None,
            is_casting: false,
            progress: 0.0,
            position: Vec2::new(350.0, 200.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 绘制并处理输入
    pub fn draw(&mut self) -> Option<FishingAction> {
        if !self.visible {
            return None;
        }

        let mouse = mouse_pos();
        let mut action = None;

        let title_rect = Rect::new(self.position.x, self.position.y, FISHING_WIDTH, 22.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        // 背景
        draw_rectangle(x, y, FISHING_WIDTH, FISHING_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.85));
        draw_rectangle_lines(x, y, FISHING_WIDTH, FISHING_HEIGHT, 1.0, DARKGRAY);
        draw_text("钓鱼", x + 10.0, y + 16.0, 14.0, GOLD);

        // 鱼竿槽
        draw_text("鱼竿:", x + 10.0, y + 42.0, 11.0, WHITE);
        let rod_rect = Rect::new(x + 60.0, y + 30.0, CELL_SIZE, CELL_SIZE);
        let rod_hl = if rod_rect.contains(mouse) { CellHighlight::Hovered } else { CellHighlight::None };
        draw_cell_frame(rod_rect, rod_hl, &CellStyle::default());
        if self.rod_slot.is_some() {
            draw_rectangle(rod_rect.x + 4.0, rod_rect.y + 4.0, CELL_SIZE - 8.0, CELL_SIZE - 8.0, Color::new(0.5, 0.4, 0.2, 0.6));
        }

        // 鱼饵槽
        draw_text("鱼饵:", x + 120.0, y + 42.0, 11.0, WHITE);
        let bait_rect = Rect::new(x + 166.0, y + 30.0, CELL_SIZE, CELL_SIZE);
        let bait_hl = if bait_rect.contains(mouse) { CellHighlight::Hovered } else { CellHighlight::None };
        draw_cell_frame(bait_rect, bait_hl, &CellStyle::default());
        if self.bait_slot.is_some() {
            draw_rectangle(bait_rect.x + 4.0, bait_rect.y + 4.0, CELL_SIZE - 8.0, CELL_SIZE - 8.0, Color::new(0.3, 0.5, 0.3, 0.6));
        }

        // 进度条
        if self.is_casting {
            let bar_x = x + 10.0;
            let bar_y = y + 80.0;
            let bar_w = FISHING_WIDTH - 20.0;
            draw_rectangle(bar_x, bar_y, bar_w, 12.0, Color::new(0.2, 0.2, 0.2, 1.0));
            draw_rectangle(bar_x, bar_y, bar_w * self.progress.clamp(0.0, 1.0), 12.0, Color::new(0.2, 0.7, 0.3, 1.0));
            draw_text("钓鱼中...", bar_x + 4.0, bar_y + 10.0, 10.0, WHITE);
        }

        // 操作按钮
        let btn_y = y + FISHING_HEIGHT - 32.0;

        let cast_label = if self.is_casting { "收竿" } else { "抛竿" };
        let cast_rect = Rect::new(x + 60.0, btn_y, 60.0, 20.0);
        draw_rectangle_lines(cast_rect.x, cast_rect.y, cast_rect.w, cast_rect.h, 1.0, GRAY);
        draw_text(cast_label, cast_rect.x + 14.0, cast_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(cast_rect) && is_mouse_button_pressed(MouseButton::Left) {
            action = Some(if self.is_casting { FishingAction::Reel } else { FishingAction::Cast });
        }

        // 关闭
        let close_rect = Rect::new(x + FISHING_WIDTH - 20.0, y + 2.0, 16.0, 16.0);
        draw_text("X", close_rect.x + 3.0, close_rect.y + 12.0, 12.0, GRAY);
        if is_mouse_over(close_rect) && is_mouse_button_pressed(MouseButton::Left) {
            self.visible = false;
            action = Some(FishingAction::Close);
        }

        action
    }
}

// ============================================================================
// FishingStatusDialogHybrid
// ============================================================================

/// 钓鱼状态浮窗
pub struct FishingStatusDialogHybrid {
    pub visible: bool,
    pub state_text: String,
    pub timer_seconds: f32,
    position: Vec2,
}

impl FishingStatusDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            state_text: String::new(),
            timer_seconds: 0.0,
            position: Vec2::new(400.0, 50.0),
        }
    }

    /// 绘制
    pub fn draw(&mut self) -> Option<FishingAction> {
        if !self.visible {
            return None;
        }

        let x = self.position.x;
        let y = self.position.y;

        draw_rectangle(x, y, FISHING_STATUS_WIDTH, FISHING_STATUS_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.7));
        draw_rectangle_lines(x, y, FISHING_STATUS_WIDTH, FISHING_STATUS_HEIGHT, 1.0, Color::new(0.3, 0.6, 0.3, 0.8));

        draw_text(&self.state_text, x + 6.0, y + 16.0, 11.0, WHITE);
        draw_text(&format!("{:.1}s", self.timer_seconds), x + 6.0, y + 32.0, 11.0, GOLD);

        None
    }
}

// ============================================================================
// RankingDialogHybrid
// ============================================================================

/// 排行榜对话框
pub struct RankingDialogHybrid {
    pub visible: bool,
    pub tab: RankingTab,
    pub entries: Vec<RankingEntry>,
    page: usize,
    position: Vec2,
    drag_helper: DragHelper,
}

impl RankingDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            tab: RankingTab::Level,
            entries: Vec::new(),
            page: 0,
            position: Vec2::new(250.0, 80.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 当前页条目
    pub fn current_page_entries(&self) -> &[RankingEntry] {
        let start = self.page * RANKING_ROWS;
        let end = (start + RANKING_ROWS).min(self.entries.len());
        if start >= self.entries.len() {
            &[]
        } else {
            &self.entries[start..end]
        }
    }

    /// 总页数
    pub fn page_count(&self) -> usize {
        if self.entries.is_empty() {
            0
        } else {
            (self.entries.len() - 1) / RANKING_ROWS + 1
        }
    }

    /// 绘制并处理输入
    pub fn draw(&mut self) -> Option<RankingAction> {
        if !self.visible {
            return None;
        }

        let mouse = mouse_pos();
        let mut action = None;

        let title_rect = Rect::new(self.position.x, self.position.y, RANKING_WIDTH, 22.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        // 背景
        draw_rectangle(x, y, RANKING_WIDTH, RANKING_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.85));
        draw_rectangle_lines(x, y, RANKING_WIDTH, RANKING_HEIGHT, 1.0, DARKGRAY);
        draw_text("排行榜", x + 10.0, y + 16.0, 14.0, GOLD);

        // 页签
        let tabs = [(RankingTab::Level, "等级"), (RankingTab::PK, "PK"), (RankingTab::Guild, "公会")];
        for (i, (tab, label)) in tabs.iter().enumerate() {
            let tab_rect = Rect::new(x + 10.0 + i as f32 * 70.0, y + 24.0, 60.0, 18.0);
            let is_active = self.tab == *tab;
            let bg = if is_active {
                Color::new(0.3, 0.3, 0.5, 0.8)
            } else if tab_rect.contains(mouse) {
                Color::new(0.2, 0.2, 0.3, 0.5)
            } else {
                Color::new(0.1, 0.1, 0.1, 0.5)
            };
            draw_rectangle(tab_rect.x, tab_rect.y, tab_rect.w, tab_rect.h, bg);
            draw_text(label, tab_rect.x + 14.0, tab_rect.y + 13.0, 11.0, if is_active { WHITE } else { GRAY });

            if is_mouse_over(tab_rect) && is_mouse_button_pressed(MouseButton::Left) && !is_active {
                self.tab = *tab;
                self.page = 0;
                action = Some(RankingAction::SwitchTab(*tab));
            }
        }

        // 表头
        let header_y = y + 48.0;
        draw_text("排名", x + 10.0, header_y + 14.0, 11.0, GOLD);
        draw_text("名字", x + 60.0, header_y + 14.0, 11.0, GOLD);
        draw_text("数值", x + 200.0, header_y + 14.0, 11.0, GOLD);
        draw_line(x + 10.0, header_y + 18.0, x + RANKING_WIDTH - 10.0, header_y + 18.0, 1.0, DARKGRAY);

        // 排行列表
        let page_entries = self.current_page_entries();
        for (i, entry) in page_entries.iter().enumerate() {
            let row_y = header_y + 22.0 + i as f32 * ROW_HEIGHT;
            let row_rect = Rect::new(x + 10.0, row_y, RANKING_WIDTH - 20.0, ROW_HEIGHT);

            if row_rect.contains(mouse) {
                draw_rectangle(row_rect.x, row_rect.y, row_rect.w, row_rect.h, Color::new(0.2, 0.2, 0.3, 0.3));
            }

            let rank_color = match entry.rank {
                1 => GOLD,
                2 => Color::new(0.75, 0.75, 0.75, 1.0),
                3 => Color::new(0.8, 0.5, 0.2, 1.0),
                _ => WHITE,
            };
            draw_text(&format!("{}", entry.rank), x + 10.0, row_y + 14.0, 11.0, rank_color);
            draw_text(&entry.name, x + 60.0, row_y + 14.0, 11.0, WHITE);
            draw_text(&entry.value, x + 200.0, row_y + 14.0, 11.0, GRAY);
        }

        // 翻页按钮
        let page_y = y + RANKING_HEIGHT - 30.0;
        let prev_rect = Rect::new(x + 80.0, page_y, 50.0, 20.0);
        draw_rectangle_lines(prev_rect.x, prev_rect.y, prev_rect.w, prev_rect.h, 1.0, GRAY);
        draw_text("上一页", prev_rect.x + 6.0, prev_rect.y + 14.0, 10.0, GRAY);
        if is_mouse_over(prev_rect) && is_mouse_button_pressed(MouseButton::Left) && self.page > 0 {
            self.page -= 1;
            action = Some(RankingAction::PrevPage);
        }

        let next_rect = Rect::new(x + 150.0, page_y, 50.0, 20.0);
        draw_rectangle_lines(next_rect.x, next_rect.y, next_rect.w, next_rect.h, 1.0, GRAY);
        draw_text("下一页", next_rect.x + 6.0, next_rect.y + 14.0, 10.0, GRAY);
        if is_mouse_over(next_rect) && is_mouse_button_pressed(MouseButton::Left) && self.page + 1 < self.page_count() {
            self.page += 1;
            action = Some(RankingAction::NextPage);
        }

        // 关闭
        let close_rect = Rect::new(x + RANKING_WIDTH - 20.0, y + 2.0, 16.0, 16.0);
        draw_text("X", close_rect.x + 3.0, close_rect.y + 12.0, 12.0, GRAY);
        if is_mouse_over(close_rect) && is_mouse_button_pressed(MouseButton::Left) {
            self.visible = false;
            action = Some(RankingAction::Close);
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
    fn test_mount_exp_percent() {
        let mut dialog = MountDialogHybrid::new();
        dialog.exp = 50;
        dialog.max_exp = 200;
        assert!((dialog.exp_percent() - 0.25).abs() < f32::EPSILON);

        dialog.max_exp = 0;
        assert!((dialog.exp_percent() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_mount_has_mount() {
        let mut dialog = MountDialogHybrid::new();
        assert!(!dialog.has_mount());
        dialog.mount_name = "Dragon".to_string();
        assert!(dialog.has_mount());
    }

    #[test]
    fn test_ranking_pagination() {
        let mut dialog = RankingDialogHybrid::new();
        assert_eq!(dialog.page_count(), 0);

        for i in 0..35 {
            dialog.entries.push(RankingEntry::new(i + 1, &format!("Player{}", i), &format!("{}", 100 - i)));
        }
        assert_eq!(dialog.page_count(), 3); // 35 / 15 = 3 pages

        let page0 = dialog.current_page_entries();
        assert_eq!(page0.len(), 15);
        assert_eq!(page0[0].rank, 1);
    }

    #[test]
    fn test_fishing_dialog_creation() {
        let dialog = FishingDialogHybrid::new();
        assert!(!dialog.visible);
        assert!(dialog.rod_slot.is_none());
        assert!(dialog.bait_slot.is_none());
        assert!(!dialog.is_casting);
        assert!((dialog.progress - 0.0).abs() < f32::EPSILON);
    }
}
