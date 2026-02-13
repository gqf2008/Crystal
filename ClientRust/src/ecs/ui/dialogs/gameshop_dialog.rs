// ============================================================================
// 商城对话框 — GameShopDialog (对应 C# GameshopDialog.cs)
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;

/// 商城物品分类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopCategory {
    All,
    Weapon,
    Armor,
    Accessory,
    Consumable,
    Special,
}

/// 商城物品信息
#[derive(Debug, Clone)]
pub struct ShopItem {
    pub item_index: i32,
    pub name: String,
    pub price_gold: u32,
    pub price_credit: u32,
    pub category: ShopCategory,
    pub stock: i32, // -1 = 无限
    pub image: i16,
}

/// 商城对话框
pub struct GameShopDialog {
    pub visible: bool,
    pub position: (f32, f32),
    pub size: (f32, f32),
    /// 当前分类
    pub current_category: ShopCategory,
    /// 商品列表
    pub items: Vec<ShopItem>,
    /// 当前页码
    pub current_page: usize,
    /// 每页数量
    pub items_per_page: usize,
    /// 选中的商品索引
    pub selected_index: Option<usize>,
}

impl GameShopDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (100.0, 50.0),
            size: (500.0, 450.0),
            current_category: ShopCategory::All,
            items: Vec::new(),
            current_page: 0,
            items_per_page: 8,
            selected_index: None,
        }
    }

    /// 设置商品列表
    pub fn set_items(&mut self, items: Vec<ShopItem>) {
        self.items = items;
        self.current_page = 0;
        self.selected_index = None;
    }

    /// 筛选当前分类的商品
    pub fn filtered_items(&self) -> Vec<&ShopItem> {
        match self.current_category {
            ShopCategory::All => self.items.iter().collect(),
            cat => self.items.iter().filter(|i| i.category == cat).collect(),
        }
    }

    /// 总页数
    pub fn total_pages(&self) -> usize {
        let count = self.filtered_items().len();
        (count + self.items_per_page - 1) / self.items_per_page.max(1)
    }

    pub fn next_page(&mut self) {
        if self.current_page + 1 < self.total_pages() {
            self.current_page += 1;
        }
    }

    pub fn prev_page(&mut self) {
        if self.current_page > 0 {
            self.current_page -= 1;
        }
    }

    pub fn close(&mut self) { self.visible = false; }

    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult {
        if !self.visible { return Ok(()); }
        // TODO: 绘制商城界面
        Ok(())
    }

    pub fn handle_click(&mut self, _x: f32, _y: f32) -> bool {
        if !self.visible { return false; }
        // TODO: 处理分类切换、商品选择、购买
        false
    }
}

impl Default for GameShopDialog {
    fn default() -> Self {
        Self::new()
    }
}
