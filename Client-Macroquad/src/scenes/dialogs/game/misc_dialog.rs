// ============================================================================
// MiscDialogHybrid - 杂项对话框集合
// （对齐 C# HelpDialog + CompassDialog + TimerDialog + RollDialog +
//   ReportDialog + KeyboardLayoutDialog + NoticeDialog + NewCharacterDialog +
//   ItemRentalDialog + TrustMerchantDialog + IntelligentCreatureDialog +
//   NPCAwakeDialog + ChatNoticeDialog）
// ============================================================================

use macroquad::prelude::*;
use super::native_ui_utils::*;

// ============================================================================
// 常量
// ============================================================================

const HELP_WIDTH: f32 = 280.0;
const HELP_HEIGHT: f32 = 320.0;
const COMPASS_SIZE: f32 = 80.0;
const TIMER_WIDTH: f32 = 120.0;
const TIMER_HEIGHT: f32 = 50.0;
const ROLL_WIDTH: f32 = 180.0;
const ROLL_HEIGHT: f32 = 140.0;
const REPORT_WIDTH: f32 = 260.0;
const REPORT_HEIGHT: f32 = 220.0;
const KEYBOARD_WIDTH: f32 = 340.0;
const KEYBOARD_HEIGHT: f32 = 280.0;
const NOTICE_WIDTH: f32 = 300.0;
const NOTICE_HEIGHT: f32 = 200.0;
const NEW_CHAR_WIDTH: f32 = 300.0;
const NEW_CHAR_HEIGHT: f32 = 280.0;
const RENTAL_WIDTH: f32 = 280.0;
const RENTAL_HEIGHT: f32 = 300.0;
const TRUST_WIDTH: f32 = 320.0;
const TRUST_HEIGHT: f32 = 360.0;
const CREATURE_WIDTH: f32 = 280.0;
const CREATURE_HEIGHT: f32 = 260.0;
const AWAKE_WIDTH: f32 = 260.0;
const AWAKE_HEIGHT: f32 = 240.0;
const CHAT_NOTICE_WIDTH: f32 = 400.0;
const CHAT_NOTICE_HEIGHT: f32 = 24.0;
const CELL_SIZE: f32 = 34.0;

// ============================================================================
// MiscAction enums
// ============================================================================

/// 帮助对话框操作
#[derive(Debug, Clone, PartialEq)]
pub enum HelpAction {
    NextPage,
    PrevPage,
    Close,
}

/// 报告对话框操作
#[derive(Debug, Clone, PartialEq)]
pub enum ReportAction {
    Submit(String),
    Close,
}

/// 新角色对话框操作
#[derive(Debug, Clone, PartialEq)]
pub enum NewCharacterAction {
    Create { name: String, class_id: u8, gender: u8 },
    Cancel,
}

/// 物品租赁操作
#[derive(Debug, Clone, PartialEq)]
pub enum RentalAction {
    Rent(u32),
    Return(u32),
    Close,
}

/// 信任商人操作
#[derive(Debug, Clone, PartialEq)]
pub enum TrustMerchantAction {
    Search(String),
    Bid { item_id: u32, amount: u64 },
    ListItem { slot: usize, price: u64 },
    Close,
}

/// 灵兽操作
#[derive(Debug, Clone, PartialEq)]
pub enum CreatureAction {
    Rename(usize, String),
    Release(usize),
    Summon(usize),
    Close,
}

/// 觉醒操作
#[derive(Debug, Clone, PartialEq)]
pub enum AwakeAction {
    Awaken,
    Close,
}

// ============================================================================
// HelpDialogHybrid
// ============================================================================

/// 帮助对话框
pub struct HelpDialogHybrid {
    pub visible: bool,
    pub pages: Vec<String>,
    pub current_page: usize,
    position: Vec2,
    drag_helper: DragHelper,
}

impl HelpDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            pages: Vec::new(),
            current_page: 0,
            position: Vec2::new(200.0, 100.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 总页数
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// 绘制并处理输入
    pub fn draw(&mut self) -> Option<HelpAction> {
        if !self.visible {
            return None;
        }

        let mouse = mouse_pos();
        let mut action = None;

        let title_rect = Rect::new(self.position.x, self.position.y, HELP_WIDTH, 22.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        draw_rectangle(x, y, HELP_WIDTH, HELP_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.85));
        draw_rectangle_lines(x, y, HELP_WIDTH, HELP_HEIGHT, 1.0, DARKGRAY);
        draw_text("帮助", x + 10.0, y + 16.0, 14.0, GOLD);

        // 内容
        if let Some(page_text) = self.pages.get(self.current_page) {
            draw_text(page_text, x + 10.0, y + 40.0, 11.0, WHITE);
        }

        // 页码
        draw_text(
            &format!("{}/{}", self.current_page + 1, self.page_count().max(1)),
            x + HELP_WIDTH / 2.0 - 15.0,
            y + HELP_HEIGHT - 44.0,
            11.0,
            GRAY,
        );

        // 翻页按钮
        let btn_y = y + HELP_HEIGHT - 30.0;
        let prev_rect = Rect::new(x + 60.0, btn_y, 60.0, 20.0);
        draw_rectangle_lines(prev_rect.x, prev_rect.y, prev_rect.w, prev_rect.h, 1.0, GRAY);
        draw_text("上一页", prev_rect.x + 10.0, prev_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(prev_rect) && is_mouse_button_pressed(MouseButton::Left) && self.current_page > 0 {
            self.current_page -= 1;
            action = Some(HelpAction::PrevPage);
        }

        let next_rect = Rect::new(x + 140.0, btn_y, 60.0, 20.0);
        draw_rectangle_lines(next_rect.x, next_rect.y, next_rect.w, next_rect.h, 1.0, GRAY);
        draw_text("下一页", next_rect.x + 10.0, next_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(next_rect) && is_mouse_button_pressed(MouseButton::Left) && self.current_page + 1 < self.page_count() {
            self.current_page += 1;
            action = Some(HelpAction::NextPage);
        }

        // 关闭
        let close_rect = Rect::new(x + HELP_WIDTH - 20.0, y + 2.0, 16.0, 16.0);
        draw_text("X", close_rect.x + 3.0, close_rect.y + 12.0, 12.0, GRAY);
        if is_mouse_over(close_rect) && is_mouse_button_pressed(MouseButton::Left) {
            self.visible = false;
            action = Some(HelpAction::Close);
        }

        action
    }
}

// ============================================================================
// CompassDialogHybrid
// ============================================================================

/// 罗盘对话框
pub struct CompassDialogHybrid {
    pub visible: bool,
    /// 方向角度 (0-360)
    pub direction: f32,
    position: Vec2,
}

impl CompassDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            direction: 0.0,
            position: Vec2::new(10.0, 100.0),
        }
    }

    /// 获取方向标签
    pub fn direction_label(&self) -> &'static str {
        let d = ((self.direction % 360.0) + 360.0) % 360.0;
        if d < 22.5 || d >= 337.5 { "N" }
        else if d < 67.5 { "NE" }
        else if d < 112.5 { "E" }
        else if d < 157.5 { "SE" }
        else if d < 202.5 { "S" }
        else if d < 247.5 { "SW" }
        else if d < 292.5 { "W" }
        else { "NW" }
    }

    /// 绘制
    pub fn draw(&self) {
        if !self.visible {
            return;
        }

        let x = self.position.x;
        let y = self.position.y;
        let center_x = x + COMPASS_SIZE / 2.0;
        let center_y = y + COMPASS_SIZE / 2.0;
        let radius = COMPASS_SIZE / 2.0 - 4.0;

        draw_circle(center_x, center_y, radius, Color::new(0.0, 0.0, 0.0, 0.6));
        draw_circle_lines(center_x, center_y, radius, 1.0, DARKGRAY);

        // 方向指针
        let rad = self.direction.to_radians();
        let end_x = center_x + rad.sin() * (radius - 6.0);
        let end_y = center_y - rad.cos() * (radius - 6.0);
        draw_line(center_x, center_y, end_x, end_y, 2.0, RED);

        // 标签
        draw_text(self.direction_label(), center_x - 6.0, center_y + 4.0, 12.0, WHITE);
    }
}

// ============================================================================
// TimerDialogHybrid
// ============================================================================

/// 倒计时对话框
pub struct TimerDialogHybrid {
    pub visible: bool,
    pub remaining_seconds: f32,
    pub label: String,
    position: Vec2,
}

impl TimerDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            remaining_seconds: 0.0,
            label: String::new(),
            position: Vec2::new(400.0, 10.0),
        }
    }

    /// 格式化剩余时间
    pub fn formatted_time(&self) -> String {
        let secs = self.remaining_seconds.max(0.0) as u32;
        let mins = secs / 60;
        let s = secs % 60;
        format!("{:02}:{:02}", mins, s)
    }

    /// 绘制
    pub fn draw(&self) {
        if !self.visible {
            return;
        }

        let x = self.position.x;
        let y = self.position.y;

        draw_rectangle(x, y, TIMER_WIDTH, TIMER_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.7));
        draw_rectangle_lines(x, y, TIMER_WIDTH, TIMER_HEIGHT, 1.0, DARKGRAY);

        if !self.label.is_empty() {
            draw_text(&self.label, x + 6.0, y + 16.0, 11.0, WHITE);
        }
        draw_text(&self.formatted_time(), x + 20.0, y + 38.0, 18.0, GOLD);
    }
}

// ============================================================================
// RollDialogHybrid
// ============================================================================

/// 掷骰子对话框
pub struct RollDialogHybrid {
    pub visible: bool,
    pub result: Option<u32>,
    pub is_rolling: bool,
    pub max_value: u32,
    position: Vec2,
    drag_helper: DragHelper,
}

impl RollDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            result: None,
            is_rolling: false,
            max_value: 100,
            position: Vec2::new(350.0, 200.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 绘制
    pub fn draw(&mut self) {
        if !self.visible {
            return;
        }

        let mouse = mouse_pos();
        let title_rect = Rect::new(self.position.x, self.position.y, ROLL_WIDTH, 22.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        draw_rectangle(x, y, ROLL_WIDTH, ROLL_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.85));
        draw_rectangle_lines(x, y, ROLL_WIDTH, ROLL_HEIGHT, 1.0, DARKGRAY);
        draw_text("掷骰子", x + 10.0, y + 16.0, 14.0, GOLD);

        // 结果显示
        let display_text = if self.is_rolling {
            "...".to_string()
        } else if let Some(val) = self.result {
            format!("{}", val)
        } else {
            "?".to_string()
        };
        draw_text(&display_text, x + ROLL_WIDTH / 2.0 - 20.0, y + 80.0, 28.0, WHITE);

        draw_text(&format!("(1-{})", self.max_value), x + ROLL_WIDTH / 2.0 - 20.0, y + 110.0, 11.0, GRAY);
    }
}

// ============================================================================
// ReportDialogHybrid
// ============================================================================

/// 举报对话框
pub struct ReportDialogHybrid {
    pub visible: bool,
    pub report_text: String,
    position: Vec2,
    drag_helper: DragHelper,
}

impl ReportDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            report_text: String::new(),
            position: Vec2::new(280.0, 180.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 绘制并处理输入
    pub fn draw(&mut self) -> Option<ReportAction> {
        if !self.visible {
            return None;
        }

        let mouse = mouse_pos();
        let mut action = None;

        let title_rect = Rect::new(self.position.x, self.position.y, REPORT_WIDTH, 22.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        draw_rectangle(x, y, REPORT_WIDTH, REPORT_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.85));
        draw_rectangle_lines(x, y, REPORT_WIDTH, REPORT_HEIGHT, 1.0, DARKGRAY);
        draw_text("举报玩家", x + 10.0, y + 16.0, 14.0, GOLD);

        draw_text("详情:", x + 10.0, y + 40.0, 11.0, WHITE);
        draw_rectangle(x + 10.0, y + 50.0, REPORT_WIDTH - 20.0, 110.0, Color::new(0.15, 0.15, 0.15, 1.0));
        draw_text(&self.report_text, x + 14.0, y + 66.0, 11.0, WHITE);

        let btn_y = y + REPORT_HEIGHT - 30.0;
        let submit_rect = Rect::new(x + 60.0, btn_y, 60.0, 20.0);
        draw_rectangle_lines(submit_rect.x, submit_rect.y, submit_rect.w, submit_rect.h, 1.0, GRAY);
        draw_text("提交", submit_rect.x + 16.0, submit_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(submit_rect) && is_mouse_button_pressed(MouseButton::Left) {
            action = Some(ReportAction::Submit(self.report_text.clone()));
        }

        let cancel_rect = Rect::new(x + 140.0, btn_y, 60.0, 20.0);
        draw_rectangle_lines(cancel_rect.x, cancel_rect.y, cancel_rect.w, cancel_rect.h, 1.0, GRAY);
        draw_text("取消", cancel_rect.x + 16.0, cancel_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(cancel_rect) && is_mouse_button_pressed(MouseButton::Left) {
            self.visible = false;
            action = Some(ReportAction::Close);
        }

        action
    }
}

// ============================================================================
// KeyboardLayoutDialogHybrid
// ============================================================================

/// 键位配置对话框
pub struct KeyboardLayoutDialogHybrid {
    pub visible: bool,
    pub bindings: Vec<(String, String)>,
    pub selected_index: Option<usize>,
    position: Vec2,
    drag_helper: DragHelper,
}

impl KeyboardLayoutDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            bindings: Vec::new(),
            selected_index: None,
            position: Vec2::new(200.0, 80.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 绘制
    pub fn draw(&mut self) {
        if !self.visible {
            return;
        }

        let mouse = mouse_pos();
        let title_rect = Rect::new(self.position.x, self.position.y, KEYBOARD_WIDTH, 22.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        draw_rectangle(x, y, KEYBOARD_WIDTH, KEYBOARD_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.85));
        draw_rectangle_lines(x, y, KEYBOARD_WIDTH, KEYBOARD_HEIGHT, 1.0, DARKGRAY);
        draw_text("键位设置", x + 10.0, y + 16.0, 14.0, GOLD);

        draw_text("动作", x + 10.0, y + 38.0, 11.0, GOLD);
        draw_text("按键", x + 180.0, y + 38.0, 11.0, GOLD);

        for (i, (action_name, key)) in self.bindings.iter().enumerate() {
            let row_y = y + 50.0 + i as f32 * 20.0;
            let row_rect = Rect::new(x + 10.0, row_y, KEYBOARD_WIDTH - 20.0, 20.0);

            let bg = if self.selected_index == Some(i) {
                Color::new(0.3, 0.3, 0.5, 0.6)
            } else if row_rect.contains(mouse) {
                Color::new(0.2, 0.2, 0.3, 0.3)
            } else {
                Color::new(0.0, 0.0, 0.0, 0.0)
            };
            draw_rectangle(row_rect.x, row_rect.y, row_rect.w, row_rect.h, bg);
            draw_text(action_name, x + 14.0, row_y + 14.0, 11.0, WHITE);
            draw_text(key, x + 184.0, row_y + 14.0, 11.0, LIME);

            if is_mouse_over(row_rect) && is_mouse_button_pressed(MouseButton::Left) {
                self.selected_index = Some(i);
            }
        }
    }
}

// ============================================================================
// NoticeDialogHybrid
// ============================================================================

/// 服务器公告对话框
pub struct NoticeDialogHybrid {
    pub visible: bool,
    pub notice_text: String,
    position: Vec2,
    drag_helper: DragHelper,
}

impl NoticeDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            notice_text: String::new(),
            position: Vec2::new(250.0, 150.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 绘制
    pub fn draw(&mut self) {
        if !self.visible {
            return;
        }

        let mouse = mouse_pos();
        let title_rect = Rect::new(self.position.x, self.position.y, NOTICE_WIDTH, 22.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        draw_rectangle(x, y, NOTICE_WIDTH, NOTICE_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.85));
        draw_rectangle_lines(x, y, NOTICE_WIDTH, NOTICE_HEIGHT, 1.0, DARKGRAY);
        draw_text("公告", x + 10.0, y + 16.0, 14.0, GOLD);
        draw_text(&self.notice_text, x + 10.0, y + 40.0, 11.0, WHITE);

        let close_rect = Rect::new(x + NOTICE_WIDTH - 20.0, y + 2.0, 16.0, 16.0);
        draw_text("X", close_rect.x + 3.0, close_rect.y + 12.0, 12.0, GRAY);
        if is_mouse_over(close_rect) && is_mouse_button_pressed(MouseButton::Left) {
            self.visible = false;
        }
    }
}

// ============================================================================
// NewCharacterDialogHybrid
// ============================================================================

/// 创建角色对话框
pub struct NewCharacterDialogHybrid {
    pub visible: bool,
    pub char_name: String,
    pub selected_class: u8,
    pub selected_gender: u8,
    position: Vec2,
    drag_helper: DragHelper,
}

impl NewCharacterDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            char_name: String::new(),
            selected_class: 0,
            selected_gender: 0,
            position: Vec2::new(250.0, 120.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 绘制并处理输入
    pub fn draw(&mut self) -> Option<NewCharacterAction> {
        if !self.visible {
            return None;
        }

        let mouse = mouse_pos();
        let mut action = None;

        let title_rect = Rect::new(self.position.x, self.position.y, NEW_CHAR_WIDTH, 22.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        draw_rectangle(x, y, NEW_CHAR_WIDTH, NEW_CHAR_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.85));
        draw_rectangle_lines(x, y, NEW_CHAR_WIDTH, NEW_CHAR_HEIGHT, 1.0, DARKGRAY);
        draw_text("创建角色", x + 10.0, y + 16.0, 14.0, GOLD);

        // 名字
        draw_text("名字:", x + 10.0, y + 44.0, 12.0, WHITE);
        draw_rectangle(x + 60.0, y + 32.0, 200.0, 18.0, Color::new(0.15, 0.15, 0.15, 1.0));
        draw_text(&self.char_name, x + 64.0, y + 44.0, 11.0, WHITE);

        // 职业选择
        draw_text("职业:", x + 10.0, y + 72.0, 12.0, WHITE);
        let classes = ["战士", "法师", "道士", "刺客"];
        for (i, class_name) in classes.iter().enumerate() {
            let btn_rect = Rect::new(x + 60.0 + i as f32 * 56.0, y + 60.0, 50.0, 18.0);
            let is_selected = self.selected_class == i as u8;
            let bg = if is_selected {
                Color::new(0.3, 0.3, 0.5, 0.8)
            } else if btn_rect.contains(mouse) {
                Color::new(0.2, 0.2, 0.3, 0.5)
            } else {
                Color::new(0.1, 0.1, 0.1, 0.5)
            };
            draw_rectangle(btn_rect.x, btn_rect.y, btn_rect.w, btn_rect.h, bg);
            draw_text(class_name, btn_rect.x + 10.0, btn_rect.y + 13.0, 11.0, if is_selected { WHITE } else { GRAY });
            if is_mouse_over(btn_rect) && is_mouse_button_pressed(MouseButton::Left) {
                self.selected_class = i as u8;
            }
        }

        // 性别选择
        draw_text("性别:", x + 10.0, y + 100.0, 12.0, WHITE);
        let genders = ["男", "女"];
        for (i, gender_name) in genders.iter().enumerate() {
            let btn_rect = Rect::new(x + 60.0 + i as f32 * 56.0, y + 88.0, 50.0, 18.0);
            let is_selected = self.selected_gender == i as u8;
            let bg = if is_selected {
                Color::new(0.3, 0.3, 0.5, 0.8)
            } else if btn_rect.contains(mouse) {
                Color::new(0.2, 0.2, 0.3, 0.5)
            } else {
                Color::new(0.1, 0.1, 0.1, 0.5)
            };
            draw_rectangle(btn_rect.x, btn_rect.y, btn_rect.w, btn_rect.h, bg);
            draw_text(gender_name, btn_rect.x + 16.0, btn_rect.y + 13.0, 11.0, if is_selected { WHITE } else { GRAY });
            if is_mouse_over(btn_rect) && is_mouse_button_pressed(MouseButton::Left) {
                self.selected_gender = i as u8;
            }
        }

        // 预览区
        draw_rectangle_lines(x + 60.0, y + 116.0, 180.0, 100.0, 1.0, Color::new(0.3, 0.3, 0.3, 0.5));
        draw_text("角色预览", x + 110.0, y + 170.0, 12.0, GRAY);

        // 按钮
        let btn_y = y + NEW_CHAR_HEIGHT - 30.0;
        let create_rect = Rect::new(x + 70.0, btn_y, 60.0, 20.0);
        draw_rectangle_lines(create_rect.x, create_rect.y, create_rect.w, create_rect.h, 1.0, GRAY);
        draw_text("创建", create_rect.x + 16.0, create_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(create_rect) && is_mouse_button_pressed(MouseButton::Left) {
            action = Some(NewCharacterAction::Create {
                name: self.char_name.clone(),
                class_id: self.selected_class,
                gender: self.selected_gender,
            });
        }

        let cancel_rect = Rect::new(x + 160.0, btn_y, 60.0, 20.0);
        draw_rectangle_lines(cancel_rect.x, cancel_rect.y, cancel_rect.w, cancel_rect.h, 1.0, GRAY);
        draw_text("取消", cancel_rect.x + 16.0, cancel_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(cancel_rect) && is_mouse_button_pressed(MouseButton::Left) {
            self.visible = false;
            action = Some(NewCharacterAction::Cancel);
        }

        action
    }
}

// ============================================================================
// ItemRentalDialogHybrid
// ============================================================================

/// 物品租赁对话框
pub struct ItemRentalDialogHybrid {
    pub visible: bool,
    pub rental_items: Vec<(u32, String, u64)>,
    pub selected_index: Option<usize>,
    position: Vec2,
    drag_helper: DragHelper,
}

impl ItemRentalDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            rental_items: Vec::new(),
            selected_index: None,
            position: Vec2::new(280.0, 120.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 绘制并处理输入
    pub fn draw(&mut self) -> Option<RentalAction> {
        if !self.visible {
            return None;
        }

        let mouse = mouse_pos();
        let mut action = None;

        let title_rect = Rect::new(self.position.x, self.position.y, RENTAL_WIDTH, 22.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        draw_rectangle(x, y, RENTAL_WIDTH, RENTAL_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.85));
        draw_rectangle_lines(x, y, RENTAL_WIDTH, RENTAL_HEIGHT, 1.0, DARKGRAY);
        draw_text("物品租赁", x + 10.0, y + 16.0, 14.0, GOLD);

        for (i, (id, name, price)) in self.rental_items.iter().enumerate().take(10) {
            let row_y = y + 34.0 + i as f32 * 22.0;
            let row_rect = Rect::new(x + 10.0, row_y, RENTAL_WIDTH - 20.0, 20.0);

            let bg = if self.selected_index == Some(i) {
                Color::new(0.3, 0.3, 0.5, 0.6)
            } else if row_rect.contains(mouse) {
                Color::new(0.2, 0.2, 0.3, 0.3)
            } else {
                Color::new(0.0, 0.0, 0.0, 0.0)
            };
            draw_rectangle(row_rect.x, row_rect.y, row_rect.w, row_rect.h, bg);
            draw_text(name, x + 14.0, row_y + 14.0, 11.0, WHITE);
            draw_text(&format!("{}g", price), x + 190.0, row_y + 14.0, 11.0, GOLD);

            if is_mouse_over(row_rect) && is_mouse_button_pressed(MouseButton::Left) {
                self.selected_index = Some(i);
            }

        }

        let btn_y = y + RENTAL_HEIGHT - 30.0;
        let rent_rect = Rect::new(x + 40.0, btn_y, 60.0, 20.0);
        draw_rectangle_lines(rent_rect.x, rent_rect.y, rent_rect.w, rent_rect.h, 1.0, GRAY);
        draw_text("租赁", rent_rect.x + 16.0, rent_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(rent_rect) && is_mouse_button_pressed(MouseButton::Left) {
            if let Some(idx) = self.selected_index {
                if let Some((id, _, _)) = self.rental_items.get(idx) {
                    action = Some(RentalAction::Rent(*id));
                }
            }
        }

        let ret_rect = Rect::new(x + 120.0, btn_y, 60.0, 20.0);
        draw_rectangle_lines(ret_rect.x, ret_rect.y, ret_rect.w, ret_rect.h, 1.0, GRAY);
        draw_text("归还", ret_rect.x + 16.0, ret_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(ret_rect) && is_mouse_button_pressed(MouseButton::Left) {
            if let Some(idx) = self.selected_index {
                if let Some((id, _, _)) = self.rental_items.get(idx) {
                    action = Some(RentalAction::Return(*id));
                }
            }
        }

        let close_rect = Rect::new(x + RENTAL_WIDTH - 20.0, y + 2.0, 16.0, 16.0);
        draw_text("X", close_rect.x + 3.0, close_rect.y + 12.0, 12.0, GRAY);
        if is_mouse_over(close_rect) && is_mouse_button_pressed(MouseButton::Left) {
            self.visible = false;
            action = Some(RentalAction::Close);
        }

        action
    }
}

// ============================================================================
// TrustMerchantDialogHybrid
// ============================================================================

/// 信任商人（拍卖）对话框
pub struct TrustMerchantDialogHybrid {
    pub visible: bool,
    pub search_text: String,
    pub listings: Vec<(u32, String, u64)>,
    pub selected_index: Option<usize>,
    position: Vec2,
    drag_helper: DragHelper,
}

impl TrustMerchantDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            search_text: String::new(),
            listings: Vec::new(),
            selected_index: None,
            position: Vec2::new(240.0, 80.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 绘制并处理输入
    pub fn draw(&mut self) -> Option<TrustMerchantAction> {
        if !self.visible {
            return None;
        }

        let mouse = mouse_pos();
        let mut action = None;

        let title_rect = Rect::new(self.position.x, self.position.y, TRUST_WIDTH, 22.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        draw_rectangle(x, y, TRUST_WIDTH, TRUST_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.85));
        draw_rectangle_lines(x, y, TRUST_WIDTH, TRUST_HEIGHT, 1.0, DARKGRAY);
        draw_text("寄售商人", x + 10.0, y + 16.0, 14.0, GOLD);

        // 搜索栏
        draw_text("搜索:", x + 10.0, y + 38.0, 11.0, WHITE);
        draw_rectangle(x + 50.0, y + 26.0, 180.0, 18.0, Color::new(0.15, 0.15, 0.15, 1.0));
        draw_text(&self.search_text, x + 54.0, y + 38.0, 11.0, WHITE);

        let search_btn = Rect::new(x + 240.0, y + 26.0, 50.0, 18.0);
        draw_rectangle_lines(search_btn.x, search_btn.y, search_btn.w, search_btn.h, 1.0, GRAY);
        draw_text("搜索", search_btn.x + 12.0, search_btn.y + 13.0, 10.0, GRAY);
        if is_mouse_over(search_btn) && is_mouse_button_pressed(MouseButton::Left) {
            action = Some(TrustMerchantAction::Search(self.search_text.clone()));
        }

        // 商品列表
        draw_text("物品", x + 10.0, y + 58.0, 11.0, GOLD);
        draw_text("价格", x + 220.0, y + 58.0, 11.0, GOLD);

        for (i, (id, name, price)) in self.listings.iter().enumerate().take(12) {
            let row_y = y + 68.0 + i as f32 * 20.0;
            let row_rect = Rect::new(x + 10.0, row_y, TRUST_WIDTH - 20.0, 20.0);

            let bg = if self.selected_index == Some(i) {
                Color::new(0.3, 0.3, 0.5, 0.6)
            } else if row_rect.contains(mouse) {
                Color::new(0.2, 0.2, 0.3, 0.3)
            } else {
                Color::new(0.0, 0.0, 0.0, 0.0)
            };
            draw_rectangle(row_rect.x, row_rect.y, row_rect.w, row_rect.h, bg);
            draw_text(name, x + 14.0, row_y + 14.0, 11.0, WHITE);
            draw_text(&format!("{}g", price), x + 224.0, row_y + 14.0, 11.0, GOLD);

            if is_mouse_over(row_rect) && is_mouse_button_pressed(MouseButton::Left) {
                self.selected_index = Some(i);
            }

        }

        // 竞标按钮
        let btn_y = y + TRUST_HEIGHT - 30.0;
        let bid_rect = Rect::new(x + 100.0, btn_y, 60.0, 20.0);
        draw_rectangle_lines(bid_rect.x, bid_rect.y, bid_rect.w, bid_rect.h, 1.0, GRAY);
        draw_text("购买", bid_rect.x + 16.0, bid_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(bid_rect) && is_mouse_button_pressed(MouseButton::Left) {
            if let Some(idx) = self.selected_index {
                if let Some((id, _, price)) = self.listings.get(idx) {
                    action = Some(TrustMerchantAction::Bid { item_id: *id, amount: *price });
                }
            }
        }

        let close_rect = Rect::new(x + TRUST_WIDTH - 20.0, y + 2.0, 16.0, 16.0);
        draw_text("X", close_rect.x + 3.0, close_rect.y + 12.0, 12.0, GRAY);
        if is_mouse_over(close_rect) && is_mouse_button_pressed(MouseButton::Left) {
            self.visible = false;
            action = Some(TrustMerchantAction::Close);
        }

        action
    }
}

// ============================================================================
// IntelligentCreatureDialogHybrid
// ============================================================================

/// 灵兽管理对话框
pub struct IntelligentCreatureDialogHybrid {
    pub visible: bool,
    pub creatures: Vec<(String, u16)>,
    pub selected_index: Option<usize>,
    position: Vec2,
    drag_helper: DragHelper,
}

impl IntelligentCreatureDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            creatures: Vec::new(),
            selected_index: None,
            position: Vec2::new(300.0, 140.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 绘制并处理输入
    pub fn draw(&mut self) -> Option<CreatureAction> {
        if !self.visible {
            return None;
        }

        let mouse = mouse_pos();
        let mut action = None;

        let title_rect = Rect::new(self.position.x, self.position.y, CREATURE_WIDTH, 22.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        draw_rectangle(x, y, CREATURE_WIDTH, CREATURE_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.85));
        draw_rectangle_lines(x, y, CREATURE_WIDTH, CREATURE_HEIGHT, 1.0, DARKGRAY);
        draw_text("灵兽", x + 10.0, y + 16.0, 14.0, GOLD);

        for (i, (name, level)) in self.creatures.iter().enumerate().take(5) {
            let slot_y = y + 34.0 + i as f32 * 36.0;
            let slot_rect = Rect::new(x + 10.0, slot_y, CREATURE_WIDTH - 20.0, 32.0);

            let bg = if self.selected_index == Some(i) {
                Color::new(0.3, 0.3, 0.5, 0.5)
            } else if slot_rect.contains(mouse) {
                Color::new(0.2, 0.2, 0.3, 0.3)
            } else {
                Color::new(0.1, 0.1, 0.1, 0.3)
            };
            draw_rectangle(slot_rect.x, slot_rect.y, slot_rect.w, slot_rect.h, bg);
            draw_text(&format!("{} Lv.{}", name, level), x + 14.0, slot_y + 20.0, 11.0, WHITE);

            if is_mouse_over(slot_rect) && is_mouse_button_pressed(MouseButton::Left) {
                self.selected_index = Some(i);
            }
        }

        let btn_y = y + CREATURE_HEIGHT - 32.0;
        let summon_rect = Rect::new(x + 10.0, btn_y, 55.0, 20.0);
        draw_rectangle_lines(summon_rect.x, summon_rect.y, summon_rect.w, summon_rect.h, 1.0, GRAY);
        draw_text("召唤", summon_rect.x + 14.0, summon_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(summon_rect) && is_mouse_button_pressed(MouseButton::Left) {
            if let Some(idx) = self.selected_index {
                action = Some(CreatureAction::Summon(idx));
            }
        }

        let release_rect = Rect::new(x + 75.0, btn_y, 55.0, 20.0);
        draw_rectangle_lines(release_rect.x, release_rect.y, release_rect.w, release_rect.h, 1.0, GRAY);
        draw_text("放生", release_rect.x + 14.0, release_rect.y + 14.0, 11.0, RED);
        if is_mouse_over(release_rect) && is_mouse_button_pressed(MouseButton::Left) {
            if let Some(idx) = self.selected_index {
                action = Some(CreatureAction::Release(idx));
            }
        }

        let close_rect = Rect::new(x + CREATURE_WIDTH - 20.0, y + 2.0, 16.0, 16.0);
        draw_text("X", close_rect.x + 3.0, close_rect.y + 12.0, 12.0, GRAY);
        if is_mouse_over(close_rect) && is_mouse_button_pressed(MouseButton::Left) {
            self.visible = false;
            action = Some(CreatureAction::Close);
        }

        action
    }
}

// ============================================================================
// NPCAwakeDialogHybrid
// ============================================================================

/// 觉醒对话框
pub struct NPCAwakeDialogHybrid {
    pub visible: bool,
    pub equipment_slot: Option<usize>,
    pub material_slots: Vec<Option<usize>>,
    pub gold_cost: u64,
    position: Vec2,
    drag_helper: DragHelper,
}

impl NPCAwakeDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            equipment_slot: None,
            material_slots: vec![None; 2],
            gold_cost: 0,
            position: Vec2::new(300.0, 160.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 绘制并处理输入
    pub fn draw(&mut self) -> Option<AwakeAction> {
        if !self.visible {
            return None;
        }

        let mouse = mouse_pos();
        let mut action = None;

        let title_rect = Rect::new(self.position.x, self.position.y, AWAKE_WIDTH, 22.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        draw_rectangle(x, y, AWAKE_WIDTH, AWAKE_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.85));
        draw_rectangle_lines(x, y, AWAKE_WIDTH, AWAKE_HEIGHT, 1.0, DARKGRAY);
        draw_text("装备觉醒", x + 10.0, y + 16.0, 14.0, GOLD);

        // 装备槽
        draw_text("装备:", x + 10.0, y + 42.0, 11.0, WHITE);
        let equip_rect = Rect::new(x + 60.0, y + 30.0, CELL_SIZE, CELL_SIZE);
        let equip_hl = if equip_rect.contains(mouse) { CellHighlight::Hovered } else { CellHighlight::None };
        draw_cell_frame(equip_rect, equip_hl, &CellStyle::default());
        if self.equipment_slot.is_some() {
            draw_rectangle(equip_rect.x + 4.0, equip_rect.y + 4.0, CELL_SIZE - 8.0, CELL_SIZE - 8.0, Color::new(0.5, 0.3, 0.6, 0.6));
        }

        // 材料槽
        draw_text("材料:", x + 10.0, y + 82.0, 11.0, WHITE);
        for i in 0..2 {
            let mat_rect = Rect::new(x + 60.0 + i as f32 * (CELL_SIZE + 6.0), y + 70.0, CELL_SIZE, CELL_SIZE);
            let mat_hl = if mat_rect.contains(mouse) { CellHighlight::Hovered } else { CellHighlight::None };
            draw_cell_frame(mat_rect, mat_hl, &CellStyle::default());
            if self.material_slots[i].is_some() {
                draw_rectangle(mat_rect.x + 4.0, mat_rect.y + 4.0, CELL_SIZE - 8.0, CELL_SIZE - 8.0, Color::new(0.3, 0.5, 0.3, 0.6));
            }
        }

        // 金币
        draw_text(&format!("金币: {}", self.gold_cost), x + 10.0, y + 122.0, 11.0, GOLD);

        // 觉醒按钮
        let btn_y = y + AWAKE_HEIGHT - 32.0;
        let awaken_rect = Rect::new(x + 80.0, btn_y, 60.0, 20.0);
        draw_rectangle_lines(awaken_rect.x, awaken_rect.y, awaken_rect.w, awaken_rect.h, 1.0, GRAY);
        draw_text("觉醒", awaken_rect.x + 16.0, awaken_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(awaken_rect) && is_mouse_button_pressed(MouseButton::Left) {
            action = Some(AwakeAction::Awaken);
        }

        let close_rect = Rect::new(x + AWAKE_WIDTH - 20.0, y + 2.0, 16.0, 16.0);
        draw_text("X", close_rect.x + 3.0, close_rect.y + 12.0, 12.0, GRAY);
        if is_mouse_over(close_rect) && is_mouse_button_pressed(MouseButton::Left) {
            self.visible = false;
            action = Some(AwakeAction::Close);
        }

        action
    }
}

// ============================================================================
// ChatNoticeDialogHybrid
// ============================================================================

/// 聊天公告横幅
pub struct ChatNoticeDialogHybrid {
    pub visible: bool,
    pub message: String,
    pub scroll_offset: f32,
    position: Vec2,
}

impl ChatNoticeDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            message: String::new(),
            scroll_offset: 0.0,
            position: Vec2::new(100.0, 0.0),
        }
    }

    /// 更新滚动位置
    pub fn update_scroll(&mut self, dt: f32) {
        self.scroll_offset -= dt * 60.0;
        if self.scroll_offset < -((self.message.len() as f32) * 8.0) {
            self.scroll_offset = CHAT_NOTICE_WIDTH;
        }
    }

    /// 绘制
    pub fn draw(&self) {
        if !self.visible || self.message.is_empty() {
            return;
        }

        let x = self.position.x;
        let y = self.position.y;

        draw_rectangle(x, y, CHAT_NOTICE_WIDTH, CHAT_NOTICE_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.6));
        draw_text(&self.message, x + self.scroll_offset, y + 16.0, 12.0, YELLOW);
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_dialog_pages() {
        let mut dialog = HelpDialogHybrid::new();
        assert_eq!(dialog.page_count(), 0);

        dialog.pages = vec!["Page 1".into(), "Page 2".into(), "Page 3".into()];
        assert_eq!(dialog.page_count(), 3);
        assert_eq!(dialog.current_page, 0);
    }

    #[test]
    fn test_compass_direction_label() {
        let mut compass = CompassDialogHybrid::new();
        compass.direction = 0.0;
        assert_eq!(compass.direction_label(), "N");
        compass.direction = 90.0;
        assert_eq!(compass.direction_label(), "E");
        compass.direction = 180.0;
        assert_eq!(compass.direction_label(), "S");
        compass.direction = 270.0;
        assert_eq!(compass.direction_label(), "W");
        compass.direction = 45.0;
        assert_eq!(compass.direction_label(), "NE");
    }

    #[test]
    fn test_timer_formatted_time() {
        let mut timer = TimerDialogHybrid::new();
        timer.remaining_seconds = 125.0;
        assert_eq!(timer.formatted_time(), "02:05");
        timer.remaining_seconds = 0.0;
        assert_eq!(timer.formatted_time(), "00:00");
        timer.remaining_seconds = -5.0;
        assert_eq!(timer.formatted_time(), "00:00");
    }

    #[test]
    fn test_new_character_dialog_creation() {
        let dialog = NewCharacterDialogHybrid::new();
        assert!(!dialog.visible);
        assert!(dialog.char_name.is_empty());
        assert_eq!(dialog.selected_class, 0);
        assert_eq!(dialog.selected_gender, 0);
    }

    #[test]
    fn test_chat_notice_scroll() {
        let mut notice = ChatNoticeDialogHybrid::new();
        notice.message = "Hello World".to_string();
        notice.scroll_offset = 100.0;
        notice.update_scroll(1.0);
        assert!(notice.scroll_offset < 100.0);
    }
}
