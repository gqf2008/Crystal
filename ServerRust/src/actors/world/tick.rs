use super::*;

/// 游戏主循环 Tick
pub struct Tick;

impl WorldActor {
    /// 玩家 Buff tick + 死亡复活（每 5 ticks）
    pub(crate) async fn tick_buffs_and_revive(&mut self) {
        if self.tick_count % 5 == 0 {
            let mut to_revive = Vec::new();
            let mut to_remove = Vec::new();
            for (session_id, record) in &self.players {
                let _ = record.actor_ref.ask(crate::actors::player::TickBuff).await;
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    if state.is_dead {
                        match self.player_death_queue.get(session_id) {
                            None => {
                                self.player_death_queue.insert(*session_id, self.tick_count);
                            }
                            Some(death_tick) => {
                                if self.tick_count >= death_tick + 60 {
                                    to_revive.push(*session_id);
                                }
                            }
                        }
                    } else if self.player_death_queue.contains_key(session_id) {
                        to_remove.push(*session_id);
                    }
                }
            }
            for session_id in to_remove {
                self.player_death_queue.remove(&session_id);
            }
            for session_id in to_revive {
                self.player_death_queue.remove(&session_id);
                if let Some(record) = self.players.get(&session_id) {
                    let _ = record.actor_ref.ask(crate::actors::player::Revive).await;
                }
            }

            // 怪物 Poison tick（与玩家同步，每 5 ticks 推进 1 秒）
            for (_, monster) in &mut self.monsters {
                if monster.poison_list.is_empty() {
                    continue;
                }
                let dmg = crate::combat::poison::tick_poisons(&mut monster.poison_list, 1);
                if dmg > 0 {
                    monster.hp = monster.hp.saturating_sub(dmg);
                }
            }
        }
    }

    /// 地图环境伤害 + 禁止坐骑地图自动下坐骑（每 20 ticks）
    pub(crate) async fn tick_environment_damage(&mut self) {
        if self.tick_count % 20 == 0 {
            for (session_id, record) in &self.players {
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    if state.is_dead { continue; }
                    if let Some(mi) = self.map_infos.get(&(state.map_index as i32)) {
                        if mi.fire || mi.lightning {
                            let in_safe = self.maps.get(&state.map_index)
                                .map(|m| m.is_safe_zone(state.x, state.y))
                                .unwrap_or(false);
                            if in_safe { continue; }
                            let damage = if mi.fire { mi.fire_damage } else { mi.lightning_damage };
                            if damage > 0 {
                                let died = record.actor_ref.ask(TakeDamage {
                                    attacker_id: 0, // environment
                                    attacker_session: 0,
                                    damage,
                                }).await.unwrap_or(false);
                                if died {
                                    self.player_death_queue.insert(*session_id, self.tick_count);
                                    broadcast_system_message(&self.gate_ref, &self.players,
                                        &format!("{} 在{}中倒下了", state.name,
                                            if mi.fire { "火海" } else { "雷暴" }));
                                } else {
                                    let msg = if mi.fire { "你受到了火焰伤害！" } else { "你受到了闪电伤害！" };
                                    send_system_message(&self.gate_ref, *session_id, msg);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 自动下坐骑：进入禁止坐骑地图时
        if self.tick_count % 20 == 0 {
            let mut to_dismount: Vec<u64> = Vec::new();
            for (session_id, record) in &self.players {
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    if state.is_mounted {
                        if let Some(mi) = self.map_infos.get(&(state.map_index as i32)) {
                            if mi.no_mount {
                                to_dismount.push(*session_id);
                            }
                        }
                    }
                }
            }
            for session_id in to_dismount {
                self.dismount_player(session_id).await;
                send_system_message(&self.gate_ref, session_id, "该地图禁止骑乘坐骑，已自动下坐骑");
            }
        }
    }

    /// 经验倍率过期、全局事件过期、随机世界事件、隐身过期（每 100 ticks）
    pub(crate) async fn tick_exp_events_and_invisibility(&mut self) {
        if self.tick_count % 100 == 0 {
            for (session_id, record) in &self.players {
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    if state.exp_multiplier > 1.0 && self.tick_count >= state.exp_multiplier_end_tick {
                        let _ = record.actor_ref.ask(SetExpMultiplier {
                            multiplier: 1.0,
                            end_tick: 0,
                        }).await;
                        send_system_message(&self.gate_ref, *session_id, "双倍经验效果已结束");
                        debug!("Exp multiplier expired for session {}", session_id);
                    }
                }
            }
            // 全局事件过期广播
            if self.tick_count >= self.global_exp_event_end_tick && self.global_exp_event_end_tick > 0 {
                let event_name = self.global_event_name.take().unwrap_or_else(|| "活动".to_string());
                self.global_exp_multiplier = 1.0;
                self.global_drop_multiplier = 1.0;
                self.global_gold_multiplier = 1.0;
                self.global_exp_event_end_tick = 0;
                for (session_id, _) in &self.players {
                    send_system_message(&self.gate_ref, *session_id, &format!("全服{}已结束", event_name));
                }
                info!("Global event ended: {}", event_name);
            }
            // 随机世界事件触发（每 36000 ticks = 1 小时，20% 概率）
            if self.tick_count > 0 && self.tick_count % 36000 == 0 && self.global_exp_event_end_tick == 0 {
                let roll = fastrand::u32(1..=100);
                if roll <= 20 {
                    let event_roll = fastrand::u32(1..=100);
                    let (name, exp_mul, drop_mul, gold_mul, duration_min) = match event_roll {
                        1..=40 => ("双倍经验", 2.0, 1.0, 1.0, 10),
                        41..=70 => ("掉落狂欢", 1.0, 2.0, 1.0, 10),
                        71..=90 => ("金币雨", 1.0, 1.0, 2.0, 10),
                        _ => ("三重盛宴", 2.0, 2.0, 2.0, 5),
                    };
                    let duration_ticks = duration_min * 600;
                    self.global_exp_multiplier = exp_mul;
                    self.global_drop_multiplier = drop_mul;
                    self.global_gold_multiplier = gold_mul;
                    self.global_exp_event_end_tick = self.tick_count + duration_ticks;
                    self.global_event_name = Some(name.to_string());
                    broadcast_system_message(&self.gate_ref, &self.players,
                        &format!("【世界事件】{} 活动已启动！经验 x{} 掉落 x{} 金币 x{}，持续 {} 分钟！",
                            name, exp_mul, drop_mul, gold_mul, duration_min));
                    info!("Random world event started: {} (exp={} drop={} gold={} for {} min)",
                        name, exp_mul, drop_mul, gold_mul, duration_min);
                }
            }
            // 隐身过期检查：从 invisible_sessions 中移除已过期玩家并广播现身
            let invis_tag = std::mem::discriminant(&crate::combat::buff::BuffType::Invisibility);
            let mut to_reveal: Vec<(u64, crate::actors::player::PlayerState)> = Vec::new();
            for session_id in &self.invisible_sessions {
                if let Some(record) = self.players.get(session_id) {
                    if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                        let still_invisible = state.buffs.iter()
                            .any(|b| std::mem::discriminant(&b.buff_type) == invis_tag);
                        if !still_invisible {
                            to_reveal.push((*session_id, state));
                        }
                    }
                }
            }
            for (session_id, state) in to_reveal {
                self.invisible_sessions.remove(&session_id);
                self.reveal_player_to_others(session_id, &state).await;
                send_system_message(&self.gate_ref, session_id, "隐身效果已结束");
            }
        }
    }

    /// PK 值衰减 + 名字颜色广播（每 10 ticks）
    pub(crate) async fn tick_pk_decay(&mut self) {
        if self.tick_count % 10 == 0 {
            let mut colour_changes = Vec::new();
            for (session_id, record) in &self.players {
                let _ = record.actor_ref.ask(crate::actors::player::DecayPkPoints).await;
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    let new_colour = name_colour_for_pk(state.pk_points);
                    let old_colour = name_colour_for_pk(record.last_pk_points);
                    if new_colour != old_colour {
                        colour_changes.push((*session_id, state.object_id, new_colour, state.pk_points));
                    }
                }
            }
            for (session_id, object_id, new_colour, pk_points) in colour_changes {
                if let Some(record) = self.players.get_mut(&session_id) {
                    record.last_pk_points = pk_points;
                }
                let packet = build_object_colour_changed_packet(object_id, new_colour);
                for (sid, _) in &self.players {
                    let _ = self.gate_ref.ask(SendToClient {
                        session_id: *sid,
                        data: packet.clone(),
                    });
                }
            }
        }
    }

    /// 钓鱼收获判定（每 tick）
    pub(crate) async fn tick_fishing(&mut self) {
        let mut caught = Vec::new(); // session_id
        let mut stopped = Vec::new(); // session_id
        for (session_id, record) in &self.players {
            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                if !state.is_fishing { continue; }

                let counter = self.fishing_tick_counters.entry(*session_id).or_insert(0);
                *counter += 1;

                // 钓鱼需要 30~70 ticks（约 3~7 秒）才有收获
                let required = 30 + ((*session_id as u32 + *counter).wrapping_mul(1103515245).wrapping_add(12345) % 41);
                if *counter >= required {
                    // 收获判定
                    let roll = ((*session_id + self.tick_count) % 100) as u8;
                    if roll < 60 {
                        // 金币 10~50
                        let gold = 10 + ((*session_id + self.tick_count) % 41) as u64;
                        let _ = record.actor_ref.ask(crate::actors::player::AddGold { amount: gold }).await;
                        send_system_message(&self.gate_ref, *session_id, &format!("钓到了宝箱！获得 {} 金币", gold));
                    } else if roll < 80 {
                        // 随机物品：从已加载的物品中挑一个低阶物品
                        let item_index = Self::random_fishing_item_index(&self.item_infos, *session_id, self.tick_count);
                        let item = crate::actors::inventory::make_item(item_index, 1);
                        let added = record.actor_ref.ask(crate::actors::player::AddItemToInventory { item }).await.unwrap_or(false);
                        if added {
                            send_system_message(&self.gate_ref, *session_id, "钓到了一件物品！");
                        } else {
                            send_system_message(&self.gate_ref, *session_id, "钓到了物品，但背包已满！");
                        }
                    } else if roll < 95 {
                        // 经验 10~30
                        let xp = 10 + ((*session_id + self.tick_count) % 21) as i32;
                        let _ = record.actor_ref.ask(crate::actors::player::AddExperience { amount: self.apply_global_exp_multiplier(xp) }).await;
                        send_system_message(&self.gate_ref, *session_id, &format!("钓到了经验珠！获得 {} 经验", xp));
                    } else {
                        send_system_message(&self.gate_ref, *session_id, "鱼跑了...");
                    }

                    if state.fishing_autocast {
                        caught.push(*session_id);
                    } else {
                        stopped.push(*session_id);
                    }
                }
            }
        }
        for session_id in caught {
            self.fishing_tick_counters.insert(session_id, 0);
            // Send bite state then auto-recast waiting state
            let bite_packet = mir2_shared::packets::server::miscellaneous::FishingUpdate { fishing_progress: 2, fishing_success: true };
            let mut body = Vec::new();
            if let Ok(()) = mir2_shared::packets::Packet::write_body(&bite_packet, &mut body) {
                let _ = self.gate_ref.ask(SendToClient {
                    session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::FishingUpdate as i16, &body),
                });
            }
            // Then immediately send waiting state for autocast
            let wait_packet = mir2_shared::packets::server::miscellaneous::FishingUpdate { fishing_progress: 1, fishing_success: false };
            let mut body2 = Vec::new();
            if let Ok(()) = mir2_shared::packets::Packet::write_body(&wait_packet, &mut body2) {
                let _ = self.gate_ref.ask(SendToClient {
                    session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::FishingUpdate as i16, &body2),
                });
            }
        }
        for session_id in stopped {
            self.fishing_tick_counters.remove(&session_id);
            if let Some(record) = self.players.get(&session_id) {
                let _ = record.actor_ref.ask(crate::actors::player::SetFishing { is_fishing: false, autocast: false }).await;
            }
            // Send idle state
            let idle_packet = mir2_shared::packets::server::miscellaneous::FishingUpdate { fishing_progress: 0, fishing_success: false };
            let mut body = Vec::new();
            if let Ok(()) = mir2_shared::packets::Packet::write_body(&idle_packet, &mut body) {
                let _ = self.gate_ref.ask(SendToClient {
                    session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::FishingUpdate as i16, &body),
                });
            }
        }
    }

    /// 地面物品过期清理（每 50 ticks）
    pub(crate) async fn tick_ground_cleanup(&mut self) {
        if self.tick_count % 50 == 0 {
            const GROUND_ITEM_LIFETIME_TICKS: u64 = 600; // ~60 秒
            let expired: Vec<_> = self.ground_items.iter()
                .filter(|gi| self.tick_count >= gi.drop_tick + GROUND_ITEM_LIFETIME_TICKS)
                .map(|gi| (gi.object_id, gi.map_index))
                .collect();
            if !expired.is_empty() {
                self.ground_items.retain(|gi| self.tick_count < gi.drop_tick + GROUND_ITEM_LIFETIME_TICKS);
                for (oid, map_idx) in &expired {
                    let remove_packet = Self::build_object_remove_packet(*oid);
                    for (sid, rec) in &self.players {
                        if let Ok(Some(s)) = rec.actor_ref.ask(GetPlayerState).await {
                            if s.map_index == *map_idx {
                                let _ = self.gate_ref.ask(SendToClient {
                                    session_id: *sid,
                                    data: remove_packet.clone(),
                                });
                            }
                        }
                    }
                }
                debug!("Cleaned up {} expired ground items", expired.len());
            }
        }
    }

    /// 怪物重生处理（每 tick）
    pub(crate) async fn tick_respawn(&mut self) {
        let mut to_respawn = Vec::new();
        for (oid, (spawn, tick)) in &self.respawn_queue {
            if self.tick_count >= *tick {
                to_respawn.push((*oid, spawn.clone()));
            }
        }
        for (oid, spawn) in to_respawn {
            self.respawn_queue.remove(&oid);
            let new_oid = self.alloc_object_id();
            let packet = build_object_monster_packet(&spawn, new_oid, &spawn.name);
            for session_id in self.players.keys() {
                let _ = self.gate_ref.ask(SendToClient {
                    session_id: *session_id,
                    data: packet.clone(),
                });
            }
            let ai_profile = self.monster_infos
                .get(&spawn.monster_index)
                .map(MonsterAiProfile::from_info)
                .unwrap_or_else(|| MonsterAiProfile {
                    ai_type: MonsterAiType::Aggressive,
                    aggro_range: 10,
                    attack_range: 1,
                    attack_cooldown: 5,
                    move_interval: 2,
                    flee_threshold: 0.0,
                });
            // 精英判定：3% 概率
            let is_elite = fastrand::u8(1..=100) <= 3;
            let (name, hp, max_hp, min_dmg, max_dmg, xp) = if is_elite {
                (
                    format!("[精英] {}", spawn.name),
                    spawn.hp.saturating_mul(2),
                    spawn.hp.saturating_mul(2),
                    (spawn.min_dmg as f32 * 1.5) as i32,
                    (spawn.max_dmg as f32 * 1.5) as i32,
                    spawn.xp.saturating_mul(2),
                )
            } else {
                (spawn.name.clone(), spawn.hp, spawn.hp, spawn.min_dmg, spawn.max_dmg, spawn.xp)
            };
            self.monsters.insert(new_oid, MonsterState {
                object_id: new_oid,
                name: name.clone(),
                image: spawn.image,
                monster_index: spawn.monster_index,
                x: spawn.x,
                y: spawn.y,
                direction: spawn.direction,
                hp,
                max_hp,
                min_dmg,
                max_dmg,
                xp,
                spawn_x: spawn.x,
                spawn_y: spawn.y,
                map_index: spawn.map_index,
                next_attack_tick: 0,
                next_move_tick: 0,
                next_summon_tick: 0,
                ai_profile,
                ai_state: MonsterAiState::Idle,
                target_session: None,
                provoked: false,
                is_elite,
                is_boss: false,
                min_ac: 0,
                max_ac: 0,
                min_mac: 0,
                max_mac: 0,
                agility: 0,
                accuracy: 0,
                armour_rate: 1.0,
                damage_rate: 1.0,
                magic_resist: 0,
                critical_rate: 0,
                critical_damage: 0,
                luck: 0,
                reflect: 0,
                damage_reduction_percent: 0,
                poison_list: Vec::new(),
            undead: false,
                behavior: crate::actors::world::ai::make_behavior(&name),
            });
            if is_elite {
                let map_name = self.map_infos.get(&(spawn.map_index as i32)).map(|m| m.title.clone()).unwrap_or_else(|| "未知地图".to_string());
                broadcast_system_message(&self.gate_ref, &self.players,
                    &format!("一只 [精英]{} 出现在 {}！勇士们，前往讨伐！", spawn.name, map_name));
                debug!("Elite monster '{}' spawned as #{} at ({},{})", name, new_oid, spawn.x, spawn.y);
            } else {
                debug!("Monster '{}' respawned as #{}", spawn.name, new_oid);
            }
        }
    }

    /// 世界Boss超时消失（每 tick）
    pub(crate) async fn tick_boss_timeout(&mut self) {
        let mut boss_despawns = Vec::new();
        for (oid, despawn_tick) in &self.world_boss_queue {
            if should_despawn_boss(self.tick_count, *despawn_tick) {
                boss_despawns.push(*oid);
            }
        }
        for oid in boss_despawns {
            self.world_boss_queue.remove(&oid);
            if let Some(monster) = self.monsters.remove(&oid) {
                let packet = Self::build_object_remove_packet(oid);
                for session_id in self.players.keys() {
                    let _ = self.gate_ref.ask(SendToClient {
                        session_id: *session_id,
                        data: packet.clone(),
                    });
                }
                broadcast_system_message(&self.gate_ref, &self.players,
                    &format!("世界Boss {} 因无人挑战而消失了", monster.name));
                debug!("World boss '{}' (#{}) despawned (timeout)", monster.name, oid);
            }
        }
    }

    /// 任务超时检查（每 100 ticks）
    pub(crate) async fn tick_quest_timeout(&mut self) {
        if self.tick_count.is_multiple_of(100) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            for (session_id, record) in &self.players {
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    for quest in &state.quest_log.quests {
                        if quest.time_limit_seconds > 0
                            && matches!(quest.status, QuestStatus::InProgress | QuestStatus::Accepted)
                            && now.saturating_sub(quest.start_time) >= quest.time_limit_seconds as u64
                        {
                            let failed = record.actor_ref.ask(crate::actors::player::FailQuest {
                                quest_index: quest.quest_index,
                            }).await.unwrap_or(false);
                            if failed {
                                send_system_message(
                                    &self.gate_ref, *session_id,
                                    &format!("任务 '{}' 已超时失败", quest.title)
                                );
                                debug!("Quest expired: {} for session {}", quest.title, session_id);
                            }
                        }
                    }
                }
            }
        }
    }

    /// 宠物自动拾取（每 tick）
    pub(crate) async fn tick_pet_pickup(&mut self) {
        let mut pet_pickups: Vec<(usize, u64)> = Vec::new(); // (ground_item_index, session_id)
        for (session_id, record) in &self.players {
            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                let creature = match state.creature_log.active_creature {
                    Some(ref c) if c.enabled && !c.is_starving() => c,
                    _ => continue,
                };
                let pickup_mode = creature.pickup_mode;
                if pickup_mode == crate::actors::creature::PickupMode::None {
                    continue;
                }
                // 找附近匹配的物品（最多拾取 1 个/ tick）
                for (gi_idx, gi) in self.ground_items.iter().enumerate() {
                    let dist = (state.x - gi.x).abs() + (state.y - gi.y).abs();
                    if dist > 3 { continue; }
                    if gi.map_index != state.map_index { continue; }
                    // 绑定物品（dropper_session 不为空）不能拾取
                    if gi.dropper_session.is_some() && gi.dropper_session != Some(*session_id) { continue; }

                    let is_gold = gi.item.item_index == 0;
                    let should_pickup = match pickup_mode {
                        crate::actors::creature::PickupMode::GoldOnly => is_gold,
                        crate::actors::creature::PickupMode::GoldAndItem => true,
                        crate::actors::creature::PickupMode::All => true,
                        _ => false,
                    };
                    if should_pickup {
                        pet_pickups.push((gi_idx, *session_id));
                        break; // 每个玩家每 tick 最多拾取 1 个
                    }
                }
            }
        }

        // 应用拾取（从后往前删除，避免索引偏移）
        pet_pickups.sort_by(|a, b| b.0.cmp(&a.0));
        pet_pickups.dedup_by(|a, b| a.0 == b.0); // 同一物品只拾取一次

        for (gi_idx, session_id) in pet_pickups {
            if gi_idx >= self.ground_items.len() { continue; }
            let gi = self.ground_items.remove(gi_idx);

            // 广播移除
            let remove_packet = Self::build_object_remove_packet(gi.object_id);
            for sid in self.players.keys() {
                let _ = self.gate_ref.ask(SendToClient {
                    session_id: *sid,
                    data: remove_packet.clone(),
                });
            }

            if let Some(record) = self.players.get(&session_id) {
                if gi.item.item_index == 0 {
                    // 金币
                    let gold = gi.item.count as u64;
                    let _ = record.actor_ref.ask(crate::actors::player::AddGold { amount: gold }).await;
                    send_system_message(&self.gate_ref, session_id,
                        &format!("宠物帮你拾取了 {} 金币", gold));
                } else {
                    // 检查背包空间
                    let has_space = record.actor_ref.ask(crate::actors::player::HasItemSpace).await.unwrap_or(false);
                    if has_space {
                        let _ = record.actor_ref.ask(crate::actors::player::AddItemToInventory {
                            item: gi.item.clone(),
                        }).await;
                        send_system_message(
                            &self.gate_ref, session_id,
                            &format!("宠物帮你拾取了物品"));
                    } else {
                        // 背包已满，把物品掉回去
                        self.ground_items.push(gi);
                        send_system_message(&self.gate_ref, session_id,
                            "宠物发现物品但你的背包已满");
                    }
                }
            }
        }
    }

    /// NPC 商店自动补货（每小时）
    pub(crate) async fn tick_shop_restock(&mut self) {
        if self.tick_count.is_multiple_of(36000) {
            let mut restocked = 0usize;
            for goods_list in self.npc_goods.values_mut() {
                for good in goods_list.iter_mut() {
                    if !good.infinite_stock && good.stock < good.max_stock {
                        good.stock = good.max_stock;
                        restocked += 1;
                    }
                }
            }
            if restocked > 0 {
                info!("NPC shop restock: {} items restocked", restocked);
            }
        }
    }

    /// 精炼自动完成（每 100 ticks）
    pub(crate) async fn tick_refine_complete(&mut self) {
        if self.tick_count.is_multiple_of(100) {
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            for (session_id, record) in &self.players {
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    if let Some(ref item) = state.refine_log.active_refine {
                        if item.status == RefineStatus::Pending && current_time >= item.finish_time {
                            let mut log = state.refine_log.clone();
                            let success = log.finish();
                            let _ = record.actor_ref.ask(SetRefineLog { refine_log: log }).await;
                            if success {
                                send_system_message(&self.gate_ref, *session_id, "精炼完成！物品已提升");
                            } else {
                                send_system_message(&self.gate_ref, *session_id, "精炼失败，物品已损毁");
                            }
                            debug!("AutoRefine: {} result={}", state.name, success);
                        }
                    }
                }
            }
        }
    }

    /// HP/MP 回复 + 宠物饥饿 tick（每 100 ticks）
    pub(crate) async fn tick_regen_and_hunger(&mut self) {
        if self.tick_count.is_multiple_of(100) {
            debug!(
                "World tick #{} (online: {}, monsters: {})",
                self.tick_count, self.players.len(), self.monsters.len()
            );

            // 每 10 秒（100 ticks @ 100ms）回复 HP/MP
            for record in self.players.values() {
                // 宠物饥饿值
                let _ = record.actor_ref.ask(TickCreatureHunger { dt_seconds: 10 });

                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    let hp_regen = 5;
                    let mp_regen = 3;
                    let new_hp = (state.hp + hp_regen).min(state.max_hp);
                    let new_mp = (state.mp + mp_regen).min(state.max_mp);

                    if new_hp != state.hp || new_mp != state.mp {
                        // 发送 HealthChanged
                        let mut health_body = Vec::new();
                        health_body.extend_from_slice(&(new_hp as u32).to_le_bytes());
                        health_body.extend_from_slice(&(new_mp as u32).to_le_bytes());
                        let _ = self.gate_ref.ask(SendToClient {
                            session_id: state.session_id,
                            data: build_packet_bytes(
                                mir2_shared::enums::ServerPacketIds::HealthChanged as i16,
                                &health_body,
                            ),
                        });
                    }
                }
            }
        }
    }

    /// 昼夜循环（每 600 ticks）
    pub(crate) async fn tick_day_night(&mut self) {
        if self.tick_count.is_multiple_of(600) {
            let hour = chrono::Local::now().hour();
            let new_light = Self::light_for_hour(hour);
            if new_light != self.current_light {
                self.current_light = new_light;
                for session_id in self.players.keys() {
                    self.send_time_of_day(*session_id, new_light);
                }
                let light_name = match new_light {
                    mir2_shared::enums::LightSetting::Dawn => "黎明",
                    mir2_shared::enums::LightSetting::Day => "白天",
                    mir2_shared::enums::LightSetting::Evening => "黄昏",
                    mir2_shared::enums::LightSetting::Night => "夜晚",
                    _ => "正常",
                };
                info!("Time of day changed to {} (hour={})", light_name, hour);
            }
        }
    }

    /// 定期自动保存（每 300 ticks）
    pub(crate) async fn tick_auto_save(&mut self) {
        if self.tick_count % 300 == 0 && !self.players.is_empty() {
            let player_count = self.players.len();
            let mut saved = 0;
            for record in self.players.values() {
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    if let Err(e) = db::save_character(&self.db_pool, &state, &record.name).await {
                        warn!("Auto-save failed for player {}: {}", record.name, e);
                    } else {
                        saved += 1;
                    }
                }
            }
            info!("Auto-saved {} players to database ({} online)", saved, player_count);
        }
    }

    /// 拍卖过期清理（每 36000 ticks = 1小时）
    pub(crate) async fn tick_auction_expiry(&mut self) {
        if self.tick_count % 36000 == 0 {
            let now = chrono::Local::now().timestamp();
            let seven_days = 7 * 24 * 60 * 60;
            let mut expired = Vec::new();
            for (idx, auction) in self.auctions.iter().enumerate() {
                if !auction.sold && (now - auction.consignment_date) > seven_days {
                    expired.push(idx);
                }
            }
            for idx in expired.into_iter().rev() {
                let auction = self.auctions.remove(idx);
                let _ = db::delete_auction(&self.db_pool, auction.auction_id as i64).await;

                // Return item to seller
                let mut seller_online = false;
                for (_, record) in &self.players {
                    if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                        if state.name == auction.seller_name {
                            let added = record.actor_ref.ask(AddItemToInventory { item: auction.item.clone() }).await.unwrap_or(false);
                            if added {
                                send_system_message(&self.gate_ref, record.session_id, "寄售物品已过期，已退回背包");
                            } else {
                                send_system_message(&self.gate_ref, record.session_id, "寄售物品已过期，背包已满，物品已通过邮件退回");
                                send_item_via_mail(&self.db_pool, &auction.seller_name, auction.item.clone(), "寄售物品退回", "寄售物品已过期，背包已满");
                            }
                            seller_online = true;
                            break;
                        }
                    }
                }
                if !seller_online {
                    // Seller offline — send item via mail
                    send_item_via_mail(&self.db_pool, &auction.seller_name, auction.item.clone(), "寄售物品退回", "寄售物品已过期");
                }
                debug!("Auction {} expired and removed", auction.auction_id);
            }
        }
    }

    /// 租赁过期处理（每 3600 ticks = 6分钟检查一次）
    pub(crate) async fn tick_rental_expiry(&mut self) {
        if self.tick_count % 3600 == 0 {
            let now = chrono::Local::now().timestamp();
            let mut expired_renters: Vec<String> = Vec::new();

            for (renter_name, rentals) in &mut self.player_rentals {
                let mut still_valid: Vec<RentedItem> = Vec::new();
                for rental in rentals.drain(..) {
                    if rental.expiry_timestamp > now {
                        still_valid.push(rental);
                        continue;
                    }
                    // Rental expired - try to remove from renter and return to owner
                    let mut returned = false;
                    for (_, record) in &self.players {
                        if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                            if state.name == *renter_name {
                                // Try to remove item from renter
                                let removed = record.actor_ref.ask(RemoveItemFromInventory {
                                    unique_id: rental.item.unique_id,
                                }).await.ok().flatten();
                                if removed.is_some() {
                                    // Return to owner if online
                                    for (_, owner_record) in &self.players {
                                        if let Ok(Some(owner_state)) = owner_record.actor_ref.ask(GetPlayerState).await {
                                            if owner_state.name == rental.owner_name {
                                                let added = owner_record.actor_ref.ask(AddItemToInventory {
                                                    item: rental.item.clone(),
                                                }).await.unwrap_or(false);
                                                if added {
                                                    send_system_message(&self.gate_ref, owner_record.session_id,
                                                        &format!("租赁物品 {} 已到期收回", rental.item.item_index));
                                                }
                                                break;
                                            }
                                        }
                                    }
                                    send_system_message(&self.gate_ref, record.session_id,
                                        &format!("租赁物品 {} 已到期，已归还给 {}", rental.item.item_index, rental.owner_name));
                                    returned = true;
                                } else {
                                    send_system_message(&self.gate_ref, record.session_id,
                                        &format!("租赁物品 {} 已到期，但物品不在背包中", rental.item.item_index));
                                }
                                break;
                            }
                        }
                    }
                    if !returned {
                        // Renter offline or item not in inventory — return to owner via online or mail
                        let mut owner_online = false;
                        for (_, owner_record) in &self.players {
                            if let Ok(Some(owner_state)) = owner_record.actor_ref.ask(GetPlayerState).await {
                                if owner_state.name == rental.owner_name {
                                    let added = owner_record.actor_ref.ask(AddItemToInventory {
                                        item: rental.item.clone(),
                                    }).await.unwrap_or(false);
                                    if added {
                                        send_system_message(&self.gate_ref, owner_record.session_id,
                                            &format!("租赁物品 {} 已到期收回", rental.item.item_index));
                                    } else {
                                        send_system_message(&self.gate_ref, owner_record.session_id,
                                            &format!("租赁物品 {} 已到期，背包已满，已通过邮件退回", rental.item.item_index));
                                        send_item_via_mail(&self.db_pool, &rental.owner_name, rental.item.clone(),
                                            "租赁物品退回", &format!("租赁物品 {} 已到期", rental.item.item_index));
                                    }
                                    owner_online = true;
                                    break;
                                }
                            }
                        }
                        if !owner_online {
                            send_item_via_mail(&self.db_pool, &rental.owner_name, rental.item.clone(),
                                "租赁物品退回", &format!("租赁物品 {} 已到期", rental.item.item_index));
                        }
                    }
                    debug!("Rental expired: {} -> {} item={}", rental.owner_name, renter_name, rental.item.item_index);
                }
                if still_valid.is_empty() {
                    expired_renters.push(renter_name.clone());
                } else {
                    *rentals = still_valid;
                }
            }
            for name in expired_renters {
                self.player_rentals.remove(&name);
            }
        }
    }

    pub(crate) async fn tick_dragon(&mut self) {
        if let Some(ref mut dragon) = self.dragon_state {
            crate::actors::world::dragon::tick_dragon_delevel(
                dragon, self.tick_count, &self.gate_ref,
            ).await;
        }
    }

    pub(crate) async fn tick_conquest(&mut self) {
        for instance in &mut self.conquest_instances {
            let now = chrono::Local::now().naive_local();
            if instance.should_start_war(&now) {
                instance.start_war("攻击方");
                let msg = format!("攻城战开始了！目标：区域 #{}", instance.id);
                broadcast_system_message(&self.gate_ref, &self.players, &msg);
            }
            // Check if war should end
            if instance.state == conquest::WarState::InProgress {
                let elapsed = chrono::Utc::now().timestamp() - instance.war_start_time;
                if elapsed >= instance.war_duration_secs {
                    if let Some(winner) = instance.end_war() {
                        let msg = format!("攻城战结束！{} 取得了区域 #{} 的控制权！", winner, instance.id);
                        broadcast_system_message(&self.gate_ref, &self.players, &msg);
                    }
                }
            }
            // KingOfHill scoring: every 60 ticks (~6 seconds), award points to players in king zone
            if instance.state == conquest::WarState::InProgress
                && instance.game == conquest::ConquestGame::KingOfHill
                && self.tick_count % 60 == 0
            {
                for (_, record) in &self.players {
                    if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                        if instance.is_in_king_zone(state.x, state.y) {
                            if let Some(ref guild) = state.guild_name {
                                instance.add_score(guild, 1);
                            }
                        }
                    }
                }
            }
        }
    }

    pub(crate) async fn tick_robots(&mut self) {
        let now = chrono::Local::now().naive_local();
        let current_minute = now.minute();
        if self.robot_tasks.is_empty() || current_minute == self.robot_last_check_minute {
            return;
        }
        self.robot_last_check_minute = current_minute;
        let mut task_indices: Vec<usize> = vec![];
        for (i, task) in self.robot_tasks.iter().enumerate() {
            if task.should_fire(&now) {
                task_indices.push(i);
            }
        }
        for idx in &task_indices {
            let page = self.robot_tasks[*idx].page.clone();
            self.robot_tasks[*idx].mark_fired(&now);
            let msg = format!("[机器人] 定时事件触发: {}", page);
            broadcast_system_message(&self.gate_ref, &self.players, &msg);
        }
    }

    pub(crate) async fn tick_spells(&mut self) {
        use mir2_shared::enums::{Spell, PoisonType};
        use crate::actors::player::{GetPlayerState, Heal};
        use crate::combat::{attack, poison};

        let now = std::time::Instant::now();
        let mut expired_ids = Vec::new();
        // 收集需要结算的 spell tick：(caster_session, spell, x, y, tick_value, 命中怪物 ids)
        let mut spell_hits: Vec<(u64, Spell, i32, i32, i32, Vec<u32>)> = Vec::new();
        let mut heal_targets: Vec<u64> = Vec::new();
        let mut heal_amounts: Vec<i32> = Vec::new();

        // 第一阶段：遍历 spell_objects，更新 tick 时间，收集命中怪物 id
        for (obj_id, spell_obj) in &mut self.spell_objects {
            let elapsed = now.duration_since(spell_obj.created_at).as_millis() as u64;
            if spell_obj.is_expired(elapsed) {
                expired_ids.push(*obj_id);
                continue;
            }
            let since_last = now.duration_since(spell_obj.last_tick).as_millis() as u64;
            if since_last < spell_obj.tick_interval_ms {
                continue;
            }
            spell_obj.last_tick = now;

            match spell_obj.spell {
                Spell::FireWall | Spell::Blizzard | Spell::MeteorStrike | Spell::PoisonCloud => {
                    // 持久伤害法术：命中 spell 位置 ±1 的怪物（C# SpellObject.ProcessSpell 按单格）
                    let hit_ids: Vec<u32> = self.monsters.iter()
                        .filter(|(_, m)| {
                            let dist = (m.x - spell_obj.x).abs() + (m.y - spell_obj.y).abs();
                            dist <= 1 && m.hp > 0
                        })
                        .map(|(id, _)| *id)
                        .collect();
                    if !hit_ids.is_empty() {
                        spell_hits.push((
                            spell_obj.caster_session,
                            spell_obj.spell,
                            spell_obj.x,
                            spell_obj.y,
                            spell_obj.tick_value,
                            hit_ids,
                        ));
                    }
                }
                Spell::HealingCircle => {
                    if let Some(record) = self.players.get(&spell_obj.caster_session) {
                        if let Ok(Some(_cs)) = record.actor_ref.ask(GetPlayerState).await {
                            for (sid, other) in &self.players {
                                if let Ok(Some(s)) = other.actor_ref.ask(GetPlayerState).await {
                                    let dist = (s.x - spell_obj.x).abs() + (s.y - spell_obj.y).abs();
                                    if dist <= 2 && !heal_targets.contains(sid) {
                                        heal_targets.push(*sid);
                                        heal_amounts.push(spell_obj.tick_value.max(25));
                                    }
                                }
                            }
                        }
                    }
                }
                Spell::ExplosiveTrap => {
                    if !spell_obj.detonated {
                        spell_obj.detonated = true;
                        let hit_ids: Vec<u32> = self.monsters.iter()
                            .filter(|(_, m)| {
                                let dist = (m.x - spell_obj.x).abs() + (m.y - spell_obj.y).abs();
                                dist <= 1 && m.hp > 0
                            })
                            .map(|(id, _)| *id)
                            .collect();
                        if !hit_ids.is_empty() {
                            spell_hits.push((
                                spell_obj.caster_session, spell_obj.spell,
                                spell_obj.x, spell_obj.y, spell_obj.tick_value, hit_ids,
                            ));
                        }
                        expired_ids.push(*obj_id);
                    }
                }
                _ => {}
            }
        }

        // 第二阶段：对每个命中的怪物走战斗公式（MAC 防御 + 暴击 + 附加状态）
        // 按施法者分组缓存 CombatStats，减少 GetPlayerState 调用
        let mut caster_cache: std::collections::HashMap<u64, crate::combat::attack::CombatStats> = std::collections::HashMap::new();
        for (caster_session, spell, _sx, _sy, tick_value, hit_ids) in spell_hits {
            // 获取施法者 CombatStats
            let attacker_stats = if let Some(cs) = caster_cache.get(&caster_session) {
                *cs
            } else {
                let stats = match self.players.get(&caster_session) {
                    Some(r) => match r.actor_ref.ask(GetPlayerState).await {
                        Ok(Some(s)) if !s.is_dead => s.to_combat_stats(),
                        _ => continue, // 施法者离线/死亡，跳过本次 tick
                    },
                    None => continue,
                };
                caster_cache.insert(caster_session, stats);
                stats
            };

            for mid in hit_ids {
                if let Some(monster) = self.monsters.get_mut(&mid) {
                    let defender_stats = monster.to_combat_stats();
                    let level_offset = 10u16; // 怪物等级暂按 0（level_offset = min(10, attacker_level)）
                    let raw_damage = tick_value.max(1);
                    let r = attack::resolve_attack(
                        &attacker_stats, &defender_stats, raw_damage,
                        mir2_shared::enums::DefenceType::Mac, level_offset,
                    );
                    if r.is_hit && r.damage > 0 {
                        monster.hp = monster.hp.saturating_sub(r.damage);
                        monster.provoked = true;
                        monster.target_session = Some(caster_session);

                        // 各法术附加状态（对齐 C# SpellObject.ProcessSpell）
                        match spell {
                            // Blizzard：1/8 概率 Slow（C# SpellObject.cs:175）
                            Spell::Blizzard => {
                                if fastrand::i32(0..8) == 0 {
                                    let dur = (5 + fastrand::i32(0..attacker_stats.freezing.max(1))) as u32;
                                    poison::apply_poison(&mut monster.poison_list,
                                        poison::Poison::new(PoisonType::SLOW, dur, 0, 2000));
                                }
                            }
                            // PoisonCloud：绿毒（C# SpellObject.cs:157，道术但已在持久列表）
                            Spell::PoisonCloud => {
                                let sc = attacker_stats.max_atk; // 暂用 atk 近似 SC（道术字段待补）
                                let poison_value = (sc / 2).min(10);
                                poison::apply_poison(&mut monster.poison_list,
                                    poison::Poison::new(PoisonType::GREEN, 12, poison_value, 1000));
                            }
                            // FireWall / MeteorStrike：纯伤害无附加
                            _ => {}
                        }
                        // 战斗触发的 Poison（攻击者 freezing/poison_attack）
                        for p in &r.applied_poisons {
                            poison::apply_poison(&mut monster.poison_list, *p);
                        }
                    }
                }
            }
        }

        for (sid, amount) in heal_targets.iter().zip(heal_amounts.iter()) {
            if let Some(record) = self.players.get(sid) {
                let _ = record.actor_ref.ask(Heal { amount: *amount }).await;
            }
        }
        for id in &expired_ids {
            self.spell_objects.remove(id);
        }
    }

    /// 弹道法术延迟结算（对齐 C# HumanObject.CompleteMagic）
    ///
    /// 每 tick 检查 pending_spell_completions 中到期的项，按 spell 分支结算：
    /// - FireBall/GreatFireBall/ThunderBolt：单目标 MAC 伤害（ThunderBolt 亡灵 +50%）
    /// - FrostCrunch：MAC 伤害 + 概率 Slow/Frozen
    /// - Vampirism：MAC 伤害 + 吸血
    pub(crate) async fn tick_spell_completions(&mut self) {
        use mir2_shared::enums::{DefenceType, Spell, PoisonType};
        use crate::combat::{attack, poison};

        if self.pending_spell_completions.is_empty() {
            return;
        }

        // 取出到期的项
        let now = self.tick_count;
        let mut ready: Vec<PendingSpellCompletion> = Vec::new();
        self.pending_spell_completions.retain(|p| {
            if p.fire_at_tick <= now {
                ready.push(p.clone());
                false
            } else {
                true
            }
        });

        if ready.is_empty() {
            return;
        }

        // 按施法者分组，减少 GetPlayerState 调用
        for pending in ready {
            // 获取施法者状态
            let record = match self.players.get(&pending.session_id) {
                Some(r) => r.clone(),
                None => continue,
            };
            let caster_state = match record.actor_ref.ask(GetPlayerState).await {
                Ok(Some(s)) => s,
                _ => continue,
            };
            if caster_state.is_dead {
                continue;
            }
            let attacker_stats = caster_state.to_combat_stats();
            let level_offset = caster_state.level.min(10) as u16;
            let spell_enum = Spell::try_from(pending.spell).unwrap_or(Spell::None);

            // 弹道类法术目标可能是怪物或玩家
            // 先查怪物（按 object_id），再查玩家
            // C# 用 InRange(target.CurrentLocation, targetLocation, 2) 防移动 miss

            match spell_enum {
                Spell::FireBall | Spell::GreatFireBall | Spell::ThunderBolt | Spell::FrostCrunch
                | Spell::Vampirism
                // 弓箭手弹道物理系（命中后按 AC 防御结算，BindingShot/NapalmShot 附加效果）
                | Spell::StraightShot | Spell::DoubleShot
                | Spell::BindingShot | Spell::NapalmShot => {
                    Self::complete_projectile_spell(
                        self, pending, &caster_state, &attacker_stats, level_offset, spell_enum,
                    ).await;
                }
                _ => {
                    debug!("tick_spell_completions: unhandled spell {:?}", spell_enum);
                }
            }
        }

        // 处理 Vampirism 吸血回血（循环外统一发，避免借用冲突）
        let heals = std::mem::take(&mut self.vamp_heals);
        for (session_id, amount) in heals {
            if let Some(record) = self.players.get(&session_id) {
                let _ = record.actor_ref.ask(crate::actors::player::Heal { amount }).await;
            }
        }
    }

    /// 弹道法术结算（单目标伤害 + 各法术附加效果）
    ///
    /// 防御类型：法师弹道（FireBall/ThunderBolt/...）用 MAC；弓箭手弹道
    /// （StraightShot/DoubleShot/BindingShot/NapalmShot）用 AC（物理）。
    async fn complete_projectile_spell(
        &mut self,
        pending: PendingSpellCompletion,
        caster_state: &crate::actors::player::PlayerState,
        attacker_stats: &crate::combat::attack::CombatStats,
        level_offset: u16,
        spell: mir2_shared::enums::Spell,
    ) {
        use mir2_shared::enums::{DefenceType, Spell, PoisonType};
        use crate::combat::{attack, poison};

        let target_id = pending.target_id;
        let raw_damage = pending.damage;

        // 弓箭手弹道走 AC 防御（物理），法师弹道走 MAC（魔法）
        let is_archer = matches!(spell,
            Spell::StraightShot | Spell::DoubleShot | Spell::BindingShot | Spell::NapalmShot);
        let defence = if is_archer { DefenceType::Ac } else { DefenceType::Mac };

        // 查找目标怪物
        let monster_hit = {
            let monster = self.monsters.iter().find(|(_, m)| m.object_id == target_id);
            if let Some((_, m)) = monster {
                // 防移动 miss：目标当前位置 vs 弹道快照位置，InRange(2)
                let dist = (m.x - pending.target_x).abs() + (m.y - pending.target_y).abs();
                if dist > 2 {
                    debug!("Projectile spell {:?} missed target {} (moved {} tiles)", spell, target_id, dist);
                    return;
                }
                // ThunderBolt 亡灵 +50%（C# HumanObject.cs:4126）
                // 注意：Rust MonsterState 暂无 undead 标记字段，跳过该加成（TODO）
                Some((m.x, m.y, m.to_combat_stats()))
            } else {
                None
            }
        };

        if let Some((mx, my, defender_stats)) = monster_hit {
            // 法术特化伤害
            let final_damage = match spell {
                // ThunderBolt 对亡灵 +50%（C# HumanObject.cs:4126）
                Spell::ThunderBolt => {
                    if let Some(m) = self.monsters.get(&target_id) {
                        if m.undead { (raw_damage as f32 * 1.5) as i32 } else { raw_damage }
                    } else { raw_damage }
                }
                _ => raw_damage,
            };

            let result = attack::resolve_attack(
                attacker_stats, &defender_stats, final_damage,
                defence, level_offset,
            );

            if result.is_hit && result.damage > 0 {
                if let Some(monster) = self.monsters.get_mut(&target_id) {
                    monster.hp = monster.hp.saturating_sub(result.damage);
                    monster.provoked = true;
                    monster.target_session = Some(pending.session_id);

                    // FrostCrunch：概率 Slow/Frozen（C# HumanObject.cs:5962）
                    if spell == Spell::FrostCrunch {
                        let magic_level = pending.spell_level;
                        // Slow：Random(100) <= magic.Level（玩家目标）或 Random(20) <= level（怪物）
                        if fastrand::i32(0..20) <= magic_level as i32 {
                            let duration = (5 + fastrand::i32(0..5)) as u32;
                            poison::apply_poison(&mut monster.poison_list,
                                poison::Poison::new(PoisonType::SLOW, duration, 0, 1000));
                        }
                        // Frozen：Random(40) <= level
                        if fastrand::i32(0..40) <= magic_level as i32 {
                            let duration = (5 + fastrand::i32(0..caster_state.freezing.max(1))) as u32;
                            poison::apply_poison(&mut monster.poison_list,
                                poison::Poison::new(PoisonType::FROZEN, duration, 0, 1000));
                        }
                    }

                    // BindingShot：命中后施加 Paralysis（定身 3s）
                    if spell == Spell::BindingShot {
                        poison::apply_poison(&mut monster.poison_list,
                            poison::Poison::new(PoisonType::PARALYSIS, 3, 0, 1000));
                    }

                    // Vampirism：吸血 = 实伤 × (level+1) × 0.25（C# HumanObject.cs:6011）
                    if spell == Spell::Vampirism {
                        let vamp = (result.damage as f32 * (pending.spell_level as f32 + 1.0) * 0.25) as i32;
                        if vamp > 0 {
                            // 收集回血请求，循环外统一发（避免借用冲突）
                            self.vamp_heals.push((pending.session_id, vamp));
                        }
                    }

                    // 施加战斗触发的 Poison（冰冻攻击/毒物攻击，来自攻击者 Stats）
                    for p in &result.applied_poisons {
                        poison::apply_poison(&mut monster.poison_list, *p);
                    }

                    debug!("Projectile {:?} hit monster {} for {} dmg (crit={})",
                        spell, target_id, result.damage, result.is_critical);
                }
            } else {
                debug!("Projectile {:?} missed/blocked target {}", spell, target_id);
            }

            // NapalmShot：命中后 3×3 AOE（爆炸溅射，排除已被直击的主目标）
            if spell == Spell::NapalmShot {
                let splash_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(id, m)| {
                        **id != target_id
                            && (m.x - mx).abs() <= 1
                            && (m.y - my).abs() <= 1
                            && m.hp > 0
                    })
                    .map(|(id, _)| *id)
                    .collect();
                for sid in splash_ids {
                    if let Some(monster) = self.monsters.get_mut(&sid) {
                        let ds = monster.to_combat_stats();
                        let r = attack::resolve_attack(
                            attacker_stats, &ds, raw_damage, DefenceType::Ac, level_offset,
                        );
                        if r.is_hit && r.damage > 0 {
                            monster.hp = monster.hp.saturating_sub(r.damage);
                            monster.provoked = true;
                            monster.target_session = Some(pending.session_id);
                            for p in &r.applied_poisons {
                                poison::apply_poison(&mut monster.poison_list, *p);
                            }
                        }
                    }
                }
                debug!("NapalmShot exploded at ({},{}) 3x3 splash", mx, my);
            }
            return;
        }

        // 目标不是怪物，查玩家（PvP 弹道，如 SoulFireBall 打玩家）
        for (other_session, other_record) in &self.players {
            if let Ok(Some(other_state)) = other_record.actor_ref.ask(GetPlayerState).await {
                if other_state.object_id != target_id {
                    continue;
                }
                let dist = (other_state.x - pending.target_x).abs() + (other_state.y - pending.target_y).abs();
                if dist > 2 {
                    continue;
                }
                let defender_stats = other_state.to_combat_stats();
                let result = attack::resolve_attack(
                    attacker_stats, &defender_stats, raw_damage,
                    defence, level_offset,
                );
                if result.is_hit && result.damage > 0 {
                    let actor_ref = other_record.actor_ref.clone();
                    let damage = result.damage;
                    let _ = actor_ref.ask(TakeDamage {
                        attacker_id: caster_state.object_id,
                        attacker_session: pending.session_id,
                        damage,
                    }).await;
                    debug!("Projectile {:?} hit player {} for {} dmg", spell, target_id, damage);
                }
                break;
            }
        }
    }
}

// ============================================================
// Handler 实现
// ============================================================

impl Message<Tick> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: Tick,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.tick_count += 1;

        // --- 怪物 AI ---
        if !self.monsters.is_empty() && !self.players.is_empty() {
            // 收集所有玩家位置（避免在循环中借用 self）
            // 预收集玩家位置 + PK 值（用于 Guard AI 红名优先）
            let player_positions: Vec<(u64, i32, i32, u32, i32)> = {
                let mut results = Vec::new();
                let invis_tag = std::mem::discriminant(&crate::combat::buff::BuffType::Invisibility);
                for (session_id, record) in &self.players {
                    if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                        if !state.is_dead {
                            // 隐身玩家不会被怪物检测到
                            let is_invisible = state.buffs.iter()
                                .any(|b| std::mem::discriminant(&b.buff_type) == invis_tag);
                            if is_invisible { continue; }
                            let in_safe = self.maps.get(&state.map_index)
                                .map(|m| m.is_safe_zone(state.x, state.y))
                                .unwrap_or(false);
                            if !in_safe {
                                results.push((*session_id, state.x, state.y, state.object_id, state.pk_points));
                            }
                        }
                    }
                }
                results
            };

            // 对每个怪物执行 AI
            let mut dead_monsters = Vec::new();
            let mut moved_monsters = Vec::new();
            let mut moved_targets: HashSet<(i32, i32)> = HashSet::new();
            let mut death_drops: Vec<(u64, i32, i32, u16)> = Vec::new();
            let mut broken_armor: Vec<(u64, EquipmentSlot)> = Vec::new();
            let mut dismount_sessions: Vec<u64> = Vec::new();
            // 预收集怪物当前位置（用于碰撞检测）
            let monster_positions: HashSet<(i32, i32)> = self.monsters.values().map(|m| (m.x, m.y)).collect();
            // 预收集怪物快照（用于 Healer AI 寻找受伤盟友）
            let monster_snapshot: Vec<(u32, i32, i32, i32, i32, u16, i32, String, u16, u8)> = self.monsters.values()
                .map(|m| (m.object_id, m.x, m.y, m.hp, m.max_hp, m.map_index, m.monster_index, m.name.clone(), m.image, m.direction))
                .collect();
            // Healer 治疗动作和 Summoner 召唤动作（在循环后应用）
            let mut heal_actions: Vec<(u32, i32)> = Vec::new();
            let mut summon_spawns: Vec<MonsterSpawn> = Vec::new();
            // Boss AI 输出队列（在循环后应用）
            let mut boss_moves: Vec<(u32, i32, i32, u8)> = Vec::new();
            let mut boss_attacks: Vec<ai::AttackAction> = Vec::new();
            let mut boss_spell_fields: Vec<ai::SpellFieldSpawn> = Vec::new();
            let mut boss_summons: Vec<ai::BossSummon> = Vec::new();
            let mut boss_heals: Vec<(u32, i32)> = Vec::new();
            let mut boss_poisons: Vec<ai::PoisonPlayer> = Vec::new();

            for (oid, monster) in &mut self.monsters {
                // ===== Boss AI 分发 =====
                // 已注册 Boss 走 behavior.process_tick，普通怪走原有内联逻辑
                if ai::is_registered_boss(&monster.name) {
                    let monster_oid = monster.object_id;
                    let monster_index = monster.monster_index;
                    let monster_map = monster.map_index;
                    let monster_name = monster.name.clone();
                    let player_snaps: Vec<ai::PlayerSnap> = player_positions.iter()
                        .map(|(s, x, y, _, _)| ai::PlayerSnap {
                            session_id: *s, x: *x, y: *y, hp: 0, map_index: monster_map, object_id: 0,
                        }).collect();
                    let monster_snaps: Vec<ai::MonsterSnap> = Vec::new();
                    let mut ctx = ai::AiCtx {
                        tick_count: self.tick_count,
                        monster_oid, monster_index,
                        players: &player_snaps,
                        monsters: &monster_snaps,
                        out_moves: &mut boss_moves,
                        out_attacks: &mut boss_attacks,
                        out_spell_fields: &mut boss_spell_fields,
                        out_summons: &mut boss_summons,
                        out_heals: &mut boss_heals,
                        out_poisons: &mut boss_poisons,
                    };
                    // 临时取出 behavior 避免 &mut monster + &mut behavior 双重借用
                    let mut behavior = std::mem::replace(
                        &mut monster.behavior,
                        Box::new(crate::actors::world::ai::DefaultBehavior::new()),
                    );
                    behavior.process_tick(monster, &mut ctx);
                    monster.behavior = behavior;
                    debug!("Boss '{}' AI tick processed", monster_name);
                    continue;
                }
                let profile = &monster.ai_profile;

                // 找最近玩家（在视野范围内）
                // Guard AI：优先攻击红名玩家（PK 值 > 0）
                let mut nearest: Option<(u64, i32, i32, i32)> = None;
                if profile.ai_type == MonsterAiType::Guard {
                    // 先找范围内的红名玩家
                    let mut red_nearest: Option<(u64, i32, i32, i32)> = None;
                    for (session, px, py, _, pk) in &player_positions {
                        let dist = (monster.x - px).abs() + (monster.y - py).abs();
                        if dist <= profile.aggro_range && *pk > 0 {
                            if red_nearest.is_none_or(|n| dist < n.3) {
                                red_nearest = Some((*session, *px, *py, dist));
                            }
                        }
                    }
                    if red_nearest.is_some() {
                        nearest = red_nearest;
                    } else {
                        for (session, px, py, _, _) in &player_positions {
                            let dist = (monster.x - px).abs() + (monster.y - py).abs();
                            if dist <= profile.aggro_range {
                                if nearest.is_none_or(|n| dist < n.3) {
                                    nearest = Some((*session, *px, *py, dist));
                                }
                            }
                        }
                    }
                } else {
                    for (session, px, py, _, _) in &player_positions {
                        let dist = (monster.x - px).abs() + (monster.y - py).abs();
                        if dist <= profile.aggro_range {
                            if nearest.is_none_or(|n| dist < n.3) {
                                nearest = Some((*session, *px, *py, dist));
                            }
                        }
                    }
                }

                // 更新目标
                if let Some((sess, _, _, _)) = nearest {
                    monster.target_session = Some(sess);
                } else {
                    monster.target_session = None;
                }

                // 低血量逃跑判定（Coward）
                let hp_pct = monster.hp as f32 / monster.max_hp as f32;
                let is_fleeing = profile.ai_type == MonsterAiType::Coward && hp_pct < profile.flee_threshold;

                // 是否在攻击冷却中
                let can_attack = self.tick_count >= monster.next_attack_tick;
                // 是否可以移动（移动间隔）
                let can_move = self.tick_count >= monster.next_move_tick;

                // Passive 怪物：未激怒时不主动攻击
                let should_chase = match profile.ai_type {
                    MonsterAiType::Passive => monster.provoked,
                    MonsterAiType::Guard => nearest.is_some_and(|(_, _, _, d)| d <= profile.aggro_range) && dist_to_spawn(monster) <= profile.aggro_range * 2,
                    _ => nearest.is_some(),
                };

                if let Some((target_session, px, py, dist)) = nearest {
                    if is_fleeing && can_move {
                        // 逃跑：远离目标
                        let (nx, ny, dir) = monster.step_away(px, py);
                        if self.maps.get(&monster.map_index).map(|m| m.is_walkable(nx, ny)).unwrap_or(true)
                            && !monster_positions.contains(&(nx, ny))
                            && moved_targets.insert((nx, ny))
                        {
                            moved_monsters.push((*oid, nx, ny, dir));
                        }
                        monster.next_move_tick = self.tick_count + profile.move_interval;
                        monster.ai_state = MonsterAiState::Flee;
                    } else if dist <= profile.attack_range && can_attack {
                        // Healer AI：优先治疗附近受伤的怪物
                        let mut did_heal = false;
                        if profile.ai_type == MonsterAiType::Healer {
                            let mut best_target: Option<(u32, i32)> = None; // (oid, deficit)
                            for (snap_oid, sx, sy, shp, smax, smap, _, _, _, _) in &monster_snapshot {
                                if *snap_oid == *oid { continue; }
                                if *smap != monster.map_index { continue; }
                                let dist_ally = (monster.x - sx).abs() + (monster.y - sy).abs();
                                if dist_ally <= profile.aggro_range && *shp < *smax {
                                    let deficit = *smax - *shp;
                                    if best_target.is_none_or(|(_, d)| deficit > d) {
                                        best_target = Some((*snap_oid, deficit));
                                    }
                                }
                            }
                            if let Some((target_oid, _)) = best_target {
                                let heal_amount = (monster.max_hp / 4).max(10);
                                heal_actions.push((target_oid, heal_amount));
                                monster.next_attack_tick = self.tick_count + profile.attack_cooldown;
                                monster.ai_state = MonsterAiState::Attack;
                                did_heal = true;
                                debug!("Monster '{}' (#{}) heals ally #{} for {} HP", monster.name, *oid, target_oid, heal_amount);
                                // 广播治疗法术效果
                                let mut heal_body = Vec::new();
                                heal_body.extend_from_slice(&monster.object_id.to_le_bytes());
                                heal_body.extend_from_slice(&(monster.x as u32).to_le_bytes());
                                heal_body.extend_from_slice(&(monster.y as u32).to_le_bytes());
                                heal_body.push(monster.direction);
                                heal_body.push(SPELL_HEALING);
                                heal_body.extend_from_slice(&0u16.to_le_bytes());
                                heal_body.push(0u8);
                                let heal_packet = build_packet_bytes(
                                    mir2_shared::enums::ServerPacketIds::ObjectAttack as i16, &heal_body);
                                for sid in self.players.keys() {
                                    let _ = self.gate_ref.ask(SendToClient {
                                        session_id: *sid,
                                        data: heal_packet.clone(),
                                    });
                                }
                            }
                        }
                        // Summoner AI：低血量时召唤援军
                        let mut did_summon = false;
                        if profile.ai_type == MonsterAiType::Summoner && !did_heal {
                            let hp_pct = monster.hp as f32 / monster.max_hp as f32;
                            if hp_pct < 0.5 && self.tick_count >= monster.next_summon_tick {
                                // 找附近可行走的位置
                                let offsets: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
                                let mut spawn_count = 0;
                                for (dx, dy) in offsets {
                                    if spawn_count >= 2 { break; }
                                    let sx = monster.x + dx;
                                    let sy = monster.y + dy;
                                    if self.maps.get(&monster.map_index).map(|m| m.is_walkable(sx, sy)).unwrap_or(false)
                                        && !monster_positions.contains(&(sx, sy))
                                    {
                                        summon_spawns.push(MonsterSpawn {
                                            name: format!("{}的召唤物", monster.name),
                                            image: monster.image,
                                            monster_index: monster.monster_index,
                                            x: sx,
                                            y: sy,
                                            direction: monster.direction,
                                            hp: (monster.max_hp / 2).max(1),
                                            min_dmg: (monster.min_dmg / 2).max(1),
                                            max_dmg: (monster.max_dmg / 2).max(1),
                                            xp: (monster.xp / 2).max(1),
                                            map_index: monster.map_index,
                                        });
                                        spawn_count += 1;
                                    }
                                }
                                if spawn_count > 0 {
                                    monster.next_summon_tick = self.tick_count + 100; // 10秒冷却
                                    monster.next_attack_tick = self.tick_count + profile.attack_cooldown;
                                    monster.ai_state = MonsterAiState::Attack;
                                    did_summon = true;
                                    debug!("Monster '{}' (#{}) summons {} adds", monster.name, *oid, spawn_count);
                                }
                            }
                        }
                        if did_heal || did_summon {
                            // 已执行特殊动作，跳过普通攻击
                        } else {
                            // 攻击
                            let dmg_range = (monster.max_dmg - monster.min_dmg).max(1);
                            let damage = ((self.tick_count.wrapping_add(*oid as u64).wrapping_mul(7)) as i32 % dmg_range)
                                + monster.min_dmg;
                            debug!("Monster '{}' (#{}) attacks Player {} for {} dmg [AI={:?}]", monster.name, *oid, target_session, damage, profile.ai_type);
                            monster.next_attack_tick = self.tick_count + profile.attack_cooldown;
                            monster.ai_state = MonsterAiState::Attack;

                        let is_ranged = matches!(profile.ai_type, MonsterAiType::Ranged | MonsterAiType::Mage);
                        let spell_id = match profile.ai_type {
                            MonsterAiType::Mage => SPELL_FIREBALL,
                            MonsterAiType::Ranged => 1u8,
                            _ => 0u8,
                        };

                        // ObjectAttack 广播
                        let mut attack_body = Vec::new();
                        attack_body.extend_from_slice(&monster.object_id.to_le_bytes());
                        attack_body.extend_from_slice(&(monster.x as u32).to_le_bytes());
                        attack_body.extend_from_slice(&(monster.y as u32).to_le_bytes());
                        attack_body.push(monster.direction);
                        attack_body.push(spell_id);
                        attack_body.extend_from_slice(&0u16.to_le_bytes());
                        attack_body.push(0u8);
                        let attack_packet = build_packet_bytes(
                            mir2_shared::enums::ServerPacketIds::ObjectAttack as i16, &attack_body);
                        if is_ranged {
                            // 远程/法术攻击广播给所有玩家（弹道动画）
                            for sid in self.players.keys() {
                                let _ = self.gate_ref.ask(SendToClient {
                                    session_id: *sid,
                                    data: attack_packet.clone(),
                                });
                            }
                        } else {
                            let _ = self.gate_ref.ask(SendToClient {
                                session_id: target_session,
                                data: attack_packet,
                            });
                        }
                        // 安全区保护：目标在安全区内则不受怪物伤害
                        let target_in_safe = self.maps.get(&monster.map_index)
                            .map(|m| m.is_safe_zone(px, py))
                            .unwrap_or(false);

                        if !target_in_safe {
                            // 伤害
                            if let Some(record) = self.players.get(&target_session) {
                                let died = record.actor_ref.ask(TakeDamage {
                                    attacker_id: monster.object_id,
                                    attacker_session: target_session,
                                    damage,
                                }).await.unwrap_or(false);

                                // 被攻击时自动下坐骑
                                if !died {
                                    dismount_sessions.push(target_session);
                                }

                                // 装备耐久损耗（存活时）
                                if !died {
                                    let armor_slots = [
                                        EquipmentSlot::Armour,
                                        EquipmentSlot::Helmet,
                                        EquipmentSlot::BraceletL,
                                        EquipmentSlot::BraceletR,
                                        EquipmentSlot::RingL,
                                        EquipmentSlot::RingR,
                                        EquipmentSlot::Shoes,
                                        EquipmentSlot::Necklace,
                                    ];
                                    let slot = armor_slots[fastrand::usize(0..armor_slots.len())];
                                    let broke = record.actor_ref.ask(crate::actors::player::DamageEquipment {
                                        slot,
                                        amount: 1,
                                    }).await.unwrap_or(false);
                                    if broke {
                                        debug!("Player session={} {:?} broke from monster damage!", target_session, slot);
                                        // 延迟到怪物循环结束后广播（避免借用冲突）
                                        broken_armor.push((target_session, slot));
                                    }
                                }

                                if died {
                                    if let Ok(Some(victim)) = record.actor_ref.ask(GetPlayerState).await {
                                        let died_packet = Self::build_object_died_packet(
                                            victim.object_id, victim.x, victim.y, victim.direction);
                                        for (sid, _) in &self.players {
                                            let _ = self.gate_ref.ask(SendToClient {
                                                session_id: *sid,
                                                data: died_packet.clone(),
                                            });
                                        }
                                        death_drops.push((target_session, victim.x, victim.y, victim.map_index));

                                        // 死亡经验惩罚：损失 2% 当前等级所需经验
                                        let penalty = (victim.max_experience / 50).max(1) as i32;
                                        let deducted = record.actor_ref.ask(crate::actors::player::DeductExperience {
                                            amount: penalty,
                                        }).await.unwrap_or(0);
                                        if deducted > 0 {
                                            send_system_message(
                                                &self.gate_ref, target_session,
                                                &format!("你损失了 {} 经验值", deducted)
                                            );
                                        }
                                    }
                                }
                            }
                        } else {
                            debug!("Monster '{}' attack on {} blocked: target in safe zone", monster.name, target_session);
                        }
                        } // close else (normal attack)
                    } else if should_chase && dist > profile.attack_range && can_move {
                        // 追击
                        let (nx, ny, dir) = monster.step_toward(px, py);
                        if self.maps.get(&monster.map_index).map(|m| m.is_walkable(nx, ny)).unwrap_or(true)
                            && !monster_positions.contains(&(nx, ny))
                            && moved_targets.insert((nx, ny))
                        {
                            moved_monsters.push((*oid, nx, ny, dir));
                        }
                        monster.next_move_tick = self.tick_count + profile.move_interval;
                        monster.ai_state = MonsterAiState::Chase;
                    }
                } else if can_move && dist_to_spawn(monster) > 2 {
                    // 无目标 → 回出生点
                    let (nx, ny, dir) = monster.step_toward(monster.spawn_x, monster.spawn_y);
                    if self.maps.get(&monster.map_index).map(|m| m.is_walkable(nx, ny)).unwrap_or(true)
                        && !monster_positions.contains(&(nx, ny))
                        && moved_targets.insert((nx, ny))
                    {
                        moved_monsters.push((*oid, nx, ny, dir));
                    }
                    monster.next_move_tick = self.tick_count + profile.move_interval;
                    monster.ai_state = MonsterAiState::Return;
                } else {
                    monster.ai_state = MonsterAiState::Idle;
                }

                // 检查死亡
                if monster.hp <= 0 {
                    dead_monsters.push(*oid);
                }
            }

            // 应用 Healer 治疗（在循环外，避免借用冲突）
            for (target_oid, heal_amount) in &heal_actions {
                if let Some(target) = self.monsters.get_mut(target_oid) {
                    target.hp = (target.hp + *heal_amount).min(target.max_hp);
                }
            }

            // 应用 Summoner 召唤（在循环外创建新怪物）
            for spawn in &summon_spawns {
                let new_oid = self.alloc_object_id();
                let packet = build_object_monster_packet(spawn, new_oid, &spawn.name);
                for session_id in self.players.keys() {
                    let _ = self.gate_ref.ask(SendToClient {
                        session_id: *session_id,
                        data: packet.clone(),
                    });
                }
                let ai_profile = self.monster_infos
                    .get(&spawn.monster_index)
                    .map(MonsterAiProfile::from_info)
                    .unwrap_or_else(|| MonsterAiProfile {
                        ai_type: MonsterAiType::Aggressive,
                        aggro_range: 10,
                        attack_range: 1,
                        attack_cooldown: 5,
                        move_interval: 2,
                        flee_threshold: 0.0,
                    });
                self.monsters.insert(new_oid, MonsterState {
                    object_id: new_oid,
                    name: spawn.name.clone(),
                    image: spawn.image,
                    monster_index: spawn.monster_index,
                    x: spawn.x,
                    y: spawn.y,
                    direction: spawn.direction,
                    hp: spawn.hp,
                    max_hp: spawn.hp,
                    min_dmg: spawn.min_dmg,
                    max_dmg: spawn.max_dmg,
                    xp: spawn.xp,
                    spawn_x: spawn.x,
                    spawn_y: spawn.y,
                    map_index: spawn.map_index,
                    next_attack_tick: 0,
                    next_move_tick: 0,
                    next_summon_tick: 0,
                    ai_profile,
                    ai_state: MonsterAiState::Idle,
                    target_session: None,
                    provoked: false,
                    is_elite: false,
                    is_boss: false,
                    min_ac: 0,
                    max_ac: 0,
                    min_mac: 0,
                    max_mac: 0,
                    agility: 0,
                    accuracy: 0,
                    armour_rate: 1.0,
                    damage_rate: 1.0,
                    magic_resist: 0,
                    critical_rate: 0,
                    critical_damage: 0,
                    luck: 0,
                    reflect: 0,
                    damage_reduction_percent: 0,
                    poison_list: Vec::new(),
            undead: false,
                    behavior: crate::actors::world::ai::make_behavior(&spawn.name),
                });
                debug!("Summoned monster '{}' as #{} at ({},{})", spawn.name, new_oid, spawn.x, spawn.y);
            }

            // ===== 应用 Boss AI 输出队列 =====
            // Boss 移动（合并到 moved_monsters 复用广播逻辑）
            for (oid, nx, ny, dir) in boss_moves.drain(..) {
                moved_monsters.push((oid, nx, ny, dir));
            }
            // Boss 攻击：广播 ObjectAttack + 对命中的玩家造成伤害
            for atk in &boss_attacks {
                let (attacker_oid, targets, damage, spell_id, attack_type, atk_x, atk_y, atk_dir) = match atk {
                    ai::AttackAction::Melee { attacker_oid, target_session, damage, spell_id, attack_type } => {
                        (*attacker_oid, vec![*target_session], *damage, *spell_id, *attack_type, 0i32, 0i32, 0u8)
                    }
                    ai::AttackAction::Range { attacker_oid, target_session, damage, spell_id, .. } => {
                        (*attacker_oid, vec![*target_session], *damage, *spell_id, 0u8, 0i32, 0i32, 0u8)
                    }
                    ai::AttackAction::Aoe { attacker_oid, center_x, center_y, radius, damage, .. } => {
                        let tgts: Vec<u64> = player_positions.iter()
                            .filter(|(_, px, py, _, _)| {
                                let dx = (px - center_x).abs();
                                let dy = (py - center_y).abs();
                                dx.max(dy) <= *radius
                            })
                            .map(|(s, _, _, _, _)| *s)
                            .collect();
                        (*attacker_oid, tgts, *damage, 0u8, 0u8, *center_x, *center_y, 0u8)
                    }
                };
                // 获取 Boss 位置用于广播
                let (boss_x, boss_y, boss_dir) = self.monsters.get(&attacker_oid)
                    .map(|m| (m.x, m.y, m.direction))
                    .unwrap_or((atk_x, atk_y, atk_dir));
                // 广播 ObjectAttack 给所有玩家（Boss 攻击动画）
                let mut attack_body = Vec::new();
                attack_body.extend_from_slice(&attacker_oid.to_le_bytes());
                attack_body.extend_from_slice(&(boss_x as u32).to_le_bytes());
                attack_body.extend_from_slice(&(boss_y as u32).to_le_bytes());
                attack_body.push(boss_dir);
                attack_body.push(spell_id);
                attack_body.extend_from_slice(&0u16.to_le_bytes());
                attack_body.push(attack_type);
                let attack_packet = build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::ObjectAttack as i16, &attack_body);
                for sid in self.players.keys() {
                    let _ = self.gate_ref.ask(SendToClient {
                        session_id: *sid,
                        data: attack_packet.clone(),
                    });
                }
                // 对命中玩家造成伤害
                for sid in &targets {
                    if let Some(record) = self.players.get(sid) {
                        let _ = record.actor_ref.ask(TakeDamage {
                            attacker_id: attacker_oid,
                            attacker_session: *sid,
                            damage,
                        }).await;
                    }
                }
            }
            // Boss 地面法术场：转为 SpellObject
            for sf in &boss_spell_fields {
                let oid = self.alloc_object_id();
                let spell_obj = spell::SpellObject::new(
                    oid, sf.spell, sf.caster_oid, sf.caster_session,
                    sf.x, sf.y, sf.duration_ms, sf.value, sf.tick_ms, 1, sf.value,
                );
                self.spell_objects.insert(oid, spell_obj);
            }
            // Boss 召唤：按名称查 MonsterInfo 后生成（对齐 C# Envir.GetMonsterInfo(name)）
            for bs in &boss_summons {
                let mon_index = self.monster_name_index.get(&bs.monster_name.to_lowercase()).copied();
                if let Some(idx) = mon_index {
                    // 先 clone MonsterInfo 避免 &self.monster_infos 与 &mut self.alloc_object_id 借用冲突
                    let info_opt = self.monster_infos.get(&idx).cloned();
                    if let Some(info) = info_opt {
                        let new_oid = self.alloc_object_id();
                        let hp = info.stats.get(&(mir2_shared::enums::Stat::HP as u8)).copied().unwrap_or(50);
                        let min_dmg = info.stats.get(&(mir2_shared::enums::Stat::MinDC as u8)).copied().unwrap_or(5);
                        let max_dmg = info.stats.get(&(mir2_shared::enums::Stat::MaxDC as u8)).copied().unwrap_or(10);
                        let map_index = self.monsters.values().next().map(|m| m.map_index).unwrap_or(0);
                        // 广播新怪物生成
                        let spawn = MonsterSpawn {
                            name: info.name.clone(),
                            image: info.image as u16,
                            monster_index: idx,
                            x: bs.x,
                            y: bs.y,
                            direction: 0,
                            hp,
                            min_dmg,
                            max_dmg,
                            xp: info.experience,
                            map_index,
                        };
                        let packet = build_object_monster_packet(&spawn, new_oid, &spawn.name);
                        for session_id in self.players.keys() {
                            let _ = self.gate_ref.ask(SendToClient {
                                session_id: *session_id,
                                data: packet.clone(),
                            });
                        }
                        let ai_profile = MonsterAiProfile::from_info(&info);
                        self.monsters.insert(new_oid, MonsterState {
                            object_id: new_oid,
                            name: spawn.name.clone(),
                            image: spawn.image,
                            monster_index: idx,
                            x: bs.x, y: bs.y, direction: 0,
                            hp, max_hp: hp, min_dmg, max_dmg, xp: spawn.xp,
                            spawn_x: bs.x, spawn_y: bs.y, map_index,
                            next_attack_tick: 0, next_move_tick: 0, next_summon_tick: 0,
                            ai_profile, ai_state: MonsterAiState::Idle,
                            target_session: None, provoked: false,
                            is_elite: false, is_boss: false,
                            min_ac: 0, max_ac: 0, min_mac: 0, max_mac: 0,
                            agility: 0, accuracy: 0,
                            armour_rate: 1.0, damage_rate: 1.0,
                            magic_resist: 0, critical_rate: 0, critical_damage: 0,
                            luck: 0, reflect: 0, damage_reduction_percent: 0,
                            poison_list: Vec::new(),
            undead: false,
                            behavior: crate::actors::world::ai::make_behavior(&spawn.name),
                        });
                        debug!("Boss summoned '{}' as #{} at ({},{}) slave={}", spawn.name, new_oid, bs.x, bs.y, bs.is_slave);
                    } else {
                        debug!("Boss summon '{}' found index {} but no MonsterInfo", bs.monster_name, idx);
                    }
                } else {
                    debug!("Boss summon '{}' not in monster_name_index (DB may lack this mob)", bs.monster_name);
                }
            }
            // Boss 对玩家的 poison
            for pp in &boss_poisons {
                if let Some(record) = self.players.get(&pp.session_id) {
                    let _ = record.actor_ref.ask(crate::actors::player::ApplyCombatPoisons {
                        poisons: vec![pp.poison],
                    }).await;
                }
            }
            // Boss 怪物互疗
            for (target_oid, amount) in &boss_heals {
                if let Some(m) = self.monsters.get_mut(target_oid) {
                    m.hp = (m.hp + *amount).min(m.max_hp);
                }
            }

            // 应用移动并广播
            for (oid, nx, ny, dir) in &moved_monsters {
                if let Some(m) = self.monsters.get_mut(oid) {
                    m.x = *nx;
                    m.y = *ny;
                    m.direction = *dir;

                    // 广播 ObjectWalk（object_id + x + y + direction，~12字节 vs ObjectMonster ~40字节）
                    let mut walk_body = Vec::new();
                    walk_body.extend_from_slice(&oid.to_le_bytes());
                    walk_body.extend_from_slice(&m.x.to_le_bytes());
                    walk_body.extend_from_slice(&m.y.to_le_bytes());
                    walk_body.push(m.direction);
                    let walk_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectWalk as i16, &walk_body);
                    for session_id in self.players.keys() {
                        let _ = self.gate_ref.ask(SendToClient {
                            session_id: *session_id,
                            data: walk_packet.clone(),
                        });
                    }
                }
            }

            // 处理破损装备广播（避免在怪物循环内借用 self）
            for (target_session, slot) in &broken_armor {
                if let Some(state) = self.recalculate_and_set_stat_bonuses(*target_session).await {
                    if *slot == EquipmentSlot::Weapon || *slot == EquipmentSlot::Armour {
                        self.broadcast_equipment_visuals(*target_session, &state).await;
                    }
                }
            }

            // 处理死亡怪物
            for oid in &dead_monsters {
                if let Some(monster) = self.monsters.remove(oid) {
                    debug!("Monster '{}' (#{}) died", monster.name, oid);
                    // 发送 ObjectDied（死亡动画）
                    let died_packet = Self::build_object_died_packet(
                        *oid, monster.x, monster.y, monster.direction);
                    // 发送 ObjectRemove（清理实体）
                    let remove_packet = Self::build_object_remove_packet(*oid);
                    for session_id in self.players.keys() {
                        let _ = self.gate_ref.ask(SendToClient {
                            session_id: *session_id,
                            data: died_packet.clone(),
                        });
                        let _ = self.gate_ref.ask(SendToClient {
                            session_id: *session_id,
                            data: remove_packet.clone(),
                        });
                    }

                    // 生成掉落物品
                    self.spawn_monster_drops(&monster).await;

                    // 世界Boss被击败广播
                    if monster.is_boss {
                        self.world_boss_queue.remove(oid);
                        broadcast_system_message(
                            &self.gate_ref, &self.players,
                            &format!("世界Boss {} 被英勇的勇士们击败了！", monster.name));
                        debug!("World boss '{}' defeated", monster.name);
                    }

                    // 发放经验（支持组队平分）
                    let mut nearest_session: Option<u64> = None;
                    let mut nearest_dist = i32::MAX;
                    let mut nearest_group_id: Option<u64> = None;
                    for (session_id, record) in &self.players {
                        if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                            let dist = (state.x - monster.x).abs() + (state.y - monster.y).abs();
                            if dist < nearest_dist {
                                nearest_dist = dist;
                                nearest_session = Some(*session_id);
                                nearest_group_id = state.group_id;
                            }
                        }
                    }
                    if let Some(session_id) = nearest_session {
                        if let Some(group_id) = nearest_group_id {
                            // 组队经验：组内所有在线成员平分
                            let mut group_sessions = Vec::new();
                            for (sid, record) in &self.players {
                                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                    if state.group_id == Some(group_id) {
                                        let dist = (state.x - monster.x).abs() + (state.y - monster.y).abs();
                                        if dist <= 12 && state.map_index == monster.map_index {
                                            group_sessions.push(*sid);
                                        }
                                    }
                                }
                            }
                            if !group_sessions.is_empty() {
                                let xp_per = (monster.xp / group_sessions.len() as i32).max(1);
                                for sid in &group_sessions {
                                    if let Some(record) = self.players.get(sid) {
                                        let _ = record.actor_ref.ask(crate::actors::player::AddExperience {
                                            amount: self.apply_global_exp_multiplier(xp_per),
                                        }).await;
                                    }
                                }
                                debug!("GroupXP: {} members split {} xp ({} each) from '{}'", group_sessions.len(), monster.xp, xp_per, monster.name);
                            }
                            // 组队师徒/夫妻经验加成
                            for sid in &group_sessions {
                                if let Some(record) = self.players.get(sid) {
                                    if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                        // 师徒加成
                                        if let Some(ref mentor_name) = state.mentor_name {
                                            for (other_sid, other_record) in &self.players {
                                                if *other_sid == *sid { continue; }
                                                if let Ok(Some(other_state)) = other_record.actor_ref.ask(GetPlayerState).await {
                                                    if other_state.name.eq_ignore_ascii_case(mentor_name)
                                                        && other_state.map_index == state.map_index {
                                                        let dist = (other_state.x - state.x).abs() + (other_state.y - state.y).abs();
                                                        if dist <= 12 {
                                                            let bonus = (monster.xp as f64 * 0.10).round() as i32;
                                                            let _ = record.actor_ref.ask(crate::actors::player::AddExperience {
                                                                amount: self.apply_global_exp_multiplier(bonus),
                                                            }).await;
                                                            let _ = other_record.actor_ref.ask(crate::actors::player::AddExperience {
                                                                amount: self.apply_global_exp_multiplier(bonus),
                                                            }).await;
                                                            send_system_message(&self.gate_ref, *sid, "师徒同心，额外获得经验！");
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        // 夫妻加成
                                        if let Some(ref spouse_name) = state.spouse_name {
                                            for (other_sid, other_record) in &self.players {
                                                if *other_sid == *sid { continue; }
                                                if let Ok(Some(other_state)) = other_record.actor_ref.ask(GetPlayerState).await {
                                                    if other_state.name.eq_ignore_ascii_case(spouse_name)
                                                        && other_state.map_index == state.map_index {
                                                        let dist = (other_state.x - state.x).abs() + (other_state.y - state.y).abs();
                                                        if dist <= 12 {
                                                            let bonus = (monster.xp as f64 * 0.10).round() as i32;
                                                            let _ = record.actor_ref.ask(crate::actors::player::AddExperience {
                                                                amount: self.apply_global_exp_multiplier(bonus),
                                                            }).await;
                                                            let _ = other_record.actor_ref.ask(crate::actors::player::AddExperience {
                                                                amount: self.apply_global_exp_multiplier(bonus),
                                                            }).await;
                                                            send_system_message(&self.gate_ref, *sid, "夫妻同心，额外获得经验！");
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // 组队任务击杀进度
                            for sid in &group_sessions {
                                if let Some(record) = self.players.get(sid) {
                                    let updates = record.actor_ref.ask(crate::actors::player::ProcessMonsterKill {
                                        monster_index: monster.monster_index,
                                    }).await.unwrap_or_default();
                                    if !updates.is_empty() {
                                        send_system_message(&self.gate_ref, *sid, &format!("任务进度更新：击杀了 {}", monster.name));
                                    }
                                    for (quest_index, _mid, complete) in updates {
                                        debug!("QuestKill: session={} quest={} monster={} complete={}", sid, quest_index, monster.monster_index, complete);
                                    }
                                }
                            }
                        } else if let Some(record) = self.players.get(&session_id) {
                            let _ = record.actor_ref.ask(crate::actors::player::AddExperience {
                                amount: self.apply_global_exp_multiplier(monster.xp),
                            }).await;
                            // 单人师徒/夫妻经验加成
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                // 师徒加成
                                if let Some(ref mentor_name) = state.mentor_name {
                                    for (other_sid, other_record) in &self.players {
                                        if *other_sid == session_id { continue; }
                                        if let Ok(Some(other_state)) = other_record.actor_ref.ask(GetPlayerState).await {
                                            if other_state.name.eq_ignore_ascii_case(mentor_name)
                                                && other_state.map_index == state.map_index {
                                                let dist = (other_state.x - state.x).abs() + (other_state.y - state.y).abs();
                                                if dist <= 12 {
                                                    let bonus = (monster.xp as f64 * 0.10).round() as i32;
                                                    let _ = record.actor_ref.ask(crate::actors::player::AddExperience {
                                                        amount: self.apply_global_exp_multiplier(bonus),
                                                    }).await;
                                                    let _ = other_record.actor_ref.ask(crate::actors::player::AddExperience {
                                                        amount: self.apply_global_exp_multiplier(bonus),
                                                    }).await;
                                                    send_system_message(
                                                        &self.gate_ref, session_id, "师徒同心，额外获得经验！");
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                                // 夫妻加成
                                if let Some(ref spouse_name) = state.spouse_name {
                                    for (other_sid, other_record) in &self.players {
                                        if *other_sid == session_id { continue; }
                                        if let Ok(Some(other_state)) = other_record.actor_ref.ask(GetPlayerState).await {
                                            if other_state.name.eq_ignore_ascii_case(spouse_name)
                                                && other_state.map_index == state.map_index {
                                                let dist = (other_state.x - state.x).abs() + (other_state.y - state.y).abs();
                                                if dist <= 12 {
                                                    let bonus = (monster.xp as f64 * 0.10).round() as i32;
                                                    let _ = record.actor_ref.ask(crate::actors::player::AddExperience {
                                                        amount: self.apply_global_exp_multiplier(bonus),
                                                    }).await;
                                                    let _ = other_record.actor_ref.ask(crate::actors::player::AddExperience {
                                                        amount: self.apply_global_exp_multiplier(bonus),
                                                    }).await;
                                                    send_system_message(
                                                        &self.gate_ref, session_id, "夫妻同心，额外获得经验！");
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // 单人任务击杀进度
                            let updates = record.actor_ref.ask(crate::actors::player::ProcessMonsterKill {
                                monster_index: monster.monster_index,
                            }).await.unwrap_or_default();
                            if !updates.is_empty() {
                                send_system_message(&self.gate_ref, session_id, &format!("任务进度更新：击杀了 {}", monster.name));
                            }
                            for (quest_index, _mid, complete) in updates {
                                debug!("QuestKill: session={} quest={} monster={} complete={}", session_id, quest_index, monster.monster_index, complete);
                            }
                        }
                    }

                    // 加入重生队列（3 秒后重生 = 30 ticks @ 100ms）
                    let respawn_tick = self.tick_count + 30;
                    let spawn = MonsterSpawn {
                        name: monster.name.clone(),
                        image: monster.image,
                        monster_index: monster.monster_index,
                        x: monster.spawn_x,
                        y: monster.spawn_y,
                        direction: monster.direction,
                        hp: monster.max_hp,
                        min_dmg: monster.min_dmg,
                        max_dmg: monster.max_dmg,
                        xp: monster.xp,
                        map_index: monster.map_index,
                    };
                    self.respawn_queue.insert(*oid, (spawn, respawn_tick));
                }
            }

            // 处理玩家死亡掉落（在怪物循环外，避免借用冲突）
            for (sid, x, y, map_index) in death_drops {
                self.handle_player_death_drop(sid, x, y, map_index).await;
            }
            // 处理被怪物攻击后的自动下坐骑（在怪物循环外，避免借用冲突）
            for sid in dismount_sessions {
                self.dismount_player(sid).await;
            }
        }

        self.tick_buffs_and_revive().await;

        self.tick_environment_damage().await;

        self.tick_exp_events_and_invisibility().await;

        self.tick_pk_decay().await;

        self.tick_fishing().await;

        self.tick_ground_cleanup().await;

        self.tick_respawn().await;

        self.tick_boss_timeout().await;

        self.tick_quest_timeout().await;

        self.tick_pet_pickup().await;

        self.tick_shop_restock().await;

        self.tick_refine_complete().await;
        self.tick_regen_and_hunger().await;

        self.tick_day_night().await;

        self.tick_auto_save().await;

        self.tick_auction_expiry().await;

        self.tick_rental_expiry().await;

        self.tick_spells().await;

        self.tick_spell_completions().await;

        self.tick_robots().await;

        self.tick_dragon().await;

        self.tick_conquest().await;
    }
}
