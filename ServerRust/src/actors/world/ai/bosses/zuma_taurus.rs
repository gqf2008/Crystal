//! ZumaTaurus（祖玛教主/祖玛金牛）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/ZumaTaurus.cs（继承 ZumaMonster）
//! 机制：
//!   - 7 阶段 HP：stage = HP / (MaxHP / 7)，每掉一阶 SpawnSlaves（召唤 Zuma 小怪）
//!   - 继承 ZumaMonster（AvoidFireWall=false，即不主动躲避火墙）
//!   - 纯近战攻击（MACAgility 防御判定，C# DefenceType.MACAgility）
//!
//! SpawnSlaves（C# ZumaTaurus.cs:55-96）：
//!   count = min(8, 40 - SlaveList.Count)，从 Zuma1..Zuma7 中随机选取。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

/// 视野范围（C# ViewRange 用于寻敌）
const VIEW_RANGE: i32 = 20;
/// 近战判定距离（C# 标准 MonsterObject InAttackRange 默认 1，但追击后贴身即攻击）
const MELEE_RANGE: i32 = 1;
/// 总阶段数（C# _stage 初始 = 7）
const TOTAL_STAGES: i32 = 7;
/// 每阶段 SpawnSlaves 召唤上限（C# count = min(8, 40 - SlaveList.Count)）
const SLAVES_PER_STAGE: usize = 8;
/// 召唤池（C# Settings.Zuma1..Zuma7）
const SLAVE_NAMES: [&str; 7] = [
    "ZumaStatue",    // Zuma1
    "ZumaGuardian",  // Zuma2
    "ZumaArcher",    // Zuma3
    "WedgeMoth",     // Zuma4
    "ZumaArcher3",   // Zuma5
    "ZumaStatue3",   // Zuma6
    "ZumaGuardian3", // Zuma7
];

pub struct ZumaTaurusBehavior {
    /// 当前 HP 阶段（对齐 C# _stage，初始 7 = 满血）
    stage: i32,
}

impl ZumaTaurusBehavior {
    pub fn new() -> Self {
        Self { stage: TOTAL_STAGES }
    }

    /// 根据当前 HP 计算阶段（对齐 C# stage = HP / (Stats[HP] / 7)）
    fn current_stage(monster: &MonsterState) -> i32 {
        if monster.max_hp < TOTAL_STAGES {
            return TOTAL_STAGES;
        }
        let per_stage = monster.max_hp / TOTAL_STAGES;
        if per_stage <= 0 {
            return TOTAL_STAGES;
        }
        // C# byte stage = (byte)(HP / (Stats[Stat.HP] / 7))
        monster.hp / per_stage
    }
}

impl MonsterBehavior for ZumaTaurusBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // ---- 7 阶段 HP 召唤（C# ZumaTaurus.cs:16-30 ProcessAI）----
        let cur_stage = Self::current_stage(monster);
        if cur_stage < self.stage {
            // C# 阶段下降时 SpawnSlaves()
            self.spawn_slaves(monster, ctx);
            self.stage = cur_stage;
        }

        // 无目标时不行动（C# ProcessTarget：Players.Count==0 或 Target==null 直接 return）
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);

        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        // ---- 纯近战攻击（C# ZumaTaurus.cs:32-53 Attack）----
        if dist <= MELEE_RANGE {
            if ctx.tick_count < monster.next_attack_tick {
                return;
            }
            monster.next_attack_tick = ctx.tick_count + 5;
            // C# DefenceType.MACAgility：伤害用 DC（magic defence 判定由 attack 应用层处理）
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                damage,
                spell_id: 0,
                attack_type: 0,
            });
        } else if ctx.tick_count >= monster.next_move_tick {
            // 追击（C# 标准 MoveTo；AvoidFireWall=false 表示不绕火墙）
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

impl ZumaTaurusBehavior {
    /// 召唤 Zuma 小怪（对齐 C# SpawnSlaves：count = min(8, 40-SlaveList.Count)，随机 Zuma1..7）
    fn spawn_slaves(&self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        for i in 0..SLAVES_PER_STAGE {
            // C# switch(Random(7))：从 Zuma1..7 中随机
            let name = SLAVE_NAMES[fastrand::usize(0..SLAVE_NAMES.len())];
            // 散布在自身周围（C# Front 失败回退 CurrentLocation）
            let dir = (i as usize) % 8;
            ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                monster_name: name.to_string(),
                x: monster.x + DIR_DX[dir] * ((i / 8) as i32 + 1),
                y: monster.y + DIR_DY[dir] * ((i / 8) as i32 + 1),
                is_slave: true, // 加入 slave_list，Boss 死亡时清理
            });
        }
    }
}
