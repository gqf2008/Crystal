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

// 重新导入组件系统
use crate::ui::dialogs::{GameShopDialog as ComponentGameShopDialog, GameShopAction, ShopItem};

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

/// 兼容原有接口的GameShopDialog包装器
pub struct GameShopDialog {
    /// 新的组件系统对话框
    component_dialog: ComponentGameShopDialog,
    /// 当前选中的主要分类
    selected_section: GameShopSection,
    /// 当前选中的职业分类
    selected_class: GameShopClass,
    /// 玩家金币
    player_gold: u32,
    /// 玩家元宝
    player_ingot: u32,
}

impl GameShopDialog {
    pub fn new() -> Self {
        let mut component_dialog = ComponentGameShopDialog::new();
        
        // 创建一些示例商城物品
        let shop_items = vec![
            ShopItem {
                id: 1,
                name: "龙纹剑".to_string(),
                description: "强力的单手剑，攻击力+50".to_string(),
                price: 100000,
                icon_index: 1,
                category: "武器".to_string(),
                in_stock: true,
            },
            ShopItem {
                id: 2,
                name: "天师道袍".to_string(),
                description: "高级法师袍，魔法防御+30".to_string(),
                price: 80000,
                icon_index: 20,
                category: "防具".to_string(),
                in_stock: true,
            },
            ShopItem {
                id: 3,
                name: "强效金疮药".to_string(),
                description: "瞬间恢复500点生命值".to_string(),
                price: 5000,
                icon_index: 40,
                category: "药品".to_string(),
                in_stock: true,
            },
            ShopItem {
                id: 4,
                name: "传送戒指".to_string(),
                description: "可以传送到指定地点".to_string(),
                price: 200000,
                icon_index: 60,
                category: "特殊".to_string(),
                in_stock: false,
            },
            ShopItem {
                id: 5,
                name: "华丽长袍".to_string(),
                description: "美观的时装，无属性加成".to_string(),
                price: 50000,
                icon_index: 80,
                category: "时装".to_string(),
                in_stock: true,
            },
        ];
        
        component_dialog.load_items(shop_items);
        
        Self {
            component_dialog,
            selected_section: GameShopSection::All,
            selected_class: GameShopClass::All,
            player_gold: 100000,
            player_ingot: 500,
        }
    }
    
    /// 显示商店对话框
    pub fn show(&mut self) {
        self.component_dialog.show();
    }
    
    /// 隐藏商店对话框
    pub fn hide(&mut self) {
        self.component_dialog.hide();
    }
    
    /// 是否可见
    pub fn is_visible(&self) -> bool {
        self.component_dialog.is_visible()
    }
    
    /// 设置玩家金币
    pub fn set_player_gold(&mut self, gold: u32) {
        self.player_gold = gold;
    }
    
    /// 设置玩家元宝
    pub fn set_player_ingot(&mut self, ingot: u32) {
        self.player_ingot = ingot;
    }
    
    /// 获取玩家金币
    pub fn get_player_gold(&self) -> u32 {
        self.player_gold
    }
    
    /// 获取玩家元宝
    pub fn get_player_ingot(&self) -> u32 {
        self.player_ingot
    }
    
    /// 处理购买事件
    pub fn handle_buy_item(&mut self, item_id: u32) -> bool {
        // 这里应该集成到游戏的购买系统
        println!("购买商品 ID: {}", item_id);
        true
    }
}

impl Dialog for GameShopDialog {
    fn id(&self) -> egui::Id {
        egui::Id::new("game_shop_dialog")
    }

    fn is_visible(&self) -> bool {
        self.component_dialog.is_visible()
    }

    fn show(&mut self) {
        self.component_dialog.show();
    }

    fn hide(&mut self) {
        self.component_dialog.hide();
    }

    fn draw(&mut self, ctx: &egui::Context) {
        if let Some(action) = self.component_dialog.draw(ctx) {
            match action {
                GameShopAction::Close => {
                    self.hide();
                }
                GameShopAction::ItemSelected(index) => {
                    println!("选中商品索引: {}", index);
                }
                GameShopAction::BuyItem(item_id) => {
                    self.handle_buy_item(item_id);
                }
                GameShopAction::PageChanged(page) => {
                    println!("切换到页面: {}", page);
                }
            }
        }
    }
}

impl Default for GameShopDialog {
    fn default() -> Self {
        Self::new()
    }
}