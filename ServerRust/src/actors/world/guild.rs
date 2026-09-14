use super::*;

pub struct RequestUserNameMsg {
    pub session_id: u64,
    pub object_id: u32,
}

impl Message<RequestUserNameMsg> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: RequestUserNameMsg, _ctx: &mut Context<Self, Self::Reply>) {
        let name = if let Some(npc) = self.npcs.get(&msg.object_id) {
            Some(npc.name.clone())
        } else if let Some(mon) = self.monsters.get(&msg.object_id) {
            Some(mon.name.clone())
        } else {
            for record in self.players.values() {
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    if state.object_id == msg.object_id {
                        // Found — send UserName response
                        let mut body = Vec::new();
                        body.extend_from_slice(&msg.object_id.to_le_bytes());
                        crate::util::wire::write_dotnet_string(&mut body, &state.name);
                        let _ = self
                            .gate_ref
                            .tell(SendToClient {
                                session_id: msg.session_id,
                                data: build_packet_bytes(
                                    mir2_shared::enums::ServerPacketIds::UserName as i16,
                                    &body,
                                ),
                            })
                            .await;
                        return;
                    }
                }
            }
            None
        };

        if let Some(name) = name {
            let mut body = Vec::new();
            body.extend_from_slice(&msg.object_id.to_le_bytes());
            crate::util::wire::write_dotnet_string(&mut body, &name);
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::UserName as i16,
                        &body,
                    ),
                })
                .await;
        }
    }
}

pub struct RequestChatItemMsg {
    pub session_id: u64,
    pub unique_id: u64,
}

impl Message<RequestChatItemMsg> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: RequestChatItemMsg, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let item_info = record
            .actor_ref
            .ask(crate::actors::player::GetItemInfo {
                unique_id: msg.unique_id,
            })
            .await
            .ok()
            .flatten();

        if let Some(item) = item_info {
            let mut stats_parts = Vec::new();
            if let Some(ref info) = item.info {
                stats_parts.push(info.name.clone());
                for (stat, value) in info.stats.iter() {
                    if value != 0 {
                        stats_parts.push(format!("{:?}: {}", stat, value));
                    }
                }
                if item.current_dura > 0 || info.durability > 0 {
                    stats_parts.push(format!("Dur: {}/{}", item.current_dura, info.durability));
                }
            } else {
                stats_parts.push(format!("Item#{}", item.item_index));
            }
            let stats_str = stats_parts.join(", ");
            let mut body = Vec::new();
            body.extend_from_slice(&msg.unique_id.to_le_bytes());
            crate::util::wire::write_dotnet_string(&mut body, &stats_str);
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ChatItemStats as i16,
                        &body,
                    ),
                })
                .await;
        }
    }
}

// ============================================================
// 轮回系统
// ============================================================

pub struct AcceptReincarnationRequest {
    pub session_id: u64,
}

impl Message<AcceptReincarnationRequest> for WorldActor {
    type Reply = ();
    async fn handle(
        &mut self,
        msg: AcceptReincarnationRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) {
        // AcceptReincarnation: dead player accepts reincarnation from host.
        // C#: if ReincarnationHost != null && ReincarnationHost.ReincarnationReady -> Revive(HP/2)
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // Check if this player has a valid reincarnation host
        if state.reincarnation_host.is_none() {
            debug!("AcceptReincarnation: {} has no host", state.name);
            return;
        }

        let host_session = state.reincarnation_host.unwrap();
        // Verify host is still online and ready
        if !self.players.contains_key(&host_session) {
            debug!("AcceptReincarnation: host disconnected for {}", state.name);
            let _ = record.actor_ref.ask(ClearReincarnation).await;
            return;
        }

        debug!(
            "AcceptReincarnation: {} accepted from host session={}",
            state.name, host_session
        );

        // Revive the dead player at half HP
        let _ = record.actor_ref.ask(ReviveAtHalfHp).await;
        // #222：与 TownRevive 同款收尾——S.Revived 清除客户端死亡态 + ObjectRevived 广播
        let _ = self
            .gate_ref
            .tell(crate::gate::actor::SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Revived as i16, &[]),
            })
            .await;
        let mut obj_body = Vec::new();
        obj_body.extend_from_slice(&state.object_id.to_le_bytes());
        obj_body.push(1u8); // effect
        let revived_packet = build_packet_bytes(
            mir2_shared::enums::ServerPacketIds::ObjectRevived as i16,
            &obj_body,
        );
        for sid in self.players.keys() {
            let _ = self
                .gate_ref
                .tell(crate::gate::actor::SendToClient {
                    session_id: *sid,
                    data: revived_packet.clone(),
                })
                .await;
        }
        // 从死亡队列移除（避免自动复活覆盖）
        self.player_death_queue.remove(&msg.session_id);

        // Clear reincarnation state on both players
        let _ = record.actor_ref.ask(ClearReincarnation).await;
        if let Some(host_record) = self.players.get(&host_session) {
            let _ = host_record.actor_ref.ask(ClearReincarnationHost).await;
        }
    }
}

pub struct CancelReincarnationRequest {
    pub session_id: u64,
}

impl Message<CancelReincarnationRequest> for WorldActor {
    type Reply = ();
    async fn handle(
        &mut self,
        msg: CancelReincarnationRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) {
        // CancelReincarnation: dead player cancels reincarnation.
        // C#: ReincarnationExpireTime = Envir.Time (immediate expiry triggers cleanup)
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        debug!("CancelReincarnation: {}", state.name);

        // Set expire time to now, triggering immediate cleanup
        let _ = record.actor_ref.ask(ClearReincarnation).await;

        // Also notify host to clear their state
        if let Some(host_session) = state.reincarnation_host {
            if let Some(host_record) = self.players.get(&host_session) {
                let _ = host_record.actor_ref.ask(ClearReincarnationHost).await;
            }
        }
    }
}

// ============================================================
// 行会战/领地
// ============================================================

/// 行会战争键（排序后行会对，保证双向唯一）
fn war_key(a: &str, b: &str) -> (String, String) {
    if a < b {
        (a.to_string(), b.to_string())
    } else {
        (b.to_string(), a.to_string())
    }
}

pub struct GuildWarReturnRequest {
    pub session_id: u64,
    pub guild_name: String,
}

impl Message<GuildWarReturnRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: GuildWarReturnRequest, _ctx: &mut Context<Self, Self::Reply>) {
        self.declare_guild_war(msg.session_id, msg.guild_name.clone())
            .await;
    }
}

impl WorldActor {
    /// 行会宣战（C# PlayerObject STARTWAR / GuildWarReturn：会长校验、费用、新手行会禁止；@startwar 复用）
    pub(crate) async fn declare_guild_war(&mut self, session_id: u64, guild_name: String) {
        // GuildWarReturn: query if a guild exists and return its war status
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        debug!(
            "GuildWarReturn: {} querying guild={}",
            state.name, guild_name
        );

        if state.guild_name.is_none() {
            send_system_message(&self.gate_ref, session_id, "你还没有加入行会");
            return;
        }

        let sender_guild = state.guild_name.as_ref().unwrap();
        if guild_name == *sender_guild {
            send_system_message(&self.gate_ref, session_id, "不能向自己的行会宣战");
            return;
        }

        // 行会信息由 SocialActor 管理，此处仅做简单校验
        if guild_name.is_empty() {
            send_system_message(&self.gate_ref, session_id, "行会名称无效");
            return;
        }

        // C# requires guild leader (rank 0) to declare war
        if state.guild_rank != GuildRank::Leader {
            send_system_message(&self.gate_ref, session_id, "只有行会会长才能宣战");
            return;
        }

        // C# GoToWar：目标行会必须存在
        let exists = self
            .social_ref
            .ask(crate::actors::social::NpcGuildExists {
                guild_name: guild_name.clone(),
            })
            .await
            .unwrap_or(false);
        if !exists {
            send_system_message(&self.gate_ref, session_id, "目标行会不存在");
            return;
        }
        // C# GuildObject.GoToWar / PlayerObject.GuildWarReturn：不能向新手行会宣战（Settings.NewbieGuild）
        let (newbie_guild, _, _) = self
            .social_ref
            .ask(crate::actors::social::NpcGetNewbieGuildConfig)
            .await
            .unwrap_or(("NewbieGuild".to_string(), true, 5i32));
        if guild_name.eq_ignore_ascii_case(&newbie_guild) {
            send_system_message(&self.gate_ref, session_id, "不能向新手行会宣战");
            return;
        }
        // C#：已在战争中不可重复宣战
        if self
            .guild_wars
            .get(sender_guild)
            .map(|s| s.contains(&guild_name))
            .unwrap_or(false)
        {
            send_system_message(&self.gate_ref, session_id, "你们已与该行会开战");
            return;
        }
        // C# 宣战费用（Settings.Guild_WarCost=3000）：行会金币不足拒绝
        let (war_cost, war_time) = self
            .social_ref
            .ask(crate::actors::social::NpcGetGuildWarSettings)
            .await
            .unwrap_or((3000u32, 180i64));
        let deducted = self
            .social_ref
            .ask(crate::actors::social::GuildDeductGold {
                guild_name: sender_guild.clone(),
                amount: war_cost as u64,
            })
            .await
            .unwrap_or(false);
        if !deducted {
            send_system_message(
                &self.gate_ref,
                session_id,
                &format!("行会金币不足，宣战需要 {} 金币", war_cost),
            );
            return;
        }

        // C#：扣费成功后 MyGuild.SendServerPacket(GuildStorageGoldChange{Type=2, Name=宣战者, Amount=费用})
        self.send_guild_storage_gold_change_to_guild(sender_guild, &state.name, war_cost, 2)
            .await;

        // Record the war declaration
        self.guild_wars
            .entry(sender_guild.clone())
            .or_default()
            .insert(guild_name.clone());
        self.guild_wars
            .entry(guild_name.clone())
            .or_default()
            .insert(sender_guild.clone());
        // #2138：宣战后同步战争状态镜像到 SocialActor（LeaveGuildRequest 退会/解散校验）
        let _ = self
            .social_ref
            .ask(crate::actors::social::NpcSetGuildWar {
                guild_name: sender_guild.clone(),
                other: guild_name.clone(),
                at_war: true,
            })
            .await;
        // C# GuildAtWar.TimeRemaining = Settings.Minute * Guild_WarTime（单位分钟）
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        self.guild_war_ends
            .insert(war_key(sender_guild, &guild_name), now + war_time * 60);

        // Notify all online members of the declaring guild
        let war_msg = format!("行会 {} 已向 {} 宣战！", sender_guild, guild_name);
        for (sid, rec) in &self.players {
            if *sid == session_id {
                continue;
            }
            if let Ok(Some(s)) = rec.actor_ref.ask(GetPlayerState).await {
                if s.guild_name.as_deref() == Some(sender_guild.as_str()) {
                    send_system_message(&self.gate_ref, *sid, &war_msg);
                }
            }
        }

        // Notify all online members of the target guild
        let target_msg = format!("行会 {} 已向你们宣战！", sender_guild);
        for (sid, rec) in &self.players {
            if let Ok(Some(s)) = rec.actor_ref.ask(GetPlayerState).await {
                if s.guild_name.as_deref() == Some(guild_name.as_str()) {
                    send_system_message(&self.gate_ref, *sid, &target_msg);
                }
            }
        }

        // C# GoToWar：双方 UpdatePlayersColours（在线成员即时刷新名字颜色）
        self.refresh_guild_war_colours(sender_guild, &guild_name)
            .await;

        // #2892 批C：**删除**「回送 GuildRequestWar」——C# `PlayerObject.GuildWarReturn`（`:10283-10325`）
        // 宣战成功后只给自己发聊天（`YouStartedWarWith`）、给对方行会发聊天，
        // 不再回送提示框（`S.GuildRequestWar` 仅由 NPC `RequestWarKey` 发出，见 `npc.rs:492`）。
        // 原先回送会让客户端在宣战后再次弹出取名框（Bevy 自造流程的遗留）。
        send_system_message(
            &self.gate_ref,
            session_id,
            &format!("已向 {} 行会宣战", guild_name),
        );
    }
}

pub struct GuildBuffUpdateRequest {
    pub session_id: u64,
    /// C# C.GuildBuffUpdate.Action（0=请求列表 1=启用 2=激活）
    pub action: u8,
    /// C# C.GuildBuffUpdate.Id
    pub buff_id: u32,
}

impl Message<GuildBuffUpdateRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: GuildBuffUpdateRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        debug!("GuildBuffUpdate: {} buff_id={}", state.name, msg.buff_id);

        let Some(guild_name) = &state.guild_name else {
            send_system_message(&self.gate_ref, msg.session_id, "你还没有加入行会");
            return;
        };

        // action=0 means "request list" - send current active buffs（C# GuildBuffUpdate）
        if msg.action == 0 {
            let buffs = self.guild_buffs(guild_name).await;
            self.send_guild_buff_list(msg.session_id, &buffs).await;
            return;
        }

        // 激活/停用需要 Leader/Officer（对齐 C# GuildRankOptions.CanActivateBuff）
        if state.guild_rank != crate::actors::guild::GuildRank::Leader
            && state.guild_rank != crate::actors::guild::GuildRank::Officer
        {
            send_system_message(&self.gate_ref, msg.session_id, "没有权限激活行会 Buff");
            return;
        }

        // 切换 buff 激活状态（C# GuildBuffUpdate enable/activate）
        let mut buffs = self.guild_buffs(guild_name).await;
        // #2571：随本次变更同步到 SocialActor 的时限记录（unix 毫秒；停用清时限）
        let mut expiry_updates: Vec<(u32, Option<i64>)> = Vec::new();
        if buffs.contains(&msg.buff_id) {
            buffs.retain(|b| *b != msg.buff_id);
            if let Some(exp) = self.guild_buff_expiries.get_mut(guild_name) {
                exp.remove(&msg.buff_id);
            }
            expiry_updates.push((msg.buff_id, None));
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                &format!("行会 Buff #{} 已停用", msg.buff_id),
            );
        } else {
            // C# PlayerObject.GuildBuffUpdate（:10375-10408）：购买校验
            let Some(info) = self.guild_buff_infos.get(&msg.buff_id).cloned() else {
                send_system_message(&self.gate_ref, msg.session_id, "行会 Buff 不存在");
                return;
            };
            let (level, spare) = self
                .social_ref
                .ask(crate::actors::social::NpcGetGuildLevelSparePoints {
                    session_id: msg.session_id,
                })
                .await
                .unwrap_or((0, 0));
            if (level as u32) < info.level_req {
                send_system_message(
                    &self.gate_ref,
                    msg.session_id,
                    &format!("行会等级不足（需要 {} 级）", info.level_req),
                );
                return;
            }
            if (spare as u32) < info.points_req {
                send_system_message(
                    &self.gate_ref,
                    msg.session_id,
                    &format!("行会剩余点数不足（需要 {}）", info.points_req),
                );
                return;
            }
            let gold_cost = if info.time_limit_minutes > 0 && info.activation_cost > 0 {
                info.activation_cost
            } else {
                0
            };
            if gold_cost > 0 {
                let gold = self
                    .social_ref
                    .ask(crate::actors::social::NpcGetGuildGold {
                        session_id: msg.session_id,
                    })
                    .await
                    .unwrap_or(0);
                if gold < gold_cost {
                    send_system_message(&self.gate_ref, msg.session_id, "行会资金不足");
                    return;
                }
            }
            // C# NewBuff charge（:948-958）：扣点数 + 金币
            let _ = self
                .social_ref
                .ask(crate::actors::social::NpcGuildBuffCharge {
                    session_id: msg.session_id,
                    points: info.points_req,
                    gold: gold_cost as u32,
                })
                .await;
            // #2136：C# GuildObject.Process 时限——激活时记录到期时间（TimeLimit 分钟，
            // unix 毫秒存档形态；#2571 重启后经 guilds.buffs_json 恢复）
            if info.time_limit_minutes > 0 {
                let expires_ms = crate::db::now_unix_ms() + info.time_limit_minutes as i64 * 60_000;
                self.guild_buff_expiries
                    .entry(guild_name.clone())
                    .or_default()
                    .insert(msg.buff_id, expires_ms);
                expiry_updates.push((msg.buff_id, Some(expires_ms)));
            }
            buffs.push(msg.buff_id);
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                &format!("行会 Buff #{} 已激活", msg.buff_id),
            );
        }
        self.set_guild_buffs(guild_name, &buffs, &expiry_updates)
            .await;

        // 广播给同公会在线成员
        let online: Vec<u64> = self.players.keys().copied().collect();
        for sid in online {
            if let Some(r) = self.players.get(&sid) {
                if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                    if os.guild_name.as_deref() == Some(guild_name.as_str()) {
                        self.send_guild_buff_list(sid, &buffs).await;
                    }
                }
            }
        }
        debug!(
            "GuildBuffUpdate: {} toggled buff {} (active={:?})",
            state.name, msg.buff_id, buffs
        );
    }
}

impl WorldActor {
    /// #1340：向行会全体在线成员广播行会仓库金币变更（C# GuildObject.SendServerPacket；
    /// wire 对齐 C# S.GuildStorageGoldChange：[Amount u32][Type u8][Name dotnet]）
    pub(crate) async fn send_guild_storage_gold_change_to_guild(
        &self,
        guild_name: &str,
        actor_name: &str,
        amount: u32,
        change_type: u8,
    ) {
        let data = build_packet_bytes(
            mir2_shared::enums::ServerPacketIds::GuildStorageGoldChange as i16,
            &guild_storage_gold_change_body(amount, change_type, actor_name),
        );
        for (sid, rec) in &self.players {
            if let Ok(Some(s)) = rec.actor_ref.ask(GetPlayerState).await {
                if s.guild_name.as_deref() == Some(guild_name) {
                    let _ = self
                        .gate_ref
                        .tell(SendToClient {
                            session_id: *sid,
                            data: data.clone(),
                        })
                        .await;
                }
            }
        }
    }

    /// #1340：宣战/停战时对双方在线成员即时刷新名字颜色（C# UpdatePlayersColours：
    /// 给双方成员 Enqueue ColourChanged + BroadcastInfo）
    pub(crate) async fn refresh_guild_war_colours(&mut self, guild_a: &str, guild_b: &str) {
        let members: Vec<u64> = self.players.keys().copied().collect();
        for sid in members {
            let Some(rec) = self.players.get(&sid).cloned() else {
                continue;
            };
            let Ok(Some(s)) = rec.actor_ref.ask(GetPlayerState).await else {
                continue;
            };
            if s.guild_name.as_deref() == Some(guild_a) || s.guild_name.as_deref() == Some(guild_b)
            {
                self.broadcast_viewer_colours(sid).await;
            }
        }
    }

    /// 读取行会激活的 Buff 列表
    pub(crate) async fn guild_buffs(&self, guild_name: &str) -> Vec<u32> {
        self.social_ref
            .ask(crate::actors::social::NpcGetGuildBuffs {
                guild_name: guild_name.to_string(),
            })
            .await
            .unwrap_or_default()
    }

    /// 写入行会激活的 Buff 列表（#2571：expiry_updates 同步时限记录并随 guilds.buffs_json 落库）
    pub(crate) async fn set_guild_buffs(
        &self,
        guild_name: &str,
        buffs: &[u32],
        expiry_updates: &[(u32, Option<i64>)],
    ) {
        let _ = self
            .social_ref
            .ask(crate::actors::social::NpcSetGuildBuffs {
                guild_name: guild_name.to_string(),
                buffs: buffs.to_vec(),
                expiry_updates: expiry_updates.to_vec(),
            })
            .await;

        // #2174：C# 激活/停用后 RefreshStats 即时生效——立即刷新该行会在线成员加成缓存
        //（原由 tick_partner_bonuses 每 50 tick 刷新，最长延迟 ~5s；此处幂等，tick 兜底保留）
        let (mut exp, mut fish, mut mine) = (0i32, 0i32, 0i32);
        for info in self.guild_buff_infos.values() {
            if buffs.contains(&info.id) {
                exp += info.buff_exp_rate;
                fish += info.buff_fish_rate;
                mine += info.buff_mine_rate;
            }
        }
        // #2310：C# RefreshGuildBuffs——行会激活 Buff 全属性（AC/DC/MC/SC/HP/MP/GemRate 等）
        let stats = super::sum_active_guild_buff_stats(&self.guild_buff_infos, buffs);
        for (sid, record) in &self.players {
            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                if state.guild_name.as_deref() != Some(guild_name) {
                    continue;
                }
                if state.guild_buff_exp_percent == exp
                    && state.guild_buff_fish_rate_percent == fish
                    && state.guild_buff_mine_rate_percent == mine
                    && state.guild_buff_stats == stats
                {
                    continue;
                }
                let mut new_state = state;
                new_state.guild_buff_exp_percent = exp;
                new_state.guild_buff_fish_rate_percent = fish;
                new_state.guild_buff_mine_rate_percent = mine;
                new_state.guild_buff_stats = stats.clone();
                let _ = record
                    .actor_ref
                    .ask(SetPlayerState { state: new_state })
                    .await;
                // C# RefreshStats：Buff 变化后立即重算玩家属性
                self.recalculate_and_set_stat_bonuses(*sid).await;
            }
        }
    }

    /// 发送 GuildBuffList 完整包（C# S.GuildBuffList：Remove + ActiveBuffs + GuildBuffs）
    /// #2537：目录段带全量 Buff 定义（C# GuildObject 下发 GuildBuffs——客户端 Buff 页数据源）
    pub(crate) async fn send_guild_buff_list(&self, session_id: u64, buffs: &[u32]) {
        // ini 定义（HashMap 无序）按 id 排序保证稳定展示
        let mut infos: Vec<&crate::util::ini::GuildBuffInfo> =
            self.guild_buff_infos.values().collect();
        infos.sort_by_key(|i| i.id);
        let guild_buffs: Vec<mir2_shared::data::client_data::GuildBuffInfo> = infos
            .iter()
            .map(|info| mir2_shared::data::client_data::GuildBuffInfo {
                id: info.id as i32,
                icon: info.icon as i32,
                name: info.name.clone(),
                level_requirement: info.level_req.min(255) as u8,
                points_requirement: info.points_req.min(255) as u8,
                time_limit: info.time_limit_minutes as i32,
                activation_cost: info.activation_cost.min(i32::MAX as u64) as i32,
                stats: super::guild_buff_stats(info),
            })
            .collect();
        let packet = mir2_shared::packets::server::special_systems::GuildBuffList {
            active_buffs: buffs.iter().map(|b| *b as i32).collect(),
            guild_buffs,
        };
        let mut body = Vec::new();
        if packet
            .write_body(&mut std::io::Cursor::new(&mut body))
            .is_ok()
        {
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::GuildBuffList as i16,
                        &body,
                    ),
                })
                .await;
        }
    }
}

pub struct GuildTerritoryPageRequest {
    pub session_id: u64,
    pub page: u32,
}

impl Message<GuildTerritoryPageRequest> for WorldActor {
    type Reply = ();
    async fn handle(
        &mut self,
        msg: GuildTerritoryPageRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) {
        self.send_guild_territory_page_packet(msg.session_id, msg.page)
            .await;
    }
}

impl WorldActor {
    /// #2380：下发行会领地列表页（C# GetGuildTerritories；客户端 GuildTerritoryPage 与 NPC [@GUILDTERRITORY] 共用）。
    ///
    /// #2892 批C：改为按 C# `ClientGTMap` 字段下发（Leader/Leader2/price/days/begin），
    /// 公会最高职务成员名从 DB 现取（C# `GuildObject.Ranks[0].Members`）。
    pub(crate) async fn send_guild_territory_page_packet(&self, session_id: u64, _page: u32) {
        let leaders = crate::db::load_guild_top_rank_members(&self.db_pool)
            .await
            .unwrap_or_default();
        let map_titles: std::collections::HashMap<i32, String> = self
            .map_infos
            .iter()
            .map(|(idx, info)| (*idx, info.title.clone()))
            .collect();
        let page = build_guild_territory_page(
            &self.conquest_instances,
            &leaders,
            &map_titles,
            self.tick_count,
        );
        let mut body = Vec::new();
        if mir2_shared::packets::Packet::write_body(&page, &mut body).is_err() {
            return;
        }
        let _ = self
            .gate_ref
            .tell(SendToClient {
                session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::GuildTerritoryPage as i16,
                    &body,
                ),
            })
            .try_send();
    }
}

/// #2892 批C：构造 C# 语义的 `S.GuildTerritoryPage`（`ClientGTMap`，`SharedData.cs:139-176`）。
///
/// 字段来源：
/// - `index` = 领地地图索引；`name` = 地图标题（C# `MapInfo.Title`）
/// - `owner` = 拥有公会名；**无主下发空串**（客户端渲染「无」并按 C# 规则判为「可用」；
///   C# 服务端写死 `"None"`、客户端却拿本地化「无」比较，是原版自身的不一致 —— Rust 取空串自洽）
/// - `leader`/`leader2` = 拥有公会最高职务档前两名成员（`GuildObject.cs:305-311`）
/// - `price` = 挂售价（未挂售 0）；`begin` = 租期剩余秒（C# `(GTBegin - Now).Seconds`，`:831`）
/// - `days` = 剩余整天数（C# 存租期天数；Rust 由剩余 tick 折算）
pub(crate) fn build_guild_territory_page(
    instances: &[crate::actors::world::conquest::ConquestInstance],
    leaders: &std::collections::HashMap<String, Vec<String>>,
    map_titles: &std::collections::HashMap<i32, String>,
    now_tick: u64,
) -> mir2_shared::packets::server::special_systems::GuildTerritoryPage {
    use mir2_shared::packets::server::special_systems::{GuildTerritoryPage, TerritoryInfo};

    let mut territories = Vec::with_capacity(instances.len());
    for inst in instances {
        let owner = inst.owner_guild.clone().unwrap_or_default();
        let (leader, leader2) = if owner.is_empty() {
            (String::new(), String::new())
        } else {
            let members = leaders.get(&owner).cloned().unwrap_or_default();
            (
                members.first().cloned().unwrap_or_default(),
                members.get(1).cloned().unwrap_or_default(),
            )
        };
        let (price, days, begin) = gt_row_finance(
            inst.for_sale,
            inst.sale_price,
            inst.rent_expire_tick,
            now_tick,
        );
        territories.push(TerritoryInfo {
            id: inst.id,
            index: inst.map_index,
            name: map_titles.get(&inst.map_index).cloned().unwrap_or_default(),
            owner,
            leader,
            leader2,
            price,
            days,
            begin,
        });
    }
    GuildTerritoryPage {
        length: territories.len() as i32,
        territories,
    }
}

/// 单行的 C# 折算式（抽出便于单测）：`price` = 挂售价（未挂售 0）；
/// `begin` = 租期剩余秒（C# `(GTBegin - Now).Seconds`，`0` = 未拥有/已到期）；
/// `days` = 剩余整天数（C# 存租期天数，Rust 由剩余 tick 折算）。
pub(crate) fn gt_row_finance(
    for_sale: bool,
    sale_price: u64,
    rent_expire_tick: u64,
    now_tick: u64,
) -> (i32, i32, i32) {
    // 世界循环 100ms/tick
    const TICKS_PER_SEC: u64 = 10;
    let price = if for_sale {
        sale_price.min(i32::MAX as u64) as i32
    } else {
        0
    };
    let begin = if rent_expire_tick > now_tick {
        ((rent_expire_tick - now_tick) / TICKS_PER_SEC).min(i32::MAX as u64) as i32
    } else {
        0
    };
    let days = if begin > 0 {
        (begin + 86_399) / 86_400
    } else {
        0
    };
    (price, days, begin)
}

/// #2820：领地购买/续租的**写入段**（C# `PlayerObject.PurchaseGuildTerritory` 的赋值 + `MyGuild.NeedSave`）。
/// 抽成纯函数以便单测：写主、清挂售与售价、按 `gt_days`（挂售成交再多 1 天）算租期、置脏；
/// DB 落库由调用方紧跟的 `persist_conquest_state` 完成。
pub(crate) fn apply_gt_purchase(
    inst: &mut crate::actors::world::conquest::ConquestInstance,
    guild_name: &str,
    now_tick: u64,
    gt_days: u32,
    sold_from_sale: bool,
) -> i32 {
    inst.owner_guild = Some(guild_name.to_string());
    if sold_from_sale {
        inst.for_sale = false;
        inst.sale_price = 0;
    }
    let days = gt_days as u64 + u64::from(sold_from_sale);
    inst.rent_expire_tick = now_tick + days * crate::actors::world::conquest::TICKS_PER_DAY;
    inst.need_save = true;
    inst.id
}

pub struct PurchaseGuildTerritoryRequest {
    pub session_id: u64,
    pub territory_id: u32,
}

impl Message<PurchaseGuildTerritoryRequest> for WorldActor {
    type Reply = ();
    async fn handle(
        &mut self,
        msg: PurchaseGuildTerritoryRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        debug!(
            "PurchaseGuildTerritory: {} territory={}",
            state.name, msg.territory_id
        );

        let Some(guild_name) = state.guild_name.clone() else {
            send_system_message(&self.gate_ref, msg.session_id, "你还没有加入行会");
            return;
        };
        if state.guild_rank != GuildRank::Leader {
            send_system_message(&self.gate_ref, msg.session_id, "只有行会会长才能购买领地");
            return;
        }
        // C# AlreadyOwnATerritory（:10483）：已有领地禁止再买
        if self
            .conquest_instances
            .iter()
            .any(|c| c.owner_guild.as_deref() == Some(guild_name.as_str()))
        {
            send_system_message(&self.gate_ref, msg.session_id, "行会已拥有领地");
            return;
        }

        let Some(idx) = self
            .conquest_instances
            .iter()
            .position(|i| i.id == msg.territory_id as i32)
        else {
            send_system_message(&self.gate_ref, msg.session_id, "领地不存在");
            return;
        };
        let (for_sale, sale_price, owner) = {
            let inst = &self.conquest_instances[idx];
            (inst.for_sale, inst.sale_price, inst.owner_guild.clone())
        };

        // C# PurchaseGuildTerritory（:10455-10525）：仅挂售领地（gt.Price > 0）
        if !for_sale || sale_price == 0 {
            if owner.is_none() {
                // 无主领地 → 回退 BUYGT 近似（固定 1M 玩家金币），并补租期（C# BUYGT GTDays）
                let cost = 1000000u64;
                if record
                    .actor_ref
                    .ask(crate::actors::player::DeductGold { amount: cost })
                    .await
                    .unwrap_or(false)
                {
                    let inst = &mut self.conquest_instances[idx];
                    // #2820：统一走抽出的写入段（置脏），紧接着落库
                    let cid = apply_gt_purchase(
                        inst,
                        &guild_name,
                        self.tick_count,
                        self.conquest_cfg.gt_days,
                        false,
                    );
                    send_system_message(
                        &self.gate_ref,
                        msg.session_id,
                        &format!("行会 {} 成功购买了领地 #{}！", guild_name, msg.territory_id),
                    );
                    // #2820：立即持久化（C# `MyGuild.NeedSave` + `Envir.SaveConquests`）
                    // ——此前只改运行时态，重启后领地主权与租期回退
                    self.persist_conquest_state(cid).await;
                } else {
                    send_system_message(
                        &self.gate_ref,
                        msg.session_id,
                        "金币不足，购买领地需要 1,000,000 金币",
                    );
                }
            } else {
                send_system_message(&self.gate_ref, msg.session_id, "该领地未在挂售");
            }
            return;
        }
        // C# AlreadyOwnTerritory（:10477）：不能买自己挂售的领地
        if owner.as_deref() == Some(guild_name.as_str()) {
            send_system_message(&self.gate_ref, msg.session_id, "行会已拥有该领地");
            return;
        }
        // C# 行会资金（:10489-10493）
        let gold = self
            .social_ref
            .ask(crate::actors::social::NpcGetGuildGold {
                session_id: msg.session_id,
            })
            .await
            .unwrap_or(0);
        if gold < sale_price {
            send_system_message(&self.gate_ref, msg.session_id, "行会资金不足");
            return;
        }
        // C# :10495-10496 扣买家行会资金（GuildStorageGoldChange Type=2）
        let _ = self
            .social_ref
            .ask(crate::actors::social::NpcGuildGoldChange {
                session_id: msg.session_id,
                amount: sale_price as u32,
                change_type: 2,
            })
            .await;
        // C# :10499-10510 卖家行会收款 + EndGT + 提示
        if let Some(seller) = owner {
            let _ = self
                .social_ref
                .ask(crate::actors::social::NpcGuildGoldGive {
                    guild_name: seller.clone(),
                    amount: sale_price as u32,
                })
                .await;
            // 通知卖家在线成员「领地已出售」（C# TerritorySold）
            for (sid, rec) in &self.players {
                if let Ok(Some(os)) = rec.actor_ref.ask(GetPlayerState).await {
                    if os.guild_name.as_deref() == Some(seller.as_str()) {
                        send_system_message(&self.gate_ref, *sid, "您的领地已出售");
                    }
                }
            }
        }
        // C# :10512-10522 买家获得领地（GTRent = Now + GTDays+1；EndGT 释放卖家）
        let gt_map_index = self.conquest_instances[idx].map_index as u16;
        let inst = &mut self.conquest_instances[idx];
        // #2820：挂售成交分支（多 1 天租期）同样走写入段并置脏
        let cid = apply_gt_purchase(
            inst,
            &guild_name,
            self.tick_count,
            self.conquest_cfg.gt_days,
            true,
        );
        send_system_message(
            &self.gate_ref,
            msg.session_id,
            &format!("行会 {} 成功购买了领地 #{}！", guild_name, msg.territory_id),
        );
        // #2820：挂售成交同样立即持久化（与无主回退分支一致）
        self.persist_conquest_state(cid).await;
        // C# 卖家 EndGT（:10504）：踢出领地地图玩家（传送回绑定点）
        self.evict_gt_map_players(gt_map_index).await;
    }
}

/// #1340：构建 GuildStorageGoldChange body（wire 对齐 C# ServerPackets.cs:4628：
/// [Amount u32][Type u8][Name dotnet string]）
fn guild_storage_gold_change_body(amount: u32, change_type: u8, name: &str) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&amount.to_le_bytes());
    body.push(change_type);
    crate::util::wire::write_dotnet_string(&mut body, name);
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2892 批C：GT 行折算 —— `price` 只在挂售时下发；`begin` = 租期剩余秒；
    /// `days` = 剩余整天数（向上取整）
    #[test]
    fn gt_row_finance_matches_csharp_fields() {
        // 未挂售 → price 0；无租期 → begin/days 0
        assert_eq!(gt_row_finance(false, 5_000_000, 0, 100), (0, 0, 0));
        // 挂售 → price = sale_price
        assert_eq!(gt_row_finance(true, 5_000_000, 0, 0).0, 5_000_000);
        // 租期剩 7200 秒（72000 ticks）→ begin 7200、days 1（向上取整）
        let (_, days, begin) = gt_row_finance(false, 0, 72_000, 0);
        assert_eq!(begin, 7200);
        assert_eq!(days, 1);
        // 已到期 → begin 0
        assert_eq!(gt_row_finance(false, 0, 100, 200), (0, 0, 0));
        // 超过 i32 上限 → 钳制，不 panic
        assert_eq!(gt_row_finance(true, u64::MAX, 0, 0).0, i32::MAX);
    }

    /// #2892 批C：`Leader`/`Leader2` 取拥有公会最高职务档前两名（C# `GuildObject.cs:305-311`）；
    /// 无主行三字段都空（客户端据此判「可用」）
    #[test]
    fn gt_page_rows_use_top_rank_members() {
        use crate::actors::world::conquest::{ConquestGame, ConquestInstance};
        let mut owned = ConquestInstance::new(3, 5, 0, ConquestGame::Classic);
        owned.owner_guild = Some("行会A".to_string());
        owned.for_sale = true;
        owned.sale_price = 1_000_000;
        let leaders = std::collections::HashMap::from([
            (
                "行会A".to_string(),
                vec!["会长甲".to_string(), "副会长乙".to_string()],
            ),
            ("行会B".to_string(), vec!["会长丙".to_string()]),
        ]);
        let titles = std::collections::HashMap::from([(5, "GT 地图".to_string())]);
        let page = build_guild_territory_page(&[owned], &leaders, &titles, 0);
        assert_eq!(page.length, 1);
        let row = &page.territories[0];
        assert_eq!(row.id, 3);
        assert_eq!(row.index, 5);
        assert_eq!(row.name, "GT 地图");
        assert_eq!(row.owner, "行会A");
        assert_eq!(row.leader, "会长甲");
        assert_eq!(row.leader2, "副会长乙");
        assert_eq!(row.price, 1_000_000);

        // 无主 → owner/leader/leader2 空串、price 0
        let mut free = ConquestInstance::new(4, 6, 0, ConquestGame::Classic);
        free.owner_guild = None;
        let page = build_guild_territory_page(&[free], &leaders, &titles, 0);
        let row = &page.territories[0];
        assert!(row.owner.is_empty());
        assert!(row.leader.is_empty());
        assert!(row.leader2.is_empty());
        assert_eq!(row.price, 0);
    }

    /// #2820：领地购买/续租写入段——写主 + 置脏 + 租期（挂售成交多 1 天、清挂售与售价）
    /// （C# `PlayerObject.PurchaseGuildTerritory` + `MyGuild.NeedSave`）
    #[test]
    fn apply_gt_purchase_writes_owner_dirty_and_rent() {
        use crate::actors::world::conquest::{ConquestGame, ConquestInstance, TICKS_PER_DAY};
        let make = |id: i32| ConquestInstance::new(id, 0, 0, ConquestGame::Classic);

        // 无主回退分支：1M 直购（sold_from_sale = false）
        let mut inst = make(7);
        inst.owner_guild = None;
        inst.need_save = false;
        let cid = apply_gt_purchase(&mut inst, "行会A", 1_000, 3, false);
        assert_eq!(cid, 7, "返回实例 id（供 persist 用）");
        assert_eq!(inst.owner_guild.as_deref(), Some("行会A"));
        assert_eq!(
            inst.rent_expire_tick,
            1_000 + 3 * TICKS_PER_DAY,
            "租期 = 现在 + gt_days 天"
        );
        assert!(inst.need_save, "购买必须置脏（否则不会被保存 ⇒ 重启回退）");

        // 挂售成交分支：清挂售/售价 + 多 1 天
        let mut inst = make(9);
        inst.for_sale = true;
        inst.sale_price = 500_000;
        inst.need_save = false;
        apply_gt_purchase(&mut inst, "行会B", 2_000, 3, true);
        assert!(!inst.for_sale, "成交后撤下挂售");
        assert_eq!(inst.sale_price, 0);
        assert_eq!(
            inst.rent_expire_tick,
            2_000 + 4 * TICKS_PER_DAY,
            "挂售成交按 C# `GTRent = Now + (GTDays+1) 天`"
        );
        assert!(inst.need_save);
    }

    #[test]
    fn test_war_key_sorted() {
        assert_eq!(war_key("A", "B"), ("A".to_string(), "B".to_string()));
        assert_eq!(war_key("B", "A"), ("A".to_string(), "B".to_string()));
        assert_eq!(war_key("A", "A"), ("A".to_string(), "A".to_string()));
    }

    #[test]
    fn guild_storage_gold_change_body_matches_csharp_wire() {
        // #1340：3000 = 0x00000BB8 LE；Type=2；dotnet string "Boss" = 4 + UTF8
        let body = guild_storage_gold_change_body(3000, 2, "Boss");
        assert_eq!(&body[0..4], &[0xB8, 0x0B, 0x00, 0x00]);
        assert_eq!(body[4], 2);
        assert_eq!(body[5], 4); // dotnet string length
        assert_eq!(&body[6..], b"Boss");
    }

    #[test]
    fn guild_storage_gold_change_body_empty_name() {
        let body = guild_storage_gold_change_body(0, 1, "");
        assert_eq!(body[4], 1);
        assert_eq!(body[5], 0);
    }
}
