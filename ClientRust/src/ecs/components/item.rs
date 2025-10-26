// ============================================================================
// 物品/装备/背包组件
// ============================================================================

use mir2_shared::enums::ItemType;

/// 地面物品组件
#[derive(Debug, Clone)]
pub struct ItemDrop {
    pub item_id: u32,
    pub item_index: u16,
    pub count: u32,
    pub owner_id: Option<u32>, // 归属玩家 (拾取保护)
}

/// 地面物品组件
#[derive(Debug, Clone)]
pub struct GroundItem {
    pub object_id: u32,
    pub item: mir2_shared::data::item::UserItem,
    pub gold_amount: u32,  // 如果是金币，这里是数量
}

/// 背包组件 - 存储玩家的物品
#[derive(Debug, Clone)]
pub struct Inventory {
    /// 背包物品列表（索引对应格子位置）
    /// None 表示空格子
    pub items: Vec<Option<mir2_shared::data::item::UserItem>>,
    
    /// 背包容量（默认40格）
    pub capacity: usize,
    
    /// 金币数量
    pub gold: u32,
    
    /// 当前负重
    pub current_weight: u16,
    
    /// 最大负重
    pub max_weight: u16,
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new(40) // 默认40格背包
    }
}

impl Inventory {
    pub fn new(capacity: usize) -> Self {
        Self {
            items: vec![None; capacity],
            capacity,
            gold: 0,
            current_weight: 0,
            max_weight: 100, // 默认最大负重100
        }
    }
    
    /// 添加物品到背包
    pub fn add_item(&mut self, item: mir2_shared::data::item::UserItem) -> bool {
        // 查找空格子
        for slot in &mut self.items {
            if slot.is_none() {
                *slot = Some(item);
                return true;
            }
        }
        false // 背包已满
    }
    
    /// 移除指定格子的物品
    pub fn remove_item(&mut self, slot_index: usize) -> Option<mir2_shared::data::item::UserItem> {
        if slot_index < self.items.len() {
            self.items[slot_index].take()
        } else {
            None
        }
    }
    
    /// 获取指定格子的物品引用
    pub fn get_item(&self, slot_index: usize) -> Option<&mir2_shared::data::item::UserItem> {
        if slot_index < self.items.len() {
            self.items[slot_index].as_ref()
        } else {
            None
        }
    }
    
    /// 设置金币数量
    pub fn set_gold(&mut self, gold: u32) {
        self.gold = gold;
    }
    
    /// 添加金币
    pub fn add_gold(&mut self, amount: u32) {
        self.gold = self.gold.saturating_add(amount);
    }
    
    /// 减少金币
    pub fn remove_gold(&mut self, amount: u32) -> bool {
        if self.gold >= amount {
            self.gold -= amount;
            true
        } else {
            false
        }
    }
}

/// 装备栏组件
#[derive(Debug, Clone)]
pub struct Equipment {
    pub weapon: Option<mir2_shared::data::item::UserItem>,       // 武器
    pub armour: Option<mir2_shared::data::item::UserItem>,       // 衣服
    pub helmet: Option<mir2_shared::data::item::UserItem>,       // 头盔
    pub necklace: Option<mir2_shared::data::item::UserItem>,     // 项链
    pub bracelet_l: Option<mir2_shared::data::item::UserItem>,   // 左手镯
    pub bracelet_r: Option<mir2_shared::data::item::UserItem>,   // 右手镯
    pub ring_l: Option<mir2_shared::data::item::UserItem>,       // 左戒指
    pub ring_r: Option<mir2_shared::data::item::UserItem>,       // 右戒指
    pub amulet: Option<mir2_shared::data::item::UserItem>,       // 护身符
    pub belt: Option<mir2_shared::data::item::UserItem>,         // 腰带
    pub boots: Option<mir2_shared::data::item::UserItem>,        // 鞋子
    pub stone: Option<mir2_shared::data::item::UserItem>,        // 宝石
    pub torch: Option<mir2_shared::data::item::UserItem>,        // 火把
    pub mount: Option<mir2_shared::data::item::UserItem>,        // 坐骑
}

impl Default for Equipment {
    fn default() -> Self {
        Self {
            weapon: None,
            armour: None,
            helmet: None,
            necklace: None,
            bracelet_l: None,
            bracelet_r: None,
            ring_l: None,
            ring_r: None,
            amulet: None,
            belt: None,
            boots: None,
            stone: None,
            torch: None,
            mount: None,
        }
    }
}

impl Equipment {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 根据装备类型获取对应槽位
    pub fn get_slot_for_type(&self, item_type: ItemType) -> Option<u8> {
        match item_type {
            ItemType::Weapon => Some(0),
            ItemType::Armour => Some(1),
            ItemType::Helmet => Some(2),
            ItemType::Necklace => Some(3),
            ItemType::Bracelet => Some(4), // 默认左手镯
            ItemType::Ring => Some(6),     // 默认左戒指
            ItemType::Amulet => Some(8),
            ItemType::Belt => Some(9),
            ItemType::Boots => Some(10),
            ItemType::Stone => Some(11),
            ItemType::Torch => Some(12),
            ItemType::Mount => Some(13),
            _ => None,
        }
    }
    
    /// 装备物品到指定槽位
    pub fn equip(&mut self, slot: u8, item: mir2_shared::data::item::UserItem) -> Option<mir2_shared::data::item::UserItem> {
        let slot_ref = match slot {
            0 => &mut self.weapon,
            1 => &mut self.armour,
            2 => &mut self.helmet,
            3 => &mut self.necklace,
            4 => &mut self.bracelet_l,
            5 => &mut self.bracelet_r,
            6 => &mut self.ring_l,
            7 => &mut self.ring_r,
            8 => &mut self.amulet,
            9 => &mut self.belt,
            10 => &mut self.boots,
            11 => &mut self.stone,
            12 => &mut self.torch,
            13 => &mut self.mount,
            _ => return None,
        };
        
        // 返回旧装备
        slot_ref.replace(item)
    }
    
    /// 卸下指定槽位的装备
    pub fn unequip(&mut self, slot: u8) -> Option<mir2_shared::data::item::UserItem> {
        let slot_ref = match slot {
            0 => &mut self.weapon,
            1 => &mut self.armour,
            2 => &mut self.helmet,
            3 => &mut self.necklace,
            4 => &mut self.bracelet_l,
            5 => &mut self.bracelet_r,
            6 => &mut self.ring_l,
            7 => &mut self.ring_r,
            8 => &mut self.amulet,
            9 => &mut self.belt,
            10 => &mut self.boots,
            11 => &mut self.stone,
            12 => &mut self.torch,
            13 => &mut self.mount,
            _ => return None,
        };
        
        slot_ref.take()
    }
}
