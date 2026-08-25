// ============================================================================
// 本地玩家状态组件（#2633 批次4：HudState/CharacterState God Resource → 玩家实体组件）
// 设计：ecsplayer-design.md §6 组件 schema、§7 挂载、§10 系统拆分。
//
// 迁移策略（设计 §11 批1）：本批「只加写路径，读者零改动」——
//   · 这些组件挂在 `LocalPlayer` 实体上（spawn_local_player_with 生成时挂默认值）；
//   · 各 ServerEvent 写系统（player_vitals_events / player_status_events /
//     inventory_events）把值**同时**写进玩家组件与原 `HudState`（双写过渡），
//     读者仍读 `HudState`，保证任何读者读到的值与之前完全一致（行为等价）。
//   · 读者迁移到组件、删除 `HudState` 双写属后续批次。
//
// 聚合原则（设计 §6）：谁一起变（同一事件写）、谁一起被读（同一批 Query）就聚成一个
// 组件；并优先复用实体已有组件（PlayerName/NetObjectId/ActorAppearance/MountState，
// 不新建）。
// ============================================================================

use bevy::prelude::*;
use mir2_shared::enums::PetMode;

use crate::game::dialogs::inventory::InvItem;

/// 生命/法力（HealthChanged{hp,mp} 恒同写；HUD 血蓝球、自动喝药、施法、角色面板同读）。
///
/// 默认值对齐 `HudState`（hp=1/max_hp=1000/mp=1/max_mp=600），使组件从生成起即为
/// `HudState` 的等值镜像，避免后续读者迁移后首帧读到全 0。
#[derive(Component, Clone, Copy, PartialEq)]
pub struct Vitals {
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
}

impl Default for Vitals {
    fn default() -> Self {
        Self {
            hp: 1,
            max_hp: 1000,
            mp: 1,
            max_mp: 600,
        }
    }
}

/// 等级/经验（LevelChanged 同写三者；HUD 经验条/等级、mentor、quest_log 同读）。
/// 默认值对齐 `HudState`（level=1/max_exp=100）。
#[derive(Component, Clone, Copy, PartialEq)]
pub struct Progression {
    pub level: u16,
    pub exp: i64,
    pub max_exp: i64,
}

impl Default for Progression {
    fn default() -> Self {
        Self {
            level: 1,
            exp: 0,
            max_exp: 100,
        }
    }
}

/// 金币（GoldGained/Lost 独立事件；HUD/背包/交易读）。单字段组件，最终消解
/// `hud.gold` 与 `hud.inventory.gold` 双源（设计 §9）。
#[derive(Component, Clone, Copy, PartialEq, Default)]
pub struct Gold(pub u32);

/// 声望/功勋（#248，CreditGained/Lost 独立事件；仅 auto/world 读）。
#[derive(Component, Clone, Copy, PartialEq, Default)]
pub struct Credit(pub u32);

/// 基础属性（#268，BaseStats 独立事件；auto/world 读）。
#[derive(Component, Clone, PartialEq, Default)]
pub struct BaseStats(pub Vec<i32>);

/// 角色面板战斗属性（UserInformation 一次同写；仅 character.rs 面板读）。
/// 对应 `CharacterState` 的 18 个面板属性 + stats（设计 §2/§6）。
#[derive(Component, Clone, Copy, PartialEq, Default)]
pub struct CombatStats {
    /// [min, max] AC/MAC/DC/MC/SC
    pub stats: [[i32; 2]; 5],
    pub critical_rate: i32,
    pub critical_damage: i32,
    pub attack_speed: i32,
    pub accuracy: i32,
    pub agility: i32,
    pub luck: i32,
    pub bag_weight: i32,
    pub wear_weight: i32,
    pub hand_weight: i32,
    pub magic_resist: i32,
    pub poison_resist: i32,
    pub health_recovery: i32,
    pub spell_recovery: i32,
    pub poison_recovery: i32,
    pub holy: i32,
    pub freezing: i32,
    pub poison_atk: i32,
}

/// 状态旗标（输入/移动/物品门控同读；各事件分别写但都是 bool，聚一起省 Query 数）。
/// 骑乘不复读——复用 `MountState`（存在即骑乘，由 object_state/spawn 维护）。
///
/// 注：sprint/sneaking 由 buff.rs 事件写（设计 §5），本批尚未迁移该写者，暂为 false。
#[derive(Component, Clone, Copy, PartialEq, Default)]
pub struct StatusFlags {
    pub dead: bool,
    pub fishing: bool,
    pub paralysis: bool,
    pub in_trap_rock: bool,
    pub sprint: bool,
    pub sneaking: bool,
    pub reincarnation_offered: bool,
}

/// 宠物模式（#1388，PetModeChanged 独立事件；HUD attack_mode_text 读）。
#[derive(Component, Clone, Copy, PartialEq)]
pub struct PetModeState(pub PetMode);

impl Default for PetModeState {
    fn default() -> Self {
        Self(PetMode::Both)
    }
}

/// 装备 14 槽（UserInformation/ItemEquipped/ItemRemoved/耐久/修理/升级写；
/// character/dura/mount/fishing/storage 读）。
///
/// 注：`InvItem` 未 derive `PartialEq`，故本组件暂不 derive `PartialEq`（后续读者
/// 迁移需要 `!=` 比较时再给 `InvItem` 补上）。
#[derive(Component, Clone)]
pub struct Loadout {
    pub slots: Vec<Option<InvItem>>,
}

impl Default for Loadout {
    fn default() -> Self {
        Self {
            // #1136：服务端补 Torch/Belt/Stone 共 14 槽（对齐 HudState.equipment 默认）
            slots: vec![None; 14],
        }
    }
}

/// 背包（UserInformation/背包 CRUD 写；~15 文件读）。
/// 已按设计 §6 剥离：`page`→背包 UI 资源、`gold`→`Gold` 组件；保留
/// `weight`/`max_weight`（服务端 bag_weight + 本地 refresh_weight）。
#[derive(Component, Clone, Default)]
pub struct Inventory {
    /// 动态格数背包（默认空，UserInformation 全量写入；ResizeInventory 扩容/缩容，#276）
    pub items: Vec<Option<InvItem>>,
    pub weight: u32,
    pub max_weight: u32,
    /// 任务物品格（C# QuestInventory；UserInformation.quest_inventory 写入）
    pub quest_inventory: Vec<Option<InvItem>>,
}

impl Inventory {
    /// 按服务端 ResizeInventory 调整格数（C# Array.Resize：截断/补空，上限 MAX_INV_SLOTS）。
    /// 逻辑同 `InventoryState::resize`；本批写路径用全量镜像（见 inventory_events），
    /// 该方法供后续「直接操作组件」的读者迁移批次使用。
    pub fn resize(&mut self, size: usize) {
        let size = size.min(crate::game::dialogs::inventory::MAX_INV_SLOTS);
        if size < self.items.len() {
            self.items.truncate(size);
        } else {
            self.items.resize(size, None);
        }
    }

    /// #1544：RefreshStats 重量（C# User.RefreshStats 从物品重量重算）。
    /// 逻辑同 `InventoryState::refresh_weight`；用途同 `resize`。
    pub fn refresh_weight(&mut self) {
        self.weight = self
            .items
            .iter()
            .flatten()
            .map(|it| it.weight as u32 * it.count as u32)
            .sum();
    }
}

/// 本地自动喝药行为（非服务端态：开关 + 冷却计时；auto_potion_system 读写）。
/// 本批仅挂载默认值，写者迁移（auto_potion_system）属后续批次（设计 §5/§11）。
#[derive(Component, Clone, Copy, PartialEq)]
pub struct AutoPotion {
    pub enabled: bool,
    pub cooldown: f32,
}

impl Default for AutoPotion {
    fn default() -> Self {
        // 对齐 HudState.auto_pot_hp 默认 true
        Self {
            enabled: true,
            cooldown: 0.0,
        }
    }
}

/// 本地玩家状态组件包：生成 `LocalPlayer` 实体时一次性挂载全部默认值（设计 §7 挂载 A）。
/// 复用组件（PlayerName/NetObjectId/ActorAppearance/MountState）由既有 spawn/object_state
/// 路径维护，不在此包内。
#[derive(Bundle, Default)]
pub struct LocalPlayerStateBundle {
    pub vitals: Vitals,
    pub progression: Progression,
    pub gold: Gold,
    pub credit: Credit,
    pub base_stats: BaseStats,
    pub combat_stats: CombatStats,
    pub status_flags: StatusFlags,
    pub pet_mode: PetModeState,
    pub loadout: Loadout,
    pub inventory: Inventory,
    pub auto_potion: AutoPotion,
}
