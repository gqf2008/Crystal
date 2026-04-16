// PlayerInventory - 背包 + 装备 + 金币
// 纯数据结构，不含网络 I/O

use mir2_shared::data::item::UserItem;

/// 装备槽位索引（12 个槽位）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipmentSlot {
    Weapon = 0,
    Armour = 1,
    Helmet = 2,
    Necklace = 3,
    BraceletL = 4,
    BraceletR = 5,
    RingL = 6,
    RingR = 7,
    Shoes = 8,
    Pendant = 9,
    Mount = 10,
    // 第 11 槽位预留
}

impl EquipmentSlot {
    pub const COUNT: usize = 12;

    pub fn from_i32(val: i32) -> Option<Self> {
        match val {
            0 => Some(Self::Weapon),
            1 => Some(Self::Armour),
            2 => Some(Self::Helmet),
            3 => Some(Self::Necklace),
            4 => Some(Self::BraceletL),
            5 => Some(Self::BraceletR),
            6 => Some(Self::RingL),
            7 => Some(Self::RingR),
            8 => Some(Self::Shoes),
            9 => Some(Self::Pendant),
            10 => Some(Self::Mount),
            _ => None,
        }
    }
}

/// 背包格子（最多 40 格）
pub const BACKPACK_SIZE: usize = 40;

/// 背包中的物品格子
#[derive(Debug, Clone)]
pub struct InventorySlot {
    pub grid: u8,
    pub item: UserItem,
}

/// 地面掉落物品（世界中，非玩家持有）
#[derive(Debug, Clone)]
pub struct GroundItem {
    pub item: UserItem,
    pub x: i32,
    pub y: i32,
    pub map_index: u16,
    /// 拾取者 session（绑定后其他人无法拾取）
    pub dropper_session: Option<u64>,
}

/// 仓库格子数
pub const STORAGE_SIZE: usize = 80;

/// 玩家背包 + 装备 + 仓库
#[derive(Debug, Clone)]
pub struct PlayerInventory {
    /// 金币
    pub gold: u64,
    /// 背包格子（40 格，索引 = grid 字段）
    pub backpack: [Option<InventorySlot>; BACKPACK_SIZE],
    /// 装备槽位（12 个）
    pub equipment: [Option<UserItem>; EquipmentSlot::COUNT],
    /// 仓库格子（80 格）
    pub storage: Vec<Option<InventorySlot>>,
}

impl Default for PlayerInventory {
    fn default() -> Self {
        Self {
            gold: 0,
            backpack: [const { None }; BACKPACK_SIZE],
            equipment: [const { None }; EquipmentSlot::COUNT],
            storage: vec![None; STORAGE_SIZE],
        }
    }
}

impl PlayerInventory {
    pub fn new() -> Self {
        Self::default()
    }

    // ============================================================
    // 背包操作
    // ============================================================

    /// 添加物品到背包（自动找空位或合并堆叠）
    /// 返回 (grid, unique_id) 或 None（背包已满）
    pub fn add_item(&mut self, mut item: UserItem) -> Option<(u8, u64)> {
        // 尝试合并到已有堆叠（相同 item_index 且可堆叠）
        if item.count > 1 {
            for s in self.backpack.iter_mut().flatten() {
                if s.item.item_index == item.item_index && s.item.count < s.item.max_dura.max(1) {
                    let can_merge = s.item.count + item.count;
                    let max_stack = s.item.max_dura.max(1);
                    if can_merge <= max_stack {
                        s.item.count = can_merge;
                        return Some((s.grid, s.item.unique_id));
                    } else {
                        s.item.count = max_stack;
                        item.count = can_merge - max_stack;
                        // 继续处理剩余
                    }
                }
            }
        }

        // 找空位
        for grid in 0..BACKPACK_SIZE {
            if self.backpack[grid].is_none() {
                item.unique_id = self.next_unique_id();
                let uid = item.unique_id;
                self.backpack[grid] = Some(InventorySlot {
                    grid: grid as u8,
                    item,
                });
                return Some((grid as u8, uid));
            }
        }

        None // 背包已满
    }

    /// 根据 unique_id 移除物品
    /// 返回移除的物品或 None
    pub fn remove_item_by_uid(&mut self, uid: u64) -> Option<UserItem> {
        for slot in &mut self.backpack {
            if let Some(s) = slot {
                if s.item.unique_id == uid {
                    return slot.take().map(|s| s.item);
                }
            }
        }
        // 也检查装备
        for eq in &mut self.equipment {
            if let Some(e) = eq {
                if e.unique_id == uid {
                    return eq.take();
                }
            }
        }
        None
    }

    /// 从指定格子移除物品
    pub fn remove_item_by_grid(&mut self, grid: u8) -> Option<UserItem> {
        let idx = grid as usize;
        if idx >= BACKPACK_SIZE {
            return None;
        }
        self.backpack[idx].take().map(|s| s.item)
    }

    /// 查询物品（按 unique_id）
    pub fn get_item(&self, uid: u64) -> Option<&UserItem> {
        for s in self.backpack.iter().flatten() {
            if s.item.unique_id == uid {
                return Some(&s.item);
            }
        }
        self.equipment.iter().flatten().find(|&e| e.unique_id == uid).map(|e| e as _)
    }

    /// 查询物品（按格子索引）
    pub fn get_item_by_grid(&self, grid: u8) -> Option<&UserItem> {
        let idx = grid as usize;
        if idx >= BACKPACK_SIZE {
            return None;
        }
        self.backpack[idx].as_ref().map(|s| &s.item)
    }

    /// 移动物品：从 from_grid 到 to_grid
    /// 返回是否成功
    pub fn move_item(&mut self, from_grid: u8, to_grid: u8) -> bool {
        let fi = from_grid as usize;
        let ti = to_grid as usize;
        if fi >= BACKPACK_SIZE || ti >= BACKPACK_SIZE || fi == ti {
            return false;
        }

        if self.backpack[ti].is_some() {
            return false; // 目标格已有物品
        }

        if let Some(slot) = self.backpack[fi].take() {
            let mut new_slot = slot;
            new_slot.grid = to_grid;
            self.backpack[ti] = Some(new_slot);
            return true;
        }

        false
    }

    /// 合并物品：将 from_grid 合并到 to_grid
    pub fn merge_item(&mut self, from_grid: u8, to_grid: u8) -> bool {
        let fi = from_grid as usize;
        let ti = to_grid as usize;
        if fi >= BACKPACK_SIZE || ti >= BACKPACK_SIZE || fi == ti {
            return false;
        }

        let from_item = match &self.backpack[fi] {
            Some(s) => s.item.clone(),
            None => return false,
        };
        let to_item = match &self.backpack[ti] {
            Some(s) => s.item.clone(),
            None => return false,
        };

        // 必须是同种物品
        if from_item.item_index != to_item.item_index {
            return false;
        }

        let max_stack = to_item.max_dura.max(1);
        let new_count = from_item.count + to_item.count;
        if new_count > max_stack {
            return false; // 超出堆叠上限
        }

        // 合并到目标格
        if let Some(s) = &mut self.backpack[ti] {
            s.item.count = new_count;
        }
        self.backpack[fi] = None;
        true
    }

    /// 拆分物品：从 grid 拆出 count 数量到空位
    pub fn split_item(&mut self, grid: u8, count: u16) -> bool {
        let idx = grid as usize;
        if idx >= BACKPACK_SIZE {
            return false;
        }

        // 先检查原格物品是否可拆分
        let (_item_count, item_data) = match &self.backpack[idx] {
            Some(s) if s.item.count > 1 && count < s.item.count => {
                (s.item.count, Some(s.item.clone()))
            }
            _ => return false,
        };
        let item_data = item_data.unwrap();

        // 找空位
        let mut new_grid = None;
        for g in 0..BACKPACK_SIZE {
            if self.backpack[g].is_none() && g != idx {
                new_grid = Some(g);
                break;
            }
        }
        let Some(new_grid) = new_grid else { return false; };

        // 从原格扣减
        if let Some(s) = &mut self.backpack[idx] {
            s.item.count -= count;
        }

        // 创建新物品
        let mut new_item = item_data;
        new_item.count = count;
        new_item.unique_id = self.next_unique_id();

        self.backpack[new_grid] = Some(InventorySlot {
            grid: new_grid as u8,
            item: new_item,
        });
        true
    }

    /// 检查背包是否有空位
    pub fn has_space(&self) -> bool {
        self.backpack.iter().any(|s| s.is_none())
    }

    /// 检查能否获得物品（对应 C# CanGainItems）
    pub fn can_gain_items(&self) -> bool {
        self.has_space()
    }

    /// 计算背包中已有物品的数量
    pub fn item_count(&self) -> usize {
        self.backpack.iter().filter(|s| s.is_some()).count()
    }

    // ============================================================
    // 装备操作
    // ============================================================

    /// 装备物品：从背包取出，放入装备槽位
    /// 返回 (旧装备 Option<UserItem>, 新装备 unique_id) 或 None
    pub fn equip_item(&mut self, grid: u8, slot: EquipmentSlot) -> Option<(Option<UserItem>, u64)> {
        let idx = grid as usize;
        if idx >= BACKPACK_SIZE {
            return None;
        }

        let item = self.backpack[idx].take()?.item;
        let old_equipment = self.equipment[slot as usize].replace(item.clone());
        Some((old_equipment, item.unique_id))
    }

    /// 卸下装备：从装备槽位放回背包
    /// 返回 (卸下物品, 放入的 grid) 或 None
    pub fn unequip_item(&mut self, slot: EquipmentSlot) -> Option<(UserItem, u8)> {
        let item = self.equipment[slot as usize].take()?;

        // 找空位
        for grid in 0..BACKPACK_SIZE {
            if self.backpack[grid].is_none() {
                self.backpack[grid] = Some(InventorySlot {
                    grid: grid as u8,
                    item: item.clone(),
                });
                return Some((item, grid as u8));
            }
        }

        // 背包已满，放回去
        self.equipment[slot as usize] = Some(item.clone());
        None
    }

    /// 修理物品：恢复耐久到最大值
    /// 返回是否成功
    pub fn repair_item(&mut self, uid: u64) -> bool {
        // 检查背包
        for s in self.backpack.iter_mut().flatten() {
            if s.item.unique_id == uid {
                s.item.current_dura = s.item.max_dura;
                s.item.dura_changed = true;
                return true;
            }
        }
        // 检查装备
        for e in self.equipment.iter_mut().flatten() {
            if e.unique_id == uid {
                e.current_dura = e.max_dura;
                e.dura_changed = true;
                return true;
            }
        }
        false
    }

    /// 获取装备
    pub fn get_equipment(&self, slot: EquipmentSlot) -> Option<&UserItem> {
        self.equipment[slot as usize].as_ref()
    }

    // ============================================================
    // 仓库操作
    // ============================================================

    /// 存入仓库：从背包取出，放入仓库
    /// 返回 (物品, 仓库格子) 或 None
    pub fn store_item(&mut self, grid: u8) -> Option<(UserItem, usize)> {
        let idx = grid as usize;
        if idx >= BACKPACK_SIZE {
            return None;
        }
        let item = self.backpack[idx].take()?.item;

        // 找仓库空位
        for storage_grid in 0..self.storage.len() {
            if self.storage[storage_grid].is_none() {
                self.storage[storage_grid] = Some(InventorySlot {
                    grid: storage_grid as u8,
                    item: item.clone(),
                });
                return Some((item, storage_grid));
            }
        }

        // 仓库已满，放回背包
        self.backpack[idx] = Some(InventorySlot { grid, item });
        None
    }

    /// 从仓库取出：从仓库取出，放入背包
    /// 返回 (物品, 背包格子) 或 None
    pub fn take_back_item(&mut self, storage_grid: u8) -> Option<(UserItem, u8)> {
        let sidx = storage_grid as usize;
        if sidx >= self.storage.len() {
            return None;
        }
        let item = self.storage[sidx].take()?.item;

        // 找背包空位
        for grid in 0..BACKPACK_SIZE {
            if self.backpack[grid].is_none() {
                self.backpack[grid] = Some(InventorySlot {
                    grid: grid as u8,
                    item: item.clone(),
                });
                return Some((item, grid as u8));
            }
        }

        // 背包已满，放回仓库
        self.storage[sidx] = Some(InventorySlot {
            grid: storage_grid,
            item,
        });
        None
    }

    /// 检查仓库是否有空位
    pub fn storage_has_space(&self) -> bool {
        self.storage.iter().any(|s| s.is_none())
    }
}

// ============================================================
// unique_id 生成
// ============================================================

/// 全局 unique_id 计数器
static NEXT_UID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// 生成新的物品唯一 ID（供背包和邮件系统使用）
pub fn generate_item_uid() -> u64 {
    NEXT_UID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

impl PlayerInventory {
    fn next_unique_id(&self) -> u64 {
        generate_item_uid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(index: i32, count: u16) -> UserItem {
        UserItem {
            unique_id: 0,
            item_index: index,
            info: None,
            current_dura: 100,
            max_dura: 100,
            count,
            gem_count: 0,
            refined_value: mir2_shared::enums::RefinedValue::None,
            refine_added: 0,
            refine_success_chance: 0,
            dura_changed: false,
            soul_bound_id: 0,
            identified: false,
            cursed: false,
            wedding_ring: 0,
            buyback_expiry_date_binary: 0,
            slots: Vec::new(),
            expire_info: None,
            rental_information: None,
            sealed_info: None,
            is_shop_item: false,
            awake: Default::default(),
            added_stats: Default::default(),
            is_gm_made: false,
        }
    }

    #[test]
    fn test_add_item() {
        let mut inv = PlayerInventory::new();
        let (grid, uid) = inv.add_item(make_item(1, 1)).unwrap();
        assert_eq!(grid, 0);
        assert!(uid > 0);
        assert_eq!(inv.item_count(), 1);
    }

    #[test]
    fn test_move_item() {
        let mut inv = PlayerInventory::new();
        inv.add_item(make_item(1, 1));
        assert!(inv.move_item(0, 5));
        assert!(inv.backpack[0].is_none());
        assert!(inv.backpack[5].is_some());
        assert!(!inv.move_item(0, 5)); // 源格已空
    }

    #[test]
    fn test_equip_unequip() {
        let mut inv = PlayerInventory::new();
        inv.add_item(make_item(1, 1));
        let (old, uid) = inv.equip_item(0, EquipmentSlot::Weapon).unwrap();
        assert!(old.is_none());
        assert!(uid > 0);
        assert!(inv.backpack[0].is_none());
        assert!(inv.get_equipment(EquipmentSlot::Weapon).is_some());

        let (item, grid) = inv.unequip_item(EquipmentSlot::Weapon).unwrap();
        assert_eq!(grid, 0);
        assert_eq!(item.unique_id, uid);
        assert!(inv.backpack[0].is_some());
    }

    #[test]
    fn test_split_item() {
        let mut inv = PlayerInventory::new();
        inv.add_item(make_item(1, 10));
        assert!(inv.split_item(0, 3));
        assert_eq!(inv.backpack[0].as_ref().unwrap().item.count, 7);
        assert_eq!(inv.backpack[1].as_ref().unwrap().item.count, 3);
    }

    #[test]
    fn test_gold() {
        let mut inv = PlayerInventory::new();
        assert_eq!(inv.gold, 0);
        inv.gold = 1000;
        inv.gold = inv.gold.saturating_sub(500);
        assert_eq!(inv.gold, 500);
    }

    #[test]
    fn test_store_item() {
        let mut inv = PlayerInventory::new();
        inv.add_item(make_item(1, 1));
        let (item, storage_grid) = inv.store_item(0).unwrap();
        assert_eq!(item.item_index, 1);
        assert!(storage_grid < STORAGE_SIZE);
        assert!(inv.backpack[0].is_none());
        assert!(inv.storage[storage_grid].is_some());
    }

    #[test]
    fn test_take_back_item() {
        let mut inv = PlayerInventory::new();
        inv.add_item(make_item(1, 1));
        inv.store_item(0);
        let (item, grid) = inv.take_back_item(0).unwrap();
        assert_eq!(item.item_index, 1);
        assert!(grid < BACKPACK_SIZE as u8);
        assert!(inv.storage[0].is_none());
        assert!(inv.backpack[grid as usize].is_some());
    }

    #[test]
    fn test_storage_full() {
        let mut inv = PlayerInventory::new();
        // Fill storage in batches (backpack is 40, storage is 80)
        for _batch in 0..2 {
            for _ in 0..BACKPACK_SIZE {
                inv.add_item(make_item(2, 1));
            }
            for g in 0..BACKPACK_SIZE as u8 {
                inv.store_item(g);
            }
        }
        assert!(!inv.storage_has_space());
        assert_eq!(inv.storage.iter().filter(|s| s.is_some()).count(), STORAGE_SIZE);
    }

    #[test]
    fn test_backpack_full_cannot_store() {
        let mut inv = PlayerInventory::new();
        // Try to store from empty backpack grid
        assert!(inv.store_item(0).is_none());
    }

    #[test]
    fn test_repair_item() {
        let mut inv = PlayerInventory::new();
        let (grid, uid) = inv.add_item(make_item(1, 1)).unwrap();

        // 模拟耐久消耗
        if let Some(s) = &mut inv.backpack[grid as usize] {
            s.item.current_dura = 50;
            assert_eq!(s.item.current_dura, 50);
        }

        // 修理
        assert!(inv.repair_item(uid));
        if let Some(s) = &inv.backpack[grid as usize] {
            assert_eq!(s.item.current_dura, s.item.max_dura);
            assert!(s.item.dura_changed);
        }

        // 修理不存在的物品
        assert!(!inv.repair_item(99999));
    }
}
