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

        // #944：服务器全局经验倍率（C# Settings.ExpRate）
        loaded_state.exp_rate = self.exp_rate;

        // #887：仓库扩容/仓库密码状态（C# AccountInfo + Settings.RequireStoragePassword，
        // 登录进图时从 accounts 表加载，UserInformation 下发真实值）
        if let Ok(Some(account)) = db::load_account(&self.db_pool, &msg.account_username).await {
            loaded_state.has_expanded_storage = account.has_expanded_storage;
            loaded_state.expanded_storage_expiry_date = account.expanded_storage_expiry_date;
            loaded_state.has_storage_password = account.has_storage_password();
            loaded_state.require_storage_password = true; // C# Settings.RequireStoragePassword 默认 true
            loaded_state.storage_password_last_set = account.storage_password_last_set;
        }

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
        // #1540：登录即同步 ClearRing 隐身（头盔宝石）
        self.sync_clear_ring_visibility(msg.session_id).await;

        // C# PlayerObject.SetBind：确保绑定点有效（无绑定点/无效时随机出生安全区）
        self.ensure_bind(msg.session_id).await;

        // C# StartGame NoReconnect：由独立消息 ApplyNoReconnect 处理
        //（避免登录 handler 内同步加载大图导致 tokio 栈溢出，#881 回归）
        if let Some(world_ref) = self.self_ref.clone() {
            let _ = world_ref.tell(crate::actors::world::ApplyNoReconnect {
                session_id: msg.session_id,
            }).try_send();
        }

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
                experience: 0,
                max_experience: 100,
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

        // #937：登录进图后同步装备临时技能（C# RefreshEquipmentStats → AddTempSkills）
        self.sync_temp_skills(msg.session_id).await;

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
        // #934：C# MapInfo.NoNames——该地图实体名字显示 ?????（含自身）
        let self_display_name = if self.map_infos.get(&(loaded_state.map_index as i32)).map(|m| m.no_names).unwrap_or(false) {
            "?????"
        } else {
            player_name.as_str()
        };
        let self_packet = build_object_player_packet(
            self_display_name, object_id, loaded_state.x, loaded_state.y, loaded_state.direction,
            loaded_state.level, self.self_name_colour(&loaded_state),
            loaded_state.class, loaded_state.gender, loaded_state.hair,
            self_weapon, self_weapon_effect, self_armor,
            loaded_state.mount_type, loaded_state.is_mounted,
            loaded_state.level_effects,
            loaded_state.guild_name.as_deref().unwrap_or(""),
            crate::actors::world::guild_rank_name(loaded_state.guild_rank),
        );
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: self_packet,
        }).await;

        // 多玩家可见性：向新玩家发送已有玩家的 ObjectPlayer（同图 + 跳过隐身，#1651/#1653）
        self.send_map_players_to(msg.session_id, &loaded_state, loaded_state.map_index).await;

        // 向已有玩家发送新玩家的 ObjectPlayer（隐身新玩家不发送，#1651/#1653）
        let invis_tag = std::mem::discriminant(&crate::combat::buff::BuffType::Invisibility);
        if loaded_state.buffs.iter().any(|b| std::mem::discriminant(&b.buff_type) == invis_tag) {
            self.invisible_sessions.insert(msg.session_id);
        }
        self.send_player_to_map(msg.session_id, &loaded_state, loaded_state.map_index).await;

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

        // C# PlayerObject 构造（~1219）：登录下发 S.SwitchGroup 同步客户端“允许组队”开关
        {
            let sg = mir2_shared::packets::server::group::SwitchGroup {
                allow_group: loaded_state.allow_group,
            };
            let mut sg_body = Vec::new();
            if sg.write_body(&mut sg_body).is_ok() {
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::SwitchGroup as i16, &sg_body),
                }).await;
            }
        }

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
            self.maps.get(&map_slot),
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
            .filter(|m| m.rarity > 0)
            .map(|m| m.name.clone())
            .collect();
        for monster in new_monsters {
            self.monsters.insert(monster.object_id, monster);
        }

        // 初始生成精英广播
        for name in &elite_broadcasts {
            let map_name = self.map_infos.get(&(map_index as i32)).map(|m| m.title.clone()).unwrap_or_else(|| "未知地图".to_string());
            broadcast_system_message(&self.gate_ref, &self.players,
                &format!("一只 {} 出现在 {}！勇士们，前往讨伐！", name, map_name));
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
        // #1426/#1428：run/steps 在 state 块内确定（块外也要用 move_type/扣忠诚度）
        let (mut run, mut steps) = (false, 1);
        if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
            if state.is_dead { return; }

            // #1426：负重超限——C# CanWalk 不含负重（超重可走）；CanRun 含负重 → Run 退化为 Walk（HumanObject.Run :2516）
            let (bag_weight, _, _) = super::compute_player_weights(&state.inventory, &self.item_infos);
            let limit = super::weight_limit(&state.inventory, state.class, state.level, mir2_shared::enums::Stat::BagWeight, &self.item_infos);
            let overweight = bag_weight > limit;
            // #1428/#1502：C# HumanObject.Run steps = RidingMount || (ActiveSwiftFeet && !Sneaking) ? 3 : 2
            run = effective_run(msg.is_run, overweight);
            let swift_feet = state.buffs.iter().any(|b| matches!(
                b.buff_type, crate::combat::buff::BuffType::MoveSpeedBoost { .. }
            ));
            steps = move_steps(run, state.is_mounted, swift_feet);
            // #1408/#1428：C# Walk/Run 对每一格做阻挡校验——NPC / 未摧毁城墙城门阻挡通行
            let dir = msg.direction as usize % 8;
            let npc_tiles: Vec<(i32, i32)> = self.npcs.values()
                .filter(|n| n.map_index == state.map_index)
                .map(|n| (n.x, n.y))
                .collect();
            let struct_tiles: Vec<(i32, i32)> = self.siege_structures.values()
                .filter(|s| s.is_blocking() && !s.is_destroyed())
                .filter(|s| self.conquest_instances.iter().any(|c| c.id == s.conquest_id && c.map_index == state.map_index as i32))
                .map(|s| (s.x, s.y))
                .collect();
            let mut blocked = false;
            for j in 1..=steps {
                blocked |= tile_blocked_by(
                    state.x + super::MON_DIR_DX[dir] * j,
                    state.y + super::MON_DIR_DY[dir] * j,
                    &npc_tiles,
                    &struct_tiles,
                );
            }
            if blocked {
                // #1427：C# Walk/Run 失败 Enqueue S.UserLocation（用服务端坐标重同步）
                send_user_location_sync(&self.gate_ref, msg.session_id, state.direction, state.x, state.y).await;
                return;
            }
        }

        // Phase 1.4: 反作弊/节流 — 速度 hack 检测 + 可配置移动节流（#1509/#1531）
        // 正常移动间隔: Walk ~200ms, Run ~150ms。阈值 50ms 容忍网络抖动,但拒绝明显的瞬移/速度 hack。
        // movement_pacing_ms > 0 时按 C# HumanObject MoveDelay=600ms/动作 节流（Slow 毒 ×2，GetDelayTime）
        const MIN_MOVE_INTERVAL_MS: u64 = 50;
        let pacing_ms = self.movement_pacing_ms;
        let interval_ms = if pacing_ms > 0 {
            // C# GetDelayTime：持有 Slow 毒时翻倍
            let slow = if let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await {
                st.poison_list.iter().any(|p| p.p_type.intersects(mir2_shared::enums::PoisonType::SLOW))
            } else {
                false
            };
            pacing_ms * if slow { 2 } else { 1 }
        } else {
            MIN_MOVE_INTERVAL_MS
        };
        if let Some(last) = self.last_move_time.get(&msg.session_id) {
            let elapsed = last.elapsed();
            if elapsed < std::time::Duration::from_millis(interval_ms) {
                warn!(
                    "Speed hack detected: session {} moved after {:?} (min={:?})",
                    msg.session_id, elapsed, interval_ms
                );
                return; // 拒绝移动
            }
        }
        self.last_move_time.insert(msg.session_id, std::time::Instant::now());

        let move_type = if run { MoveType::Run } else { MoveType::Walk };

        // 发送移动请求到 PlayerActor
        // #1427：C# Walk/Run 失败 Enqueue S.UserLocation（目标不可走/眩晕/非法方向等）
        if let Ok(success) = record.actor_ref.ask(MoveRequest {
            session_id: msg.session_id,
            direction: msg.direction,
            is_run: run,
        }).await {
            if !success {
                if let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await {
                    send_user_location_sync(&self.gate_ref, msg.session_id, st.direction, st.x, st.y).await;
                }
                return;
            }
        } else {
            return;
        }

        // C# HumanObject Walk/Run：移动打断专注（3s 内不提供专注加成）
        self.interrupt_concentration(msg.session_id).await;

        // C# HumanObject Walk/Run：骑乘移动扣坐骑忠诚度（Walk=1 / Run=2，LoyaltyDelay 限速）
        let _ = record.actor_ref.tell(crate::actors::player::DecreaseMountLoyalty {
            amount: if run { 2 } else { 1 },
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
                // C# MovementInfo.NeedHole：需源格有 DigOutZombie/DigOutArmadillo 洞口 SpellObject 才能传送
                if mv.need_hole {
                    let has_hole = self.spell_objects.values().any(|so| {
                        so.map_index == state.map_index && so.x == state.x && so.y == state.y
                            && (so.spell == mir2_shared::enums::Spell::DigOutZombie
                                || so.spell == mir2_shared::enums::Spell::DigOutArmadillo)
                    });
                    if !has_hole {
                        send_system_message(&self.gate_ref, msg.session_id, "这里需要先挖开洞口才能通过");
                        return;
                    }
                }

                // C# MovementInfo.ConquestIndex：行会需拥有对应攻城领地才能传送
                if mv.conquest_index > 0 {
                    let owns = state.guild_name.as_ref().is_some_and(|guild| {
                        self.conquest_instances.iter().any(|c| {
                            c.id == mv.conquest_index && c.owner_guild.as_deref() == Some(guild.as_str())
                        })
                    });
                    if !owns {
                        send_system_message(&self.gate_ref, msg.session_id, "你的行会未拥有该领地，无法通过");
                        return;
                    }
                }

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
                    // #935：C# RequiredGroup——必须组队才能进入（GM 豁免）
                    if dest_mi.required_group && !state.is_gm {
                        let required = 2.max(dest_mi.required_group_size);
                        let have = self.group_member_count(msg.session_id).await;
                        if (have as i32) < required {
                            send_system_message(
                                &self.gate_ref,
                                msg.session_id,
                                &format!("该地图需要至少 {} 人组队才能进入", required),
                            );
                            return;
                        }
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
                            self.maps.get(&(dest_map_index as u16)),
                        ).await;
                        for npc in new_npcs {
                            self.npcs.insert(npc.object_id, npc);
                        }
                        let elite_broadcasts: Vec<String> = new_monsters.iter()
                            .filter(|m| m.rarity > 0).map(|m| m.name.clone()).collect();
                        for monster in new_monsters {
                            self.monsters.insert(monster.object_id, monster);
                        }

                        // 初始生成精英广播
                        for name in &elite_broadcasts {
                            let map_name = self.map_infos.get(&(dest_map_index)).map(|m| m.title.clone()).unwrap_or_else(|| "未知地图".to_string());
                            broadcast_system_message(
                                &self.gate_ref, &self.players,
                                &format!("一只 {} 出现在 {}！勇士们，前往讨伐！", name, map_name));
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

                        // #1653：进图传送玩家可见性同步（C# PlayerObject.Teleport → GetObjectsPassive）
                        if let Ok(Some(mover_state)) = record.actor_ref.ask(GetPlayerState).await {
                            // 通知旧地图其他玩家移除 mover
                            let mut rm = Vec::new();
                            rm.extend_from_slice(&mover_state.object_id.to_le_bytes());
                            let remove_packet = build_packet_bytes(
                                mir2_shared::enums::ServerPacketIds::ObjectRemove as i16, &rm);
                            broadcast_to_map(&self.gate_ref, &self.players, old_map, &remove_packet).await;
                            // 向 mover 发送新地图其他玩家
                            self.send_map_players_to(msg.session_id, &mover_state, dest_map_u16).await;
                            // 向新地图其他玩家发送 mover（隐身跳过）
                            self.send_player_to_map(msg.session_id, &mover_state, dest_map_u16).await;
                            // #1661：英雄随主人跨图召回（C# HeroObject.OwnerRecall → Teleport(Owner.CurrentMap, Owner.Back)）
                            if self.hero_ai_states.contains_key(&msg.session_id) {
                                let hero_oid = mover_state.object_id
                                    .wrapping_add(crate::actors::world::hero::HERO_OID_OFFSET);
                                let mut rh = Vec::new();
                                rh.extend_from_slice(&hero_oid.to_le_bytes());
                                let hero_remove = build_packet_bytes(
                                    mir2_shared::enums::ServerPacketIds::ObjectRemove as i16, &rh);
                                broadcast_to_map(&self.gate_ref, &self.players, old_map, &hero_remove).await;
                                if let Some(ai) = self.hero_ai_states.get_mut(&msg.session_id) {
                                    let (hx, hy) = crate::actors::world::point_move(
                                        mover_state.x, mover_state.y, mover_state.direction, 1);
                                    ai.x = hx;
                                    ai.y = hy;
                                    ai.direction = mover_state.direction;
                                }
                                // 新地图广播英雄生成（C# CurrentMap.Broadcast）
                                self.broadcast_hero_spawn(msg.session_id).await;
                            }
                        }
                    }
                }
            }
        }
    }
}

impl WorldActor {
    /// 构建玩家 ObjectPlayer 数据包（观察者相对色/行会战；C# GetNameColour）
    async fn build_player_object_packet(
        &self,
        target: &crate::actors::player::PlayerState,
        viewer: Option<&crate::actors::player::PlayerState>,
    ) -> Vec<u8> {
        let target_weapon = target.inventory.get_equipment(EquipmentSlot::Weapon)
            .and_then(|item| self.item_infos.get(&item.item_index))
            .map(|info| info.shape as i16).unwrap_or(-1);
        let target_armor = target.inventory.get_equipment(EquipmentSlot::Armour)
            .and_then(|item| self.item_infos.get(&item.item_index))
            .map(|info| info.shape as i16).unwrap_or(0);
        let target_weapon_effect = target.inventory.get_equipment(EquipmentSlot::Weapon)
            .and_then(|item| self.item_infos.get(&item.item_index))
            .map(|info| info.effect as i16).unwrap_or(0);
        let (at_war, enemy) = super::guild_war_flags(
            viewer.and_then(|v| v.guild_name.as_deref()),
            target.guild_name.as_deref(),
            &self.guild_wars,
        );
        let colour = super::name_colour_for_viewer(
            target.pk_points,
            super::is_brown(target.brown_until_ms),
            self.is_conquest_map(target.map_index),
            target.guild_name.as_deref(),
            viewer.and_then(|v| v.guild_name.as_deref()),
            at_war,
            enemy,
        );
        // #934：C# MapInfo.NoNames——按目标所在地图掩码
        let display_name = if self.map_infos.get(&(target.map_index as i32)).map(|m| m.no_names).unwrap_or(false) {
            "?????"
        } else {
            target.name.as_str()
        };
        build_object_player_packet(
            display_name, target.object_id, target.x, target.y, target.direction, target.level,
            colour,
            target.class, target.gender, target.hair,
            target_weapon, target_weapon_effect, target_armor,
            target.mount_type, target.is_mounted,
            target.level_effects,
            target.guild_name.as_deref().unwrap_or(""),
            crate::actors::world::guild_rank_name(target.guild_rank),
        )
    }

    /// #1653：把同图其他玩家的 ObjectPlayer 发给 viewer（登录/进图同步；跳过隐身与跨图）
    async fn send_map_players_to(
        &self,
        viewer_session: u64,
        viewer_state: &crate::actors::player::PlayerState,
        map_index: u16,
    ) {
        let invis_tag = std::mem::discriminant(&crate::combat::buff::BuffType::Invisibility);
        for (sid, rec) in &self.players {
            if *sid == viewer_session { continue; }
            let Ok(Some(ep_state)) = rec.actor_ref.ask(GetPlayerState).await else { continue };
            if ep_state.map_index != map_index { continue; }
            let is_invisible = ep_state.buffs.iter()
                .any(|b| std::mem::discriminant(&b.buff_type) == invis_tag);
            if is_invisible { continue; }
            let packet = self.build_player_object_packet(&ep_state, Some(viewer_state)).await;
            let _ = self.gate_ref.tell(SendToClient { session_id: viewer_session, data: packet }).await;
        }
    }

    /// #1653：把 mover 的 ObjectPlayer 发给同图其他玩家（跳过隐身；颜色按各观察者计算）
    async fn send_player_to_map(
        &self,
        mover_session: u64,
        mover_state: &crate::actors::player::PlayerState,
        map_index: u16,
    ) {
        let invis_tag = std::mem::discriminant(&crate::combat::buff::BuffType::Invisibility);
        if mover_state.buffs.iter().any(|b| std::mem::discriminant(&b.buff_type) == invis_tag) {
            return;
        }
        for (sid, rec) in &self.players {
            if *sid == mover_session { continue; }
            let Ok(Some(viewer)) = rec.actor_ref.ask(GetPlayerState).await else { continue };
            if viewer.map_index != map_index { continue; }
            let packet = self.build_player_object_packet(mover_state, Some(&viewer)).await;
            let _ = self.gate_ref.tell(SendToClient { session_id: *sid, data: packet }).await;
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

        // #1655：C# 客户端转向节流 2500ms（PlayerObject.cs:1440）；服务端限流防广播风暴
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        if let Some(last) = self.last_turn_ms.get(&msg.session_id) {
            if now_ms - *last < 100 {
                return;
            }
        }
        self.last_turn_ms.insert(msg.session_id, now_ms);

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

/// 登录/复活后应用 NoReconnect 地图规则（C# PlayerObject.StartGame：
/// 当前地图 NoReconnect → 传送到 NoReconnectMap 随机点）。
/// 独立消息处理：避免在登录/Tick handler 内同步加载大图导致 tokio 栈溢出（#881）。
pub struct ApplyNoReconnect {
    pub session_id: u64,
}

impl Message<ApplyNoReconnect> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: ApplyNoReconnect, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        let Some(mi) = self.map_infos.get(&(state.map_index as i32)).cloned() else {
            return;
        };
        if !mi.no_reconnect || mi.no_reconnect_map.is_empty() {
            return;
        }
        let Some(dest_mi) = self.map_infos.values()
            .find(|m| m.file_name.eq_ignore_ascii_case(&mi.no_reconnect_map))
            .cloned()
        else {
            return;
        };
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
        crate::actors::world::npc_script::teleport_player(
            self, msg.session_id, dest_map_index, rx, ry).await;
        info!("NoReconnect: moved session {} to map {} ({},{})", msg.session_id, dest_map_index, rx, ry);
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

            // #1127：断线同样持久化英雄列表——save_character 会 DELETE heroes 子表但不重建，
            // 若断线路径不补 save_heroes，英雄会在重启/再登录后永久丢失（与 PlayerLogOut 对齐）
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
                warn!("Failed to save heroes for {} on disconnect: {}", record.name, e);
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
        // #1578：C# MirConnection.LogOut——攻击/施法后 10s 内 LogOut 失败（S.LogOutFailed 空包）
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let block_until = self.player_logout_block_ms.get(&msg.session_id).copied();
        if crate::actors::world::combat::logout_blocked(now_ms, block_until) {
            debug!("Logout blocked: session={} until={} (C# LogTime)", msg.session_id, block_until.unwrap_or(0));
            let _ = self.gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::LogOutFailed as i16,
                    &[],
                ),
            }).await;
            return;
        }
        self.player_logout_block_ms.remove(&msg.session_id);

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
            Some(r) => r.clone(),
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

        // #1659：普通聊天限流（防刷屏广播；喊话另有 10s 冷却）
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        if let Some(last) = self.last_chat_ms.get(&msg.session_id) {
            if now_ms - *last < 200 {
                return;
            }
        }
        self.last_chat_ms.insert(msg.session_id, now_ms);

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
                if matches!(cmd.as_str(), "LEVEL" | "GOLD" | "MAKE" | "MONSTER" | "GOTO" | "RECALLMOB" | "CLEARBAG" | "REVIVE" | "GIVEGOLD" | "GIVESKILL" | "CLEARMOB" | "ADJUSTPKPOINT" | "CHANGEGENDER" | "HAIR" | "SETLIGHT" | "LEVELHERO" | "INFO" | "SETFLAG" | "CLEARFLAGS" | "DELETESKILL" | "GIVEHEROSKILL" | "GAMEMASTER" | "MOB" | "KILL" | "DIE" | "RELOADDROPS" | "RELOADNPCS" | "SUPERMAN" | "OBSERVER" | "CHANGECLASS" | "SETQUEST" | "CLEARQUESTS" | "GIVEPEARLS" | "GIVECREDIT" | "MAPMOVE" | "LISTFLAGS" | "STARTWAR" | "CREATEGUILD") {
                    let is_gm = if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await { state.is_gm } else { false };
                    if !is_gm {
                        send_system_message(&self.gate_ref, msg.session_id, "你没有权限使用此命令");
                        return;
                    }
                    match cmd.as_str() {
                        // @level [玩家] <等级>（#1468：C# GM LEVEL parts>=3 改目标玩家）
                        "LEVEL" => {
                            let lv = parts.last().and_then(|s| s.parse::<u16>().ok()).unwrap_or(1).min(200);
                            if parts.len() >= 3 {
                                // C#：@level <玩家> <等级>——改目标在线玩家
                                let name = parts.get(1).copied().unwrap_or("");
                                let mut found = None;
                                for (_sid, other) in &self.players {
                                    if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                                        if os.name.eq_ignore_ascii_case(name) {
                                            found = Some(*_sid);
                                            break;
                                        }
                                    }
                                }
                                let Some(target_sid) = found else {
                                    send_system_message(&self.gate_ref, msg.session_id, "未找到在线玩家");
                                    return;
                                };
                                if let Some(r) = self.players.get(&target_sid) {
                                    let _ = r.actor_ref.ask(crate::actors::player::ChangeLevel { level: lv }).await;
                                }
                                send_system_message(&self.gate_ref, msg.session_id, &format!("已设置 {} 的等级为 {}", name, lv));
                            } else {
                                let _ = record.actor_ref.ask(crate::actors::player::ChangeLevel { level: lv }).await;
                                send_system_message(&self.gate_ref, msg.session_id, &format!("等级已设置为 {}", lv));
                            }
                        }
                        // @gold <n>
                        "GOLD" => {
                            let g = parts.get(1).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                            let _ = record.actor_ref.ask(crate::actors::player::AddGold { amount: g }).await;
                            send_system_message(&self.gate_ref, msg.session_id, &format!("已获得 {} 金币", g));
                        }
                        // @make <物品名|索引> [数量]（#1471：C# MAKE 索引/名称双查）
                        "MAKE" => {
                            let name = parts.get(1).copied().unwrap_or("");
                            let count = parts.get(2).and_then(|s| s.parse::<u16>().ok()).unwrap_or(1).max(1);
                            let item_idx = if let Ok(idx) = name.parse::<i32>() {
                                if self.item_infos.contains_key(&idx) { Some(idx) } else { None }
                            } else {
                                self.item_infos.iter().find(|(_, i)| i.name.eq_ignore_ascii_case(&name)).map(|(k, _)| *k)
                            };
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
                        // @monster|@mob <怪物名|索引> [数量]（#1472：C# MOB 索引/名称双查）
                        "MONSTER" | "MOB" => {
                            let key = parts.get(1).copied().unwrap_or("");
                            let idx_opt = if let Ok(idx) = key.parse::<i32>() {
                                if self.monster_infos.contains_key(&idx) { Some(idx) } else { None }
                            } else {
                                self.monster_name_index.get(&key.to_lowercase()).copied()
                            };
                            let count = parts.get(2).and_then(|s| s.parse::<u32>().ok()).unwrap_or(1).max(1);
                            let state = match record.actor_ref.ask(GetPlayerState).await {
                                Ok(Some(s)) => s,
                                _ => return,
                            };
                            let spawned = if let Some(idx) = idx_opt {
                                // 按索引取怪名后走 spawn_monster_named（#1472）
                                match self.monster_infos.get(&idx).map(|i| i.name.clone()) {
                                    Some(name) => self.spawn_monster_named(&name, state.x, state.y, count, state.map_index).await,
                                    None => 0,
                                }
                            } else {
                                0
                            };
                            let msg_text = if spawned > 0 {
                                format!("已召唤 {} x{}", key, spawned)
                            } else {
                                format!("未找到怪物：{}", key)
                            };
                            send_system_message(&self.gate_ref, msg.session_id, &msg_text);
                        }
                        // @goto <玩家名>：传送到目标身边（C# case "GOTO" ~2915；独立消息处理）
                        "GOTO" => {
                            let Some(target_name) = parts.get(1).copied() else { return; };
                            if let Some(world_ref) = self.self_ref.clone() {
                                let _ = world_ref.tell(crate::actors::world::GmGotoRequest {
                                    session_id: msg.session_id,
                                    target_name: target_name.to_string(),
                                }).try_send();
                            }
                        }
                        // @recallmob <怪物名|id> [数量] [x] [y]（C# case "RECALLMOB" ~2992）
                        "RECALLMOB" => {
                            let Some(name) = parts.get(1).copied() else { return; };
                            let count = parts.get(2).and_then(|s| s.parse::<u32>().ok()).unwrap_or(1).max(1);
                            let state = match record.actor_ref.ask(GetPlayerState).await {
                                Ok(Some(s)) => s,
                                _ => return,
                            };
                            let x = parts.get(3).and_then(|s| s.parse::<i32>().ok()).unwrap_or(state.x);
                            let y = parts.get(4).and_then(|s| s.parse::<i32>().ok()).unwrap_or(state.y);
                            let spawned = self.spawn_monster_named(name, x, y, count, state.map_index).await;
                            if spawned > 0 {
                                send_system_message(&self.gate_ref, msg.session_id, &format!("已召唤 {} x{}", name, spawned));
                            } else {
                                send_system_message(&self.gate_ref, msg.session_id, &format!("未找到怪物：{}", name));
                            }
                        }
                        // @clearbag [玩家名]（C# case "CLEARBAG" ~2419）
                        "CLEARBAG" => {
                            match parts.get(1).copied() {
                                None => {
                                    let _ = record.actor_ref.ask(crate::actors::player::ClearBackpack).await;
                                    send_system_message(&self.gate_ref, msg.session_id, "背包已清空");
                                }
                                Some(n) => {
                                    let mut found = false;
                                    for (_sid, other) in &self.players {
                                        if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                                            if os.name.eq_ignore_ascii_case(n) {
                                                let _ = other.actor_ref.ask(crate::actors::player::ClearBackpack).await;
                                                send_system_message(&self.gate_ref, msg.session_id, &format!("已清空 {} 的背包", os.name));
                                                found = true;
                                                break;
                                            }
                                        }
                                    }
                                    if !found {
                                        send_system_message(&self.gate_ref, msg.session_id, &format!("未找到在线玩家：{}", n));
                                    }
                                }
                            }
                        }
                        // @revive [玩家名]（C# case "REVIVE" ~4055）
                        "REVIVE" => {
                            match parts.get(1).copied() {
                                None => {
                                    let _ = record.actor_ref.ask(crate::actors::player::Revive).await;
                                    send_system_message(&self.gate_ref, msg.session_id, "你已复活");
                                }
                                Some(n) => {
                                    let mut found = false;
                                    for (_sid, other) in &self.players {
                                        if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                                            if os.name.eq_ignore_ascii_case(n) {
                                                let _ = other.actor_ref.ask(crate::actors::player::Revive).await;
                                                send_system_message(&self.gate_ref, msg.session_id, &format!("已复活 {}", os.name));
                                                found = true;
                                                break;
                                            }
                                        }
                                    }
                                    if !found {
                                        send_system_message(&self.gate_ref, msg.session_id, &format!("未找到在线玩家：{}", n));
                                    }
                                }
                            }
                        }
                        // @givegold [玩家] <数量>（C# case "GIVEGOLD" ~3071）
                        "GIVEGOLD" => {
                            let amount = parts.last().and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                            if amount == 0 { return; }
                            match parts.get(1).copied() {
                                Some(n) if parts.len() >= 3 => {
                                    let mut found = false;
                                    for (_sid, other) in &self.players {
                                        if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                                            if os.name.eq_ignore_ascii_case(n) {
                                                let _ = other.actor_ref.ask(crate::actors::player::AddGold { amount }).await;
                                                send_system_message(&self.gate_ref, msg.session_id, &format!("已给 {} {} 金币", os.name, amount));
                                                found = true;
                                                break;
                                            }
                                        }
                                    }
                                    if !found {
                                        send_system_message(&self.gate_ref, msg.session_id, &format!("未找到在线玩家：{}", n));
                                    }
                                }
                                _ => {
                                    let _ = record.actor_ref.ask(crate::actors::player::AddGold { amount }).await;
                                    send_system_message(&self.gate_ref, msg.session_id, &format!("已获得 {} 金币", amount));
                                }
                            }
                        }
                        // @giveskill [玩家] <技能名> <等级0-3>（C# case "GIVESKILL" ~3167）
                        "GIVESKILL" => {
                            // 技能名：有 3+ 参数时是最后两个（[玩家] 技能 等级）；2 参数时是 (技能 等级)
                            let (skill_arg, level_arg) = if parts.len() >= 3 {
                                (parts.get(parts.len() - 2).copied().unwrap_or(""), parts.get(parts.len() - 1).copied().unwrap_or("0"))
                            } else {
                                (parts.get(1).copied().unwrap_or(""), parts.get(2).copied().unwrap_or("0"))
                            };
                            let Some(info) = self.magic_infos.values().find(|m| m.name.eq_ignore_ascii_case(skill_arg)).cloned() else {
                                send_system_message(&self.gate_ref, msg.session_id, &format!("未找到技能：{}", skill_arg));
                                return;
                            };
                            let level = level_arg.parse::<u8>().unwrap_or(0).min(3);
                            let target_sid = if parts.len() >= 4 {
                                // [玩家] 技能 等级
                                let name = parts.get(1).copied().unwrap_or("");
                                let mut found = None;
                                for (_sid, other) in &self.players {
                                    if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                                        if os.name.eq_ignore_ascii_case(name) {
                                            found = Some((*_sid, other.actor_ref.clone()));
                                            break;
                                        }
                                    }
                                }
                                found.map(|(sid, _)| sid)
                            } else {
                                Some(msg.session_id)
                            };
                            let Some(target_sid) = target_sid else {
                                send_system_message(&self.gate_ref, msg.session_id, "未找到在线玩家");
                                return;
                            };
                            let target = match self.players.get(&target_sid) {
                                Some(r) => r.clone(),
                                None => return,
                            };
                            let mut state = match target.actor_ref.ask(GetPlayerState).await {
                                Ok(Some(s)) => s,
                                _ => return,
                            };
                            if !state.magics.iter().any(|m| m.spell == info.spell) {
                                let mut m = crate::actors::player::PlayerMagic::new(info.spell);
                                m.level = level;
                                state.magics.push(m);
                                let _ = target.actor_ref.ask(SetPlayerState { state }).await;
                                self.send_new_magic_packet(target_sid, info.spell).await;
                                send_system_message(&self.gate_ref, msg.session_id, &format!("已传授技能 {}", info.name));
                            } else {
                                send_system_message(&self.gate_ref, msg.session_id, "对方已学会该技能");
                            }
                        }
                        // @clearmob（C# case "CLEARMOB" ~3399；简化：清空当前地图怪物）
                        "CLEARMOB" => {
                            let state = match record.actor_ref.ask(GetPlayerState).await {
                                Ok(Some(s)) => s,
                                _ => return,
                            };
                            // #1469：C# CLEARMOB parts.Length>1 按地图名清指定图
                            let map_index = if parts.len() > 1 {
                                let name = parts.get(1).copied().unwrap_or("");
                                match self.map_infos.values()
                                    .find(|m| m.file_name.eq_ignore_ascii_case(name))
                                    .map(|m| m.index as u16)
                                {
                                    Some(idx) => idx,
                                    None => {
                                        send_system_message(&self.gate_ref, msg.session_id, &format!("未找到地图：{}", name));
                                        return;
                                    }
                                }
                            } else {
                                state.map_index
                            };
                            let ids: Vec<u32> = self.monsters.iter()
                                .filter(|(_, m)| m.map_index == map_index)
                                .map(|(oid, _)| *oid)
                                .collect();
                            let mut removed = 0;
                            for oid in &ids {
                                if self.monsters.remove(oid).is_some() {
                                    removed += 1;
                                    let packet = Self::build_object_remove_packet(*oid);
                                    broadcast_to_map(&self.gate_ref, &self.players, map_index, &packet).await;
                                }
                            }
                            send_system_message(&self.gate_ref, msg.session_id, &format!("已清除 {} 只怪物", removed));
                        }
                        // @adjustpkpoint [玩家] <点数>（C# case "ADJUSTPKPOINT" ~3494：直接设置 PK）
                        "ADJUSTPKPOINT" => {
                            let target_sid = if parts.len() >= 3 {
                                let name = parts.get(1).copied().unwrap_or("");
                                let mut found = None;
                                for (_sid, other) in &self.players {
                                    if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                                        if os.name.eq_ignore_ascii_case(name) {
                                            found = Some(*_sid);
                                            break;
                                        }
                                    }
                                }
                                found
                            } else {
                                Some(msg.session_id)
                            };
                            let Some(target_sid) = target_sid else {
                                send_system_message(&self.gate_ref, msg.session_id, "未找到在线玩家");
                                return;
                            };
                            let target = match self.players.get(&target_sid) {
                                Some(r) => r.clone(),
                                None => return,
                            };
                            let want = parts.last().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                            let state = match target.actor_ref.ask(GetPlayerState).await {
                                Ok(Some(s)) => s,
                                _ => return,
                            };
                            let delta = want - state.pk_points;
                            let _ = target.actor_ref.ask(crate::actors::player::AddPkPoints { points: delta }).await;
                            self.broadcast_viewer_colours(target_sid).await;
                            send_system_message(&self.gate_ref, msg.session_id, &format!("已设置 PK 值为 {}", want));
                        }
                        // @changegender [玩家]（C# case "CHANGEGENDER" ~2237：切换性别）
                        "CHANGEGENDER" => {
                            let target_sid = if let Some(n) = parts.get(1).copied() {
                                let mut found = None;
                                for (_sid, other) in &self.players {
                                    if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                                        if os.name.eq_ignore_ascii_case(n) {
                                            found = Some(*_sid);
                                            break;
                                        }
                                    }
                                }
                                found
                            } else {
                                Some(msg.session_id)
                            };
                            let Some(target_sid) = target_sid else {
                                send_system_message(&self.gate_ref, msg.session_id, "未找到在线玩家");
                                return;
                            };
                            let target = match self.players.get(&target_sid) {
                                Some(r) => r.clone(),
                                None => return,
                            };
                            let mut state = match target.actor_ref.ask(GetPlayerState).await {
                                Ok(Some(s)) => s,
                                _ => return,
                            };
                            state.gender = match state.gender {
                                mir2_shared::enums::MirGender::Male => mir2_shared::enums::MirGender::Female,
                                mir2_shared::enums::MirGender::Female => mir2_shared::enums::MirGender::Male,
                            };
                            let _ = target.actor_ref.ask(SetPlayerState { state }).await;
                            send_system_message(
                                &self.gate_ref,
                                msg.session_id,
                                if target_sid == msg.session_id {
                                    "性别已修改，请重新登录生效"
                                } else {
                                    "已修改目标性别（重登生效）"
                                },
                            );
                        }
                        // @hair [发型0-8]（C# case "HAIR" ~3458：设置自己发型）
                        "HAIR" => {
                            let hair = match parts.get(1).copied() {
                                Some(h) => h.parse::<u8>().ok().unwrap_or(0).min(8),
                                None => fastrand::u8(0..9),
                            };
                            let mut state = match record.actor_ref.ask(GetPlayerState).await {
                                Ok(Some(s)) => s,
                                _ => return,
                            };
                            state.hair = hair;
                            let _ = record.actor_ref.ask(SetPlayerState { state }).await;
                            send_system_message(&self.gate_ref, msg.session_id, &format!("发型已设置为 {}", hair));
                        }
                        // @setlight <0-4>（C# case "SETLIGHT" ~4137：设置当前光照并广播）
                        "SETLIGHT" => {
                            let light_val = parts.get(1).and_then(|s| s.parse::<u8>().ok()).unwrap_or(255);
                            let setting = match light_val {
                                0 => mir2_shared::enums::LightSetting::Normal,
                                1 => mir2_shared::enums::LightSetting::Dawn,
                                2 => mir2_shared::enums::LightSetting::Day,
                                3 => mir2_shared::enums::LightSetting::Evening,
                                4 => mir2_shared::enums::LightSetting::Night,
                                _ => {
                                    send_system_message(&self.gate_ref, msg.session_id, "用法：@setlight <0=Normal 1=Dawn 2=Day 3=Evening 4=Night>");
                                    return;
                                }
                            };
                            self.current_light = setting;
                            for sid in self.players.keys() {
                                self.send_time_of_day(*sid, setting);
                            }
                            send_system_message(&self.gate_ref, msg.session_id, &format!("光照已设置为 {:?}", setting));
                        }
                        // @levelhero <等级> / @levelhero <玩家> <等级>（C# case "LEVELHERO" ~2312）
                        "LEVELHERO" => {
                            let (target_sid, level) = if parts.len() >= 3 {
                                let name = parts.get(1).copied().unwrap_or("");
                                let lv = parts.get(2).and_then(|s| s.parse::<u16>().ok()).unwrap_or(0);
                                let mut found = None;
                                for (_sid, other) in &self.players {
                                    if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                                        if os.name.eq_ignore_ascii_case(name) {
                                            found = Some(*_sid);
                                            break;
                                        }
                                    }
                                }
                                (found, lv)
                            } else {
                                (Some(msg.session_id), parts.get(1).and_then(|s| s.parse::<u16>().ok()).unwrap_or(0))
                            };
                            let Some(target_sid) = target_sid else {
                                send_system_message(&self.gate_ref, msg.session_id, "未找到在线玩家");
                                return;
                            };
                            if level == 0 { return; }
                            let target = match self.players.get(&target_sid) {
                                Some(r) => r.clone(),
                                None => return,
                            };
                            let state = match target.actor_ref.ask(GetPlayerState).await {
                                Ok(Some(s)) => s,
                                _ => return,
                            };
                            let hero_index = state.hero_index as i32;
                            let old = self.player_heroes.get(&target_sid)
                                .and_then(|hs| hs.iter().find(|h| h.index == hero_index))
                                .map(|h| h.level);
                            if let Some(heroes) = self.player_heroes.get_mut(&target_sid) {
                                if let Some(hero) = heroes.iter_mut().find(|h| h.index == hero_index) {
                                    hero.level = level;
                                }
                            }
                            let heroes = self.player_heroes.get(&target_sid).cloned().unwrap_or_default();
                            super::send_manage_heroes_packet(&self.gate_ref, target_sid, &state, &heroes);
                            self.send_hero_information_packet(target_sid).await;
                            match old {
                                Some(old) => send_system_message(&self.gate_ref, msg.session_id, &format!("英雄等级 {} -> {}", old, level)),
                                None => send_system_message(&self.gate_ref, msg.session_id, &format!("英雄等级已设为 {}", level)),
                            }
                        }
                        // @info [玩家名]（C# case "INFO" ~3724：查看玩家/怪物信息）
                        "INFO" => {
                            match parts.get(1).copied() {
                                Some(n) => {
                                    let mut found_player = false;
                                    for (_sid, other) in &self.players {
                                        if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                                            if os.name.eq_ignore_ascii_case(n) {
                                                send_system_message(
                                                    &self.gate_ref,
                                                    msg.session_id,
                                                    &format!("玩家信息：{} 等级={} 位置=({}, {}) 地图={} HP={}/{} MP={}/{}",
                                                        os.name, os.level, os.x, os.y, os.map_index, os.hp, os.max_hp, os.mp, os.max_mp),
                                                );
                                                found_player = true;
                                                break;
                                            }
                                        }
                                    }
                                    if !found_player {
                                        if let Some(m) = self.monsters.values().find(|m| m.name.eq_ignore_ascii_case(n)) {
                                            send_system_message(
                                                &self.gate_ref,
                                                msg.session_id,
                                                &format!("怪物信息：#{} {} HP={}/{} 位置=({}, {}) 地图={}",
                                                    m.monster_index, m.name, m.hp, m.max_hp, m.x, m.y, m.map_index),
                                            );
                                        } else {
                                            send_system_message(&self.gate_ref, msg.session_id, &format!("未找到目标：{}", n));
                                        }
                                    }
                                }
                                None => {
                                    if let Ok(Some(s)) = record.actor_ref.ask(GetPlayerState).await {
                                        send_system_message(
                                            &self.gate_ref,
                                            msg.session_id,
                                            &format!("玩家信息：{} 等级={} 位置=({}, {}) 地图={} HP={}/{} MP={}/{}",
                                                s.name, s.level, s.x, s.y, s.map_index, s.hp, s.max_hp, s.mp, s.max_mp),
                                        );
                                    }
                                }
                            }
                        }
                        // @gamemaster（C# case "GAMEMASTER" ~2448：切换 GM 保护模式，PvP 不可攻击）
                        "GAMEMASTER" => {
                            if self.gm_protected.contains(&msg.session_id) {
                                self.gm_protected.remove(&msg.session_id);
                                send_system_message(&self.gate_ref, msg.session_id, "已关闭 GM 保护模式（可被攻击）");
                            } else {
                                self.gm_protected.insert(msg.session_id);
                                send_system_message(&self.gate_ref, msg.session_id, "已开启 GM 保护模式（不可被攻击）");
                            }
                        }

                        // @kill [玩家]（C# case "KILL"：GM 击杀目标玩家）
                        "KILL" => {
                            let name = parts.get(1).copied().unwrap_or("");
                            let mut found = None;
                            for (_sid, other) in &self.players {
                                if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                                    if os.name.eq_ignore_ascii_case(name) {
                                        found = Some(*_sid);
                                        break;
                                    }
                                }
                            }
                            let Some(target_sid) = found else {
                                send_system_message(&self.gate_ref, msg.session_id, "未找到在线玩家");
                                return;
                            };
                            let Some(target) = self.players.get(&target_sid).cloned() else { return; };
                            if let Ok(Some(st)) = target.actor_ref.ask(GetPlayerState).await {
                                let died = target.actor_ref.ask(crate::actors::player::TakeDamage {
                                    attacker_id: 0, attacker_session: 0, damage: i32::MAX,
                                }).await.unwrap_or(false);
                                if died {
                                    let died_packet = Self::build_object_died_packet(st.object_id, st.x, st.y, st.direction);
                                    for (sid, _) in &self.players {
                                        let _ = self.gate_ref.tell(SendToClient {
                                            session_id: *sid,
                                            data: died_packet.clone(),
                                        }).await;
                                    }
                                    self.handle_player_death_drop(target_sid, st.x, st.y, st.map_index, false).await;
                                    send_system_message(&self.gate_ref, msg.session_id, &format!("已击杀 {}", st.name));
                                }
                            }
                        }

                        // @die（C# case "DIE"：自杀）
                        "DIE" => {
                            if let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await {
                                let died = record.actor_ref.ask(crate::actors::player::TakeDamage {
                                    attacker_id: 0, attacker_session: 0, damage: i32::MAX,
                                }).await.unwrap_or(false);
                                if died {
                                    let died_packet = Self::build_object_died_packet(st.object_id, st.x, st.y, st.direction);
                                    for (sid, _) in &self.players {
                                        let _ = self.gate_ref.tell(SendToClient {
                                            session_id: *sid,
                                            data: died_packet.clone(),
                                        }).await;
                                    }
                                    self.handle_player_death_drop(msg.session_id, st.x, st.y, st.map_index, false).await;
                                }
                            }
                        }

                        // @reloaddrops（C# case "RELOADDROPS"：重载掉落表）
                        "RELOADDROPS" => {
                            let item_name_index: std::collections::HashMap<String, i32> = self.item_infos.iter()
                                .map(|(idx, i)| (i.name.to_lowercase(), *idx))
                                .collect();
                            let drop_dir = self.map_dir.join("Envir").join("Drops");
                            if drop_dir.exists() {
                                if let Err(e) = db::import_drops_from_dir(&drop_dir, &self.monster_infos, &item_name_index, &self.db_pool).await {
                                    send_system_message(&self.gate_ref, msg.session_id, &format!("掉落重载失败：{}", e));
                                    return;
                                }
                            }
                            match db::load_monster_drops(&self.db_pool).await {
                                Ok(d) => { self.monster_drops = d; }
                                Err(e) => {
                                    send_system_message(&self.gate_ref, msg.session_id, &format!("掉落重载失败：{}", e));
                                    return;
                                }
                            }
                            self.fishing_drops = load_fishing_drops(&drop_dir, &item_name_index);
                            send_system_message(&self.gate_ref, msg.session_id, "掉落表已重载");
                        }

                        // @reloadnpcs（C# case "RELOADNPCS"：重载 NPC 配置/脚本）
                        "RELOADNPCS" => {
                            let npc_dir = self.map_dir.join("Envir").join("NPCs");
                            if npc_dir.exists() {
                                let npc_infos_vec: Vec<db::NPCInfo> = self.npc_infos.values().cloned().collect();
                                if let Err(e) = db::import_npc_scripts_from_dir(&npc_dir, &npc_infos_vec, &self.db_pool).await {
                                    send_system_message(&self.gate_ref, msg.session_id, &format!("NPC 重载失败：{}", e));
                                    return;
                                }
                            }
                            match db::load_npc_infos(&self.db_pool).await {
                                Ok(m) => { self.npc_infos = m.into_iter().map(|n| (n.index, n)).collect(); }
                                Err(e) => {
                                    send_system_message(&self.gate_ref, msg.session_id, &format!("NPC 重载失败：{}", e));
                                    return;
                                }
                            }
                            match db::load_npc_scripts(&self.db_pool).await {
                                Ok(s) => { self.npc_scripts = s; }
                                Err(e) => {
                                    send_system_message(&self.gate_ref, msg.session_id, &format!("NPC 重载失败：{}", e));
                                    return;
                                }
                            }
                            match db::load_npc_goods(&self.db_pool).await {
                                Ok(g) => { self.npc_goods = g; }
                                Err(e) => {
                                    send_system_message(&self.gate_ref, msg.session_id, &format!("NPC 重载失败：{}", e));
                                    return;
                                }
                            }
                            send_system_message(&self.gate_ref, msg.session_id, "NPC 配置已重载");
                        }

                        // @superman（C# case "SUPERMAN"：切换 GM 无敌模式 GMNeverDie）
                        "SUPERMAN" => {
                            let current = if let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await { st.gm_never_die } else { false };
                            let enabled = !current;
                            let _ = record.actor_ref.ask(crate::actors::player::SetGmNeverDie { enabled }).await;
                            send_system_message(&self.gate_ref, msg.session_id,
                                if enabled { "已开启无敌模式（不会死亡）" } else { "已关闭无敌模式" });
                        }

                        // @observer（C# case "OBSERVER"：GM 观战隐身）
                        "OBSERVER" => {
                            if let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await {
                                let hidden = !self.invisible_sessions.contains(&msg.session_id);
                                if hidden {
                                    self.invisible_sessions.insert(msg.session_id);
                                    self.hide_player_from_others(msg.session_id, &st).await;
                                    send_system_message(&self.gate_ref, msg.session_id, "已进入观战模式（对他人隐身）");
                                } else {
                                    self.invisible_sessions.remove(&msg.session_id);
                                    self.reveal_player_to_others(msg.session_id, &st).await;
                                    send_system_message(&self.gate_ref, msg.session_id, "已退出观战模式");
                                }
                            }
                        }

                        // @changeclass [玩家] <职业>（C# case "CHANGECLASS"：GM 转职）
                        "CHANGECLASS" => {
                            let class_str = parts.last().copied().unwrap_or("");
                            let class_opt = match class_str.to_uppercase().as_str() {
                                "WARRIOR" => Some(mir2_shared::enums::MirClass::Warrior),
                                "WIZARD" => Some(mir2_shared::enums::MirClass::Wizard),
                                "TAOIST" => Some(mir2_shared::enums::MirClass::Taoist),
                                "ASSASSIN" => Some(mir2_shared::enums::MirClass::Assassin),
                                "ARCHER" => Some(mir2_shared::enums::MirClass::Archer),
                                _ => None,
                            };
                            let Some(class) = class_opt else {
                                send_system_message(&self.gate_ref, msg.session_id, "用法：@changeclass [玩家] <Warrior|Wizard|Taoist|Assassin|Archer>");
                                return;
                            };
                            let target_sid = if parts.len() >= 3 {
                                let name = parts.get(1).copied().unwrap_or("");
                                let mut found = None;
                                for (_sid, other) in &self.players {
                                    if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                                        if os.name.eq_ignore_ascii_case(name) {
                                            found = Some(*_sid);
                                            break;
                                        }
                                    }
                                }
                                let Some(sid) = found else {
                                    send_system_message(&self.gate_ref, msg.session_id, "未找到在线玩家");
                                    return;
                                };
                                Some(sid)
                            } else {
                                Some(msg.session_id)
                            };
                            if let Some(sid) = target_sid {
                                if let Some(r) = self.players.get(&sid) {
                                    let _ = r.actor_ref.ask(crate::actors::player::ChangeClass { class }).await;
                                    if let Ok(Some(st)) = r.actor_ref.ask(GetPlayerState).await {
                                        self.refresh_player_appearance(sid).await;
                                        send_system_message(&self.gate_ref, msg.session_id, &format!("{} 已转职为 {:?}", st.name, class));
                                    }
                                }
                            }
                        }

                        // @setquest <id> <0|1> [玩家]（C# case "SETQUEST"：0=取消 1=完成）
                        "SETQUEST" => {
                            let quest_id = parts.get(1).and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                            let state = parts.get(2).and_then(|s| s.parse::<i32>().ok()).unwrap_or(-1);
                            if quest_id < 1 || !matches!(state, 0 | 1) {
                                send_system_message(&self.gate_ref, msg.session_id, "用法：@setquest <任务ID> <0=取消|1=完成> [玩家]");
                                return;
                            }
                            let target_sid = if parts.len() >= 4 {
                                let name = parts.get(3).copied().unwrap_or("");
                                let mut found = None;
                                for (_sid, other) in &self.players {
                                    if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                                        if os.name.eq_ignore_ascii_case(name) {
                                            found = Some(*_sid);
                                            break;
                                        }
                                    }
                                }
                                let Some(sid) = found else {
                                    send_system_message(&self.gate_ref, msg.session_id, "未找到在线玩家");
                                    return;
                                };
                                Some(sid)
                            } else {
                                Some(msg.session_id)
                            };
                            if let Some(sid) = target_sid {
                                if let Some(r) = self.players.get(&sid) {
                                    let _ = r.actor_ref.ask(crate::actors::player::GmSetQuest { quest_index: quest_id, complete: state == 1 }).await;
                                    send_system_message(&self.gate_ref, msg.session_id, &format!("任务 {} 已{}", quest_id, if state == 1 { "完成" } else { "取消" }));
                                }
                            }
                        }

                        // @clearquests [玩家]（C# case "CLEARQUESTS"：清空任务）
                        "CLEARQUESTS" => {
                            let target_sid = if let Some(n) = parts.get(1).copied() {
                                let mut found = None;
                                for (_sid, other) in &self.players {
                                    if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                                        if os.name.eq_ignore_ascii_case(n) {
                                            found = Some(*_sid);
                                            break;
                                        }
                                    }
                                }
                                let Some(sid) = found else {
                                    send_system_message(&self.gate_ref, msg.session_id, "未找到在线玩家");
                                    return;
                                };
                                Some(sid)
                            } else {
                                Some(msg.session_id)
                            };
                            if let Some(sid) = target_sid {
                                if let Some(r) = self.players.get(&sid) {
                                    let _ = r.actor_ref.ask(crate::actors::player::GmClearQuests).await;
                                    send_system_message(&self.gate_ref, msg.session_id, "任务已清空");
                                }
                            }
                        }

                        // @setflag <index> [玩家]（C# case "SETFLAG" ~3351：切换 flag）
                        "SETFLAG" => {
                            let Some(flag) = parts.get(1).and_then(|s| s.parse::<i32>().ok()) else {
                                send_system_message(&self.gate_ref, msg.session_id, "用法：@setflag <index> [玩家]");
                                return;
                            };
                            let target_sid = if let Some(n) = parts.get(2).copied() {
                                let mut found = None;
                                for (_sid, other) in &self.players {
                                    if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                                        if os.name.eq_ignore_ascii_case(n) {
                                            found = Some(*_sid);
                                            break;
                                        }
                                    }
                                }
                                found
                            } else {
                                Some(msg.session_id)
                            };
                            let Some(target_sid) = target_sid else {
                                send_system_message(&self.gate_ref, msg.session_id, "未找到在线玩家");
                                return;
                            };
                            let target = match self.players.get(&target_sid) {
                                Some(r) => r.clone(),
                                None => return,
                            };
                            let mut state = match target.actor_ref.ask(GetPlayerState).await {
                                Ok(Some(s)) => s,
                                _ => return,
                            };
                            let key = format!("NPC_FLAG_{}", flag);
                            let cur = state.flags.get(&key).copied().unwrap_or(0);
                            state.flags.insert(key, if cur == 0 { 1 } else { 0 });
                            let _ = target.actor_ref.ask(SetPlayerState { state }).await;
                            // 990-998 等级特效即时刷新
                            if (990..=998).contains(&flag) {
                                if let Some(world_ref) = self.self_ref.clone() {
                                    let _ = world_ref.tell(crate::actors::world::effects::RefreshLevelEffects {
                                        session_id: target_sid,
                                    }).try_send();
                                }
                            }
                            send_system_message(&self.gate_ref, msg.session_id, &format!("已切换 flag {} -> {}", flag, 1 - cur));
                        }
                        // @clearflags [玩家]（C# case "CLEARFLAGS" ~3383：清空 flags）
                        "CLEARFLAGS" => {
                            let target_sid = if let Some(n) = parts.get(1).copied() {
                                let mut found = None;
                                for (_sid, other) in &self.players {
                                    if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                                        if os.name.eq_ignore_ascii_case(n) {
                                            found = Some(*_sid);
                                            break;
                                        }
                                    }
                                }
                                found
                            } else {
                                Some(msg.session_id)
                            };
                            let Some(target_sid) = target_sid else {
                                send_system_message(&self.gate_ref, msg.session_id, "未找到在线玩家");
                                return;
                            };
                            let target = match self.players.get(&target_sid) {
                                Some(r) => r.clone(),
                                None => return,
                            };
                            let mut state = match target.actor_ref.ask(GetPlayerState).await {
                                Ok(Some(s)) => s,
                                _ => return,
                            };
                            state.flags.clear();
                            let _ = target.actor_ref.ask(SetPlayerState { state }).await;
                            if let Some(world_ref) = self.self_ref.clone() {
                                let _ = world_ref.tell(crate::actors::world::effects::RefreshLevelEffects {
                                    session_id: target_sid,
                                }).try_send();
                            }
                            send_system_message(&self.gate_ref, msg.session_id, "flags 已清空");
                        }
                        // @deleteskill [玩家] <技能名>（C# case "DELETESKILL" ~4075）
                        "DELETESKILL" => {
                            let (target_sid, skill_arg) = if parts.len() >= 3 {
                                let name = parts.get(1).copied().unwrap_or("");
                                let mut found = None;
                                for (_sid, other) in &self.players {
                                    if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                                        if os.name.eq_ignore_ascii_case(name) {
                                            found = Some(*_sid);
                                            break;
                                        }
                                    }
                                }
                                (found, parts.get(2).copied().unwrap_or(""))
                            } else {
                                (Some(msg.session_id), parts.get(1).copied().unwrap_or(""))
                            };
                            let Some(target_sid) = target_sid else {
                                send_system_message(&self.gate_ref, msg.session_id, "未找到在线玩家");
                                return;
                            };
                            let Some(info) = self.magic_infos.values().find(|m| m.name.eq_ignore_ascii_case(skill_arg)).cloned() else {
                                send_system_message(&self.gate_ref, msg.session_id, &format!("未找到技能：{}", skill_arg));
                                return;
                            };
                            let target = match self.players.get(&target_sid) {
                                Some(r) => r.clone(),
                                None => return,
                            };
                            let mut state = match target.actor_ref.ask(GetPlayerState).await {
                                Ok(Some(s)) => s,
                                _ => return,
                            };
                            let before = state.magics.len();
                            state.magics.retain(|m| m.spell != info.spell);
                            if state.magics.len() != before {
                                let _ = target.actor_ref.ask(SetPlayerState { state }).await;
                                self.send_remove_magic_packet(target_sid, info.spell).await;
                                send_system_message(&self.gate_ref, msg.session_id, &format!("已删除技能 {}", info.name));
                            } else {
                                send_system_message(&self.gate_ref, msg.session_id, "对方未学会该技能");
                            }
                        }
                        // @giveheroskill [玩家] <技能名> <等级0-3>（对齐 C# HEROGIVESKILL 语义）
                        "GIVEHEROSKILL" => {
                            let (target_sid, skill_arg, level_arg) = if parts.len() >= 4 {
                                let name = parts.get(1).copied().unwrap_or("");
                                let mut found = None;
                                for (_sid, other) in &self.players {
                                    if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                                        if os.name.eq_ignore_ascii_case(name) {
                                            found = Some(*_sid);
                                            break;
                                        }
                                    }
                                }
                                (found, parts.get(2).copied().unwrap_or(""), parts.get(3).copied().unwrap_or("0"))
                            } else {
                                (Some(msg.session_id), parts.get(1).copied().unwrap_or(""), parts.get(2).copied().unwrap_or("0"))
                            };
                            let Some(target_sid) = target_sid else {
                                send_system_message(&self.gate_ref, msg.session_id, "未找到在线玩家");
                                return;
                            };
                            let Some(info) = self.magic_infos.values().find(|m| m.name.eq_ignore_ascii_case(skill_arg)).cloned() else {
                                send_system_message(&self.gate_ref, msg.session_id, &format!("未找到技能：{}", skill_arg));
                                return;
                            };
                            let level = level_arg.parse::<u8>().unwrap_or(0).min(3);
                            let target = match self.players.get(&target_sid) {
                                Some(r) => r.clone(),
                                None => return,
                            };
                            let mut state = match target.actor_ref.ask(GetPlayerState).await {
                                Ok(Some(s)) => s,
                                _ => return,
                            };
                            if !state.hero_magics.iter().any(|m| m.spell == info.spell) {
                                let mut m = crate::actors::player::PlayerMagic::new(info.spell);
                                m.level = level;
                                state.hero_magics.push(m);
                                let _ = target.actor_ref.ask(SetPlayerState { state }).await;
                                self.send_hero_information_packet(target_sid).await;
                                send_system_message(&self.gate_ref, msg.session_id, &format!("英雄已学会技能 {}", info.name));
                            } else {
                                send_system_message(&self.gate_ref, msg.session_id, "英雄已学会该技能");
                            }
                        }
                        // @givepearls [玩家] <数量>（C# case "GIVEPEARLS" ~3103：GainPearls，上限 int.MaxValue）
                        "GIVEPEARLS" => {
                            let amount = parts.last().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                            if amount == 0 { return; }
                            match parts.get(1).copied() {
                                Some(n) if parts.len() >= 3 => {
                                    let mut found = false;
                                    for (_sid, other) in &self.players {
                                        if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                                            if os.name.eq_ignore_ascii_case(n) {
                                                let _ = other.actor_ref.ask(crate::actors::player::GainPearls { amount }).await;
                                                send_system_message(&self.gate_ref, msg.session_id, &format!("已给 {} {} 珍珠", os.name, amount));
                                                found = true;
                                                break;
                                            }
                                        }
                                    }
                                    if !found {
                                        send_system_message(&self.gate_ref, msg.session_id, &format!("未找到在线玩家：{}", n));
                                    }
                                }
                                _ => {
                                    let _ = record.actor_ref.ask(crate::actors::player::GainPearls { amount }).await;
                                    send_system_message(&self.gate_ref, msg.session_id, &format!("已获得 {} 珍珠", amount));
                                }
                            }
                        }
                        // @givecredit [玩家] <数量>（C# case "GIVECREDIT" ~3135：账户积分，上限 uint.MaxValue）
                        "GIVECREDIT" => {
                            let amount = parts.last().and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                            if amount == 0 { return; }
                            let target_sid = if parts.len() >= 3 {
                                let name = parts.get(1).copied().unwrap_or("");
                                let mut found = None;
                                for (_sid, other) in &self.players {
                                    if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                                        if os.name.eq_ignore_ascii_case(name) {
                                            found = Some(*_sid);
                                            break;
                                        }
                                    }
                                }
                                found
                            } else {
                                Some(msg.session_id)
                            };
                            let Some(target_sid) = target_sid else {
                                send_system_message(&self.gate_ref, msg.session_id, "未找到在线玩家");
                                return;
                            };
                            self.npc_change_credit(target_sid, amount as i64).await;
                        }
                        // @mapmove <地图名> [x] [y]（C# case "MAPMOVE" ~2872：按地图名传送，无坐标随机落点）
                        "MAPMOVE" => {
                            let Some(map_name) = parts.get(1).copied() else {
                                send_system_message(&self.gate_ref, msg.session_id, "用法：@mapmove <地图名> [x] [y]");
                                return;
                            };
                            let Some(mi) = self.map_infos.values().find(|m| m.file_name.eq_ignore_ascii_case(map_name)).cloned() else {
                                send_system_message(&self.gate_ref, msg.session_id, &format!("未找到地图：{}", map_name));
                                return;
                            };
                            let map_index = mi.index as u16;
                            let (x, y) = if let (Some(x), Some(y)) = (
                                parts.get(2).and_then(|s| s.parse::<i32>().ok()),
                                parts.get(3).and_then(|s| s.parse::<i32>().ok()),
                            ) {
                                (x, y)
                            } else {
                                // C# TeleportRandom(200, 0, map)：无坐标时随机落点
                                let (w, h) = match self.get_or_load_map(&mi.file_name, map_index) {
                                    Some(m) => (m.width as i32, m.height as i32),
                                    None => (200, 200),
                                };
                                (fastrand::i32(0..w.max(1)), fastrand::i32(0..h.max(1)))
                            };
                            crate::actors::world::npc_script::teleport_player(self, msg.session_id, map_index, x, y).await;
                            send_system_message(&self.gate_ref, msg.session_id, &format!("已传送至 {} ({}, {})", mi.title, x, y));
                        }
                        // @listflags（C# case "LISTFLAGS" ~3372：列出玩家 flags）
                        "LISTFLAGS" => {
                            let state = match record.actor_ref.ask(GetPlayerState).await {
                                Ok(Some(s)) => s,
                                _ => return,
                            };
                            if state.flags.is_empty() {
                                send_system_message(&self.gate_ref, msg.session_id, "当前没有 flag");
                            } else {
                                for (k, v) in &state.flags {
                                    send_system_message(&self.gate_ref, msg.session_id, &format!("flag {} = {}", k, v));
                                }
                            }
                        }
                        // @startwar <行会名>（C# case "STARTWAR" ~3597：GM + 会长宣战，复用 GuildWarReturn 宣战流程）
                        "STARTWAR" => {
                            let Some(enemy) = parts.get(1).copied() else {
                                send_system_message(&self.gate_ref, msg.session_id, "用法：@startwar <行会名>");
                                return;
                            };
                            self.declare_guild_war(msg.session_id, enemy.to_string()).await;
                        }
                        // @createguild [玩家] <行会名>（C# case "CREATEGUILD" ~3264：GM 直接建会，跳过等级/金币）
                        "CREATEGUILD" => {
                            let (target_sid, guild_name) = if parts.len() >= 3 {
                                let name = parts.get(1).copied().unwrap_or("");
                                let mut found = None;
                                for (_sid, other) in &self.players {
                                    if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                                        if os.name.eq_ignore_ascii_case(name) {
                                            found = Some(*_sid);
                                            break;
                                        }
                                    }
                                }
                                (found, parts.get(2).copied().unwrap_or(""))
                            } else {
                                (Some(msg.session_id), parts.get(1).copied().unwrap_or(""))
                            };
                            let Some(target_sid) = target_sid else {
                                send_system_message(&self.gate_ref, msg.session_id, "未找到在线玩家");
                                return;
                            };
                            let _ = self.social_ref.ask(crate::actors::social::GmCreateGuildRequest {
                                session_id: target_sid,
                                guild_name: guild_name.to_string(),
                            }).await;
                        }
                        _ => {}
                    }
                    return;
                }
            }
        }

        // #888：@ADDSTORAGE —— 1,000,000 金币购买 10 天仓库扩容（C# PlayerObject case "ADDSTORAGE"，
        // 无 GM 校验；首次 80→160，已扩容则 +10 天续期；下发 LoseGold + ResizeStorage + 系统消息）
        if let Some(cmd_rest) = message.strip_prefix('@') {
            let parts: Vec<&str> = cmd_rest.split_whitespace().collect();
            if parts.first().is_some_and(|c| c.eq_ignore_ascii_case("ADDSTORAGE")) {
                const COST: u64 = 1_000_000;
                const ADDED_SECS: i64 = 10 * 24 * 60 * 60; // C# new TimeSpan(10,0,0,0) = 10 天

                if let Some(record) = self.players.get(&msg.session_id) {
                    // 金币校验（C# Account.Gold >= cost；不足 → LowGold 系统消息）
                    if !record.actor_ref.ask(crate::actors::player::HasGold { amount: COST }).await.unwrap_or(false) {
                        send_system_message(&self.gate_ref, msg.session_id, "金币不足，无法购买仓库扩容（需要 1,000,000 金币）。");
                        return;
                    }
                    let deducted = record.actor_ref.ask(crate::actors::player::DeductGold { amount: COST }).await.unwrap_or(false);
                    if !deducted {
                        return;
                    }
                    // C# S.LoseGold（DeductGold 只刷 UserInformation，这里补发扣金包）
                    send_gold_changed_packet(&self.gate_ref, msg.session_id, COST);

                    let now_unix = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);

                    let mut new_state = match record.actor_ref.ask(GetPlayerState).await {
                        Ok(Some(s)) => s,
                        _ => return,
                    };
                    // 扩容（C# Account.ExpandStorage：80 → 160）
                    let new_len = new_state.inventory.expand_storage();
                    new_state.has_expanded_storage = true;
                    // 到期时间：已激活则 +10 天，否则从现在起 +10 天（C# ExpandedStorageExpiryDate）
                    let expiry = if new_state.expanded_storage_expiry_date > now_unix {
                        new_state.expanded_storage_expiry_date + ADDED_SECS
                    } else {
                        now_unix + ADDED_SECS
                    };
                    new_state.expanded_storage_expiry_date = expiry;
                    let _ = record.actor_ref.ask(SetPlayerState { state: new_state }).await;

                    // C# S.ResizeStorage{Size, HasExpandedStorage=true, ExpiryTime}
                    let resize = mir2_shared::packets::server::ui_events::ResizeStorage {
                        size: new_len as i32,
                        has_expanded_storage: true,
                        expiry_time: expiry,
                    };
                    let mut resize_body = Vec::new();
                    if resize.write_body(&mut resize_body).is_ok() {
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: msg.session_id,
                            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ResizeStorage as i16, &resize_body),
                        }).await;
                    }

                    // DB 持久化（重启不丢；C# AccountInfo 存档）
                    if let Err(e) = db::update_account_storage_expansion(
                        &self.db_pool,
                        &record.account_username,
                        true,
                        expiry,
                    ).await {
                        warn!("Failed to persist storage expansion for {}: {}", record.name, e);
                    }

                    // C# ExpandedStorageExpiresOn + 到期时间
                    let dt = chrono::DateTime::from_timestamp(expiry, 0)
                        .map(|d| d.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| expiry.to_string());
                    send_system_message(
                        &self.gate_ref,
                        msg.session_id,
                        &format!("仓库扩容成功！仓库已扩容至 {} 格，到期时间：{}", new_len, dt),
                    );
                }
                return;
            }
        }

        // #891：@TIME / @MAP / @ROLL（C# 非 GM 聊天命令：PlayerObject case "TIME"/"MAP"/"ROLL"）
        if let Some(cmd_rest) = message.strip_prefix('@') {
            let parts: Vec<&str> = cmd_rest.split_whitespace().collect();
            let cmd = parts.first().map(|s| s.to_uppercase());
            match cmd.as_deref() {
                Some("TIME") => {
                    // C#：TheTimeIs + Envir.Now.ToString("hh:mm tt")（12 小时制 AM/PM）
                    let now = chrono::Local::now().format("%I:%M %p").to_string();
                    send_system_message(&self.gate_ref, msg.session_id, &format!("服务器时间：{}", now));
                    return;
                }
                Some("MAP") => {
                    // C#：YouAreInMapId + CurrentMap.Info.Title / FileName
                    let (map_title, map_file) = if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                        self.map_infos.get(&(state.map_index as i32))
                            .map(|m| (m.title.clone(), m.file_name.clone()))
                            .unwrap_or_else(|| ("未知地图".to_string(), String::new()))
                    } else {
                        ("未知地图".to_string(), String::new())
                    };
                    send_system_message(&self.gate_ref, msg.session_id, &format!("你所在的地图：{} ({})", map_title, map_file));
                    return;
                }
                Some("TELEPORT") | Some("MOVE") => {
                    // C# case "MOVE"（~2850）：GM 或 Teleport 特殊装备可用；10s 冷却；NoPosition 地图非 GM 禁止
                    let state = match record.actor_ref.ask(GetPlayerState).await {
                        Ok(Some(s)) => s,
                        _ => return,
                    };
                    let has_tp = super::has_special_equipped(&state, mir2_shared::enums::SpecialItemMode::TELEPORT);
                    if !state.is_gm && !has_tp {
                        send_system_message(&self.gate_ref, msg.session_id, "你没有权限使用此命令");
                        return;
                    }
                    if !state.is_gm
                        && self.map_infos.get(&(state.map_index as i32)).map(|m| m.no_position).unwrap_or(false)
                    {
                        send_system_message(&self.gate_ref, msg.session_id, "该地图禁止传送");
                        return;
                    }
                    if !state.is_gm {
                        let now_ms = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis() as i64)
                            .unwrap_or(0);
                        if let Some(last) = self.last_teleport_time.get(&msg.session_id).copied() {
                            if now_ms - last < 10_000 {
                                send_system_message(&self.gate_ref, msg.session_id, "传送冷却中，请稍后再试");
                                return;
                            }
                        }
                        self.last_teleport_time.insert(msg.session_id, now_ms);
                    }
                    let x = parts.get(1).and_then(|s| s.parse::<i32>().ok());
                    let y = parts.get(2).and_then(|s| s.parse::<i32>().ok());
                    if let (Some(x), Some(y)) = (x, y) {
                        let _ = record.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                            x, y, direction: 4, map_index: None, is_mounted: None,
                        }).await;
                        send_system_message(&self.gate_ref, msg.session_id, &format!("已传送至 ({}, {})", x, y));
                    } else {
                        send_system_message(&self.gate_ref, msg.session_id, "用法：@move <x> <y>");
                    }
                    return;
                }
                Some("FIND") => {
                    // C# case "FIND"（~3229）：GM 或 Probe 特殊装备可用；非 GM 180s 冷却
                    let state = match record.actor_ref.ask(GetPlayerState).await {
                        Ok(Some(s)) => s,
                        _ => return,
                    };
                    let has_probe = super::has_special_equipped(&state, mir2_shared::enums::SpecialItemMode::PROBE);
                    if !state.is_gm && !has_probe {
                        send_system_message(&self.gate_ref, msg.session_id, "你没有权限使用此命令");
                        return;
                    }
                    let Some(target_name) = parts.get(1).copied() else {
                        send_system_message(&self.gate_ref, msg.session_id, "用法：@find <玩家名>");
                        return;
                    };
                    if !state.is_gm {
                        let now_ms = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis() as i64)
                            .unwrap_or(0);
                        if let Some(last) = self.last_probe_time.get(&msg.session_id).copied() {
                            if now_ms - last < 180_000 {
                                send_system_message(&self.gate_ref, msg.session_id, "探测冷却中，请稍后再试");
                                return;
                            }
                        }
                        self.last_probe_time.insert(msg.session_id, now_ms);
                    }
                    for (_sid, other) in &self.players {
                        if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                            if os.name.eq_ignore_ascii_case(target_name) {
                                let title = self.map_infos.get(&(os.map_index as i32))
                                    .map(|m| m.title.clone())
                                    .unwrap_or_else(|| "未知地图".to_string());
                                send_system_message(
                                    &self.gate_ref,
                                    msg.session_id,
                                    &format!("{} 在 {} ({}, {})", os.name, title, os.x, os.y),
                                );
                                return;
                            }
                        }
                    }
                    send_system_message(&self.gate_ref, msg.session_id, &format!("未找到在线玩家：{}", target_name));
                    return;
                }
                Some("SUMMONHERO") => {
                    // C# case "SUMMONHERO"（~3697）：有英雄时召唤/收起出战英雄
                    let state = match record.actor_ref.ask(GetPlayerState).await {
                        Ok(Some(s)) => s,
                        _ => return,
                    };
                    if state.hero_index == 0 {
                        send_system_message(&self.gate_ref, msg.session_id, "你没有英雄");
                        return;
                    }
                    let mut new_state = state.clone();
                    new_state.hero_despawned = !new_state.hero_despawned;
                    let now_spawned = !new_state.hero_despawned;
                    let object_id = state.object_id;
                    let _ = record.actor_ref.ask(SetPlayerState { state: new_state }).await;
                    if now_spawned {
                        // 召唤英雄（C# SummonHero：生成英雄对象 + 下发完整信息）
                        self.broadcast_hero_spawn(msg.session_id).await;
                        self.send_hero_information_packet(msg.session_id).await;
                        send_system_message(&self.gate_ref, msg.session_id, "英雄已出战");
                    } else {
                        // 收起英雄（C# DespawnHero）
                        self.broadcast_hero_remove(object_id).await;
                        send_system_message(&self.gate_ref, msg.session_id, "英雄已收起");
                    }
                    return;
                }
                Some("CLEARBUFFS") => {
                    // C# case "CLEARBUFFS"（~2411）：清除自己全部 Buff（无 GM 校验）
                    let _ = record.actor_ref.ask(crate::actors::player::ClearAllBuffs).await;
                    send_system_message(&self.gate_ref, msg.session_id, "已清除全部状态效果");
                    return;
                }
                Some("LEAVEGUILD") => {
                    // C# case "LEAVEGUILD"（~3251）：退会；开战期间禁止（CannotLeaveGuildAtWar）
                    let state = match record.actor_ref.ask(GetPlayerState).await {
                        Ok(Some(s)) => s,
                        _ => return,
                    };
                    let Some(guild) = state.guild_name.clone() else {
                        return;
                    };
                    if self.guild_wars.get(&guild).map(|s| !s.is_empty()).unwrap_or(false) {
                        send_system_message(&self.gate_ref, msg.session_id, "行会战争中无法离开行会");
                        return;
                    }
                    let _ = self.social_ref.ask(crate::actors::social::LeaveGuildRequest {
                        session_id: msg.session_id,
                    }).await;
                    return;
                }
                Some("RECALL") => {
                    // C# case "RECALL"（~2471）：仅 GM——把指定玩家传送到自己面前（Teleport(CurrentMap, Front)）
                    let (is_gm, my_state) = match record.actor_ref.ask(GetPlayerState).await {
                        Ok(Some(s)) => (s.is_gm, s),
                        _ => return,
                    };
                    if !is_gm {
                        send_system_message(&self.gate_ref, msg.session_id, "你没有权限使用此命令");
                        return;
                    }
                    let target_name = match parts.get(1) {
                        Some(n) => *n,
                        None => {
                            send_system_message(&self.gate_ref, msg.session_id, "用法：@recall <玩家名>");
                            return;
                        }
                    };
                    // 面前格子（C# Front）
                    const DIR_DX: [i32; 8] = [0, 1, 1, 1, 0, -1, -1, -1];
                    const DIR_DY: [i32; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];
                    let dir = (my_state.direction as usize) % 8;
                    let fx = my_state.x + DIR_DX[dir];
                    let fy = my_state.y + DIR_DY[dir];
                    for (sid, other) in &self.players {
                        if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                            if os.name.eq_ignore_ascii_case(target_name) {
                                let _ = other.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                                    x: fx,
                                    y: fy,
                                    direction: os.direction,
                                    map_index: Some(my_state.map_index),
                                    is_mounted: None,
                                }).await;
                                let mut body = Vec::new();
                                body.push(os.direction);
                                body.extend_from_slice(&fx.to_le_bytes());
                                body.extend_from_slice(&fy.to_le_bytes());
                                let _ = self.gate_ref.tell(SendToClient {
                                    session_id: *sid,
                                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::UserLocation as i16, &body),
                                }).await;
                                send_system_message(&self.gate_ref, msg.session_id, &format!("已将 {} 传送到你身边", os.name));
                                return;
                            }
                        }
                    }
                    send_system_message(&self.gate_ref, msg.session_id, &format!("未找到在线玩家：{}", target_name));
                    return;
                }
                Some("ALLOWTRADE") => {
                    // C# case "ALLOWTRADE"（~3309）：切换 AllowTrade + 系统消息
                    let mut new_state = match record.actor_ref.ask(GetPlayerState).await {
                        Ok(Some(s)) => s,
                        _ => return,
                    };
                    new_state.allow_trade = !new_state.allow_trade;
                    let enabled = new_state.allow_trade;
                    let _ = record.actor_ref.ask(SetPlayerState { state: new_state }).await;
                    send_system_message(
                        &self.gate_ref,
                        msg.session_id,
                        if enabled { "已开启交易（其他玩家可向你发起交易）" } else { "已关闭交易" },
                    );
                    return;
                }
                Some("STARTCONQUEST") | Some("RESETCONQUEST") => {
                    // C# case "STARTCONQUEST"（~3854）/ "RESETCONQUEST"（~3900）：攻城 GM 命令
                    let (is_gm, my_guild) = match record.actor_ref.ask(GetPlayerState).await {
                        Ok(Some(s)) => (s.is_gm, s.guild_name),
                        _ => return,
                    };
                    if !is_gm {
                        send_system_message(&self.gate_ref, msg.session_id, "你没有权限使用此命令");
                        return;
                    }
                    let Some(guild) = my_guild else {
                        send_system_message(&self.gate_ref, msg.session_id, "需要行会才能使用攻城命令");
                        return;
                    };
                    let Some(id_str) = parts.get(1) else {
                        send_system_message(&self.gate_ref, msg.session_id, &format!("用法：@{} <领地ID>", parts[0].to_lowercase()));
                        return;
                    };
                    let Ok(conquest_id) = id_str.parse::<i32>() else { return; };

                    let is_start = parts[0].eq_ignore_ascii_case("STARTCONQUEST");
                    let mut found = false;
                    if let Some(inst) = self.conquest_instances.iter_mut().find(|c| c.id == conquest_id) {
                        found = true;
                        if is_start {
                            if inst.state == crate::actors::world::conquest::WarState::InProgress {
                                inst.end_war();
                                broadcast_system_message(
                                    &self.gate_ref, &self.players,
                                    &format!("攻城战 {} 已停止", conquest_id),
                                );
                            } else {
                                inst.start_war(&guild);
                                broadcast_system_message(
                                    &self.gate_ref, &self.players,
                                    &format!("攻城战 {} 已开始（攻击方：{}）", conquest_id, guild),
                                );
                            }
                        } else if inst.state != crate::actors::world::conquest::WarState::InProgress {
                            inst.reset();
                            send_system_message(&self.gate_ref, msg.session_id, &format!("领地 {} 已重置", conquest_id));
                        } else {
                            send_system_message(&self.gate_ref, msg.session_id, "攻城进行中无法重置");
                        }
                    }
                    if !found {
                        send_system_message(&self.gate_ref, msg.session_id, &format!("未找到领地：{}", conquest_id));
                    }
                    return;
                }
                Some("GATES") => {
                    // C# case "GATES"（~3932）：行会城门开关（领地拥有者 + 副会长/会长 + 非开战）
                    let state = match record.actor_ref.ask(GetPlayerState).await {
                        Ok(Some(s)) => s,
                        _ => return,
                    };
                    let Some(guild) = state.guild_name.clone() else {
                        send_system_message(&self.gate_ref, msg.session_id, "没有行会，无法控制城门");
                        return;
                    };
                    if state.guild_rank == crate::actors::guild::GuildRank::Member {
                        send_system_message(&self.gate_ref, msg.session_id, "没有权限控制城门");
                        return;
                    }
                    let Some(inst) = self.conquest_instances.iter()
                        .find(|c| c.owner_guild.as_deref() == Some(guild.as_str()))
                        .cloned()
                    else {
                        send_system_message(&self.gate_ref, msg.session_id, "你的行会没有领地");
                        return;
                    };
                    if inst.state == crate::actors::world::conquest::WarState::InProgress {
                        send_system_message(&self.gate_ref, msg.session_id, "攻城期间无法控制城门");
                        return;
                    }
                    // 参数：OPEN / CLOSE / 无参（逐门翻转，C# 按各门当前 Closed 状态）
                    let cmd = parts.get(1).map(|s| s.to_uppercase());
                    let force_close = match cmd.as_deref() {
                        Some("CLOSE") => Some(true),
                        Some("OPEN") => Some(false),
                        None => None,
                        Some(_) => {
                            send_system_message(&self.gate_ref, msg.session_id, "用法：@gates [open|close]");
                            return;
                        }
                    };
                    // 收集领地城门（排序与 find_siege_structure 的 1-based id 一致）
                    let mut gates: Vec<u32> = self.siege_structures.iter()
                        .filter(|(_, s)| {
                            s.conquest_id == inst.id
                                && s.structure_type == crate::actors::world::conquest::SiegeStructureType::CastleGate
                                && s.hp > 0
                        })
                        .map(|(oid, _)| *oid)
                        .collect();
                    gates.sort();
                    let map_index = inst.map_index as u16;
                    let mut any_closed = false;
                    for (i, oid) in gates.iter().enumerate() {
                        let gate_id = (i + 1) as u8;
                        let close = match force_close {
                            Some(c) => c,
                            None => !self.siege_structures.get(oid).map(|s| s.is_open).unwrap_or(false),
                        };
                        if let Some(s) = self.siege_structures.get_mut(oid) {
                            s.is_open = !close;
                        }
                        any_closed |= close;
                        super::broadcast_opendoor_async(
                            &self.gate_ref,
                            &self.players,
                            map_index,
                            gate_id,
                            close,
                            msg.session_id,
                        ).await;
                    }
                    if !gates.is_empty() {
                        send_system_message(
                            &self.gate_ref,
                            msg.session_id,
                            if any_closed { "城门已关闭" } else { "城门已打开" },
                        );
                    } else {
                        send_system_message(&self.gate_ref, msg.session_id, "领地没有可控制的城门");
                    }
                    return;
                }
                Some("ALLOWGUILD") => {
                    // C# case "ALLOWGUILD"（~2466）：切换 EnableGuildInvite + 提示
                    let mut new_state = match record.actor_ref.ask(GetPlayerState).await {
                        Ok(Some(s)) => s,
                        _ => return,
                    };
                    new_state.enable_guild_invite = !new_state.enable_guild_invite;
                    let enabled = new_state.enable_guild_invite;
                    let _ = record.actor_ref.ask(SetPlayerState { state: new_state }).await;
                    send_system_message(
                        &self.gate_ref,
                        msg.session_id,
                        if enabled { "已开启行会邀请（他人可邀请你加入行会）" } else { "已关闭行会邀请" },
                    );
                    return;
                }
                Some("ALLOWOBSERVE") => {
                    // C# case "ALLOWOBSERVE"：AllowObserve = !AllowObserve + S.AllowObserve
                    let mut new_state = match record.actor_ref.ask(GetPlayerState).await {
                        Ok(Some(s)) => s,
                        _ => return,
                    };
                    new_state.allow_observe = !new_state.allow_observe;
                    let allowed = new_state.allow_observe;
                    let _ = record.actor_ref.ask(SetPlayerState { state: new_state }).await;
                    let pkt = mir2_shared::packets::server::miscellaneous::AllowObserve { allowed };
                    let mut observe_body = Vec::new();
                    if pkt.write_body(&mut observe_body).is_ok() {
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: msg.session_id,
                            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::AllowObserve as i16, &observe_body),
                        }).await;
                    }
                    send_system_message(
                        &self.gate_ref,
                        msg.session_id,
                        if allowed { "已允许其他玩家观察你" } else { "已禁止其他玩家观察你" },
                    );
                    return;
                }
                Some("ADDINVENTORY") => {
                    // C# case "ADDINVENTORY"（~3644）：cost = 1M + openLevel*1M，
                    // openLevel = (Inventory.Length - 46) / 4（Rust 基线 40 → (len-40)/4）；
                    // ResizeInventory 首次 +8 之后 +4 上限 86；成功发 LoseGold + ResizeInventory
                    let mut new_state = match record.actor_ref.ask(GetPlayerState).await {
                        Ok(Some(s)) => s,
                        _ => return,
                    };
                    let level = (new_state.inventory.backpack.len() as i64
                        - crate::actors::inventory::BACKPACK_SIZE as i64).max(0) / 4;
                    let cost = 1_000_000u64 + (level as u64) * 1_000_000u64;
                    if !record.actor_ref.ask(crate::actors::player::HasGold { amount: cost }).await.unwrap_or(false) {
                        send_system_message(&self.gate_ref, msg.session_id, "金币不足，无法扩展背包。");
                        return;
                    }
                    let deducted = record.actor_ref.ask(crate::actors::player::DeductGold { amount: cost }).await.unwrap_or(false);
                    if !deducted {
                        return;
                    }
                    // C# S.LoseGold（DeductGold 只刷 UserInformation，这里补发扣金包）
                    send_gold_changed_packet(&self.gate_ref, msg.session_id, cost);

                    let new_len = new_state.inventory.resize_inventory();
                    let _ = record.actor_ref.ask(SetPlayerState { state: new_state }).await;

                    // C# S.ResizeInventory{Size}
                    let resize = mir2_shared::packets::server::ui_events::ResizeInventory { size: new_len as i32 };
                    let mut resize_body = Vec::new();
                    if resize.write_body(&mut resize_body).is_ok() {
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: msg.session_id,
                            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ResizeInventory as i16, &resize_body),
                        }).await;
                    }
                    send_system_message(&self.gate_ref, msg.session_id, &format!("背包扩容成功！背包已扩容至 {} 格", new_len));
                    return;
                }
                Some("ROLL") => {
                    // C#：Envir.Random.Next(5) + 1（1~5）；GroupMembers == null 直接 return；
                    // 向所有组员发 ChatType.Group 消息 HasRolledNumber
                    let (player_name, group_id) = if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                        (state.name.clone(), state.group_id)
                    } else {
                        return;
                    };
                    let Some(gid) = group_id else { return; };
                    let dice = fastrand::i32(1..=5);
                    let text = format_roll_message(&player_name, dice);
                    let mut body = Vec::new();
                    write_dotnet_string(&mut body, &text);
                    body.push(mir2_shared::enums::ChatType::Group as u8);
                    let packet = build_packet_bytes(mir2_shared::enums::ServerPacketIds::Chat as i16, &body);
                    for (sid, other) in &self.players {
                        if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                            if os.group_id == Some(gid) {
                                let _ = self.gate_ref.tell(SendToClient {
                                    session_id: *sid,
                                    data: packet.clone(),
                                }).await;
                            }
                        }
                    }
                    return;
                }
                _ => {}
            }
        }

        // Check for social chat commands and forward to SocialActor
        let parts: Vec<&str> = message.split_whitespace().collect();
        // 去掉前导 @（C# 客户端命令如 @ride 均带 @）
        let cmd = parts.first().unwrap_or(&"").trim_start_matches('@').to_uppercase();
        match cmd.as_str() {
            "GROUPRECALL" | "RECALLMEMBER" | "RECALLLOVER" | "ENABLEGROUPRECALL" | "DISABLEGROUPRECALL" | "RIDE" => {
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

        // #1344：普通聊天对齐 C#（PlayerObject.Chat → CurrentMap.Broadcast(S.ObjectChat)）：
        // 只广播同地图玩家（此前全服串线），并改用 S.ObjectChat（带 object_id）
        let (sender_object_id, sender_map) = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(state)) => (state.object_id, state.map_index),
            _ => return,
        };
        let formatted = format!("[{}]: {}", player_name, message);
        debug!("Chat from {}: {}", player_name, message);
        let packet = build_packet_bytes(
            mir2_shared::enums::ServerPacketIds::ObjectChat as i16,
            &object_chat_body(sender_object_id, &formatted, mir2_shared::enums::ChatType::Normal as u8),
        );
        for (sid, other) in &self.players {
            // 不给自己回发（本地已 add_message）
            if *sid == msg.session_id {
                continue;
            }
            if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                if os.map_index == sender_map {
                    let _ = self.gate_ref.tell(SendToClient {
                        session_id: *sid,
                        data: packet.clone(),
                    }).await;
                }
            }
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

        // 更新玩家宠物模式（WorldActor 缓存供宠物 AI 读取；PlayerActor 持久化）
        let _ = record.actor_ref.ask(SetPetMode { mode: msg.mode }).await;
        self.player_pet_modes.insert(msg.session_id, msg.mode);

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
        // #937：卸装/换装后临时技能同步
        if success {
            self.sync_temp_skills(msg.session_id).await;
        }
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
        married_date: 0,
        allow_mentor: false,
        mentor_name: None,
        creature_log: CreatureLog::new(),
        hero_index: 0,
        hero_behaviour: 0,
        hero_despawned: false,
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
        gm_never_die: false, // #1480：GM 无敌模式（C# GMNeverDie）
        special_shot_armed: 0, // #1483：弓手特殊箭武装（0=无 1=Vampire 2=Poison）
        has_expanded_storage: false,
        expanded_storage_expiry_date: 0,
        has_storage_password: false,
        require_storage_password: false,
        storage_password_last_set: 0,
        allow_observe: false,
        enable_guild_invite: false,
        allow_trade: false,
        allow_group: false,
        pk_points: 0,
        pk_kill_count: 0,
        buffs: Vec::new(),
        magics: Vec::new(),
        flags: std::collections::HashMap::new(),
        exp_multiplier: 1.0,
        exp_rate: 1.0,
        exp_multiplier_end_tick: 0,
            drop_multiplier: 1.0,
            drop_multiplier_end_tick: 0,
            item_drop_rate_percent: 0,
            gold_drop_rate_percent: 0,
            elements_level: 0,
            has_elemental: false,
            concentration_interrupted: false,
            concentration_interrupt_time: 0,
            bind_map_index: 0,
            bind_x: 0,
            bind_y: 0,
            level_effects: 0,
            is_mentor: false,
            mentee_exp: 0,
            mentor_damage_bonus: false,
            newbie_exp_bonus: false,
            exp_bonus_lover_percent: 0,
            exp_bonus_mentee_percent: 0,
            exp_bonus_newbie_percent: 0,
            guild_buff_exp_percent: 0,
            guild_buff_fish_rate_percent: 0,
            no_experience_map: false,
            brown_until_ms: 0,
            mount_loyalty_decrease_time: 0,
            mount_loyalty_increase_time: 0,
            torch_burn_time: 0,
            last_damage_ms: 0,
            pot_hp_amount: 0,
            pot_mp_amount: 0,
            pot_time_ms: 0,
    }
}

/// #891：@ROLL 组队掷骰消息（C# HasRolledNumber："{Name} 掷出了 {N} 点"）
fn format_roll_message(player_name: &str, dice: i32) -> String {
    format!("{} 掷出了 {} 点", player_name, dice)
}

/// #1408：目标格是否被阻挡（NPC 或攻城结构占用，C# 移动阻挡语义）
fn tile_blocked_by(tx: i32, ty: i32, npc_tiles: &[(i32, i32)], struct_tiles: &[(i32, i32)]) -> bool {
    npc_tiles.contains(&(tx, ty)) || struct_tiles.contains(&(tx, ty))
}

/// #1344：构建 S.ObjectChat body（wire 对齐 C# ObjectChat：[ObjectID u32][Text dotnet][ChatType u8]）
fn object_chat_body(object_id: u32, text: &str, chat_type: u8) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&object_id.to_le_bytes());
    write_dotnet_string(&mut body, text);
    body.push(chat_type);
    body
}

/// #1426：C# HumanObject.Run——超重时 CanRun=false，Run 退化为 Walk（HumanObject.cs :2516）
fn effective_run(is_run: bool, overweight: bool) -> bool {
    is_run && !overweight
}

/// #1428/#1502：C# HumanObject.Run steps = RidingMount || (ActiveSwiftFeet && !Sneaking) ? 3 : 2；Walk = 1
/// SwiftFeet 用 MoveSpeedBoost buff 表示（Rust 仅 SwiftFeet 施放产生该 buff，无歧义）
fn move_steps(run: bool, is_mounted: bool, swift_feet: bool) -> i32 {
    if !run {
        1
    } else if is_mounted || swift_feet {
        3
    } else {
        2
    }
}

/// #1427：回发 S.UserLocation 让客户端重同步（C# Walk/Run 失败 Enqueue UserLocation；
/// wire 与 PlayerActor.send_user_location 一致：[direction u8][x i32][y i32]）
async fn send_user_location_sync(
    gate_ref: &ActorRef<GateActor>,
    session_id: u64,
    direction: u8,
    x: i32,
    y: i32,
) {
    let mut body = Vec::new();
    body.push(direction);
    body.extend_from_slice(&x.to_le_bytes());
    body.extend_from_slice(&y.to_le_bytes());
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::UserLocation as i16, &body),
    }).await;
}

#[cfg(test)]
mod tests {
    use super::{effective_run, format_roll_message, move_steps, object_chat_body, tile_blocked_by};

    #[test]
    fn test_roll_message_format() {
        assert_eq!(format_roll_message("张三", 4), "张三 掷出了 4 点");
        assert_eq!(format_roll_message("Legacy", 1), "Legacy 掷出了 1 点");
    }

    #[test]
    fn test_roll_dice_in_range() {
        // fastrand 1..=5 范围校验（C# Envir.Random.Next(5) + 1）
        for _ in 0..200 {
            let d = fastrand::i32(1..=5);
            assert!((1..=5).contains(&d), "dice out of range: {}", d);
        }
    }

    #[test]
    fn test_object_chat_body_wire() {
        // #1344：object_id=0x01020304 LE；dotnet "hi" = 长度4 + 2字节；chat_type=Normal(0)
        let body = object_chat_body(0x01020304, "hi", 0);
        assert_eq!(&body[0..4], &[0x04, 0x03, 0x02, 0x01]);
        assert_eq!(body[4], 2); // dotnet string length
        assert_eq!(&body[5..7], b"hi");
        assert_eq!(body[7], 0); // ChatType::Normal
    }

    #[test]
    fn test_object_chat_body_empty_text() {
        let body = object_chat_body(7, "", 5);
        assert_eq!(&body[0..4], &[7, 0, 0, 0]);
        assert_eq!(body[4], 0);
        assert_eq!(body[5], 5); // ChatType
    }

    #[test]
    fn test_effective_run_and_move_steps() {
        // #1426：超重 run 退化为 walk；walk 不受负重影响
        assert!(effective_run(true, false));
        assert!(!effective_run(true, true));
        assert!(!effective_run(false, true));
        // #1428/#1502：骑乘或 SwiftFeet（MoveSpeedBoost）run 3 格，普通 run 2 格，walk 1 格
        assert_eq!(move_steps(false, false, false), 1);
        assert_eq!(move_steps(false, true, false), 1);
        assert_eq!(move_steps(false, false, true), 1);
        assert_eq!(move_steps(true, false, false), 2);
        assert_eq!(move_steps(true, true, false), 3);
        assert_eq!(move_steps(true, false, true), 3);
        assert_eq!(move_steps(true, true, true), 3);
    }

    #[test]
    fn test_tile_blocked_by() {
        // #1408：NPC 或攻城结构占用目标格 → 阻挡
        let npcs = vec![(170i32, 667i32), (200, 300)];
        let walls = vec![(350i32, 350i32)];
        assert!(tile_blocked_by(170, 667, &npcs, &walls));
        assert!(tile_blocked_by(350, 350, &npcs, &walls));
        assert!(!tile_blocked_by(171, 667, &npcs, &walls));
        assert!(!tile_blocked_by(100, 100, &[], &[]));
    }
}
