// PlayerInventory - 背包 + 装备 + 金币
// 纯数据结构，不含网络 I/O

use mir2_shared::data::item::UserItem;

/// 装备槽位索引（14 个槽位，#1136：补 C# Torch/Belt/Stone，保持既有编号）
/// 客户端 UI 已按 C# 14 槽布局（EQUIP_SLOTS），经 SERVER_SLOT_TO_POS 映射；
/// 本批次仅补齐缺失槽位，不做全量重编号（客户端与服务端编号一致即可）。
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
    // #1136：C# 三槽（DB ItemType：Torch=12 / Belt=9 / Stone=11）
    Torch = 11,
    Belt = 12,
    Stone = 13,
}

impl EquipmentSlot {
    pub const COUNT: usize = 14;

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
            11 => Some(Self::Torch),
            12 => Some(Self::Belt),
            13 => Some(Self::Stone),
            _ => None,
        }
    }
}

/// 背包格子（最多 40 格）
pub const BACKPACK_SIZE: usize = 40;
/// 任务物品格（C# QuestInventory 40 格；InventoryDialog QuestGrid 8x5）
pub const QUEST_INVENTORY_SIZE: usize = 40;

/// 背包中的物品格子
#[derive(Debug, Clone)]
pub struct InventorySlot {
    pub grid: u8,
    pub item: UserItem,
}

/// 地面掉落物品（世界中，非玩家持有）
#[derive(Debug, Clone)]
pub struct GroundItem {
    pub object_id: u32,
    pub item: UserItem,
    pub x: i32,
    pub y: i32,
    pub map_index: u16,
    /// 拾取者 session（绑定后其他人无法拾取）
    pub dropper_session: Option<u64>,
    /// 掉落时的 tick 计数（用于过期清理）
    pub drop_tick: u64,
    /// 是否玩家死亡掉落（C# PlayerDiedItemTimeOut=120s，比普通 ItemTimeOut=30s 更久）
    pub death_drop: bool,
}

/// 仓库格子数
pub const STORAGE_SIZE: usize = 80;

/// 玩家背包 + 装备 + 仓库
#[derive(Debug, Clone)]
pub struct PlayerInventory {
    /// 金币
    pub gold: u64,
    /// 背包格子（默认 40 格，可扩容到 86；索引 = grid 字段，C# Inventory 46→86）
    pub backpack: Vec<Option<InventorySlot>>,
    /// 装备槽位（12 个）
    pub equipment: [Option<UserItem>; EquipmentSlot::COUNT],
    /// 仓库格子（80 格）
    pub storage: Vec<Option<InventorySlot>>,
    /// 任务物品格（C# QuestInventory，40 格，独立于背包）
    pub quest_inventory: Vec<Option<UserItem>>,
}

impl Default for PlayerInventory {
    fn default() -> Self {
        Self {
            gold: 0,
            backpack: vec![None; BACKPACK_SIZE],
            equipment: [const { None }; EquipmentSlot::COUNT],
            storage: vec![None; STORAGE_SIZE],
            quest_inventory: vec![None; QUEST_INVENTORY_SIZE],
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
        for grid in 0..self.backpack.len() {
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

    /// 尝试将物品放入指定背包格子，如果该格为空则成功
    pub fn try_place_item_at(&mut self, item: UserItem, idx: usize) -> bool {
        if idx >= self.backpack.len() { return false; }
        if self.backpack[idx].is_some() { return false; }
        self.backpack[idx] = Some(InventorySlot {
            grid: idx as u8,
            item,
        });
        true
    }
}

/// 钓具穿戴（C# EquipSlotItem GridTo=Fishing：背包钓具放入鱼竿 slots[slot]）
/// 成功时旧钓具（若有）放回背包；失败返回原因
pub fn equip_fishing_gear(
    inventory: &mut PlayerInventory,
    rod_uid: u64,
    slot: usize,
    gear_uid: u64,
) -> Result<(), &'static str> {
    if !fishing_slot_type_ok(slot, None) {
        return Err("无效钓具槽");
    }
    // 鱼竿：Weapon 装备槽且为钓鱼竿（shape 49/50）
    let rod = match inventory.equipment.get_mut(EquipmentSlot::Weapon as usize) {
        Some(Some(r)) => r,
        _ => return Err("未装备鱼竿"),
    };
    if rod.unique_id != rod_uid {
        return Err("鱼竿不匹配");
    }
    let shape = rod.info.as_ref().map(|i| i.shape as i32).unwrap_or(0);
    if !crate::actors::world::is_fishing_rod_shape(shape) {
        return Err("不是钓鱼竿");
    }
    if rod.slots.len() < 5 {
        rod.slots.resize(5, None);
    }
    // 背包中找钓具
    let gear = inventory
        .backpack
        .iter()
        .find(|s| s.as_ref().map_or(false, |sl| sl.item.unique_id == gear_uid))
        .and_then(|s| s.as_ref().map(|sl| sl.item.clone()));
    let Some(gear) = gear else { return Err("背包中找不到钓具") };
    // 类型校验
    let gear_type = gear.info.as_ref().map(|i| i.item_type);
    if !fishing_slot_type_ok(slot, gear_type) {
        return Err("钓具类型与槽位不符");
    }
    // 从背包移除
    let mut taken = None;
    for s in inventory.backpack.iter_mut() {
        if s.as_ref().map_or(false, |sl| sl.item.unique_id == gear_uid) {
            taken = s.take();
            break;
        }
    }
    let Some(taken) = taken else { return Err("背包中找不到钓具") };
    // 旧钓具回背包（优先原槽，其次空格；无空格则撤销并还原钓具）
    if let Some(old) = rod.slots[slot].take() {
        let old_grid = taken.grid as usize;
        if old_grid < inventory.backpack.len() && inventory.backpack[old_grid].is_none() {
            inventory.backpack[old_grid] = Some(InventorySlot { grid: old_grid as u8, item: old });
        } else {
            let mut placed = false;
            for (i, s) in inventory.backpack.iter_mut().enumerate() {
                if s.is_none() {
                    *s = Some(InventorySlot { grid: i as u8, item: old });
                    placed = true;
                    break;
                }
            }
            if !placed {
                let tg = taken.grid as usize;
                inventory.backpack[tg] = Some(taken);
                return Err("背包已满");
            }
        }
    }
    rod.slots[slot] = Some(taken.item);
    Ok(())
}

/// 钓具耐久结果（#1313 子批3）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FishingGearDamageResult {
    /// 无鱼竿或无该槽钓具
    NoGear,
    /// 耐久归零损坏并移除
    Broken,
    /// 正常扣耐久
    Ok,
}

impl PlayerInventory {
    /// 当前鱼竿（Weapon 装备槽）
    pub fn fishing_rod(&self) -> Option<&UserItem> {
        self.equipment.get(EquipmentSlot::Weapon as usize).and_then(|e| e.as_ref())
    }

    /// 鱼竿 Bait 槽鱼饵数量（C# GetBait）
    pub fn fishing_bait_count(&self) -> u16 {
        self.fishing_rod()
            .and_then(|r| r.slots.get(2).and_then(|s| s.as_ref()))
            .map(|b| b.count)
            .unwrap_or(0)
    }

    /// 消耗鱼竿 Bait 槽鱼饵（C# ConsumeItem；数量归零移除）
    pub fn fishing_consume_bait(&mut self, amount: u16) -> bool {
        let Some(rod) = self.equipment.get_mut(EquipmentSlot::Weapon as usize).and_then(|e| e.as_mut()) else { return false };
        if rod.slots.len() < 5 { rod.slots.resize(5, None); }
        let Some(slot) = rod.slots.get_mut(2).and_then(|s| s.as_mut()) else { return false };
        if slot.count < amount { return false; }
        slot.count -= amount;
        if slot.count == 0 { rod.slots[2] = None; }
        true
    }

    /// 钓具耐久 -amount（C# DamagedFishingItem；归零损坏并移除）
    pub fn fishing_gear_damage(&mut self, slot: usize, amount: u16) -> FishingGearDamageResult {
        let Some(rod) = self.equipment.get_mut(EquipmentSlot::Weapon as usize).and_then(|e| e.as_mut()) else { return FishingGearDamageResult::NoGear };
        if rod.slots.len() < 5 { rod.slots.resize(5, None); }
        let dura = {
            let s = rod.slots.get_mut(slot).and_then(|s| s.as_mut());
            match s {
                Some(s) => {
                    if s.current_dura <= amount { 0 } else { s.current_dura -= amount; s.current_dura }
                }
                None => return FishingGearDamageResult::NoGear,
            }
        };
        if dura == 0 {
            rod.slots[slot] = None;
            FishingGearDamageResult::Broken
        } else {
            FishingGearDamageResult::Ok
        }
    }

    /// 鱼竿耐久 -amount（C# DamageItem(rod,1)）
    pub fn fishing_rod_durability_loss(&mut self, amount: u16) {
        if let Some(rod) = self.equipment.get_mut(EquipmentSlot::Weapon as usize).and_then(|e| e.as_mut()) {
            rod.current_dura = rod.current_dura.saturating_sub(amount);
        }
    }
}

/// 钓具槽类型校验（C# FishingSlot：Hook=0 Float=1 Bait=2 Finder=3 Reel=4；SharedRust ItemType：Hook=31..Reel=35）
/// gear_item_type=None（信息缺失）时不拦截；slot 非法返回 false
pub fn fishing_slot_type_ok(slot: usize, gear_item_type: Option<mir2_shared::enums::ItemType>) -> bool {
    use mir2_shared::enums::ItemType;
    let expected = match slot {
        0 => Some(ItemType::Hook),
        1 => Some(ItemType::Float),
        2 => Some(ItemType::Bait),
        3 => Some(ItemType::Finder),
        4 => Some(ItemType::Reel),
        _ => None,
    };
    match (expected, gear_item_type) {
        (Some(e), Some(t)) => e == t,
        (Some(_), None) => true,
        _ => false,
    }
}

impl PlayerInventory {
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
        if idx >= self.backpack.len() {
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

    /// 查询物品（可变引用，按 unique_id）
    pub fn get_item_mut(&mut self, uid: u64) -> Option<&mut UserItem> {
        for s in self.backpack.iter_mut().flatten() {
            if s.item.unique_id == uid {
                return Some(&mut s.item);
            }
        }
        self.equipment.iter_mut().flatten().find(|e| e.unique_id == uid)
    }

    /// 查询物品（按格子索引）
    pub fn get_item_by_grid(&self, grid: u8) -> Option<&UserItem> {
        let idx = grid as usize;
        if idx >= self.backpack.len() {
            return None;
        }
        self.backpack[idx].as_ref().map(|s| &s.item)
    }

    /// 移动物品：从 from_grid 到 to_grid
    /// 返回是否成功
    pub fn move_item(&mut self, from_grid: u8, to_grid: u8) -> bool {
        let fi = from_grid as usize;
        let ti = to_grid as usize;
        if fi >= self.backpack.len() || ti >= self.backpack.len() || fi == ti {
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
        if fi >= self.backpack.len() || ti >= self.backpack.len() || fi == ti {
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
        if idx >= self.backpack.len() {
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
        for g in 0..self.backpack.len() {
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


    /// 按 unique_id 拆分物品：从 uid 所在格拆出 count 数量到空位
    pub fn split_item_by_uid(&mut self, uid: u64, count: u16) -> bool {
        let mut src_idx = None;
        for (i, slot) in self.backpack.iter().enumerate() {
            if let Some(s) = slot {
                if s.item.unique_id == uid && s.item.count > 1 && count > 0 && count < s.item.count {
                    src_idx = Some(i);
                    break;
                }
            }
        }
        let Some(idx) = src_idx else { return false; };
        let item_data = match &self.backpack[idx] {
            Some(s) => s.item.clone(),
            None => return false,
        };
        let mut new_grid = None;
        for g in 0..self.backpack.len() {
            if self.backpack[g].is_none() && g != idx {
                new_grid = Some(g);
                break;
            }
        }
        let Some(new_grid) = new_grid else { return false; };
        if let Some(s) = &mut self.backpack[idx] {
            s.item.count -= count;
        }
        let mut new_item = item_data;
        new_item.count = count;
        new_item.unique_id = self.next_unique_id();
        self.backpack[new_grid] = Some(InventorySlot {
            grid: new_grid as u8,
            item: new_item,
        });
        true
    }

    /// 按 unique_id 移除 count 数量（count >= 原数量时移除整叠），返回被移除的物品
    pub fn remove_item_by_uid_partial(&mut self, uid: u64, count: u16) -> Option<UserItem> {
        for slot in &mut self.backpack {
            if let Some(s) = slot {
                if s.item.unique_id != uid {
                    continue;
                }
                if count >= s.item.count {
                    return slot.take().map(|s| s.item);
                }
                let mut removed = s.item.clone();
                removed.count = count;
                s.item.count -= count;
                return Some(removed);
            }
        }
        None
    }

    /// 按 unique_id 合并：from_uid 整叠合并到 to_uid 叠（同物品且不超堆叠上限）
    pub fn merge_item_by_uid(&mut self, from_uid: u64, to_uid: u64) -> bool {
        let mut from_idx = None;
        let mut to_idx = None;
        for (i, slot) in self.backpack.iter().enumerate() {
            if let Some(s) = slot {
                if s.item.unique_id == from_uid {
                    from_idx = Some(i);
                }
                if s.item.unique_id == to_uid {
                    to_idx = Some(i);
                }
            }
        }
        let (fi, ti) = match (from_idx, to_idx) {
            (Some(f), Some(t)) if f != t => (f, t),
            _ => return false,
        };
        let from_item = match &self.backpack[fi] {
            Some(s) => s.item.clone(),
            None => return false,
        };
        let to_item = match &self.backpack[ti] {
            Some(s) => s.item.clone(),
            None => return false,
        };
        if from_item.item_index != to_item.item_index {
            return false;
        }
        let max_stack = to_item.max_dura.max(1);
        let new_count = from_item.count as u32 + to_item.count as u32;
        if new_count > max_stack as u32 {
            return false;
        }
        if let Some(s) = &mut self.backpack[ti] {
            s.item.count = new_count as u16;
        }
        self.backpack[fi] = None;
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

    /// 按 item_index 统计背包中该物品的总数量（包含堆叠）
    pub fn count_item_by_index(&self, item_index: i32) -> u16 {
        self.backpack.iter().flatten()
            .filter(|s| s.item.item_index == item_index)
            .map(|s| s.item.count)
            .sum()
    }

    /// 按 item_index 从背包中移除指定数量的物品
    /// 返回是否成功移除了全部数量
    pub fn remove_item_by_index(&mut self, item_index: i32, mut count: u16) -> bool {
        if self.count_item_by_index(item_index) < count {
            return false;
        }
        for s in self.backpack.iter_mut().flatten() {
            if s.item.item_index == item_index {
                if s.item.count > count {
                    s.item.count -= count;
                    return true;
                } else {
                    count -= s.item.count;
                    s.item.count = 0;
                    // 标记为空（后续清理）
                }
                if count == 0 {
                    break;
                }
            }
        }
        // 清理空槽位
        for slot in self.backpack.iter_mut() {
            if let Some(ref s) = slot {
                if s.item.count == 0 {
                    *slot = None;
                }
            }
        }
        true
    }

    /// 按 item_index 从背包中移除物品，仅考虑 current_dura >= min_dura 的物品
    /// （C# TakeItem dura；min_dura 为 None 时不过滤）
    /// C# TakeItem 语义：移除尽可能多（不要求全量）；返回是否移除了至少一个
    pub fn remove_item_by_index_with_dura(&mut self, item_index: i32, mut count: u16, min_dura: Option<u32>) -> bool {
        let mut removed_any = false;
        for s in self.backpack.iter_mut().flatten() {
            if s.item.item_index != item_index { continue; }
            if min_dura.map(|d| (s.item.current_dura as u32) >= d).unwrap_or(true) == false { continue; }
            if s.item.count > count {
                s.item.count -= count;
                removed_any = true;
                count = 0;
                break;
            }
            count -= s.item.count;
            s.item.count = 0;
            removed_any = true;
            if count == 0 {
                break;
            }
        }
        // 清理空槽位
        for slot in self.backpack.iter_mut() {
            if let Some(ref s) = slot {
                if s.item.count == 0 {
                    *slot = None;
                }
            }
        }
        removed_any
    }

    /// 从背包中随机选择一个物品并移除返回（用于死亡掉落）
    pub fn random_drop_one(&mut self) -> Option<UserItem> {
        let occupied: Vec<usize> = self.backpack.iter().enumerate()
            .filter(|(_, s)| s.is_some())
            .map(|(i, _)| i)
            .collect();
        if occupied.is_empty() {
            return None;
        }
        let idx = occupied[fastrand::usize(0..occupied.len())];
        self.backpack[idx].take().map(|s| s.item)
    }

    // ============================================================
    // 装备操作
    // ============================================================

    /// 装备物品：从背包取出，放入装备槽位
    /// 返回 (旧装备 Option<UserItem>, 新装备 unique_id) 或 None
    pub fn equip_item(&mut self, grid: u8, slot: EquipmentSlot) -> Option<(Option<UserItem>, u64)> {
        let idx = grid as usize;
        if idx >= self.backpack.len() {
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
        for grid in 0..self.backpack.len() {
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

    /// 死亡掉落：直接从装备槽位取走物品（不放回背包）
    pub fn take_equipment(&mut self, slot: EquipmentSlot) -> Option<UserItem> {
        self.equipment[slot as usize].take()
    }

    /// 修理物品：恢复耐久到最大值（C# RepairItem：非特殊修理 MaxDura 衰减 1/30 缺口）
    /// 返回是否成功
    pub fn repair_item(&mut self, uid: u64, special: bool) -> bool {
        // C#：if (!special) MaxDura = max(0, MaxDura - (MaxDura - CurrentDura) / 30)
        let apply_decay = |item: &mut mir2_shared::data::item::UserItem| {
            if !special {
                let gap = item.max_dura.saturating_sub(item.current_dura);
                item.max_dura = item.max_dura.saturating_sub(gap / 30);
            }
            item.current_dura = item.max_dura;
            item.dura_changed = true;
        };
        // 检查背包
        for s in self.backpack.iter_mut().flatten() {
            if s.item.unique_id == uid {
                apply_decay(&mut s.item);
                return true;
            }
        }
        // 检查装备
        for e in self.equipment.iter_mut().flatten() {
            if e.unique_id == uid {
                apply_decay(e);
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
        if idx >= self.backpack.len() {
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
        for grid in 0..self.backpack.len() {
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

    /// 存入仓库指定格（C# StoreItem{From=背包格, To=仓库格} 语义）：优先目标格，占用则找第一个空位
    pub fn store_item_to(&mut self, from: i32, to: i32) -> Option<(UserItem, usize)> {
        let idx = from as usize;
        if idx >= self.backpack.len() {
            return None;
        }
        let item = self.backpack[idx].take()?.item;
        let mut target = to;
        if target < 0 || target as usize >= self.storage.len() || self.storage[target as usize].is_some() {
            target = self.storage.iter().position(|s| s.is_none())? as i32;
        }
        self.storage[target as usize] = Some(InventorySlot {
            grid: target as u8,
            item: item.clone(),
        });
        Some((item, target as usize))
    }

    /// 从仓库指定格取出（C# TakeBackItem{From=仓库格, To=背包格} 语义）：优先目标格，占用则找第一个空位
    pub fn take_back_item_to(&mut self, from: i32, to: i32) -> Option<(UserItem, u8)> {
        let sidx = from as usize;
        if sidx >= self.storage.len() {
            return None;
        }
        let item = self.storage[sidx].take()?.item;
        let mut target = to;
        if target < 0 || target as usize >= self.backpack.len() || self.backpack[target as usize].is_some() {
            target = self.backpack.iter().position(|s| s.is_none())? as i32;
        }
        self.backpack[target as usize] = Some(InventorySlot {
            grid: target as u8,
            item: item.clone(),
        });
        Some((item, target as u8))
    }

    /// 检查仓库是否有空位
    pub fn storage_has_space(&self) -> bool {
        self.storage.iter().any(|s| s.is_none())
    }

    /// 仓库扩容（C# AccountInfo.ExpandStorage：StorageGridSize=80 → 160；
    /// 已扩容（160）时保持原长度，由调用方负责续期/扣金）
    pub fn expand_storage(&mut self) -> usize {
        if self.storage.len() == STORAGE_SIZE {
            self.storage.resize(STORAGE_SIZE * 2, None);
        }
        self.storage.len()
    }

    /// 背包扩容（C# CharacterInfo.ResizeInventory：首次 +8，之后每次 +4，上限 86；
    /// Rust 基线 40 格起步，相对增长模式与 C# 一致）
    pub fn resize_inventory(&mut self) -> usize {
        const MAX_INVENTORY_SIZE: usize = 86; // C# ResizeInventory 上限 86
        let len = self.backpack.len();
        if len >= MAX_INVENTORY_SIZE {
            return len;
        }
        let grow = if len == BACKPACK_SIZE { 8 } else { 4 };
        let new_len = (len + grow).min(MAX_INVENTORY_SIZE);
        self.backpack.resize(new_len, None);
        new_len
    }
    /// 镶嵌宝石：将 from_grid 的宝石插入 to_grid 装备的第一个空槽位
    /// 返回 (source_uid, target_uid) 或 None
    pub fn socket_gem(&mut self, from_grid: u8, to_grid: u8, target_slot_count: usize) -> Option<(u64, u64)> {
        let fi = from_grid as usize;
        let ti = to_grid as usize;
        if fi >= self.backpack.len() || ti >= self.backpack.len() || fi == ti {
            return None;
        }

        // 先检查目标是否有空槽位（避免提前取走源物品后失败导致物品丢失）
        {
            let target_slot = self.backpack.get(ti)?;
            let target = &target_slot.as_ref()?.item;
            let mut slots = target.slots.clone();
            if slots.len() < target_slot_count {
                slots.resize_with(target_slot_count, || None);
            }
            if !slots.iter().any(|s| s.is_none()) {
                return None;
            }
        }

        // 取出源物品（宝石）
        let source = self.backpack.get_mut(fi)?.take()?;
        let source_uid = source.item.unique_id;

        // 获取目标物品（装备）
        let target_slot = self.backpack.get_mut(ti)?;
        let target = &mut target_slot.as_mut()?.item;

        // 确保目标槽位数组已初始化
        if target.slots.len() < target_slot_count {
            target.slots.resize_with(target_slot_count, || None);
        }

        // 查找第一个空槽位
        let empty_idx = target.slots.iter().position(|s| s.is_none())?;

        // 镶嵌宝石
        target.slots[empty_idx] = Some(source.item);
        target.gem_count = target.gem_count.saturating_add(1);
        let target_uid = target.unique_id;

        Some((source_uid, target_uid))
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

    /// 添加任务物品到任务格（C# GainQuestItem：找空位，不合并），返回分配的唯一 ID
    pub fn add_quest_item(&mut self, mut item: UserItem) -> Option<u64> {
        for grid in 0..self.quest_inventory.len() {
            if self.quest_inventory[grid].is_none() {
                item.unique_id = self.next_unique_id();
                let uid = item.unique_id;
                self.quest_inventory[grid] = Some(item);
                return Some(uid);
            }
        }
        None
    }

    /// 按 item_index 统计任务格中该物品总数量
    pub fn count_quest_item_by_index(&self, item_index: i32) -> u16 {
        self.quest_inventory.iter().flatten()
            .filter(|i| i.item_index == item_index)
            .map(|i| i.count)
            .sum()
    }

    /// 从任务格移除指定数量（C# RecalculateQuestBag 逐格删除），返回 (unique_id, removed) 列表供 S.DeleteQuestItem 下发
    pub fn remove_quest_item_by_index(&mut self, item_index: i32, mut count: u16) -> Vec<(u64, u16)> {
        let mut removed = Vec::new();
        for slot in self.quest_inventory.iter_mut() {
            if let Some(item) = slot {
                if item.item_index != item_index { continue; }
                if item.count > count {
                    item.count -= count;
                    removed.push((item.unique_id, count));
                    break;
                } else {
                    count -= item.count;
                    removed.push((item.unique_id, item.count));
                    *slot = None;
                    if count == 0 { break; }
                }
            }
        }
        removed
    }
}

/// 创建一个测试/辅助用的 UserItem（unique_id=0，由 add_item 自动分配）
pub fn make_item(index: i32, count: u16) -> UserItem {
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_expand_storage() {
        // #888：C# ExpandStorage 80 → 160；已扩容时不变
        let mut inv = PlayerInventory::new();
        assert_eq!(inv.storage.len(), STORAGE_SIZE);
        let len = inv.expand_storage();
        assert_eq!(len, STORAGE_SIZE * 2);
        assert_eq!(inv.storage.len(), STORAGE_SIZE * 2);
        // 再次调用不重复扩容
        assert_eq!(inv.expand_storage(), STORAGE_SIZE * 2);
        // 扩容后新格子可用（160 格可全部占用）
        for g in 0..(STORAGE_SIZE * 2) {
            inv.storage[g] = Some(InventorySlot { grid: g as u8, item: make_item(3, 1) });
        }
        assert!(!inv.storage_has_space());
        assert_eq!(inv.storage.iter().filter(|s| s.is_some()).count(), STORAGE_SIZE * 2);
    }

    #[test]
    fn test_resize_inventory() {
        // #899：C# CharacterInfo.ResizeInventory：首次 +8，之后 +4，上限 86
        let mut inv = PlayerInventory::new();
        assert_eq!(inv.backpack.len(), BACKPACK_SIZE);
        // 首次 40 → 48
        assert_eq!(inv.resize_inventory(), 48);
        assert_eq!(inv.backpack.len(), 48);
        // 之后每次 +4：48→52→56→…→86
        assert_eq!(inv.resize_inventory(), 52);
        assert_eq!(inv.resize_inventory(), 56);
        // 快速扩到上限
        for _ in 0..20 { inv.resize_inventory(); }
        assert_eq!(inv.backpack.len(), 86);
        // 上限后不再增长
        assert_eq!(inv.resize_inventory(), 86);
        assert_eq!(inv.backpack.len(), 86);
        // 扩容后新格子可用（48 格可全部占用）
        let mut inv2 = PlayerInventory::new();
        inv2.resize_inventory();
        for g in 0..48 {
            inv2.backpack[g] = Some(InventorySlot { grid: g as u8, item: make_item(9, 1) });
        }
        assert_eq!(inv2.backpack.iter().filter(|s| s.is_some()).count(), 48);
        // 越界校验按当前长度
        assert!(inv2.store_item(48).is_none()); // 48 已在扩容后范围内（格子满则返回 None 前已取走）
    }

    #[test]
    fn test_backpack_full_cannot_store() {
        let mut inv = PlayerInventory::new();
        // Try to store from empty backpack grid
        assert!(inv.store_item(0).is_none());
    }

    #[test]
    fn test_remove_item_by_index_with_dura() {
        let mut inv = PlayerInventory::new();
        // 两把同 id 武器，耐久不同
        let (g1, _) = inv.add_item(make_item(7, 1)).unwrap();
        let (g2, _) = inv.add_item(make_item(7, 1)).unwrap();
        inv.backpack[g1 as usize].as_mut().unwrap().item.current_dura = 50;
        inv.backpack[g2 as usize].as_mut().unwrap().item.current_dura = 5000;

        // 无 dura 过滤：全部可移除
        assert!(inv.remove_item_by_index_with_dura(7, 2, None));
        assert_eq!(inv.count_item_by_index(7), 0);

        // 重新放入：一把 50、一把 5000
        let (g1, _) = inv.add_item(make_item(7, 1)).unwrap();
        let (g2, _) = inv.add_item(make_item(7, 1)).unwrap();
        inv.backpack[g1 as usize].as_mut().unwrap().item.current_dura = 50;
        inv.backpack[g2 as usize].as_mut().unwrap().item.current_dura = 5000;

        // C# TakeItem：移除尽可能多——数量不足也移除能匹配的（5000 那把）
        assert!(inv.remove_item_by_index_with_dura(7, 2, Some(1000)));
        assert_eq!(inv.count_item_by_index(7), 1);
        // 剩余那把是耐久 50 的
        let remaining = inv.backpack.iter().flatten().next().unwrap();
        assert_eq!(remaining.item.current_dura, 50);

        // 没有匹配耐久的物品 → 不移除
        assert!(!inv.remove_item_by_index_with_dura(7, 1, Some(1000)));
        assert_eq!(inv.count_item_by_index(7), 1);
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
        assert!(inv.repair_item(uid, false));
        if let Some(s) = &inv.backpack[grid as usize] {
            assert_eq!(s.item.current_dura, s.item.max_dura);
            assert!(s.item.dura_changed);
        }

        // 修理不存在的物品
        assert!(!inv.repair_item(99999, false));
    }

    #[test]
    fn test_socket_gem() {
        let mut inv = PlayerInventory::new();
        // 添加宝石到格子 0
        let mut gem = make_item(100, 1);
        gem.unique_id = 1;
        inv.backpack[0] = Some(InventorySlot { grid: 0, item: gem });

        // 添加装备到格子 1（预设 2 个槽位）
        let mut equip = make_item(200, 1);
        equip.unique_id = 2;
        equip.slots = vec![None, None];
        inv.backpack[1] = Some(InventorySlot { grid: 1, item: equip });

        // 镶嵌成功
        let result = inv.socket_gem(0, 1, 2);
        assert!(result.is_some());
        let (source_uid, target_uid) = result.unwrap();
        assert_eq!(source_uid, 1);
        assert_eq!(target_uid, 2);

        // 源格已空
        assert!(inv.backpack[0].is_none());
        // 目标装备槽位被填充
        let target = inv.backpack[1].as_ref().unwrap();
        assert_eq!(target.item.gem_count, 1);
        assert!(target.item.slots[0].is_some());
        assert_eq!(target.item.slots[0].as_ref().unwrap().unique_id, 1);
        assert!(target.item.slots[1].is_none());
    }

    #[test]
    fn test_socket_gem_no_empty_slots() {
        let mut inv = PlayerInventory::new();
        let mut gem = make_item(100, 1);
        gem.unique_id = 1;
        inv.backpack[0] = Some(InventorySlot { grid: 0, item: gem });

        // 装备槽位已满
        let mut equip = make_item(200, 1);
        equip.unique_id = 2;
        equip.slots = vec![Some(make_item(300, 1))];
        inv.backpack[1] = Some(InventorySlot { grid: 1, item: equip });

        // 镶嵌失败（无空槽）
        assert!(inv.socket_gem(0, 1, 1).is_none());
        // 源物品应保留
        assert!(inv.backpack[0].is_some());
    }

    #[test]
    fn test_socket_gem_same_grid() {
        let mut inv = PlayerInventory::new();
        let gem = make_item(100, 1);
        inv.backpack[0] = Some(InventorySlot { grid: 0, item: gem });

        // 同格子镶嵌应失败
        assert!(inv.socket_gem(0, 0, 2).is_none());
    }

    /// #1159：C# RepairItem——非特殊修理 MaxDura 衰减 (MaxDura-CurrentDura)/30，特殊修理不衰减
    #[test]
    fn test_repair_item_max_dura_decay() {
        let mut inv = PlayerInventory::new();
        let mut item = make_item(100, 1);
        item.unique_id = 100;
        item.max_dura = 1000;
        item.current_dura = 700; // 缺口 300 → 衰减 300/30=10
        inv.backpack[0] = Some(InventorySlot { grid: 0, item });
        assert!(inv.repair_item(100, false));
        let it = inv.backpack[0].as_ref().unwrap().item.clone();
        assert_eq!(it.max_dura, 990);
        assert_eq!(it.current_dura, 990);

        // 特殊修理：不衰减 MaxDura
        let mut item2 = make_item(101, 1);
        item2.unique_id = 101;
        item2.max_dura = 1000;
        item2.current_dura = 500;
        inv.backpack[1] = Some(InventorySlot { grid: 1, item: item2 });
        assert!(inv.repair_item(101, true));
        let it2 = inv.backpack[1].as_ref().unwrap().item.clone();
        assert_eq!(it2.max_dura, 1000);
        assert_eq!(it2.current_dura, 1000);
    }
    #[test]
    fn fishing_slot_type_ok_matches() {
        use mir2_shared::enums::ItemType;
        assert!(fishing_slot_type_ok(0, Some(ItemType::Hook)));
        assert!(fishing_slot_type_ok(1, Some(ItemType::Float)));
        assert!(fishing_slot_type_ok(2, Some(ItemType::Bait)));
        assert!(fishing_slot_type_ok(3, Some(ItemType::Finder)));
        assert!(fishing_slot_type_ok(4, Some(ItemType::Reel)));
        assert!(!fishing_slot_type_ok(0, Some(ItemType::Float)));
        assert!(!fishing_slot_type_ok(5, None));
        assert!(fishing_slot_type_ok(0, None));
    }

    fn rod_item(shape: i16) -> UserItem {
        let mut r = make_item(9101, 1);
        r.unique_id = 7001;
        r.info = Some(mir2_shared::data::item::ItemInfo {
            item_type: mir2_shared::enums::ItemType::Weapon,
            shape,
            ..Default::default()
        });
        r
    }

    #[test]
    fn equip_fishing_gear_ok_and_replace() {
        let mut inv = PlayerInventory::default();
        inv.equipment[EquipmentSlot::Weapon as usize] = Some(rod_item(49));
        let mut hook = make_item(9102, 1);
        hook.unique_id = 8001;
        let (_, uid) = inv.add_item(hook.clone()).expect("add hook");
        assert!(equip_fishing_gear(&mut inv, 7001, 0, uid).is_ok());
        let rod = inv.equipment[EquipmentSlot::Weapon as usize].as_ref().unwrap();
        assert_eq!(rod.slots[0].as_ref().unwrap().unique_id, uid);
        assert!(!inv.backpack.iter().any(|s| s.as_ref().map_or(false, |sl| sl.item.unique_id == uid)));
        let mut hook2 = make_item(9103, 1);
        hook2.unique_id = 8002;
        let (_, uid2) = inv.add_item(hook2.clone()).expect("add hook2");
        assert!(equip_fishing_gear(&mut inv, 7001, 0, uid2).is_ok());
        let rod = inv.equipment[EquipmentSlot::Weapon as usize].as_ref().unwrap();
        assert_eq!(rod.slots[0].as_ref().unwrap().unique_id, uid2);
        assert!(inv.backpack.iter().any(|s| s.as_ref().map_or(false, |sl| sl.item.unique_id == uid)));
    }

    #[test]
    fn equip_fishing_gear_rejects_non_rod_and_bad_slot() {
        let mut inv = PlayerInventory::default();
        inv.equipment[EquipmentSlot::Weapon as usize] = Some(rod_item(0));
        let mut g = make_item(9102, 1);
        g.unique_id = 8001;
        let (_, uid) = inv.add_item(g).expect("add");
        assert_eq!(equip_fishing_gear(&mut inv, 7001, 0, uid), Err("不是钓鱼竿"));
        inv.equipment[EquipmentSlot::Weapon as usize] = Some(rod_item(49));
        assert_eq!(equip_fishing_gear(&mut inv, 7001, 5, uid), Err("无效钓具槽"));
    }

    fn rod_with_gear(rod_dura: u16, slots: Vec<Option<UserItem>>) -> UserItem {
        let mut r = make_item(9101, 1);
        r.unique_id = 7001;
        r.current_dura = rod_dura;
        r.info = Some(mir2_shared::data::item::ItemInfo {
            item_type: mir2_shared::enums::ItemType::Weapon,
            shape: 49,
            ..Default::default()
        });
        r.slots = slots;
        r
    }

    fn gear_item(uid: u64, count: u16, dura: u16) -> UserItem {
        let mut g = make_item(9102, count);
        g.unique_id = uid;
        g.current_dura = dura;
        g.max_dura = dura;
        g.info = Some(mir2_shared::data::item::ItemInfo {
            item_type: mir2_shared::enums::ItemType::Hook,
            ..Default::default()
        });
        g
    }

    #[test]
    fn fishing_bait_consume() {
        let mut inv = PlayerInventory::default();
        inv.equipment[EquipmentSlot::Weapon as usize] = Some(rod_with_gear(
            100,
            vec![None, None, Some(gear_item(9001, 2, 10)), None, None],
        ));
        assert_eq!(inv.fishing_bait_count(), 2);
        assert!(inv.fishing_consume_bait(1));
        assert_eq!(inv.fishing_bait_count(), 1);
        assert!(inv.fishing_consume_bait(1));
        assert_eq!(inv.fishing_bait_count(), 0);
        assert!(!inv.fishing_consume_bait(1));
    }

    #[test]
    fn fishing_gear_durability() {
        let mut inv = PlayerInventory::default();
        inv.equipment[EquipmentSlot::Weapon as usize] = Some(rod_with_gear(
            100,
            vec![Some(gear_item(9001, 1, 5)), None, None, None, None],
        ));
        assert_eq!(inv.fishing_gear_damage(0, 1), FishingGearDamageResult::Ok);
        assert_eq!(inv.fishing_gear_damage(0, 4), FishingGearDamageResult::Broken);
        assert_eq!(inv.fishing_gear_damage(0, 1), FishingGearDamageResult::NoGear);
        assert_eq!(inv.fishing_gear_damage(2, 1), FishingGearDamageResult::NoGear);
    }

    #[test]
    fn fishing_rod_durability_loss() {
        let mut inv = PlayerInventory::default();
        inv.equipment[EquipmentSlot::Weapon as usize] = Some(rod_with_gear(2, vec![None, None, None, None, None]));
        inv.fishing_rod_durability_loss(1);
        assert_eq!(inv.fishing_rod().unwrap().current_dura, 1);
        inv.fishing_rod_durability_loss(5);
        assert_eq!(inv.fishing_rod().unwrap().current_dura, 0);
    }

}
