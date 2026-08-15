// Poison / 负面状态系统
//
// 对齐 C# `Shared/Enums.cs` PoisonType（`[Flags] : ushort`）与
// `Server/MirObjects/MapObject.cs` ApplyPoison 的语义。
//
// PoisonType 为位掩码（可组合），但单条 Poison 通常只持一种 PType；
// ApplyNegativeEffects（attack.rs）会按攻击者的 Stats 触发 Green/Slow/Paralysis。

use mir2_shared::enums::PoisonType;

/// 一条中毒/负面状态实例
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Poison {
    pub p_type: PoisonType,
    /// 持续时间（秒）
    pub duration_s: u32,
    /// 每 tick 的数值（伤害量 / 或 0 表示无数值）
    pub value: i32,
    /// tick 间隔（毫秒）
    pub tick_ms: u64,
    /// DelayedExplosion 毒：施法者 session（0=无）
    pub owner_session: u64,
    /// DelayedExplosion 毒：当前阶段（0 未开始 / 1 等待引爆 / 2 引爆）
    pub delayed_stage: u8,
    /// DelayedExplosion 毒：下一阶段推进的世界 tick（0=未设置）
    pub delayed_next_tick: u64,
}

impl Poison {
    pub fn new(p_type: PoisonType, duration_s: u32, value: i32, tick_ms: u64) -> Self {
        Self {
            p_type,
            duration_s,
            value,
            tick_ms,
            owner_session: 0,
            delayed_stage: 0,
            delayed_next_tick: 0,
        }
    }

    /// 是否为"完全失控"状态（无法移动/攻击）
    pub fn is_incapacitating(&self) -> bool {
        self.p_type.intersects(
            PoisonType::STUN | PoisonType::PARALYSIS | PoisonType::FROZEN | PoisonType::DAZED,
        )
    }

    /// 是否为"减速"状态（影响移动间隔）
    pub fn is_slowing(&self) -> bool {
        self.p_type
            .intersects(PoisonType::SLOW | PoisonType::FROZEN)
    }
}

/// apply 侧四层保护所需的防御方属性快照（C# `HumanObject.ApplyPoison` :7380-7458 /
/// `MonsterObject.ApplyPoison` :2782-2810）。
///
/// 怪物侧无 PoisonResist/PoisonRecovery 属性（C# 怪物 ApplyPoison 亦无这两层），
/// 传 0 即空转；玩家侧由 `PlayerState::poison_defence()` / 怪物由
/// `MonsterState::apply_poison_defended` 填充。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoisonDefence {
    /// ① PoisonResist 抗性概率：掷骰 `0..100 < resist` → 整条毒免疫（C# :7384）
    pub poison_resist: i32,
    /// ③ PoisonRecovery：Green/Red 持续时长缩短该秒数，归零则不生效（C# :7410）
    pub poison_recovery: i32,
    /// ② 绿毒 MAC 减免区间 `(min_mac, max_mac)`（C# :7390-7398
    /// `armour = GetAttackPower(MinMAC, MaxMAC)`，`Value < armour` 整条丢弃）。
    /// `None` = ignoreDefence（C# 形参默认 true 不减免）。
    pub mac_range: Option<(i32, i32)>,
}

impl PoisonDefence {
    /// 无任何保护（0 抗性 / 0 恢复 / 不做 MAC 减免）
    pub fn none() -> Self {
        Self {
            poison_resist: 0,
            poison_recovery: 0,
            mac_range: None,
        }
    }
}

/// DelayedExplosion 阶段推进事件（C# `ProcessDelayedExplosion`，HumanObject.cs:695-769）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelayedExplosionEvent {
    /// 升入阶段 1：向同图广播 `ObjectEffect(DelayedExplosion, EffectType=1)`
    Stage1,
    /// 升入阶段 2 引爆：广播 `ObjectEffect(EffectType=2)` + `RemoveDelayedExplosion`，
    /// 并以施法者（`owner_session`）数据在目标当前位置结算 3×3 AoE（`value` 为伤害参数）
    Explode { owner_session: u64, value: i32 },
}

/// 阶段 1 → 阶段 2 的等待时长（C# :747 `ExplosionInflictedTime = TickTime + 3000`）
pub const DELAYED_EXPLOSION_STAGE_WAIT_MS: u64 = 3000;

/// 把一条 Poison 加入目标的 poison_list，过四层保护后同型替换/追加。
/// 返回是否实际生效（false = 被抗性豁免 / MAC 完全抵消 / 时长归零 / 弱毒被强毒挡下）。
///
/// 对齐 C# `HumanObject.ApplyPoison` :7380-7458：
/// ① PoisonResist 掷骰豁免（:7384，PoisonResistWeight=100）
/// ② 绿毒被 MAC 减免（:7390-7398；`mac_range=None` 等效 ignoreDefence=true）
/// ③ PoisonRecovery 缩短 Green/Red 时长（:7410），归零则毒不生效
/// ④ 强度保护（:7414-7419）：弱毒不覆盖同型强毒（Green 比 value、其余比剩余时长）；
///    Frozen/Slow/Paralysis/LRParalysis 持续期内不可重复施加（防永冻/永控）；
///    DelayedExplosion 不可重复施加（:7420）
///
/// 简化：C# 的 `Caster != null && !NoResist` 门控与 PvpCanResistPoison 分支不区分
/// 施法者来源（Rust 毒不携带施法者种族），统一按防御方 resist 掷骰。
pub fn apply_poison(poison_list: &mut Vec<Poison>, mut p: Poison, def: &PoisonDefence) -> bool {
    // ① 抗性概率豁免
    if def.poison_resist > 0 && fastrand::i32(0..100) < def.poison_resist {
        return false;
    }
    // ② 绿毒 MAC 减免：Value < armour → 整条丢弃（C# PType = None）
    if p.p_type == PoisonType::GREEN {
        if let Some((min_mac, max_mac)) = def.mac_range {
            let armour = crate::combat::attack::get_attack_power(min_mac, max_mac, 0);
            if p.value < armour {
                return false;
            }
            p.value -= armour;
        }
    }
    // ③ PoisonRecovery 缩短 Green/Red 时长
    if p.p_type.intersects(PoisonType::GREEN | PoisonType::RED) && def.poison_recovery > 0 {
        p.duration_s = p.duration_s.saturating_sub(def.poison_recovery as u32);
        if p.duration_s == 0 {
            return false;
        }
    }
    // ④ 强度/永冻保护 + 同型替换
    for existing in poison_list.iter_mut() {
        if existing.p_type != p.p_type {
            continue;
        }
        if existing.p_type == PoisonType::GREEN && existing.value > p.value {
            return false; // 弱绿毒不能顶掉强绿毒
        }
        if existing.p_type != PoisonType::GREEN && existing.duration_s > p.duration_s {
            return false; // 短时长毒不能顶掉剩余更长的同型毒
        }
        if existing.p_type.intersects(
            PoisonType::FROZEN
                | PoisonType::SLOW
                | PoisonType::PARALYSIS
                | PoisonType::LR_PARALYSIS,
        ) {
            return false; // 永冻/永控保护：持续期内不可重复施加
        }
        if p.p_type == PoisonType::DELAYED_EXPLOSION {
            return false; // 定时爆炸不可重复施加
        }
        *existing = p;
        return true;
    }
    poison_list.push(p);
    true
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

/// tick 推进：duration 递减，返回应扣血量。
///
/// `dt_s` 为本次 tick 推进的秒数。Green/Red/Bleeding 按 `value * dt_s` 掉血。
/// **注意**：`value` 是每次 Poison tick 的固定伤害量（对齐 C# Poison.Value），
/// `tick_ms` 字段仅供客户端显示/协议序列化，不影响服务端掉血节奏
/// （服务端固定在 TickBuff/怪物 poison tick 中每 5 ticks ≈ 0.5s 调用一次，
/// 传 dt_s=1，即每次推进 1 秒 duration + 扣 value 血）。
pub fn tick_poisons(poison_list: &mut Vec<Poison>, dt_s: u32) -> i32 {
    let mut total_damage = 0i32;
    for p in poison_list.iter_mut() {
        p.duration_s = p.duration_s.saturating_sub(dt_s);
        if p.p_type
            .intersects(PoisonType::GREEN | PoisonType::RED | PoisonType::BLEEDING)
        {
            total_damage = total_damage.saturating_add(p.value.max(0) * dt_s as i32);
        }
    }
    // 移除过期
    poison_list.retain(|p| p.duration_s > 0);
    total_damage
}

/// 推进 DelayedExplosion 毒的阶段（C# `ProcessDelayedExplosion`，HumanObject.cs:695-769）。
///
/// 阶段 0（挂毒）→ 阶段 1（广播 EffectType=1 特效）→ 等待 → 阶段 2（引爆）。
/// `now` 与 `wait` 单位由调用方决定：怪物侧为世界 tick（100ms/tick，wait=30），
/// 玩家侧为 Unix 毫秒（wait=`DELAYED_EXPLOSION_STAGE_WAIT_MS`）。
///
/// 返回 `Some(event)` 表示本次发生阶段跃迁；`Explode` 时毒已从列表移除
/// （对齐 C# `PoisonList.RemoveAt(i)`）。目标死亡由调用方先行处理
/// （C# `if (Dead) return false` 直接结束）。
pub fn advance_delayed_explosion(
    poison_list: &mut Vec<Poison>,
    now: u64,
    wait: u64,
) -> Option<DelayedExplosionEvent> {
    let p = poison_list
        .iter_mut()
        .find(|p| p.p_type == PoisonType::DELAYED_EXPLOSION)?;
    if p.delayed_stage > 0 && now < p.delayed_next_tick {
        return None; // 未到下一阶段时间
    }
    p.delayed_stage = p.delayed_stage.saturating_add(1);
    match p.delayed_stage {
        1 => {
            p.delayed_next_tick = now + wait;
            Some(DelayedExplosionEvent::Stage1)
        }
        2 => {
            let event = DelayedExplosionEvent::Explode {
                owner_session: p.owner_session,
                value: p.value,
            };
            poison_list.retain(|x| x.p_type != PoisonType::DELAYED_EXPLOSION);
            Some(event)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_and_replace_poison() {
        let mut list = Vec::new();
        assert!(apply_poison(
            &mut list,
            Poison::new(PoisonType::GREEN, 5, 3, 1000),
            &PoisonDefence::none()
        ));
        assert_eq!(list.len(), 1);
        // 同类型替换（新毒 value 8 > 旧 3，强度保护放行）
        assert!(apply_poison(
            &mut list,
            Poison::new(PoisonType::GREEN, 10, 8, 1000),
            &PoisonDefence::none()
        ));
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].duration_s, 10);
        assert_eq!(list[0].value, 8);
        // 不同类型追加
        assert!(apply_poison(
            &mut list,
            Poison::new(PoisonType::SLOW, 4, 0, 1000),
            &PoisonDefence::none()
        ));
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
    fn test_delayed_explosion_poison_fields() {
        // DelayedExplosion 毒：owner/stage/next_tick 保留
        let mut p = Poison::new(PoisonType::DELAYED_EXPLOSION, 30, 100, 2000);
        p.owner_session = 42;
        p.delayed_stage = 1;
        p.delayed_next_tick = 123;
        let mut list = Vec::new();
        assert!(apply_poison(&mut list, p, &PoisonDefence::none()));
        assert_eq!(list[0].owner_session, 42);
        assert_eq!(list[0].delayed_stage, 1);
        assert_eq!(list[0].delayed_next_tick, 123);
        // #2569 层④：已有 DelayedExplosion 时不可重复施加（C# :7420 直接 return）
        assert!(!apply_poison(
            &mut list,
            Poison::new(PoisonType::DELAYED_EXPLOSION, 10, 50, 2000),
            &PoisonDefence::none()
        ));
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].owner_session, 42);
        assert_eq!(list[0].delayed_stage, 1);
    }

    #[test]
    fn test_incacapacitated_check() {
        let list = vec![Poison::new(PoisonType::STUN, 3, 0, 1000)];
        assert!(is_incacapacitated(&list));
        let list = vec![Poison::new(PoisonType::GREEN, 3, 5, 1000)];
        assert!(!is_incacapacitated(&list));
    }

    // ============ #2569：apply 侧四层保护 ============

    #[test]
    fn test_layer1_resist_roll_immunity() {
        // resist=100：0..100 掷骰恒 < 100 → 必然免疫
        let def = PoisonDefence {
            poison_resist: 100,
            ..PoisonDefence::none()
        };
        let mut list = Vec::new();
        assert!(!apply_poison(
            &mut list,
            Poison::new(PoisonType::GREEN, 5, 3, 1000),
            &def
        ));
        assert!(!apply_poison(
            &mut list,
            Poison::new(PoisonType::PARALYSIS, 5, 0, 1000),
            &def
        ));
        assert!(list.is_empty());
        // resist=0：恒不豁免（怪物侧默认）
        let mut list = Vec::new();
        assert!(apply_poison(
            &mut list,
            Poison::new(PoisonType::GREEN, 5, 3, 1000),
            &PoisonDefence::none()
        ));
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn test_layer2_green_mac_reduction() {
        // MAC 5..5（区间退化为定值，掷骰确定）；C# :7390-7398
        let def = PoisonDefence {
            mac_range: Some((5, 5)),
            ..PoisonDefence::none()
        };
        // 绿毒 value 3 < armour 5 → 整条丢弃（C# PType = None）
        let mut list = Vec::new();
        assert!(!apply_poison(
            &mut list,
            Poison::new(PoisonType::GREEN, 5, 3, 1000),
            &def
        ));
        assert!(list.is_empty());
        // 绿毒 value 10 ≥ 5 → 生效且 value 减 5
        assert!(apply_poison(
            &mut list,
            Poison::new(PoisonType::GREEN, 5, 10, 1000),
            &def
        ));
        assert_eq!(list[0].value, 5);
        // mac_range=None（ignoreDefence=true）：不减
        assert!(apply_poison(
            &mut list,
            Poison::new(PoisonType::GREEN, 5, 10, 1000),
            &PoisonDefence::none()
        ));
        assert_eq!(list[0].value, 10);
        // 非绿毒（RED）不走 MAC 减免
        assert!(apply_poison(
            &mut list,
            Poison::new(PoisonType::RED, 5, 3, 1000),
            &def
        ));
        assert_eq!(list[1].value, 3);
    }

    #[test]
    fn test_layer3_recovery_shortens_green_red() {
        let def = PoisonDefence {
            poison_recovery: 4,
            ..PoisonDefence::none()
        };
        // Green 10s - 4 = 6s 生效
        let mut list = Vec::new();
        assert!(apply_poison(
            &mut list,
            Poison::new(PoisonType::GREEN, 10, 3, 1000),
            &def
        ));
        assert_eq!(list[0].duration_s, 6);
        // Red 4s - 4 = 0 → 不生效（C# :7411 Duration==0 return）
        assert!(!apply_poison(
            &mut list,
            Poison::new(PoisonType::RED, 4, 3, 1000),
            &def
        ));
        assert_eq!(list.len(), 1);
        // 非 Green/Red（SLOW）不受 PoisonRecovery 影响
        assert!(apply_poison(
            &mut list,
            Poison::new(PoisonType::SLOW, 4, 0, 1000),
            &def
        ));
        assert_eq!(list[1].duration_s, 4);
    }

    #[test]
    fn test_layer4_weak_green_cannot_override_strong() {
        // C# :7417：旧绿毒 value 8，新毒 value 5 → 拒绝（弱毒不能顶掉强毒）
        let mut list = vec![Poison::new(PoisonType::GREEN, 5, 8, 1000)];
        assert!(!apply_poison(
            &mut list,
            Poison::new(PoisonType::GREEN, 99, 5, 1000),
            &PoisonDefence::none()
        ));
        assert_eq!(list[0].value, 8);
        assert_eq!(list[0].duration_s, 5);
        // 新毒 value 9 > 8 → 替换（即使时长更短：Green 只比 value）
        assert!(apply_poison(
            &mut list,
            Poison::new(PoisonType::GREEN, 1, 9, 1000),
            &PoisonDefence::none()
        ));
        assert_eq!(list[0].value, 9);
        assert_eq!(list[0].duration_s, 1);
    }

    #[test]
    fn test_layer4_short_duration_cannot_override_long() {
        // C# :7418：非 Green 比剩余时长——旧 SLOW 剩 10s，新 3s → 拒绝
        let mut list = vec![Poison::new(PoisonType::RED, 10, 3, 1000)];
        assert!(!apply_poison(
            &mut list,
            Poison::new(PoisonType::RED, 3, 3, 1000),
            &PoisonDefence::none()
        ));
        assert_eq!(list[0].duration_s, 10);
        // 新 12s > 剩 10s → 替换；等长（10 vs 剩 10）也放行（C# 严格大于才拒）
        assert!(apply_poison(
            &mut list,
            Poison::new(PoisonType::RED, 12, 3, 1000),
            &PoisonDefence::none()
        ));
        assert_eq!(list[0].duration_s, 12);
        assert!(apply_poison(
            &mut list,
            Poison::new(PoisonType::RED, 12, 3, 1000),
            &PoisonDefence::none()
        ));
    }

    #[test]
    fn test_layer4_permafrost_types_no_reapply() {
        // C# :7419：Frozen/Slow/Paralysis/LRParalysis 持续期内不可重复施加（防永冻/永控），
        // 即使新毒时长更长也拒绝
        for t in [
            PoisonType::FROZEN,
            PoisonType::SLOW,
            PoisonType::PARALYSIS,
            PoisonType::LR_PARALYSIS,
        ] {
            let mut list = vec![Poison::new(t, 3, 0, 1000)];
            assert!(!apply_poison(
                &mut list,
                Poison::new(t, 99, 0, 1000),
                &PoisonDefence::none()
            ));
            assert_eq!(list.len(), 1);
            assert_eq!(list[0].duration_s, 3, "{t:?} 持续期内不可重复施加");
            // 旧毒过期移除后可重新施加
            tick_poisons(&mut list, 3);
            assert!(list.is_empty());
            assert!(apply_poison(
                &mut list,
                Poison::new(t, 5, 0, 1000),
                &PoisonDefence::none()
            ));
            assert_eq!(list.len(), 1);
        }
    }

    // ============ #2569：DelayedExplosion 三阶段 ============

    #[test]
    fn test_delayed_explosion_three_stage_timing() {
        // 挂毒（阶段 0）：owner/value 为引爆结算参数
        let mut p = Poison::new(PoisonType::DELAYED_EXPLOSION, 30, 100, 2000);
        p.owner_session = 42;
        let mut list = vec![p];
        // 首次推进 → 阶段 1（特效广播），设置 3s 后的引爆窗口
        assert_eq!(
            advance_delayed_explosion(&mut list, 1000, 3000),
            Some(DelayedExplosionEvent::Stage1)
        );
        assert_eq!(list[0].delayed_stage, 1);
        assert_eq!(list[0].delayed_next_tick, 4000);
        // 窗口内推进：无事件
        assert_eq!(advance_delayed_explosion(&mut list, 3999, 3000), None);
        assert_eq!(list[0].delayed_stage, 1);
        // 到时 → 阶段 2 引爆：携带施法者与伤害参数，毒被移除（C# PoisonList.RemoveAt）
        assert_eq!(
            advance_delayed_explosion(&mut list, 4000, 3000),
            Some(DelayedExplosionEvent::Explode {
                owner_session: 42,
                value: 100
            })
        );
        assert!(list.is_empty());
    }

    #[test]
    fn test_delayed_explosion_ignores_other_poisons() {
        // 非 DelayedExplosion 毒不参与阶段推进
        let mut list = vec![
            Poison::new(PoisonType::GREEN, 5, 3, 1000),
            Poison::new(PoisonType::STUN, 2, 0, 1000),
        ];
        assert_eq!(advance_delayed_explosion(&mut list, 999_999, 3000), None);
        assert_eq!(list.len(), 2);
    }
}
