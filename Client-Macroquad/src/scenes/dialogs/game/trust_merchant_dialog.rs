// ============================================================================
// TrustMerchantDialogHybrid - 寄售行/拍卖行对话框
// ============================================================================
//
// C# 参考: Client/MIRScenes/Dialogs/TrustMerchantDialog.cs (~1563 行)
// - 搜索/过滤功能
// - 商品列表展示（分页）
// - 寄售/购买
// - 我的交易列表
//
// ============================================================================

use macroquad::prelude::*;
use crate::ui::text_renderer::draw_text_cn;
use mir2_shared::data::item::UserItem;

/// 拍卖行商品条目
#[derive(Debug, Clone)]
pub struct MerchantItem {
    pub item: UserItem,
    pub price: u32,
    pub seller: String,
    pub remaining_hours: u32,
}

/// 页签类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MerchantTab {
    Buy,
    Sell,
    MyListings,
}

/// PR #1156: 价格过滤 (Normal/High/Low)
/// 对齐 master C# `Shared/Enums.cs::MarketPriceFilter`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketPriceFilter {
    Normal = 0,
    High = 1,
    Low = 2,
}

impl MarketPriceFilter {
    pub fn next(self) -> Self {
        match self {
            Self::Normal => Self::High,
            Self::High => Self::Low,
            Self::Low => Self::Normal,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "正常",
            Self::High => "高价",
            Self::Low => "低价",
        }
    }
}

/// 寄售行对话框
pub struct TrustMerchantDialogHybrid {
    pub visible: bool,
    pub items: Vec<MerchantItem>,
    pub current_tab: MerchantTab,
    pub current_page: i32,
    pub total_pages: i32,
    scroll_offset: f32,
    /// PR #1156: 当前价格过滤
    pub price_filter: MarketPriceFilter,
}

impl Default for TrustMerchantDialogHybrid {
    fn default() -> Self {
        Self {
            visible: false,
            items: Vec::new(),
            current_tab: MerchantTab::Buy,
            current_page: 1,
            total_pages: 1,
            scroll_offset: 0.0,
            price_filter: MarketPriceFilter::Normal,
        }
    }
}

impl TrustMerchantDialogHybrid {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// 更新商品列表
    pub fn update_items(&mut self, items: Vec<MerchantItem>, page: i32, total: i32) {
        self.items = items;
        self.current_page = page.max(1);
        self.total_pages = total.max(1);
        self.scroll_offset = 0.0;
        // PR #1156: 重新计算过滤后的顺序
        self.apply_price_filter();
    }

    /// 设置页签
    pub fn set_tab(&mut self, tab: MerchantTab) {
        self.current_tab = tab;
        self.scroll_offset = 0.0;
    }

    /// PR #1156: 循环切换价格过滤 (Normal → High → Low → Normal)
    pub fn cycle_price_filter(&mut self) {
        self.price_filter = self.price_filter.next();
        self.apply_price_filter();
    }

    /// PR #1156: 客户端排序 (master C# 用 LINQ OrderBy;我们用 sort_by)
    fn apply_price_filter(&mut self) {
        match self.price_filter {
            MarketPriceFilter::Normal => {
                // 保持原顺序 (server 发的顺序)
            }
            MarketPriceFilter::High => {
                self.items.sort_by(|a, b| b.price.cmp(&a.price));
            }
            MarketPriceFilter::Low => {
                self.items.sort_by(|a, b| a.price.cmp(&b.price));
            }
        }
    }

    /// 绘制
    pub fn draw(&mut self, screen_w: f32, screen_h: f32, mouse_pos: Vec2,
                mouse_wheel: f32, left_clicked: bool) -> bool {
        if !self.visible {
            return false;
        }

        let padding = 15.0;
        let title_h = 30.0;
        let tab_h = 25.0;
        let item_h = 35.0;
        let page_h = 25.0;
        let dialog_w = 450.0;
        let dialog_h = 400.0;

        let dialog_x = (screen_w - dialog_w) / 2.0;
        let dialog_y = (screen_h - dialog_h) / 2.0;

        let mouse_over = mouse_pos.x >= dialog_x && mouse_pos.x <= dialog_x + dialog_w
            && mouse_pos.y >= dialog_y && mouse_pos.y <= dialog_y + dialog_h;

        if mouse_over && mouse_wheel != 0.0 {
            self.scroll_offset = (self.scroll_offset - mouse_wheel * 15.0).max(0.0);
        }

        // 背景
        draw_rectangle(dialog_x, dialog_y, dialog_w, dialog_h, Color::from_rgba(25, 25, 40, 230));

        // 标题
        draw_text_cn("寄售行", dialog_x + 15.0, dialog_y + 10.0, 16.0,
            Color::from_rgba(255, 220, 100, 255));

        // 页签栏
        let tab_y = dialog_y + title_h + padding;
        let tab_w = 80.0;
        let tab_gap = 5.0;
        let tabs = ["购买", "寄售", "我的"];
        let tab_enums = [MerchantTab::Buy, MerchantTab::Sell, MerchantTab::MyListings];

        for (i, (&label, &tab_enum)) in tabs.iter().zip(tab_enums.iter()).enumerate() {
            let tab_x = dialog_x + padding + i as f32 * (tab_w + tab_gap);
            let is_active = self.current_tab == tab_enum;
            let tab_color = if is_active {
                Color::from_rgba(80, 70, 30, 255)
            } else {
                Color::from_rgba(40, 40, 40, 200)
            };
            draw_rectangle(tab_x, tab_y, tab_w, tab_h, tab_color);
            draw_text_cn(label, tab_x + 15.0, tab_y + 5.0, 13.0, WHITE);

            if left_clicked && mouse_pos.x >= tab_x && mouse_pos.x <= tab_x + tab_w
                && mouse_pos.y >= tab_y && mouse_pos.y <= tab_y + tab_h {
                self.set_tab(tab_enum);
            }
        }

        // 商品列表
        let list_y = tab_y + tab_h + padding;
        let list_h = dialog_h - list_y - page_h - padding * 3.0;

        for (i, merchant_item) in self.items.iter().enumerate() {
            let y = list_y + i as f32 * item_h - self.scroll_offset;
            if y < list_y || y + item_h > list_y + list_h {
                continue;
            }

            let name = merchant_item.item.info.as_ref()
                .map(|info| info.name.as_str()).unwrap_or("未知物品");

            draw_text_cn(name, dialog_x + 15.0, y + 5.0, 13.0,
                Color::from_rgba(200, 200, 255, 255));
            draw_text_cn(&format!("{} 金币 | 卖家: {} | 剩余: {}h",
                merchant_item.price, merchant_item.seller, merchant_item.remaining_hours),
                dialog_x + 15.0, y + 20.0, 11.0,
                Color::from_rgba(150, 150, 150, 255));
        }

        // 分页栏
        let page_y = dialog_y + dialog_h - page_h - padding;
        draw_text_cn(&format!("第 {}/{} 页  共 {} 件商品",
            self.current_page, self.total_pages, self.items.len()),
            dialog_x + 15.0, page_y + 5.0, 12.0, WHITE);

        // PR #1156: 价格 filter label (clickable,cycles Normal→High→Low)
        // 放在分页栏右侧
        let filter_x = dialog_x + dialog_w - 130.0;
        let filter_w = 110.0;
        let mouse_over_filter = mouse_pos.x >= filter_x
            && mouse_pos.x <= filter_x + filter_w
            && mouse_pos.y >= page_y && mouse_pos.y <= page_y + page_h;
        let filter_label = self.price_filter.label();
        draw_text_cn(&format!("价格: {}", filter_label),
            filter_x, page_y + 5.0, 12.0,
            if mouse_over_filter { Color::from_rgba(255, 220, 100, 255) }
            else { Color::from_rgba(180, 180, 180, 255) });
        if left_clicked && mouse_over_filter {
            self.cycle_price_filter();
            tracing::info!("💰 TrustMerchant: price filter -> {:?}", self.price_filter);
        }

        // 关闭按钮
        let close_x = dialog_x + dialog_w - 70.0;
        let mouse_over_close = mouse_pos.x >= close_x && mouse_pos.x <= close_x + 55.0
            && mouse_pos.y >= page_y && mouse_pos.y <= page_y + page_h;
        draw_rectangle(close_x, page_y, 55.0, page_h,
            if mouse_over_close { Color::from_rgba(150, 50, 50, 255) }
            else { Color::from_rgba(100, 30, 30, 255) });
        draw_text_cn("关闭", close_x + 12.0, page_y + 5.0, 14.0, WHITE);

        if left_clicked && mouse_over_close {
            self.close();
        }

        mouse_over
    }
}
