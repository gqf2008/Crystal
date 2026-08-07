/// 持续性法术对象，对应 C# SpellObject
/// 代表地图上的火墙、暴风雪、毒云等持续效果

use std::time::Instant;

/// 法术对象实例
#[derive(Debug, Clone)]
pub struct SpellObject {
    /// 唯一 ID（用于网络包）
    pub object_id: u32,
    /// 法术类型
    pub spell: mir2_shared::enums::Spell,
    /// 施法者 object_id
    pub caster_id: u32,
    /// 施法者 session_id
    pub caster_session: u64,
    /// 所在地图索引（防跨图命中）
    pub map_index: u16,
    /// 施法位置（格子坐标）
    pub x: i32,
    pub y: i32,
    /// 朝向
    pub direction: u8,
    /// 每跳间隔（毫秒），对应 C# TickSpeed
    pub tick_interval_ms: u64,
    /// 上次处理时间
    pub last_tick: Instant,
    /// 创建时间
    pub created_at: Instant,
    /// 过期时间（毫秒），对应 C# TickTime
    pub expires_at_ms: u64,
    /// 当前值（伤害/治疗量）
    pub value: i32,
    /// 每跳伤害/治疗
    pub tick_value: i32,
    /// 是否可见
    pub visible: bool,
    /// 施法者是否已死亡
    pub caster_dead: bool,
    /// 法术等级
    pub spell_level: u8,
    /// 额外数据（如火墙的 MAC 伤害、毒云的 SC 加成）
    pub bonus: i32,
    /// 是否已爆炸（陷阱类）
    pub detonated: bool,
    /// 关联的陷阱 ID（用于连锁爆炸）
    pub linked_trap_id: Option<u32>,
    /// 目标对象 ID（DelayedExplosion 挂毒用）
    pub target_id: Option<u32>,
}

impl SpellObject {
    pub fn new(
        object_id: u32,
        spell: mir2_shared::enums::Spell,
        caster_id: u32,
        caster_session: u64,
        map_index: u16,
        x: i32,
        y: i32,
        expires_at_ms: u64,
        tick_value: i32,
        tick_interval_ms: u64,
        spell_level: u8,
        bonus: i32,
    ) -> Self {
        let now = Instant::now();
        Self {
            object_id,
            spell,
            caster_id,
            caster_session,
            map_index,
            x,
            y,
            direction: 0,
            tick_interval_ms,
            last_tick: now,
            created_at: now,
            expires_at_ms,
            value: 0,
            tick_value,
            visible: true,
            caster_dead: false,
            spell_level,
            bonus,
            detonated: false,
            linked_trap_id: None,
            target_id: None,
        }
    }

    /// 是否需要过期
    pub fn is_expired(&self, elapsed_ms: u64) -> bool {
        elapsed_ms >= self.expires_at_ms
    }
}

/// 法术参数配置
struct SpellConfig {
    spell: mir2_shared::enums::Spell,
    duration_ms: u64,
    tick_interval_ms: u64,
    tick_value: i32,
}

fn make_spell_config(
    spell: mir2_shared::enums::Spell,
    _level: u8,
    stat: i32,
) -> SpellConfig {
    use mir2_shared::enums::Spell;
    match spell {
        Spell::FireWall => SpellConfig {
            spell: Spell::FireWall,
            duration_ms: 30_000,
            tick_interval_ms: 2000,
            tick_value: stat.max(10) * 2,
        },
        Spell::PoisonCloud => SpellConfig {
            spell: Spell::PoisonCloud,
            duration_ms: 20_000,
            tick_interval_ms: 2000,
            tick_value: stat.max(5) * 3,
        },
        Spell::Blizzard => SpellConfig {
            spell: Spell::Blizzard,
            duration_ms: 25_000,
            tick_interval_ms: 2000,
            tick_value: stat.max(15) * 3,
        },
        Spell::MeteorStrike => SpellConfig {
            spell: Spell::MeteorStrike,
            duration_ms: 20_000,
            tick_interval_ms: 2500,
            tick_value: stat.max(20) * 4,
        },
        Spell::HealingCircle => SpellConfig {
            spell: Spell::HealingCircle,
            duration_ms: 15_000,
            tick_interval_ms: 1500,
            tick_value: stat.max(5) * 2 + 25,
        },
        Spell::ExplosiveTrap => SpellConfig {
            spell: Spell::ExplosiveTrap,
            duration_ms: 60_000,
            tick_interval_ms: 1000,
            tick_value: stat.max(30) * 2,
        },
        Spell::DelayedExplosion => SpellConfig {
            spell: Spell::DelayedExplosion,
            // expires_at_ms 在施法时按距离覆盖（距离*50+500ms），tick 间隔调小以便及时引爆
            duration_ms: 60_000,
            tick_interval_ms: 100,
            tick_value: stat.max(30) * 2,
        },
        Spell::Portal => SpellConfig {
            spell: Spell::Portal,
            duration_ms: 60_000,
            tick_interval_ms: 500,
            tick_value: 0,
        },
        _ => SpellConfig {
            spell,
            duration_ms: 10_000,
            tick_interval_ms: 2000,
            tick_value: stat.max(1) * 2,
        },
    }
}

/// 创建持久法术对象
pub fn create_persistent_spell(
    object_id: u32, caster_id: u32, caster_session: u64, map_index: u16,
    x: i32, y: i32, level: u8, stat: i32, spell: mir2_shared::enums::Spell,
) -> SpellObject {
    let cfg = make_spell_config(spell, level, stat);
    SpellObject::new(
        object_id, cfg.spell,
        caster_id, caster_session, map_index, x, y,
        cfg.duration_ms, cfg.tick_value, cfg.tick_interval_ms,
        level, stat,
    )
}
