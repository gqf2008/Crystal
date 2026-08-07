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

/// PvP 灰名标记（C# HumanObject.Attacked：受害者 PK<200 且不在开战时，攻击者 BrownTime=1 分钟）
pub struct MarkBrown {
    pub attacker_session: u64,
    pub victim_session: u64,
}

impl Message<MarkBrown> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: MarkBrown, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let victim = match self.players.get(&msg.victim_session) {
            Some(r) => r.clone(),
            None => return,
        };
        let victim_state = match victim.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        // C#：受害者 PK>=200（红名）时攻击者不灰名
        if victim_state.pk_points >= 200 {
            return;
        }
        let attacker = match self.players.get(&msg.attacker_session) {
            Some(r) => r.clone(),
            None => return,
        };
        // C# AtWar：双方行会处于开战状态则不灰名
        if let Some(victim_guild) = &victim_state.guild_name {
            if let Ok(Some(attacker_state)) = attacker.actor_ref.ask(GetPlayerState).await {
                if let Some(attacker_guild) = &attacker_state.guild_name {
                    if self.guild_wars.get(victim_guild)
                        .map(|set| set.contains(attacker_guild))
                        .unwrap_or(false)
                    {
                        return;
                    }
                }
            }
        }
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let _ = attacker.actor_ref.ask(crate::actors::player::SetBrownTime {
            until_ms: now_ms + 60_000, // C# Settings.Minute
        }).await;
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
                            // C# Functions.InRange = 切比雪夫
                            let dist = (os.x - state.x).abs().max((os.y - state.y).abs());
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

    // ===== 经验加成百分比缓存（#989 死锁修复）=====
    // AddExperience 原实现会反向 ask WorldActor（GetLoverExpBonus/GetMenteeExpBonus/
    // GetNewbieGuildConfig），当击杀经验在 WorldActor tick 内发放时形成互相等待死锁。
    // 这里在独立消息（ProcessElementalTick）中把加成缓存进 PlayerState，
    // AddExperience 只读缓存，不再反向 ask。
    const LOVER_EXP_BONUS: i32 = 5;   // C# Settings.LoverEXPBonus
    const MENTEE_EXP_BONUS: i32 = 10; // C# Settings.MentorExpBoost
    let newbie_cfg = world.social_ref
        .ask(crate::actors::social::NpcGetNewbieGuildConfig)
        .await
        .unwrap_or(("NewbieGuild".to_string(), true, 5));
    let mut exp_updates: Vec<(u64, i32, i32, i32)> = Vec::new();
    for (sid, record) in &world.players {
        if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
            // 配偶加成（C# GainExp：Lover 同图 + InRange(16) + 存活）
            let mut lover = 0i32;
            if let Some(spouse) = state.spouse_name.clone() {
                for (_, other) in &world.players {
                    if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                        if os.is_dead || !os.name.eq_ignore_ascii_case(&spouse) || os.map_index != state.map_index {
                            continue;
                        }
                        // C# Functions.InRange = 切比雪夫
                        if (os.x - state.x).abs().max((os.y - state.y).abs()) <= DATA_RANGE {
                            lover = LOVER_EXP_BONUS;
                            break;
                        }
                    }
                }
            }
            // 徒弟加成（C# GainExp：Mentee 同图 + InRange(16) + 同组 + 导师存活）
            let mut mentee = 0i32;
            if !state.is_mentor {
                if let Some(mentor_name) = state.mentor_name.clone() {
                    for (_, other) in &world.players {
                        if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                            if os.is_dead || !os.name.eq_ignore_ascii_case(&mentor_name) || os.map_index != state.map_index {
                                continue;
                            }
                            // C# Functions.InRange = 切比雪夫
                            if (os.x - state.x).abs().max((os.y - state.y).abs()) <= DATA_RANGE
                                && os.group_id.is_some() && os.group_id == state.group_id
                            {
                                mentee = MENTEE_EXP_BONUS;
                                break;
                            }
                        }
                    }
                }
            }
            let newbie = if state.newbie_exp_bonus { newbie_cfg.2 } else { 0 };
            if lover != state.exp_bonus_lover_percent
                || mentee != state.exp_bonus_mentee_percent
                || newbie != state.exp_bonus_newbie_percent
            {
                exp_updates.push((*sid, lover, mentee, newbie));
            }
        }
    }
    for (sid, lover, mentee, newbie) in exp_updates {
        if let Some(record) = world.players.get(&sid) {
            if let Ok(Some(mut st)) = record.actor_ref.ask(GetPlayerState).await {
                st.exp_bonus_lover_percent = lover;
                st.exp_bonus_mentee_percent = mentee;
                st.exp_bonus_newbie_percent = newbie;
                let _ = record.actor_ref.ask(crate::actors::player::SetPlayerState { state: st }).await;
            }
        }
    }
}
