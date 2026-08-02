use super::*;

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
            }).await;
        }
        if completed_quest.gold_reward > 0 {
            let _ = record.actor_ref.ask(AddGold { amount: completed_quest.gold_reward }).await;
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
        }

        send_system_message(&self.gate_ref, msg.session_id, &format!("任务完成！获得 {} 经验，{} 金币", completed_quest.exp_reward, completed_quest.gold_reward));
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
