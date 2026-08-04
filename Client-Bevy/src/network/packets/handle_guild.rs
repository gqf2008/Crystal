use bevy::prelude::*;
use mir2_shared::packets::base::{Packet, PacketHeader};
use crate::network::*;
use crate::ui::login::AuthFeedback;
use super::*;

// 网络包解码分派（#72 拆分）：handle_guild 处理 arms_guild.rs 的服务端包分支。
// 由 packets.rs::handle_packet 调度器按 opcode 调用；返回 true 表示已处理。

#[allow(clippy::too_many_arguments, unused_variables)]
pub(crate) fn handle_guild(
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
    const HANDLED: &[i16] = &[ServerPacketIds::UserStorage as i16, ServerPacketIds::GuildStatus as i16, ServerPacketIds::GuildStorageList as i16, ServerPacketIds::NPCMarket as i16, ServerPacketIds::NPCMarketPage as i16, ServerPacketIds::ConsignItem as i16, ServerPacketIds::MarketSuccess as i16, ServerPacketIds::MarketFail as i16, ServerPacketIds::GameShopInfo as i16, ServerPacketIds::GameShopStock as i16, ServerPacketIds::GuildTerritoryPage as i16, ServerPacketIds::GuildRequestWar as i16];
    let handled = HANDLED.contains(&opcode);
    match opcode {
        // ---- M18: 仓库 ----
        x if x == ServerPacketIds::UserStorage as i16 => {
            match player::UserStorage::read_body(&mut cur) {
                Ok(p) => {
                    let items: Vec<Option<InvItem>> = p
                        .storage
                        .iter()
                        .map(|s| s.as_ref().map(to_inv_item))
                        .collect();
                    tracing::info!(
                        "🏬 仓库 {} 格（{} 件物品）",
                        items.len(),
                        items.iter().flatten().count()
                    );
                    // 仓库数据/打开对话框逻辑移入 storage 消费端
                    server_events.write(ServerEvent::StorageOpened { items, visible: true });
                }
                Err(e) => tracing::warn!("⚠️ UserStorage 解析失败: {} (len={})", e, payload.len()),
            }
        }

        // ---- M27: 行会 ----
        x if x == ServerPacketIds::GuildStatus as i16 => {
            use byteorder::{LittleEndian, ReadBytesExt};
            // 双格式：1 字节 in_guild / 完整行会信息（服务端 send_guild_info_packet 复用此 opcode）
            let body = &payload[PacketHeader::HEADER_SIZE..];
            if body.len() == 1 {
                let in_guild = body[0] != 0;
                server_events.write(ServerEvent::GuildInGuild { in_guild });
                tracing::info!("🏰 行会状态: {}", if in_guild { "在行会中" } else { "未加入行会" });
            } else {
                let mut cur = std::io::Cursor::new(body);
                let name = mir2_shared::binary::read_dotnet_string(&mut cur).unwrap_or_default();
                let leader = mir2_shared::binary::read_dotnet_string(&mut cur).unwrap_or_default();
                let notice_count = cur.read_u8().unwrap_or(0) as usize;
                let mut notice = Vec::new();
                for _ in 0..notice_count {
                    match mir2_shared::binary::read_dotnet_string(&mut cur) {
                        Ok(l) => notice.push(l),
                        Err(_) => break,
                    }
                }
                let member_count = cur.read_u8().unwrap_or(0) as usize;
                let mut members = Vec::new();
                for _ in 0..member_count {
                    let mname = mir2_shared::binary::read_dotnet_string(&mut cur).unwrap_or_default();
                    let rank = cur.read_u8().unwrap_or(0);
                    let online = cur.read_u8().unwrap_or(0) != 0;
                    members.push(UiGuildMember { name: mname, rank, online });
                }
                let mut gold_buf = [0u8; 4];
                let gold = if std::io::Read::read_exact(&mut cur, &mut gold_buf).is_ok() {
                    u32::from_le_bytes(gold_buf)
                } else {
                    0
                };
                let member_count = members.len();
                server_events.write(ServerEvent::GuildData {
                    name,
                    leader,
                    notice,
                    members,
                    gold,
                });
                tracing::info!("🏰 行会信息: 已广播（成员 {}）", member_count);
            }
        }
        // ---- M32: 行会仓库物品列表 ----
        x if x == ServerPacketIds::GuildStorageList as i16 => {
            let body = &payload[PacketHeader::HEADER_SIZE..];
            let mut cur = std::io::Cursor::new(body);
            match mir2_shared::packets::server::guild::GuildStorageList::read_body(&mut cur) {
                Ok(p) => {
                    let items: Vec<Option<StorageItem>> = p
                        .items
                        .iter()
                        .take(100)
                        .map(|opt| {
                            opt.as_ref().map(|gsi| StorageItem {
                                unique_id: gsi.item.unique_id,
                                item_index: gsi.item.item_index,
                                name: gsi
                                    .item
                                    .info
                                    .as_ref()
                                    .map(|i| i.name.clone())
                                    .or_else(|| guild.item_names.get(&gsi.item.item_index).cloned())
                                    .unwrap_or_default(),
                                count: gsi.item.count,
                            })
                        })
                        .collect();
                    let count = items.iter().filter_map(|s| s.as_ref()).count();
                    let total = items.len();
                    server_events.write(ServerEvent::GuildStorage { items });
                    tracing::info!("🏰 仓库物品列表: {} 格（{} 件）", total, count);
                }
                Err(e) => {
                    tracing::warn!("⚠️ GuildStorageList 解析失败: {} (len={})", e, payload.len())
                }
            }
        }
        // ---- M34: 市场 ----
        x if x == ServerPacketIds::NPCMarket as i16 => {
            // [count i32][per page: 7-bit dotnet]
            let body = &payload[PacketHeader::HEADER_SIZE..];
            let mut cur = std::io::Cursor::new(body);
            use byteorder::{LittleEndian, ReadBytesExt};
            let count = cur.read_i32::<LittleEndian>().unwrap_or(0).max(0) as usize;
            // 只取页数（页名可跳过）
            let mut pages = 0usize;
            for _ in 0..count {
                if mir2_shared::binary::read_dotnet_string(&mut cur).is_ok() {
                    pages += 1;
                } else {
                    break;
                }
            }
            market.pages = pages.max(1);
            tracing::info!("🏪 市场页数: {}", market.pages);
        }
        x if x == ServerPacketIds::NPCMarketPage as i16 => {
            // [count i32][per listing: auction_id u64][UserItem][7-bit seller][price u32][date i64]
            let body = &payload[PacketHeader::HEADER_SIZE..];
            let mut cur = std::io::Cursor::new(body);
            use byteorder::{LittleEndian, ReadBytesExt};
            let count = cur.read_i32::<LittleEndian>().unwrap_or(0).max(0) as usize;
            let mut listings = Vec::with_capacity(count);
            let mut ok = true;
            for _ in 0..count {
                let auction_id = match cur.read_u64::<LittleEndian>() {
                    Ok(v) => v,
                    Err(_) => { ok = false; break; }
                };
                let item = match mir2_shared::data::item::UserItem::read_from(&mut cur, i32::MAX, i32::MAX) {
                    Ok(v) => v,
                    Err(_) => { ok = false; break; }
                };
                let seller = match mir2_shared::binary::read_dotnet_string(&mut cur) {
                    Ok(v) => v,
                    Err(_) => { ok = false; break; }
                };
                let price = match cur.read_u32::<LittleEndian>() {
                    Ok(v) => v,
                    Err(_) => { ok = false; break; }
                };
                let _date = match cur.read_i64::<LittleEndian>() {
                    Ok(v) => v,
                    Err(_) => { ok = false; break; }
                };
                let name = item
                    .info
                    .as_ref()
                    .map(|i| i.name.clone())
                    .or_else(|| market.item_names.get(&item.item_index).cloned())
                    .unwrap_or_else(|| format!("#{}", item.item_index));
                listings.push(MarketItem {
                    auction_id,
                    unique_id: item.unique_id,
                    name,
                    item_index: item.item_index,
                    count: item.count,
                    seller,
                    price,
                });
            }
            if ok {
                market.listings = listings;
                tracing::info!(
                    "🏪 市场列表: {} 件",
                    market.listings.len()
                );
            } else {
                tracing::warn!("⚠️ NPCMarketPage 解析失败: (len={})", payload.len());
            }
        }
        x if x == ServerPacketIds::ConsignItem as i16 => {
            // [unique_id u64][success u8]
            let body = &payload[PacketHeader::HEADER_SIZE..];
            let mut cur = std::io::Cursor::new(body);
            use byteorder::{LittleEndian, ReadBytesExt};
            if let Ok(uid) = cur.read_u64::<LittleEndian>() {
                let ok = cur.read_u8().unwrap_or(0) != 0;
                if ok {
                    market.consign_ok = Some(uid);
                    market.message = format!("寄售成功 uid={}", uid);
                    tracing::info!("🏪 寄售成功: uid={}", uid);
                } else {
                    market.message = "寄售失败".to_string();
                    tracing::warn!("🏪 寄售失败: uid={}", uid);
                }
            }
        }
        x if x == ServerPacketIds::MarketSuccess as i16 => {
            let body = &payload[PacketHeader::HEADER_SIZE..];
            let mut cur = std::io::Cursor::new(body);
            match mir2_shared::binary::read_dotnet_string(&mut cur) {
                Ok(msg) => {
                    market.message = msg.clone();
                    tracing::info!("🏪 市场成功: {}", msg);
                }
                Err(e) => tracing::warn!("⚠️ MarketSuccess 解析失败: {} (len={})", e, payload.len()),
            }
        }
        x if x == ServerPacketIds::MarketFail as i16 => {
            let reason = payload.get(PacketHeader::HEADER_SIZE).copied().unwrap_or(0);
            market.message = format!("市场操作失败（原因 {}）", reason);
            tracing::warn!("🏪 市场失败原因: {}", reason);
        }
        // ---- M35: 商城 ----
        x if x == ServerPacketIds::GameShopInfo as i16 => {
            // [count i32][per: item_index i32][gold u32][credit u32][count i32][class u8]
            //      [category 7-bit][stock i32][is_bought u8][deal u8]...[credit u32][gold u32]
            let body = &payload[PacketHeader::HEADER_SIZE..];
            let mut cur = std::io::Cursor::new(body);
            use byteorder::{LittleEndian, ReadBytesExt};
            let count = cur.read_i32::<LittleEndian>().unwrap_or(0).max(0) as usize;
            let mut items = Vec::with_capacity(count);
            let mut ok = true;
            for _ in 0..count {
                let item_index = match cur.read_i32::<LittleEndian>() { Ok(v) => v, Err(_) => { ok = false; break; } };
                let gold_price = match cur.read_u32::<LittleEndian>() { Ok(v) => v, Err(_) => { ok = false; break; } };
                let credit_price = match cur.read_u32::<LittleEndian>() { Ok(v) => v, Err(_) => { ok = false; break; } };
                let _count = match cur.read_i32::<LittleEndian>() { Ok(v) => v, Err(_) => { ok = false; break; } };
                let _class = match cur.read_u8() { Ok(v) => v, Err(_) => { ok = false; break; } };
                let category = match mir2_shared::binary::read_dotnet_string(&mut cur) { Ok(v) => v, Err(_) => { ok = false; break; } };
                let stock = match cur.read_i32::<LittleEndian>() { Ok(v) => v, Err(_) => { ok = false; break; } };
                let _is_bought = match cur.read_u8() { Ok(v) => v, Err(_) => { ok = false; break; } };
                let _deal = match cur.read_u8() { Ok(v) => v, Err(_) => { ok = false; break; } };
                let name = shop.item_names.get(&item_index).cloned().unwrap_or_default();
                items.push(UiShopItem { item_index, name, gold_price, credit_price, category, stock });
            }
            if ok {
                let _credit = cur.read_u32::<LittleEndian>().unwrap_or(0);
                let gold = cur.read_u32::<LittleEndian>().unwrap_or(0);
                shop.items = items;
                shop.gold = gold;
                tracing::info!(
                    "🛒 商城目录: {} 件，金币 {}",
                    shop.items.len(),
                    shop.gold
                );
            } else {
                tracing::warn!("⚠️ GameShopInfo 解析失败: (len={})", payload.len());
            }
        }
        x if x == ServerPacketIds::GameShopStock as i16 => {
            // [item_id i32][stock i32]
            let body = &payload[PacketHeader::HEADER_SIZE..];
            if body.len() >= 8 {
                let item_id = i32::from_le_bytes(body[0..4].try_into().unwrap_or([0; 4]));
                let stock = i32::from_le_bytes(body[4..8].try_into().unwrap_or([0; 4]));
                shop.message = format!("商品 #{} 库存剩余 {}", item_id, stock);
                if let Some(it) = shop.items.iter_mut().find(|i| i.item_index == item_id) {
                    it.stock = stock;
                }
                tracing::info!("🛒 商城库存: #{} 剩余 {}", item_id, stock);
            }
        }
        // ---- M36: 行会领地/宣战 ----
        x if x == ServerPacketIds::GuildTerritoryPage as i16 => {
            // [count i32][per: id i32][map_index i32][owner 7-bit dotnet][state u8]
            let body = &payload[PacketHeader::HEADER_SIZE..];
            let mut cur = std::io::Cursor::new(body);
            use byteorder::{LittleEndian, ReadBytesExt};
            let count = cur.read_i32::<LittleEndian>().unwrap_or(0).max(0) as usize;
            let mut rows = Vec::with_capacity(count);
            let mut ok = true;
            for _ in 0..count {
                let id = match cur.read_i32::<LittleEndian>() { Ok(v) => v, Err(_) => { ok = false; break; } };
                let map_index = match cur.read_i32::<LittleEndian>() { Ok(v) => v, Err(_) => { ok = false; break; } };
                let owner = match mir2_shared::binary::read_dotnet_string(&mut cur) { Ok(v) => v, Err(_) => { ok = false; break; } };
                let state = match cur.read_u8() { Ok(v) => v, Err(_) => { ok = false; break; } };
                rows.push(TerritoryRow { id, map_index, owner, state });
            }
            if ok {
                territory.rows = rows;
                tracing::info!(
                    "🏯 领地列表: {} 个（无主 {}）",
                    territory.rows.len(),
                    territory.rows.iter().filter(|r| r.owner.is_empty()).count()
                );
            } else {
                tracing::warn!("⚠️ GuildTerritoryPage 解析失败: (len={})", payload.len());
            }
        }
        x if x == ServerPacketIds::GuildRequestWar as i16 => {
            // [guild_name 7-bit dotnet]
            let body = &payload[PacketHeader::HEADER_SIZE..];
            let mut cur = std::io::Cursor::new(body);
            match mir2_shared::binary::read_dotnet_string(&mut cur) {
                Ok(name) => {
                    territory.war_message = format!("已向 {} 行会宣战", name);
                    tracing::info!("🏯 宣战确认: {}", name);
                }
                Err(e) => {
                    tracing::warn!("⚠️ GuildRequestWar 解析失败: {} (len={})", e, payload.len())
                }
            }
        }

        _ => {}
    }
    handled
}
