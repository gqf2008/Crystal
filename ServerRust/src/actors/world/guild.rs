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
            for (_, record) in &self.players {
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    if state.object_id == msg.object_id {
                        // Found — send UserName response
                        let mut body = Vec::new();
                        body.extend_from_slice(&msg.object_id.to_le_bytes());
                        crate::util::wire::write_dotnet_string(&mut body, &state.name);
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: msg.session_id,
                            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::UserName as i16, &body),
                        }).await;
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
            let _ = self.gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::UserName as i16, &body),
            }).await;
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
            Some(r) => r.clone(), None => return,
        };
        let item_info = record.actor_ref.ask(crate::actors::player::GetItemInfo {
            unique_id: msg.unique_id,
        }).await.ok().flatten();

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
            let _ = self.gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ChatItemStats as i16, &body),
            }).await;
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
    async fn handle(&mut self, msg: AcceptReincarnationRequest, _ctx: &mut Context<Self, Self::Reply>) {
        // AcceptReincarnation: dead player accepts reincarnation from host.
        // C#: if ReincarnationHost != null && ReincarnationHost.ReincarnationReady -> Revive(HP/2)
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

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

        debug!("AcceptReincarnation: {} accepted from host session={}", state.name, host_session);

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
    async fn handle(&mut self, msg: CancelReincarnationRequest, _ctx: &mut Context<Self, Self::Reply>) {
        // CancelReincarnation: dead player cancels reincarnation.
        // C#: ReincarnationExpireTime = Envir.Time (immediate expiry triggers cleanup)
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

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

pub struct GuildWarReturnRequest {
    pub session_id: u64,
    pub guild_name: String,
}

impl Message<GuildWarReturnRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: GuildWarReturnRequest, _ctx: &mut Context<Self, Self::Reply>) {
        // GuildWarReturn: query if a guild exists and return its war status
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        debug!("GuildWarReturn: {} querying guild={}", state.name, msg.guild_name);

        if state.guild_name.is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "你还没有加入行会");
            return;
        }

        let sender_guild = state.guild_name.as_ref().unwrap();
        if msg.guild_name == *sender_guild {
            send_system_message(&self.gate_ref, msg.session_id, "不能向自己的行会宣战");
            return;
        }

        // 行会信息由 SocialActor 管理，此处仅做简单校验
        if msg.guild_name.is_empty() {
            send_system_message(&self.gate_ref, msg.session_id, "行会名称无效");
            return;
        }

        // C# requires guild leader (rank 0) to declare war
        if state.guild_rank != GuildRank::Leader {
            send_system_message(&self.gate_ref, msg.session_id, "只有行会会长才能宣战");
            return;
        }

        // Record the war declaration
        self.guild_wars.entry(sender_guild.clone()).or_default().insert(msg.guild_name.clone());
        self.guild_wars.entry(msg.guild_name.clone()).or_default().insert(sender_guild.clone());

        // Notify all online members of the declaring guild
        let war_msg = format!("行会 {} 已向 {} 宣战！", sender_guild, msg.guild_name);
        for (sid, rec) in &self.players {
            if *sid == msg.session_id { continue; }
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
                if s.guild_name.as_deref() == Some(msg.guild_name.as_str()) {
                    send_system_message(&self.gate_ref, *sid, &target_msg);
                }
            }
        }

        // Send GuildRequestWar packet back to the declarer
        use mir2_shared::packets::server::miscellaneous::GuildRequestWar;
        let war_packet = GuildRequestWar { guild_name: msg.guild_name.clone() };
        let mut war_body = Vec::new();
        if let Ok(()) = mir2_shared::packets::Packet::write_body(&war_packet, &mut war_body) {
            let _ = self.gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GuildRequestWar as i16, &war_body),
            }).await;
        }

        send_system_message(&self.gate_ref, msg.session_id, &format!("已向 {} 行会宣战", msg.guild_name));
    }
}

pub struct GuildBuffUpdateRequest {
    pub session_id: u64,
    pub buff_id: u32,
}

impl Message<GuildBuffUpdateRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: GuildBuffUpdateRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        debug!("GuildBuffUpdate: {} buff_id={}", state.name, msg.buff_id);

        if state.guild_name.is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "你还没有加入行会");
            return;
        }

        // buff_id=0 means "request buff list" - send empty list (no buffs defined yet)
        if msg.buff_id == 0 {
            let body = Vec::new();
            let _ = self.gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GuildBuffList as i16, &body),
            }).await;
        } else {
            debug!("GuildBuffUpdate: {} buff_id={} (guild buff system not implemented)", state.name, msg.buff_id);
        }
    }
}

pub struct GuildTerritoryPageRequest {
    pub session_id: u64,
    pub page: u32,
}

impl Message<GuildTerritoryPageRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: GuildTerritoryPageRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let count = self.conquest_instances.len() as i32;
        let mut body = Vec::new();
        body.extend_from_slice(&count.to_le_bytes());
        for instance in &self.conquest_instances {
            body.extend_from_slice(&instance.id.to_le_bytes());
            body.extend_from_slice(&instance.map_index.to_le_bytes());
            let owner = instance.owner_guild.as_deref().unwrap_or("");
            crate::util::wire::write_dotnet_string(&mut body, owner);
            body.push(instance.state.clone() as u8);
        }
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GuildTerritoryPage as i16, &body),
        }).await;
    }
}

pub struct PurchaseGuildTerritoryRequest {
    pub session_id: u64,
    pub territory_id: u32,
}

impl Message<PurchaseGuildTerritoryRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: PurchaseGuildTerritoryRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        debug!("PurchaseGuildTerritory: {} territory={}", state.name, msg.territory_id);

        if state.guild_name.is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "你还没有加入行会");
            return;
        }

        if state.guild_rank != GuildRank::Leader {
            send_system_message(&self.gate_ref, msg.session_id, "只有行会会长才能购买领地");
            return;
        }

        // Check if territory exists and is purchasable
        let instance = self.conquest_instances.iter_mut()
            .find(|i| i.id == msg.territory_id as i32);
        match instance {
            Some(inst) if inst.owner_guild.is_none() => {
                let cost = 1000000; // 1M gold base cost
                let guild = state.guild_name.clone().unwrap();
                // Deduct gold from guild storage via PlayerActor
                if record.actor_ref.ask(crate::actors::player::DeductGold { amount: cost }).await.unwrap_or(false) {
                    inst.owner_guild = Some(guild.clone());
                    send_system_message(&self.gate_ref, msg.session_id,
                        &format!("行会 {} 成功购买了领地 #{}！", guild, msg.territory_id));
                } else {
                    send_system_message(&self.gate_ref, msg.session_id, "金币不足，购买领地需要 1,000,000 金币");
                }
            }
            Some(inst) => {
                let owner = inst.owner_guild.as_deref().unwrap_or("未知");
                send_system_message(&self.gate_ref, msg.session_id,
                    &format!("该领地已被 {} 占领", owner));
            }
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "领地不存在");
            }
        }
    }
}
