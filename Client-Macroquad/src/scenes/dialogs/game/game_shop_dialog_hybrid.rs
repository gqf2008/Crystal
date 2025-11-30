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
use macroquad::ui::{self, Skin, hash};
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;

/// 商城主分类
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShopSectionHybrid {
    All,        // 全部
    TopItems,   // 热销
    Deals,      // 特价
    New,        // 新品
}

impl ShopSectionHybrid {
    pub const ALL: &'static [Self] = &[Self::All, Self::TopItems, Self::Deals, Self::New];
    
    pub fn name(&self) -> &'static str {
        match self {
            Self::All => "全部",
            Self::TopItems => "热销",
            Self::Deals => "特价",
            Self::New => "新品",
        }
    }
}

/// 商城职业分类
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShopClassHybrid {
    All,        // 全部
    Warrior,    // 战士
    Assassin,   // 刺客
    Taoist,     // 道士
    Wizard,     // 法师
    Archer,     // 弓箭手
}

impl ShopClassHybrid {
    pub const ALL: &'static [Self] = &[
        Self::All, Self::Warrior, Self::Assassin, 
        Self::Taoist, Self::Wizard, Self::Archer
    ];
    
    pub fn name(&self) -> &'static str {
        match self {
            Self::All => "全",
            Self::Warrior => "战",
            Self::Assassin => "刺",
            Self::Taoist => "道",
            Self::Wizard => "法",
            Self::Archer => "弓",
        }
    }
}

/// 商品分类
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShopCategoryHybrid {
    Weapon,     // 武器
    Armor,      // 防具
    Potion,     // 药品
    Special,    // 特殊
    Fashion,    // 时装
}

/// 商城物品
#[derive(Debug, Clone)]
pub struct ShopItemHybrid {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub icon_index: usize,
    pub price_gold: u32,
    pub price_ingot: u32,
    pub category: ShopCategoryHybrid,
    pub in_stock: bool,
    pub hot: bool,
    pub new: bool,
    pub stock: u32,
    pub count: u32,
}

/// 商城对话框（混合版本）
pub struct GameShopDialogHybrid {
    // 状态
    visible: bool,
    position: Vec2,
    dragging: bool,
    drag_offset: Vec2,
    
    // 纹理 - 主要
    background_texture: Option<Texture2D>,      // Title[749] - 主背景
    cell_texture: Option<Texture2D>,            // Title[750] - 商品格子
    filter_bg_texture: Option<Texture2D>,       // Title[769] - 分类列表背景
    viewer_bg_texture: Option<Texture2D>,       // Title[785] - 预览窗口背景
    
    // 纹理 - Section Tabs (主分类)
    section_tab_textures: Vec<(Option<Texture2D>, Option<Texture2D>)>,  // (normal, selected)
    
    // 纹理 - Class Tabs (职业)
    class_tab_textures: Vec<(Option<Texture2D>, Option<Texture2D>, Option<Texture2D>)>,  // (normal, hover, pressed)
    
    // 纹理 - 按钮
    buy_btn_textures: (Option<Texture2D>, Option<Texture2D>, Option<Texture2D>),  // Title[778-780]
    preview_btn_textures: (Option<Texture2D>, Option<Texture2D>, Option<Texture2D>),  // Title[781-783]
    close_btn_textures: (Option<Texture2D>, Option<Texture2D>, Option<Texture2D>),  // Prguse2[360-362]
    
    // 纹理 - 滚动条
    scroll_up_textures: (Option<Texture2D>, Option<Texture2D>, Option<Texture2D>),  // Prguse2[197-199]
    scroll_down_textures: (Option<Texture2D>, Option<Texture2D>, Option<Texture2D>),  // Prguse2[207-209]
    scroll_bar_textures: (Option<Texture2D>, Option<Texture2D>),  // Prguse2[205-206]
    
    // 纹理 - 方向按钮
    left_btn_textures: (Option<Texture2D>, Option<Texture2D>, Option<Texture2D>),  // Prguse2[240-242]
    right_btn_textures: (Option<Texture2D>, Option<Texture2D>, Option<Texture2D>),  // Prguse2[243-245]
    
    // 纹理 - 标题图标
    title_label_texture: Option<Texture2D>,  // Title[26]
    
    // 纹理 - 支付方式复选框
    checkbox_textures: (Option<Texture2D>, Option<Texture2D>),  // Prguse[2086-2087]
    
    transparent_skin: Option<Skin>,
    
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
    categories: Vec<String>,
    category_scroll: usize,
    
    // 预览
    preview_item: Option<usize>,
    preview_direction: u8,
    
    // 悬停提示
    hover_item: Option<usize>,
    
    // 搜索
    search_text: String,
    search_active: bool,
    
    // 支付方式
    pay_with_gold: bool,
    
    // 购买数量 (每个格子的数量)
    quantities: [u8; 8],
}

impl GameShopDialogHybrid {
    // 常量 - 基于 egui 版本的原版位置
    const DIALOG_WIDTH: f32 = 720.0;
    const DIALOG_HEIGHT: f32 = 480.0;
    const TITLE_HEIGHT: f32 = 35.0;
    
    // 网格位置 (原版)
    const GRID_START_X: f32 = 152.0;
    const GRID_ROW1_Y: f32 = 115.0;   // 原版: 115
    const GRID_ROW2_Y: f32 = 275.0;   // 原版: 275
    const CELL_WIDTH: f32 = 125.0;
    const CELL_HEIGHT: f32 = 146.0;
    const CELL_SPACING: f32 = 132.0;
    
    // Section tabs 位置 (原版: 138, 68)
    const SECTION_TAB_X: f32 = 138.0;
    const SECTION_TAB_Y: f32 = 68.0;
    const SECTION_TAB_W: f32 = 71.0;
    const SECTION_TAB_H: f32 = 23.0;
    
    // Class tabs 位置 (原版: 539, 37)
    const CLASS_TAB_X: f32 = 539.0;
    const CLASS_TAB_Y: f32 = 38.0;
    const CLASS_TAB_SIZE: f32 = 23.0;
    
    // 分类列表位置 (原版: 11, 102)
    const FILTER_BG_X: f32 = 11.0;
    const FILTER_BG_Y: f32 = 102.0;
    const CATEGORY_LIST_X: f32 = 20.0;
    const CATEGORY_LIST_Y: f32 = 120.0;
    const CATEGORY_LINE_HEIGHT: f32 = 14.0;
    const CATEGORY_MAX_VISIBLE: usize = 22;
    
    // 滚动条位置 (原版: 120, 103/421)
    const SCROLL_X: f32 = 120.0;
    const SCROLL_UP_Y: f32 = 103.0;
    const SCROLL_DOWN_Y: f32 = 421.0;
    const SCROLL_BTN_W: f32 = 16.0;
    const SCROLL_BTN_H: f32 = 14.0;
    
    pub fn new() -> Self {
        let shop_items = Self::create_sample_items();
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
            categories: vec![
                "武器".into(), "防具".into(), "头盔".into(), "项链".into(),
                "手镯".into(), "戒指".into(), "腰带".into(), "靴子".into(),
                "药品".into(), "特殊物品".into(), "时装".into(), "宝石".into(),
                "材料".into(), "卷轴".into(), "坐骑".into(), "技能书".into(),
                "消耗品".into(), "任务物品".into(), "其他".into(),
            ],
            category_scroll: 0,
            preview_item: None,
            preview_direction: 6,
            hover_item: None,
            search_text: String::new(),
            search_active: false,
            pay_with_gold: true,
            quantities: [1; 8],
        };
        dialog.filter_items();
        dialog
    }
    
    /// 创建示例商品
    fn create_sample_items() -> Vec<ShopItemHybrid> {
        vec![
            ShopItemHybrid {
                id: 1, name: "龙纹剑".into(), description: "攻击力+50".into(),
                icon_index: 1, price_gold: 100000, price_ingot: 500,
                category: ShopCategoryHybrid::Weapon, in_stock: true,
                hot: true, new: false, stock: 10, count: 1,
            },
            ShopItemHybrid {
                id: 2, name: "天师道袍".into(), description: "魔防+30".into(),
                icon_index: 20, price_gold: 80000, price_ingot: 400,
                category: ShopCategoryHybrid::Armor, in_stock: true,
                hot: false, new: true, stock: 5, count: 1,
            },
            ShopItemHybrid {
                id: 3, name: "强效金疮药".into(), description: "恢复500HP".into(),
                icon_index: 40, price_gold: 1000, price_ingot: 5,
                category: ShopCategoryHybrid::Potion, in_stock: true,
                hot: false, new: false, stock: 0, count: 10,
            },
            ShopItemHybrid {
                id: 4, name: "传送戒指".into(), description: "随机传送".into(),
                icon_index: 60, price_gold: 500000, price_ingot: 2000,
                category: ShopCategoryHybrid::Special, in_stock: false,
                hot: true, new: true, stock: 0, count: 1,
            },
            ShopItemHybrid {
                id: 5, name: "华丽时装".into(), description: "外观装饰".into(),
                icon_index: 80, price_gold: 0, price_ingot: 1000,
                category: ShopCategoryHybrid::Fashion, in_stock: true,
                hot: false, new: true, stock: 20, count: 1,
            },
            ShopItemHybrid {
                id: 6, name: "裁决之杖".into(), description: "攻击力+80".into(),
                icon_index: 5, price_gold: 200000, price_ingot: 1000,
                category: ShopCategoryHybrid::Weapon, in_stock: true,
                hot: true, new: false, stock: 3, count: 1,
            },
            ShopItemHybrid {
                id: 7, name: "法神披风".into(), description: "魔攻+40".into(),
                icon_index: 25, price_gold: 150000, price_ingot: 750,
                category: ShopCategoryHybrid::Armor, in_stock: true,
                hot: false, new: false, stock: 8, count: 1,
            },
            ShopItemHybrid {
                id: 8, name: "太阳水".into(), description: "恢复300MP".into(),
                icon_index: 45, price_gold: 800, price_ingot: 4,
                category: ShopCategoryHybrid::Potion, in_stock: true,
                hot: false, new: false, stock: 0, count: 10,
            },
            ShopItemHybrid {
                id: 9, name: "复活戒指".into(), description: "死亡复活".into(),
                icon_index: 65, price_gold: 1000000, price_ingot: 5000,
                category: ShopCategoryHybrid::Special, in_stock: true,
                hot: true, new: true, stock: 1, count: 1,
            },
            ShopItemHybrid {
                id: 10, name: "新年套装".into(), description: "限定外观".into(),
                icon_index: 85, price_gold: 0, price_ingot: 2000,
                category: ShopCategoryHybrid::Fashion, in_stock: true,
                hot: true, new: true, stock: 50, count: 1,
            },
        ]
    }
    
    /// 加载纹理
    pub async fn load_textures(&mut self) {
        println!("🛒 GameShopDialogHybrid: 加载纹理...");
        
        // 主要纹理
        if let Some(info) = LibraryName::Title.get_texture(749) {
            self.background_texture = info.image;
            println!("  ✅ Title[749] 主背景");
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
        self.filtered_items = self.shop_items.iter().filter(|item| {
            let section_match = match self.current_section {
                ShopSectionHybrid::All => true,
                ShopSectionHybrid::TopItems => item.hot,
                ShopSectionHybrid::Deals => item.price_gold > 0 && item.price_ingot > 0,
                ShopSectionHybrid::New => item.new,
            };
            section_match
        }).cloned().collect();
        
        self.current_page = 0;
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
    
    /// 更新和绘制
    pub fn update_and_draw(&mut self) {
        if !self.visible { return; }
        
        let pos = self.position;
        
        // 1. 绘制背景
        self.draw_background(pos);
        
        // 2. 绘制标题
        self.draw_title(pos);
        
        // 3. 绘制分类标签
        self.draw_section_tabs(pos);
        self.draw_class_tabs(pos);
        
        // 4. 绘制左侧分类列表
        self.draw_category_list(pos);
        
        // 5. 绘制商品网格
        self.draw_item_grid(pos);
        
        // 6. 绘制分页
        self.draw_pagination(pos);
        
        // 7. 绘制货币信息和支付方式
        self.draw_currency_info(pos);
        self.draw_payment_options(pos);
        
        // 8. 绘制搜索框
        self.draw_search_box(pos);
        
        // 9. 绘制预览窗口
        self.draw_preview_window(pos);
        
        // 10. 绘制悬停提示框（需要在最后绘制以显示在最上层）
        self.draw_item_tooltip();
        
        // 11. 处理拖拽（使用mqui）
        self.handle_dragging(pos);
        
        // 12. 处理关闭按钮
        self.handle_close_button(pos);
    }
    
    /// 绘制背景
    fn draw_background(&self, pos: Vec2) {
        if let Some(ref tex) = self.background_texture {
            draw_texture_ex(tex, pos.x, pos.y, WHITE, DrawTextureParams {
                dest_size: Some(vec2(Self::DIALOG_WIDTH, Self::DIALOG_HEIGHT)),
                ..Default::default()
            });
        } else {
            // 备用背景
            draw_rectangle(pos.x, pos.y, Self::DIALOG_WIDTH, Self::DIALOG_HEIGHT, 
                Color::from_rgba(40, 40, 50, 240));
            draw_rectangle_lines(pos.x, pos.y, Self::DIALOG_WIDTH, Self::DIALOG_HEIGHT, 
                2.0, Color::from_rgba(100, 100, 120, 255));
        }
    }
    
    /// 绘制标题
    fn draw_title(&self, pos: Vec2) {
        // 标题图标 Title[26] (原版位置: 18, 9)
        if let Some(ref tex) = self.title_label_texture {
            draw_texture_ex(tex, pos.x + 18.0, pos.y + 9.0, WHITE, DrawTextureParams::default());
        } else {
            draw_text_cn("🛒 游戏商城", pos.x + 20.0, pos.y + 25.0, 18.0, 
                Color::from_rgba(255, 215, 0, 255));
        }
    }
    
    /// 绘制主分类标签 (使用纹理)
    fn draw_section_tabs(&mut self, pos: Vec2) {
        for (i, section) in ShopSectionHybrid::ALL.iter().enumerate() {
            let tab_x = pos.x + Self::SECTION_TAB_X + (i as f32 * Self::SECTION_TAB_W);
            let tab_y = pos.y + Self::SECTION_TAB_Y;
            let is_selected = self.current_section == *section;
            
            let mouse_pos = mouse_position();
            let hovered = mouse_pos.0 >= tab_x && mouse_pos.0 <= tab_x + Self::SECTION_TAB_W
                && mouse_pos.1 >= tab_y && mouse_pos.1 <= tab_y + Self::SECTION_TAB_H;
            
            // 使用纹理或备用绘制
            if i < self.section_tab_textures.len() {
                let (ref normal_tex, ref selected_tex) = self.section_tab_textures[i];
                let tex_to_use = if is_selected || hovered { selected_tex } else { normal_tex };
                
                if let Some(tex) = tex_to_use {
                    draw_texture_ex(tex, tab_x, tab_y, WHITE, DrawTextureParams::default());
                } else {
                    self.draw_fallback_section_tab(tab_x, tab_y, section, is_selected, hovered);
                }
            } else {
                self.draw_fallback_section_tab(tab_x, tab_y, section, is_selected, hovered);
            }
            
            // 处理点击
            if hovered && is_mouse_button_pressed(MouseButton::Left) && !self.dragging {
                self.current_section = *section;
                self.filter_items();
                println!("🏷️ 切换分类: {}", section.name());
            }
        }
    }
    
    /// 备用分类标签绘制
    fn draw_fallback_section_tab(&self, x: f32, y: f32, section: &ShopSectionHybrid, selected: bool, hovered: bool) {
        let bg_color = if selected {
            Color::from_rgba(200, 180, 140, 255)
        } else if hovered {
            Color::from_rgba(100, 100, 140, 255)
        } else {
            Color::from_rgba(60, 60, 80, 255)
        };
        draw_rectangle(x, y, Self::SECTION_TAB_W, Self::SECTION_TAB_H, bg_color);
        draw_rectangle_lines(x, y, Self::SECTION_TAB_W, Self::SECTION_TAB_H, 1.0, 
            Color::from_rgba(150, 150, 170, 255));
        
        let text_color = if selected { BLACK } else { WHITE };
        draw_text_cn(section.name(), x + 15.0, y + 16.0, 12.0, text_color);
    }
    
    /// 绘制职业分类标签 (使用纹理)
    fn draw_class_tabs(&mut self, pos: Vec2) {
        for (i, class) in ShopClassHybrid::ALL.iter().enumerate() {
            let tab_x = pos.x + Self::CLASS_TAB_X + (i as f32 * Self::CLASS_TAB_SIZE);
            let tab_y = pos.y + Self::CLASS_TAB_Y;
            let is_selected = self.current_class == *class;
            
            let mouse_pos = mouse_position();
            let hovered = mouse_pos.0 >= tab_x && mouse_pos.0 <= tab_x + Self::CLASS_TAB_SIZE
                && mouse_pos.1 >= tab_y && mouse_pos.1 <= tab_y + Self::CLASS_TAB_SIZE - 3.0;
            let pressed = hovered && is_mouse_button_down(MouseButton::Left);
            
            // 使用纹理或备用绘制
            if i < self.class_tab_textures.len() {
                let (ref normal_tex, ref hover_tex, ref pressed_tex) = self.class_tab_textures[i];
                let tex_to_use = if pressed {
                    pressed_tex
                } else if is_selected || hovered {
                    hover_tex
                } else {
                    normal_tex
                };
                
                if let Some(tex) = tex_to_use {
                    draw_texture_ex(tex, tab_x, tab_y, WHITE, DrawTextureParams::default());
                } else {
                    self.draw_fallback_class_tab(tab_x, tab_y, class, is_selected, hovered);
                }
            } else {
                self.draw_fallback_class_tab(tab_x, tab_y, class, is_selected, hovered);
            }
            
            // 处理点击
            if hovered && is_mouse_button_pressed(MouseButton::Left) && !self.dragging {
                self.current_class = *class;
                self.filter_items();
                println!("🏷️ 切换职业: {:?}", class);
            }
        }
    }
    
    /// 备用职业标签绘制
    fn draw_fallback_class_tab(&self, x: f32, y: f32, class: &ShopClassHybrid, selected: bool, hovered: bool) {
        let bg_color = if selected {
            Color::from_rgba(200, 180, 140, 255)
        } else if hovered {
            Color::from_rgba(100, 100, 140, 255)
        } else {
            Color::from_rgba(60, 60, 80, 255)
        };
        draw_rectangle(x, y, Self::CLASS_TAB_SIZE, Self::CLASS_TAB_SIZE - 3.0, bg_color);
        
        let text_color = if selected { BLACK } else { WHITE };
        draw_text_cn(class.name(), x + 4.0, y + 14.0, 12.0, text_color);
    }
    
    /// 绘制左侧分类列表
    fn draw_category_list(&mut self, pos: Vec2) {
        // 绘制分类列表背景 Title[769]
        let filter_bg_x = pos.x + Self::FILTER_BG_X;
        let filter_bg_y = pos.y + Self::FILTER_BG_Y;
        
        if let Some(ref tex) = self.filter_bg_texture {
            draw_texture_ex(tex, filter_bg_x, filter_bg_y, WHITE, DrawTextureParams::default());
        } else {
            // 备用背景
            draw_rectangle(filter_bg_x, filter_bg_y, 110.0, 340.0,
                Color::from_rgba(30, 30, 40, 200));
        }
        
        // 绘制分类项
        let list_x = pos.x + Self::CATEGORY_LIST_X;
        let list_y = pos.y + Self::CATEGORY_LIST_Y;
        
        for i in 0..Self::CATEGORY_MAX_VISIBLE {
            let idx = self.category_scroll + i;
            if idx >= self.categories.len() { break; }
            
            let item_y = list_y + (i as f32 * Self::CATEGORY_LINE_HEIGHT);
            let mouse_pos = mouse_position();
            let hovered = mouse_pos.0 >= list_x && mouse_pos.0 <= list_x + 100.0
                && mouse_pos.1 >= item_y && mouse_pos.1 <= item_y + Self::CATEGORY_LINE_HEIGHT;
            
            // 悬停效果
            if hovered {
                draw_rectangle(list_x - 5.0, item_y, 100.0, Self::CATEGORY_LINE_HEIGHT,
                    Color::from_rgba(80, 80, 100, 150));
            }
            
            // 文字
            let text_color = if hovered {
                Color::from_rgba(255, 215, 0, 255)
            } else {
                WHITE
            };
            draw_text_cn(&self.categories[idx], list_x, item_y + 11.0, 9.0, text_color);
            
            // 点击
            if hovered && is_mouse_button_pressed(MouseButton::Left) && !self.dragging {
                println!("📁 选择分类: {}", self.categories[idx]);
            }
        }
        
        // 滚动条
        self.draw_category_scrollbar(pos);
    }
    
    /// 绘制分类滚动条 (使用纹理)
    fn draw_category_scrollbar(&mut self, pos: Vec2) {
        let scroll_x = pos.x + Self::SCROLL_X;
        let mouse_pos = mouse_position();
        
        // 上箭头 Prguse2[197-199]
        let up_y = pos.y + Self::SCROLL_UP_Y;
        let up_hovered = mouse_pos.0 >= scroll_x && mouse_pos.0 <= scroll_x + Self::SCROLL_BTN_W
            && mouse_pos.1 >= up_y && mouse_pos.1 <= up_y + Self::SCROLL_BTN_H;
        let up_pressed = up_hovered && is_mouse_button_down(MouseButton::Left);
        
        let up_tex = if up_pressed {
            &self.scroll_up_textures.2
        } else if up_hovered {
            &self.scroll_up_textures.1
        } else {
            &self.scroll_up_textures.0
        };
        
        if let Some(tex) = up_tex {
            draw_texture_ex(tex, scroll_x, up_y, WHITE, DrawTextureParams::default());
        } else {
            draw_rectangle(scroll_x, up_y, Self::SCROLL_BTN_W, Self::SCROLL_BTN_H, 
                Color::from_rgba(80, 80, 100, 255));
            draw_text_cn("▲", scroll_x + 2.0, up_y + 10.0, 10.0, WHITE);
        }
        
        if up_hovered && is_mouse_button_pressed(MouseButton::Left) {
            if self.category_scroll > 0 {
                self.category_scroll -= 1;
            }
        }
        
        // 下箭头 Prguse2[207-209]
        let down_y = pos.y + Self::SCROLL_DOWN_Y;
        let down_hovered = mouse_pos.0 >= scroll_x && mouse_pos.0 <= scroll_x + Self::SCROLL_BTN_W
            && mouse_pos.1 >= down_y && mouse_pos.1 <= down_y + Self::SCROLL_BTN_H;
        let down_pressed = down_hovered && is_mouse_button_down(MouseButton::Left);
        
        let down_tex = if down_pressed {
            &self.scroll_down_textures.2
        } else if down_hovered {
            &self.scroll_down_textures.1
        } else {
            &self.scroll_down_textures.0
        };
        
        if let Some(tex) = down_tex {
            draw_texture_ex(tex, scroll_x, down_y, WHITE, DrawTextureParams::default());
        } else {
            draw_rectangle(scroll_x, down_y, Self::SCROLL_BTN_W, Self::SCROLL_BTN_H, 
                Color::from_rgba(80, 80, 100, 255));
            draw_text_cn("▼", scroll_x + 2.0, down_y + 10.0, 10.0, WHITE);
        }
        
        if down_hovered && is_mouse_button_pressed(MouseButton::Left) {
            let max_scroll = self.categories.len().saturating_sub(Self::CATEGORY_MAX_VISIBLE);
            if self.category_scroll < max_scroll {
                self.category_scroll += 1;
            }
        }
        
        // 滚动块 Prguse2[205-206]
        let scrollbar_height = Self::SCROLL_DOWN_Y - Self::SCROLL_UP_Y - Self::SCROLL_BTN_H;
        let scroll_ratio = if self.categories.len() > Self::CATEGORY_MAX_VISIBLE {
            self.category_scroll as f32 / (self.categories.len() - Self::CATEGORY_MAX_VISIBLE) as f32
        } else {
            0.0
        };
        
        let bar_y = pos.y + Self::SCROLL_UP_Y + Self::SCROLL_BTN_H + (scroll_ratio * (scrollbar_height - 20.0));
        let bar_hovered = mouse_pos.0 >= scroll_x && mouse_pos.0 <= scroll_x + Self::SCROLL_BTN_W
            && mouse_pos.1 >= bar_y && mouse_pos.1 <= bar_y + 20.0;
        
        let bar_tex = if bar_hovered { &self.scroll_bar_textures.1 } else { &self.scroll_bar_textures.0 };
        
        if let Some(tex) = bar_tex {
            draw_texture_ex(tex, scroll_x, bar_y, WHITE, DrawTextureParams::default());
        } else {
            draw_rectangle(scroll_x, bar_y, Self::SCROLL_BTN_W, 20.0, 
                Color::from_rgba(100, 100, 120, 255));
        }
    }
    
    /// 绘制商品网格
    fn draw_item_grid(&mut self, pos: Vec2) {
        // 每帧重置悬停状态
        self.hover_item = None;
        
        let start_idx = self.current_page * self.items_per_page;
        
        for i in 0..self.items_per_page {
            let item_idx = start_idx + i;
            
            // 计算网格位置
            let grid_x = if i < 4 {
                pos.x + Self::GRID_START_X + (i as f32 * Self::CELL_SPACING)
            } else {
                pos.x + Self::GRID_START_X + ((i - 4) as f32 * Self::CELL_SPACING)
            };
            let grid_y = if i < 4 {
                pos.y + Self::GRID_ROW1_Y
            } else {
                pos.y + Self::GRID_ROW2_Y
            };
            
            // 绘制商品格子
            if item_idx < self.filtered_items.len() {
                self.draw_item_cell(grid_x, grid_y, item_idx);
            } else {
                // 空格子
                self.draw_empty_cell(grid_x, grid_y);
            }
        }
    }
    
    /// 绘制商品格子 (基于原版位置)
    fn draw_item_cell(&mut self, x: f32, y: f32, item_idx: usize) {
        let item = &self.filtered_items[item_idx];
        
        // 格子背景 Title[750]
        if let Some(ref tex) = self.cell_texture {
            draw_texture_ex(tex, x, y, WHITE, DrawTextureParams::default());
        } else {
            draw_rectangle(x, y, Self::CELL_WIDTH, Self::CELL_HEIGHT,
                Color::from_rgba(50, 50, 60, 255));
            draw_rectangle_lines(x, y, Self::CELL_WIDTH, Self::CELL_HEIGHT,
                1.0, Color::from_rgba(100, 100, 120, 255));
        }
        
        // 物品名称 (原版位置: 0, 13 居中)
        let name_color = if item.in_stock {
            Color::from_rgba(255, 215, 0, 255)
        } else {
            GRAY
        };
        draw_text_cn(&item.name, x + Self::CELL_WIDTH / 2.0 - 20.0, y + 16.0, 10.0, name_color);
        
        // 物品图标 (原版位置: 12, 40, 尺寸32x32)
        let icon_x = x + 12.0;
        let icon_y = y + 40.0;
        let icon_w = 32.0;
        let icon_h = 32.0;
        
        // 检测图标区域悬停（用于显示物品提示框）
        let mouse_pos = mouse_position();
        let icon_hovered = mouse_pos.0 >= icon_x && mouse_pos.0 <= icon_x + icon_w
            && mouse_pos.1 >= icon_y && mouse_pos.1 <= icon_y + icon_h;
        if icon_hovered {
            self.hover_item = Some(item_idx);
        }
        
        if let Some(info) = LibraryName::Items.get_texture(item.icon_index) {
            if let Some(ref tex) = info.image {
                // 居中绘制
                let tex_w = info.width as f32;
                let tex_h = info.height as f32;
                let offset_x = (32.0 - tex_w.min(32.0)) / 2.0;
                let offset_y = (32.0 - tex_h.min(32.0)) / 2.0;
                draw_texture_ex(tex, icon_x + offset_x, icon_y + offset_y, WHITE, DrawTextureParams {
                    dest_size: Some(vec2(tex_w.min(32.0), tex_h.min(32.0))),
                    ..Default::default()
                });
            }
        } else {
            draw_rectangle(icon_x, icon_y, 32.0, 32.0, Color::from_rgba(60, 60, 70, 255));
        }
        
        // STOCK标签 (原版位置: 53, 37)
        draw_text_cn("STOCK:", x + 53.0, y + 45.0, 7.0, GRAY);
        
        // 库存数量 (原版位置: 93, 37)
        let stock_text = if item.stock >= 99 {
            "99+".to_string()
        } else if item.stock == 0 {
            "∞".to_string()
        } else {
            item.stock.to_string()
        };
        draw_text_cn(&stock_text, x + 93.0, y + 45.0, 7.0, WHITE);
        
        // 购买数量选择按钮 (原版位置: quantityDown=55,56 quantityUp=97,56 quantity=74,56)
        // 计算当前格子在页面中的索引 (0-7)
        let start_idx = self.current_page * self.items_per_page;
        let grid_idx = item_idx - start_idx;
        
        if grid_idx < 8 {
            let qty = self.quantities[grid_idx];
            
            // 减少按钮 Prguse2[240-242] (原版位置: 55, 56)
            let down_x = x + 55.0;
            let down_y = y + 56.0;
            let btn_w = 16.0;
            let btn_h = 14.0;
            
            let down_hovered = mouse_pos.0 >= down_x && mouse_pos.0 <= down_x + btn_w
                && mouse_pos.1 >= down_y && mouse_pos.1 <= down_y + btn_h;
            let down_pressed = down_hovered && is_mouse_button_down(MouseButton::Left);
            
            let down_tex = if down_pressed {
                &self.left_btn_textures.2
            } else if down_hovered {
                &self.left_btn_textures.1
            } else {
                &self.left_btn_textures.0
            };
            
            if let Some(tex) = down_tex {
                draw_texture_ex(tex, down_x, down_y, WHITE, DrawTextureParams::default());
            } else {
                let color = if down_hovered {
                    Color::from_rgba(120, 120, 160, 255)
                } else {
                    Color::from_rgba(80, 80, 120, 255)
                };
                draw_rectangle(down_x, down_y, btn_w, btn_h, color);
                draw_text_cn("-", down_x + 5.0, down_y + 10.0, 10.0, WHITE);
            }
            
            if down_hovered && is_mouse_button_pressed(MouseButton::Left) && !self.dragging {
                if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
                    self.quantities[grid_idx] = qty.saturating_sub(10).max(1);
                } else {
                    self.quantities[grid_idx] = qty.saturating_sub(1).max(1);
                }
            }
            
            // 数量显示 (原版位置: 74, 56, 尺寸20x13)
            let qty_x = x + 74.0;
            let qty_y = y + 56.0;
            draw_text_cn(&qty.to_string(), qty_x + 4.0, qty_y + 10.0, 8.0, WHITE);
            
            // 增加按钮 Prguse2[243-245] (原版位置: 97, 56)
            let up_x = x + 97.0;
            let up_y = y + 56.0;
            
            let up_hovered = mouse_pos.0 >= up_x && mouse_pos.0 <= up_x + btn_w
                && mouse_pos.1 >= up_y && mouse_pos.1 <= up_y + btn_h;
            let up_pressed = up_hovered && is_mouse_button_down(MouseButton::Left);
            
            let up_tex = if up_pressed {
                &self.right_btn_textures.2
            } else if up_hovered {
                &self.right_btn_textures.1
            } else {
                &self.right_btn_textures.0
            };
            
            if let Some(tex) = up_tex {
                draw_texture_ex(tex, up_x, up_y, WHITE, DrawTextureParams::default());
            } else {
                let color = if up_hovered {
                    Color::from_rgba(120, 120, 160, 255)
                } else {
                    Color::from_rgba(80, 80, 120, 255)
                };
                draw_rectangle(up_x, up_y, btn_w, btn_h, color);
                draw_text_cn("+", up_x + 4.0, up_y + 10.0, 10.0, WHITE);
            }
            
            if up_hovered && is_mouse_button_pressed(MouseButton::Left) && !self.dragging {
                let max_qty = if item.stock > 0 && item.stock < 99 {
                    item.stock as u8
                } else {
                    99
                };
                if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
                    self.quantities[grid_idx] = (qty + 10).min(max_qty);
                } else {
                    self.quantities[grid_idx] = (qty + 1).min(max_qty);
                }
            }
        }
        
        // 物品数量 (原版位置: 16, 60)
        if item.count > 1 {
            draw_text_cn(&format!("x{}", item.count), x + 16.0, y + 68.0, 7.0, WHITE);
        }
        
        // 元宝价格 (原版位置: 2, 81 右对齐)
        if item.price_ingot > 0 {
            draw_text_cn(&format!("{}", item.price_ingot), x + 75.0, y + 89.0, 8.0,
                Color::from_rgba(0, 255, 255, 255));
        }
        
        // 金币价格 (原版位置: 2, 102 右对齐)
        if item.price_gold > 0 {
            draw_text_cn(&format!("{}", item.price_gold), x + 75.0, y + 110.0, 8.0,
                Color::from_rgba(255, 215, 0, 255));
        }
        
        // 热销/新品标记
        if item.hot {
            draw_text_cn("🔥", x + Self::CELL_WIDTH - 18.0, y + 12.0, 12.0, RED);
        }
        if item.new {
            draw_text_cn("NEW", x + 5.0, y + 12.0, 7.0, GREEN);
        }
        
        let is_previewable = matches!(item.category, ShopCategoryHybrid::Weapon | ShopCategoryHybrid::Armor);
        
        // Preview按钮 Title[781-783] (原版位置: 8, 122)
        if is_previewable {
            let preview_x = x + 8.0;
            let preview_y = y + 122.0;
            
            // 使用纹理实际尺寸
            let (preview_w, preview_h) = if let Some(ref tex) = self.preview_btn_textures.0 {
                (tex.width(), tex.height())
            } else {
                (32.0, 24.0)  // 默认尺寸
            };
            
            let preview_hovered = mouse_pos.0 >= preview_x && mouse_pos.0 <= preview_x + preview_w
                && mouse_pos.1 >= preview_y && mouse_pos.1 <= preview_y + preview_h;
            let preview_pressed = preview_hovered && is_mouse_button_down(MouseButton::Left);
            
            let preview_tex = if preview_pressed {
                &self.preview_btn_textures.2
            } else if preview_hovered {
                &self.preview_btn_textures.1
            } else {
                &self.preview_btn_textures.0
            };
            
            if let Some(tex) = preview_tex {
                draw_texture_ex(tex, preview_x, preview_y, WHITE, DrawTextureParams::default());
            } else {
                let color = if preview_hovered {
                    Color::from_rgba(120, 120, 180, 255)
                } else {
                    Color::from_rgba(80, 80, 140, 255)
                };
                draw_rectangle(preview_x, preview_y, preview_w, preview_h, color);
            }
            
            if preview_hovered && is_mouse_button_pressed(MouseButton::Left) && !self.dragging {
                self.preview_item = Some(item_idx);
                println!("👁️ 预览: {}", item.name);
            }
        }
        
        // Buy按钮 Title[778-780] (原版位置: 42/75, 122)
        let buy_x = if is_previewable { x + 75.0 } else { x + 42.0 };
        let buy_y = y + 122.0;
        
        // 使用纹理实际尺寸
        let (buy_w, buy_h) = if let Some(ref tex) = self.buy_btn_textures.0 {
            (tex.width(), tex.height())
        } else {
            (32.0, 24.0)  // 默认尺寸
        };
        
        let buy_hovered = mouse_pos.0 >= buy_x && mouse_pos.0 <= buy_x + buy_w
            && mouse_pos.1 >= buy_y && mouse_pos.1 <= buy_y + buy_h;
        let buy_pressed = buy_hovered && is_mouse_button_down(MouseButton::Left);
        
        let buy_tex = if buy_pressed {
            &self.buy_btn_textures.2
        } else if buy_hovered {
            &self.buy_btn_textures.1
        } else {
            &self.buy_btn_textures.0
        };
        
        if let Some(tex) = buy_tex {
            draw_texture_ex(tex, buy_x, buy_y, WHITE, DrawTextureParams::default());
        } else {
            let color = if buy_hovered {
                Color::from_rgba(120, 180, 120, 255)
            } else {
                Color::from_rgba(80, 140, 80, 255)
            };
            draw_rectangle(buy_x, buy_y, buy_w, buy_h, color);
        }
        
        if buy_hovered && is_mouse_button_pressed(MouseButton::Left) && !self.dragging {
            println!("💰 购买: {}", item.name);
        }
    }
    
    /// 绘制空格子 (原版也使用 Title[750] 纹理)
    fn draw_empty_cell(&self, x: f32, y: f32) {
        // 原版中空格子也会绘制背景纹理 Title[750]
        if let Some(ref tex) = self.cell_texture {
            draw_texture_ex(tex, x, y, WHITE, DrawTextureParams::default());
        } else {
            // 备用绘制
            draw_rectangle(x, y, Self::CELL_WIDTH, Self::CELL_HEIGHT,
                Color::from_rgba(40, 40, 50, 150));
            draw_rectangle_lines(x, y, Self::CELL_WIDTH, Self::CELL_HEIGHT,
                1.0, Color::from_rgba(60, 60, 70, 255));
        }
    }
    
    /// 绘制分页控制 (原版位置: PageLabel=597,446(83x17) PreviousButton=600,448 NextButton=660,448)
    fn draw_pagination(&mut self, pos: Vec2) {
        let total_pages = if self.filtered_items.is_empty() {
            1
        } else {
            (self.filtered_items.len() + self.items_per_page - 1) / self.items_per_page
        };
        
        let mouse_pos = mouse_position();
        
        // 上一页按钮 Prguse2[240-242] (原版: 600, 448)
        let prev_x = pos.x + 600.0;
        let prev_y = pos.y + 448.0;
        let btn_w = 16.0;
        let btn_h = 14.0;
        
        let prev_hovered = mouse_pos.0 >= prev_x && mouse_pos.0 <= prev_x + btn_w
            && mouse_pos.1 >= prev_y && mouse_pos.1 <= prev_y + btn_h;
        let prev_pressed = prev_hovered && is_mouse_button_down(MouseButton::Left);
        
        // 使用 left_btn_textures (Prguse2[240-242])
        let prev_tex = if prev_pressed {
            &self.left_btn_textures.2
        } else if prev_hovered {
            &self.left_btn_textures.1
        } else {
            &self.left_btn_textures.0
        };
        
        if let Some(tex) = prev_tex {
            draw_texture_ex(tex, prev_x, prev_y, WHITE, DrawTextureParams::default());
        } else {
            let color = if prev_hovered {
                Color::from_rgba(120, 120, 160, 255)
            } else {
                Color::from_rgba(80, 80, 120, 255)
            };
            draw_rectangle(prev_x, prev_y, btn_w, btn_h, color);
            draw_text_cn("◀", prev_x + 3.0, prev_y + 10.0, 10.0, WHITE);
        }
        
        if prev_hovered && is_mouse_button_pressed(MouseButton::Left) && self.current_page > 0 {
            self.current_page -= 1;
            self.preview_item = None;
            self.quantities = [1; 8];  // 重置购买数量
            println!("📄 上一页: {}", self.current_page + 1);
        }
        
        // 下一页按钮 Prguse2[243-245] (原版: 660, 448)
        let next_x = pos.x + 660.0;
        let next_y = pos.y + 448.0;
        
        let next_hovered = mouse_pos.0 >= next_x && mouse_pos.0 <= next_x + btn_w
            && mouse_pos.1 >= next_y && mouse_pos.1 <= next_y + btn_h;
        let next_pressed = next_hovered && is_mouse_button_down(MouseButton::Left);
        
        // 使用 right_btn_textures (Prguse2[243-245])
        let next_tex = if next_pressed {
            &self.right_btn_textures.2
        } else if next_hovered {
            &self.right_btn_textures.1
        } else {
            &self.right_btn_textures.0
        };
        
        if let Some(tex) = next_tex {
            draw_texture_ex(tex, next_x, next_y, WHITE, DrawTextureParams::default());
        } else {
            let color = if next_hovered {
                Color::from_rgba(120, 120, 160, 255)
            } else {
                Color::from_rgba(80, 80, 120, 255)
            };
            draw_rectangle(next_x, next_y, btn_w, btn_h, color);
            draw_text_cn("▶", next_x + 3.0, next_y + 10.0, 10.0, WHITE);
        }
        
        if next_hovered && is_mouse_button_pressed(MouseButton::Left) && self.current_page < total_pages - 1 {
            self.current_page += 1;
            self.preview_item = None;
            self.quantities = [1; 8];  // 重置购买数量
            println!("📄 下一页: {}", self.current_page + 1);
        }
        
        // 页码显示 (原版: 597, 446, 尺寸83x17, 居中对齐)
        // 页码在按钮上方显示，居中在83px宽度内
        let page_label_x = pos.x + 597.0;
        let page_label_y = pos.y + 446.0;
        let page_text = format!("{} / {}", self.current_page + 1, total_pages);
        // 83px宽度内居中 (597 + 83/2 = 638.5)
        draw_text_cn(&page_text, page_label_x + 30.0, page_label_y + 11.0, 9.0, WHITE);
    }
    
    /// 绘制货币信息 (原版位置: totalCredits=5,449 totalGold=123,449)
    fn draw_currency_info(&self, pos: Vec2) {
        // 元宝显示 (原版位置: 5, 449, 右对齐100宽)
        let credits_x = pos.x + 5.0;
        let credits_y = pos.y + 449.0;
        draw_text_cn(&format!("{}", self.player_ingot), credits_x + 60.0, credits_y + 12.0,
            10.0, Color::from_rgba(0, 255, 255, 255));
        
        // 金币显示 (原版位置: 123, 449, 右对齐100宽)
        let gold_x = pos.x + 123.0;
        let gold_y = pos.y + 449.0;
        draw_text_cn(&format!("{}", self.player_gold), gold_x + 60.0, gold_y + 12.0, 
            10.0, Color::from_rgba(255, 215, 0, 255));
    }
    
    /// 绘制支付方式选择 (原版位置: PaymentTypeGold=250,449 PaymentTypeCredit=340,449)
    fn draw_payment_options(&mut self, pos: Vec2) {
        let mouse_pos = mouse_position();
        
        // Buy with Gold 复选框 (原版位置: 250, 449)
        let gold_x = pos.x + 250.0;
        let gold_y = pos.y + 449.0;
        let checkbox_size = 14.0;
        
        let gold_hovered = mouse_pos.0 >= gold_x && mouse_pos.0 <= gold_x + 120.0
            && mouse_pos.1 >= gold_y && mouse_pos.1 <= gold_y + checkbox_size;
        
        // 绘制复选框
        let gold_tex = if self.pay_with_gold {
            &self.checkbox_textures.1  // 选中
        } else {
            &self.checkbox_textures.0  // 未选中
        };
        
        if let Some(tex) = gold_tex {
            draw_texture_ex(tex, gold_x, gold_y, WHITE, DrawTextureParams::default());
        } else {
            draw_rectangle(gold_x, gold_y, checkbox_size, checkbox_size, 
                Color::from_rgba(60, 60, 80, 255));
            draw_rectangle_lines(gold_x, gold_y, checkbox_size, checkbox_size,
                1.0, Color::from_rgba(150, 150, 170, 255));
            if self.pay_with_gold {
                draw_text_cn("✓", gold_x + 2.0, gold_y + 11.0, 10.0, GREEN);
            }
        }
        draw_text_cn("Buy with Gold", gold_x + 18.0, gold_y + 11.0, 9.0, 
            if gold_hovered { WHITE } else { GRAY });
        
        if gold_hovered && is_mouse_button_pressed(MouseButton::Left) && !self.dragging {
            self.pay_with_gold = true;
        }
        
        // Buy with Credits 复选框 (原版位置: 340, 449)
        let credit_x = pos.x + 340.0;
        let credit_y = pos.y + 449.0;
        
        let credit_hovered = mouse_pos.0 >= credit_x && mouse_pos.0 <= credit_x + 130.0
            && mouse_pos.1 >= credit_y && mouse_pos.1 <= credit_y + checkbox_size;
        
        let credit_tex = if !self.pay_with_gold {
            &self.checkbox_textures.1  // 选中
        } else {
            &self.checkbox_textures.0  // 未选中
        };
        
        if let Some(tex) = credit_tex {
            draw_texture_ex(tex, credit_x, credit_y, WHITE, DrawTextureParams::default());
        } else {
            draw_rectangle(credit_x, credit_y, checkbox_size, checkbox_size, 
                Color::from_rgba(60, 60, 80, 255));
            draw_rectangle_lines(credit_x, credit_y, checkbox_size, checkbox_size,
                1.0, Color::from_rgba(150, 150, 170, 255));
            if !self.pay_with_gold {
                draw_text_cn("✓", credit_x + 2.0, credit_y + 11.0, 10.0, GREEN);
            }
        }
        draw_text_cn("Buy with Credits", credit_x + 18.0, credit_y + 11.0, 9.0,
            if credit_hovered { WHITE } else { GRAY });
        
        if credit_hovered && is_mouse_button_pressed(MouseButton::Left) && !self.dragging {
            self.pay_with_gold = false;
        }
    }
    
    /// 绘制搜索框 (原版位置: 540, 69, 尺寸140x16)
    fn draw_search_box(&mut self, pos: Vec2) {
        let search_x = pos.x + 540.0;
        let search_y = pos.y + 69.0;
        let search_w = 140.0;
        let search_h = 16.0;
        
        let mouse_pos = mouse_position();
        let hovered = mouse_pos.0 >= search_x && mouse_pos.0 <= search_x + search_w
            && mouse_pos.1 >= search_y && mouse_pos.1 <= search_y + search_h;
        
        // 背景
        let bg_color = if self.search_active {
            Color::from_rgba(20, 20, 30, 255)
        } else if hovered {
            Color::from_rgba(15, 15, 25, 255)
        } else {
            Color::from_rgba(4, 4, 4, 255)
        };
        draw_rectangle(search_x, search_y, search_w, search_h, bg_color);
        draw_rectangle_lines(search_x, search_y, search_w, search_h,
            1.0, if self.search_active { 
                Color::from_rgba(100, 150, 200, 255)
            } else {
                Color::from_rgba(80, 80, 100, 255)
            });
        
        // 点击激活搜索框
        if hovered && is_mouse_button_pressed(MouseButton::Left) {
            self.search_active = true;
        } else if !hovered && is_mouse_button_pressed(MouseButton::Left) {
            self.search_active = false;
        }
        
        // 显示搜索文本或占位符
        if self.search_text.is_empty() && !self.search_active {
            draw_text_cn("搜索...", search_x + 4.0, search_y + 12.0, 9.0, GRAY);
        } else {
            draw_text_cn(&self.search_text, search_x + 4.0, search_y + 12.0, 9.0, WHITE);
            // 光标
            if self.search_active {
                let cursor_x = search_x + 4.0 + self.search_text.chars().count() as f32 * 6.0;
                if (get_time() * 2.0) as i32 % 2 == 0 {
                    draw_line(cursor_x, search_y + 3.0, cursor_x, search_y + 13.0, 1.0, WHITE);
                }
            }
        }
        
        // 处理键盘输入
        if self.search_active {
            // 退格删除
            if is_key_pressed(KeyCode::Backspace) && !self.search_text.is_empty() {
                self.search_text.pop();
                self.filter_items();
            }
            // ESC取消
            if is_key_pressed(KeyCode::Escape) {
                self.search_active = false;
            }
            // 获取输入字符 (简化处理，只支持基本ASCII)
            for key in get_keys_pressed() {
                if let Some(c) = key_to_char(key) {
                    if self.search_text.len() < 23 {
                        self.search_text.push(c);
                        self.filter_items();
                    }
                }
            }
        }
    }
    
    /// 绘制预览窗口 (使用 Title[785] 纹理)
    fn draw_preview_window(&mut self, pos: Vec2) {
        if let Some(idx) = self.preview_item {
            if idx >= self.filtered_items.len() {
                self.preview_item = None;
                return;
            }
            
            let item = &self.filtered_items[idx];
            let preview_w = 260.0;
            let preview_h = 300.0;
            let preview_x = pos.x + Self::DIALOG_WIDTH - preview_w - 30.0;
            let preview_y = pos.y + 120.0;
            
            // 半透明遮罩
            draw_rectangle(pos.x, pos.y, Self::DIALOG_WIDTH, Self::DIALOG_HEIGHT,
                Color::from_rgba(0, 0, 0, 80));
            
            // 预览窗口背景 Title[785]
            if let Some(ref tex) = self.viewer_bg_texture {
                draw_texture_ex(tex, preview_x, preview_y, WHITE, DrawTextureParams::default());
            } else {
                draw_rectangle(preview_x, preview_y, preview_w, preview_h,
                    Color::from_rgba(40, 40, 50, 250));
                draw_rectangle_lines(preview_x, preview_y, preview_w, preview_h,
                    2.0, Color::from_rgba(150, 150, 170, 255));
            }
            
            // 标题
            draw_text_cn(&item.name, preview_x + preview_w / 2.0 - 30.0, preview_y + 30.0, 16.0,
                Color::from_rgba(255, 215, 0, 255));
            
            // 描述
            draw_text_cn(&item.description, preview_x + preview_w / 2.0 - 40.0, preview_y + 55.0, 12.0,
                Color::from_rgba(200, 200, 200, 255));
            
            // 预览区域 (原版位置: 105, 160 居中)
            let preview_area_x = preview_x + 80.0;
            let preview_area_y = preview_y + 100.0;
            draw_rectangle(preview_area_x, preview_area_y, 100.0, 80.0,
                Color::from_rgba(20, 20, 30, 180));
            
            // 图标（大）
            let icon_x = preview_area_x + 18.0;
            let icon_y = preview_area_y + 8.0;
            if let Some(info) = LibraryName::Items.get_texture(item.icon_index) {
                if let Some(ref tex) = info.image {
                    draw_texture_ex(tex, icon_x, icon_y, WHITE, DrawTextureParams {
                        dest_size: Some(vec2(64.0, 64.0)),
                        ..Default::default()
                    });
                }
            } else {
                draw_rectangle(icon_x, icon_y, 64.0, 64.0, Color::from_rgba(60, 60, 70, 255));
            }
            
            // 价格
            if item.price_gold > 0 {
                draw_text_cn(&format!("金币: {}", item.price_gold), preview_x + 20.0, preview_y + 210.0,
                    12.0, Color::from_rgba(255, 215, 0, 255));
            }
            if item.price_ingot > 0 {
                draw_text_cn(&format!("元宝: {}", item.price_ingot), preview_x + 20.0, preview_y + 230.0,
                    12.0, Color::from_rgba(0, 255, 255, 255));
            }
            
            let mouse_pos = mouse_position();
            
            // 方向控制按钮 (原版位置: LeftDirection 81,282  RightDirection 160,282)
            let left_x = preview_x + 81.0;
            let right_x = preview_x + 160.0;
            let dir_y = preview_y + 252.0;
            let dir_w = 24.0;
            let dir_h = 20.0;
            
            // 左转按钮 Prguse2[240-242]
            let left_hovered = mouse_pos.0 >= left_x && mouse_pos.0 <= left_x + dir_w
                && mouse_pos.1 >= dir_y && mouse_pos.1 <= dir_y + dir_h;
            let left_pressed = left_hovered && is_mouse_button_down(MouseButton::Left);
            
            let left_tex = if left_pressed {
                &self.left_btn_textures.2
            } else if left_hovered {
                &self.left_btn_textures.1
            } else {
                &self.left_btn_textures.0
            };
            
            if let Some(tex) = left_tex {
                draw_texture_ex(tex, left_x, dir_y, WHITE, DrawTextureParams::default());
            } else {
                draw_rectangle(left_x, dir_y, dir_w, dir_h, Color::from_rgba(80, 80, 120, 255));
                draw_text_cn("◀", left_x + 6.0, dir_y + 14.0, 12.0, WHITE);
            }
            
            if left_hovered && is_mouse_button_pressed(MouseButton::Left) {
                self.preview_direction = if self.preview_direction == 1 { 8 } else { self.preview_direction - 1 };
                println!("🔄 预览方向: {}", self.preview_direction);
            }
            
            // 右转按钮 Prguse2[243-245]
            let right_hovered = mouse_pos.0 >= right_x && mouse_pos.0 <= right_x + dir_w
                && mouse_pos.1 >= dir_y && mouse_pos.1 <= dir_y + dir_h;
            let right_pressed = right_hovered && is_mouse_button_down(MouseButton::Left);
            
            let right_tex = if right_pressed {
                &self.right_btn_textures.2
            } else if right_hovered {
                &self.right_btn_textures.1
            } else {
                &self.right_btn_textures.0
            };
            
            if let Some(tex) = right_tex {
                draw_texture_ex(tex, right_x, dir_y, WHITE, DrawTextureParams::default());
            } else {
                draw_rectangle(right_x, dir_y, dir_w, dir_h, Color::from_rgba(80, 80, 120, 255));
                draw_text_cn("▶", right_x + 6.0, dir_y + 14.0, 12.0, WHITE);
            }
            
            if right_hovered && is_mouse_button_pressed(MouseButton::Left) {
                self.preview_direction = if self.preview_direction == 8 { 1 } else { self.preview_direction + 1 };
                println!("🔄 预览方向: {}", self.preview_direction);
            }
            
            // 方向显示
            draw_text_cn(&format!("方向: {}/8", self.preview_direction), preview_x + 105.0, dir_y + 14.0, 10.0,
                Color::from_rgba(150, 150, 150, 255));
            
            // 关闭按钮 (原版位置: 230, 8)
            let close_x = preview_x + 230.0;
            let close_y = preview_y + 8.0;
            let close_hovered = mouse_pos.0 >= close_x && mouse_pos.0 <= close_x + 20.0
                && mouse_pos.1 >= close_y && mouse_pos.1 <= close_y + 20.0;
            let close_pressed = close_hovered && is_mouse_button_down(MouseButton::Left);
            
            let close_tex = if close_pressed {
                &self.close_btn_textures.2
            } else if close_hovered {
                &self.close_btn_textures.1
            } else {
                &self.close_btn_textures.0
            };
            
            if let Some(tex) = close_tex {
                draw_texture_ex(tex, close_x, close_y, WHITE, DrawTextureParams::default());
            } else {
                let close_color = if close_hovered {
                    Color::from_rgba(200, 80, 80, 255)
                } else {
                    Color::from_rgba(150, 50, 50, 255)
                };
                draw_rectangle(close_x, close_y, 20.0, 20.0, close_color);
                draw_text_cn("×", close_x + 5.0, close_y + 15.0, 14.0, WHITE);
            }
            
            if close_hovered && is_mouse_button_pressed(MouseButton::Left) {
                self.preview_item = None;
            }
            
            // ESC 关闭
            if is_key_pressed(KeyCode::Escape) {
                self.preview_item = None;
            }
        }
    }
    
    /// 处理拖拽
    fn handle_dragging(&mut self, pos: Vec2) {
        if let Some(skin) = &self.transparent_skin {
            ui::root_ui().push_skin(skin);
        }
        
        // 标题栏拖拽区域
        let drag_id = hash!("shop_drag");
        let title_rect = Rect::new(pos.x, pos.y, Self::DIALOG_WIDTH - 30.0, Self::TITLE_HEIGHT);
        
        let drag_result = ui::widgets::Group::new(drag_id, vec2(title_rect.w, title_rect.h))
            .position(vec2(title_rect.x, title_rect.y))
            .draggable(true)
            .ui(&mut ui::root_ui(), |_| {});
        
        // Drag 是枚举：Dragging(Vec2, Vec2), Hovered, Clicked, None
        match drag_result {
            ui::Drag::Dragging(_, _) => {
                let mouse_pos = mouse_position();
                if !self.dragging {
                    self.dragging = true;
                    self.drag_offset = vec2(mouse_pos.0, mouse_pos.1) - pos;
                }
                self.position = vec2(mouse_pos.0, mouse_pos.1) - self.drag_offset;
            }
            _ => {
                self.dragging = false;
            }
        }
        
        if let Some(_) = &self.transparent_skin {
            ui::root_ui().pop_skin();
        }
    }
    
    /// 处理关闭按钮 (使用纹理)
    fn handle_close_button(&mut self, pos: Vec2) {
        let close_x = pos.x + Self::DIALOG_WIDTH - 25.0;
        let close_y = pos.y + 8.0;
        let close_size = 20.0;
        
        let mouse_pos = mouse_position();
        let hovered = mouse_pos.0 >= close_x && mouse_pos.0 <= close_x + close_size
            && mouse_pos.1 >= close_y && mouse_pos.1 <= close_y + close_size;
        let pressed = hovered && is_mouse_button_down(MouseButton::Left);
        
        // 绘制关闭按钮 Prguse2[360-362]
        let close_tex = if pressed {
            &self.close_btn_textures.2
        } else if hovered {
            &self.close_btn_textures.1
        } else {
            &self.close_btn_textures.0
        };
        
        if let Some(tex) = close_tex {
            draw_texture_ex(tex, close_x, close_y, WHITE, DrawTextureParams::default());
        } else {
            let color = if hovered {
                Color::from_rgba(200, 80, 80, 255)
            } else {
                Color::from_rgba(150, 50, 50, 255)
            };
            draw_rectangle(close_x, close_y, close_size, close_size, color);
            draw_text_cn("×", close_x + 5.0, close_y + 15.0, 14.0, WHITE);
        }
        
        // 处理点击
        if hovered && is_mouse_button_pressed(MouseButton::Left) {
            self.close();
            println!("❌ 关闭商城");
        }
    }
    
    /// 绘制物品悬停提示框
    fn draw_item_tooltip(&self) {
        if let Some(idx) = self.hover_item {
            if idx >= self.filtered_items.len() {
                return;
            }
            
            let item = &self.filtered_items[idx];
            let mouse = mouse_position();
            
            // 提示框内容
            let lines = vec![
                item.name.clone(),
                item.description.clone(),
                String::new(),  // 空行
                format!("分类: {:?}", item.category),
                if item.price_gold > 0 {
                    format!("金币: {}", item.price_gold)
                } else {
                    String::new()
                },
                if item.price_ingot > 0 {
                    format!("元宝: {}", item.price_ingot)
                } else {
                    String::new()
                },
                if item.count > 1 {
                    format!("数量: {}", item.count)
                } else {
                    String::new()
                },
                if item.stock > 0 {
                    format!("库存: {}", item.stock)
                } else if item.stock == 0 {
                    "库存: ∞".to_string()
                } else {
                    String::new()
                },
            ];
            
            // 过滤空行并计算尺寸
            let valid_lines: Vec<&String> = lines.iter().filter(|s| !s.is_empty()).collect();
            let line_height = 16.0;
            let padding = 8.0;
            let max_width = valid_lines.iter()
                .map(|s| s.chars().count() as f32 * 8.0)
                .fold(150.0f32, |a, b| a.max(b));
            let tooltip_w = max_width + padding * 2.0;
            let tooltip_h = valid_lines.len() as f32 * line_height + padding * 2.0;
            
            // 计算位置（在鼠标右下方，避免超出屏幕）
            let screen_w = screen_width();
            let screen_h = screen_height();
            let offset_x = 15.0;
            let offset_y = 10.0;
            
            let mut tooltip_x = mouse.0 + offset_x;
            let mut tooltip_y = mouse.1 + offset_y;
            
            // 边界检查
            if tooltip_x + tooltip_w > screen_w {
                tooltip_x = mouse.0 - tooltip_w - 5.0;
            }
            if tooltip_y + tooltip_h > screen_h {
                tooltip_y = mouse.1 - tooltip_h - 5.0;
            }
            
            // 绘制背景
            draw_rectangle(
                tooltip_x,
                tooltip_y,
                tooltip_w,
                tooltip_h,
                Color::from_rgba(20, 20, 30, 240)
            );
            
            // 绘制边框
            draw_rectangle_lines(
                tooltip_x,
                tooltip_y,
                tooltip_w,
                tooltip_h,
                2.0,
                Color::from_rgba(100, 100, 130, 255)
            );
            
            // 绘制物品名称（金色，第一行）
            let mut y_offset = tooltip_y + padding + 12.0;
            if !item.name.is_empty() {
                let name_color = if item.in_stock {
                    Color::from_rgba(255, 215, 0, 255)  // 金色
                } else {
                    GRAY
                };
                draw_text_cn(&item.name, tooltip_x + padding, y_offset, 12.0, name_color);
                y_offset += line_height;
            }
            
            // 绘制描述（白色）
            if !item.description.is_empty() {
                draw_text_cn(&item.description, tooltip_x + padding, y_offset, 11.0, WHITE);
                y_offset += line_height;
            }
            
            // 绘制其他信息（灰色）
            for line in &valid_lines[2..] {
                draw_text_cn(line, tooltip_x + padding, y_offset, 10.0, 
                    Color::from_rgba(180, 180, 180, 255));
                y_offset += line_height;
            }
            
            // 如果缺货，显示红色提示
            if !item.in_stock {
                draw_text_cn("[已售罄]", tooltip_x + padding, y_offset, 10.0, RED);
            }
        }
    }
}

impl Default for GameShopDialogHybrid {
    fn default() -> Self {
        Self::new()
    }
}

/// 将KeyCode转换为字符 (简化版，只支持基本ASCII)
fn key_to_char(key: KeyCode) -> Option<char> {
    match key {
        KeyCode::A => Some('a'),
        KeyCode::B => Some('b'),
        KeyCode::C => Some('c'),
        KeyCode::D => Some('d'),
        KeyCode::E => Some('e'),
        KeyCode::F => Some('f'),
        KeyCode::G => Some('g'),
        KeyCode::H => Some('h'),
        KeyCode::I => Some('i'),
        KeyCode::J => Some('j'),
        KeyCode::K => Some('k'),
        KeyCode::L => Some('l'),
        KeyCode::M => Some('m'),
        KeyCode::N => Some('n'),
        KeyCode::O => Some('o'),
        KeyCode::P => Some('p'),
        KeyCode::Q => Some('q'),
        KeyCode::R => Some('r'),
        KeyCode::S => Some('s'),
        KeyCode::T => Some('t'),
        KeyCode::U => Some('u'),
        KeyCode::V => Some('v'),
        KeyCode::W => Some('w'),
        KeyCode::X => Some('x'),
        KeyCode::Y => Some('y'),
        KeyCode::Z => Some('z'),
        KeyCode::Key0 => Some('0'),
        KeyCode::Key1 => Some('1'),
        KeyCode::Key2 => Some('2'),
        KeyCode::Key3 => Some('3'),
        KeyCode::Key4 => Some('4'),
        KeyCode::Key5 => Some('5'),
        KeyCode::Key6 => Some('6'),
        KeyCode::Key7 => Some('7'),
        KeyCode::Key8 => Some('8'),
        KeyCode::Key9 => Some('9'),
        KeyCode::Space => Some(' '),
        KeyCode::Minus => Some('-'),
        _ => None,
    }
}
