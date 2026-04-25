use super::*;

// ============================================================
// 英雄系统 Handler
// ============================================================

/// 切换英雄
pub struct ChangeHeroRequest {
    pub session_id: u64,
    pub hero_index: u8,
}

impl Message<ChangeHeroRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: ChangeHeroRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        if msg.hero_index == 0 && state.hero_index == 0 {
            send_system_message(&self.gate_ref, msg.session_id, "你没有可用的英雄");
            return;
        }

        let _ = record.actor_ref.ask(SetHeroIndex { hero_index: msg.hero_index });
        send_hero_update_packet(&self.gate_ref, msg.session_id, msg.hero_index);
        debug!("Hero switched: {} -> index {}", state.name, msg.hero_index);
    }
}

/// 从英雄背包取回物品
pub struct TakeBackHeroItemRequest {
    pub session_id: u64,
    pub grid: u8,
    pub unique_id: u64,
}

impl Message<TakeBackHeroItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: TakeBackHeroItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 从英雄背包移除物品并添加到主背包
        let _ = record.actor_ref.ask(crate::actors::player::TakeBackHeroItem {
            grid: msg.grid,
        }).await;

        debug!("Hero item taken back: {} grid={} uid={}", state.name, msg.grid, msg.unique_id);
    }
}

/// 转移物品到英雄背包
pub struct TransferHeroItemRequest {
    pub session_id: u64,
    pub grid: u8,
    pub unique_id: u64,
}

impl Message<TransferHeroItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: TransferHeroItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 从主背包移除物品并添加到英雄背包
        let _ = record.actor_ref.ask(crate::actors::player::TransferHeroItem {
            grid: msg.grid,
        }).await;

        debug!("Hero item transferred: {} grid={} uid={}", state.name, msg.grid, msg.unique_id);
    }
}

// ============================================================
// 宠物系统 Handler
// ============================================================

/// 更新/设置宠物
pub struct UpdateIntelligentCreature {
    pub session_id: u64,
    pub creature_type: u8,
    pub pickup_mode: u8,
}

impl Message<UpdateIntelligentCreature> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: UpdateIntelligentCreature, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let creature_type = CreatureType::from(msg.creature_type);
        let pickup = PickupMode::from(msg.pickup_mode);

        if creature_type == CreatureType::None {
            // 关闭宠物
            let mut log = state.creature_log;
            log.active_creature = None;
            let _ = record.actor_ref.ask(SetCreature { creature_log: log });
            send_system_message(&self.gate_ref, msg.session_id, "宠物已关闭");
            return;
        }

        // 设置或更新宠物
        let mut log = state.creature_log;
        if let Some(ref mut c) = log.active_creature {
            // 更新已有宠物
            c.pickup_mode = pickup;
        } else {
            // 创建新宠物
            let mut creature = IntelligentCreature::new(creature_type);
            creature.pickup_mode = pickup;
            creature.enabled = true;
            log.active_creature = Some(creature);
        }
        let creature_ref = log.active_creature.clone();
        let _ = record.actor_ref.ask(SetCreature { creature_log: log });

        send_creature_list_packet(&self.gate_ref, msg.session_id, creature_ref.as_ref());
        debug!("UpdateIntelligentCreature: {} type={:?} mode={:?}", state.name, creature_type, pickup);
    }
}

/// 宠物拾取地面物品
pub struct IntelligentCreaturePickup {
    pub session_id: u64,
    pub x: i32,
    pub y: i32,
}

impl Message<IntelligentCreaturePickup> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: IntelligentCreaturePickup, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查是否有激活的宠物
        let pickup_mode = match &state.creature_log.active_creature {
            Some(c) if c.enabled && !c.is_starving() => c.pickup_mode,
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "没有可用的宠物");
                return;
            }
        };

        // 根据拾取模式过滤
        if pickup_mode == PickupMode::None {
            send_system_message(&self.gate_ref, msg.session_id, "宠物拾取模式未设置");
            return;
        }

        // 查找附近的地面物品（同地图）
        let distance = 3; // 宠物拾取范围
        let item_idx = self.ground_items.iter().position(|item| {
            item.map_index == state.map_index
                && (item.x - msg.x).abs() <= distance
                && (item.y - msg.y).abs() <= distance
        });

        if let Some(idx) = item_idx {
            let item = self.ground_items.remove(idx);
            let picked_oid = item.object_id;
            // 将物品添加到玩家背包
            let mut picked_up = false;
            if let Some(rec) = self.players.get(&msg.session_id) {
                if let Ok(true) = rec.actor_ref.ask(AddItemToInventory {
                    item: item.item.clone(),
                }).await {
                    picked_up = true;
                }
            }
            if picked_up {
                // 广播 ObjectRemove
                let remove_packet = Self::build_object_remove_packet(picked_oid);
                for (sid, rec) in &self.players {
                    if let Ok(Some(s)) = rec.actor_ref.ask(GetPlayerState).await {
                        if s.map_index == state.map_index {
                            let _ = self.gate_ref.ask(SendToClient {
                                session_id: *sid,
                                data: remove_packet.clone(),
                            });
                        }
                    }
                }
                debug!("Creature pickup: {} picked up item at ({},{})",
                       state.name, msg.x, msg.y);
            } else {
                // 添加失败，放回去
                self.ground_items.push(item);
            }
        }
    }
}

/// 请求宠物更新列表
pub struct RequestIntelligentCreatureUpdates {
    pub session_id: u64,
    pub request_updates: bool,
}

impl Message<RequestIntelligentCreatureUpdates> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: RequestIntelligentCreatureUpdates, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 发送当前宠物列表
        let creature_ref = state.creature_log.active_creature.clone();
        send_creature_list_packet(&self.gate_ref, msg.session_id, creature_ref.as_ref());
    }
}
