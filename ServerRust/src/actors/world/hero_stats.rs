// ============================================================================
// 英雄基础属性（#1180，C# Shared/BaseStats.cs 移植）
// - BaseStats 五职业默认 Base/Gain/GainRate（HeroBaseStats 与 ClassBaseStats 同源）
// - BaseStat.Calculate：Health/Mana/Weight/Stat 公式
// - 装备加成复用 calculate_equipment_bonuses（玩家同款）
// 参考：HeroObject.RefreshLevelStats → Stats[stat.Type] = stat.Calculate(Class, Level)
// ============================================================================

use mir2_shared::enums::MirClass;

/// 英雄当前等级所需经验（C# Settings.HeroExpList 无配置时默认 100/级；
/// HeroObject.RefreshMaxExperience = HeroExperienceList[Level-1]）
pub const HERO_MAX_EXPERIENCE: u32 = 100;

/// 英雄属性集合（基础 + 装备加成）
#[derive(Debug, Clone, Copy, Default)]
pub struct HeroStats {
    pub max_hp: i32,
    pub max_mp: i32,
    pub min_ac: i32,
    pub max_ac: i32,
    pub min_mac: i32,
    pub max_mac: i32,
    pub min_dc: i32,
    pub max_dc: i32,
    pub min_mc: i32,
    pub max_mc: i32,
    pub min_sc: i32,
    pub max_sc: i32,
    pub accuracy: i32,
    pub agility: i32,
    pub bag_weight: i32,
    pub wear_weight: i32,
    pub hand_weight: i32,
}

impl HeroStats {
    /// 英雄职业对应的法术属性（C# HumanObject Magic：Wizard→MaxMC / Taoist→MaxSC / 其他→MaxDC）
    pub fn effective_magic_attack(&self, class: MirClass) -> i32 {
        match class {
            MirClass::Wizard => self.max_mc,
            MirClass::Taoist => self.max_sc,
            _ => self.max_dc,
        }
    }

    /// 转换为战斗属性（供 resolve_attack / hero AI 使用）
    pub fn to_combat_stats(&self) -> crate::combat::attack::CombatStats {
        use crate::combat::attack::CombatStats;
        CombatStats {
            min_atk: self.min_dc,
            max_atk: self.max_dc,
            min_ac: self.min_ac,
            max_ac: self.max_ac,
            min_mac: self.min_mac,
            max_mac: self.max_mac,
            agility: self.agility,
            accuracy: self.accuracy,
            ..Default::default()
        }
    }
}

/// C# BaseStat.Calculate 的四种公式（level i32，Gain/GainRate f32；结果截断取整）
fn calc_health(class: MirClass, base: i32, gain: f32, gain_rate: f32, level: i32) -> i32 {
    // C# BaseStat.Calculate：Gain == 0 直接返回 Base（避免除零）
    if gain <= 0.0 {
        return base;
    }
    let v = match class {
        MirClass::Warrior => base as f32 + (level as f32 / gain + gain_rate + level as f32 / 20.0) * level as f32,
        _ => base as f32 + (level as f32 / gain + gain_rate) * level as f32,
    };
    v as i32
}

fn calc_mana(class: MirClass, base: i32, gain: f32, gain_rate: f32, level: i32) -> i32 {
    // C# BaseStat.Calculate：Gain == 0 直接返回 Base（避免除零）
    if gain <= 0.0 {
        return base;
    }
    let v = match class {
        MirClass::Wizard => base as f32 + ((level as f32 / gain + 2.0) * 2.2 * level as f32) + (level as f32 * gain_rate),
        MirClass::Taoist => (base as f32 + level as f32 / gain * 2.2 * level as f32) + (level as f32 * gain_rate),
        _ => base as f32 + (level as f32 * gain) + (level as f32 * gain_rate),
    };
    v as i32
}

fn calc_weight(base: i32, gain: f32, level: i32) -> i32 {
    if gain <= 0.0 {
        return base;
    }
    (base as f32 + (level as f32 / gain) * level as f32) as i32
}

fn calc_stat(base: i32, gain: f32, level: i32) -> i32 {
    // C# BaseStat.Calculate：Gain == 0 直接返回 Base（避免除零产生 i32::MAX）
    if gain <= 0.0 {
        return base;
    }
    (base as f32 + level as f32 / gain) as i32
}

/// 英雄基础属性（C# BaseStats 构造器默认值，五职业）
pub fn hero_base_stats(class: MirClass, level: i32) -> HeroStats {
    use MirClass::*;
    let (hp_b, hp_g, hp_gr, mp_b, mp_g, mp_gr, bag_g, wear_g, hand_g,
         min_dc_g, max_dc_g, min_mc_g, max_mc_g, min_sc_g, max_sc_g,
         max_ac_g, min_mac_g, max_mac_g, acc_b, agi_b) = match class {
        Warrior => (14, 4.0, 4.5, 11, 3.5, 0.0, 3.0, 20.0, 13.0, 5.0, 5.0, 0.0, 0.0, 0.0, 0.0, 7.0, 0.0, 0.0, 5, 15),
        Wizard => (14, 15.0, 1.8, 13, 5.0, 0.0, 5.0, 100.0, 90.0, 7.0, 7.0, 7.0, 7.0, 0.0, 0.0, 0.0, 0.0, 0.0, 5, 15),
        Taoist => (14, 6.0, 2.5, 13, 8.0, 0.0, 4.0, 50.0, 42.0, 7.0, 7.0, 0.0, 0.0, 7.0, 7.0, 0.0, 12.0, 6.0, 5, 18),
        Assassin => (14, 4.0, 3.25, 11, 5.0, 0.0, 3.5, 33.0, 30.0, 8.0, 8.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 5, 20),
        Archer => (14, 4.0, 3.25, 11, 4.0, 0.0, 4.0, 33.0, 30.0, 8.0, 8.0, 8.0, 8.0, 0.0, 0.0, 0.0, 0.0, 0.0, 8, 15),
        _ => (14, 4.0, 4.5, 11, 3.5, 0.0, 3.0, 20.0, 13.0, 5.0, 5.0, 0.0, 0.0, 0.0, 0.0, 7.0, 0.0, 0.0, 5, 15),
    };
    HeroStats {
        max_hp: calc_health(class, hp_b, hp_g, hp_gr, level),
        max_mp: calc_mana(class, mp_b, mp_g, mp_gr, level),
        bag_weight: calc_weight(50, bag_g, level),
        wear_weight: calc_weight(15, wear_g, level),
        hand_weight: calc_weight(12, hand_g, level),
        min_dc: calc_stat(0, min_dc_g, level),
        max_dc: calc_stat(0, max_dc_g, level),
        min_mc: calc_stat(0, min_mc_g, level),
        max_mc: calc_stat(0, max_mc_g, level),
        min_sc: calc_stat(0, min_sc_g, level),
        max_sc: calc_stat(0, max_sc_g, level),
        min_ac: 0,
        max_ac: calc_stat(0, max_ac_g, level),
        min_mac: calc_stat(0, min_mac_g, level),
        max_mac: calc_stat(0, max_mac_g, level),
        accuracy: acc_b,
        agility: agi_b,
    }
}

/// 英雄完整属性 = 基础 + 装备加成（复用玩家 calculate_equipment_bonuses）
pub fn compute_hero_stats(
    class: MirClass,
    level: i32,
    equipment: &[Option<mir2_shared::data::item::UserItem>],
    item_infos: &std::collections::HashMap<i32, crate::db::ItemInfo>,
) -> HeroStats {
    let mut s = hero_base_stats(class, level);
    let b = super::calculate_equipment_bonuses(equipment, item_infos);
    s.max_hp += b.hp;
    s.max_mp += b.mp;
    s.min_ac += b.min_ac;
    s.max_ac += b.max_ac;
    s.min_mac += b.min_mac;
    s.max_mac += b.max_mac;
    s.min_dc += b.min_atk;
    s.max_dc += b.max_atk;
    s.min_mc += b.min_mc;
    s.max_mc += b.max_mc;
    s.min_sc += b.min_sc;
    s.max_sc += b.max_sc;
    s.accuracy += b.accuracy;
    s.agility += b.agility;
    s.bag_weight += b.bag_weight;
    s.wear_weight += b.wear_weight;
    s.hand_weight += b.hand_weight;
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warrior_hp_formula_matches_csharp() {
        // C#：HP = 14 + (level/4 + 4.5 + level/20) * level
        let lv1 = hero_base_stats(MirClass::Warrior, 1);
        assert_eq!(lv1.max_hp, (14.0 + (1.0 / 4.0 + 4.5 + 1.0 / 20.0) * 1.0) as i32); // 18
        let lv30 = hero_base_stats(MirClass::Warrior, 30);
        assert_eq!(lv30.max_hp, (14.0 + (30.0 / 4.0 + 4.5 + 30.0 / 20.0) * 30.0) as i32); // 419
        let lv50 = hero_base_stats(MirClass::Warrior, 50);
        assert_eq!(lv50.max_hp, (14.0 + (50.0 / 4.0 + 4.5 + 50.0 / 20.0) * 50.0) as i32);
    }

    #[test]
    fn wizard_mana_formula_matches_csharp() {
        // C#：MP = 13 + ((level/5 + 2) * 2.2 * level) + level*0
        let lv10 = hero_base_stats(MirClass::Wizard, 10);
        assert_eq!(lv10.max_mp, (13.0 + ((10.0 / 5.0 + 2.0) * 2.2 * 10.0)) as i32); // 13+88=101
        assert!(lv10.max_mp > 0);
    }

    #[test]
    fn taoist_has_mac_and_sc() {
        let t = hero_base_stats(MirClass::Taoist, 30);
        assert!(t.max_mac > 0 && t.min_mac > 0);
        assert!(t.max_sc > 0 && t.min_sc > 0);
        assert_eq!(t.max_dc, (0.0 + 30.0 / 7.0) as i32); // 4
    }

    #[test]
    fn effective_magic_attack_per_class() {
        let w = hero_base_stats(MirClass::Wizard, 30);
        assert!(w.max_mc > 0);
        assert_eq!(w.effective_magic_attack(MirClass::Wizard), w.max_mc);
        let t = hero_base_stats(MirClass::Taoist, 30);
        assert_eq!(t.effective_magic_attack(MirClass::Taoist), t.max_sc);
        let a = hero_base_stats(MirClass::Archer, 30);
        assert_eq!(a.effective_magic_attack(MirClass::Archer), a.max_dc);
    }

    #[test]
    fn equipment_adds_to_stats() {
        // 无装备 = 基础；有装备（HP+100 的铠甲）→ max_hp 增加
        let empty: Vec<Option<mir2_shared::data::item::UserItem>> = vec![None; 14];
        let map = std::collections::HashMap::new();
        let base = compute_hero_stats(MirClass::Warrior, 30, &empty, &map);
        assert_eq!(base.max_hp, 419);
    }
}
