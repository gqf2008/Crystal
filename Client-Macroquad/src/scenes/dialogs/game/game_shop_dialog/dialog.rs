// ============================================================================
// GameShopDialogHybrid - 游戏商城对话框（混合版本）
// ============================================================================
//
// 使用 macroquad 原生绘制 + macroquad::ui 拖拽
// 
// 功能：
// - 商品列表显示（4x2网格）
// - 分类筛选（主分类 + 职业分类）
// - 商品购买
// - 商品预览
// - 分页浏览
// ============================================================================

use macroquad::prelude::*;
use macroquad::ui::{self, Skin};
use crate::resources::LibraryName;

use super::types::{ShopSectionHybrid, ShopClassHybrid, ShopItemHybrid};
use super::sample_items::create_sample_items;

/// 商城对话框（混合版本）
pub struct GameShopDialogHybrid {
    // 状态
    pub(super) visible: bool,
    pub(super) position: Vec2,
    pub(super) dragging: bool,
    pub(super) drag_offset: Vec2,
    
    // 纹理 - 主要
    pub(super) background_texture: Option<Texture2D>,      // Title[749] - 主背景
    pub(super) cell_texture: Option<Texture2D>,            // Title[750] - 商品格子
    pub(super) filter_bg_texture: Option<Texture2D>,       // Title[769] - 分类列表背景
    pub(super) viewer_bg_texture: Option<Texture2D>,       // Title[785] - 预览窗口背景
    
    // 纹理 - Section Tabs (主分类)
    pub(super) section_tab_textures: Vec<(Option<Texture2D>, Option<Texture2D>)>,  // (normal, selected)
    
    // 纹理 - Class Tabs (职业)
    pub(super) class_tab_textures: Vec<(Option<Texture2D>, Option<Texture2D>, Option<Texture2D>)>,  // (normal, hover, pressed)
    
    // 纹理 - 按钮
    pub(super) buy_btn_textures: (Option<Texture2D>, Option<Texture2D>, Option<Texture2D>),  // Title[778-780]
    pub(super) preview_btn_textures: (Option<Texture2D>, Option<Texture2D>, Option<Texture2D>),  // Title[781-783]
    pub(super) close_btn_textures: (Option<Texture2D>, Option<Texture2D>, Option<Texture2D>),  // Prguse2[360-362]
    
    // 纹理 - 滚动条
    pub(super) scroll_up_textures: (Option<Texture2D>, Option<Texture2D>, Option<Texture2D>),  // Prguse2[197-199]
    pub(super) scroll_down_textures: (Option<Texture2D>, Option<Texture2D>, Option<Texture2D>),  // Prguse2[207-209]
    pub(super) scroll_bar_textures: (Option<Texture2D>, Option<Texture2D>),  // Prguse2[205-206]
    
    // 纹理 - 方向按钮
    pub(super) left_btn_textures: (Option<Texture2D>, Option<Texture2D>, Option<Texture2D>),  // Prguse2[240-242]
    pub(super) right_btn_textures: (Option<Texture2D>, Option<Texture2D>, Option<Texture2D>),  // Prguse2[243-245]
    
    // 纹理 - 标题图标
    pub(super) title_label_texture: Option<Texture2D>,  // Title[26]
    
    // 纹理 - 支付方式复选框
    pub(super) checkbox_textures: (Option<Texture2D>, Option<Texture2D>),  // Prguse[2086-2087]
    
    pub(super) transparent_skin: Option<Skin>,
    
    // 分类
    pub current_section: ShopSectionHybrid,
    pub current_class: ShopClassHybrid,
    
    // 商品
    pub shop_items: Vec<ShopItemHybrid>,
    pub filtered_items: Vec<ShopItemHybrid>,
    
    // 分页
    pub current_page: usize,
    pub items_per_page: usize,
    
    // 玩家货币
    pub player_gold: u32,
    pub player_ingot: u32,
    
    // 分类列表
    pub(super) categories: Vec<String>,
    pub(super) category_scroll: usize,
    pub(super) selected_category: Option<usize>,
    
    // 预览
    pub(super) preview_item: Option<usize>,
    pub(super) preview_direction: u8,
    
    // 悬停提示
    pub(super) hover_item: Option<usize>,
    
    // 搜索
    pub(super) search_text: String,
    pub(super) search_active: bool,
    
    // 支付方式
    pub(super) pay_with_gold: bool,
    
    // 购买数量 (每个格子的数量)
    pub(super) quantities: [u8; 8],
}

impl GameShopDialogHybrid {
    const SHOW_ALL_CATEGORY: &'static str = "Show All";

    fn item_matches_section(&self, item: &ShopItemHybrid) -> bool {
        match self.current_section {
            ShopSectionHybrid::All => true,
            ShopSectionHybrid::TopItems => item.hot,
            // 对齐 C#：DealItems
            ShopSectionHybrid::Deals => item.deal,
            // 对齐 C#：NewItems (7 天内)
            ShopSectionHybrid::New => item.days_ago <= 7,
        }
    }

    fn item_matches_class(&self, item: &ShopItemHybrid) -> bool {
        // 对齐 C#：item.Class == ClassFilter || item.Class == "All" || ClassFilter == "Show All"
        self.current_class == ShopClassHybrid::All
            || item.class == ShopClassHybrid::All
            || item.class == self.current_class
    }

    fn item_matches_search(&self, item: &ShopItemHybrid) -> bool {
        if self.search_text.is_empty() {
            return true;
        }
        let needle = self.search_text.to_lowercase();
        item.name.to_lowercase().contains(&needle)
    }

    fn selected_category_name(&self) -> &str {
        self.selected_category
            .and_then(|idx| self.categories.get(idx))
            .map(|s| s.as_str())
            .unwrap_or(Self::SHOW_ALL_CATEGORY)
    }

    /// 与 C# 原版 GetCategories 对齐：基于当前 Section/Class/Search 生成分类列表，并重置为 Show All
    pub fn rebuild_categories(&mut self) {
        let mut list: Vec<String> = vec![Self::SHOW_ALL_CATEGORY.to_string()];

        for item in self.shop_items.iter() {
            if !self.item_matches_class(item) {
                continue;
            }
            if !self.item_matches_section(item) {
                continue;
            }
            if !self.item_matches_search(item) {
                continue;
            }
            let cat = item.category.name();
            if cat.is_empty() {
                continue;
            }
            if !list.iter().any(|s| s == cat) {
                list.push(cat.to_string());
            }
        }

        self.categories = list;
        self.category_scroll = 0;
        self.selected_category = Some(0);
    }

    /// 与 C# 原版行为对齐：重置分类/分页并刷新列表（GetCategories -> UpdateShop）
    pub fn refresh_categories_and_items(&mut self) {
        self.rebuild_categories();
        self.current_page = 0;
        self.preview_item = None;
        self.quantities = [1; 8];
        self.filter_items();
    }

    // 常量 - 基于 egui 版本的原版位置
    // 与 C# 原版一致：Title[749] 实际尺寸为 696x476（MirImageControl.AutoSize = true）
    pub(super) const DIALOG_WIDTH: f32 = 696.0;
    pub(super) const DIALOG_HEIGHT: f32 = 476.0;
    pub(super) const TITLE_HEIGHT: f32 = 35.0;
    
    // 网格位置 (原版)
    pub(super) const GRID_START_X: f32 = 152.0;
    pub(super) const GRID_ROW1_Y: f32 = 115.0;   // 原版: 115
    pub(super) const GRID_ROW2_Y: f32 = 275.0;   // 原版: 275
    pub(super) const CELL_WIDTH: f32 = 125.0;
    pub(super) const CELL_HEIGHT: f32 = 146.0;
    pub(super) const CELL_SPACING: f32 = 132.0;
    
    // Section tabs 位置 (原版: 138, 68)
    pub(super) const SECTION_TAB_X: f32 = 138.0;
    pub(super) const SECTION_TAB_Y: f32 = 68.0;
    pub(super) const SECTION_TAB_W: f32 = 71.0;
    pub(super) const SECTION_TAB_H: f32 = 23.0;
    
    // Class tabs 位置 (原版: 539, 37)
    pub(super) const CLASS_TAB_X: f32 = 539.0;
    pub(super) const CLASS_TAB_Y: f32 = 38.0;
    pub(super) const CLASS_TAB_SIZE: f32 = 23.0;
    
    // 分类列表位置 (原版: 11, 102)
    pub(super) const FILTER_BG_X: f32 = 11.0;
    pub(super) const FILTER_BG_Y: f32 = 102.0;
    // 与 C# 原版 MirLabel Filters 对齐: Location=(15, 103 + 15*i), Size=(90, 20)
    pub(super) const CATEGORY_LIST_X: f32 = 15.0;
    pub(super) const CATEGORY_LIST_Y: f32 = 103.0;
    pub(super) const CATEGORY_ITEM_W: f32 = 90.0;
    pub(super) const CATEGORY_ITEM_H: f32 = 20.0;
    pub(super) const CATEGORY_ITEM_STEP: f32 = 15.0;
    pub(super) const CATEGORY_MAX_VISIBLE: usize = 22;
    
    // 滚动条位置 (原版: 120, 103/421)
    pub(super) const SCROLL_X: f32 = 120.0;
    pub(super) const SCROLL_UP_Y: f32 = 103.0;
    pub(super) const SCROLL_DOWN_Y: f32 = 421.0;
    pub(super) const SCROLL_BTN_W: f32 = 16.0;
    pub(super) const SCROLL_BTN_H: f32 = 14.0;
    
    pub fn new() -> Self {
        let shop_items = create_sample_items();
        let mut dialog = Self {
            visible: false,
            position: vec2(200.0, 100.0),
            dragging: false,
            drag_offset: Vec2::ZERO,
            
            // 纹理初始化
            background_texture: None,
            cell_texture: None,
            filter_bg_texture: None,
            viewer_bg_texture: None,
            section_tab_textures: Vec::new(),
            class_tab_textures: Vec::new(),
            buy_btn_textures: (None, None, None),
            preview_btn_textures: (None, None, None),
            close_btn_textures: (None, None, None),
            scroll_up_textures: (None, None, None),
            scroll_down_textures: (None, None, None),
            scroll_bar_textures: (None, None),
            left_btn_textures: (None, None, None),
            right_btn_textures: (None, None, None),
            title_label_texture: None,
            checkbox_textures: (None, None),
            transparent_skin: None,
            
            current_section: ShopSectionHybrid::All,
            current_class: ShopClassHybrid::All,
            shop_items,
            filtered_items: Vec::new(),
            current_page: 0,
            items_per_page: 8,
            player_gold: 999999,
            player_ingot: 10000,
            categories: Vec::new(),
            category_scroll: 0,
            selected_category: None,
            preview_item: None,
            preview_direction: 6,
            hover_item: None,
            search_text: String::new(),
            search_active: false,
            pay_with_gold: true,
            quantities: [1; 8],
        };
        dialog.refresh_categories_and_items();
        dialog
    }
    
    /// 加载纹理
    pub fn load_textures(&mut self) {
        println!("🛒 GameShopDialogHybrid: 加载纹理...");
        
        // 主要纹理
        if let Some(info) = LibraryName::Title.get_texture(749) {
            self.background_texture = info.image;
            println!("  ✅ Title[749] 主背景: {}x{}", info.width, info.height);
        }
        if let Some(info) = LibraryName::Title.get_texture(750) {
            self.cell_texture = info.image;
            println!("  ✅ Title[750] 商品格子");
        }
        if let Some(info) = LibraryName::Title.get_texture(769) {
            self.filter_bg_texture = info.image;
            println!("  ✅ Title[769] 分类列表背景");
        }
        if let Some(info) = LibraryName::Title.get_texture(785) {
            self.viewer_bg_texture = info.image;
            println!("  ✅ Title[785] 预览窗口背景");
        }
        
        // Section Tabs 纹理 (All: 770/771, Deals: 772/773, New: 774/775, TopItems: 776/777)
        let section_indices = [(770, 771), (776, 777), (772, 773), (774, 775)];  // All, TopItems, Deals, New
        for (normal, selected) in section_indices.iter() {
            let normal_tex = LibraryName::Title.get_texture(*normal).and_then(|i| i.image);
            let selected_tex = LibraryName::Title.get_texture(*selected).and_then(|i| i.image);
            self.section_tab_textures.push((normal_tex, selected_tex));
        }
        println!("  ✅ Section tabs 纹理加载: {} 组", self.section_tab_textures.len());
        
        // Class Tabs 纹理 (每个职业3个状态: normal/hover/pressed)
        let class_indices = [
            (751, 752, 753),  // All
            (754, 755, 756),  // Warrior
            (757, 758, 759),  // Assassin
            (760, 761, 762),  // Taoist
            (763, 764, 765),  // Wizard
            (766, 767, 768),  // Archer
        ];
        for (normal, hover, pressed) in class_indices.iter() {
            let normal_tex = LibraryName::Title.get_texture(*normal).and_then(|i| i.image);
            let hover_tex = LibraryName::Title.get_texture(*hover).and_then(|i| i.image);
            let pressed_tex = LibraryName::Title.get_texture(*pressed).and_then(|i| i.image);
            self.class_tab_textures.push((normal_tex, hover_tex, pressed_tex));
        }
        println!("  ✅ Class tabs 纹理加载: {} 组", self.class_tab_textures.len());
        
        // 按钮纹理
        self.buy_btn_textures = (
            LibraryName::Title.get_texture(778).and_then(|i| i.image),
            LibraryName::Title.get_texture(779).and_then(|i| i.image),
            LibraryName::Title.get_texture(780).and_then(|i| i.image),
        );
        self.preview_btn_textures = (
            LibraryName::Title.get_texture(781).and_then(|i| i.image),
            LibraryName::Title.get_texture(782).and_then(|i| i.image),
            LibraryName::Title.get_texture(783).and_then(|i| i.image),
        );
        println!("  ✅ Buy/Preview 按钮纹理");
        
        // 关闭按钮 (Prguse2[360-362])
        self.close_btn_textures = (
            LibraryName::Prguse2.get_texture(360).and_then(|i| i.image),
            LibraryName::Prguse2.get_texture(361).and_then(|i| i.image),
            LibraryName::Prguse2.get_texture(362).and_then(|i| i.image),
        );
        println!("  ✅ 关闭按钮纹理");
        
        // 滚动条纹理
        self.scroll_up_textures = (
            LibraryName::Prguse2.get_texture(197).and_then(|i| i.image),
            LibraryName::Prguse2.get_texture(198).and_then(|i| i.image),
            LibraryName::Prguse2.get_texture(199).and_then(|i| i.image),
        );
        self.scroll_down_textures = (
            LibraryName::Prguse2.get_texture(207).and_then(|i| i.image),
            LibraryName::Prguse2.get_texture(208).and_then(|i| i.image),
            LibraryName::Prguse2.get_texture(209).and_then(|i| i.image),
        );
        self.scroll_bar_textures = (
            LibraryName::Prguse2.get_texture(205).and_then(|i| i.image),
            LibraryName::Prguse2.get_texture(206).and_then(|i| i.image),
        );
        println!("  ✅ 滚动条纹理");
        
        // 方向按钮纹理
        self.left_btn_textures = (
            LibraryName::Prguse2.get_texture(240).and_then(|i| i.image),
            LibraryName::Prguse2.get_texture(241).and_then(|i| i.image),
            LibraryName::Prguse2.get_texture(242).and_then(|i| i.image),
        );
        self.right_btn_textures = (
            LibraryName::Prguse2.get_texture(243).and_then(|i| i.image),
            LibraryName::Prguse2.get_texture(244).and_then(|i| i.image),
            LibraryName::Prguse2.get_texture(245).and_then(|i| i.image),
        );
        println!("  ✅ 方向按钮纹理");
        
        // 标题图标 Title[26]
        self.title_label_texture = LibraryName::Title.get_texture(26).and_then(|i| i.image);
        println!("  ✅ 标题图标纹理");
        
        // 支付方式复选框 Prguse[2086-2087]
        self.checkbox_textures = (
            LibraryName::Prguse.get_texture(2086).and_then(|i| i.image),
            LibraryName::Prguse.get_texture(2087).and_then(|i| i.image),
        );
        println!("  ✅ 复选框纹理");
        
        // 创建透明皮肤
        self.create_transparent_skin();
        
        println!("  ✅ 商城对话框纹理加载完成");
    }
    
    /// 创建透明皮肤
    fn create_transparent_skin(&mut self) {
        // 创建 1x1 透明像素
        let transparent_pixel = Image {
            bytes: vec![0, 0, 0, 0],
            width: 1,
            height: 1,
        };
        
        // 完全透明的样式
        let transparent_style = ui::root_ui()
            .style_builder()
            .background(transparent_pixel.clone())
            .background_hovered(transparent_pixel.clone())
            .background_clicked(transparent_pixel.clone())
            .color(Color::new(0.0, 0.0, 0.0, 0.0))
            .color_hovered(Color::new(0.0, 0.0, 0.0, 0.0))
            .color_clicked(Color::new(0.0, 0.0, 0.0, 0.0))
            .build();
        
        self.transparent_skin = Some(Skin {
            group_style: transparent_style.clone(),
            button_style: transparent_style.clone(),
            label_style: transparent_style,
            ..ui::root_ui().default_skin()
        });
    }
    
    /// 过滤商品
    pub fn filter_items(&mut self) {
        let selected_cat = self.selected_category_name().to_string();
        let show_all = selected_cat == Self::SHOW_ALL_CATEGORY;

        let mut list: Vec<ShopItemHybrid> = self
            .shop_items
            .iter()
            .filter(|item| {
                self.item_matches_class(item)
                    && self.item_matches_section(item)
                    && self.item_matches_search(item)
                    && (show_all || item.category.name() == selected_cat)
            })
            .cloned()
            .collect();

        // C# 原版对 filteredShop 做 FriendlyName 排序
        list.sort_by(|a, b| a.name.cmp(&b.name));
        self.filtered_items = list;

        // 与 C# UpdateShop 对齐：过滤后回到第一页，并关闭预览
        self.current_page = 0;
        self.preview_item = None;
    }
    
    // 基本控制方法
    pub fn open(&mut self) { self.visible = true; }
    pub fn close(&mut self) { self.visible = false; self.preview_item = None; }
    pub fn toggle(&mut self) { 
        if self.visible { self.close(); } else { self.open(); }
    }
    pub fn is_visible(&self) -> bool { self.visible }
    pub fn set_position(&mut self, pos: Vec2) { self.position = pos; }
    
    /// 检查点是否在对话框区域内
    pub fn contains(&self, pos: Vec2) -> bool {
        if !self.visible { return false; }
        let rect = Rect::new(self.position.x, self.position.y, Self::DIALOG_WIDTH, Self::DIALOG_HEIGHT);
        rect.contains(pos)
    }
}

impl Default for GameShopDialogHybrid {
    fn default() -> Self {
        Self::new()
    }
}
