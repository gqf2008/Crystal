use super::*;

/// 拾取地面物品
pub struct PickUpRequest {
    pub session_id: u64,
}

/// 背包内移动物品
pub struct MoveItemRequest {
    pub session_id: u64,
    pub grid: u8,
    pub from: i32,
    pub to: i32,
}

/// 使用物品
pub struct UseItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
}

/// 装备物品
pub struct EquipItemRequest {
    pub session_id: u64,
    pub grid: u8,
    pub unique_id: u64,
    pub slot: i32,
}

/// 卸下装备
pub struct RemoveItemRequest {
    pub session_id: u64,
    pub grid: u8,
    pub unique_id: u64,
}

/// 丢弃物品
pub struct DropItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
    pub count: u16,
}

/// 合并物品
pub struct MergeItemRequest {
    pub session_id: u64,
    pub grid_from: u8,
    pub grid_to: u8,
    pub from_uid: u64,
    pub to_uid: u64,
}

/// 拆分物品
pub struct SplitItemRequest {
    pub session_id: u64,
    pub grid: u8,
    pub unique_id: u64,
    pub count: u32,
}

/// 丢弃金币
pub struct DropGoldRequest {
    pub session_id: u64,
    pub amount: u32,
}

/// 购买物品（NPC 商店）
pub struct BuyItemRequest {
    pub session_id: u64,
    pub npc_id: u32,
    pub item_index: u32,
    pub count: u32,
}

/// 出售物品（NPC 商店）
pub struct SellItemRequest {
    pub session_id: u64,
    pub grid: u8,
    pub unique_id: u64,
    pub count: u32,
}

// ============================================================
// 修理系统 Handler
// ============================================================

/// 修理费用：每缺失 1 点耐久 = 1 金币
const REPAIR_COST_PER_DURA: u64 = 1;

/// 修理物品请求
pub struct RepairItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
}

/// 快捷装备栏装备
pub struct EquipSlotItemRequest {
    pub session_id: u64,
    pub grid: u8,
    pub unique_id: u64,
    pub to_slot: i32,
    pub grid_to: u8,
}

/// 更换结婚戒指
pub struct ReplaceWedRingRequest {
    pub session_id: u64,
    pub unique_id: u64,
}

/// 存入仓库
pub struct StoreItemRequest {
    pub session_id: u64,
    pub grid: u8,
    pub uid: u64,
    pub count: u32,
}

/// 从仓库取出
pub struct TakeBackItemRequest {
    pub session_id: u64,
    pub grid: u8,
    pub uid: u64,
    pub count: u32,
}

/// 合成物品请求
pub struct CraftItemRequest {
    pub session_id: u64,
    pub recipe_id: u32,
}

/// 回购物品请求（从 NPC 回购最近卖出的物品）
pub struct BuyItemBackRequest {
    pub session_id: u64,
    pub item_index: u32,
}

pub struct CombineItemRequest {
    pub session_id: u64,
    pub from_grid: u32,
    pub to_grid: u32,
}

pub struct DisassembleItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
}

impl Message<PickUpRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: PickUpRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if state.is_dead { return; }
        let player_pos = (state.x, state.y);

        // 查找附近可拾取的物品（1 格内，同地图）
        const OWNERSHIP_TICKS: u64 = 300; // ~30 秒保护期
        let pickup_idx = self.ground_items.iter().position(|gi| {
            if gi.map_index != state.map_index { return false; }
            if (gi.x - player_pos.0).abs() > 1 { return false; }
            if (gi.y - player_pos.1).abs() > 1 { return false; }
            // 所有权保护：保护期内只有掉落者可拾取
            if let Some(dropper) = gi.dropper_session {
                if self.tick_count < gi.drop_tick + OWNERSHIP_TICKS && dropper != msg.session_id {
                    return false;
                }
            }
            true
        });

        if let Some(idx) = pickup_idx {
            let ground_item = self.ground_items.remove(idx);
            let picked_oid = ground_item.object_id;
            debug!(
                "Player session={} picked up item uid={} at ({}, {})",
                msg.session_id, ground_item.item.unique_id, ground_item.x, ground_item.y
            );

            // 通知 PlayerActor 添加到背包
            let mut picked_up = false;
            if let Ok(success) = record.actor_ref.ask(AddItemToInventory {
                item: ground_item.item.clone(),
            }).await {
                if success {
                    picked_up = true;
                } else {
                    // 背包已满，放回去
                    self.ground_items.push(ground_item);
                    send_system_message(&self.gate_ref, msg.session_id, "背包已满");
                }
            } else {
                self.ground_items.push(ground_item);
            }

            // 拾取成功：完整 UserInformation 刷新（背包新增物品）
            if picked_up {
                if let Ok(Some(new_state)) = record.actor_ref.ask(GetPlayerState).await {
                    let packet = super::build_user_information_packet(&new_state, &self.item_infos);
                    let _ = self.gate_ref.tell(SendToClient {
                        session_id: msg.session_id,
                        data: packet,
                    }).await;
                }
            }

            // 拾取成功：广播 ObjectRemove 给同地图玩家
            if picked_up {
                let remove_packet = Self::build_object_remove_packet(picked_oid);
                for (sid, rec) in &self.players {
                    if let Ok(Some(s)) = rec.actor_ref.ask(GetPlayerState).await {
                        if s.map_index == state.map_index {
                            let _ = self.gate_ref.tell(SendToClient {
                                session_id: *sid,
                                data: remove_packet.clone(),
                            }).await;
                        }
                    }
                }
                // 检查任务物品进度
                let updates = record.actor_ref.ask(crate::actors::player::CheckQuestItemProgress).await.unwrap_or_default();
                if !updates.is_empty() {
                    send_system_message(&self.gate_ref, msg.session_id, "任务进度更新：获得物品");
                }
                for (quest_index, _item_index, complete) in updates {
                    debug!("QuestItem: session={} quest={} complete={}", msg.session_id, quest_index, complete);
                }
            }
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "附近没有可以拾取的物品。");
        }
    }
}

impl Message<MoveItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: MoveItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        // 客户端发来的 grid 是 MirGridType，实际移动的源/目标槽位是 from/to
        let success = record.actor_ref.ask(InventoryMoveItem {
            from_grid: msg.from as u8,
            to_grid: msg.to as u8,
        }).await.unwrap_or(false);

        if success {
            // 发送 ItemChanged 通知（用 MoveItem 响应）
            send_move_item_response(&self.gate_ref, msg.session_id, msg.grid, msg.from, msg.to, true);
        }
    }
}

impl Message<UseItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: UseItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        // Check map no_drug flag
        let player_state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if player_state.is_dead { return; }
        if let Some(mi) = self.map_infos.get(&(player_state.map_index as i32)) {
            if mi.no_drug {
                send_system_message(&self.gate_ref, msg.session_id, "该地图无法使用物品");
                return;
            }
        }

        // 查询物品信息
        let user_item = record.actor_ref.ask(GetItemInfo { unique_id: msg.unique_id }).await.unwrap_or(None);
        let item_index = match user_item {
            Some(ref item) => item.item_index,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "物品不存在");
                return;
            }
        };

        let item_db = self.item_infos.get(&item_index).cloned();

        // 消耗品：扣减 count 或移除
        let consumed = record.actor_ref.ask(ConsumeItem { unique_id: msg.unique_id }).await.unwrap_or(false);
        if !consumed {
            send_system_message(&self.gate_ref, msg.session_id, "使用物品失败");
            return;
        }

        debug!("Player session={} used item uid={} index={}", msg.session_id, msg.unique_id, item_index);

        // 特殊物品：双倍经验卷（不依赖 item_type）
        if item_index == 4 {
            let duration_ticks = 6000; // 10分钟 = 6000 ticks @ 100ms
            let end_tick = self.tick_count + duration_ticks;
            let _ = record.actor_ref.ask(SetExpMultiplier {
                multiplier: 2.0,
                end_tick,
            }).await;
            send_system_message(&self.gate_ref, msg.session_id, "双倍经验效果已启动，持续10分钟！");
            debug!("DoubleExpScroll: {} activated 2x exp for 10 min", player_state.name);
        }

        // 根据物品类型执行效果
        if let Some(ref db) = item_db {
            match db.item_type {
                // Potion
                13 => {
                    use mir2_shared::enums::Stat;
                    let hp_recover = db.stats.get(&(Stat::HP as u8)).copied().unwrap_or(0);
                    let mp_recover = db.stats.get(&(Stat::MP as u8)).copied().unwrap_or(0);
                    if hp_recover > 0 {
                        let _ = record.actor_ref.ask(crate::actors::player::Heal {
                            amount: hp_recover,
                        }).await;
                    }
                    if mp_recover > 0 {
                        let _ = record.actor_ref.ask(crate::actors::player::AddMP {
                            amount: mp_recover,
                        }).await;
                    }
                    if hp_recover > 0 || mp_recover > 0 {
                        debug!("Potion: {} recovered hp={} mp={}", player_state.name, hp_recover, mp_recover);
                    }
                }
                // Scroll (回城卷 / 随机传送卷)
                17 => {
                    if let Some(mi) = self.map_infos.get(&(player_state.map_index as i32)) {
                        if mi.no_escape {
                            send_system_message(&self.gate_ref, msg.session_id, "该地图无法使用传送卷");
                            return;
                        }
                        match item_index {
                            // 回城卷 -> 传送到当前地图安全区
                            2 => {
                                if mi.no_town_teleport {
                                    send_system_message(&self.gate_ref, msg.session_id, "该地图无法使用回城卷");
                                    return;
                                }
                                let (tx, ty) = self.maps.get(&player_state.map_index)
                                    .and_then(|m| m.safe_zone_rects.first())
                                    .map(|(x1, y1, x2, y2)| ((x1 + x2) / 2, (y1 + y2) / 2))
                                    .unwrap_or((330, 330));
                                let _ = record.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                                    x: tx,
                                    y: ty,
                                    direction: player_state.direction,
                                    map_index: None,
                                    is_mounted: None,
                                }).await;
                                send_system_message(&self.gate_ref, msg.session_id, "已返回安全区");
                                debug!("Scroll: {} teleported to safe zone ({}, {})", player_state.name, tx, ty);
                            }
                            // 随机传送卷 -> 传送到当前地图随机可行走位置
                            3 => {
                                if mi.no_random {
                                    send_system_message(&self.gate_ref, msg.session_id, "该地图无法使用随机传送卷");
                                    return;
                                }
                                if let Some(map) = self.maps.get(&player_state.map_index) {
                                    let (max_x, max_y) = (map.width as i32, map.height as i32);
                                    let mut attempts = 0;
                                    let mut rx = player_state.x;
                                    let mut ry = player_state.y;
                                    while attempts < 20 {
                                        let cx = fastrand::i32(0..max_x);
                                        let cy = fastrand::i32(0..max_y);
                                        if map.is_walkable(cx, cy) {
                                            rx = cx;
                                            ry = cy;
                                            break;
                                        }
                                        attempts += 1;
                                    }
                                    let _ = record.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                                        x: rx,
                                        y: ry,
                                        direction: player_state.direction,
                                        map_index: None,
                                        is_mounted: None,
                                    }).await;
                                    send_system_message(&self.gate_ref, msg.session_id, "随机传送完成");
                                    debug!("RandomScroll: {} teleported to ({}, {})", player_state.name, rx, ry);
                                }
                            }
                            _ => {
                                // 未知卷轴 -> 默认回城行为
                                let (tx, ty) = self.maps.get(&player_state.map_index)
                                    .and_then(|m| m.safe_zone_rects.first())
                                    .map(|(x1, y1, x2, y2)| ((x1 + x2) / 2, (y1 + y2) / 2))
                                    .unwrap_or((330, 330));
                                let _ = record.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                                    x: tx,
                                    y: ty,
                                    direction: player_state.direction,
                                    map_index: None,
                                    is_mounted: None,
                                }).await;
                                send_system_message(&self.gate_ref, msg.session_id, "已返回安全区");
                            }
                        }
                    }
                }
                // LotteryTicket (item_type=12, per C# PlayerObject)
                12 => {
                    let effect = db.effect.max(1) as usize;
                    let prizes: [(&str, i64); 6] = [
                        ("一等奖！获得 1,000,000 金币", 1_000_000),
                        ("二等奖！获得 200,000 金币", 200_000),
                        ("三等奖！获得 100,000 金币", 100_000),
                        ("四等奖！获得 10,000 金币", 10_000),
                        ("五等奖！获得 1,000 金币", 1_000),
                        ("六等奖！获得 500 金币", 500),
                    ];
                    let mut won = false;
                    for (i, (msg_text, gold)) in prizes.iter().enumerate() {
                        if fastrand::usize(..effect * (i + 1)) == 0 {
                            let _ = record.actor_ref.ask(crate::actors::player::AddGold {
                                amount: *gold as u64,
                            }).await;
                            send_system_message(&self.gate_ref, msg.session_id, msg_text);
                            won = true;
                            break;
                        }
                    }
                    if !won {
                        send_system_message(&self.gate_ref, msg.session_id, "很遗憾，你没有中奖。");
                    }
                }
                _ => {}
            }
        }

        // 发送 UseItem 响应
        send_use_item_response(&self.gate_ref, msg.session_id, msg.unique_id);
    }
}

impl Message<EquipItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: EquipItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let slot = match EquipmentSlot::from_i32(msg.slot) {
            Some(s) => s,
            None => return,
        };

        // 按 unique_id 在背包中定位格子（客户端发来的 grid 是 MirGridType）
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        let Some(grid_idx) = state.inventory.backpack.iter().position(|s| {
            s.as_ref().map_or(false, |slot| slot.item.unique_id == msg.unique_id)
        }) else {
            send_system_message(&self.gate_ref, msg.session_id, "找不到该物品");
            return;
        };

        let result = record.actor_ref.ask(InventoryEquipItem {
            grid: grid_idx as u8,
            slot,
        }).await.unwrap_or(None);

        match result {
            Some((_old_equipment, _new_uid)) => {
                debug!("Player session={} equipped item uid={} to slot {}", msg.session_id, msg.unique_id, msg.slot);
                send_equip_item_response(&self.gate_ref, msg.session_id, msg.grid, msg.unique_id, msg.slot, true);

                // 重新计算装备加成 + 广播视觉变化
                if let Some(state) = self.recalculate_and_set_stat_bonuses(msg.session_id).await {
                    self.broadcast_equipment_visuals(msg.session_id, &state).await;
                }
            }
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "装备失败");
            }
        }
    }
}

impl Message<RemoveItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: RemoveItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        // 找到该 uid 在哪个装备槽位
        let mut found_slot = None;
        for slot_idx in 0..EquipmentSlot::COUNT {
            let slot = EquipmentSlot::from_i32(slot_idx as i32).unwrap();
            let eq_info = record.actor_ref.ask(GetEquipmentInfo { slot }).await.unwrap_or(None);
            if let Some(eq) = eq_info {
                if eq.unique_id == msg.unique_id {
                    found_slot = Some(slot);
                    break;
                }
            }
        }

        let Some(slot) = found_slot else { return; };

        let result = record.actor_ref.ask(InventoryUnequipItem { slot }).await;
        match result {
            Ok(true) => {
                debug!("Player session={} unequipped item uid={} from slot {:?}", msg.session_id, msg.unique_id, slot);
                send_remove_item_response(&self.gate_ref, msg.session_id, msg.grid, msg.unique_id, true);

                // 重新计算装备加成 + 广播视觉变化
                if let Some(state) = self.recalculate_and_set_stat_bonuses(msg.session_id).await {
                    self.broadcast_equipment_visuals(msg.session_id, &state).await;
                }
            }
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "背包已满，无法卸下装备");
            }
        }
    }
}

impl Message<DropItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: DropItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let actor_ref = match self.players.get(&msg.session_id) {            Some(r) => r.actor_ref.clone(),            None => return,
        };

        let state = match actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if state.is_dead { return; }

        let item = actor_ref.ask(DropInventoryItem {
            unique_id: msg.unique_id,
            count: msg.count,
        }).await.unwrap_or(None);
        if let Some(mut item) = item {
            let player_pos = (state.x, state.y);

            debug!("Player session={} dropped item uid={}", msg.session_id, msg.unique_id);

            // 补 ItemInfo（ObjectItem 携带 info 供客户端渲染图标/名称）
            super::enrich_item_info(&mut item, &self.item_infos);

            // 广播 ObjectItem 给所有玩家
            let drop_oid = self.alloc_object_id();
            let object_item = mir2_shared::packets::server::ObjectItem {
                object_id: drop_oid,
                item: item.clone(),
                location_x: player_pos.0,
                location_y: player_pos.1,
            };
            let mut buf = Vec::new();
            if mir2_shared::packets::base::serialize_packet(&mut std::io::Cursor::new(&mut buf), &object_item).is_ok() {
                for sid in self.players.keys() {
                    let _ = self.gate_ref.tell(SendToClient { session_id: *sid, data: buf.clone() }).await;
                }
            }

            // 添加到地面物品
            self.ground_items.push(GroundItem {
                object_id: drop_oid,
                item: item.clone(),
                x: player_pos.0,
                y: player_pos.1,
                map_index: state.map_index,
                dropper_session: Some(msg.session_id),
                drop_tick: self.tick_count,
            });

            send_drop_item_response(&self.gate_ref, msg.session_id, msg.unique_id, msg.count as u32, true);
            // 完整 UserInformation 刷新（含背包/装备，客户端按权威状态重建）
            // 注意：build_user_information_packet 已含包头发送帧，直接 SendToClient
            if let Ok(Some(state)) = actor_ref.ask(GetPlayerState).await {
                let packet = super::build_user_information_packet(&state, &self.item_infos);
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: msg.session_id,
                    data: packet,
                }).await;
            }
        }
    }
}

impl Message<MergeItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: MergeItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let actor_ref = match self.players.get(&msg.session_id) {            Some(r) => r.actor_ref.clone(),            None => return,
        };

        let success = actor_ref.ask(MergeInventoryItemByUid {
            from_uid: msg.from_uid,
            to_uid: msg.to_uid,
        }).await.unwrap_or(false);

        if success {
            send_merge_item_response(&self.gate_ref, msg.session_id, msg.grid_from, msg.grid_to, msg.from_uid, msg.to_uid, true);
            // 完整 UserInformation 刷新（含背包/装备，客户端按权威状态重建）
            // 注意：build_user_information_packet 已含包头发送帧，直接 SendToClient
            if let Ok(Some(state)) = actor_ref.ask(GetPlayerState).await {
                let packet = super::build_user_information_packet(&state, &self.item_infos);
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: msg.session_id,
                    data: packet,
                }).await;
            }
        }
    }
}

impl Message<SplitItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: SplitItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let actor_ref = match self.players.get(&msg.session_id) {            Some(r) => r.actor_ref.clone(),            None => return,
        };

        let success = actor_ref.ask(InventorySplitItem {
            unique_id: msg.unique_id,
            count: msg.count as u16,
        }).await.unwrap_or(false);

        if success {
            send_split_item_response(&self.gate_ref, msg.session_id, msg.grid, msg.unique_id, msg.count);
            // 完整 UserInformation 刷新（含背包/装备，客户端按权威状态重建）
            // 注意：build_user_information_packet 已含包头发送帧，直接 SendToClient
            if let Ok(Some(state)) = actor_ref.ask(GetPlayerState).await {
                let packet = super::build_user_information_packet(&state, &self.item_infos);
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: msg.session_id,
                    data: packet,
                }).await;
            }
        }
    }
}

impl Message<DropGoldRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: DropGoldRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if state.is_dead { return; }

        if msg.amount == 0 {
            return;
        }

        if state.inventory.gold < msg.amount as u64 {
            send_system_message(&self.gate_ref, msg.session_id, "金币不足");
            return;
        }

        let amount = msg.amount as u64;
        let success = record.actor_ref.ask(DropGold { amount }).await.unwrap_or(false);
        if success {
            let player_pos = match record.actor_ref.ask(GetPlayerState).await {
                Ok(Some(s)) => (s.x, s.y),
                _ => return,
            };

            // 广播 ObjectGold 给所有玩家
            let drop_oid = self.alloc_object_id();
            let object_gold = mir2_shared::packets::server::ObjectGold {
                object_id: drop_oid,
                gold: amount as u32,
                location_x: player_pos.0,
                location_y: player_pos.1,
            };
            let mut buf = Vec::new();
            if mir2_shared::packets::base::serialize_packet(
                &mut std::io::Cursor::new(&mut buf), &object_gold).is_ok() {
                for sid in self.players.keys() {
                    let _ = self.gate_ref.tell(SendToClient { session_id: *sid, data: buf.clone() }).await;
                }
            }

            // 地面金币（用特殊物品表示）
            let gold_item = mir2_shared::data::item::UserItem {
                item_index: 0, // 0 = gold marker
                count: amount as u16,
                ..Default::default()
            };
            self.ground_items.push(GroundItem {
                object_id: drop_oid,
                item: gold_item,
                x: player_pos.0,
                y: player_pos.1,
                map_index: state.map_index,
                dropper_session: Some(msg.session_id),
                drop_tick: self.tick_count,
            });

            // 通知客户端金币变化
            send_gold_changed_packet(&self.gate_ref, msg.session_id, state.inventory.gold - amount);
            debug!("DropGold: {} dropped {} gold", state.name, msg.amount);
        }
    }
}

impl Message<BuyItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: BuyItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 查找 NPC 并验证商品是否在销售列表中
        let npc_db_index = match self.npcs.get(&msg.npc_id) {
            Some(n) => n.db_index,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "找不到该 NPC");
                return;
            }
        };

        // 获取商品列表（可变引用以便扣减库存）
        let goods_list = match self.npc_goods.get_mut(&npc_db_index) {
            Some(list) => list,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "该 NPC 不出售任何物品");
                return;
            }
        };
        let good_idx = match goods_list.iter().position(|g| g.item_index == msg.item_index as i32) {
            Some(idx) => idx,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "该 NPC 不出售此物品");
                return;
            }
        };

        // 检查库存
        let good = &goods_list[good_idx];
        if !good.infinite_stock && good.stock <= 0 {
            send_system_message(&self.gate_ref, msg.session_id, "该物品已售罄");
            return;
        }
        if !good.infinite_stock && good.stock < msg.count as i32 {
            send_system_message(&self.gate_ref, msg.session_id, &format!("库存不足（仅剩 {} 个）", good.stock));
            return;
        }

        // Validate item against DB-loaded item_infos
        let item_db = match self.item_infos.get(&(msg.item_index as i32)) {
            Some(i) => i,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "物品不存在");
                return;
            }
        };

        // 计算价格：优先使用 npc_goods 中的自定义价格，否则使用 item_db.price * NPC rate（整数运算避免浮点误差）
        let npc_rate = self.npc_infos.get(&npc_db_index).map(|n| n.rate).unwrap_or(100).max(1) as u64;
        let base_price = if good.price > 0 { good.price as u64 } else { item_db.price as u64 };
        let price_per_unit = ((base_price * npc_rate) / 100).max(1);
        let total_price = price_per_unit * msg.count as u64;

        // 检查金币
        if state.inventory.gold < total_price {
            send_system_message(&self.gate_ref, msg.session_id, "金币不足");
            return;
        }

        // 扣除金币
        let _ = record.actor_ref.ask(DeductGold { amount: total_price }).await;

        // 扣减库存
        if !goods_list[good_idx].infinite_stock {
            goods_list[good_idx].stock -= msg.count as i32;
        }

        // Create item from DB template
        let item = mir2_shared::data::item::UserItem {
            item_index: msg.item_index as i32,
            count: msg.count as u16,
            max_dura: item_db.durability as u16,
            current_dura: item_db.durability as u16,
            ..Default::default()
        };

        let _ = record.actor_ref.ask(AddItemToInventory { item }).await;
        let updates = record.actor_ref.ask(crate::actors::player::CheckQuestItemProgress).await.unwrap_or_default();
        if !updates.is_empty() {
            send_system_message(&self.gate_ref, msg.session_id, "任务进度更新：获得物品");
        }
        send_system_message(&self.gate_ref, msg.session_id, &format!("购买成功 (花费 {} 金币)", total_price));
        let npc_name = self.npcs.get(&msg.npc_id).map(|n| n.name.as_str()).unwrap_or("?");
        debug!("BuyItem: {} bought item={} ({}) x{} for {} gold from NPC '{}' (stock={})", state.name, item_db.name, msg.item_index, msg.count, total_price, npc_name,
            if goods_list[good_idx].infinite_stock { "∞".to_string() } else { goods_list[good_idx].stock.to_string() });
    }
}

impl Message<SellItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: SellItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查物品是否在背包中
        let item_data = match state.inventory.get_item(msg.unique_id) {
            Some(i) => i.clone(),
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "物品不存在");
                return;
            }
        };

        // 移除物品
        let removed = record.actor_ref.ask(RemoveItemFromInventory { unique_id: msg.unique_id }).await.unwrap_or(None);
        if removed.is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "移除物品失败");
            return;
        }

        // 定价：基于 DB 中物品的 price（卖价通常为买价的一半）
        let item_db_price = self.item_infos.get(&item_data.item_index)
            .map(|i| i.price as u64)
            .unwrap_or(item_data.item_index as u64 * 5);
        let total_gold = (item_db_price / 2).max(1) * msg.count as u64;

        let success = record.actor_ref.ask(AddGold { amount: total_gold }).await.unwrap_or(false);
        if success {
            // 记录到回购列表（最多保留 10 个）
            let buyback = BuybackItem {
                item: item_data.clone(),
                sell_price: total_gold,
            };
            let list = self.buyback_items.entry(msg.session_id).or_default();
            list.insert(0, buyback);
            while list.len() > 10 {
                list.pop();
            }
            send_sell_item_response(&self.gate_ref, msg.session_id, msg.unique_id, msg.count, true);
            debug!("SellItem: {} sold item={} x{} for {} gold", state.name, item_data.item_index, msg.count, total_gold);
        }
    }
}

impl Message<RepairItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: RepairItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 获取物品信息计算修理费
        let item_data = match state.inventory.get_item(msg.unique_id) {
            Some(i) => i.clone(),
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "物品不存在");
                return;
            }
        };

        // 计算耐久缺失和修理费
        let dura_deficit = item_data.max_dura.saturating_sub(item_data.current_dura) as u64;
        if dura_deficit == 0 {
            send_system_message(&self.gate_ref, msg.session_id, "该物品不需要修理");
            return;
        }
        let repair_cost = dura_deficit * REPAIR_COST_PER_DURA;

        // 检查金币
        if state.inventory.gold < repair_cost {
            send_system_message(&self.gate_ref, msg.session_id, &format!("金币不足（需要 {} 金币）", repair_cost));
            return;
        }

        // 扣除金币
        let _ = record.actor_ref.ask(DeductGold { amount: repair_cost }).await;

        // 执行修理
        let success = record.actor_ref.ask(crate::actors::player::RepairItem { unique_id: msg.unique_id }).await.unwrap_or(false);
        if success {
            send_system_message(&self.gate_ref, msg.session_id, &format!("修理成功（花费 {} 金币）", repair_cost));
            debug!("RepairItem: {} repaired item={} cost={}", state.name, msg.unique_id, repair_cost);
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "修理失败");
        }
    }
}

impl Message<EquipSlotItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: EquipSlotItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };

        let equip_slot = match crate::actors::inventory::EquipmentSlot::from_i32(msg.to_slot) {
            Some(s) => s,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "无效装备槽");
                return;
            }
        };

        // 从 source grid 中查找物品的 backpack index
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let grid_idx = state.inventory.backpack.iter()
            .position(|s| s.as_ref().map_or(false, |slot| slot.item.unique_id == msg.unique_id));

        let Some(grid) = grid_idx else {
            send_system_message(&self.gate_ref, msg.session_id, "找不到该物品");
            return;
        };

        let result = record.actor_ref.ask(crate::actors::player::InventoryEquipItem {
            grid: grid as u8,
            slot: equip_slot,
        }).await.unwrap_or(None);

        if result.is_some() {
            debug!("EquipSlotItem: {} equipped uid={} to slot {:?}", state.name, msg.unique_id, equip_slot);
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "装备失败");
        }
    }
}

impl Message<ReplaceWedRingRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: ReplaceWedRingRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查物品是否在背包中
        if state.inventory.get_item(msg.unique_id).is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "物品不存在");
            return;
        }

        // 找到该物品在背包中的格子
        let grid = state.inventory.backpack.iter()
            .find_map(|s| s.as_ref().filter(|slot| slot.item.unique_id == msg.unique_id).map(|slot| slot.grid));

        let Some(grid) = grid else {
            send_system_message(&self.gate_ref, msg.session_id, "物品不在背包中");
            return;
        };

        // 装备到戒指槽（优先左戒指槽，如果已有则右戒指槽）
        let target_slot = if state.inventory.get_equipment(crate::actors::inventory::EquipmentSlot::RingL).is_none() {
            crate::actors::inventory::EquipmentSlot::RingL
        } else {
            crate::actors::inventory::EquipmentSlot::RingR
        };

        let result = record.actor_ref.ask(crate::actors::player::InventoryEquipItem { grid, slot: target_slot }).await.unwrap_or(None);
        if result.is_some() {
            send_system_message(&self.gate_ref, msg.session_id, "戒指已更换");
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "戒指装备失败");
        }
    }
}

impl Message<StoreItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: StoreItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查仓库是否有空位
        if !state.inventory.storage_has_space() {
            send_system_message(&self.gate_ref, msg.session_id, "仓库已满");
            return;
        }

        // 检查物品是否在背包中
        if state.inventory.get_item(msg.uid).is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "物品不存在");
            return;
        }

        // 执行存入
        let result = record.actor_ref.ask(StoreItem { grid: msg.grid }).await;
        match result {
            Ok(true) => {
                send_store_item_packet(&self.gate_ref, msg.session_id, msg.grid, true);
                debug!("StoreItem: {} grid={} uid={}", state.name, msg.grid, msg.uid);
            }
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "存入仓库失败");
            }
        }
    }
}

impl Message<TakeBackItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: TakeBackItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查背包是否有空位
        if !state.inventory.has_space() {
            send_system_message(&self.gate_ref, msg.session_id, "背包已满");
            return;
        }

        // 执行取出
        let result = record.actor_ref.ask(TakeBackItem { grid: msg.grid }).await;
        match result {
            Ok(true) => {
                send_take_back_item_packet(&self.gate_ref, msg.session_id, msg.grid, true);
                debug!("TakeBackItem: {} grid={} uid={}", state.name, msg.grid, msg.uid);
            }
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "取出物品失败");
            }
        }
    }
}

impl Message<CraftItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: CraftItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 查找配方
        let recipes = get_craft_recipes();
        let recipe = match recipes.iter().find(|r| r.recipe_id == msg.recipe_id) {
            Some(r) => r.clone(),
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "未知配方");
                let mut body = Vec::new();
                body.extend_from_slice(&msg.recipe_id.to_le_bytes());
                body.extend_from_slice(&0u16.to_le_bytes());
                body.push(0u8); // success = false
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::CraftItem as i16, &body),
                }).await;
                return;
            }
        };

        // 检查背包空间
        if !state.inventory.has_space() {
            send_system_message(&self.gate_ref, msg.session_id, "背包已满");
            let mut body = Vec::new();
            body.extend_from_slice(&msg.recipe_id.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.push(0u8);
            let _ = self.gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::CraftItem as i16, &body),
            }).await;
            return;
        }

        // 检查材料
        for ing in &recipe.ingredients {
            let has = record.actor_ref.ask(crate::actors::player::HasItem {
                item_index: ing.item_index,
                count: ing.count,
            }).await.unwrap_or(false);
            if !has {
                send_system_message(&self.gate_ref, msg.session_id, "材料不足");
                let mut body = Vec::new();
                body.extend_from_slice(&msg.recipe_id.to_le_bytes());
                body.extend_from_slice(&0u16.to_le_bytes());
                body.push(0u8);
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::CraftItem as i16, &body),
                }).await;
                return;
            }
        }

        // 扣除材料
        for ing in &recipe.ingredients {
            let _ = record.actor_ref.ask(crate::actors::player::RemoveItemByIndex {
                item_index: ing.item_index,
                count: ing.count,
            }).await;
        }

        // 成功率判定
        let success = fastrand::u8(0..100) < recipe.success_rate;

        if success {
            let mut item = mir2_shared::data::item::UserItem {
                item_index: recipe.product_index,
                count: recipe.product_count,
                ..Default::default()
            };
            if let Some(info) = self.item_infos.get(&recipe.product_index) {
                item.max_dura = info.durability as u16;
                item.current_dura = info.durability as u16;
            }
            let _ = record.actor_ref.ask(crate::actors::player::AddItemToInventory { item }).await;
            send_system_message(&self.gate_ref, msg.session_id, "合成成功！");
            debug!("CraftItem: {} recipe={} success", state.name, msg.recipe_id);
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "合成失败，材料已消耗");
            debug!("CraftItem: {} recipe={} failed", state.name, msg.recipe_id);
        }

        // 发送 CraftItem 响应
        let mut body = Vec::new();
        body.extend_from_slice(&msg.recipe_id.to_le_bytes());
        body.extend_from_slice(&(if success { recipe.product_count } else { 0 }).to_le_bytes());
        body.push(if success { 1u8 } else { 0u8 });
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::CraftItem as i16, &body),
        }).await;
    }
}

impl Message<BuyItemBackRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: BuyItemBackRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 查找回购列表中的对应物品
        let list = match self.buyback_items.get_mut(&msg.session_id) {
            Some(l) => l,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "没有可回购的物品");
                return;
            }
        };
        let idx = match list.iter().position(|b| b.item.item_index == msg.item_index as i32) {
            Some(i) => i,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "该物品已无法回购");
                return;
            }
        };

        let buyback = list.remove(idx);
        let cost = buyback.sell_price * 2;

        // 检查背包空间
        if !state.inventory.has_space() {
            send_system_message(&self.gate_ref, msg.session_id, "背包已满");
            list.insert(idx, buyback);
            return;
        }

        // 扣除金币
        let deducted = record.actor_ref.ask(crate::actors::player::DeductGold { amount: cost }).await.unwrap_or(false);
        if !deducted {
            send_system_message(&self.gate_ref, msg.session_id, "金币不足");
            list.insert(idx, buyback);
            return;
        }

        // 添加物品到背包
        let _ = record.actor_ref.ask(crate::actors::player::AddItemToInventory {
            item: buyback.item.clone(),
        }).await;

        send_system_message(&self.gate_ref, msg.session_id, &format!("回购成功，花费 {} 金币", cost));
        debug!("BuyItemBack: {} item_index={} cost={}", state.name, msg.item_index, cost);
    }
}

impl Message<CombineItemRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: CombineItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        let from_grid = msg.from_grid as u8;
        let to_grid = msg.to_grid as u8;

        // 获取源物品和目标物品
        let source = match record.actor_ref.ask(crate::actors::player::GetItemInfoByGrid { grid: from_grid }).await {
            Ok(Some(i)) => i,
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "找不到源物品");
                self.send_combine_item_response(msg.session_id, 0, 0, false, false);
                return;
            }
        };
        let target = match record.actor_ref.ask(crate::actors::player::GetItemInfoByGrid { grid: to_grid }).await {
            Ok(Some(i)) => i,
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "找不到目标物品");
                self.send_combine_item_response(msg.session_id, 0, 0, false, false);
                return;
            }
        };

        // 获取物品信息
        let source_info = match self.item_infos.get(&source.item_index) {
            Some(i) => i,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "无法识别源物品");
                self.send_combine_item_response(msg.session_id, source.unique_id, target.unique_id, false, false);
                return;
            }
        };
        let target_info = match self.item_infos.get(&target.item_index) {
            Some(i) => i,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "无法识别目标物品");
                self.send_combine_item_response(msg.session_id, source.unique_id, target.unique_id, false, false);
                return;
            }
        };

        // 源物品必须是宝石 (ItemType::Gem = 18)
        if source_info.item_type != 18 {
            send_system_message(&self.gate_ref, msg.session_id, "源物品不是宝石");
            self.send_combine_item_response(msg.session_id, source.unique_id, target.unique_id, false, false);
            return;
        }

        // 目标物品必须是可镶嵌的装备
        let can_socket = matches!(target_info.item_type,
            1 | 2 | 4 | 5 | 6 | 7 | 9 | 10 | 19
        );
        if !can_socket {
            send_system_message(&self.gate_ref, msg.session_id, "该物品无法镶嵌宝石");
            self.send_combine_item_response(msg.session_id, source.unique_id, target.unique_id, false, false);
            return;
        }

        // 检查目标物品是否有空槽位
        let slot_count = target_info.slots as usize;
        let filled_slots = target.slots.iter().filter(|s| s.is_some()).count();
        if slot_count == 0 || filled_slots >= slot_count {
            send_system_message(&self.gate_ref, msg.session_id, "目标物品没有空槽位");
            self.send_combine_item_response(msg.session_id, source.unique_id, target.unique_id, false, false);
            return;
        }

        // 执行镶嵌
        let result = record.actor_ref.ask(crate::actors::player::SocketGem {
            from_grid,
            to_grid,
            target_slot_count: slot_count,
        }).await.ok().flatten();

        if let Some((source_uid, target_uid)) = result {
            send_system_message(&self.gate_ref, msg.session_id, "宝石镶嵌成功！");
            self.send_combine_item_response(msg.session_id, source_uid, target_uid, true, true);
            debug!("CombineItem: {} socketed gem {} into item {}", state.name, source_uid, target_uid);
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "宝石镶嵌失败");
            self.send_combine_item_response(msg.session_id, source.unique_id, target.unique_id, false, false);
        }
    }
}

impl Message<DisassembleItemRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: DisassembleItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        // 查找物品
        let item = match record.actor_ref.ask(crate::actors::player::GetItemInfo { unique_id: msg.unique_id }).await {
            Ok(Some(i)) => i,
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "找不到该物品");
                return;
            }
        };

        // 获取物品信息
        let item_info = match self.item_infos.get(&item.item_index) {
            Some(i) => i,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "该物品无法分解");
                return;
            }
        };

        // 只有装备类物品可以分解（有耐久度的非消耗品）
        if item_info.durability <= 0 || item_info.item_type == 0 {
            send_system_message(&self.gate_ref, msg.session_id, "该物品无法分解");
            return;
        }

        // 分解产出 = 根据等级和类型决定
        let grade = item_info.grade.max(1);
        let item_name = item_info.name.clone();
        let (mat_index, mat_count, mat_name) = match item_info.item_type {
            // 武器 -> 铁矿石
            1 => (500, grade as u16, "铁矿石"),
            // 盔甲/饰品 -> 布料/皮革
            2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 => (501, grade as u16, "皮革"),
            _ => (502, (grade / 2).max(1) as u16, "宝石碎片"),
        };

        // 移除原物品
        let removed = record.actor_ref.ask(crate::actors::player::RemoveItemFromInventory {
            unique_id: msg.unique_id,
        }).await.ok().flatten();
        if removed.is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "分解失败：无法移除物品");
            return;
        }

        // 给予材料
        let material = crate::actors::inventory::make_item(mat_index, mat_count);
        let added = record.actor_ref.ask(crate::actors::player::AddItemToInventory { item: material }).await.unwrap_or(false);
        if added {
            send_system_message(&self.gate_ref, msg.session_id,
                &format!("分解成功！获得 {} x{}", mat_name, mat_count));
        } else {
            // 背包满了：把材料丢到地上
            let drop_oid = self.alloc_object_id();
            let object_item = mir2_shared::packets::server::ObjectItem {
                object_id: drop_oid,
                item: mir2_shared::data::item::UserItem {
                    item_index: mat_index,
                    count: mat_count,
                    ..Default::default()
                },
                location_x: state.x,
                location_y: state.y,
            };
            let mut buf = Vec::new();
            if let Err(e) = mir2_shared::packets::base::serialize_packet(&mut std::io::Cursor::new(&mut buf), &object_item) {
                warn!("Failed to serialize disassemble drop: {}", e);
            } else {
                for sid in self.players.keys() {
                    let _ = self.gate_ref.tell(SendToClient { session_id: *sid, data: buf.clone() }).await;
                }
                self.ground_items.push(GroundItem {
                    object_id: drop_oid,
                    item: mir2_shared::data::item::UserItem {
                        item_index: mat_index,
                        count: mat_count,
                        ..Default::default()
                    },
                    x: state.x,
                    y: state.y,
                    map_index: state.map_index,
                    dropper_session: Some(msg.session_id),
                    drop_tick: self.tick_count,
                });
            }
            send_system_message(&self.gate_ref, msg.session_id,
                &format!("分解成功！背包已满，{} x{} 已掉落在地", mat_name, mat_count));
        }
        debug!("DisassembleItem: {} disassembled {} into {} x{}", state.name, item_name, mat_name, mat_count);
    }
}
