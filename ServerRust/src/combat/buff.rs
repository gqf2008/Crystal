// Buff/Debuff 系统
// 纯函数 + 数据结构，由 WorldActor 调用

use serde::{Deserialize, Serialize};

/// 减伤 buff 来源（C# ProcessBuffs 过期 Down 特效：MagicShield/ElementalBarrier；Other 无特效）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShieldKind {
    MagicShield,
    ElementalBarrier,
    Other,
}

/// Buff 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuffType {
    /// HP 持续回复
    HpRegen { amount_per_tick: i32 },
    /// MP 持续回复
    MpRegen { amount_per_tick: i32 },
    /// 攻击力提升（英雄 `UltimateEnhancer` 按职业映射的通用攻击/魔法/道术加成；C# 图标 35「终极强化」）
    AttackBoost { bonus: i32 },
    // ===== #2892 批D 单元②：C# 把「药水攻击加成」与「战士怒气」分成两个 BuffType =====
    /// Buff 药水的攻击加成（C# `BuffType.Impact`，图标 249，`PlayerObject.cs:5854` / `HeroObject.cs:379`）
    Impact { bonus: i32 },
    /// 战士怒气（C# `BuffType.Rage`，图标 49，`HumanObject.cs:4970`：`MinDC/MaxDC += value`）
    Rage { bonus: i32 },
    /// 防御力提升（SoulShield/BlessedArmour）
    DefenseBoost { bonus: i32 },
    /// 物理防御提升（BlessedArmour，C# Stat.AC）
    AcDefenseBoost { bonus: i32 },
    /// 魔法防御提升（SoulShield，C# Stat.MAC）
    MacDefenseBoost { bonus: i32 },
    /// 负重上限提升（C# BuffType.BagWeight：Buff 药水 Stat.BagWeight）
    BagWeightBoost { bonus: i32 },
    /// 伤害百分比减免（MagicShield/ElementalBarrier，C# Stat.DamageReductionPercent）
    DamageReduction { percent: i32, kind: ShieldKind },
    /// 中毒（持续掉血）
    Poison { damage_per_tick: i32 },
    /// 沉默（无法使用技能）
    Silence,
    /// 眩晕（无法移动/攻击）
    Stun,
    // ===== 隐身三态（C# `BuffType` 分开的三个条目，图标/文案/可见性规则各不相同）=====
    /// 隐身（C# `BuffType.Hiding`，图标 17，对多数怪物隐形）
    Hiding,
    /// 月影隐身（C# `BuffType.MoonLight`，图标 65，远距离对玩家与怪物隐形）
    MoonLight,
    /// 暗身术（C# `BuffType.DarkBody`，图标 70，对多数怪物隐形且可移动）
    DarkBody,
    // ===== 刺客/弓箭手扩展（对齐 C# BuffType）=====
    /// 攻击速度提升（Haste，降低攻击冷却）
    AttackSpeedBoost { percent: i32 },
    /// 移动速度提升（SwiftFeet/LightBody，降低移动间隔）
    MoveSpeedBoost { percent: i32 },
    /// 敏捷提升（LightBody）
    AgilityBoost { bonus: i32 },
    /// 暴击率提升（**Rust 扩展**：C# 没有暴击率 Buff，脚本关键字 `CRITICALRATEBOOST` 用）
    CriticalRateBoost { bonus: i32 },
    /// 魔力恢复提升（Concentration）
    MpRegenBoost { bonus: i32 },
    /// 魔力上限提升（MagicBooster）
    MaxMpBoost { bonus: i32 },
    /// 生命上限提升（C# BuffType.HealthAid：Buff 药水 Stat.HP / NPC 脚本 MAXHPBOOST）
    MaxHpBoost { bonus: i32 },
    /// 魔法攻击提升（UltimateEnhancer 法师/弓手，C# Stat.MaxMC）
    McBoost { bonus: i32 },
    /// 道术提升（UltimateEnhancer 道士，C# Stat.MaxSC）
    ScBoost { bonus: i32 },
    /// 反伤（EnergyShield 概率回血用，此处简化为固定反伤）
    Reflect { percent: i32 },
    /// 嘲讽/吸引仇恨（LionRoar/BattleCry）
    Taunt,
    /// 减速（Slow poison 的 buff 表现）
    Slow { percent: i32 },
    /// 冰冻（Frozen poison 的 buff 表现，完全无法行动）
    Frozen,
    /// 变身（C# BuffType.Transform：使用 Transform 面具/卷轴，values=shape 客户端渲染变身外观）
    Transform { shape: i16 },
    /// 传送后魔法惩罚（C# BuffType.TemporalFlux：Teleport/Blink/StormEscape 后 30s，施法耗蓝 +30%）
    TeleportManaPenalty { percent: i32 },
    /// 诅咒（C# BuffType.Curse：MaxDC/MC/SC RatePercent 降低输出，玩家另 AttackSpeedRatePercent 降低攻速）
    Curse { percent: i32 },
    /// 犀牛祭司减益（C# BuffType.RhinoPriestDebuff：MaxDC/MC/SC 固定值降低，时长 5+damage 秒）
    RhinoPriestDebuff {
        max_dc: i32,
        max_mc: i32,
        max_sc: i32,
    },
}

/// Buff 实例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuffInstance {
    pub buff_type: BuffType,
    pub remaining_ticks: u32,
    /// 每多少 tick 触发一次效果
    pub tick_interval: u32,
    /// 内部计数器
    pub tick_counter: u32,
    /// 来源对象 ID（可选，用于区分来源）
    pub source_id: Option<u32>,
    /// 是否暂停（C# Buff.Paused：暂停期间不倒计时/不触发效果；@TOGGLETRANSFORM #2144）
    pub paused: bool,
}

impl BuffInstance {
    pub fn new(buff_type: BuffType, duration_ticks: u32, tick_interval: u32) -> Self {
        Self {
            buff_type,
            remaining_ticks: duration_ticks,
            tick_interval,
            tick_counter: 0,
            source_id: None,
            paused: false,
        }
    }

    pub fn with_source(mut self, source_id: u32) -> Self {
        self.source_id = Some(source_id);
        self
    }

    /// 转持久化形态：remaining_ticks × 世界 tick(100ms) → 墙钟到期（C# ExpireTime 语义）
    pub fn to_saved(&self, now_ms: i64) -> SavedBuff {
        SavedBuff {
            buff_type: self.buff_type,
            tick_interval: self.tick_interval,
            tick_counter: self.tick_counter,
            source_id: self.source_id,
            paused: self.paused,
            expire_at_ms: now_ms + (self.remaining_ticks as i64 * BUFF_TICK_MS as i64),
        }
    }
}

/// 世界 tick 时长（ms）：BuffInstance.remaining_ticks 按世界 tick 递减；生产配置 tick_ms=100
pub const BUFF_TICK_MS: u64 = 100;

/// Buff 持久化形态（对齐 C# CharacterInfo.Buffs：绝对到期时间，离线时间计入衰减）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedBuff {
    pub buff_type: BuffType,
    pub tick_interval: u32,
    pub tick_counter: u32,
    pub source_id: Option<u32>,
    pub paused: bool,
    /// 墙钟到期毫秒（C# Buff.ExpireTime）
    pub expire_at_ms: i64,
}

impl SavedBuff {
    /// 还原为 BuffInstance：按离线时长衰减 remaining_ticks；已过期返回 None
    pub fn into_instance(self, now_ms: i64) -> Option<BuffInstance> {
        let remaining_ms = self.expire_at_ms - now_ms;
        if remaining_ms <= 0 {
            return None;
        }
        Some(BuffInstance {
            buff_type: self.buff_type,
            remaining_ticks: (((remaining_ms as u64) / BUFF_TICK_MS).max(1)).min(u32::MAX as u64)
                as u32,
            tick_interval: self.tick_interval,
            tick_counter: self.tick_counter,
            source_id: self.source_id,
            paused: self.paused,
        })
    }
}

/// Buff 计时效果结果
#[derive(Debug, Clone)]
pub struct BuffTickResult {
    pub hp_change: i32,
    pub mp_change: i32,
    pub expired: bool,
}

/// 对所有 Buff 进行 tick 处理
pub fn tick_buffs(buffs: &mut [BuffInstance], _dt: u32) -> Vec<BuffTickResult> {
    let mut results = Vec::new();

    for buff in buffs.iter_mut() {
        if buff.remaining_ticks == 0 {
            continue;
        }
        // C# Buff.Process：Paused 期间不递减剩余时长、不触发效果
        if buff.paused {
            continue;
        }

        buff.tick_counter += 1;
        buff.remaining_ticks = buff.remaining_ticks.saturating_sub(1);

        let mut result = BuffTickResult {
            hp_change: 0,
            mp_change: 0,
            expired: buff.remaining_ticks == 0,
        };

        // 到达 tick 间隔时触发效果
        if buff.tick_counter >= buff.tick_interval {
            buff.tick_counter = 0;

            match &buff.buff_type {
                BuffType::HpRegen { amount_per_tick } => {
                    result.hp_change = *amount_per_tick;
                }
                BuffType::MpRegen { amount_per_tick } => {
                    result.mp_change = *amount_per_tick;
                }
                BuffType::Poison { damage_per_tick } => {
                    result.hp_change = -(*damage_per_tick);
                }
                _ => {}
            }
        }

        results.push(result);
    }

    results
}

/// 移除已过期的 Buff
pub fn expire_buffs(buffs: &mut Vec<BuffInstance>) {
    buffs.retain(|b| b.remaining_ticks > 0);
}

/// #2853：C# `HumanObject.Struck`（`HumanObject.cs:7355-7365`）/ `Attacked`（`:7163-7173`、`:7273-7283`）——
/// 受击时 MagicShield / ElementalBarrier 的剩余时长按 `(damage - armour) * 60ms` 缩短
/// （两者 `StackType` 均为 `ResetDuration`：`ExpireTime = 旧 ExpireTime - (damage-armour)*60`）。
///
/// 本端 `BuffInstance` 以 `BUFF_TICK_MS`（100ms）为粒度，折算口径取 **向下取整**
/// `(damage * 60) / 100`：不足 1 tick 的余量丢弃（C# 为毫秒精度）。
/// 调用方传入的 `damage` 已是净伤害（`armour >= damage` 的完全吸收分支不会走到这里），
/// 即 C# 的 `damage - armour`。返回被扣除的 tick 总数（供测试与调试）。
pub fn shrink_shield_buffs(buffs: &mut [BuffInstance], damage: i32) -> u32 {
    if damage <= 0 {
        return 0;
    }
    let ticks = ((damage as u64 * 60) / BUFF_TICK_MS).min(u32::MAX as u64) as u32;
    let mut applied = 0u32;
    for buff in buffs.iter_mut() {
        if !matches!(
            buff.buff_type,
            BuffType::DamageReduction {
                kind: ShieldKind::MagicShield | ShieldKind::ElementalBarrier,
                ..
            }
        ) {
            continue;
        }
        let before = buff.remaining_ticks;
        buff.remaining_ticks = buff.remaining_ticks.saturating_sub(ticks);
        applied += before - buff.remaining_ticks;
    }
    applied
}

/// 添加 Buff（同类型的新 Buff 替换旧的）
pub fn apply_buff(buffs: &mut Vec<BuffInstance>, new_buff: BuffInstance) {
    // 移除同类型的旧 Buff
    let buff_type_tag = std::mem::discriminant(&new_buff.buff_type);
    buffs.retain(|b| std::mem::discriminant(&b.buff_type) != buff_type_tag);
    buffs.push(new_buff);
}

/// 移除指定类型的 Buff
pub fn remove_buff_by_type(buffs: &mut Vec<BuffInstance>, buff_type: &BuffType) {
    let tag = std::mem::discriminant(buff_type);
    buffs.retain(|b| std::mem::discriminant(&b.buff_type) != tag);
}

/// 计算 Buff 对属性的加成（攻击力/防御力/敏捷/暴击等）
pub fn get_stat_bonus(buffs: &[BuffInstance], stat_type: &BuffType) -> i32 {
    buffs
        .iter()
        .filter(|b| std::mem::discriminant(&b.buff_type) == std::mem::discriminant(stat_type))
        .map(|b| match (&b.buff_type, stat_type) {
            (BuffType::AttackBoost { bonus }, BuffType::AttackBoost { .. }) => *bonus,
            (BuffType::Impact { bonus }, BuffType::Impact { .. }) => *bonus,
            (BuffType::Rage { bonus }, BuffType::Rage { .. }) => *bonus,
            (BuffType::DefenseBoost { bonus }, BuffType::DefenseBoost { .. }) => *bonus,
            (BuffType::AgilityBoost { bonus }, BuffType::AgilityBoost { .. }) => *bonus,
            (BuffType::CriticalRateBoost { bonus }, BuffType::CriticalRateBoost { .. }) => *bonus,
            (BuffType::MpRegenBoost { bonus }, BuffType::MpRegenBoost { .. }) => *bonus,
            (BuffType::MaxMpBoost { bonus }, BuffType::MaxMpBoost { .. }) => *bonus,
            (BuffType::MaxHpBoost { bonus }, BuffType::MaxHpBoost { .. }) => *bonus,
            (BuffType::McBoost { bonus }, BuffType::McBoost { .. }) => *bonus,
            (BuffType::ScBoost { bonus }, BuffType::ScBoost { .. }) => *bonus,
            (BuffType::AttackSpeedBoost { percent }, BuffType::AttackSpeedBoost { .. }) => *percent,
            (BuffType::MoveSpeedBoost { percent }, BuffType::MoveSpeedBoost { .. }) => *percent,
            (BuffType::Curse { percent }, BuffType::Curse { .. }) => *percent,
            (BuffType::Reflect { percent }, BuffType::Reflect { .. }) => *percent,
            (BuffType::AcDefenseBoost { bonus }, BuffType::AcDefenseBoost { .. }) => *bonus,
            (BuffType::MacDefenseBoost { bonus }, BuffType::MacDefenseBoost { .. }) => *bonus,
            (BuffType::BagWeightBoost { bonus }, BuffType::BagWeightBoost { .. }) => *bonus,
            _ => 0,
        })
        .sum()
}

/// RhinoPriestDebuff：取 MaxDC/MC/SC 固定减益之和（C# RhinoPriestDebuff，值均为负数）
pub fn get_rhino_priest_debuff(buffs: &[BuffInstance]) -> (i32, i32, i32) {
    buffs
        .iter()
        .fold((0, 0, 0), |(dc, mc, sc), b| match b.buff_type {
            BuffType::RhinoPriestDebuff {
                max_dc,
                max_mc,
                max_sc,
            } => (dc + max_dc, mc + max_mc, sc + max_sc),
            _ => (dc, mc, sc),
        })
}

/// #1906：是否 Debuff（对齐 C# BuffProperty.Debuff；PowerBead Effect==1 净化用）
pub fn is_debuff(buff_type: &BuffType) -> bool {
    match buff_type {
        BuffType::Curse { .. }
        | BuffType::RhinoPriestDebuff { .. }
        | BuffType::Slow { .. }
        | BuffType::Frozen
        | BuffType::Stun
        | BuffType::Silence
        | BuffType::TeleportManaPenalty { .. }
        | BuffType::Poison { .. } => true,
        // 负值增益（C# 用负 Stats 表达减益，如 PK 惩罚）也算 Debuff
        BuffType::AttackBoost { bonus }
        | BuffType::Impact { bonus }
        | BuffType::Rage { bonus }
        | BuffType::McBoost { bonus }
        | BuffType::ScBoost { bonus } => *bonus < 0,
        _ => false,
    }
}

/// 检查是否处于失控状态（Stun/Frozen 等，无法行动/攻击）
pub fn is_incacapacitated(buffs: &[BuffInstance]) -> bool {
    buffs
        .iter()
        .any(|b| matches!(b.buff_type, BuffType::Stun | BuffType::Frozen))
}

/// C# 的三种隐身 BuffType（Hiding / MoonLight / DarkBody）——三者可见性规则**不同**
/// （`MapObject.AddBuff`，`MapObject.cs:654-667`）：
/// - `Hiding`/`ClearRing` → 只置 `Hidden`（`S.ObjectHidden`）→ 他人看到 **50% 透明**
///   （C# 客户端 `MapObject.cs:5006` `SetOpacity(0.5F)`），且怪物不选中它；
/// - `MoonLight`/`DarkBody` → 额外置 `Sneaking`（→ `Observer` → `S.ObjectRemove`）→ 对他人**完全消失**。
/// 所以「对他人移除」用 [`is_sneaking_type`]，「半透明 / 怪物忽略」用本函数。
pub fn is_invisible_type(t: &BuffType) -> bool {
    matches!(
        t,
        BuffType::Hiding | BuffType::MoonLight | BuffType::DarkBody
    )
}

/// C# `Sneaking`（`MapObject.cs:112-131`：`Observer = true` → `S.ObjectRemove`）对应的隐身类型：
/// 只有 `MoonLight` 与 `DarkBody`（`Hiding`/`ClearRing` 仅半透明 + 不被怪物选中）。
pub fn is_sneaking_type(t: &BuffType) -> bool {
    matches!(t, BuffType::MoonLight | BuffType::DarkBody)
}

/// C# `MapObject.Hidden`（`Server/MirObjects/MapObject.cs:80-92` 属性 + `:654-667` `AddBuff`）：
/// `Hiding`/`MoonLight`/`DarkBody` 任一 buff，或头盔宝石 `SpecialItemMode.ClearRing 0x0004`
/// （`:501-504` 每秒补 `BuffType.ClearRing`）都会置 `Hidden = true` —— 半透明 + `HideFromTargets()`。
/// 与「对他人移除」的 [`is_sneaking_type`] **不是**同一档，勿混用。
pub fn has_hidden(buffs: &[BuffInstance], has_clear_ring: bool) -> bool {
    has_clear_ring || buffs.iter().any(|b| is_invisible_type(&b.buff_type))
}

/// C# `MapObject.Sneaking`（`MapObject.cs:126-131` → `SneakingActive` → `Observer` → `S.ObjectRemove`）：
/// 只有 `MoonLight`/`DarkBody` 置位（`Hiding`/`ClearRing` 不置，见 [`is_sneaking_type`]）。
pub fn has_sneaking(buffs: &[BuffInstance]) -> bool {
    buffs.iter().any(|b| is_sneaking_type(&b.buff_type))
}

/// 检查是否隐身
pub fn is_invisible(buffs: &[BuffInstance]) -> bool {
    buffs.iter().any(|b| is_invisible_type(&b.buff_type))
}

/// 检查是否被沉默（无法施法）
pub fn is_silenced(buffs: &[BuffInstance]) -> bool {
    buffs
        .iter()
        .any(|b| matches!(b.buff_type, BuffType::Silence))
}

/// 检查是否减速（影响移动间隔）
pub fn is_slowed(buffs: &[BuffInstance]) -> bool {
    buffs
        .iter()
        .any(|b| matches!(b.buff_type, BuffType::Slow { .. }))
}

#[cfg(test)]
mod tests {
    /// #2212：SavedBuff/BuffType serde roundtrip + 离线衰减/过期丢弃
    #[test]
    fn saved_buff_roundtrip_and_decay() {
        let buff = BuffInstance::new(BuffType::AttackBoost { bonus: 5 }, 300, 10).with_source(42);
        let now = 1_700_000_000_000i64;
        let saved = buff.to_saved(now);
        // 300 ticks * 100ms = 30s
        assert_eq!(saved.expire_at_ms, now + 30_000);

        let json = serde_json::to_string(&saved).unwrap();
        let back: SavedBuff = serde_json::from_str(&json).unwrap();
        assert_eq!(back.buff_type, BuffType::AttackBoost { bonus: 5 });
        assert_eq!(back.tick_interval, 10);
        assert_eq!(back.source_id, Some(42));

        // 立即还原：剩余 ticks 不变（30000/100=300）
        let restored = back.clone().into_instance(now).unwrap();
        assert_eq!(restored.remaining_ticks, 300);
        assert_eq!(restored.paused, false);

        // 离线 15s：剩余 150 ticks
        let half = back.clone().into_instance(now + 15_000).unwrap();
        assert_eq!(half.remaining_ticks, 150);

        // 离线超时：丢弃
        assert!(back.into_instance(now + 31_000).is_none());
    }
    use super::*;

    #[test]
    fn get_stat_bonus_includes_ac_mac_defense_boosts() {
        // C# buff Stats：AcDefenseBoost/MacDefenseBoost 应计入属性加成
        let buffs = vec![
            BuffInstance::new(BuffType::AcDefenseBoost { bonus: 7 }, 70, 5),
            BuffInstance::new(BuffType::MacDefenseBoost { bonus: 6 }, 70, 5),
        ];
        assert_eq!(
            get_stat_bonus(&buffs, &BuffType::AcDefenseBoost { bonus: 0 }),
            7
        );
        assert_eq!(
            get_stat_bonus(&buffs, &BuffType::MacDefenseBoost { bonus: 0 }),
            6
        );
        assert_eq!(
            get_stat_bonus(&buffs, &BuffType::DefenseBoost { bonus: 0 }),
            0
        );
    }

    #[test]
    fn get_stat_bonus_includes_max_hp_boost() {
        // C# BuffType.HealthAid：生命上限加成应计入
        let buffs = vec![BuffInstance::new(
            BuffType::MaxHpBoost { bonus: 30 },
            600,
            1,
        )];
        assert_eq!(
            get_stat_bonus(&buffs, &BuffType::MaxHpBoost { bonus: 0 }),
            30
        );
        assert_eq!(
            get_stat_bonus(&buffs, &BuffType::MaxMpBoost { bonus: 0 }),
            0
        );
    }

    #[test]
    fn get_stat_bonus_includes_bag_weight_boost() {
        // C# BuffType.BagWeight：负重上限加成应计入
        let buffs = vec![BuffInstance::new(
            BuffType::BagWeightBoost { bonus: 500 },
            600,
            1,
        )];
        assert_eq!(
            get_stat_bonus(&buffs, &BuffType::BagWeightBoost { bonus: 0 }),
            500
        );
        assert_eq!(
            get_stat_bonus(&buffs, &BuffType::DefenseBoost { bonus: 0 }),
            0
        );
    }

    /// #2853：受击按 `(damage-armour)*60ms` 缩短 MagicShield/ElementalBarrier 剩余时长
    /// （C# `HumanObject.cs:7355-7365`；本端 100ms tick 粒度 → 向下取整）
    #[test]
    fn shrink_shield_buffs_matches_csharp() {
        let mut buffs = vec![
            BuffInstance::new(
                BuffType::DamageReduction {
                    percent: 40,
                    kind: ShieldKind::MagicShield,
                },
                960, // 96 秒
                1,
            ),
            BuffInstance::new(
                BuffType::DamageReduction {
                    percent: 30,
                    kind: ShieldKind::ElementalBarrier,
                },
                200,
                1,
            ),
            // 非护盾 buff 不受影响（C# 只处理 MagicShield/ElementalBarrier）
            BuffInstance::new(BuffType::HpRegen { amount_per_tick: 5 }, 50, 1),
        ];

        // damage=100 → 100*60/100 = 60 tick（6 秒），两个护盾各扣 60
        assert_eq!(shrink_shield_buffs(&mut buffs, 100), 120);
        assert_eq!(buffs[0].remaining_ticks, 900);
        assert_eq!(buffs[1].remaining_ticks, 140);
        assert_eq!(buffs[2].remaining_ticks, 50);

        // damage=1 → 60ms < 1 tick → 不扣（向下取整）
        assert_eq!(shrink_shield_buffs(&mut buffs, 1), 0);
        assert_eq!(buffs[0].remaining_ticks, 900);

        // damage<=0 → 不动
        assert_eq!(shrink_shield_buffs(&mut buffs, 0), 0);
        assert_eq!(shrink_shield_buffs(&mut buffs, -5), 0);
        assert_eq!(buffs[0].remaining_ticks, 900);

        // 超大伤害不下溢（saturating）
        assert_eq!(shrink_shield_buffs(&mut buffs, i32::MAX), 900 + 140);
        assert_eq!(buffs[0].remaining_ticks, 0);
        assert_eq!(buffs[1].remaining_ticks, 0);

        // 无护盾 buff → 0
        let mut only_regen = vec![BuffInstance::new(
            BuffType::HpRegen { amount_per_tick: 5 },
            50,
            1,
        )];
        assert_eq!(shrink_shield_buffs(&mut only_regen, 100), 0);
        assert_eq!(only_regen[0].remaining_ticks, 50);
    }

    #[test]
    fn test_apply_and_expire_buff() {
        let mut buffs = Vec::new();
        let buff = BuffInstance::new(BuffType::HpRegen { amount_per_tick: 5 }, 3, 1);
        apply_buff(&mut buffs, buff);

        assert_eq!(buffs.len(), 1);
        assert_eq!(buffs[0].remaining_ticks, 3);

        // 经过 3 次 tick 后应该过期
        for _ in 0..3 {
            tick_buffs(&mut buffs, 1);
        }
        expire_buffs(&mut buffs);
        assert_eq!(buffs.len(), 0);
    }

    #[test]
    fn test_buff_tick_results() {
        let mut buffs = vec![
            BuffInstance::new(BuffType::HpRegen { amount_per_tick: 5 }, 5, 1),
            BuffInstance::new(BuffType::Poison { damage_per_tick: 3 }, 5, 1),
        ];

        let results = tick_buffs(&mut buffs, 1);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].hp_change, 5);
        assert_eq!(results[1].hp_change, -3);
    }

    #[test]
    fn test_buff_replacement() {
        let mut buffs = Vec::new();
        apply_buff(
            &mut buffs,
            BuffInstance::new(BuffType::HpRegen { amount_per_tick: 5 }, 3, 1),
        );
        // 添加新的同类型 Buff 应该替换旧的
        apply_buff(
            &mut buffs,
            BuffInstance::new(
                BuffType::HpRegen {
                    amount_per_tick: 10,
                },
                5,
                1,
            ),
        );

        assert_eq!(buffs.len(), 1);
        assert_eq!(buffs[0].remaining_ticks, 5);
        match &buffs[0].buff_type {
            BuffType::HpRegen { amount_per_tick } => assert_eq!(*amount_per_tick, 10),
            other => assert!(false, "expected HpRegen, got {other:?}"),
        }
    }

    #[test]
    fn test_stat_bonus() {
        let buffs = vec![
            BuffInstance::new(BuffType::AttackBoost { bonus: 10 }, 5, 1),
            BuffInstance::new(BuffType::DefenseBoost { bonus: 5 }, 5, 1),
        ];

        let atk_bonus = get_stat_bonus(&buffs, &BuffType::AttackBoost { bonus: 0 });
        assert_eq!(atk_bonus, 10);

        let def_bonus = get_stat_bonus(&buffs, &BuffType::DefenseBoost { bonus: 0 });
        assert_eq!(def_bonus, 5);
    }

    #[test]
    fn test_paused_buff_does_not_tick() {
        // C# PauseBuff：暂停期间剩余时长冻结、不触发效果（@TOGGLETRANSFORM #2144）
        let mut buffs = vec![BuffInstance::new(
            BuffType::HpRegen { amount_per_tick: 5 },
            3,
            1,
        )];
        buffs[0].paused = true;
        let results = tick_buffs(&mut buffs, 1);
        assert!(results.is_empty());
        assert_eq!(buffs[0].remaining_ticks, 3);
        // 恢复后继续倒计时
        buffs[0].paused = false;
        let results2 = tick_buffs(&mut buffs, 1);
        assert_eq!(results2.len(), 1);
        assert_eq!(results2[0].hp_change, 5);
        assert_eq!(buffs[0].remaining_ticks, 2);
    }

    /// #2892：三种隐身的**可见性分档**（C# `MapObject.AddBuff`，`MapObject.cs:654-667`）——
    /// `Hiding`/`ClearRing` 只置 `Hidden`（半透明 + 怪物忽略）；`MoonLight`/`DarkBody` 额外置
    /// `Sneaking`（`Observer` → `S.ObjectRemove`，对他人完全消失）。
    ///
    /// 阳性对照：把 `is_sneaking_type` 改成与 `is_invisible_type` 相同（三者都算 sneaking，
    /// 即修正前本端的合并行为）→ 本测试的 `!is_sneaking_type(Hiding)` 断言 FAILED。
    #[test]
    fn invisibility_levels_match_csharp() {
        use crate::combat::buff::BuffType;
        assert!(is_invisible_type(&BuffType::Hiding));
        assert!(
            !is_sneaking_type(&BuffType::Hiding),
            "C# `Hiding` 只置 Hidden（半透明），不置 Sneaking"
        );
        for t in [BuffType::MoonLight, BuffType::DarkBody] {
            assert!(is_invisible_type(&t), "{t:?} 也应算隐身");
            assert!(is_sneaking_type(&t), "{t:?} 应置 Sneaking（对他人移除）");
        }
        assert!(!is_sneaking_type(&BuffType::Stun));
        assert!(!is_sneaking_type(&BuffType::AttackBoost { bonus: 0 }));
    }

    /// #2892：`Hidden` 与 `Sneaking` 是**两档**（C# `Server/MirObjects/MapObject.cs:654-667`）——
    /// `Hiding` 只置 `Hidden`；`ClearRing`（无 buff，来自装备宝石）也只置 `Hidden`；
    /// `MoonLight`/`DarkBody` 两档都置。
    ///
    /// 阳性对照（2026-09-15 实测）：
    /// ① 把 `has_hidden` 改成只看 `has_clear_ring`（忽略 buff）→ `has_hidden(&[hiding], false)` 断言 FAILED；
    /// ② 把 `has_sneaking` 改成 `is_invisible_type` → `!has_sneaking(&[hiding])` 断言 FAILED。
    #[test]
    fn hidden_and_sneaking_are_separate_tiers() {
        let mk = |t: BuffType| BuffInstance::new(t, 10, 1);
        let hiding = mk(BuffType::Hiding);
        let moon = mk(BuffType::MoonLight);
        let dark = mk(BuffType::DarkBody);

        assert!(
            has_hidden(std::slice::from_ref(&hiding), false),
            "Hiding 置 Hidden（半透明）"
        );
        assert!(
            !has_sneaking(std::slice::from_ref(&hiding)),
            "Hiding 不置 Sneaking（不 ObjectRemove）"
        );
        assert!(has_hidden(&[], true), "ClearRing 宝石置 Hidden");
        assert!(!has_sneaking(&[]), "ClearRing 不置 Sneaking");
        for b in [moon, dark] {
            assert!(has_hidden(std::slice::from_ref(&b), false));
            assert!(has_sneaking(std::slice::from_ref(&b)));
        }
        assert!(!has_hidden(&[], false), "无 buff 无宝石 → 不 Hidden");
        assert!(!has_sneaking(&[]), "无 buff → 不 Sneaking");
    }
}
