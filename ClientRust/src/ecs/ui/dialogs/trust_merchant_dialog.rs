// ============================================================================
// 寄售商人对话框 — TrustMerchantDialog (对应 C# TrustMerchantDialog.cs)
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;

/// 寄售物品信息
#[derive(Debug, Clone)]
pub struct ConsignmentItem {
    pub item_name: String,
    pub price: u32,
    pub seller: String,
    pub listing_date: String,
}

/// 寄售商人对话框
pub struct TrustMerchantDialog {
    pub visible: bool,
    pub position: (f32, f32),
    pub size: (f32, f32),
    /// 寄售物品列表
    pub items: Vec<ConsignmentItem>,
    /// 寄售物品槽
    pub sell_item_slot: Option<super::controls::CellItemInfo>,
    /// 设定价格
    pub sell_price: u32,
    /// 选中的物品索引
    pub selected_index: Option<usize>,
    /// 当前页码
    pub current_page: usize,
}

impl TrustMerchantDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (100.0, 100.0),
            size: (380.0, 400.0),
            items: Vec::new(),
            sell_item_slot: None,
            sell_price: 0,
            selected_index: None,
            current_page: 0,
        }
    }

    pub fn close(&mut self) { self.visible = false; }

    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult {
        if !self.visible { return Ok(()); }
        // TODO: 绘制寄售商人界面
        Ok(())
    }
}

impl Default for TrustMerchantDialog {
    fn default() -> Self {
        Self::new()
    }
}
