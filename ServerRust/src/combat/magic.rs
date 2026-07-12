// 魔法/技能伤害公式，对应 C# MirDatabase/MagicInfo.cs UserMagic

use crate::db::MagicInfo;

/// 法术的魔法攻击力分量（C# MPower()）。
/// C# 语义：Bonus>0 时在 [Base, Base+Bonus) 间随机，否则取 Base。
/// **与等级无关**——Level 只出现在 GetPower 的 (Level+1) 乘数里。
fn magic_mpower(info: &MagicInfo) -> f64 {
    if info.mpower_bonus > 0 {
        let range = info.mpower_base + info.mpower_bonus;
        fastrand::i32(info.mpower_base..range) as f64
    } else {
        info.mpower_base as f64
    }
}

/// 法术的基础攻击力分量（C# DefPower()）。
/// C# 语义：Bonus>0 时在 [Base, Base+Bonus) 间随机，否则取 Base。
fn magic_def_power(info: &MagicInfo) -> f64 {
    if info.power_bonus > 0 {
        let range = info.power_base + info.power_bonus;
        fastrand::i32(info.power_base..range) as f64
    } else {
        info.power_base as f64
    }
}

/// C# GetPower() — 法术威力 = round(MPower()/4 * (Level+1) + DefPower())
pub fn magic_power(info: &MagicInfo, level: u8) -> f64 {
    let raw = (magic_mpower(info) / 4.0) * (level as f64 + 1.0) + magic_def_power(info);
    raw.round()
}

/// C# GetMultiplier() — 法术倍率
pub fn magic_multiplier(info: &MagicInfo, level: u8) -> f64 {
    info.multiplier_base + (level as f64) * info.multiplier_bonus
}

/// C# GetDamage(DamageBase) — 最终伤害计算
/// `stat` = 施法者的 MC（法师）或 SC（道士）
pub fn calc_magic_damage(info: &MagicInfo, level: u8, stat: i32) -> i32 {
    let base = stat as f64;
    let power = magic_power(info, level);
    let mult = magic_multiplier(info, level);
    ((base + power) * mult) as i32
}

/// C# GetDelay() — 法术冷却时间（毫秒）
pub fn magic_delay(info: &MagicInfo, level: u8) -> i32 {
    (info.delay_base - (level as i32 * info.delay_reduction)).max(500)
}

/// 法术消耗 MP
pub fn magic_cost(info: &MagicInfo, level: u8) -> i32 {
    info.base_cost + (level as i32 * info.level_cost)
}

/// 巫师：MC 转额外伤害（对应 C# SC/MC 属性对法术的加成）
pub fn wizard_magic_bonus(mc: i32) -> f64 {
    mc as f64
}

/// 道士：SC 转额外伤害
pub fn taoist_magic_bonus(sc: i32) -> f64 {
    sc as f64
}
