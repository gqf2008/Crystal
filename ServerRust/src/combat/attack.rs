// 攻击计算（命中/伤害/暴击）
// 纯函数，由 WorldActor 调用

/// 计算基础伤害：在攻击力范围内随机取整，减去防御力
pub fn calculate_damage(attacker_atk_min: i32, attacker_atk_max: i32, defender_def: i32) -> i32 {
    let atk_max = attacker_atk_max.max(attacker_atk_min);
    let atk_min = attacker_atk_min.min(attacker_atk_max);
    let _atk_range = (atk_max - atk_min).max(1);

    // 使用简单线性同余生成伪随机（避免依赖额外 rng）
    let range = (atk_max - atk_min).max(1) as u64;
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let rand_offset = (seed.wrapping_mul(1103515245).wrapping_add(12345)) % range;
    let atk = atk_min + rand_offset as i32;

    atk.saturating_sub(defender_def.max(0)).max(1)
}

/// 计算暴击伤害（暴击时 x2）
pub fn calculate_critical(damage: i32, is_critical: bool) -> i32 {
    if is_critical {
        damage * 2
    } else {
        damage
    }
}

/// 暴击判定（10% 基础概率）
/// 注意：此函数是无状态的，调用者需确保 randomness 来自外部
pub fn check_critical_from_hash(hash: u64) -> bool {
    (hash % 100) < 10
}

/// 暴击判定（10% 基础概率）- 简单版本，不依赖外部随机
pub fn check_critical_simple() -> bool {
    // Phase 2: 固定 10% 暴击率，后续可装备修正
    // 使用时间戳的低字节作为伪随机种子
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64 % 100)
        .unwrap_or(0);
    seed < 10
}

/// 命中判定：简单基于攻方攻击等级和守方防御等级
pub fn check_hit(_attacker_accuracy: i32, _defender_dodge: i32) -> bool {
    // Phase 2: 默认命中，后续可加入命中率计算
    true
}

/// 攻击结果
pub struct AttackResult {
    pub damage: i32,
    pub is_critical: bool,
    pub is_hit: bool,
}

/// 完整攻击计算：命中 → 伤害 → 暴击
pub fn resolve_attack(attacker_atk_min: i32, attacker_atk_max: i32, defender_def: i32) -> AttackResult {
    let is_critical = check_critical_simple();
    let is_hit = check_hit(0, 0); // Phase 2: 默认命中

    let base_damage = calculate_damage(attacker_atk_min, attacker_atk_max, defender_def);
    let damage = calculate_critical(base_damage, is_critical);

    AttackResult {
        damage: if is_hit { damage } else { 0 },
        is_critical,
        is_hit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_damage_calculation() {
        // 攻击 5-10 vs 防御 2 → 至少 1 伤害
        let dmg = calculate_damage(5, 10, 2);
        assert!(dmg >= 1);
        assert!(dmg <= 8); // max atk 10 - def 2 = 8
    }

    #[test]
    fn test_damage_zero_defense() {
        let dmg = calculate_damage(10, 20, 0);
        assert!(dmg >= 10);
        assert!(dmg <= 20);
    }

    #[test]
    fn test_damage_high_defense() {
        // 防御 > 攻击 → 保底 1 伤害
        let dmg = calculate_damage(5, 10, 50);
        assert_eq!(dmg, 1);
    }

    #[test]
    fn test_critical_damage() {
        let base = 10;
        assert_eq!(calculate_critical(base, false), 10);
        assert_eq!(calculate_critical(base, true), 20);
    }

    #[test]
    fn test_resolve_attack_returns_result() {
        let result = resolve_attack(5, 10, 2);
        assert!(result.damage >= 1);
        assert!(result.is_hit); // Phase 2: 默认命中
    }
}
