//! 商城对话框类型定义

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
