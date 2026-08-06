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
/// 客户端删除物品（C# C.DeleteItem）
pub struct DeleteItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
    pub count: u16,
    pub hero: bool,
}

impl Message<DeleteItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: DeleteItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };
        let _ = record.actor_ref.ask(crate::actors::player::DeleteItemFromInventory {
            unique_id: msg.unique_id,
            count: msg.count,
            hero: msg.hero,
        }).await;
    }
}

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

/// 购买物品（NPC 商店；npc 由 session_npc 会话上下文解析）
pub struct BuyItemRequest {
    pub session_id: u64,
    pub item_index: u64,
    pub count: u32,
}

/// 出售物品（NPC 商店）
pub struct SellItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
    pub count: u32,
}

// ============================================================
// 修理系统 Handler
// ============================================================

/// 修理费用：每缺失 1 点耐久 = 1 金币
/// 修理物品请求
pub struct RepairItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
    /// 特殊修理（C# SRepairItem：费用 ×3，走 SRepairKey NPC）
    pub special: bool,
}

/// 物品单价（对齐 C# Shared/Data/ItemData.cs Price() 去掉 *Count 部分）：
/// p = floor(p/2 + (p/2)*(CurrentDura/MaxDura) + Price/2)（Durability>0 时），
/// p *= AddedStats.Count*0.1 + 1
fn compute_item_price_per_unit(item: &mir2_shared::data::item::UserItem, info: &db::ItemInfo) -> u64 {
    let mut p = info.price as f64;
    if info.durability > 0 {
        let r = (info.price as f64 / 2.0) / info.durability as f64;
        let max_dura = item.max_dura as f64;
        let p_base = max_dura * r;
        let ratio = if item.max_dura > 0 { item.current_dura as f64 / max_dura } else { 0.0 };
        p = (p_base / 2.0 + (p_base / 2.0) * ratio + info.price as f64 / 2.0).floor();
    }
    p *= item.added_stats.len() as f64 * 0.1 + 1.0;
    p as u64
}

/// 计算修理费（对齐 C# Shared/Data/ItemData.cs RepairPrice()）
/// p = floor(MaxDura * (Price/2 / Durability) + Price/2) * (AddedStats.Count*0.1 + 1)
/// cost = p * Count - Price；有租赁信息 ×2；特殊修理 ×3
fn compute_repair_cost(item: &mir2_shared::data::item::UserItem, info: &db::ItemInfo, special: bool) -> u64 {
    let durability = info.durability;
    if durability <= 0 {
        return 0;
    }
    let price = info.price as f64;
    let max_dura = item.max_dura as f64;
    let added_count = item.added_stats.len() as f64;
    let p_float = (max_dura * (price / 2.0 / durability as f64) + price / 2.0).floor()
        * (added_count * 0.1 + 1.0);
    let p = p_float as u64;
    let mut cost = p.saturating_mul(item.count as u64).saturating_sub(info.price as u64);
    if item.rental_information.is_some() {
        cost = cost.saturating_mul(2);
    }
    if special {
        cost = cost.saturating_mul(3);
    }
    cost
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

/// 存入仓库（C# StoreItem{From=背包格, To=仓库格}）
pub struct StoreItemRequest {
    pub session_id: u64,
    pub from: i32,
    pub to: i32,
}

/// 从仓库取出（C# TakeBackItem{From=仓库格, To=背包格}）
pub struct TakeBackItemRequest {
    pub session_id: u64,
    pub from: i32,
    pub to: i32,
}

/// 合成物品请求
pub struct CraftItemRequest {
    pub session_id: u64,
    pub recipe_id: u32,
}

/// 回购物品请求（从 NPC 回购最近卖出的物品）
pub struct BuyItemBackRequest {
    pub session_id: u64,
    pub unique_id: u64,
    pub count: u32,
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

        // #218：英雄技能书学习（物品在英雄背包）
        if let Some(hero_item) = record
            .actor_ref
            .ask(crate::actors::player::GetHeroItemInfo {
                unique_id: msg.unique_id,
            })
            .await
            .unwrap_or(None)
        {
            if let Some(db) = self.item_infos.get(&hero_item.item_index) {
                if db.item_type == 20 { // Book（C# ItemType.Book=20）
                    let _ = record
                        .actor_ref
                        .ask(crate::actors::player::ConsumeHeroItem {
                            unique_id: msg.unique_id,
                        })
                        .await;
                    let spell_cs = db.shape;
                    if self.magic_infos.contains_key(&(spell_cs as u32)) {
                        let learned = record
                            .actor_ref
                            .ask(crate::actors::player::IsHeroMagicLearned { spell: spell_cs })
                            .await
                            .unwrap_or(false);
                        if learned {
                            send_system_message(
                                &self.gate_ref,
                                msg.session_id,
                                "英雄已经学会这个技能",
                            );
                        } else {
                            let ok = record
                                .actor_ref
                                .ask(crate::actors::player::LearnHeroMagic { spell: spell_cs })
                                .await
                                .unwrap_or(false);
                            if ok {
                                if let Some(info) = self.magic_infos.get(&(spell_cs as u32)) {
                                    let pm = crate::actors::player::PlayerMagic::new(spell_cs);
                                    let cm = super::build_client_magic(info, &pm);
                                    let new_magic = mir2_shared::packets::server::magic::NewMagic {
                                        magic: cm,
                                        hero: true,
                                    };
                                    let mut body = Vec::new();
                                    if new_magic.write_body(&mut body).is_ok() {
                                        let _ = self
                                            .gate_ref
                                            .tell(SendToClient {
                                                session_id: msg.session_id,
                                                data: build_packet_bytes(
                                                    mir2_shared::enums::ServerPacketIds::NewMagic
                                                        as i16,
                                                    &body,
                                                ),
                                            })
                                            .await;
                                    }
                                }
                                send_system_message(
                                    &self.gate_ref,
                                    msg.session_id,
                                    "英雄学会了技能！",
                                );
                                tracing::info!(
                                    "🦸 {} 英雄学会技能 spell={}", player_state.name, spell_cs
                                );
                            }
                        }
                    } else {
                        send_system_message(&self.gate_ref, msg.session_id, "这本技能书无法使用");
                    }
                }
            }
            return;
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

        // C# UseItem：NeedIdentify 且未鉴定 → 自动鉴定（PlayerObject.cs:4960）
        if item_db.as_ref().map(|i| !i.is_identified()).unwrap_or(false) {
            let _ = record.actor_ref.ask(crate::actors::player::SetItemIdentified {
                unique_id: msg.unique_id,
            }).await;
        }

        // C# UseItem：仅可处理类型才消耗（Potion=13/Scroll=17/Book=20/Food=27/彩票=Scroll shape 12）；
        // 未处理类型不消耗（C# PlayerObject.cs UseItem default: return;）
        // 注意：DB item_type 为 C# 原始值（SharedRust 枚举 +3，不可用于 DB 比较）
        let item_type = item_db.as_ref().map(|i| i.item_type).unwrap_or(-1);
        let item_shape = item_db.as_ref().map(|i| i.shape).unwrap_or(-1);
        let usable = item_type == 13 // Potion
            || item_type == 17 // Scroll
            || item_type == 20 // Book
            || item_type == 27 // Food
            || (item_type == 17 && item_shape == 12); // LotteryTicket（C# Scroll shape 12）
        if !usable {
            send_system_message(&self.gate_ref, msg.session_id, "该物品无法使用");
            send_use_item_response(&self.gate_ref, msg.session_id, msg.unique_id);
            return;
        }

        debug!("Player session={} used item uid={} index={}", msg.session_id, msg.unique_id, item_index);

        // C#：经验药水为 Potion shape 4（EXP Buff），另行实现；此处不再按 item_index 特判

        // 根据物品类型执行效果
        if let Some(ref db) = item_db {
            match db.item_type {
                // Potion（C# UseItem：Shape 0/1 回血回蓝；Shape 3 临时属性 Buff）
                13 => {
                    use mir2_shared::enums::Stat;
                    use crate::combat::buff::{BuffType, BuffInstance};
                    let shape = db.shape;
                    let get = |stat: Stat| db.stats.get(&(stat as u8)).copied().unwrap_or(0);
                    if shape == 3 {
                        // C#：Buff 药水，时长 = Durability * Settings.Minute（60000ms → 600 ticks）
                        let ticks = (db.durability.max(1) as u32).saturating_mul(600);
                        let mut applied = false;
                        let apply = |bt: BuffType| async move {
                            let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff {
                                buff: BuffInstance::new(bt, ticks, 1),
                            }).await;
                        };
                        if get(Stat::MaxDC) > 0 || get(Stat::MinDC) > 0 {
                            apply(BuffType::AttackBoost { bonus: get(Stat::MaxDC).max(get(Stat::MinDC)) }).await;
                            applied = true;
                        }
                        if get(Stat::MaxMC) > 0 || get(Stat::MinMC) > 0 {
                            apply(BuffType::McBoost { bonus: get(Stat::MaxMC).max(get(Stat::MinMC)) }).await;
                            applied = true;
                        }
                        if get(Stat::MaxSC) > 0 || get(Stat::MinSC) > 0 {
                            apply(BuffType::ScBoost { bonus: get(Stat::MaxSC).max(get(Stat::MinSC)) }).await;
                            applied = true;
                        }
                        if get(Stat::AttackSpeed) > 0 {
                            apply(BuffType::AttackSpeedBoost { percent: get(Stat::AttackSpeed) }).await;
                            applied = true;
                        }
                        if get(Stat::HP) > 0 {
                            apply(BuffType::HpRegen { amount_per_tick: get(Stat::HP) }).await;
                            applied = true;
                        }
                        if get(Stat::MP) > 0 {
                            apply(BuffType::MpRegen { amount_per_tick: get(Stat::MP) }).await;
                            applied = true;
                        }
                        if applied {
                            debug!("Potion: {} shape=3 buff potion {} ticks", player_state.name, ticks);
                        }
                    } else {
                        let hp_recover = get(Stat::HP);
                        let mp_recover = get(Stat::MP);
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
                }
                // Scroll（C# UseItem Scroll：按 item.Info.Shape 分支，shape=0~6/12）
                17 => {
                    let shape = db.shape;
                    match shape {
                        // 0 DungeonEscape（C# TeleportEscape(20)：传回绑定点±100）
                        // 1 TownTeleport（C# Teleport(BindMap, BindLocation)）
                        // Rust 暂无绑定点系统，回退到当前地图安全区中心
                        0 | 1 => {
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
                            send_system_message(&self.gate_ref, msg.session_id,
                                if shape == 0 { "已脱离迷宫，返回安全区" } else { "已返回安全区" });
                            debug!("Scroll: {} shape={} teleported to safe zone ({}, {})", player_state.name, shape, tx, ty);
                        }
                        // 2 RandomTeleport（C# TeleportRandom(200, Durability)：随机可行走格）
                        2 => {
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
                        // 3 BenedictionOil（C# TryLuckWeapon：武器幸运赌博）
                        3 => {
                            use mir2_shared::enums::Stat;
                            let weapon = record.actor_ref.ask(crate::actors::player::GetEquipmentInfo {
                                slot: crate::actors::inventory::EquipmentSlot::Weapon,
                            }).await.unwrap_or(None);
                            let Some(weapon) = weapon else {
                                send_system_message(&self.gate_ref, msg.session_id, "没有装备武器");
                                return;
                            };
                            let luck = weapon.added_stats.get(Stat::Luck);
                            if luck >= 7 {
                                send_system_message(&self.gate_ref, msg.session_id, "武器幸运已达上限");
                                return;
                            }
                            // C# BindMode.DontUpgrade = 0x40（绑定禁止升级）
                            let dont_upgrade = self.item_infos.get(&weapon.item_index)
                                .map(|i| (i.bind_mode & 0x40) != 0).unwrap_or(false);
                            if dont_upgrade {
                                send_system_message(&self.gate_ref, msg.session_id, "该武器无法使用祝福油");
                                return;
                            }
                            // C#：20% 诅咒（Luck > -MaxLuck 且 random(20)==0）；否则 Luck<=0 或 random(10*Luck)==0 时 +1
                            let delta = if luck > -7 && fastrand::i32(..20) == 0 {
                                -1
                            } else if luck <= 0 || fastrand::i32(..(10 * luck.max(1))) == 0 {
                                1
                            } else {
                                0
                            };
                            if delta != 0 {
                                let _ = record.actor_ref.ask(crate::actors::player::AddWeaponLuck { delta }).await;
                                send_system_message(&self.gate_ref, msg.session_id,
                                    if delta > 0 { "武器幸运提升！" } else { "武器受到诅咒，幸运下降！" });
                            } else {
                                send_system_message(&self.gate_ref, msg.session_id, "武器没有变化");
                            }
                        }
                        // 4 RepairOil（C#：武器部分修理，MaxDura 少量下降）
                        4 => {
                            let weapon = record.actor_ref.ask(crate::actors::player::GetEquipmentInfo {
                                slot: crate::actors::inventory::EquipmentSlot::Weapon,
                            }).await.unwrap_or(None);
                            let Some(weapon) = weapon else {
                                send_system_message(&self.gate_ref, msg.session_id, "没有装备武器");
                                return;
                            };
                            if weapon.current_dura >= weapon.max_dura {
                                send_system_message(&self.gate_ref, msg.session_id, "武器无需修理");
                                return;
                            }
                            // C# BindMode.DontRepair = 0x20
                            let dont_repair = self.item_infos.get(&weapon.item_index)
                                .map(|i| (i.bind_mode & 0x20) != 0).unwrap_or(false);
                            if dont_repair {
                                send_system_message(&self.gate_ref, msg.session_id, "该武器无法修理");
                                return;
                            }
                            let repaired = record.actor_ref.ask(crate::actors::player::RepairWeapon { full: false }).await.unwrap_or(None);
                            if let Some((uid, max_dura, cur_dura)) = repaired {
                                let packet = mir2_shared::packets::server::item::ItemRepaired { unique_id: uid, max_dura, current_dura: cur_dura };
                                let mut body = Vec::new();
                                if packet.write_body(&mut body).is_ok() {
                                    let _ = self.gate_ref.tell(SendToClient {
                                        session_id: msg.session_id,
                                        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ItemRepaired as i16, &body),
                                    }).await;
                                }
                                send_system_message(&self.gate_ref, msg.session_id, "武器已部分修复");
                            }
                        }
                        // 5 WarGodOil（C#：武器完全修理，禁止 DontRepair/NoSRepair）
                        5 => {
                            let weapon = record.actor_ref.ask(crate::actors::player::GetEquipmentInfo {
                                slot: crate::actors::inventory::EquipmentSlot::Weapon,
                            }).await.unwrap_or(None);
                            let Some(weapon) = weapon else {
                                send_system_message(&self.gate_ref, msg.session_id, "没有装备武器");
                                return;
                            };
                            if weapon.current_dura >= weapon.max_dura {
                                send_system_message(&self.gate_ref, msg.session_id, "武器无需修理");
                                return;
                            }
                            // C# BindMode.DontRepair = 0x20 / NoSRepair = 0x400
                            let no_repair = self.item_infos.get(&weapon.item_index)
                                .map(|i| (i.bind_mode & (0x20 | 0x400)) != 0).unwrap_or(false);
                            if no_repair {
                                send_system_message(&self.gate_ref, msg.session_id, "该武器无法修理");
                                return;
                            }
                            let repaired = record.actor_ref.ask(crate::actors::player::RepairWeapon { full: true }).await.unwrap_or(None);
                            if let Some((uid, max_dura, cur_dura)) = repaired {
                                let packet = mir2_shared::packets::server::item::ItemRepaired { unique_id: uid, max_dura, current_dura: cur_dura };
                                let mut body = Vec::new();
                                if packet.write_body(&mut body).is_ok() {
                                    let _ = self.gate_ref.tell(SendToClient {
                                        session_id: msg.session_id,
                                        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ItemRepaired as i16, &body),
                                    }).await;
                                }
                                send_system_message(&self.gate_ref, msg.session_id, "武器已完全修复");
                            }
                        }
                        // 6 ResurrectionScroll（C#：NoReincarnation 地图禁用；死亡时 MP/HP 回满复活）
                        6 => {
                            if let Some(mi) = self.map_infos.get(&(player_state.map_index as i32)) {
                                if mi.no_reincarnation {
                                    send_system_message(&self.gate_ref, msg.session_id, "该地图无法使用复活卷");
                                    return;
                                }
                            }
                            if player_state.is_dead {
                                let _ = record.actor_ref.ask(crate::actors::player::Revive).await;
                                send_system_message(&self.gate_ref, msg.session_id, "你已复活！");
                                debug!("ResurrectionScroll: {} revived", player_state.name);
                            }
                        }
                        // 12 LotteryTicket（C# Scroll shape 12：按 Effect 概率中奖）
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
                        _ => {
                            send_system_message(&self.gate_ref, msg.session_id, "该卷轴无法使用");
                            return;
                        }
                    }
                }
                // Food（喂坐骑：恢复坐骑耐久 + S.ItemRepaired，C# UseItem Food）
                t if t == 27 => { // Food（C# ItemType.Food=27；喂坐骑恢复耐久 + S.ItemRepaired）
                    let fed = record.actor_ref.ask(crate::actors::player::FeedMount {
                        amount: db.durability as u16,
                    }).await.unwrap_or(None);
                    if let Some((uid, max_dura, cur_dura)) = fed {
                        let packet = mir2_shared::packets::server::item::ItemRepaired {
                            unique_id: uid,
                            max_dura,
                            current_dura: cur_dura,
                        };
                        let mut body = Vec::new();
                        if packet.write_body(&mut body).is_ok() {
                            let _ = self.gate_ref.tell(SendToClient {
                                session_id: msg.session_id,
                                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ItemRepaired as i16, &body),
                            }).await;
                        }
                        send_system_message(&self.gate_ref, msg.session_id, "坐骑吃饱了！");
                        debug!("FeedMount: {} fed mount (uid={} dura={}/{})", player_state.name, uid, cur_dura, max_dura);
                    } else {
                        send_system_message(&self.gate_ref, msg.session_id, "没有可喂养的坐骑或坐骑已满");
                    }
                }
                // Book（技能书，#212：C# UseItem Book → magic = (Spell)item.Info.Shape）
                t if t == 20 => { // Book（C# ItemType.Book=20；SharedRust 枚举 +3 不可用）
                    let spell_cs = db.shape;
                    if self.magic_infos.contains_key(&(spell_cs as u32)) {
                        let learned = record
                            .actor_ref
                            .ask(crate::actors::player::IsMagicLearned { spell: spell_cs })
                            .await
                            .unwrap_or(false);
                        if learned {
                            send_system_message(
                                &self.gate_ref,
                                msg.session_id,
                                "你已经学会这个技能",
                            );
                        } else {
                            let ok = record
                                .actor_ref
                                .ask(crate::actors::player::LearnMagic { spell: spell_cs })
                                .await
                                .unwrap_or(false);
                            if ok {
                                if let Some(info) = self.magic_infos.get(&(spell_cs as u32)) {
                                    let pm = crate::actors::player::PlayerMagic::new(spell_cs);
                                    let client_magic = super::build_client_magic(info, &pm);
                                    let new_magic = mir2_shared::packets::server::magic::NewMagic {
                                        magic: client_magic,
                                        hero: false,
                                    };
                                    let mut body = Vec::new();
                                    if new_magic.write_body(&mut body).is_ok() {
                                        let _ = self
                                            .gate_ref
                                            .tell(SendToClient {
                                                session_id: msg.session_id,
                                                data: build_packet_bytes(
                                                    mir2_shared::enums::ServerPacketIds::NewMagic
                                                        as i16,
                                                    &body,
                                                ),
                                            })
                                            .await;
                                    }
                                }
                                send_system_message(
                                    &self.gate_ref,
                                    msg.session_id,
                                    "你学会了技能！",
                                );
                                tracing::info!(
                                    "📖 {} 学会技能 spell={}",
                                    player_state.name,
                                    spell_cs
                                );
                            }
                        }
                    } else {
                        send_system_message(&self.gate_ref, msg.session_id, "这本技能书无法使用");
                    }
                }
                _ => {}
            }
        }

        // C# UseItem：switch 成功后统一消耗（item.Count>1 ? Count-- : 移除；失败分支已提前 return 不消耗）
        let consumed = record.actor_ref.ask(ConsumeItem { unique_id: msg.unique_id }).await.unwrap_or(false);
        if !consumed {
            send_system_message(&self.gate_ref, msg.session_id, "使用物品失败");
            return;
        }

        // 发送 UseItem 响应
        send_use_item_response(&self.gate_ref, msg.session_id, msg.unique_id);
    }
}

/// 装备校验（对齐 C# HumanObject.CanEquipItem：槽位类型/性别/职业/RequiredType）
fn can_equip_item(item_info: &db::ItemInfo, slot: crate::actors::inventory::EquipmentSlot, state: &crate::actors::player::PlayerState) -> bool {
    use crate::actors::inventory::EquipmentSlot;
    // C# ItemType 枚举值（DB 落库 C# 原始值）：Weapon=1 Armour=2 Helmet=4 Necklace=5 Bracelet=6
    // Ring=7 Amulet=8 Boots=10 Mount=19（SharedRust 枚举 +3，不可用于 DB 比较）
    let type_ok = match slot {
        EquipmentSlot::Weapon => item_info.item_type == 1,
        EquipmentSlot::Armour => item_info.item_type == 2,
        EquipmentSlot::Helmet => item_info.item_type == 4,
        EquipmentSlot::Necklace => item_info.item_type == 5,
        EquipmentSlot::BraceletL => item_info.item_type == 6,
        EquipmentSlot::BraceletR => item_info.item_type == 6
            || item_info.item_type == 8,
        EquipmentSlot::RingL | EquipmentSlot::RingR => item_info.item_type == 7,
        EquipmentSlot::Shoes => item_info.item_type == 10,
        EquipmentSlot::Pendant => item_info.item_type == 8,
        EquipmentSlot::Mount => item_info.item_type == 19,
        _ => false,
    };
    if !type_ok {
        return false;
    }
    // 性别位标志（C# RequiredGender：Male=1 Female=2）
    let req_gender = item_info.required_gender as u8;
    if req_gender != 0 {
        let gender_bit = match state.gender {
            mir2_shared::enums::MirGender::Male => 0x01,
            mir2_shared::enums::MirGender::Female => 0x02,
        };
        if (req_gender & gender_bit) == 0 {
            return false;
        }
    }
    // 职业位标志（C# RequiredClass：Warrior=1 Wizard=2 Taoist=4 Assassin=8 Archer=16）
    let req_class = item_info.required_class as u8;
    if req_class != 0 {
        let class_bit = match state.class {
            mir2_shared::enums::MirClass::Warrior => 0x01,
            mir2_shared::enums::MirClass::Wizard => 0x02,
            mir2_shared::enums::MirClass::Taoist => 0x04,
            mir2_shared::enums::MirClass::Assassin => 0x08,
            mir2_shared::enums::MirClass::Archer => 0x10,
        };
        if (req_class & class_bit) == 0 {
            return false;
        }
    }
    // RequiredType / RequiredAmount（C# RequiredType：Level/MaxAC/MaxMAC/MaxDC/MaxMC/MaxSC/MaxLevel/Min*）
    let required = item_info.required_type;
    if required != 0 {
        let amount = item_info.required_amount;
        let value = match mir2_shared::enums::RequiredType::try_from(required as u8) {
            Ok(mir2_shared::enums::RequiredType::Level) => state.level as i32,
            Ok(mir2_shared::enums::RequiredType::MaxAc) => state.max_ac,
            Ok(mir2_shared::enums::RequiredType::MaxMac) => state.max_mac,
            Ok(mir2_shared::enums::RequiredType::MaxDc) => state.max_attack,
            Ok(mir2_shared::enums::RequiredType::MaxMc) => state.max_mc,
            Ok(mir2_shared::enums::RequiredType::MaxSc) => state.max_sc,
            Ok(mir2_shared::enums::RequiredType::MaxLevel) => state.level as i32,
            Ok(mir2_shared::enums::RequiredType::MinAc) => state.min_ac,
            Ok(mir2_shared::enums::RequiredType::MinMac) => state.min_mac,
            Ok(mir2_shared::enums::RequiredType::MinDc) => state.min_attack,
            Ok(mir2_shared::enums::RequiredType::MinMc) => state.min_mc,
            Ok(mir2_shared::enums::RequiredType::MinSc) => state.min_sc,
            _ => i32::MAX,
        };
        if required == mir2_shared::enums::RequiredType::MaxLevel as i32 {
            if (state.level as i32) > amount {
                return false;
            }
        } else if value < amount {
            return false;
        }
    }
    true
}

/// 广播 S.MountUpdate 给所有玩家（对齐 C# S.MountUpdate）
async fn broadcast_mount_update(world: &WorldActor, object_id: u32, mount_type: i16, riding: bool) {
    let packet = mir2_shared::packets::server::miscellaneous::MountUpdate {
        object_id,
        mount_type,
        riding_mount: riding,
    };
    let mut body = Vec::new();
    if packet.write_body(&mut body).is_ok() {
        for sid in world.players.keys() {
            let _ = world.gate_ref.tell(SendToClient { session_id: *sid, data: body.clone() }).await;
        }
    }
}

impl Message<EquipItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: EquipItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        // #206：英雄装备（C.EquipItem Grid=HeroInventory → hero_inventory.equipment）
        if msg.grid == mir2_shared::enums::MirGridType::HeroInventory as u8 {
            let slot = match EquipmentSlot::from_i32(msg.slot) {
                Some(s) => s,
                None => return,
            };
            // C# CanEquipItem 校验（英雄用主人职业/性别/等级近似）
            let state = match record.actor_ref.ask(GetPlayerState).await {
                Ok(Some(s)) => s,
                _ => return,
            };
            let item = state.hero_inventory.backpack.iter().flatten()
                .find(|s| s.item.unique_id == msg.unique_id)
                .map(|s| s.item.clone());
            let equippable = item.as_ref().and_then(|it| self.item_infos.get(&it.item_index))
                .map(|info| can_equip_item(info, slot, &state))
                .unwrap_or(false);
            if !equippable {
                send_system_message(&self.gate_ref, msg.session_id, "该物品无法装备到此位置");
                return;
            }
            let ok = record
                .actor_ref
                .ask(crate::actors::player::HeroEquipItem {
                    slot,
                    unique_id: msg.unique_id,
                })
                .await
                .unwrap_or(false);
            if ok {
                self.send_hero_information_packet(msg.session_id).await;
                tracing::info!("🦸 英雄装备 uid={} -> slot {}", msg.unique_id, msg.slot);
            } else {
                send_system_message(&self.gate_ref, msg.session_id, "装备失败");
            }
            return;
        }

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

        // C# CanEquipItem 校验（槽位类型/性别/职业/RequiredType）
        let equippable = self.item_infos.get(&state.inventory.backpack[grid_idx].as_ref().unwrap().item.item_index)
            .map(|info| can_equip_item(info, slot, &state))
            .unwrap_or(false);
        if !equippable {
            send_system_message(&self.gate_ref, msg.session_id, "该物品无法装备到此位置");
            send_equip_item_response(&self.gate_ref, msg.session_id, msg.grid, msg.unique_id, msg.slot, false);
            return;
        }

        // C# EquipItem：NeedIdentify 且未鉴定 → 自动鉴定（PlayerObject.cs:5660）
        if self.item_infos.get(&state.inventory.backpack[grid_idx].as_ref().unwrap().item.item_index)
            .map(|i| !i.is_identified()).unwrap_or(false)
        {
            let _ = record.actor_ref.ask(crate::actors::player::SetItemIdentified {
                unique_id: msg.unique_id,
            }).await;
        }

        let result = record.actor_ref.ask(InventoryEquipItem {
            grid: grid_idx as u8,
            slot,
        }).await.unwrap_or(None);

        match result {
            Some((_old_equipment, _new_uid)) => {
                debug!("Player session={} equipped item uid={} to slot {}", msg.session_id, msg.unique_id, msg.slot);
                send_equip_item_response(&self.gate_ref, msg.session_id, msg.grid, msg.unique_id, msg.slot, true);

                // C# 装备坐骑 → 骑乘 + 广播 MountUpdate
                if slot == crate::actors::inventory::EquipmentSlot::Mount {
                    let mount_type = self.item_infos.get(&state.inventory.backpack[grid_idx].as_ref().unwrap().item.item_index)
                        .map(|i| i.shape as i16)
                        .unwrap_or(0);
                    let _ = record.actor_ref.ask(crate::actors::player::SetMountState { mounted: true, mount_type }).await;
                    broadcast_mount_update(self, state.object_id, mount_type, true).await;
                }

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

        // #206：英雄卸下（C.RemoveItem Grid=HeroEquipment → hero_inventory.backpack）
        if msg.grid == mir2_shared::enums::MirGridType::HeroEquipment as u8 {
            let ok = record
                .actor_ref
                .ask(crate::actors::player::HeroRemoveItem {
                    unique_id: msg.unique_id,
                })
                .await
                .unwrap_or(false);
            if ok {
                self.send_hero_information_packet(msg.session_id).await;
                tracing::info!("🦸 英雄卸下 uid={}", msg.unique_id);
            } else {
                send_system_message(&self.gate_ref, msg.session_id, "背包已满，无法卸下装备");
            }
            return;
        }

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

                // C# 卸下坐骑 → 下马 + 广播 MountUpdate
                if slot == crate::actors::inventory::EquipmentSlot::Mount {
                    let _ = record.actor_ref.ask(crate::actors::player::SetMountState { mounted: false, mount_type: 0 }).await;
                    broadcast_mount_update(self, record.object_id, 0, false).await;
                }

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

        // C# DropItem：NoThrowItem 地图禁止丢弃
        if self.map_infos.get(&(state.map_index as i32)).map(|m| m.no_throw_item).unwrap_or(false) {
            send_system_message(&self.gate_ref, msg.session_id, "该地图无法丢弃物品");
            send_drop_item_response(&self.gate_ref, msg.session_id, msg.unique_id, msg.count as u32, false);
            return;
        }
        // C# DropItem：BindMode.DontDrop 物品不可丢弃（移除前校验）
        let dont_drop = state.inventory.get_item(msg.unique_id)
            .and_then(|it| self.item_infos.get(&it.item_index))
            .map(|i| (i.bind_mode & mir2_shared::enums::BindMode::DONT_DROP.bits() as i32) != 0)
            .unwrap_or(false);
        if dont_drop {
            send_system_message(&self.gate_ref, msg.session_id, "该物品无法丢弃");
            send_drop_item_response(&self.gate_ref, msg.session_id, msg.unique_id, msg.count as u32, false);
            return;
        }

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
            send_gold_changed_packet(&self.gate_ref, msg.session_id, amount);
            debug!("DropGold: {} dropped {} gold", state.name, msg.amount);
        }
    }
}

impl Message<BuyItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: BuyItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => { warn!("BuyItem: no player record for session {}", msg.session_id); return; }
        };

        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => { warn!("BuyItem: no player state for session {}", msg.session_id); return; }
        };

        // 查找 NPC 并验证商品是否在销售列表中（客户端 BuyItem 不含 npc_id）
        let (npc_oid, npc_db_index) = match self.session_npc.get(&msg.session_id) {
            Some(npc_oid) => match self.npcs.get(npc_oid) {
                Some(n) => (*npc_oid, n.db_index),
                None => {
                    send_system_message(&self.gate_ref, msg.session_id, "找不到该 NPC");
                    return;
                }
            },
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "请先与 NPC 对话");
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

        // C# BuyItem：按 Info.StackSize 分批创建（堆叠物品合并、非堆叠一格一个；背包满则停止）
        let stack_size = item_db.stack_size.max(1) as u16;
        let mut left = msg.count as u16;
        let mut added = 0u16;
        while left > 0 {
            let batch = left.min(stack_size);
            let item = mir2_shared::data::item::UserItem {
                item_index: msg.item_index as i32,
                count: batch,
                max_dura: item_db.durability as u16,
                current_dura: item_db.durability as u16,
                ..Default::default()
            };
            let ok = record.actor_ref.ask(AddItemToInventory { item }).await.unwrap_or(false);
            if !ok {
                break;
            }
            added += batch;
            left -= batch;
        }
        if added < msg.count as u16 {
            send_system_message(&self.gate_ref, msg.session_id, "背包空间不足，部分物品未购买");
            // 退款未购买部分
            let refund = (msg.count as u16 - added) as u64 * price_per_unit;
            if refund > 0 {
                let _ = record.actor_ref.ask(crate::actors::player::AddGold { amount: refund }).await;
            }
        }
        // 完整 UserInformation 刷新（背包 + 金币）
        if let Ok(Some(new_state)) = record.actor_ref.ask(GetPlayerState).await {
            let packet = super::build_user_information_packet(&new_state, &self.item_infos);
            let _ = self.gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: packet,
            }).await;
        }
        let updates = record.actor_ref.ask(crate::actors::player::CheckQuestItemProgress).await.unwrap_or_default();
        if !updates.is_empty() {
            send_system_message(&self.gate_ref, msg.session_id, "任务进度更新：获得物品");
        }
        send_system_message(&self.gate_ref, msg.session_id, &format!("购买成功 (花费 {} 金币)", total_price));
        let npc_name = self.npcs.get(&npc_oid).map(|n| n.name.as_str()).unwrap_or("?");
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

        // C# BindMode.DontSell：不可出售
        let item_db = self.item_infos.get(&item_data.item_index).cloned();
        let dont_sell = item_db.as_ref()
            .map(|i| (i.bind_mode & mir2_shared::enums::BindMode::DONT_SELL.bits() as i32) != 0)
            .unwrap_or(false);
        if dont_sell {
            send_system_message(&self.gate_ref, msg.session_id, "该物品无法出售");
            return;
        }

        // 移除物品（C# SellItem：堆叠按 count 拆分，非堆叠整件移除）
        let removed = record.actor_ref.ask(crate::actors::player::RemoveItemFromInventoryCount {
            unique_id: msg.unique_id,
            count: msg.count as u16,
        }).await.unwrap_or(None);
        if removed.is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "移除物品失败");
            return;
        }

        // 定价：C# Price() / 2（单价含耐久比例/附加属性；按卖出数量计）
        let per_unit = item_db
            .as_ref()
            .map(|info| compute_item_price_per_unit(&item_data, info))
            .unwrap_or_else(|| item_data.item_index as u64 * 5);
        let total_gold = (per_unit / 2).max(1) * msg.count as u64;

        let success = record.actor_ref.ask(AddGold { amount: total_gold }).await.unwrap_or(false);
        if success {
            // 记录到回购列表（最多保留 10 个）
            let buyback = BuybackItem {
                item: removed.as_ref().cloned().unwrap_or_else(|| item_data.clone()),
                sell_price: total_gold,
            };
            let list = self.buyback_items.entry(msg.session_id).or_default();
            list.insert(0, buyback);
            while list.len() > 10 {
                list.pop();
            }
            send_sell_item_response(&self.gate_ref, msg.session_id, msg.unique_id, msg.count, true);
            // 完整 UserInformation 刷新（背包 + 金币）
            if let Ok(Some(new_state)) = record.actor_ref.ask(GetPlayerState).await {
                let packet = super::build_user_information_packet(&new_state, &self.item_infos);
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: msg.session_id,
                    data: packet,
                }).await;
            }
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

        // 计算耐久缺失和修理费（C# ItemData.RepairPrice）
        let dura_deficit = item_data.max_dura.saturating_sub(item_data.current_dura) as u64;
        if dura_deficit == 0 {
            send_system_message(&self.gate_ref, msg.session_id, "该物品不需要修理");
            return;
        }
        let item_db = self.item_infos.get(&item_data.item_index).cloned();
        let repair_cost = item_db
            .as_ref()
            .map(|info| compute_repair_cost(&item_data, info, msg.special))
            .unwrap_or(0);
        if repair_cost == 0 {
            send_system_message(&self.gate_ref, msg.session_id, "该物品无法修理");
            return;
        }

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

        // 检查物品是否在背包中
        if msg.from < 0 || msg.from as usize >= state.inventory.backpack.len() || state.inventory.backpack[msg.from as usize].is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "物品不存在");
            return;
        }

        // 执行存入（目标格优先，占用则找第一个空位）
        let result = record.actor_ref.ask(StoreItemTo {
            from: msg.from,
            to: msg.to,
        }).await;
        match result {
            Ok(Some((_, storage_grid))) => {
                debug!("StoreItem: {} from={} to_storage={}", state.name, msg.from, storage_grid);
                // 完整刷新：仓库 + 背包
                if let Ok(Some(new_state)) = record.actor_ref.ask(GetPlayerState).await {
                    self.send_user_storage(msg.session_id, &new_state.inventory.storage);
                    let packet = super::build_user_information_packet(&new_state, &self.item_infos);
                    let _ = self.gate_ref.tell(SendToClient {
                        session_id: msg.session_id,
                        data: packet,
                    }).await;
                }
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

        // 检查物品是否在仓库中
        if msg.from < 0 || msg.from as usize >= state.inventory.storage.len() || state.inventory.storage[msg.from as usize].is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "仓库该格为空");
            return;
        }

        // 执行取出（目标格优先，占用则找第一个空位）
        let result = record.actor_ref.ask(TakeBackItemTo {
            from: msg.from,
            to: msg.to,
        }).await;
        match result {
            Ok(Some((_, backpack_grid))) => {
                debug!("TakeBackItem: {} from_storage={} to={}", state.name, msg.from, backpack_grid);
                if let Ok(Some(new_state)) = record.actor_ref.ask(GetPlayerState).await {
                    self.send_user_storage(msg.session_id, &new_state.inventory.storage);
                    let packet = super::build_user_information_packet(&new_state, &self.item_infos);
                    let _ = self.gate_ref.tell(SendToClient {
                        session_id: msg.session_id,
                        data: packet,
                    }).await;
                }
            }
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "取出物品失败");
            }
        }
    }
}

/// 合成失败响应（S.CraftItem { recipe_id, 0, false } + 系统消息）
async fn send_craft_fail(gate_ref: &kameo::actor::ActorRef<crate::gate::actor::GateActor>, session_id: u64, recipe_id: u32, reason: &str) {
    send_system_message(gate_ref, session_id, reason);
    let mut body = Vec::new();
    body.extend_from_slice(&recipe_id.to_le_bytes());
    body.extend_from_slice(&0u16.to_le_bytes());
    body.push(0u8);
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::CraftItem as i16, &body),
    }).await;
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

        // 查找配方（DB recipes 表，对齐 C# RecipeInfo）
        let recipe = match self.recipe_infos.iter().find(|r| r.recipe_id == msg.recipe_id as i32) {
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

        // 等级/性别/职业/任务/flag 需求（C# RecipeInfo 需求）
        if let Some(req_level) = recipe.required_level {
            if (state.level as u16) < req_level {
                send_craft_fail(&self.gate_ref, msg.session_id, msg.recipe_id, "等级不足").await;
                return;
            }
        }
        if let Some(req_gender) = recipe.required_gender {
            if state.gender as u8 != req_gender {
                send_craft_fail(&self.gate_ref, msg.session_id, msg.recipe_id, "性别不符合要求").await;
                return;
            }
        }
        if !recipe.required_classes.is_empty() && !recipe.required_classes.contains(&(state.class as u8)) {
            send_craft_fail(&self.gate_ref, msg.session_id, msg.recipe_id, "职业不符合要求").await;
            return;
        }
        for q in &recipe.required_quests {
            if !state.quest_log.completed_indices.contains(q) {
                send_craft_fail(&self.gate_ref, msg.session_id, msg.recipe_id, "任务未完成").await;
                return;
            }
        }
        for f in &recipe.required_flags {
            if state.flags.get(&format!("NPC_FLAG_{}", f)).copied().unwrap_or(0) < 1 {
                send_craft_fail(&self.gate_ref, msg.session_id, msg.recipe_id, "条件未满足").await;
                return;
            }
        }
        // 工具检查（不消耗）
        for tool in &recipe.tools {
            let has = record.actor_ref.ask(crate::actors::player::HasItem { item_index: *tool, count: 1 }).await.unwrap_or(false);
            if !has {
                send_craft_fail(&self.gate_ref, msg.session_id, msg.recipe_id, "缺少工具").await;
                return;
            }
        }
        // 金币费用
        if recipe.gold_cost > 0 {
            if state.inventory.gold < recipe.gold_cost as u64 {
                send_craft_fail(&self.gate_ref, msg.session_id, msg.recipe_id, "金币不足").await;
                return;
            }
            let _ = record.actor_ref.ask(crate::actors::player::DeductGold { amount: recipe.gold_cost as u64 }).await;
        }

        // 检查背包空间
        if !state.inventory.has_space() {
            send_craft_fail(&self.gate_ref, msg.session_id, msg.recipe_id, "背包已满").await;
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
        let success = fastrand::u8(0..100) < recipe.chance;

        if success {
            let mut item = mir2_shared::data::item::UserItem {
                item_index: recipe.product_item_index,
                count: recipe.product_count,
                ..Default::default()
            };
            if let Some(info) = self.item_infos.get(&recipe.product_item_index) {
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

        // 查找回购列表中的对应物品（按 unique_id，C# BuyItemBack 语义）
        let list = match self.buyback_items.get_mut(&msg.session_id) {
            Some(l) => l,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "没有可回购的物品");
                return;
            }
        };
        let idx = match list.iter().position(|b| b.item.unique_id == msg.unique_id) {
            Some(i) => i,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "该物品已无法回购");
                return;
            }
        };

        let buyback = list.remove(idx);
        // 数量按回购请求扣减（整件回购时 count 即物品数量）
        let count = (msg.count as u16).min(buyback.item.count.max(1));
        let mut item = buyback.item.clone();
        item.count = count;
        // C#：按单价收费（sell_price 是整堆 Price()/2，除以原堆数量得到单价）
        let per_unit = buyback.sell_price / buyback.item.count.max(1) as u64;
        let cost = per_unit.saturating_mul(count as u64).max(1);

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

        // 添加物品到背包 + 完整刷新（背包 + 金币）
        let _ = record.actor_ref.ask(crate::actors::player::AddItemToInventory {
            item: item.clone(),
        }).await;
        if let Ok(Some(new_state)) = record.actor_ref.ask(GetPlayerState).await {
            let packet = super::build_user_information_packet(&new_state, &self.item_infos);
            let _ = self.gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: packet,
            }).await;
        }
        send_system_message(&self.gate_ref, msg.session_id, &format!("回购成功，花费 {} 金币", cost));
        debug!("BuyItemBack: {} uid={} count={} cost={}", state.name, msg.unique_id, count, cost);
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
