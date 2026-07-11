// 战斗计算（命中/伤害/暴击/防御/反伤/减伤/吸血/负面效果）
//
// 对齐 C# 权威实现：
// - `Server/MirObjects/MapObject.cs` GetArmour / GetAttackPower / GetDefencePower / ApplyNegativeEffects
// - `Server/MirObjects/HumanObject.cs` Attacked（13 步全流程）
// - `Server/MirObjects/MonsterObject.cs` 暴击
// - `Server/Settings.cs` 权重常量
//
// 全部为纯函数，由 WorldActor 调用。随机性来自 fastrand（已在线程局部播种）。

use mir2_shared::enums::{DefenceType, PoisonType};
use crate::combat::poison::Poison;

// ============================================================
// Settings 权重常量（对齐 C# Server/Settings.cs:296-304）
// ============================================================
pub const MAGIC_RESIST_WEIGHT: i32 = 10;
pub const POISON_RESIST_WEIGHT: i32 = 10;
pub const CRITICAL_RATE_WEIGHT: i32 = 5;
pub const CRITICAL_DAMAGE_WEIGHT: i32 = 50;
pub const FREEZING_ATTACK_WEIGHT: i32 = 10;
pub const POISON_ATTACK_WEIGHT: i32 = 10;
pub const MAX_LUCK: i32 = 10;

// ============================================================
// CombatStats：从 PlayerState / MonsterState 提取的战斗属性快照
// ============================================================
#[derive(Debug, Clone, Copy, Default)]
pub struct CombatStats {
    pub min_atk: i32,
    pub max_atk: i32,
    pub min_ac: i32,
    pub max_ac: i32,
    pub min_mac: i32,
    pub max_mac: i32,
    pub agility: i32,
    pub accuracy: i32,
    pub luck: i32,
    pub critical_rate: i32,
    pub critical_damage: i32,
    pub magic_resist: i32,
    pub reflect: i32,
    pub damage_reduction_percent: i32,
    pub attack_bonus: i32,
    pub hp_drain_rate_percent: i32,
    pub energy_shield_percent: i32,
    pub energy_shield_hp_gain: i32,
    /// 护甲倍率（C# ArmourRate，默认 1.0；怪物个体可调）
    pub armour_rate: f32,
    /// 伤害倍率（C# DamageRate，默认 1.0）
    pub damage_rate: f32,
    /// 冰冻攻击（ApplyNegativeEffects 用）
    pub freezing: i32,
    /// 毒物攻击（ApplyNegativeEffects 用）
    pub poison_attack: i32,
}

// ============================================================
// 基础随机辅助（fastrand，i32 范围）
// ============================================================

/// 返回 [0, n) 的随机整数。n <= 0 时返回 0（避免 panic，对齐 C# 边界）。
#[inline]
fn rand_below(n: i32) -> i32 {
    if n <= 0 {
        return 0;
    }
    fastrand::i32(0..n)
}

/// 返回 [min, max] 闭区间的随机整数（对齐 C# Envir.Random.Next(min, max+1)）。
#[inline]
fn rand_in_range(min: i32, max: i32) -> i32 {
    if min >= max {
        return min;
    }
    fastrand::i32(min..=max)
}

// ============================================================
// 攻击力随机 GetAttackPower（MapObject.cs:243）
// ============================================================

/// 幸运/诅咒判定。
/// - Luck > 0：概率 Luck/MaxLuck 命中 max
/// - Luck < 0：概率 |Luck|/MaxLuck 命中 min
/// - 否则：[min, max] 随机
pub fn get_attack_power(min: i32, max: i32, luck: i32) -> i32 {
    let min = min.max(0);
    let max = if min > max { min } else { max };

    if luck > 0 {
        if luck > rand_below(MAX_LUCK) {
            return max;
        }
    } else if luck < 0 {
        if luck < -rand_below(MAX_LUCK) {
            return min;
        }
    }
    rand_in_range(min, max)
}

// ============================================================
// 护甲随机 GetDefencePower（MapObject.cs:274）
// ============================================================

/// 护甲在 [min, max] 间随机（无幸运判定）。
pub fn get_defence_power(min: i32, max: i32) -> i32 {
    let min = min.max(0);
    let max = if min > max { min } else { max };
    rand_in_range(min, max)
}

// ============================================================
// 命中+护甲判定 GetArmour（MapObject.cs:460）
// ============================================================

/// 返回 (armour, hit)。对齐 C# DefenceType 七态。
pub fn get_armour(defender: &CombatStats, defence_type: DefenceType, attacker_accuracy: i32) -> (i32, bool) {
    let mut armour = 0i32;
    let mut hit = true;

    match defence_type {
        DefenceType::AcAgility => {
            // Agility 闪避
            if rand_below(defender.agility + 1) > attacker_accuracy {
                hit = false;
            }
            armour = get_defence_power(defender.min_ac, defender.max_ac);
        }
        DefenceType::Ac => {
            armour = get_defence_power(defender.min_ac, defender.max_ac);
        }
        DefenceType::MacAgility => {
            // MagicResist 抵抗 + Agility 闪避
            if rand_below(MAGIC_RESIST_WEIGHT) < defender.magic_resist {
                hit = false;
            }
            if rand_below(defender.agility + 1) > attacker_accuracy {
                hit = false;
            }
            armour = get_defence_power(defender.min_mac, defender.max_mac);
        }
        DefenceType::Mac => {
            if rand_below(MAGIC_RESIST_WEIGHT) < defender.magic_resist {
                hit = false;
            }
            armour = get_defence_power(defender.min_mac, defender.max_mac);
        }
        DefenceType::Agility => {
            if rand_below(defender.agility + 1) > attacker_accuracy {
                hit = false;
            }
            // 无护甲
        }
        DefenceType::Repulsion | DefenceType::None => {
            // 必中，无护甲（C# switch 无此 case，fall-through default）
        }
    }
    (armour, hit)
}

// ============================================================
// 暴击（HumanObject.cs:7156 / MonsterObject.cs:2594）
// ============================================================

/// 暴击判定：Random(100) < CriticalRate * CriticalRateWeight
pub fn check_critical(critical_rate: i32) -> bool {
    rand_below(100) < critical_rate.saturating_mul(CRITICAL_RATE_WEIGHT)
}

/// 暴击伤害加成：damage += floor(damage * (CriticalDamage / CriticalDamageWeight) * 10)
/// 即 damage *= (1 + CriticalDamage / 5)（CriticalDamageWeight=50）
pub fn apply_critical(damage: i32, critical_damage: i32) -> i32 {
    // i64 防溢出，对齐 C# Math.Min(int.MaxValue, ...)
    let bonus = (damage as i64 * critical_damage as i64 * 10 / CRITICAL_DAMAGE_WEIGHT as i64) / 1;
    let bonus = (bonus as f64).floor() as i64;
    let total = damage as i64 + bonus;
    total.min(i32::MAX as i64) as i32
}

// ============================================================
// ApplyNegativeEffects（MapObject.cs:509）
// ============================================================

/// 触发攻击者的负面效果（麻痹/冰冻→减速/毒攻击→绿毒）。
/// 仅物理系攻击触发（type 非 MAC/MACAgility）。返回应施加的 Poison 列表。
///
/// 注意：C# 此处依赖 `SpecialMode.Paralize`（特殊装备戒指），Rust 暂未实现 SpecialMode，
/// 因此 Paralize 分支暂不触发（返回空）；Freezing/PoisonAttack 按 Stats 触发。
pub fn apply_negative_effects(
    attacker: &CombatStats,
    defence_type: DefenceType,
    level_offset: u16,
) -> Vec<Poison> {
    let mut poisons = Vec::new();
    // 魔法攻击不触发物理系负面
    if matches!(defence_type, DefenceType::Mac | DefenceType::MacAgility) {
        return poisons;
    }

    // Paralize：需 SpecialMode，暂跳过（TODO: 接入 SpecialMode 后补）

    // Freezing → Slow
    if attacker.freezing > 0 {
        if rand_below(FREEZING_ATTACK_WEIGHT) < attacker.freezing && rand_below(level_offset as i32) == 0 {
            let duration = (3 + rand_below(attacker.freezing)).min(10) as u32;
            poisons.push(Poison::new(PoisonType::SLOW, duration, 0, 1000));
        }
    }

    // PoisonAttack → Green
    if attacker.poison_attack > 0 {
        if rand_below(POISON_ATTACK_WEIGHT) < attacker.poison_attack && rand_below(level_offset as i32) == 0 {
            let value = (3 + rand_below(attacker.poison_attack)).min(10);
            poisons.push(Poison::new(PoisonType::GREEN, 5, value, 1000));
        }
    }

    poisons
}

// ============================================================
// AttackResult + 完整 Attacked 流程
// ============================================================

#[derive(Debug, Clone)]
pub struct AttackResult {
    /// 实际造成的伤害（已减护甲，>= 0）
    pub damage: i32,
    /// 是否命中（未闪避）
    pub is_hit: bool,
    /// 是否暴击
    pub is_critical: bool,
    /// 反弹给攻击者的伤害（>0 表示触发 Reflect，此时 defender 不掉血）
    pub reflected: i32,
    /// 攻击者吸血量（>0 表示攻击者应回血此值）
    pub hp_drain: i32,
    /// 应施加给防御者的 Poison 列表
    pub applied_poisons: Vec<Poison>,
}

/// 完整攻击结算，对齐 C# HumanObject.Attacked 的 13 步流程。
///
/// 参数：
/// - `attacker`：攻击者属性快照
/// - `defender`：防御者属性快照
/// - `raw_damage`：攻击者本次攻击的原始伤害（调用方先用 get_attack_power 计算）
/// - `defence_type`：本次攻击的防御类型
/// - `level_offset`：等级差（0-10），影响负面效果触发
pub fn resolve_attack(
    attacker: &CombatStats,
    defender: &CombatStats,
    raw_damage: i32,
    defence_type: DefenceType,
    level_offset: u16,
) -> AttackResult {
    // [1] GetArmour 命中 + 护甲
    let (armour, hit) = get_armour(defender, defence_type, attacker.accuracy);
    if !hit {
        return AttackResult {
            damage: 0,
            is_hit: false,
            is_critical: false,
            reflected: 0,
            hp_drain: 0,
            applied_poisons: Vec::new(),
        };
    }

    // [2] ArmourRate / DamageRate 倍率（clamp i32，对齐 C# decimal 钳位）
    let armour = clamp_i32((armour as f32 * defender.armour_rate) as i64);
    let mut damage = clamp_i32((raw_damage as f32 * defender.damage_rate) as i64);

    // [3] AttackBonus
    damage = clamp_i32(damage as i64 + attacker.attack_bonus as i64);

    // [4] Reflect（命中则反弹全额，defender 不掉血）
    if rand_below(100) < defender.reflect {
        return AttackResult {
            damage: 0,
            is_hit: true,
            is_critical: false,
            reflected: damage,
            hp_drain: 0,
            applied_poisons: Vec::new(),
        };
    }

    // [5] DamageReductionPercent（MagicShield / ElementalBarrier）
    if defender.damage_reduction_percent > 0 {
        damage = clamp_i32(damage as i64 - (damage as i64 * defender.damage_reduction_percent as i64) / 100);
    }

    // [6] armour >= damage → miss（护甲完全抵消）
    if armour >= damage {
        return AttackResult {
            damage: 0,
            is_hit: true, // 命中了但被护甲完全挡下（C# 此处广播 Miss）
            is_critical: false,
            reflected: 0,
            hp_drain: 0,
            applied_poisons: Vec::new(),
        };
    }

    // [7] 破隐：由调用方处理（RemoveBuff MoonLight/DarkBody），此处不涉及

    // [8] EnergyShield：概率回血（defender 自身回血，不影响 damage）
    // 注意：EnergyShield 的回血由调用方根据返回值应用；此处只在 result 标注
    // 为简化，EnergyShield 回血逻辑留给调用方读取 defender stats 自行处理
    // （C# 在 Attacked 内直接 ChangeHP，Rust 端把 HP 变更权交给 PlayerActor）

    // [9] 暴击
    let mut is_critical = false;
    if check_critical(attacker.critical_rate) {
        damage = apply_critical(damage, attacker.critical_damage);
        is_critical = true;
    }

    // [10] MagicShield/ElementalBarrier 时长扣减：由调用方处理（需访问 buff 列表）
    // 此处公式层不持有 buff，留给调用方

    // [11] HPDrainRatePercent：攻击者吸血累积
    let mut hp_drain = 0i32;
    if attacker.hp_drain_rate_percent > 0 {
        let net = (damage - armour).max(0);
        // C# HpDrain += ((net / 100) * HPDrainRatePercent)，>2 时才回血
        // 此处直接返回本次应回血量（调用方累积 >2 后回血）
        hp_drain = ((net as f32 / 100.0) * attacker.hp_drain_rate_percent as f32) as i32;
    }

    // [12] ApplyNegativeEffects
    let applied_poisons = apply_negative_effects(attacker, defence_type, level_offset);

    // [13] ChangeHP(armour - damage)：返回实际伤害 = damage - armour
    let actual_damage = (damage - armour).max(0);

    AttackResult {
        damage: actual_damage,
        is_hit: true,
        is_critical,
        reflected: 0,
        hp_drain,
        applied_poisons,
    }
}

/// i32 钳位（对齐 C# Math.Max(int.MinValue, Math.Min(int.MaxValue, (decimal)x))）
#[inline]
fn clamp_i32(v: i64) -> i32 {
    v.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

// ============================================================
// 从 PlayerState / MonsterState 构建 CombatStats 的辅助
// （定义在各 actor 模块更自然，但放在这里集中管理公式侧转换）
// ============================================================

impl CombatStats {
    /// 全零默认（用于无属性目标，如某些环境物体）
    pub fn zeroed() -> Self {
        Self::default()
    }
}

// ============================================================
// 兼容旧 API：保留 calculate_damage 供未迁移的调用点使用
// （任务 5 会把 4 个 resolve_attack 调用点全部迁移到新签名）
// ============================================================

/// 旧版基础伤害计算（已废弃，保留过渡）
#[deprecated(note = "使用 get_attack_power + resolve_attack 替代")]
pub fn calculate_damage(attacker_atk_min: i32, attacker_atk_max: i32, defender_def: i32) -> i32 {
    let atk = get_attack_power(attacker_atk_min, attacker_atk_max, 0);
    atk.saturating_sub(defender_def.max(0)).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_attacker() -> CombatStats {
        CombatStats {
            min_atk: 10,
            max_atk: 20,
            accuracy: 10,
            armour_rate: 1.0,
            damage_rate: 1.0,
            ..Default::default()
        }
    }

    fn dummy_defender() -> CombatStats {
        CombatStats {
            min_ac: 2,
            max_ac: 5,
            agility: 5,
            armour_rate: 1.0,
            damage_rate: 1.0,
            ..Default::default()
        }
    }

    #[test]
    fn test_get_attack_power_in_range() {
        for _ in 0..100 {
            let v = get_attack_power(10, 20, 0);
            assert!((10..=20).contains(&v));
        }
    }

    #[test]
    fn test_get_attack_power_lucky_hits_max() {
        // Luck=10 应几乎总是命中 max（10 > rand(0,10) 概率 100%）
        let mut hit_max = 0;
        for _ in 0..100 {
            if get_attack_power(10, 20, 10) == 20 {
                hit_max += 1;
            }
        }
        assert!(hit_max >= 95, "expected lucky to hit max ~always, got {}/100", hit_max);
    }

    #[test]
    fn test_get_defence_power_in_range() {
        for _ in 0..100 {
            let v = get_defence_power(2, 5);
            assert!((2..=5).contains(&v));
        }
    }

    #[test]
    fn test_get_armour_ac_type_always_hits() {
        let defender = dummy_defender();
        let mut hit_count = 0;
        for _ in 0..100 {
            let (_, hit) = get_armour(&defender, DefenceType::Ac, 10);
            if hit {
                hit_count += 1;
            }
        }
        assert_eq!(hit_count, 100, "AC type should always hit");
    }

    #[test]
    fn test_get_armour_agility_can_miss() {
        // 高 Agility 防御者 vs 低 Accuracy 攻击者应频繁闪避
        let defender = CombatStats { agility: 100, ..dummy_defender() };
        let mut miss_count = 0;
        for _ in 0..1000 {
            let (_, hit) = get_armour(&defender, DefenceType::AcAgility, 5);
            if !hit {
                miss_count += 1;
            }
        }
        assert!(miss_count > 800, "high agility should dodge often, got {} misses/1000", miss_count);
    }

    #[test]
    fn test_check_critical_with_high_rate() {
        // critical_rate=20 → 20*5=100，Random(100)<100 几乎必触发
        let mut crit_count = 0;
        for _ in 0..100 {
            if check_critical(20) {
                crit_count += 1;
            }
        }
        assert!(crit_count >= 95);
    }

    #[test]
    fn test_apply_critical_formula() {
        // damage=100, critical_damage=5 → 100 + floor(100 * 5/50 * 10) = 100 + 100 = 200
        assert_eq!(apply_critical(100, 5), 200);
        // damage=100, critical_damage=0 → 无加成
        assert_eq!(apply_critical(100, 0), 100);
    }

    #[test]
    fn test_resolve_attack_basic_damage() {
        let attacker = dummy_attacker();
        let defender = dummy_defender();
        let result = resolve_attack(&attacker, &defender, 15, DefenceType::Ac, 0);
        assert!(result.is_hit);
        assert!(result.damage >= 1);
        // 15 - armour(2..5) = 10..13
        assert!((10..=13).contains(&result.damage) || result.is_critical);
    }

    #[test]
    fn test_resolve_attack_misses_when_not_hit() {
        // 极高 Agility 防御者
        let attacker = dummy_attacker();
        let defender = CombatStats { agility: 1000, ..dummy_defender() };
        let mut miss_count = 0;
        for _ in 0..100 {
            let r = resolve_attack(&attacker, &defender, 15, DefenceType::AcAgility, 0);
            if !r.is_hit {
                miss_count += 1;
            }
        }
        assert!(miss_count > 90);
    }

    #[test]
    fn test_resolve_attack_reflect() {
        // defender.reflect=100 → 必触发反伤
        let attacker = dummy_attacker();
        let defender = CombatStats { reflect: 100, ..dummy_defender() };
        let result = resolve_attack(&attacker, &defender, 15, DefenceType::Ac, 0);
        assert!(result.reflected > 0, "reflect should trigger");
        assert_eq!(result.damage, 0, "reflected hit deals 0 to defender");
    }

    #[test]
    fn test_resolve_attack_damage_reduction() {
        // 50% 减伤：raw 15 → 15*0.5=7.5→7，护甲 2..5 → 伤害 2..5
        let attacker = dummy_attacker();
        let defender = CombatStats {
            damage_reduction_percent: 50,
            min_ac: 0,
            max_ac: 0,
            ..dummy_defender()
        };
        let result = resolve_attack(&attacker, &defender, 15, DefenceType::Ac, 0);
        // 无护甲 + 50% 减伤：15 → 7
        assert!(result.damage <= 8, "damage should be reduced, got {}", result.damage);
    }

    #[test]
    fn test_resolve_attack_armour_absorbs_all() {
        // 护甲 > 伤害 → 完全抵挡
        let attacker = dummy_attacker();
        let defender = CombatStats {
            min_ac: 100,
            max_ac: 100,
            ..dummy_defender()
        };
        let result = resolve_attack(&attacker, &defender, 15, DefenceType::Ac, 0);
        assert_eq!(result.damage, 0, "armour should absorb all damage");
    }

    #[test]
    fn test_resolve_attack_hp_drain() {
        let attacker = CombatStats { hp_drain_rate_percent: 100, ..dummy_attacker() };
        let defender = CombatStats { min_ac: 0, max_ac: 0, ..dummy_defender() };
        let result = resolve_attack(&attacker, &defender, 15, DefenceType::Ac, 0);
        assert!(result.hp_drain > 0, "should drain some HP, got {}", result.hp_drain);
    }

    #[test]
    fn test_apply_negative_effects_physical_only() {
        // MAC 攻击不应触发负面
        let attacker = CombatStats { freezing: 100, poison_attack: 100, ..Default::default() };
        let poisons = apply_negative_effects(&attacker, DefenceType::Mac, 0);
        assert!(poisons.is_empty(), "MAC attack should not trigger physical negative effects");
    }

    #[test]
    fn test_apply_negative_effects_freezing_triggers_slow() {
        let attacker = CombatStats { freezing: 100, ..Default::default() };
        let mut slow_count = 0;
        for _ in 0..100 {
            let poisons = apply_negative_effects(&attacker, DefenceType::AcAgility, 0);
            if poisons.iter().any(|p| p.p_type == PoisonType::SLOW) {
                slow_count += 1;
            }
        }
        assert!(slow_count > 90, "high freezing should slow often, got {}/100", slow_count);
    }
}
