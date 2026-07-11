// Poison / 负面状态系统
//
// 对齐 C# `Shared/Enums.cs` PoisonType（`[Flags] : ushort`）与
// `Server/MirObjects/MapObject.cs` ApplyPoison 的语义。
//
// PoisonType 为位掩码（可组合），但单条 Poison 通常只持一种 PType；
// ApplyNegativeEffects（attack.rs）会按攻击者的 Stats 触发 Green/Slow/Paralysis。

use mir2_shared::enums::PoisonType;

/// 一条中毒/负面状态实例
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Poison {
    pub p_type: PoisonType,
    /// 持续时间（秒）
    pub duration_s: u32,
    /// 每 tick 的数值（伤害量 / 或 0 表示无数值）
    pub value: i32,
    /// tick 间隔（毫秒）
    pub tick_ms: u64,
}

impl Poison {
    pub fn new(p_type: PoisonType, duration_s: u32, value: i32, tick_ms: u64) -> Self {
        Self { p_type, duration_s, value, tick_ms }
    }

    /// 是否为"完全失控"状态（无法移动/攻击）
    pub fn is_incapacitating(&self) -> bool {
        self.p_type.intersects(
            PoisonType::STUN | PoisonType::PARALYSIS | PoisonType::FROZEN | PoisonType::DAZED,
        )
    }

    /// 是否为"减速"状态（影响移动间隔）
    pub fn is_slowing(&self) -> bool {
        self.p_type.intersects(PoisonType::SLOW | PoisonType::FROZEN)
    }
}

/// 把一条 Poison 加入目标的 poison_list。
///
/// C# MapObject.ApplyPoison 的简化版：同一 PType 的旧毒会被覆盖
/// （C# 的堆叠/刷新语义由 BuffStackType 在 buff 系统处理；Poison 这里
/// 采用"同类型替换"以避免无限堆积）。
pub fn apply_poison(poison_list: &mut Vec<Poison>, p: Poison) {
    if let Some(existing) = poison_list.iter_mut().find(|x| x.p_type == p.p_type) {
        *existing = p;
    } else {
        poison_list.push(p);
    }
}

/// 清除指定类型的 Poison
pub fn remove_poison(poison_list: &mut Vec<Poison>, p_type: PoisonType) {
    poison_list.retain(|p| p.p_type != p_type);
}

/// 检查是否处于某种失控状态（无法行动/攻击）
pub fn is_incacapacitated(poison_list: &[Poison]) -> bool {
    poison_list.iter().any(|p| p.is_incapacitating())
}

/// 检查是否处于减速状态
pub fn is_slowed(poison_list: &[Poison]) -> bool {
    poison_list.iter().any(|p| p.is_slowing())
}

/// tick 推进：duration 递减，返回 (应扣血量, 是否有过期项被移除)
///
/// `dt_s` 为本次 tick 推进的秒数。Green/Red/Bleeding 按 value 掉血；
/// 其他类型只维持状态不造成数值。归零的 Poison 自动移除。
pub fn tick_poisons(poison_list: &mut Vec<Poison>, dt_s: u32) -> i32 {
    let mut total_damage = 0i32;
    // 先推进 duration 并累计伤害
    for p in poison_list.iter_mut() {
        p.duration_s = p.duration_s.saturating_sub(dt_s);
        if p.p_type.intersects(PoisonType::GREEN | PoisonType::RED | PoisonType::BLEEDING) {
            // value 是每秒伤害量（对齐 C# Poison.Value 的掉血语义）
            total_damage = total_damage.saturating_add(p.value.max(0) * dt_s as i32);
        }
    }
    // 移除过期
    poison_list.retain(|p| p.duration_s > 0);
    total_damage
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_and_replace_poison() {
        let mut list = Vec::new();
        apply_poison(&mut list, Poison::new(PoisonType::GREEN, 5, 3, 1000));
        assert_eq!(list.len(), 1);
        // 同类型替换
        apply_poison(&mut list, Poison::new(PoisonType::GREEN, 10, 8, 1000));
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].duration_s, 10);
        assert_eq!(list[0].value, 8);
        // 不同类型追加
        apply_poison(&mut list, Poison::new(PoisonType::SLOW, 4, 0, 1000));
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_tick_green_damage_and_expiry() {
        let mut list = vec![Poison::new(PoisonType::GREEN, 5, 3, 1000)];
        // 推进 2 秒：伤害 3*2=6，剩余 3 秒
        let dmg = tick_poisons(&mut list, 2);
        assert_eq!(dmg, 6);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].duration_s, 3);
        // 再推进 3 秒：过期移除
        let dmg = tick_poisons(&mut list, 3);
        assert_eq!(dmg, 9);
        assert!(list.is_empty());
    }

    #[test]
    fn test_incacapacitated_check() {
        let list = vec![Poison::new(PoisonType::STUN, 3, 0, 1000)];
        assert!(is_incacapacitated(&list));
        let list = vec![Poison::new(PoisonType::GREEN, 3, 5, 1000)];
        assert!(!is_incacapacitated(&list));
    }
}
