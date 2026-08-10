//! WoomaTaurus（沃玛教主）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/WoomaTaurus.cs（继承 FlamingWooma）
//! 机制：
//!   - 7 阶段 HP：stage = HP / (MaxHP/7)，每掉一阶进入 8s 狂暴（加速移动/攻击）
//!     —— 任务要求"召唤沃玛"，原版无显式召唤，作为阶段触发的沃玛系召唤补充
//!   - TeleDelay=10s：若四周 8 格有 >=5 格被阻挡（被围困），TeleportRandom(4,0) 逃跑
//!   - 继承 FlamingWooma：近战火焰攻击
//!
//! ProcessAI（C# WoomaTaurus.cs:18-80）：tele/狂暴/阶段。
//! FlamingWooma 基类：近战 Type=0/1 火焰 MAC 攻击。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

/// 视野范围
const VIEW_RANGE: i32 = 20;
/// 近战判定（C# FlamingWooma InAttackRange 默认 1）
const MELEE_RANGE: i32 = 1;
/// 传送检测周期：10s（C# TeleDelay = 10000ms）
const TELE_CHECK_TICKS: u64 = 100;
/// 狂暴持续时间：8s（C# _madTime = Envir.Time + 8000）
const RAGE_DURATION_TICKS: u64 = 80;
/// 总阶段数（C# _stage = 7）
const TOTAL_STAGES: i32 = 7;
/// 每阶段召唤数
const SLAVES_PER_STAGE: usize = 3;
/// 召唤池（沃玛系：C# 沃玛洞穴 Wooma* 怪物）
const SLAVE_NAMES: [&str; 4] = [
    "FlamingWooma",
    "WoomaSoldier",
    "WoomaFighter",
    "WoomaGeneral",
];

pub struct WoomaTaurusBehavior {
    stage: i32,
    next_tele_tick: u64,
    /// 狂暴到期 tick（>0 = 狂暴中）
    rage_end_tick: u64,
    spawned: bool,
}

impl WoomaTaurusBehavior {
    pub fn new() -> Self {
        Self {
            stage: TOTAL_STAGES,
            next_tele_tick: 0,
            rage_end_tick: 0,
            spawned: false,
        }
    }

    fn current_stage(monster: &MonsterState) -> i32 {
        if monster.max_hp < TOTAL_STAGES {
            return TOTAL_STAGES;
        }
        let per_stage = monster.max_hp / TOTAL_STAGES;
        if per_stage <= 0 {
            return TOTAL_STAGES;
        }
        monster.hp / per_stage
    }
}

impl MonsterBehavior for WoomaTaurusBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if !self.spawned {
            self.next_tele_tick = ctx.tick_count + TELE_CHECK_TICKS;
            self.spawned = true;
        }

        // 狂暴到期检查（C# if _madTime>0 && Envir.Time>_madTime → RefreshAll）
        if self.rage_end_tick > 0 && ctx.tick_count >= self.rage_end_tick {
            self.rage_end_tick = 0;
        }

        // ---- 7 阶段 HP：阶段下降 → 狂暴 + 召唤沃玛 ----
        let cur_stage = Self::current_stage(monster);
        if cur_stage < self.stage {
            self.rage_end_tick = ctx.tick_count + RAGE_DURATION_TICKS;
            // 阶段触发召唤沃玛系小怪（任务核心机制）
            for i in 0..SLAVES_PER_STAGE {
                let dir = (i as usize) % 8;
                let name = SLAVE_NAMES[fastrand::usize(0..SLAVE_NAMES.len())];
                ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                    monster_name: name.to_string(),
                    x: monster.x + DIR_DX[dir] * 2,
                    y: monster.y + DIR_DY[dir] * 2,
                    is_slave: true,
                    summoner_oid: Some(monster.object_id),
                });
            }
            self.stage = cur_stage;
        }

        // ---- 传送逃围（C# TeleDelay 周期：周围 8 格被阻挡 >= 5 → TeleportRandom）----
        if ctx.tick_count >= self.next_tele_tick {
            self.next_tele_tick = ctx.tick_count + TELE_CHECK_TICKS;
            // C# WoomaTaurus.cs:28-51：数 8 邻格被阻挡（非法/墙/阻挡对象）>= 5 才逃
            let blocked = wooma_taurus_blocked_count(
                &ctx.is_walkable,
                ctx.players,
                ctx.monsters,
                monster.x,
                monster.y,
                monster.map_index,
            );
            if blocked >= 5 {
                // 逃跑传送：全图随机 walkable 格（C# TeleportRandom(4,0)）；
                // 推多个候选，tick 端校验 walkable，最后有效者生效
                let (mw, mh) = ctx.map_size;
                for _ in 0..10 {
                    ctx.out_moves.push((
                        monster.object_id,
                        fastrand::i32(0..mw.max(1)),
                        fastrand::i32(0..mh.max(1)),
                        monster.direction,
                    ));
                }
                return;
            }
        }

        // 无目标则返回
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);

        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        // ---- 近战火焰攻击（C# FlamingWooma.Attack）----
        if dist <= MELEE_RANGE && ctx.tick_count >= monster.next_attack_tick {
            // 狂暴期攻击冷却减半
            let cooldown = if self.rage_end_tick > 0 { 3 } else { 5 };
            monster.next_attack_tick = ctx.tick_count + cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                damage,
                spell_id: 0,
                attack_type: 0,
            });
        } else if dist > MELEE_RANGE && ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            // 狂暴期移动加速
            let move_cd = if self.rage_end_tick > 0 { 1 } else { 2 };
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + move_cd;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

/// #1834：C# WoomaTaurus 逃围判定——周围 8 格中“不可走（非法/墙）或玩家/怪物占用”的数量。
fn wooma_taurus_blocked_count(
    is_walkable: &dyn Fn(i32, i32) -> bool,
    players: &[crate::actors::world::ai::ctx::PlayerSnap],
    monsters: &[crate::actors::world::ai::ctx::MonsterSnap],
    x: i32,
    y: i32,
    map_index: u16,
) -> usize {
    let mut blocked = 0usize;
    for d in 0..8usize {
        let nx = x + DIR_DX[d];
        let ny = y + DIR_DY[d];
        if !is_walkable(nx, ny) {
            blocked += 1;
            continue;
        }
        if players.iter().any(|p| p.map_index == map_index && p.x == nx && p.y == ny) {
            blocked += 1;
            continue;
        }
        if monsters.iter().any(|m| m.map_index == map_index && m.x == nx && m.y == ny) {
            blocked += 1;
        }
    }
    blocked
}

#[cfg(test)]
mod tests {
    use super::wooma_taurus_blocked_count;
    use crate::actors::world::ai::ctx::{MonsterSnap, PlayerSnap};

    fn player(sid: u64, x: i32, y: i32) -> PlayerSnap {
        PlayerSnap { session_id: sid, x, y, hp: 100, map_index: 1, object_id: sid as u32, level: 30, pk_points: 0, min_dc: 10 }
    }
    fn monster(oid: u32, x: i32, y: i32) -> MonsterSnap {
        MonsterSnap { object_id: oid, x, y, hp: 10, max_hp: 10, map_index: 1, monster_index: 1 }
    }

    #[test]
    fn test_wooma_blocked_count_open_area() {
        // 开阔地无阻挡：8 格全部可走且无人 → 0
        let walkable = |_: i32, _: i32| true;
        assert_eq!(wooma_taurus_blocked_count(&walkable, &[], &[], 10, 10, 1), 0);
    }

    #[test]
    fn test_wooma_blocked_count_walls_and_invalid() {
        // 墙：4 面不可走 → 4；再加越界/对象
        let walkable = |x: i32, y: i32| !(x == 9 || x == 11 || y == 9 || y == 11);
        let players = [player(1, 10, 9), player(2, 11, 10)];
        let monsters = [monster(1, 10, 11)];
        // (10,10) 的 8 邻：(9,9)(10,9)(11,9)(9,10)(11,10)(9,11)(10,11)(11,11)
        // 墙：x==9 的 3 格(9,9)(9,10)(9,11) + x==11 的(11,9)(11,10)(11,11) + y==9 的(10,9) + y==11 的(10,11) → 8 全墙
        assert_eq!(wooma_taurus_blocked_count(&walkable, &players, &monsters, 10, 10, 1), 8);
    }

    #[test]
    fn test_wooma_blocked_count_occupied_cells() {
        // 可走但被玩家/怪物占用 → 计为阻挡
        let walkable = |_: i32, _: i32| true;
        let players = [player(1, 11, 10), player(2, 10, 11)];
        let monsters = [monster(1, 9, 10)];
        assert_eq!(wooma_taurus_blocked_count(&walkable, &players, &monsters, 10, 10, 1), 3);
        // 跨图对象不计
        let mut p = player(1, 11, 10);
        p.map_index = 2;
        assert_eq!(wooma_taurus_blocked_count(&walkable, &[p], &[], 10, 10, 1), 0);
    }
}
