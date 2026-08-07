// 配偶/师徒加成（C# PlayerObject.GainExp：Lover/Mentee 同图 + 近距离 + 存活才加经验）
//
// C# Settings.LoverEXPBonus = 5（默认）；Globals.DataRange = 16。
// Mentee 经验加成需 is_mentor 方向标记 + 同组判定，留作后续。

use super::*;

/// 查询配偶经验加成百分比（C# GainExp：HasBuff(Lover) && 配偶同图、InRange(16)、存活）
pub struct GetLoverExpBonus {
    pub session_id: u64,
}

impl Message<GetLoverExpBonus> for WorldActor {
    type Reply = i32;

    async fn handle(&mut self, msg: GetLoverExpBonus, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        // C# Settings.LoverEXPBonus 默认 5
        const LOVER_EXP_BONUS: i32 = 5;
        // C# Globals.DataRange = 16
        const DATA_RANGE: i32 = 16;

        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return 0,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return 0,
        };
        let Some(spouse) = state.spouse_name.clone() else {
            return 0;
        };
        for (_, other) in &self.players {
            if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                if os.is_dead || !os.name.eq_ignore_ascii_case(&spouse) {
                    continue;
                }
                if os.map_index != state.map_index {
                    continue;
                }
                let dist = (os.x - state.x).abs() + (os.y - state.y).abs();
                if dist > DATA_RANGE {
                    continue;
                }
                return LOVER_EXP_BONUS;
            }
        }
        0
    }
}

/// 查询徒弟经验加成百分比（C# GainExp：Mentee 同图 + InRange(16) + 同组 + 导师存活 → +MentorExpBoost%）
pub struct GetMenteeExpBonus {
    pub session_id: u64,
}

impl Message<GetMenteeExpBonus> for WorldActor {
    type Reply = i32;

    async fn handle(&mut self, msg: GetMenteeExpBonus, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        // C# Settings.MentorExpBoost 默认 10
        const MENTEE_EXP_BONUS: i32 = 10;
        // C# Globals.DataRange = 16
        const DATA_RANGE: i32 = 16;

        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return 0,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return 0,
        };
        // 只有徒弟（有导师且非导师）享受加成（C# Info.Mentor != 0 && !Info.IsMentor）
        let Some(mentor_name) = state.mentor_name.clone() else {
            return 0;
        };
        if state.is_mentor {
            return 0;
        }
        for (_, other) in &self.players {
            if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                if os.is_dead || !os.name.eq_ignore_ascii_case(&mentor_name) {
                    continue;
                }
                if os.map_index != state.map_index {
                    continue;
                }
                let dist = (os.x - state.x).abs() + (os.y - state.y).abs();
                if dist > DATA_RANGE {
                    continue;
                }
                // C#：需与导师同组
                if os.group_id.is_some() && os.group_id == state.group_id {
                    return MENTEE_EXP_BONUS;
                }
            }
        }
        0
    }
}

/// WorldActor 转发：查询新手行会配置（PlayerActor 无 social_ref，经 world 转发）
pub struct GetNewbieGuildConfig;

impl Message<GetNewbieGuildConfig> for WorldActor {
    type Reply = (String, bool, i32);

    async fn handle(&mut self, _msg: GetNewbieGuildConfig, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.social_ref
            .ask(crate::actors::social::NpcGetNewbieGuildConfig)
            .await
            .unwrap_or(("NewbieGuild".to_string(), true, 5))
    }
}

/// 每 50 ticks 刷新导师伤害加成（C# ProcessBuffs 维护 BuffType.Mentor +
/// MonsterObject.Attacked 的 MentorDamageRatePercent 判定：导师 + 徒弟近身同组存活）
pub(crate) async fn tick_partner_bonuses(world: &mut WorldActor) {
    use super::*;
    if world.tick_count % 50 != 0 {
        return;
    }
    const DATA_RANGE: i32 = 16;
    let mut updates: Vec<(u64, bool)> = Vec::new(); // (session, active)
    for (sid, record) in &world.players {
        if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
            // 只有导师且徒弟在线时可能激活
            let active = if state.is_mentor {
                if let Some(mentee_name) = state.mentor_name.clone() {
                    let mut found = false;
                    for (_, other) in &world.players {
                        if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                            if os.is_dead || !os.name.eq_ignore_ascii_case(&mentee_name) {
                                continue;
                            }
                            if os.map_index != state.map_index {
                                continue;
                            }
                            let dist = (os.x - state.x).abs() + (os.y - state.y).abs();
                            if dist > DATA_RANGE {
                                continue;
                            }
                            // C#：徒弟需与导师同组（MonsterObject.Attacked 中 GroupMembers.Contains）
                            if os.group_id.is_some() && os.group_id == state.group_id {
                                found = true;
                                break;
                            }
                        }
                    }
                    found
                } else {
                    false
                }
            } else {
                false
            };
            if active != state.mentor_damage_bonus {
                updates.push((*sid, active));
            }
        }
    }
    for (sid, active) in updates {
        if let Some(record) = world.players.get(&sid) {
            let _ = record.actor_ref.ask(crate::actors::player::SetMentorDamageBonus { active }).await;
        }
    }
}
