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
    pub stock: u32,          // 库存数量 (0表示无限)
    pub count: u32,          // 每次购买数量
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
    /// 窗口位置
    pub position: egui::Pos2,
    /// 是否正在拖拽
    dragging: bool,
    /// 拖拽偏移
    drag_offset: egui::Vec2,
    /// 当前选中的主要分类
    pub selected_section: GameShopSection,
    /// 当前选中的职业分类
    pub selected_class: GameShopClass,
    /// 商城物品列表
    pub shop_items: Vec<ShopItem>,
    /// 过滤后的物品列表
    pub filtered_items: Vec<ShopItem>,
    /// 滚动偏移
    scroll_offset: f32,
    /// 选中的物品索引
    pub selected_item: Option<usize>,
    /// 商品预览器
    pub item_viewer: Option<ShopItemViewer>,
    /// 购买数量
    buy_quantity: u32,
    /// 玩家金币
    pub player_gold: u32,
    /// 玩家元宝
    pub player_ingot: u32,
    /// 当前页面
    pub current_page: usize,
    /// 每页显示物品数量 (4x2 = 8个)
    pub items_per_page: usize,
    /// 左侧分类列表滚动位置 (0-based索引)
    category_scroll_index: usize,
    /// 分类列表
    categories: Vec<String>,
    /// 每个格子的购买数量 [0-7]
    pub quantities: [u8; 8],
    /// 搜索文本
    pub search_text: String,
    /// 支付方式: true=金币, false=元宝
    pub pay_with_gold: bool,
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
                stock: 10,
                count: 1,
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
                stock: 5,
                count: 1,
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
                stock: 0,  // 无限库存
                count: 10,
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
                stock: 0,
                count: 1,
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
                stock: 20,
                count: 1,
            },
        ];

        let mut dialog = Self {
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
            category_scroll_index: 0,
            categories: vec![
                "武器".to_string(),
                "防具".to_string(),
                "头盔".to_string(),
                "项链".to_string(),
                "手镯".to_string(),
                "戒指".to_string(),
                "腰带".to_string(),
                "靴子".to_string(),
                "药品".to_string(),
                "特殊物品".to_string(),
                "时装".to_string(),
                "宝石".to_string(),
                "材料".to_string(),
                "卷轴".to_string(),
                "坐骑".to_string(),
                "技能书".to_string(),
                "消耗品".to_string(),
                "任务物品".to_string(),
                "其他".to_string(),
            ],
            pay_with_gold: true,      // 默认用金币支付
            quantities: [1; 8],       // 每个单元格默认数量为1
            search_text: String::new(), // 搜索框初始为空
        };
        
        // 初始化过滤的物品列表
        dialog.filter_items();
        dialog
    }


    /// 绘制对话框背景
    fn draw_background(&self, ui: &mut egui::Ui, ctx: &egui::Context) -> egui::Rect {
        // 使用原版传奇2商城背景纹理 (Title[749])
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
    pub fn filter_items(&mut self) {
        let search_lower = self.search_text.to_lowercase();
        
        self.filtered_items = self.shop_items
            .iter()
            .filter(|item| {
                // 搜索过滤
                let search_match = if search_lower.is_empty() {
                    true
                } else {
                    item.name.to_lowercase().contains(&search_lower) ||
                    item.description.to_lowercase().contains(&search_lower)
                };
                
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
                
                search_match && section_match && class_match
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
            
            // 普通和选中的纹理索引
            let (normal_idx, selected_idx) = match section {
                GameShopSection::All => (770, 771),
                GameShopSection::TopItems => (776, 777),
                GameShopSection::Deals => (772, 773),
                GameShopSection::New => (774, 775),
            };
            
            let tab_x = bg_rect.min.x + 138.0 + (i as f32 * 71.0);
            let tab_pos = egui::pos2(tab_x, section_y);
            let tab_size = egui::vec2(71.0, 23.0);
            let tab_rect = egui::Rect::from_min_size(tab_pos, tab_size);
            
            // 交互检测
            let response = ui.interact(tab_rect, egui::Id::new(format!("section_tab_{}", i)), egui::Sense::click());
            
            // 确定显示的纹理（悬停时显示选中状态）
            let display_idx = if response.hovered() && !is_selected {
                selected_idx  // 悬停时显示高亮
            } else if is_selected {
                selected_idx  // 选中状态
            } else {
                normal_idx    // 正常状态
            };
            
            // 使用纹理渲染tab按钮
            if let Some(info) = LibraryName::Title.get_egui_texture(ctx, display_idx) {
                if let Some(tab_texture) = info.egui_texture {
                    ui.painter().image(
                        tab_texture.id(),
                        tab_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                    
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
            
            let tab_x = bg_rect.min.x + 539.0 + (i as f32 * 23.0);
            let tab_pos = egui::pos2(tab_x, class_y);
            let tab_size = egui::vec2(23.0, 20.0);
            let tab_rect = egui::Rect::from_min_size(tab_pos, tab_size);
            
            // 交互检测（用于判断悬停和点击状态）
            let response = ui.interact(tab_rect, egui::Id::new(format!("class_tab_{}", i)), egui::Sense::click());
            
            // 根据状态选择纹理索引: normal/hover/pressed
            let texture_index = if is_selected {
                // 已选中的标签页使用选中状态纹理
                match class {
                    GameShopClass::All => 752,
                    GameShopClass::Warrior => 755,
                    GameShopClass::Assassin => 758,
                    GameShopClass::Taoist => 761,
                    GameShopClass::Wizard => 764,
                    GameShopClass::Archer => 767,
                }
            } else if response.is_pointer_button_down_on() {
                // 按下状态 (pressed)
                match class {
                    GameShopClass::All => 753,
                    GameShopClass::Warrior => 756,
                    GameShopClass::Assassin => 759,
                    GameShopClass::Taoist => 762,
                    GameShopClass::Wizard => 765,
                    GameShopClass::Archer => 768,
                }
            } else if response.hovered() {
                // 悬停状态 (hover)
                match class {
                    GameShopClass::All => 752,
                    GameShopClass::Warrior => 755,
                    GameShopClass::Assassin => 758,
                    GameShopClass::Taoist => 761,
                    GameShopClass::Wizard => 764,
                    GameShopClass::Archer => 767,
                }
            } else {
                // 正常状态 (normal)
                match class {
                    GameShopClass::All => 751,
                    GameShopClass::Warrior => 754,
                    GameShopClass::Assassin => 757,
                    GameShopClass::Taoist => 760,
                    GameShopClass::Wizard => 763,
                    GameShopClass::Archer => 766,
                }
            };
            
            // 使用纹理渲染tab按钮
            if let Some(info) = LibraryName::Title.get_egui_texture(ctx, texture_index) {
                if let Some(tab_texture) = info.egui_texture {
                    ui.painter().image(
                        tab_texture.id(),
                        tab_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                    
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
        
        // 收集数量调整操作 (grid_idx, delta, shift_held)
        let mut qty_changes: Vec<(usize, i8, bool)> = Vec::new();
        // 收集需要知道的最大数量信息 (grid_idx, max_qty)
        let mut max_qtys: Vec<(usize, u8)> = Vec::new();
        
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
                
                // 物品名称 (位置: 0, 13) - 金色，字体稍大
                ui.painter().text(
                    egui::pos2(cell_rect.min.x + cell_width / 2.0, cell_rect.min.y + 13.0),
                    egui::Align2::CENTER_TOP,
                    &item.name,
                    egui::FontId::proportional(10.0),
                    if item.in_stock { egui::Color32::from_rgb(255, 215, 0) } else { egui::Color32::GRAY },
                );
                
                // 物品图标区域 (位置: 12, 40, 尺寸: 32x32)
                let icon_base_pos = egui::pos2(cell_rect.min.x + 12.0, cell_rect.min.y + 40.0);
                
                // 绘制物品图标纹理 (使用Items库)
                if let Some(info) = LibraryName::Items.get_egui_texture(ctx, item.icon_index as usize) {
                    if let Some(item_texture) = info.egui_texture {
                        // 获取纹理实际尺寸
                        let texture_size = egui::vec2(info.width as f32, info.height as f32);
                        
                        // 计算居中偏移 (32x32区域内居中)
                        let offset_x = (32.0 - texture_size.x) / 2.0;
                        let offset_y = (32.0 - texture_size.y) / 2.0;
                        
                        let icon_rect = egui::Rect::from_min_size(
                            egui::pos2(icon_base_pos.x + offset_x, icon_base_pos.y + offset_y),
                            texture_size
                        );
                        
                        // 绘制图标
                        ui.painter().image(
                            item_texture.id(),
                            icon_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
                
                // 交互区域 - 悬停时显示详细信息
                let interact_rect = egui::Rect::from_min_size(icon_base_pos, egui::vec2(32.0, 32.0));
                let icon_response = ui.interact(interact_rect, egui::Id::new(format!("icon_{}", item.id)), egui::Sense::hover());
                if icon_response.hovered() {
                    // 显示详细的物品信息（跟随鼠标位置）
                    if let Some(pointer_pos) = ctx.pointer_hover_pos() {
                        egui::Area::new(egui::Id::new(format!("tooltip_{}", item.id)))
                            .fixed_pos(pointer_pos + egui::vec2(10.0, 10.0))
                            .order(egui::Order::Tooltip)
                            .show(ctx, |ui| {
                                egui::Frame::new()
                                    .fill(egui::Color32::from_rgba_premultiplied(40, 40, 40, 150))  // 更透明的背景
                                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(220, 220, 220)))  // 更亮的边框
                                    .inner_margin(8.0)  // 内边距
                                    .show(ui, |ui| {
                                        ui.style_mut().visuals.override_text_color = Some(egui::Color32::WHITE);  // 确保文字是纯白色
                                        ui.set_max_width(250.0);
                                        
                                        // 物品名称（金色）
                                        ui.label(egui::RichText::new(&item.name)
                                            .color(egui::Color32::from_rgb(255, 215, 0))
                                            .size(14.0)
                                            .strong());
                                        
                                        ui.separator();
                                        
                                        // 物品描述
                                        ui.label(&item.description);
                                        
                                        ui.separator();
                                        
                                        // 价格信息
                                        if item.price_gold > 0 {
                                            ui.label(egui::RichText::new(format!("金币: {}", item.price_gold))
                                                .color(egui::Color32::from_rgb(255, 215, 0)));
                                        }
                                        if item.price_ingot > 0 {
                                            ui.label(egui::RichText::new(format!("元宝: {}", item.price_ingot))
                                                .color(egui::Color32::from_rgb(0, 255, 255)));
                                        }
                                        
                                        // 库存信息
                                        if item.stock == 0 {
                                            ui.label("库存: 无限");
                                        } else {
                                            ui.label(format!("库存: {}", item.stock));
                                        }
                                        
                                        // 数量信息
                                        if item.count > 1 {
                                            ui.label(format!("每次购买: {} 个", item.count));
                                        }
                                    });
                            });
                    }
                }
                
                // STOCK标签 (位置: 53, 37)
                ui.painter().text(
                    egui::pos2(cell_rect.min.x + 53.0, cell_rect.min.y + 37.0),
                    egui::Align2::LEFT_TOP,
                    "STOCK:",
                    egui::FontId::proportional(7.0),
                    egui::Color32::GRAY,
                );
                
                // 库存数量 (位置: 93, 37)
                let stock_text = if item.stock >= 99 {
                    "99+".to_string()
                } else if item.stock == 0 {
                    "∞".to_string()
                } else {
                    item.stock.to_string()
                };
                ui.painter().text(
                    egui::pos2(cell_rect.min.x + 93.0, cell_rect.min.y + 37.0),
                    egui::Align2::LEFT_TOP,
                    &stock_text,
                    egui::FontId::proportional(7.0),
                    egui::Color32::WHITE,
                );
                
                // 物品数量 (位置: 16, 60)
                if item.count > 1 {
                    ui.painter().text(
                        egui::pos2(cell_rect.min.x + 16.0, cell_rect.min.y + 60.0),
                        egui::Align2::LEFT_TOP,
                        &format!("x{}", item.count),
                        egui::FontId::proportional(7.0),
                        egui::Color32::WHITE,
                    );
                }
                
                // 数量调整按钮 (位置: 55-97, 56)
                // 计算当前格子在页面中的索引 (0-7)
                let grid_idx = i;
                let current_qty = self.quantities[grid_idx];
                
                // 减少按钮 (Prguse2[240-242])
                let qty_down_rect = egui::Rect::from_min_size(
                    egui::pos2(cell_rect.min.x + 55.0, cell_rect.min.y + 56.0),
                    egui::vec2(16.0, 14.0)
                );
                let qty_down_response = ui.interact(qty_down_rect, egui::Id::new(format!("qty_down_{}", item.id)), egui::Sense::click());
                
                // 绘制减少按钮纹理
                let down_tex_idx = if qty_down_response.is_pointer_button_down_on() {
                    242 // pressed
                } else if qty_down_response.hovered() {
                    241 // hover
                } else {
                    240 // normal
                };
                if let Some(info) = LibraryName::Prguse2.get_egui_texture(ctx, down_tex_idx) {
                    if let Some(tex) = info.egui_texture {
                        ui.painter().image(
                            tex.id(),
                            qty_down_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                } else {
                    // 备用绘制
                    let color = if qty_down_response.hovered() { 
                        egui::Color32::from_rgb(120, 120, 160) 
                    } else { 
                        egui::Color32::from_rgb(80, 80, 120) 
                    };
                    ui.painter().rect_filled(qty_down_rect, 2.0, color);
                    ui.painter().text(
                        qty_down_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "-",
                        egui::FontId::proportional(10.0),
                        egui::Color32::WHITE,
                    );
                }
                
                // 数量显示 (位置: 74, 56)
                ui.painter().text(
                    egui::pos2(cell_rect.min.x + 82.0, cell_rect.min.y + 56.0 + 7.0),
                    egui::Align2::CENTER_CENTER,
                    &current_qty.to_string(),
                    egui::FontId::proportional(8.0),
                    egui::Color32::WHITE,
                );
                
                // 增加按钮 (Prguse2[243-245])
                let qty_up_rect = egui::Rect::from_min_size(
                    egui::pos2(cell_rect.min.x + 97.0, cell_rect.min.y + 56.0),
                    egui::vec2(16.0, 14.0)
                );
                let qty_up_response = ui.interact(qty_up_rect, egui::Id::new(format!("qty_up_{}", item.id)), egui::Sense::click());
                
                // 绘制增加按钮纹理
                let up_tex_idx = if qty_up_response.is_pointer_button_down_on() {
                    245 // pressed
                } else if qty_up_response.hovered() {
                    244 // hover
                } else {
                    243 // normal
                };
                if let Some(info) = LibraryName::Prguse2.get_egui_texture(ctx, up_tex_idx) {
                    if let Some(tex) = info.egui_texture {
                        ui.painter().image(
                            tex.id(),
                            qty_up_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                } else {
                    // 备用绘制
                    let color = if qty_up_response.hovered() { 
                        egui::Color32::from_rgb(120, 120, 160) 
                    } else { 
                        egui::Color32::from_rgb(80, 80, 120) 
                    };
                    ui.painter().rect_filled(qty_up_rect, 2.0, color);
                    ui.painter().text(
                        qty_up_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "+",
                        egui::FontId::proportional(10.0),
                        egui::Color32::WHITE,
                    );
                }
                
                // 处理数量按钮点击
                let shift_held = ui.input(|i| i.modifiers.shift);
                if qty_down_response.clicked() && !self.dragging {
                    qty_changes.push((grid_idx, -1, shift_held));
                }
                if qty_up_response.clicked() && !self.dragging {
                    let max_qty = if item.stock > 0 && item.stock < 99 {
                        item.stock as u8
                    } else {
                        99
                    };
                    max_qtys.push((grid_idx, max_qty));
                    qty_changes.push((grid_idx, 1, shift_held));
                }
                
                // 元宝价格 (位置: 2, 81)
                if item.price_ingot > 0 {
                    ui.painter().text(
                        egui::pos2(cell_rect.min.x + 97.0, cell_rect.min.y + 81.0),
                        egui::Align2::RIGHT_TOP,
                        &format!("{}", item.price_ingot),
                        egui::FontId::proportional(8.0),
                        egui::Color32::from_rgb(0, 255, 255),
                    );
                }
                
                // 金币价格 (位置: 2, 102)
                if item.price_gold > 0 {
                    ui.painter().text(
                        egui::pos2(cell_rect.min.x + 97.0, cell_rect.min.y + 102.0),
                        egui::Align2::RIGHT_TOP,
                        &format!("{}", item.price_gold),
                        egui::FontId::proportional(8.0),
                        egui::Color32::from_rgb(255, 215, 0),
                    );
                }
                
                // Preview按钮 (Title[781-783], 位置: 8, 122) - 仅武器/装备显示
                let is_previewable = matches!(item.category, ShopCategory::Weapon | ShopCategory::Armor);
                if is_previewable {
                    let preview_rect = egui::Rect::from_min_size(
                        egui::pos2(cell_rect.min.x + 8.0, cell_rect.min.y + 122.0),
                        egui::vec2(32.0, 16.0)
                    );
                    let preview_response = ui.interact(preview_rect, egui::Id::new(format!("preview_{}", item.id)), egui::Sense::click());
                    
                    // 绘制Preview按钮纹理
                    let preview_texture_index = if preview_response.clicked() {
                        783 // pressed
                    } else if preview_response.hovered() {
                        782 // hover
                    } else {
                        781 // normal
                    };
                    
                    if let Some(info) = LibraryName::Title.get_egui_texture(ctx, preview_texture_index) {
                        if let Some(preview_texture) = info.egui_texture {
                            ui.painter().image(
                                preview_texture.id(),
                                preview_rect,
                                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                egui::Color32::WHITE,
                            );
                        }
                    }
                    
                    if preview_response.clicked() && !self.dragging {
                        // 创建预览器
                        let viewer_size = egui::vec2(260.0, 300.0);
                        let viewer_pos = if i % 4 < 2 {
                            egui::pos2(bg_rect.max.x - viewer_size.x - 30.0, bg_rect.min.y + 120.0)
                        } else {
                            egui::pos2(bg_rect.min.x + 50.0, bg_rect.min.y + 120.0)
                        };
                        self.item_viewer = Some(ShopItemViewer::new(item.clone(), viewer_pos));
                        self.selected_item = Some(item_index);
                    }
                }
                
                // Buy按钮 (Title[778-780], 位置: 42/75, 122)
                let buy_x = if is_previewable { 75.0 } else { 42.0 };
                let buy_rect = egui::Rect::from_min_size(
                    egui::pos2(cell_rect.min.x + buy_x, cell_rect.min.y + 122.0),
                    egui::vec2(32.0, 16.0)
                );
                let buy_response = ui.interact(buy_rect, egui::Id::new(format!("buy_{}", item.id)), egui::Sense::click());
                
                // 绘制Buy按钮纹理
                let buy_texture_index = if buy_response.clicked() {
                    780 // pressed
                } else if buy_response.hovered() {
                    779 // hover
                } else {
                    778 // normal
                };
                
                if let Some(info) = LibraryName::Title.get_egui_texture(ctx, buy_texture_index) {
                    if let Some(buy_texture) = info.egui_texture {
                        ui.painter().image(
                            buy_texture.id(),
                            buy_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
                
                if buy_response.clicked() && !self.dragging {
                    let qty = self.quantities[i];
                    let payment = if self.pay_with_gold { "金币" } else { "元宝" };
                    let price = if self.pay_with_gold { item.price_gold } else { item.price_ingot };
                    let total = price * qty as u32;
                    println!("💰 购买商品: {} x{} = {} {} (使用{})", item.name, qty, total, payment, payment);
                    // TODO: 发送购买请求到服务器
                }
                
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
            }
            // 空单元格不绘制任何内容
        }
        
        // 应用数量变化
        for (grid_idx, delta, shift_held) in qty_changes {
            let max_qty = max_qtys.iter()
                .find(|(idx, _)| *idx == grid_idx)
                .map(|(_, max)| *max)
                .unwrap_or(99);
            
            let current = self.quantities[grid_idx];
            let step = if shift_held { 10 } else { 1 };
            
            if delta > 0 {
                self.quantities[grid_idx] = (current + step).min(max_qty);
            } else {
                self.quantities[grid_idx] = current.saturating_sub(step).max(1);
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

    /// 绘制左侧分类列表区域
    fn draw_category_list(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 绘制FilterBackground背景 Title[769] 位置(11, 102)
        let filter_bg_pos = egui::pos2(bg_rect.min.x + 11.0, bg_rect.min.y + 102.0);
        if let Some(info) = LibraryName::Title.get_egui_texture(ctx, 769) {
            if let Some(bg_texture) = info.egui_texture {
                let bg_size = egui::vec2(info.width as f32, info.height as f32);
                let filter_rect = egui::Rect::from_min_size(filter_bg_pos, bg_size);
                ui.painter().image(
                    bg_texture.id(),
                    filter_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        }
        
        // 绘制分类列表文本 (显示22个)
        let list_start_y = bg_rect.min.y + 120.0;
        let line_height = 14.0;
        let max_visible = 22;
        
        for i in 0..max_visible {
            let category_index = self.category_scroll_index + i;
            if category_index >= self.categories.len() {
                break;
            }
            
            let category = &self.categories[category_index].clone();
            let text_pos = egui::pos2(bg_rect.min.x + 20.0, list_start_y + (i as f32 * line_height));
            
            // 创建可点击区域
            let item_rect = egui::Rect::from_min_size(
                egui::pos2(bg_rect.min.x + 15.0, list_start_y + (i as f32 * line_height)),
                egui::vec2(100.0, line_height)
            );
            let item_response = ui.interact(item_rect, egui::Id::new(format!("category_{}", category_index)), egui::Sense::click());
            
            // 悬停效果
            let text_color = if item_response.hovered() {
                egui::Color32::from_rgb(255, 215, 0)  // 金色高亮
            } else {
                egui::Color32::WHITE
            };
            
            ui.painter().text(
                text_pos,
                egui::Align2::LEFT_TOP,
                category,
                egui::FontId::proportional(9.0),
                text_color,
            );
            
            // 处理点击
            if item_response.clicked() && !self.dragging {
                println!("📁 选择分类: {}", category);
                // TODO: 根据分类筛选商品
            }
        }
        
        // 处理鼠标滚轮
        let list_area = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 15.0, list_start_y),
            egui::vec2(100.0, line_height * max_visible as f32)
        );
        let list_response = ui.interact(list_area, egui::Id::new("category_list_area"), egui::Sense::hover());
        
        if list_response.hovered() {
            let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll_delta != 0.0 {
                let scroll_lines = (-scroll_delta / 20.0) as i32;  // 每20像素滚动一行
                let new_index = (self.category_scroll_index as i32 + scroll_lines)
                    .max(0)
                    .min((self.categories.len().saturating_sub(max_visible)) as i32) as usize;
                self.category_scroll_index = new_index;
            }
        }
        
        // 绘制滚动条
        self.draw_category_scrollbar(ui, ctx, bg_rect);
    }
    
    /// 绘制分类列表滚动条
    fn draw_category_scrollbar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 上箭头按钮 Prguse2[197/198/199] 位置(120, 103)
        let up_btn_rect = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 120.0, bg_rect.min.y + 103.0),
            egui::vec2(16.0, 14.0)
        );
        let up_response = ui.interact(up_btn_rect, egui::Id::new("category_scroll_up"), egui::Sense::click());
        let up_index = if up_response.clicked() {
            199
        } else if up_response.hovered() {
            198
        } else {
            197
        };
        
        if let Some(info) = LibraryName::Prguse2.get_egui_texture(ctx, up_index) {
            if let Some(texture) = info.egui_texture {
                ui.painter().image(
                    texture.id(),
                    up_btn_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        }
        
        if up_response.clicked() && self.category_scroll_index > 0 {
            self.category_scroll_index -= 1;
        }
        
        // 下箭头按钮 Prguse2[207/208/209] 位置(120, 421)
        let down_btn_rect = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 120.0, bg_rect.min.y + 421.0),
            egui::vec2(16.0, 14.0)
        );
        let down_response = ui.interact(down_btn_rect, egui::Id::new("category_scroll_down"), egui::Sense::click());
        let down_index = if down_response.clicked() {
            209
        } else if down_response.hovered() {
            208
        } else {
            207
        };
        
        if let Some(info) = LibraryName::Prguse2.get_egui_texture(ctx, down_index) {
            if let Some(texture) = info.egui_texture {
                ui.painter().image(
                    texture.id(),
                    down_btn_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        }
        
        if down_response.clicked() && self.category_scroll_index + 22 < self.categories.len() {
            self.category_scroll_index += 1;
        }
        
        // 滚动块 Prguse2[205/206] 位置(120, 117) - 可拖拽
        let scrollbar_height = 421.0 - 117.0;  // 304px
        let scroll_ratio = if self.categories.len() > 22 {
            self.category_scroll_index as f32 / (self.categories.len() - 22) as f32
        } else {
            0.0
        };
        
        let position_y = bg_rect.min.y + 117.0 + (scroll_ratio * (scrollbar_height - 20.0));
        let position_rect = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 120.0, position_y),
            egui::vec2(16.0, 20.0)
        );
        
        let position_response = ui.interact(position_rect, egui::Id::new("category_scroll_bar"), egui::Sense::drag());
        let position_index = if position_response.hovered() || position_response.dragged() {
            206
        } else {
            205
        };
        
        if let Some(info) = LibraryName::Prguse2.get_egui_texture(ctx, position_index) {
            if let Some(texture) = info.egui_texture {
                ui.painter().image(
                    texture.id(),
                    position_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        }
        
        // 处理拖拽 - 直接使用鼠标位置而不是增量
        if position_response.dragged() {
            if let Some(pointer_pos) = ui.ctx().pointer_hover_pos() {
                let relative_y = pointer_pos.y - bg_rect.min.y - 117.0;
                let clamped_y = relative_y.clamp(0.0, scrollbar_height - 20.0);
                let new_ratio = clamped_y / (scrollbar_height - 20.0);
                
                if self.categories.len() > 22 {
                    self.category_scroll_index = ((self.categories.len() - 22) as f32 * new_ratio) as usize;
                    self.category_scroll_index = self.category_scroll_index.min(self.categories.len().saturating_sub(22));
                }
            }
        }
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
                self.quantities = [1; 8]; // 重置数量
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
                self.quantities = [1; 8]; // 重置数量
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

    /// 绘制支付方式选择 (原版位置: PaymentTypeGold=250,449 PaymentTypeCredit=340,449)
    fn draw_payment_options(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context, bg_rect: &egui::Rect) {
        let checkbox_size = 14.0;
        
        // Buy with Gold 复选框 (原版位置: 250, 449)
        let gold_rect = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 250.0, bg_rect.min.y + 449.0),
            egui::vec2(checkbox_size, checkbox_size)
        );
        let gold_label_rect = egui::Rect::from_min_size(
            egui::pos2(gold_rect.max.x + 4.0, gold_rect.min.y),
            egui::vec2(80.0, checkbox_size)
        );
        
        // 绘制金币复选框纹理 (Prguse2中寻找checkbox纹理)
        if self.pay_with_gold {
            ui.painter().rect_filled(gold_rect, 2.0, egui::Color32::from_rgb(40, 80, 40));
            ui.painter().text(
                gold_rect.center(),
                egui::Align2::CENTER_CENTER,
                "✓",
                egui::FontId::proportional(10.0),
                egui::Color32::GREEN,
            );
        } else {
            ui.painter().rect_filled(gold_rect, 2.0, egui::Color32::from_rgb(60, 60, 80));
        }
        ui.painter().rect_stroke(
            gold_rect, 2.0, 
            egui::Stroke::new(1.0, egui::Color32::from_rgb(150, 150, 170)),
            egui::epaint::StrokeKind::Outside,
        );
        
        // 交互区域（包括标签）
        let gold_interact_rect = egui::Rect::from_min_max(gold_rect.min, gold_label_rect.max);
        let gold_response = ui.interact(gold_interact_rect, egui::Id::new("pay_gold"), egui::Sense::click());
        
        let gold_text_color = if gold_response.hovered() { 
            egui::Color32::WHITE 
        } else { 
            egui::Color32::GRAY 
        };
        ui.painter().text(
            egui::pos2(gold_label_rect.min.x, gold_label_rect.center().y),
            egui::Align2::LEFT_CENTER,
            "Buy with Gold",
            egui::FontId::proportional(9.0),
            gold_text_color,
        );
        
        if gold_response.clicked() && !self.dragging {
            self.pay_with_gold = true;
        }
        
        // Buy with Credits 复选框 (原版位置: 340, 449)
        let credit_rect = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 340.0, bg_rect.min.y + 449.0),
            egui::vec2(checkbox_size, checkbox_size)
        );
        let credit_label_rect = egui::Rect::from_min_size(
            egui::pos2(credit_rect.max.x + 4.0, credit_rect.min.y),
            egui::vec2(90.0, checkbox_size)
        );
        
        // 绘制元宝复选框
        if !self.pay_with_gold {
            ui.painter().rect_filled(credit_rect, 2.0, egui::Color32::from_rgb(40, 80, 40));
            ui.painter().text(
                credit_rect.center(),
                egui::Align2::CENTER_CENTER,
                "✓",
                egui::FontId::proportional(10.0),
                egui::Color32::GREEN,
            );
        } else {
            ui.painter().rect_filled(credit_rect, 2.0, egui::Color32::from_rgb(60, 60, 80));
        }
        ui.painter().rect_stroke(
            credit_rect, 2.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(150, 150, 170)),
            egui::epaint::StrokeKind::Outside,
        );
        
        let credit_interact_rect = egui::Rect::from_min_max(credit_rect.min, credit_label_rect.max);
        let credit_response = ui.interact(credit_interact_rect, egui::Id::new("pay_credit"), egui::Sense::click());
        
        let credit_text_color = if credit_response.hovered() { 
            egui::Color32::WHITE 
        } else { 
            egui::Color32::GRAY 
        };
        ui.painter().text(
            egui::pos2(credit_label_rect.min.x, credit_label_rect.center().y),
            egui::Align2::LEFT_CENTER,
            "Buy with Credits",
            egui::FontId::proportional(9.0),
            credit_text_color,
        );
        
        if credit_response.clicked() && !self.dragging {
            self.pay_with_gold = false;
        }
    }

    /// 绘制搜索框 (原版位置: 540, 69, 尺寸140x16)
    fn draw_search_box(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context, bg_rect: &egui::Rect) {
        let search_rect = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 540.0, bg_rect.min.y + 69.0),
            egui::vec2(140.0, 16.0)
        );
        
        // 背景
        ui.painter().rect_filled(search_rect, 2.0, egui::Color32::from_rgb(4, 4, 4));
        ui.painter().rect_stroke(
            search_rect, 2.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 120)),
            egui::epaint::StrokeKind::Outside,
        );
        
        // 使用egui的TextEdit widget
        let text_edit_rect = search_rect.shrink(2.0);
        
        // 创建一个子区域来放置TextEdit
        #[allow(deprecated)]
        let _response = ui.allocate_ui_at_rect(text_edit_rect, |ui| {
            ui.style_mut().visuals.extreme_bg_color = egui::Color32::TRANSPARENT;
            ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
            ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
            ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
            
            let text_edit = egui::TextEdit::singleline(&mut self.search_text)
                .hint_text("搜索...")
                .desired_width(130.0)
                .frame(false)
                .text_color(egui::Color32::WHITE);
            
            let response = ui.add(text_edit);
            
            // 如果搜索文本变化，触发过滤
            if response.changed() {
                self.filter_items();
                self.current_page = 0;
                self.quantities = [1; 8];
            }
            
            response
        });
    }

    /// 绘制关闭按钮
    fn draw_close_button(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) -> bool {
        // 使用Prguse2纹理360/361/362绘制关闭按钮
        // 关闭按钮位置（右上角）
        let close_size = egui::vec2(20.0, 20.0);
        let close_rect = egui::Rect::from_min_size(
            egui::pos2(bg_rect.max.x - 25.0, bg_rect.min.y + 5.0),
            close_size
        );
     
        let response = ui.interact(close_rect, egui::Id::new("shop_close"), egui::Sense::click());
        
        // 根据状态选择纹理索引
        let texture_index = if response.clicked() {
            362 // pressed
        } else if response.hovered() {
            361 // hover
        } else {
            360 // normal
        };
        
        // 绘制关闭按钮纹理
        if let Some(info) = LibraryName::Prguse2.get_egui_texture(ctx, texture_index) {
            if let Some(close_texture) = info.egui_texture {
                ui.painter().image(
                    close_texture.id(),
                    close_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        } else {
            // 备用：绘制简单关闭按钮
            ui.painter().rect_filled(close_rect, 2.0, egui::Color32::from_rgb(150, 50, 50));
            ui.painter().rect_stroke(
                close_rect,
                2.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 100, 100)),
                egui::epaint::StrokeKind::Outside,
            );
            ui.painter().text(
                close_rect.center(),
                egui::Align2::CENTER_CENTER,
                "×",
                egui::FontId::proportional(14.0),
                egui::Color32::WHITE,
            );
        }

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
         if !*open {
            return;
        }
        
        // 使用 Area 创建自由浮动窗口
        egui::Area::new(egui::Id::new("game_shop_dialog"))
            .default_pos(self.position)
            .movable(false)  // 使用自定义拖拽
            .interactable(true)
            .show(ctx, |ui| {
                // 绘制背景
                let bg_rect = self.draw_background(ui, ctx);
                
                // 处理窗口拖拽
                self.handle_window_dragging(ui, ctx, &bg_rect);
                
                // 绘制分类标签页
                self.draw_category_tabs(ui, ctx, &bg_rect);
                
                // 绘制左侧分类列表
                self.draw_category_list(ui, ctx, &bg_rect);
                
                // 绘制物品列表
                self.draw_item_grid(ui, ctx, &bg_rect);
                
                // 绘制货币信息
                self.draw_currency_info(ui, &bg_rect);
                
                // 绘制支付方式选择
                self.draw_payment_options(ui, ctx, &bg_rect);
                
                // 绘制搜索框
                self.draw_search_box(ui, ctx, &bg_rect);
                
                // 绘制关闭按钮
                if self.draw_close_button(ui, ctx, &bg_rect) {
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
    }
}