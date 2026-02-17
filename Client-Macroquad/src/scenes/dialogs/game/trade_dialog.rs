// ============================================================================
// TradeDialogHybrid - 玩家交易对话框（对齐 C# TradeDialog + GuestTradeDialog）
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/TradeDialogs.cs
// - TradeDialog：己方交易面板（5x2 = 10 格物品 + 金币输入）
//   - 背景：Prguse[389]
//   - 确认按钮：Title[520-522]
//   - 关闭按钮：Prguse2[360-362]
//   - 物品格子：5列2行，起始 (10, 39)，间距 (37, 33)
// - GuestTradeDialog：对方交易面板（只读，同样 10 格 + 金币显示）
//   - 背景：Prguse[389]
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::*;

// ============================================================================
// 常量
// ============================================================================

/// 交易格子列数
const TRADE_COLS: usize = 5;
/// 交易格子行数
const TRADE_ROWS: usize = 2;
/// 交易格子总数
const TRADE_SLOTS: usize = TRADE_COLS * TRADE_ROWS;
/// 格子尺寸
const CELL_SIZE: f32 = 32.0;
/// 格子 X 间距
const CELL_SPACING_X: f32 = 37.0;
/// 格子 Y 间距
const CELL_SPACING_Y: f32 = 33.0;
/// 格子起始偏移
const CELL_OFFSET_X: f32 = 10.0;
const CELL_OFFSET_Y: f32 = 39.0;

// ============================================================================
// 类型定义
// ============================================================================

/// 交易物品
#[derive(Debug, Clone)]
pub struct TradeItem {
    pub icon_index: usize,
    pub name: String,
    pub count: u32,
    pub unique_id: u64,
}

impl TradeItem {
    pub fn new(icon_index: usize, name: &str, count: u32, unique_id: u64) -> Self {
        Self {
            icon_index,
            name: name.to_string(),
            count,
            unique_id,
        }
    }
}

/// 交易操作事件
#[derive(Debug, Clone)]
pub enum TradeAction {
    /// 确认/锁定交易
    Confirm,
    /// 取消交易
    Cancel,
    /// 输入金币数量
    SetGold,
    /// 点击己方物品格子（用于放入物品）
    ClickOwnSlot { slot: usize },
}

/// 交易对话框（包含己方面板和对方面板）
pub struct TradeDialogHybrid {
    /// 是否可见
    visible: bool,
    /// 己方面板位置
    own_position: Vec2,
    /// 对方面板位置
    guest_position: Vec2,
    /// 是否已锁定确认
    locked: bool,
    /// 对方是否已锁定确认
    guest_locked: bool,

    // === 数据 ===
    /// 己方名称
    own_name: String,
    /// 对方名称
    guest_name: String,
    /// 己方提供金币
    own_gold: u64,
    /// 对方提供金币
    guest_gold: u64,
    /// 己方物品
    own_items: [Option<TradeItem>; TRADE_SLOTS],
    /// 对方物品
    guest_items: [Option<TradeItem>; TRADE_SLOTS],

    // === 纹理 ===
    bg_texture: BackgroundTexture,
    confirm_btn: ButtonTextures,
    close_btn: ButtonTextures,

    // === 拖动 ===
    drag_helper_own: DragHelper,
    drag_helper_guest: DragHelper,

    // === 交互 ===
    hovered_own_slot: Option<usize>,
    hovered_guest_slot: Option<usize>,
}

impl TradeDialogHybrid {
    pub fn new() -> Self {
        let sw = screen_width();
        let sh = screen_height();
        Self {
            visible: false,
            own_position: vec2(sw / 2.0 - 214.0, sh - 350.0),
            guest_position: vec2(sw / 2.0 + 10.0, sh - 350.0),
            locked: false,
            guest_locked: false,

            own_name: String::new(),
            guest_name: String::new(),
            own_gold: 0,
            guest_gold: 0,
            own_items: Default::default(),
            guest_items: Default::default(),

            bg_texture: BackgroundTexture::new(),
            confirm_btn: ButtonTextures::new(),
            close_btn: ButtonTextures::new(),

            drag_helper_own: DragHelper::new(),
            drag_helper_guest: DragHelper::new(),

            hovered_own_slot: None,
            hovered_guest_slot: None,
        }
    }

    /// 加载纹理
    pub fn load_textures(&mut self) {
        println!("💰 TradeDialog: 加载纹理...");

        // 背景 (Prguse[389])
        self.bg_texture = BackgroundTexture::load(LibraryName::Prguse, 389, None);

        // 确认按钮 (Title[520-522])
        self.confirm_btn = ButtonTextures::load_from_indices(LibraryName::Title, [520, 521, 522]);

        // 关闭按钮 (Prguse2[360-362])
        self.close_btn = ButtonTextures::load_from_indices(LibraryName::Prguse2, [360, 361, 362]);

        println!("  ✅ 交易对话框纹理加载完成");
    }

    // === 公共 API ===

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// 开始交易
    pub fn trade_accept(&mut self, own_name: &str, guest_name: &str) {
        self.own_name = own_name.to_string();
        self.guest_name = guest_name.to_string();
        self.locked = false;
        self.guest_locked = false;
        self.own_gold = 0;
        self.guest_gold = 0;
        self.own_items = Default::default();
        self.guest_items = Default::default();
        self.visible = true;
    }

    /// 重置交易状态
    pub fn trade_reset(&mut self) {
        self.locked = false;
        self.guest_locked = false;
        self.own_gold = 0;
        self.guest_gold = 0;
        self.own_items = Default::default();
        self.guest_items = Default::default();
        self.visible = false;
    }

    /// 设置己方物品
    pub fn set_own_item(&mut self, slot: usize, item: Option<TradeItem>) {
        if slot < TRADE_SLOTS {
            self.own_items[slot] = item;
        }
    }

    /// 设置对方物品
    pub fn set_guest_item(&mut self, slot: usize, item: Option<TradeItem>) {
        if slot < TRADE_SLOTS {
            self.guest_items[slot] = item;
        }
    }

    /// 设置己方金币
    pub fn set_own_gold(&mut self, gold: u64) {
        self.own_gold = gold;
    }

    /// 设置对方金币
    pub fn set_guest_gold(&mut self, gold: u64) {
        self.guest_gold = gold;
    }

    /// 设置锁定状态
    pub fn set_locked(&mut self, locked: bool) {
        self.locked = locked;
    }

    /// 设置对方锁定状态
    pub fn set_guest_locked(&mut self, locked: bool) {
        self.guest_locked = locked;
    }

    // === 绘制 ===

    /// 绘制并处理输入
    pub fn draw(&mut self) -> Option<TradeAction> {
        if !self.visible {
            return None;
        }

        let mouse_pos = vec2(mouse_position().0, mouse_position().1);
        let mut action = None;

        // 绘制己方面板
        let own_name = self.own_name.clone();
        let own_gold = self.own_gold;
        let locked = self.locked;
        if let Some(a) = self.draw_own_panel(&own_name, own_gold, locked, mouse_pos) {
            action = Some(a);
        }

        // 绘制对方面板
        self.draw_guest_panel(mouse_pos);

        action
    }

    /// 绘制己方交易面板
    fn draw_own_panel(
        &mut self,
        name: &str,
        gold: u64,
        locked: bool,
        mouse_pos: Vec2,
    ) -> Option<TradeAction> {
        let mut action = None;

        // 窗口拖动
        let bg_size = self.bg_texture.size;
        let drag_rect = Rect::new(self.own_position.x, self.own_position.y, bg_size.x, bg_size.y);
        self.own_position = self.drag_helper_own.update(drag_rect, self.own_position, mouse_pos);
        let pos = self.own_position;

        // 背景
        self.bg_texture.draw(pos);

        // 标题 — 己方名称
        draw_text_cn(
            name,
            pos.x + 30.0,
            pos.y + 20.0,
            12.0,
            WHITE,
        );

        // 锁定标识
        if locked {
            draw_text_cn("✓ 已确认", pos.x + 140.0, pos.y + 20.0, 10.0, GREEN);
        }

        // 物品格子
        self.hovered_own_slot = None;
        for col in 0..TRADE_COLS {
            for row in 0..TRADE_ROWS {
                let idx = col * TRADE_ROWS + row;
                let cell_x = pos.x + CELL_OFFSET_X + col as f32 * CELL_SPACING_X;
                let cell_y = pos.y + CELL_OFFSET_Y + row as f32 * CELL_SPACING_Y;
                let cell_rect = Rect::new(cell_x, cell_y, CELL_SIZE, CELL_SIZE);

                // 格子背景
                draw_rectangle_lines(cell_x, cell_y, CELL_SIZE, CELL_SIZE, 1.0, Color::new(0.4, 0.4, 0.4, 0.6));

                if let Some(item) = &self.own_items[idx] {
                    // 绘制物品图标
                    if let Some(info) = LibraryName::Items.get_texture(item.icon_index) {
                        if let Some(tex) = &info.image {
                            draw_texture_ex(
                                tex,
                                cell_x,
                                cell_y,
                                WHITE,
                                DrawTextureParams {
                                    dest_size: Some(vec2(CELL_SIZE, CELL_SIZE)),
                                    ..Default::default()
                                },
                            );
                        }
                    }

                    // 数量
                    if item.count > 1 {
                        let count_text = format!("{}", item.count);
                        draw_text_cn(&count_text, cell_x + 2.0, cell_y + CELL_SIZE - 2.0, 8.0, YELLOW);
                    }
                }

                // 悬停
                if cell_rect.contains(mouse_pos) {
                    self.hovered_own_slot = Some(idx);
                    draw_rectangle(cell_x, cell_y, CELL_SIZE, CELL_SIZE, Color::new(1.0, 1.0, 1.0, 0.15));

                    // 点击
                    if is_mouse_button_pressed(MouseButton::Left) && !locked {
                        action = Some(TradeAction::ClickOwnSlot { slot: idx });
                    }
                }
            }
        }

        // 金币显示
        let gold_text = if gold > 0 {
            format!("{}", gold)
        } else {
            "0".to_string()
        };
        draw_text_cn(&gold_text, pos.x + 55.0, pos.y + 133.0, 10.0, YELLOW);

        // 金币点击区域
        { // own panel
            let gold_rect = Rect::new(pos.x + 35.0, pos.y + 120.0, 90.0, 15.0);
            if gold_rect.contains(mouse_pos) && is_mouse_button_pressed(MouseButton::Left) && !locked {
                action = Some(TradeAction::SetGold);
            }
        }

        // 确认按钮
        let btn_x = pos.x + 135.0;
        let btn_y = pos.y + 120.0;
        let btn_rect = Rect::new(btn_x, btn_y, 48.0, 25.0);
        let btn_state = ButtonState::from_mouse(btn_rect, mouse_pos);
        self.confirm_btn.draw(vec2(btn_x, btn_y), btn_state);
        if ButtonState::is_clicked(btn_rect, mouse_pos) {
            action = Some(TradeAction::Confirm);
        }

        // 关闭按钮
        let close_x = pos.x + bg_size.x - 23.0;
        let close_y = pos.y + 3.0;
        let close_rect = Rect::new(close_x, close_y, 20.0, 20.0);
        let close_state = ButtonState::from_mouse(close_rect, mouse_pos);
        self.close_btn.draw(vec2(close_x, close_y), close_state);
        if ButtonState::is_clicked(close_rect, mouse_pos) {
            action = Some(TradeAction::Cancel);
        }

        // 工具提示
        if let Some(idx) = self.hovered_own_slot {
            if let Some(item) = &self.own_items[idx] {
                draw_trade_tooltip(mouse_pos, &item.name, item.count);
            }
        }

        action
    }

    /// 绘制对方交易面板（只读）
    fn draw_guest_panel(&mut self, mouse_pos: Vec2) {
        let bg_size = self.bg_texture.size;
        let drag_rect = Rect::new(self.guest_position.x, self.guest_position.y, bg_size.x, bg_size.y);
        self.guest_position = self.drag_helper_guest.update(drag_rect, self.guest_position, mouse_pos);
        let pos = self.guest_position;

        // 背景
        self.bg_texture.draw(pos);

        // 标题 — 对方名称
        draw_text_cn(
            &self.guest_name,
            pos.x + 30.0,
            pos.y + 20.0,
            12.0,
            WHITE,
        );

        // 锁定标识
        if self.guest_locked {
            draw_text_cn("✓ 已确认", pos.x + 140.0, pos.y + 20.0, 10.0, GREEN);
        }

        // 物品格子（只读）
        self.hovered_guest_slot = None;
        for col in 0..TRADE_COLS {
            for row in 0..TRADE_ROWS {
                let idx = col * TRADE_ROWS + row;
                let cell_x = pos.x + CELL_OFFSET_X + col as f32 * CELL_SPACING_X;
                let cell_y = pos.y + CELL_OFFSET_Y + row as f32 * CELL_SPACING_Y;
                let cell_rect = Rect::new(cell_x, cell_y, CELL_SIZE, CELL_SIZE);

                draw_rectangle_lines(cell_x, cell_y, CELL_SIZE, CELL_SIZE, 1.0, Color::new(0.4, 0.4, 0.4, 0.6));

                if let Some(item) = &self.guest_items[idx] {
                    if let Some(info) = LibraryName::Items.get_texture(item.icon_index) {
                        if let Some(tex) = &info.image {
                            draw_texture_ex(
                                tex,
                                cell_x,
                                cell_y,
                                WHITE,
                                DrawTextureParams {
                                    dest_size: Some(vec2(CELL_SIZE, CELL_SIZE)),
                                    ..Default::default()
                                },
                            );
                        }
                    }

                    if item.count > 1 {
                        let count_text = format!("{}", item.count);
                        draw_text_cn(&count_text, cell_x + 2.0, cell_y + CELL_SIZE - 2.0, 8.0, YELLOW);
                    }
                }

                if cell_rect.contains(mouse_pos) {
                    self.hovered_guest_slot = Some(idx);
                    draw_rectangle(cell_x, cell_y, CELL_SIZE, CELL_SIZE, Color::new(1.0, 1.0, 1.0, 0.1));
                }
            }
        }

        // 金币
        let gold_text = if self.guest_gold > 0 {
            format!("{}", self.guest_gold)
        } else {
            "0".to_string()
        };
        draw_text_cn(&gold_text, pos.x + 55.0, pos.y + 133.0, 10.0, YELLOW);

        // 工具提示
        if let Some(idx) = self.hovered_guest_slot {
            if let Some(item) = &self.guest_items[idx] {
                draw_trade_tooltip(mouse_pos, &item.name, item.count);
            }
        }
    }
}

/// 绘制物品工具提示
fn draw_trade_tooltip(mouse_pos: Vec2, name: &str, count: u32) {
    let tooltip = if count > 1 {
        format!("{}\n数量: {}", name, count)
    } else {
        name.to_string()
    };
    let tip_x = mouse_pos.x + 15.0;
    let tip_y = mouse_pos.y + 15.0;
    let lines: Vec<&str> = tooltip.lines().collect();
    let tip_w = 160.0;
    let tip_h = lines.len() as f32 * 16.0 + 8.0;

    draw_rectangle(tip_x, tip_y, tip_w, tip_h, Color::new(0.0, 0.0, 0.0, 0.85));
    draw_rectangle_lines(tip_x, tip_y, tip_w, tip_h, 1.0, Color::new(0.6, 0.6, 0.6, 0.8));

    for (j, line) in lines.iter().enumerate() {
        draw_text_cn(
            line,
            tip_x + 6.0,
            tip_y + 14.0 + j as f32 * 16.0,
            12.0,
            WHITE,
        );
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trade_item_creation() {
        let item = TradeItem::new(42, "大太阳水", 10, 12345);
        assert_eq!(item.icon_index, 42);
        assert_eq!(item.name, "大太阳水");
        assert_eq!(item.count, 10);
        assert_eq!(item.unique_id, 12345);
    }

    #[test]
    fn test_trade_dialog_accept_reset() {
        let mut dialog = TradeDialogHybrid::new();
        assert!(!dialog.visible());

        dialog.trade_accept("玩家A", "玩家B");
        assert!(dialog.visible());
        assert!(!dialog.locked);
        assert!(!dialog.guest_locked);
        assert_eq!(dialog.own_gold, 0);
        assert_eq!(dialog.guest_gold, 0);

        dialog.set_own_gold(1000);
        dialog.set_locked(true);
        assert_eq!(dialog.own_gold, 1000);
        assert!(dialog.locked);

        dialog.trade_reset();
        assert!(!dialog.visible());
        assert!(!dialog.locked);
        assert_eq!(dialog.own_gold, 0);
    }

    #[test]
    fn test_trade_slots() {
        let mut dialog = TradeDialogHybrid::new();
        dialog.set_own_item(0, Some(TradeItem::new(1, "药品", 5, 100)));
        dialog.set_guest_item(3, Some(TradeItem::new(2, "武器", 1, 200)));

        assert!(dialog.own_items[0].is_some());
        assert_eq!(dialog.own_items[0].as_ref().unwrap().name, "药品");
        assert!(dialog.guest_items[3].is_some());
        assert_eq!(dialog.guest_items[3].as_ref().unwrap().name, "武器");

        // Out of bounds should be safe
        dialog.set_own_item(TRADE_SLOTS + 1, Some(TradeItem::new(1, "test", 1, 1)));
    }
}
