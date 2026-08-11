use super::*;

impl WorldActor {
    /// #2014：quest 是否已关联到任意 NPC（collect/finish 列表）；未关联（数据未配置）时不做 NPC 强制校验
    pub(crate) fn quest_has_npc_link(&self, quest_index: i32, finish: bool) -> bool {
        self.npc_infos.values().any(|info| {
            if finish {
                info.finish_quest_indexes.contains(&quest_index)
            } else {
                info.collect_quest_indexes.contains(&quest_index)
            }
        })
    }

    /// #2014：C# AcceptQuest/FinishQuest——同图 + DataRange(16) 内存在可接/可交该任务的 NPC
    pub(crate) fn quest_npc_in_range(&self, player_map: u16, px: i32, py: i32, quest_index: i32, finish: bool) -> bool {
        self.npcs.values().any(|npc| {
            npc.map_index == player_map
                && crate::actors::world::ai::max_distance(px, py, npc.x, npc.y) <= 16
                && self.npc_infos.get(&npc.db_index)
                    .map(|info| if finish {
                        info.finish_quest_indexes.contains(&quest_index)
                    } else {
                        info.collect_quest_indexes.contains(&quest_index)
                    })
                    .unwrap_or(false)
        })
    }
}

/// 接受任务
pub struct AcceptQuestRequest {
    pub session_id: u64,
    pub npc_index: i32,
    pub quest_index: i32,
}

/// 完成任务
pub struct FinishQuestRequest {
    pub session_id: u64,
    pub quest_index: i32,
    pub selected_item_index: i32,
}

/// 放弃任务
pub struct AbandonQuestRequest {
    pub session_id: u64,
    pub quest_index: i32,
}

impl Message<AcceptQuestRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: AcceptQuestRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };

        // Validate quest exists in DB
        let quest_db = self.quest_infos.get(&msg.quest_index).cloned();
        let Some(quest_db) = quest_db else {
            send_system_message(&self.gate_ref, msg.session_id, "任务不存在");
            return;
        };

        // #2014：C# AcceptQuest（11251-11264）——同图 DataRange(16) 内存在可接该任务的 NPC（数据驱动）
        if self.quest_has_npc_link(msg.quest_index, false) {
            let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await else { return };
            if !self.quest_npc_in_range(state.map_index, state.x, state.y, msg.quest_index, false) {
                send_system_message(&self.gate_ref, msg.session_id, "请到对应 NPC 处接取任务");
                return;
            }
        }

        // Check level requirement
        if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
            if state.level < quest_db.required_min_level as u16 {
                send_system_message(&self.gate_ref, msg.session_id, "等级不足");
                return;
            }
            if quest_db.required_max_level > 0 && state.level > quest_db.required_max_level as u16 {
                send_system_message(&self.gate_ref, msg.session_id, "等级过高");
                return;
            }
            // #2004：C# QuestInfo.CanAccept——RequiredClass 位掩码（Warrior=1/Wizard=2/Taoist=4/Assassin=8/Archer=16；0=不限制）
            if quest_db.required_class != 0 {
                let class_bit: i32 = match state.class {
                    mir2_shared::enums::MirClass::Warrior => 1,
                    mir2_shared::enums::MirClass::Wizard => 2,
                    mir2_shared::enums::MirClass::Taoist => 4,
                    mir2_shared::enums::MirClass::Assassin => 8,
                    mir2_shared::enums::MirClass::Archer => 16,
                };
                if quest_db.required_class & class_bit == 0 {
                    send_system_message(&self.gate_ref, msg.session_id, "职业不符合");
                    return;
                }
            }
        }
        // #2004：C# QuestInfo.CanAccept——RequiredQuest 前置任务（需已完成）
        if quest_db.required_quest > 0 {
            if let Ok(false) = record.actor_ref.ask(HasCompletedQuest { quest_index: quest_db.required_quest }).await {
                send_system_message(&self.gate_ref, msg.session_id, "需要先完成前置任务");
                return;
            }
        }

        // 检查是否已接受该任务
        if let Ok(Some(_quest)) = record.actor_ref.ask(GetQuest { quest_index: msg.quest_index }).await {
            send_system_message(&self.gate_ref, msg.session_id, "该任务已接受");
            return;
        }

        // 检查是否已完成过该任务
        if let Ok(true) = record.actor_ref.ask(HasCompletedQuest { quest_index: msg.quest_index }).await {
            send_system_message(&self.gate_ref, msg.session_id, "该任务已完成");
            return;
        }

        // Create quest instance from DB data
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let quest = make_quest_instance(&quest_db, now);
        let accepted = match record.actor_ref.ask(AcceptQuest { quest }).await {
            Ok(s) => s, _ => return,
        };

        if accepted {
            send_system_message(&self.gate_ref, msg.session_id, "任务已接受");
            // M43：推送任务进度到客户端任务日志（C# S.ChangeQuest 语义）
            if let Ok(Some(q)) = record.actor_ref.ask(GetQuest { quest_index: msg.quest_index }).await {
                crate::actors::social_packets::send_quest_change_packet(&self.gate_ref, msg.session_id, &q);
            }
            debug!("Quest accepted: {} ({}) by session {}", quest_db.name, msg.quest_index, msg.session_id);
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "任务接受失败");
        }
    }
}

impl Message<FinishQuestRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: FinishQuestRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };

        // #2014：C# FinishQuest（11350-11363）——同图 DataRange(16) 内存在可交该任务的 NPC（数据驱动）
        if self.quest_has_npc_link(msg.quest_index, true) {
            let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await else { return };
            if !self.quest_npc_in_range(state.map_index, state.x, state.y, msg.quest_index, true) {
                send_system_message(&self.gate_ref, msg.session_id, "请到对应 NPC 处交付任务");
                return;
            }
        }

        // #2002：C# FinishQuest——交任务前检查背包空间（CanGainItems → CannotHandInQuestBagFull）
        if let Some(quest_db) = self.quest_infos.get(&msg.quest_index) {
            let has_item_reward = !quest_db.fixed_rewards.is_empty()
                || (msg.selected_item_index >= 0
                    && quest_db.select_rewards.get(msg.selected_item_index as usize).is_some());
            if has_item_reward {
                let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await else { return };
                if !st.inventory.can_gain_items() {
                    send_system_message(&self.gate_ref, msg.session_id, "背包已满，无法领取任务奖励");
                    return;
                }
            }
        }

        let completed_quest = match record.actor_ref.ask(CompleteQuest { quest_index: msg.quest_index }).await {
            Ok(Some(q)) => q,
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "任务不存在");
                return;
            }
        };

        // 发放奖励
        if completed_quest.exp_reward > 0 {
            let _ = record.actor_ref.ask(AddExperience {
                amount: self.apply_global_exp_multiplier(completed_quest.exp_reward as i32),
                experience_list: self.experience_list.clone(),
            }).await;
        }
        if completed_quest.gold_reward > 0 {
            // #2000：C# FinishQuest——GoldReward * Settings.DropRate（与 NPC 脚本 COMPLETEQUEST 一致）
            let gold = (completed_quest.gold_reward as f64 * self.drop_rate) as u64;
            let _ = record.actor_ref.ask(AddGold { amount: gold }).await;
        }
        // #2000：C# FinishQuest——GainCredit(CreditReward)（账户积分，上限 uint.MaxValue）
        if completed_quest.credit_reward > 0 {
            let username = record.account_username.clone();
            let current = db::get_account_credit(&self.db_pool, &username).await.unwrap_or(0);
            let remaining = (u32::MAX as u64).saturating_sub(current.min(u32::MAX as u64));
            let delta = (completed_quest.credit_reward as u64).min(remaining) as i64;
            if delta > 0 {
                if let Err(e) = db::add_account_credit(&self.db_pool, &username, delta).await {
                    warn!("Quest CreditReward failed for {}: {}", username, e);
                } else {
                    // C# GainCredit：S.GainedCredit（客户端积分浮字）
                    let packet = mir2_shared::packets::server::drops::GainedCredit { credit: delta as u32 };
                    let mut body = Vec::new();
                    if packet.write_body(&mut body).is_ok() {
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: msg.session_id,
                            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GainedCredit as i16, &body),
                        }).await;
                    }
                }
            }
        }

        // 发放固定物品奖励
        if let Some(quest_db) = self.quest_infos.get(&msg.quest_index) {
            for reward in &quest_db.fixed_rewards {
                let mut item = mir2_shared::data::item::UserItem {
                    item_index: reward.item_index,
                    count: reward.count,
                    ..Default::default()
                };
                if let Some(info) = self.item_infos.get(&reward.item_index) {
                    item.max_dura = info.durability as u16;
                    item.current_dura = info.durability as u16;
                }
                let _ = record.actor_ref.ask(crate::actors::player::AddItemToInventory { item }).await;
            }
            if !quest_db.fixed_rewards.is_empty() {
                let _ = record.actor_ref.ask(crate::actors::player::CheckQuestItemProgress).await;
            }
            // #1998：C# PlayerObject.FinishQuest（11394-11414）——selectedItemIndex>=0 时
            // 按 SelectRewards 列表下标发放可选奖励（越界/负数不发放）
            if msg.selected_item_index >= 0 {
                if let Some(reward) = quest_db.select_rewards.get(msg.selected_item_index as usize) {
                    let mut item = mir2_shared::data::item::UserItem {
                        item_index: reward.item_index,
                        count: reward.count,
                        ..Default::default()
                    };
                    if let Some(info) = self.item_infos.get(&reward.item_index) {
                        item.max_dura = info.durability as u16;
                        item.current_dura = info.durability as u16;
                    }
                    let _ = record.actor_ref.ask(crate::actors::player::AddItemToInventory { item }).await;
                    debug!("Quest {} select reward #{} (item {}) granted to session {}",
                           msg.quest_index, msg.selected_item_index, reward.item_index, msg.session_id);
                }
            }
        }

        send_system_message(&self.gate_ref, msg.session_id, &format!(
            "任务完成！获得 {} 经验，{} 金币{}",
            completed_quest.exp_reward,
            completed_quest.gold_reward,
            if completed_quest.credit_reward > 0 { format!("，{} 信用", completed_quest.credit_reward) } else { String::new() },
        ));
        send_quest_complete_packet(&self.gate_ref, msg.session_id, completed_quest.quest_index);
        debug!("Quest completed: {} by session {}", msg.quest_index, msg.session_id);
    }
}

impl Message<AbandonQuestRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: AbandonQuestRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };

        let abandoned = match record.actor_ref.ask(AbandonQuest { quest_index: msg.quest_index }).await {
            Ok(s) => s, _ => return,
        };

        if abandoned {
            send_system_message(&self.gate_ref, msg.session_id, "任务已放弃");
            debug!("Quest abandoned: {} by session {}", msg.quest_index, msg.session_id);
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "任务不存在");
        }
    }
}

// ============================================================
// 任务分享
// ============================================================

pub struct ShareQuestRequest {
    pub session_id: u64,
    pub quest_id: u32,
}

impl Message<ShareQuestRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ShareQuestRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        // Must be in a group to share
        let group_id = match state.group_id {
            Some(gid) => gid,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "你需要加入队伍才能分享任务");
                return;
            }
        };

        // Verify the player has the quest
        let has_quest = match record.actor_ref.ask(GetQuest { quest_index: msg.quest_id as i32 }).await {
            Ok(Some(_)) => true,
            _ => false,
        };
        if !has_quest {
            send_system_message(&self.gate_ref, msg.session_id, "你没有这个任务");
            return;
        }

        // Send ShareQuest packet to all group members (except self)
        use mir2_shared::packets::server::miscellaneous::ShareQuest as ShareQuestPacket;
        let packet = ShareQuestPacket { quest_id: msg.quest_id as i32 };
        let mut body = Vec::new();
        if let Ok(()) = mir2_shared::packets::Packet::write_body(&packet, &mut body) {
            let data = build_packet_bytes(mir2_shared::enums::ServerPacketIds::ShareQuest as i16, &body);
            for (sid, rec) in &self.players {
                if *sid == msg.session_id { continue; }
                if let Ok(Some(s)) = rec.actor_ref.ask(GetPlayerState).await {
                    if s.group_id == Some(group_id) {
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: *sid,
                            data: data.clone(),
                        }).await;
                    }
                }
            }
        }

        send_system_message(&self.gate_ref, msg.session_id, &format!("已分享任务 #{}", msg.quest_id));
        debug!("ShareQuest: {} quest_id={}", state.name, msg.quest_id);
    }
}
