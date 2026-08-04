use bevy::prelude::*;
use mir2_shared::packets::base::{Packet, PacketHeader};
use crate::network::*;
use crate::ui::login::AuthFeedback;
use super::*;

// 网络包解码分派（#72 拆分）：handle_npc_items 处理 arms_npc_items.rs 的服务端包分支。
// 由 packets.rs::handle_packet 调度器按 opcode 调用；返回 true 表示已处理。

#[allow(clippy::too_many_arguments, unused_variables)]
pub(crate) fn handle_npc_items(
    net: &mut NetConnection,
    session: &mut SessionState,
    auth: &mut AuthFeedback,
    game_data: &mut GameData,
    net_objects: &mut MessageWriter<NetObject>,
    net_removals: &mut MessageWriter<NetObjectRemoved>,
    motions: &mut MessageWriter<NetMotion>,
    hud: &mut HudState,
    chat: &mut ChatState,
    npc_dialog: &mut NpcDialogState,
    npc_goods: &mut NpcGoodsState,
    combat_evt: &mut MessageWriter<CombatEvent>,
    weather: &mut WeatherState,
    magics: &mut MagicsState,
    storage: &mut StorageState,
    sell_panel: &mut SellPanelState,
    group: &mut GroupState,
    mail: &mut MailState,
    trade: &mut TradeState,
    friend: &mut FriendState,
    guild: &mut GuildState,
    ranking: &mut RankingState,
    mentor: &mut MentorState,
    market: &mut MarketState,
    shop: &mut GameShopState,
    territory: &mut GuildTerritoryState,
    effects: &mut MessageWriter<PendingEffect>,
    server_events: &mut MessageWriter<ServerEvent>,
    control: &mut ControlState,
    fishing: &mut FishingState,
    refine: &mut RefineState,
    craft: &mut CraftState,
    rental: &mut ItemRentalState,
    quest_log: &mut QuestLogState,
    buff: &mut BuffState,
    report: &mut ReportState,
    inspect: &mut InspectState,
    creature: &mut CreatureState,
    hero: &mut HeroState,
    relationship: &mut RelationshipState,
    big_map: &mut crate::game::dialogs::big_map::BigMapState,
    awake: &mut crate::game::dialogs::npc_awake::NpcAwakeState,
    roll: &mut crate::game::dialogs::roll::RollState,
    mgr: &mut crate::game::dialogs::DialogManager,
    next: &mut NextState<AppState>,
    payload: &[u8],
) -> bool {
    use mir2_shared::packets::server::*;

    let mut cur = std::io::Cursor::new(payload);
    let Ok(header) = PacketHeader::read_from(&mut cur) else {
        return false;
    };
    let opcode = header.opcode;
    const HANDLED: &[i16] = &[ServerPacketIds::NPCResponse as i16, ServerPacketIds::ObjectStruck as i16, ServerPacketIds::ObjectDied as i16, ServerPacketIds::Death as i16, ServerPacketIds::Revived as i16, ServerPacketIds::ObjectRevived as i16, ServerPacketIds::DamageIndicator as i16, ServerPacketIds::NPCGoods as i16, ServerPacketIds::MoveItem as i16, ServerPacketIds::EquipItem as i16, ServerPacketIds::RemoveItem as i16, ServerPacketIds::UseItem as i16, ServerPacketIds::SplitItem as i16, ServerPacketIds::DropItem as i16, ServerPacketIds::MergeItem as i16, ServerPacketIds::SellItem as i16];
    let handled = HANDLED.contains(&opcode);
    match opcode {

        // ---- M9: NPC 对话 ----
        x if x == ServerPacketIds::NPCResponse as i16 => {
            match npc_interaction::NPCResponse::read_body(&mut cur) {
            Ok(p) => {
                tracing::info!("🧙 NPC 对话: {} 行", p.page.len());
                server_events.write(crate::network::server_event::from_packet::npc_dialog(&p));
            }
            Err(e) => tracing::warn!("⚠️ NPCResponse 解析失败: {} (len={})", e, payload.len()),
            }
        }

        // ---- M10: 战斗反馈 ----
        x if x == ServerPacketIds::ObjectStruck as i16 => {
            if let Ok(p) = combat::ObjectStruck::read_body(&mut cur) {
                combat_evt.write(CombatEvent::Struck { object_id: p.object_id, direction: p.direction });
                // M38：选中的目标受击 → 命中爆炸特效
                if control.attack_target == Some(p.object_id) {
                    effects.write(PendingEffect::Burst {
                        target_id: p.object_id,
                        color: [1.0, 0.7, 0.2],
                    });
                }
            }
        }
        x if x == ServerPacketIds::ObjectDied as i16 => {
            if let Ok(p) = combat::ObjectDied::read_body(&mut cur) {
                combat_evt.write(CombatEvent::Died { object_id: p.object_id, death_type: p.death_type });
            }
        }
        // ---- M46: 玩家死亡/复活 ----
        x if x == ServerPacketIds::Death as i16 => {
            // 容忍空 body：ServerRust 早期版本发空 Death 包（#55 实测），
            // 标准协议为 [loc_x i32][loc_y i32][direction u8]；解析失败也进入死亡状态
            let mut loc = (0u32, 0u32);
            let mut parsed = false;
            if let Ok(p) = combat::Death::read_body(&mut cur) {
                loc = (p.location_x, p.location_y);
                parsed = true;
            }
            let pid = hud.player_object_id.unwrap_or(100);
            hud.dead = true;
            combat_evt.write(CombatEvent::Died { object_id: pid, death_type: 0 });
            tracing::info!("💀 玩家死亡 ({},{}){}", loc.0, loc.1, if parsed { "" } else { "（空 body 容错）" });
        }
        x if x == ServerPacketIds::Revived as i16 => {
            if combat::Revived::read_body(&mut cur).is_ok() {
                let pid = hud.player_object_id.unwrap_or(100);
                hud.dead = false;
                combat_evt.write(CombatEvent::Revived { object_id: pid });
                tracing::info!("💚 玩家复活");
            }
        }
        x if x == ServerPacketIds::ObjectRevived as i16 => {
            if let Ok(p) = combat::ObjectRevived::read_body(&mut cur) {
                combat_evt.write(CombatEvent::Revived { object_id: p.object_id });
                tracing::debug!("💚 对象复活 id={}", p.object_id);
            }
        }
        x if x == ServerPacketIds::DamageIndicator as i16 => {
            if let Ok(p) = combat::DamageIndicator::read_body(&mut cur) {
                combat_evt.write(CombatEvent::Damage { object_id: p.object_id, damage: p.damage, dmg_type: p.damage_type });
            }
        }

        // ---- M9: NPC 商店 ----
        x if x == ServerPacketIds::NPCGoods as i16 => {
            // C# 协议：NPCGoods 的 body 是 gzip 压缩的（C# ServerPackets.NPCGoods.Compressed == true，
            // Rust SharedRust 对应 is_compressed()==true），必须解压后再解析。
            let mut body = Vec::new();
            let mut gz = flate2::read::GzDecoder::new(std::io::Cursor::new(
                &payload[PacketHeader::HEADER_SIZE..],
            ));
            match std::io::Read::read_to_end(&mut gz, &mut body) {
                Ok(_) => {
                    let mut cur = std::io::Cursor::new(body);
                    match npc_interaction::NPCGoods::read_body(&mut cur) {
                        Ok(p) => {
                            // C# 语义：Sell/Repair/SpecialRepair 面板 → 打开出售/修理面板（NPCDropDialog），
                            // 其余（Buy/BuySub/Craft）→ 商品对话框
                            if matches!(
                                p.panel_type,
                                mir2_shared::enums::PanelType::Sell
                                    | mir2_shared::enums::PanelType::Repair
                                    | mir2_shared::enums::PanelType::SpecialRepair
                            ) {
                                npc_goods.visible = false;
                                sell_panel.mode = Some(p.panel_type);
                                sell_panel.target = None;
                                sell_panel.visible = true;
                                tracing::info!("🧰 NPC 面板: {:?}", p.panel_type);
                                // C# NPCDropDialog.Show() 同时打开背包
                                if !mgr.is_open(crate::game::dialogs::DialogKind::Inventory) {
                                    mgr.open.push(crate::game::dialogs::DialogKind::Inventory);
                                }
                                return true;
                            }
                            let goods: Vec<GoodsEntry> = p
                                .list
                                .iter()
                                .map(|item| GoodsEntry {
                                    item_index: item.item_index,
                                    unique_id: item.unique_id,
                                    name: item
                                        .info
                                        .as_ref()
                                        .map(|i| i.name.clone())
                                        .unwrap_or_else(|| format!("#{}", item.item_index)),
                                    price: item.info.as_ref().map(|i| i.price).unwrap_or(0),
                                    count: item.count,
                                })
                                .collect();
                            tracing::info!("🏪 NPC 商品: {} 件 (rate={})", goods.len(), p.rate);
                            npc_goods.goods = goods;
                            npc_goods.selected = None;
                            npc_goods.visible = true;
                        }
                        Err(e) => {
                            tracing::warn!("⚠️ NPCGoods 解析失败: {} (len={})", e, payload.len())
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("⚠️ NPCGoods gzip 解压失败: {} (len={})", e, payload.len())
                }
            }
        }

        // ---- M13: 物品操作响应 ----
        x if x == ServerPacketIds::MoveItem as i16 => {
            if let Ok(p) = item_operations::MoveItem::read_body(&mut cur) {
                if p.success && p.grid == mir2_shared::enums::MirGridType::Inventory {
                    let from = p.from as usize;
                    let to = p.to as usize;
                    if from < hud.inventory.items.len() && to < hud.inventory.items.len() {
                        hud.inventory.items.swap(from, to);
                        tracing::info!("📦 移动物品 {} -> {}", p.from, p.to);
                    }
                }
            }
        }

        // ---- M13: 装备/使用响应（本地同步） ----
        x if x == ServerPacketIds::EquipItem as i16 => {
            if let Ok(p) = item_operations::EquipItem::read_body(&mut cur) {
                if p.success {
                    let to = p.to as usize;
                    // 从背包移除并放入装备槽
                    let from_idx = hud
                        .inventory
                        .items
                        .iter()
                        .position(|s| s.as_ref().map(|it| it.unique_id) == Some(p.unique_id));
                    if let Some(from_idx) = from_idx {
                        let item = hud.inventory.items[from_idx].take();
                        if let Some(item) = item {
                            let name = item.name.clone();
                            if to < hud.equipment.len() {
                                let old = hud.equipment[to].take();
                                hud.equipment[to] = Some(item);
                                // 旧装备放回背包空格
                                if let Some(old) = old {
                                    if let Some(empty) = hud.inventory.items.iter_mut().find(|s| s.is_none()) {
                                        *empty = Some(old);
                                    }
                                }
                            }
                            tracing::info!("⚔️ 装备成功: {} -> 槽 {}", name, p.to);
                        }
                    }
                }
            }
        }
        x if x == ServerPacketIds::RemoveItem as i16 => {
            if let Ok(p) = item_operations::RemoveItem::read_body(&mut cur) {
                if p.success {
                    let item = hud
                        .equipment
                        .iter_mut()
                        .flatten()
                        .find(|it| it.unique_id == p.unique_id)
                        .cloned();
                    if let Some(item) = item {
                        for slot in hud.equipment.iter_mut() {
                            if slot.as_ref().map(|it| it.unique_id) == Some(p.unique_id) {
                                *slot = None;
                                break;
                            }
                        }
                        if let Some(empty) = hud.inventory.items.iter_mut().find(|s| s.is_none()) {
                            *empty = Some(item);
                        }
                        tracing::info!("🛡️ 卸下装备 uid={}", p.unique_id);
                    }
                }
            }
        }
        x if x == ServerPacketIds::UseItem as i16 => {
            if let Ok(p) = item_operations::UseItem::read_body(&mut cur) {
                let idx = hud
                    .inventory
                    .items
                    .iter()
                    .position(|s| s.as_ref().map(|it| it.unique_id) == Some(p.unique_id));
                if let Some(idx) = idx {
                    let count = hud.inventory.items[idx].as_ref().map(|it| it.count).unwrap_or(0);
                    if count > 1 {
                        if let Some(it) = hud.inventory.items[idx].as_mut() {
                            it.count -= 1;
                        }
                    } else {
                        hud.inventory.items[idx] = None;
                    }
                    tracing::info!("💊 使用物品 uid={} 剩余 {}", p.unique_id, count.saturating_sub(1));
                }
            }
        }
        x if x == ServerPacketIds::SplitItem as i16 => {
            // 拆分响应后服务端会跟完整 UserInformation 刷新（权威重建背包）
            if let Ok(p) = item::SplitItem::read_body(&mut cur) {
                tracing::info!("🔪 拆分响应: grid={:?} uid={} count={}", p.grid, p.unique_id, p.count);
            }
        }
        x if x == ServerPacketIds::DropItem as i16 => {
            if let Ok(p) = item_operations::DropItem::read_body(&mut cur) {
                tracing::info!("🗑️ 丢弃响应: uid={} count={} success={}", p.unique_id, p.count, p.success);
            }
        }
        x if x == ServerPacketIds::MergeItem as i16 => {
            if let Ok(p) = item_operations::MergeItem::read_body(&mut cur) {
                tracing::info!("🧬 合并响应: from={} to={} success={}", p.id_from, p.id_to, p.success);
            }
        }
        x if x == ServerPacketIds::SellItem as i16 => {
            if let Ok(p) = item::SellItem::read_body(&mut cur) {
                tracing::info!("💰 出售响应: uid={} count={} success={}", p.unique_id, p.count, p.success);
            }
        }

        // ---- M13: 技能 ----

        _ => {}
    }
    handled
}
