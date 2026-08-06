use bevy::prelude::*;
use mir2_shared::packets::base::{Packet, PacketHeader};
use crate::network::*;
use crate::ui::login::AuthFeedback;
use super::*;

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
    const HANDLED: &[i16] = &[ServerPacketIds::CraftItem as i16, ServerPacketIds::ItemRentalRequest as i16, ServerPacketIds::UpdateRentalItem as i16, ServerPacketIds::ItemRentalFee as i16, ServerPacketIds::ItemRentalPeriod as i16, ServerPacketIds::DepositRentalItem as i16, ServerPacketIds::RetrieveRentalItem as i16, ServerPacketIds::ItemRentalLock as i16, ServerPacketIds::ItemRentalPartnerLock as i16, ServerPacketIds::CanConfirmItemRental as i16, ServerPacketIds::ConfirmItemRental as i16, ServerPacketIds::CancelItemRental as i16, ServerPacketIds::GetRentedItems as i16, ServerPacketIds::ChangeQuest as i16, ServerPacketIds::CompleteQuest as i16, ServerPacketIds::NewQuestInfo as i16, ServerPacketIds::ShareQuest as i16, ServerPacketIds::GainedQuestItem as i16, ServerPacketIds::DeleteQuestItem as i16, ServerPacketIds::NewRecipeInfo as i16, ServerPacketIds::PauseBuff as i16, ServerPacketIds::RefreshItem as i16, ServerPacketIds::SetBindingShot as i16, ServerPacketIds::BaseStatsInfo as i16, ServerPacketIds::HeroBaseStatsInfo as i16, ServerPacketIds::NPCDisassemble as i16, ServerPacketIds::NPCDowngrade as i16, ServerPacketIds::NPCReset as i16, ServerPacketIds::GuildBuffList as i16, ServerPacketIds::NPCPearlGoods as i16, ServerPacketIds::NPCRequestInput as i16, ServerPacketIds::HeroHealthChanged as i16, ServerPacketIds::GainHeroExperience as i16, ServerPacketIds::HeroLevelChanged as i16, ServerPacketIds::AddBuff as i16, ServerPacketIds::RemoveBuff as i16, ServerPacketIds::PlayerInspect as i16, ServerPacketIds::UpdateIntelligentCreatureList as i16, ServerPacketIds::ChangeHero as i16, ServerPacketIds::MarriageRequest as i16, ServerPacketIds::LoverUpdate as i16, ServerPacketIds::DivorceRequest as i16, ServerPacketIds::ObjectColourChanged as i16, ServerPacketIds::ManageHeroes as i16, ServerPacketIds::NewHero as i16, ServerPacketIds::SetHeroBehaviour as i16, ServerPacketIds::SetAutoPotValue as i16, ServerPacketIds::SetAutoPotItem as i16, ServerPacketIds::HeroInformation as i16];
    let handled = HANDLED.contains(&opcode);
    match opcode {
        // ---- M41: 合成 ----
        x if x == ServerPacketIds::CraftItem as i16 => {
            // 服务端实际 wire：[recipe_id u32][count u16][success u8]
            let body = &payload[PacketHeader::HEADER_SIZE..];
            if body.len() >= 7 {
                let recipe_id = u32::from_le_bytes(body[0..4].try_into().unwrap_or([0; 4]));
                let count = u16::from_le_bytes(body[4..6].try_into().unwrap_or([0; 2]));
                let success = body[6] != 0;
                server_events.write(ServerEvent::CraftResult { recipe_id, count, success });
                tracing::info!("🔧 CraftItem: recipe={} count={} success={}", recipe_id, count, success);
            }
        }
        // ---- M42: 物品租赁 ----
        x if x == ServerPacketIds::ItemRentalRequest as i16 => {
            server_events.write(ServerEvent::RentalRequestReceived);
            tracing::info!("📦 收到租赁请求");
        }
        x if x == ServerPacketIds::UpdateRentalItem as i16 => {
            // [hasdata u8][fee u32][period i32]
            let body = &payload[PacketHeader::HEADER_SIZE..];
            if body.len() >= 9 {
                let has_item = body[0] != 0;
                let fee = u32::from_le_bytes(body[1..5].try_into().unwrap_or([0; 4]));
                let period = i32::from_le_bytes(body[5..9].try_into().unwrap_or([0; 4]));
                server_events.write(ServerEvent::RentalItemUpdate { has_item, fee, period });
                tracing::info!("📦 UpdateRentalItem: item={} fee={} period={}", has_item, fee, period);
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
            server_events.write(ServerEvent::RentalLocked);
            tracing::info!("📦 租赁锁定（本侧）");
        }
        x if x == ServerPacketIds::ItemRentalPartnerLock as i16 => {
            server_events.write(ServerEvent::RentalPartnerLocked);
            tracing::info!("📦 租赁锁定（对方）");
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
        // #270：英雄状态/经验/等级
        x if x == ServerPacketIds::HeroHealthChanged as i16 => {
            if let Ok(p) = combat::HeroHealthChanged::read_body(&mut cur) {
                tracing::debug!("⭐ 英雄 HP/MP {}/{}", p.hp, p.mp);
            }
        }
        x if x == ServerPacketIds::GainHeroExperience as i16 => {
            if let Ok(p) = experience::GainHeroExperience::read_body(&mut cur) {
                tracing::debug!("⭐ 英雄经验 +{}", p.amount);
            }
        }
        x if x == ServerPacketIds::HeroLevelChanged as i16 => {
            if let Ok(p) = experience::HeroLevelChanged::read_body(&mut cur) {
                tracing::debug!("⭐ 英雄等级 Lv.{}", p.level);
            }
        }

        // #268：杂项协议（租赁/基础属性/觉醒拆卸/行会Buff/珍珠/NPC输入）
        x if x == ServerPacketIds::GetRentedItems as i16 => {
            if let Ok(p) = rental_system::GetRentedItems::read_body(&mut cur) {
                tracing::info!("📦 租赁物品列表: {} 件", p.items.len());
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
        x if x == ServerPacketIds::NPCDisassemble as i16 => {
            if awakening_system::NPCDisassemble::read_body(&mut cur).is_ok() {
                tracing::debug!("🔧 NPC 拆卸面板");
            }
        }
        x if x == ServerPacketIds::NPCDowngrade as i16 => {
            if awakening_system::NPCDowngrade::read_body(&mut cur).is_ok() {
                tracing::debug!("⬇️ NPC 降级面板");
            }
        }
        x if x == ServerPacketIds::NPCReset as i16 => {
            if awakening_system::NPCReset::read_body(&mut cur).is_ok() {
                tracing::debug!("🔄 NPC 重置面板");
            }
        }
        x if x == ServerPacketIds::GuildBuffList as i16 => {
            if let Ok(p) = special_systems::GuildBuffList::read_body(&mut cur) {
                tracing::info!("🏴 行会技能 Buff: {:?}", p.active_buffs.len());
            }
        }
        x if x == ServerPacketIds::NPCPearlGoods as i16 => {
            if let Ok(p) = special_systems::NPCPearlGoods::read_body(&mut cur) {
                tracing::info!("🫧 珍珠商品: {:?}", p);
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
                tracing::debug!("🎯 定身射击 enabled={}", p.enabled);
            }
        }

        // #260：任务数据包
        x if x == ServerPacketIds::NewQuestInfo as i16 => {
            if let Ok(p) = quest::NewQuestInfo::read_body(&mut cur) {
                server_events.write(ServerEvent::QuestInfo {
                    id: p.quest.index,
                    name: p.quest.name.clone(),
                    tasks: p.quest.task_description,
                });
                tracing::info!("📜 任务信息: #{} {}", p.quest.index, p.quest.name);
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
            if let Ok(p) = miscellaneous::GainedQuestItem::read_body(&mut cur) {
                tracing::info!("🎁 任务物品获得 #{}", p.item_id);
            }
        }
        x if x == ServerPacketIds::DeleteQuestItem as i16 => {
            if let Ok(p) = miscellaneous::DeleteQuestItem::read_body(&mut cur) {
                tracing::info!("🗑️ 任务物品删除 #{}", p.item_id);
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
            // [tag u8][remaining_ticks u32]
            let body = &payload[PacketHeader::HEADER_SIZE..];
            if body.len() >= 5 {
                let tag = body[0];
                let ticks = u32::from_le_bytes(body[1..5].try_into().unwrap_or([0; 4]));
                server_events.write(ServerEvent::BuffAdded { tag, ticks });
                tracing::info!("✨ AddBuff: tag={} ticks={}", tag, ticks);
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
            // [count u8][per: uid u64][index i32][dura i32][max_dura i32]
            let body = &payload[PacketHeader::HEADER_SIZE..];
            let mut cur = std::io::Cursor::new(body);
            use byteorder::{LittleEndian, ReadBytesExt};
            let _oid = match cur.read_u32::<LittleEndian>() { Ok(v) => v, Err(_) => { tracing::warn!("⚠️ PlayerInspect 解析失败"); return true; } };
            let name = mir2_shared::binary::read_dotnet_string(&mut cur).unwrap_or_default();
            let guild = mir2_shared::binary::read_dotnet_string(&mut cur).unwrap_or_default();
            let level = match cur.read_u16::<LittleEndian>() { Ok(v) => v, Err(_) => { tracing::warn!("⚠️ PlayerInspect 解析失败"); return true; } };
            let class = cur.read_u8().unwrap_or(0);
            let gender = cur.read_u8().unwrap_or(0);
            let count = cur.read_u8().unwrap_or(0) as usize;
            let mut items = Vec::with_capacity(count);
            let mut ok = true;
            for _ in 0..count {
                let unique_id = match cur.read_u64::<LittleEndian>() { Ok(v) => v, Err(_) => { ok = false; break; } };
                let item_index = match cur.read_i32::<LittleEndian>() { Ok(v) => v, Err(_) => { ok = false; break; } };
                let current_dura = match cur.read_i32::<LittleEndian>() { Ok(v) => v, Err(_) => { ok = false; break; } };
                let max_dura = match cur.read_i32::<LittleEndian>() { Ok(v) => v, Err(_) => { ok = false; break; } };
                items.push(InspectItem { unique_id, item_index, current_dura, max_dura });
            }
            if ok {
                let item_count = items.len();
                server_events.write(ServerEvent::InspectPlayer {
                    name: name.clone(),
                    guild,
                    level,
                    class,
                    gender,
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
                creatures.push(CreatureEntry { creature_type, pickup_mode, enabled, hunger, name });
            }
            if ok {
                let count = creatures.len();
                server_events.write(ServerEvent::CreatureList { creatures });
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
            // [married u8]
            let body = &payload[PacketHeader::HEADER_SIZE..];
            let married = body.first().copied().unwrap_or(0) != 0;
            server_events.write(ServerEvent::MarriageStatus { married });
            tracing::info!("💍 LoverUpdate: married={}", married);
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
                });
                tracing::info!("🦸 英雄列表: {} 个", p.heroes.len());
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
        _ => {}
    }
    handled
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::server_event::ServerEvent;
    use bevy::ecs::message::Messages;
    use mir2_shared::data::item::UserItem;
    use mir2_shared::packets::base::{Packet, PacketHeader};
    use mir2_shared::packets::server::hero::HeroInformation;

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
}
