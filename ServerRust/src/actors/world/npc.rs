use super::*;
use sqlx::Row;

/// NPC 对话请求（从 GateActor 转发）
pub struct NPCCallRequest {
    pub session_id: u64,
    pub npc_object_id: u32,
    pub key: String,
}

impl Message<NPCCallRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: NPCCallRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => {
                warn!("NPC call from unknown session {}", msg.session_id);
                return;
            }
        };

        // 获取玩家状态
        let player_state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            Ok(None) => return,
            Err(_) => return,
        };
        if player_state.is_dead {
            return;
        }
        let player_pos = (player_state.x, player_state.y);

        // 查找对应的 NPC
        let npc = match self.npcs.get(&msg.npc_object_id) {
            Some(n) => n.clone(),
            None => {
                warn!("NPC call for unknown object_id {}", msg.npc_object_id);
                return;
            }
        };

        // 距离校验（NPC 交互范围 2 格）
        let dist = (npc.x - player_pos.0).abs() + (npc.y - player_pos.1).abs();
        if dist > 2 {
            debug!("Player too far from NPC {} (dist={})", npc.name, dist);
            return;
        }

        // DB NPC visibility: time/level/class restrictions
        if npc.db_index > 0 {
            if let Some(npc_db) = self.npc_infos.get(&npc.db_index) {
                if npc_db.min_lev > 0 && player_state.level < npc_db.min_lev as u16 {
                    debug!("NPC {} requires min level {}", npc.name, npc_db.min_lev);
                    return;
                }
                if npc_db.max_lev > 0 && player_state.level > npc_db.max_lev as u16 {
                    debug!("NPC {} requires max level {}", npc.name, npc_db.max_lev);
                    return;
                }
                if let Some(ref required) = npc_db.class_required {
                    let class_name = format!("{:?}", player_state.class);
                    if !required.is_empty() && required != &class_name {
                        debug!("NPC {} requires class {} (player is {})", npc.name, required, class_name);
                        return;
                    }
                }
                if let Some(ref dow) = npc_db.day_of_week {
                    let today = chrono::Utc::now().format("%A").to_string();
                    let today_short = &today[..3];
                    if !dow.is_empty() && !dow.contains(&today) && !dow.contains(today_short) {
                        debug!("NPC {} not available on {}", npc.name, today);
                        return;
                    }
                }
                // Time-based visibility: hour_start/minute_start to hour_end/minute_end
                if npc_db.time_visible > 0 {
                    let now = chrono::Local::now();
                    let current_minutes = now.hour() as i32 * 60 + now.minute() as i32;
                    let start_minutes = npc_db.hour_start * 60 + npc_db.minute_start;
                    let end_minutes = npc_db.hour_end * 60 + npc_db.minute_end;
                    let in_window = if start_minutes <= end_minutes {
                        current_minutes >= start_minutes && current_minutes <= end_minutes
                    } else {
                        // Crosses midnight (e.g. 22:00 to 06:00)
                        current_minutes >= start_minutes || current_minutes <= end_minutes
                    };
                    if !in_window {
                        debug!("NPC {} not available at {}:{} (window {}:{}-{}:{})",
                            npc.name, now.hour(), now.minute(),
                            npc_db.hour_start, npc_db.minute_start, npc_db.hour_end, npc_db.minute_end);
                        return;
                    }
                }
                // Flag requirement check
                if npc_db.flag_needed > 0 {
                    let flag_key = format!("NPC_VISIBLE_{}", npc_db.flag_needed);
                    let has_flag = player_state.flags.get(&flag_key).copied().unwrap_or(0) > 0;
                    if !has_flag {
                        debug!("NPC {} requires flag {}", npc.name, npc_db.flag_needed);
                        return;
                    }
                }
            }
        }

        debug!("Player called NPC '{}' (#{}) with key='{}'", npc.name, msg.npc_object_id, msg.key);

        // 记录会话当前 NPC（BuyItem 等包不含 npc_id，按会话解析）
        self.session_npc.insert(msg.session_id, npc.object_id);

        // [@BuyBack] 是引擎级按键（C# NPCScript.BuyBackKey）：直接发回购商品列表，
        // 不走脚本页（脚本页只有提示文本，C# 引擎同样只发商品）
        if msg.key.eq_ignore_ascii_case("[@BuyBack]") {
            self.send_buyback_goods(msg.session_id, &npc);
            return;
        }

        // 优先使用 DB 脚本（支持 GOTO 跳转）
        let mut dialog_lines = Vec::new();
        let mut current_key = msg.key.clone();
        let mut goto_depth = 0;
        const MAX_GOTO_DEPTH: usize = 10;

        // 自定义变量暂存（C# 引擎跨 section 复用）
        let mut custom_vars: std::collections::HashMap<String, String> = std::collections::HashMap::new();

        while goto_depth < MAX_GOTO_DEPTH {
            goto_depth += 1;
            // DB 存储的 page_name 是全大写（如 [@MAIN]），查找时归一化
            let normalized_key = current_key.to_uppercase();
            let script_key = (npc.db_index, normalized_key.clone());
            if let Some(lines) = self.npc_scripts.get(&script_key).cloned() {
                // C# 格式（含 [@section]/#IF/#SAY 等指令）走新引擎
                let joined = lines.join("\n");
                if npc_script::is_csharp_format(&joined) {
                    let parsed = npc_script::ParsedScript::parse(&joined);
                    // 目标 section：优先匹配 current_key 对应段名，否则 @main
                    let want_name = current_key
                        .trim_start_matches('[')
                        .trim_start_matches('@')
                        .trim_end_matches(']')
                        .to_string();
                    let target_section = parsed
                        .find(&want_name)
                        .or_else(|| parsed.main_section());
                    if let Some(section) = target_section {
                        let res = parsed
                            .execute_section(section, self, msg.session_id, &npc, &mut custom_vars)
                            .await;
                        if let Some(target) = res.goto {
                            current_key = format!("[@{}]", target).to_uppercase();
                            // 重用已解析脚本里的目标段（单页内 GOTO）
                            if let Some(next_sec) = parsed.find(&target) {
                                let r2 = parsed
                                    .execute_section(next_sec, self, msg.session_id, &npc, &mut custom_vars)
                                    .await;
                                dialog_lines = r2.say_lines;
                                break;
                            }
                            // 目标段不在本页：回到外层按 page key 查找
                            continue;
                        }
                        dialog_lines = res.say_lines;
                        break;
                    } else {
                        // 没有匹配段，回退到默认 Main 问候
                        dialog_lines = vec![
                            format!("{}：你想说什么？", npc.name),
                        ];
                        break;
                    }
                }

                // 旧的 <CMD> 格式：沿用 eval_npc_script
                let mut lines = lines;
                for line in &mut lines {
                    *line = line.replace("$USERNAME", &player_state.name)
                                .replace("$NPCNAME", &npc.name)
                                .replace("$LEVEL", &player_state.level.to_string());
                }
                let (out, goto) = self.eval_npc_script(&mut lines, msg.session_id, &npc).await;
                if let Some(target) = goto {
                    current_key = format!("[@{}]", target);
                    continue;
                }
                dialog_lines = out;
                break;
            } else {
                dialog_lines = match current_key.as_str() {
                    "[@Main]" => {
                        let mut lines = vec![format!("欢迎来到{}", npc.name)];
                        if npc.db_index > 0 {
                            if let Some(npc_db) = self.npc_infos.get(&npc.db_index) {
                                let pending: Vec<&db::QuestInfo> = npc_db.collect_quest_indexes.iter()
                                    .filter_map(|qi| self.quest_infos.get(qi))
                                    .collect();
                                let finishable: Vec<&db::QuestInfo> = npc_db.finish_quest_indexes.iter()
                                    .filter_map(|qi| self.quest_infos.get(qi))
                                    .collect();
                                if !pending.is_empty() {
                                    lines.push("——可接受任务——".into());
                                    for q in &pending {
                                        lines.push(format!("[{}] {}", q.name, q.file_name));
                                    }
                                }
                                if !finishable.is_empty() {
                                    lines.push("——可完成任务——".into());
                                    for q in &finishable {
                                        lines.push(format!("[{}] {}", q.name, q.file_name));
                                    }
                                }
                            }
                        }
                        if lines.len() == 1 {
                            lines.push("有什么我可以帮你的吗？".into());
                        }
                        if npc.db_index > 0 && self.npc_goods.get(&npc.db_index).is_some_and(|g| !g.is_empty()) {
                            lines.push("<购买/@Buy>".into());
                        }
                        if self.buyback_items.get(&msg.session_id).is_some_and(|l| !l.is_empty()) {
                            lines.push("<回购/@BuyBack>".into());
                        }
                        lines.push("<出售/@Sell>".into());
                        lines.push("<修理/@Repair>".into());
                        lines.push("<仓库/@Storage>".into());
                        lines
                    }
                    "[@Buy]" => {
                        self.send_npc_goods(msg.session_id, &npc);
                        return;
                    }
                    "[@BuyBack]" => {
                        self.send_buyback_goods(msg.session_id, &npc);
                        return;
                    }
                    "[@Sell]" => {
                        dialog_lines = vec![
                            format!("{}: 请把要出售的物品放入窗口。", npc.name),
                        ];
                        self.send_npc_panel(msg.session_id, mir2_shared::enums::PanelType::Sell);
                        break;
                    }
                    "[@Repair]" => {
                        dialog_lines = vec![
                            format!("{}: 我会帮你修好装备的。", npc.name),
                        ];
                        self.send_npc_panel(msg.session_id, mir2_shared::enums::PanelType::Repair);
                        break;
                    }
                    "[@Storage]" => {
                        // #200：仓库密码保护——有密码先解锁（C# StorageKey：SendStorage + NPCStorage，客户端弹解锁框）
                        let has_pwd = match self.players.get(&msg.session_id) {
                            Some(r) => db::account_has_storage_password(&self.db_pool, &r.account_username)
                                .await
                                .unwrap_or(false),
                            None => false,
                        };
                        if has_pwd {
                            // 通知客户端弹解锁框；仓库内容等解锁成功后再下发
                            let mut body = Vec::new();
                            if mir2_shared::packets::base::serialize_packet(
                                &mut std::io::Cursor::new(&mut body),
                                &mir2_shared::packets::server::npc::NPCStorage,
                            )
                            .is_err()
                            {
                                warn!("Failed to serialize NPCStorage");
                            } else {
                                let _ = self.gate_ref.tell(SendToClient {
                                    session_id: msg.session_id,
                                    data: body,
                                }).await;
                            }
                            dialog_lines = vec![format!("{}: 请输入仓库密码。", npc.name)];
                        } else {
                            dialog_lines = vec![format!("{}: 请妥善保管你的物品。", npc.name)];
                            self.send_user_storage(msg.session_id, &player_state.inventory.storage);
                        }
                        break;
                    }
                    _ => vec![
                        format!("{} 说：", npc.name),
                        format!("你说了：{}", msg.key),
                    ],
                };
                break;
            }
        }

        debug!("Send NPCResponse {} lines", dialog_lines.len());
        let mut body = Vec::new();
        body.extend_from_slice(&(dialog_lines.len() as i32).to_le_bytes());
        for line in &dialog_lines {
            write_dotnet_string(&mut body, line);
        }
        let packet = build_packet_bytes(mir2_shared::enums::ServerPacketIds::NPCResponse as i16, &body);

        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: packet,
        }).await;
    }
}

/// 传送到 NPC 请求
pub struct TeleportToNPCRequest {
    pub session_id: u64,
    pub npc_id: u32,
}

impl Message<TeleportToNPCRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: TeleportToNPCRequest, _ctx: &mut Context<Self, Self::Reply>) {
        // 按 object_id 查找 NPC
        let npc = self.npcs.get(&msg.npc_id).cloned();
        let Some(npc) = npc else {
            send_system_message(&self.gate_ref, msg.session_id, "找不到该 NPC");
            return;
        };

        // 传送到 NPC 附近
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if state.is_dead { return; }

        let new_x = npc.x;
        let new_y = npc.y;

        // 更新玩家位置
        let _ = record.actor_ref.ask(SetPlayerPosition { x: new_x, y: new_y, direction: npc.direction, map_index: None, is_mounted: None }).await;
        let mut body = Vec::new();
        body.extend_from_slice(&new_x.to_le_bytes());
        body.extend_from_slice(&new_y.to_le_bytes());
        body.push(npc.direction);
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::UserLocation as i16, &body),
        }).await;

        info!("TeleportToNPC: {} -> {} ({}, {})", state.name, npc.name, new_x, new_y);
    }
}

/// 请求地图信息（传送）
pub struct RequestMapInfoRequest {
    pub session_id: u64,
    pub map_id: i32,
}

impl Message<RequestMapInfoRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: RequestMapInfoRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let _state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 世界地图配置每连接下发一次（C# CheckMapInfo：WorldMapSetupSent）
        if let Some(rec) = self.players.get_mut(&msg.session_id) {
            if !rec.world_map_setup_sent {
                rec.world_map_setup_sent = true;
                let wm = super::build_world_map_setup_packet(&self.map_infos, super::TELEPORT_TO_NPC_COST);
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: msg.session_id,
                    data: wm,
                }).await;
                info!("WorldMapSetup: sent to session {} (on RequestMapInfo)", msg.session_id);
            }
        }

        // C# CheckMapInfo 语义：按 map_index 回 NewMapInfo（大地图 NPC 列表），不传送
        let Some(dest_mi) = self.map_infos.get(&msg.map_id) else {
            debug!("RequestMapInfo: unknown map {}", msg.map_id);
            return;
        };
        let npcs: Vec<db::NPCInfo> = self.npc_infos.values()
            .filter(|n| n.map_index == msg.map_id && n.show_on_big_map)
            .cloned()
            .collect();
        let new_map_info = super::build_new_map_info_packet_from_db(dest_mi.index, &dest_mi.title, &npcs);
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: new_map_info,
        }).await;
        info!("RequestMapInfo: session={} map={} ({}) npcs={}", msg.session_id, msg.map_id, dest_mi.title, npcs.len());
    }
}

/// 搜索地图/NPC
pub struct SearchMapRequest {
    pub session_id: u64,
    pub keyword: String,
}

impl Message<SearchMapRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: SearchMapRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let keyword_lower = msg.keyword.to_lowercase();

        // 搜索匹配的地图
        let matched_maps: Vec<_> = self.map_infos.values()
            .filter(|m| m.title.to_lowercase().contains(&keyword_lower) || m.file_name.to_lowercase().contains(&keyword_lower))
            .collect();

        // 搜索匹配的 NPC
        let matched_npcs: Vec<_> = self.npcs.values()
            .filter(|n| n.name.to_lowercase().contains(&keyword_lower))
            .collect();

        if matched_maps.is_empty() && matched_npcs.is_empty() {
            send_system_message(&self.gate_ref, msg.session_id, "未找到匹配结果");
            return;
        }

        let mut result = String::new();
        if !matched_maps.is_empty() {
            result.push_str(&format!("地图({}): ", matched_maps.len()));
            for (i, m) in matched_maps.iter().take(5).enumerate() {
                if i > 0 { result.push_str(", "); }
                result.push_str(&format!("{}(#{}))", m.title, m.index));
            }
        }
        if !matched_npcs.is_empty() {
            if !result.is_empty() { result.push_str(" | "); }
            result.push_str(&format!("NPC({}): ", matched_npcs.len()));
            for (i, n) in matched_npcs.iter().take(5).enumerate() {
                if i > 0 { result.push_str(", "); }
                result.push_str(&format!("{}({},{})", n.name, n.x, n.y));
            }
        }
        send_system_message(&self.gate_ref, msg.session_id, &result);
        debug!("SearchMap: {} maps, {} NPCs matching '{}'", matched_maps.len(), matched_npcs.len(), msg.keyword);
    }
}

/// 创建角色请求
pub struct NewCharacterRequest {
    pub session_id: u64,
    pub name: String,
    pub class: u8,
    pub gender: u8,
    pub hair: u16,
    pub account_username: String,
}

impl Message<NewCharacterRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: NewCharacterRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("NewCharacterRequest handler entered: {}", msg.name);
        // 验证角色名称
        if msg.name.is_empty() || msg.name.len() > 20 {
            send_system_message(&self.gate_ref, msg.session_id, "角色名称无效");
            return;
        }
        // 检查名称是否已被使用（在线玩家）
        for r in self.players.values() {
            if r.name.eq_ignore_ascii_case(&msg.name) {
                send_system_message(&self.gate_ref, msg.session_id, "角色名称已被使用");
                return;
            }
        }
        // 检查数据库中是否已有该角色
        match db::load_character(&self.db_pool, &msg.name).await {
            Ok(Some(_)) => {
                send_system_message(&self.gate_ref, msg.session_id, "角色名称已被使用");
                return;
            }
            Err(e) => {
                warn!("Failed to check character name '{}': {}", msg.name, e);
            }
            Ok(None) => {}
        }

        // 创建默认角色状态并保存到数据库
        let class = mir2_shared::enums::MirClass::try_from(msg.class)
            .unwrap_or(mir2_shared::enums::MirClass::Warrior);
        let gender = mir2_shared::enums::MirGender::try_from(msg.gender)
            .unwrap_or(mir2_shared::enums::MirGender::Male);
        let default_state = PlayerState {
            object_id: 0,
            name: msg.name.clone(),
            map_index: 0,
            x: 330,
            y: 330,
            direction: 4,
            attack_mode: mir2_shared::enums::AttackMode::Peace,
            pet_mode: mir2_shared::enums::PetMode::Both,
            hidden: false,
            session_id: 0,
            class,
            gender,
            hair: msg.hair as u8,
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
        };
        debug!("NewCharacter: saving '{}' ...", msg.name);
        match db::save_character(&self.db_pool, &default_state, &msg.account_username).await {
            Ok(_) => info!("NewCharacter: saved '{}' ok", msg.name),
            Err(e) => warn!("Failed to save new character '{}': {}", msg.name, e),
        }

        // 发送 NewCharacterSuccess（SelectInfo：name + index + level + class + gender + last_access）
        let mut body = Vec::new();
        mir2_shared::binary::write_dotnet_string(&mut body, &msg.name).ok();
        body.extend_from_slice(&0i32.to_le_bytes()); // index = 0（新角色列表首位）
        body.extend_from_slice(&1u16.to_le_bytes()); // level = 1
        body.push(msg.class);
        body.push(msg.gender);
        body.extend_from_slice(&0i64.to_le_bytes()); // last_access ticks
        let data = build_packet_bytes(mir2_shared::enums::ServerPacketIds::NewCharacterSuccess as i16, &body);
        debug!("NewCharacterSuccess: session={} name={} bytes={}", msg.session_id, msg.name, data.len());
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data,
        }).await;

        debug!("NewCharacter: session={} name={} class={} gender={}", msg.session_id, msg.name, msg.class, msg.gender);
    }
}

/// 删除角色请求
pub struct DeleteCharacterRequest {
    pub session_id: u64,
    pub character_index: i32,
    pub account_username: String,
}

impl Message<DeleteCharacterRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: DeleteCharacterRequest, _ctx: &mut Context<Self, Self::Reply>) {
        // 从数据库删除角色及相关数据
        // 注意：当前仅返回成功，后续可加入"需要密码确认"等逻辑
        let success = true;

        let mut body = Vec::new();
        body.extend_from_slice(&msg.character_index.to_le_bytes());
        body.push(if success { 1u8 } else { 0u8 });
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::DeleteCharacter as i16, &body),
        }).await;

        debug!("DeleteCharacter: session={} index={}", msg.session_id, msg.character_index);
    }
}

/// 创建英雄请求（C# C.NewHero：name/gender/class）
pub struct NewHeroRequest {
    pub session_id: u64,
    pub name: String,
    pub gender: mir2_shared::enums::MirGender,
    pub class: mir2_shared::enums::MirClass,
}

impl Message<NewHeroRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: NewHeroRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // #188：真正创建英雄（内存态；DB 持久化后续批次）
        // C# S.NewHero.Result：1=BadName 4=MaxHeroes 10=Success
        let has_hero = self.player_heroes.get(&msg.session_id).is_some_and(|v| !v.is_empty());
        let result = hero_create_result(&msg.name, has_hero);
        if result == 10 {
            self.player_heroes.entry(msg.session_id).or_default().push(HeroInfo {
                index: 1,
                name: msg.name.clone(),
                level: 1,
                class: msg.class,
                gender: msg.gender,
            });
            let _ = record.actor_ref.ask(SetHeroIndex { hero_index: 1 }).await;
            // #198：创建成功后生成英雄对象
            self.broadcast_hero_spawn(msg.session_id).await;
            // #203：下发完整英雄信息（背包/装备/自动药）
            self.send_hero_information_packet(msg.session_id).await;
        }
        let body = vec![result];
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::NewHero as i16, &body),
        }).await;
        // 重新下发英雄列表
        let state_after = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => state.clone(),
        };
        let heroes = self.player_heroes.get(&msg.session_id).cloned().unwrap_or_default();
        send_manage_heroes_packet(&self.gate_ref, msg.session_id, &state_after, &heroes);
        debug!("NewHero: {} name={} gender={:?} class={:?} result={}", state.name, msg.name, msg.gender, msg.class, result);
    }
}
// ============================================================
// 钓鱼系统
// ============================================================

pub struct FishingCastRequest {
    pub session_id: u64,
    pub fishing_type: u8,
}

impl Message<FishingCastRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: FishingCastRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };
        if state.is_dead { return; }

        // Check for fishing rod in weapon slot
        let has_rod = state.inventory.get_equipment(crate::actors::inventory::EquipmentSlot::Weapon)
            .and_then(|item| self.item_infos.get(&item.item_index))
            .map(|info| {
                let n = info.name.to_lowercase();
                n.contains("rod") || n.contains("fishing") || n.contains("竿") || n.contains("鱼")
            })
            .unwrap_or(false);
        if !has_rod {
            send_system_message(&self.gate_ref, msg.session_id, "你需要装备鱼竿才能钓鱼");
            return;
        }

        let _ = record.actor_ref.ask(SetFishing { is_fishing: true, autocast: false }).await;

        // Send FishingUpdate: progress=1 (waiting), success=false
        use mir2_shared::packets::server::miscellaneous::FishingUpdate;
        let packet = FishingUpdate { fishing_progress: 1, fishing_success: false };
        let mut body = Vec::new();
        if let Ok(()) = mir2_shared::packets::Packet::write_body(&packet, &mut body) {
            let _ = self.gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::FishingUpdate as i16, &body),
            }).await;
        }

        debug!("FishingCast: {} type={}", state.name, msg.fishing_type);
    }
}

pub struct FishingChangeAutocastRequest {
    pub session_id: u64,
    pub enabled: bool,
}

impl Message<FishingChangeAutocastRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: FishingChangeAutocastRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        let _ = record.actor_ref.ask(SetFishing { is_fishing: state.is_fishing, autocast: msg.enabled }).await;

        // Send FishingUpdate: progress=5 (autocast toggle), success=enabled
        use mir2_shared::packets::server::miscellaneous::FishingUpdate;
        let packet = FishingUpdate { fishing_progress: 5, fishing_success: msg.enabled };
        let mut body = Vec::new();
        if let Ok(()) = mir2_shared::packets::Packet::write_body(&packet, &mut body) {
            let _ = self.gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::FishingUpdate as i16, &body),
            }).await;
        }

        debug!("FishingChangeAutocast: {} enabled={}", state.name, msg.enabled);
    }
}

// ============================================================
// 开门
// ============================================================

pub struct OpendoorRequest {
    pub session_id: u64,
    pub door_index: u8,
}

impl Message<OpendoorRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: OpendoorRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        debug!("Opendoor: {} door_index={}", state.name, msg.door_index);

        // Track open door state per map
        let map_key = state.map_index;
        self.open_doors.insert((map_key, msg.door_index));

        // Send Opendoor response to the player
        send_opendoor(&self.gate_ref, msg.session_id, msg.door_index, false).await;

        // Broadcast to all other players on the same map
        broadcast_opendoor_async(&self.gate_ref, &self.players, map_key, msg.door_index, false, msg.session_id).await;
    }
}

// ============================================================
// NPC确认输入
// ============================================================

pub struct NPCConfirmInputRequest {
    pub session_id: u64,
    pub npc_id: u32,
    pub input_text: String,
}

impl Message<NPCConfirmInputRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: NPCConfirmInputRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        debug!("NPCConfirmInput: {} npc_id={} input={}", state.name, msg.npc_id, msg.input_text);

        // Try to match input as a quest file_name for quick acceptance
        let npc = match self.npcs.get(&msg.npc_id) {
            Some(n) => n,
            None => return,
        };
        if npc.db_index > 0 {
            if let Some(npc_db) = self.npc_infos.get(&npc.db_index) {
                // Check if input matches a collectable quest
                let quest_db = npc_db.collect_quest_indexes.iter()
                    .filter_map(|qi| self.quest_infos.get(qi))
                    .find(|q| q.file_name == msg.input_text || q.name == msg.input_text);
                if let Some(quest_db) = quest_db {
                    if state.level >= quest_db.required_min_level as u16
                        && (quest_db.required_max_level == 0 || state.level <= quest_db.required_max_level as u16)
                    {
                        // Check not already accepted
                        if let Ok(None) = record.actor_ref.ask(GetQuest { quest_index: quest_db.index }).await {
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_secs())
                                .unwrap_or(0);
                            let quest = make_quest_instance(quest_db, now);
                            if let Ok(true) = record.actor_ref.ask(AcceptQuest { quest }).await {
                                send_system_message(&self.gate_ref, msg.session_id,
                                    &format!("任务已接受: {}", quest_db.name));
                            }
                            return;
                        }
                    }
                }
                // Check if input matches a finishable quest
                let quest_db = npc_db.finish_quest_indexes.iter()
                    .filter_map(|qi| self.quest_infos.get(qi))
                    .find(|q| q.file_name == msg.input_text || q.name == msg.input_text);
                if let Some(quest_db) = quest_db {
                    // Complete the quest
                    if let Ok(Some(quest)) = record.actor_ref.ask(GetQuest { quest_index: quest_db.index }).await {
                        if quest.status == QuestStatus::InProgress {
                            let _ = record.actor_ref.ask(CompleteQuest { quest_index: quest_db.index }).await;
                            // Grant rewards
                            let _ = record.actor_ref.ask(AddExperience { amount: self.apply_global_exp_multiplier(quest_db.exp_reward) }).await;
                            let _ = record.actor_ref.ask(AddGold { amount: quest_db.gold_reward.max(0) as u64 }).await;
                            send_system_message(&self.gate_ref, msg.session_id,
                                &format!("任务完成: +{}经验, +{}金币", quest_db.exp_reward, quest_db.gold_reward.max(0)));
                            return;
                        }
                    }
                }
            }
        }

        send_system_message(&self.gate_ref, msg.session_id, "无法识别该指令");
    }
}

// ============================================================
// 游戏商店/举报/排名
// ============================================================

/// 游戏商店商品定义
struct ShopItem {
    item_index: i32,
    gold_price: u32,
    credit_price: u32,
    count: i32,
    class: u8,
    category: &'static str,
    stock: i32,
}

/// 游戏商店硬编码目录（fallback，当 DB 无数据时使用）
fn game_shop_catalog_fallback() -> &'static [ShopItem] {
    &[
        // 经验丹 - 增加1000经验
        ShopItem { item_index: 1, gold_price: 5000, credit_price: 100, count: 1, class: 255, category: "消耗品", stock: 999 },
        // 回城卷
        ShopItem { item_index: 2, gold_price: 1000, credit_price: 20, count: 1, class: 255, category: "消耗品", stock: 999 },
        // 随机传送卷
        ShopItem { item_index: 3, gold_price: 2000, credit_price: 40, count: 1, class: 255, category: "消耗品", stock: 999 },
        // 双倍经验卷
        ShopItem { item_index: 4, gold_price: 10000, credit_price: 200, count: 1, class: 255, category: "消耗品", stock: 999 },
        // 经验丹x10
        ShopItem { item_index: 5, gold_price: 40000, credit_price: 800, count: 10, class: 255, category: "消耗品", stock: 999 },
    ]
}

/// 发送游戏商店目录给玩家
fn send_game_shop_catalog(gate_ref: &ActorRef<GateActor>, session_id: u64, gold: u32, shop_items: &[db::GameShopItem]) {
    use mir2_shared::packets::server::special_systems::{GameShopInfo, GameShopItem as ProtoItem};

    let items: Vec<ProtoItem> = if shop_items.is_empty() {
        // Fallback to hardcoded
        game_shop_catalog_fallback().iter().map(|s| ProtoItem {
            item_index: s.item_index,
            gold_price: s.gold_price,
            credit_price: s.credit_price,
            count: s.count,
            class: s.class,
            category: s.category.to_string(),
            stock: s.stock,
            is_bought: false,
            deal: false,
        }).collect()
    } else {
        shop_items.iter().map(|s| ProtoItem {
            item_index: s.item_index,
            gold_price: s.gold_price,
            credit_price: s.credit_price,
            count: s.count as i32,
            class: 255, // DB class_name is string; use default
            category: s.category.clone(),
            stock: s.stock,
            is_bought: false,
            deal: s.deal,
        }).collect()
    };

    let packet = GameShopInfo {
        items,
        credit: 0,
        gold,
    };

    let mut body = Vec::new();
    let _ = packet.write_body(&mut body);
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GameShopInfo as i16, &body),
    }).try_send();
}

pub struct GameshopBuyRequest {
    pub session_id: u64,
    pub item_id: u32,
    pub count: u32,
}

impl Message<GameshopBuyRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: GameshopBuyRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        // item_id=0 请求商店目录
        if msg.item_id == 0 {
            debug!("GameShop: {} requesting catalog", state.name);
            send_game_shop_catalog(&self.gate_ref, msg.session_id, state.inventory.gold as u32, &self.game_shop_items);
            return;
        }

        // 查找商品（优先 DB，fallback 硬编码）
        let db_item = self.game_shop_items.iter().find(|i| i.item_index as u32 == msg.item_id);
        let fallback = game_shop_catalog_fallback().iter().find(|i| i.item_index as u32 == msg.item_id);
        let (item_price, item_count) = if let Some(di) = db_item {
            (di.gold_price as u64, di.count as u32)
        } else if let Some(fi) = fallback {
            (fi.gold_price as u64, fi.count as u32)
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "商品不存在");
            return;
        };

        let buy_count = msg.count.max(1).min(item_count);
        let total_gold = item_price.saturating_mul(buy_count as u64);

        debug!("GameshopBuy: {} item={} count={} gold={}", state.name, msg.item_id, buy_count, total_gold);

        // 检查金币
        if state.inventory.gold < total_gold as u64 {
            send_system_message(&self.gate_ref, msg.session_id, "金币不足");
            return;
        }

        // 先构建邮件（在扣金币前，避免扣款后交付失败导致玩家损失）
        let shop_item = self.game_shop_items.iter().find(|i| i.item_index as u32 == msg.item_id);
        let item_index = if let Some(si) = shop_item {
            si.item_index
        } else {
            msg.item_id as i32
        };

        let mail_items: Vec<mir2_shared::data::item::UserItem> = if let Some(item_db) = self.item_infos.get(&item_index) {
            (0..buy_count).map(|_| {
                let uid = generate_item_uid();
                mir2_shared::data::item::UserItem {
                    unique_id: uid,
                    item_index: item_db.index,
                    count: 1,
                    current_dura: item_db.durability as u16,
                    max_dura: item_db.durability as u16,
                    // 验证已鉴定:start_item 永远已鉴定,否则查 bool_flags bit 0。
                    // (item_db.is_identified() 抽象此逻辑,见 db::ItemInfo)
                    identified: item_db.is_identified(),
                    ..Default::default()
                }
            }).collect()
        } else {
            (0..buy_count).map(|_| {
                mir2_shared::data::item::UserItem {
                    unique_id: generate_item_uid(),
                    item_index,
                    ..Default::default()
                }
            }).collect()
        };

        let mail = MailMessage {
            mail_id: generate_mail_id(),
            sender_name: "GameShop".to_string(),
            receiver_name: state.name.clone(),
            subject: "商城购买".to_string(),
            body: format!("您购买了 {} 件商品", buy_count),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
            read: false,
            collected: false,
            locked: false,
            gold: 0,
            items: mail_items,
        };

        // 扣款
        let _ = record.actor_ref.ask(DeductGold { amount: total_gold as u64 }).await;

        // 发送邮件
        send_mail_received_packet(&self.gate_ref, msg.session_id, &mail);
        let _ = record.actor_ref.ask(crate::actors::player::AddMail { mail }).await;

        send_system_message(&self.gate_ref, msg.session_id,
            &format!("购买成功！已扣除金币 {}，物品已通过邮件发送", total_gold));

        // 发送库存更新
        let stock_remaining = item_count.saturating_sub(buy_count);
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GameShopStock as i16, &{
                let mut body = Vec::new();
                body.extend_from_slice(&(msg.item_id as i32).to_le_bytes());
                body.extend_from_slice(&stock_remaining.to_le_bytes());
                body
            }),
        }).await;
    }
}

pub struct ReportIssueRequest {
    pub session_id: u64,
    pub issue_type: u8,
    pub description: String,
}

impl Message<ReportIssueRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ReportIssueRequest, _ctx: &mut Context<Self, Self::Reply>) {
        if let Some(record) = self.players.get(&msg.session_id) {
            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                let _ = crate::actors::world::report::save_report(
                    &self.db_pool, &state.name, msg.issue_type, &msg.description,
                ).await;
            }
        }
        send_system_message(&self.gate_ref, msg.session_id, "举报信息已提交，感谢您的反馈");
    }
}

pub struct GetRankingRequest {
    pub session_id: u64,
    pub rank_type: u8,
}

impl Message<GetRankingRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: GetRankingRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("GetRanking: session={} type={}", msg.session_id, msg.rank_type);

        // Collect online players
        let mut entries: Vec<(String, u8, i32, i64)> = Vec::new();
        for (_, record) in &self.players {
            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                entries.push((
                    state.name.clone(),
                    state.class as u8,
                    state.level as i32,
                    state.experience,
                ));
            }
        }
        // Supplement with DB-backed top players for more complete rankings
        if let Ok(db_rows) = sqlx::query(
            "SELECT name, class, level, experience FROM characters ORDER BY level DESC, experience DESC LIMIT 50"
        )
        .fetch_all(&self.db_pool)
        .await
        {
            for row in db_rows {
                let name: String = row.get("name");
                let class_val: i32 = row.get("class");
                let class = mir2_shared::enums::MirClass::try_from(class_val as u8)
                    .unwrap_or(mir2_shared::enums::MirClass::Warrior) as u8;
                let level: i32 = row.get("level");
                let experience: i64 = row.get("experience");
                if !entries.iter().any(|(n, _, _, _)| n == &name) {
                    entries.push((name, class, level, experience));
                }
            }
        }

        // 按等级降序、经验降序排序
        entries.sort_by(|a, b| {
            b.2.cmp(&a.2).then_with(|| b.3.cmp(&a.3))
        });

        // 取前 20 名
        let rankings: Vec<mir2_shared::packets::server::special_systems::RankInfo> = entries
            .into_iter()
            .take(20)
            .enumerate()
            .map(|(idx, (name, class, level, experience))| {
                mir2_shared::packets::server::special_systems::RankInfo {
                    rank: (idx + 1) as i32,
                    player_name: name,
                    class,
                    level,
                    experience,
                }
            })
            .collect();

        let packet = mir2_shared::packets::server::special_systems::Rankings { rankings };
        let mut body = Vec::new();
        if packet.write_body(&mut body).is_ok() {
            let _ = self.gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Rankings as i16, &body),
            }).await;
        }
    }
}

// =============================================================================
// PR #1126: KR NPC/Quest Linking — info request handlers
// =============================================================================

/// PR #1126: Client requests detailed monster info (tooltip on hover).
pub struct RequestMonsterInfoRequest {
    pub session_id: u64,
    pub monster_index: i32,
}

impl Message<RequestMonsterInfoRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: RequestMonsterInfoRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let mi = match self.monster_infos.get(&msg.monster_index) {
            Some(m) => m,
            None => {
                warn!("RequestMonsterInfo: monster_index={} not found", msg.monster_index);
                return;
            }
        };

        // Build a ClientMonsterInfo matching master wire order.
        // Fields not present in our DB schema (game_name, can_recall) are filled
        // with safe defaults — master has them in extra columns we don't have.
        let info = mir2_shared::data::client_data::ClientMonsterInfo {
            index: mi.index,
            name: mi.name.clone(),
            game_name: String::new(), // not stored in our DB
            image: {
                // mi.image is i32 (Monster enum index); ClientMonsterInfo expects
                // mir2_shared::enums::Monster. try_from will fail for unknown
                // values; default to Monster::Guard in that case.
                let raw = mi.image as u16;
                mir2_shared::enums::Monster::try_from(raw)
                    .unwrap_or(mir2_shared::enums::Monster::Guard)
            },
            ai: mi.ai as u8,
            effect: mi.effect as u8,
            level: mi.level as u16,
            view_range: mi.view_range as u8,
            cool_eye: mi.cool_eye as u8,
            light: mi.light as u8,
            attack_speed: mi.attack_speed as u16,
            move_speed: mi.move_speed as u16,
            experience: mi.experience as u32,
            can_push: mi.can_push,
            can_tame: mi.can_tame,
            auto_rev: mi.auto_rev,
            undead: mi.undead,
            can_recall: false, // not stored
            stats: {
                let mut s = mir2_shared::data::stats::Stats::new();
                for (k, v) in mi.stats.iter() {
                    let stat = mir2_shared::enums::Stat::try_from(*k)
                        .unwrap_or(mir2_shared::enums::Stat::MinAC);
                    s.set(stat, *v);
                }
                s
            },
        };

        let packet = mir2_shared::packets::server::NewMonsterInfo { info };
        let mut body = Vec::new();
        if packet.write_body(&mut body).is_ok() {
            let _ = self.gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::NewMonsterInfo as i16,
                    &body,
                ),
            }).await;
        }
    }
}

/// PR #1126: Client requests detailed NPC info (tooltip on hover).
pub struct RequestNPCInfoRequest {
    pub session_id: u64,
    pub npc_index: i32,
}

impl Message<RequestNPCInfoRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: RequestNPCInfoRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let ni = match self.npc_infos.get(&msg.npc_index) {
            Some(n) => n,
            None => {
                warn!("RequestNPCInfo: npc_index={} not found", msg.npc_index);
                return;
            }
        };

        // ClientNPCInfo (5-field version preserved for wire compat with Rust client).
        // object_id = 0 (not a live object, just a template lookup).
        let info = mir2_shared::data::client_data::ClientNPCInfo {
            object_id: 0,
            name: ni.name.clone(),
            location: mir2_shared::map::Point { x: ni.x, y: ni.y },
            icon: ni.image,
            can_teleport_to: false, // not stored; UI defaults to false
        };

        let packet = mir2_shared::packets::server::NewNPCInfo { info };
        let mut body = Vec::new();
        if packet.write_body(&mut body).is_ok() {
            let _ = self.gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::NewNPCInfo as i16,
                    &body,
                ),
            }).await;
        }
    }
}

/// PR #1126: Client requests detailed item info (tooltip on hover).
/// Note: ItemInfo schema in ServerRust does not yet carry the fields needed
/// for a full ClientItemInfo payload. This handler is wired but currently
/// only logs the request. Full implementation will be a follow-up commit.
pub struct RequestItemInfoRequest {
    pub session_id: u64,
    pub item_index: i32,
}

impl Message<RequestItemInfoRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: RequestItemInfoRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        debug!("RequestItemInfo: session={} idx={} (no-op until ItemInfo schema is extended)",
            msg.session_id, msg.item_index);
    }
}

/// #200：仓库解锁成功后下发仓库内容（GateActor 校验通过后通知）
pub struct StorageUnlockedRequest {
    pub session_id: u64,
}

impl Message<StorageUnlockedRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: StorageUnlockedRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        self.send_user_storage(msg.session_id, &state.inventory.storage);
        info!("Storage unlocked for session {}", msg.session_id);
    }
}
