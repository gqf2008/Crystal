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
            // HalfMoon/CrossHalfMoon 溅射目标（循环外应用，避免借用冲突）
            let mut halfmoon_splash: Vec<(u32, i32)> = Vec::new();
            let mut primary_target_oid: u32 = 0; // 主目标 oid（溅射排除用）
            for (oid, monster) in &mut self.monsters {
                let dist = (monster.x - target_x).abs() + (monster.y - target_y).abs();
                if dist <= 1 {
                    // 命中怪物 - 使用完整战斗公式（命中/护甲/暴击/反伤/吸血/负面）
                    let attacker_stats = state.to_combat_stats();
                    let defender_stats = monster.to_combat_stats();
                    let raw_damage = combat_attack::get_attack_power(
                        attacker_stats.min_atk, attacker_stats.max_atk, attacker_stats.luck,
                    );
                    // LevelOffset: 防御方等级高于攻击方时为 0，否则取等级差上限 10
                    // 怪物暂无 level 字段（按 0 处理），玩家攻击怪物时 level_offset = min(10, player_level)
                    let level_offset = state.level.min(10) as u16;
                    let attack_result = combat_attack::resolve_attack(
                        &attacker_stats, &defender_stats, raw_damage,
                        mir2_shared::enums::DefenceType::AcAgility, level_offset,
                    );
                    let damage = attack_result.damage;
                    monster.take_damage(damage);
                    monster.provoked = true;
                    monster.target_session = Some(msg.session_id);
                    // 施加战斗触发的 Poison（冰冻/毒攻），经 behavior.on_poison 过滤
                    for p in &attack_result.applied_poisons {
                        monster.try_apply_poison(*p);
                    }

                    // ===== 战士近战技能触发 =====
                    // Slaying（攻杀）：学了且按等级概率触发，额外伤害（C# 攻杀 = 暴击型额外伤害）
                    let mut slaying_bonus = 0i32;
                    if let Some(lv) = state.magics.iter().find(|m| m.spell == SPELL_SLAYING as i32).map(|m| m.level) {
                        // 概率：level/5（C# 攻杀触发率与等级相关）
                        if fastrand::i32(0..5) < lv as i32 {
                            slaying_bonus = (damage as f32 * (0.5 + lv as f32 * 0.3)) as i32;
                            monster.take_damage(slaying_bonus);
                        }
                    }
                    // HalfMoon（半月）/ CrossHalfMoon（十字半月）：学了则溅射周围怪物
                    let halfmoon_lv = state.magics.iter()
                        .find(|m| m.spell == SPELL_HALFMOON as i32 || m.spell == SPELL_CROSS_HALFMOON as i32)
                        .map(|m| m.level);
                    if let Some(_lv) = halfmoon_lv {
                        // HalfMoon 溅射：记录溅射参数，循环外应用（避免 &self.monsters 借用冲突）
                        let splash_dmg = (damage / 2).max(1);
                        halfmoon_splash.push((0, splash_dmg)); // 标记触发，object_id=0 表示待循环外填充
                        let _ = target_x; // 循环外用 target_x/target_y 收集
                    }
                    let total_dmg = damage + slaying_bonus;
                    debug!("Player {} hit monster '{}' (#{}) for {} dmg (crit={}, slaying={}) (hp={}/{})",
                           result.object_id, monster.name, *oid, total_dmg, attack_result.is_critical, slaying_bonus, monster.hp, monster.max_hp);

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

                    primary_target_oid = *oid;
                    hit_monster = true;
                    break; // 一次只打一只
                }
            }

            // 应用 HalfMoon/CrossHalfMoon 溅射（循环外，避免借用冲突）
            if !halfmoon_splash.is_empty() {
                let splash_dmg = halfmoon_splash[0].1; // 所有溅射伤害相同
                let splash_targets: Vec<u32> = self.monsters.iter()
                    .filter(|(id, m)| {
                        **id != primary_target_oid // 排除主目标（C# HalfMoon 不重复打主目标）
                            && m.hp > 0
                            && (m.x - target_x).abs() <= 1
                            && (m.y - target_y).abs() <= 1
                    })
                    .map(|(id, _)| *id)
                    .collect();
                for sid in splash_targets {
                    if let Some(sm) = self.monsters.get_mut(&sid) {
                        sm.take_damage(splash_dmg);
                        sm.provoked = true;
                        sm.target_session = Some(msg.session_id);
                    }
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
                        if let Some(state) = self.recalculate_and_set_stat_bonuses(msg.session_id).await {
                            self.broadcast_equipment_visuals(msg.session_id, &state).await;
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

                            // 使用完整战斗公式（玩家攻击玩家 PvP）
                            let attacker_stats = state.to_combat_stats();
                            let defender_stats = other_state.to_combat_stats();
                            let raw_damage = combat_attack::get_attack_power(
                                attacker_stats.min_atk, attacker_stats.max_atk, attacker_stats.luck,
                            );
                            // LevelOffset：防御方等级更高时为 0
                            let level_offset = if other_state.level > state.level {
                                0
                            } else {
                                (state.level - other_state.level).min(10) as u16
                            };
                            let attack_result = combat_attack::resolve_attack(
                                &attacker_stats, &defender_stats, raw_damage,
                                mir2_shared::enums::DefenceType::AcAgility, level_offset,
                            );
                            let damage = attack_result.damage;
                            // 施加战斗触发的 Poison 给目标玩家
                            if !attack_result.applied_poisons.is_empty() {
                                let _ = other_actor.ask(crate::actors::player::ApplyCombatPoisons {
                                    poisons: attack_result.applied_poisons,
                                }).await;
                            }
                            if other_actor.ask(TakeDamage {
                                attacker_id: result.object_id,
                                attacker_session: msg.session_id,
                                damage,
                            }).await.unwrap_or(false) {
                                let died_packet = Self::build_object_died_packet(
                                    other_state.object_id, other_state.x, other_state.y, other_state.direction);
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
        let target_x = state.x + MON_DIR_DX[dir];
        let target_y = state.y + MON_DIR_DY[dir];

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

            // Drop table by mine type
            let roll = (msg.session_id.wrapping_add(tokio::time::Instant::now().elapsed().as_millis() as u64) % 100) as u8;
            let (drop_item_index, drop_count, drop_name) = match mine_index {
                // Iron mine: iron ore (70%), copper ore (30%), silver ore (5%), black iron (1%)
                1 if roll < 40 => (500, 1 + (roll % 3) as u16, "铁矿石"),
                1 if roll < 65 => (503, 1, "铜矿石"),
                1 if roll < 70 => (504, 1, "银矿石"),
                1 if roll < 71 => (505, 1, "黑铁矿石"),
                // Gold mine: gold ore (40%), silver (20%), platinum (10%), ruby (5%)
                2 if roll < 40 => (501, 1, "金矿石"),
                2 if roll < 60 => (504, 1 + (roll % 2) as u16, "银矿石"),
                2 if roll < 70 => (506, 1, "铂金矿石"),
                2 if roll < 75 => (507, 1, "红宝石原石"),
                // Gem mine: nephrite (20%), amethyst (15%), diamond (5%), sapphire (3%)
                3 if roll < 20 => (508, 1, "软玉原石"),
                3 if roll < 35 => (509, 1, "紫水晶原石"),
                3 if roll < 40 => (510, 1, "钻石原石"),
                3 if roll < 43 => (511, 1, "蓝宝石原石"),
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
                let attacker_stats = state.to_combat_stats();
                let defender_stats = monster.to_combat_stats();
                let raw_damage = combat_attack::get_attack_power(
                    attacker_stats.min_atk, attacker_stats.max_atk, attacker_stats.luck,
                );
                let level_offset = state.level.min(10) as u16;
                let attack_result = combat_attack::resolve_attack(
                    &attacker_stats, &defender_stats, raw_damage,
                    // 远程物理攻击用 AC 防御（无 Agility 闪避，远程难躲）
                    mir2_shared::enums::DefenceType::Ac, level_offset,
                );
                let damage = attack_result.damage;
                monster.take_damage(damage);
                monster.provoked = true;
                monster.target_session = Some(msg.session_id);
                for p in &attack_result.applied_poisons {
                    crate::combat::poison::apply_poison(&mut monster.poison_list, *p);
                }
                debug!("RangeAttack: {} -> monster {} for {} damage (crit={})", state.name, monster_id, damage, attack_result.is_critical);
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

                        let attacker_stats = state.to_combat_stats();
                        let defender_stats = other_state.to_combat_stats();
                        let raw_damage = combat_attack::get_attack_power(
                            attacker_stats.min_atk, attacker_stats.max_atk, attacker_stats.luck,
                        );
                        let level_offset = if other_state.level > state.level {
                            0
                        } else {
                            (state.level - other_state.level).min(10) as u16
                        };
                        let attack_result = combat_attack::resolve_attack(
                            &attacker_stats, &defender_stats, raw_damage,
                            mir2_shared::enums::DefenceType::Ac, level_offset,
                        );
                        let damage = attack_result.damage;
                        if !attack_result.applied_poisons.is_empty() {
                            let _ = other.actor_ref.ask(crate::actors::player::ApplyCombatPoisons {
                                poisons: attack_result.applied_poisons,
                            }).await;
                        }
                        if other.actor_ref.ask(TakeDamage {
                            attacker_id: object_id,
                            attacker_session: msg.session_id,
                            damage,
                        }).await.unwrap_or(false) {
                            // 目标死亡处理
                            let died_packet = Self::build_object_died_packet(
                                other_state.object_id, other_state.x, other_state.y, other_state.direction);
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

/// 弹道法术的延迟结算项（对齐 C# DelayedAction(DelayedType.Magic, fireTime, ...)）
///
/// 法师弹道类法术（FireBall/ThunderBolt/FrostCrunch/Vampirism）施法时
/// 不立即结算，而是按距离计算飞行时间后推入此队列，由主 tick 在到期时结算。
#[derive(Debug, Clone)]
pub struct PendingSpellCompletion {
    /// 到期 tick（WorldActor.tick_count）
    pub fire_at_tick: u64,
    pub session_id: u64,
    /// 法术原始值（u8，对应 Spell 枚举判别值）
    pub spell: u8,
    /// 目标 object_id（弹道类）
    pub target_id: u32,
    /// 目标快照位置（防移动 miss 校验用）
    pub target_x: i32,
    pub target_y: i32,
    /// 预计算的原始伤害（magic.GetDamage(MC) 结果）
    pub damage: i32,
    /// 施法者魔法属性（MC），用于 Vampirism 吸血计算
    pub magic_stat: i32,
    /// 法术等级
    pub spell_level: u8,
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

        // Pre-allocate object ID for persistent spells (before spell_db borrow)
        let needs_spell_obj = matches!(msg.spell,
            SPELL_FIREWALL | SPELL_BLIZZARD | SPELL_METEOR_STRIKE | SPELL_POISON_CLOUD | SPELL_HEALING_CIRCLE | SPELL_EXPLOSIVE_TRAP
        );
        let spell_oid = if needs_spell_obj { Some(self.alloc_object_id()) } else { None };

        // Validate spell exists in DB
        let spell_db = self.magic_infos.get(&(msg.spell as u32));

        // 检查玩家是否已学习该技能（基础攻击魔法不需要学习）
        let basic_spells = [0, 1]; // None, 基础攻击
        if !basic_spells.contains(&msg.spell) && !state.magics.iter().any(|m| m.spell == msg.spell as i32) {
            send_system_message(&self.gate_ref, msg.session_id, "你尚未学会这个技能");
            return;
        }
        let spell_range = spell_db.map(|m| m.range as i32).unwrap_or(2);
        let power = spell_db.map(|m| m.power_base).unwrap_or(10); // for buff/heal scaling
        // Use spell level from PlayerMagic if learned
        let spell_level = state.magics.iter()
            .find(|m| m.spell == msg.spell as i32)
            .map(|m| m.level)
            .unwrap_or(0);

        // Global timestamp for CD + XP
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        // Cooldown check
        if let Some(spell_info) = spell_db {
            let delay_ms = crate::combat::magic::magic_delay(spell_info, spell_level);
            let last_cast = state.magics.iter()
                .find(|m| m.spell == msg.spell as i32)
                .map(|m| m.cast_time)
                .unwrap_or(0);
            if last_cast > 0 && (now_ms - last_cast) < delay_ms as i64 {
                let remaining = delay_ms as i64 - (now_ms - last_cast);
                send_system_message(&self.gate_ref, msg.session_id, &format!("技能冷却中，还需 {} 秒", remaining / 1000));
                return;
            }
        }

        let mp_cost = spell_db.map(|m| crate::combat::magic::magic_cost(m, spell_level)).unwrap_or(5);

        // Decide which stat feeds this spell
        let magic_stat = match state.class {
            mir2_shared::enums::MirClass::Wizard => state.effective_max_mc(),
            mir2_shared::enums::MirClass::Taoist => state.effective_max_sc(),
            _ => state.effective_max_attack(), // Warriors/Assassins/Archers use Attack
        };

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
            level: spell_level,
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

        // 创建持久法术对象（火墙、暴风雪等）
        let spell_enum = mir2_shared::enums::Spell::try_from(msg.spell)
            .unwrap_or(mir2_shared::enums::Spell::None);
        let is_persistent = matches!(spell_enum,
            mir2_shared::enums::Spell::FireWall | mir2_shared::enums::Spell::Blizzard
            | mir2_shared::enums::Spell::MeteorStrike | mir2_shared::enums::Spell::PoisonCloud
            | mir2_shared::enums::Spell::HealingCircle | mir2_shared::enums::Spell::ExplosiveTrap
            | mir2_shared::enums::Spell::Portal
        );
        let persistent_spell = if is_persistent {
            spell_oid.map(|oid| spell::create_persistent_spell(
                oid, object_id, msg.session_id, state.map_index,
                target_x, target_y, spell_level, magic_stat, spell_enum,
            ))
        } else {
            None
        };
        if let Some(spell_obj) = persistent_spell {
            let spell_type = mir2_shared::enums::Spell::try_from(msg.spell)
                .unwrap_or(mir2_shared::enums::Spell::None);
            let object_spell = mir2_shared::packets::server::magic_combat::ObjectSpell {
                object_id: spell_obj.object_id,
                location_x: spell_obj.x,
                location_y: spell_obj.y,
                spell: spell_type,
            };
            let mut os_body = Vec::new();
            if object_spell.write_body(&mut os_body).is_ok() {
                let spell_packet = build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::ObjectSpell as i16, &os_body,
                );
                // Send to self + nearby players
                let session_ids: Vec<u64> = std::iter::once(msg.session_id)
                    .chain(self.other_players(msg.session_id).iter().map(|p| p.session_id))
                    .collect();
                for sid in &session_ids {
                    let _ = self.gate_ref.tell(SendToClient {
                        session_id: *sid,
                        data: spell_packet.clone(),
                    }).await;
                }
            }
            self.spell_objects.insert(spell_obj.object_id, spell_obj);
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
            // MagicShield：C# 用 Stat.DamageReductionPercent（百分比减伤），非 DefenseBoost
            // 强度 = (level+2)*10%（Lv0=20/Lv1=30/Lv2=40），持续 = GetPower(MC+15) 秒
            SPELL_MAGIC_SHIELD => {
                let reduction_pct = ((spell_level as i32 + 2) * 10).min(80);
                // 持续时间近似：power 已含 MC 加成，转成 ticks（100ms/tick）
                let duration_ticks = ((power.max(15) as u32) * 10).min(6000); // 上限 10 分钟
                let _ = record.actor_ref.ask(crate::actors::player::ApplyDamageReduction {
                    percent: reduction_pct,
                    duration_ticks,
                }).await;
                debug!("Magic: {} casts MagicShield (damage -{}%)", state.name, reduction_pct);
            }
            // SoulShield：MAC 魔法防御 buff（C# Stat.MaxMAC/MinMAC）
            // 注意：Rust 当前 DefenseBoost 不区分 AC/MAC，暂用 DefenseBoost 近似（buff 系统扩展后细分）
            SPELL_SOUL_SHIELD => {
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::DefenseBoost { bonus: (power / 3).max(3) },
                    600, // 60秒
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts SoulShield (MAC defense +{})", state.name, (power / 3).max(3));
            }
            // BlessedArmour：AC 物理防御 buff（C# Stat.MaxAC/MinAC）
            // 修复：原来错误实现为 AttackBoost，C# 实际是 AC 防御
            SPELL_BLESSED_ARMOUR => {
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::DefenseBoost { bonus: (power / 2).max(5) },
                    600,
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts BlessedArmour (AC defense +{})", state.name, (power / 2).max(5));
            }
            // --- 道士 Debuff/控制类 ---
            // Poisoning：对目标怪物施毒（绿毒持续掉血/红毒降防御，C# Poisoning 消耗毒药物品）
            SPELL_POISONING => {
                let hit_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| {
                        let dist = (m.x - target_x).abs() + (m.y - target_y).abs();
                        dist <= spell_range.max(1) && m.hp > 0
                    })
                    .map(|(id, _)| *id)
                    .collect();
                // 绿毒（持续掉血），value 基于 SC
                let poison_value = (magic_stat / 4).max(3).min(10);
                for mid in hit_ids {
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        crate::combat::poison::apply_poison(&mut monster.poison_list,
                            crate::combat::poison::Poison::new(mir2_shared::enums::PoisonType::GREEN, 10, poison_value, 2000));
                        monster.provoked = true;
                        monster.target_session = Some(msg.session_id);
                    }
                }
                debug!("Magic: {} casts Poisoning (green poison {}dmg/tick)", state.name, poison_value);
            }
            // TrapHexagon：定身目标怪物（C# 限制移动，施加 Slow/Paralysis）
            SPELL_TRAP_HEXAGON => {
                let hit_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| {
                        let dist = (m.x - target_x).abs() + (m.y - target_y).abs();
                        dist <= 1 && m.hp > 0
                    })
                    .map(|(id, _)| *id)
                    .collect();
                let trapped_count = hit_ids.len();
                for mid in hit_ids {
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        let duration = (3 + spell_level as u32 * 2).min(15);
                        crate::combat::poison::apply_poison(&mut monster.poison_list,
                            crate::combat::poison::Poison::new(mir2_shared::enums::PoisonType::PARALYSIS, duration, 0, 1000));
                    }
                }
                debug!("Magic: {} casts TrapHexagon (trapped {} monsters)", state.name, trapped_count);
            }
            // --- 道士 Buff/辅助类 ---
            // Hiding：自身隐身（怪物失去目标，C# BuffType.Hiding）
            SPELL_HIDING => {
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::Invisibility,
                    (30 + spell_level as u32 * 10) * 10, // 30-60s，100ms/tick
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                self.invisible_sessions.insert(msg.session_id);
                debug!("Magic: {} casts Hiding (invisible)", state.name);
            }
            // MassHiding：组队隐身（简化：自身 + 附近组员）
            SPELL_MASS_HIDING => {
                // 先给自身
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::Invisibility,
                    (20 + spell_level as u32 * 10) * 10,
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                self.invisible_sessions.insert(msg.session_id);
                // 给附近组员（3 格内）
                let group_id = state.group_id;
                if let Some(gid) = group_id {
                    for (sid, other) in &self.players {
                        if *sid == msg.session_id { continue; }
                        if let Ok(Some(s)) = other.actor_ref.ask(GetPlayerState).await {
                            if s.group_id == Some(gid) {
                                let dist = (s.x - state.x).abs() + (s.y - state.y).abs();
                                if dist <= 3 {
                                    let buff2 = crate::combat::buff::BuffInstance::new(
                                        crate::combat::buff::BuffType::Invisibility,
                                        (20 + spell_level as u32 * 10) * 10,
                                        5,
                                    );
                                    let _ = other.actor_ref.ask(crate::actors::player::ApplyBuff { buff: buff2 }).await;
                                    self.invisible_sessions.insert(*sid);
                                }
                            }
                        }
                    }
                }
                debug!("Magic: {} casts MassHiding", state.name);
            }
            // Purification：解毒（清除自身所有 Poison，C# 清除 debuff）
            SPELL_PURIFICATION => {
                let _ = record.actor_ref.ask(crate::actors::player::PurifyPoisons).await;
                debug!("Magic: {} casts Purification (cleared poisons)", state.name);
            }
            // ShoulderDash：野蛮冲撞（向前冲刺 2 格，推开/伤害路径上的怪物）
            SPELL_SHOULDER_DASH => {
                let dir = msg.direction as usize % 8;
                let mut new_x = state.x;
                let mut new_y = state.y;
                let mut pushed_damage = 0i32;
                for step in 0..2 {
                    let nx = new_x + MON_DIR_DX[dir];
                    let ny = new_y + MON_DIR_DY[dir];
                    let walkable = self.maps.get(&state.map_index)
                        .map(|m| m.is_walkable(nx, ny))
                        .unwrap_or(false);
                    if !walkable { break; }
                    // 伤害路径上的怪物（推开效果简化为伤害）
                    let hit: Option<u32> = self.monsters.iter()
                        .find(|(_, m)| m.x == nx && m.y == ny && m.hp > 0)
                        .map(|(id, _)| *id);
                    if let Some(mid) = hit {
                        let dmg = (state.effective_max_attack() / 2).max(5);
                        if let Some(m) = self.monsters.get_mut(&mid) {
                            m.take_damage(dmg);
                            m.provoked = true;
                            m.target_session = Some(msg.session_id);
                            pushed_damage += dmg;
                        }
                    }
                    new_x = nx;
                    new_y = ny;
                    let _ = step;
                }
                if new_x != state.x || new_y != state.y {
                    let _ = record.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                        x: new_x, y: new_y, direction: msg.direction,
                        map_index: None, is_mounted: None,
                    }).await;
                }
                debug!("Magic: {} casts ShoulderDash (dashed to {},{}, dealt {} dmg)",
                    state.name, new_x, new_y, pushed_damage);
            }
            // Thrusting：刺杀（直线穿透 2 格，打前方 2 个格子）
            // 简化：作为直线 AoE 伤害前方 2 格的怪物
            SPELL_THRUSTING => {
                let dir = msg.direction as usize % 8;
                let attacker_stats = state.to_combat_stats();
                let level_offset = state.level.min(10) as u16;
                let raw_damage = crate::combat::attack::get_attack_power(
                    attacker_stats.min_atk, attacker_stats.max_atk, attacker_stats.luck,
                );
                let mut cx = state.x;
                let mut cy = state.y;
                for _ in 0..2 {
                    cx += MON_DIR_DX[dir];
                    cy += MON_DIR_DY[dir];
                    let hit = self.monsters.iter()
                        .find(|(_, m)| m.x == cx && m.y == cy && m.hp > 0)
                        .map(|(id, _)| *id);
                    if let Some(mid) = hit {
                        if let Some(m) = self.monsters.get_mut(&mid) {
                            let ds = m.to_combat_stats();
                            let r = combat_attack::resolve_attack(
                                &attacker_stats, &ds, raw_damage,
                                mir2_shared::enums::DefenceType::AcAgility, level_offset,
                            );
                            if r.is_hit && r.damage > 0 {
                                m.take_damage(r.damage);
                                m.provoked = true;
                                m.target_session = Some(msg.session_id);
                            }
                        }
                    }
                }
                debug!("Magic: {} casts Thrusting (line pierce 2)", state.name);
            }
            // --- 传送类 ---
            // Teleport：随机传送（C# MagicTeleport 选随机点）
            // Blink：定点传送，距离上限=Range，成功率=(level+1)/4
            SPELL_TELEPORT | SPELL_BLINK => {
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
                let (max_x, max_y) = self.maps.get(&state.map_index)
                    .map(|m| (m.width as i32, m.height as i32))
                    .unwrap_or((i32::MAX, i32::MAX));

                // Blink 专属：距离校验 + 成功率
                if msg.spell == SPELL_BLINK {
                    let dist = ((state.x - target_x).abs() + (state.y - target_y).abs()) as i32;
                    let range = spell_db.map(|m| m.range as i32).unwrap_or(10);
                    if dist > range {
                        send_system_message(&self.gate_ref, msg.session_id, "距离超出闪现范围");
                        return;
                    }
                    // 成功率 (level+1)/4：Random(4) >= level+1 则失败
                    if fastrand::i32(0..4) >= spell_level as i32 + 1 {
                        debug!("Magic: {} Blink failed (random miss)", state.name);
                        return;
                    }
                }

                let tx = target_x.clamp(0, max_x - 1);
                let ty = target_y.clamp(0, max_y - 1);
                let _ = record.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                    x: tx,
                    y: ty,
                    direction: msg.direction,
                    map_index: None,
                    is_mounted: None,
                }).await;
                debug!("Magic: {} teleports/blinks to ({}, {})", state.name, tx, ty);
            }
            // --- 弹道类法术（任务3）：FireBall/GreatFireBall/ThunderBolt/FrostCrunch/Vampirism ---
            // 对齐 C# HumanObject Fireball()/ThunderBolt()/Vampirism()：创建 DelayedAction，延迟后结算
            SPELL_FIREBALL | SPELL_GREAT_FIREBALL | SPELL_THUNDERBOLT
            | SPELL_FROST_CRUNCH | SPELL_VAMPIRISM => {
                let raw_damage = if let Some(info) = spell_db {
                    crate::combat::magic::calc_magic_damage(info, spell_level, magic_stat)
                } else {
                    fastrand::i32(5..=15)
                }.max(1);

                // 弹道延迟：FireBall 系 = 距离×50ms + 500ms；ThunderBolt/Vampirism = 固定 500ms
                let target_dist = ((state.x - target_x).abs() + (state.y - target_y).abs()) as u64;
                let delay_ms = match msg.spell {
                    SPELL_FIREBALL | SPELL_GREAT_FIREBALL | SPELL_FROST_CRUNCH => {
                        target_dist * 50 + 500
                    }
                    _ => 500, // ThunderBolt / Vampirism 固定 500ms
                };
                // tick_count 每 100ms +1，延迟按 100ms 取整（最少 1 tick）
                let fire_at_tick = self.tick_count + (delay_ms / 100).max(1);

                self.pending_spell_completions.push(PendingSpellCompletion {
                    fire_at_tick,
                    session_id: msg.session_id,
                    spell: msg.spell,
                    target_id: msg.target_id,
                    target_x,
                    target_y,
                    damage: raw_damage,
                    magic_stat,
                    spell_level,
                });
                debug!("Magic: {} casts projectile spell={} dmg={} delay={}ms (fires @tick {})",
                    state.name, msg.spell, raw_damage, delay_ms, fire_at_tick);
            }
            // --- 即时 AoE 类法术（任务4）---
            // FireBang/IceStorm：3×3 AoE，MAC 伤害（C# Map.cs:952）
            SPELL_FIREBANG | SPELL_ICE_STORM => {
                let raw_damage = if let Some(info) = spell_db {
                    crate::combat::magic::calc_magic_damage(info, spell_level, magic_stat)
                } else { fastrand::i32(5..=15) }.max(1);
                let attacker_stats = state.to_combat_stats();
                let level_offset = state.level.min(10) as u16;
                // 3×3：target 周围 ±1 格
                let hit_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| {
                        let dx = (m.x - target_x).abs();
                        let dy = (m.y - target_y).abs();
                        dx <= 1 && dy <= 1 && m.hp > 0
                    })
                    .map(|(id, _)| *id)
                    .collect();
                for mid in hit_ids {
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        let defender_stats = monster.to_combat_stats();
                        let r = combat_attack::resolve_attack(
                            &attacker_stats, &defender_stats, raw_damage,
                            mir2_shared::enums::DefenceType::Mac, level_offset,
                        );
                        if r.is_hit && r.damage > 0 {
                            monster.take_damage(r.damage);
                            monster.provoked = true;
                            monster.target_session = Some(msg.session_id);
                            for p in &r.applied_poisons {
                                crate::combat::poison::apply_poison(&mut monster.poison_list, *p);
                            }
                        }
                    }
                }
                debug!("Magic: {} casts FireBang/IceStorm (3x3) dmg={}", state.name, raw_damage);
            }
            // Lightning：直线 6 格，每格首目标，MAC（C# Map.cs:1189）
            SPELL_LIGHTNING => {
                let raw_damage = if let Some(info) = spell_db {
                    crate::combat::magic::calc_magic_damage(info, spell_level, magic_stat)
                } else { fastrand::i32(5..=15) }.max(1);
                let attacker_stats = state.to_combat_stats();
                let level_offset = state.level.min(10) as u16;
                let dir = msg.direction as usize % 8;
                let mut cx = state.x;
                let mut cy = state.y;
                for _ in 0..6 {
                    cx += MON_DIR_DX[dir];
                    cy += MON_DIR_DY[dir];
                    // 找该格第一个怪物
                    let hit = self.monsters.iter()
                        .find(|(_, m)| m.x == cx && m.y == cy && m.hp > 0)
                        .map(|(id, _)| *id);
                    if let Some(mid) = hit {
                        if let Some(monster) = self.monsters.get_mut(&mid) {
                            let defender_stats = monster.to_combat_stats();
                            let r = combat_attack::resolve_attack(
                                &attacker_stats, &defender_stats, raw_damage,
                                mir2_shared::enums::DefenceType::Mac, level_offset,
                            );
                            if r.is_hit && r.damage > 0 {
                                monster.take_damage(r.damage);
                                monster.provoked = true;
                                monster.target_session = Some(msg.session_id);
                                for p in &r.applied_poisons {
                                    crate::combat::poison::apply_poison(&mut monster.poison_list, *p);
                                }
                            }
                        }
                        // C# 每格 break（只打第一个），但外层 i 继续 → 每格各打第一个
                    }
                }
                debug!("Magic: {} casts Lightning (line 6) dmg={}", state.name, raw_damage);
            }
            // ThunderStorm/FlameField：5×5 自身周围，MAC（C# Map.cs:1303）
            // ThunderStorm 对非亡灵伤害 ×1/10（Rust 暂无 undead 字段，全额伤害，TODO）
            SPELL_THUNDERSTORM | SPELL_FLAME_FIELD => {
                let raw_damage = if let Some(info) = spell_db {
                    crate::combat::magic::calc_magic_damage(info, spell_level, magic_stat)
                } else { fastrand::i32(5..=15) }.max(1);
                let attacker_stats = state.to_combat_stats();
                let level_offset = state.level.min(10) as u16;
                let hit_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| {
                        let dx = (m.x - state.x).abs();
                        let dy = (m.y - state.y).abs();
                        dx <= 2 && dy <= 2 && m.hp > 0
                    })
                    .map(|(id, _)| *id)
                    .collect();
                for mid in hit_ids {
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        let defender_stats = monster.to_combat_stats();
                        // ThunderStorm 对非亡灵伤害 ×1/10（C# Map.cs:1332），FlameField 全额
                        let is_thunderstorm = msg.spell == SPELL_THUNDERSTORM;
                        let adjusted_dmg = if is_thunderstorm && !monster.undead {
                            raw_damage / 10
                        } else {
                            raw_damage
                        };
                        let r = combat_attack::resolve_attack(
                            &attacker_stats, &defender_stats, adjusted_dmg,
                            mir2_shared::enums::DefenceType::Mac, level_offset,
                        );
                        if r.is_hit && r.damage > 0 {
                            monster.take_damage(r.damage);
                            monster.provoked = true;
                            monster.target_session = Some(msg.session_id);
                            for p in &r.applied_poisons {
                                crate::combat::poison::apply_poison(&mut monster.poison_list, *p);
                            }
                        }
                    }
                }
                debug!("Magic: {} casts ThunderStorm/FlameField (5x5) dmg={}", state.name, raw_damage);
            }
            // ===== 弓箭手（Archer）弹道物理系法术 =====
            // StraightShot：单目标弹道，延迟 = 距离×50ms + 500ms，AC 防御（弓箭手物理）
            // DoubleShot：对目标连发 2 次弹道（第二次延迟 +200ms）
            // BindingShot：弹道 + 命中后 Paralysis（在 complete_projectile_spell 结算）
            // NapalmShot：弹道 + 命中后 3×3 AOE（在 complete_projectile_spell 结算）
            // 伤害基于 DC（物理攻击），用 magic_stat（弓箭手类 = effective_max_attack）
            SPELL_STRAIGHT_SHOT | SPELL_DOUBLE_SHOT | SPELL_BINDING_SHOT | SPELL_NAPALM_SHOT => {
                // 弓箭手弹道伤害：DC × 法术倍率（power_base 近似），最少 1
                let raw_damage = (magic_stat + (power as i32) / 2).max(1);

                // 弹道延迟：距离×50ms + 500ms
                let target_dist = ((state.x - target_x).abs() + (state.y - target_y).abs()) as u64;
                let base_delay_ms = target_dist * 50 + 500;
                // tick_count 每 100ms +1，按 100ms 取整（最少 1 tick）
                let fire_at_tick = self.tick_count + (base_delay_ms / 100).max(1);

                self.pending_spell_completions.push(PendingSpellCompletion {
                    fire_at_tick,
                    session_id: msg.session_id,
                    spell: msg.spell,
                    target_id: msg.target_id,
                    target_x,
                    target_y,
                    damage: raw_damage,
                    magic_stat,
                    spell_level,
                });

                // DoubleShot：额外发一发，延迟 +200ms（2 ticks）
                if msg.spell == SPELL_DOUBLE_SHOT {
                    self.pending_spell_completions.push(PendingSpellCompletion {
                        fire_at_tick: fire_at_tick + 2,
                        session_id: msg.session_id,
                        spell: msg.spell,
                        target_id: msg.target_id,
                        target_x,
                        target_y,
                        damage: raw_damage,
                        magic_stat,
                        spell_level,
                    });
                }
                debug!("Magic: {} casts Archer projectile spell={} dmg={} delay={}ms (DoubleShot={})",
                    state.name, msg.spell, raw_damage, base_delay_ms, msg.spell == SPELL_DOUBLE_SHOT);
            }
            // Concentration：自身 MP 恢复 buff（MpRegenBoost），持续 60s
            SPELL_CONCENTRATION => {
                let bonus = 3 + spell_level as i32 * 2;
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::MpRegenBoost { bonus },
                    600, // 60s = 600 ticks
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts Concentration (MP regen +{})", state.name, bonus);
            }
            // ElementalBarrier：自身减伤 buff（DamageReduction），持续 30s
            // 与 MagicShield 同机制（用 ApplyDamageReduction 设 PlayerState.damage_reduction_percent）
            SPELL_ELEMENTAL_BARRIER => {
                let reduction_pct = ((spell_level as i32 + 1) * 10).min(80);
                let duration_ticks = 300; // 30s = 300 ticks
                let _ = record.actor_ref.ask(crate::actors::player::ApplyDamageReduction {
                    percent: reduction_pct,
                    duration_ticks,
                }).await;
                debug!("Magic: {} casts ElementalBarrier (damage -{}%)", state.name, reduction_pct);
            }
            // Mirroring：自身反伤 buff（Reflect），持续 30s
            SPELL_MIRRORING => {
                let reflect_pct = 10 + spell_level as i32 * 5;
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::Reflect { percent: reflect_pct },
                    300, // 30s = 300 ticks
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts Mirroring (reflect {}%)", state.name, reflect_pct);
            }
            // ===== 刺客法术（Assassin，buff 系 + 位移系 + 物理攻击系）=====
            // Haste：攻击速度提升（降低攻击冷却，C# Stat.AttackSpeed）
            SPELL_HASTE => {
                let pct = 15 + spell_level as i32 * 10;
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::AttackSpeedBoost { percent: pct },
                    600, // 60s
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts Haste (attack speed +{}%)", state.name, pct);
            }
            // LightBody：敏捷+移动速度（C# Agility + MoveSpeed）
            SPELL_LIGHT_BODY => {
                let agi_bonus = 5 + spell_level as i32 * 3;
                let buff1 = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::AgilityBoost { bonus: agi_bonus }, 600, 5);
                let buff2 = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::MoveSpeedBoost { percent: 10 }, 600, 5);
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff: buff1 }).await;
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff: buff2 }).await;
                debug!("Magic: {} casts LightBody (agility +{}, speed +10%)", state.name, agi_bonus);
            }
            // Fury：攻击力提升（C# Stat.MinDC/MaxDC）
            SPELL_FURY => {
                let atk_bonus = (power / 3).max(5);
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::AttackBoost { bonus: atk_bonus }, 600, 5);
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts Fury (attack +{})", state.name, atk_bonus);
            }
            // Rage：暴击率提升（C# Stat.CriticalRate）
            SPELL_RAGE => {
                let crit_bonus = 3 + spell_level as i32 * 2;
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::CriticalRateBoost { bonus: crit_bonus }, 600, 5);
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts Rage (critical rate +{})", state.name, crit_bonus);
            }
            // SwiftFeet：移动速度大幅提升
            SPELL_SWIFT_FEET => {
                let spd_pct = 30 + spell_level as i32 * 10;
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::MoveSpeedBoost { percent: spd_pct }, 300, 5);
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts SwiftFeet (move speed +{}%)", state.name, spd_pct);
            }
            // MoonLight：隐身（刺客版，怪物失去目标）
            SPELL_MOON_LIGHT => {
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::Invisibility,
                    (30 + spell_level as u32 * 10) * 10, 5);
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                self.invisible_sessions.insert(msg.session_id);
                debug!("Magic: {} casts MoonLight (invisible)", state.name);
            }
            // DarkBody：隐身 + 攻击力（刺客终极隐身）
            SPELL_DARK_BODY => {
                let atk_bonus = (power / 4).max(3);
                let buff1 = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::Invisibility,
                    (20 + spell_level as u32 * 10) * 10, 5);
                let buff2 = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::AttackBoost { bonus: atk_bonus },
                    (20 + spell_level as u32 * 10) * 10, 5);
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff: buff1 }).await;
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff: buff2 }).await;
                self.invisible_sessions.insert(msg.session_id);
                debug!("Magic: {} casts DarkBody (invisible + attack {})", state.name, atk_bonus);
            }
            // HeavenlySword：直线 3 格 AoE（物理 AC 防御，类似 Thrusting 但更长）
            SPELL_HEAVENLY_SWORD => {
                let dir = msg.direction as usize % 8;
                let attacker_stats = state.to_combat_stats();
                let level_offset = state.level.min(10) as u16;
                let raw_damage = crate::combat::attack::get_attack_power(
                    attacker_stats.min_atk, attacker_stats.max_atk, attacker_stats.luck);
                let mut cx = state.x;
                let mut cy = state.y;
                let mut hit_ids: Vec<u32> = Vec::new();
                for _ in 0..3 {
                    cx += MON_DIR_DX[dir];
                    cy += MON_DIR_DY[dir];
                    if let Some((&mid, _)) = self.monsters.iter().find(|(_, m)| m.x == cx && m.y == cy && m.hp > 0) {
                        hit_ids.push(mid);
                    }
                }
                for mid in hit_ids {
                    if let Some(m) = self.monsters.get_mut(&mid) {
                        let ds = m.to_combat_stats();
                        let r = combat_attack::resolve_attack(
                            &attacker_stats, &ds, raw_damage,
                            mir2_shared::enums::DefenceType::AcAgility, level_offset);
                        if r.is_hit && r.damage > 0 {
                            m.take_damage(r.damage);
                            m.provoked = true;
                            m.target_session = Some(msg.session_id);
                            for p in &r.applied_poisons {
                                crate::combat::poison::apply_poison(&mut m.poison_list, *p);
                            }
                        }
                    }
                }
                debug!("Magic: {} casts HeavenlySword (line 3 AoE)", state.name);
            }
            // CrescentSlash：前方扇形 AoE（前+左前+右前 3 格）
            SPELL_CRESCENT_SLASH => {
                let dir = msg.direction as usize % 8;
                let attacker_stats = state.to_combat_stats();
                let level_offset = state.level.min(10) as u16;
                let raw_damage = crate::combat::attack::get_attack_power(
                    attacker_stats.min_atk, attacker_stats.max_atk, attacker_stats.luck);
                // 扇形：前方 dir + 左前 (dir+7)%8 + 右前 (dir+1)%8
                let fan_dirs = [dir, (dir + 7) % 8, (dir + 1) % 8];
                let mut hit_ids: Vec<u32> = Vec::new();
                for fd in fan_dirs {
                    let tx = state.x + MON_DIR_DX[fd];
                    let ty = state.y + MON_DIR_DY[fd];
                    if let Some((&mid, _)) = self.monsters.iter().find(|(_, m)| m.x == tx && m.y == ty && m.hp > 0) {
                        hit_ids.push(mid);
                    }
                }
                for mid in hit_ids {
                    if let Some(m) = self.monsters.get_mut(&mid) {
                        let ds = m.to_combat_stats();
                        let r = combat_attack::resolve_attack(
                            &attacker_stats, &ds, raw_damage,
                            mir2_shared::enums::DefenceType::AcAgility, level_offset);
                        if r.is_hit && r.damage > 0 {
                            m.take_damage(r.damage);
                            m.provoked = true;
                            m.target_session = Some(msg.session_id);
                        }
                    }
                }
                debug!("Magic: {} casts CrescentSlash (fan 3 AoE)", state.name);
            }
            // FlashDash：向前突进 4 格（纯位移，成功率 (level+1)/4）
            SPELL_FLASH_DASH => {
                if fastrand::i32(0..4) >= spell_level as i32 + 1 {
                    debug!("Magic: {} FlashDash failed (random)", state.name);
                    // 失败仍消耗 MP，不 return（继续走 XP 流程）
                } else {
                    let dir = msg.direction as usize % 8;
                    let (max_x, max_y) = self.maps.get(&state.map_index)
                        .map(|m| (m.width as i32, m.height as i32))
                        .unwrap_or((i32::MAX, i32::MAX));
                    let tx = (state.x + MON_DIR_DX[dir] * 4).clamp(0, max_x - 1);
                    let ty = (state.y + MON_DIR_DY[dir] * 4).clamp(0, max_y - 1);
                    let _ = record.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                        x: tx, y: ty, direction: msg.direction,
                        map_index: None, is_mounted: None,
                    }).await;
                    debug!("Magic: {} casts FlashDash to ({},{})", state.name, tx, ty);
                }
            }
            // BackStep：向后跳跃 3 格（direction 相反方向）
            SPELL_BACK_STEP => {
                let dir = msg.direction as usize % 8;
                let back_dir = (dir + 4) % 8; // 反方向
                let (max_x, max_y) = self.maps.get(&state.map_index)
                    .map(|m| (m.width as i32, m.height as i32))
                    .unwrap_or((i32::MAX, i32::MAX));
                let tx = (state.x + MON_DIR_DX[back_dir] * 3).clamp(0, max_x - 1);
                let ty = (state.y + MON_DIR_DY[back_dir] * 3).clamp(0, max_y - 1);
                let _ = record.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                    x: tx, y: ty, direction: back_dir as u8,
                    map_index: None, is_mounted: None,
                }).await;
                debug!("Magic: {} casts BackStep to ({},{})", state.name, tx, ty);
            }
            // --- 召唤系法术（道士/法师/弓箭手）：在施法者前方 1 格 spawn 一只战斗召唤物 ---
            SPELL_SUMMON_SKELETON | SPELL_SUMMON_SHINSU | SPELL_SUMMON_HOLY_DEVA
            | SPELL_SUMMON_VAMPIRE | SPELL_SUMMON_TOAD | SPELL_SUMMON_SNAKES => {
                // 召唤物名映射（对齐 C# HumanObject.SummonXxx，名需在 DB monster_infos 里）
                let summon_name: &str = match msg.spell {
                    SPELL_SUMMON_SKELETON => "Skeleton",
                    SPELL_SUMMON_SHINSU => "Shinsu",
                    SPELL_SUMMON_HOLY_DEVA => "HolyDeva",
                    SPELL_SUMMON_VAMPIRE => "Vampire",
                    SPELL_SUMMON_TOAD => "Toad",
                    SPELL_SUMMON_SNAKES => "Snakes",
                    _ => unreachable!(),
                };
                let (max_x, max_y) = self.maps.get(&state.map_index)
                    .map(|m| (m.width as i32, m.height as i32))
                    .unwrap_or((i32::MAX, i32::MAX));
                let dir = msg.direction as usize % 8;
                // 召唤物生成在施法者前方 1 格（对齐 C# target point）
                let sx = (state.x + MON_DIR_DX[dir]).clamp(0, max_x - 1);
                let sy = (state.y + MON_DIR_DY[dir]).clamp(0, max_y - 1);

                // 限制同一主人同时拥有的召唤物数量（C# 默认 1，高级技能 2）
                let max_slaves = if msg.spell == SPELL_SUMMON_SHINSU
                    || msg.spell == SPELL_SUMMON_SNAKES { 2 } else { 1 };
                let current_slaves = self.monsters.values()
                    .filter(|m| m.master_session == Some(msg.session_id))
                    .count();
                if current_slaves >= max_slaves {
                    // 已达上限：移除最早的召唤物（按 object_id 最小者）
                    if let Some(victim_id) = self.monsters.iter()
                        .filter(|(_, m)| m.master_session == Some(msg.session_id))
                        .map(|(id, _)| *id)
                        .min() {
                        if self.monsters.remove(&victim_id).is_some() {
                            let rm_pkt = Self::build_object_remove_packet(victim_id);
                            for sid in self.players.keys() {
                                let _ = self.gate_ref.ask(SendToClient {
                                    session_id: *sid,
                                    data: rm_pkt.clone(),
                                });
                            }
                            debug!("Magic: {} reached slave cap, recalled monster {}",
                                state.name, victim_id);
                        }
                    }
                }

                // 按 monster_name_index 查 MonsterInfo（lowercase key，对齐 tick.rs boss_summons）
                let mon_index = self.monster_name_index.get(&summon_name.to_lowercase()).copied();
                match mon_index {
                    Some(idx) => {
                        // 先 clone MonsterInfo 避免 &self.monster_infos 与 &mut self.alloc_object_id 借用冲突
                        let info_opt = self.monster_infos.get(&idx).cloned();
                        if let Some(info) = info_opt {
                            let new_oid = self.alloc_object_id();
                            let hp = info.stats.get(&(mir2_shared::enums::Stat::HP as u8)).copied().unwrap_or(50);
                            let min_dmg = info.stats.get(&(mir2_shared::enums::Stat::MinDC as u8)).copied().unwrap_or(5);
                            let max_dmg = info.stats.get(&(mir2_shared::enums::Stat::MaxDC as u8)).copied().unwrap_or(10);
                            // 广播 ObjectMonster 给所有玩家（spawn 通知）
                            let spawn = MonsterSpawn {
                                name: info.name.clone(),
                                image: info.image as u16,
                                monster_index: idx,
                                x: sx,
                                y: sy,
                                direction: dir as u8,
                                hp,
                                min_dmg,
                                max_dmg,
                                xp: info.experience,
                                map_index: state.map_index,
                            };
                            let packet = build_object_monster_packet(&spawn, new_oid, &spawn.name);
                            for session_id in self.players.keys() {
                                let _ = self.gate_ref.ask(SendToClient {
                                    session_id: *session_id,
                                    data: packet.clone(),
                                });
                            }
                            let ai_profile = MonsterAiProfile::from_info(&info);
                            // 召唤物：target_session=主人、provoked=true 主动攻击
                            self.monsters.insert(new_oid, MonsterState {
                                object_id: new_oid,
                                name: spawn.name.clone(),
                                image: spawn.image,
                                monster_index: idx,
                                x: sx, y: sy, direction: dir as u8,
                                hp, max_hp: hp, min_dmg, max_dmg, xp: spawn.xp,
                                spawn_x: sx, spawn_y: sy, map_index: state.map_index,
                                next_attack_tick: 0, next_move_tick: 0, next_summon_tick: 0,
                                ai_profile, ai_state: MonsterAiState::Idle,
                                target_session: Some(msg.session_id), provoked: true,
                                is_elite: false, is_boss: false,
                                min_ac: 0, max_ac: 0, min_mac: 0, max_mac: 0,
                                agility: 0, accuracy: 0,
                                armour_rate: 1.0, damage_rate: 1.0,
                                magic_resist: 0, critical_rate: 0, critical_damage: 0,
                                luck: 0, reflect: 0, damage_reduction_percent: 0,
                                poison_list: Vec::new(),
                                undead: info.undead,
                                master_session: Some(msg.session_id),
                                recall_at_tick: self.tick_count + 6000,
                                behavior: crate::actors::world::ai::make_behavior(&spawn.name),
                            });
                            debug!("Magic: {} casts summon '{}' as #{} at ({},{}) (slave of {})",
                                state.name, summon_name, new_oid, sx, sy, msg.session_id);
                        } else {
                            warn!("Summon '{}' found index {} but no MonsterInfo (DB missing mob)",
                                summon_name, idx);
                            send_system_message(&self.gate_ref, msg.session_id,
                                "召唤失败：怪物资料缺失");
                        }
                    }
                    None => {
                        warn!("Summon '{}' not in monster_name_index (DB may lack this mob)", summon_name);
                        send_system_message(&self.gate_ref, msg.session_id, "召唤失败：未知怪物");
                    }
                }
            }
            // ===== 特殊/辅助类法术（任务：补齐剩余主动法术）=====
            // --- 战士系 ---
            // LionRoar：嘲讽范围内怪物（吸引仇恨，对齐 C# WarriorObject.LionRoar）
            // 范围 = Range（默认 5 格），命中怪物 provoked + target_session=施法者
            SPELL_LION_ROAR => {
                let range = spell_range.max(3);
                let hit_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| {
                        let dist = (m.x - state.x).abs() + (m.y - state.y).abs();
                        dist <= range && m.hp > 0 && m.master_session.is_none()
                    })
                    .map(|(id, _)| *id)
                    .collect();
                let count = hit_ids.len();
                for mid in hit_ids {
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        monster.provoked = true;
                        monster.target_session = Some(msg.session_id);
                        // 嘲讽 buff（简化：标记仇恨，无数值）
                        let buff = crate::combat::buff::BuffInstance::new(
                            crate::combat::buff::BuffType::Taunt, 300, 5);
                        let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                    }
                }
                debug!("Magic: {} casts LionRoar (taunted {} monsters)", state.name, count);
            }
            // ProtectionField：群体减伤（自身 + 附近组员，对齐 C# WarriorObject.ProtectionField）
            // 简化：自身 + 3 格内同组玩家获得 DamageReduction buff
            SPELL_PROTECTION_FIELD => {
                let reduction_pct = ((spell_level as i32 + 1) * 10).min(50);
                let duration_ticks = (30 + spell_level as u32 * 10) * 10; // 30-60s
                let group_id = state.group_id;
                // 自身
                let _ = record.actor_ref.ask(crate::actors::player::ApplyDamageReduction {
                    percent: reduction_pct, duration_ticks,
                }).await;
                let mut protected = 1u32;
                // 附近组员
                if let Some(gid) = group_id {
                    for (sid, other) in &self.players {
                        if *sid == msg.session_id { continue; }
                        if let Ok(Some(s)) = other.actor_ref.ask(GetPlayerState).await {
                            if s.group_id == Some(gid) && !s.is_dead {
                                let dist = (s.x - state.x).abs() + (s.y - state.y).abs();
                                if dist <= 3 {
                                    let _ = other.actor_ref.ask(crate::actors::player::ApplyDamageReduction {
                                        percent: reduction_pct, duration_ticks,
                                    }).await;
                                    protected += 1;
                                }
                            }
                        }
                    }
                }
                debug!("Magic: {} casts ProtectionField (protected {} players, -{}%)",
                    state.name, protected, reduction_pct);
            }
            // CounterAttack：反击 buff（对齐 C# Stat.CounterAttack，受击时反弹伤害）
            // 简化：用 Reflect buff 近似（反伤百分比）
            SPELL_COUNTER_ATTACK => {
                let reflect_pct = 15 + spell_level as i32 * 10;
                let duration_ticks = (15 + spell_level as u32 * 5) * 10; // 15-30s
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::Reflect { percent: reflect_pct },
                    duration_ticks, 5);
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts CounterAttack (reflect {}% for {}s)",
                    state.name, reflect_pct, duration_ticks / 10);
            }
            // Entrapment：拉拽目标到自身附近 + 麻痹（对齐 C# AssassinObject/Warrior Entrapment）
            // 简化：将目标格子怪物移到施法者前方 1 格，并施加 Paralysis
            SPELL_ENTRAPMENT => {
                let dir = msg.direction as usize % 8;
                let pull_x = state.x + MON_DIR_DX[dir];
                let pull_y = state.y + MON_DIR_DY[dir];
                // 找目标位置怪物（target_x/target_y 或前方 1 格）
                let target_mid: Option<u32> = self.monsters.iter()
                    .find(|(_, m)| {
                        (m.x == target_x && m.y == target_y && m.hp > 0)
                            || (m.x == pull_x && m.y == pull_y && m.hp > 0)
                    })
                    .map(|(id, _)| *id);
                if let Some(mid) = target_mid {
                    let walkable = self.maps.get(&state.map_index)
                        .map(|m| m.is_walkable(pull_x, pull_y))
                        .unwrap_or(true);
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        if walkable {
                            monster.x = pull_x;
                            monster.y = pull_y;
                            monster.direction = ((dir + 4) % 8) as u8; // 朝向施法者
                        }
                        // 麻痹 2-5 秒
                        let para_dur = (2 + spell_level as u32).min(5);
                        crate::combat::poison::apply_poison(&mut monster.poison_list,
                            crate::combat::poison::Poison::new(
                                mir2_shared::enums::PoisonType::PARALYSIS, para_dur, 0, 1000));
                        monster.provoked = true;
                        monster.target_session = Some(msg.session_id);
                        // 广播移动
                        let mut walk_body = Vec::new();
                        walk_body.extend_from_slice(&mid.to_le_bytes());
                        walk_body.extend_from_slice(&monster.x.to_le_bytes());
                        walk_body.extend_from_slice(&monster.y.to_le_bytes());
                        walk_body.push(monster.direction);
                        let walk_packet = build_packet_bytes(
                            mir2_shared::enums::ServerPacketIds::ObjectWalk as i16, &walk_body);
                        for session_id in self.players.keys() {
                            let _ = self.gate_ref.ask(SendToClient {
                                session_id: *session_id,
                                data: walk_packet.clone(),
                            });
                        }
                        debug!("Magic: {} casts Entrapment (pulled monster {} paralysis {}s)",
                            state.name, mid, para_dur);
                    }
                }
            }
            // --- 法师系 ---
            // TurnUndead：秒杀低级亡灵（对齐 C# WizardObject.TurnUndead）
            // 命中目标格子亡灵怪物，按等级差概率秒杀（hp=0）
            SPELL_TURN_UNDEAD => {
                // 目标格子的亡灵怪物
                let hit_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| {
                        let dist = (m.x - target_x).abs() + (m.y - target_y).abs();
                        dist <= spell_range.max(1) && m.hp > 0 && m.undead
                    })
                    .map(|(id, _)| *id)
                    .collect();
                let mut killed = 0u32;
                for mid in hit_ids {
                    // 查 MonsterInfo.level 用于等级差判定
                    let mon_level = self.monsters.get(&mid)
                        .and_then(|m| self.monster_infos.get(&m.monster_index))
                        .map(|i| i.level).unwrap_or(0);
                    // 等级差：玩家等级越高，秒杀概率越大
                    // C# 概率近似：基础 30% + 等级差*10%，封顶 90%
                    let level_diff = (state.level as i32 - mon_level).max(0);
                    let chance = (30 + level_diff * 10).min(90);
                    if fastrand::i32(0..100) < chance {
                        if let Some(monster) = self.monsters.get_mut(&mid) {
                            monster.hp = 0;
                            monster.provoked = true;
                            monster.target_session = Some(msg.session_id);
                            killed += 1;
                        }
                    }
                }
                debug!("Magic: {} casts TurnUndead (killed {} undead)", state.name, killed);
            }
            // Repulsion：推开周围怪物（对齐 C# WizardObject.Repulsion）
            // 命中 1-2 格内怪物，将其沿反方向推 1-2 格（受 can_push 限制）
            SPELL_REPULSION => {
                let push_range = (1 + spell_level as i32 / 2).min(2); // Lv0=1, Lv2+=2
                // 收集 (怪物id, 推动方向) —— 方向 = 怪物相对施法者
                let mut pushes: Vec<(u32, usize)> = Vec::new();
                for (id, m) in self.monsters.iter() {
                    if m.hp <= 0 || m.master_session.is_some() { continue; }
                    let dx = m.x - state.x;
                    let dy = m.y - state.y;
                    let dist = dx.abs() + dy.abs();
                    if dist == 0 || dist > 2 { continue; }
                    // 推动方向：取 8 方向中最接近 (dx,dy) 的
                    let push_dir = best_dir(dx, dy);
                    pushes.push((*id, push_dir));
                }
                let (max_x, max_y) = self.maps.get(&state.map_index)
                    .map(|m| (m.width as i32, m.height as i32))
                    .unwrap_or((i32::MAX, i32::MAX));
                // 预取每只候选怪物的当前位置 + can_push（避免后续 &self.monsters 与 &mut 冲突）
                let mut candidates: Vec<(u32, usize, i32, i32)> = Vec::new(); // (id, dir, x, y)
                for (mid, pdir) in pushes {
                    let can_push = self.monsters.get(&mid)
                        .and_then(|m| self.monster_infos.get(&m.monster_index))
                        .map(|i| i.can_push).unwrap_or(true);
                    if !can_push { continue; }
                    if let Some(m) = self.monsters.get(&mid) {
                        candidates.push((mid, pdir, m.x, m.y));
                    }
                }
                // 被占用格子集合（用于阻挡判定），随移动动态更新
                let mut occupied: std::collections::HashSet<(i32, i32)> = self.monsters.values()
                    .filter(|m| m.hp > 0).map(|m| (m.x, m.y)).collect();
                let mut moved_packets: Vec<(u32, i32, i32, u8)> = Vec::new();
                for (mid, pdir, start_x, start_y) in candidates {
                    let mut nx = start_x;
                    let mut ny = start_y;
                    for _ in 0..push_range {
                        let tx = nx + MON_DIR_DX[pdir];
                        let ty = ny + MON_DIR_DY[pdir];
                        if tx < 0 || ty < 0 || tx >= max_x || ty >= max_y { break; }
                        let walkable = self.maps.get(&state.map_index)
                            .map(|m| m.is_walkable(tx, ty)).unwrap_or(true);
                        if !walkable { break; }
                        // 不能推到其他怪物身上（动态占用表）
                        if occupied.contains(&(tx, ty)) { break; }
                        nx = tx; ny = ty;
                    }
                    if nx != start_x || ny != start_y {
                        // 更新占用表：释放旧格、占用新格
                        occupied.remove(&(start_x, start_y));
                        occupied.insert((nx, ny));
                        if let Some(monster) = self.monsters.get_mut(&mid) {
                            monster.x = nx;
                            monster.y = ny;
                            monster.direction = ((pdir + 4) % 8) as u8; // 朝向施法者
                            monster.provoked = true;
                            monster.target_session = Some(msg.session_id);
                            moved_packets.push((mid, nx, ny, monster.direction));
                        }
                    }
                }
                // 广播被推动怪物的移动
                for (mid, mx, my, mdir) in moved_packets {
                    let mut walk_body = Vec::new();
                    walk_body.extend_from_slice(&mid.to_le_bytes());
                    walk_body.extend_from_slice(&mx.to_le_bytes());
                    walk_body.extend_from_slice(&my.to_le_bytes());
                    walk_body.push(mdir);
                    let walk_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectWalk as i16, &walk_body);
                    for session_id in self.players.keys() {
                        let _ = self.gate_ref.ask(SendToClient {
                            session_id: *session_id,
                            data: walk_packet.clone(),
                        });
                    }
                }
                debug!("Magic: {} casts Repulsion", state.name);
            }
            // ElectricShock：驯服怪物（对齐 C# WizardObject.ElectricShock）
            // 概率将目标怪物变为召唤物（master_session=施法者），受 can_tame 限制
            SPELL_ELECTRIC_SHOCK => {
                let target_mid: Option<u32> = self.monsters.iter()
                    .find(|(_, m)| {
                        let dist = (m.x - target_x).abs() + (m.y - target_y).abs();
                        dist <= 1 && m.hp > 0 && m.master_session.is_none()
                    })
                    .map(|(id, _)| *id);
                if let Some(mid) = target_mid {
                    let can_tame = self.monsters.get(&mid)
                        .and_then(|m| self.monster_infos.get(&m.monster_index))
                        .map(|i| i.can_tame).unwrap_or(false);
                    if can_tame {
                        // 成功率：基础 20% + 法术等级*15%，封顶 80%
                        let chance = (20 + spell_level as i32 * 15).min(80);
                        if fastrand::i32(0..100) < chance {
                            if let Some(monster) = self.monsters.get_mut(&mid) {
                                monster.master_session = Some(msg.session_id);
                                monster.target_session = None;
                                monster.provoked = false;
                                monster.recall_at_tick = self.tick_count + 12000; // 20 分钟后消失
                                debug!("Magic: {} casts ElectricShock (tamed monster {})", state.name, mid);
                                send_system_message(&self.gate_ref, msg.session_id, "驯服成功！");
                            }
                        } else {
                            // 失败时激怒怪物
                            if let Some(monster) = self.monsters.get_mut(&mid) {
                                monster.provoked = true;
                                monster.target_session = Some(msg.session_id);
                            }
                            debug!("Magic: {} ElectricShock failed on monster {}", state.name, mid);
                        }
                    } else {
                        debug!("Magic: {} ElectricShock: monster {} not tamable", state.name, mid);
                    }
                }
            }
            // MagicBooster：MP 上限提升 buff（对齐 C# Stat.MaxMP）
            SPELL_MAGIC_BOOSTER => {
                let bonus = (power / 2).max(20);
                let duration_ticks = (60 + spell_level as u32 * 15) * 10; // 60-105s
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::MaxMpBoost { bonus },
                    duration_ticks, 5);
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts MagicBooster (MaxMP +{})", state.name, bonus);
            }
            // --- 道士系 ---
            // Revelation：显血/反隐（对齐 C# TaoistObject.Revelation）
            // 简化：移除范围内敌方玩家隐身 + 标记自身可看见隐身单位（持续 buff）
            SPELL_REVELATION => {
                let reveal_range = spell_range.max(3).min(8);
                let duration_ticks = (30 + spell_level as u32 * 10) * 10; // 30-60s
                // 自身获得反隐 buff（用 Invisibility 标记自身可见隐身不可行，此处用 Reflect 占位）
                // 实际效果：移除附近敌方隐身玩家
                let mut revealed: Vec<u64> = Vec::new();
                for (sid, other) in &self.players {
                    if *sid == msg.session_id { continue; }
                    if self.invisible_sessions.contains(sid) {
                        if let Ok(Some(s)) = other.actor_ref.ask(GetPlayerState).await {
                            if !s.is_dead && s.map_index == state.map_index {
                                let dist = (s.x - state.x).abs() + (s.y - state.y).abs();
                                if dist <= reveal_range {
                                    revealed.push(*sid);
                                }
                            }
                        }
                    }
                }
                for sid in &revealed {
                    self.invisible_sessions.remove(sid);
                    if let Some(other) = self.players.get(sid) {
                        let _ = other.actor_ref.ask(crate::actors::player::RemoveBuff {
                            buff_type: crate::combat::buff::BuffType::Invisibility,
                        }).await;
                    }
                }
                // 给自身一个占位 buff 记录持续时间（反隐能力）
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::Reflect { percent: 0 }, duration_ticks, 5);
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts Revelation (revealed {} hidden players)",
                    state.name, revealed.len());
            }
            // Reincarnation：复活死亡玩家（对齐 C# TaoistObject.Reincarnation）
            // 简化：找附近（3格内）死亡玩家，原地半血复活
            SPELL_REINCARNATION => {
                let revive_range = 3;
                // 从 player_death_queue 找附近死亡玩家
                let mut target_dead: Option<u64> = None;
                for sid in self.player_death_queue.keys() {
                    if *sid == msg.session_id { continue; }
                    if let Some(other) = self.players.get(sid) {
                        if let Ok(Some(s)) = other.actor_ref.ask(GetPlayerState).await {
                            if s.is_dead && s.map_index == state.map_index {
                                let dist = (s.x - state.x).abs() + (s.y - state.y).abs();
                                if dist <= revive_range {
                                    target_dead = Some(*sid);
                                    break;
                                }
                            }
                        }
                    }
                }
                if let Some(dead_sid) = target_dead {
                    if let Some(other) = self.players.get(&dead_sid) {
                        // 获取死亡玩家状态用于位置
                        if let Ok(Some(dead_state)) = other.actor_ref.ask(GetPlayerState).await {
                            let rx = dead_state.x;
                            let ry = dead_state.y;
                            let rmap = dead_state.map_index;
                            // 原地复活（半血），对齐 C# Reincarnation Revive(HP/2)
                            let revived = other.actor_ref.ask(crate::actors::player::RevivePlayer {
                                x: rx, y: ry, map_index: rmap,
                            }).await.unwrap_or(false);
                            if revived {
                                // 从死亡队列移除（避免自动复活覆盖）
                                self.player_death_queue.remove(&dead_sid);
                                debug!("Magic: {} casts Reincarnation (revived player {})",
                                    state.name, dead_sid);
                                send_system_message(&self.gate_ref, msg.session_id, "轮回术成功，玩家已复活！");
                                send_system_message(&self.gate_ref, dead_sid, "你被轮回术复活了！");
                            }
                        }
                    }
                } else {
                    send_system_message(&self.gate_ref, msg.session_id, "附近没有可复活的目标");
                    debug!("Magic: {} casts Reincarnation but no target", state.name);
                }
            }
            // --- 刺客系 ---
            // PoisonSword：武器涂毒 buff（对齐 C# AssassinObject.PoisonSword）
            // 简化：用 AttackBoost + 占位记录（攻击触发由 attack.rs 检测 buff 实现）
            // 此处给自身一个短时攻击 buff（数值=绿毒强度近似）
            SPELL_POISON_SWORD => {
                let poison_value = (magic_stat / 6).max(3).min(15);
                let duration_ticks = (30 + spell_level as u32 * 10) * 10; // 30-60s
                // 攻击力小幅提升 + 记录涂毒状态（用 Reflect percent=0 占位标记，attack.rs 可检测）
                let buff1 = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::AttackBoost { bonus: poison_value / 2 },
                    duration_ticks, 5);
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff: buff1 }).await;
                debug!("Magic: {} casts PoisonSword (attack +{}, poison {} ready)",
                    state.name, poison_value / 2, poison_value);
            }
            // --- 默认：其他伤害类（接入战斗公式 MAC）---
            _ => {
                let raw_damage = if let Some(info) = spell_db {
                    crate::combat::magic::calc_magic_damage(info, spell_level, magic_stat)
                } else {
                    fastrand::i32(5..=15)
                }.max(1);
                let attacker_stats = state.to_combat_stats();
                let level_offset = state.level.min(10) as u16;
                let hit_monster_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| {
                        let dist = (m.x - target_x).abs() + (m.y - target_y).abs();
                        dist <= spell_range && m.hp > 0
                    })
                    .map(|(id, _)| *id)
                    .collect();

                for monster_id in hit_monster_ids {
                    if let Some(monster) = self.monsters.get_mut(&monster_id) {
                        let defender_stats = monster.to_combat_stats();
                        let r = combat_attack::resolve_attack(
                            &attacker_stats, &defender_stats, raw_damage,
                            mir2_shared::enums::DefenceType::Mac, level_offset,
                        );
                        if r.is_hit && r.damage > 0 {
                            monster.take_damage(r.damage);
                            monster.provoked = true;
                            monster.target_session = Some(msg.session_id);
                            for p in &r.applied_poisons {
                                crate::combat::poison::apply_poison(&mut monster.poison_list, *p);
                            }
                        }
                        debug!("Magic: {} spell={} lv={} -> monster {} for {} dmg (crit={})",
                            state.name, msg.spell, spell_level, monster_id, r.damage, r.is_critical);
                    }
                }
            }
        }

        // Spell XP gain and cast_time update
        if !basic_spells.contains(&msg.spell) {
            let _ = record.actor_ref.ask(crate::actors::player::GainSpellExp {
                spell: msg.spell,
                amount: 1,
                cast_time: now_ms,
            }).await;
        }
    }
}

/// 取最接近位移向量 (dx, dy) 的 8 方向索引（对齐 MON_DIR_DX/MON_DIR_DY）
/// 用于 Repulsion 等推开/弹射效果的推动方向计算
fn best_dir(dx: i32, dy: i32) -> usize {
    let mut best = 4usize; // 默认朝下（索引 4）
    let mut best_score = i64::MIN;
    for dir in 0..8usize {
        let sx = MON_DIR_DX[dir] as i64;
        let sy = MON_DIR_DY[dir] as i64;
        // 点积越大表示方向越一致
        let score = sx * dx as i64 + sy * dy as i64;
        if score > best_score {
            best_score = score;
            best = dir;
        }
    }
    best
}
