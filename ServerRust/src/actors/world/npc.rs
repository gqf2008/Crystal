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

        // C# MirConnection.cs:1508-1511：ObjectID = uint.MaxValue → 默认 NPC [@_Client]
        if msg.npc_object_id == u32::MAX {
            self.queue_default_npc(msg.session_id, "_client");
            return;
        }

        // C# MirConnection.cs:1502-1504：ObjectID == 默认 NPC 对象 ID → 默认 NPC 脚本（key 为页面名）
        if msg.npc_object_id == self.default_npc_object_id {
            self.queue_default_npc(msg.session_id, &msg.key);
            // C# CallDefaultNPC（PlayerObject.cs:7887）：下发 S.NPCUpdate（客户端刷新当前 NPC）
            let packet = mir2_shared::packets::server::npc_interaction::NPCUpdate {
                npc_id: self.default_npc_object_id,
            };
            let mut body = Vec::new();
            if packet.write_body(&mut body).is_ok() {
                let _ = self
                    .gate_ref
                    .tell(SendToClient {
                        session_id: msg.session_id,
                        data: build_packet_bytes(
                            mir2_shared::enums::ServerPacketIds::NPCUpdate as i16,
                            &body,
                        ),
                    })
                    .await;
            }
            return;
        }

        // 查找对应的 NPC
        let npc = match self.npcs.get(&msg.npc_object_id) {
            Some(n) => n.clone(),
            None => {
                warn!("NPC call for unknown object_id {}", msg.npc_object_id);
                return;
            }
        };

        // 距离校验（NPC 交互范围 2 格）；#1640：必须同图（C# CurrentMap.NPCs 语义）
        if npc.map_index != player_state.map_index {
            debug!(
                "NPC {} on map {} but player on map {} (cross-map call rejected)",
                npc.name, npc.map_index, player_state.map_index
            );
            return;
        }
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
                        debug!(
                            "NPC {} requires class {} (player is {})",
                            npc.name, required, class_name
                        );
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
                        debug!(
                            "NPC {} not available at {}:{} (window {}:{}-{}:{})",
                            npc.name,
                            now.hour(),
                            now.minute(),
                            npc_db.hour_start,
                            npc_db.minute_start,
                            npc_db.hour_end,
                            npc_db.minute_end
                        );
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

        debug!(
            "Player called NPC '{}' (#{}) with key='{}'",
            npc.name, msg.npc_object_id, msg.key
        );

        // 记录会话当前 NPC（BuyItem 等包不含 npc_id，按会话解析）
        self.session_npc.insert(msg.session_id, npc.object_id);
        // 新对话页 → 退出珍珠购买模式（C# NPCPage.Key 变化语义）
        self.session_pearl_shop.remove(&msg.session_id);

        // [@BuyBack] 是引擎级按键（C# NPCScript.BuyBackKey）：直接发回购商品列表，
        // 不走脚本页（脚本页只有提示文本，C# 引擎同样只发商品）
        if msg.key.eq_ignore_ascii_case("[@BuyBack]") {
            self.send_buyback_goods(msg.session_id, &npc);
            return;
        }

        // #1356：觉醒/分解/降级/重置 是引擎级按键（C# NPCScript AwakeningKey/DisassembleKey/
        // DowngradeKey/ResetKey → S.NPCAwakening/S.NPCDisassemble/S.NPCDowngrade/S.NPCReset）
        let panel_key = msg.key.to_uppercase();
        let panel_service = match panel_key.as_str() {
            "[@AWAKENING]" => Some(0u8),
            "[@DISASSEMBLE]" => Some(1u8),
            "[@DOWNGRADE]" => Some(2u8),
            "[@RESET]" => Some(3u8),
            _ => None,
        };
        if let Some(service) = panel_service {
            self.send_awakening_panel(msg.session_id, service).await;
            return;
        }
        // #珍珠商店：[@PEARLBUY] 引擎级按键（C# NPCScript.PearlBuyKey → S.NPCPearlGoods）
        if msg.key.eq_ignore_ascii_case("[@PEARLBUY]") {
            self.send_pearl_goods(msg.session_id, &npc).await;
            return;
        }

        // #2368：引擎级特殊页（C# NPCScript.ProcessSpecial）——必须在 DB 脚本查找之前拦截。
        // 这些 key 在 DB 里大多有脚本页（[@BUYSELL] 80+ NPC / [@BUY] 39 / [@SELL] 42 / [@REPAIR] 56 / [@STORAGE] 15），
        // 若走脚本只会显示 #SAY 文字，面板（NPCGoods/NPCSell/...）永远不下发 → 商店/出售/修理/仓库打不开。
        let engine_key = msg.key.to_uppercase();
        match engine_npc_action(&engine_key) {
            // C# BuyKey/BuyNewKey：SendNPCGoods(PanelType.Buy)
            Some(EngineNpcAction::Goods) => {
                self.send_npc_goods(msg.session_id, &npc);
                return;
            }
            // C# BuySellKey/BuySellNewKey：SendNPCGoods(Buy) + NPCSell
            Some(EngineNpcAction::GoodsAndSell) => {
                self.send_npc_goods(msg.session_id, &npc);
                self.send_npc_panel(msg.session_id, mir2_shared::enums::PanelType::Sell);
                return;
            }
            // C# SellKey：NPCSell
            Some(EngineNpcAction::Sell) => {
                self.send_npc_panel(msg.session_id, mir2_shared::enums::PanelType::Sell);
                return;
            }
            // C# RepairKey：NPCRepair；SRepairKey：NPCSRepair
            Some(EngineNpcAction::Repair) => {
                self.send_npc_panel(msg.session_id, mir2_shared::enums::PanelType::Repair);
                return;
            }
            Some(EngineNpcAction::SpecialRepair) => {
                self.send_npc_panel(msg.session_id, mir2_shared::enums::PanelType::SpecialRepair);
                return;
            }
            // C# BuyUsedKey：SendNPCGoods(UsedGoods, BuySub)（#2376 实现二手货列表）
            Some(EngineNpcAction::BuySub) => {
                self.send_used_goods(msg.session_id, &npc);
                return;
            }
            // C# StorageKey：ResetStorageUnlock + SendStorage + NPCStorage（含密码解锁）
            Some(EngineNpcAction::Storage) => {
                let has_pwd = match self.players.get(&msg.session_id) {
                    Some(r) => db::account_has_storage_password(&self.db_pool, &r.account_username)
                        .await
                        .unwrap_or(false),
                    None => false,
                };
                let mut lines = if has_pwd {
                    let mut body = Vec::new();
                    if mir2_shared::packets::base::serialize_packet(
                        &mut body,
                        &mir2_shared::packets::server::npc::NPCStorage,
                    )
                    .is_err()
                    {
                        warn!("Failed to serialize NPCStorage");
                    } else {
                        let _ = self
                            .gate_ref
                            .tell(SendToClient {
                                session_id: msg.session_id,
                                data: body,
                            })
                            .await;
                    }
                    vec![format!("{}: 请输入仓库密码。", npc.name)]
                } else {
                    self.send_user_storage(msg.session_id, &player_state.inventory.storage);
                    vec![format!("{}: 请妥善保管你的物品。", npc.name)]
                };
                let mut body2 = Vec::new();
                body2.extend_from_slice(&(lines.len() as i32).to_le_bytes());
                for line in &mut lines {
                    write_dotnet_string(&mut body2, line);
                }
                let _ = self
                    .gate_ref
                    .tell(SendToClient {
                        session_id: msg.session_id,
                        data: build_packet_bytes(
                            mir2_shared::enums::ServerPacketIds::NPCResponse as i16,
                            &body2,
                        ),
                    })
                    .await;
                return;
            }
            // C# CraftKey：SendNPCGoods(可制作配方产物, PanelType.Craft；NPCScript.cs:952-956)
            Some(EngineNpcAction::Craft) => {
                self.send_craft_goods(msg.session_id, &npc, &player_state)
                    .await;
                return;
            }
            // C# RefineKey：S.NPCRefine{Rate=Settings.RefineCost, Refining=CurrentRefine!=null}（:958-966）
            Some(EngineNpcAction::Refine) => {
                // C# RefineKey（NPCScript.cs:982-990）：Refining = CurrentRefine != null
                let packet = mir2_shared::packets::server::npc::NPCRefine {
                    rate: self.refine_cfg.cost as f32,
                    refining: player_state.refine_log.active_refine.is_some(),
                };
                let mut body = Vec::new();
                if mir2_shared::packets::Packet::write_body(&packet, &mut body).is_ok() {
                    let _ = self
                        .gate_ref
                        .tell(SendToClient {
                            session_id: msg.session_id,
                            data: build_packet_bytes(
                                mir2_shared::enums::ServerPacketIds::NPCRefine as i16,
                                &body,
                            ),
                        })
                        .await;
                }
                return;
            }
            // C# RefineCheckKey：S.NPCCheckRefine（空包；:967-969）
            Some(EngineNpcAction::CheckRefine) => {
                let packet = mir2_shared::packets::server::npc::NPCCheckRefine;
                let mut body = Vec::new();
                if mir2_shared::packets::Packet::write_body(&packet, &mut body).is_ok() {
                    let _ = self
                        .gate_ref
                        .tell(SendToClient {
                            session_id: msg.session_id,
                            data: build_packet_bytes(
                                mir2_shared::enums::ServerPacketIds::NPCCheckRefine as i16,
                                &body,
                            ),
                        })
                        .await;
                }
                return;
            }

            // C# RefineCollectKey：player.CollectRefine()（PlayerObject.cs:12858-12891）
            Some(EngineNpcAction::RefineCollect) => {
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let Some(active) = player_state.refine_log.active_refine.as_ref() else {
                    send_system_message(&self.gate_ref, msg.session_id, "没有精炼进行中");
                    self.send_npc_collect_refine(msg.session_id, false);
                    return;
                };
                let status = active.status;
                let finish_time = active.finish_time;
                // C# CollectRefine：CollectTime > Envir.Time → 未完成
                if status == crate::actors::refine::RefineStatus::Pending
                    && current_time < finish_time
                {
                    let remaining = finish_time - current_time;
                    send_system_message(
                        &self.gate_ref,
                        msg.session_id,
                        &format!("精炼进行中，剩余 {} 秒", remaining),
                    );
                    self.send_npc_collect_refine(msg.session_id, false);
                    return;
                }
                // 就绪且未结算：先按 CheckRefine 语义结算（成功应用属性/失败粉碎）
                let mut log = player_state.refine_log;
                if status == crate::actors::refine::RefineStatus::Pending {
                    match log
                        .settle_check(self.refine_cfg.crit_chance, self.refine_cfg.crit_increase)
                    {
                        Some(crate::actors::refine::RefineCheckResult::Applied) => {}
                        Some(crate::actors::refine::RefineCheckResult::Destroyed) => {
                            let _ = log.cancel();
                            let _ = record
                                .actor_ref
                                .ask(crate::actors::player::SetRefineLog { refine_log: log })
                                .await;
                            send_system_message(
                                &self.gate_ref,
                                msg.session_id,
                                "精炼失败，物品已粉碎",
                            );
                            self.send_npc_collect_refine(msg.session_id, false);
                            return;
                        }
                        None => {}
                    }
                }
                let Some(ri) = log.retrieve() else {
                    send_system_message(&self.gate_ref, msg.session_id, "没有精炼进行中");
                    self.send_npc_collect_refine(msg.session_id, false);
                    return;
                };
                let Some(item) = ri.item else {
                    let _ = record
                        .actor_ref
                        .ask(crate::actors::player::SetRefineLog { refine_log: log })
                        .await;
                    send_system_message(&self.gate_ref, msg.session_id, "精炼物品缺失");
                    self.send_npc_collect_refine(msg.session_id, false);
                    return;
                };
                // C# CollectRefine：背包无空格→失败（回放精炼日志避免物品丢失）
                let ok = record
                    .actor_ref
                    .ask(crate::actors::player::AddItemToInventory { item: item.clone() })
                    .await
                    .unwrap_or(false);
                if !ok {
                    let _ = log.deposit_item(item);
                    let _ = record
                        .actor_ref
                        .ask(crate::actors::player::SetRefineLog { refine_log: log })
                        .await;
                    send_system_message(&self.gate_ref, msg.session_id, "背包已满");
                    self.send_npc_collect_refine(msg.session_id, false);
                    return;
                }
                let _ = record
                    .actor_ref
                    .ask(crate::actors::player::SetRefineLog { refine_log: log })
                    .await;
                send_system_message(&self.gate_ref, msg.session_id, "精炼物品已取回");
                self.send_npc_collect_refine(msg.session_id, true);
                // 完整 UserInformation 刷新（背包 + 金币）
                if let Ok(Some(new_state)) = record.actor_ref.ask(GetPlayerState).await {
                    let packet = super::build_user_information_packet(&new_state, &self.item_infos);
                    let _ = self
                        .gate_ref
                        .tell(SendToClient {
                            session_id: msg.session_id,
                            data: packet,
                        })
                        .await;
                }
                return;
            }
            // C# HeroManageKey：player.ManageHeroes()（S.ManageHeroes；NPCScript.cs:1090-1092）
            Some(EngineNpcAction::ManageHero) => {
                let heroes = self
                    .player_heroes
                    .get(&msg.session_id)
                    .cloned()
                    .unwrap_or_default();
                super::send_manage_heroes_packet(
                    &self.gate_ref,
                    msg.session_id,
                    &player_state,
                    &heroes,
                );
                return;
            }
            // C# ReplaceWedRingKey：S.NPCReplaceWedRing { Rate = Settings.ReplaceWedRingCost=125 }（:1043-1044）
            Some(EngineNpcAction::ReplaceWedRing) => {
                let packet = mir2_shared::packets::server::npc::NPCReplaceWedRing { rate: 125.0 };
                let mut body = Vec::new();
                if mir2_shared::packets::Packet::write_body(&packet, &mut body).is_ok() {
                    let _ = self
                        .gate_ref
                        .tell(SendToClient {
                            session_id: msg.session_id,
                            data: build_packet_bytes(
                                mir2_shared::enums::ServerPacketIds::NPCReplaceWedRing as i16,
                                &body,
                            ),
                        })
                        .await;
                }
                return;
            }
            // C# GuildTerritoryKey：player.GetGuildTerritories(0)（S.GuildTerritoryPage 第 0 页）
            Some(EngineNpcAction::GuildTerritory) => {
                self.send_guild_territory_page_packet(msg.session_id, 0);
                return;
            }
            // C# GuildCreateKey（NPCScript.cs:1050-1062）：已在行会拒绝；否则 S.GuildNameRequest（等级在创建时校验）
            Some(EngineNpcAction::CreateGuild) => {
                if player_state.guild_name.is_some() {
                    send_system_message(&self.gate_ref, msg.session_id, "你已经有行会了");
                    return;
                }
                let body = Vec::new();
                let _ = self
                    .gate_ref
                    .tell(SendToClient {
                        session_id: msg.session_id,
                        data: build_packet_bytes(
                            mir2_shared::enums::ServerPacketIds::GuildNameRequest as i16,
                            &body,
                        ),
                    })
                    .await;
                return;
            }
            // C# RequestWarKey（NPCScript.cs:1064-1078）：无行会/非会长拒绝；否则 S.GuildRequestWar（客户端输入目标行会名）
            Some(EngineNpcAction::RequestWar) => {
                if player_state.guild_name.is_none() {
                    send_system_message(&self.gate_ref, msg.session_id, "你没有行会");
                    return;
                }
                if player_state.guild_rank != crate::actors::guild::GuildRank::Leader {
                    send_system_message(&self.gate_ref, msg.session_id, "只有行会会长才能宣战");
                    return;
                }
                let packet = mir2_shared::packets::server::miscellaneous::GuildRequestWar {
                    guild_name: String::new(),
                };
                let mut body = Vec::new();
                if mir2_shared::packets::Packet::write_body(&packet, &mut body).is_ok() {
                    let _ = self
                        .gate_ref
                        .tell(SendToClient {
                            session_id: msg.session_id,
                            data: build_packet_bytes(
                                mir2_shared::enums::ServerPacketIds::GuildRequestWar as i16,
                                &body,
                            ),
                        })
                        .await;
                }
                return;
            }
            // C# SendParcelKey：S.MailSendRequest（打开写信框；:1074）
            Some(EngineNpcAction::SendParcel) => {
                let packet = mir2_shared::packets::server::mail_system::MailSendRequest;
                let mut body = Vec::new();
                if mir2_shared::packets::Packet::write_body(&packet, &mut body).is_ok() {
                    let _ = self
                        .gate_ref
                        .tell(SendToClient {
                            session_id: msg.session_id,
                            data: build_packet_bytes(
                                mir2_shared::enums::ServerPacketIds::MailSendRequest as i16,
                                &body,
                            ),
                        })
                        .await;
                }
                return;
            }
            // C# CollectParcelKey（NPCScript.cs:1081-1093）：只把包裹从邮局取回（collected=true），不转移金币/物品
            Some(EngineNpcAction::CollectParcel) => {
                let released = record
                    .actor_ref
                    .ask(crate::actors::player::ReleaseMailParcels)
                    .await
                    .unwrap_or(0usize);
                // C# 结果码：0=已取回，-1=无可取回
                let result: i8 = if released > 0 { 0 } else { -1 };
                let pkt = mir2_shared::packets::server::mail_system::ParcelCollected { result };
                let mut body = Vec::new();
                if mir2_shared::packets::Packet::write_body(&pkt, &mut body).is_ok() {
                    let _ = self
                        .gate_ref
                        .tell(SendToClient {
                            session_id: msg.session_id,
                            data: build_packet_bytes(
                                mir2_shared::enums::ServerPacketIds::ParcelCollected as i16,
                                &body,
                            ),
                        })
                        .await;
                }
                // C# GetMail：刷新邮件列表
                if let Ok(Some(new_state)) = record.actor_ref.ask(GetPlayerState).await {
                    for mail in &new_state.mailbox.inbox {
                        send_mail_received_packet(&self.gate_ref, msg.session_id, mail);
                    }
                }
                return;
            }
            // C# ConsignKey：S.NPCConsign（空包；:1041-1043）
            Some(EngineNpcAction::Consign) => {
                let packet = mir2_shared::packets::server::market_system::NPCConsign {};
                let mut body = Vec::new();
                if mir2_shared::packets::Packet::write_body(&packet, &mut body).is_ok() {
                    let _ = self
                        .gate_ref
                        .tell(SendToClient {
                            session_id: msg.session_id,
                            data: build_packet_bytes(
                                mir2_shared::enums::ServerPacketIds::NPCConsign as i16,
                                &body,
                            ),
                        })
                        .await;
                }
                return;
            }
            None => {}
        }

        // 优先使用 DB 脚本（支持 GOTO 跳转）
        let mut dialog_lines = Vec::new();
        let mut current_key = msg.key.clone();
        let mut goto_depth = 0;
        const MAX_GOTO_DEPTH: usize = 10;

        // 自定义变量暂存（C# 引擎跨 section 复用）
        let mut custom_vars: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        while goto_depth < MAX_GOTO_DEPTH {
            goto_depth += 1;
            // DB 存储的 page_name 是全大写（如 [@MAIN]），查找时归一化
            let normalized_key = current_key.to_uppercase();
            let script_key = (npc.db_index, normalized_key.clone());
            if let Some(lines) = self.npc_scripts.get(&script_key).cloned() {
                // #2018：C# ParseInclude——运行时展开 #INCLUDE（Envir 根 = quest_dir 父目录）
                let lines = npc_script::expand_includes(
                    &lines,
                    self.script_dir.parent().unwrap_or(std::path::Path::new("")),
                );
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
                    let target_section = parsed.find(&want_name).or_else(|| parsed.main_section());
                    if let Some(section) = target_section {
                        let res = parsed
                            .execute_section(section, self, msg.session_id, &npc, &mut custom_vars)
                            .await;
                        if let Some(target) = res.goto {
                            current_key = format!("[@{}]", target).to_uppercase();
                            // 重用已解析脚本里的目标段（单页内 GOTO）
                            if let Some(next_sec) = parsed.find(&target) {
                                let r2 = parsed
                                    .execute_section(
                                        next_sec,
                                        self,
                                        msg.session_id,
                                        &npc,
                                        &mut custom_vars,
                                    )
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
                        dialog_lines = vec![format!("{}：你想说什么？", npc.name)];
                        break;
                    }
                }

                // 旧的 <CMD> 格式：沿用 eval_npc_script
                let mut lines = lines;
                for line in &mut lines {
                    *line = line
                        .replace("$USERNAME", &player_state.name)
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
                                let pending: Vec<&db::QuestInfo> = npc_db
                                    .collect_quest_indexes
                                    .iter()
                                    .filter_map(|qi| self.quest_infos.get(qi))
                                    .collect();
                                let finishable: Vec<&db::QuestInfo> = npc_db
                                    .finish_quest_indexes
                                    .iter()
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
                        if npc.db_index > 0
                            && self
                                .npc_goods
                                .get(&npc.db_index)
                                .is_some_and(|g| !g.is_empty())
                        {
                            lines.push("<购买/@Buy>".into());
                        }
                        if self
                            .buyback_items
                            .get(&msg.session_id)
                            .is_some_and(|l| !l.is_empty())
                        {
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
                        dialog_lines = vec![format!("{}: 请把要出售的物品放入窗口。", npc.name)];
                        self.send_npc_panel(msg.session_id, mir2_shared::enums::PanelType::Sell);
                        break;
                    }
                    "[@Repair]" => {
                        dialog_lines = vec![format!("{}: 我会帮你修好装备的。", npc.name)];
                        self.send_npc_panel(msg.session_id, mir2_shared::enums::PanelType::Repair);
                        break;
                    }
                    "[@Storage]" => {
                        // #200：仓库密码保护——有密码先解锁（C# StorageKey：SendStorage + NPCStorage，客户端弹解锁框）
                        let has_pwd = match self.players.get(&msg.session_id) {
                            Some(r) => {
                                db::account_has_storage_password(&self.db_pool, &r.account_username)
                                    .await
                                    .unwrap_or(false)
                            }
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
                                let _ = self
                                    .gate_ref
                                    .tell(SendToClient {
                                        session_id: msg.session_id,
                                        data: body,
                                    })
                                    .await;
                            }
                            dialog_lines = vec![format!("{}: 请输入仓库密码。", npc.name)];
                        } else {
                            dialog_lines = vec![format!("{}: 请妥善保管你的物品。", npc.name)];
                            self.send_user_storage(msg.session_id, &player_state.inventory.storage);
                        }
                        break;
                    }
                    _ => vec![format!("{} 说：", npc.name), format!("你说了：{}", msg.key)],
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
        let packet = build_packet_bytes(
            mir2_shared::enums::ServerPacketIds::NPCResponse as i16,
            &body,
        );

        let _ = self
            .gate_ref
            .tell(SendToClient {
                session_id: msg.session_id,
                data: packet,
            })
            .await;
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
        if state.is_dead {
            return;
        }

        // #2032：C# TeleportToNPC（7579-7598）——同图（CurrentMap.NPCs）+ CanTeleportTo 校验
        if npc.map_index != state.map_index {
            send_system_message(&self.gate_ref, msg.session_id, "该 NPC 不在当前地图");
            return;
        }
        let can_teleport = self
            .npc_infos
            .get(&npc.db_index)
            .map(|i| i.can_teleport_to)
            .unwrap_or(false);
        if !can_teleport {
            send_system_message(&self.gate_ref, msg.session_id, "该 NPC 无法传送到达");
            return;
        }
        // #2032：C# cost = Settings.TeleportToNPCCost（3000），金币不足拒绝
        let cost = self.setup_cfg.teleport_to_npc_cost.max(0) as u64;
        if state.inventory.gold < cost {
            send_system_message(&self.gate_ref, msg.session_id, "金币不足，无法传送");
            return;
        }
        let _ = record.actor_ref.ask(DeductGold { amount: cost }).await;
        super::send_gold_changed_packet(&self.gate_ref, msg.session_id, cost);

        // #2032：C# 落点 = NPC 前方格（ob.Front）；不可走则从当前格周围 7 向兜底（ShiftDirection）
        let dir = npc.direction as usize;
        let walkable = |x: i32, y: i32| {
            self.maps
                .get(&npc.map_index)
                .map(|m| m.is_walkable(x, y))
                .unwrap_or(false)
        };
        let (mut tx, mut ty) = (
            npc.x + super::MON_DIR_DX[dir],
            npc.y + super::MON_DIR_DY[dir],
        );
        if !walkable(tx, ty) {
            (tx, ty) = (state.x, state.y);
            for j in 0..7 {
                let d = (dir + j) % 8;
                let (cx, cy) = (
                    state.x + super::MON_DIR_DX[d],
                    state.y + super::MON_DIR_DY[d],
                );
                if walkable(cx, cy) {
                    (tx, ty) = (cx, cy);
                    break;
                }
            }
        }
        let new_x = tx;
        let new_y = ty;

        // 更新玩家位置
        let _ = record
            .actor_ref
            .ask(SetPlayerPosition {
                x: new_x,
                y: new_y,
                direction: npc.direction,
                map_index: None,
                is_mounted: None,
            })
            .await;
        let mut body = Vec::new();
        body.extend_from_slice(&new_x.to_le_bytes());
        body.extend_from_slice(&new_y.to_le_bytes());
        body.push(npc.direction);
        let _ = self
            .gate_ref
            .tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::UserLocation as i16,
                    &body,
                ),
            })
            .await;

        info!(
            "TeleportToNPC: {} -> {} ({}, {})",
            state.name, npc.name, new_x, new_y
        );
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
                let wm = super::build_world_map_setup_packet(
                    &self.map_infos,
                    self.setup_cfg.teleport_to_npc_cost,
                );
                let _ = self
                    .gate_ref
                    .tell(SendToClient {
                        session_id: msg.session_id,
                        data: wm,
                    })
                    .await;
                info!(
                    "WorldMapSetup: sent to session {} (on RequestMapInfo)",
                    msg.session_id
                );
            }
        }

        // C# CheckMapInfo 语义：按 map_index 回 NewMapInfo（大地图 NPC 列表），不传送
        let Some(dest_mi) = self.map_infos.get(&msg.map_id) else {
            debug!("RequestMapInfo: unknown map {}", msg.map_id);
            return;
        };
        let npcs: Vec<db::NPCInfo> = self
            .npc_infos
            .values()
            .filter(|n| n.map_index == msg.map_id && n.show_on_big_map)
            .cloned()
            .collect();
        let new_map_info =
            super::build_new_map_info_packet_from_db(dest_mi.index, &dest_mi.title, &npcs);
        let _ = self
            .gate_ref
            .tell(SendToClient {
                session_id: msg.session_id,
                data: new_map_info,
            })
            .await;
        info!(
            "RequestMapInfo: session={} map={} ({}) npcs={}",
            msg.session_id,
            msg.map_id,
            dest_mi.title,
            npcs.len()
        );
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
        // C# PlayerObject.SearchMap（~7595）：keyword < 3 字符 → 无响应
        let keyword = msg.keyword.trim();
        if keyword.len() < 3 {
            return;
        }
        let keyword_lower = keyword.to_lowercase();

        // C# GetWorldMap：Title.StartsWith(name) && BigMap > 0（首个匹配）
        let matched_map = self
            .map_infos
            .values()
            .find(|m| m.big_map > 0 && m.title.to_lowercase().starts_with(&keyword_lower));
        // C# GetWorldMapNPC：GameName.StartsWith(name) && ShowOnBigMap（首个匹配；Rust 用 name 近似）
        let matched_npc = if matched_map.is_none() {
            self.npc_infos
                .values()
                .find(|n| n.show_on_big_map && n.name.to_lowercase().starts_with(&keyword_lower))
        } else {
            None
        };

        // C# S.SearchMapResult：MapIndex(int，默认 -1) + NPCIndex(uint)
        let (map_index, npc_index) = match (matched_map, matched_npc) {
            (Some(m), _) => (m.index, 0u32),
            (None, Some(n)) => (n.map_index, n.index.max(0) as u32),
            (None, None) => (-1, 0u32),
        };
        let mut body = Vec::new();
        body.extend_from_slice(&map_index.to_le_bytes());
        body.extend_from_slice(&npc_index.to_le_bytes());
        let _ = self
            .gate_ref
            .tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::SearchMapResult as i16,
                    &body,
                ),
            })
            .await;
        // 客户端 SearchMapResult 渲染前保留系统消息兜底（Rust 附加）
        if map_index == -1 {
            send_system_message(&self.gate_ref, msg.session_id, "未找到匹配结果");
        } else {
            let kind = if npc_index != 0 { "NPC" } else { "地图" };
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                &format!("已找到{}：{}", kind, map_index),
            );
        }
        debug!(
            "SearchMap: '{}' -> map={} npc={}",
            keyword, map_index, npc_index
        );
    }
}

/// 发送 S.NewCharacter{Result}（对齐 C# Envir.NewCharacter 失败响应）
fn send_new_character_result(
    gate_ref: &kameo::actor::ActorRef<crate::gate::actor::GateActor>,
    session_id: u64,
    result: u8,
) {
    let body = vec![result];
    let _ = gate_ref
        .tell(SendToClient {
            session_id,
            data: build_packet_bytes(
                mir2_shared::enums::ServerPacketIds::NewCharacter as i16,
                &body,
            ),
        })
        .try_send();
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
        // C# Settings.AllowNewCharacter：全局禁止创建角色 → S.NewCharacter{Result=0}
        if !self
            .social_ref
            .ask(crate::actors::social::NpcGetAllowNewCharacter)
            .await
            .unwrap_or(true)
        {
            send_new_character_result(&self.gate_ref, msg.session_id, 0);
            return;
        }
        // C# 规则（Globals.MinCharacterNameLength=3 / MaxCharacterNameLength=15 / Envir.CharacterReg）：
        // 名称 3..15 字符，仅中文/下划线/ASCII 字母数字
        let name_len = msg.name.chars().count();
        let valid_name = (3..=15).contains(&name_len)
            && msg.name.chars().all(|c| {
                c == '_' || c.is_ascii_alphanumeric() || ('\u{4e00}'..='\u{9fa5}').contains(&c)
            });
        if !valid_name {
            // C# CharacterReg 不匹配 → Result=1
            send_new_character_result(&self.gate_ref, msg.session_id, 1);
            return;
        }
        // #2346：C# Envir.cs:4015——DisabledCharNames（DisabledChars.txt，大写）→ Result=1（C# 有 !IsGm 豁免）
        if self.disabled_char_names.contains(&msg.name.to_uppercase()) {
            send_new_character_result(&self.gate_ref, msg.session_id, 1);
            return;
        }
        // C# Globals.MaxCharacterCount = 4：账号角色数上限
        let existing_count = db::list_character_summaries(&self.db_pool, &msg.account_username)
            .await
            .unwrap_or_default()
            .len();
        if existing_count >= 4 {
            // C# Globals.MaxCharacterCount → Result=4
            send_new_character_result(&self.gate_ref, msg.session_id, 4);
            return;
        }
        // 检查名称是否已被使用（在线玩家）→ C# Result=5
        for r in self.players.values() {
            if r.name.eq_ignore_ascii_case(&msg.name) {
                send_new_character_result(&self.gate_ref, msg.session_id, 5);
                return;
            }
        }
        // 检查数据库中是否已有该角色
        match db::load_character(&self.db_pool, &msg.name).await {
            Ok(Some(_)) => {
                send_new_character_result(&self.gate_ref, msg.session_id, 5);
                return;
            }
            Err(e) => {
                warn!("Failed to check character name '{}': {}", msg.name, e);
            }
            Ok(None) => {}
        }

        // C#：性别不合法 → Result=2
        let gender = match mir2_shared::enums::MirGender::try_from(msg.gender) {
            Ok(g) => g,
            Err(_) => {
                send_new_character_result(&self.gate_ref, msg.session_id, 2);
                return;
            }
        };
        // C#：职业不合法 → Result=3
        let class = match mir2_shared::enums::MirClass::try_from(msg.class) {
            Ok(c) => c,
            Err(_) => {
                send_new_character_result(&self.gate_ref, msg.session_id, 3);
                return;
            }
        };
        // C# Settings.AllowCreateAssassin/AllowCreateArcher → Result=3
        let (allow_assassin, allow_archer) = self
            .social_ref
            .ask(crate::actors::social::NpcGetCreateClassOptions)
            .await
            .unwrap_or((true, true));
        if (class == mir2_shared::enums::MirClass::Assassin && !allow_assassin)
            || (class == mir2_shared::enums::MirClass::Archer && !allow_archer)
        {
            send_new_character_result(&self.gate_ref, msg.session_id, 3);
            return;
        }
        // #2434：新角色出生地图——DB 地图 idx 从 1 开始（新手村 file "0" 通常为 idx 1），按文件名解析
        let starting_map_index = self
            .map_infos
            .values()
            .find(|m| m.file_name == "0" || m.file_name.starts_with("0."))
            .map(|m| m.index)
            .unwrap_or(0);
        let mut default_state = PlayerState {
            object_id: 0,
            name: msg.name.clone(),
            map_index: starting_map_index as u16,
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
            maximum_hero_count: 1,
            step_counter: 0,
            run_counter: 0,
            run_time_ms: 0,
            cell_time_ms: 0,
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
            allow_marriage: false,
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
            mentor_damage_rate_percent: 0,
            mentee_exp_bank: 1,
            mentor_skill_boost: true,
            is_gm: false,
            gm_never_die: false,   // #1480：GM 无敌模式（C# GMNeverDie）
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
            exp_multiplier_pause_in_safe: false,
            drop_multiplier_pause_in_safe: false,
            item_drop_rate_percent: 0,
            gold_drop_rate_percent: 0,
            elements_level: 0,
            has_elemental: false,
            concentration_interrupted: false,
            concentration_interrupt_time: 0,
            bind_map_index: starting_map_index,
            bind_x: 330,
            bind_y: 330,
            level_effects: 0,
            is_mentor: false,
            mentee_exp: 0,
            mentor_exp: 0,
            mentor_date: 0,
            mentor_damage_bonus: false,
            newbie_exp_bonus: false,
            exp_bonus_lover_percent: 0,
            exp_bonus_mentee_percent: 0,
            exp_bonus_newbie_percent: 0,
            guild_buff_exp_percent: 0,
            guild_buff_fish_rate_percent: 0,
            mine_rate_percent: 0,
            gem_rate_percent: 0,
            craft_rate_percent: 0,
            hp_rate_percent: 0,
            mp_rate_percent: 0,
            max_ac_rate_percent: 0,
            max_mac_rate_percent: 0,
            max_dc_rate_percent: 0,
            max_mc_rate_percent: 0,
            max_sc_rate_percent: 0,
            attack_speed_rate_percent: 0,
            chat_banned_until_ms: 0,
            chat_window_start_ms: 0,
            chat_tick: 0,
            char_ban_expiry_ticks: 0,
            char_ban_reason: String::new(),
            skill_gain_multiplier: 0,
            guild_buff_mine_rate_percent: 0,
            guild_buff_stats: mir2_shared::data::stats::Stats::new(),
            no_experience_map: false,
            brown_until_ms: 0,
            mount_loyalty_decrease_time: 0,
            mount_loyalty_increase_time: 0,
            torch_burn_time: 0,
            last_damage_ms: 0,
            pot_hp_amount: 0,
            pot_mp_amount: 0,
            pot_time_ms: 0,
        };
        // #1527：新建角色初始属性按 C# Settings.ClassBaseStats[Class].Calculate(1)（与升级重算同一映射）
        {
            let base_stats = mir2_shared::data::stats::BaseStats::new(class);
            for bs in &base_stats.stats {
                let val = bs.calculate(class, default_state.level as i32);
                use mir2_shared::enums::Stat;
                match bs.stat {
                    Stat::HP => {
                        default_state.max_hp = val;
                        default_state.hp = val;
                    }
                    Stat::MP => {
                        default_state.max_mp = val;
                        default_state.mp = val;
                    }
                    Stat::MinDC => default_state.min_attack = val,
                    Stat::MaxDC => default_state.max_attack = val,
                    Stat::MinMC => default_state.min_mc = val,
                    Stat::MaxMC => default_state.max_mc = val,
                    Stat::MinSC => default_state.min_sc = val,
                    Stat::MaxSC => default_state.max_sc = val,
                    Stat::MinAC => default_state.min_ac = val,
                    Stat::MaxAC => {
                        default_state.max_ac = val;
                        default_state.defence = val;
                    }
                    Stat::MinMAC => default_state.min_mac = val,
                    Stat::MaxMAC => default_state.max_mac = val,
                    Stat::Agility => default_state.agility = val,
                    Stat::Accuracy => default_state.accuracy = val,
                    _ => {}
                }
            }
        }
        // 经验曲线（C# RefreshMaxExperience：MaxExperience = ExperienceList[Level-1]）
        if let Some(first) = self.experience_list.first() {
            default_state.max_experience = *first;
        }
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
        let data = build_packet_bytes(
            mir2_shared::enums::ServerPacketIds::NewCharacterSuccess as i16,
            &body,
        );
        debug!(
            "NewCharacterSuccess: session={} name={} bytes={}",
            msg.session_id,
            msg.name,
            data.len()
        );
        let _ = self
            .gate_ref
            .tell(SendToClient {
                session_id: msg.session_id,
                data,
            })
            .await;

        debug!(
            "NewCharacter: session={} name={} class={} gender={}",
            msg.session_id, msg.name, msg.class, msg.gender
        );
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
        // C# Settings.AllowDeleteCharacter：关闭时 Result=0
        if !self
            .social_ref
            .ask(crate::actors::social::NpcGetAllowDeleteCharacter)
            .await
            .unwrap_or(true)
        {
            let body = vec![0u8]; // Result = 0
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::DeleteCharacter as i16,
                        &body,
                    ),
                })
                .await;
            debug!(
                "DeleteCharacter denied (AllowDeleteCharacter=false): session={}",
                msg.session_id
            );
            return;
        }

        // 按索引找到属于该账号的角色（C#：按 Account.Characters 索引查找）
        let chars = match db::list_characters_by_account(&self.db_pool, &msg.account_username).await
        {
            Ok(c) => c,
            Err(e) => {
                warn!(
                    "DeleteCharacter: failed to list characters for {}: {}",
                    msg.account_username, e
                );
                return;
            }
        };
        let idx = msg.character_index.max(0) as usize;
        let Some((char_name, _, _, _)) = chars.get(idx) else {
            // C#：找不到 → S.DeleteCharacter { Result = 1 }
            let body = vec![1u8];
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::DeleteCharacter as i16,
                        &body,
                    ),
                })
                .await;
            debug!(
                "DeleteCharacter: index {} not found for account {}",
                msg.character_index, msg.account_username
            );
            return;
        };

        // 真删（含全部子表数据；C# 软删 Deleted 标记，Rust 直接清除，观察行为一致）
        if let Err(e) = db::delete_character(&self.db_pool, char_name).await {
            warn!("DeleteCharacter: failed to delete '{}': {}", char_name, e);
            return;
        }
        // C#：成功 → S.DeleteCharacterSuccess { CharacterIndex }
        let packet = mir2_shared::packets::server::account::DeleteCharacterSuccess {
            character_index: msg.character_index,
        };
        let mut body = Vec::new();
        if packet.write_body(&mut body).is_ok() {
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::DeleteCharacterSuccess as i16,
                        &body,
                    ),
                })
                .await;
        }
        debug!(
            "DeleteCharacter: deleted '{}' (index={}) for account {}",
            char_name, msg.character_index, msg.account_username
        );
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

        // C# Settings.AllowNewHero → Result=0
        let (allow_new_hero, can_create_class, hero_required_level) = self
            .social_ref
            .ask(crate::actors::social::NpcGetHeroCreateOptions)
            .await
            .unwrap_or((true, vec![true; 5], 22));
        if !allow_new_hero {
            let body = vec![0u8];
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::NewHero as i16,
                        &body,
                    ),
                })
                .await;
            return;
        }
        // #2366：C# Settings.Hero_RequiredLevel（NPC [@CREATEHERO] 页等级门槛，NPCScript.cs:1117-1121）
        // Bevy 客户端从 Hero 管理对话框直发 NewHero，绕过 NPC 页 → 服务端补校验
        if state.level < hero_required_level as u16 {
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                &format!("需要 {} 级才能创建英雄", hero_required_level),
            );
            return;
        }

        // C# CharacterReg：名称 3..15，中文/下划线/ASCII 字母数字 → Result=1
        let name_len = msg.name.chars().count();
        let valid_name = (3..=15).contains(&name_len)
            && msg.name.chars().all(|c| {
                c == '_' || c.is_ascii_alphanumeric() || ('\u{4e00}'..='\u{9fa5}').contains(&c)
            });
        if !valid_name {
            let body = vec![1u8];
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::NewHero as i16,
                        &body,
                    ),
                })
                .await;
            return;
        }
        // #2346：C# Envir.cs:4085——DisabledCharNames（DisabledChars.txt，大写）→ Result=1（C# 有 !IsGm 豁免）
        if self.disabled_char_names.contains(&msg.name.to_uppercase()) {
            let body = vec![1u8];
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::NewHero as i16,
                        &body,
                    ),
                })
                .await;
            return;
        }
        // C# Settings.Hero_CanCreateClass → Result=3
        let class_idx = msg.class as u8 as usize;
        if !can_create_class.get(class_idx).copied().unwrap_or(true) {
            let body = vec![3u8];
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::NewHero as i16,
                        &body,
                    ),
                })
                .await;
            return;
        }
        // #188：创建英雄（C# CreateHero :9595-9599：heroCount >= MaximumHeroCount → Result=4；成功分配下一空闲 index）
        let hero_count = self
            .player_heroes
            .get(&msg.session_id)
            .map(|v| v.len())
            .unwrap_or(0);
        let result = hero_create_result(&msg.name, hero_count, state.maximum_hero_count);
        if result == 10 {
            // 下一空闲 index（1..=maximum_hero_count）
            let next_index = (1..=state.maximum_hero_count as i32)
                .find(|i| {
                    !self
                        .player_heroes
                        .get(&msg.session_id)
                        .map(|hs| hs.iter().any(|h| h.index == *i))
                        .unwrap_or(false)
                })
                .unwrap_or(hero_count as i32 + 1) as u8;
            self.player_heroes
                .entry(msg.session_id)
                .or_default()
                .push(HeroInfo {
                    index: next_index as i32,
                    name: msg.name.clone(),
                    level: 1,
                    class: msg.class,
                    gender: msg.gender,
                    dead: false,
                    sealed: false,
                    autopot: false,
                    experience: 0,
                    // #2418：英雄经验曲线第 1 档（C# HeroExperienceList[Level-1]，Level=1 → [0]）
                    max_experience: self
                        .hero_exp_list
                        .first()
                        .copied()
                        .unwrap_or(super::hero_stats::HERO_MAX_EXPERIENCE),
                });
            // C# CreateHero（PlayerObject.cs:9610）：有封印符配置（HeroSealItemName）且背包有空位时，
            // 英雄以"英雄封印符"形式发放（不出战，使用后恢复）；否则直接创建为出战英雄。
            let seal_item = self
                .item_infos
                .values()
                .find(|i| i.name.eq_ignore_ascii_case("SealedHero"))
                .cloned();
            if let Some(si) = seal_item {
                if state.inventory.has_space() {
                    if let Some(hs) = self.player_heroes.get_mut(&msg.session_id) {
                        if let Some(h) = hs.iter_mut().find(|h| h.index == next_index as i32) {
                            h.sealed = true;
                        }
                    }
                    let item = mir2_shared::data::item::UserItem {
                        item_index: si.index,
                        count: 1,
                        added_stats: {
                            let mut m = mir2_shared::data::stats::Stats::default();
                            m.set(mir2_shared::enums::Stat::Hero, next_index as i32);
                            m
                        },
                        ..Default::default()
                    };
                    let _ = record
                        .actor_ref
                        .ask(crate::actors::player::AddItemToInventory { item })
                        .await;
                    send_system_message(
                        &self.gate_ref,
                        msg.session_id,
                        "英雄已创建，封印符已放入背包，使用后即可出战",
                    );
                } else {
                    let _ = record
                        .actor_ref
                        .ask(SetHeroIndex {
                            hero_index: next_index,
                        })
                        .await;
                    self.broadcast_hero_spawn(msg.session_id).await;
                    self.send_hero_information_packet(msg.session_id).await;
                }
            } else {
                let _ = record
                    .actor_ref
                    .ask(SetHeroIndex {
                        hero_index: next_index,
                    })
                    .await;
                // #198：创建成功后生成英雄对象
                self.broadcast_hero_spawn(msg.session_id).await;
                // #203：下发完整英雄信息（背包/装备/自动药）
                self.send_hero_information_packet(msg.session_id).await;
            }
            // 持久化英雄
            let heroes = self
                .player_heroes
                .get(&msg.session_id)
                .cloned()
                .unwrap_or_default();
            let db_heroes: Vec<db::DbHero> = heroes
                .iter()
                .map(|h| db::DbHero {
                    index: h.index,
                    name: h.name.clone(),
                    level: h.level,
                    class: h.class as u8,
                    gender: h.gender as u8,
                    dead: h.dead,
                    sealed: h.sealed,
                    autopot: h.autopot,
                    experience: h.experience,
                    max_experience: h.max_experience,
                })
                .collect();
            if let Err(e) = db::save_heroes(&self.db_pool, &state.name, &db_heroes).await {
                warn!("Failed to save heroes on NewHero: {}", e);
            }
        }
        let body = vec![result];
        let _ = self
            .gate_ref
            .tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::NewHero as i16,
                    &body,
                ),
            })
            .await;
        // 重新下发英雄列表
        let state_after = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => state.clone(),
        };
        let heroes = self
            .player_heroes
            .get(&msg.session_id)
            .cloned()
            .unwrap_or_default();
        send_manage_heroes_packet(&self.gate_ref, msg.session_id, &state_after, &heroes);
        debug!(
            "NewHero: {} name={} gender={:?} class={:?} result={}",
            state.name, msg.name, msg.gender, msg.class, result
        );
    }
}
impl WorldActor {
    /// #珍珠商店：下发 S.NPCPearlGoods（商品=该 NPC 商店商品，珍珠价 = 金币价 × rate；对齐 C# PearlBuyKey）
    pub(crate) async fn send_pearl_goods(&mut self, session_id: u64, npc: &NpcState) {
        use mir2_shared::packets::server::special_systems::NPCPearlGoods;
        let goods = self
            .npc_goods
            .get(&npc.db_index)
            .cloned()
            .unwrap_or_default();
        let mut items = Vec::new();
        for good in &goods {
            let mut item = mir2_shared::data::item::UserItem {
                item_index: good.item_index,
                count: good.count as u16,
                // #2376：常规商店商品 unique_id = item_index（C# 客户端 BuyItem 发 UniqueID 语义）
                unique_id: good.item_index as u64,
                ..Default::default()
            };
            enrich_item_info(&mut item, &self.item_infos);
            items.push(item);
        }
        let rate = if npc.db_index > 0 {
            self.npc_infos
                .get(&npc.db_index)
                .map(|n| n.rate as f32 / 100.0)
                .unwrap_or(1.0)
        } else {
            1.0
        };
        let pkt = NPCPearlGoods {
            list: items,
            rate,
            panel_type: mir2_shared::enums::PanelType::Buy,
        };
        let mut body = Vec::new();
        if pkt.write_body(&mut body).is_ok() {
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::NPCPearlGoods as i16,
                        &body,
                    ),
                })
                .await;
        }
        self.session_pearl_shop.insert(session_id);
        debug!(
            "珍珠商店: session={} goods={} rate={}",
            session_id,
            goods.len(),
            rate
        );
    }

    /// #1356：下发觉醒面板打开包（0=觉醒 1=分解 2=降级 3=重置；C# S.NPCAwakening/S.NPCDisassemble/
    /// S.NPCDowngrade/S.NPCReset）
    pub(crate) async fn send_awakening_panel(&self, session_id: u64, service: u8) {
        use mir2_shared::packets::server::awakening_system::{
            NPCAwakening, NPCDisassemble, NPCDowngrade, NPCReset,
        };
        let (opcode, body) = match service {
            1 => {
                let p = NPCDisassemble {};
                let mut b = Vec::new();
                let _ = p.write_body(&mut b);
                (
                    mir2_shared::enums::ServerPacketIds::NPCDisassemble as i16,
                    b,
                )
            }
            2 => {
                let p = NPCDowngrade {};
                let mut b = Vec::new();
                let _ = p.write_body(&mut b);
                (mir2_shared::enums::ServerPacketIds::NPCDowngrade as i16, b)
            }
            3 => {
                let p = NPCReset {};
                let mut b = Vec::new();
                let _ = p.write_body(&mut b);
                (mir2_shared::enums::ServerPacketIds::NPCReset as i16, b)
            }
            _ => {
                let p = NPCAwakening {};
                let mut b = Vec::new();
                let _ = p.write_body(&mut b);
                (mir2_shared::enums::ServerPacketIds::NPCAwakening as i16, b)
            }
        };
        let _ = self
            .gate_ref
            .tell(SendToClient {
                session_id,
                data: build_packet_bytes(opcode, &body),
            })
            .await;
    }

    /// NPC 脚本 REVIVEHERO：复活当前英雄（对齐 C# ActionType.ReviveHero，简化：清 dead 标记）
    pub(crate) async fn npc_revive_hero(&mut self, session_id: u64) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        // C# ReviveHero：CurrentMap.Info.NoHero → 禁止复活
        if self
            .map_infos
            .get(&(state.map_index as i32))
            .map(|m| m.no_hero)
            .unwrap_or(false)
        {
            send_system_message(&self.gate_ref, session_id, "该地图无法复活英雄");
            return;
        }
        let Some(heroes) = self.player_heroes.get(&session_id).cloned() else {
            send_system_message(&self.gate_ref, session_id, "你没有英雄");
            return;
        };
        let Some(hero) = heroes
            .iter()
            .find(|h| h.index as u8 == state.hero_index)
            .cloned()
        else {
            return;
        };
        // C# ReviveHero：仅当英雄死亡（HP == 0）时复活；Rust 用 dead 标记 + AI HP<=0 判定
        let ai_dead = self
            .hero_ai_states
            .get(&session_id)
            .map(|ai| ai.hp <= 0)
            .unwrap_or(false);
        if hero.dead || ai_dead {
            if let Some(hs) = self.player_heroes.get_mut(&session_id) {
                if let Some(h) = hs.iter_mut().find(|h| h.index == hero.index) {
                    h.dead = false;
                }
            }
            // 复活回满 HP/MP（C# CurrentHero.HP = Hero.Stats[HP]、MP = Stats[MP]）
            if let Some(ai) = self.hero_ai_states.get_mut(&session_id) {
                ai.hp = ai.max_hp;
                ai.mp = ai.max_mp;
            }
            // DB 保存用更新后的列表：只复活当前英雄，其他英雄保持原 dead/sealed
            let db_heroes: Vec<db::DbHero> = self
                .player_heroes
                .get(&session_id)
                .map(|hs| {
                    hs.iter()
                        .map(|h| db::DbHero {
                            index: h.index,
                            name: h.name.clone(),
                            level: h.level,
                            class: h.class as u8,
                            gender: h.gender as u8,
                            dead: h.dead,
                            sealed: h.sealed,
                            autopot: h.autopot,
                            experience: h.experience,
                            max_experience: h.max_experience,
                        })
                        .collect()
                })
                .unwrap_or_default();
            if let Err(e) = db::save_heroes(&self.db_pool, &state.name, &db_heroes).await {
                warn!("Failed to save heroes on ReviveHero: {}", e);
            }
            send_system_message(
                &self.gate_ref,
                session_id,
                &format!("英雄 {} 已复活", hero.name),
            );
        }
        debug!("NPC ReviveHero: session={}", session_id);
    }

    /// NPC 脚本 SEALHERO：封印当前英雄（对齐 C# ActionType.SealHero）
    ///
    /// C# SealHero：背包有空位 → 生成"英雄封印符"物品（AddedStats[Stat.Hero]=英雄索引）、
    /// 收起出战英雄、英雄置 sealed；之后可用封印符恢复。
    pub(crate) async fn npc_seal_hero(&mut self, session_id: u64) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        let Some(heroes) = self.player_heroes.get(&session_id).cloned() else {
            send_system_message(&self.gate_ref, session_id, "你没有英雄");
            return;
        };
        let Some(hero) = heroes
            .iter()
            .find(|h| h.index as u8 == state.hero_index)
            .cloned()
        else {
            send_system_message(&self.gate_ref, session_id, "你没有出战的英雄");
            return;
        };
        // C# SealHero：背包无空位时封印失败（FreeSpace == 0 → return）
        if !state.inventory.has_space() {
            send_system_message(&self.gate_ref, session_id, "背包没有空位，无法封印英雄");
            return;
        }
        // C# Settings.HeroSealItemName = "SealedHero"：找不到封印符物品则无法封印
        let seal_item = match self
            .item_infos
            .values()
            .find(|i| i.name.eq_ignore_ascii_case("SealedHero"))
            .cloned()
        {
            Some(it) => it,
            None => {
                send_system_message(&self.gate_ref, session_id, "无法封印英雄（缺少封印符配置）");
                return;
            }
        };
        if hero.sealed {
            send_system_message(&self.gate_ref, session_id, "英雄已被封印");
            return;
        }
        // 收起出战英雄（C# DespawnHero + UpdateHeroSpawnState(None)）
        let _ = record
            .actor_ref
            .ask(crate::actors::player::SetHeroIndex { hero_index: 0 })
            .await;
        crate::actors::social_packets::send_hero_update_packet(&self.gate_ref, session_id, 0);
        self.broadcast_hero_remove(state.object_id).await;
        // 英雄置 sealed
        if let Some(hs) = self.player_heroes.get_mut(&session_id) {
            if let Some(h) = hs.iter_mut().find(|h| h.index == hero.index) {
                h.sealed = true;
            }
        }
        // 生成封印符物品：AddedStats[Stat.Hero] = 英雄索引（C# item.AddedStats[Stat.Hero] = CurrentHero.Index）
        let item = mir2_shared::data::item::UserItem {
            item_index: seal_item.index,
            count: 1,
            added_stats: {
                let mut m = mir2_shared::data::stats::Stats::default();
                m.set(mir2_shared::enums::Stat::Hero, hero.index);
                m
            },
            ..Default::default()
        };
        let _ = record
            .actor_ref
            .ask(crate::actors::player::AddItemToInventory { item })
            .await;
        // 持久化
        let heroes_now = self
            .player_heroes
            .get(&session_id)
            .cloned()
            .unwrap_or_default();
        let db_heroes: Vec<db::DbHero> = heroes_now
            .iter()
            .map(|h| db::DbHero {
                index: h.index,
                name: h.name.clone(),
                level: h.level,
                class: h.class as u8,
                gender: h.gender as u8,
                dead: h.dead,
                sealed: h.sealed,
                autopot: h.autopot,
                experience: h.experience,
                max_experience: h.max_experience,
            })
            .collect();
        if let Err(e) = db::save_heroes(&self.db_pool, &state.name, &db_heroes).await {
            warn!("Failed to save heroes on SealHero: {}", e);
        }
        send_system_message(
            &self.gate_ref,
            session_id,
            &format!("英雄 {} 已封印，封印符已放入背包", hero.name),
        );
        debug!("NPC SealHero: session={}", session_id);
    }

    /// NPC 脚本 DELETEHERO：删除当前英雄（对齐 C# ActionType.DeleteHero）
    pub(crate) async fn npc_delete_hero(&mut self, session_id: u64) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        let Some(heroes) = self.player_heroes.get(&session_id).cloned() else {
            send_system_message(&self.gate_ref, session_id, "你没有英雄");
            return;
        };
        let before = heroes.len();
        let remaining: Vec<HeroInfo> = heroes
            .into_iter()
            .filter(|h| h.index as u8 != state.hero_index)
            .collect();
        if remaining.len() < before {
            self.player_heroes.insert(session_id, remaining.clone());
            // 清空当前英雄索引 + 移除英雄对象
            let _ = record
                .actor_ref
                .ask(crate::actors::player::SetHeroIndex { hero_index: 0 })
                .await;
            self.broadcast_hero_remove(state.object_id).await;
            let db_heroes: Vec<db::DbHero> = remaining
                .iter()
                .map(|h| db::DbHero {
                    index: h.index,
                    name: h.name.clone(),
                    level: h.level,
                    class: h.class as u8,
                    gender: h.gender as u8,
                    dead: h.dead,
                    sealed: h.sealed,
                    autopot: h.autopot,
                    experience: h.experience,
                    max_experience: h.max_experience,
                })
                .collect();
            if let Err(e) = db::save_heroes(&self.db_pool, &state.name, &db_heroes).await {
                warn!("Failed to save heroes on DeleteHero: {}", e);
            }
            send_system_message(&self.gate_ref, session_id, "英雄已删除");
        }
        debug!("NPC DeleteHero: session={}", session_id);
    }
}

impl WorldActor {
    /// NPC 脚本 TAKECONQUESTGOLD：所有者行会取走攻城金库（对齐 C# ActionType.TakeConquestGold）
    pub(crate) async fn npc_take_conquest_gold(&mut self, session_id: u64, conquest_index: i32) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        let Some(guild_name) = &state.guild_name else {
            return;
        };
        let Some(conquest) = self
            .conquest_instances
            .iter_mut()
            .find(|c| c.id == conquest_index)
        else {
            return;
        };
        if conquest.owner_guild.as_deref() != Some(guild_name.as_str()) {
            return;
        }
        let amount = conquest.gold_storage;
        if amount > 0 {
            conquest.gold_storage = 0;
            let _ = self
                .social_ref
                .ask(crate::actors::social::NpcGuildGoldChange {
                    session_id,
                    amount: amount.min(u32::MAX as u64) as u32,
                    change_type: 3,
                })
                .await;
            send_system_message(
                &self.gate_ref,
                session_id,
                &format!("已从攻城金库取走 {} 金币", amount),
            );
        }
        debug!(
            "NPC TakeConquestGold: conquest={} gold={}",
            conquest_index, amount
        );
    }

    /// NPC 脚本 SETCONQUESTRATE：所有者设置税率（对齐 C# ActionType.SetConquestRate / NPCRate）
    pub(crate) async fn npc_set_conquest_rate(
        &mut self,
        session_id: u64,
        conquest_index: i32,
        rate: u8,
    ) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        let Some(guild_name) = &state.guild_name else {
            return;
        };
        let Some(conquest) = self
            .conquest_instances
            .iter_mut()
            .find(|c| c.id == conquest_index)
        else {
            return;
        };
        if conquest.owner_guild.as_deref() == Some(guild_name.as_str()) {
            conquest.tax_rate = rate;
            debug!(
                "NPC SetConquestRate: conquest={} rate={}",
                conquest_index, rate
            );
        }
    }

    /// NPC 脚本 STARTCONQUEST：开/停战争（对齐 C# ActionType.StartConquest：强制 StartWar / WarIsOn=false）
    pub(crate) async fn npc_start_conquest(&mut self, session_id: u64, conquest_index: i32) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        let Some(conquest) = self
            .conquest_instances
            .iter_mut()
            .find(|c| c.id == conquest_index)
        else {
            return;
        };
        if conquest.state == crate::actors::world::conquest::WarState::InProgress {
            conquest.end_war();
            send_system_message(
                &self.gate_ref,
                session_id,
                &format!("领地 {} 战争已停止", conquest.id),
            );
        } else {
            let attacker = state.guild_name.clone().unwrap_or_default();
            conquest.start_war(&attacker);
            send_system_message(
                &self.gate_ref,
                session_id,
                &format!("领地 {} 战争已开始", conquest.id),
            );
        }
        debug!(
            "NPC StartConquest: conquest={} state={:?}",
            conquest_index, conquest.state
        );
    }

    /// NPC 脚本 SCHEDULECONQUEST：宣战（对齐 C# ActionType.ScheduleConquest：非所有者且未开战 → 设 Attacker）
    pub(crate) async fn npc_schedule_conquest(&mut self, session_id: u64, conquest_index: i32) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        let Some(guild_name) = &state.guild_name else {
            return;
        };
        let Some(conquest) = self
            .conquest_instances
            .iter_mut()
            .find(|c| c.id == conquest_index)
        else {
            return;
        };
        if conquest.owner_guild.as_deref() != Some(guild_name.as_str())
            && conquest.state == crate::actors::world::conquest::WarState::Idle
        {
            conquest.attacker_guild = Some(guild_name.clone());
            send_system_message(
                &self.gate_ref,
                session_id,
                &format!("行会 {} 已宣战领地 {}", guild_name, conquest.id),
            );
            debug!(
                "NPC ScheduleConquest: conquest={} attacker={}",
                conquest_index, guild_name
            );
        }
    }
}

impl WorldActor {
    /// 攻城结构查找：conquest 内指定类型，id 作为 1-based 序号（对齐 C# GateList.Index 概念）
    pub(crate) fn find_siege_structure(
        &self,
        conquest_id: i32,
        kind: crate::actors::world::conquest::SiegeStructureType,
        id: i32,
    ) -> Option<u32> {
        // #1523：按数据索引精确匹配（C# GateList.Index / WallList.Index）
        self.siege_structures
            .iter()
            .find(|(_, s)| {
                s.conquest_id == conquest_id && s.structure_type == kind && s.index == id
            })
            .map(|(oid, _)| *oid)
    }

    /// NPC 脚本 CONQUESTGATE/CONQUESTWALL：修复攻城结构（对齐 C#：GM 免费，非 GM 扣行会金币）
    pub(crate) async fn npc_repair_siege_structure(
        &mut self,
        session_id: u64,
        conquest_index: i32,
        id: i32,
        kind: crate::actors::world::conquest::SiegeStructureType,
    ) {
        let Some(oid) = self.find_siege_structure(conquest_index, kind.clone(), id) else {
            return;
        };
        let Some(structure) = self.siege_structures.get(&oid).cloned() else {
            return;
        };
        let cost = structure.repair_cost();
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if !state.is_gm {
            if cost == 0 {
                return;
            }
            let guild_gold = self
                .social_ref
                .ask(crate::actors::social::NpcGetGuildGold { session_id })
                .await
                .unwrap_or(0);
            if guild_gold < cost {
                return;
            }
            let _ = self
                .social_ref
                .ask(crate::actors::social::NpcGuildGoldChange {
                    session_id,
                    amount: cost.min(u32::MAX as u64) as u32,
                    change_type: 2,
                })
                .await;
        }
        if let Some(s) = self.siege_structures.get_mut(&oid) {
            s.repair_full();
        }
        debug!(
            "NPC RepairSiege: conquest={} id={} kind={:?} cost={}",
            conquest_index, id, kind, cost
        );
    }

    /// NPC 脚本 OPENGATE/CLOSEGATE：城门开关
    pub(crate) async fn npc_open_close_gate(
        &mut self,
        _session_id: u64,
        conquest_index: i32,
        gate_id: i32,
        open: bool,
    ) {
        let Some(oid) = self.find_siege_structure(
            conquest_index,
            crate::actors::world::conquest::SiegeStructureType::CastleGate,
            gate_id,
        ) else {
            return;
        };
        if let Some(s) = self.siege_structures.get_mut(&oid) {
            s.is_open = open;
        }
        debug!(
            "NPC OpenCloseGate: conquest={} gate={} open={}",
            conquest_index, gate_id, open
        );
    }

    /// NPC 脚本 CONQUESTREPAIRALL：GM 修复全部结构（对齐 C# ActionType.ConquestRepairAll）
    pub(crate) async fn npc_repair_all(&mut self, session_id: u64, conquest_index: i32) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if !state.is_gm {
            return;
        }
        for (_, s) in self.siege_structures.iter_mut() {
            if s.conquest_id == conquest_index {
                s.repair_full();
            }
        }
        debug!("NPC RepairAll: conquest={}", conquest_index);
    }
}

impl WorldActor {
    /// NPC 脚本 GIVECREDIT/TAKECREDIT：增减账户积分（对齐 C# ActionType.GiveCredit/TakeCredit；db 权威）
    pub(crate) async fn npc_change_credit(&mut self, session_id: u64, delta: i64) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let username = record.account_username.clone();
        // C# GiveCredit：账户积分上限 uint.MaxValue（正数 delta clamp）
        let delta = if delta > 0 {
            let current = db::get_account_credit(&self.db_pool, &username)
                .await
                .unwrap_or(0);
            let remaining = (u32::MAX as u64).saturating_sub(current.min(u32::MAX as u64));
            (delta as u64).min(remaining) as i64
        } else {
            delta
        };
        if delta == 0 {
            return;
        }
        if let Err(e) = db::add_account_credit(&self.db_pool, &username, delta).await {
            warn!("NPC ChangeCredit: failed for {}: {}", username, e);
            return;
        }
        let current = db::get_account_credit(&self.db_pool, &username)
            .await
            .unwrap_or(0);
        send_system_message(
            &self.gate_ref,
            session_id,
            &format!("账户积分变化 {}（当前 {}）", delta, current),
        );
        // C# GainCredit：S.GainedCredit（客户端积分浮字，仅正向）
        if delta > 0 {
            let packet = mir2_shared::packets::server::drops::GainedCredit {
                credit: delta.min(u32::MAX as i64) as u32,
            };
            let mut body = Vec::new();
            if packet.write_body(&mut body).is_ok() {
                let _ = self
                    .gate_ref
                    .tell(SendToClient {
                        session_id,
                        data: build_packet_bytes(
                            mir2_shared::enums::ServerPacketIds::GainedCredit as i16,
                            &body,
                        ),
                    })
                    .await;
            }
        }
        // C# TakeCredit（NPCSegment.cs:3304）：S.LoseCredit（客户端积分浮字，仅负向）
        if delta < 0 {
            let packet = mir2_shared::packets::server::drops::LoseCredit {
                credit: (-delta).min(u32::MAX as i64) as u32,
            };
            let mut body = Vec::new();
            if packet.write_body(&mut body).is_ok() {
                let _ = self
                    .gate_ref
                    .tell(SendToClient {
                        session_id,
                        data: build_packet_bytes(
                            mir2_shared::enums::ServerPacketIds::LoseCredit as i16,
                            &body,
                        ),
                    })
                    .await;
            }
        }
        debug!(
            "NPC ChangeCredit: {} delta={} current={}",
            username, delta, current
        );
    }
}

impl WorldActor {
    /// NPC 脚本 CONQUESTSIEGE/CONQUESTGUARD：生成攻城器/守卫结构（对齐 C# ActionType；数据层，地图表现留待攻城专项）
    /// guard_id：CONQUESTGUARD 的守卫索引（C# ConquestGuildArcherInfo 按 id 落点，Info.Location）
    pub(crate) async fn npc_spawn_siege_structure(
        &mut self,
        session_id: u64,
        conquest_index: i32,
        kind: crate::actors::world::conquest::SiegeStructureType,
        guard_id: Option<i32>,
    ) {
        let Some(conquest) = self
            .conquest_instances
            .iter()
            .find(|c| c.id == conquest_index)
            .cloned()
        else {
            return;
        };
        let oid = self.alloc_object_id();
        let mut structure = match kind {
            crate::actors::world::conquest::SiegeStructureType::Catapult => {
                crate::actors::world::conquest::SiegeStructure::catapult(oid)
            }
            crate::actors::world::conquest::SiegeStructureType::ArcherTower => {
                crate::actors::world::conquest::SiegeStructure::archer_tower(oid)
            }
            _ => return,
        };
        structure.conquest_id = conquest_index;
        // #1513：箭塔按守卫 id 落点（C# ArcherMonster.Spawn(ConquestMap, Info.Location)）；未配置回退 (0,0)
        structure.x = 0;
        structure.y = 0;
        if kind == crate::actors::world::conquest::SiegeStructureType::ArcherTower {
            if let Some(gid) = guard_id {
                structure.index = gid; // #1523：AFFORDGUARD/CONQUESTGUARD 按守卫索引定位
                if let Some(g) = conquest.guards.iter().find(|g| g.index == gid) {
                    structure.x = g.x;
                    structure.y = g.y;
                    structure.repair_cost = g.repair_cost;
                }
            }
        }
        let (sx, sy) = (structure.x, structure.y);
        if let Some(c) = self
            .conquest_instances
            .iter_mut()
            .find(|c| c.id == conquest_index)
        {
            c.siege_structure_ids.push(oid);
        }
        self.siege_structures.insert(oid, structure);
        send_system_message(
            &self.gate_ref,
            session_id,
            &format!("已生成攻城结构 #{}", oid),
        );
        debug!(
            "NPC SpawnSiege: conquest={} kind={:?} oid={} guard={:?} at ({}, {})",
            conquest_index, kind, oid, guard_id, sx, sy
        );
    }
}

impl WorldActor {
    /// 查找玩家行会拥有的领地（owner == guild_name）
    fn guild_gt(&self, guild_name: &str) -> Option<usize> {
        self.conquest_instances
            .iter()
            .position(|c| c.owner_guild.as_deref() == Some(guild_name))
    }

    /// NPC 脚本 BUYGT：会长购买当前地图领地（对齐 C# ActionType.BuyGT，简化：买第一个无主领地）
    pub(crate) async fn npc_gt_buy(&mut self, session_id: u64) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if state.guild_rank != crate::actors::guild::GuildRank::Leader {
            send_system_message(&self.gate_ref, session_id, "只有行会会长才能购买领地");
            return;
        }
        let Some(guild_name) = &state.guild_name else {
            return;
        };
        if self.guild_gt(guild_name).is_some() {
            send_system_message(&self.gate_ref, session_id, "行会已拥有领地");
            return;
        }
        let buy_gold = self.conquest_cfg.buy_gold;
        let gold = self
            .social_ref
            .ask(crate::actors::social::NpcGetGuildGold { session_id })
            .await
            .unwrap_or(0);
        if gold < buy_gold {
            send_system_message(
                &self.gate_ref,
                session_id,
                &format!("行会资金不足（需要 {}）", buy_gold),
            );
            return;
        }
        let Some(idx) = self
            .conquest_instances
            .iter()
            .position(|c| c.owner_guild.is_none())
        else {
            send_system_message(&self.gate_ref, session_id, "没有可购买的领地");
            return;
        };
        let _ = self
            .social_ref
            .ask(crate::actors::social::NpcGuildGoldChange {
                session_id,
                amount: buy_gold as u32,
                change_type: 2,
            })
            .await;
        self.conquest_instances[idx].owner_guild = Some(guild_name.clone());
        // C# BUYGT：GTRent = Now + GTDays
        self.conquest_instances[idx].rent_expire_tick = self.tick_count
            + self.conquest_cfg.gt_days as u64 * crate::actors::world::conquest::TICKS_PER_DAY;
        send_system_message(&self.gate_ref, session_id, "领地购买成功");
        debug!(
            "NPC BuyGT: {} bought conquest {}",
            guild_name, self.conquest_instances[idx].id
        );
    }

    /// NPC 脚本 TELEPORTGT：传送到行会领地（对齐 C# ActionType.TeleportGT）
    pub(crate) async fn npc_gt_teleport(&mut self, session_id: u64) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        let Some(guild_name) = &state.guild_name else {
            return;
        };
        let Some(gt) = self.guild_gt(guild_name) else {
            send_system_message(&self.gate_ref, session_id, "行会未拥有领地");
            return;
        };
        let map_index = self.conquest_instances[gt].map_index as u16;
        // C# TeleportGT：传送到 GT 地图第一个安全区 SafeZones[0].Location；无安全区回退 (330,330) 并告警
        let (tx, ty) = self
            .map_infos
            .get(&self.conquest_instances[gt].map_index)
            .and_then(|mi| mi.safe_zones.first())
            .map(|sz| (sz.x, sz.y))
            .unwrap_or_else(|| {
                warn!(
                    "NPC TeleportGT: map {} has no safe zone, fallback (330,330)",
                    map_index
                );
                (330, 330)
            });
        // 完整跨图传送（复用 teleport_player：get_or_load_map + SetPlayerPosition + MapChanged + UserLocation）
        super::npc_script::teleport_player(&mut *self, session_id, map_index, tx, ty).await;
        send_system_message(&self.gate_ref, session_id, "已传送至行会领地");
        debug!("NPC TeleportGT: {} -> map {}", guild_name, map_index);
    }

    /// NPC 脚本 EXTENDGT：会长延长领地租期（对齐 C# ActionType.ExtendGT：+Settings.GTDays）
    pub(crate) async fn npc_gt_extend(&mut self, session_id: u64) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if state.guild_rank != crate::actors::guild::GuildRank::Leader {
            send_system_message(&self.gate_ref, session_id, "只有行会会长才能延长领地");
            return;
        }
        let Some(guild_name) = &state.guild_name else {
            return;
        };
        let Some(gt) = self.guild_gt(guild_name) else {
            send_system_message(&self.gate_ref, session_id, "行会未拥有领地");
            return;
        };
        let extend_gold = self.conquest_cfg.extend_gold;
        let gold = self
            .social_ref
            .ask(crate::actors::social::NpcGetGuildGold { session_id })
            .await
            .unwrap_or(0);
        if gold < extend_gold {
            send_system_message(
                &self.gate_ref,
                session_id,
                &format!("行会资金不足（需要 {}）", extend_gold),
            );
            return;
        }
        let _ = self
            .social_ref
            .ask(crate::actors::social::NpcGuildGoldChange {
                session_id,
                amount: extend_gold as u32,
                change_type: 2,
            })
            .await;
        let days = self.conquest_cfg.gt_days;
        // C# EXTENDGT：GTRent += GTDays
        self.conquest_instances[gt].rent_expire_tick = self.conquest_instances[gt]
            .rent_expire_tick
            .saturating_add(days as u64 * crate::actors::world::conquest::TICKS_PER_DAY);
        let left = self.conquest_instances[gt].gt_days_left(self.tick_count);
        send_system_message(
            &self.gate_ref,
            session_id,
            &format!("领地租期延长 {} 天（剩余 {} 天）", days, left),
        );
        debug!("NPC ExtendGT: {} +{}d (left {})", guild_name, days, left);
    }

    /// NPC 脚本 DISPLAYGTRENTALDAYS：显示剩余天数（对齐 C# ActionType.DisplayGTRentalDays）
    pub(crate) async fn npc_gt_display_days(&mut self, session_id: u64) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        let Some(guild_name) = &state.guild_name else {
            return;
        };
        let Some(gt) = self.guild_gt(guild_name) else {
            send_system_message(&self.gate_ref, session_id, "行会未拥有领地");
            return;
        };
        let left = self.conquest_instances[gt].gt_days_left(self.tick_count);
        send_system_message(&self.gate_ref, session_id, &format!("领地剩余 {} 天", left));
        debug!("NPC DisplayGTRentalDays: {} days={}", guild_name, left);
    }

    /// NPC 脚本 GTALLRECALL：会长召回所有在线同公会玩家（对齐 C# ActionType.GTAllRecall）
    pub(crate) async fn npc_gt_recall_all(&mut self, session_id: u64) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if state.guild_rank != crate::actors::guild::GuildRank::Leader {
            send_system_message(&self.gate_ref, session_id, "只有行会会长才能召回成员");
            return;
        }
        let Some(guild_name) = state.guild_name.clone() else {
            return;
        };
        let mut targets = Vec::new();
        for (sid, r) in &self.players {
            if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                if os.guild_name.as_deref() == Some(guild_name.as_str()) && *sid != session_id {
                    targets.push(*sid);
                }
            }
        }
        for sid in &targets {
            if let Some(r) = self.players.get(sid) {
                // C# GTAllRecall：目标坐标 = 会长坐标 + Random.Next(4) 偏移，避免叠格
                let _ = r
                    .actor_ref
                    .ask(crate::actors::player::SetPlayerPosition {
                        x: state.x + fastrand::i32(0..4),
                        y: state.y + fastrand::i32(0..4),
                        direction: state.direction,
                        map_index: Some(state.map_index),
                        is_mounted: None,
                    })
                    .await;
            }
        }
        send_system_message(&self.gate_ref, session_id, "已召回行会成员");
        debug!("NPC GTAllRecall: {} members", targets.len());
    }

    /// NPC 脚本 GTRECALL <name>：会长召回指定同公会玩家（对齐 C# ActionType.GTRecall）
    pub(crate) async fn npc_gt_recall(&mut self, session_id: u64, member_name: &str) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if state.guild_rank != crate::actors::guild::GuildRank::Leader {
            send_system_message(&self.gate_ref, session_id, "只有行会会长才能召回成员");
            return;
        }
        let Some(guild_name) = state.guild_name.clone() else {
            return;
        };
        for r in self.players.values() {
            if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                if os.guild_name.as_deref() == Some(guild_name.as_str())
                    && os.name.eq_ignore_ascii_case(member_name)
                {
                    // C# GTRecall：目标坐标 = 会长坐标 + Random.Next(4) 偏移
                    let _ = r
                        .actor_ref
                        .ask(crate::actors::player::SetPlayerPosition {
                            x: state.x + fastrand::i32(0..4),
                            y: state.y + fastrand::i32(0..4),
                            direction: state.direction,
                            map_index: Some(state.map_index),
                            is_mounted: None,
                        })
                        .await;
                    send_system_message(&self.gate_ref, session_id, &format!("已召回 {}", os.name));
                    return;
                }
            }
        }
        send_system_message(
            &self.gate_ref,
            session_id,
            &format!("未找到在线成员 {}", member_name),
        );
        debug!("NPC GTRecall: {} not found", member_name);
    }

    /// NPC 脚本 GTSALE <price>：会长挂售领地（对齐 C# ActionType.GTSale，最低 200 万）
    pub(crate) async fn npc_gt_sale(&mut self, session_id: u64, price: u64) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if state.guild_rank != crate::actors::guild::GuildRank::Leader {
            send_system_message(&self.gate_ref, session_id, "只有行会会长才能挂售领地");
            return;
        }
        let Some(guild_name) = &state.guild_name else {
            return;
        };
        let Some(gt) = self.guild_gt(guild_name) else {
            send_system_message(&self.gate_ref, session_id, "行会未拥有领地");
            return;
        };
        // C# GTForSale：gt.Price > 0 → 已在挂售（:874-878）
        if self.conquest_instances[gt].for_sale {
            send_system_message(&self.gate_ref, session_id, "领地已在挂售中，请先取消挂售");
            return;
        }
        if price < self.conquest_cfg.gt_sale_min_price {
            send_system_message(
                &self.gate_ref,
                session_id,
                &format!("挂售价格最低 {}", self.conquest_cfg.gt_sale_min_price),
            );
            return;
        }
        self.conquest_instances[gt].for_sale = true;
        self.conquest_instances[gt].sale_price = price;
        send_system_message(
            &self.gate_ref,
            session_id,
            &format!("领地已挂售，价格 {}", price),
        );
        debug!("NPC GTSale: {} price={}", guild_name, price);
    }

    /// NPC 脚本 GTCANCELSALE：取消挂售（对齐 C# ActionType.GTCancelSale）
    pub(crate) async fn npc_gt_cancel_sale(&mut self, session_id: u64) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if state.guild_rank != crate::actors::guild::GuildRank::Leader {
            send_system_message(&self.gate_ref, session_id, "只有行会会长才能取消挂售");
            return;
        }
        let Some(guild_name) = &state.guild_name else {
            return;
        };
        let Some(gt) = self.guild_gt(guild_name) else {
            return;
        };
        // C# EndGTSale：gt.Price <= 0 → 未在挂售（:892-896）
        if !self.conquest_instances[gt].for_sale {
            send_system_message(&self.gate_ref, session_id, "领地未在挂售");
            return;
        }
        self.conquest_instances[gt].for_sale = false;
        self.conquest_instances[gt].sale_price = 0;
        send_system_message(&self.gate_ref, session_id, "已取消领地挂售");
        debug!("NPC GTCancelSale: {}", guild_name);
    }
}

impl WorldActor {
    /// C# CraftKey：SendNPCGoods(可制作配方产物, PanelType.Craft)——按 recipe_infos 过滤 CanCraft
    /// （NPCScript.cs:952-956；等级/性别/职业/任务/flags 校验与 CraftItemRequest 一致）
    pub(crate) async fn send_craft_goods(
        &mut self,
        session_id: u64,
        npc: &NpcState,
        player_state: &crate::actors::player::PlayerState,
    ) {
        let mut items = Vec::new();
        for recipe in &self.recipe_infos {
            if recipe.product_item_index <= 0 {
                continue;
            }
            // C# RecipeInfo.CanCraft
            if let Some(lv) = recipe.required_level {
                if player_state.level < lv {
                    continue;
                }
            }
            if let Some(g) = recipe.required_gender {
                if player_state.gender as u8 != g {
                    continue;
                }
            }
            if !recipe.required_classes.is_empty()
                && !recipe
                    .required_classes
                    .contains(&(player_state.class as u8))
            {
                continue;
            }
            if recipe
                .required_quests
                .iter()
                .any(|q| !player_state.quest_log.completed_indices.contains(q))
            {
                continue;
            }
            if recipe.required_flags.iter().any(|f| {
                player_state
                    .flags
                    .get(&format!("NPC_FLAG_{}", f))
                    .copied()
                    .unwrap_or(0)
                    < 1
            }) {
                continue;
            }
            let mut item = mir2_shared::data::item::UserItem {
                item_index: recipe.product_item_index,
                count: recipe.product_count,
                // #2536：合成产物 unique_id = recipe_id（Rust C.CraftItem 按 recipe_id 寻址；
                // C# 客户端按 ItemIndex 匹配配方，不受影响）
                unique_id: recipe.recipe_id as u64,
                ..Default::default()
            };
            if let Some(info) = self.item_infos.get(&recipe.product_item_index) {
                item.max_dura = info.durability as u16;
                item.current_dura = info.durability as u16;
            }
            crate::actors::world::enrich_item_info(&mut item, &self.item_infos);
            items.push(item);
        }
        self.send_npc_goods_items_panel(
            session_id,
            npc,
            items,
            mir2_shared::enums::PanelType::Craft,
        );
    }
}

impl WorldActor {
    /// #2378：发送 S.NPCCollectRefine（C# CollectRefine 结果包；try_send 尽力送达）
    fn send_npc_collect_refine(&self, session_id: u64, success: bool) {
        let packet = mir2_shared::packets::server::npc::NPCCollectRefine { success };
        let mut body = Vec::new();
        if mir2_shared::packets::Packet::write_body(&packet, &mut body).is_ok() {
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::NPCCollectRefine as i16,
                        &body,
                    ),
                })
                .try_send();
        }
    }
}

/// #2368：引擎级特殊页 → 面板动作（C# NPCScript.ProcessSpecial 商店类 key）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EngineNpcAction {
    /// C# BuyKey/BuyNewKey：SendNPCGoods(PanelType.Buy)
    Goods,
    /// C# BuySellKey/BuySellNewKey：SendNPCGoods(Buy) + NPCSell
    GoodsAndSell,
    /// C# SellKey：NPCSell
    Sell,
    /// C# RepairKey：NPCRepair
    Repair,
    /// C# SRepairKey：NPCSRepair
    SpecialRepair,
    /// C# BuyUsedKey：SendNPCGoods(UsedGoods, BuySub)
    BuySub,
    /// C# StorageKey：ResetStorageUnlock + SendStorage + NPCStorage
    Storage,
    /// C# CraftKey：SendNPCGoods(可制作配方产物, PanelType.Craft)
    Craft,
    /// C# RefineKey：S.NPCRefine
    Refine,
    /// C# RefineCheckKey：S.NPCCheckRefine
    CheckRefine,
    /// C# RefineCollectKey：CollectRefine（NPCCollectRefine + 取回精炼物品）
    RefineCollect,
    /// C# HeroManageKey：S.ManageHeroes
    ManageHero,
    /// C# ReplaceWedRingKey：S.NPCReplaceWedRing
    ReplaceWedRing,
    /// C# GuildTerritoryKey：S.GuildTerritoryPage（第 0 页）
    GuildTerritory,
    /// C# GuildCreateKey：S.GuildNameRequest
    CreateGuild,
    /// C# RequestWarKey：S.GuildRequestWar
    RequestWar,
    /// C# SendParcelKey：S.MailSendRequest
    SendParcel,
    /// C# CollectParcelKey：收取全部包裹 + S.ParcelCollected
    CollectParcel,
    /// C# ConsignKey：S.NPCConsign
    Consign,
}

pub(crate) fn engine_npc_action(key: &str) -> Option<EngineNpcAction> {
    match key.to_uppercase().as_str() {
        "[@BUY]" | "[@BUYNEW]" => Some(EngineNpcAction::Goods),
        "[@BUYSELL]" | "[@BUYSELLNEW]" => Some(EngineNpcAction::GoodsAndSell),
        "[@SELL]" => Some(EngineNpcAction::Sell),
        "[@REPAIR]" => Some(EngineNpcAction::Repair),
        "[@SREPAIR]" => Some(EngineNpcAction::SpecialRepair),
        "[@BUYUSED]" => Some(EngineNpcAction::BuySub),
        "[@STORAGE]" => Some(EngineNpcAction::Storage),
        "[@CRAFT]" => Some(EngineNpcAction::Craft),
        "[@REFINE]" => Some(EngineNpcAction::Refine),
        "[@REFINECHECK]" => Some(EngineNpcAction::CheckRefine),
        "[@REFINECOLLECT]" => Some(EngineNpcAction::RefineCollect),
        "[@MANAGEHERO]" => Some(EngineNpcAction::ManageHero),
        "[@REPLACEWEDDINGRING]" => Some(EngineNpcAction::ReplaceWedRing),
        "[@GUILDTERRITORY]" => Some(EngineNpcAction::GuildTerritory),
        "[@CREATEGUILD]" => Some(EngineNpcAction::CreateGuild),
        "[@REQUESTWAR]" => Some(EngineNpcAction::RequestWar),
        "[@SENDPARCEL]" => Some(EngineNpcAction::SendParcel),
        "[@COLLECTPARCEL]" => Some(EngineNpcAction::CollectParcel),
        "[@CONSIGN]" => Some(EngineNpcAction::Consign),
        _ => None,
    }
}

// ============================================================
// 钓鱼系统
// ============================================================

/// C# PlayerObject.FishingCast 钓具加成（PlayerObject.cs:10964-11066）
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct FishingGearBonuses {
    /// C# flexibilityStat（byte）：鱼竿 CriticalRate + Hook 槽（收获时进度>50 半值加入）
    pub flexibility: i32,
    /// C# successStat（sbyte）：鱼竿 MaxAC + Bait/Reel 槽 MaxAC
    pub success_stat: i32,
    /// C# failedAddSuccessMin（byte）：Finder 槽 MinAC（FishingChanceCounter!=0 时随机补偿下限）
    pub failed_add_min: i32,
    /// C# failedAddSuccessMax（byte）：Finder 槽 MaxAC（补偿上限，C# Random.Next 上界开区间）
    pub failed_add_max: i32,
    /// C# nibbleMin（byte）：Float 槽 MinAC（咬钩窗口下限；Rust 未实现咬钩阶段，仅记录）
    pub nibble_min: i32,
    /// C# nibbleMax（byte）：Float 槽 MaxAC（咬钩窗口上限；FishingNibbleChance = 5 + Random(nibbleMin, nibbleMax)）
    pub nibble_max: i32,
    /// C# FishingAutoReelChance（sbyte）：Reel 槽 MaxMAC（自动收竿概率；Rust 未实现 FishFound 阶段，仅记录）
    pub auto_reel_chance: i32,
}

/// 计算 C# FishingCast 钓具加成（PlayerObject.cs:10964-11066）：
/// 基础值 = 鱼竿 Stats（flexibility: CriticalRate；success: MaxAC），随后遍历鱼竿 Slots[0..5]：
/// - Hook：flexibility += AddedStats[CriticalRate] + realItem.Stats[CriticalRate]
/// - Float：nibbleMin += realItem.Stats[MinAC]；nibbleMax += realItem.Stats[MaxAC]
/// - Bait：successStat += realItem.Stats[MaxAC]
/// - Finder：failedAddSuccessMin += realItem.Stats[MinAC]；failedAddSuccessMax += realItem.Stats[MaxAC]
/// - Reel：FishingAutoReelChance += realItem.Stats[MaxMAC]；successStat += realItem.Stats[MaxAC]
/// 数值按 C# 目标类型钳制：flexibility/nibble/failedAdd 为 byte（0..=255），success/autoReel 为 sbyte（-128..=127）。
/// 注：DB item_type 为 C# 编号（Hook=28..Reel=32），经 shared_item_type 转 SharedRust 枚举后匹配。
fn compute_fishing_gear_bonuses(
    rod_info: Option<&crate::db::ItemInfo>,
    slots: &[Option<mir2_shared::data::item::UserItem>],
    item_infos: &std::collections::HashMap<i32, crate::db::ItemInfo>,
) -> FishingGearBonuses {
    use mir2_shared::enums::{ItemType, Stat};

    let stat =
        |info: &crate::db::ItemInfo, s: Stat| info.stats.get(&(s as u8)).copied().unwrap_or(0);

    let mut b = FishingGearBonuses::default();
    if let Some(rod) = rod_info {
        b.flexibility += stat(rod, Stat::CriticalRate);
        b.success_stat += stat(rod, Stat::MaxAC);
    }
    for slot in slots.iter().flatten() {
        let Some(info) = item_infos.get(&slot.item_index) else {
            continue;
        };
        match shared_item_type(info.item_type) {
            ItemType::Hook => {
                b.flexibility +=
                    slot.added_stats.get(Stat::CriticalRate) + stat(info, Stat::CriticalRate);
            }
            ItemType::Float => {
                b.nibble_min += stat(info, Stat::MinAC);
                b.nibble_max += stat(info, Stat::MaxAC);
            }
            ItemType::Bait => {
                b.success_stat += stat(info, Stat::MaxAC);
            }
            ItemType::Finder => {
                b.failed_add_min += stat(info, Stat::MinAC);
                b.failed_add_max += stat(info, Stat::MaxAC);
            }
            ItemType::Reel => {
                b.auto_reel_chance += stat(info, Stat::MaxMAC);
                b.success_stat += stat(info, Stat::MaxAC);
            }
            _ => {}
        }
    }
    b.flexibility = b.flexibility.clamp(0, 255);
    b.nibble_min = b.nibble_min.clamp(0, 255);
    b.nibble_max = b.nibble_max.clamp(0, 255);
    b.failed_add_min = b.failed_add_min.clamp(0, 255);
    b.failed_add_max = b.failed_add_max.clamp(0, 255);
    b.success_stat = b.success_stat.clamp(-128, 127);
    b.auto_reel_chance = b.auto_reel_chance.clamp(-128, 127);
    b
}

/// C# FishingCast 抛竿成功率（PlayerObject.cs:11054-11060）：
/// SuccessStart + successStat + (FishingChanceCounter!=0 ? Random(failedAddSuccessMin, failedAddSuccessMax) : 0)
///     + FishingChanceCounter*SuccessMultiplier + FishRatePercent，钳制 0..=100。
fn compute_fishing_chance(
    success_start: i32,
    success_stat: i32,
    fish_rate_percent: i32,
    success_counter: i32,
    success_multiplier: i32,
    failed_add_min: i32,
    failed_add_max: i32,
) -> i32 {
    let failed_add = if success_counter != 0 {
        cs_random_next(failed_add_min, failed_add_max)
    } else {
        0
    };
    (success_start
        + success_stat
        + fish_rate_percent
        + failed_add
        + success_counter * success_multiplier)
        .clamp(0, 100)
}

/// C# Envir.Random.Next(min, max)：上界开区间（min <= result < max）；min == max 返回 min（C# 文档行为）。
/// min > max 是 C# 会抛异常的非法数据，这里保守返回 min，避免服务器 panic。
pub(crate) fn cs_random_next(min: i32, max: i32) -> i32 {
    if min >= max {
        min
    } else {
        fastrand::i32(min..max)
    }
}

pub struct FishingCastRequest {
    pub session_id: u64,
    pub fishing_type: u8,
}

impl Message<FishingCastRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: FishingCastRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if state.is_dead {
            return;
        }

        // #2386：C# FishingCast(false)——收竿（无需重新校验鱼竿/水域；有鱼才判定收获）
        if msg.fishing_type != 0 {
            let fish_found = self
                .fishing_sessions
                .get(&msg.session_id)
                .map(|s| s.fish_found)
                .unwrap_or(false);
            self.reel_fishing(msg.session_id, fish_found).await;
            return;
        }

        // C# FishingCast：鱼竿校验（IsFishingRod = shape 49/50 + CurrentDura != 0）
        let rod_item = match state
            .inventory
            .get_equipment(crate::actors::inventory::EquipmentSlot::Weapon)
        {
            Some(r) => r.clone(),
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "你需要装备鱼竿才能钓鱼");
                return;
            }
        };
        let rod_info = self.item_infos.get(&rod_item.item_index).cloned();
        let is_rod = rod_info
            .as_ref()
            .map(|i| is_fishing_rod_shape(i.shape))
            .unwrap_or(false);
        if !is_rod || rod_item.current_dura == 0 {
            send_system_message(&self.gate_ref, msg.session_id, "你需要装备鱼竿才能钓鱼");
            return;
        }

        // C# FishingCast：前方 3 格 + 水格（Cell.FishingAttribute >= 0）
        let (fx, fy) = point_move(state.x, state.y, state.direction, 3);
        let Some(map_data) = self.maps.get(&state.map_index).cloned() else {
            return;
        };
        if !map_data.is_valid(fx, fy) || map_data.fishing_attribute(fx, fy) < 0 {
            send_system_message(&self.gate_ref, msg.session_id, "这里不是水域，无法钓鱼");
            return;
        }
        let cell_attribute = map_data.fishing_attribute(fx, fy);

        // C# FishingCast：鱼钩必需（rod.Slots[Hook] == null → NeedHook；#2352）
        if rod_item.slots.first().and_then(|s| s.as_ref()).is_none() {
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                "你需要鱼钩（放在鱼竿鱼钩槽）",
            );
            return;
        }

        // C# FishingCast：successStat = rod.Info.Stats[Stat.MaxAC]；flexibilityStat = CriticalRate
        // + 5 个钓具插槽加成（#2352：Hook 灵活度 / Bait、Reel 成功率 / Finder 失败补偿）
        // （行会 BuffFishRate 已由 tick 缓存；Float 咬钩窗口与 Reel 自动收竿见 compute_fishing_gear_bonuses 注释）
        let bonuses =
            compute_fishing_gear_bonuses(rod_info.as_ref(), &rod_item.slots, &self.item_infos);
        let success_stat = bonuses.success_stat;
        let flexibility = bonuses.flexibility;
        let fish_rate = state.guild_buff_fish_rate_percent;
        // C#：FishingChance = SuccessStart + successStat
        //     + (FishingChanceCounter!=0 ? Random(failedAddSuccessMin, failedAddSuccessMax) : 0)
        //     + FishingChanceCounter*SuccessMultiplier + FishRatePercent
        //（FishingChanceCounter 跨抛竿保留：每次满进度 +1，成功收获清零；失败补偿 = Finder 槽）
        let success_counter = self
            .fishing_success_counters
            .get(&msg.session_id)
            .copied()
            .unwrap_or(0);
        let chance = compute_fishing_chance(
            self.fishing_cfg.success_start,
            success_stat,
            fish_rate,
            success_counter as i32,
            self.fishing_cfg.success_multiplier,
            bonuses.failed_add_min,
            bonuses.failed_add_max,
        );

        // #1313：抛竿消耗鱼竿 Bait 槽 1 个鱼饵（C# GetBait/ConsumeItem；无饵不能抛竿）
        if !record
            .actor_ref
            .ask(crate::actors::player::FishingConsumeBait { amount: 1 })
            .await
            .unwrap_or(false)
        {
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                "你需要鱼饵（放在鱼竿鱼饵槽）",
            );
            return;
        }
        // C#：抛竿鱼竿耐久 -1、鱼钩耐久 -1；有失败计数时探鱼器耐久 -1
        let _ = record
            .actor_ref
            .ask(crate::actors::player::FishingRodDurability { amount: 1 })
            .await;
        let _ = record
            .actor_ref
            .ask(crate::actors::player::FishingGearDamageMsg { slot: 0, amount: 1 })
            .await;
        if success_counter != 0 {
            let _ = record
                .actor_ref
                .ask(crate::actors::player::FishingGearDamageMsg { slot: 3, amount: 1 })
                .await;
        }

        // #2386：咬钩概率（C# FishingNibbleChance = 5 + Random(nibbleMin,nibbleMax)）与自动收竿概率（Reel 槽）
        let nibble_chance =
            (5 + cs_random_next(bonuses.nibble_min, bonuses.nibble_max)).clamp(0, 100);
        let auto_reel_chance = bonuses.auto_reel_chance.clamp(0, 100);
        self.fishing_sessions.insert(
            msg.session_id,
            FishingSession {
                cell_attribute,
                chance,
                flexibility,
                nibble_chance,
                auto_reel_chance,
                fish_found: false,
                found_tick: 0,
                progress: 0,
            },
        );

        let _ = record
            .actor_ref
            .ask(SetFishing {
                is_fishing: true,
                autocast: false,
            })
            .await;

        // Send FishingUpdate: progress=1 (waiting), success=false
        use mir2_shared::packets::server::miscellaneous::FishingUpdate;
        let packet = FishingUpdate {
            fishing_progress: 1,
            fishing_success: false,
        };
        let mut body = Vec::new();
        if let Ok(()) = mir2_shared::packets::Packet::write_body(&packet, &mut body) {
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::FishingUpdate as i16,
                        &body,
                    ),
                })
                .await;
        }

        debug!(
            "FishingCast: {} type={} at ({},{}) attr={} chance={} flexibility={}",
            state.name, msg.fishing_type, fx, fy, cell_attribute, chance, flexibility
        );
    }
}

pub struct FishingChangeAutocastRequest {
    pub session_id: u64,
    pub enabled: bool,
}

impl Message<FishingChangeAutocastRequest> for WorldActor {
    type Reply = ();
    async fn handle(
        &mut self,
        msg: FishingChangeAutocastRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let _ = record
            .actor_ref
            .ask(SetFishing {
                is_fishing: state.is_fishing,
                autocast: msg.enabled,
            })
            .await;

        // Send FishingUpdate: progress=5 (autocast toggle), success=enabled
        use mir2_shared::packets::server::miscellaneous::FishingUpdate;
        let packet = FishingUpdate {
            fishing_progress: 5,
            fishing_success: msg.enabled,
        };
        let mut body = Vec::new();
        if let Ok(()) = mir2_shared::packets::Packet::write_body(&packet, &mut body) {
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::FishingUpdate as i16,
                        &body,
                    ),
                })
                .await;
        }

        debug!(
            "FishingChangeAutocast: {} enabled={}",
            state.name, msg.enabled
        );
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
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        debug!("Opendoor: {} door_index={}", state.name, msg.door_index);

        // Track open door state per map
        let map_key = state.map_index;
        self.open_doors.insert((map_key, msg.door_index));

        // Send Opendoor response to the player
        send_opendoor(&self.gate_ref, msg.session_id, msg.door_index, false).await;

        // Broadcast to all other players on the same map
        broadcast_opendoor_async(
            &self.gate_ref,
            &self.players,
            map_key,
            msg.door_index,
            false,
            msg.session_id,
        )
        .await;
    }
}

// ============================================================
// NPC确认输入
// ============================================================

pub struct NPCConfirmInputRequest {
    pub session_id: u64,
    pub npc_id: u32,
    /// C# C.NPCConfirmInput.PageName（默认 NPC 路由用段名）
    pub page_name: String,
    pub input_text: String,
}

impl Message<NPCConfirmInputRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: NPCConfirmInputRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        debug!(
            "NPCConfirmInput: {} npc_id={} page={} input={}",
            state.name, msg.npc_id, msg.page_name, msg.input_text
        );

        // C# MirConnection.cs:2166-2168：NPCID == 默认 NPC 对象 ID → 默认 NPC 脚本（PageName 为段名）
        if msg.npc_id == self.default_npc_object_id {
            self.queue_default_npc(msg.session_id, &msg.page_name);
            // C# CallDefaultNPC（PlayerObject.cs:7887）：下发 S.NPCUpdate（客户端刷新当前 NPC）
            let packet = mir2_shared::packets::server::npc_interaction::NPCUpdate {
                npc_id: self.default_npc_object_id,
            };
            let mut body = Vec::new();
            if packet.write_body(&mut body).is_ok() {
                let _ = self
                    .gate_ref
                    .tell(SendToClient {
                        session_id: msg.session_id,
                        data: build_packet_bytes(
                            mir2_shared::enums::ServerPacketIds::NPCUpdate as i16,
                            &body,
                        ),
                    })
                    .await;
            }
            return;
        }

        // Try to match input as a quest file_name for quick acceptance
        let npc = match self.npcs.get(&msg.npc_id) {
            Some(n) => n,
            None => return,
        };
        if npc.db_index > 0 {
            if let Some(npc_db) = self.npc_infos.get(&npc.db_index) {
                // Check if input matches a collectable quest
                let quest_db = npc_db
                    .collect_quest_indexes
                    .iter()
                    .filter_map(|qi| self.quest_infos.get(qi))
                    .find(|q| q.file_name == msg.input_text || q.name == msg.input_text);
                if let Some(quest_db) = quest_db {
                    if state.level >= quest_db.required_min_level as u16
                        && (quest_db.required_max_level == 0
                            || state.level <= quest_db.required_max_level as u16)
                    {
                        // Check not already accepted
                        if let Ok(None) = record
                            .actor_ref
                            .ask(GetQuest {
                                quest_index: quest_db.index,
                            })
                            .await
                        {
                            // #2016：C# AcceptQuest（11269-11274）——已完成任务不可再接
                            if let Ok(true) = record
                                .actor_ref
                                .ask(HasCompletedQuest {
                                    quest_index: quest_db.index,
                                })
                                .await
                            {
                                send_system_message(&self.gate_ref, msg.session_id, "该任务已完成");
                                return;
                            }
                            // #2016：C# AcceptQuest（11276）——并发任务上限（Globals.MaxConcurrentQuests=20）
                            if let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await {
                                if !st.quest_log.can_accept() {
                                    send_system_message(
                                        &self.gate_ref,
                                        msg.session_id,
                                        "任务数量已达上限（20）",
                                    );
                                    return;
                                }
                            }
                            // #2026：C# QuestInfo.CanAccept——RequiredClass 位掩码 + RequiredQuest 前置（对齐包路径 #2004）
                            if let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await {
                                if quest_db.required_class != 0 {
                                    let class_bit: i32 = match st.class {
                                        mir2_shared::enums::MirClass::Warrior => 1,
                                        mir2_shared::enums::MirClass::Wizard => 2,
                                        mir2_shared::enums::MirClass::Taoist => 4,
                                        mir2_shared::enums::MirClass::Assassin => 8,
                                        mir2_shared::enums::MirClass::Archer => 16,
                                    };
                                    if quest_db.required_class & class_bit == 0 {
                                        send_system_message(
                                            &self.gate_ref,
                                            msg.session_id,
                                            "职业不符合",
                                        );
                                        return;
                                    }
                                }
                            }
                            if quest_db.required_quest > 0 {
                                if let Ok(false) = record
                                    .actor_ref
                                    .ask(HasCompletedQuest {
                                        quest_index: quest_db.required_quest,
                                    })
                                    .await
                                {
                                    send_system_message(
                                        &self.gate_ref,
                                        msg.session_id,
                                        "需要先完成前置任务",
                                    );
                                    return;
                                }
                            }
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_secs())
                                .unwrap_or(0);
                            let quest = make_quest_instance(quest_db, now);
                            if let Ok(true) = record.actor_ref.ask(AcceptQuest { quest }).await {
                                send_system_message(
                                    &self.gate_ref,
                                    msg.session_id,
                                    &format!("任务已接受: {}", quest_db.name),
                                );
                                // #2026：M43 ChangeQuest 任务日志推送（与包路径一致）
                                if let Ok(Some(q)) = record
                                    .actor_ref
                                    .ask(GetQuest {
                                        quest_index: quest_db.index,
                                    })
                                    .await
                                {
                                    crate::actors::social_packets::send_quest_change_packet(
                                        &self.gate_ref,
                                        msg.session_id,
                                        &q,
                                    );
                                }
                            }
                            return;
                        }
                    }
                }
                // Check if input matches a finishable quest
                let quest_db = npc_db
                    .finish_quest_indexes
                    .iter()
                    .filter_map(|qi| self.quest_infos.get(qi))
                    .find(|q| q.file_name == msg.input_text || q.name == msg.input_text);
                if let Some(quest_db) = quest_db {
                    // Complete the quest
                    if let Ok(Some(quest)) = record
                        .actor_ref
                        .ask(GetQuest {
                            quest_index: quest_db.index,
                        })
                        .await
                    {
                        if quest.status == QuestStatus::InProgress {
                            let _ = record
                                .actor_ref
                                .ask(CompleteQuest {
                                    quest_index: quest_db.index,
                                })
                                .await;
                            // #2022：C# FinishQuest——交任务扣除携带物品
                            self.take_quest_carry_items(msg.session_id, quest_db.index)
                                .await;
                            // Grant rewards（C# FinishQuest：Exp×ExpRate / Gold×DropRate / GainCredit）
                            let _ = record
                                .actor_ref
                                .ask(AddExperience {
                                    amount: self.apply_global_exp_multiplier(quest_db.exp_reward),
                                    experience_list: self.experience_list.clone(),
                                })
                                .await;
                            let gold = (quest_db.gold_reward.max(0) as f64 * self.drop_rate) as u64;
                            let _ = record.actor_ref.ask(AddGold { amount: gold }).await;
                            // #2024：C# FinishQuest——信用奖励（GainCredit + S.GainedCredit）
                            self.grant_quest_credit(msg.session_id, quest_db.credit_reward as i64)
                                .await;
                            // #2024：C# SendUpdateQuest Remove——S.CompleteQuest 完成包（与包/脚本路径一致）
                            crate::actors::social_packets::send_quest_complete_packet(
                                &self.gate_ref,
                                msg.session_id,
                                quest_db.index,
                            );
                            send_system_message(
                                &self.gate_ref,
                                msg.session_id,
                                &format!("任务完成: +{}经验, +{}金币", quest_db.exp_reward, gold),
                            );
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
        ShopItem {
            item_index: 1,
            gold_price: 5000,
            credit_price: 100,
            count: 1,
            class: 255,
            category: "消耗品",
            stock: 999,
        },
        // 回城卷
        ShopItem {
            item_index: 2,
            gold_price: 1000,
            credit_price: 20,
            count: 1,
            class: 255,
            category: "消耗品",
            stock: 999,
        },
        // 随机传送卷
        ShopItem {
            item_index: 3,
            gold_price: 2000,
            credit_price: 40,
            count: 1,
            class: 255,
            category: "消耗品",
            stock: 999,
        },
        // 双倍经验卷
        ShopItem {
            item_index: 4,
            gold_price: 10000,
            credit_price: 200,
            count: 1,
            class: 255,
            category: "消耗品",
            stock: 999,
        },
        // 经验丹x10
        ShopItem {
            item_index: 5,
            gold_price: 40000,
            credit_price: 800,
            count: 10,
            class: 255,
            category: "消耗品",
            stock: 999,
        },
    ]
}

/// 发送游戏商店目录给玩家（登录 GetGameShop 与请求目录共用；C# S.GameShopInfo）
pub(crate) fn send_game_shop_catalog(
    gate_ref: &ActorRef<GateActor>,
    session_id: u64,
    gold: u32,
    shop_items: &[db::GameShopItem],
) {
    use mir2_shared::packets::server::special_systems::{GameShopInfo, GameShopItem as ProtoItem};

    let items: Vec<ProtoItem> = if shop_items.is_empty() {
        // Fallback to hardcoded
        game_shop_catalog_fallback()
            .iter()
            .map(|s| ProtoItem {
                item_index: s.item_index,
                gold_price: s.gold_price,
                credit_price: s.credit_price,
                count: s.count,
                class: s.class,
                category: s.category.to_string(),
                stock: s.stock,
                is_bought: false,
                deal: false,
            })
            .collect()
    } else {
        shop_items
            .iter()
            .map(|s| ProtoItem {
                item_index: s.item_index,
                gold_price: s.gold_price,
                credit_price: s.credit_price,
                count: s.count as i32,
                class: 255, // DB class_name is string; use default
                category: s.category.clone(),
                stock: s.stock,
                is_bought: false,
                deal: s.deal,
            })
            .collect()
    };

    let packet = GameShopInfo {
        items,
        credit: 0,
        gold,
    };

    let mut body = Vec::new();
    let _ = packet.write_body(&mut body);
    let _ = gate_ref
        .tell(SendToClient {
            session_id,
            data: build_packet_bytes(
                mir2_shared::enums::ServerPacketIds::GameShopInfo as i16,
                &body,
            ),
        })
        .try_send();
}

pub struct GameshopBuyRequest {
    pub session_id: u64,
    pub item_id: u32,
    pub count: u32,
    /// #2566：C# C.GameshopBuy.PType——0=Credit（账号信用点）/ 1=Gold（金币）
    pub p_type: i32,
}

/// #2566：C# GameshopBuy 货币分支（PlayerObject.cs:13815-13833）：
/// PType 0=Credit（CreditPrice×Quantity，需 CanBuyCredit）/ 1=Gold（GoldPrice×Quantity，需 CanBuyGold）；
/// 其余 PType 或货币未开放 → None（C# ReceiveChat 后 return）
/// 返回 (is_gold, 总花费)
pub(crate) fn gameshop_currency_cost(
    p_type: i32,
    can_buy_credit: bool,
    can_buy_gold: bool,
    credit_price: u64,
    gold_price: u64,
    quantity: u64,
) -> Option<(bool, u64)> {
    match p_type {
        0 if can_buy_credit => Some((false, credit_price.saturating_mul(quantity))),
        1 if can_buy_gold => Some((true, gold_price.saturating_mul(quantity))),
        _ => None,
    }
}

/// #2566：C# GSpurchases 限购（PlayerObject.cs:13751-13768）——Stock!=0 时需
/// Stock - purchased - Quantity >= 0；Stock==0 视为不限量
pub(crate) fn gameshop_stock_available(stock: i32, purchased: i64, quantity: i64) -> bool {
    stock == 0 || (stock as i64 - purchased - quantity) >= 0
}

impl Message<GameshopBuyRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: GameshopBuyRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // item_id=0 请求商店目录
        if msg.item_id == 0 {
            debug!("GameShop: {} requesting catalog", state.name);
            send_game_shop_catalog(
                &self.gate_ref,
                msg.session_id,
                state.inventory.gold as u32,
                &self.game_shop_items,
            );
            return;
        }

        // #2566：C# Quantity 范围 1..=99（PlayerObject.cs:13720，越界直接丢弃）
        if !(1..=99).contains(&msg.count) {
            return;
        }

        // 查找商品（优先 DB，fallback 硬编码）
        let db_item = self
            .game_shop_items
            .iter()
            .find(|i| i.item_index as u32 == msg.item_id);
        let fallback = game_shop_catalog_fallback()
            .iter()
            .find(|i| i.item_index as u32 == msg.item_id);
        // #2566：统一商品视图（双货币价格 + 限购/购买开关；fallback 目录无开关数据，视为双币均可）
        let (
            item_index,
            gindex,
            credit_price,
            gold_price,
            item_count,
            stock,
            can_buy_credit,
            can_buy_gold,
        ) = if let Some(di) = db_item {
            (
                di.item_index,
                di.gindex,
                di.credit_price as u64,
                di.gold_price as u64,
                di.count as u32,
                di.stock,
                di.can_buy_credit,
                di.can_buy_gold,
            )
        } else if let Some(fi) = fallback {
            (
                fi.item_index,
                fi.item_index,
                fi.credit_price as u64,
                fi.gold_price as u64,
                fi.count as u32,
                fi.stock,
                true,
                true,
            )
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "商品不存在");
            return;
        };

        let buy_count = msg.count.max(1).min(item_count);

        // #2566：C# 邮件容量上限——(Quantity×Count)/StackSize > 5 直接丢弃（PlayerObject.cs:13748）
        let stack_size = self
            .item_infos
            .get(&item_index)
            .map(|i| i.stack_size.max(1) as u64)
            .unwrap_or(1);
        if (buy_count as u64 * item_count as u64) / stack_size > 5 {
            return;
        }

        // #2566：每账号限购（C# GSpurchases；Stock==0 不限）——超量拒绝
        let username = record.account_username.clone();
        let mut purchased = 0i64;
        if stock != 0 {
            purchased = db::get_gameshop_purchases(&self.db_pool, &username, gindex)
                .await
                .unwrap_or(0);
            if !gameshop_stock_available(stock, purchased, buy_count as i64) {
                send_system_message(&self.gate_ref, msg.session_id, "购买数量超过限购余量");
                return;
            }
        }

        // #2566：按 PType 分支货币（0=Credit 账号信用点 / 1=Gold 金币；非法值拒绝）
        let Some((is_gold, total_cost)) = gameshop_currency_cost(
            msg.p_type,
            can_buy_credit,
            can_buy_gold,
            credit_price,
            gold_price,
            buy_count as u64,
        ) else {
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                "货币类型无效或该商品不支持此货币购买",
            );
            return;
        };

        debug!(
            "GameshopBuy: {} item={} count={} p_type={} cost={}",
            state.name, msg.item_id, buy_count, msg.p_type, total_cost
        );

        // 检查余额（C#：cost <= Account.Credit / goldcost <= Account.Gold）
        if is_gold {
            if state.inventory.gold < total_cost {
                send_system_message(&self.gate_ref, msg.session_id, "金币不足");
                return;
            }
        } else {
            let credit = db::get_account_credit(&self.db_pool, &username)
                .await
                .unwrap_or(0);
            if credit < total_cost {
                send_system_message(&self.gate_ref, msg.session_id, "信用点不足");
                return;
            }
        }

        // 先构建邮件（在扣费前，避免扣款后交付失败导致玩家损失）

        let mail_items: Vec<mir2_shared::data::item::UserItem> =
            if let Some(item_db) = self.item_infos.get(&item_index) {
                (0..buy_count)
                    .map(|_| {
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
                    })
                    .collect()
            } else {
                (0..buy_count)
                    .map(|_| mir2_shared::data::item::UserItem {
                        unique_id: generate_item_uid(),
                        item_index,
                        ..Default::default()
                    })
                    .collect()
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

        // 扣费：金币走背包；信用点原子扣账号（C# Account.Credit -= CreditCost）
        if is_gold {
            let _ = record
                .actor_ref
                .ask(DeductGold { amount: total_cost })
                .await;
        } else {
            match db::try_deduct_account_credit(&self.db_pool, &username, total_cost as i64).await {
                Ok(true) => {}
                Ok(false) => {
                    send_system_message(&self.gate_ref, msg.session_id, "信用点不足");
                    return;
                }
                Err(e) => {
                    warn!("GameshopBuy credit deduct failed for {}: {}", username, e);
                    send_system_message(&self.gate_ref, msg.session_id, "信用点扣除失败");
                    return;
                }
            }
            // C# S.LoseCredit（信用点扣除回包，PlayerObject.cs:13835）
            let packet = mir2_shared::packets::server::drops::LoseCredit {
                credit: total_cost as u32,
            };
            let mut body = Vec::new();
            if packet.write_body(&mut body).is_ok() {
                let _ = self
                    .gate_ref
                    .tell(SendToClient {
                        session_id: msg.session_id,
                        data: build_packet_bytes(
                            mir2_shared::enums::ServerPacketIds::LoseCredit as i16,
                            &body,
                        ),
                    })
                    .await;
            }
        }

        // #2566：累加每账号限购计数（C# GSpurchases[GIndex] += Quantity，仅限量商品）
        if stock != 0 {
            if let Err(e) =
                db::add_gameshop_purchases(&self.db_pool, &username, gindex, buy_count as i64).await
            {
                warn!(
                    "GameshopBuy purchase counter update failed for {}: {}",
                    username, e
                );
            }
        }

        // 发送邮件
        send_mail_received_packet(&self.gate_ref, msg.session_id, &mail);
        let _ = record
            .actor_ref
            .ask(crate::actors::player::AddMail { mail })
            .await;

        send_system_message(
            &self.gate_ref,
            msg.session_id,
            &format!(
                "购买成功！已扣除{} {}，物品已通过邮件发送",
                if is_gold { "金币" } else { "信用点" },
                total_cost
            ),
        );

        // 发送库存更新（#2566：限量商品余量 = Stock - 已购 - 本次）
        let stock_remaining = if stock != 0 {
            (stock as i64 - purchased - buy_count as i64).max(0) as u32
        } else {
            item_count.saturating_sub(buy_count)
        };
        let _ = self
            .gate_ref
            .tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::GameShopStock as i16,
                    &{
                        let mut body = Vec::new();
                        body.extend_from_slice(&(msg.item_id as i32).to_le_bytes());
                        body.extend_from_slice(&stock_remaining.to_le_bytes());
                        body
                    },
                ),
            })
            .await;
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
                    &self.db_pool,
                    &state.name,
                    msg.issue_type,
                    &msg.description,
                )
                .await;
            }
        }
        send_system_message(
            &self.gate_ref,
            msg.session_id,
            "举报信息已提交，感谢您的反馈",
        );
    }
}

pub struct GetRankingRequest {
    pub session_id: u64,
    pub rank_type: u8,
    pub online_only: bool,
}

impl Message<GetRankingRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: GetRankingRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!(
            "GetRanking: session={} type={}",
            msg.session_id, msg.rank_type
        );

        // #1323：请求者名字（计算 MyRank）
        let requester_name = match self.players.get(&msg.session_id) {
            Some(r) => match r.actor_ref.ask(GetPlayerState).await {
                Ok(Some(s)) => s.name.clone(),
                _ => String::new(),
            },
            None => String::new(),
        };

        // Collect online players
        let mut entries: Vec<(u32, String, u8, i32, i64)> = Vec::new();
        for record in self.players.values() {
            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                entries.push((
                    state.object_id,
                    state.name.clone(),
                    state.class as u8,
                    state.level as i32,
                    state.experience,
                ));
            }
        }
        // Supplement with DB-backed top players for more complete rankings
        // 仅在线：跳过 DB 补全（C# OnlineOnly 只显示当前在线玩家）
        if !msg.online_only {
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
                    if !entries.iter().any(|(_, n, _, _, _)| n == &name) {
                        entries.push((0, name, class, level, experience));
                    }
                }
            }
        }

        // 按等级降序、经验降序排序
        entries.sort_by(|a, b| b.3.cmp(&a.3).then_with(|| b.4.cmp(&a.4)));

        // #1323：请求者自己的排名（C# MyRank；未上榜=0）
        let my_rank = entries
            .iter()
            .position(|(_, n, _, _, _)| n == &requester_name)
            .map(|i| i as i32 + 1)
            .unwrap_or(0);
        // 取前 20 名
        let rankings: Vec<mir2_shared::packets::server::special_systems::RankInfo> = entries
            .into_iter()
            .take(20)
            .enumerate()
            .map(|(idx, (player_id, name, class, level, experience))| {
                mir2_shared::packets::server::special_systems::RankInfo {
                    rank: (idx + 1) as i32,
                    player_id,
                    player_name: name,
                    class,
                    level,
                    experience,
                }
            })
            .collect();

        let packet = mir2_shared::packets::server::special_systems::Rankings { rankings, my_rank };
        let mut body = Vec::new();
        if packet.write_body(&mut body).is_ok() {
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::Rankings as i16,
                        &body,
                    ),
                })
                .await;
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
                warn!(
                    "RequestMonsterInfo: monster_index={} not found",
                    msg.monster_index
                );
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
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::NewMonsterInfo as i16,
                        &body,
                    ),
                })
                .await;
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
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::NewNPCInfo as i16,
                        &body,
                    ),
                })
                .await;
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
        // C# PlayerObject.RequestItemInfo（:7538-7544）：CheckItemInfo → S.NewItemInfo { Info }；物品不存在不响应
        if !self.item_infos.contains_key(&msg.item_index) {
            debug!(
                "RequestItemInfo: session={} idx={} unknown item",
                msg.session_id, msg.item_index
            );
            return;
        }
        let mut tmp = mir2_shared::data::item::UserItem {
            item_index: msg.item_index,
            ..Default::default()
        };
        enrich_item_info(&mut tmp, &self.item_infos);
        let Some(shared_info) = tmp.info else { return };
        let packet = mir2_shared::packets::server::item::NewItemInfo { info: shared_info };
        let mut body = Vec::new();
        if packet.write_body(&mut body).is_ok() {
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::NewItemInfo as i16,
                        &body,
                    ),
                })
                .await;
        }
        debug!(
            "RequestItemInfo: session={} idx={} sent NewItemInfo",
            msg.session_id, msg.item_index
        );
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
#[cfg(test)]
mod tests {
    use super::*;
    use mir2_shared::data::item::UserItem;
    use mir2_shared::data::stats::Stats;
    use mir2_shared::enums::{ItemType, Stat};
    use std::collections::HashMap;

    fn info(index: i32, cs_type: i32, stats: &[(Stat, i32)]) -> crate::db::ItemInfo {
        crate::db::ItemInfo {
            index,
            item_type: cs_type,
            stats: stats.iter().map(|(s, v)| (*s as u8, *v)).collect(),
            ..Default::default()
        }
    }

    fn gear(index: i32, added: &[(Stat, i32)]) -> UserItem {
        let mut added_stats = Stats::new();
        for (s, v) in added {
            added_stats.set(*s, *v);
        }
        UserItem {
            item_index: index,
            added_stats,
            ..Default::default()
        }
    }

    /// C# 编号：Hook=28 Float=29 Bait=30 Finder=31 Reel=32（DB item_infos.type）
    #[test]
    fn test_rod_only_bonuses() {
        let rod = info(793, 1, &[(Stat::CriticalRate, 5), (Stat::MaxAC, 3)]);
        let infos = HashMap::from([(793, rod.clone())]);
        let b = compute_fishing_gear_bonuses(Some(&rod), &[], &infos);
        assert_eq!(b.flexibility, 5);
        assert_eq!(b.success_stat, 3);
        assert_eq!(b.failed_add_min, 0);
        assert_eq!(b.failed_add_max, 0);
        assert_eq!(b.nibble_min, 0);
        assert_eq!(b.nibble_max, 0);
        assert_eq!(b.auto_reel_chance, 0);
    }

    #[test]
    fn test_hook_bonus_uses_added_and_real_stats() {
        let rod = info(793, 1, &[]);
        let hook = info(795, 28, &[(Stat::CriticalRate, 7)]);
        let infos = HashMap::from([(793, rod.clone()), (795, hook.clone())]);
        // AddedStats[CriticalRate]=2 + realItem.Stats[CriticalRate]=7
        let b = compute_fishing_gear_bonuses(
            Some(&rod),
            &[
                Some(gear(795, &[(Stat::CriticalRate, 2)])),
                None,
                None,
                None,
                None,
            ],
            &infos,
        );
        assert_eq!(b.flexibility, 9);
    }

    #[test]
    fn test_float_bait_finder_reel_bonuses() {
        let rod = info(793, 1, &[(Stat::MaxAC, 3)]);
        let float = info(796, 29, &[(Stat::MinAC, 1), (Stat::MaxAC, 2)]);
        let bait = info(798, 30, &[(Stat::MaxAC, 4)]);
        let finder = info(800, 31, &[(Stat::MinAC, 5), (Stat::MaxAC, 6)]);
        let reel = info(802, 32, &[(Stat::MaxMAC, 7), (Stat::MaxAC, 8)]);
        let infos = HashMap::from([
            (793, rod.clone()),
            (796, float.clone()),
            (798, bait.clone()),
            (800, finder.clone()),
            (802, reel.clone()),
        ]);
        let slots = [
            None,
            Some(gear(796, &[])),
            Some(gear(798, &[])),
            Some(gear(800, &[])),
            Some(gear(802, &[])),
        ];
        let b = compute_fishing_gear_bonuses(Some(&rod), &slots, &infos);
        assert_eq!(b.success_stat, 3 + 4 + 8); // 鱼竿 + Bait + Reel
        assert_eq!(b.nibble_min, 1);
        assert_eq!(b.nibble_max, 2);
        assert_eq!(b.failed_add_min, 5);
        assert_eq!(b.failed_add_max, 6);
        assert_eq!(b.auto_reel_chance, 7);
    }

    #[test]
    fn test_bonuses_clamped_like_cs_byte_sbyte() {
        let rod = info(793, 1, &[]);
        let hook = info(795, 28, &[(Stat::CriticalRate, 300)]);
        let reel = info(802, 32, &[(Stat::MaxMAC, 300), (Stat::MaxAC, 300)]);
        let infos = HashMap::from([(793, rod.clone()), (795, hook.clone()), (802, reel.clone())]);
        let slots = [
            Some(gear(795, &[(Stat::CriticalRate, 300)])),
            None,
            None,
            None,
            Some(gear(802, &[])),
        ];
        let b = compute_fishing_gear_bonuses(Some(&rod), &slots, &infos);
        assert_eq!(b.flexibility, 255); // byte
        assert_eq!(b.success_stat, 127); // sbyte
        assert_eq!(b.auto_reel_chance, 127); // sbyte
    }

    /// #2368：引擎级特殊页 key → 面板动作（C# ProcessSpecial 商店类）
    #[test]
    fn engine_npc_action_matches_csharp_process_special() {
        use super::EngineNpcAction as A;
        assert_eq!(engine_npc_action("[@BUY]"), Some(A::Goods));
        assert_eq!(engine_npc_action("[@BUYNEW]"), Some(A::Goods));
        assert_eq!(engine_npc_action("[@BUYSELL]"), Some(A::GoodsAndSell));
        assert_eq!(engine_npc_action("[@BUYSELLNEW]"), Some(A::GoodsAndSell));
        assert_eq!(engine_npc_action("[@SELL]"), Some(A::Sell));
        assert_eq!(engine_npc_action("[@REPAIR]"), Some(A::Repair));
        assert_eq!(engine_npc_action("[@SREPAIR]"), Some(A::SpecialRepair));
        assert_eq!(engine_npc_action("[@BUYUSED]"), Some(A::BuySub));
        assert_eq!(engine_npc_action("[@STORAGE]"), Some(A::Storage));
        assert_eq!(engine_npc_action("[@CRAFT]"), Some(A::Craft));
        assert_eq!(engine_npc_action("[@REFINE]"), Some(A::Refine));
        assert_eq!(engine_npc_action("[@REFINECHECK]"), Some(A::CheckRefine));
        assert_eq!(
            engine_npc_action("[@REFINECOLLECT]"),
            Some(A::RefineCollect)
        );
        assert_eq!(engine_npc_action("[@MANAGEHERO]"), Some(A::ManageHero));
        assert_eq!(
            engine_npc_action("[@REPLACEWEDDINGRING]"),
            Some(A::ReplaceWedRing)
        );
        assert_eq!(
            engine_npc_action("[@GUILDTERRITORY]"),
            Some(A::GuildTerritory)
        );
        assert_eq!(engine_npc_action("[@CREATEGUILD]"), Some(A::CreateGuild));
        assert_eq!(engine_npc_action("[@REQUESTWAR]"), Some(A::RequestWar));
        assert_eq!(engine_npc_action("[@SENDPARCEL]"), Some(A::SendParcel));
        assert_eq!(
            engine_npc_action("[@COLLECTPARCEL]"),
            Some(A::CollectParcel)
        );
        assert_eq!(engine_npc_action("[@CONSIGN]"), Some(A::Consign));
        // 大小写不敏感
        assert_eq!(engine_npc_action("[@buysell]"), Some(A::GoodsAndSell));
        assert_eq!(engine_npc_action("[@craft]"), Some(A::Craft));
        // 普通页不拦截
        assert_eq!(engine_npc_action("[@MAIN]"), None);
        assert_eq!(engine_npc_action("[@MARKET]"), None); // 客户端按钮直接开市场
    }

    #[test]
    fn test_fishing_chance_failed_add_applied_only_with_counter() {
        // 无失败计数：不加 Finder 补偿
        // 有失败计数：+ Random(failedAddMin, failedAddMax)，min==max 时确定返回 min
        let c0 = compute_fishing_chance(10, 20, 5, 0, 10, 9, 9);
        assert_eq!(c0, 35); // 10 + 20 + 5 + 0
        let c1 = compute_fishing_chance(10, 20, 5, 1, 10, 9, 9);
        assert_eq!(c1, 10 + 20 + 5 + 9 + 1 * 10);
        // 钳制 0..=100
        let c2 = compute_fishing_chance(10, 200, 5, 0, 10, 0, 0);
        assert_eq!(c2, 100);
    }

    #[test]
    fn test_cs_random_next_semantics() {
        assert_eq!(cs_random_next(0, 0), 0); // C# Next(0,0) 返回 0
        assert_eq!(cs_random_next(9, 9), 9);
        assert_eq!(cs_random_next(5, 3), 5); // 非法区间保守返回 min，不 panic
        for _ in 0..100 {
            let v = cs_random_next(0, 10);
            assert!((0..10).contains(&v));
        }
    }

    #[test]
    fn test_missing_gear_info_skipped() {
        let rod = info(793, 1, &[(Stat::CriticalRate, 5)]);
        let infos = HashMap::from([(793, rod.clone())]);
        // 槽内有物品但 item_infos 无记录 → 跳过不崩溃
        let b = compute_fishing_gear_bonuses(Some(&rod), &[Some(gear(9999, &[]))], &infos);
        assert_eq!(b.flexibility, 5);
    }

    #[test]
    fn test_shared_item_type_mapping() {
        assert_eq!(shared_item_type(28), ItemType::Hook);
        assert_eq!(shared_item_type(29), ItemType::Float);
        assert_eq!(shared_item_type(30), ItemType::Bait);
        assert_eq!(shared_item_type(31), ItemType::Finder);
        assert_eq!(shared_item_type(32), ItemType::Reel);
    }

    /// #2566：C# GameshopBuy 双货币分支（PType 0=Credit/1=Gold，PlayerObject.cs:13815-13833）
    #[test]
    fn gameshop_currency_cost_branches_by_ptype() {
        // PType=0：CreditPrice×Quantity（需 CanBuyCredit）
        assert_eq!(
            gameshop_currency_cost(0, true, true, 100, 5_000, 3),
            Some((false, 300))
        );
        // PType=1：GoldPrice×Quantity（需 CanBuyGold）
        assert_eq!(
            gameshop_currency_cost(1, true, true, 100, 5_000, 3),
            Some((true, 15_000))
        );
        // 货币未开放 → None（C# YouDontHaveEnoughCurrency）
        assert_eq!(gameshop_currency_cost(0, false, true, 100, 5_000, 3), None);
        assert_eq!(gameshop_currency_cost(1, true, false, 100, 5_000, 3), None);
        // 非法 PType → None
        assert_eq!(gameshop_currency_cost(2, true, true, 100, 5_000, 3), None);
        assert_eq!(gameshop_currency_cost(-1, true, true, 100, 5_000, 3), None);
        // 饱和乘法不溢出
        assert_eq!(
            gameshop_currency_cost(1, true, true, 0, u64::MAX, 2),
            Some((true, u64::MAX))
        );
    }

    /// #2566：C# GSpurchases 限购（Stock!=0 时 Stock - purchased - Quantity >= 0）
    #[test]
    fn gameshop_stock_available_matches_csharp() {
        // Stock==0 → 不限量
        assert!(gameshop_stock_available(0, 1_000, 99));
        // 恰好买满 → 允许
        assert!(gameshop_stock_available(10, 5, 5));
        // 超出余量 → 拒绝
        assert!(!gameshop_stock_available(10, 5, 6));
        assert!(!gameshop_stock_available(10, 11, 1));
        // 未购买过 → 全额可用
        assert!(gameshop_stock_available(10, 0, 10));
    }
}
