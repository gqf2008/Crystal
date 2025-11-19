// ============================================================================
// GameShopDialog - 基于组件系统重构的商店对话框
// ============================================================================
// 
// 【功能说明】
// 1. 使用TexturedDialog基类实现基础对话框功能
// 2. 集成ShopItemViewer实现商品预览
// 3. 统一的事件处理和状态管理
// 4. 完全基于原版Crystal客户端架构
// 
// ============================================================================

use egui_macroquad::egui;
use crate::resources::LibraryName;
use crate::ui::components::{TexturedDialog, TexturedButton, ShopItemViewer, DialogType};

/// 商店商品信息
#[derive(Debug, Clone)]
pub struct ShopItem {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub price: u32,
    pub icon_index: usize,
    pub category: String,
    pub in_stock: bool,
}

/// 商店对话框
pub struct GameShopDialog {
    /// 基础对话框
    pub dialog: TexturedDialog,
    /// 商品预览器
    pub item_viewer: ShopItemViewer,
    /// 商品列表
    pub items: Vec<ShopItem>,
    /// 当前选中的商品索引
    pub selected_item: Option<usize>,
    /// 分页相关
    pub current_page: usize,
    pub items_per_page: usize,
    /// 按钮
    pub prev_page_button: TexturedButton,
    pub next_page_button: TexturedButton,
    pub buy_button: TexturedButton,
    pub close_button: TexturedButton,
}

impl GameShopDialog {
    pub fn new() -> Self {
        // 创建基础对话框（原版GameshopDialog的尺寸和位置）
        let dialog = TexturedDialog::new("game_shop_dialog", "商店")
            .with_type(DialogType::Normal)
            .with_background(LibraryName::Title, 603) // 原版的Title[603]
            .with_rect(egui::pos2(200.0, 100.0), egui::vec2(312.0, 433.0))
            .with_close_button(None); // 使用自定义关闭按钮
        
        // 创建商品预览器
        let item_viewer = ShopItemViewer::new();
        
        // 创建按钮（基于原版位置）
        let prev_page_button = TexturedButton::new()
            .with_library(LibraryName::Prguse2)
            .with_states(202, Some(203), Some(204), None)
            .with_size(egui::vec2(16.0, 14.0))
            .with_tooltip("上一页");
            
        let next_page_button = TexturedButton::new()
            .with_library(LibraryName::Prguse2)
            .with_states(205, Some(206), Some(207), None)
            .with_size(egui::vec2(16.0, 14.0))
            .with_tooltip("下一页");
            
        let buy_button = TexturedButton::new()
            .with_library(LibraryName::Prguse2)
            .with_states(313, Some(314), Some(315), None)
            .with_size(egui::vec2(53.0, 17.0))
            .with_tooltip("购买");
            
        let close_button = TexturedButton::new()
            .with_library(LibraryName::Prguse2)
            .with_states(360, Some(361), Some(362), None)
            .with_size(egui::vec2(20.0, 20.0))
            .with_tooltip("关闭");
        
        Self {
            dialog,
            item_viewer,
            items: Vec::new(),
            selected_item: None,
            current_page: 0,
            items_per_page: 8, // 原版每页8个商品
            prev_page_button,
            next_page_button,
            buy_button,
            close_button,
        }
    }
    
    /// 加载商品数据
    pub fn load_items(&mut self, items: Vec<ShopItem>) {
        self.items = items;
        self.current_page = 0;
        self.selected_item = None;
    }
    
    /// 显示商店
    pub fn show(&mut self) {
        self.dialog.show();
    }
    
    /// 隐藏商店
    pub fn hide(&mut self) {
        self.dialog.hide();
        self.item_viewer.hide();
    }
    
    /// 是否可见
    pub fn is_visible(&self) -> bool {
        self.dialog.visible
    }
    
    /// 获取当前页商品
    fn get_current_page_items(&self) -> &[ShopItem] {
        let start = self.current_page * self.items_per_page;
        let end = (start + self.items_per_page).min(self.items.len());
        if start >= self.items.len() {
            &[]
        } else {
            &self.items[start..end]
        }
    }
    
    /// 获取总页数
    fn get_total_pages(&self) -> usize {
        if self.items.is_empty() {
            0
        } else {
            (self.items.len() + self.items_per_page - 1) / self.items_per_page
        }
    }
    
    /// 选择商品
    fn select_item(&mut self, index: usize) {
        if let Some(_item) = self.items.get(index) {
            self.selected_item = Some(index);
            
            // 显示商品预览
            self.item_viewer.current_item_index = index as i32;
            self.item_viewer.total_items = self.items.len() as i32;
            self.item_viewer.show();
        }
    }
    
    /// 购买当前选中商品
    fn buy_selected_item(&mut self) -> Option<u32> {
        if let Some(index) = self.selected_item {
            if let Some(item) = self.items.get(index) {
                if item.in_stock {
                    return Some(item.id);
                }
            }
        }
        None
    }
    
    /// 绘制对话框
    pub fn draw(&mut self, ctx: &egui::Context) -> Option<GameShopAction> {
        if !self.dialog.visible {
            return None;
        }
        
        // 绘制基础对话框
        let should_close = self.dialog.draw_base(ctx);
        if should_close {
            self.hide();
            return Some(GameShopAction::Close);
        }
        
        // 绘制对话框内容并收集操作
        let mut action = None;
        let area_response = egui::Area::new(egui::Id::new("shop_content"))
            .fixed_pos(self.dialog.position)
            .movable(false)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                // 绘制商品网格
                let grid_action = self.draw_item_grid(ui, ctx);
                
                // 绘制按钮
                let button_action = self.draw_buttons(ui, ctx);
                
                // 绘制页面信息
                self.draw_page_info(ui, ctx);
                
                // 返回第一个非None的操作
                grid_action.or(button_action)
            });
        
        action = area_response.inner;
        
        // 绘制商品预览器（独立层级）
        self.item_viewer.draw(ctx);
        
        action
    }
    
    /// 绘制商品网格
    fn draw_item_grid(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) -> Option<GameShopAction> {
        let current_items = self.get_current_page_items();
        let grid_start = egui::vec2(20.0, 50.0); // 相对位置
        let cell_size = egui::vec2(36.0, 36.0);
        let grid_cols = 4;
        let grid_spacing = egui::vec2(40.0, 40.0);
        
        for (i, item) in current_items.iter().enumerate() {
            let row = i / grid_cols;
            let col = i % grid_cols;
            let cell_pos = grid_start + egui::vec2(
                col as f32 * grid_spacing.x,
                row as f32 * grid_spacing.y
            );
            let cell_rect = egui::Rect::from_min_size(self.dialog.position + cell_pos, cell_size);
            
            // 绘制商品图标
            if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, item.icon_index) {
                if let Some(texture) = info.egui_texture {
                    ui.painter().image(
                        texture.id(),
                        cell_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
            }
            
            // 处理点击
            let response = ui.interact(
                cell_rect,
                egui::Id::new(format!("shop_item_{}", i)),
                egui::Sense::click()
            );
            
            if response.clicked() {
                let global_index = self.current_page * self.items_per_page + i;
                self.select_item(global_index);
                return Some(GameShopAction::ItemSelected(global_index));
            }
            
            // 绘制选中高亮
            if let Some(selected) = self.selected_item {
                let global_index = self.current_page * self.items_per_page + i;
                if selected == global_index {
                    ui.painter().rect_stroke(
                        cell_rect.expand(2.0),
                        2.0,
                        egui::Stroke::new(2.0, egui::Color32::YELLOW),
                        egui::epaint::StrokeKind::Outside,
                    );
                }
            }
            
            // 缺货标识
            if !item.in_stock {
                ui.painter().rect_filled(
                    cell_rect,
                    0.0,
                    egui::Color32::from_black_alpha(128),
                );
                ui.painter().text(
                    cell_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "缺货",
                    egui::FontId::proportional(10.0),
                    egui::Color32::RED,
                );
            }
        }
        
        None
    }
    
    /// 绘制按钮
    fn draw_buttons(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) -> Option<GameShopAction> {
        // 上一页按钮
        let prev_pos = egui::pos2(237.0, 380.0); // 相对位置
        let prev_rect = egui::Rect::from_min_size(self.dialog.position + prev_pos.to_vec2(), egui::vec2(16.0, 14.0));
        let mut prev_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(prev_rect)
                .layout(*ui.layout())
        );
        
        self.prev_page_button = self.prev_page_button.clone().with_enabled(self.current_page > 0);
        if self.prev_page_button.draw(&mut prev_ui) {
            if self.current_page > 0 {
                self.current_page -= 1;
                return Some(GameShopAction::PageChanged(self.current_page));
            }
        }
        
        // 下一页按钮
        let next_pos = egui::pos2(256.0, 380.0); // 相对位置
        let next_rect = egui::Rect::from_min_size(self.dialog.position + next_pos.to_vec2(), egui::vec2(16.0, 14.0));
        let mut next_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(next_rect)
                .layout(*ui.layout())
        );
        
        self.next_page_button = self.next_page_button.clone().with_enabled(self.current_page + 1 < self.get_total_pages());
        if self.next_page_button.draw(&mut next_ui) {
            if self.current_page + 1 < self.get_total_pages() {
                self.current_page += 1;
                return Some(GameShopAction::PageChanged(self.current_page));
            }
        }
        
        // 购买按钮
        let buy_pos = egui::pos2(235.0, 400.0); // 相对位置
        let buy_rect = egui::Rect::from_min_size(self.dialog.position + buy_pos.to_vec2(), egui::vec2(53.0, 17.0));
        let mut buy_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(buy_rect)
                .layout(*ui.layout())
        );
        
        self.buy_button = self.buy_button.clone().with_enabled(self.selected_item.is_some());
        if self.buy_button.draw(&mut buy_ui) {
            if let Some(item_id) = self.buy_selected_item() {
                return Some(GameShopAction::BuyItem(item_id));
            }
        }
        
        // 关闭按钮
        let close_pos = egui::pos2(280.0, 15.0); // 相对位置
        let close_rect = egui::Rect::from_min_size(self.dialog.position + close_pos.to_vec2(), egui::vec2(20.0, 20.0));
        let mut close_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(close_rect)
                .layout(*ui.layout())
        );
        
        if self.close_button.draw(&mut close_ui) {
            self.hide();
            return Some(GameShopAction::Close);
        }
        
        None
    }
    
    /// 绘制页面信息
    fn draw_page_info(&self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        let total_pages = self.get_total_pages();
        if total_pages > 1 {
            let info_pos = self.dialog.position + egui::vec2(20.0, 380.0);
            ui.painter().text(
                info_pos,
                egui::Align2::LEFT_CENTER,
                format!("第 {} / {} 页", self.current_page + 1, total_pages),
                egui::FontId::proportional(12.0),
                egui::Color32::WHITE,
            );
        }
        
        // 显示商品总数
        let count_pos = self.dialog.position + egui::vec2(20.0, 400.0);
        ui.painter().text(
            count_pos,
            egui::Align2::LEFT_CENTER,
            format!("共 {} 件商品", self.items.len()),
            egui::FontId::proportional(11.0),
            egui::Color32::from_rgb(200, 200, 200),
        );
    }
}

/// 商店对话框操作
#[derive(Debug, Clone)]
pub enum GameShopAction {
    /// 关闭对话框
    Close,
    /// 选中商品
    ItemSelected(usize),
    /// 购买商品
    BuyItem(u32),
    /// 页面改变
    PageChanged(usize),
}

impl Default for GameShopDialog {
    fn default() -> Self {
        Self::new()
    }
}