use bevy::prelude::*;
use mir2_shared::packets::base::{Packet, PacketHeader};
use crate::network::*;
use crate::ui::login::AuthFeedback;
use super::*;
// #2630：显式引入本处理器构造的 UI 载荷类型（原经 super::* 隐私链隐式传入，见 handle_guild 注）。
use crate::game::dialogs::creature::CreatureEntry;
use crate::game::dialogs::inspect::InspectItem;
use crate::game::dialogs::quest_log::QuestEntry;

/// #2791 单元④：`S.AddBuff` 解析（与 `ServerRust::actors::player::build_add_buff_body` 同序）：
/// `[tag u8][remaining_ms u32][paused u8][value_count u8][values i32…]`
pub(crate) fn parse_add_buff_body(body: &[u8]) -> Option<(u8, u32, bool, Vec<i32>)> {
    let tag = *body.first()?;
    let remaining_ms = u32::from_le_bytes(body.get(1..5)?.try_into().ok()?);
    let paused = *body.get(5)? != 0;
    let count = *body.get(6)? as usize;
    let mut values = Vec::with_capacity(count);
    for i in 0..count {
        let s = 7 + i * 4;
        values.push(i32::from_le_bytes(body.get(s..s + 4)?.try_into().ok()?));
    }
    Some((tag, remaining_ms, paused, values))
}

// 网络包解码分派（#72 拆分）：handle_progress 处理 arms_progress.rs 的服务端包分支。
// 由 packets.rs::handle_packet 调度器按 opcode 调用；返回 true 表示已处理。

#[allow(clippy::too_many_arguments, unused_variables)]
pub(crate) fn handle_progress(    server_events: &mut MessageWriter<ServerEvent>,
    payload: &[u8],) -> bool {
    use mir2_shared::packets::server::*;

    let mut cur = std::io::Cursor::new(payload);
    let Ok(header) = PacketHeader::read_from(&mut cur) else {
        return false;
    };
    let opcode = header.opcode;
    const HANDLED: &[i16] = &[ServerPacketIds::CraftItem as i16, ServerPacketIds::ItemRentalRequest as i16, ServerPacketIds::UpdateRentalItem as i16, ServerPacketIds::ItemRentalFee as i16, ServerPacketIds::ItemRentalPeriod as i16, ServerPacketIds::DepositRentalItem as i16, ServerPacketIds::RetrieveRentalItem as i16, ServerPacketIds::ItemRentalLock as i16, ServerPacketIds::ItemRentalPartnerLock as i16, ServerPacketIds::CanConfirmItemRental as i16, ServerPacketIds::ConfirmItemRental as i16, ServerPacketIds::CancelItemRental as i16, ServerPacketIds::GetRentedItems as i16, ServerPacketIds::ChangeQuest as i16, ServerPacketIds::CompleteQuest as i16, ServerPacketIds::NPCAwakening as i16, ServerPacketIds::NewQuestInfo as i16, ServerPacketIds::ShareQuest as i16, ServerPacketIds::GainedQuestItem as i16, ServerPacketIds::DeleteQuestItem as i16, ServerPacketIds::NewRecipeInfo as i16, ServerPacketIds::PauseBuff as i16, ServerPacketIds::RefreshItem as i16, ServerPacketIds::SetBindingShot as i16, ServerPacketIds::BaseStatsInfo as i16, ServerPacketIds::HeroBaseStatsInfo as i16, ServerPacketIds::NPCDisassemble as i16, ServerPacketIds::NPCDowngrade as i16, ServerPacketIds::NPCReset as i16, ServerPacketIds::GuildBuffList as i16, ServerPacketIds::NPCPearlGoods as i16, ServerPacketIds::NPCRequestInput as i16, ServerPacketIds::NewChatItem as i16, ServerPacketIds::HeroHealthChanged as i16, ServerPacketIds::GainHeroExperience as i16, ServerPacketIds::HeroLevelChanged as i16, ServerPacketIds::NewIntelligentCreature as i16, ServerPacketIds::IntelligentCreatureEnableRename as i16, ServerPacketIds::IntelligentCreaturePickup as i16, ServerPacketIds::ResizeInventory as i16, ServerPacketIds::ResizeStorage as i16, ServerPacketIds::PlayerUpdate as i16, ServerPacketIds::NewMonsterInfo as i16, ServerPacketIds::NewNPCInfo as i16, ServerPacketIds::StoreItem as i16, ServerPacketIds::TakeBackItem as i16, ServerPacketIds::RemoveSlotItem as i16, ServerPacketIds::RetrieveTradeItem as i16, ServerPacketIds::AllowObserve as i16, ServerPacketIds::AddBuff as i16, ServerPacketIds::RemoveBuff as i16, ServerPacketIds::PlayerInspect as i16, ServerPacketIds::UpdateIntelligentCreatureList as i16, ServerPacketIds::ChangeHero as i16, ServerPacketIds::MarriageRequest as i16, ServerPacketIds::LoverUpdate as i16, ServerPacketIds::DivorceRequest as i16, ServerPacketIds::ObjectColourChanged as i16, ServerPacketIds::ManageHeroes as i16, ServerPacketIds::NewHero as i16, ServerPacketIds::SetHeroBehaviour as i16, ServerPacketIds::SetAutoPotValue as i16, ServerPacketIds::SetAutoPotItem as i16, ServerPacketIds::HeroInformation as i16, ServerPacketIds::AddMember as i16, ServerPacketIds::SwitchGroup as i16, ServerPacketIds::CancelReincarnation as i16, ServerPacketIds::GuildStorageGoldChange as i16, ServerPacketIds::GuildStorageItemChange as i16, ServerPacketIds::NewHeroInfo as i16, ServerPacketIds::TakeBackHeroItem as i16, ServerPacketIds::TransferHeroItem as i16, ServerPacketIds::UnlockHeroAutoPot as i16, ServerPacketIds::ChangePasswordBanned as i16, ServerPacketIds::DefaultNPC as i16, ServerPacketIds::DepositRefineItem as i16, ServerPacketIds::RefineCancel as i16, ServerPacketIds::RefineItem as i16, ServerPacketIds::RetrieveRefineItem as i16, ServerPacketIds::HeroCreateRequest as i16, ServerPacketIds::UpdateHeroSpawnState as i16, ServerPacketIds::Magic as i16, ServerPacketIds::MapInformation as i16, ServerPacketIds::SearchMapResult as i16, ServerPacketIds::WorldMapSetup as i16, ServerPacketIds::NPCCheckRefine as i16, ServerPacketIds::NPCCollectRefine as i16, ServerPacketIds::NPCRefine as i16, ServerPacketIds::NPCRepair as i16, ServerPacketIds::NPCReplaceWedRing as i16, ServerPacketIds::NPCSRepair as i16, ServerPacketIds::NPCSell as i16, ServerPacketIds::NewItemInfo as i16, ServerPacketIds::RepairItem as i16, ServerPacketIds::SplitItem1 as i16, ServerPacketIds::ObjectHero as i16, ServerPacketIds::ObjectHidden as i16, ServerPacketIds::UserSlotsRefresh as i16];
    let handled = HANDLED.contains(&opcode);
    match opcode {
        // ---- M41: 合成 ----
        x if x == ServerPacketIds::CraftItem as i16 => {
            // #2573：SharedRust canonical wire [unique_id u64][count u16][success u8] = 11B
            let body = &payload[PacketHeader::HEADER_SIZE..];
            if body.len() >= 11 {
                let unique_id = u64::from_le_bytes(body[0..8].try_into().unwrap_or([0; 8]));
                let count = u16::from_le_bytes(body[8..10].try_into().unwrap_or([0; 2]));
                let success = body[10] != 0;
                let recipe_id = unique_id as u32;
                server_events.write(ServerEvent::CraftResult { recipe_id, count, success });
                tracing::info!("🔧 CraftItem: recipe={} count={} success={}", recipe_id, count, success);
            }
        }
        // ---- M42: 物品租赁 ----
        x if x == ServerPacketIds::ItemRentalRequest as i16 => {
            // #2720：C# `S.ItemRentalRequest{Name, Renting}` —— 两端各收一份定角色
            match mir2_shared::packets::server::rental_system::ItemRentalRequest::read_body(
                &mut std::io::Cursor::new(&payload[PacketHeader::HEADER_SIZE..]),
            ) {
                Ok(p) => {
                    server_events.write(ServerEvent::RentalRequest {
                        renting: p.renting,
                        name: p.name.clone(),
                    });
                    tracing::info!("📦 租赁会话建立: renting={} 对方={}", p.renting, p.name);
                }
                Err(e) => tracing::warn!("📦 ItemRentalRequest 解析失败: {}", e),
            }
        }
        x if x == ServerPacketIds::UpdateRentalItem as i16 => {
            // #2720：C# `{HasData, LoanItem}` + Rust 扩展 [fee u32][period i32]
            match mir2_shared::packets::server::rental_system::UpdateRentalItem::read_body(
                &mut std::io::Cursor::new(&payload[PacketHeader::HEADER_SIZE..]),
            ) {
                Ok(p) => {
                    server_events.write(ServerEvent::RentalItemUpdate {
                        item: p.item.as_ref().map(to_inv_item),
                        fee: p.rental_fee,
                        period: p.rental_period,
                    });
                    tracing::info!(
                        "📦 UpdateRentalItem: item={} fee={} period={}",
                        p.item.is_some(),
                        p.rental_fee,
                        p.rental_period
                    );
                }
                Err(e) => tracing::warn!("📦 UpdateRentalItem 解析失败: {}", e),
            }
        }
        x if x == ServerPacketIds::ItemRentalFee as i16 => {
            let body = &payload[PacketHeader::HEADER_SIZE..];
            if body.len() >= 4 {
                let fee = u32::from_le_bytes(body[0..4].try_into().unwrap_or([0; 4]));
                server_events.write(ServerEvent::RentalFee { fee });
            }
        }
        x if x == ServerPacketIds::ItemRentalPeriod as i16 => {
            let body = &payload[PacketHeader::HEADER_SIZE..];
            if body.len() >= 4 {
                let period = i32::from_le_bytes(body[0..4].try_into().unwrap_or([0; 4]));
                server_events.write(ServerEvent::RentalPeriod { period });
            }
        }
        x if x == ServerPacketIds::DepositRentalItem as i16 => {
            // [uid u64][success u8]
            let body = &payload[PacketHeader::HEADER_SIZE..];
            if body.len() >= 9 {
                let uid = u64::from_le_bytes(body[0..8].try_into().unwrap_or([0; 8]));
                let success = body[8] != 0;
                server_events.write(ServerEvent::RentalDeposit { uid, success });
                tracing::info!("📦 存入租赁物品 uid={} success={}", uid, success);
            }
        }
        x if x == ServerPacketIds::RetrieveRentalItem as i16 => {
            let body = &payload[PacketHeader::HEADER_SIZE..];
            if body.len() >= 9 {
                let uid = u64::from_le_bytes(body[0..8].try_into().unwrap_or([0; 8]));
                let success = body[8] != 0;
                server_events.write(ServerEvent::RentalRetrieve { uid, success });
            }
        }
        x if x == ServerPacketIds::ItemRentalLock as i16 => {
            // #2720：C# `{Success, GoldLocked, ItemLocked}`
            match mir2_shared::packets::server::rental_system::ItemRentalLock::read_body(
                &mut std::io::Cursor::new(&payload[PacketHeader::HEADER_SIZE..]),
            ) {
                Ok(p) => {
                    server_events.write(ServerEvent::RentalLocked {
                        gold_locked: p.gold_locked,
                        item_locked: p.item_locked,
                    });
                    tracing::info!(
                        "📦 租赁锁定（本侧）: gold={} item={}",
                        p.gold_locked,
                        p.item_locked
                    );
                }
                Err(e) => tracing::warn!("📦 ItemRentalLock 解析失败: {}", e),
            }
        }
        x if x == ServerPacketIds::ItemRentalPartnerLock as i16 => {
            // #2720：C# `{GoldLocked, ItemLocked}`
            match mir2_shared::packets::server::rental_system::ItemRentalPartnerLock::read_body(
                &mut std::io::Cursor::new(&payload[PacketHeader::HEADER_SIZE..]),
            ) {
                Ok(p) => {
                    server_events.write(ServerEvent::RentalPartnerLocked {
                        gold_locked: p.gold_locked,
                        item_locked: p.item_locked,
                    });
                    tracing::info!(
                        "📦 租赁锁定（对方）: gold={} item={}",
                        p.gold_locked,
                        p.item_locked
                    );
                }
                Err(e) => tracing::warn!("📦 ItemRentalPartnerLock 解析失败: {}", e),
            }
        }
        x if x == ServerPacketIds::CanConfirmItemRental as i16 => {
            let body = &payload[PacketHeader::HEADER_SIZE..];
            let can_confirm = body.first().copied().unwrap_or(0) != 0;
            server_events.write(ServerEvent::RentalCanConfirm { can_confirm });
            tracing::info!("📦 CanConfirmItemRental: {}", can_confirm);
        }
        x if x == ServerPacketIds::ConfirmItemRental as i16 => {
            let body = &payload[PacketHeader::HEADER_SIZE..];
            let success = body.first().copied().unwrap_or(0) != 0;
            server_events.write(ServerEvent::RentalConfirmed { success });
            tracing::info!("📦 ConfirmItemRental: {}", success);
        }
        x if x == ServerPacketIds::CancelItemRental as i16 => {
            server_events.write(ServerEvent::RentalCancelled);
            tracing::info!("📦 租赁取消");
        }
        // #274：智能宠物 / 仓库扩容
        x if x == ServerPacketIds::NewIntelligentCreature as i16 => {
            if let Ok(p) = special_systems::NewIntelligentCreature::read_body(&mut cur) {
                server_events.write(ServerEvent::CreatureAcquired {
                    creature_type: p.creature_type as u8,
                });
                tracing::info!("🐾 获得新宠物 type={:?}", p.creature_type);
            }
        }
        x if x == ServerPacketIds::IntelligentCreatureEnableRename as i16 => {
            if let Ok(p) = special_systems::IntelligentCreatureEnableRename::read_body(&mut cur) {
                server_events.write(ServerEvent::CreatureRenameEnabled {
                    can_rename: p.can_rename,
                });
                tracing::info!("✏️ 宠物可重命名: {}", p.can_rename);
            }
        }
        x if x == ServerPacketIds::IntelligentCreaturePickup as i16 => {
            if let Ok(p) = special_systems::IntelligentCreaturePickup::read_body(&mut cur) {
                server_events.write(ServerEvent::CreaturePickupToggled { enabled: p.enabled });
                tracing::info!("🎒 宠物拾取模式: {}", p.enabled);
            }
        }
        x if x == ServerPacketIds::ResizeStorage as i16 => {
            if let Ok(p) = ui_events::ResizeStorage::read_body(&mut cur) {
                server_events.write(ServerEvent::StorageResized {
                    size: p.size.max(0) as usize,
                });
                tracing::info!("📦 仓库扩容: {}", p.size);
            }
        }
        x if x == ServerPacketIds::ResizeInventory as i16 => {
            if let Ok(p) = ui_events::ResizeInventory::read_body(&mut cur) {
                server_events.write(ServerEvent::InventoryResized {
                    size: p.size.max(0) as usize,
                });
                tracing::info!("🎒 背包扩容: {}", p.size);
            }
        }
        // #279：玩家外观刷新（换装/光照）
        x if x == ServerPacketIds::PlayerUpdate as i16 => {
            if let Ok(p) = player::PlayerUpdate::read_body(&mut cur) {
                server_events.write(ServerEvent::PlayerUpdate {
                    object_id: p.object_id,
                    light: p.light,
                    weapon: p.weapon,
                    weapon_effect: p.weapon_effect,
                    armor: p.armor,
                    wings_effect: p.wings_effect,
                });
                tracing::info!("🧍 PlayerUpdate id={} weapon={} armor={}", p.object_id, p.weapon, p.armor);
            }
        }
        // #279：怪物/NPC 信息缓存
        x if x == ServerPacketIds::NewMonsterInfo as i16 => {
            if let Ok(p) = info::NewMonsterInfo::read_body(&mut cur) {
                server_events.write(ServerEvent::MonsterInfo { info: p.info });
            }
        }
        x if x == ServerPacketIds::NewNPCInfo as i16 => {
            if let Ok(p) = info::NewNPCInfo::read_body(&mut cur) {
                server_events.write(ServerEvent::NpcInfo { info: p.info });
            }
        }
        // #279：仓库存取/槽位移除/交易取回/观察许可 回执
        x if x == ServerPacketIds::StoreItem as i16 => {
            if let Ok(p) = item_operations::StoreItem::read_body(&mut cur) {
                server_events.write(ServerEvent::ItemStored { from: p.from, to: p.to, success: p.success });
                tracing::info!("📦 StoreItem {} -> {} success={}", p.from, p.to, p.success);
            }
        }
        x if x == ServerPacketIds::TakeBackItem as i16 => {
            if let Ok(p) = item_operations::TakeBackItem::read_body(&mut cur) {
                server_events.write(ServerEvent::ItemTakenBack { from: p.from, to: p.to, success: p.success });
                tracing::info!("📦 TakeBackItem {} -> {} success={}", p.from, p.to, p.success);
            }
        }
        x if x == ServerPacketIds::RemoveSlotItem as i16 => {
            if let Ok(p) = item_operations::RemoveSlotItem::read_body(&mut cur) {
                server_events.write(ServerEvent::SlotItemRemoved {
                    grid: p.grid,
                    grid_to: p.grid_to,
                    unique_id: p.unique_id,
                    to: p.to,
                    success: p.success,
                });
                tracing::info!("🗑️ RemoveSlotItem uid={} -> {} success={}", p.unique_id, p.to, p.success);
            }
        }
        x if x == ServerPacketIds::RetrieveTradeItem as i16 => {
            if let Ok(p) = miscellaneous::RetrieveTradeItem::read_body(&mut cur) {
                server_events.write(ServerEvent::TradeItemRetrieved { from_slot: p.from_slot, success: p.success });
                tracing::info!("🔄 RetrieveTradeItem slot={} success={}", p.from_slot, p.success);
            }
        }
        x if x == ServerPacketIds::AllowObserve as i16 => {
            if let Ok(p) = miscellaneous::AllowObserve::read_body(&mut cur) {
                server_events.write(ServerEvent::ObserveAllowed { allowed: p.allowed });
                tracing::info!("👀 AllowObserve allowed={}", p.allowed);
            }
        }

        // #285：聊天物品信息（C# S.NewChatItem：完整 UserItem）
        x if x == ServerPacketIds::NewChatItem as i16 => {
            if let Ok(p) = miscellaneous::NewChatItem::read_body(&mut cur) {
                let item = to_inv_item(&p.item);
                server_events.write(ServerEvent::ChatItemReceived { item: item.clone() });
                tracing::info!("💬 聊天物品: {} (uid={})", item.name, item.unique_id);
            }
        }
        // #270：英雄状态/经验/等级
        x if x == ServerPacketIds::HeroHealthChanged as i16 => {
            if let Ok(p) = combat::HeroHealthChanged::read_body(&mut cur) {
                server_events.write(ServerEvent::HeroHealthChanged { hp: p.hp, mp: p.mp });
                tracing::debug!("⭐ 英雄 HP/MP {}/{}", p.hp, p.mp);
            }
        }
        x if x == ServerPacketIds::GainHeroExperience as i16 => {
            if let Ok(p) = experience::GainHeroExperience::read_body(&mut cur) {
                server_events.write(ServerEvent::GainHeroExperience { amount: p.amount });
                tracing::debug!("⭐ 英雄经验 +{}", p.amount);
            }
        }
        x if x == ServerPacketIds::HeroLevelChanged as i16 => {
            if let Ok(p) = experience::HeroLevelChanged::read_body(&mut cur) {
                server_events.write(ServerEvent::HeroLevelChanged {
                    level: p.level,
                    exp: p.experience,
                    max_exp: p.max_experience,
                });
                tracing::debug!("⭐ 英雄等级 Lv.{}", p.level);
            }
        }

        // #268：杂项协议（租赁/基础属性/觉醒拆卸/行会Buff/珍珠/NPC输入）
        x if x == ServerPacketIds::GetRentedItems as i16 => {
            if let Ok(p) = rental_system::GetRentedItems::read_body(&mut cur) {
                server_events.write(ServerEvent::RentedItems { items: p.items });
                tracing::info!("📦 租赁物品列表已下发");
            }
        }
        x if x == ServerPacketIds::BaseStatsInfo as i16 => {
            if let Ok(p) = miscellaneous::BaseStatsInfo::read_body(&mut cur) {
                server_events.write(ServerEvent::BaseStats { stats: p.stats });
            }
        }
        x if x == ServerPacketIds::HeroBaseStatsInfo as i16 => {
            if let Ok(p) = miscellaneous::HeroBaseStatsInfo::read_body(&mut cur) {
                tracing::info!("⭐ 英雄基础属性: {} 项", p.stats.len());
            }
        }
        x if x == ServerPacketIds::NPCAwakening as i16 => {
            // #1356：C# S.NPCAwakening → 打开觉醒面板
            if awakening_system::NPCAwakening::read_body(&mut cur).is_ok() {
                server_events.write(ServerEvent::NpcAwakePanel { service: 0 });
                tracing::debug!("⚒️ NPC 觉醒面板");
            }
        }
        x if x == ServerPacketIds::NPCDisassemble as i16 => {
            // #1356：C# S.NPCDisassemble → 打开分解面板
            if awakening_system::NPCDisassemble::read_body(&mut cur).is_ok() {
                server_events.write(ServerEvent::NpcAwakePanel { service: 1 });
                tracing::debug!("🔧 NPC 拆卸面板");
            }
        }
        x if x == ServerPacketIds::NPCDowngrade as i16 => {
            // #1356：C# S.NPCDowngrade → 打开降级面板
            if awakening_system::NPCDowngrade::read_body(&mut cur).is_ok() {
                server_events.write(ServerEvent::NpcAwakePanel { service: 2 });
                tracing::debug!("⬇️ NPC 降级面板");
            }
        }
        x if x == ServerPacketIds::NPCReset as i16 => {
            // #1356：C# S.NPCReset → 打开重置面板
            if awakening_system::NPCReset::read_body(&mut cur).is_ok() {
                server_events.write(ServerEvent::NpcAwakePanel { service: 3 });
                tracing::debug!("🔄 NPC 重置面板");
            }
        }
        x if x == ServerPacketIds::GuildBuffList as i16 => {
            // #2537：行会技能——激活列表 + Buff 目录入 GuildState（原仅打日志）
            if let Ok(p) = special_systems::GuildBuffList::read_body(&mut cur) {
                let (n_active, n_catalog) = (p.active_buffs.len(), p.guild_buffs.len());
                server_events.write(ServerEvent::GuildBuffList {
                    active: p.active_buffs,
                    catalog: p.guild_buffs,
                });
                tracing::info!("🏴 行会技能: 激活 {} / 目录 {}", n_active, n_catalog);
            }
        }
        x if x == ServerPacketIds::NPCPearlGoods as i16 => {
            // #珍珠商店：S.NPCPearlGoods（List<UserItem>+Rate f32+Type u8）
            if let Ok(p) = special_systems::NPCPearlGoods::read_body(&mut cur) {
                let goods: Vec<crate::game::dialogs::npc_goods::GoodsEntry> = p
                    .list
                    .iter()
                    .map(|item| crate::game::dialogs::npc_goods::GoodsEntry {
                        item_index: item.item_index,
                        unique_id: item.unique_id,
                        name: item
                            .info
                            .as_ref()
                            .map(|i| i.name.clone())
                            .unwrap_or_else(|| format!("#{}", item.item_index)),
                        price: item.info.as_ref().map(|i| i.price).unwrap_or(0),
                        count: item.count,
                        image: item.info.as_ref().map(|i| i.image).unwrap_or(0),
                        item_type: item.info.as_ref().map(|i| i.item_type as u8).unwrap_or(0),
                        tool_tip: item.info.as_ref().and_then(|i| i.tool_tip.clone()),
                        stack_size: item.info.as_ref().map(|i| i.stack_size).unwrap_or(1),
                    })
                    .collect();
                let rate = p.rate;
                let pearl_count = goods.len();
                server_events.write(ServerEvent::PearlShop { goods, rate });
                tracing::info!("🫧 NPC 珍珠商品: {} 件 (rate={})", pearl_count, rate);
            }
        }
        x if x == ServerPacketIds::NPCRequestInput as i16 => {
            if let Ok(p) = npc::NPCRequestInput::read_body(&mut cur) {
                tracing::info!("⌨️ NPC 请求输入: npc={} page={}", p.npc_id, p.page_name);
                server_events.write(ServerEvent::NpcInputRequest {
                    npc_id: p.npc_id,
                    page_name: p.page_name,
                });
            }
        }

        // #262：配方 / Buff 暂停 / 杂项
        x if x == ServerPacketIds::NewRecipeInfo as i16 => {
            if let Ok(p) = ui_events::NewRecipeInfo::read_body(&mut cur) {
                server_events.write(ServerEvent::RecipeLearned {
                    recipe_id: p.recipe_id,
                    info: p.info,
                });
                tracing::info!("📖 学会配方 #{}", p.recipe_id);
            }
        }
        x if x == ServerPacketIds::PauseBuff as i16 => {
            if let Ok(p) = buff::PauseBuff::read_body(&mut cur) {
                server_events.write(ServerEvent::BuffPaused {
                    buff_type: p.buff_type as u8,
                    object_id: p.object_id,
                    paused: p.paused,
                });
                tracing::info!("⏸️ Buff 暂停 id={} paused={}", p.object_id, p.paused);
            }
        }
        x if x == ServerPacketIds::RefreshItem as i16 => {
            if let Ok(p) = item::RefreshItem::read_body(&mut cur) {
                tracing::debug!("🔄 刷新物品 uid={}", p.item.unique_id);
            }
        }
        x if x == ServerPacketIds::SetBindingShot as i16 => {
            if let Ok(p) = ui_events::SetBindingShot::read_body(&mut cur) {
                tracing::debug!("🎯 定身射击 id={} enabled={} value={}", p.object_id, p.enabled, p.value);
            }
        }

        // #260：任务数据包
        x if x == ServerPacketIds::NewQuestInfo as i16 => {
            if let Ok(p) = quest::NewQuestInfo::read_body(&mut cur) {
                // #2535：完整定义交 QuestCatalog（可接任务/奖励由目录推导），不再当已接任务写日志
                server_events.write(ServerEvent::QuestInfo { info: p.quest });
            }
        }
        x if x == ServerPacketIds::ShareQuest as i16 => {
            if let Ok(p) = miscellaneous::ShareQuest::read_body(&mut cur) {
                server_events.write(ServerEvent::QuestShared {
                    quest_id: p.quest_id,
                });
                tracing::info!("🔗 共享任务 #{}", p.quest_id);
            }
        }
        x if x == ServerPacketIds::GainedQuestItem as i16 => {
            // #1342：C# S.GainedQuestItem 携带完整 UserItem
            if let Ok(p) = miscellaneous::GainedQuestItem::read_body(&mut cur) {
                server_events.write(ServerEvent::QuestItemGained { item: to_inv_item(&p.item) });
                tracing::info!("🎁 任务物品获得 #{}", p.item.item_index);
            }
        }
        x if x == ServerPacketIds::DeleteQuestItem as i16 => {
            // #1342：C# S.DeleteQuestItem UniqueID u64 + Count u16
            if let Ok(p) = miscellaneous::DeleteQuestItem::read_body(&mut cur) {
                server_events.write(ServerEvent::QuestItemDeleted { unique_id: p.unique_id, count: p.count });
                tracing::info!("🗑️ 任务物品删除 uid={} x{}", p.unique_id, p.count);
            }
        }

        // ---- M43: 任务日志 ----
        x if x == ServerPacketIds::ChangeQuest as i16 => {
            // [id i32][count i32][task dotnet...][taken u8][completed u8][new u8]
            let body = &payload[PacketHeader::HEADER_SIZE..];
            let mut cur = std::io::Cursor::new(body);
            use byteorder::{LittleEndian, ReadBytesExt};
            let id = match cur.read_i32::<LittleEndian>() { Ok(v) => v, Err(_) => { tracing::warn!("⚠️ ChangeQuest 解析失败"); return true; } };
            let count = cur.read_i32::<LittleEndian>().unwrap_or(0).max(0) as usize;
            let mut tasks = Vec::with_capacity(count);
            let mut ok = true;
            for _ in 0..count {
                match mir2_shared::binary::read_dotnet_string(&mut cur) {
                    Ok(t) => tasks.push(t),
                    Err(_) => { ok = false; break; }
                }
            }
            if !ok { tracing::warn!("⚠️ ChangeQuest 任务解析失败"); return true; }
            let taken = cur.read_u8().unwrap_or(0) != 0;
            let completed = cur.read_u8().unwrap_or(0) != 0;
            let is_new = cur.read_u8().unwrap_or(0) != 0;
            let name = tasks.first().cloned().unwrap_or_else(|| format!("#{}", id));
            let entry = QuestEntry { id, name, tasks, taken, completed, is_new };
            server_events.write(ServerEvent::QuestChanged { entry });
            tracing::info!("📜 ChangeQuest: id={} completed={}", id, completed);
        }
        x if x == ServerPacketIds::CompleteQuest as i16 => {
            // [quest_index i32]
            let body = &payload[PacketHeader::HEADER_SIZE..];
            if body.len() >= 4 {
                let id = i32::from_le_bytes(body[0..4].try_into().unwrap_or([0; 4]));
                server_events.write(ServerEvent::QuestCompleted { id });
                tracing::info!("📜 CompleteQuest: {}", id);
            }
        }
        // ---- M44: 状态/Buff ----
        x if x == ServerPacketIds::AddBuff as i16 => {
            // #2791 单元④：[tag u8][remaining_ms u32][paused u8][value_count u8][values i32…]
            let body = &payload[PacketHeader::HEADER_SIZE..];
            if let Some((tag, remaining_ms, paused, values)) = parse_add_buff_body(body) {
                server_events.write(ServerEvent::BuffAdded {
                    tag,
                    remaining_ms,
                    paused,
                    values: values.clone(),
                });
                tracing::info!(
                    "✨ AddBuff: tag={} {}ms paused={} values={:?}",
                    tag,
                    remaining_ms,
                    paused,
                    values
                );
            }
        }
        x if x == ServerPacketIds::RemoveBuff as i16 => {
            // [tag u8]
            let body = &payload[PacketHeader::HEADER_SIZE..];
            if let Some(tag) = body.first().copied() {
                server_events.write(ServerEvent::BuffRemoved { tag });
                tracing::info!("✨ RemoveBuff: tag={}", tag);
            }
        }
        // ---- M46: 查看玩家 ----
        x if x == ServerPacketIds::PlayerInspect as i16 => {
            // [object_id u32][name dotnet][guild dotnet][level u16][class u8][gender u8]
            // [lover_name dotnet][allow_observe u8][count u8][per: slot u8][uid u64][index i32][image i32][dura i32][max_dura i32]
            // （#2607：slot/image 新增；#2611：allow_observe 新增——Observe 按钮门控；
            //  #2786：lover_name 新增——观察窗伴侣钮 Hint，服务端 `inspect_identity_bytes` 同序）
            let body = &payload[PacketHeader::HEADER_SIZE..];
            let mut cur = std::io::Cursor::new(body);
            use byteorder::{LittleEndian, ReadBytesExt};
            let Some(id) = parse_inspect_identity(&mut cur) else {
                tracing::warn!("⚠️ PlayerInspect 解析失败");
                return true;
            };
            let InspectIdentity {
                name,
                guild,
                level,
                class,
                gender,
                lover_name,
                allow_observe,
                ..
            } = id;
            let count = cur.read_u8().unwrap_or(0) as usize;
            let mut items = Vec::with_capacity(count);
            let mut ok = true;
            for _ in 0..count {
                let slot = match cur.read_u8() { Ok(v) => v, Err(_) => { ok = false; break; } };
                let unique_id = match cur.read_u64::<LittleEndian>() { Ok(v) => v, Err(_) => { ok = false; break; } };
                let item_index = match cur.read_i32::<LittleEndian>() { Ok(v) => v, Err(_) => { ok = false; break; } };
                let image = match cur.read_i32::<LittleEndian>() { Ok(v) => v, Err(_) => { ok = false; break; } };
                let current_dura = match cur.read_i32::<LittleEndian>() { Ok(v) => v, Err(_) => { ok = false; break; } };
                let max_dura = match cur.read_i32::<LittleEndian>() { Ok(v) => v, Err(_) => { ok = false; break; } };
                items.push(InspectItem { slot, unique_id, item_index, image, current_dura, max_dura });
            }
            if ok {
                let item_count = items.len();
                server_events.write(ServerEvent::InspectPlayer {
                    name: name.clone(),
                    guild,
                    level,
                    class,
                    gender,
                    lover_name,
                    allow_observe,
                    items,
                });
                tracing::info!(
                    "🔍 PlayerInspect: {} Lv.{} 装备 {} 件",
                    name,
                    level,
                    item_count
                );
            } else {
                tracing::warn!("⚠️ PlayerInspect 装备解析失败");
            }
        }
        // ---- M47: 宠物 ----
        x if x == ServerPacketIds::UpdateIntelligentCreatureList as i16 => {
            // [count i32][per: type u8][pickup u8][enabled u8][hunger u8][name dotnet]
            // [active u8][filter 9×u8][grade u8][rules: minimal i32][mouse u8][mouseR i32]
            // [auto u8][autoR i32][semi u8][semiR i32][blackstone u8]（#2757 起）
            // [icon i32][fullness i32][expire i64][blackstone_time i32]（#2761 起）
            // 包尾：[creature_summoned u8][summoned_type u8][pearl_count i32]（#2761，C# 尾部三字段）
            let body = &payload[PacketHeader::HEADER_SIZE..];
            let mut cur = std::io::Cursor::new(body);
            use byteorder::{LittleEndian, ReadBytesExt};
            let count = cur.read_i32::<LittleEndian>().unwrap_or(0).max(0) as usize;
            let mut creatures = Vec::with_capacity(count);
            let mut ok = true;
            for _ in 0..count {
                let creature_type = match cur.read_u8() { Ok(v) => v, Err(_) => { ok = false; break; } };
                let pickup_mode = match cur.read_u8() { Ok(v) => v, Err(_) => { ok = false; break; } };
                let enabled = match cur.read_u8() { Ok(v) => v, Err(_) => { ok = false; break; } } != 0;
                let hunger = match cur.read_u8() { Ok(v) => v, Err(_) => { ok = false; break; } };
                let name = match mir2_shared::binary::read_dotnet_string(&mut cur) { Ok(v) => v, Err(_) => { ok = false; break; } };
                let active = match cur.read_u8() { Ok(v) => v, Err(_) => { ok = false; break; } } != 0;
                let mut filter = [0u8; 9];
                for b in filter.iter_mut() {
                    *b = match cur.read_u8() { Ok(v) => v, Err(_) => { ok = false; break; } };
                }
                let grade = match cur.read_u8() { Ok(v) => v, Err(_) => { ok = false; break; } };
                // #2757：宠物规则（C# `IntelligentCreatureRules`）——字段不足（旧服务端）即全禁用默认
                let rules = mir2_shared::data::client_data::IntelligentCreatureRules::read_from(
                    &mut cur,
                )
                .unwrap_or_default();
                // #2761：图标/完整度/到期剩余秒/黑石计时——字段不足（旧服务端）即取 0
                let icon = cur.read_i32::<LittleEndian>().unwrap_or(0);
                let fullness = cur.read_i32::<LittleEndian>().unwrap_or(0);
                let expire_secs = cur.read_i64::<LittleEndian>().unwrap_or(0);
                let blackstone_time = cur.read_i32::<LittleEndian>().unwrap_or(0);
                creatures.push(CreatureEntry {
                    creature_type,
                    pickup_mode,
                    enabled,
                    hunger,
                    name,
                    active,
                    filter,
                    grade,
                    rules,
                    icon,
                    fullness,
                    expire_secs,
                    blackstone_time,
                });
            }
            // #2761：C# `S.UpdateIntelligentCreatureList` 包尾三字段（缺字段 = 旧服务端，取默认）
            let summoned = cur.read_u8().unwrap_or(0) != 0;
            let summoned_type = cur.read_u8().unwrap_or(0);
            let pearl_count = cur.read_i32::<LittleEndian>().unwrap_or(0);
            if ok {
                let count = creatures.len();
                server_events.write(ServerEvent::CreatureList {
                    creatures,
                    summoned,
                    summoned_type,
                    pearl_count,
                });
                tracing::info!("🐾 宠物列表: {} 个", count);
            } else {
                tracing::warn!("⚠️ UpdateIntelligentCreatureList 解析失败");
            }
        }
        // ---- M48: 英雄 ----
        x if x == ServerPacketIds::ChangeHero as i16 => {
            // [hero_index u8]
            let body = &payload[PacketHeader::HEADER_SIZE..];
            let idx = body.first().copied().unwrap_or(0);
            server_events.write(ServerEvent::HeroChanged { index: idx });
            tracing::info!("🦸 ChangeHero: index={}", idx);
        }
        // ---- M49: 婚姻/关系 ----
        x if x == ServerPacketIds::MarriageRequest as i16 => {
            // [lover dotnet]
            let body = &payload[PacketHeader::HEADER_SIZE..];
            let mut cur = std::io::Cursor::new(body);
            match mir2_shared::binary::read_dotnet_string(&mut cur) {
                Ok(name) => {
                    server_events.write(ServerEvent::MarriageInvite { name: name.clone() });
                    tracing::info!("💍 收到求婚: {}", name);
                }
                Err(_) => tracing::warn!("⚠️ MarriageRequest 解析失败"),
            }
        }
        x if x == ServerPacketIds::LoverUpdate as i16 => {
            // #1329：全量 [Name dotnet][Date i64][MapName dotnet][MarriedDays i16]（C# S.LoverUpdate）
            match social_system::LoverUpdate::read_body(&mut cur) {
                Ok(p) => {
                    tracing::info!("💍 LoverUpdate: lover_name={}", p.lover_name);
                    server_events.write(ServerEvent::LoverUpdate {
                        lover_name: p.lover_name,
                        date: p.date,
                        map_name: p.map_name,
                        married_days: p.married_days,
                    });
                }
                Err(_) => tracing::warn!("⚠️ LoverUpdate 解析失败"),
            }
        }
        x if x == ServerPacketIds::DivorceRequest as i16 => {
            server_events.write(ServerEvent::DivorceRequest);
            tracing::info!("💔 收到离婚请求");
        }

        x if x == ServerPacketIds::ObjectColourChanged as i16 => {
            // C# S.ObjectColourChanged：PK 名字染色（object_id + ARGB）
            if let Ok(p) = buff::ObjectColourChanged::read_body(&mut cur) {
                server_events.write(ServerEvent::ObjectColourChanged {
                    object_id: p.object_id,
                    name_colour_argb: p.name_colour_argb,
                });
                tracing::debug!("🎨 名字染色: obj={} argb={}", p.object_id, p.name_colour_argb);
            }
        }
        x if x == ServerPacketIds::ManageHeroes as i16 => {
            // C# S.ManageHeroes：英雄列表（max_count + current + heroes）
            if let Ok(p) = hero::ManageHeroes::read_body(&mut cur) {
                server_events.write(ServerEvent::HeroManageReceived {
                    heroes: p.heroes.clone(),
                    current: p.current_hero.clone(),
                    max_count: p.max_count,
                });
                tracing::info!(
                    "🦸 英雄列表: {} 个（总名额 {}）",
                    p.heroes.len(),
                    p.max_count
                );
            }
        }
        x if x == ServerPacketIds::NewHero as i16 => {
            // C# S.NewHero.Result（1 字节）
            if let Ok(p) = miscellaneous::NewHero::read_body(&mut cur) {
                server_events.write(ServerEvent::NewHeroResult { result: p.result });
                tracing::info!("🦸 创建英雄结果: {}", p.result);
            }
        }
        x if x == ServerPacketIds::SetHeroBehaviour as i16 => {
            // C# S.SetHeroBehaviour：1 字节 behaviour
            if let Ok(p) = hero::SetHeroBehaviour::read_body(&mut cur) {
                server_events.write(ServerEvent::HeroBehaviourSet { behaviour: p.behaviour as u8 });
                tracing::info!("🦸 英雄行为确认: {:?}", p.behaviour);
            }
        }
        x if x == ServerPacketIds::SetAutoPotValue as i16 => {
            if let Ok(p) = hero::SetAutoPotValue::read_body(&mut cur) {
                server_events.write(ServerEvent::HeroAutoPotSet { stat: p.stat, value: p.value });
                tracing::debug!("🦸 自动药阈值: stat={} value={}", p.stat, p.value);
            }
        }
        x if x == ServerPacketIds::SetAutoPotItem as i16 => {
            if let Ok(p) = miscellaneous::SetAutoPotItem::read_body(&mut cur) {
                server_events.write(ServerEvent::HeroAutoPotItemSet { grid: p.grid, item_index: p.item_index });
                tracing::debug!("🦸 自动药物品: grid={} item={}", p.grid, p.item_index);
            }
        }
        x if x == ServerPacketIds::HeroInformation as i16 => {
            // C# S.HeroInformation：英雄完整信息（含背包/装备/自动药，#203）
            if let Ok(p) = hero::HeroInformation::read_body(&mut cur) {
                let inventory: Vec<Option<InvItem>> = p
                    .inventory
                    .as_ref()
                    .map(|inv| inv.iter().map(|s| s.as_ref().map(to_inv_item)).collect())
                    .unwrap_or_default();
                let equipment: Vec<Option<InvItem>> = p
                    .equipment
                    .as_ref()
                    .map(|eq| eq.iter().map(|s| s.as_ref().map(to_inv_item)).collect())
                    .unwrap_or_default();
                server_events.write(ServerEvent::HeroInformation {
                    object_id: p.object_id,
                    name: p.name.clone(),
                    class: p.class as u8,
                    gender: p.gender as u8,
                    level: p.level,
                    hp: p.hp,
                    mp: p.mp,
                    exp: p.experience,
                    max_exp: p.max_experience.max(1),
                    inventory,
                    equipment,
                    magics: p.magics.clone(),
                    auto_pot: p.auto_pot,
                    auto_hp_percent: p.auto_hp_percent,
                    auto_mp_percent: p.auto_mp_percent,
                    hp_item_index: p.hp_item_index,
                    mp_item_index: p.mp_item_index,
                });
                tracing::info!(
                    "🦸 HeroInformation: {} Lv.{} 背包 {} 格 装备 {} 格",
                    p.name,
                    p.level,
                    p.inventory.as_ref().map(|v| v.len()).unwrap_or(0),
                    p.equipment.as_ref().map(|v| v.len()).unwrap_or(0)
                );
            }
        }
        // #291：C# 服务端包面收尾（AddMember）
        x if x == ServerPacketIds::AddMember as i16 => {
            if group::AddMember::read_body(&mut cur).is_ok() {
                tracing::info!("📦 AddMember 解码");
            }
        }
        // #291：C# 服务端包面收尾（SwitchGroup）——同步客户端“允许组队”开关
        x if x == ServerPacketIds::SwitchGroup as i16 => {
            if let Ok(p) = group::SwitchGroup::read_body(&mut cur) {
                tracing::info!("📦 SwitchGroup 解码 allow_group={}", p.allow_group);
                server_events.write(ServerEvent::GroupAllowChanged {
                    allow_group: p.allow_group,
                });
            }
        }
        // #291：C# 服务端包面收尾（CancelReincarnation）
        x if x == ServerPacketIds::CancelReincarnation as i16 => {
            if miscellaneous::CancelReincarnation::read_body(&mut cur).is_ok() {
                tracing::info!("📦 CancelReincarnation 解码");
            }
        }
        // #295：行会仓库金币变化（C# S.GuildStorageGoldChange）
        x if x == ServerPacketIds::GuildStorageGoldChange as i16 => {
            if let Ok(p) = miscellaneous::GuildStorageGoldChange::read_body(&mut cur) {
                server_events.write(ServerEvent::GuildStorageGoldChanged {
                    amount: p.amount,
                    change_type: p.change_type,
                    name: p.name.clone(),
                });
                // C#：金币变化在公会聊天提示
                server_events.write(ServerEvent::Chat {
                    text: format!(
                        "{} 向行会仓库{}了 {} 金币",
                        p.name,
                        if p.change_type == 0 { "存入" } else { "取出" },
                        p.amount
                    ),
                    chat_type: mir2_shared::enums::ChatType::Guild,
                });
                tracing::info!(
                    "💰 行会仓库金币 {} {} 金币（by {}）",
                    if p.change_type == 0 { "存入" } else { "取出" },
                    p.amount,
                    p.name
                );
            }
        }
        // #295：行会仓库物品变化（C# S.GuildStorageItemChange）
        x if x == ServerPacketIds::GuildStorageItemChange as i16 => {
            if let Ok(p) = miscellaneous::GuildStorageItemChange::read_body(&mut cur) {
                let item = p.item.as_ref().map(|(_, it)| to_inv_item(it));
                server_events.write(ServerEvent::GuildStorageItemChanged {
                    change_type: p.change_type,
                    to: p.to,
                    from: p.from,
                    item,
                });
                tracing::info!(
                    "📦 行会仓库物品变化 type={} to={} from={} has_item={}",
                    p.change_type,
                    p.to,
                    p.from,
                    p.item.is_some()
                );
            }
        }
        // #291：C# 服务端包面收尾（NewHeroInfo）
        x if x == ServerPacketIds::NewHeroInfo as i16 => {
            if miscellaneous::NewHeroInfo::read_body(&mut cur).is_ok() {
                tracing::info!("📦 NewHeroInfo 解码");
            }
        }
        // #291：C# 服务端包面收尾（TakeBackHeroItem）
        x if x == ServerPacketIds::TakeBackHeroItem as i16 => {
            if miscellaneous::TakeBackHeroItem::read_body(&mut cur).is_ok() {
                tracing::info!("📦 TakeBackHeroItem 解码");
            }
        }
        // #291：C# 服务端包面收尾（TransferHeroItem）
        x if x == ServerPacketIds::TransferHeroItem as i16 => {
            if miscellaneous::TransferHeroItem::read_body(&mut cur).is_ok() {
                tracing::info!("📦 TransferHeroItem 解码");
            }
        }
        // #291：C# 服务端包面收尾（UnlockHeroAutoPot）
        x if x == ServerPacketIds::UnlockHeroAutoPot as i16 => {
            if miscellaneous::UnlockHeroAutoPot::read_body(&mut cur).is_ok() {
                tracing::info!("📦 UnlockHeroAutoPot 解码");
            }
        }
        // #291：C# 服务端包面收尾（ChangePasswordBanned）
        x if x == ServerPacketIds::ChangePasswordBanned as i16 => {
            if login::ChangePasswordBanned::read_body(&mut cur).is_ok() {
                tracing::info!("📦 ChangePasswordBanned 解码");
            }
        }
        // #291：C# 服务端包面收尾（DefaultNPC）
        x if x == ServerPacketIds::DefaultNPC as i16 => {
            if npc_interaction::DefaultNPC::read_body(&mut cur).is_ok() {
                tracing::info!("📦 DefaultNPC 解码");
            }
        }
        // #2720：精炼材料存入确认（C# S.DepositRefineItem：[from][to][success]）
        x if x == ServerPacketIds::DepositRefineItem as i16 => {
            if let Ok(p) = item_operations::DepositRefineItem::read_body(&mut cur) {
                server_events.write(ServerEvent::RefineDeposited {
                    from: p.from,
                    to: p.to,
                    success: p.success,
                });
                tracing::info!("🔨 精炼材料存入 from={} to={} ok={}", p.from, p.to, p.success);
            }
        }
        // #2720：精炼取消/重置（C# GameScene.RefineCancel → RefineDialog.RefineReset）
        x if x == ServerPacketIds::RefineCancel as i16 => {
            if let Ok(p) = item_operations::RefineCancel::read_body(&mut cur) {
                server_events.write(ServerEvent::RefineCancelled { unlock: p.unlock });
                tracing::info!("🔨 精炼取消 unlock={}", p.unlock);
            }
        }
        // #2720：精炼开始确认（C# GameScene.RefineItem → RefineDialog.RefineReset）
        x if x == ServerPacketIds::RefineItem as i16 => {
            if let Ok(p) = item_operations::RefineItem::read_body(&mut cur) {
                server_events.write(ServerEvent::RefineStarted {
                    unique_id: p.unique_id,
                });
                tracing::info!("🔨 精炼开始 uid={}", p.unique_id);
            }
        }
        // #2720：精炼材料取回确认（C# S.RetrieveRefineItem：[from][to][success]）
        x if x == ServerPacketIds::RetrieveRefineItem as i16 => {
            if let Ok(p) = item_operations::RetrieveRefineItem::read_body(&mut cur) {
                server_events.write(ServerEvent::RefineRetrieved {
                    from: p.from,
                    to: p.to,
                    success: p.success,
                });
                tracing::info!("🔨 精炼材料取回 from={} to={} ok={}", p.from, p.to, p.success);
            }
        }
        // #291：C# 服务端包面收尾（HeroCreateRequest）
        x if x == ServerPacketIds::HeroCreateRequest as i16 => {
            if hero::HeroCreateRequest::read_body(&mut cur).is_ok() {
                tracing::info!("📦 HeroCreateRequest 解码");
            }
        }
        // #291：C# 服务端包面收尾（UpdateHeroSpawnState）
        x if x == ServerPacketIds::UpdateHeroSpawnState as i16 => {
            if hero::UpdateHeroSpawnState::read_body(&mut cur).is_ok() {
                tracing::info!("📦 UpdateHeroSpawnState 解码");
            }
        }
        // #291：C# 服务端包面收尾（Magic）
        x if x == ServerPacketIds::Magic as i16 => {
            if magic_combat::Magic::read_body(&mut cur).is_ok() {
                tracing::info!("📦 Magic 解码");
            }
        }
        // #291：C# 服务端包面收尾（MapInformation）
        x if x == ServerPacketIds::MapInformation as i16 => {
            if map::MapInformation::read_body(&mut cur).is_ok() {
                tracing::info!("📦 MapInformation 解码");
            }
        }
        // #291：C# 服务端包面收尾（SearchMapResult）
        x if x == ServerPacketIds::SearchMapResult as i16 => {
            if map::SearchMapResult::read_body(&mut cur).is_ok() {
                tracing::info!("📦 SearchMapResult 解码");
            }
        }
        // #300：世界地图（S.WorldMapSetupInfo → ServerEvent::WorldMapSetup，C# 线格式）
        x if x == ServerPacketIds::WorldMapSetup as i16 => {
            if let Ok(p) = map::WorldMapSetupInfo::read_body(&mut cur) {
                let n = p.world_maps.len();
                server_events.write(ServerEvent::WorldMapSetup {
                    enabled: p.enabled,
                    icons: p.world_maps,
                    teleport_cost: p.teleport_cost,
                });
                tracing::info!("📦 WorldMapSetup 解码（{} 个世界地图点, cost={}）", n, p.teleport_cost);
            }
        }
        // #2720：精炼入口/查看/收取（C# GameScene.NPCRefine / NPCCheckRefine / NPCCollectRefine）
        x if x == ServerPacketIds::NPCCheckRefine as i16 => {
            if npc::NPCCheckRefine::read_body(&mut cur).is_ok() {
                server_events.write(ServerEvent::NpcCheckRefinePanel);
                tracing::info!("🔨 NPC 精炼查看入口");
            }
        }
        x if x == ServerPacketIds::NPCCollectRefine as i16 => {
            if npc::NPCCollectRefine::read_body(&mut cur).is_ok() {
                server_events.write(ServerEvent::NpcCollectRefine);
                tracing::info!("🔨 NPC 精炼收取");
            }
        }
        x if x == ServerPacketIds::NPCRefine as i16 => {
            if let Ok(p) = npc::NPCRefine::read_body(&mut cur) {
                server_events.write(ServerEvent::NpcRefinePanel {
                    rate: p.rate,
                    refining: p.refining,
                });
                tracing::info!("🔨 NPC 精炼入口 rate={} refining={}", p.rate, p.refining);
            }
        }
        // #291：C# 服务端包面收尾（NPCRepair）
        x if x == ServerPacketIds::NPCRepair as i16 => {
            if npc::NPCRepair::read_body(&mut cur).is_ok() {
                tracing::info!("📦 NPCRepair 解码");
            }
        }
        // #291：C# 服务端包面收尾（NPCReplaceWedRing）
        x if x == ServerPacketIds::NPCReplaceWedRing as i16 => {
            if npc::NPCReplaceWedRing::read_body(&mut cur).is_ok() {
                tracing::info!("📦 NPCReplaceWedRing 解码");
            }
        }
        // #291：C# 服务端包面收尾（NPCSRepair）
        x if x == ServerPacketIds::NPCSRepair as i16 => {
            if npc::NPCSRepair::read_body(&mut cur).is_ok() {
                tracing::info!("📦 NPCSRepair 解码");
            }
        }
        // #291：C# 服务端包面收尾（NPCSell）
        x if x == ServerPacketIds::NPCSell as i16 => {
            if npc::NPCSell::read_body(&mut cur).is_ok() {
                tracing::info!("📦 NPCSell 解码");
            }
        }
        // #291：C# 服务端包面收尾（NewItemInfo）
        x if x == ServerPacketIds::NewItemInfo as i16 => {
            if item::NewItemInfo::read_body(&mut cur).is_ok() {
                tracing::info!("📦 NewItemInfo 解码");
            }
        }
        // #291：C# 服务端包面收尾（RepairItem）
        x if x == ServerPacketIds::RepairItem as i16 => {
            if item::RepairItem::read_body(&mut cur).is_ok() {
                tracing::info!("📦 RepairItem 解码");
            }
        }
        // #291：C# 服务端包面收尾（SplitItem1）
        x if x == ServerPacketIds::SplitItem1 as i16 => {
            if let Ok(p) = item::SplitItem1::read_body(&mut cur) {
                // #2742：C# `GameScene.SplitItem1` 在此解锁被拆分的来源格
                server_events.write(ServerEvent::SplitItem1Result {
                    unique_id: p.unique_id,
                });
                tracing::info!("📦 SplitItem1 解码 uid={}", p.unique_id);
            }
        }
        // #291：C# 服务端包面收尾（ObjectHero）
        x if x == ServerPacketIds::ObjectHero as i16 => {
            if objects::ObjectHero::read_body(&mut cur).is_ok() {
                tracing::info!("📦 ObjectHero 解码");
            }
        }
        // #291：C# 服务端包面收尾（ObjectHidden）
        x if x == ServerPacketIds::ObjectHidden as i16 => {
            if object::ObjectHidden::read_body(&mut cur).is_ok() {
                tracing::info!("📦 ObjectHidden 解码");
            }
        }
        // #291：C# 服务端包面收尾（UserSlotsRefresh）
        x if x == ServerPacketIds::UserSlotsRefresh as i16 => {
            if user::UserSlotsRefresh::read_body(&mut cur).is_ok() {
                tracing::info!("📦 UserSlotsRefresh 解码");
            }
        }

        _ => {}
    }
    handled
}

/// #2786：`PlayerInspect` 身份段（服务端 `world::inspect_identity_bytes` 的对应解析）。
/// 两端各写一份手写字节契约，互为依据（批15/16 惯例）。
#[derive(Debug, PartialEq, Eq)]
struct InspectIdentity {
    object_id: u32,
    name: String,
    guild: String,
    level: u16,
    class: u8,
    gender: u8,
    lover_name: String,
    allow_observe: bool,
}

fn parse_inspect_identity<R: std::io::Read>(cur: &mut R) -> Option<InspectIdentity> {
    use byteorder::{LittleEndian, ReadBytesExt};
    let object_id = cur.read_u32::<LittleEndian>().ok()?;
    let name = mir2_shared::binary::read_dotnet_string(cur).ok()?;
    let guild = mir2_shared::binary::read_dotnet_string(cur).ok()?;
    let level = cur.read_u16::<LittleEndian>().ok()?;
    let class = cur.read_u8().ok()?;
    let gender = cur.read_u8().ok()?;
    let lover_name = mir2_shared::binary::read_dotnet_string(cur).ok()?;
    let allow_observe = cur.read_u8().ok()? == 1;
    Some(InspectIdentity {
        object_id,
        name,
        guild,
        level,
        class,
        gender,
        lover_name,
        allow_observe,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::server_event::ServerEvent;
    use bevy::ecs::message::Messages;
    use mir2_shared::data::item::UserItem;
    use mir2_shared::packets::base::{Packet, PacketHeader};
    use mir2_shared::packets::server::hero::HeroInformation;

    /// #2786：`PlayerInspect` 身份段字节契约——同一串字节（服务端
    /// `inspect_identity_bytes` 的测试里手推的同值）必须解出期望字段，
    /// 特别是 `lover_name` 在 `gender` 与 `allow_observe` 之间。
    #[test]
    fn player_inspect_identity_wire_contract() {
        // object_id=7, name="abc", guild="行会", level=30, class=0, gender=1,
        // lover_name="老婆", allow_observe=1（UTF-8 字面量手推；dotnet 字符串 = 7bit 长度 + UTF-8）
        let bytes: Vec<u8> = vec![
            7, 0, 0, 0, // object_id
            3, b'a', b'b', b'c', // name
            6, 0xe8, 0xa1, 0x8c, 0xe4, 0xbc, 0x9a, // guild 行会
            30, 0, // level
            0, // class
            1, // gender
            6, 0xe8, 0x80, 0x81, 0xe5, 0xa9, 0x86, // lover_name 老婆
            1,    // allow_observe
        ];
        let mut cur = std::io::Cursor::new(bytes.as_slice());
        let id = parse_inspect_identity(&mut cur).expect("身份段应可解析");
        assert_eq!(
            id,
            InspectIdentity {
                object_id: 7,
                name: "abc".to_string(),
                guild: "行会".to_string(),
                level: 30,
                class: 0,
                gender: 1,
                lover_name: "老婆".to_string(),
                allow_observe: true,
            }
        );
        assert_eq!(cur.position() as usize, bytes.len(), "身份段应恰好读完");

        // 负控：少写 lover_name（旧格式）→ 后续字节被当成 lover_name，字段必错
        let old: Vec<u8> = vec![
            7, 0, 0, 0, 3, b'a', b'b', b'c', 6, 0xe8, 0xa1, 0x8c, 0xe4, 0xbc, 0x9a, 30, 0, 0, 1,
            1, // 这里直接是 allow_observe
        ];
        let mut cur = std::io::Cursor::new(old.as_slice());
        let parsed = parse_inspect_identity(&mut cur);
        assert!(
            parsed.is_none() || parsed.unwrap().lover_name != "老婆",
            "旧格式（无 lover_name）不得解析出配偶名"
        );
    }

    /// 构造 S.HeroInformation 全量包并走 handle_progress 解码（#203）
    fn build_hero_info_payload() -> Vec<u8> {
        let mut item = UserItem::new(2001);
        item.unique_id = 77;
        item.count = 3;
        let pkt = HeroInformation {
            object_id: 0x1000_0001,
            name: "HeroX".to_string(),
            class: mir2_shared::enums::MirClass::Wizard,
            gender: mir2_shared::enums::MirGender::Female,
            level: 25,
            hair: 2,
            hp: 300,
            mp: 150,
            experience: 1000,
            max_experience: 5000,
            inventory: Some(vec![Some(item.clone()), None]),
            equipment: Some(vec![Some(item)]),
            magics: Vec::new(),
            auto_pot: true,
            auto_hp_percent: 50,
            auto_mp_percent: 30,
            hp_item_index: 5,
            mp_item_index: 6,
        };
        let mut body = Vec::new();
        pkt.write_body(&mut body).unwrap();
        let mut payload = Vec::new();
        PacketHeader::new((4 + body.len()) as u16, HeroInformation::OPCODE)
            .write_to(&mut payload)
            .unwrap();
        payload.extend_from_slice(&body);
        payload
    }

    fn decode_system(mut events: MessageWriter<ServerEvent>, mut payload: Local<Option<Vec<u8>>>) {
        let payload = payload.get_or_insert_with(build_hero_info_payload);
        let _ = handle_progress(&mut events, payload);
    }

    #[test]
    fn hero_information_decode_to_server_event() {
        let mut app = App::new();
        app.init_resource::<Messages<ServerEvent>>();
        app.add_systems(Update, decode_system);
        app.update();

        let mut messages = app.world_mut().resource_mut::<Messages<ServerEvent>>();
        let drained: Vec<ServerEvent> = messages.drain().collect();
        assert_eq!(drained.len(), 1);
        match &drained[0] {
            ServerEvent::HeroInformation {
                name,
                level,
                inventory,
                equipment,
                auto_hp_percent,
                auto_mp_percent,
                hp_item_index,
                mp_item_index,
                ..
            } => {
                assert_eq!(name, "HeroX");
                assert_eq!(*level, 25);
                assert_eq!(inventory.len(), 2);
                assert!(inventory[0].is_some());
                assert!(inventory[1].is_none());
                assert_eq!(equipment.len(), 1);
                assert_eq!(*auto_hp_percent, 50);
                assert_eq!(*auto_mp_percent, 30);
                assert_eq!(*hp_item_index, 5);
                assert_eq!(*mp_item_index, 6);
            }
            other => panic!("unexpected event: {:?}", other),
        }
    }

    /// 构造 S.UpdateIntelligentCreatureList 包；`with_progress=false`/`with_rules=false`
    /// 依次模拟 #2761 / #2757 之前的老服务端。
    /// 字节顺序即 wire 契约（与 ServerRust `send_creature_list_packet` 一致）：
    /// [count i32][type u8][pickup u8][enabled u8][hunger u8][name dotnet][active u8]
    /// [filter 9×u8][grade u8][rules: minimal i32][mouse u8][mouseR i32][auto u8][autoR i32]
    /// [semi u8][semiR i32][blackstone u8][icon i32][fullness i32][expire i64][bstone_time i32]
    /// [creature_summoned u8][summoned_type u8][pearl_count i32]
    fn build_creature_list_payload(with_rules: bool, with_progress: bool) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&1i32.to_le_bytes()); // count
        body.push(2); // type = BabyPig
        body.push(1); // pickup
        body.push(1); // enabled
        body.push(42); // hunger
        let name = "小猪";
        body.extend_from_slice(&[name.len() as u8]);
        body.extend_from_slice(name.as_bytes());
        body.push(1); // active
        body.extend_from_slice(&[0u8; 9]); // filter
        body.push(3); // grade
        if with_rules {
            // C# `BabyDragon` 行（仅借其数值覆盖四个字段同时非零的情形）
            body.extend_from_slice(&7000i32.to_le_bytes());
            body.push(1);
            body.extend_from_slice(&7i32.to_le_bytes());
            body.push(1);
            body.extend_from_slice(&5i32.to_le_bytes());
            body.push(1);
            body.extend_from_slice(&5i32.to_le_bytes());
            body.push(0);
        }
        if with_progress {
            body.extend_from_slice(&507i32.to_le_bytes()); // icon（C# BabyDragon 行）
            body.extend_from_slice(&7500i32.to_le_bytes()); // fullness
            body.extend_from_slice(&3600i64.to_le_bytes()); // expire_in_secs
            body.extend_from_slice(&5400i32.to_le_bytes()); // blackstone_time
            body.push(1); // creature_summoned
            body.push(9); // summoned_type
            body.extend_from_slice(&4321i32.to_le_bytes()); // pearl_count
        }
        let mut payload = Vec::new();
        PacketHeader::new(
            (PacketHeader::HEADER_SIZE + body.len()) as u16,
            ServerPacketIds::UpdateIntelligentCreatureList as i16,
        )
        .write_to(&mut payload)
        .unwrap();
        payload.extend_from_slice(&body);
        payload
    }

    fn decode_creature_list(mut events: MessageWriter<ServerEvent>, mut payload: Local<Option<Vec<u8>>>) {
        let payload = payload.get_or_insert_with(|| build_creature_list_payload(true, true));
        let _ = handle_progress(&mut events, payload);
    }

    fn decode_creature_list_legacy(
        mut events: MessageWriter<ServerEvent>,
        mut payload: Local<Option<Vec<u8>>>,
    ) {
        let payload = payload.get_or_insert_with(|| build_creature_list_payload(false, false));
        let _ = handle_progress(&mut events, payload);
    }

    /// 取出唯一 `ServerEvent::CreatureList`：`(creatures, summoned, summoned_type, pearl_count)`
    fn drain_creature_list(
        app: &mut App,
    ) -> (
        Vec<crate::game::dialogs::creature::CreatureEntry>,
        bool,
        u8,
        i32,
    ) {
        let mut messages = app.world_mut().resource_mut::<Messages<ServerEvent>>();
        let drained: Vec<ServerEvent> = messages.drain().collect();
        assert_eq!(drained.len(), 1, "应恰好产出 1 个 ServerEvent");
        match drained.into_iter().next().unwrap() {
            ServerEvent::CreatureList {
                creatures,
                summoned,
                summoned_type,
                pearl_count,
            } => (creatures, summoned, summoned_type, pearl_count),
            other => panic!("unexpected event: {:?}", other),
        }
    }

    /// #2757/#2761：规则字段（C# `IntelligentCreatureRules`）与进度字段（图标/完整度/到期/黑石）
    /// 随条目解析，包尾三字段（召唤态/召唤种类/玩家珍珠数）随列表解析。
    #[test]
    fn creature_list_decodes_rules_and_progress_fields() {
        let mut app = App::new();
        app.init_resource::<Messages<ServerEvent>>();
        app.add_systems(Update, decode_creature_list);
        app.update();

        let (creatures, summoned, summoned_type, pearl_count) = drain_creature_list(&mut app);
        assert_eq!(creatures.len(), 1);
        let c = &creatures[0];
        assert_eq!((c.creature_type, c.hunger, c.grade), (2, 42, 3));
        assert_eq!(c.name, "小猪");
        assert!(c.active);
        assert_eq!(
            c.rules,
            mir2_shared::data::client_data::IntelligentCreatureRules {
                minimal_fullness: 7000,
                mouse_pickup_enabled: true,
                mouse_pickup_range: 7,
                auto_pickup_enabled: true,
                auto_pickup_range: 5,
                semi_auto_pickup_enabled: true,
                semi_auto_pickup_range: 5,
                can_produce_black_stone: false,
            }
        );
        assert_eq!(
            (c.icon, c.fullness, c.expire_secs, c.blackstone_time),
            (507, 7500, 3600, 5400)
        );
        assert_eq!((summoned, summoned_type, pearl_count), (true, 9, 4321));
    }

    /// #2757/#2761：老服务端不带规则/进度字段时列表仍可解析，两者取默认（其余字段不受影响）。
    #[test]
    fn creature_list_without_rules_falls_back_to_disabled() {
        let mut app = App::new();
        app.init_resource::<Messages<ServerEvent>>();
        app.add_systems(Update, decode_creature_list_legacy);
        app.update();

        let (creatures, summoned, summoned_type, pearl_count) = drain_creature_list(&mut app);
        assert_eq!(creatures.len(), 1);
        let c = &creatures[0];
        assert_eq!((c.creature_type, c.hunger, c.grade), (2, 42, 3));
        assert_eq!(
            c.rules,
            mir2_shared::data::client_data::IntelligentCreatureRules::default()
        );
        assert!(!c.rules.semi_auto_pickup_enabled && !c.rules.can_produce_black_stone);
        assert_eq!(
            (c.icon, c.fullness, c.expire_secs, c.blackstone_time),
            (0, 0, 0, 0)
        );
        assert_eq!((summoned, summoned_type, pearl_count), (false, 0, 0));
    }

    /// #2761：与 ServerRust `test_creature_list_body_matches_documented_wire` **同一串字节**
    /// （各自按同一条 wire 文档手推）→ 两端顺序互为见证：服务端断言「发出的是这串」，
    /// 本测试断言「这串解码成期望值」。
    fn build_server_literal_creature_list_payload() -> Vec<u8> {
        #[rustfmt::skip]
        let body: Vec<u8> = vec![
            0x01, 0x00, 0x00, 0x00, 0x02, 0x01, 0x01, 0x28, 0x06, 0xE5,
            0xB0, 0x8F, 0xE7, 0x8C, 0xAA, 0x01, 0x01, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xA0, 0x0F, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x01, 0x03, 0x00, 0x00, 0x00, 0x00, 0xF4, 0x01, 0x00, 0x00,
            0xA0, 0x0F, 0x00, 0x00, 0x80, 0x3A, 0x09, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x10, 0x0E, 0x00, 0x00, 0x01, 0x02, 0x09, 0x03,
            0x00, 0x00,
        ];
        let mut payload = Vec::new();
        PacketHeader::new(
            (PacketHeader::HEADER_SIZE + body.len()) as u16,
            ServerPacketIds::UpdateIntelligentCreatureList as i16,
        )
        .write_to(&mut payload)
        .unwrap();
        payload.extend_from_slice(&body);
        payload
    }

    fn decode_server_literal(
        mut events: MessageWriter<ServerEvent>,
        mut payload: Local<Option<Vec<u8>>>,
    ) {
        let payload = payload.get_or_insert_with(build_server_literal_creature_list_payload);
        let _ = handle_progress(&mut events, payload);
    }

    #[test]
    fn creature_list_wire_contract_matches_server_literal() {
        let mut app = App::new();
        app.init_resource::<Messages<ServerEvent>>();
        app.add_systems(Update, decode_server_literal);
        app.update();

        let (creatures, summoned, summoned_type, pearl_count) = drain_creature_list(&mut app);
        assert_eq!(creatures.len(), 1);
        let c = &creatures[0];
        assert_eq!(
            (
                c.creature_type,
                c.pickup_mode,
                c.enabled,
                c.hunger,
                c.active,
                c.grade
            ),
            (2, 1, true, 40, true, 0)
        );
        assert_eq!(c.name, "小猪");
        assert_eq!(c.filter, [1, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(
            c.rules,
            mir2_shared::data::client_data::IntelligentCreatureRules {
                minimal_fullness: 4000,
                semi_auto_pickup_enabled: true,
                semi_auto_pickup_range: 3,
                ..Default::default()
            }
        );
        assert_eq!(
            (c.icon, c.fullness, c.expire_secs, c.blackstone_time),
            (500, 4000, 604_800, 3600)
        );
        assert_eq!((summoned, summoned_type, pearl_count), (true, 2, 777));
    }

    /// 构造 mock 服务端（`--auto-enter` 离线实机走的路径）的宠物列表包。
    fn build_mock_creature_list_payload() -> Vec<u8> {
        let mut body = Vec::new();
        crate::network::mock::packets::MockCreatureList
            .write_body(&mut body)
            .unwrap();
        let mut payload = Vec::new();
        PacketHeader::new(
            (PacketHeader::HEADER_SIZE + body.len()) as u16,
            ServerPacketIds::UpdateIntelligentCreatureList as i16,
        )
        .write_to(&mut payload)
        .unwrap();
        payload.extend_from_slice(&body);
        payload
    }

    fn decode_mock_creature_list(
        mut events: MessageWriter<ServerEvent>,
        mut payload: Local<Option<Vec<u8>>>,
    ) {
        let payload = payload.get_or_insert_with(build_mock_creature_list_payload);
        let _ = handle_progress(&mut events, payload);
    }

    /// #2757/#2761：mock 必须与真实服务端同格式——mock 的两条样本解码出 C# `IntelligentCreatureInfo`
    /// 的 Chick 行（M11/A7/S7 + 黑石 + 图标 501 + 完整度 7500 + 到期 7 天 + 黑石 1h）
    /// 与 BabyPig 行（Semi 3 / 满 4000 / 图标 500 / 永久），包尾带召唤态与珍珠数，
    /// 否则实机截图验证不具备说服力。
    #[test]
    fn mock_creature_list_matches_csharp_rules() {
        let mut app = App::new();
        app.init_resource::<Messages<ServerEvent>>();
        app.add_systems(Update, decode_mock_creature_list);
        app.update();

        let (creatures, summoned, summoned_type, pearl_count) = drain_creature_list(&mut app);
        assert_eq!(creatures.len(), 2);
        let c = &creatures[0];
        assert_eq!(c.name, "小鸡");
        assert!(c.active);
        assert_eq!(
            (c.icon, c.fullness, c.expire_secs, c.blackstone_time),
            (501, 7500, 7 * 86400, 3600)
        );
        assert_eq!(
            c.rules,
            mir2_shared::data::client_data::IntelligentCreatureRules {
                // C# `IntelligentCreatureInfo.MinimalFullness` 字段默认 1000（Chick 行未显式给）
                minimal_fullness: 1000,
                mouse_pickup_enabled: true,
                mouse_pickup_range: 11,
                auto_pickup_enabled: true,
                auto_pickup_range: 7,
                semi_auto_pickup_enabled: true,
                semi_auto_pickup_range: 7,
                can_produce_black_stone: true,
                ..Default::default()
            }
        );
        let pig = &creatures[1];
        assert_eq!(pig.name, "小猪");
        assert_eq!(
            (pig.icon, pig.fullness, pig.expire_secs, pig.blackstone_time),
            (500, 10000, 0, 0)
        );
        assert_eq!(
            pig.rules,
            mir2_shared::data::client_data::IntelligentCreatureRules {
                minimal_fullness: 4000,
                semi_auto_pickup_enabled: true,
                semi_auto_pickup_range: 3,
                ..Default::default()
            }
        );
        // 包尾三字段：第 1 条（小鸡）为召唤中 → summoned_type 与它的 type 字节一致
        assert!(summoned);
        assert_eq!(summoned_type, c.creature_type);
        assert_eq!(pearl_count, 1234);
    }
}
