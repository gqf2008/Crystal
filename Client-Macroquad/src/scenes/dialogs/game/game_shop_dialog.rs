// ============================================================================
// GameShopDialog - 基于新组件系统的商城对话框
// ============================================================================
// 
// 【功能说明】
// 1. 使用MirDialog和MirButton组件实现
// 2. 集成ShopItemViewer实现商品预览
// 3. 统一的状态管理和事件处理
// 4. 完全兼容原版Crystal客户端架构
// 
// ============================================================================

use egui_macroquad::egui;
use crate::resources::LibraryName;
use crate::scenes::dialogs::Dialog;

/// 商城主要分类页 (Section Tabs)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameShopSection {
    All,        // 全部商品
    TopItems,   // 热销商品
    Deals,      // 特价商品
    New,        // 新品
}

impl GameShopSection {
    pub const ALL: &'static [GameShopSection] = &[
        GameShopSection::All,
        GameShopSection::TopItems,
        GameShopSection::Deals,
        GameShopSection::New,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            GameShopSection::All => "全部商品",
            GameShopSection::TopItems => "热销商品",
            GameShopSection::Deals => "特价商品",
            GameShopSection::New => "新品",
        }
    }
}

/// 商城职业分类 (Class Tabs)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameShopClass {
    All,        // 全职业
    Warrior,    // 战士
    Assassin,   // 刺客
    Taoist,     // 道士
    Wizard,     // 法师
    Archer,     // 弓箭手
}

impl GameShopClass {
    pub const ALL: &'static [GameShopClass] = &[
        GameShopClass::All,
        GameShopClass::Warrior,
        GameShopClass::Assassin,
        GameShopClass::Taoist,
        GameShopClass::Wizard,
        GameShopClass::Archer,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            GameShopClass::All => "全部",
            GameShopClass::Warrior => "战士",
            GameShopClass::Assassin => "刺客",
            GameShopClass::Taoist => "道士",
            GameShopClass::Wizard => "法师",
            GameShopClass::Archer => "弓箭手",
        }
    }
}

/// 商城分类
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShopCategory {
    Weapon,    // 武器
    Armor,     // 防具
    Potion,    // 药品
    Special,   // 特殊
    Fashion,   // 时装
}

/// 商城物品信息
#[derive(Debug, Clone)]
pub struct ShopItem {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub icon_index: usize,
    pub price_gold: u32,      // 金币价格
    pub price_ingot: u32,     // 元宝价格
    pub category: ShopCategory,
    pub in_stock: bool,       // 是否有库存
    pub hot: bool,           // 是否热销
    pub new: bool,           // 是否新品
}

/// 商品预览状态
#[derive(Debug, Clone)]
pub struct ShopItemViewer {
    /// 预览的物品
    pub item: ShopItem,
    /// 当前查看方向 (1-8)
    pub direction: u8,
    /// 是否可见
    pub visible: bool,
    /// 位置（动态调整，避免超出边界）
    pub position: egui::Pos2,
    /// 是否正在拖拽
    pub dragging: bool,
    /// 拖拽偏移量
    pub drag_offset: egui::Vec2,
}

impl ShopItemViewer {
    pub fn new(item: ShopItem, position: egui::Pos2) -> Self {
        Self {
            item,
            direction: 6, // 默认方向
            visible: true,
            position,
            dragging: false,
            drag_offset: egui::Vec2::ZERO,
        }
    }
}

/// 游戏商城对话框
pub struct GameShopDialog {
    /// 是否可见
    visible: bool,
    /// 窗口位置
    position: egui::Pos2,
    /// 是否正在拖拽
    dragging: bool,
    /// 拖拽偏移
    drag_offset: egui::Vec2,
    /// 当前选中的主要分类
    selected_section: GameShopSection,
    /// 当前选中的职业分类
    selected_class: GameShopClass,
    /// 商城物品列表
    shop_items: Vec<ShopItem>,
    /// 过滤后的物品列表
    filtered_items: Vec<ShopItem>,
    /// 滚动偏移
    scroll_offset: f32,
    /// 选中的物品索引
    selected_item: Option<usize>,
    /// 商品预览器
    item_viewer: Option<ShopItemViewer>,
    /// 购买数量
    buy_quantity: u32,
    /// 玩家金币
    player_gold: u32,
    /// 玩家元宝
    player_ingot: u32,
    /// 当前页面
    current_page: usize,
    /// 每页显示物品数量 (4x2 = 8个)
    items_per_page: usize,
}

impl GameShopDialog {
    pub fn new() -> Self {
        // 创建一些示例商城物品
        let shop_items = vec![
            ShopItem {
                id: 1,
                name: "龙纹剑".to_string(),
                description: "强力的单手剑，攻击力+50".to_string(),
                icon_index: 1,
                price_gold: 100000,
                price_ingot: 500,
                category: ShopCategory::Weapon,
                in_stock: true,
                hot: true,
                new: false,
            },
            ShopItem {
                id: 2,
                name: "天师道袍".to_string(),
                description: "高级法师袍，魔法防御+30".to_string(),
                icon_index: 20,
                price_gold: 80000,
                price_ingot: 400,
                category: ShopCategory::Armor,
                in_stock: true,
                hot: false,
                new: true,
            },
            ShopItem {
                id: 3,
                name: "强效金疮药".to_string(),
                description: "瞬间恢复500点生命值".to_string(),
                icon_index: 40,
                price_gold: 1000,
                price_ingot: 5,
                category: ShopCategory::Potion,
                in_stock: true,
                hot: false,
                new: false,
            },
            ShopItem {
                id: 4,
                name: "传送戒指".to_string(),
                description: "可以传送到任意地点的神奇戒指".to_string(),
                icon_index: 60,
                price_gold: 500000,
                price_ingot: 2000,
                category: ShopCategory::Special,
                in_stock: false,
                hot: true,
                new: true,
            },
            ShopItem {
                id: 5,
                name: "华丽时装".to_string(),
                description: "美观的装饰性服装".to_string(),
                icon_index: 80,
                price_gold: 0,
                price_ingot: 1000,
                category: ShopCategory::Fashion,
                in_stock: true,
                hot: false,
                new: true,
            },
        ];

        let mut dialog = Self {
            visible: false,
            position: egui::pos2(300.0, 150.0),
            dragging: false,
            drag_offset: egui::Vec2::ZERO,
            selected_section: GameShopSection::All,
            selected_class: GameShopClass::All,
            shop_items,
            filtered_items: Vec::new(),
            scroll_offset: 0.0,
            selected_item: None,
            item_viewer: None,
            buy_quantity: 1,
            player_gold: 999999,
            player_ingot: 10000,
            current_page: 0,
            items_per_page: 8, // 4x2网格
        };
        
        // 初始化过滤的物品列表
        dialog.filter_items();
        dialog
    }

    /// 切换可见性
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        println!("🛒 商城对话框: {}", if self.visible { "打开" } else { "关闭" });
    }

    /// 获取可见性
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// 设置可见性
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// 绘制对话框背景
    fn draw_background(&self, ui: &mut egui::Ui, ctx: &egui::Context) -> egui::Rect {
        // 使用原版传奇2商城背景纹理 (Title[749] - 从原版代码获取)
        if let Some(info) = LibraryName::Title.get_egui_texture(ctx, 749) {
            if let Some(bg_texture) = info.egui_texture {
                let bg_size = bg_texture.size_vec2();
                let bg_rect = egui::Rect::from_min_size(self.position, bg_size);
                
                ui.painter().image(
                    bg_texture.id(),
                    bg_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                
                return bg_rect;
            }
        }
        
        // 降级：使用自定义背景
        let bg_size = egui::vec2(500.0, 400.0);
        let bg_rect = egui::Rect::from_min_size(self.position, bg_size);
        
        ui.painter().rect_filled(
            bg_rect,
            5.0,
            egui::Color32::from_rgba_premultiplied(30, 30, 40, 240),
        );

        // 绘制标题
        ui.painter().text(
            egui::pos2(bg_rect.min.x + 20.0, bg_rect.min.y + 15.0),
            egui::Align2::LEFT_CENTER,
            "🛒 游戏商城",
            egui::FontId::proportional(16.0),
            egui::Color32::from_rgb(255, 215, 0),
        );

        bg_rect
    }

    /// 过滤物品列表
    fn filter_items(&mut self) {
        self.filtered_items = self.shop_items
            .iter()
            .filter(|item| {
                // 根据选中的分类过滤
                let section_match = match self.selected_section {
                    GameShopSection::All => true,
                    GameShopSection::TopItems => item.hot,
                    GameShopSection::Deals => item.price_gold > 0 && item.price_ingot > 0, // 假设特价是同时有金币和元宝价格
                    GameShopSection::New => item.new,
                };
                
                // 根据职业过滤 (目前简单实现，可以根据需要扩展)
                let class_match = match self.selected_class {
                    GameShopClass::All => true,
                    _ => true, // 暂时允许所有职业访问所有物品
                };
                
                section_match && class_match
            })
            .cloned()
            .collect();
    }

    /// 绘制分类标签页
    fn draw_category_tabs(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // Section tabs (主要tab页) - 位置基于原版：138, 68开始
        let section_y = bg_rect.min.y + 68.0;
        
        for (i, section) in GameShopSection::ALL.iter().enumerate() {
            let is_selected = self.selected_section == *section;
            let texture_index = match section {
                GameShopSection::All => if is_selected { 771 } else { 770 },
                GameShopSection::TopItems => if is_selected { 777 } else { 776 },
                GameShopSection::Deals => if is_selected { 773 } else { 772 },
                GameShopSection::New => if is_selected { 775 } else { 774 },
            };
            
            let tab_x = bg_rect.min.x + 138.0 + (i as f32 * 71.0);
            let tab_pos = egui::pos2(tab_x, section_y);
            
            // 使用纹理渲染tab按钮
            if let Some(info) = LibraryName::Title.get_egui_texture(ctx, texture_index) {
                if let Some(tab_texture) = info.egui_texture {
                    let tab_size = egui::vec2(71.0, 23.0);
                    let tab_rect = egui::Rect::from_min_size(tab_pos, tab_size);
                    
                    ui.painter().image(
                        tab_texture.id(),
                        tab_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                    
                    let response = ui.interact(tab_rect, egui::Id::new(format!("section_tab_{}", i)), egui::Sense::click());
                    if response.clicked() && !self.dragging {
                        self.selected_section = *section;
                        self.selected_class = GameShopClass::All;
                        self.current_page = 0;
                        self.selected_item = None;
                        self.item_viewer = None; // 关闭预览器
                        self.filter_items();
                        println!("🏷️ 切换到分类: {}", section.display_name());
                    }
                } else {
                    // 备用文本按钮
                    self.draw_fallback_section_tab(ui, tab_pos, section, is_selected, i);
                }
            } else {
                // 备用文本按钮
                self.draw_fallback_section_tab(ui, tab_pos, section, is_selected, i);
            }
        }
        
        // Class tabs (职业tab页) - 位置基于原版：539, 37开始
        let class_y = bg_rect.min.y + 38.0;
        
        for (i, class) in GameShopClass::ALL.iter().enumerate() {
            let is_selected = self.selected_class == *class;
            let texture_index = match class {
                GameShopClass::All => if is_selected { 752 } else { 751 },
                GameShopClass::Warrior => if is_selected { 755 } else { 754 },
                GameShopClass::Assassin => if is_selected { 758 } else { 757 },
                GameShopClass::Taoist => if is_selected { 761 } else { 760 },
                GameShopClass::Wizard => if is_selected { 764 } else { 763 },
                GameShopClass::Archer => if is_selected { 767 } else { 766 },
            };
            
            let tab_x = bg_rect.min.x + 539.0 + (i as f32 * 23.0);
            let tab_pos = egui::pos2(tab_x, class_y);
            
            // 使用纹理渲染tab按钮
            if let Some(info) = LibraryName::Title.get_egui_texture(ctx, texture_index) {
                if let Some(tab_texture) = info.egui_texture {
                    let tab_size = egui::vec2(23.0, 20.0);
                    let tab_rect = egui::Rect::from_min_size(tab_pos, tab_size);
                    
                    ui.painter().image(
                        tab_texture.id(),
                        tab_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                    
                    let response = ui.interact(tab_rect, egui::Id::new(format!("class_tab_{}", i)), egui::Sense::click());
                    if response.clicked() && !self.dragging {
                        self.selected_class = *class;
                        self.current_page = 0;
                        self.filter_items();
                        println!("🏷️ 切换到职业: {}", class.display_name());
                    }
                } else {
                    // 备用文本按钮
                    self.draw_fallback_class_tab(ui, tab_pos, class, is_selected, i);
                }
            } else {
                // 备用文本按钮
                self.draw_fallback_class_tab(ui, tab_pos, class, is_selected, i);
            }
        }
    }
    
    /// 绘制备用的分类tab
    fn draw_fallback_section_tab(&mut self, ui: &mut egui::Ui, pos: egui::Pos2, section: &GameShopSection, is_selected: bool, index: usize) {
        let tab_size = egui::vec2(71.0, 23.0);
        let tab_rect = egui::Rect::from_min_size(pos, tab_size);
        
        let bg_color = if is_selected {
            egui::Color32::from_rgb(230, 200, 160)
        } else {
            egui::Color32::from_rgb(80, 80, 120)
        };
        
        ui.painter().rect_filled(tab_rect, 3.0, bg_color);
        ui.painter().text(
            tab_rect.center(),
            egui::Align2::CENTER_CENTER,
            section.display_name(),
            egui::FontId::proportional(10.0),
            egui::Color32::WHITE,
        );
        
        let response = ui.interact(tab_rect, egui::Id::new(format!("section_tab_fallback_{}", index)), egui::Sense::click());
        if response.clicked() {
            self.selected_section = *section;
            self.selected_class = GameShopClass::All;
            self.current_page = 0;
            self.filter_items();
        }
    }
    
    /// 绘制备用的职业tab
    fn draw_fallback_class_tab(&mut self, ui: &mut egui::Ui, pos: egui::Pos2, class: &GameShopClass, is_selected: bool, index: usize) {
        let tab_size = egui::vec2(23.0, 20.0);
        let tab_rect = egui::Rect::from_min_size(pos, tab_size);
        
        let bg_color = if is_selected {
            egui::Color32::from_rgb(230, 200, 160)
        } else {
            egui::Color32::from_rgb(80, 80, 120)
        };
        
        ui.painter().rect_filled(tab_rect, 3.0, bg_color);
        ui.painter().text(
            tab_rect.center(),
            egui::Align2::CENTER_CENTER,
            &class.display_name()[0..2], // 只显示前两个字符
            egui::FontId::proportional(8.0),
            egui::Color32::WHITE,
        );
        
        let response = ui.interact(tab_rect, egui::Id::new(format!("class_tab_fallback_{}", index)), egui::Sense::click());
        if response.clicked() {
            self.selected_class = *class;
            self.current_page = 0;
            self.filter_items();
        }
    }

    /// 绘制物品网格 (4x2布局，基于原版尺寸125x146)
    fn draw_item_grid(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 原版网格布局：第一行位置152,115，第二行位置152,275，每个单元格间距132
        let grid_start_x = bg_rect.min.x + 152.0;
        let grid_row1_y = bg_rect.min.y + 115.0;
        let grid_row2_y = bg_rect.min.y + 275.0;
        let cell_width = 125.0;
        let cell_height = 146.0;
        let cell_spacing = 132.0;

        // 计算当前页显示的物品
        let start_index = self.current_page * self.items_per_page;
        let selected_item = self.selected_item;
        
        // 绘制4x2网格
        for i in 0..self.items_per_page {
            let item_index = start_index + i;
            let has_item = item_index < self.filtered_items.len();
            
            // 计算网格位置
            let grid_x = if i < 4 { 
                grid_start_x + (i as f32 * cell_spacing) 
            } else { 
                grid_start_x + ((i - 4) as f32 * cell_spacing) 
            };
            let grid_y = if i < 4 { grid_row1_y } else { grid_row2_y };
            
            let cell_rect = egui::Rect::from_min_size(
                egui::pos2(grid_x, grid_y),
                egui::vec2(cell_width, cell_height)
            );
            
            if has_item {
                let item = &self.filtered_items[item_index];
                // 使用Title[750]纹理作为单元格背景
                if let Some(info) = LibraryName::Title.get_egui_texture(ctx, 750) {
                    if let Some(cell_texture) = info.egui_texture {
                        ui.painter().image(
                            cell_texture.id(),
                            cell_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                } else {
                    // 备用背景
                    ui.painter().rect_filled(
                        cell_rect,
                        3.0,
                        egui::Color32::from_rgb(60, 60, 70),
                    );
                    ui.painter().rect_stroke(
                        cell_rect,
                        3.0,
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 100)),
                        egui::epaint::StrokeKind::Outside,
                    );
                }
                
                // 物品图标区域 (居中显示)
                let icon_size = 64.0;
                let icon_rect = egui::Rect::from_center_size(
                    egui::pos2(cell_rect.center().x, cell_rect.min.y + 50.0),
                    egui::vec2(icon_size, icon_size)
                );
                
                // 绘制物品图标背景
                ui.painter().rect_filled(
                    icon_rect,
                    2.0,
                    egui::Color32::from_rgba_premultiplied(40, 40, 50, 180)
                );
                
                // 物品名称
                ui.painter().text(
                    egui::pos2(cell_rect.center().x, cell_rect.min.y + 90.0),
                    egui::Align2::CENTER_TOP,
                    &item.name,
                    egui::FontId::proportional(11.0),
                    if item.in_stock { egui::Color32::WHITE } else { egui::Color32::GRAY },
                );
                
                // 价格信息
                let price_text = if item.price_gold > 0 && item.price_ingot > 0 {
                    format!("{}金 | {}宝", item.price_gold, item.price_ingot)
                } else if item.price_gold > 0 {
                    format!("{}金币", item.price_gold)
                } else {
                    format!("{}元宝", item.price_ingot)
                };
                
                ui.painter().text(
                    egui::pos2(cell_rect.center().x, cell_rect.min.y + 110.0),
                    egui::Align2::CENTER_TOP,
                    &price_text,
                    egui::FontId::proportional(9.0),
                    egui::Color32::from_rgb(255, 215, 0),
                );
                
                // 状态标签
                if item.hot {
                    ui.painter().text(
                        egui::pos2(cell_rect.max.x - 10.0, cell_rect.min.y + 10.0),
                        egui::Align2::RIGHT_TOP,
                        "🔥",
                        egui::FontId::proportional(12.0),
                        egui::Color32::RED,
                    );
                }
                if item.new {
                    ui.painter().text(
                        egui::pos2(cell_rect.min.x + 10.0, cell_rect.min.y + 10.0),
                        egui::Align2::LEFT_TOP,
                        "NEW",
                        egui::FontId::proportional(8.0),
                        egui::Color32::GREEN,
                    );
                }
                
                // 库存状态
                if !item.in_stock {
                    ui.painter().text(
                        egui::pos2(cell_rect.center().x, cell_rect.min.y + 125.0),
                        egui::Align2::CENTER_TOP,
                        "缺货",
                        egui::FontId::proportional(10.0),
                        egui::Color32::RED,
                    );
                }
                
                // 点击交互 - 使用item的唯一ID避免冲突，添加调试信息
                let cell_id = format!("shop_cell_{}_{}_grid_{}", item.id, item_index, i);
                let response = ui.interact(cell_rect, egui::Id::new(&cell_id), egui::Sense::click());
                if response.clicked() && !self.dragging {
                    println!("🖱️ 点击商品: {} (index: {}, grid: {}, id: {})", item.name, item_index, i, cell_id);
                    
                    // 如果点击的是已选中的商品，关闭预览器
                    if Some(item_index) == self.selected_item && self.item_viewer.is_some() {
                        self.item_viewer = None;
                        self.selected_item = None;
                        println!("❌ 关闭商品预览: {}", item.name);
                    } else {
                        // 否则显示新的预览器
                        self.selected_item = Some(item_index);
                        
                        // 创建商品预览器 - 调整位置避免遮挡重要UI元素
                        let viewer_size = egui::vec2(260.0, 300.0);
                        // 使用网格索引而不是坐标判断位置，更准确
                        let viewer_pos = if i % 4 < 2 {
                            // 如果是左半部分（第0,1列），预览器显示在右侧
                            let right_x = bg_rect.max.x - viewer_size.x - 30.0; // 留出关闭按钮空间
                            egui::pos2(right_x, bg_rect.min.y + 120.0)
                        } else {
                            // 如果是右半部分（第2,3列），预览器显示在左侧
                            egui::pos2(bg_rect.min.x + 50.0, bg_rect.min.y + 120.0)
                        };
                        
                        self.item_viewer = Some(ShopItemViewer::new(item.clone(), viewer_pos));
                        println!("🛍️ 选择商品: {} - 显示预览 (位置: {:?})", item.name, viewer_pos);
                    }
                }
                
                // 选中高亮
                if Some(item_index) == selected_item {
                    ui.painter().rect_stroke(
                        cell_rect,
                        3.0,
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 215, 0)),
                        egui::epaint::StrokeKind::Outside,
                    );
                }
            } else {
                // 绘制空单元格
                ui.painter().rect_filled(
                    cell_rect,
                    3.0,
                    egui::Color32::from_rgba_premultiplied(30, 30, 40, 100),
                );
                ui.painter().rect_stroke(
                    cell_rect,
                    3.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 60)),
                    egui::epaint::StrokeKind::Outside,
                );
            }
        }
        
        // 绘制分页控制
        self.draw_pagination(ui, bg_rect);
    }
    

    
    /// 绘制商品预览器 - 返回是否需要关闭
    fn draw_item_viewer_impl(ui: &mut egui::Ui, ctx: &egui::Context, viewer: &mut ShopItemViewer, bg_rect: &egui::Rect) -> bool {
        // 使用原版GameShopViewer的纹理索引785，尺寸约为260x300
        let viewer_size = egui::vec2(260.0, 300.0);
        
        // 预处理位置，稍后会根据拖拽调整
        let mut final_viewer_rect = egui::Rect::from_min_size(viewer.position, viewer_size);
        
        // 处理预览器拖拽（标题栏区域） - 优化性能
        let title_area = egui::Rect::from_min_size(
            final_viewer_rect.min,
            egui::vec2(viewer_size.x, 30.0) // 标题栏高度
        );
        
        // 使用更高优先级的ID避免与主对话框拖拽冲突
        let viewer_drag_id = format!("viewer_title_drag_{}", viewer.item.id);
        let title_response = ui.interact(
            title_area, 
            egui::Id::new(&viewer_drag_id), 
            egui::Sense::drag()
        );
        
        // 优化拖拽性能：只在需要时更新位置，避免与主对话框拖拽冲突
        if title_response.drag_started() && !viewer.dragging {
            viewer.dragging = true;
            println!("🔄 开始拖拽预览器: {}", viewer.item.name);
            if let Some(pointer_pos) = ctx.input(|i| i.pointer.hover_pos()) {
                viewer.drag_offset = viewer.position.to_vec2() - pointer_pos.to_vec2();
            }
        } else if title_response.dragged() && viewer.dragging {
            // 只有在预览器拖拽状态下才响应拖拽
            viewer.position += title_response.drag_delta();
        } else if viewer.dragging && (!ctx.input(|i| i.pointer.primary_down()) || title_response.drag_stopped()) {
            viewer.dragging = false;
            println!("🔄 停止拖拽预览器: {}", viewer.item.name);
        }
        
        // 确保预览器不会超出主窗口边界
        if final_viewer_rect.max.x > bg_rect.max.x {
            viewer.position.x = bg_rect.max.x - viewer_size.x - 10.0;
        }
        if final_viewer_rect.max.y > bg_rect.max.y {
            viewer.position.y = bg_rect.max.y - viewer_size.y - 10.0;
        }
        if viewer.position.x < bg_rect.min.x {
            viewer.position.x = bg_rect.min.x;
        }
        if viewer.position.y < bg_rect.min.y {
            viewer.position.y = bg_rect.min.y;
        }
        
        // 更新最终绘制矩形
        final_viewer_rect = egui::Rect::from_min_size(viewer.position, viewer_size);
        
        // ⚠️ 重要：不要对整个预览器区域添加交互，避免阻挡其他UI元素
        
        // 绘制预览器背景（使用Title[785]纹理 - 原版商品预览窗口）
        if let Some(info) = LibraryName::Title.get_egui_texture(ctx, 785) {
            if let Some(viewer_texture) = info.egui_texture {
                ui.painter().image(
                    viewer_texture.id(),
                    final_viewer_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            } else {
                println!("⚠️ Title[785] 纹理加载成功但egui_texture为空");
                // 备用背景
                ui.painter().rect_filled(
                    final_viewer_rect,
                    5.0,
                    egui::Color32::from_rgba_premultiplied(40, 40, 50, 240),
                );
                ui.painter().rect_stroke(
                    final_viewer_rect,
                    5.0,
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(200, 200, 200)),
                    egui::epaint::StrokeKind::Outside,
                );
            }
        } else {
            println!("⚠️ Title[785] 纹理加载失败，使用备用背景");
            // 备用背景 - 如果纹理加载失败
            ui.painter().rect_filled(
                final_viewer_rect,
                5.0,
                egui::Color32::from_rgba_premultiplied(40, 40, 50, 240),
            );
            ui.painter().rect_stroke(
                final_viewer_rect,
                5.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(200, 200, 200)),
                egui::epaint::StrokeKind::Outside,
            );
        }
        
        // 绘制物品预览区域（基于原版位置：105, 160）
        let preview_area = egui::Rect::from_center_size(
            egui::pos2(final_viewer_rect.min.x + 130.0, final_viewer_rect.min.y + 180.0),
            egui::vec2(100.0, 80.0)
        );
        
        ui.painter().rect_filled(
            preview_area,
            3.0,
            egui::Color32::from_rgba_premultiplied(20, 20, 30, 180)
        );
        
        // 绘制物品图标（放大版本）
        let icon_rect = egui::Rect::from_center_size(
            preview_area.center(),
            egui::vec2(64.0, 64.0)
        );
        
        ui.painter().rect_filled(
            icon_rect,
            2.0,
            egui::Color32::from_rgb(60, 60, 70)
        );
        
        // 绘制物品名称
        ui.painter().text(
            egui::pos2(final_viewer_rect.center().x, final_viewer_rect.min.y + 30.0),
            egui::Align2::CENTER_TOP,
            &viewer.item.name,
            egui::FontId::proportional(16.0),
            egui::Color32::WHITE,
        );
        
        // 绘制物品描述
        ui.painter().text(
            egui::pos2(final_viewer_rect.center().x, final_viewer_rect.min.y + 50.0),
            egui::Align2::CENTER_TOP,
            &viewer.item.description,
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(200, 200, 200),
        );
        
        // 绘制方向控制按钮 (原版位置：LeftDirection 81,282  RightDirection 160,282)
        let left_btn_rect = egui::Rect::from_min_size(
            egui::pos2(final_viewer_rect.min.x + 81.0, final_viewer_rect.min.y + 282.0),
            egui::vec2(24.0, 20.0)
        );
        let right_btn_rect = egui::Rect::from_min_size(
            egui::pos2(final_viewer_rect.min.x + 160.0, final_viewer_rect.min.y + 282.0),
            egui::vec2(24.0, 20.0)
        );
        
        // 左转按钮 (Prguse2纹理：240/241/242)
        let left_response = ui.interact(left_btn_rect, egui::Id::new("viewer_left"), egui::Sense::click());
        let left_texture_index = if left_response.clicked() {
            242 // pressed
        } else if left_response.hovered() {
            241 // hover
        } else {
            240 // normal
        };
        
        if let Some(info) = LibraryName::Prguse2.get_egui_texture(ctx, left_texture_index) {
            if let Some(left_texture) = info.egui_texture {
                ui.painter().image(
                    left_texture.id(),
                    left_btn_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        } else {
            // 备用左转按钮
            ui.painter().rect_filled(left_btn_rect, 3.0, egui::Color32::from_rgb(80, 80, 120));
            ui.painter().text(
                left_btn_rect.center(),
                egui::Align2::CENTER_CENTER,
                "◀",
                egui::FontId::proportional(12.0),
                egui::Color32::WHITE,
            );
        }
        
        if left_response.clicked() {
            viewer.direction = if viewer.direction == 1 { 8 } else { viewer.direction - 1 };
            println!("🔄 预览方向: {}", viewer.direction);
        }
        
        // 右转按钮 (Prguse2纹理：243/244/245)
        let right_response = ui.interact(right_btn_rect, egui::Id::new("viewer_right"), egui::Sense::click());
        let right_texture_index = if right_response.clicked() {
            245 // pressed
        } else if right_response.hovered() {
            244 // hover
        } else {
            243 // normal
        };
        
        if let Some(info) = LibraryName::Prguse2.get_egui_texture(ctx, right_texture_index) {
            if let Some(right_texture) = info.egui_texture {
                ui.painter().image(
                    right_texture.id(),
                    right_btn_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        } else {
            // 备用右转按钮
            ui.painter().rect_filled(right_btn_rect, 3.0, egui::Color32::from_rgb(80, 80, 120));
            ui.painter().text(
                right_btn_rect.center(),
                egui::Align2::CENTER_CENTER,
                "▶",
                egui::FontId::proportional(12.0),
                egui::Color32::WHITE,
            );
        }
        
        if right_response.clicked() {
            viewer.direction = if viewer.direction == 8 { 1 } else { viewer.direction + 1 };
            println!("🔄 预览方向: {}", viewer.direction);
        }
        
        // 关闭按钮 (原版位置：230, 8，使用Prguse纹理361/362/363)
        let close_btn_rect = egui::Rect::from_min_size(
            egui::pos2(final_viewer_rect.min.x + 230.0, final_viewer_rect.min.y + 8.0),
            egui::vec2(20.0, 20.0)
        );
        
        let close_response = ui.interact(close_btn_rect, egui::Id::new("viewer_close"), egui::Sense::click());
        let close_texture_index = if close_response.clicked() {
            363 // pressed
        } else if close_response.hovered() {
            362 // hover
        } else {
            361 // normal
        };
        
        // 绘制关闭按钮纹理
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, close_texture_index) {
            if let Some(close_texture) = info.egui_texture {
                ui.painter().image(
                    close_texture.id(),
                    close_btn_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        } else {
            // 备用关闭按钮
            ui.painter().rect_filled(close_btn_rect, 10.0, egui::Color32::from_rgb(180, 60, 60));
            ui.painter().text(
                close_btn_rect.center(),
                egui::Align2::CENTER_CENTER,
                "✕",
                egui::FontId::proportional(12.0),
                egui::Color32::WHITE,
            );
        }
        
        let mut should_close = close_response.clicked();
        if should_close {
            println!("❌ 关闭商品预览");
        }
        
        // 按ESC键关闭预览器
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            should_close = true;
            println!("❌ 按ESC键关闭商品预览");
        }
        
        // 显示当前方向
        ui.painter().text(
            egui::pos2(final_viewer_rect.center().x, final_viewer_rect.min.y + 280.0),
            egui::Align2::CENTER_TOP,
            &format!("方向: {}/8", viewer.direction),
            egui::FontId::proportional(10.0),
            egui::Color32::from_rgb(150, 150, 150),
        );
        
        should_close
    }

    /// 绘制分页控制
    fn draw_pagination(&mut self, ui: &mut egui::Ui, bg_rect: &egui::Rect) {
        let total_pages = if self.filtered_items.is_empty() { 
            1 
        } else { 
            (self.filtered_items.len() + self.items_per_page - 1) / self.items_per_page 
        };
        
        // 页码显示位置 (基于原版PageNumberLabel位置597, 446)
        let page_rect = egui::Rect::from_center_size(
            egui::pos2(bg_rect.min.x + 597.0, bg_rect.min.y + 446.0),
            egui::vec2(83.0, 17.0)
        );
        
        let page_text = format!("{} / {}", self.current_page + 1, total_pages);
        ui.painter().text(
            page_rect.center(),
            egui::Align2::CENTER_CENTER,
            &page_text,
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
        
        // 上一页/下一页按钮位置 (基于原版PreviousButton位置600, 448)
        let prev_button_rect = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 560.0, bg_rect.min.y + 448.0),
            egui::vec2(30.0, 20.0)
        );
        let next_button_rect = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 690.0, bg_rect.min.y + 448.0),
            egui::vec2(30.0, 20.0)
        );
        
        // 上一页按钮
        if self.current_page > 0 {
            ui.painter().rect_filled(prev_button_rect, 3.0, egui::Color32::from_rgb(80, 80, 120));
            ui.painter().text(
                prev_button_rect.center(),
                egui::Align2::CENTER_CENTER,
                "◀",
                egui::FontId::proportional(12.0),
                egui::Color32::WHITE,
            );
            
            let prev_response = ui.interact(prev_button_rect, egui::Id::new("shop_prev_page"), egui::Sense::click());
            if prev_response.clicked() {
                self.current_page -= 1;
                self.selected_item = None;
                self.item_viewer = None; // 关闭预览器
                println!("📄 切换到第{}页", self.current_page + 1);
            }
        }
        
        // 下一页按钮
        if self.current_page < total_pages - 1 {
            ui.painter().rect_filled(next_button_rect, 3.0, egui::Color32::from_rgb(80, 80, 120));
            ui.painter().text(
                next_button_rect.center(),
                egui::Align2::CENTER_CENTER,
                "▶",
                egui::FontId::proportional(12.0),
                egui::Color32::WHITE,
            );
            
            let next_response = ui.interact(next_button_rect, egui::Id::new("shop_next_page"), egui::Sense::click());
            if next_response.clicked() {
                self.current_page += 1;
                self.selected_item = None;
                self.item_viewer = None; // 关闭预览器
                println!("📄 切换到第{}页", self.current_page + 1);
            }
        }
    }



    /// 绘制玩家货币信息
    fn draw_currency_info(&self, ui: &mut egui::Ui, bg_rect: &egui::Rect) {
        let info_y = bg_rect.max.y - 35.0;
        
        ui.painter().text(
            egui::pos2(bg_rect.min.x + 20.0, info_y),
            egui::Align2::LEFT_TOP,
            &format!("金币: {}", self.player_gold),
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(255, 215, 0),
        );

        ui.painter().text(
            egui::pos2(bg_rect.min.x + 150.0, info_y),
            egui::Align2::LEFT_TOP,
            &format!("元宝: {}", self.player_ingot),
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(0, 255, 255),
        );
    }

    /// 绘制关闭按钮
    fn draw_close_button(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context, bg_rect: &egui::Rect) -> bool {
        // 关闭按钮位置（右上角）
        let close_size = egui::vec2(20.0, 20.0);
        let close_rect = egui::Rect::from_min_size(
            egui::pos2(bg_rect.max.x - 25.0, bg_rect.min.y + 5.0),
            close_size
        );

        // 绘制关闭按钮背景
        ui.painter().rect_filled(close_rect, 2.0, egui::Color32::from_rgb(150, 50, 50));
        ui.painter().rect_stroke(
            close_rect,
            2.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 100, 100)),
            egui::epaint::StrokeKind::Outside,
        );

        // 绘制关闭符号 "×"
        ui.painter().text(
            close_rect.center(),
            egui::Align2::CENTER_CENTER,
            "×",
            egui::FontId::proportional(14.0),
            egui::Color32::WHITE,
        );

        let response = ui.interact(close_rect, egui::Id::new("shop_close"), egui::Sense::click());
        let is_clicked = response.clicked();
        if response.hovered() {
            response.on_hover_text("关闭");
        }

        is_clicked
    }

    /// 处理窗口拖拽
    fn handle_window_dragging(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 标题栏区域作为拖拽区域，但要避免与tab按钮和关闭按钮重叠
        let title_area = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 30.0, bg_rect.min.y), // 避开左侧tab区域
            egui::vec2(bg_rect.width() - 120.0, 30.0), // 避开右侧关闭按钮区域
        );
        
        let drag_response = ui.interact(title_area, egui::Id::new("shop_main_drag"), egui::Sense::drag());
        
        if drag_response.drag_started() && !self.dragging {
            self.dragging = true;
            if let Some(pointer_pos) = ctx.pointer_interact_pos() {
                self.drag_offset = self.position.to_vec2() - pointer_pos.to_vec2();
            }
        }
        
        if self.dragging && drag_response.dragged() {
            // 使用egui内置的拖拽增量，更流畅
            self.position += drag_response.drag_delta();
        }
        
        if drag_response.drag_stopped() || !ctx.input(|i| i.pointer.primary_down()) {
            self.dragging = false;
        }
    }

    /// 独立绘制商品预览器，使用模态对话框模式避免阻挡主对话框交互
    fn draw_item_viewer_separate(ctx: &egui::Context, viewer: &mut ShopItemViewer, _main_dialog_pos: &egui::Pos2) -> bool {
        let mut should_close = false;
        
        // 1. 绘制半透明遮罩层（阻止底层交互，点击外部关闭）
        egui::Area::new(egui::Id::new(format!("modal_overlay_{}", viewer.item.id)))
            .fixed_pos(egui::pos2(0.0, 0.0))
            .movable(false)
            .interactable(true)  // 消费所有点击事件
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let screen_size = ctx.screen_rect().size();
                let overlay_rect = egui::Rect::from_min_size(
                    egui::pos2(0.0, 0.0),
                    screen_size,
                );
                // 绘制半透明遮罩
                ui.painter().rect_filled(
                    overlay_rect,
                    0.0,
                    egui::Color32::from_black_alpha(64),  // 轻微半透明
                );
                // 点击遮罩区域关闭预览器
                if ui.allocate_rect(overlay_rect, egui::Sense::click()).clicked() {
                    should_close = true;
                    println!("📱 点击外部关闭商品预览");
                }
            });
        
        // 2. 在遮罩层上方显示预览对话框
        if let Some(response) = egui::Window::new(&viewer.item.name)
            .id(egui::Id::new(format!("shop_item_viewer_{}", viewer.item.id)))
            .fixed_pos(viewer.position)
            .fixed_size(egui::vec2(260.0, 280.0))
            .title_bar(true)  // 使用标准标题栏提供拖拽功能
            .resizable(false)
            .collapsible(false) 
            .order(egui::Order::Tooltip)  // 🔧 最高层级，在遮罩层之上
            .show(ctx, |ui| {
                // 使用标准 egui 布局，避免手动交互区域
                
                // 物品图标
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    
                    // 图标区域
                    if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, viewer.item.icon_index) {
                        if let Some(item_texture) = info.egui_texture {
                            ui.image(egui::ImageSource::Texture(egui::load::SizedTexture::new(
                                item_texture.id(),
                                egui::vec2(64.0, 64.0)
                            )));
                        }
                    } else {
                        // 备用图标显示
                        ui.allocate_space(egui::vec2(64.0, 64.0));
                    }
                    
                    // 物品信息
                    ui.vertical(|ui| {
                        ui.label(&viewer.item.name);
                        ui.separator();
                        ui.label(&viewer.item.description);
                        ui.separator();
                        if viewer.item.price_gold > 0 {
                            ui.label(format!("金币: {}", viewer.item.price_gold));
                        }
                        if viewer.item.price_ingot > 0 {
                            ui.label(format!("元宝: {}", viewer.item.price_ingot));
                        }
                    });
                });
                
                ui.separator();
                
                // 方向控制
                ui.horizontal(|ui| {
                    ui.label("预览方向:");
                    if ui.button("◀").clicked() {
                        viewer.direction = if viewer.direction == 1 { 8 } else { viewer.direction - 1 };
                        println!("🔄 预览方向: {}", viewer.direction);
                    }
                    ui.label(format!("{}", viewer.direction));
                    if ui.button("▶").clicked() {
                        viewer.direction = if viewer.direction == 8 { 1 } else { viewer.direction + 1 };
                        println!("🔄 预览方向: {}", viewer.direction);
                    }
                });
                
                ui.separator();
                
                // 关闭按钮
                if ui.button("关闭").clicked() {
                    should_close = true;
                    println!("❌ 关闭商品预览");
                }
                
                // 返回是否需要关闭
                should_close
            }) {
            
            // 更新位置（Window 会自动处理拖拽，我们只需要同步位置）
            viewer.position = response.response.rect.min;
            
            // 检查是否需要关闭
            if let Some(inner_result) = response.inner {
                should_close = inner_result;
            }
        }
        
        // 按ESC键关闭预览器
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            should_close = true;
            println!("❌ ESC键关闭商品预览");
        }
        
        should_close
    }
}

impl Dialog for GameShopDialog {
    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        if !self.visible {
            *open = false;
            return;
        }
        
        // 使用 Area 创建自由浮动窗口
        egui::Area::new(egui::Id::new("game_shop_dialog"))
            .fixed_pos(self.position)
            .movable(false)  // 使用自定义拖拽
            .order(egui::Order::Middle)  // 设置中等优先级，避免与其他UI冲突
            .show(ctx, |ui| {
                // 绘制背景
                let bg_rect = self.draw_background(ui, ctx);
                
                // 处理窗口拖拽
                self.handle_window_dragging(ui, ctx, &bg_rect);
                
                // 绘制分类标签页
                self.draw_category_tabs(ui, ctx, &bg_rect);
                
                // 绘制物品列表
                self.draw_item_grid(ui, ctx, &bg_rect);
                
                // 绘制货币信息
                self.draw_currency_info(ui, &bg_rect);
                
                // 绘制关闭按钮
                if self.draw_close_button(ui, ctx, &bg_rect) {
                    self.visible = false;
                    *open = false;
                }
                
            });
        
        // 商品预览器使用独立的 Area，层级更高，避免阻挡主对话框交互
        let mut close_viewer = false;
        if let Some(ref mut viewer) = self.item_viewer {
            if viewer.visible {
                close_viewer = GameShopDialog::draw_item_viewer_separate(ctx, viewer, &self.position);
            }
        }
        if close_viewer {
            self.item_viewer = None;
        }
        
        *open = self.visible;
    }
}