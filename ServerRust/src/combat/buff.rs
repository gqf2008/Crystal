// Buff/Debuff 系统
// 纯函数 + 数据结构，由 WorldActor 调用

/// Buff 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuffType {
    /// HP 持续回复
    HpRegen { amount_per_tick: i32 },
    /// MP 持续回复
    MpRegen { amount_per_tick: i32 },
    /// 攻击力提升（Fury/Rage 类）
    AttackBoost { bonus: i32 },
    /// 防御力提升（SoulShield/BlessedArmour）
    DefenseBoost { bonus: i32 },
    /// 物理防御提升（BlessedArmour，C# Stat.AC）
    AcDefenseBoost { bonus: i32 },
    /// 魔法防御提升（SoulShield，C# Stat.MAC）
    MacDefenseBoost { bonus: i32 },
    /// 伤害百分比减免（MagicShield/ElementalBarrier，C# Stat.DamageReductionPercent）
    DamageReduction { percent: i32 },
    /// 中毒（持续掉血）
    Poison { damage_per_tick: i32 },
    /// 沉默（无法使用技能）
    Silence,
    /// 眩晕（无法移动/攻击）
    Stun,
    /// 隐身（Hiding/MassHiding/MoonLight/DarkBody）
    Invisibility,
    // ===== 刺客/弓箭手扩展（对齐 C# BuffType）=====
    /// 攻击速度提升（Haste，降低攻击冷却）
    AttackSpeedBoost { percent: i32 },
    /// 移动速度提升（SwiftFeet/LightBody，降低移动间隔）
    MoveSpeedBoost { percent: i32 },
    /// 敏捷提升（LightBody）
    AgilityBoost { bonus: i32 },
    /// 暴击率提升（Rage）
    CriticalRateBoost { bonus: i32 },
    /// 魔力恢复提升（Concentration）
    MpRegenBoost { bonus: i32 },
    /// 魔力上限提升（MagicBooster）
    MaxMpBoost { bonus: i32 },
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
}

/// Buff 实例
#[derive(Debug, Clone)]
pub struct BuffInstance {
    pub buff_type: BuffType,
    pub remaining_ticks: u32,
    /// 每多少 tick 触发一次效果
    pub tick_interval: u32,
    /// 内部计数器
    pub tick_counter: u32,
    /// 来源对象 ID（可选，用于区分来源）
    pub source_id: Option<u32>,
}

impl BuffInstance {
    pub fn new(buff_type: BuffType, duration_ticks: u32, tick_interval: u32) -> Self {
        Self {
            buff_type,
            remaining_ticks: duration_ticks,
            tick_interval,
            tick_counter: 0,
            source_id: None,
        }
    }

    pub fn with_source(mut self, source_id: u32) -> Self {
        self.source_id = Some(source_id);
        self
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
            (BuffType::DefenseBoost { bonus }, BuffType::DefenseBoost { .. }) => *bonus,
            (BuffType::AgilityBoost { bonus }, BuffType::AgilityBoost { .. }) => *bonus,
            (BuffType::CriticalRateBoost { bonus }, BuffType::CriticalRateBoost { .. }) => *bonus,
            (BuffType::MpRegenBoost { bonus }, BuffType::MpRegenBoost { .. }) => *bonus,
            (BuffType::MaxMpBoost { bonus }, BuffType::MaxMpBoost { .. }) => *bonus,
            (BuffType::McBoost { bonus }, BuffType::McBoost { .. }) => *bonus,
            (BuffType::ScBoost { bonus }, BuffType::ScBoost { .. }) => *bonus,
            (BuffType::AttackSpeedBoost { percent }, BuffType::AttackSpeedBoost { .. }) => *percent,
            (BuffType::MoveSpeedBoost { percent }, BuffType::MoveSpeedBoost { .. }) => *percent,
            (BuffType::Reflect { percent }, BuffType::Reflect { .. }) => *percent,
            _ => 0,
        })
        .sum()
}

/// 检查是否处于失控状态（Stun/Frozen 等，无法行动/攻击）
pub fn is_incacapacitated(buffs: &[BuffInstance]) -> bool {
    buffs.iter().any(|b| matches!(b.buff_type, BuffType::Stun | BuffType::Frozen))
}

/// 检查是否隐身
pub fn is_invisible(buffs: &[BuffInstance]) -> bool {
    buffs.iter().any(|b| matches!(b.buff_type, BuffType::Invisibility))
}

/// 检查是否被沉默（无法施法）
pub fn is_silenced(buffs: &[BuffInstance]) -> bool {
    buffs.iter().any(|b| matches!(b.buff_type, BuffType::Silence))
}

/// 检查是否减速（影响移动间隔）
pub fn is_slowed(buffs: &[BuffInstance]) -> bool {
    buffs.iter().any(|b| matches!(b.buff_type, BuffType::Slow { .. }))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        apply_buff(&mut buffs, BuffInstance::new(BuffType::HpRegen { amount_per_tick: 5 }, 3, 1));
        // 添加新的同类型 Buff 应该替换旧的
        apply_buff(&mut buffs, BuffInstance::new(BuffType::HpRegen { amount_per_tick: 10 }, 5, 1));

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
}
