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
    const HANDLED: &[i16] = &[ServerPacketIds::CraftItem as i16, ServerPacketIds::ItemRentalRequest as i16, ServerPacketIds::UpdateRentalItem as i16, ServerPacketIds::ItemRentalFee as i16, ServerPacketIds::ItemRentalPeriod as i16, ServerPacketIds::DepositRentalItem as i16, ServerPacketIds::RetrieveRentalItem as i16, ServerPacketIds::ItemRentalLock as i16, ServerPacketIds::ItemRentalPartnerLock as i16, ServerPacketIds::CanConfirmItemRental as i16, ServerPacketIds::ConfirmItemRental as i16, ServerPacketIds::CancelItemRental as i16, ServerPacketIds::ChangeQuest as i16, ServerPacketIds::CompleteQuest as i16, ServerPacketIds::AddBuff as i16, ServerPacketIds::RemoveBuff as i16, ServerPacketIds::PlayerInspect as i16, ServerPacketIds::UpdateIntelligentCreatureList as i16, ServerPacketIds::ChangeHero as i16, ServerPacketIds::MarriageRequest as i16, ServerPacketIds::LoverUpdate as i16, ServerPacketIds::DivorceRequest as i16, ServerPacketIds::ObjectColourChanged as i16, ServerPacketIds::ManageHeroes as i16, ServerPacketIds::NewHero as i16, ServerPacketIds::SetHeroBehaviour as i16, ServerPacketIds::SetAutoPotValue as i16, ServerPacketIds::SetAutoPotItem as i16];
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
        _ => {}
    }
    handled
}
