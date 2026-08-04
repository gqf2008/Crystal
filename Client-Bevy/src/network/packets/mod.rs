// 网络包解码分派（#72 拆分）

use bevy::prelude::*;
use mir2_shared::packets::base::{Packet, PacketHeader};
use super::*;
use crate::ui::login::AuthFeedback;

mod handle_auth;
mod handle_guild;
mod handle_npc_items;
mod handle_progress;
mod handle_social;

use handle_auth::handle_auth;
use handle_guild::handle_guild;
use handle_npc_items::handle_npc_items;
use handle_progress::handle_progress;
use handle_social::handle_social;

// 网络包解码分派（#72 拆分）：handle_packet 调度器按 opcode 分发到类别处理函数。

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_packet(    net: &mut NetConnection,
    session: &mut SessionState,
    auth: &mut AuthFeedback,
    game_data: &mut GameData,
    net_objects: &mut MessageWriter<NetObject>,
    net_removals: &mut MessageWriter<NetObjectRemoved>,
    motions: &mut MessageWriter<NetMotion>,
    combat_evt: &mut MessageWriter<CombatEvent>,
    effects: &mut MessageWriter<PendingEffect>,
    server_events: &mut MessageWriter<ServerEvent>,
    control: &mut ControlState,
    next: &mut NextState<AppState>,
    payload: &[u8],) {
    use mir2_shared::packets::server::*;

    let mut cur = std::io::Cursor::new(payload);
    let Ok(header) = PacketHeader::read_from(&mut cur) else {
        return;
    };
    let opcode = header.opcode;
    if handle_auth(net, session, auth, game_data, net_objects, net_removals, motions, combat_evt, effects, server_events, control, next, payload) {
        return;
    }
    if handle_npc_items(net, session, auth, game_data, net_objects, net_removals, motions, combat_evt, effects, server_events, control, next, payload) {
        return;
    }
    if handle_guild(net, session, auth, game_data, net_objects, net_removals, motions, combat_evt, effects, server_events, control, next, payload) {
        return;
    }
    if handle_progress(server_events, payload) {
        return;
    }
    if handle_social(net, session, auth, game_data, net_objects, net_removals, motions, combat_evt, effects, server_events, control, next, payload) {
        return;
    }
    tracing::debug!("未处理服务器包 opcode {:04X}", opcode);
}

// ---- 辅助函数（原 handle_packet 同文件内）----
fn to_inv_item(item: &mir2_shared::data::item::UserItem) -> InvItem {
    InvItem {
        unique_id: item.unique_id,
        item_index: item.item_index,
        name: item
            .info
            .as_ref()
            .map(|i| i.name.clone())
            .unwrap_or_else(|| format!("#{}", item.item_index)),
        image: item.info.as_ref().map(|i| i.image).unwrap_or(0),
        count: item.count,
        item_type: item.info.as_ref().map(|i| i.item_type as u8).unwrap_or(0),
        shape: item.info.as_ref().map(|i| i.shape).unwrap_or(0),
        current_dura: item.current_dura,
        max_dura: item.max_dura,
        slots: item.slots.iter().map(|s| s.as_ref().map(to_inv_item)).collect(),
    }
}

/// 解析服务端 ReceiveMail（同 opcode 双格式）：
/// - 条目包：mail_id, sender, subject, timestamp, read, collected, gold, item_count
/// - 全文包：mail_id, sender, subject, body, timestamp, read, collected, gold, item_count, items...
/// 先尝试全文格式，失败再按条目格式（条目包按全文解析时 timestamp 首字节必然导致 7-bit 长度越界）
fn parse_receive_mail(payload: &[u8]) -> Option<(MailEntry, Option<MailDetail>)> {
    use mir2_shared::binary::read_dotnet_string;
    use byteorder::{LittleEndian, ReadBytesExt};

    fn parse_content(
        payload: &[u8],
    ) -> Option<(MailEntry, Option<MailDetail>)> {
        let mut cur = std::io::Cursor::new(payload);
        let mail_id = cur.read_u64::<LittleEndian>().ok()?;
        let sender = read_dotnet_string(&mut cur).ok()?;
        let subject = read_dotnet_string(&mut cur).ok()?;
        let body = read_dotnet_string(&mut cur).ok()?;
        let _timestamp = cur.read_i64::<LittleEndian>().ok()?;
        let read_flag = cur.read_u8().ok()? != 0;
        let _collected = cur.read_u8().ok()? != 0;
        let gold = cur.read_u32::<LittleEndian>().ok()?;
        let item_count = cur.read_u8().ok()? as usize;
        let mut items = Vec::new();
        for _ in 0..item_count {
            let _uid = cur.read_u64::<LittleEndian>().ok()?;
            let _idx = cur.read_u32::<LittleEndian>().ok()?;
            let name = read_dotnet_string(&mut cur).ok()?;
            let _count = cur.read_u16::<LittleEndian>().ok()?;
            let _cd = cur.read_u16::<LittleEndian>().ok()?;
            let _md = cur.read_u16::<LittleEndian>().ok()?;
            items.push(name);
        }
        if payload.len() as u64 != cur.position() {
            return None;
        }
        Some((
            MailEntry {
                mail_id,
                sender: sender.clone(),
                subject: subject.clone(),
                unread: !read_flag,
                gold,
            },
            Some(MailDetail {
                mail_id,
                sender,
                subject,
                body,
                gold,
                items,
            }),
        ))
    }

    fn parse_entry(payload: &[u8]) -> Option<(MailEntry, Option<MailDetail>)> {
        let mut cur = std::io::Cursor::new(payload);
        let mail_id = cur.read_u64::<LittleEndian>().ok()?;
        let sender = read_dotnet_string(&mut cur).ok()?;
        let subject = read_dotnet_string(&mut cur).ok()?;
        let _timestamp = cur.read_i64::<LittleEndian>().ok()?;
        let read_flag = cur.read_u8().ok()? != 0;
        let _collected = cur.read_u8().ok()? != 0;
        let gold = cur.read_u32::<LittleEndian>().ok()?;
        let _item_count = cur.read_u8().ok()?;
        if payload.len() as u64 != cur.position() {
            return None;
        }
        Some((
            MailEntry {
                mail_id,
                sender,
                subject,
                unread: !read_flag,
                gold,
            },
            None,
        ))
    }

    parse_content(payload).or_else(|| parse_entry(payload))
}


