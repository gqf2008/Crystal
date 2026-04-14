// Buff/Debuff 系统
// 纯函数 + 数据结构，由 WorldActor 调用

/// Buff 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuffType {
    /// HP 持续回复
    HpRegen { amount_per_tick: i32 },
    /// MP 持续回复
    MpRegen { amount_per_tick: i32 },
    /// 攻击力提升
    AttackBoost { bonus: i32 },
    /// 防御力提升
    DefenseBoost { bonus: i32 },
    /// 中毒（持续掉血）
    Poison { damage_per_tick: i32 },
    /// 沉默（无法使用技能）
    Silence,
    /// 眩晕（无法移动）
    Stun,
    /// 隐身
    Invisibility,
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
pub fn tick_buffs(buffs: &mut Vec<BuffInstance>, _dt: u32) -> Vec<BuffTickResult> {
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

/// 计算 Buff 对属性的加成（攻击力/防御力等）
pub fn get_stat_bonus(buffs: &[BuffInstance], stat_type: &BuffType) -> i32 {
    buffs
        .iter()
        .filter(|b| std::mem::discriminant(&b.buff_type) == std::mem::discriminant(stat_type))
        .map(|b| match (&b.buff_type, stat_type) {
            (BuffType::AttackBoost { bonus }, BuffType::AttackBoost { .. }) => *bonus,
            (BuffType::DefenseBoost { bonus }, BuffType::DefenseBoost { .. }) => *bonus,
            _ => 0,
        })
        .sum()
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
