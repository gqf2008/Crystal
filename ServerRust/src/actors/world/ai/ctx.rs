//! AI 上下文 + 输出动作（借用隔离，复用 tick_monsters 的输出队列模式）
//!
//! behavior 通过 AiCtx 读取怪物自身状态 + 玩家快照，通过输出队列推动作，
//! 循环外由 tick_monsters 统一应用（避免 &mut self.monsters 借用冲突）。

use mir2_shared::enums::Spell;
use crate::combat::poison::Poison;

/// 玩家快照（behavior 不可直接访问 PlayerActor，只读此快照）
#[derive(Debug, Clone, Copy)]
pub struct PlayerSnap {
    pub session_id: u64,
    pub x: i32,
    pub y: i32,
    pub hp: i32,
    pub map_index: u16,
    pub object_id: u32,
    pub level: u16,
}

/// 怪物快照（供 Boss 互查，如 Healer 治疗友军、Yimoogi 分身聚合）
#[derive(Debug, Clone, Copy)]
pub struct MonsterSnap {
    pub object_id: u32,
    pub x: i32,
    pub y: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub map_index: u16,
    pub monster_index: i32,
}

/// AI 产生的攻击动作（广播 + 延迟伤害）
#[derive(Debug, Clone)]
pub enum AttackAction {
    /// 近战单体（C# ObjectAttack + DelayedAction Damage）
    Melee {
        attacker_oid: u32,
        target_session: u64,
        damage: i32,
        spell_id: u8,
        attack_type: u8,
    },
    /// 远程弹道（C# ObjectRangeAttack + DelayedAction RangeDamage）
    Range {
        attacker_oid: u32,
        target_session: u64,
        target_object_id: u32,
        damage: i32,
        spell_id: u8,
    },
    /// 范围 AOE（C# FindAllTargets 循环 Attacked）
    Aoe {
        attacker_oid: u32,
        center_x: i32,
        center_y: i32,
        radius: i32,
        damage: i32,
        spell_id: u8,
    },
    /// 直线攻击（C# LineAttack(damage, range)：沿 direction 逐格命中第一个目标）
    Line {
        attacker_oid: u32,
        origin_x: i32,
        origin_y: i32,
        direction: u8,
        range: i32,
        damage: i32,
        spell_id: u8,
    },
}

/// 地面法术场生成（Boss 投放 SpellObject，如 TreeQueen 根刺、HornedCommander RockSpike）
#[derive(Debug, Clone)]
pub struct SpellFieldSpawn {
    pub spell: Spell,
    pub x: i32,
    pub y: i32,
    pub value: i32,
    pub duration_ms: u64,
    pub tick_ms: u64,
    pub caster_oid: u32,
    pub caster_session: u64,
}

/// 召唤物生成（Boss 召唤小怪）
#[derive(Debug, Clone)]
pub struct BossSummon {
    /// 召唤的怪物名称（用于查 MonsterInfo，对齐 C# Envir.GetMonsterInfo(name)）
    pub monster_name: String,
    pub x: i32,
    pub y: i32,
    pub is_slave: bool, // true=加入 slave_list（死亡时清理）
}

/// 对玩家的 poison 施加（C# PoisonTarget）
#[derive(Debug, Clone)]
pub struct PoisonPlayer {
    pub session_id: u64,
    pub poison: Poison,
}

/// 推开玩家（C# MapObject.Pushed：沿 dir 推 distance 格，遇阻挡停止）
#[derive(Debug, Clone, Copy)]
pub struct PushPlayer {
    pub session_id: u64,
    pub dir: u8,
    pub distance: i32,
}

/// Boss 延迟攻击（C# DelayedAction DelayedType.Damage：到点对范围内玩家造成伤害）
#[derive(Debug, Clone, Copy)]
pub struct DelayedAttack {
    /// 相对当前 tick 的延迟（100ms/tick）
    pub delay_ticks: u64,
    pub center_x: i32,
    pub center_y: i32,
    pub radius: i32,
    pub damage: i32,
    pub attacker_oid: u32,
    pub map_index: u16,
}

/// AI 上下文（每 tick 每怪构建一次）
pub struct AiCtx<'a> {
    pub tick_count: u64,
    /// 当前怪物的 object_id（用于输出动作关联）
    pub monster_oid: u32,
    /// 当前怪物的 monster_index（Boss 注册用）
    pub monster_index: i32,
    /// 地图尺寸 (宽, 高)，用于全图随机传送等（缺省 200×200）
    pub map_size: (i32, i32),
    /// 龙系统当前等级（0=未激活；EvilMir DragonLink 攻击加成用）
    pub dragon_level: u8,
    /// 玩家快照（全图，behavior 自行按距离/map 过滤）
    pub players: &'a [PlayerSnap],
    /// 怪物快照（全图，供 Boss 互查）
    pub monsters: &'a [MonsterSnap],
    /// 输出：移动 (oid, x, y, dir)
    pub out_moves: &'a mut Vec<(u32, i32, i32, u8)>,
    /// 输出：攻击动作
    pub out_attacks: &'a mut Vec<AttackAction>,
    /// 输出：地面法术场
    pub out_spell_fields: &'a mut Vec<SpellFieldSpawn>,
    /// 输出：召唤物
    pub out_summons: &'a mut Vec<BossSummon>,
    /// 输出：怪物互疗 (target_oid, amount)
    pub out_heals: &'a mut Vec<(u32, i32)>,
    /// 输出：对玩家的 poison
    pub out_poisons: &'a mut Vec<PoisonPlayer>,
    /// 输出：推开玩家
    pub out_pushes: &'a mut Vec<PushPlayer>,
    /// 输出：传送玩家（C# Target.Teleport；session, x, y, dir）
    pub out_player_teleports: &'a mut Vec<(u64, i32, i32, u8)>,
    /// 输出：延迟攻击（C# DelayedAction DelayedType.Damage）
    pub out_delayed_attacks: &'a mut Vec<DelayedAttack>,
    /// 输出：怪物嘲讽（C# StoneTrap：target_oid 攻击 taunter_oid）→ monster_targets
    pub out_monster_taunts: &'a mut Vec<(u32, u32)>,
}

impl<'a> AiCtx<'a> {
    /// 查找范围内所有玩家（对齐 C# FindAllTargets(range, point, false)）
    pub fn find_targets_in_range(&self, cx: i32, cy: i32, radius: i32, map_index: u16) -> Vec<&PlayerSnap> {
        self.players.iter()
            .filter(|p| p.map_index == map_index)
            .filter(|p| {
                let dx = (p.x - cx).abs();
                let dy = (p.y - cy).abs();
                dx.max(dy) <= radius // 切比雪夫距离（对齐 C# MaxDistance）
            })
            .collect()
    }

    /// 查找最近玩家（对齐 C# FindTarget 的最近目标选取）
    pub fn nearest_target(&self, cx: i32, cy: i32, view_range: i32, map_index: u16) -> Option<&PlayerSnap> {
        self.players.iter()
            .filter(|p| p.map_index == map_index && p.hp > 0)
            .min_by_key(|p| {
                let dx = (p.x - cx).abs();
                let dy = (p.y - cy).abs();
                dx.max(dy) // 切比雪夫距离（对齐 C# MaxDistance，与过滤一致）
            })
            .filter(|p| {
                let dx = (p.x - cx).abs();
                let dy = (p.y - cy).abs();
                dx.max(dy) <= view_range
            })
    }
}
