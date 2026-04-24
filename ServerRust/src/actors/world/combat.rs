use super::*;

/// 攻击请求（从 GateActor 转发）
pub struct WorldAttackRequest {
    pub session_id: u64,
    pub direction: u8,
    pub spell: u8,
}

/// 采集请求（从 GateActor 转发）
pub struct HarvestRequest {
    pub session_id: u64,
    pub direction: u8,
}

impl Message<WorldAttackRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: WorldAttackRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => {
                warn!("Attack request for unknown session {}", msg.session_id);
                return;
            }
        };

        // 发送攻击请求到 PlayerActor，同时获取玩家属性用于伤害计算
        let attacker_state = record.actor_ref.ask(GetPlayerState).await.ok().flatten();
        if let Some(ref state) = attacker_state {
            if state.is_dead { return; }
        }

        // 攻击时自动下坐骑
        self.dismount_player(msg.session_id).await;

        // 攻击时打破隐身
        if self.invisible_sessions.remove(&msg.session_id) {
            if let Some(ref state) = attacker_state {
                let _ = record.actor_ref.ask(crate::actors::player::RemoveBuff {
                    buff_type: crate::combat::buff::BuffType::Invisibility,
                }).await;
                self.reveal_player_to_others(msg.session_id, state).await;
            }
        }

        if let (Some(ref state), Ok(Some(result))) = (attacker_state, record.actor_ref.ask(AttackRequest {
            session_id: msg.session_id,
            direction: msg.direction,
            spell: msg.spell,
        }).await) {
            // 广播 ObjectAttack 给其他玩家
            let others: Vec<_> = self.other_players(msg.session_id)
                .into_iter()
                .map(|r| (r.actor_ref.clone(), r.session_id))
                .collect();

            let mut attack_body = Vec::new();
            attack_body.extend_from_slice(&result.object_id.to_le_bytes());
            attack_body.extend_from_slice(&(result.x as u32).to_le_bytes());
            attack_body.extend_from_slice(&(result.y as u32).to_le_bytes());
            attack_body.push(result.direction);
            attack_body.push(result.spell);
            attack_body.extend_from_slice(&0u16.to_le_bytes()); // spell_level
            attack_body.push(0u8); // attack_type
            let packet = build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectAttack as i16, &attack_body);

            // --- 检测是否命中怪物 ---
            // 计算攻击方向的前方位置
            let atk_dir = result.direction as usize % 8;
            let target_x = result.x + MON_DIR_DX[atk_dir];
            let target_y = result.y + MON_DIR_DY[atk_dir];

            let mut hit_monster = false;
            for (oid, monster) in &mut self.monsters {
                let dist = (monster.x - target_x).abs() + (monster.y - target_y).abs();
                if dist <= 1 {
                    // 命中怪物 - 使用战斗模块计算伤害（包含 Buff 加成）
                    let attack_result = combat_attack::resolve_attack(
                        state.effective_min_attack(), state.effective_max_attack(), 0
                    );
                    let damage = attack_result.damage;
                    monster.hp = monster.hp.saturating_sub(damage);
                    monster.provoked = true;
                    monster.target_session = Some(msg.session_id);
                    debug!("Player {} hit monster '{}' (#{}) for {} dmg (crit={}) (hp={}/{})",
                           result.object_id, monster.name, *oid, damage, attack_result.is_critical, monster.hp, monster.max_hp);

                    // 发送 ObjectStruck（受击动画）
                    let mut struck_body = Vec::new();
                    struck_body.extend_from_slice(&monster.object_id.to_le_bytes());
                    struck_body.extend_from_slice(&result.object_id.to_le_bytes());
                    struck_body.extend_from_slice(&(monster.x as u32).to_le_bytes());
                    struck_body.extend_from_slice(&(monster.y as u32).to_le_bytes());
                    struck_body.push(monster.direction);
                    let struck_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectStruck as i16, &struck_body);

                    // 发送 DamageIndicator（伤害数字）
                    let mut dmg_body = Vec::new();
                    dmg_body.extend_from_slice(&damage.to_le_bytes());
                    dmg_body.push(0u8); // damage_type = normal
                    dmg_body.extend_from_slice(&monster.object_id.to_le_bytes());
                    let dmg_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::DamageIndicator as i16, &dmg_body);

                    // 发送 ObjectHealth（百分比血条）
                    let percent = ((monster.hp.max(0) as f32 / monster.max_hp as f32) * 100.0) as u8;
                    let mut health_body = Vec::new();
                    health_body.extend_from_slice(&monster.object_id.to_le_bytes());
                    health_body.push(percent);
                    health_body.extend_from_slice(&0u16.to_le_bytes()); // expire
                    let health_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectHealth as i16, &health_body);

                    // 广播给所有玩家
                    for session_id in self.players.keys() {
                        let _ = self.gate_ref.ask(SendToClient {
                            session_id: *session_id,
                            data: struck_packet.clone(),
                        });
                        let _ = self.gate_ref.ask(SendToClient {
                            session_id: *session_id,
                            data: dmg_packet.clone(),
                        });
                        let _ = self.gate_ref.ask(SendToClient {
                            session_id: *session_id,
                            data: health_packet.clone(),
                        });
                    }

                    hit_monster = true;
                    break; // 一次只打一只
                }
            }

            // 武器耐久损耗（每次攻击一次）
            if hit_monster {
                if let Some(record) = self.players.get(&msg.session_id) {
                    let broke = record.actor_ref.ask(crate::actors::player::DamageEquipment {
                        slot: EquipmentSlot::Weapon,
                        amount: 1,
                    }).await.unwrap_or(false);
                    if broke {
                        debug!("Player {} weapon broke!", result.object_id);
                        if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                            let (b_min, b_max, b_def, b_hp, b_mp) = calculate_equipment_bonuses(
                                &state.inventory.equipment, &self.item_infos,
                            );
                            let _ = record.actor_ref.ask(crate::actors::player::SetStatBonuses {
                                bonus_min_attack: b_min,
                                bonus_max_attack: b_max,
                                bonus_defence: b_def,
                                bonus_max_hp: b_hp,
                                bonus_max_mp: b_mp,
                            }).await;
                            let weapon_shape = state.inventory.get_equipment(EquipmentSlot::Weapon)
                                .and_then(|item| self.item_infos.get(&item.item_index))
                                .map(|info| info.shape as i16).unwrap_or(-1);
                            let armor_shape = state.inventory.get_equipment(EquipmentSlot::Armour)
                                .and_then(|item| self.item_infos.get(&item.item_index))
                                .map(|info| info.shape as i16).unwrap_or(0);
                            let weapon_effect = state.inventory.get_equipment(EquipmentSlot::Weapon)
                                .and_then(|item| self.item_infos.get(&item.item_index))
                                .map(|info| info.effect as i16).unwrap_or(0);
                            let light: u8 = state.inventory.get_equipment(EquipmentSlot::Weapon)
                                .and_then(|item| self.item_infos.get(&item.item_index))
                                .map(|info| info.light as u8)
                                .unwrap_or(0)
                                .max(state.inventory.get_equipment(EquipmentSlot::Armour)
                                    .and_then(|item| self.item_infos.get(&item.item_index))
                                    .map(|info| info.light as u8)
                                    .unwrap_or(0));
                            for other in self.other_players(msg.session_id) {
                                send_player_update(
                                    &self.gate_ref, other.session_id, state.object_id,
                                    light, weapon_shape, weapon_effect, armor_shape, 0,
                                );
                            }
                        }
                    }
                }
            }

            // --- 玩家间伤害（仅在未命中怪物时） ---
            if !hit_monster {
                for (other_actor, other_session) in others {
                    // 获取其他玩家位置做距离检测
                    if let Ok(Some(other_state)) = other_actor.ask(GetPlayerState).await {
                        // 计算曼哈顿距离（Mir2 使用 8 方向近战范围约 1-2 格）
                        let dist = (other_state.x - result.x).abs() + (other_state.y - result.y).abs();
                        const MELEE_RANGE: i32 = 2; // 近战有效范围

                        // 发送 ObjectAttack 动画（无论距离）
                        let _ = self.gate_ref.ask(SendToClient {
                            session_id: other_session,
                            data: packet.clone(),
                        });

                        // 只有范围内的玩家才受到伤害
                        if dist <= MELEE_RANGE {
                            // 攻击模式检查
                            if !can_attack_player(state, &other_state) {
                                continue;
                            }

                            // 安全区保护：双方任一在安全区内则禁止伤害
                            let attacker_safe = self.maps.get(&state.map_index)
                                .map(|m| m.is_safe_zone(state.x, state.y))
                                .unwrap_or(false);
                            let target_safe = self.maps.get(&other_state.map_index)
                                .map(|m| m.is_safe_zone(other_state.x, other_state.y))
                                .unwrap_or(false);
                            if attacker_safe || target_safe {
                                continue;
                            }

                            // 使用战斗模块计算伤害（包含 Buff 加成）
                            let attack_result = combat_attack::resolve_attack(
                                state.effective_min_attack(), state.effective_max_attack(), other_state.effective_defence()
                            );
                            let damage = attack_result.damage;
                            if other_actor.ask(TakeDamage {
                                attacker_id: result.object_id,
                                attacker_session: msg.session_id,
                                damage,
                            }).await.unwrap_or(false) {
                                let mut died_body = Vec::new();
                                died_body.extend_from_slice(&other_state.object_id.to_le_bytes());
                                died_body.extend_from_slice(&(other_state.x as u32).to_le_bytes());
                                died_body.extend_from_slice(&(other_state.y as u32).to_le_bytes());
                                died_body.push(other_state.direction);
                                died_body.push(0u8);
                                let died_packet = build_packet_bytes(
                                    mir2_shared::enums::ServerPacketIds::ObjectDied as i16, &died_body);
                                for (sid, _) in &self.players {
                                    let _ = self.gate_ref.ask(SendToClient {
                                        session_id: *sid,
                                        data: died_packet.clone(),
                                    });
                                }
                                self.handle_player_death_drop(other_session, other_state.x, other_state.y, other_state.map_index).await;

                                // 击杀玩家：增加 PK 值并广播名字颜色变化
                                let _ = record.actor_ref.ask(crate::actors::player::AddPkPoints { points: 100 }).await;
                                if let Ok(Some(attacker_state)) = record.actor_ref.ask(GetPlayerState).await {
                                    let colour_packet = build_object_colour_changed_packet(
                                        attacker_state.object_id,
                                        name_colour_for_pk(attacker_state.pk_points),
                                    );
                                    for (sid, _) in &self.players {
                                        let _ = self.gate_ref.ask(SendToClient {
                                            session_id: *sid,
                                            data: colour_packet.clone(),
                                        });
                                    }
                                    if let Some(r) = self.players.get_mut(&msg.session_id) {
                                        r.last_pk_points = attacker_state.pk_points;
                                    }
                                }
                            }
                            debug!("Hit! {} damaged {} for {} (dist={}, crit={})",
                                   result.object_id, other_state.name, damage, dist, attack_result.is_critical);
                        }
                    }
                }
            } else {
                // 命中怪物时也要广播 ObjectAttack 给所有玩家
                for (_other_actor, other_session) in &self.players.iter().map(|(s, r)| (r.actor_ref.clone(), *s)).collect::<Vec<_>>() {
                    let _ = self.gate_ref.ask(SendToClient {
                        session_id: *other_session,
                        data: packet.clone(),
                    });
                }
            }
        }
    }
}

// ============================================================
// 采集系统（Harvest：挖矿/采集）
// ============================================================

/// 方向到坐标偏移（8 方向）
const HARVEST_DIR_DX: [i32; 8] = [0, 1, 1, 1, 0, -1, -1, -1];
const HARVEST_DIR_DY: [i32; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];

impl Message<HarvestRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: HarvestRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if state.is_dead { return; }

        let dir = msg.direction as usize % 8;
        let target_x = state.x + HARVEST_DIR_DX[dir];
        let target_y = state.y + HARVEST_DIR_DY[dir];

        debug!(
            "Harvest: {} session={} dir={} target=({}, {})",
            state.name, msg.session_id, dir, target_x, target_y
        );

        // 检查当前地图是否可采集
        let map_info = self.map_infos.get(&(state.map_index as i32));
        let mine_index = map_info.map(|m| m.mine_index).unwrap_or(0);
        if mine_index <= 0 {
            send_system_message(&self.gate_ref, msg.session_id, "这里没有什么可采集的");
            return;
        }

        // 检查是否持有镐类工具
        let has_pickaxe = state.inventory.backpack.iter().chain(state.inventory.storage.iter())
            .any(|slot| {
                if let Some(item) = slot {
                    self.item_infos.get(&item.item.item_index)
                        .map(|info| {
                            let n = info.name.to_lowercase();
                            n.contains('镐') || n.contains("pick") || n.contains("hoe") || n.contains("锄")
                        })
                        .unwrap_or(false)
                } else {
                    false
                }
            });
        if !has_pickaxe {
            send_system_message(&self.gate_ref, msg.session_id, "你需要一把镐才能采矿");
            return;
        }

        // 广播 ObjectHarvest 给附近其他玩家
        let harvest_body = {
            let mut b = Vec::new();
            b.extend_from_slice(&state.object_id.to_le_bytes());
            b.extend_from_slice(&(target_x as i32).to_le_bytes());
            b.extend_from_slice(&(target_y as i32).to_le_bytes());
            b.push(msg.direction);
            build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectHarvest as i16, &b)
        };
        for other in self.other_players(msg.session_id) {
            let _ = self.gate_ref.ask(SendToClient {
                session_id: other.session_id,
                data: harvest_body.clone(),
            });
        }

        // 延迟处理采集结果
        let object_id = state.object_id;
        let gate_ref = self.gate_ref.clone();
        let actor_ref = record.actor_ref.clone();
        let item_infos = self.item_infos.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(1500)).await;
            let mut b = Vec::new();
            b.extend_from_slice(&object_id.to_le_bytes());
            b.extend_from_slice(&(target_x as i32).to_le_bytes());
            b.extend_from_slice(&(target_y as i32).to_le_bytes());
            b.push(msg.direction);
            let packet = build_packet_bytes(
                mir2_shared::enums::ServerPacketIds::ObjectHarvested as i16, &b,
            );
            let _ = gate_ref.ask(SendToClient {
                session_id: msg.session_id,
                data: packet,
            });

            // 掉落判定
            let roll = (msg.session_id.wrapping_add(tokio::time::Instant::now().elapsed().as_millis() as u64) % 100) as u8;
            let (drop_item_index, drop_count, drop_name) = match mine_index {
                1 if roll < 70 => (500, 1 + (roll % 2) as u16, "铁矿石"),
                2 if roll < 50 => (501, 1, "金矿石"),
                3 if roll < 30 => (502, 1, "宝石"),
                _ => (0, 0, ""),
            };
            if drop_item_index > 0 {
                let item_name = item_infos.get(&drop_item_index)
                    .map(|i| i.name.clone())
                    .unwrap_or_else(|| drop_name.to_string());
                let item = mir2_shared::data::item::UserItem {
                    item_index: drop_item_index,
                    count: drop_count,
                    ..Default::default()
                };
                let _ = actor_ref.ask(crate::actors::player::AddItemToInventory { item }).await;
                send_system_message(&gate_ref, msg.session_id,
                    &format!("采集成功！获得了 {} x{}", item_name, drop_count));
            } else {
                send_system_message(&gate_ref, msg.session_id, "采集成功，但这次什么也没有挖到");
            }
        });
    }
}

/// 查看玩家信息
pub struct InspectPlayerRequest {
    pub session_id: u64,
    pub target_id: u32,
}

impl Message<InspectPlayerRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: InspectPlayerRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let mut target_state: Option<crate::actors::player::PlayerState> = None;
        for r in self.players.values() {
            if let Ok(Some(s)) = r.actor_ref.ask(GetPlayerState).await {
                if s.object_id == msg.target_id {
                    target_state = Some(s);
                    break;
                }
            }
        }

        let Some(target) = target_state else {
            send_system_message(&self.gate_ref, msg.session_id, "找不到目标玩家");
            return;
        };

        // 发送 PlayerInspect 包
        send_inspect_packet(&self.gate_ref, msg.session_id, &target);
    }
}

/// 观察玩家
pub struct ObservePlayerRequest {
    pub session_id: u64,
    pub target_id: u32,
}

impl Message<ObservePlayerRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: ObservePlayerRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let mut target_state: Option<crate::actors::player::PlayerState> = None;
        for r in self.players.values() {
            if let Ok(Some(s)) = r.actor_ref.ask(GetPlayerState).await {
                if s.object_id == msg.target_id {
                    target_state = Some(s);
                    break;
                }
            }
        }

        let Some(target) = target_state else {
            return;
        };

        // Send AllowObserve(true)
        let mut allow_body = Vec::new();
        allow_body.push(1u8);
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::AllowObserve as i16, &allow_body),
        });

        // Send PlayerInspect with target info
        send_inspect_packet(&self.gate_ref, msg.session_id, &target);
    }
}

/// 城镇复活请求
pub struct TownReviveRequest {
    pub session_id: u64,
}

impl Message<TownReviveRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: TownReviveRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if !state.is_dead { return; }

        // 复活：重置 HP/MP 到最大值，回到地图出生点
        let spawn_x = DEFAULT_SPAWN_X;
        let spawn_y = DEFAULT_SPAWN_Y;

        let _ = record.actor_ref.ask(crate::actors::player::RevivePlayer {
            x: spawn_x,
            y: spawn_y,
            map_index: state.map_index,
        }).await;

        // 发送 HealthChanged 通知
        let mut health_body = Vec::new();
        health_body.extend_from_slice(&(state.max_hp as u32).to_le_bytes());
        health_body.extend_from_slice(&(state.max_mp as u32).to_le_bytes());
        let _ = self.gate_ref.ask(crate::gate::actor::SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::HealthChanged as i16, &health_body),
        });

        debug!("TownRevive: {} revived at ({}, {})", state.name, spawn_x, spawn_y);
    }
}

/// 远程攻击请求（同普通攻击，但带目标位置）
pub struct RangeAttackRequest {
    pub session_id: u64,
    pub direction: u8,
    pub target_id: u32,
    pub target_x: i32,
    pub target_y: i32,
}

impl Message<RangeAttackRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: RangeAttackRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if state.is_dead { return; }

        let object_id = state.object_id;
        let target_x = msg.target_x;
        let target_y = msg.target_y;

        // 广播 ObjectAttack 给其他玩家
        let others: Vec<_> = self.other_players(msg.session_id)
            .into_iter().cloned()
            .collect();
        for other in &others {
            let mut body = Vec::new();
            body.extend_from_slice(&object_id.to_le_bytes());
            body.push(msg.direction);
            body.push(0u8); // spell = 0 (range attack)
            let _ = self.gate_ref.ask(SendToClient {
                session_id: other.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectAttack as i16, &body),
            });
        }

        // 检测范围内的怪物
        let hit_monster_ids: Vec<u32> = self.monsters.iter()
            .filter(|(_, m)| {
                let dist = (m.x - target_x).abs() + (m.y - target_y).abs();
                dist <= 1
            })
            .map(|(id, _)| *id)
            .collect();

        let mut hit_monster = false;
        for monster_id in hit_monster_ids {
            if let Some(monster) = self.monsters.get_mut(&monster_id) {
                let attack_result = combat_attack::resolve_attack(
                    state.effective_min_attack(), state.effective_max_attack(), 0
                );
                let damage = attack_result.damage;
                monster.hp = monster.hp.saturating_sub(damage);
                monster.provoked = true;
                monster.target_session = Some(msg.session_id);
                debug!("RangeAttack: {} -> monster {} for {} damage", state.name, monster_id, damage);
                hit_monster = true;
                if monster.hp <= 0 {
                    // 死亡由 Tick 循环处理（广播 ObjectDied + 重生）
                }
            }
        }

        // 未命中怪物时尝试命中玩家（PvP）
        if !hit_monster {
            for other in &others {
                if let Ok(Some(other_state)) = other.actor_ref.ask(GetPlayerState).await {
                    let dist = (other_state.x - target_x).abs() + (other_state.y - target_y).abs();
                    if dist <= 1 {
                        // 攻击模式检查
                        if !can_attack_player(&state, &other_state) {
                            continue;
                        }
                        // 安全区保护
                        let attacker_safe = self.maps.get(&state.map_index)
                            .map(|m| m.is_safe_zone(state.x, state.y))
                            .unwrap_or(false);
                        let target_safe = self.maps.get(&other_state.map_index)
                            .map(|m| m.is_safe_zone(other_state.x, other_state.y))
                            .unwrap_or(false);
                        if attacker_safe || target_safe {
                            continue;
                        }

                        let attack_result = combat_attack::resolve_attack(
                            state.effective_min_attack(), state.effective_max_attack(), other_state.effective_defence()
                        );
                        let damage = attack_result.damage;
                        if other.actor_ref.ask(TakeDamage {
                            attacker_id: object_id,
                            attacker_session: msg.session_id,
                            damage,
                        }).await.unwrap_or(false) {
                            // 目标死亡处理
                            let mut died_body = Vec::new();
                            died_body.extend_from_slice(&other_state.object_id.to_le_bytes());
                            died_body.extend_from_slice(&(other_state.x as u32).to_le_bytes());
                            died_body.extend_from_slice(&(other_state.y as u32).to_le_bytes());
                            died_body.push(other_state.direction);
                            died_body.push(0u8);
                            let died_packet = build_packet_bytes(
                                mir2_shared::enums::ServerPacketIds::ObjectDied as i16, &died_body);
                            for (sid, _) in &self.players {
                                let _ = self.gate_ref.ask(SendToClient {
                                    session_id: *sid,
                                    data: died_packet.clone(),
                                });
                            }
                            self.handle_player_death_drop(other.session_id, other_state.x, other_state.y, other_state.map_index).await;

                            // 增加 PK 值
                            let _ = record.actor_ref.ask(crate::actors::player::AddPkPoints { points: 100 }).await;
                            if let Ok(Some(attacker_state)) = record.actor_ref.ask(GetPlayerState).await {
                                let colour_packet = build_object_colour_changed_packet(
                                    attacker_state.object_id,
                                    name_colour_for_pk(attacker_state.pk_points),
                                );
                                for (sid, _) in &self.players {
                                    let _ = self.gate_ref.ask(SendToClient {
                                        session_id: *sid,
                                        data: colour_packet.clone(),
                                    });
                                }
                                if let Some(r) = self.players.get_mut(&msg.session_id) {
                                    r.last_pk_points = attacker_state.pk_points;
                                }
                            }
                        }
                        debug!("RangeAttack PvP: {} damaged {} for {}", state.name, other_state.name, damage);
                        break; // 远程攻击只命中一个目标
                    }
                }
            }
        }
    }
}

/// 技能释放请求
pub struct MagicRequest {
    pub session_id: u64,
    pub direction: u8,
    pub spell: u8,
    pub target_id: u32,
    pub target_x: i32,
    pub target_y: i32,
}

impl Message<MagicRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: MagicRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => { return; }
        };
        if state.is_dead { return; }

        // 施法时自动下坐骑
        self.dismount_player(msg.session_id).await;

        // 施法时打破隐身
        if self.invisible_sessions.remove(&msg.session_id) {
            let _ = record.actor_ref.ask(crate::actors::player::RemoveBuff {
                buff_type: crate::combat::buff::BuffType::Invisibility,
            }).await;
            self.reveal_player_to_others(msg.session_id, &state).await;
        }

        // Validate spell exists in DB
        let spell_db = self.magic_infos.get(&(msg.spell as u32));

        // 检查玩家是否已学习该技能（基础攻击魔法不需要学习）
        let basic_spells = [0, 1]; // None, 基础攻击
        if !basic_spells.contains(&msg.spell) && !state.magics.iter().any(|m| m.spell == msg.spell as i32) {
            send_system_message(&self.gate_ref, msg.session_id, "你尚未学会这个技能");
            return;
        }
        let spell_range = spell_db.map(|m| m.range as i32).unwrap_or(2);
        let power = spell_db.map(|m| m.power_base).unwrap_or(10);
        let mp_cost = spell_db.map(|m| m.base_cost).unwrap_or(5);

        // 检查并扣除 MP
        if state.mp < mp_cost {
            send_system_message(&self.gate_ref, msg.session_id, "魔法值不足");
            return;
        }
        let mp_ok = record.actor_ref.ask(DeductMP { amount: mp_cost }).await.unwrap_or(false);
        if !mp_ok {
            send_system_message(&self.gate_ref, msg.session_id, "魔法值不足");
            return;
        }

        let object_id = state.object_id;
        let target_x = msg.target_x;
        let target_y = msg.target_y;

        // 发送 MagicCast 给施法者（确认施法）
        let spell_enum = mir2_shared::enums::Spell::try_from(msg.spell)
            .unwrap_or(mir2_shared::enums::Spell::None);
        let magic_cast = mir2_shared::packets::server::magic_combat::MagicCast { spell: spell_enum };
        let mut cast_body = Vec::new();
        if magic_cast.write_body(&mut cast_body).is_ok() {
            let _ = self.gate_ref.ask(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::MagicCast as i16, &cast_body),
            });
        }

        // 广播 ObjectMagic 给其他玩家
        let others: Vec<_> = self.other_players(msg.session_id)
            .into_iter().cloned()
            .collect();
        let object_magic = mir2_shared::packets::server::magic_combat::ObjectMagic {
            object_id,
            location_x: state.x,
            location_y: state.y,
            direction: mir2_shared::enums::MirDirection::try_from(msg.direction)
                .unwrap_or(mir2_shared::enums::MirDirection::Up),
            spell: spell_enum,
            target_id: msg.target_id,
            target_x,
            target_y,
            cast: true,
            level: 0,
            self_broadcast: false,
            secondary_target_ids: Vec::new(),
        };
        let mut om_body = Vec::new();
        if object_magic.write_body(&mut om_body).is_ok() {
            for other in &others {
                let _ = self.gate_ref.ask(SendToClient {
                    session_id: other.session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectMagic as i16, &om_body),
                });
            }
        }

        // 根据魔法类型执行不同效果
        match msg.spell {
            // --- 治愈类 ---
            SPELL_HEALING | SPELL_MASS_HEALING | SPELL_HEALING_CIRCLE => {
                let heal_amount = power.max(10);
                let _ = record.actor_ref.ask(crate::actors::player::Heal {
                    amount: heal_amount,
                }).await;
                debug!("Magic: {} casts Healing(spell={}) for {} HP", state.name, msg.spell, heal_amount);
            }
            // --- Buff 类 ---
            SPELL_MAGIC_SHIELD => {
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::DefenseBoost { bonus: (power / 2).max(5) },
                    300, // 30秒 @ 100ms tick
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts MagicShield (defense +{})", state.name, (power / 2).max(5));
            }
            SPELL_SOUL_SHIELD => {
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::DefenseBoost { bonus: (power / 3).max(3) },
                    600, // 60秒
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts SoulShield (defense +{})", state.name, (power / 3).max(3));
            }
            SPELL_BLESSED_ARMOUR => {
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::AttackBoost { bonus: (power / 2).max(5) },
                    600,
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts BlessedArmour (attack +{})", state.name, (power / 2).max(5));
            }
            // --- 传送类 ---
            SPELL_TELEPORT => {
                if let Some(mi) = self.map_infos.get(&(state.map_index as i32)) {
                    if mi.no_teleport {
                        send_system_message(&self.gate_ref, msg.session_id, "该地图无法使用传送魔法");
                        return;
                    }
                    if mi.no_escape {
                        send_system_message(&self.gate_ref, msg.session_id, "该地图无法使用传送魔法");
                        return;
                    }
                }
                // 限制在地图边界内
                let (max_x, max_y) = self.maps.get(&state.map_index)
                    .map(|m| (m.width as i32, m.height as i32))
                    .unwrap_or((i32::MAX, i32::MAX));
                let tx = target_x.clamp(0, max_x - 1);
                let ty = target_y.clamp(0, max_y - 1);
                let _ = record.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                    x: tx,
                    y: ty,
                    direction: msg.direction,
                    map_index: None,
                    is_mounted: None,
                }).await;
                debug!("Magic: {} teleports to ({}, {})", state.name, tx, ty);
            }
            // --- 默认：伤害类 ---
            _ => {
                // 技能命中范围内的怪物
                let hit_monster_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| {
                        let dist = (m.x - target_x).abs() + (m.y - target_y).abs();
                        dist <= spell_range
                    })
                    .map(|(id, _)| *id)
                    .collect();

                // 魔法伤害 = spell power + 玩家魔法加成
                let power_min = spell_db.map(|m| m.power_base).unwrap_or(10).max(1);
                let power_max = spell_db.map(|m| (m.power_base + m.power_bonus).max(power_min)).unwrap_or(power_min + 5);
                let magic_bonus = state.min_attack / 4; // 简化：攻击力的一部分转化为魔法伤害
                for monster_id in hit_monster_ids {
                    if let Some(monster) = self.monsters.get_mut(&monster_id) {
                        let base_damage = fastrand::i32(power_min..=power_max);
                        let damage = (base_damage + magic_bonus).max(1);
                        monster.hp = monster.hp.saturating_sub(damage);
                        monster.provoked = true;
                        monster.target_session = Some(msg.session_id);
                        debug!("Magic: {} spell={} -> monster {} for {} damage (base={} bonus={})", state.name, msg.spell, monster_id, damage, base_damage, magic_bonus);
                    }
                }
            }
        }
    }
}
