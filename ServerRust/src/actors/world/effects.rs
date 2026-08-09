// 等级特效（C# HumanObject.LevelEffects / SetLevelEffects）
//
// flags 990-998 → LevelEffects 位掩码（Mist=1/RedDragon=2/BlueDragon=4/Rebirth1=8/...
// /Phoenix=256），NPC 脚本 REFRESHEFFECTS 触发刷新并广播 ObjectLevelEffects。

use super::*;

/// flags 990-998 → LevelEffects 位（C# HumanObject.SetLevelEffects）
const LEVEL_EFFECT_FLAGS: [(i32, u16); 9] = [
    (990, 1),   // Mist
    (991, 2),   // RedDragon
    (992, 4),   // BlueDragon
    (993, 8),   // Rebirth1
    (994, 16),  // Rebirth2
    (995, 32),  // Rebirth3
    (996, 64),  // NewBlue
    (997, 128), // YellowDragon
    (998, 256), // Phoenix
];

impl WorldActor {
    /// 根据 flags 990-998 刷新玩家等级特效（C# HumanObject.SetLevelEffects）
    pub(crate) async fn refresh_level_effects(&mut self, session_id: u64) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        let mut effects = 0u16;
        for (flag, bit) in LEVEL_EFFECT_FLAGS {
            // #1503：兼容两种键格式——@setflag 写入 NPC_FLAG_<n>，旧数据/脚本可能用纯数字键
            let plain = flag.to_string();
            let prefixed = format!("NPC_FLAG_{}", flag);
            let v = state.flags.get(&plain).copied().unwrap_or(0)
                .max(state.flags.get(&prefixed).copied().unwrap_or(0));
            if v != 0 {
                effects |= bit;
            }
        }
        let _ = record.actor_ref.ask(crate::actors::player::SetLevelEffects { effects }).await;
        debug!("LevelEffects: {} -> {:04x}", state.name, effects);
    }

    /// 每 50 ticks 刷新新手行会经验 buff（C# ProcessBuffs 维护 BuffType.Newbie：
    /// 行会 == Settings.NewbieGuild 且开关开启时获得 ExpRatePercent 加成）
    pub(crate) async fn tick_newbie_bonus(&mut self) {
        use super::*;
        if self.tick_count % 50 != 0 {
            return;
        }
        let (newbie_guild, enabled, _exp) = self.social_ref
            .ask(crate::actors::social::NpcGetNewbieGuildConfig)
            .await
            .unwrap_or(("NewbieGuild".to_string(), true, 5));
        let mut updates: Vec<(u64, bool)> = Vec::new();
        for (sid, record) in &self.players {
            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                let active = enabled && state.guild_name.as_deref() == Some(newbie_guild.as_str());
                if active != state.newbie_exp_bonus {
                    updates.push((*sid, active));
                }
            }
        }
        for (sid, active) in updates {
            if let Some(record) = self.players.get(&sid) {
                let _ = record.actor_ref.ask(crate::actors::player::SetNewbieExpBonus { active }).await;
            }
        }
    }

    /// 广播 ObjectLevelEffects（C# RefreshEffects：Enqueue 自己 + Broadcast 同图）
    pub(crate) async fn broadcast_level_effects(&self, session_id: u64) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        let packet = mir2_shared::packets::server::movement::ObjectLevelEffects {
            object_id: state.object_id,
            level_effects: state.level_effects,
        };
        let mut body = Vec::new();
        if packet.write_body(&mut body).is_ok() {
            let pkt = build_packet_bytes(
                mir2_shared::enums::ServerPacketIds::ObjectLevelEffects as i16, &body);
            for (sid, r) in &self.players {
                if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                    if os.map_index == state.map_index {
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: *sid, data: pkt.clone(),
                        }).await;
                    }
                }
            }
        }
    }
}

/// #963：刷新玩家等级特效（GM @SETFLAG 后即时生效，C# SetLevelEffects）
pub struct RefreshLevelEffects {
    pub session_id: u64,
}

impl Message<RefreshLevelEffects> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: RefreshLevelEffects, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.refresh_level_effects(msg.session_id).await;
    }
}
