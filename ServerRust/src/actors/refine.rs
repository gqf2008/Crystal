// 精炼系统（Refining）
// 纯数据结构，由 WorldActor 调用

/// C# Info.Refine 材料槽数量（PlayerInfo.Refine；PlayerObject.cs:12535）
pub const REFINE_MATERIAL_SLOTS: usize = 10;
// #2392：其余 Refine* 已收入 util::config::RefineConfig（C# Settings.Refine*），由 WorldActor.refine_cfg 传入

/// 精炼状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum RefineStatus {
    None = 0,
    Pending = 1,    // 等待精炼完成
    Ready = 2,      // 精炼完成，可取回
    Failed = 3,     // 精炼失败
}

/// 正在精炼的物品
#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct RefiningItem {
    /// 原始物品唯一 ID（背包中的 unique_id）
    pub original_uid: u64,
    /// 精炼物品索引
    pub item_index: u32,
    /// 开始时间（秒）
    pub start_time: u64,
    /// 预计完成时间（秒）
    pub finish_time: u64,
    /// 状态
    pub status: RefineStatus,
    /// 精炼成功率（0-100）
    pub success_chance: u8,
    /// 精炼物品完整数据（C# Info.CurrentRefine；存入时克隆，取回/取消时返还）
    pub item: Option<mir2_shared::data::item::UserItem>,
}

/// 精炼日志（每个玩家一个）
#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct RefineLog {
    /// 当前正在精炼的物品
    pub active_refine: Option<RefiningItem>,
    /// 精炼历史计数
    pub total_refines: u32,
    /// 成功计数
    pub successful_refines: u32,
    /// C# Info.Refine：精炼材料槽（10 格，PlayerObject.cs:12722-12751）
    pub materials: Vec<Option<mir2_shared::data::item::UserItem>>,
}

impl Default for RefineLog {
    fn default() -> Self {
        Self {
            active_refine: None,
            total_refines: 0,
            successful_refines: 0,
            materials: vec![None; REFINE_MATERIAL_SLOTS],
        }
    }
}

/// C# PlayerObject.RefineItem（:12811-12845）：精炼成功率公式
/// 输入为材料聚合值（C# :12710-12751 计算）；无属性材料（refine_stat<=0）→ 0（C# :12753）
#[allow(clippy::too_many_arguments)]
pub fn refine_success_chance(
    refine_stat: i32,
    item_required_amount: i32,
    required_level: i32,
    item_amount: i32,
    durability_count: i32,
    current_dura_count: i32,
    ore_amount: i32,
    ore_purity: i32,
    luck: i32,
    added_dc: i32,
    added_mc: i32,
    added_sc: i32,
    is_weapon: bool,
    base_chance: i32,
    wep_stat_reduce: i32,
    item_stat_reduce: i32,
) -> i32 {
    // C# :12753 无属性材料 → 0（RefineAdded 仍设 RefineIncrease，但无 RefinedValue 不应用）
    if refine_stat <= 0 {
        return 0;
    }
    // itemSuccess（C# :12811-12821）：先钳 0..10，再按条件 +10/+10/+5
    let mut item_success = (refine_stat * 5 - item_required_amount + 5).clamp(0, 10);
    if item_amount > 0 && (required_level / item_amount) > (item_required_amount - 5) {
        item_success += 10;
    }
    if item_amount > 0 && durability_count == item_amount {
        item_success += 10;
    }
    if item_amount > 0 && current_dura_count == item_amount {
        item_success += 5;
    }
    // oreSuccess（C# :12823-12827）
    let mut ore_success = 0;
    if item_amount > 0 && ore_amount >= item_amount {
        ore_success += 15;
    }
    if item_amount > 0 && ore_amount > 0 && (ore_purity / ore_amount) >= (refine_stat / item_amount) {
        ore_success += 15;
    }
    if ore_amount > 0 && ore_purity == refine_stat {
        ore_success += 5;
    }
    // luckSuccess（C# :12829-12831，上限 10）
    let luck_success = (luck + 5).clamp(0, 10);
    // addedStats 惩罚（C# :12838-12843；武器 RefineWepStatReduce=6 / 其他 RefineItemStatReduce=15，上限 50；#2392 配置化）
    let added = ((added_dc + added_mc + added_sc) * if is_weapon { wep_stat_reduce } else { item_stat_reduce }).clamp(0, 50);
    // RefineBaseChance=20（C# Settings.cs:251；#2392 配置化）
    (item_success + ore_success + luck_success + base_chance - added).max(0)
}

/// C# RefineItem 材料聚合（PlayerObject.cs:12710-12751）
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefineMaterials {
    pub total_dc: i32,
    pub total_mc: i32,
    pub total_sc: i32,
    pub required_level: i32,
    pub item_amount: i32,
    pub durability_count: i32,
    pub current_dura_count: i32,
    pub ore_amount: i32,
    pub ore_purity: i32,
}

/// 聚合精炼材料（C# PlayerObject.cs:12722-12751）：武器材料跳过；DC/MC/SC 材料累计属性；
/// FriendlyName == RefineOreName 累计矿纯度。
pub fn refine_material_aggregates(
    materials: &[Option<mir2_shared::data::item::UserItem>],
    item_infos: &std::collections::HashMap<i32, crate::db::ItemInfo>,
    refine_ore_name: &str,
) -> RefineMaterials {
    use mir2_shared::enums::Stat;
    let mut agg = RefineMaterials::default();
    for m in materials.iter().flatten() {
        let Some(info) = item_infos.get(&m.item_index) else { continue };
        // C# :12727-12731 武器材料跳过（ItemType.Weapon=1）
        if info.item_type == 1 {
            continue;
        }
        let dc = info.stats.get(&(Stat::MaxDC as u8)).copied().unwrap_or(0);
        let mc = info.stats.get(&(Stat::MaxMC as u8)).copied().unwrap_or(0);
        let sc = info.stats.get(&(Stat::MaxSC as u8)).copied().unwrap_or(0);
        if dc > 0 || mc > 0 || sc > 0 {
            // C# :12735-12741：Min+Max+AddedStats（DC/MC/SC 同理）
            agg.total_dc += info.stats.get(&(Stat::MinDC as u8)).copied().unwrap_or(0) + dc + m.added_stats.get(Stat::MaxDC);
            agg.total_mc += info.stats.get(&(Stat::MinMC as u8)).copied().unwrap_or(0) + mc + m.added_stats.get(Stat::MaxMC);
            agg.total_sc += info.stats.get(&(Stat::MinSC as u8)).copied().unwrap_or(0) + sc + m.added_stats.get(Stat::MaxSC);
            agg.required_level += info.required_amount;
            // C# :12739 floor(MaxDura/1000) == floor(Info.Durability/1000)
            if m.max_dura as i32 / 1000 == info.durability / 1000 {
                agg.durability_count += 1;
            }
            // C# :12740 floor(CurrentDura/1000) == floor(MaxDura/1000)
            if m.current_dura as i32 / 1000 == m.max_dura as i32 / 1000 {
                agg.current_dura_count += 1;
            }
            agg.item_amount += 1;
        }
        // C# :12744-12748 矿（FriendlyName == RefineOreName）
        if info.name == refine_ore_name {
            agg.ore_purity += m.current_dura as i32 / 1000;
            agg.ore_amount += 1;
        }
    }
    agg
}

/// CheckRefine 结算结果（C# PlayerObject.cs:12925-12971）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefineCheckResult {
    /// 成功：已应用 RefineAdded 属性（可取回）
    Applied,
    /// 失败/无 RefinedValue：物品被粉碎（C# :12961-12967）
    Destroyed,
}

impl RefineLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// 开始精炼
    pub fn start_refine(&mut self, item_index: u32, current_time: u64, duration_seconds: u64, success_chance: u8) -> RefiningItem {
        let item = RefiningItem {
            original_uid: 0, // 由 WorldActor 设置
            item_index,
            start_time: current_time,
            finish_time: current_time + duration_seconds,
            status: RefineStatus::Pending,
            success_chance,
            item: None,
        };
        self.active_refine = Some(item.clone());
        item
    }

    /// 开始精炼（C# RefineItem：已存入物品转 Pending + finish_time + 成功率，保留 item）
    pub fn begin_refine(&mut self, current_time: u64, duration_seconds: u64, success_chance: u8) -> bool {
        let Some(item) = self.active_refine.as_mut() else { return false };
        if item.status != RefineStatus::None { return false; }
        item.start_time = current_time;
        item.finish_time = current_time + duration_seconds;
        item.status = RefineStatus::Pending;
        item.success_chance = success_chance;
        true
    }

    /// 存入精炼物品（C# RefineItem：完整物品克隆进 CurrentRefine，WorldActor 负责从背包移除）
    pub fn deposit_item(&mut self, item: mir2_shared::data::item::UserItem) -> bool {
        if self.active_refine.is_some() {
            return false; // 已有精炼进行中
        }
        let uid = item.unique_id;
        let item_index = item.item_index as u32;
        self.active_refine = Some(RefiningItem {
            original_uid: uid,
            item_index,
            start_time: 0,
            finish_time: 0,
            status: RefineStatus::None,
            success_chance: 0,
            item: Some(item),
        });
        true
    }

    /// 取消精炼
    pub fn cancel(&mut self) -> Option<RefiningItem> {
        self.active_refine.take()
    }

    /// 存入材料（C# DepositRefineItem：背包 → 精炼材料格）
    pub fn deposit_material(&mut self, slot: usize, item: mir2_shared::data::item::UserItem) -> bool {
        if slot >= self.materials.len() || self.materials[slot].is_some() {
            return false;
        }
        self.materials[slot] = Some(item);
        true
    }

    /// 取回材料（C# RetrieveRefineItem：精炼材料格 → 背包）
    pub fn retrieve_material(&mut self, slot: usize) -> Option<mir2_shared::data::item::UserItem> {
        if slot >= self.materials.len() {
            return None;
        }
        self.materials[slot].take()
    }

    /// 取回全部材料并重置 10 格（C# RefineCancel / RefineItem 消耗：PlayerObject.cs:12603-12639 / 12750）
    pub fn take_all_materials(&mut self) -> Vec<Option<mir2_shared::data::item::UserItem>> {
        let taken = std::mem::take(&mut self.materials);
        self.materials = vec![None; REFINE_MATERIAL_SLOTS];
        taken
    }

    /// 取回精炼物品
    pub fn retrieve(&mut self) -> Option<RefiningItem> {
        self.active_refine.take()
    }

    /// C# CheckRefine 结算（PlayerObject.cs:12925-12971）：失败 → RefinedValue=None → 物品粉碎；
    /// 成功 → 按 RefinedValue 加 MaxDC/MC/SC + RefineAdded（暴击 ×2）；清空精炼字段。
    /// 返回 None 表示未到结算时机/无精炼。
    pub fn settle_check(&mut self, crit_chance: u8, crit_increase: u8) -> Option<RefineCheckResult> {
        use mir2_shared::enums::{RefinedValue, Stat};
        let item = self.active_refine.as_mut()?;
        if item.status != RefineStatus::Pending {
            return None;
        }
        item.status = RefineStatus::Ready;
        self.total_refines += 1;
        let Some(ui) = item.item.as_mut() else {
            return Some(RefineCheckResult::Destroyed);
        };
        // C# :12925 失败（Random(1,100) > RefineSuccessChance）→ RefinedValue = None
        if fastrand::u16(1..100) > ui.refine_success_chance.min(99) as u16 {
            ui.refined_value = RefinedValue::None;
        }
        // C# :12930-12933 暴击：Random(1,100) < RefineCritChance → RefineAdded *= RefineCritIncrease
        if fastrand::u16(1..100) < crit_chance as u16 {
            ui.refine_added = ui.refine_added.saturating_mul(crit_increase);
        }
        // C# :12935-12960 应用属性
        let applied = match ui.refined_value {
            RefinedValue::Dc if ui.refine_added > 0 => {
                let cur = ui.added_stats.get(Stat::MaxDC);
                ui.added_stats.set(Stat::MaxDC, cur + i32::from(ui.refine_added));
                true
            }
            RefinedValue::Mc if ui.refine_added > 0 => {
                let cur = ui.added_stats.get(Stat::MaxMC);
                ui.added_stats.set(Stat::MaxMC, cur + i32::from(ui.refine_added));
                true
            }
            RefinedValue::Sc if ui.refine_added > 0 => {
                let cur = ui.added_stats.get(Stat::MaxSC);
                ui.added_stats.set(Stat::MaxSC, cur + i32::from(ui.refine_added));
                true
            }
            _ => false,
        };
        // C# :12939-12940 / 12958-12959 / 12965：清空精炼字段
        ui.refined_value = RefinedValue::None;
        ui.refine_added = 0;
        ui.refine_success_chance = 0;
        if applied {
            self.successful_refines += 1;
        }
        Some(if applied { RefineCheckResult::Applied } else { RefineCheckResult::Destroyed })
    }

    /// 检查精炼是否完成
    pub fn is_ready(&self, current_time: u64) -> bool {
        if let Some(ref item) = self.active_refine {
            item.status == RefineStatus::Pending && current_time >= item.finish_time
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refine_success_chance_matches_csharp_formula() {
        // 无材料 → 0（C# :12753）
        assert_eq!(refine_success_chance(0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, true, 20, 6, 15), 0);
        // 全条件：itemSuccess=10+10+5=25, oreSuccess=15+15+5=35, luck=5, base=20 → 85
        assert_eq!(refine_success_chance(10, 20, 0, 1, 1, 1, 1, 10, 0, 0, 0, 0, true, 20, 6, 15), 85);
        // luck 钳位：luck=20 → luckSuccess=10 → 90
        assert_eq!(refine_success_chance(10, 20, 0, 1, 1, 1, 1, 10, 20, 0, 0, 0, true, 20, 6, 15), 90);
        // 武器 addedStats 惩罚（×6）：added_dc=5 → -30 → 55
        assert_eq!(refine_success_chance(10, 20, 0, 1, 1, 1, 1, 10, 0, 5, 0, 0, true, 20, 6, 15), 55);
        // 非武器（×15）：-75 → 惩罚封顶 50 → 85-50=35
        assert_eq!(refine_success_chance(10, 20, 0, 1, 1, 1, 1, 10, 0, 5, 0, 0, false, 20, 6, 15), 35);
        // 惩罚封顶 50：-50 → 35
        assert_eq!(refine_success_chance(10, 20, 0, 1, 1, 1, 1, 10, 0, 20, 0, 0, true, 20, 6, 15), 35);
        // 无 ore：oreSuccess=0 → 25+5+20=50
        assert_eq!(refine_success_chance(10, 20, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, true, 20, 6, 15), 50);
        // 低材料：itemSuccess=0, luck=5, base=20 → 25
        assert_eq!(refine_success_chance(1, 100, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, true, 20, 6, 15), 25);
    }

    #[test]
    fn test_start_refine() {
        let mut log = RefineLog::new();
        let item = log.start_refine(100, 0, 3600, 80);
        assert_eq!(item.item_index, 100);
        assert_eq!(item.status, RefineStatus::Pending);
        assert_eq!(item.success_chance, 80);
        assert!(log.active_refine.is_some());
    }

    #[test]
    fn test_deposit_and_cancel() {
        let mut log = RefineLog::new();
        let mut it = mir2_shared::data::item::UserItem::default();
        it.unique_id = 12345;
        assert!(log.deposit_item(it));
        assert!(log.active_refine.is_some());
        let retrieved = log.cancel();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().original_uid, 12345);
        assert!(log.active_refine.is_none());
    }

    #[test]
    fn test_deposit_twice_fails() {
        let mut log = RefineLog::new();
        let mut it = mir2_shared::data::item::UserItem::default();
        it.unique_id = 1;
        assert!(log.deposit_item(it));
        let mut it2 = mir2_shared::data::item::UserItem::default();
        it2.unique_id = 2;
        assert!(!log.deposit_item(it2)); // 已有物品
    }

    #[test]
    fn test_materials_default_slots() {
        let log = RefineLog::new();
        assert_eq!(log.materials.len(), REFINE_MATERIAL_SLOTS);
        assert!(log.materials.iter().all(|s| s.is_none()));
    }

    #[test]
    fn test_deposit_retrieve_material() {
        let mut log = RefineLog::new();
        let mut it = mir2_shared::data::item::UserItem::default();
        it.unique_id = 42;
        assert!(log.deposit_material(0, it.clone()));
        assert!(!log.deposit_material(0, mir2_shared::data::item::UserItem::default())); // 占用
        assert!(!log.deposit_material(99, mir2_shared::data::item::UserItem::default())); // 越界
        let got = log.retrieve_material(0).unwrap();
        assert_eq!(got.unique_id, 42);
        assert!(log.retrieve_material(0).is_none());
        assert!(log.retrieve_material(99).is_none());
    }

    #[test]
    fn test_take_all_materials_resets_slots() {
        let mut log = RefineLog::new();
        log.deposit_material(0, mir2_shared::data::item::UserItem::default());
        log.deposit_material(1, mir2_shared::data::item::UserItem::default());
        let all = log.take_all_materials();
        assert_eq!(all.iter().flatten().count(), 2);
        assert_eq!(log.materials.len(), REFINE_MATERIAL_SLOTS);
        assert!(log.materials.iter().all(|s| s.is_none()));
    }

    #[test]
    fn test_refine_material_aggregates() {
        use mir2_shared::data::stats::Stats;
        use mir2_shared::enums::Stat;
        use std::collections::HashMap;
        // DC 材料：Info MinDC=1 MaxDC=3（DB 键已 +3 转 SharedRust Stat），added MaxDC=2 → total_dc=6
        let mut item = mir2_shared::data::item::UserItem::default();
        item.item_index = 10;
        item.added_stats.set(Stat::MaxDC, 2);
        item.max_dura = 2000;
        item.current_dura = 2000;
        // 矿：name=BlackIronOre，CurrentDura=5000 → ore_purity=5
        let mut ore = mir2_shared::data::item::UserItem::default();
        ore.item_index = 20;
        ore.current_dura = 5000;
        let mut infos = HashMap::new();
        infos.insert(10, crate::db::ItemInfo {
            item_type: 2, // Armour
            required_amount: 5,
            durability: 2000,
            stats: HashMap::from([(Stat::MinDC as u8, 1), (Stat::MaxDC as u8, 3)]),
            ..Default::default()
        });
        infos.insert(20, crate::db::ItemInfo {
            item_type: 2,
            name: "BlackIronOre".to_string(),
            ..Default::default()
        });
        let agg = refine_material_aggregates(&[Some(item), Some(ore)], &infos, "BlackIronOre");
        assert_eq!(agg.total_dc, 6);
        assert_eq!(agg.total_mc, 0);
        assert_eq!(agg.item_amount, 1);
        assert_eq!(agg.required_level, 5);
        assert_eq!(agg.durability_count, 1);
        assert_eq!(agg.current_dura_count, 1);
        assert_eq!(agg.ore_amount, 1);
        assert_eq!(agg.ore_purity, 5);

        // 武器材料跳过（ItemType.Weapon=1）
        infos.insert(30, crate::db::ItemInfo { item_type: 1, ..Default::default() });
        let mut weapon = mir2_shared::data::item::UserItem::default();
        weapon.item_index = 30;
        let agg2 = refine_material_aggregates(&[Some(weapon)], &infos, "BlackIronOre");
        assert_eq!(agg2.item_amount, 0);
    }

    #[test]
    fn test_settle_check_applies_on_success() {
        use mir2_shared::enums::{RefinedValue, Stat};
        let mut log = RefineLog::new();
        let mut it = mir2_shared::data::item::UserItem::default();
        it.unique_id = 1;
        it.refined_value = RefinedValue::Dc;
        it.refine_added = 1;
        it.refine_success_chance = 100; // 必成功（C# Random(1,100) > 100 恒 false）
        assert!(log.deposit_item(it));
        assert!(log.begin_refine(0, 3600, 100));
        assert_eq!(log.settle_check(10, 2), Some(RefineCheckResult::Applied));
        let it = log.active_refine.as_ref().unwrap().item.as_ref().unwrap();
        let added = it.added_stats.get(Stat::MaxDC);
        assert!((1..=2).contains(&added), "added={}", added); // 暴击可能 ×2
        assert_eq!(it.refined_value, RefinedValue::None);
        assert_eq!(it.refine_added, 0);
        assert_eq!(log.successful_refines, 1);
        assert_eq!(log.total_refines, 1);
    }

    #[test]
    fn test_settle_check_destroys_on_fail() {
        use mir2_shared::enums::{RefinedValue, Stat};
        let mut log = RefineLog::new();
        let mut it = mir2_shared::data::item::UserItem::default();
        it.unique_id = 2;
        it.refined_value = RefinedValue::Dc;
        it.refine_added = 1;
        it.refine_success_chance = 0; // 必失败（Random(1,100) > 0 恒 true → RefinedValue=None）
        assert!(log.deposit_item(it));
        assert!(log.begin_refine(0, 3600, 0));
        assert_eq!(log.settle_check(10, 2), Some(RefineCheckResult::Destroyed));
        assert_eq!(log.successful_refines, 0);
        assert_eq!(log.total_refines, 1);
        let it = log.active_refine.as_ref().unwrap().item.as_ref().unwrap();
        assert_eq!(it.added_stats.get(Stat::MaxDC), 0);
        assert_eq!(it.refined_value, RefinedValue::None);
    }

    #[test]
    fn test_deposit_retrieve_roundtrip_keeps_item() {
        // C# CollectRefine：返还完整物品（唯一 ID / 索引一致）
        let mut log = RefineLog::new();
        let mut it = mir2_shared::data::item::UserItem::default();
        it.unique_id = 77;
        it.item_index = 5;
        assert!(log.deposit_item(it.clone()));
        assert!(log.begin_refine(0, 3600, 80));
        let retrieved = log.retrieve().unwrap();
        assert_eq!(retrieved.original_uid, 77);
        assert_eq!(retrieved.item.as_ref().unwrap().unique_id, 77);
        assert_eq!(retrieved.item.as_ref().unwrap().item_index, 5);
    }

    #[test]
    fn test_begin_refine_requires_deposit() {
        let mut log = RefineLog::new();
        assert!(!log.begin_refine(0, 3600, 80));
    }

    #[test]
    fn test_is_ready() {
        let mut log = RefineLog::new();
        assert!(!log.is_ready(0));

        log.start_refine(100, 0, 3600, 80);
        assert!(!log.is_ready(1000)); // Not ready yet
        assert!(log.is_ready(3600));  // Ready now
    }

    #[test]
    fn test_retrieve() {
        let mut log = RefineLog::new();
        log.start_refine(100, 0, 3600, 80);
        let item = log.retrieve();
        assert!(item.is_some());
        assert!(log.active_refine.is_none());
    }

    /// #2378：CollectRefine 语义——结算（Applied）后 retrieve 返回已应用属性的物品，精炼字段已清空
    #[test]
    fn test_settle_applied_then_retrieve_returns_upgraded_item() {
        use mir2_shared::enums::{RefinedValue, Stat};
        let mut log = RefineLog::new();
        let mut it = mir2_shared::data::item::UserItem::default();
        it.unique_id = 9;
        it.refined_value = RefinedValue::Dc;
        it.refine_added = 1;
        it.refine_success_chance = 100; // 必成功
        assert!(log.deposit_item(it));
        assert!(log.begin_refine(0, 3600, 100));
        assert_eq!(log.settle_check(10, 2), Some(RefineCheckResult::Applied));
        let ri = log.retrieve().unwrap();
        let item = ri.item.unwrap();
        assert!(item.added_stats.get(Stat::MaxDC) >= 1);
        assert_eq!(item.refined_value, RefinedValue::None);
        assert_eq!(item.refine_added, 0);
        assert!(log.active_refine.is_none());
    }
}
