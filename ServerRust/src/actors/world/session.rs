use super::*;

impl WorldActor {
    /// 登录公告（C# Settings.Notice + S.UpdateNotice）
    async fn send_login_notice(&mut self, session_id: u64, character_name: &str) {
        // 读取 Notice.txt（首行 `Title=`，其余为消息；C# Settings.LoadNotice）
        let path = std::path::Path::new(&self.notice_path);
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return, // 无公告文件
        };
        let mut lines = content.lines();
        let mut title = String::new();
        if let Some(first) = lines.next() {
            if let Some(t) = first.strip_prefix("Title=") {
                title = t.trim().to_string();
            }
        }
        let message = lines.collect::<Vec<_>>().join("\n");
        if message.trim().is_empty() {
            return;
        }
        let last_update = std::fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let last_access = db::get_character_last_access(&self.db_pool, character_name).await.unwrap_or(0);
        if last_update <= last_access {
            return;
        }
        let packet = mir2_shared::packets::server::ui_events::UpdateNotice {
            notice: mir2_shared::data::notice::Notice { title, message },
        };
        let mut body = Vec::new();
        if packet.write_body(&mut body).is_ok() {
            let _ = self.gate_ref.tell(SendToClient {
                session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::UpdateNotice as i16, &body),
            }).await;
        }
        info!("Login notice sent to {} (session={})", character_name, session_id);
    }
}

/// 开始游戏请求（从 GateActor 转发）
pub struct StartGameRequest {
    pub session_id: u64,
    pub character_index: i32,
    pub account_username: String,
}

/// 移动请求（从 GateActor 转发）
pub struct WorldMoveRequest {
    pub session_id: u64,
    pub direction: u8,
    pub is_run: bool,
}

/// 转向请求（从 GateActor 转发）
pub struct WorldTurnRequest {
    pub session_id: u64,
    pub direction: u8,
}

/// 玩家断开连接
pub struct PlayerDisconnected {
    pub session_id: u64,
}

/// 玩家主动登出（从 GateActor 转发）
pub struct PlayerLogOut {
    pub session_id: u64,
}

/// 聊天请求（从 GateActor 转发）
pub struct ChatRequest {
    pub session_id: u64,
    pub message: String,
}

/// 切换攻击模式请求（从 GateActor 转发）
pub struct ChangeAModeRequest {
    pub session_id: u64,
    pub mode: mir2_shared::enums::AttackMode,
}

/// 切换宠物模式请求（从 GateActor 转发）
pub struct ChangePModeRequest {
    pub session_id: u64,
    pub mode: mir2_shared::enums::PetMode,
}

/// 设置技能快捷键请求（从 GateActor 转发）
pub struct SetSpellKeyRequest {
    pub session_id: u64,
    pub spell: i32,
    pub key: u8,
    pub old_key: u8,
}

/// 技能开关切换请求（从 GateActor 转发）
pub struct SpellToggleRequest {
    pub session_id: u64,
    pub spell: i32,
    pub can_use: i8,
}

/// 设置英雄行为模式请求（从 GateActor 转发）
pub struct SetHeroBehaviourRequest {
    pub session_id: u64,
    pub behaviour: u8,
}

/// 设置自动药水阈值请求（从 GateActor 转发）
pub struct SetAutoPotValueRequest {
    pub session_id: u64,
    pub stat: u8,
    pub value: u32,
}

/// 设置自动药水物品请求（从 GateActor 转发）
pub struct SetAutoPotItemRequest {
    pub session_id: u64,
    pub grid: u8,
    pub item_index: i32,
}

/// 从装备插槽移除物品请求（从 GateActor 转发）
pub struct RemoveSlotItemRequest {
    pub session_id: u64,
    pub grid: u8,
    pub grid_to: u8,
    pub unique_id: u64,
    pub to: i32,
    pub from_unique_id: u64,
}

impl Message<StartGameRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: StartGameRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        info!(
            "StartGame: session={}, account={}, character_index={}",
            msg.session_id, msg.account_username, msg.character_index
        );

        // 尝试从数据库加载角色
        let mut state: Option<PlayerState> = None;
        match db::list_characters_by_account(&self.db_pool, &msg.account_username).await {
            Ok(chars) => {
                if !chars.is_empty() {
                    let idx = msg.character_index.max(0) as usize;
                    if idx < chars.len() {
                        let (char_name, _map_idx, _x, _y) = &chars[idx];
                        info!("Loading character '{}' for account '{}'", char_name, msg.account_username);
                        if let Ok(Some(loaded)) = db::load_character(&self.db_pool, char_name).await {
                            state = Some(loaded);
                        } else {
                            warn!("Failed to load character '{}' from DB", char_name);
                        }
                    }
                } else {
                    info!("No characters found for account '{}'", msg.account_username);
                }
            }
            Err(e) => {
                warn!("Failed to list characters for account '{}': {}", msg.account_username, e);
            }
        }

        // C#：找不到角色 → S.StartGame { Result = 2 }（不再隐式创建默认角色，避免绕过角色上限）
        let state = match state {
            Some(s) => s,
            None => {
                let packet = mir2_shared::packets::server::login::StartGame { result: 2, resolution: 0 };
                let mut body = Vec::new();
                if packet.write_body(&mut body).is_ok() {
                    let _ = self.gate_ref.tell(SendToClient {
                        session_id: msg.session_id,
                        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::StartGame as i16, &body),
                    }).await;
                }
                warn!("StartGame rejected: character_index {} not found for account {}", msg.character_index, msg.account_username);
                return;
            }
        };

        // C# Settings.AllowStartGame：非 GM 且关闭时 → S.StartGame{Result=0}（GM 用加载角色的 is_gm 判断）
        if !self.social_ref.ask(crate::actors::social::NpcGetAllowStartGame).await.unwrap_or(true) && !state.is_gm {
            let packet = mir2_shared::packets::server::login::StartGame { result: 0, resolution: 0 };
            let mut body = Vec::new();
            if packet.write_body(&mut body).is_ok() {
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::StartGame as i16, &body),
                }).await;
            }
            warn!("StartGame rejected: AllowStartGame=false for account {}", msg.account_username);
            return;
        }

        let object_id = self.alloc_object_id();
        let player_name = state.name.clone();
        let map_index = state.map_index;

        // 创建 PlayerActor
        let player_ref = PlayerActor::spawn((
            object_id,
            player_name.clone(),
            msg.session_id,
            map_index,
            self.gate_ref.clone(),
            self.self_ref.clone().expect("world self_ref set"),
        ));

        // 加载地图 — 优先用 DB 中的 map_infos 获取文件名
        // Phase A fix: 如果 map_index 在 DB 里找不到(idx 从 1 开始),fallback 到第一张可用地图
        let (map_file, map_title, map_info_idx) = if let Some(m) = self.map_infos.get(&(map_index as i32)) {
            (m.file_name.clone(), m.title.clone(), m.index)
        } else if let Some(first) = self.map_infos.values().next() {
            info!("map_index {} not in DB, using first available: {} ({})", map_index, first.file_name, first.title);
            (first.file_name.clone(), first.title.clone(), first.index)
        } else {
            ("0".to_string(), "Unknown".to_string(), 0) // "0" = first .map file
        };

        let map_slot = map_info_idx as u16;
        if self.get_or_load_map(&map_file, map_slot).is_some() {
            info!("Map '{}' loaded for player {} (slot {})", map_file, player_name, map_slot);
        }

        // 注入地图数据（按真实 map_index 查找）
        if let Some(map_data) = self.maps.get(&map_slot).cloned() {
            let _ = player_ref.ask(SetMapData { map: map_data }).await;
        }

        // 注入数据库加载的状态
        let mut loaded_state = state;
        loaded_state.object_id = object_id;
        loaded_state.session_id = msg.session_id;

        // 位置为 (0,0) 或不可走时，放回地图安全区出生点（#57：
        // Type1 地图解析修正前保存的坐标可能是墙内孤岛，导致玩家无法移动/寻路）
        let pos_walkable = self
            .maps
            .get(&map_slot)
            .map(|m| m.is_valid(loaded_state.x, loaded_state.y) && m.is_walkable(loaded_state.x, loaded_state.y))
            .unwrap_or(false);
        if (loaded_state.x == 0 && loaded_state.y == 0) || !pos_walkable {
            if let Some(mi) = self.map_infos.get(&(map_index as i32)) {
                if let Some(sz) = mi.safe_zones.iter().find(|s| s.start_point) {
                    info!(
                        "Placing {} at safe zone spawn ({}, {}){}",
                        player_name,
                        sz.x,
                        sz.y,
                        if pos_walkable { "" } else { "（原位置不可走）" }
                    );
                    loaded_state.x = sz.x;
                    loaded_state.y = sz.y;
                }
            }
        }

        // C# StartGame（PlayerObject.cs:1073）：当前地图 NoReconnect → 登录时传送到 NoReconnectMap 随机点
        if let Some(mi) = self.map_infos.get(&(map_index as i32)).cloned() {
            if mi.no_reconnect && !mi.no_reconnect_map.is_empty() {
                if let Some(dest_mi) = self.map_infos.values()
                    .find(|m| m.file_name.eq_ignore_ascii_case(&mi.no_reconnect_map))
                    .cloned()
                {
                    let dest_map_index = dest_mi.index as u16;
                    self.get_or_load_map(&dest_mi.file_name, dest_map_index);
                    let (rx, ry) = if let Some(map) = self.maps.get(&dest_map_index) {
                        let mut pt = (map.width as i32 / 2, map.height as i32 / 2);
                        for _ in 0..40 {
                            let cx = fastrand::i32(0..map.width as i32);
                            let cy = fastrand::i32(0..map.height as i32);
                            if map.is_walkable(cx, cy) {
                                pt = (cx, cy);
                                break;
                            }
                        }
                        pt
                    } else {
                        (330, 330)
                    };
                    info!("NoReconnect: moving {} from map {} to {} ({},{})",
                          player_name, map_index, dest_mi.file_name, rx, ry);
                    loaded_state.map_index = dest_map_index;
                    loaded_state.x = rx;
                    loaded_state.y = ry;
                }
            }
        }

        // 初始化装备属性加成（从已装备物品计算）
        let b = calculate_equipment_bonuses(&loaded_state.inventory.equipment, &self.item_infos);
        loaded_state.bonus_min_attack = b.min_atk;
        loaded_state.bonus_max_attack = b.max_atk;
        loaded_state.bonus_defence = b.max_ac;
        loaded_state.bonus_max_hp = b.hp;
        loaded_state.bonus_max_mp = b.mp;
        loaded_state.bonus_min_mc = b.min_mc;
        loaded_state.bonus_max_mc = b.max_mc;
        loaded_state.bonus_min_sc = b.min_sc;
        loaded_state.bonus_max_sc = b.max_sc;
        // 战斗公式扩展字段
        loaded_state.bonus_min_ac = b.min_ac;
        loaded_state.bonus_max_ac = b.max_ac;
        loaded_state.bonus_min_mac = b.min_mac;
        loaded_state.bonus_max_mac = b.max_mac;
        loaded_state.luck = b.luck;
        loaded_state.critical_rate = b.critical_rate;
        loaded_state.critical_damage = b.critical_damage;
        loaded_state.magic_resist = b.magic_resist;
        loaded_state.reflect = b.reflect;
        loaded_state.attack_bonus = b.attack_bonus;
        loaded_state.hp_drain_rate_percent = b.hp_drain_rate_percent;
        loaded_state.freezing = b.freezing;
        loaded_state.poison_attack = b.poison_attack;
        loaded_state.health_recovery = b.health_recovery;
        loaded_state.spell_recovery = b.spell_recovery;
        loaded_state.attack_speed = b.attack_speed;
        loaded_state.poison_resist = b.poison_resist;
        loaded_state.holy = b.holy;

        // 给装备/背包物品补 ItemInfo（含 special_mode，供复活戒指等逻辑读取）
        for slot in loaded_state.inventory.equipment.iter_mut() {
            if let Some(item) = slot {
                super::enrich_item_info(item, &self.item_infos);
            }
        }
        for slot in loaded_state.inventory.backpack.iter_mut() {
            if let Some(s) = slot {
                super::enrich_item_info(&mut s.item, &self.item_infos);
            }
        }

        let _ = player_ref.ask(SetPlayerState { state: loaded_state.clone() }).await;

        self.players.insert(msg.session_id, PlayerRecord {
            actor_ref: player_ref.clone(),
            session_id: msg.session_id,
            name: player_name.clone(),
            account_username: msg.account_username.clone(),
            last_pk_points: loaded_state.pk_points,
            last_colour: 0,
            object_id: loaded_state.object_id,
            world_map_setup_sent: false,
        });

        info!("Player {} entered world (object_id={}, session={})",
              player_name, object_id, msg.session_id);

        // C# PlayerObject.SetBind：确保绑定点有效（无绑定点/无效时随机出生安全区）
        self.ensure_bind(msg.session_id).await;

        // C# PlayerObject.StartGame → SetLevelEffects：按 flags 990-998 刷新等级特效
        self.refresh_level_effects(msg.session_id).await;
        // 广播 ObjectLevelEffects（覆盖初始 ObjectPlayer 的 0 特效；C# Enqueue + Broadcast）
        self.broadcast_level_effects(msg.session_id).await;

        // C# StartGame：下发玩家所属行会的激活 Buff 列表（S.GuildBuffList）
        if let Some(guild_name) = &loaded_state.guild_name {
            let buffs = self.social_ref.ask(crate::actors::social::NpcGetGuildBuffs {
                guild_name: guild_name.clone(),
            }).await.unwrap_or_default();
            let packet = mir2_shared::packets::server::special_systems::GuildBuffList {
                active_buffs: buffs.iter().map(|b| *b as i32).collect(),
            };
            let mut body = Vec::new();
            if packet.write_body(&mut body).is_ok() {
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GuildBuffList as i16, &body),
                }).await;
            }
        }

        // #188：下发英雄列表（ManageHeroes）
        // #194：从 DB 载入英雄（重启不丢）
        if let Ok(db_heroes) = db::load_heroes(&self.db_pool, &player_name).await {
            self.player_heroes.insert(msg.session_id, db_heroes.into_iter().map(|h| HeroInfo {
                index: h.index,
                name: h.name,
                level: h.level,
                class: mir2_shared::enums::MirClass::try_from(h.class).unwrap_or(mir2_shared::enums::MirClass::Warrior),
                gender: mir2_shared::enums::MirGender::try_from(h.gender).unwrap_or(mir2_shared::enums::MirGender::Male),
                dead: h.dead,
                sealed: h.sealed,
            }).collect());
        }
        let heroes = self.player_heroes.get(&msg.session_id).cloned().unwrap_or_default();
        send_manage_heroes_packet(&self.gate_ref, msg.session_id, &loaded_state, &heroes);
        // #198：有英雄则生成英雄对象
        if loaded_state.hero_index > 0 {
            self.broadcast_hero_spawn(msg.session_id).await;
            // #203：下发完整英雄信息（背包/装备/自动药）
            self.send_hero_information_packet(msg.session_id).await;
        }

        // 通知 SocialActor 玩家上线（组队/好友/行会查询依赖在线表）
        let _ = self.social_ref.tell(crate::actors::social::SocialPlayerJoined {
            session_id: msg.session_id,
            actor_ref: player_ref.clone(),
            name: player_name.clone(),
        }).try_send();

        // C# ApplyMapEntryRules：登录进入世界后应用地图规则（NoGroup/NoPets/NoIntelligentCreatures/NoHero）
        super::npc_script::apply_map_entry_rules(self, msg.session_id).await;

        // C# PlayerObject.cs:1172：公告非空且文件修改时间 > 上次下线时间 → S.UpdateNotice
        self.send_login_notice(msg.session_id, &player_name).await;

        // 行会在线状态由 SocialActor 管理

        // 发送玩家自身的 ObjectPlayer（客户端据此生成本地玩家实体并驱动移动/拾取）
        let self_weapon = loaded_state.inventory.get_equipment(EquipmentSlot::Weapon)
            .and_then(|item| self.item_infos.get(&item.item_index))
            .map(|info| info.shape as i16).unwrap_or(-1);
        let self_armor = loaded_state.inventory.get_equipment(EquipmentSlot::Armour)
            .and_then(|item| self.item_infos.get(&item.item_index))
            .map(|info| info.shape as i16).unwrap_or(0);
        let self_weapon_effect = loaded_state.inventory.get_equipment(EquipmentSlot::Weapon)
            .and_then(|item| self.item_infos.get(&item.item_index))
            .map(|info| info.effect as i16).unwrap_or(0);
        let self_packet = build_object_player_packet(
            &player_name, object_id, loaded_state.x, loaded_state.y, loaded_state.direction,
            loaded_state.level, name_colour_for_pk(loaded_state.pk_points, is_brown(loaded_state.brown_until_ms)),
            loaded_state.class, loaded_state.gender, loaded_state.hair,
            self_weapon, self_weapon_effect, self_armor,
            loaded_state.mount_type, loaded_state.is_mounted,
            loaded_state.level_effects,
        );
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: self_packet,
        }).await;

        // 多玩家可见性：向新玩家发送已有玩家的 ObjectPlayer
        let existing_players: Vec<_> = self.players.values()
            .filter(|r| r.session_id != msg.session_id)
            .cloned()
            .collect();

        let invis_tag = std::mem::discriminant(&crate::combat::buff::BuffType::Invisibility);
        for existing in &existing_players {
            if let Ok(Some(ep_state)) = existing.actor_ref.ask(GetPlayerState).await {
                // 跳过隐身玩家
                let is_invisible = ep_state.buffs.iter()
                    .any(|b| std::mem::discriminant(&b.buff_type) == invis_tag);
                if is_invisible { continue; }
                let ep_weapon = ep_state.inventory.get_equipment(EquipmentSlot::Weapon)
                    .and_then(|item| self.item_infos.get(&item.item_index))
                    .map(|info| info.shape as i16).unwrap_or(-1);
                let ep_armor = ep_state.inventory.get_equipment(EquipmentSlot::Armour)
                    .and_then(|item| self.item_infos.get(&item.item_index))
                    .map(|info| info.shape as i16).unwrap_or(0);
                let ep_weapon_effect = ep_state.inventory.get_equipment(EquipmentSlot::Weapon)
                    .and_then(|item| self.item_infos.get(&item.item_index))
                    .map(|info| info.effect as i16).unwrap_or(0);
                let packet = build_object_player_packet(
                    &ep_state.name, ep_state.object_id, ep_state.x, ep_state.y, ep_state.direction, ep_state.level,
                    name_colour_for_pk(ep_state.pk_points, is_brown(ep_state.brown_until_ms)),
                    ep_state.class, ep_state.gender, ep_state.hair,
                    ep_weapon, ep_weapon_effect, ep_armor,
                    ep_state.mount_type, ep_state.is_mounted,
                    ep_state.level_effects,
                );
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: msg.session_id,
                    data: packet,
                }).await;
            }
        }

        // 向已有玩家发送新玩家的 ObjectPlayer（隐身新玩家不发送）
        let new_is_invisible = loaded_state.buffs.iter()
            .any(|b| std::mem::discriminant(&b.buff_type) == invis_tag);
        if new_is_invisible {
            self.invisible_sessions.insert(msg.session_id);
        }
        if !new_is_invisible {
            let new_weapon = loaded_state.inventory.get_equipment(EquipmentSlot::Weapon)
                .and_then(|item| self.item_infos.get(&item.item_index))
                .map(|info| info.shape as i16).unwrap_or(-1);
            let new_armor = loaded_state.inventory.get_equipment(EquipmentSlot::Armour)
                .and_then(|item| self.item_infos.get(&item.item_index))
                .map(|info| info.shape as i16).unwrap_or(0);
            let new_weapon_effect = loaded_state.inventory.get_equipment(EquipmentSlot::Weapon)
                .and_then(|item| self.item_infos.get(&item.item_index))
                .map(|info| info.effect as i16).unwrap_or(0);
            let new_player_packet = build_object_player_packet(
                &player_name, object_id, loaded_state.x, loaded_state.y, loaded_state.direction, loaded_state.level,
                name_colour_for_pk(loaded_state.pk_points, is_brown(loaded_state.brown_until_ms)),
                loaded_state.class, loaded_state.gender, loaded_state.hair,
                new_weapon, new_weapon_effect, new_armor,
                loaded_state.mount_type, loaded_state.is_mounted,
                loaded_state.level_effects,
            );
            for existing in &existing_players {
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: existing.session_id,
                    data: new_player_packet.clone(),
                }).await;
            }
        }

        // 发送游戏进入序列（使用真实状态数据）
        let is_big_map = self.map_infos.get(&map_info_idx).map(|m| m.big_map).unwrap_or(false);
        send_game_entry_sequence(
            self.gate_ref.clone(),
            msg.session_id,
            &loaded_state,
            &map_file,
            &map_title,
            is_big_map,
            &self.item_infos,
        ).await;

        // 发送地图上的 NPC 和怪物
        let spawn_dir = self.spawn_dir.clone();
        let spawn_ctx = SpawnContext {
            map_info: self.map_infos.get(&map_info_idx),
            monster_infos: &self.monster_infos,
            npc_infos: &self.npc_infos,
            dragon_info: self.dragon_info.as_ref(),
            rarity: self.rarity_cfg.clone(),
        };
        let (new_npcs, new_monsters) = spawn_npcs_and_monsters(
            self.gate_ref.clone(),
            &spawn_dir,
            &map_file,
            loaded_state.map_index,
            msg.session_id,
            &mut self.next_object_id,
            &spawn_ctx,
        ).await;
        for npc in new_npcs {
            self.npcs.insert(npc.object_id, npc);
        }
        // M53：发送 NewMapInfo（大地图 NPC 列表，供 BigMapDialog 显示）
        let map_npcs: Vec<crate::actors::world::NpcState> = self.npcs.values()
            .filter(|n| n.map_index == loaded_state.map_index)
            .cloned()
            .collect();
        if !map_npcs.is_empty() {
            let new_map_info = super::build_new_map_info_packet(
                map_info_idx,
                &map_title,
                &map_npcs,
                &self.npc_infos,
            );
            let _ = self.gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: new_map_info,
            }).await;
            info!("NewMapInfo: map={} npcs={}", map_info_idx, map_npcs.len());
        }
        // #302：世界地图配置（C# CheckMapInfo 每连接下发一次）
        if let Some(rec) = self.players.get_mut(&msg.session_id) {
            if !rec.world_map_setup_sent {
                rec.world_map_setup_sent = true;
                let wm = super::build_world_map_setup_packet(&self.map_infos, super::TELEPORT_TO_NPC_COST);
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: msg.session_id,
                    data: wm,
                }).await;
                info!("WorldMapSetup: sent to session {}", msg.session_id);
            }
        }
        // 先收集精英广播信息（move 前遍历）
        let elite_broadcasts: Vec<String> = new_monsters.iter()
            .filter(|m| m.is_elite)
            .map(|m| m.name.clone())
            .collect();
        for monster in new_monsters {
            self.monsters.insert(monster.object_id, monster);
        }

        // 初始生成精英广播
        for name in &elite_broadcasts {
            let map_name = self.map_infos.get(&(map_index as i32)).map(|m| m.title.clone()).unwrap_or_else(|| "未知地图".to_string());
            broadcast_system_message(&self.gate_ref, &self.players,
                &format!("一只 [精英]{} 出现在 {}！勇士们，前往讨伐！", name.strip_prefix("[精英] ").unwrap_or(name), map_name));
        }

        // 同步当前地图上的地面物品给新玩家
        let map_index_val = loaded_state.map_index;
        let ground_sync: Vec<_> = self.ground_items.iter()
            .filter(|gi| gi.map_index == map_index_val)
            .map(|gi| (gi.object_id, gi.item.clone(), gi.x, gi.y))
            .collect();
        for (drop_oid, item, x, y) in ground_sync {
            if item.item_index == 0 {
                let object_gold = mir2_shared::packets::server::ObjectGold {
                    object_id: drop_oid,
                    gold: item.count as u32,
                    location_x: x,
                    location_y: y,
                };
                let mut buf = Vec::new();
                if mir2_shared::packets::base::serialize_packet(
                    &mut std::io::Cursor::new(&mut buf), &object_gold).is_ok() {
                    let _ = self.gate_ref.tell(SendToClient { session_id: msg.session_id, data: buf }).await;
                }
            } else {
                let object_item = mir2_shared::packets::server::ObjectItem {
                    object_id: drop_oid,
                    item,
                    location_x: x,
                    location_y: y,
                };
                let mut buf = Vec::new();
                if mir2_shared::packets::base::serialize_packet(
                    &mut std::io::Cursor::new(&mut buf), &object_item).is_ok() {
                    let _ = self.gate_ref.tell(SendToClient { session_id: msg.session_id, data: buf }).await;
                }
            }
        }

        // 同步当前地图上已打开的门给新玩家
        let open_doors_sync: Vec<_> = self.open_doors.iter()
            .filter(|(map_idx, _)| *map_idx == map_index_val)
            .map(|(_, door_idx)| *door_idx)
            .collect();
        for door_idx in open_doors_sync {
            send_opendoor(&self.gate_ref, msg.session_id, door_idx, false).await;
        }

        // 发送已学习的技能列表给客户端
        for magic in &loaded_state.magics {
            if let Some(info) = self.magic_infos.get(&(magic.spell as u32)) {
                let client_magic = super::build_client_magic(info, magic);
                let new_magic = mir2_shared::packets::server::magic::NewMagic {
                    magic: client_magic,
                    hero: false,
                };
                let mut body = Vec::new();
                if new_magic.write_body(&mut body).is_ok() {
                    let _ = self.gate_ref.tell(SendToClient {
                        session_id: msg.session_id,
                        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::NewMagic as i16, &body),
                    }).await;
                }
                // Send SpellToggle for toggled-on spells
                if magic.toggled {
                    let mut toggle_body = Vec::new();
                    toggle_body.extend_from_slice(&loaded_state.object_id.to_le_bytes());
                    toggle_body.push(magic.spell as u8);
                    toggle_body.push(1u8); // canUse = true
                    let _ = self.gate_ref.tell(SendToClient {
                        session_id: msg.session_id,
                        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::SpellToggle as i16, &toggle_body),
                    }).await;
                }
            }
        }

        // 发送当前昼夜光照给新玩家
        self.send_time_of_day(msg.session_id, self.current_light);

        // 发送自动药水设置（恢复持久化数据）
        if loaded_state.auto_pot_hp > 0 {
            let mut body = Vec::new();
            body.push(12u8); // Stat = HP (C# Stat.HP = 12)
            body.extend_from_slice(&loaded_state.auto_pot_hp.to_le_bytes());
            let _ = self.gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::SetAutoPotValue as i16, &body),
            }).await;
        }
        if loaded_state.auto_pot_mp > 0 {
            let mut body = Vec::new();
            body.push(13u8); // Stat = MP (C# Stat.MP = 13)
            body.extend_from_slice(&loaded_state.auto_pot_mp.to_le_bytes());
            let _ = self.gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::SetAutoPotValue as i16, &body),
            }).await;
        }

        // 发送欢迎消息
        let online_count = self.players.len();
        let light_name = match self.current_light {
            mir2_shared::enums::LightSetting::Dawn => "黎明",
            mir2_shared::enums::LightSetting::Day => "白天",
            mir2_shared::enums::LightSetting::Evening => "黄昏",
            mir2_shared::enums::LightSetting::Night => "夜晚",
            _ => "正常",
        };
        send_system_message(&self.gate_ref, msg.session_id,
            &format!("欢迎来到水晶世界！当前在线玩家: {} 人，当前时间: {}", online_count, light_name));
    }
}

impl Message<WorldMoveRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: WorldMoveRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => {
                warn!("Move request for unknown session {}", msg.session_id);
                return;
            }
        };
        if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
            if state.is_dead { return; }
        }

        // Phase 1.4: 反作弊 — 速度 hack 检测
        // 正常移动间隔: Walk ~200ms, Run ~150ms。
        // 阈值 50ms 容忍网络抖动,但拒绝明显的瞬移/速度 hack。
        const MIN_MOVE_INTERVAL_MS: u64 = 50;
        if let Some(last) = self.last_move_time.get(&msg.session_id) {
            let elapsed = last.elapsed();
            if elapsed < std::time::Duration::from_millis(MIN_MOVE_INTERVAL_MS) {
                warn!(
                    "Speed hack detected: session {} moved after {:?} (min={:?})",
                    msg.session_id, elapsed, MIN_MOVE_INTERVAL_MS
                );
                return; // 拒绝移动
            }
        }
        self.last_move_time.insert(msg.session_id, std::time::Instant::now());

        let move_type = if msg.is_run { MoveType::Run } else { MoveType::Walk };

        // 发送移动请求到 PlayerActor
        if let Ok(success) = record.actor_ref.ask(MoveRequest {
            session_id: msg.session_id,
            direction: msg.direction,
            is_run: msg.is_run,
        }).await {
            if !success {
                return;
            }
        } else {
            return;
        }

        // C# HumanObject Walk/Run：移动打断专注（3s 内不提供专注加成）
        self.interrupt_concentration(msg.session_id).await;

        // C# HumanObject Walk/Run：骑乘移动扣坐骑忠诚度（Walk=1 / Run=2，LoyaltyDelay 限速）
        let _ = record.actor_ref.tell(crate::actors::player::DecreaseMountLoyalty {
            amount: if msg.is_run { 2 } else { 1 },
        }).try_send();

        // C# HumanObject Walk/Run：进入安全区时更新绑定点（SetBindSafeZone）
        if let Ok(Some(post_state)) = record.actor_ref.ask(GetPlayerState).await {
            self.update_bind_safe_zone(msg.session_id, post_state.map_index, post_state.x, post_state.y).await;
        }

        // 获取移动后的状态并广播给其他玩家
        if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
            // 隐身玩家移动时不广播给其他人
            if !self.invisible_sessions.contains(&msg.session_id) {
                let others: Vec<_> = self.other_players(msg.session_id)
                    .into_iter()
                    .map(|r| r.actor_ref.clone())
                    .collect();

                for other in others {
                    let _ = other.ask(BroadcastMovement {
                        object_id: state.object_id,
                        x: state.x,
                        y: state.y,
                        direction: state.direction,
                        move_type,
                        exclude_session: msg.session_id,
                    }).await;
                }
            }

            // 检查是否踩到地图传送点（Movement）— O(1) index lookup
            let mv = self.movement_index.get(&(state.map_index as i32, state.x, state.y)).cloned();

            if let Some(mv) = mv {
                let dest_map_index = mv.map_index;
                let dest_x = mv.dest_x;
                let dest_y = mv.dest_y;

                // C# NeedMove 语义：暂存传送点（不直接传送），由 NPC 脚本 ENTERMAP 指令执行
                if mv.need_move {
                    self.session_last_movement.insert(msg.session_id, (dest_map_index as u16, dest_x, dest_y));
                    debug!("Movement staged for ENTERMAP: {} ({},{}) -> {} ({},{})",
                           state.map_index, state.x, state.y, dest_map_index, dest_x, dest_y);
                    return;
                }

                // Look up dest map file name from DB-loaded map_infos
                let dest_map_info = self.map_infos.get(&dest_map_index).cloned();

                if let Some(dest_mi) = dest_map_info {
                    if dest_mi.no_teleport {
                        debug!("Movement trigger blocked: map {} has no_teleport", dest_map_index);
                        return;
                    }
                    // Check no_escape on source map
                    if let Some(src_mi) = self.map_infos.get(&(state.map_index as i32)) {
                        if src_mi.no_escape {
                            debug!("Movement trigger blocked: source map {} has no_escape", state.map_index);
                            return;
                        }
                    }

                    let dest_file = dest_mi.file_name.clone();
                    let dest_title = dest_mi.title.clone();
                    let is_big_map = dest_mi.big_map;
                    let player_ref = record.actor_ref.clone();
                    let player_name = record.name.clone();

                    // Load dest map（按目标 map_index 加载，支持多图并存）
                    let dest_slot = dest_map_index as u16;
                    if self.get_or_load_map(&dest_file, dest_slot).is_some() {
                        info!("Player {} teleported via movement: {} ({},{}) -> {} ({},{})",
                            player_name, state.map_index, state.x, state.y,
                            dest_map_index, dest_x, dest_y);

                        // Inject new map data into player for collision/pathfinding
                        if let Some(map_data) = self.maps.get(&dest_slot).cloned() {
                            let _ = player_ref.ask(SetMapData { map: map_data }).await;
                        }

                        // Update player position
                        let _ = player_ref.ask(SetPlayerPosition {
                            x: dest_x,
                            y: dest_y,
                            direction: state.direction,
                            map_index: Some(dest_map_index as u16),
                            is_mounted: None,
                        }).await;

                        // Send MapChanged packet
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: msg.session_id,
                            data: build_map_changed_packet(dest_map_index as u16, &dest_file, &dest_title, dest_x, dest_y, is_big_map),
                        }).await;

                        // Send UserLocation to confirm new position
                        if let Ok(Some(new_state)) = player_ref.ask(GetPlayerState).await {
                            let mut loc_body = Vec::new();
                            loc_body.extend_from_slice(&(new_state.x as u32).to_le_bytes());
                            loc_body.extend_from_slice(&(new_state.y as u32).to_le_bytes());
                            loc_body.push(new_state.direction);
                            let _ = self.gate_ref.tell(SendToClient {
                                session_id: msg.session_id,
                                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::UserLocation as i16, &loc_body),
                            }).await;
                        }

                        // 清理旧地图视野：发送 ObjectRemove 给该玩家（移除旧地图上的怪物/玩家/地面物品）
                        let old_map = state.map_index;
                        for (oid, monster) in &self.monsters {
                            if monster.map_index == old_map {
                                let mut rb = Vec::new();
                                rb.extend_from_slice(&oid.to_le_bytes());
                                let _ = self.gate_ref.tell(SendToClient {
                                    session_id: msg.session_id,
                                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectRemove as i16, &rb),
                                }).await;
                            }
                        }
                        for (sid, rec) in &self.players {
                            if *sid != msg.session_id {
                                if let Ok(Some(s)) = rec.actor_ref.ask(GetPlayerState).await {
                                    if s.map_index == old_map {
                                        let mut rb = Vec::new();
                                        rb.extend_from_slice(&s.object_id.to_le_bytes());
                                        let _ = self.gate_ref.tell(SendToClient {
                                            session_id: msg.session_id,
                                            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectRemove as i16, &rb),
                                        }).await;
                                    }
                                }
                            }
                        }
                        for gi in &self.ground_items {
                            if gi.map_index == old_map {
                                let mut rb = Vec::new();
                                rb.extend_from_slice(&gi.object_id.to_le_bytes());
                                let _ = self.gate_ref.tell(SendToClient {
                                    session_id: msg.session_id,
                                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectRemove as i16, &rb),
                                }).await;
                            }
                        }

                        // 发送新地图上的 NPC 和怪物给该玩家
                        let spawn_ctx = SpawnContext {
                            map_info: self.map_infos.get(&dest_map_index),
                            monster_infos: &self.monster_infos,
                            npc_infos: &self.npc_infos,
                            dragon_info: self.dragon_info.as_ref(),
                            rarity: self.rarity_cfg.clone(),
                        };
                        let dest_file_clone = dest_file.clone();
                        let (new_npcs, new_monsters) = spawn_npcs_and_monsters(
                            self.gate_ref.clone(),
                            &self.spawn_dir,
                            &dest_file_clone,
                            dest_map_index as u16,
                            msg.session_id,
                            &mut self.next_object_id,
                            &spawn_ctx,
                        ).await;
                        for npc in new_npcs {
                            self.npcs.insert(npc.object_id, npc);
                        }
                        let elite_broadcasts: Vec<String> = new_monsters.iter()
                            .filter(|m| m.is_elite).map(|m| m.name.clone()).collect();
                        for monster in new_monsters {
                            self.monsters.insert(monster.object_id, monster);
                        }

                        // 初始生成精英广播
                        for name in &elite_broadcasts {
                            let map_name = self.map_infos.get(&(dest_map_index)).map(|m| m.title.clone()).unwrap_or_else(|| "未知地图".to_string());
                            broadcast_system_message(
                                &self.gate_ref, &self.players,
                                &format!("一只 [精英]{} 出现在 {}！勇士们，前往讨伐！", name.strip_prefix("[精英] ").unwrap_or(name), map_name));
                        }

                        // 同步新地图上的地面物品
                        let dest_map_u16 = dest_map_index as u16;
                        for gi in &self.ground_items {
                            if gi.map_index != dest_map_u16 { continue; }
                            if gi.item.item_index == 0 {
                                let object_gold = mir2_shared::packets::server::ObjectGold {
                                    object_id: gi.object_id,
                                    gold: gi.item.count as u32,
                                    location_x: gi.x,
                                    location_y: gi.y,
                                };
                                let mut buf = Vec::new();
                                if mir2_shared::packets::base::serialize_packet(
                                    &mut std::io::Cursor::new(&mut buf), &object_gold).is_ok() {
                                    let _ = self.gate_ref.tell(SendToClient { session_id: msg.session_id, data: buf }).await;
                                }
                            } else {
                                let object_item = mir2_shared::packets::server::ObjectItem {
                                    object_id: gi.object_id,
                                    item: gi.item.clone(),
                                    location_x: gi.x,
                                    location_y: gi.y,
                                };
                                let mut buf = Vec::new();
                                if mir2_shared::packets::base::serialize_packet(
                                    &mut std::io::Cursor::new(&mut buf), &object_item).is_ok() {
                                    let _ = self.gate_ref.tell(SendToClient { session_id: msg.session_id, data: buf }).await;
                                }
                            }
                        }

                        // 同步新地图上已打开的门
                        for (map_idx, door_idx) in &self.open_doors {
                            if *map_idx == dest_map_u16 {
                                send_opendoor(&self.gate_ref, msg.session_id, *door_idx, false).await;
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Message<WorldTurnRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: WorldTurnRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => {
                warn!("Turn request for unknown session {}", msg.session_id);
                return;
            }
        };

        let _ = record.actor_ref.ask(TurnRequest {
            session_id: msg.session_id,
            direction: msg.direction,
        }).await;

        // 广播转向
        if let Ok(Some(state)) = record.actor_ref.ask(crate::actors::player::GetPlayerState).await {
            let others: Vec<_> = self.other_players(msg.session_id)
                .into_iter()
                .map(|r| r.actor_ref.clone())
                .collect();

            for other in others {
                let _ = other.ask(BroadcastMovement {
                    object_id: state.object_id,
                    x: state.x,
                    y: state.y,
                    direction: state.direction,
                    move_type: MoveType::Turn,
                    exclude_session: msg.session_id,
                }).await;
            }
        }
    }
}

impl Message<PlayerDisconnected> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: PlayerDisconnected,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.remove(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        self.invisible_sessions.remove(&msg.session_id);
        self.market_search_cache.remove(&msg.session_id);

        info!("Player removed from world (session={})", msg.session_id);

        // #835：断线即离队——先清 group_id 再保存，避免陈旧组队引用被持久化
        let _ = record.actor_ref.ask(crate::actors::player::SetGroupId { group_id: None }).await;

        // 保存玩家数据到数据库
        if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
            if let Err(e) = db::save_character(&self.db_pool, &state, &record.account_username).await {
                warn!("Failed to save player {} on disconnect: {}", record.name, e);
            } else {
                info!("Player {} saved to database on disconnect", record.name);
            }
            // C# LastLogoutDate：记录最后下线时间（选角界面/安全区下线加成用）
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            if let Err(e) = db::update_last_access(&self.db_pool, &record.name, now).await {
                warn!("Failed to update last_access for {} on disconnect: {}", record.name, e);
            }

            // 行会离线状态由 SocialActor 管理
        }

        // M61：若该地图已无其他玩家，清理该地图的 NPC/怪物（修复多次登录 NPC 泄漏）
        if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
            self.cleanup_map_spawns(state.map_index).await;
        }

        // 通知 SocialActor 玩家下线（组队/好友在线表清理）
        let _ = self.social_ref.tell(crate::actors::social::SocialPlayerLeft {
            session_id: msg.session_id,
        }).try_send();

        // 组队离线状态由 SocialActor 管理

        // 通知其他玩家该玩家已离开
        if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
            let others: Vec<_> = self.other_players(msg.session_id)
                .into_iter()
                .map(|r| (r.actor_ref.clone(), r.session_id))
                .collect();

            let opcode = mir2_shared::enums::ServerPacketIds::ObjectRemove as i16;
            let mut body = Vec::new();
            body.extend_from_slice(&state.object_id.to_le_bytes());
            let packet = build_packet_bytes(opcode, &body);

            for (_, other_session) in others {
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: other_session,
                    data: packet.clone(),
                }).await;
            }
        }
    }
}

impl Message<PlayerLogOut> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: PlayerLogOut,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.remove(&msg.session_id) {
            Some(r) => r,
            None => {
                warn!("Logout request for unknown session {}", msg.session_id);
                return;
            }
        };
        self.invisible_sessions.remove(&msg.session_id);
        self.market_search_cache.remove(&msg.session_id);

        // Clean up active rental sessions involving this player
        if let Some(session) = self.rental_sessions.remove(&msg.session_id) {
            // This player was the renter (initiator) - return item to owner
            if let Some(item) = session.owner_item {
                if let Some(owner_record) = self.players.get(&session.partner_session) {
                    let _ = owner_record.actor_ref.ask(AddItemToInventory { item }).await;
                    send_system_message(&self.gate_ref, session.partner_session, "租赁对方已下线，物品已退回");
                }
            }
        }
        // Check if this player is the owner in someone else's rental session
        let renter_session = self.rental_sessions.iter()
            .find(|(_, s)| s.partner_session == msg.session_id)
            .map(|(k, _)| *k);
        if let Some(renter_sid) = renter_session {
            if let Some(session) = self.rental_sessions.remove(&renter_sid) {
                // Return item to this player (owner, who is logging out)
                if let Some(item) = session.owner_item {
                    let _ = record.actor_ref.ask(AddItemToInventory { item }).await;
                }
                send_system_message(&self.gate_ref, renter_sid, "租赁对方已下线，租赁已取消");
            }
        }

        if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
            info!("Player {} logged out (session={})", state.name, msg.session_id);
            // M61：该地图无其他玩家时清理 NPC/怪物
            self.cleanup_map_spawns(state.map_index).await;
        // 通知 SocialActor 玩家下线（组队/好友在线表清理）
        let _ = self.social_ref.tell(crate::actors::social::SocialPlayerLeft {
            session_id: msg.session_id,
        }).try_send();

            // #835：断线即离队——先清 group_id 再保存，避免陈旧组队引用被持久化
            let _ = record.actor_ref.ask(crate::actors::player::SetGroupId { group_id: None }).await;
            // 重新取状态（group_id 已清）
            let state = match record.actor_ref.ask(GetPlayerState).await {
                Ok(Some(s)) => s,
                _ => return,
            };
            // 保存玩家数据到数据库
            if let Err(e) = db::save_character(&self.db_pool, &state, &record.account_username).await {
                warn!("Failed to save player {} on logout: {}", record.name, e);
            } else {
                info!("Player {} saved to database on logout", record.name);
            }
            // C# LastLogoutDate：记录最后下线时间
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            if let Err(e) = db::update_last_access(&self.db_pool, &record.name, now).await {
                warn!("Failed to update last_access for {} on logout: {}", record.name, e);
            }

            // #194：保存英雄列表到 DB（重启不丢）
            let heroes = self.player_heroes.get(&msg.session_id).cloned().unwrap_or_default();
            let db_heroes: Vec<db::DbHero> = heroes.iter().map(|h| db::DbHero {
                index: h.index,
                name: h.name.clone(),
                level: h.level,
                class: h.class as u8,
                gender: h.gender as u8,
                dead: h.dead,
                sealed: h.sealed,
            }).collect();
            if let Err(e) = db::save_heroes(&self.db_pool, &record.name, &db_heroes).await {
                warn!("Failed to save heroes for {} on logout: {}", record.name, e);
            }
            // #198：移除英雄对象
            self.broadcast_hero_remove(record.object_id).await;

            // 发送 LogOutSuccess 给客户端（带角色列表，C# SelectScene 用）
            let characters = db::list_character_summaries(&self.db_pool, &record.account_username).await.unwrap_or_default();
            let mut body = Vec::new();
            body.extend_from_slice(&(characters.len() as i32).to_le_bytes());
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            for (i, c) in characters.iter().enumerate() {
                body.extend_from_slice(&(i as i32).to_le_bytes()); // index
                crate::util::wire::write_dotnet_string(&mut body, &c.name);
                body.extend_from_slice(&c.level.to_le_bytes());
                body.push(c.class);
                body.push(c.gender);
                // .NET DateTime ticks（last_access 未存 DB → 用当前时间）
                let ticks = 621355968000000000i64 + now_secs * 10_000_000;
                body.extend_from_slice(&ticks.to_le_bytes());
            }
            let _ = self.gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::LogOutSuccess as i16, &body),
            }).await;

            // 通知其他玩家该玩家已离开
            let others: Vec<_> = self.other_players(msg.session_id)
                .into_iter()
                .map(|r| (r.actor_ref.clone(), r.session_id))
                .collect();

            let opcode = mir2_shared::enums::ServerPacketIds::ObjectRemove as i16;
            let mut remove_body = Vec::new();
            remove_body.extend_from_slice(&state.object_id.to_le_bytes());
            let packet = build_packet_bytes(opcode, &remove_body);

            for (_, other_session) in others {
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: other_session,
                    data: packet.clone(),
                }).await;
            }
        }
        // 玩家已从 self.players 移除，无需再发 PlayerDisconnected
    }
}

impl WorldActor {
    /// M61：地图上无其他玩家时，清理该地图的 NPC/怪物（避免多次登录泄漏）
    pub(crate) async fn cleanup_map_spawns(&mut self, map_index: u16) {
        let mut others_on_map = 0usize;
        for r in self.players.values() {
            if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                if os.map_index == map_index {
                    others_on_map += 1;
                }
            }
        }
        if others_on_map > 0 {
            return;
        }
        let npc_count = self.npcs.values().filter(|n| n.map_index == map_index).count();
        let npc_ids: Vec<u32> = self
            .npcs
            .iter()
            .filter(|(_, n)| n.map_index == map_index)
            .map(|(id, _)| *id)
            .collect();
        for id in &npc_ids {
            self.npcs.remove(id);
        }
        let mon_count = self.monsters.values().filter(|m| m.map_index == map_index).count();
        let mon_ids: Vec<u32> = self
            .monsters
            .iter()
            .filter(|(_, m)| m.map_index == map_index)
            .map(|(id, _)| *id)
            .collect();
        for id in &mon_ids {
            self.monsters.remove(id);
        }
        info!("Map {} spawns cleaned (npcs={} monsters={})", map_index, npc_count, mon_count);
    }
}

impl Message<ChatRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: ChatRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        use mir2_shared::globals::MAX_CHAT_LENGTH;

        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => {
                warn!("Chat from unknown session {}", msg.session_id);
                return;
            }
        };

        // 截断过长消息（避免 UTF-8 边界截断导致 panic）
        let message = if msg.message.len() > MAX_CHAT_LENGTH {
            msg.message.chars().take(MAX_CHAT_LENGTH).collect()
        } else {
            msg.message
        };

        if message.trim().is_empty() {
            return;
        }

        // C#：! 前缀喊话（HasMapShout/HasServerShout 卷轴 + 8 级门槛 + 10 秒冷却）
        if let Some(shout_msg) = message.strip_prefix('!') {
            let shout_msg = shout_msg.trim();
            if !shout_msg.is_empty() {
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);
                let (level, map_shout, server_shout, last_shout_time) =
                    if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                        (state.level, state.has_map_shout, state.has_server_shout, state.last_shout_time)
                    } else {
                        (0u16, false, false, 0i64)
                    };
                if now_ms < last_shout_time + 10_000 {
                    send_system_message(&self.gate_ref, msg.session_id, "喊话冷却中，请稍后再试");
                    return;
                }
                if level < 8 && !map_shout && !server_shout {
                    send_system_message(&self.gate_ref, msg.session_id, "需要 8 级才能喊话");
                    return;
                }
                let sender_map = if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    state.map_index
                } else {
                    return;
                };
                if map_shout {
                    // 地图喊话（C# ChatType.Shout2），消耗卷轴标记
                    let _ = record.actor_ref.ask(crate::actors::player::SetShoutState {
                        map_shout: false,
                        server_shout: false,
                        last_shout_time: now_ms,
                    }).await;
                    for (sid, other) in &self.players {
                        if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                            if os.map_index == sender_map {
                                let mut body = Vec::new();
                                write_dotnet_string(&mut body, &format!("(!){}:{}", record.name, shout_msg));
                                body.push(mir2_shared::enums::ChatType::Shout2 as u8);
                                let _ = self.gate_ref.tell(SendToClient {
                                    session_id: *sid,
                                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Chat as i16, &body),
                                }).await;
                            }
                        }
                    }
                    return;
                } else if server_shout {
                    // 全服喊话（C# ChatType.Shout3），消耗卷轴标记
                    let _ = record.actor_ref.ask(crate::actors::player::SetShoutState {
                        map_shout: false,
                        server_shout: false,
                        last_shout_time: now_ms,
                    }).await;
                    for sid in self.players.keys() {
                        let mut body = Vec::new();
                        write_dotnet_string(&mut body, &format!("(!!){}:{}", record.name, shout_msg));
                        body.push(mir2_shared::enums::ChatType::Shout3 as u8);
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: *sid,
                            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Chat as i16, &body),
                        }).await;
                    }
                    return;
                } else {
                    // 8 级+ 普通喊话：同地图（C# Shout 范围内）；记录冷却
                    let _ = record.actor_ref.ask(crate::actors::player::SetShoutState {
                        map_shout: false,
                        server_shout: false,
                        last_shout_time: now_ms,
                    }).await;
                    for (sid, other) in &self.players {
                        if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                            if os.map_index == sender_map {
                                let mut body = Vec::new();
                                write_dotnet_string(&mut body, &format!("[喊话] {}: {}", record.name, shout_msg));
                                body.push(mir2_shared::enums::ChatType::Shout as u8);
                                let _ = self.gate_ref.tell(SendToClient {
                                    session_id: *sid,
                                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Chat as i16, &body),
                                }).await;
                            }
                        }
                    }
                    return;
                }
            }
        }

        // C# GM 命令（@ 前缀，仅 GM；C# PlayerObject Chat @level/@gold/@teleport/@make/@monster）
        if let Some(cmd_rest) = message.strip_prefix('@') {
            let parts: Vec<&str> = cmd_rest.split_whitespace().collect();
            if !parts.is_empty() {
                let cmd = parts[0].to_uppercase();
                if matches!(cmd.as_str(), "LEVEL" | "GOLD" | "TELEPORT" | "MAKE" | "MONSTER") {
                    let is_gm = if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await { state.is_gm } else { false };
                    if !is_gm {
                        send_system_message(&self.gate_ref, msg.session_id, "你没有权限使用此命令");
                        return;
                    }
                    match cmd.as_str() {
                        // @level <n>
                        "LEVEL" => {
                            let lv = parts.get(1).and_then(|s| s.parse::<u16>().ok()).unwrap_or(1).min(200);
                            let _ = record.actor_ref.ask(crate::actors::player::ChangeLevel { level: lv }).await;
                            send_system_message(&self.gate_ref, msg.session_id, &format!("等级已设置为 {}", lv));
                        }
                        // @gold <n>
                        "GOLD" => {
                            let g = parts.get(1).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                            let _ = record.actor_ref.ask(crate::actors::player::AddGold { amount: g }).await;
                            send_system_message(&self.gate_ref, msg.session_id, &format!("已获得 {} 金币", g));
                        }
                        // @teleport <x> <y>
                        "TELEPORT" => {
                            let x = parts.get(1).and_then(|s| s.parse::<i32>().ok());
                            let y = parts.get(2).and_then(|s| s.parse::<i32>().ok());
                            if let (Some(x), Some(y)) = (x, y) {
                                let _ = record.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                                    x, y, direction: 4, map_index: None, is_mounted: None,
                                }).await;
                                send_system_message(&self.gate_ref, msg.session_id, &format!("已传送至 ({}, {})", x, y));
                            } else {
                                send_system_message(&self.gate_ref, msg.session_id, "用法：@teleport <x> <y>");
                            }
                        }
                        // @make <物品名> [数量]
                        "MAKE" => {
                            let name = parts.get(1).copied().unwrap_or("");
                            let count = parts.get(2).and_then(|s| s.parse::<u16>().ok()).unwrap_or(1).max(1);
                            let item_idx = self.item_infos.iter().find(|(_, i)| i.name.eq_ignore_ascii_case(name)).map(|(k, _)| *k);
                            if let Some(idx) = item_idx {
                                let mut item = crate::actors::inventory::make_item(idx, count);
                                if let Some(info) = self.item_infos.get(&idx) {
                                    item.max_dura = info.durability as u16;
                                    item.current_dura = info.durability as u16;
                                }
                                let _ = record.actor_ref.ask(crate::actors::player::AddItemToInventory { item }).await;
                                send_system_message(&self.gate_ref, msg.session_id, &format!("已生成 {} x{}", name, count));
                            } else {
                                send_system_message(&self.gate_ref, msg.session_id, &format!("未找到物品：{}", name));
                            }
                        }
                        // @monster <怪物名> [数量]
                        "MONSTER" => {
                            let name = parts.get(1).copied().unwrap_or("");
                            let count = parts.get(2).and_then(|s| s.parse::<u32>().ok()).unwrap_or(1).max(1);
                            let state = match record.actor_ref.ask(GetPlayerState).await {
                                Ok(Some(s)) => s,
                                _ => return,
                            };
                            let spawned = self.spawn_monster_named(name, state.x, state.y, count, state.map_index).await;
                            let msg_text = if spawned > 0 {
                                format!("已召唤 {} x{}", name, spawned)
                            } else {
                                format!("未找到怪物：{}", name)
                            };
                            send_system_message(&self.gate_ref, msg.session_id, &msg_text);
                        }
                        _ => {}
                    }
                    return;
                }
            }
        }

        // Check for social chat commands and forward to SocialActor
        let parts: Vec<&str> = message.split_whitespace().collect();
        // 去掉前导 @（C# 客户端命令如 @ride 均带 @）
        let cmd = parts.first().unwrap_or(&"").trim_start_matches('@').to_uppercase();
        match cmd.as_str() {
            "GROUPRECALL" | "RECALLMEMBER" | "RECALL" | "ENABLEGROUPRECALL" | "DISABLEGROUPRECALL" | "RIDE" => {
                let args: Vec<String> = parts.iter().skip(1).map(|s| s.to_string()).collect();
                let _ = self.social_ref.ask(SocialChatCommand {
                    session_id: msg.session_id,
                    command: cmd,
                    args,
                }).await;
                return;
            }
            _ => {}
        }

        // 获取玩家名称、组队和公会信息
        let (player_name, group_id, guild_name) = if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
            (state.name, state.group_id, state.guild_name.clone())
        } else {
            return;
        };

        // 私聊 /w <name> <message>
        if let Some(whisper_cmd) = message.strip_prefix("/w ").or_else(|| message.strip_prefix("/W ")) {
            let mut whisper_parts = whisper_cmd.splitn(2, ' ');
            let target_name = whisper_parts.next().unwrap_or("").trim();
            let whisper_msg = whisper_parts.next().unwrap_or("").trim();
            if !target_name.is_empty() && !whisper_msg.is_empty() {
                let mut found = false;
                for (sid, other) in &self.players {
                    if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                        if os.name.eq_ignore_ascii_case(target_name) {
                            found = true;
                            // 发给目标: WhisperIn
                            let mut in_body = Vec::new();
                            write_dotnet_string(&mut in_body, &format!("{}: {}", player_name, whisper_msg));
                            in_body.push(mir2_shared::enums::ChatType::WhisperIn as u8);
                            let _ = self.gate_ref.tell(SendToClient {
                                session_id: *sid,
                                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Chat as i16, &in_body),
                            }).await;
                            // 发给自己: WhisperOut
                            let mut out_body = Vec::new();
                            write_dotnet_string(&mut out_body, &format!("-> {}: {}", target_name, whisper_msg));
                            out_body.push(mir2_shared::enums::ChatType::WhisperOut as u8);
                            let _ = self.gate_ref.tell(SendToClient {
                                session_id: msg.session_id,
                                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Chat as i16, &out_body),
                            }).await;
                            debug!("Whisper: {} -> {}: {}", player_name, target_name, whisper_msg);
                            break;
                        }
                    }
                }
                if !found {
                    send_system_message(&self.gate_ref, msg.session_id, "目标玩家不在线");
                }
                return;
            }
        }

        // 组队聊天 /g <message> 或 ! <message>
        let group_msg = message.strip_prefix("/g ").or_else(|| message.strip_prefix("/G "))
            .or_else(|| message.strip_prefix("! "));
        if let Some(gmsg) = group_msg {
            let gmsg = gmsg.trim();
            if !gmsg.is_empty() {
                if let Some(gid) = group_id {
                    let mut sent = false;
                    for (sid, other) in &self.players {
                        if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                            if os.group_id == Some(gid) {
                                let mut body = Vec::new();
                                write_dotnet_string(&mut body, &format!("[组队] {}: {}", player_name, gmsg));
                                body.push(mir2_shared::enums::ChatType::Group as u8);
                                let _ = self.gate_ref.tell(SendToClient {
                                    session_id: *sid,
                                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Chat as i16, &body),
                                }).await;
                                sent = true;
                            }
                        }
                    }
                    if sent {
                        debug!("Group chat: {} (group={}): {}", player_name, gid, gmsg);
                    }
                    return;
                } else {
                    send_system_message(&self.gate_ref, msg.session_id, "你不在队伍中");
                    return;
                }
            }
        }

        // 公会聊天 /guild <message> 或 /gu <message>
        let guild_msg = message.strip_prefix("/guild ").or_else(|| message.strip_prefix("/GUILD "))
            .or_else(|| message.strip_prefix("/gu ")).or_else(|| message.strip_prefix("/GU "));
        if let Some(gmsg) = guild_msg {
            let gmsg = gmsg.trim();
            if !gmsg.is_empty() {
                if let Some(ref gname) = guild_name {
                    let mut sent = false;
                    for (sid, other) in &self.players {
                        if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                            if os.guild_name.as_ref() == Some(gname) {
                                let mut body = Vec::new();
                                write_dotnet_string(&mut body, &format!("[公会] {}: {}", player_name, gmsg));
                                body.push(mir2_shared::enums::ChatType::Guild as u8);
                                let _ = self.gate_ref.tell(SendToClient {
                                    session_id: *sid,
                                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Chat as i16, &body),
                                }).await;
                                sent = true;
                            }
                        }
                    }
                    if sent {
                        debug!("Guild chat: {} (guild={}): {}", player_name, gname, gmsg);
                    }
                    return;
                } else {
                    send_system_message(&self.gate_ref, msg.session_id, "你不在公会中");
                    return;
                }
            }
        }

        // 喊话 /s <message> — 同地图广播
        if let Some(smsg) = message.strip_prefix("/s ").or_else(|| message.strip_prefix("/S ")) {
            let smsg = smsg.trim();
            if !smsg.is_empty() {
                let sender_map = if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    state.map_index
                } else {
                    return;
                };
                let mut sent = 0usize;
                for (sid, other) in &self.players {
                    if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                        if os.map_index == sender_map {
                            let mut body = Vec::new();
                            write_dotnet_string(&mut body, &format!("[喊话] {}: {}", player_name, smsg));
                            body.push(mir2_shared::enums::ChatType::Shout as u8);
                            let _ = self.gate_ref.tell(SendToClient {
                                session_id: *sid,
                                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Chat as i16, &body),
                            }).await;
                            sent += 1;
                        }
                    }
                }
                debug!("Shout: {} on map {}: {} ({} recipients)", player_name, sender_map, smsg, sent);
                return;
            }
        }

        // GM 全服公告 /announce <message>
        if let Some(amsg) = message.strip_prefix("/announce ").or_else(|| message.strip_prefix("/ANNOUNCE ")) {
            let amsg = amsg.trim();
            if !amsg.is_empty() {
                if amsg.len() > MAX_CHAT_LENGTH {
                    send_system_message(&self.gate_ref, msg.session_id, "公告内容过长");
                    return;
                }
                let is_gm = if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    state.is_gm
                } else {
                    false
                };
                if is_gm {
                    broadcast_system_message(&self.gate_ref, &self.players,
                        &format!("[公告] {}", amsg));
                    debug!("Announce: {}", amsg);
                } else {
                    send_system_message(&self.gate_ref, msg.session_id, "你没有权限使用此命令");
                }
                return;
            }
        }

        // GM 经验活动 /expevent <multiplier> <duration_minutes>
        if let Some(eargs) = message.strip_prefix("/expevent ").or_else(|| message.strip_prefix("/EXPEVENT ")) {
            let is_gm = if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                state.is_gm
            } else {
                false
            };
            if !is_gm {
                send_system_message(&self.gate_ref, msg.session_id, "你没有权限使用此命令");
                return;
            }
            let parts: Vec<&str> = eargs.trim().split_whitespace().collect();
            if parts.len() >= 2 {
                if let (Ok(mul), Ok(dur)) = (parts[0].parse::<f64>(), parts[1].parse::<u64>()) {
                    if mul < 1.0 || mul > 10.0 {
                        send_system_message(&self.gate_ref, msg.session_id, "倍率范围: 1.0 ~ 10.0");
                        return;
                    }
                    let dur = dur.min(1440);
                    let duration_ticks = dur * 600; // minutes -> ticks (1 min = 600 ticks @ 100ms)
                    self.global_exp_multiplier = mul;
                    self.global_drop_multiplier = mul;
                    self.global_gold_multiplier = mul;
                    self.global_exp_event_end_tick = self.tick_count + duration_ticks;
                    self.global_event_name = Some("经验活动".to_string());
                    broadcast_system_message(&self.gate_ref, &self.players,
                        &format!("【服务器活动】经验倍率 x{} 已启动，持续 {} 分钟！", mul, dur));
                    debug!("GM {} started exp event: x{} for {} min", msg.session_id, mul, dur);
                } else {
                    send_system_message(&self.gate_ref, msg.session_id, "用法: /expevent <倍率> <分钟>");
                }
            } else {
                send_system_message(&self.gate_ref, msg.session_id, "用法: /expevent <倍率> <分钟>");
            }
            return;
        }

        // 在线人数 /online
        if message.trim().eq_ignore_ascii_case("/online") || message.trim().eq_ignore_ascii_case("/who") {
            let count = self.players.len();
            send_system_message(&self.gate_ref, msg.session_id,
                &format!("当前在线玩家: {} 人", count));
            return;
        }

        // #285：聊天物品链接 → 向在线玩家推送 S.NewChatItem
        self.send_chat_item_links(msg.session_id, &message).await;

        let formatted = format!("[{}]: {}", player_name, message);
        debug!("Chat from {}: {}", player_name, message);

        // 广播给所有在线玩家（ChatType::Normal = 0）
        // 客户端 read_body 期望: [message: DotNetString][chat_type: u8]
        let mut body = Vec::new();
        write_dotnet_string(&mut body, &formatted);
        body.push(0u8); // ChatType::Normal
        let packet = build_packet_bytes(mir2_shared::enums::ServerPacketIds::Chat as i16, &body);

        for session_id in self.players.keys() {
            // 不给自己回发（本地已 add_message）
            if *session_id == msg.session_id {
                continue;
            }
            let _ = self.gate_ref.tell(SendToClient {
                session_id: *session_id,
                data: packet.clone(),
            }).await;
        }
    }
}

impl Message<ChangeAModeRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: ChangeAModeRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        // 更新玩家攻击模式
        let _ = record.actor_ref.ask(SetAttackMode { mode: msg.mode }).await;

        // 发送 ChangeAMode 确认包给客户端
        let body = vec![msg.mode as u8];
        let packet = build_packet_bytes(mir2_shared::enums::ServerPacketIds::ChangeAMode as i16, &body);
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: packet,
        }).await;
        debug!("ChangeAMode: session={} mode={:?}", msg.session_id, msg.mode);
    }
}

impl Message<ChangePModeRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: ChangePModeRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        // 更新玩家宠物模式
        let _ = record.actor_ref.ask(SetPetMode { mode: msg.mode }).await;

        // 发送 ChangePMode 确认包给客户端
        let body = vec![msg.mode as u8];
        let packet = build_packet_bytes(mir2_shared::enums::ServerPacketIds::ChangePMode as i16, &body);
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: packet,
        }).await;
        debug!("ChangePMode: session={} mode={:?}", msg.session_id, msg.mode);
    }
}

impl Message<SetSpellKeyRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetSpellKeyRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let _ = record.actor_ref.ask(SetSpellKey {
            spell: msg.spell,
            key: msg.key,
            old_key: msg.old_key,
        }).await;
    }
}

impl Message<SpellToggleRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SpellToggleRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // can_use: -1 = hero toggle (skip for now), 0 = off, 1 = on
        if msg.can_use < 0 { return; }
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let toggled = msg.can_use > 0;
        let object_id = record.object_id;
        let _ = record.actor_ref.ask(ToggleSpell {
            spell: msg.spell,
            toggled,
        }).await;
        // Send SpellToggle confirmation to client
        let mut body = Vec::new();
        body.extend_from_slice(&object_id.to_le_bytes());
        body.push(msg.spell as u8);
        body.push(if toggled { 1u8 } else { 0u8 });
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::SpellToggle as i16, &body),
        }).await;
    }
}

impl Message<SetHeroBehaviourRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetHeroBehaviourRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let _ = record.actor_ref.ask(SetHeroBehaviour { behaviour: msg.behaviour }).await;
        // Send HeroBehaviour confirmation to client
        let body = vec![msg.behaviour];
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::SetHeroBehaviour as i16, &body),
        }).await;
    }
}

impl Message<SetAutoPotValueRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetAutoPotValueRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let _ = record.actor_ref.ask(SetAutoPotValue { stat: msg.stat, value: msg.value }).await;
        // Send SetAutoPotValue confirmation to client
        let mut body = Vec::new();
        body.push(msg.stat);
        body.extend_from_slice(&msg.value.to_le_bytes());
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::SetAutoPotValue as i16, &body),
        }).await;
    }
}

impl Message<SetAutoPotItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetAutoPotItemRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        // C# SetAutoPotItem：物品不存在则置 0
        let item_index = if self.item_infos.contains_key(&msg.item_index) { msg.item_index } else { 0 };
        let _ = record.actor_ref.ask(SetAutoPotItem { grid: msg.grid, item_index }).await;
        // Send SetAutoPotItem confirmation to client
        let mut body = Vec::new();
        body.push(msg.grid);
        body.extend_from_slice(&item_index.to_le_bytes());
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::SetAutoPotItem as i16, &body),
        }).await;
    }
}

impl Message<RemoveSlotItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: RemoveSlotItemRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let success = record.actor_ref.ask(RemoveSlotItemMsg {
            grid: msg.grid,
            grid_to: msg.grid_to,
            unique_id: msg.unique_id,
            to: msg.to,
            from_unique_id: msg.from_unique_id,
        }).await.unwrap_or(false);
        // Send RemoveSlotItem response to client
        let mut body = Vec::new();
        body.push(msg.grid);
        body.push(msg.grid_to);
        body.extend_from_slice(&msg.unique_id.to_le_bytes());
        body.extend_from_slice(&msg.to.to_le_bytes());
        body.push(if success { 1u8 } else { 0u8 });
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::RemoveSlotItem as i16, &body),
        }).await;
    }
}

fn create_default_player_state(session_id: u64, object_id: u32) -> crate::actors::player::PlayerState {
    use crate::actors::player::PlayerState;
    use crate::actors::inventory::PlayerInventory;
    use crate::actors::friend::FriendList;
    use crate::actors::mail::Mailbox;
    use crate::actors::quest::QuestLog;
    use crate::actors::creature::CreatureLog;
    use crate::actors::refine::RefineLog;
    use crate::actors::guild::GuildRank;

    PlayerState {
        object_id: 0,
        name: format!("Player_{}", object_id),
        map_index: 0,
        x: 330,
        y: 330,
        direction: 4,
        attack_mode: mir2_shared::enums::AttackMode::Peace,
        pet_mode: mir2_shared::enums::PetMode::Both,
        hidden: false,
        session_id,
        class: mir2_shared::enums::MirClass::Warrior,
        gender: mir2_shared::enums::MirGender::Male,
        hair: 0,
        level: 1,
        experience: 0,
        max_experience: 100,
        can_gain_exp: true,
        pearl_count: 0,
        hp: 120,
        max_hp: 120,
        mp: 60,
        max_mp: 60,
        min_attack: 5,
        max_attack: 10,
        defence: 2,
        min_mc: 0,
        max_mc: 0,
        min_sc: 0,
        max_sc: 0,
        bonus_min_attack: 0,
        bonus_max_attack: 0,
        bonus_defence: 0,
        bonus_max_hp: 0,
        bonus_max_mp: 0,
        bonus_min_mc: 0,
        bonus_max_mc: 0,
        bonus_min_sc: 0,
        bonus_max_sc: 0,
        freezing: 0,
        poison_attack: 0,
        health_recovery: 0,
        spell_recovery: 0,
        attack_speed: 0,
        poison_resist: 0,
        poison_recovery: 0,
        holy: 0,
        accuracy: 0,
        agility: 0,
        min_ac: 0,
        max_ac: 0,
        min_mac: 0,
        max_mac: 0,
        bonus_min_ac: 0,
        bonus_max_ac: 0,
        bonus_min_mac: 0,
        bonus_max_mac: 0,
        luck: 0,
        critical_rate: 0,
        critical_damage: 0,
        magic_resist: 0,
        reflect: 0,
        damage_reduction_percent: 0,
        attack_bonus: 0,
        hp_drain_rate_percent: 0,
        energy_shield_percent: 0,
        energy_shield_hp_gain: 0,
        poison_list: Vec::new(),
        inventory: PlayerInventory::new(),
        group_id: None,
        friend_list: FriendList::new(),
        mailbox: Mailbox::new(),
        guild_name: None,
        guild_rank: GuildRank::Member,
        quest_log: QuestLog::new(),
        spouse_name: None,
        allow_mentor: false,
        mentor_name: None,
        creature_log: CreatureLog::new(),
        hero_index: 0,
        hero_behaviour: 0,
        auto_pot_hp: 0,
        auto_pot_mp: 0,
        auto_pot_hp_item: 0,
        auto_pot_mp_item: 0,
        hero_inventory: PlayerInventory::new(),
        hero_magics: Vec::new(),
        refine_log: RefineLog::new(),
        is_fishing: false,
        fishing_autocast: false,
        reincarnation_host: None,
        reincarnation_ready: false,
        reincarnation_expire_time: 0,
        enable_group_recall: false,
        last_recall_time: 0,
        is_dead: false,
            unlock_curse: false,
            last_revival_time: 0,
            last_access: 0,
            rested_counter: 0,
            rested_exp_percent: 0,
            rested_exp_end_tick: 0,
            has_map_shout: false,
            has_server_shout: false,
            last_shout_time: 0,
        is_mounted: false,
        mount_type: 0,
        allow_lover_recall: false,
        is_gm: false,
        pk_points: 0,
        pk_kill_count: 0,
        buffs: Vec::new(),
        magics: Vec::new(),
        flags: std::collections::HashMap::new(),
        exp_multiplier: 1.0,
        exp_multiplier_end_tick: 0,
            drop_multiplier: 1.0,
            drop_multiplier_end_tick: 0,
            elements_level: 0,
            has_elemental: false,
            concentration_interrupted: false,
            concentration_interrupt_time: 0,
            bind_map_index: 0,
            bind_x: 0,
            bind_y: 0,
            level_effects: 0,
            is_mentor: false,
            mentor_damage_bonus: false,
            newbie_exp_bonus: false,
            brown_until_ms: 0,
            mount_loyalty_decrease_time: 0,
            mount_loyalty_increase_time: 0,
    }
}
