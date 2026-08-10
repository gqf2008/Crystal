//! AI 上下文 + 输出动作（借用隔离，复用 tick_monsters 的输出队列模式）
//!
//! behavior 通过 AiCtx 读取怪物自身状态 + 玩家快照，通过输出队列推动作，
//! 循环外由 tick_monsters 统一应用（避免 &mut self.monsters 借用冲突）。

use mir2_shared::enums::{PetMode, Spell};
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
    /// #1385：PK 值（C# PKPoints；守卫红名目标判定用）
    pub pk_points: i32,
    /// #1828：最小攻击力（C# MinDC；DarkCaptain/SnowWolfKing 选更弱目标用）
    pub min_dc: i32,
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
    /// 弧形攻击（C# HalfmoonAttack/ThreeQuarterMoonAttack：从 PreviousDir(direction) 起连续 count 个方向、距离 1）
    Arc {
        attacker_oid: u32,
        center_x: i32,
        center_y: i32,
        direction: u8,
        count: u8,
        damage: i32,
        spell_id: u8,
        attack_type: u8,
    },
    /// 锥形攻击（C# TriangleAttack：沿 direction 每行 center + Left/Right 扩展，limit_width 限制单侧格数；-1=不限）
    Triangle {
        attacker_oid: u32,
        center_x: i32,
        center_y: i32,
        direction: u8,
        distance: u8,
        limit_width: i32,
        damage: i32,
        spell_id: u8,
        attack_type: u8,
    },
    /// 精确格集合攻击（C# 自定义几何：behavior 按 C# 逐格算好 cells，tick 只按集合过滤）
    Cells {
        attacker_oid: u32,
        center_x: i32,
        center_y: i32,
        cells: Vec<(i32, i32)>,
        damage: i32,
        spell_id: u8,
        attack_type: u8,
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
    /// #1434：召唤者（master）object_id；is_slave=true 时用于登记 SlaveList（master 死亡级联清理）
    pub summoner_oid: Option<u32>,
}

/// #1437：TrapRock 子岩生成（C# TrapRock.Show：目标四角生成 ChildRock，立即可见、同目标）
#[derive(Debug, Clone)]
pub struct ChildRockSpawn {
    pub monster_name: String,
    pub x: i32,
    pub y: i32,
    pub target_session: u64,
    pub target_x: i32,
    pub target_y: i32,
    pub parent_oid: u32,
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

/// Boss 延迟攻击（C# DelayedAction DelayedType.Damage：到点对指定格集合内玩家造成伤害）
#[derive(Debug, Clone)]
pub struct DelayedAttack {
    /// 相对当前 tick 的延迟（100ms/tick）
    pub delay_ticks: u64,
    pub center_x: i32,
    pub center_y: i32,
    /// 精确命中格集合（C# FullmoonAttack distance=2 等自定义几何）
    pub cells: Vec<(i32, i32)>,
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
    /// #1834：地图可行走判定（越界返回 false，对应 C# ValidPoint/walkable）
    pub is_walkable: &'a dyn Fn(i32, i32) -> bool,
    /// 龙系统当前等级（0=未激活；EvilMir DragonLink 攻击加成用）
    pub dragon_level: u8,
    /// 玩家快照（全图，behavior 自行按距离/map 过滤）
    pub players: &'a [PlayerSnap],
    /// 怪物快照（全图，供 Boss 互查）
    pub monsters: &'a [MonsterSnap],
    /// #1396：怪物 index → 名称映射（FloatingRock 克隆目标解析用）
    pub monster_name_by_index: &'a std::collections::HashMap<i32, String>,
    /// #1441：当前怪物存活的 slave 数（C# SlaveList.Count；tick 预计算）
    pub slave_count: usize,
    /// 输出：移动 (oid, x, y, dir)
    pub out_moves: &'a mut Vec<(u32, i32, i32, u8)>,
    /// 输出：后跳 (oid, dir, max_distance)（#1801：C# S.ObjectBackStep）
    pub out_backsteps: &'a mut Vec<(u32, u8, i32)>,
    /// 输出：攻击动作
    pub out_attacks: &'a mut Vec<AttackAction>,
    /// 输出：地面法术场
    pub out_spell_fields: &'a mut Vec<SpellFieldSpawn>,
    /// 输出：召唤物
    pub out_summons: &'a mut Vec<BossSummon>,
    /// 输出：TrapRock 子岩（#1437）
    pub out_child_rocks: &'a mut Vec<ChildRockSpawn>,
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
    /// 输出：怪物自传送 (oid, x, y)（C# TeleportRandom/Teleport：RedFoxman/WhiteFoxman 等）
    pub out_monster_teleports: &'a mut Vec<(u32, i32, i32)>,
    /// 输出：给玩家加 buff (session, BuffInstance)（C# AddBuff：YinDevilNode/PowerBead 等）
    pub out_player_buffs: &'a mut Vec<(u64, crate::combat::buff::BuffInstance)>,
    /// 输出：对象显示/隐藏广播（C# ObjectShow/ObjectHide，如 Shinsu 形态切换）
    pub out_show_hide: &'a mut Vec<(u32, bool)>,
    /// 输出：坐下/起身广播（#1354 C# ObjectSitDown；oid, x, y, dir, sitting）
    pub out_sit_down: &'a mut Vec<(u32, i32, i32, u8, bool)>,
    /// 输出：对象特效广播（#1364 C# ObjectEffect；oid, SpellEffect——如 DeathCrawlerBreath）
    pub out_effects: &'a mut Vec<(u32, mir2_shared::enums::SpellEffect)>,
    /// 输出：净化玩家毒（#1391 C# PowerBead Effect==1；session——PlayerActor.PurifyPoisons）
    pub out_player_purges: &'a mut Vec<u64>,
    /// 输出：对玩家回血（session, amount；C# MasterVampire / Healer 治疗玩家）
    pub out_player_heals: &'a mut Vec<(u64, i32)>,
    /// 当前怪物的宠物等级（C# MonsterObject.PetLevel；非宠物=0）
    pub pet_level: i32,
    /// 主人宠物模式（C# Master.PMode；非宠物=None）
    pub master_pet_mode: Option<PetMode>,
    /// 主人的当前目标（仅当目标是玩家快照时；C# Master.Target）
    pub master_target: Option<PlayerSnap>,
    /// 主人是否正在攻击怪物（#471 pet_targets 协战目标；Shinsu 形态切换用）
    pub has_master_monster_target: bool,
}

impl<'a> AiCtx<'a> {
    /// 宠物目标选择（C# MonsterObject.ProcessSearch/ProcessTarget + Master.PMode）：
    /// - 非宠物：正常怪，最近玩家目标
    /// - FocusMasterTarget：只攻击主人目标（无法解析则无目标 → 跟随主人）
    /// - MoveOnly/None：不攻击（跟随主人）
    /// - Both/AttackOnly：不自主攻击玩家（#471：由 pet_targets 协战打主人攻击的怪物）
    pub fn pet_target(&self, x: i32, y: i32, view_range: i32, map_index: u16) -> Option<PlayerSnap> {
        match self.master_pet_mode {
            None => self.nearest_target(x, y, view_range, map_index).copied(),
            Some(PetMode::MoveOnly) | Some(PetMode::None) => None,
            Some(PetMode::FocusMasterTarget) => self.master_target.filter(|t| t.map_index == map_index),
            Some(PetMode::Both) | Some(PetMode::AttackOnly) => None,
        }
    }

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

    /// 弧形/自定义格命中目标（C# HalfmoonAttack 每格取第一个可攻击对象；此处近似为格内全部玩家）
    pub fn find_targets_in_cells(&self, cells: &[(i32, i32)], map_index: u16) -> Vec<&PlayerSnap> {
        self.players.iter()
            .filter(|p| p.map_index == map_index)
            .filter(|p| cells.contains(&(p.x, p.y)))
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
