// ============================================================================
// NPCDropDialogHybrid - NPC 物品操作对话框（对齐 C# NPCDropDialog）
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/NPCDialogs.cs:883-1316
// - 背景：Prguse[392]
// - 单物品格子 (38, 72)
// - 确认按钮：Title[520-522]
// - 保持按钮：Title[508-511]
// - 关闭按钮：Prguse2[360-362]
// - 支持 11 种面板类型：Sell/Repair/SpecialRepair/Consign/Refine 等
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::*;

// ============================================================================
// 常量
// ============================================================================

/// 物品格子位置
const ITEM_X: f32 = 38.0;
const ITEM_Y: f32 = 72.0;
/// 格子尺寸
const CELL_SIZE: f32 = 32.0;
/// 窗口尺寸
const DIALOG_WIDTH: f32 = 204.0;
const DIALOG_HEIGHT: f32 = 152.0;

// ============================================================================
// 类型定义
// ============================================================================

/// 面板类型（NPC 操作模式）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelType {
    /// 出售物品
    Sell,
    /// 购买（当前不用于 Drop 面板）
    Buy,
    /// 购买子项
    BuySub,
    /// 制作
    Craft,
    /// 普通修理
    Repair,
    /// 特殊修理（3 倍费用）
    SpecialRepair,
    /// 市场寄售
    Consign,
    /// 精炼
    Refine,
    /// 检查精炼
    CheckRefine,
    /// 分解
    Disassemble,
    /// 降级觉醒
    Downgrade,
    /// 重置附加属性
    Reset,
    /// 替换婚戒
    ReplaceWedRing,
}

impl PanelType {
    /// 返回面板标题
    pub fn title(&self) -> &'static str {
        match self {
            PanelType::Sell => "出售物品",
            PanelType::Buy | PanelType::BuySub => "购买物品",
            PanelType::Craft => "制作物品",
            PanelType::Repair => "修理物品",
            PanelType::SpecialRepair => "特殊修理",
            PanelType::Consign => "市场寄售",
            PanelType::Refine => "精炼物品",
            PanelType::CheckRefine => "检查精炼",
            PanelType::Disassemble => "分解物品",
            PanelType::Downgrade => "降级觉醒",
            PanelType::Reset => "重置属性",
            PanelType::ReplaceWedRing => "替换婚戒",
        }
    }
}

/// 放入的物品信息
#[derive(Debug, Clone)]
pub struct DropItem {
    pub icon_index: usize,
    pub name: String,
    pub unique_id: u64,
    /// 操作所需金币
    pub gold_cost: u64,
}

/// 操作结果
#[derive(Debug, Clone, PartialEq)]
pub enum DropAction {
    /// 确认操作（附带操作类型和物品 ID）
    Confirm(u64),
    /// 取消 / 关闭
    Close,
    /// 切换保持模式
    ToggleHold,
}

/// NPC 物品操作对话框
pub struct NPCDropDialogHybrid {
    pub visible: bool,
    pub panel_type: PanelType,
    pub hold: bool,
    position: Vec2,
    item: Option<DropItem>,
    // UI 纹理
    bg_texture: BackgroundTexture,
    confirm_btn: ButtonTextures,
    hold_btn: ButtonTextures,
    close_btn: CloseButton,
    // 拖动
    drag_helper: DragHelper,
    // NPC 修理费率
    npc_rate: f32,
}

impl NPCDropDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            panel_type: PanelType::Sell,
            hold: false,
            position: Vec2::new(320.0, 300.0),
            item: None,
            bg_texture: BackgroundTexture::new(),
            confirm_btn: ButtonTextures::new(),
            hold_btn: ButtonTextures::new(),
            close_btn: CloseButton::new(),
            drag_helper: DragHelper::new(),
            npc_rate: 1.0,
        }
    }

    pub fn load_textures(&mut self) {
        self.bg_texture = BackgroundTexture::load(LibraryName::Prguse, 392, None);
        self.confirm_btn = ButtonTextures::load_from_library(LibraryName::Title, 520);
        self.hold_btn = ButtonTextures::load_from_indices(LibraryName::Title, [508, 509, 511]);
        self.close_btn = CloseButton::load_prguse2();
    }

    /// 显示对话框
    pub fn show(&mut self, panel_type: PanelType, npc_rate: f32) {
        self.visible = true;
        self.panel_type = panel_type;
        self.npc_rate = npc_rate;
        self.hold = false;
        self.item = None;
    }

    /// 隐藏对话框
    pub fn hide(&mut self) {
        self.visible = false;
        self.item = None;
    }

    /// 设置当前放入的物品
    pub fn set_item(&mut self, item: DropItem) {
        self.item = Some(item);
    }

    /// 清除当前物品
    pub fn clear_item(&mut self) {
        self.item = None;
    }

    /// 获取当前物品
    pub fn current_item(&self) -> Option<&DropItem> {
        self.item.as_ref()
    }

    /// 计算 NPC 修理费用
    fn apply_npc_rate(&self, base_cost: u64, multiplier: f32) -> u64 {
        (base_cost as f32 * multiplier * self.npc_rate) as u64
    }

    /// 计算并格式化金币信息
    fn gold_info(&self) -> String {
        if let Some(item) = &self.item {
            let cost = item.gold_cost;
            match self.panel_type {
                // 出售价格 = 物品价值 / 2 (整数截断，与 C# 一致)
                PanelType::Sell => format!("出售价格: {} 金", cost / 2),
                PanelType::Repair => format!("修理费用: {} 金", self.apply_npc_rate(cost, 1.0)),
                PanelType::SpecialRepair => format!("特修费用: {} 金", self.apply_npc_rate(cost, 3.0)),
                PanelType::Consign => "设定寄售价格".to_string(),
                PanelType::Refine | PanelType::CheckRefine => format!("精炼费用: {} 金", cost),
                PanelType::Disassemble => format!("分解费用: {} 金", cost),
                PanelType::Downgrade => format!("降级费用: {} 金", cost),
                PanelType::Reset => format!("重置费用: {} 金", cost),
                PanelType::ReplaceWedRing => format!("替换费用: {} 金", cost),
                _ => String::new(),
            }
        } else {
            "请放入物品".to_string()
        }
    }

    /// 绘制并处理输入，返回动作
    pub fn draw(&mut self) -> Option<DropAction> {
        if !self.visible {
            return None;
        }

        let mouse = mouse_pos();
        let mut action = None;

        // --- 拖动 ---
        let title_rect = Rect::new(self.position.x, self.position.y, DIALOG_WIDTH, 20.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        // --- 背景 ---
        self.bg_texture.draw(vec2(x, y));

        // --- 标题 ---
        let title = self.panel_type.title();
        draw_text_cn(title, x + 10.0, y + 8.0, 13.0, GOLD);

        // --- 物品格子 ---
        let cell_rect = Rect::new(x + ITEM_X, y + ITEM_Y, CELL_SIZE, CELL_SIZE);
        let cell_highlight = if cell_rect.contains(mouse) { CellHighlight::Hovered } else { CellHighlight::None };
        draw_cell_frame(cell_rect, cell_highlight, &CellStyle { border_color: LIME, ..CellStyle::default() });

        if let Some(item) = &self.item {
            // 绘制物品图标
            if let Some(info) = LibraryName::Items.get_texture(item.icon_index) {
                if let Some(tex) = &info.image {
                    draw_item_icon(cell_rect, tex, 1.0);
                }
            }

            // 悬停工具提示
            if cell_rect.contains(mouse) {
                draw_tooltip(mouse, &item.name);
            }
        }

        // --- 金币信息 ---
        let info = self.gold_info();
        draw_text_cn(&info, x + 10.0, y + 115.0, 11.0, WHITE);

        // --- 确认按钮 ---
        let confirm_rect = Rect::new(x + 115.0, y + 120.0, self.confirm_btn.size.x.max(40.0), self.confirm_btn.size.y.max(22.0));
        if self.confirm_btn.draw_button(confirm_rect, mouse) {
            if let Some(item) = &self.item {
                action = Some(DropAction::Confirm(item.unique_id));
                if !self.hold {
                    self.item = None;
                }
            }
        }

        // --- 保持按钮 ---
        let hold_rect = Rect::new(x + 85.0, y + 42.0, self.hold_btn.size.x.max(40.0), self.hold_btn.size.y.max(22.0));
        if self.hold_btn.draw_button(hold_rect, mouse) {
            self.hold = !self.hold;
            action = Some(DropAction::ToggleHold);
        }
        if self.hold {
            draw_rectangle_lines(hold_rect.x, hold_rect.y, hold_rect.w, hold_rect.h, 1.0, LIME);
        }

        // --- 关闭按钮 ---
        let win_size = vec2(DIALOG_WIDTH, DIALOG_HEIGHT);
        if self.close_btn.draw(self.position, win_size, mouse) {
            self.visible = false;
            self.item = None;
            action = Some(DropAction::Close);
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
    fn test_npc_drop_dialog_creation() {
        let dialog = NPCDropDialogHybrid::new();
        assert!(!dialog.visible);
        assert_eq!(dialog.panel_type, PanelType::Sell);
        assert!(!dialog.hold);
        assert!(dialog.item.is_none());
    }

    #[test]
    fn test_panel_types_have_titles() {
        let types = [
            PanelType::Sell, PanelType::Buy, PanelType::Repair,
            PanelType::SpecialRepair, PanelType::Consign, PanelType::Refine,
            PanelType::Disassemble, PanelType::Downgrade, PanelType::Reset,
        ];
        for pt in &types {
            assert!(!pt.title().is_empty(), "{:?} has empty title", pt);
        }
    }

    #[test]
    fn test_show_and_hide() {
        let mut dialog = NPCDropDialogHybrid::new();
        dialog.show(PanelType::Repair, 1.5);
        assert!(dialog.visible);
        assert_eq!(dialog.panel_type, PanelType::Repair);
        assert!(!dialog.hold);

        dialog.set_item(DropItem {
            icon_index: 10,
            name: "铁剑".into(),
            unique_id: 42,
            gold_cost: 500,
        });
        assert!(dialog.current_item().is_some());

        dialog.hide();
        assert!(!dialog.visible);
        assert!(dialog.current_item().is_none());
    }

    #[test]
    fn test_gold_info_sell() {
        let mut dialog = NPCDropDialogHybrid::new();
        dialog.panel_type = PanelType::Sell;
        dialog.set_item(DropItem {
            icon_index: 1,
            name: "木盾".into(),
            unique_id: 1,
            gold_cost: 1000,
        });
        let info = dialog.gold_info();
        assert!(info.contains("500"), "sell price should be half: {}", info);
    }
}
