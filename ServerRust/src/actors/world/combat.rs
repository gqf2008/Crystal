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

            // #77 诊断：攻击时打印玩家/目标格与附近怪物，核对客户端-服务端坐标同步
            debug!(
                "Attack {} at ({},{}) dir={} target=({},{})",
                state.name, state.x, state.y, result.direction, target_x, target_y
            );
            let nearby: Vec<String> = self
                .monsters
                .iter()
                .filter(|(_, m)| (m.x - state.x).abs() <= 5 && (m.y - state.y).abs() <= 5)
                .map(|(id, m)| format!("{}#{}@({},{}) hp={}", m.name, id, m.x, m.y, m.hp))
                .collect();
            if !nearby.is_empty() {
                debug!("Attack nearby: {}", nearby.join(", "));
            }

            // #471：主人当前召唤的宠物（协战目标分配用）
            let pet_ids: Vec<u32> = self.monsters.iter()
                .filter(|(_, m)| m.master_session == Some(msg.session_id))
                .map(|(id, _)| *id)
                .collect();
            let mut hit_monster = false;
            // HalfMoon/CrossHalfMoon 溅射目标（循环外应用，避免借用冲突）
            let mut halfmoon_splash: Vec<(u32, i32)> = Vec::new();
            // C# 弧/十字几何命中的格子（围绕玩家）
            let mut halfmoon_cells: Vec<(i32, i32)> = Vec::new();
            let mut primary_target_oid: u32 = 0; // 主目标 oid（溅射排除用）
            for (oid, monster) in &mut self.monsters {
                let dist = (monster.x - target_x).abs() + (monster.y - target_y).abs();
                // #471：主人近战不攻击自己的召唤宠物（宠物是友方）
                if monster.master_session == Some(msg.session_id) {
                    continue;
                }
                // 近战只打正前方那一格（C# 语义）：dist==0 才命中。此前 <=1 会把
                // 攻击格旁边的守卫/怪物一并命中（#77 实测守卫被打死都不掉血挡住击杀）
                if dist == 0 {
                    // 命中怪物 - 使用完整战斗公式（命中/护甲/暴击/反伤/吸血/负面）
                    let attacker_stats = state.to_combat_stats();
                    let defender_stats = monster.to_combat_stats();
                    let mut raw_damage = combat_attack::get_attack_power(
                        attacker_stats.min_atk, attacker_stats.max_atk, attacker_stats.luck,
                    );
                    // C# Hemorrhage：武装状态（下次命中触发）时触发击伤害 = base × (0.2+0.05Lv)
                    let hemorrhage_armed = self.hemorrhage_armed.remove(&msg.session_id);
                    if hemorrhage_armed {
                        if let Some(magic) = state.magics.iter().find(|m| m.spell == (SPELL_HEMORRHAGE as i32 - 3)) {
                            let lv = magic.level as i32;
                            raw_damage = ((raw_damage as f32) * (0.2 + 0.05 * lv as f32)).max(1.0) as i32;
                        }
                    }
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
                    // Slaying（攻杀）：C# Envir.cs 无倍率配置 → GetDamage = base×1.0（无额外伤害，仅技能经验）
                    let mut slaying_bonus = 0i32;
                    if let Some(lv) = state.magics.iter().find(|m| m.spell == SPELL_SLAYING as i32).map(|m| m.level) {
                        // 概率：level/5（C# 攻杀触发率与等级相关）
                        if fastrand::i32(0..5) < lv as i32 {
                            debug!("Player {} Slaying triggered (level {})", result.object_id, lv);
                        }
                    }
                    // #312：FlamingSword —— C# Envir.cs MultiplierBase=1.4（无等级加成）：单次 1.4×
                    // Rust 主击已按 base 结算，此处追加 0.4× 近似合计 1.4×（防御只算一次）
                    let mut flaming_bonus = 0i32;
                    if let Some((expire, lv)) = self.flaming_sword.get(&msg.session_id).copied() {
                        self.flaming_sword.remove(&msg.session_id);
                        if self.tick_count < expire {
                            // C# Envir.cs FlamingSword：1.4+0.4Lv 单次（主击已计 base，追加 0.4+0.4Lv）
                            flaming_bonus = (damage as f32 * (0.4 + 0.4 * lv as f32)) as i32;
                            monster.take_damage(flaming_bonus);
                            debug!("Player {} FlamingSword bonus +{} on '{}' (#{})",
                                   result.object_id, flaming_bonus, monster.name, *oid);
                        }
                    }
                    // #318：TwinDrakeBlade/DoubleSlash —— 下一次近战攻击双段伤害（C# MultiplierBase=0.8/Bonus=0.1，一次性）
                    let mut second_hit = 0i32;
                    if let Some((expire, lv, kind)) = self.double_hit_melee.get(&msg.session_id).copied() {
                        self.double_hit_melee.remove(&msg.session_id);
                        if self.tick_count < expire {
                            second_hit = (damage as f32 * (0.8 + 0.1 * lv as f32)) as i32;
                            monster.take_damage(second_hit);
                            let label = if kind == 0 { "TwinDrakeBlade" } else { "DoubleSlash" };
                            debug!("Player {} {} second hit +{} on '{}' (#{})",
                                   result.object_id, label, second_hit, monster.name, *oid);
                        }
                    }
                    // #448：FatalSword —— 被动：每次近战 10% 概率触发，下一击 +5*(Lv+1) 平伤
                    // （C# HumanObject.cs:3063 触发 / 6789 消费；defence=Agility 由 resolve 阶段近似）
                    let fatal_armed = self.fatal_sword_armed.remove(&msg.session_id);
                    if let Some(magic) = state.magics.iter().find(|m| m.spell == (SPELL_FATAL_SWORD as i32 - 3)) {
                        if !fatal_armed && fastrand::i32(0..10) == 0 {
                            self.fatal_sword_armed.insert(msg.session_id);
                            debug!("Player {} FatalSword armed", result.object_id);
                        }
                        if fatal_armed {
                            let fatal_bonus = 5 * (magic.level as i32 + 1); // C# GetPower = (MPowerBase 20/4)*(Lv+1)
                            monster.take_damage(fatal_bonus);
                            debug!("Player {} FatalSword bonus +{} on '{}' (#{})",
                                   result.object_id, fatal_bonus, monster.name, *oid);
                        }
                    }
                    // #345：MPEater —— 近战被动吸蓝（C# HumanObject.cs:3078）
                    if let Some(magic) = state.magics.iter().find(|m| m.spell == (SPELL_MPEATER as i32 - 3)) {
                        let lv = magic.level as i32;
                        let acc = state.accuracy;
                        let base_count = 1 + acc / 2;
                        let max_count = base_count + lv * 5;
                        let add = fastrand::i32(base_count..=(max_count.max(base_count)));
                        let count = self.mp_eater_count.entry(msg.session_id).or_insert(0);
                        *count += add;
                        debug!("Player {} MPEater count={} (add={})", result.object_id, *count, add);
                        if *count >= 100 {
                            let add_mp = mp_eater_restore(lv, acc);
                            let _ = record.actor_ref.ask(crate::actors::player::AddMP { amount: add_mp }).await;
                            *count = 0;
                            debug!("Player {} MPEater restored {} MP", result.object_id, add_mp);
                        }
                    }
                    // #345：Hemorrhage —— 近战被动放血（C# HumanObject.cs:3110：count>=55 武装，下次命中触发）
                    if let Some(magic) = state.magics.iter().find(|m| m.spell == (SPELL_HEMORRHAGE as i32 - 3)) {
                        let lv = magic.level as i32;
                        let add = fastrand::i32(1..=(1 + lv * 2));
                        let count = self.hemorrhage_count.entry(msg.session_id).or_insert(0);
                        *count += add;
                        debug!("Player {} Hemorrhage count={} (add={})", result.object_id, *count, add);
                        if hemorrhage_armed {
                            // C#：武装命中 → 施放流血毒 + 复位
                            let duration = hemorrhage_duration(lv, state.luck).max(1) as u32;
                            let value = hemorrhage_value(state.effective_max_attack());
                            crate::combat::poison::apply_poison(
                                &mut monster.poison_list,
                                crate::combat::poison::Poison::new(
                                    mir2_shared::enums::PoisonType::BLEEDING, duration, value, 1000,
                                ),
                            );
                            *count = 0;
                            debug!("Player {} Hemorrhage bleeding on '{}' (dur={}s value={})",
                                   result.object_id, monster.name, duration, value);
                        } else if *count >= 55 {
                            // C#：武装（下次命中触发）
                            self.hemorrhage_armed.insert(msg.session_id);
                        }
                    }
                    // HalfMoon / CrossHalfMoon：C# 需 toggle 开启（HumanObject.cs:2929/3001）
                    // 倍率：HalfMoon 0.3+0.1Lv / CrossHalfMoon 0.4+0.1Lv（Envir.cs UpdateMagicInfo）
                    let halfmoon = state.magics.iter()
                        .find(|m| (m.spell == SPELL_HALFMOON as i32 || m.spell == SPELL_CROSS_HALFMOON as i32) && m.toggled)
                        .map(|m| (m.spell, m.level));
                    if let Some((spell_id, lv)) = halfmoon {
                        let mult = if spell_id == SPELL_HALFMOON as i32 {
                            0.3 + 0.1 * lv as f32
                        } else {
                            0.4 + 0.1 * lv as f32
                        };
                        let splash_dmg = ((damage as f32) * mult).max(1.0) as i32;
                        halfmoon_splash.push((0, splash_dmg)); // 标记触发
                        // C# 几何：HalfMoon 从正前方逆时针起 4 格弧；CrossHalfMoon 周围 8 格（都跳过正前方）
                        if halfmoon_cells.is_empty() {
                            let front = atk_dir;
                            if spell_id == SPELL_HALFMOON as i32 {
                                for k in 0..4usize {
                                    let d = (front + 7 + k) % 8;
                                    if d == front { continue; }
                                    halfmoon_cells.push((state.x + MON_DIR_DX[d], state.y + MON_DIR_DY[d]));
                                }
                            } else {
                                for d in 0..8usize {
                                    if d == front { continue; }
                                    halfmoon_cells.push((state.x + MON_DIR_DX[d], state.y + MON_DIR_DY[d]));
                                }
                            }
                        }
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
                    health_body.extend_from_slice(&3u16.to_le_bytes()); // expire（秒，C# ObjectHealth 语义，血条显示 3 秒）
                    let health_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectHealth as i16, &health_body);

                    // 广播给所有玩家
                    for session_id in self.players.keys() {
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: *session_id,
                            data: struck_packet.clone(),
                        }).await;
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: *session_id,
                            data: dmg_packet.clone(),
                        }).await;
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: *session_id,
                            data: health_packet.clone(),
                        }).await;
                    }

                    primary_target_oid = *oid;
                    hit_monster = true;
                    // #471：主人攻击的怪物作为所有宠物协战目标
                    for pid in &pet_ids {
                        self.pet_targets.insert(*pid, *oid);
                        debug!("Pet #{} target set -> monster #{}", pid, *oid);
                    }
                    break; // 一次只打一只
                }
            }

            // 应用 HalfMoon/CrossHalfMoon 溅射（循环外，避免借用冲突；C# 每格命中第一个目标）
            if !halfmoon_splash.is_empty() {
                let splash_dmg = halfmoon_splash[0].1;
                for (cx, cy) in &halfmoon_cells {
                    let mid = self.monsters.iter()
                        .find(|(id, m)| **id != primary_target_oid && m.hp > 0 && m.x == *cx && m.y == *cy)
                        .map(|(id, _)| *id);
                    if let Some(mid) = mid {
                        if let Some(sm) = self.monsters.get_mut(&mid) {
                            sm.take_damage(splash_dmg);
                            sm.provoked = true;
                            sm.target_session = Some(msg.session_id);
                        }
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
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: other_session,
                            data: packet.clone(),
                        }).await;

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
                            // C#：PvP 命中广播 ObjectStruck + DamageIndicator 给同图其他玩家
                    if damage > 0 {
                        self.broadcast_pvp_hit(
                            other_state.object_id, result.object_id,
                            other_state.x, other_state.y, other_state.direction, damage, other_state.map_index,
                        ).await;
                    }
                    // CounterAttack：受击方 7s 窗口激活时反击攻击者（C# HumanObject.cs 7212/7302）
                    if let Some((expire, lv)) = self.counter_attack.get(&other_session).copied() {
                        if self.tick_count <= expire {
                            self.counter_attack.remove(&other_session);
                            let counter_dmg = combat_attack::get_attack_power(
                                other_state.min_attack + other_state.bonus_min_attack,
                                other_state.max_attack + other_state.bonus_max_attack,
                                other_state.luck,
                            ).max(1);
                            let _ = record.actor_ref.ask(TakeDamage {
                                attacker_id: other_state.object_id,
                                attacker_session: other_session,
                                damage: counter_dmg,
                            }).await;
                            // 攻击者吃 Stun（Lv+1）秒
                            let _ = record.actor_ref.ask(crate::actors::player::ApplyCombatPoisons {
                                poisons: vec![crate::combat::poison::Poison::new(
                                    mir2_shared::enums::PoisonType::STUN, lv as u32 + 1, 0, 1000)],
                            }).await;
                            debug!("Player {} counter-attacked player {} ({} dmg)",
                                   other_session, msg.session_id, counter_dmg);
                        }
                    }
                    if other_actor.ask(TakeDamage {
                                attacker_id: result.object_id,
                                attacker_session: msg.session_id,
                                damage,
                            }).await.unwrap_or(false) {
                                let died_packet = Self::build_object_died_packet(
                                    other_state.object_id, other_state.x, other_state.y, other_state.direction);
                                for (sid, _) in &self.players {
                                    let _ = self.gate_ref.tell(SendToClient {
                                        session_id: *sid,
                                        data: died_packet.clone(),
                                    }).await;
                                }
                                self.handle_player_death_drop(other_session, other_state.x, other_state.y, other_state.map_index).await;

                                // 击杀玩家：增加 PK 值并广播名字颜色变化
                                let _ = record.actor_ref.ask(crate::actors::player::AddPkPoints { points: 100 }).await;
        // C# Die：击杀玩家 1/4 概率诅咒武器（Luck -1，Luck > -MaxLuck 时）
        if let Ok(Some(weapon)) = record.actor_ref.ask(crate::actors::player::GetEquipmentInfo {
            slot: crate::actors::inventory::EquipmentSlot::Weapon,
        }).await {
            if weapon.added_stats.get(mir2_shared::enums::Stat::Luck) > -10 && fastrand::i32(..4) == 0 { // C# Settings.MaxLuck = 10
                let _ = record.actor_ref.ask(crate::actors::player::AddWeaponLuck { delta: -1 }).await;
                send_system_message(&self.gate_ref, msg.session_id, "你的武器受到了诅咒！");
                debug!("Weapon cursed on player kill: {} -> {}", record.name, weapon.item_index);
            }
        }
                                if let Ok(Some(attacker_state)) = record.actor_ref.ask(GetPlayerState).await {
                                    let colour_packet = build_object_colour_changed_packet(
                                        attacker_state.object_id,
                                        name_colour_for_pk(attacker_state.pk_points),
                                    );
                                    for (sid, _) in &self.players {
                                        let _ = self.gate_ref.tell(SendToClient {
                                            session_id: *sid,
                                            data: colour_packet.clone(),
                                        }).await;
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
                    let _ = self.gate_ref.tell(SendToClient {
                        session_id: *other_session,
                        data: packet.clone(),
                    }).await;
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
            let _ = self.gate_ref.tell(SendToClient {
                session_id: other.session_id,
                data: harvest_body.clone(),
            }).await;
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
            let _ = gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: packet,
            }).await;

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
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::AllowObserve as i16, &allow_body),
        }).await;

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

        // 复活：重置 HP/MP 到最大值，回到地图安全区出生点
        // （#57：硬编码 DEFAULT_SPAWN (330,330) 在 0.map 上不可走，复活后玩家卡墙内无法移动）
        let (spawn_x, spawn_y) = self
            .map_infos
            .get(&(state.map_index as i32))
            .and_then(|mi| mi.safe_zones.iter().find(|s| s.start_point))
            .map(|sz| (sz.x, sz.y))
            .unwrap_or((DEFAULT_SPAWN_X, DEFAULT_SPAWN_Y));

        let _ = record.actor_ref.ask(crate::actors::player::RevivePlayer {
            x: spawn_x,
            y: spawn_y,
            map_index: state.map_index,
        }).await;

        // 发送 HealthChanged 通知
        let mut health_body = Vec::new();
        health_body.extend_from_slice(&(state.max_hp as u32).to_le_bytes());
        health_body.extend_from_slice(&(state.max_mp as u32).to_le_bytes());
        let _ = self.gate_ref.tell(crate::gate::actor::SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::HealthChanged as i16, &health_body),
        }).await;

        // 发送 Revived 包（C# S.Revived，空 body）：客户端靠它清除死亡状态恢复输入，
        // 只有 HealthChanged 不够——#55 实测客户端一直处于死亡状态
        let _ = self.gate_ref.tell(crate::gate::actor::SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Revived as i16, &[]),
        }).await;
        // ObjectRevived 广播：其他玩家看到复活动画
        let mut obj_body = Vec::new();
        obj_body.extend_from_slice(&state.object_id.to_le_bytes());
        obj_body.push(1u8); // effect
        let revived_packet = build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectRevived as i16, &obj_body);
        for sid in self.players.keys() {
            let _ = self.gate_ref.tell(crate::gate::actor::SendToClient {
                session_id: *sid,
                data: revived_packet.clone(),
            }).await;
        }

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
            let _ = self.gate_ref.tell(SendToClient {
                session_id: other.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectAttack as i16, &body),
            }).await;
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
                        // C#：PvP 命中广播 ObjectStruck + DamageIndicator 给同图其他玩家
                if damage > 0 {
                    self.broadcast_pvp_hit(
                        other_state.object_id, object_id,
                        other_state.x, other_state.y, other_state.direction, damage, other_state.map_index,
                    ).await;
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
                                let _ = self.gate_ref.tell(SendToClient {
                                    session_id: *sid,
                                    data: died_packet.clone(),
                                }).await;
                            }
                            self.handle_player_death_drop(other.session_id, other_state.x, other_state.y, other_state.map_index).await;

                            // 增加 PK 值
                            let _ = record.actor_ref.ask(crate::actors::player::AddPkPoints { points: 100 }).await;
        // C# Die：击杀玩家 1/4 概率诅咒武器（Luck -1，Luck > -MaxLuck 时）
        if let Ok(Some(weapon)) = record.actor_ref.ask(crate::actors::player::GetEquipmentInfo {
            slot: crate::actors::inventory::EquipmentSlot::Weapon,
        }).await {
            if weapon.added_stats.get(mir2_shared::enums::Stat::Luck) > -10 && fastrand::i32(..4) == 0 { // C# Settings.MaxLuck = 10
                let _ = record.actor_ref.ask(crate::actors::player::AddWeaponLuck { delta: -1 }).await;
                send_system_message(&self.gate_ref, msg.session_id, "你的武器受到了诅咒！");
                debug!("Weapon cursed on player kill: {} -> {}", record.name, weapon.item_index);
            }
        }
                            if let Ok(Some(attacker_state)) = record.actor_ref.ask(GetPlayerState).await {
                                let colour_packet = build_object_colour_changed_packet(
                                    attacker_state.object_id,
                                    name_colour_for_pk(attacker_state.pk_points),
                                );
                                for (sid, _) in &self.players {
                                    let _ = self.gate_ref.tell(SendToClient {
                                        session_id: *sid,
                                        data: colour_packet.clone(),
                                    }).await;
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
    /// FireBounce 剩余弹跳次数（0 = 非链式法术；C# bounce = magic.Level + 2）
    pub bounce: i32,
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

/// #306 HellFire：C# HumanObject.HellFire —— 前向直线 + Lv3 两条对角线，各 4 格
fn hellfire_cells(cx: i32, cy: i32, dir: u8, level: u8) -> Vec<(i32, i32)> {
    let dirs: Vec<usize> = if level >= 3 {
        vec![dir as usize % 8, (dir as usize + 7) % 8, (dir as usize + 1) % 8]
    } else {
        vec![dir as usize % 8]
    };
    let mut cells = Vec::new();
    for d in dirs {
        let mut x = cx;
        let mut y = cy;
        for _ in 0..4 {
            x += MON_DIR_DX[d];
            y += MON_DIR_DY[d];
            cells.push((x, y));
        }
    }
    cells
}

/// #306 IceThrust：C# HumanObject.IceThrust —— 前方 1 格主目标 + 相邻 8 格溅射
fn icethrust_cells(cx: i32, cy: i32, dir: u8) -> Vec<(i32, i32)> {
    let d = dir as usize % 8;
    let (tx, ty) = (cx + MON_DIR_DX[d], cy + MON_DIR_DY[d]);
    let mut cells = vec![(tx, ty)];
    for ox in -1..=1 {
        for oy in -1..=1 {
            if ox == 0 && oy == 0 {
                continue;
            }
            cells.push((tx + ox, ty + oy));
        }
    }
    cells
}

/// #306 Curse：C# Map.cs —— 7×7 区域
fn curse_cells(tx: i32, ty: i32) -> Vec<(i32, i32)> {
    let mut cells = Vec::new();
    for x in (tx - 3)..=(tx + 3) {
        for y in (ty - 3)..=(ty + 3) {
            cells.push((x, y));
        }
    }
    cells
}

/// #409 OneWithNature：5×5 区域（C# Map.cs:2101 location ±2）
fn curse_cells_5x5(tx: i32, ty: i32) -> Vec<(i32, i32)> {
    let mut cells = Vec::new();
    for x in (tx - 2)..=(tx + 2) {
        for y in (ty - 2)..=(ty + 2) {
            cells.push((x, y));
        }
    }
    cells
}

/// #328 Plague：C# Map.cs GetPointsInEffectiveSquare(location, 3) —— 3×3 区域
fn plague_cells(tx: i32, ty: i32) -> Vec<(i32, i32)> {
    let mut cells = Vec::new();
    for x in (tx - 1)..=(tx + 1) {
        for y in (ty - 1)..=(ty + 1) {
            cells.push((x, y));
        }
    }
    cells
}

/// #328 Plague：C# 随机毒表（Random.Next(15)：0-2 Slow、3-4 Frozen、5-9 Green、10-14 None）
fn plague_poison(roll: i32) -> mir2_shared::enums::PoisonType {
    if roll < 3 {
        mir2_shared::enums::PoisonType::SLOW
    } else if roll < 5 {
        mir2_shared::enums::PoisonType::FROZEN
    } else if roll < 10 {
        mir2_shared::enums::PoisonType::GREEN
    } else {
        mir2_shared::enums::PoisonType::NONE
    }
}

/// #328 Plague：毒强度（C# Red → value/15+Lv+1；其余 value+(Lv+1)*2）
fn plague_temp_value(value: i32, level: u8, poison: mir2_shared::enums::PoisonType) -> i32 {
    if poison == mir2_shared::enums::PoisonType::RED {
        value / 15 + level as i32 + 1
    } else {
        value + (level as i32 + 1) * 2
    }
}

/// #328 Plague：毒持续时间（C# 2*(Lv+1)+value/10）
fn plague_duration(level: u8, value: i32) -> i32 {
    2 * (level as i32 + 1) + value / 10
}

/// #345 MPEater：恢复 MP = 5*(Lv + Acc/4)（C# HumanObject.cs:3086）
fn mp_eater_restore(level: i32, accuracy: i32) -> i32 {
    5 * (level + accuracy / 4)
}

/// #345 Hemorrhage：流血持续时间 = Lv*2 + Luck/6（C# HumanObject.cs:3122）
fn hemorrhage_duration(level: i32, luck: i32) -> i32 {
    level * 2 + luck / 6
}

/// #345 Hemorrhage：流血强度 = MaxDC + 1（C# HumanObject.cs:3126）
fn hemorrhage_value(max_dc: i32) -> i32 {
    max_dc + 1
}

/// #377 弓手三连箭：状态持续时间 = 5 + 5*Lv（C# SpecialArrowShot buffTime）
pub(crate) fn special_shot_buff_time(level: u8) -> i32 {
    5 + 5 * level as i32
}

/// #395 幻觉：持续时间 = 随机 10-29 秒（C# HumanObject.cs:6342）
fn hallucination_duration() -> i32 {
    10 + fastrand::i32(0..20)
}

/// #395 幻觉：成功率（C#：roll 范围 Level+20+Lv*5，roll <= target.Level+10 失败；怪物按 Level=0）
fn hallucination_success(level: u8, caster_level: u16) -> bool {
    let roll = fastrand::i32(0..(caster_level as i32 + 20 + level as i32 * 5));
    roll > 10
}

impl WorldActor {
    /// #306：广播法术命中（ObjectStruck + DamageIndicator，对齐 C# Attacked() 表现）
    pub(crate) async fn broadcast_spell_hit(
        &self,
        hits: &[(u32, i32, i32, u8, i32)],
        attacker_id: u32,
    ) {
        for (oid, x, y, dir, damage) in hits {
            let mut struck_body = Vec::new();
            struck_body.extend_from_slice(&oid.to_le_bytes());
            struck_body.extend_from_slice(&attacker_id.to_le_bytes());
            struck_body.extend_from_slice(&(*x as u32).to_le_bytes());
            struck_body.extend_from_slice(&(*y as u32).to_le_bytes());
            struck_body.push(*dir);
            let struck_packet = build_packet_bytes(
                mir2_shared::enums::ServerPacketIds::ObjectStruck as i16, &struck_body);
            let mut dmg_body = Vec::new();
            dmg_body.extend_from_slice(&damage.to_le_bytes());
            dmg_body.push(0u8);
            dmg_body.extend_from_slice(&oid.to_le_bytes());
            let dmg_packet = build_packet_bytes(
                mir2_shared::enums::ServerPacketIds::DamageIndicator as i16, &dmg_body);
            for session_id in self.players.keys() {
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: *session_id,
                    data: struck_packet.clone(),
                }).await;
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: *session_id,
                    data: dmg_packet.clone(),
                }).await;
            }
        }
    }
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
            | SPELL_DELAYED_EXPLOSION
        );
        let spell_oid = if needs_spell_obj { Some(self.alloc_object_id()) } else { None };

        // Validate spell exists in DB
        // DB magic_infos/player_magics 使用 C# 枚举编号，客户端发来的是 SharedRust(+3)
        let spell_cs = msg.spell.saturating_sub(3);
        let spell_db = self.magic_infos.get(&(spell_cs as u32));

        // 检查玩家是否已学习该技能（基础攻击魔法不需要学习）
        let basic_spells = [0, 1]; // None, 基础攻击（C# 编号）
        if !basic_spells.contains(&spell_cs) && !state.magics.iter().any(|m| m.spell == spell_cs as i32) {
            send_system_message(&self.gate_ref, msg.session_id, "你尚未学会这个技能");
            return;
        }
        let spell_range = spell_db.map(|m| m.range as i32).unwrap_or(2);
        let power = spell_db.map(|m| m.power_base).unwrap_or(10); // for buff/heal scaling
        // Use spell level from PlayerMagic if learned
        let spell_level = state.magics.iter()
            .find(|m| m.spell == spell_cs as i32)
            .map(|m| m.level)
            .unwrap_or(0);

        // C#：施法广播 S.ObjectSpell 给同图其他玩家（ObjectID + 位置 + Spell）
        let spell_enum = mir2_shared::enums::Spell::try_from(msg.spell).unwrap_or(mir2_shared::enums::Spell::None);
        let obj_spell = mir2_shared::packets::server::magic_combat::ObjectSpell {
            object_id: state.object_id,
            location_x: state.x,
            location_y: state.y,
            spell: spell_enum,
        };
        let mut ob = Vec::new();
        if obj_spell.write_body(&mut ob).is_ok() {
            let pkt = build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectSpell as i16, &ob);
            for (sid, r) in &self.players {
                if *sid == msg.session_id { continue; }
                if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                    if os.map_index == state.map_index {
                        let _ = self.gate_ref.tell(SendToClient { session_id: *sid, data: pkt.clone() }).await;
                    }
                }
            }
        }

        // Global timestamp for CD + XP
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        // Cooldown check
        if let Some(spell_info) = spell_db {
            let delay_ms = crate::combat::magic::magic_delay(spell_info, spell_level);
            let last_cast = state.magics.iter()
                .find(|m| m.spell == spell_cs as i32)
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
        // #312：冥想被动——施法后有概率返还 MP（C# HumanObject.cs:3827，概率≈(Lv+集中)/8）
        let meditation_lv = state.magics.iter()
            .find(|m| m.spell == (SPELL_MEDITATION as i32 - 3))
            .map(|m| m.level)
            .unwrap_or(0);
        if meditation_lv > 0 && fastrand::i32(0..8) < meditation_lv as i32 {
            let _ = record.actor_ref.ask(crate::actors::player::AddMP { amount: mp_cost as i32 }).await;
            send_system_message(&self.gate_ref, msg.session_id, &format!("冥想恢复 {} 魔法值", mp_cost));
            debug!("Magic: {} Meditation refunded {} MP", state.name, mp_cost);
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
            let _ = self.gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::MagicCast as i16, &cast_body),
            }).await;
        }

        // MeteorShower：主目标是怪物时，取周围 4 格内最多 3 个副目标（伤害减半，C# HumanObject.cs:5835）
        let meteor_secondary: Vec<(u32, i32, i32)> =
            if spell_enum == mir2_shared::enums::Spell::MeteorShower {
                let mut ids = Vec::new();
                if let Some(m) = self.monsters.get(&msg.target_id) {
                    if m.hp > 0 {
                        let mut nearby: Vec<(u32, i32, i32)> = self.monsters.iter()
                            .filter(|(id, mm)| {
                                **id != msg.target_id
                                    && mm.hp > 0
                                    && (mm.x - m.x).abs() <= 4
                                    && (mm.y - m.y).abs() <= 4
                            })
                            .map(|(id, mm)| (*id, mm.x, mm.y))
                            .collect();
                        // 按距离升序取前 3（近似 C# FindAllNearby(4)）
                        nearby.sort_by_key(|(_, x, y)| (x - m.x).abs() + (y - m.y).abs());
                        ids = nearby.into_iter().take(3).collect();
                    }
                }
                ids
            } else {
                Vec::new()
            };

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
            secondary_target_ids: meteor_secondary.iter().map(|(id, _, _)| *id).collect(),
        };
        let mut om_body = Vec::new();
        if object_magic.write_body(&mut om_body).is_ok() {
            for other in &others {
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: other.session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectMagic as i16, &om_body),
                }).await;
            }
        }

        // 创建持久法术对象（火墙、暴风雪等）
        let spell_enum = mir2_shared::enums::Spell::try_from(msg.spell)
            .unwrap_or(mir2_shared::enums::Spell::None);
        let is_persistent = matches!(spell_enum,
            mir2_shared::enums::Spell::FireWall | mir2_shared::enums::Spell::Blizzard
            | mir2_shared::enums::Spell::MeteorStrike | mir2_shared::enums::Spell::PoisonCloud
            | mir2_shared::enums::Spell::HealingCircle | mir2_shared::enums::Spell::ExplosiveTrap
            | mir2_shared::enums::Spell::Portal | mir2_shared::enums::Spell::DelayedExplosion
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
            // Healing：单目标友方（C# HumanObject.cs：health = GetDamage(SC*2) + Level）
            // MassHealing：目标点 3×3 内自己+同组（C# Map.cs：value = GetDamage(SC)）
            SPELL_HEALING | SPELL_MASS_HEALING => {
                let sc_power = crate::combat::attack::get_attack_power(
                    state.min_sc + state.bonus_min_sc,
                    state.max_sc + state.bonus_max_sc,
                    0,
                );
                if msg.spell == SPELL_HEALING {
                    // 友方目标：点击自己/同组玩家
                    let mut target_session = msg.session_id;
                    if msg.target_id != 0 {
                        for (sid, r) in &self.players {
                            if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                                if os.object_id == msg.target_id {
                                    let friendly = *sid == msg.session_id
                                        || (os.group_id.is_some() && os.group_id == state.group_id);
                                    if friendly {
                                        target_session = *sid;
                                    }
                                    break;
                                }
                            }
                        }
                    }
                    let amount = sc_power * 2 + state.level as i32; // C# GetDamage(SC*2) + Level
                    if let Some(r) = self.players.get(&target_session) {
                        let _ = r.actor_ref.ask(crate::actors::player::Heal { amount }).await;
                    }
                    debug!("Magic: {} casts Healing on session {} (+{} HP)", state.name, target_session, amount);
                } else {
                    let cx = if target_x == 0 && target_y == 0 { state.x } else { target_x };
                    let cy = if target_x == 0 && target_y == 0 { state.y } else { target_y };
                    let amount = sc_power.max(1);
                    let mut healed = 0u32;
                    for (sid, r) in &self.players {
                        if let Ok(Some(s)) = r.actor_ref.ask(GetPlayerState).await {
                            let friendly = *sid == msg.session_id
                                || (s.group_id.is_some() && s.group_id == state.group_id);
                            if friendly && !s.is_dead
                                && (s.x - cx).abs() <= 1 && (s.y - cy).abs() <= 1
                            {
                                let _ = r.actor_ref.ask(crate::actors::player::Heal { amount }).await;
                                healed += 1;
                            }
                        }
                    }
                    debug!("Magic: {} casts MassHealing (3x3, healed {} players, +{} HP)",
                           state.name, healed, amount);
                }
            }
            // HealingCircle：持续治疗场由 SpellObject 每跳治疗（C# 无即时自疗）
            SPELL_HEALING_CIRCLE => {
                debug!("Magic: {} casts HealingCircle (persistent field)", state.name);
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
            // SoulShield / BlessedArmour：目标点 7×7 友方护盾（C# HumanObject.cs + Map.cs）
            // bonus = 目标等级/7+4；时长 = SC*4 + (Lv+1)*50 秒
            SPELL_SOUL_SHIELD | SPELL_BLESSED_ARMOUR => {
                let is_soul = msg.spell == SPELL_SOUL_SHIELD;
                let sc = state.effective_max_sc();
                let duration_ticks = ((sc * 4 + (spell_level as i32 + 1) * 50).max(1) as u32) * 10;
                let cx = if target_x == 0 && target_y == 0 { state.x } else { target_x };
                let cy = if target_x == 0 && target_y == 0 { state.y } else { target_y };
                let mut targets: Vec<u64> = vec![msg.session_id];
                if let Some(gid) = state.group_id {
                    for (sid, other) in &self.players {
                        if *sid == msg.session_id { continue; }
                        if let Ok(Some(s)) = other.actor_ref.ask(GetPlayerState).await {
                            if s.group_id == Some(gid)
                                && (s.x - cx).abs() <= 3 && (s.y - cy).abs() <= 3 {
                                targets.push(*sid);
                            }
                        }
                    }
                }
                for sid in &targets {
                    let Some(other) = self.players.get(sid) else { continue; };
                    let level = if *sid == msg.session_id {
                        state.level
                    } else {
                        other.actor_ref.ask(GetPlayerState).await.ok().flatten().map(|s| s.level).unwrap_or(0)
                    };
                    let bonus = (level as i32 / 7 + 4).max(1);
                    let buff = crate::combat::buff::BuffInstance::new(
                        if is_soul {
                            crate::combat::buff::BuffType::MacDefenseBoost { bonus }
                        } else {
                            crate::combat::buff::BuffType::AcDefenseBoost { bonus }
                        },
                        duration_ticks,
                        5,
                    );
                    let _ = other.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                }
                debug!("Magic: {} casts {} on {} targets (+{}, {}s)",
                       state.name, if is_soul { "SoulShield" } else { "BlessedArmour" },
                       targets.len(), state.level as i32 / 7 + 4, duration_ticks / 10);
            }
            // --- 道士 Debuff/控制类 ---
            // Poisoning：对目标怪物施毒（绿毒持续掉血/红毒降防御，C# Poisoning 消耗毒药物品）
            SPELL_POISONING => {
                // C# HumanObject.cs:6043：单目标（点击格首个怪物），value = GetDamage(SC)
                let value = crate::combat::attack::get_attack_power(
                    state.min_sc + state.bonus_min_sc,
                    state.max_sc + state.bonus_max_sc,
                    0,
                ).max(1);
                let mid = self.monsters.iter()
                    .find(|(_, m)| (m.x - target_x).abs() <= 1 && (m.y - target_y).abs() <= 1 && m.hp > 0)
                    .map(|(id, _)| *id);
                if let Some(mid) = mid {
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        // C# Shape1 绿毒：Duration = value*2 + (Lv+1)*7；Value = value/15 + Lv + 1 + Random(PoisonAttack)
                        let duration = (value * 2 + (spell_level as i32 + 1) * 7).max(1) as u32;
                        let poison_value = (value / 15 + spell_level as i32 + 1
                            + fastrand::i32(0..state.poison_attack.max(1))).max(1);
                        crate::combat::poison::apply_poison(&mut monster.poison_list,
                            crate::combat::poison::Poison::new(
                                mir2_shared::enums::PoisonType::GREEN, duration, poison_value, 2000,
                            ));
                        monster.provoked = true;
                        monster.target_session = Some(msg.session_id);
                        debug!("Magic: {} casts Poisoning -> monster {} ({}s, {}dmg/tick)",
                               state.name, mid, duration, poison_value);
                    }
                } else {
                    debug!("Magic: {} casts Poisoning (no target near {},{})", state.name, target_x, target_y);
                }
            }
            // TrapHexagon：定身目标怪物（C# HumanObject.cs + Map.cs：跳过等级 > 施法+2 的怪物，
            // 时长 = (Lv*5+10) 秒）
            SPELL_TRAP_HEXAGON => {
                let hit_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| {
                        let dist = (m.x - target_x).abs() + (m.y - target_y).abs();
                        dist <= 1 && m.hp > 0
                            && self.monster_infos.get(&m.monster_index).map(|i| i.level).unwrap_or(0) <= state.level as i32 + 2
                    })
                    .map(|(id, _)| *id)
                    .collect();
                let trapped_count = hit_ids.len();
                let duration = (spell_level as u32 * 5 + 10) as u32;
                for mid in hit_ids {
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        crate::combat::poison::apply_poison(&mut monster.poison_list,
                            crate::combat::poison::Poison::new(mir2_shared::enums::PoisonType::PARALYSIS, duration, 0, 1000));
                    }
                }
                debug!("Magic: {} casts TrapHexagon (trapped {} monsters, {}s)", state.name, trapped_count, duration);
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
            if let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await {
                self.broadcast_object_hidden(st.object_id, true, st.map_index).await;
            }
                debug!("Magic: {} casts Hiding (invisible)", state.name);
            }
            // MassHiding：组队隐身（目标点 3×3 友方 + C# 时长公式）
            SPELL_MASS_HIDING => {
                // C# 时长：value = GetAttackPower(MinSC,MaxSC)/2 + (Lv+1)*2 秒（HumanObject.cs:4500）
                let sc_power = crate::combat::attack::get_attack_power(
                    state.min_sc + state.bonus_min_sc,
                    state.max_sc + state.bonus_max_sc,
                    0,
                );
                let duration_ticks = ((sc_power / 2 + (spell_level as i32 + 1) * 2).max(1) as u32) * 10;
                // C# Map.cs MassHiding：目标点 3×3（±1）范围内友方（自己/同组）隐身
                let cx = if target_x == 0 && target_y == 0 { state.x } else { target_x };
                let cy = if target_x == 0 && target_y == 0 { state.y } else { target_y };
                let mut targets: Vec<u64> = vec![msg.session_id];
                if let Some(gid) = state.group_id {
                    for (sid, other) in &self.players {
                        if *sid == msg.session_id { continue; }
                        if let Ok(Some(s)) = other.actor_ref.ask(GetPlayerState).await {
                            if s.group_id == Some(gid)
                                && (s.x - cx).abs() <= 1 && (s.y - cy).abs() <= 1 {
                                targets.push(*sid);
                            }
                        }
                    }
                }
                for sid in &targets {
                    let buff = crate::combat::buff::BuffInstance::new(
                        crate::combat::buff::BuffType::Invisibility,
                        duration_ticks,
                        5,
                    );
                    let Some(other) = self.players.get(sid) else { continue; };
                    let _ = other.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                    self.invisible_sessions.insert(*sid);
                    if let Ok(Some(st)) = other.actor_ref.ask(GetPlayerState).await {
                        self.broadcast_object_hidden(st.object_id, true, st.map_index).await;
                    }
                }
                debug!("Magic: {} casts MassHiding on {} targets ({}s)", state.name, targets.len(), duration_ticks / 10);
            }
            // Purification：解毒/清除 debuff（C# HumanObject.cs:4440 + CompleteMagic 6246）
            // 友方目标（自己/同组），成功率 Random(4) <= Lv（Lv0=25%）
            SPELL_PURIFICATION => {
                if fastrand::i32(0..4) > spell_level as i32 {
                    debug!("Magic: {} casts Purification (failed)", state.name);
                    return;
                }
                let mut target_session = msg.session_id;
                if msg.target_id != 0 {
                    for (sid, r) in &self.players {
                        if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                            if os.object_id == msg.target_id {
                                let friendly = *sid == msg.session_id
                                    || (os.group_id.is_some() && os.group_id == state.group_id);
                                if friendly {
                                    target_session = *sid;
                                }
                                break;
                            }
                        }
                    }
                }
                if let Some(r) = self.players.get(&target_session) {
                    let _ = r.actor_ref.ask(crate::actors::player::PurifyPoisons).await;
                }
                debug!("Magic: {} casts Purification on session {} (success)", state.name, target_session);
            }
            // Entrapment：困魔咒（C# HumanObject.cs:4893 + CompleteMagic 6315）——
            // 拉拽目标怪物朝施法者反方向靠近（对角 min(|dx|,|dy|)，十字轴 |axis|-2），并麻痹 round((Lv+1)*0.8) 秒
            SPELL_ENTRAPMENT => {
                let mid = self.monsters.iter()
                    .filter(|(_, m)| m.hp > 0 && (m.x - target_x).abs() <= 1 && (m.y - target_y).abs() <= 1)
                    .map(|(id, _)| *id)
                    .next();
                let Some(mid) = mid else { return; };
                let (mx, my, mlevel) = match self.monsters.get(&mid) {
                    Some(m) => (m.x, m.y, self.monster_infos.get(&m.monster_index).map(|i| i.level).unwrap_or(0)),
                    None => return,
                };
                let dist = (state.x - mx).abs().max((state.y - my).abs());
                // C#：MaxDistance > 7 或目标等级 >= 施法等级 + 5 + Random(8) → 失败
                if dist > 7 || mlevel >= state.level as i32 + 5 + fastrand::i32(0..8) {
                    return;
                }
                // C#：Random(30) >= (Lv+1)*3 + (Level - targetLevel + 9) → 失败
                let levelgap = state.level as i32 - mlevel + 9;
                if fastrand::i32(0..30) >= ((spell_level as i32 + 1) * 3) + levelgap {
                    return;
                }
                // 麻痹时长（怪物）：round((Lv+1)*0.8)
                let duration = (((spell_level as i32 + 1) as f64) * 0.8).round() as u32;
                if duration > 0 {
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        crate::combat::poison::apply_poison(&mut monster.poison_list,
                            crate::combat::poison::Poison::new(mir2_shared::enums::PoisonType::PARALYSIS, duration, 0, 1000));
                    }
                }
                // 拉拽方向 = 施法者朝向的反方向（C# (Direction - 4) % 8）
                let pull_dir = ((msg.direction as usize + 4) % 8) as u8;
                let pulldistance = if pull_dir % 2 > 0 {
                    ((state.x - mx).abs().min((state.y - my).abs())).max(0)
                } else {
                    match pull_dir {
                        0 | 4 => ((state.y - my).abs() - 2).max(0), // Up/Down
                        _ => ((state.x - mx).abs() - 2).max(0),      // Left/Right
                    }
                };
                let moved = self.push_monster(mid, pull_dir, pulldistance.max(1)).await;
                debug!("Magic: {} casts Entrapment -> monster {} pulled {} tiles ({}s paralysis)",
                    state.name, mid, moved, duration);
            }
            // ShoulderDash：野蛮冲撞（C# HumanObject.cs：只冲刺 2 格 + 推开路径上等级更低的目标 1 格，无伤害）
            SPELL_SHOULDER_DASH => {
                let dir = msg.direction as usize % 8;
                let mut new_x = state.x;
                let mut new_y = state.y;
                let mut pushed = 0usize;
                for step in 0..2 {
                    let nx = new_x + MON_DIR_DX[dir];
                    let ny = new_y + MON_DIR_DY[dir];
                    let walkable = self.maps.get(&state.map_index)
                        .map(|m| m.is_walkable(nx, ny))
                        .unwrap_or(false);
                    if !walkable { break; }
                    // C#：路径上等级 < 施法等级的目标才推送
                    let hit: Option<(u32, i32)> = self.monsters.iter()
                        .find(|(_, m)| m.x == nx && m.y == ny && m.hp > 0)
                        .map(|(id, m)| (*id, self.monster_infos.get(&m.monster_index).map(|i| i.level).unwrap_or(0)));
                    if let Some((mid, mlevel)) = hit {
                        if mlevel < state.level as i32 {
                            let _ = self.push_monster(mid, dir as u8, 1).await;
                            pushed += 1;
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
                    self.broadcast_position_change(msg.session_id, new_x, new_y, msg.direction).await;
                }
                debug!("Magic: {} casts ShoulderDash (dashed to {},{}, pushed {} monsters)",
                    state.name, new_x, new_y, pushed);
            }
            // Thrusting：刺杀（直线穿透 2 格，打前方 2 个格子）
            SPELL_THRUSTING => {
                let dir = msg.direction as usize % 8;
                let attacker_stats = state.to_combat_stats();
                let level_offset = state.level.min(10) as u16;
                let raw = crate::combat::attack::get_attack_power(
                    attacker_stats.min_atk, attacker_stats.max_atk, attacker_stats.luck,
                );
                // C# Envir.cs Thrusting：倍率 0.25+0.25Lv（GetDamage = base × Multiplier）
                let raw_damage = ((raw as f32) * (0.25 + 0.25 * spell_level as f32)).max(1.0) as i32;
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
            SPELL_TELEPORT | SPELL_BLINK | SPELL_STORM_ESCAPE => {
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

                // Blink/StormEscape：距离校验 + 成功率（C# Random(4) >= Lv+1 失败）
                if msg.spell == SPELL_BLINK || msg.spell == SPELL_STORM_ESCAPE {
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
                self.broadcast_position_change(msg.session_id, tx, ty, msg.direction).await;
                debug!("Magic: {} teleports/blinks to ({}, {})", state.name, tx, ty);
            }
            // --- 弹道类法术（任务3）：FireBall/GreatFireBall/ThunderBolt/FrostCrunch/Vampirism ---
            // 对齐 C# HumanObject Fireball()/ThunderBolt()/Vampirism()：创建 DelayedAction，延迟后结算
            SPELL_FIREBALL | SPELL_GREAT_FIREBALL | SPELL_THUNDERBOLT
            | SPELL_FROST_CRUNCH | SPELL_VAMPIRISM | SPELL_FLAME_DISRUPTOR | SPELL_SOUL_FIREBALL
            | SPELL_METEOR_SHOWER => {
                let raw_damage = if let Some(info) = spell_db {
                    crate::combat::magic::calc_magic_damage(info, spell_level, magic_stat)
                } else {
                    fastrand::i32(5..=15)
                }.max(1);

                // 弹道延迟：FireBall 系 = 距离×50ms + 500ms；ThunderBolt/Vampirism = 固定 500ms
                let target_dist = ((state.x - target_x).abs() + (state.y - target_y).abs()) as u64;
                let delay_ms = match msg.spell {
                    SPELL_FIREBALL | SPELL_GREAT_FIREBALL | SPELL_FROST_CRUNCH | SPELL_METEOR_SHOWER => {
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
                    bounce: 0,
                });

                // MeteorShower：副目标（最多 3 个，周围 4 格）各吃 50% 伤害（C# HumanObject.cs:5852）
                if msg.spell == SPELL_METEOR_SHOWER {
                    for (sid, sx, sy) in &meteor_secondary {
                        self.pending_spell_completions.push(PendingSpellCompletion {
                            fire_at_tick,
                            session_id: msg.session_id,
                            spell: msg.spell,
                            target_id: *sid,
                            target_x: *sx,
                            target_y: *sy,
                            damage: (raw_damage / 2).max(1),
                            magic_stat,
                            spell_level,
                            bounce: 0,
                        });
                    }
                }
                debug!("Magic: {} casts projectile spell={} dmg={} delay={}ms secondary={} (fires @tick {})",
                    state.name, msg.spell, raw_damage, delay_ms, meteor_secondary.len(), fire_at_tick);
            }
            // FireBounce：链式弹射（C# HumanObject.cs:5811；首跳延迟=距离×50+500ms，后续每跳=距离×50ms）
            SPELL_FIRE_BOUNCE => {
                let raw_damage = if let Some(info) = spell_db {
                    crate::combat::magic::calc_magic_damage(info, spell_level, magic_stat)
                } else { fastrand::i32(5..=15) }.max(1);
                let target_dist = ((state.x - target_x).abs() + (state.y - target_y).abs()) as u64;
                let delay_ms = target_dist * 50 + 500;
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
                    bounce: spell_level as i32 + 2, // C# bounce = magic.Level + 2
                });
                debug!("Magic: {} casts FireBounce dmg={} bounce={} delay={}ms",
                    state.name, raw_damage, spell_level as i32 + 2, delay_ms);
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
            // ThunderStorm 对非亡灵伤害 ×1/10（下方按 monster.undead 调整）
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
            // #306：HellFire —— 三向直线 AoE（C# HumanObject.HellFire：Lv3 三向，各 4 格，MAC）
            SPELL_HELLFIRE => {
                let raw_damage = if let Some(info) = spell_db {
                    crate::combat::magic::calc_magic_damage(info, spell_level, magic_stat)
                } else { fastrand::i32(8..=20) }.max(1);
                let attacker_stats = state.to_combat_stats();
                let level_offset = state.level.min(10) as u16;
                let cells = hellfire_cells(state.x, state.y, msg.direction, spell_level);
                let hit_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| m.hp > 0 && cells.contains(&(m.x, m.y)))
                    .map(|(id, _)| *id)
                    .collect();
                let mut spell_hits: Vec<(u32, i32, i32, u8, i32)> = Vec::new();
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
                            spell_hits.push((mid, monster.x, monster.y, monster.direction, r.damage));
                        }
                    }
                }
                self.broadcast_spell_hit(&spell_hits, object_id).await;
                debug!("Magic: {} casts HellFire ({} cells) dmg={} hits={}", state.name, cells.len(), raw_damage, spell_hits.len());
            }
            // #306：IceThrust —— 前方 1 格幸运暴击 + 60% 溅射（C# HumanObject.IceThrust）
            SPELL_ICETHRUST => {
                let mut raw_damage = if let Some(info) = spell_db {
                    crate::combat::magic::calc_magic_damage(info, spell_level, magic_stat)
                } else { fastrand::i32(8..=20) }.max(1);
                // C#：Random.Next(100) < (1 + Luck) → 伤害翻倍
                if fastrand::i32(0..100) < (1 + state.luck) {
                    raw_damage *= 2;
                }
                let attacker_stats = state.to_combat_stats();
                let level_offset = state.level.min(10) as u16;
                let cells = icethrust_cells(state.x, state.y, msg.direction);
                let mut spell_hits: Vec<(u32, i32, i32, u8, i32)> = Vec::new();
                for (i, (cx, cy)) in cells.iter().enumerate() {
                    let dmg = if i == 0 { raw_damage } else { (raw_damage as f32 * 0.6) as i32 };
                    let hit: Option<u32> = self.monsters.iter()
                        .find(|(_, m)| m.x == *cx && m.y == *cy && m.hp > 0)
                        .map(|(id, _)| *id);
                    if let Some(mid) = hit {
                        if let Some(monster) = self.monsters.get_mut(&mid) {
                            let defender_stats = monster.to_combat_stats();
                            let r = combat_attack::resolve_attack(
                                &attacker_stats, &defender_stats, dmg,
                                mir2_shared::enums::DefenceType::Mac, level_offset,
                            );
                            if r.is_hit && r.damage > 0 {
                                monster.take_damage(r.damage);
                                monster.provoked = true;
                                monster.target_session = Some(msg.session_id);
                                spell_hits.push((mid, monster.x, monster.y, monster.direction, r.damage));
                            }
                        }
                    }
                }
                self.broadcast_spell_hit(&spell_hits, object_id).await;
                debug!("Magic: {} casts IceThrust dmg={} hits={}", state.name, raw_damage, spell_hits.len());
            }
            // #306：Curse —— 7×7 区域 40% 概率 Slow 毒 + 减伤（C# Map.cs:1837，value2=1+(Lv+1)*2）
            SPELL_CURSE => {
                let value2 = 1 + (spell_level as i32 + 1) * 2;
                // C# Curse：Random(10-(Lv+1)*2) > 2 失败（Lv0≈37.5% → Lv3=100%）
                let chance_n = (10 - (spell_level as i32 + 1) * 2).max(1);
                if fastrand::i32(0..chance_n) > 2 {
                    debug!("Magic: {} casts Curse (failed, n={})", state.name, chance_n);
                    return;
                }
                // C# damage = magic.GetDamage(SC)，Envir.cs MPowerBase=20 → +5(Lv+1)
                let sc_power = crate::combat::attack::get_attack_power(
                    state.min_sc + state.bonus_min_sc,
                    state.max_sc + state.bonus_max_sc,
                    0,
                );
                let damage = (sc_power + 5 * (spell_level as i32 + 1)).max(1);
                let cells = curse_cells(target_x, target_y);
                let hit_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| m.hp > 0 && cells.contains(&(m.x, m.y)))
                    .map(|(id, _)| *id)
                    .collect();
                let candidate_count = hit_ids.len();
                for mid in hit_ids {
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        // Slow 毒（C# Duration=damage 秒，Value=value2）
                        crate::combat::poison::apply_poison(
                            &mut monster.poison_list,
                            crate::combat::poison::Poison::new(
                                mir2_shared::enums::PoisonType::SLOW,
                                damage.max(1) as u32,
                                value2,
                                1000,
                            ),
                        );
                        monster.provoked = true;
                        monster.target_session = Some(msg.session_id);
                        // 减伤：value2%（C# 降低 MaxDC/MaxMC/MaxSC 输出百分比），持续 damage 秒
                        let until = self.tick_count + (damage.max(1) as u64) * 10;
                        self.cursed_monsters.insert(mid, (value2, until));
                    }
                }
                debug!("Magic: {} casts Curse (7x7, {} candidates, rate={}%)", state.name, candidate_count, value2);
            }
            // ===== 弓箭手（Archer）弹道物理系法术 =====
            // StraightShot：单目标弹道，延迟 = 距离×50ms + 500ms，AC 防御（弓箭手物理）
            // DoubleShot：对目标连发 2 次弹道（第二次延迟 +200ms）
            // BindingShot：弹道 + 命中后 Paralysis（在 complete_projectile_spell 结算）
            // NapalmShot：弹道 + 命中后 3×3 AOE（在 complete_projectile_spell 结算）
            // 伤害基于 DC（物理攻击），用 magic_stat（弓箭手类 = effective_max_attack）
            SPELL_STRAIGHT_SHOT | SPELL_DOUBLE_SHOT | SPELL_BINDING_SHOT | SPELL_NAPALM_SHOT | SPELL_CAT_TONGUE
            | SPELL_VAMPIRE_SHOT | SPELL_POISON_SHOT | SPELL_CRIPPLE_SHOT | SPELL_ELEMENTAL_SHOT => {
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
                    bounce: 0,
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
                        bounce: 0,
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
            // ElementalBarrier：自身减伤 buff（DamageReduction）
            // C# 时长 = magic.GetPower(MC随机) + barrierPower(0) = MC 随机秒（HumanObject.cs:3726/6417）
            SPELL_ELEMENTAL_BARRIER => {
                let reduction_pct = ((spell_level as i32 + 1) * 10).min(80);
                let mc_power = crate::combat::attack::get_attack_power(
                    state.min_mc + state.bonus_min_mc,
                    state.max_mc + state.bonus_max_mc,
                    0,
                ).max(1);
                let duration_ticks = (mc_power as u32) * 10;
                let _ = record.actor_ref.ask(crate::actors::player::ApplyDamageReduction {
                    percent: reduction_pct,
                    duration_ticks,
                }).await;
                debug!("Magic: {} casts ElementalBarrier (damage -{}%, {}s)",
                       state.name, reduction_pct, mc_power);
            }
            // Mirroring：分身术（C# HumanObject.cs Mirroring）——召唤 Clone 分身宠物（Settings.CloneName="Clone"）
            SPELL_MIRRORING => {
                const CLONE_NAME: &str = "Clone";
                // 已有存活分身 → 移除（C# monster.Die()）
                let existing: Option<u32> = self.monsters.iter()
                    .find(|(_, m)| m.master_session == Some(msg.session_id)
                        && m.name.eq_ignore_ascii_case(CLONE_NAME) && m.hp > 0)
                    .map(|(id, _)| *id);
                if let Some(oid) = existing {
                    if self.monsters.remove(&oid).is_some() {
                        let rm = Self::build_object_remove_packet(oid);
                        for sid in self.players.keys() {
                            let _ = self.gate_ref.tell(SendToClient {
                                session_id: *sid,
                                data: rm.clone(),
                            }).await;
                        }
                    }
                    debug!("Magic: {} Mirroring removed existing clone #{}", state.name, oid);
                    return;
                }
                // 生成在前方 1 格（C# Front）
                let dir = msg.direction as usize % 8;
                let (sx, sy) = (state.x + MON_DIR_DX[dir], state.y + MON_DIR_DY[dir]);
                let mon_index = self.monster_name_index.get(CLONE_NAME.to_lowercase().as_str()).copied();
                match mon_index {
                    Some(idx) => {
                        if let Some(info) = self.monster_infos.get(&idx).cloned() {
                            let new_oid = self.alloc_object_id();
                            let hp = info.stats.get(&(mir2_shared::enums::Stat::HP as u8)).copied().unwrap_or(50);
                            let min_dmg = info.stats.get(&(mir2_shared::enums::Stat::MinDC as u8)).copied().unwrap_or(5);
                            let max_dmg = info.stats.get(&(mir2_shared::enums::Stat::MaxDC as u8)).copied().unwrap_or(10);
                            let spawn = MonsterSpawn {
                                name: info.name.clone(),
                                image: info.image as u16,
                                monster_index: idx,
                                x: sx, y: sy,
                                direction: msg.direction,
                                hp, min_dmg, max_dmg,
                                xp: info.experience,
                                map_index: state.map_index,
                            };
                            let packet = build_object_monster_packet(&spawn, new_oid, &spawn.name);
                            for sid in self.players.keys() {
                                let _ = self.gate_ref.tell(SendToClient {
                                    session_id: *sid,
                                    data: packet.clone(),
                                }).await;
                            }
                            let ai_profile = MonsterAiProfile::from_info(&info);
                            self.monsters.insert(new_oid, MonsterState {
                                object_id: new_oid,
                                name: spawn.name.clone(),
                                image: spawn.image,
                                monster_index: idx,
                                x: sx, y: sy, direction: msg.direction,
                                hp, max_hp: hp, min_dmg, max_dmg, xp: spawn.xp,
                                spawn_x: sx, spawn_y: sy, map_index: state.map_index,
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
                                undead: info.undead,
                                master_session: Some(msg.session_id),
                                recall_at_tick: 0,
                                behavior: crate::actors::world::ai::make_behavior(&spawn.name),
                            });
                            self.pet_levels.insert(new_oid, spell_level as i32);
                            debug!("Magic: {} casts Mirroring -> clone #{} at ({},{})",
                                   state.name, new_oid, sx, sy);
                        } else {
                            warn!("Mirroring '{}' found index {} but no MonsterInfo", CLONE_NAME, idx);
                        }
                    }
                    None => {
                        warn!("Mirroring '{}' not in monster_name_index (DB may lack this mob)", CLONE_NAME);
                    }
                }
            }
            // ===== 刺客法术（Assassin，buff 系 + 位移系 + 物理攻击系）=====
            // Haste：攻击速度提升（C# CompleteMagic 6149：AttackSpeed stat += Lv*2+2，时长 25+15Lv 秒）
            SPELL_HASTE => {
                // stat 2..8 ≈ 20..80% 冷却缩减（近似）
                let pct = (2 + spell_level as i32 * 2) * 10;
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::AttackSpeedBoost { percent: pct },
                    (25 + spell_level as u32 * 15) * 10,
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts Haste (attack speed +{}%, {}s)",
                       state.name, pct, 25 + spell_level as i32 * 15);
            }
            // LightBody：敏捷提升（C# CompleteMagic 6187：Agility += (Lv+1)*2，时长 (Lv+1)*30 秒）
            SPELL_LIGHT_BODY => {
                let agi_bonus = (spell_level as i32 + 1) * 2;
                let buff1 = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::AgilityBoost { bonus: agi_bonus },
                    (spell_level as u32 + 1) * 300,
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff: buff1 }).await;
                debug!("Magic: {} casts LightBody (agility +{}, {}s)",
                       state.name, agi_bonus, (spell_level as i32 + 1) * 30);
            }
            // Fury：攻速提升（C# CompleteMagic 6160：Stat.AttackSpeed=4，时长 60+10Lv 秒）
            SPELL_FURY => {
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::AttackSpeedBoost { percent: 4 },
                    (60 + spell_level as u32 * 10) * 10,
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts Fury (attack speed +4, {}s)",
                       state.name, 60 + spell_level as i32 * 10);
            }
            // Rage：DC 提升（C# HumanObject.cs Rage：MaxDC/MinDC += round(MaxDC*(0.12+0.03Lv))，18+6Lv 秒）
            SPELL_RAGE => {
                let add_value = (state.max_attack as f32 * (0.12 + 0.03 * spell_level as f32)).round() as i32;
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::AttackBoost { bonus: add_value.max(1) },
                    (18 + spell_level as u32 * 6) * 10,
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts Rage (DC +{}, {}s)",
                       state.name, add_value.max(1), 18 + spell_level as i32 * 6);
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
            // C# 时长：(GetAttackPower(MinAC,MaxAC) + (Lv+1)*5) * 500ms
            SPELL_MOON_LIGHT => {
                let ac_power = crate::combat::attack::get_attack_power(
                    state.min_ac + state.bonus_min_ac,
                    state.max_ac + state.bonus_max_ac,
                    0,
                );
                let duration_ticks = ((ac_power + (spell_level as i32 + 1) * 5).max(1) as u32) * 5;
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::Invisibility,
                    duration_ticks, 5);
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                self.invisible_sessions.insert(msg.session_id);
            if let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await {
                self.broadcast_object_hidden(st.object_id, true, st.map_index).await;
            }
                debug!("Magic: {} casts MoonLight (invisible {}s)", state.name, duration_ticks / 10);
            }
            // DarkBody：刺客分身（C# HumanObject.cs:5323）——召唤 AssassinClone 宠物；已有存活分身则移除
            SPELL_DARK_BODY => {
                const CLONE_NAME: &str = "AssassinClone";
                // 已有存活分身 → 移除（C# monster.Die()）
                let existing: Option<u32> = self.monsters.iter()
                    .find(|(_, m)| m.master_session == Some(msg.session_id)
                        && m.name.eq_ignore_ascii_case(CLONE_NAME) && m.hp > 0)
                    .map(|(id, _)| *id);
                if let Some(oid) = existing {
                    if self.monsters.remove(&oid).is_some() {
                        let rm = Self::build_object_remove_packet(oid);
                        for sid in self.players.keys() {
                            let _ = self.gate_ref.tell(SendToClient {
                                session_id: *sid,
                                data: rm.clone(),
                            }).await;
                        }
                    }
                    debug!("Magic: {} DarkBody removed existing clone #{}", state.name, oid);
                    return;
                }
                // 目标玩家 session（C# monster.Target = 点击目标）
                let target_session: Option<u64> = if msg.target_id != 0 {
                    let mut found = None;
                    for (sid, r) in &self.players {
                        if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                            if os.object_id == msg.target_id {
                                found = Some(*sid);
                                break;
                            }
                        }
                    }
                    found
                } else {
                    None
                };
                let mon_index = self.monster_name_index.get(CLONE_NAME.to_lowercase().as_str()).copied();
                match mon_index {
                    Some(idx) => {
                        if let Some(info) = self.monster_infos.get(&idx).cloned() {
                            let new_oid = self.alloc_object_id();
                            let hp = info.stats.get(&(mir2_shared::enums::Stat::HP as u8)).copied().unwrap_or(50);
                            let min_dmg = info.stats.get(&(mir2_shared::enums::Stat::MinDC as u8)).copied().unwrap_or(5);
                            let max_dmg = info.stats.get(&(mir2_shared::enums::Stat::MaxDC as u8)).copied().unwrap_or(10);
                            let spawn = MonsterSpawn {
                                name: info.name.clone(),
                                image: info.image as u16,
                                monster_index: idx,
                                x: state.x,
                                y: state.y,
                                direction: msg.direction,
                                hp,
                                min_dmg,
                                max_dmg,
                                xp: info.experience,
                                map_index: state.map_index,
                            };
                            let packet = build_object_monster_packet(&spawn, new_oid, &spawn.name);
                            for sid in self.players.keys() {
                                let _ = self.gate_ref.tell(SendToClient {
                                    session_id: *sid,
                                    data: packet.clone(),
                                }).await;
                            }
                            let ai_profile = MonsterAiProfile::from_info(&info);
                            self.monsters.insert(new_oid, MonsterState {
                                object_id: new_oid,
                                name: spawn.name.clone(),
                                image: spawn.image,
                                monster_index: idx,
                                x: state.x, y: state.y, direction: msg.direction,
                                hp, max_hp: hp, min_dmg, max_dmg, xp: spawn.xp,
                                spawn_x: state.x, spawn_y: state.y, map_index: state.map_index,
                                next_attack_tick: 0, next_move_tick: 0, next_summon_tick: 0,
                                ai_profile, ai_state: MonsterAiState::Idle,
                                target_session, provoked: target_session.is_some(),
                                is_elite: false, is_boss: false,
                                min_ac: 0, max_ac: 0, min_mac: 0, max_mac: 0,
                                agility: 0, accuracy: 0,
                                armour_rate: 1.0, damage_rate: 1.0,
                                magic_resist: 0, critical_rate: 0, critical_damage: 0,
                                luck: 0, reflect: 0, damage_reduction_percent: 0,
                                poison_list: Vec::new(),
                                undead: info.undead,
                                master_session: Some(msg.session_id),
                                recall_at_tick: 0,
                                behavior: crate::actors::world::ai::make_behavior(&spawn.name),
                            });
                            self.pet_levels.insert(new_oid, spell_level as i32);
                            debug!("Magic: {} casts DarkBody -> clone #{} at ({},{})",
                                   state.name, new_oid, state.x, state.y);
                        } else {
                            warn!("DarkBody '{}' found index {} but no MonsterInfo", CLONE_NAME, idx);
                        }
                    }
                    None => {
                        warn!("DarkBody '{}' not in monster_name_index (DB may lack this mob)", CLONE_NAME);
                    }
                }
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
            // BladeAvalanche：冰刀斩（C# HumanObject.cs:4903）——3 列（前左/前/前右）×3 行前向 AoE
            // 前 2 行全额、第 3 行 60%；幸运暴击翻倍；MAC 防御
            SPELL_BLADE_AVALANCHE => {
                let mut raw = crate::combat::attack::get_attack_power(
                    state.min_attack + state.bonus_min_attack,
                    state.max_attack + state.bonus_max_attack,
                    state.luck,
                ).max(1);
                // C#：Random(0..100) <= 1+Luck → 翻倍
                if fastrand::i32(0..100) <= 1 + state.luck {
                    raw *= 2;
                }
                // C# Envir.cs BladeAvalanche：倍率 1+0.4Lv（幸运翻倍保留）
                let raw = ((raw as f32) * (1.0 + 0.4 * spell_level as f32)).max(1.0) as i32;
                let attacker_stats = state.to_combat_stats();
                let level_offset = state.level.min(10) as u16;
                let dir = msg.direction as usize % 8;
                let prev = (dir + 7) % 8;
                let next = (dir + 1) % 8;
                let mut hit_count = 0;
                for col_dir in [prev, dir, next] {
                    let start_x = state.x + MON_DIR_DX[col_dir];
                    let start_y = state.y + MON_DIR_DY[col_dir];
                    for j in 0..3i32 {
                        let hx = start_x + MON_DIR_DX[dir] * j;
                        let hy = start_y + MON_DIR_DY[dir] * j;
                        let cell_dmg = if j <= 1 { raw } else { ((raw as f64) * 0.6) as i32 };
                        let hit_ids: Vec<u32> = self.monsters.iter()
                            .filter(|(_, m)| m.x == hx && m.y == hy && m.hp > 0)
                            .map(|(id, _)| *id)
                            .collect();
                        for mid in hit_ids {
                            if let Some(monster) = self.monsters.get_mut(&mid) {
                                let ds = monster.to_combat_stats();
                                let r = combat_attack::resolve_attack(
                                    &attacker_stats, &ds, cell_dmg,
                                    mir2_shared::enums::DefenceType::Mac, level_offset,
                                );
                                if r.is_hit && r.damage > 0 {
                                    monster.take_damage(r.damage);
                                    monster.provoked = true;
                                    monster.target_session = Some(msg.session_id);
                                    hit_count += 1;
                                }
                            }
                        }
                    }
                }
                debug!("Magic: {} casts BladeAvalanche (3x3 front, hits {})", state.name, hit_count);
            }
            // CrescentSlash：前方扇形 AoE（前+左前+右前 3 格）
            SPELL_CRESCENT_SLASH => {
                let dir = msg.direction as usize % 8;
                let attacker_stats = state.to_combat_stats();
                let level_offset = state.level.min(10) as u16;
                let raw = crate::combat::attack::get_attack_power(
                    attacker_stats.min_atk, attacker_stats.max_atk, attacker_stats.luck);
                // C# Envir.cs CrescentSlash：倍率 1+0.4Lv
                let raw_damage = ((raw as f32) * (1.0 + 0.4 * spell_level as f32)).max(1.0) as i32;
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
                    self.broadcast_position_change(msg.session_id, tx, ty, msg.direction).await;
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
                self.broadcast_position_change(msg.session_id, tx, ty, back_dir as u8).await;
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

                // C# SummonXxx：已有同名存活宠物 → 召回（传送到施法者前方 1 格）并返回，不重复生成
                let existing: Option<u32> = self.monsters.iter()
                    .find(|(_, m)| m.master_session == Some(msg.session_id)
                        && m.name.eq_ignore_ascii_case(summon_name) && m.hp > 0)
                    .map(|(id, _)| *id);
                if let Some(oid) = existing {
                    if let Some(m) = self.monsters.get_mut(&oid) {
                        m.x = sx;
                        m.y = sy;
                        m.direction = dir as u8;
                        let mut walk_body = Vec::new();
                        walk_body.extend_from_slice(&oid.to_le_bytes());
                        walk_body.extend_from_slice(&sx.to_le_bytes());
                        walk_body.extend_from_slice(&sy.to_le_bytes());
                        walk_body.push(dir as u8);
                        let walk_packet = build_packet_bytes(
                            mir2_shared::enums::ServerPacketIds::ObjectWalk as i16, &walk_body);
                        for sid in self.players.keys() {
                            let _ = self.gate_ref.tell(SendToClient {
                                session_id: *sid,
                                data: walk_packet.clone(),
                            }).await;
                        }
                        debug!("Magic: {} recalls existing summon '{}' #{}", state.name, summon_name, oid);
                    }
                    return;
                }

                // C#：道士/法师召唤永久；弓手召唤 AliveTime：
                // Vampire=Lv*1500+15000ms，Toad=Lv*2000+25000ms，Snakes=Lv*1500+20000ms
                let recall_at_tick = match msg.spell {
                    SPELL_SUMMON_VAMPIRE => spell_level as u64 * 15 + 150,
                    SPELL_SUMMON_TOAD => spell_level as u64 * 20 + 250,
                    SPELL_SUMMON_SNAKES => spell_level as u64 * 15 + 200,
                    _ => 0,
                };

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
                                let _ = self.gate_ref.tell(SendToClient {
                                    session_id: *session_id,
                                    data: packet.clone(),
                                }).await;
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
                                recall_at_tick: if recall_at_tick > 0 { self.tick_count + recall_at_tick } else { 0 },
                                behavior: crate::actors::world::ai::make_behavior(&spawn.name),
                            });
                            // 记录召唤物等级（C# MonsterObject.PetLevel = magic.Level）
                            self.pet_levels.insert(new_oid, spell_level as i32);
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
            // Stonetrap：召唤“石头”宠物到目标点（C# HumanObject.cs:5739 ArcherSummonStone / 6724 CompleteMagic）
            SPELL_STONETRAP => {
                const STONE_NAME: &str = "StoneTrap";
                let (max_x, max_y) = self.maps.get(&state.map_index)
                    .map(|m| (m.width as i32, m.height as i32))
                    .unwrap_or((i32::MAX, i32::MAX));
                let sx = target_x.clamp(0, max_x - 1);
                let sy = target_y.clamp(0, max_y - 1);

                // 已存在存活石头 → 拒绝（C# Only one active Stone alive）
                let has_alive_stone = self.monsters.values().any(|m| {
                    m.master_session == Some(msg.session_id)
                        && m.name.eq_ignore_ascii_case(STONE_NAME)
                        && m.hp > 0
                });
                if has_alive_stone {
                    send_system_message(&self.gate_ref, msg.session_id, "已有一只存活的石阵，无法重复召唤");
                    return;
                }
                // 宠物数量超限 → 拒绝（C# Pets.Count >= magic.Level + 1）
                let pet_count = self.monsters.values()
                    .filter(|m| m.master_session == Some(msg.session_id))
                    .count();
                if pet_count >= spell_level as usize + 1 {
                    send_system_message(&self.gate_ref, msg.session_id, "召唤物数量已达上限");
                    return;
                }

                // 按名查 MonsterInfo（lowercase key，对齐 tick.rs boss_summons）
                let mon_index = self.monster_name_index.get(&STONE_NAME.to_lowercase()).copied();
                match mon_index {
                    Some(idx) => {
                        let info_opt = self.monster_infos.get(&idx).cloned();
                        if let Some(info) = info_opt {
                            let new_oid = self.alloc_object_id();
                            let hp = info.stats.get(&(mir2_shared::enums::Stat::HP as u8)).copied().unwrap_or(50);
                            let min_dmg = info.stats.get(&(mir2_shared::enums::Stat::MinDC as u8)).copied().unwrap_or(5);
                            let max_dmg = info.stats.get(&(mir2_shared::enums::Stat::MaxDC as u8)).copied().unwrap_or(10);
                            let spawn = MonsterSpawn {
                                name: info.name.clone(),
                                image: info.image as u16,
                                monster_index: idx,
                                x: sx,
                                y: sy,
                                direction: msg.direction,
                                hp,
                                min_dmg,
                                max_dmg,
                                xp: info.experience,
                                map_index: state.map_index,
                            };
                            let packet = build_object_monster_packet(&spawn, new_oid, &spawn.name);
                            for session_id in self.players.keys() {
                                let _ = self.gate_ref.tell(SendToClient {
                                    session_id: *session_id,
                                    data: packet.clone(),
                                }).await;
                            }
                            let ai_profile = MonsterAiProfile::from_info(&info);
                            // 石阵存活时长：C# DieTime = now + (level*5+10) 秒
                            let duration_ticks = (spell_level as u64 * 5 + 10) * 10;
                            self.monsters.insert(new_oid, MonsterState {
                                object_id: new_oid,
                                name: spawn.name.clone(),
                                image: spawn.image,
                                monster_index: idx,
                                x: sx, y: sy, direction: msg.direction,
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
                                recall_at_tick: self.tick_count + duration_ticks,
                                behavior: crate::actors::world::ai::make_behavior(&spawn.name),
                            });
                            debug!("Magic: {} casts Stonetrap '{}' as #{} at ({},{}) ({}s)",
                                state.name, STONE_NAME, new_oid, sx, sy, spell_level as u64 * 5 + 10);
                        } else {
                            warn!("Stonetrap '{}' found index {} but no MonsterInfo (DB missing mob)",
                                STONE_NAME, idx);
                            send_system_message(&self.gate_ref, msg.session_id, "召唤失败：怪物资料缺失");
                        }
                    }
                    None => {
                        warn!("Stonetrap '{}' not in monster_name_index (DB may lack this mob)", STONE_NAME);
                        send_system_message(&self.gate_ref, msg.session_id, "召唤失败：未知怪物");
                    }
                }
            }
            // ===== 特殊/辅助类法术（任务：补齐剩余主动法术）=====
            // --- 战士系 ---
            // LionRoar：嘲讽范围内怪物（吸引仇恨，对齐 C# WarriorObject.LionRoar）
            // 范围 = Range（默认 5 格），命中怪物 provoked + target_session=施法者
            SPELL_LION_ROAR | SPELL_BATTLE_CRY => {
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
            // ProtectionField：防护领域（C# HumanObject.cs ProtectionField）——
            // 仅自身 AC 提升：MaxAC/MinAC += round(MaxAC*(0.2+0.03Lv))，时长 45+15Lv 秒
            SPELL_PROTECTION_FIELD => {
                let add_value = (state.max_ac as f32 * (0.2 + 0.03 * spell_level as f32)).round() as i32;
                let duration_ticks = (45 + spell_level as u32 * 15) * 10;
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::AcDefenseBoost { bonus: add_value.max(1) },
                    duration_ticks,
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts ProtectionField (AC +{}, {}s)",
                       state.name, add_value.max(1), 45 + spell_level as i32 * 15);
            }
            // CounterAttack：反击（C# HumanObject.cs:8550）——施放进入 7 秒窗口，受击时反击并消耗
            SPELL_COUNTER_ATTACK => {
                if self.counter_attack.contains_key(&msg.session_id) {
                    debug!("Magic: {} casts CounterAttack but already active", state.name);
                    return;
                }
                self.counter_attack.insert(msg.session_id, (self.tick_count + 70, spell_level));
                debug!("Magic: {} arms CounterAttack (7s window)", state.name);
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
            // #318：TwinDrakeBlade —— 施放后 10 秒内下一次近战攻击双段伤害（C# HumanObject.cs:8530）
            SPELL_TWIN_DRAKE_BLADE => {
                self.double_hit_melee.insert(msg.session_id, (self.tick_count + 100, spell_level, 0));
                debug!("Magic: {} casts TwinDrakeBlade (next melee double-hit, 10s)", state.name);
            }
            // #318：DoubleSlash —— 同上双段近战（刺客）
            SPELL_DOUBLE_SLASH => {
                self.double_hit_melee.insert(msg.session_id, (self.tick_count + 100, spell_level, 1));
                debug!("Magic: {} casts DoubleSlash (next melee double-hit, 10s)", state.name);
            }
            // #318：SlashingBurst —— 前方第 1 格 DC 伤害（AC 防御）+ 向前冲刺 2 格
            // （C# HumanObject.cs:5159 + Map.cs：count=1 只结算 1 格，DefenceType.AC）
            SPELL_SLASHING_BURST => {
                let dir = msg.direction as usize % 8;
                // C# Envir.cs SlashingBurst：倍率 3.25+0.25Lv（DC）
                let raw = crate::combat::attack::get_attack_power(
                    state.min_attack + state.bonus_min_attack,
                    state.max_attack + state.bonus_max_attack,
                    state.luck,
                );
                let raw_damage = ((raw as f32) * (3.25 + 0.25 * spell_level as f32)).max(1.0) as i32;
                let mut new_x = state.x;
                let mut new_y = state.y;
                let mut slashed_damage = 0i32;
                for step in 0..2 {
                    let nx = new_x + MON_DIR_DX[dir];
                    let ny = new_y + MON_DIR_DY[dir];
                    let walkable = self.maps.get(&state.map_index)
                        .map(|m| m.is_walkable(nx, ny))
                        .unwrap_or(false);
                    if !walkable { break; }
                    let hit: Option<u32> = self.monsters.iter()
                        .find(|(_, m)| m.x == nx && m.y == ny && m.hp > 0)
                        .map(|(id, _)| *id);
                    if let Some(mid) = hit {
                        if let Some(m) = self.monsters.get_mut(&mid) {
                            // C#：只结算前方第 1 格（Map.cs SlashingBurst count=1），AC 防御
                            if step == 0 {
                                let attacker_stats = state.to_combat_stats();
                                let defender_stats = m.to_combat_stats();
                                let level_offset = state.level.min(10) as u16;
                                let r = combat_attack::resolve_attack(
                                    &attacker_stats, &defender_stats, raw_damage,
                                    mir2_shared::enums::DefenceType::Ac, level_offset,
                                );
                                if r.is_hit && r.damage > 0 {
                                    m.take_damage(r.damage);
                                    m.provoked = true;
                                    m.target_session = Some(msg.session_id);
                                    slashed_damage += r.damage;
                                }
                            }
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
                    self.broadcast_position_change(msg.session_id, new_x, new_y, msg.direction).await;
                }
                debug!("Magic: {} casts SlashingBurst (dashed to {},{}, dealt {} dmg)",
                       state.name, new_x, new_y, slashed_damage);
            }
            // #328：Plague —— 3×3 区域随机毒 + MaxSC×2 MAC 伤害（C# Map.cs:1972）
            SPELL_PLAGUE => {
                let value = if let Some(info) = spell_db {
                    crate::combat::magic::calc_magic_damage(info, spell_level, magic_stat)
                } else { fastrand::i32(5..=12) }.max(1);
                let damage = (magic_stat * 2).max(1);
                let attacker_stats = state.to_combat_stats();
                let level_offset = state.level.min(10) as u16;
                let cells = plague_cells(target_x, target_y);
                let hit_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| m.hp > 0 && cells.contains(&(m.x, m.y)))
                    .map(|(id, _)| *id)
                    .collect();
                let mut spell_hits: Vec<(u32, i32, i32, u8, i32)> = Vec::new();
                for mid in hit_ids {
                    // 随机毒（C# Map.cs 概率表）
                    let ptype = plague_poison(fastrand::i32(0..15));
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        let temp_value = plague_temp_value(value, spell_level, ptype);
                        if ptype != mir2_shared::enums::PoisonType::NONE {
                            let dur = plague_duration(spell_level, value).max(1) as u32;
                            crate::combat::poison::apply_poison(
                                &mut monster.poison_list,
                                crate::combat::poison::Poison::new(ptype, dur, temp_value, 1000),
                            );
                        }
                        let defender_stats = monster.to_combat_stats();
                        let r = combat_attack::resolve_attack(
                            &attacker_stats, &defender_stats, damage,
                            mir2_shared::enums::DefenceType::Mac, level_offset,
                        );
                        if r.is_hit && r.damage > 0 {
                            monster.take_damage(r.damage);
                            monster.provoked = true;
                            monster.target_session = Some(msg.session_id);
                            spell_hits.push((mid, monster.x, monster.y, monster.direction, r.damage));
                        }
                    }
                }
                self.broadcast_spell_hit(&spell_hits, object_id).await;
                debug!("Magic: {} casts Plague (3x3, {} hit, dmg={})", state.name, spell_hits.len(), damage);
            }
            // #328：Trap —— 目标怪物 60 秒麻痹（C# Map.cs:2048 ShockTime）
            SPELL_TRAP => {
                // C# Map.cs Trap：目标等级 >= 施法等级+2 时跳过
                let hit: Option<(u32, i32)> = self.monsters.iter()
                    .find(|(_, m)| m.x == target_x && m.y == target_y && m.hp > 0)
                    .map(|(id, m)| (*id, self.monster_infos.get(&m.monster_index).map(|i| i.level).unwrap_or(0)));
                if let Some((mid, mlevel)) = hit {
                    if mlevel >= state.level as i32 + 2 {
                        debug!("Magic: {} casts Trap -> monster {} level {} too high", state.name, mid, mlevel);
                        return;
                    }
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        crate::combat::poison::apply_poison(
                            &mut monster.poison_list,
                            crate::combat::poison::Poison::new(
                                mir2_shared::enums::PoisonType::PARALYSIS, 60, 0, 1000,
                            ),
                        );
                        monster.provoked = true;
                        monster.target_session = Some(msg.session_id);
                        debug!("Magic: {} casts Trap -> monster {} paralyzed 60s", state.name, mid);
                    }
                } else {
                    debug!("Magic: {} casts Trap (no target at {},{})", state.name, target_x, target_y);
                }
            }
            // #345：MoonMist —— 隐身 + 自身周围 5×5 AC 范围伤害（C# HumanObject.cs:4565 + Map.cs:1347）
            SPELL_MOON_MIST => {
                // C#：已有 MoonLight buff 时不重复施放
                if self.invisible_sessions.contains(&msg.session_id) {
                    debug!("Magic: {} casts MoonMist but already invisible, skipped", state.name);
                    return;
                }
                // C# 时长：(GetAttackPower(MinAC, MaxAC) + (Lv+1)*5) * 500ms
                let ac_power = crate::combat::attack::get_attack_power(
                    state.min_ac + state.bonus_min_ac,
                    state.max_ac + state.bonus_max_ac,
                    0,
                );
                let duration_ticks = ((ac_power + (spell_level as i32 + 1) * 5).max(1) as u32) * 5;
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::Invisibility,
                    duration_ticks,
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                self.invisible_sessions.insert(msg.session_id);
            if let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await {
                self.broadcast_object_hidden(st.object_id, true, st.map_index).await;
            }
                let raw_damage = (magic_stat + (power as i32) / 2).max(1);
                let attacker_stats = state.to_combat_stats();
                let level_offset = state.level.min(10) as u16;
                // C# Map.cs:1347：location ±2 = 5×5
                let cells = curse_cells_5x5(state.x, state.y);
                let hit_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| m.hp > 0 && cells.contains(&(m.x, m.y)))
                    .map(|(id, _)| *id)
                    .collect();
                let mut spell_hits: Vec<(u32, i32, i32, u8, i32)> = Vec::new();
                for mid in hit_ids {
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        let ds = monster.to_combat_stats();
                        let r = combat_attack::resolve_attack(
                            &attacker_stats, &ds, raw_damage,
                            mir2_shared::enums::DefenceType::Ac, level_offset,
                        );
                        if r.is_hit && r.damage > 0 {
                            monster.take_damage(r.damage);
                            monster.provoked = true;
                            monster.target_session = Some(msg.session_id);
                            spell_hits.push((mid, monster.x, monster.y, monster.direction, r.damage));
                        }
                    }
                }
                self.broadcast_spell_hit(&spell_hits, object_id).await;
                debug!("Magic: {} casts MoonMist (invisible {}s + 5x5 AC dmg={} hits={})",
                       state.name, duration_ticks / 10, raw_damage, spell_hits.len());
            }
            // #395：ImmortalSkin —— AC 提升 + DC 交换（C# HumanObject.cs:6171 CompleteMagic）
            // MaxDC = round(MaxDC * (0.05+0.01Lv)) * -1；MaxAC = round(MaxAC * (0.10+0.07Lv))
            SPELL_IMMORTAL_SKIN => {
                let ac_bonus = (state.max_ac as f32 * (0.10 + 0.07 * spell_level as f32)) as i32;
                let dc_penalty = ((state.max_attack + state.bonus_max_attack) as f32
                    * (0.05 + 0.01 * spell_level as f32)).round() as i32;
                let duration_ticks = ((60 + spell_level as i32) as u32) * 10;
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::AcDefenseBoost { bonus: ac_bonus.max(1) },
                    duration_ticks,
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                if dc_penalty > 0 {
                    let dc_buff = crate::combat::buff::BuffInstance::new(
                        crate::combat::buff::BuffType::AttackBoost { bonus: -dc_penalty },
                        duration_ticks,
                        5,
                    );
                    let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff: dc_buff }).await;
                }
                debug!("Magic: {} casts ImmortalSkin (AC +{}, DC -{}, {}s)",
                       state.name, ac_bonus.max(1), dc_penalty, 60 + spell_level as i32);
            }
            // #395：Hallucination —— 概率成功，怪物 10-29s 失去目标不攻击（C# HumanObject.cs:6342）
            SPELL_HALLUCINATION => {
                if hallucination_success(spell_level, state.level) {
                    let hit: Option<u32> = self.monsters.iter()
                        .find(|(_, m)| m.x == target_x && m.y == target_y && m.hp > 0)
                        .map(|(id, _)| *id);
                    if let Some(mid) = hit {
                        let dur = hallucination_duration();
                        let until = self.tick_count + dur as u64 * 10;
                        self.hallucinated.insert(mid, until);
                        if let Some(monster) = self.monsters.get_mut(&mid) {
                            monster.target_session = None;
                            monster.ai_state = crate::actors::world::MonsterAiState::Idle;
                        }
                        debug!("Magic: {} casts Hallucination -> monster {} confused {}s", state.name, mid, dur);
                    } else {
                        debug!("Magic: {} casts Hallucination (no target at {},{})", state.name, target_x, target_y);
                    }
                } else {
                    debug!("Magic: {} casts Hallucination (failed)", state.name);
                }
            }
            // #409：OneWithNature —— 5×5 AoE MAC 伤害 + 必中 Green 毒（C# Map.cs：持有 PoisonShot buff 时）
            // 吸血（VampireShot buff）暂不模拟
            SPELL_ONE_WITH_NATURE => {
                let raw_damage = if let Some(info) = spell_db {
                    crate::combat::magic::calc_magic_damage(info, spell_level, magic_stat)
                } else { fastrand::i32(8..=20) }.max(1);
                let attacker_stats = state.to_combat_stats();
                let level_offset = state.level.min(10) as u16;
                let cells = curse_cells_5x5(target_x, target_y);
                let hit_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| m.hp > 0 && cells.contains(&(m.x, m.y)))
                    .map(|(id, _)| *id)
                    .collect();
                let mut spell_hits: Vec<(u32, i32, i32, u8, i32)> = Vec::new();
                for mid in hit_ids {
                    if let Some(monster) = self.monsters.get_mut(&mid) {
                        let ds = monster.to_combat_stats();
                        let r = combat_attack::resolve_attack(
                            &attacker_stats, &ds, raw_damage,
                            mir2_shared::enums::DefenceType::Mac, level_offset,
                        );
                        if r.is_hit && r.damage > 0 {
                            monster.take_damage(r.damage);
                            monster.provoked = true;
                            monster.target_session = Some(msg.session_id);
                            spell_hits.push((mid, monster.x, monster.y, monster.direction, r.damage));
                        }
                        // C#：持有 PoisonShot buff 时必中绿毒（Duration = value*2 + (Lv+1)*7；
                        // Value = value/15 + Lv + 1 + Random(PoisonAttack)）
                        let dur = (raw_damage * 2 + (spell_level as i32 + 1) * 7).max(1) as u32;
                        let val = (raw_damage / 15 + spell_level as i32 + 1
                            + fastrand::i32(0..state.poison_attack.max(1))).max(1);
                        crate::combat::poison::apply_poison(&mut monster.poison_list,
                            crate::combat::poison::Poison::new(
                                mir2_shared::enums::PoisonType::GREEN, dur, val, 2000,
                            ));
                    }
                }
                self.broadcast_spell_hit(&spell_hits, object_id).await;
                debug!("Magic: {} casts OneWithNature (5x5, {} hit, dmg={})",
                       state.name, spell_hits.len(), raw_damage);
            }
            // #409：MentalState —— 模式 0/1/2 循环（C# HumanObject.cs:8571）
            SPELL_MENTAL_STATE => {
                let cur = self.mental_state.entry(msg.session_id).or_insert(0);
                *cur = (*cur + 1) % 3;
                let label = match *cur {
                    1 => "特技射击",
                    2 => "组队模式",
                    _ => "攻击模式",
                };
                send_system_message(&self.gate_ref, msg.session_id, &format!("精神状态切换到：{}", label));
                debug!("Magic: {} casts MentalState -> {}", state.name, label);
            }
            // #427：UltimateEnhancer —— 友方目标 DC/MC/SC 提升（C# HumanObject.cs:4784）
            // 按目标职业加成：战士/刺客→DC，法师/弓手→MC，道士→SC；C# 需 amulet（Rust 暂不实现门槛）
            SPELL_ULTIMATE_ENHANCER => {
                let sc = state.effective_max_sc();
                let value = if sc >= 5 { (sc / 5).min(8) } else { 1 };
                let duration_ticks = ((sc * 4 + (spell_level as i32 + 1) * 50) as u32) * 10;
                // 目标选择：msg.target_id 指向自己或同组玩家 → 对其施放；否则自己
                let mut target_session = msg.session_id;
                let mut target_class = state.class;
                if msg.target_id != 0 {
                    let mut found_any = false;
                    for (sid, r) in &self.players {
                        if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                            if os.object_id == msg.target_id {
                                found_any = true;
                                let friendly = *sid == msg.session_id
                                    || (os.group_id.is_some() && os.group_id == state.group_id);
                                if friendly {
                                    target_session = *sid;
                                    target_class = os.class;
                                }
                                break;
                            }
                        }
                    }
                    // 自己的召唤物目标：怪物无 buff 系统，按 DC 提升近似作用于自身
                    if !found_any || target_session == msg.session_id {
                        if self.monsters.get(&msg.target_id)
                            .map(|m| m.master_session == Some(msg.session_id))
                            .unwrap_or(false)
                        {
                            target_class = state.class; // DC（怪物默认）
                        }
                    }
                }
                let (buff, label) = match target_class {
                    mir2_shared::enums::MirClass::Wizard | mir2_shared::enums::MirClass::Archer =>
                        (crate::combat::buff::BuffType::McBoost { bonus: value }, "MC"),
                    mir2_shared::enums::MirClass::Taoist =>
                        (crate::combat::buff::BuffType::ScBoost { bonus: value }, "SC"),
                    _ => (crate::combat::buff::BuffType::AttackBoost { bonus: value }, "DC"),
                };
                let inst = crate::combat::buff::BuffInstance::new(buff, duration_ticks, 5);
                if target_session == msg.session_id {
                    let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff: inst }).await;
                } else if let Some(r) = self.players.get(&target_session) {
                    let _ = r.actor_ref.ask(crate::actors::player::ApplyBuff { buff: inst }).await;
                }
                debug!("Magic: {} casts UltimateEnhancer on session {} ({} +{}, {}s)",
                       state.name, target_session, label, value, duration_ticks / 10);
            }
            // #448：FatalSword —— C# 中为被动技能（近战 10% 触发），无主动施放分支；施放不消耗
            SPELL_FATAL_SWORD => {
                debug!("Magic: {} casts FatalSword (passive-only in C#, no active effect)", state.name);
            }
            // #448：PetEnhancer —— 召唤宠物 DC/AC 提升（C# HumanObject.cs:6363 CompleteMagic）
            // dcInc = 2 + 宠物等级*2；acInc = 4 + 宠物等级；时长 ≈ SC 秒（GetPower 默认 0）
            SPELL_PET_ENHANCER => {
                let sc = state.effective_max_sc();
                let duration_s = sc.max(1) as u32;
                let until = self.tick_count + (duration_s as u64) * 10;
                let pet: Option<u32> = self.monsters.iter()
                    .find(|(_, m)| m.master_session == Some(msg.session_id)
                        && (m.x - target_x).abs() <= 2 && (m.y - target_y).abs() <= 2)
                    .map(|(id, _)| *id);
                if let Some(pid) = pet {
                    let pet_lv = self.pet_levels.get(&pid).copied().unwrap_or(0);
                    let dc_inc = 2 + pet_lv * 2;
                    let ac_inc = 4 + pet_lv;
                    self.pet_enhanced.insert(pid, (until, dc_inc, ac_inc));
                    debug!("Magic: {} casts PetEnhancer -> pet {} (DC+{} AC+{}, {}s)",
                           state.name, pid, dc_inc, ac_inc, duration_s);
                } else {
                    debug!("Magic: {} casts PetEnhancer (no pet near {},{})", state.name, target_x, target_y);
                }
            }
            // #312：FlamingSword —— 施放后 10 秒内下一次近战攻击附加火焰加成（C# HumanObject.cs:8538）
            SPELL_FLAMING_SWORD => {
                self.flaming_sword.insert(msg.session_id, (self.tick_count + 100, spell_level));
                debug!("Magic: {} casts FlamingSword (next melee +{:.2}x, 10s)",
                       state.name, 1.4 + 0.4 * spell_level as f32);
            }
            // #312：EnergyShield —— 减伤 buff（C# HumanObject.cs:4751，chance=10-(Luck/3+Lv+1)，吸收百分比转 HP）
            SPELL_ENERGY_SHIELD => {
                let chance = (10 - (state.luck / 3 + spell_level as i32 + 1)).max(2);
                let percent = ((1.0 / chance as f32) * 100.0).round() as i32;
                let duration_ticks = ((30 + 50 * spell_level as i32) as u32) * 10;
                let _ = record.actor_ref.ask(crate::actors::player::ApplyDamageReduction {
                    percent,
                    duration_ticks,
                }).await;
                debug!("Magic: {} casts EnergyShield (damage -{}%, {}s)",
                       state.name, percent, 30 + 50 * spell_level as i32);
            }
            // Repulsion/EnergyRepulsor：推开周围怪物（C# 两者共用 Repulsion 方法）
            // 命中 1-2 格内怪物，将其沿反方向推 1-2 格（受 can_push 限制）
            SPELL_REPULSION | SPELL_ENERGY_REPULSOR | SPELL_FIRE_BURST => {
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
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: *session_id,
                            data: walk_packet.clone(),
                        }).await;
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
            // MagicBooster：MC 提升（C# HumanObject.cs:4345 + CompleteMagic 6228：MinMC/MaxMC += 6+Lv*6，60s）
            SPELL_MAGIC_BOOSTER => {
                let bonus = 6 + spell_level as i32 * 6;
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::McBoost { bonus },
                    600, // 60s
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts MagicBooster (MC +{})", state.name, bonus);
            }
            // --- 道士系 ---
            // Revelation：显血（C# HumanObject.cs:6284）——单目标（点击玩家/怪物），
            // Random(4)<=Lv 成功，value 秒内显示目标 HP（value = GetAttackPower(MinSC,MaxSC)+GetPower(0)）
            SPELL_REVELATION => {
                if fastrand::i32(0..4) > spell_level as i32 {
                    debug!("Magic: {} casts Revelation (failed)", state.name);
                    return;
                }
                let value = crate::combat::attack::get_attack_power(
                    state.min_sc + state.bonus_min_sc,
                    state.max_sc + state.bonus_max_sc,
                    0,
                ).max(1);
                let until = self.tick_count + (value as u64) * 10;
                // 目标：点击的玩家优先，其次点击格怪物
                let mut target_oid: Option<u32> = None;
                for (_sid, r) in &self.players {
                    if let Ok(Some(s)) = r.actor_ref.ask(GetPlayerState).await {
                        if s.object_id == msg.target_id {
                            target_oid = Some(s.object_id);
                            break;
                        }
                    }
                }
                if target_oid.is_none() {
                    target_oid = self.monsters.iter()
                        .find(|(_, m)| (m.x - target_x).abs() <= 1 && (m.y - target_y).abs() <= 1 && m.hp > 0)
                        .map(|(id, _)| *id);
                }
                if let Some(oid) = target_oid {
                    self.revealed_hp.insert(oid, until);
                    // 广播一次 ObjectHealth（客户端显示血条）
                    let (hp, max_hp) = if let Some(m) = self.monsters.get(&oid) {
                        (m.hp, m.max_hp)
                    } else {
                        let mut pos = (0i32, 1i32);
                        for (_sid, r) in &self.players {
                            if let Ok(Some(s)) = r.actor_ref.ask(GetPlayerState).await {
                                if s.object_id == oid {
                                    pos = (s.hp, s.max_hp);
                                    break;
                                }
                            }
                        }
                        pos
                    };
                    let percent = ((hp.max(0) as f32 / max_hp.max(1) as f32) * 100.0) as u8;
                    let mut body = Vec::new();
                    body.extend_from_slice(&oid.to_le_bytes());
                    body.push(percent);
                    body.extend_from_slice(&3u16.to_le_bytes());
                    let pkt = build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectHealth as i16, &body);
                    for sid in self.players.keys() {
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: *sid,
                            data: pkt.clone(),
                        }).await;
                    }
                    debug!("Magic: {} casts Revelation -> oid {} ({}s)", state.name, oid, value);
                } else {
                    debug!("Magic: {} casts Revelation (no target at {},{})", state.name, target_x, target_y);
                }
            }
            // Reincarnation：复活死亡玩家（对齐 C# TaoistObject.Reincarnation）
            // 实现：找附近 3 格死亡玩家 → OfferReincarnation（30s 有效期）+ RequestReincarnation（客户端确认）
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
                        // #222：对齐 C# offer/accept 链路——设置轮回状态并请求确认
                        let expire_tick = self.tick_count + 300; // 30s 有效期
                        let _ = other
                            .actor_ref
                            .ask(crate::actors::player::OfferReincarnation {
                                host_session: msg.session_id,
                                expire_tick,
                            })
                            .await;
                        // 发送 S.RequestReincarnation（空包）给死亡玩家
                        let req =
                            mir2_shared::packets::server::miscellaneous::RequestReincarnation {};
                        let mut body = Vec::new();
                        if req.write_body(&mut body).is_ok() {
                            let _ = self
                                .gate_ref
                                .tell(SendToClient {
                                    session_id: dead_sid,
                                    data: build_packet_bytes(
                                        mir2_shared::enums::ServerPacketIds::RequestReincarnation
                                            as i16,
                                        &body,
                                    ),
                                })
                                .await;
                        }
                        debug!(
                            "Magic: {} casts Reincarnation (offered player {})",
                            state.name, dead_sid
                        );
                        send_system_message(
                            &self.gate_ref,
                            msg.session_id,
                            "轮回术已施展，等待对方确认…",
                        );
                    }
                } else {
                    send_system_message(&self.gate_ref, msg.session_id, "附近没有可复活的目标");
                    debug!("Magic: {} casts Reincarnation but no target", state.name);
                }
            }
            // --- 刺客系 ---
            // PoisonSword：C# HumanObject.cs:5289 —— 前左起 5 格弧即时涂绿毒（需毒药道具，Rust 不实现门槛）
            SPELL_POISON_SWORD => {
                // C# power = magic.GetDamage(GetAttackPower(MinDC,MaxDC))；PoisonSword 无倍率/MPower 配置 → = DC
                let power = crate::combat::attack::get_attack_power(
                    state.min_attack + state.bonus_min_attack,
                    state.max_attack + state.bonus_max_attack,
                    state.luck,
                ).max(1);
                let front = msg.direction as usize % 8;
                let mut poisoned = 0;
                for k in 0..5usize {
                    let d = (front + 7 + k) % 8; // PreviousDir 起顺时针 5 个方向
                    let hx = state.x + MON_DIR_DX[d];
                    let hy = state.y + MON_DIR_DY[d];
                    let mid = self.monsters.iter()
                        .find(|(_, m)| m.x == hx && m.y == hy && m.hp > 0)
                        .map(|(id, _)| *id);
                    if let Some(mid) = mid {
                        if let Some(monster) = self.monsters.get_mut(&mid) {
                            // C#：Duration = 3 + power/10 + Lv*3；Value = power/10 + Lv + 1 + Random(PoisonAttack)
                            let duration = (3 + power / 10 + spell_level as i32 * 3).max(1) as u32;
                            let value = (power / 10 + spell_level as i32 + 1
                                + fastrand::i32(0..state.poison_attack.max(1))).max(1);
                            crate::combat::poison::apply_poison(
                                &mut monster.poison_list,
                                crate::combat::poison::Poison::new(
                                    mir2_shared::enums::PoisonType::GREEN, duration, value, 1000,
                                ),
                            );
                            monster.provoked = true;
                            monster.target_session = Some(msg.session_id);
                            poisoned += 1;
                        }
                    }
                }
                debug!("Magic: {} casts PoisonSword (arc 5, poisoned {})", state.name, poisoned);
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

#[cfg(test)]
mod spell_geometry_tests {
    use super::*;

    #[test]
    fn hellfire_lv0_single_line() {
        // 面朝下（dir=4），从 (10,10) 出发，4 格直线
        let cells = hellfire_cells(10, 10, 4, 0);
        assert_eq!(cells.len(), 4);
        assert_eq!(cells[0], (10, 11));
        assert_eq!(cells[3], (10, 14));
    }

    #[test]
    fn hellfire_lv3_three_lines() {
        // dir=4（下），Lv3 → 下 + 右下 + 左下，共 12 格
        let cells = hellfire_cells(10, 10, 4, 3);
        assert_eq!(cells.len(), 12);
        // 前 4 格为直线（下）
        assert_eq!(cells[0], (10, 11));
        assert_eq!(cells[4], (11, 11)); // 右下
        assert_eq!(cells[8], (9, 11)); // 左下
    }

    #[test]
    fn icethrust_target_and_splash() {
        let cells = icethrust_cells(10, 10, 2); // 右
        assert_eq!(cells.len(), 9);
        assert_eq!(cells[0], (11, 10)); // 主目标
        // 溅射含 (11,9) (11,11) (10,10) (12,10)
        assert!(cells.contains(&(11, 9)));
        assert!(cells.contains(&(11, 11)));
        assert!(cells.contains(&(10, 10)));
        assert!(cells.contains(&(12, 10)));
    }

    #[test]
    fn curse_area_7x7() {
        let cells = curse_cells(50, 60);
        assert_eq!(cells.len(), 49);
        assert!(cells.contains(&(47, 57)));
        assert!(cells.contains(&(53, 63)));
    }

    #[test]
    fn plague_area_3x3() {
        let cells = plague_cells(50, 60);
        assert_eq!(cells.len(), 9);
        assert!(cells.contains(&(49, 59)));
        assert!(cells.contains(&(51, 61)));
        assert!(cells.contains(&(50, 60)));
    }

    #[test]
    fn plague_poison_table() {
        use mir2_shared::enums::PoisonType;
        assert_eq!(plague_poison(0), PoisonType::SLOW);
        assert_eq!(plague_poison(2), PoisonType::SLOW);
        assert_eq!(plague_poison(3), PoisonType::FROZEN);
        assert_eq!(plague_poison(4), PoisonType::FROZEN);
        assert_eq!(plague_poison(5), PoisonType::GREEN);
        assert_eq!(plague_poison(9), PoisonType::GREEN);
        assert_eq!(plague_poison(10), PoisonType::NONE);
        assert_eq!(plague_poison(14), PoisonType::NONE);
    }

    #[test]
    fn plague_values() {
        use mir2_shared::enums::PoisonType;
        // Red：value/15 + Lv + 1
        assert_eq!(plague_temp_value(30, 3, PoisonType::RED), 6);
        // 其他：value + (Lv+1)*2
        assert_eq!(plague_temp_value(30, 3, PoisonType::GREEN), 38);
        // 持续：2*(Lv+1)+value/10
        assert_eq!(plague_duration(3, 30), 11);
    }

    #[test]
    fn mp_eater_restore_value() {
        assert_eq!(mp_eater_restore(3, 0), 15);
        assert_eq!(mp_eater_restore(3, 4), 20);
    }

    #[test]
    fn hemorrhage_values() {
        assert_eq!(hemorrhage_duration(3, 6), 7);
        assert_eq!(hemorrhage_duration(0, 0), 0);
        assert_eq!(hemorrhage_value(50), 51);
    }

    #[test]
    fn special_shot_buff_time_value() {
        assert_eq!(special_shot_buff_time(0), 5);
        assert_eq!(special_shot_buff_time(3), 20);
    }

    #[test]
    fn hallucination_duration_range() {
        for _ in 0..100 {
            let d = hallucination_duration();
            assert!((10..=29).contains(&d), "duration out of range: {}", d);
        }
    }

    #[test]
    fn one_with_nature_area_5x5() {
        let cells = curse_cells_5x5(50, 60);
        assert_eq!(cells.len(), 25);
        assert!(cells.contains(&(48, 58)));
        assert!(cells.contains(&(52, 62)));
    }

    #[test]
    fn hallucination_success_high_level() {
        // 高等级：roll 范围很大，失败阈值 10 → 几乎必成功（10000 次抽样至少成功一次）
        let mut ok = false;
        for _ in 0..10000 {
            if hallucination_success(3, 30) {
                ok = true;
                break;
            }
        }
        assert!(ok);
    }
}
