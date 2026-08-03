// Social packet helpers - shared packet building functions for SocialActor
// Extracted from WorldActor to be reusable across actors

use std::collections::HashMap;

use kameo::actor::ActorRef;
use tracing::debug;

use crate::actors::group::{Group, GroupMember};
use crate::actors::trade::{TradeSession, TradeSide};
use crate::actors::friend::FriendEntry;
use crate::actors::guild::Guild;
use crate::actors::mail::MailMessage;
use crate::actors::player::{GetPlayerState, PlayerActor, SetGroupId};
use crate::gate::actor::{GateActor, SendToClient};
use crate::util::wire::{build_packet_bytes, write_dotnet_string};

// ============================================================
// 系统消息
// ============================================================

/// Send a system chat message to a single client.
pub fn send_system_message(gate_ref: &ActorRef<GateActor>, session_id: u64, message: &str) {
    use mir2_shared::enums::ServerPacketIds;
    let mut body = Vec::new();
    write_dotnet_string(&mut body, message);
    body.push(mir2_shared::enums::ChatType::System as u8); // ChatType::System=5（SharedRust 枚举与 C# 差 3）
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(ServerPacketIds::Chat as i16, &body),
    }).try_send();
}

// ============================================================
// 组队系统
// ============================================================

/// Send group member list to a client.
pub fn send_group_members_map(gate_ref: &ActorRef<GateActor>, session_id: u64, members: &[GroupMember]) {
    let mut body = Vec::new();
    // [count: i32 LE][members...]
    body.extend_from_slice(&(members.len() as i32).to_le_bytes());
    for member in members {
        write_dotnet_string(&mut body, &member.name);
        body.push(if member.is_leader { 1u8 } else { 0u8 });
        body.push(if member.online { 1u8 } else { 0u8 });
    }
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GroupMembersMap as i16, &body),
    }).try_send();
}

/// Send group invite to a client.
pub fn send_group_invite_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, inviter_name: &str, inviter_id: u64) {
    let mut body = Vec::new();
    write_dotnet_string(&mut body, inviter_name);
    body.extend_from_slice(&inviter_id.to_le_bytes());
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GroupInvite as i16, &body),
    }).try_send();
}

// ============================================================
// 交易系统
// ============================================================

/// Send trade invite to a client.
pub fn send_trade_invite_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, inviter_name: &str) {
    let mut body = Vec::new();
    write_dotnet_string(&mut body, inviter_name);
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::TradeRequest as i16, &body),
    }).try_send();
}

/// Send trade open (partner info) to a client.
pub fn send_trade_open_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, partner_name: &str) {
    let mut body = Vec::new();
    write_dotnet_string(&mut body, partner_name);
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::TradeRequest as i16, &body),
    }).try_send();
}

/// Send trade gold update to a client.
pub fn send_trade_gold_update_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, _other_session: u64, amount: u64) {
    let mut body = Vec::new();
    body.extend_from_slice(&amount.to_le_bytes());
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::TradeGold as i16, &body),
    }).try_send();
}

/// Send trade confirm (lock status) to a client.
pub fn send_trade_confirm_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, side_a: &TradeSide, side_b: &TradeSide) {
    let mut body = Vec::new();
    body.push(if side_a.locked { 1u8 } else { 0u8 });
    body.push(if side_b.locked { 1u8 } else { 0u8 });
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::TradeConfirm as i16, &body),
    }).try_send();
}

/// Send trade cancel to a client.
pub fn send_trade_cancel_packet(gate_ref: &ActorRef<GateActor>, session_id: u64) {
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::TradeCancel as i16, &[]),
    }).try_send();
}

/// Send trade success to a client.
pub fn send_trade_success_packet(gate_ref: &ActorRef<GateActor>, session_id: u64) {
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::TradeConfirm as i16, &[1u8, 1u8]),
    }).try_send();
}

/// Send trade close to a client.
pub fn send_trade_close_packet(gate_ref: &ActorRef<GateActor>, session_id: u64) {
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::TradeCancel as i16, &[]),
    }).try_send();
}

/// Send trade item update to a client.
pub fn send_trade_item_update_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, uid: u64, grid: u8, count: u16, is_add: bool) {
    let mut body = Vec::new();
    body.extend_from_slice(&uid.to_le_bytes());
    body.push(grid);
    body.extend_from_slice(&count.to_le_bytes());
    body.push(if is_add { 1u8 } else { 0u8 });
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::TradeItem as i16, &body),
    }).try_send();
}

/// Send DepositTradeItem response to a client.
pub fn send_deposit_trade_item_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, from_slot: i32, success: bool) {
    let mut body = Vec::new();
    body.extend_from_slice(&from_slot.to_le_bytes());
    body.push(if success { 1u8 } else { 0u8 });
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::DepositTradeItem as i16, &body),
    }).try_send();
}

/// Send RetrieveTradeItem response to a client.
pub fn send_retrieve_trade_item_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, from_slot: i32, success: bool) {
    let mut body = Vec::new();
    body.extend_from_slice(&from_slot.to_le_bytes());
    body.push(if success { 1u8 } else { 0u8 });
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::RetrieveTradeItem as i16, &body),
    }).try_send();
}

/// Find a trade session by session_id (immutable).
pub fn find_trade(active_trades: &HashMap<u64, TradeSession>, session_id: u64) -> Option<&TradeSession> {
    active_trades.values().find(|t| {
        t.side_a.session_id == session_id || t.side_b.session_id == session_id
    })
}

/// Find a trade session by session_id (mutable).
pub fn find_trade_mut(active_trades: &mut HashMap<u64, TradeSession>, session_id: u64) -> Option<&mut TradeSession> {
    active_trades.values_mut().find(|t| {
        t.side_a.session_id == session_id || t.side_b.session_id == session_id
    })
}

// ============================================================
// 好友系统
// ============================================================

/// Send friends list to a client.
pub fn send_friends_list_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, friends: &[FriendEntry], online_object_ids: &[u32]) {
    let mut body = Vec::new();
    // [count: i32 LE][friends...]
    body.extend_from_slice(&(friends.len() as i32).to_le_bytes());
    for friend in friends {
        body.extend_from_slice(&friend.object_id.to_le_bytes());
        write_dotnet_string(&mut body, &friend.name);
        write_dotnet_string(&mut body, &friend.memo);
        body.push(if online_object_ids.contains(&friend.object_id) { 1u8 } else { 0u8 });
    }
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::FriendUpdate as i16, &body),
    }).try_send();
}

/// Send friend add result to a client.
pub fn send_friend_add_result_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, friend: &FriendEntry, online: bool) {
    let mut body = Vec::new();
    body.extend_from_slice(&friend.object_id.to_le_bytes());
    write_dotnet_string(&mut body, &friend.name);
    write_dotnet_string(&mut body, &friend.memo);
    body.push(if online { 1u8 } else { 0u8 });
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::FriendUpdate as i16, &body),
    }).try_send();
}

// ============================================================
// 邮件系统
// ============================================================

/// Send mail received notification to a client.
pub fn send_mail_received_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, mail: &MailMessage) {
    let mut body = Vec::new();
    body.extend_from_slice(&mail.mail_id.to_le_bytes());
    write_dotnet_string(&mut body, &mail.sender_name);
    write_dotnet_string(&mut body, &mail.subject);
    body.extend_from_slice(&mail.timestamp.to_le_bytes());
    body.push(if mail.read { 1u8 } else { 0u8 });
    body.push(if mail.collected { 1u8 } else { 0u8 });
    body.extend_from_slice(&(mail.gold as u32).to_le_bytes());
    body.push(mail.items.len() as u8);
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ReceiveMail as i16, &body),
    }).try_send();
}

/// Send full mail content to a client.
pub fn send_mail_content_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, mail: &MailMessage) {
    let mut body = Vec::new();
    body.extend_from_slice(&mail.mail_id.to_le_bytes());
    write_dotnet_string(&mut body, &mail.sender_name);
    write_dotnet_string(&mut body, &mail.subject);
    write_dotnet_string(&mut body, &mail.body);
    body.extend_from_slice(&mail.timestamp.to_le_bytes());
    body.push(if mail.read { 1u8 } else { 0u8 });
    body.push(if mail.collected { 1u8 } else { 0u8 });
    body.extend_from_slice(&(mail.gold as u32).to_le_bytes());
    body.push(mail.items.len() as u8);
    // Send attachment item info
    for item in &mail.items {
        body.extend_from_slice(&item.unique_id.to_le_bytes());
        body.extend_from_slice(&(item.item_index as u32).to_le_bytes());
        write_dotnet_string(&mut body, &item.info.as_ref().map(|i| i.name.clone()).unwrap_or_default());
        body.extend_from_slice(&item.count.to_le_bytes());
        body.extend_from_slice(&item.current_dura.to_le_bytes());
        body.extend_from_slice(&item.max_dura.to_le_bytes());
    }
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ReceiveMail as i16, &body),
    }).try_send();
}

// ============================================================
// 行会系统
// ============================================================

/// Send guild invite to a client（C# S.GuildInvite{Name}）
pub fn send_guild_invite_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, guild_name: &str) {
    let mut body = Vec::new();
    write_dotnet_string(&mut body, guild_name);
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GuildInvite as i16, &body),
    }).try_send();
}

/// Send guild status (in/out) to a client.
pub fn send_guild_status_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, in_guild: bool) {
    let mut body = Vec::new();
    body.push(if in_guild { 1u8 } else { 0u8 });
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GuildStatus as i16, &body),
    }).try_send();
}

/// Send guild info to a client.
pub fn send_guild_info_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, guild: &Guild) {
    let mut body = Vec::new();
    write_dotnet_string(&mut body, &guild.name);
    write_dotnet_string(&mut body, guild.leader_name());
    // Notice (5 lines)
    body.push(guild.notice.len() as u8);
    for line in &guild.notice {
        write_dotnet_string(&mut body, line);
    }
    // Member list
    body.push(guild.member_count() as u16 as u8);
    for member in &guild.members {
        write_dotnet_string(&mut body, &member.name);
        body.push(member.rank as u8);
        body.push(if member.session_id.is_some() { 1u8 } else { 0u8 });
    }
    // Guild gold
    body.extend_from_slice(&(guild.gold as u32).to_le_bytes());
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GuildStatus as i16, &body),
    }).try_send();
}

/// Send guild member change notification to a client.
pub fn send_guild_member_change_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, member_name: &str, joined: bool) {
    let mut body = Vec::new();
    body.push(if joined { 1u8 } else { 0u8 });
    write_dotnet_string(&mut body, member_name);
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GuildMemberChange as i16, &body),
    }).try_send();
}

/// Send guild notice change to a client.
pub fn send_guild_notice_change_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, notice: &[String]) {
    let mut body = Vec::new();
    body.push(notice.len() as u8);
    for line in notice {
        write_dotnet_string(&mut body, line);
    }
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GuildNoticeChange as i16, &body),
    }).try_send();
}

/// Send guild member update to a client.
pub fn send_guild_member_update_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, member_name: &str, rank: u8, online: bool) {
    let mut body = Vec::new();
    write_dotnet_string(&mut body, member_name);
    body.push(rank);
    body.push(if online { 1u8 } else { 0u8 });
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GuildMemberChange as i16, &body),
    }).try_send();
}

// ============================================================
// 婚姻/师徒系统
// ============================================================

/// Send marriage status to a client.
pub fn send_marriage_status_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, married: bool) {
    let mut body = Vec::new();
    body.push(if married { 1u8 } else { 0u8 });
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::LoverUpdate as i16, &body),
    }).try_send();
}

/// Send marriage invite to a client.
pub fn send_marriage_invite_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, requester_name: &str) {
    let mut body = Vec::new();
    write_dotnet_string(&mut body, requester_name);
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::MarriageRequest as i16, &body),
    }).try_send();
}

/// Send divorce request to a client.
pub fn send_divorce_request_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, requester_name: &str) {
    let mut body = Vec::new();
    write_dotnet_string(&mut body, requester_name);
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::DivorceRequest as i16, &body),
    }).try_send();
}

/// Send divorce to a client (confirmation/completion).
pub fn send_divorce_packet(gate_ref: &ActorRef<GateActor>, session_id: u64) {
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::DivorceRequest as i16, &[]),
    }).try_send();
}

/// Send mentor invite to a client（C# S.MentorRequest：Name + Level u16）
pub fn send_mentor_invite_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, requester_name: &str, requester_level: u16) {
    let mut body = Vec::new();
    write_dotnet_string(&mut body, requester_name);
    body.extend_from_slice(&requester_level.to_le_bytes());
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::MentorRequest as i16, &body),
    }).try_send();
}

/// Send mentor update to a client（C# S.MentorUpdate：Name + Level + Online + MenteeEXP）
pub fn send_mentor_update_packet(
    gate_ref: &ActorRef<GateActor>,
    session_id: u64,
    mentor_name: &str,
    mentor_level: u32,
    mentor_online: bool,
    mentee_exp: i64,
) {
    let mut body = Vec::new();
    write_dotnet_string(&mut body, mentor_name);
    body.extend_from_slice(&(mentor_level as i32).to_le_bytes());
    body.push(if mentor_online { 1u8 } else { 0u8 });
    body.extend_from_slice(&mentee_exp.to_le_bytes());
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::MentorUpdate as i16, &body),
    }).try_send();
}

/// Send mentor cancel to a client (sends empty MentorUpdate to clear mentor state).
pub fn send_mentor_cancel_packet(gate_ref: &ActorRef<GateActor>, session_id: u64) {
    send_mentor_update_packet(gate_ref, session_id, "", 0, false, 0);
}

// ============================================================
// 英雄系统
// ============================================================

/// Send hero update to a client.
pub fn send_hero_update_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, hero_index: u8) {
    let body = vec![hero_index];
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ChangeHero as i16, &body),
    }).try_send();
}

// ============================================================
// 任务系统
// ============================================================

/// Send quest complete to a client.
pub fn send_quest_complete_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, quest_index: i32) {
    let mut body = Vec::new();
    body.extend_from_slice(&quest_index.to_le_bytes());
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::CompleteQuest as i16, &body),
    }).try_send();
}

// ============================================================
// 宠物系统
// ============================================================

/// Send intelligent creature list to a client.
pub fn send_creature_list_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, creature: Option<&crate::actors::creature::IntelligentCreature>) {
    let mut body = Vec::new();
    if let Some(c) = creature {
        body.extend_from_slice(&1i32.to_le_bytes());
        body.push(c.creature_type as u8);
        body.push(c.pickup_mode as u8);
        body.push(if c.enabled { 1u8 } else { 0u8 });
        body.push(c.hunger);
        write_dotnet_string(&mut body, c.custom_name.as_deref().unwrap_or(""));
    } else {
        body.extend_from_slice(&0i32.to_le_bytes());
    }
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::UpdateIntelligentCreatureList as i16, &body),
    }).try_send();
}

// ============================================================
// 仓库/金币
// ============================================================

/// Send store item response to a client.
pub fn send_store_item_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, _grid: u8, success: bool) {
    let mut body = Vec::new();
    body.extend_from_slice(&0i32.to_le_bytes()); // from
    body.extend_from_slice(&0i32.to_le_bytes()); // to
    body.push(if success { 1u8 } else { 0u8 });
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::StoreItem as i16, &body),
    }).try_send();
}

/// Send take back item response to a client.
pub fn send_take_back_item_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, _grid: u8, success: bool) {
    let mut body = Vec::new();
    body.extend_from_slice(&0i32.to_le_bytes()); // from
    body.extend_from_slice(&0i32.to_le_bytes()); // to
    body.push(if success { 1u8 } else { 0u8 });
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::TakeBackItem as i16, &body),
    }).try_send();
}

/// Send gold changed notification to a client.
pub fn send_gold_changed_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, new_gold: u64) {
    let mut body = Vec::new();
    body.extend_from_slice(&(new_gold as u32).to_le_bytes());
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::LoseGold as i16, &body),
    }).try_send();
}

// ============================================================
// 组队管理函数 (adapted to take fields as parameters)
// ============================================================

/// Join or create a group.
///
/// Parameters:
/// - `gate_ref`: reference to GateActor for sending packets
/// - `players`: map of session_id to PlayerActor ActorRef
/// - `groups`: mutable reference to the groups map
/// - `next_group_id`: mutable reference to the next group ID counter
/// - `joiner_session`: session ID of the player joining
/// - `target_session`: session ID of the target player
/// - `joiner_name`: name of the joining player
pub async fn join_or_create_group(
    gate_ref: &ActorRef<GateActor>,
    players: &HashMap<u64, ActorRef<PlayerActor>>,
    groups: &mut HashMap<u64, Group>,
    next_group_id: &mut u64,
    joiner_session: u64,
    target_session: u64,
    joiner_name: &str,
) {
    let joiner_name = joiner_name.to_string();

    // Get joiner info
    let joiner_record = match players.get(&joiner_session) {
        Some(r) => r,
        None => return,
    };
    let joiner_state = match joiner_record.ask(GetPlayerState).await {
        Ok(Some(s)) => s,
        _ => return,
    };

    let joiner_member = GroupMember {
        session_id: joiner_state.session_id,
        name: joiner_state.name.clone(),
        is_leader: false,
        online: true,
    };

    // Check target player is online
    let target_record = match players.get(&target_session) {
        Some(r) => r,
        None => {
            send_system_message(gate_ref, joiner_session, "目标玩家不在线");
            return;
        }
    };

    let target_state = match target_record.ask(GetPlayerState).await {
        Ok(Some(s)) => s,
        _ => return,
    };

    if let Some(target_group_id) = target_state.group_id {
        // Join existing group
        if let Some(group) = groups.get_mut(&target_group_id) {
            if !group.add_member(joiner_member) {
                send_system_message(gate_ref, joiner_session, "队伍已满或你已在队伍中");
                return;
            }
            // Update joiner's group_id
            if let Some(record) = players.get(&joiner_session) {
                let _ = record.ask(SetGroupId { group_id: Some(target_group_id) });
            }
            send_system_message(gate_ref, joiner_session, &format!("已加入队伍 #{}", target_group_id));
            broadcast_group_update(gate_ref, target_group_id, groups);
            debug!("Player {} joined group #{}", joiner_name, target_group_id);
        }
    } else {
        // Create new group
        let group_id = *next_group_id;
        *next_group_id += 1;

        let target_member = GroupMember {
            session_id: target_session,
            name: target_state.name.clone(),
            is_leader: true,
            online: true,
        };

        let mut group = Group::new(group_id, target_member);
        group.add_member(joiner_member);

        // Update both players' group_id
        if let Some(record) = players.get(&target_session) {
            let _ = record.ask(SetGroupId { group_id: Some(group_id) });
        }
        if let Some(record) = players.get(&joiner_session) {
            let _ = record.ask(SetGroupId { group_id: Some(group_id) });
        }

        groups.insert(group_id, group);
        send_system_message(gate_ref, joiner_session, &format!("队伍 #{} 已创建", group_id));
        send_system_message(gate_ref, target_session, &format!("队伍 #{} 已创建", group_id));
        debug!("Created group #{} with {} and {}", group_id, target_state.name, joiner_name);
    }
}

/// Leave a group.
pub fn leave_group(
    gate_ref: &ActorRef<GateActor>,
    players: &HashMap<u64, ActorRef<PlayerActor>>,
    groups: &mut HashMap<u64, Group>,
    group_id: u64,
    session_id: u64,
    name: &str,
) {
    if let Some(group) = groups.get_mut(&group_id) {
        if group.remove_member(session_id).is_some() {
            if let Some(record) = players.get(&session_id) {
                let _ = record.ask(SetGroupId { group_id: None });
            }
            send_system_message(gate_ref, session_id, "已离开队伍");
            debug!("Player {} left group #{}", name, group_id);

            if group.member_count() == 0 {
                groups.remove(&group_id);
            } else {
                broadcast_group_update(gate_ref, group_id, groups);
            }
        }
    }
}

/// Broadcast group update to all members.
pub fn broadcast_group_update(gate_ref: &ActorRef<GateActor>, group_id: u64, groups: &HashMap<u64, Group>) {
    if let Some(group) = groups.get(&group_id) {
        for member in &group.members {
            if member.online {
                send_group_members_map(gate_ref, member.session_id, &group.members);
            }
        }
    }
}

/// Handle player disconnect for group state.
pub async fn handle_player_group_disconnect(
    gate_ref: &ActorRef<GateActor>,
    players: &HashMap<u64, ActorRef<PlayerActor>>,
    groups: &mut HashMap<u64, Group>,
    session_id: u64,
) {
    let group_id = match players.get(&session_id) {
        Some(record) => {
            match record.ask(GetPlayerState).await {
                Ok(Some(s)) => s.group_id,
                _ => None,
            }
        }
        None => None,
    };

    if let Some(gid) = group_id {
        if let Some(group) = groups.get_mut(&gid) {
            group.set_online(session_id, false);
            for member in &group.members {
                if member.online && member.session_id != session_id {
                    send_group_members_map(gate_ref, member.session_id, &group.members);
                    send_system_message(gate_ref, member.session_id, &format!("{} 已离线", member.name));
                }
            }
        }
    }
}

/// Send guild storage item list to a client（C# S.GuildStorageList 语义，M32）
pub fn send_guild_storage_list_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, guild: &Guild) {
    use mir2_shared::enums::ServerPacketIds;
    use mir2_shared::data::client_data::GuildStorageItem;
    use mir2_shared::packets::base::Packet;
    use mir2_shared::packets::server::guild::GuildStorageList;
    let items: Vec<Option<GuildStorageItem>> = guild
        .storage_items
        .iter()
        .map(|slot| {
            slot.as_ref().map(|(item, qty)| GuildStorageItem {
                item: {
                    let mut it = item.clone();
                    it.count = (*qty).min(u16::MAX as u32) as u16;
                    it
                },
                user_id: 0,
            })
        })
        .collect();
    let mut body = Vec::new();
    let packet = GuildStorageList { items };
    match packet.write_body(&mut body) {
        Ok(()) => {
            let _ = gate_ref.tell(SendToClient {
                session_id,
                data: build_packet_bytes(ServerPacketIds::GuildStorageList as i16, &body),
            }).try_send();
        }
        Err(e) => tracing::error!("GuildStorageList write failed: {:?}", e),
    }
}
