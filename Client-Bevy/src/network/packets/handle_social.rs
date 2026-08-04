use bevy::prelude::*;
use mir2_shared::packets::base::{Packet, PacketHeader};
use crate::network::*;
use crate::ui::login::AuthFeedback;
use super::*;

// 网络包解码分派（#72 拆分）：handle_social 处理 arms_social.rs 的服务端包分支。
// 由 packets.rs::handle_packet 调度器按 opcode 调用；返回 true 表示已处理。

#[allow(clippy::too_many_arguments, unused_variables)]
pub(crate) fn handle_social(
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
    const HANDLED: &[i16] = &[ServerPacketIds::FishingUpdate as i16, ServerPacketIds::MentorRequest as i16, ServerPacketIds::MentorUpdate as i16, ServerPacketIds::GuildNoticeChange as i16, ServerPacketIds::GuildMemberChange as i16, ServerPacketIds::Rankings as i16, ServerPacketIds::GuildInvite as i16, ServerPacketIds::FriendUpdate as i16, ServerPacketIds::TradeRequest as i16, ServerPacketIds::TradeGold as i16, ServerPacketIds::TradeConfirm as i16, ServerPacketIds::TradeCancel as i16, ServerPacketIds::TradeItem as i16, ServerPacketIds::DepositTradeItem as i16, ServerPacketIds::ReceiveMail as i16, ServerPacketIds::GroupMembersMap as i16, ServerPacketIds::GroupInvite as i16, ServerPacketIds::DeleteGroup as i16, ServerPacketIds::DeleteMember as i16, ServerPacketIds::NewMagic as i16, ServerPacketIds::MagicDelay as i16, ServerPacketIds::MagicCast as i16, ServerPacketIds::KeepAlive as i16];
    let handled = HANDLED.contains(&opcode);
    match opcode {
        // ---- M39: 钓鱼 ----
        x if x == ServerPacketIds::FishingUpdate as i16 => {
            // [progress i32][success u8]
            let body = &payload[PacketHeader::HEADER_SIZE..];
            if body.len() >= 5 {
                let progress = i32::from_le_bytes(body[0..4].try_into().unwrap_or([0; 4]));
                let success = body[4] != 0;
                server_events.write(ServerEvent::FishingUpdate { progress, success });
            }
        }
        x if x == ServerPacketIds::MentorRequest as i16 => {
            // [name dotnet][level u16]（C# S.MentorRequest）
            let body = &payload[PacketHeader::HEADER_SIZE..];
            let mut cur = std::io::Cursor::new(body);
            match mir2_shared::binary::read_dotnet_string(&mut cur) {
                Ok(name) => {
                    let mut lb = [0u8; 2];
                    let level = if std::io::Read::read_exact(&mut cur, &mut lb).is_ok() {
                        u16::from_le_bytes(lb)
                    } else {
                        0
                    };
                    server_events.write(ServerEvent::MentorInvite {
                        name: name.clone(),
                        level,
                    });
                    tracing::info!("🧑‍🏫 收到拜师邀请: {} Lv.{}", name, level);
                }
                Err(e) => {
                    tracing::warn!("⚠️ MentorRequest 解析失败: {} (len={})", e, payload.len())
                }
            }
        }
        x if x == ServerPacketIds::MentorUpdate as i16 => {
            use byteorder::ReadBytesExt;
            // [name dotnet][level i32][online u8][exp i64]（C# S.MentorUpdate 语义）
            let body = &payload[PacketHeader::HEADER_SIZE..];
            let mut cur = std::io::Cursor::new(body);
            let name = mir2_shared::binary::read_dotnet_string(&mut cur).unwrap_or_default();
            let mut lb = [0u8; 4];
            let level = if std::io::Read::read_exact(&mut cur, &mut lb).is_ok() {
                i32::from_le_bytes(lb).max(0) as u32
            } else {
                0
            };
            let online = cur.read_u8().unwrap_or(0) != 0;
            let mut eb = [0u8; 8];
            let exp = if std::io::Read::read_exact(&mut cur, &mut eb).is_ok() {
                i64::from_le_bytes(eb)
            } else {
                0
            };
            server_events.write(ServerEvent::MentorUpdate {
                name: name.clone(),
                level,
                online,
                mentee_exp: exp,
            });
            tracing::info!(
                "🧑‍🏫 师徒更新: {} Lv.{} 在线={} 经验={}",
                if name.is_empty() { "无" } else { &name },
                level,
                online,
                exp
            );
        }
        x if x == ServerPacketIds::GuildNoticeChange as i16 => {
            use byteorder::{LittleEndian, ReadBytesExt};
            // [count u8][lines dotnet...]
            let body = &payload[PacketHeader::HEADER_SIZE..];
            let mut cur = std::io::Cursor::new(body);
            let count = cur.read_u8().unwrap_or(0) as usize;
            let mut notice = Vec::new();
            for _ in 0..count {
                match mir2_shared::binary::read_dotnet_string(&mut cur) {
                    Ok(l) => notice.push(l),
                    Err(_) => break,
                }
            }
            server_events.write(ServerEvent::GuildNotice { notice: notice.clone() });
            tracing::info!("🏰 行会公告更新: {:?}", notice);
        }
        x if x == ServerPacketIds::GuildMemberChange as i16 => {
            use byteorder::{LittleEndian, ReadBytesExt};
            // 双格式：加入/离开 [joined u8][name dotnet] / 成员更新 [name dotnet][rank u8][online u8]
            let body = &payload[PacketHeader::HEADER_SIZE..];
            let mut handled = false;
            if body.len() >= 2 && body[0] <= 1 {
                let mut cur = std::io::Cursor::new(&body[1..]);
                if let Ok(name) = mir2_shared::binary::read_dotnet_string(&mut cur) {
                    if cur.position() as usize == body.len() - 1 {
                        let joined = body[0] != 0;
                        tracing::info!("🏰 行会成员{}: {}", if joined { "加入" } else { "离开" }, name);
                        if joined {
                            server_events.write(ServerEvent::GuildMemberChanged {
                                name: name.clone(),
                                rank: 2,
                                online: true,
                                joined: true,
                                removed: false,
                            });
                        } else {
                            server_events.write(ServerEvent::GuildMemberChanged {
                                name: name.clone(),
                                rank: 0,
                                online: false,
                                joined: false,
                                removed: true,
                            });
                        }
                        handled = true;
                    }
                }
            }
            if !handled {
                let mut cur = std::io::Cursor::new(body);
                if let Ok(name) = mir2_shared::binary::read_dotnet_string(&mut cur) {
                    let rank = cur.read_u8().unwrap_or(2);
                    let online = cur.read_u8().unwrap_or(0) != 0;
                    server_events.write(ServerEvent::GuildMemberChanged {
                        name: name.clone(),
                        rank,
                        online,
                        joined: false,
                        removed: false,
                    });
                    tracing::info!("🏰 行会成员更新: {} rank={} online={}", name, rank, online);
                }
            }
        }

        // ---- M31: 排行榜 ----
        x if x == ServerPacketIds::Rankings as i16 => {
            use byteorder::ReadBytesExt;
            // 手动解析服务端实际 wire：[rank_type u8][my_rank i32][count i32]
            //   [per: rank i32][name dotnet][class u8][level i32][exp i64]...[listings_count i32][count i32]
            let body = &payload[PacketHeader::HEADER_SIZE..];
            let mut cur = std::io::Cursor::new(body);
            let _rank_type = cur.read_u8().unwrap_or(0);
            let mut my_rank_buf = [0u8; 4];
            if std::io::Read::read_exact(&mut cur, &mut my_rank_buf).is_err() {
                server_events.write(ServerEvent::RankingsCleared);
                tracing::warn!("⚠️ Rankings 解析失败: (len={})", payload.len());
            } else {
                let mut count_buf = [0u8; 4];
                let count = if std::io::Read::read_exact(&mut cur, &mut count_buf).is_ok() {
                    i32::from_le_bytes(count_buf).max(0) as usize
                } else {
                    0
                };
                let mut entries = Vec::new();
                let mut ok = true;
                for _ in 0..count {
                    let mut rb = [0u8; 4];
                    if std::io::Read::read_exact(&mut cur, &mut rb).is_err() { ok = false; break; }
                    let rank = i32::from_le_bytes(rb);
                    let player_name = match mir2_shared::binary::read_dotnet_string(&mut cur) {
                        Ok(n) => n,
                        Err(_) => { ok = false; break; }
                    };
                    let class = cur.read_u8().unwrap_or(0);
                    let mut lb = [0u8; 4];
                    if std::io::Read::read_exact(&mut cur, &mut lb).is_err() { ok = false; break; }
                    let level = i32::from_le_bytes(lb);
                    let mut eb = [0u8; 8];
                    if std::io::Read::read_exact(&mut cur, &mut eb).is_err() { ok = false; break; }
                    let experience = i64::from_le_bytes(eb);
                    entries.push(RankEntry { rank, player_name, class, level, experience });
                }
                if ok {
                    let count = entries.len();
                    server_events.write(ServerEvent::Rankings { entries });
                    tracing::info!("🏅 排行榜: {} 条", count);
                } else {
                    tracing::warn!("⚠️ Rankings 解析失败: (len={})", payload.len());
                }
            }
        }

        // ---- M28: 行会邀请 ----
        x if x == ServerPacketIds::GuildInvite as i16 => {
            // [guild_name dotnet]（C# S.GuildInvite{Name}）
            let body = &payload[PacketHeader::HEADER_SIZE..];
            match mir2_shared::binary::read_dotnet_string(&mut std::io::Cursor::new(body)) {
                Ok(name) => {
                    server_events.write(ServerEvent::GuildInvited { name: name.clone() });
                    tracing::info!("🏰 收到行会邀请: {}", name);
                }
                Err(e) => tracing::warn!("⚠️ GuildInvite 解析失败: {} (len={})", e, payload.len()),
            }
        }

        // ---- M25: 好友 ----
        x if x == ServerPacketIds::FriendUpdate as i16 => {
            // 服务端 wire：列表包 [count i32][oid u32][name][memo][online]... / 单个包 [oid u32][name][memo][online]
            let body = &payload[PacketHeader::HEADER_SIZE..];
            let mut parsed: Option<Vec<FriendEntry>> = None;
            if body.len() >= 4 {
                let count = i32::from_le_bytes(body[0..4].try_into().unwrap_or([0; 4]));
                if (0..=200).contains(&count) {
                    let mut entries = Vec::new();
                    let mut cur = std::io::Cursor::new(&body[4..]);
                    let mut ok = true;
                    for _ in 0..count {
                        let mut oid_buf = [0u8; 4];
                        if std::io::Read::read_exact(&mut cur, &mut oid_buf).is_err() { ok = false; break; }
                        let object_id = u32::from_le_bytes(oid_buf);
                        let name = match mir2_shared::binary::read_dotnet_string(&mut cur) {
                            Ok(n) => n,
                            Err(_) => { ok = false; break; }
                        };
                        let memo = match mir2_shared::binary::read_dotnet_string(&mut cur) {
                            Ok(m) => m,
                            Err(_) => { ok = false; break; }
                        };
                        let mut online_buf = [0u8; 1];
                        if std::io::Read::read_exact(&mut cur, &mut online_buf).is_err() { ok = false; break; }
                        entries.push(FriendEntry { object_id, name, memo, online: online_buf[0] != 0 });
                    }
                    if ok && count as usize == entries.len() {
                        parsed = Some(entries);
                    }
                }
            }
            if parsed.is_none() {
                // 单个添加包
                let mut cur = std::io::Cursor::new(body);
                let mut oid_buf = [0u8; 4];
                if std::io::Read::read_exact(&mut cur, &mut oid_buf).is_ok() {
                    let object_id = u32::from_le_bytes(oid_buf);
                    if let (Ok(name), Ok(memo)) = (
                        mir2_shared::binary::read_dotnet_string(&mut cur),
                        mir2_shared::binary::read_dotnet_string(&mut cur),
                    ) {
                        let mut online_buf = [0u8; 1];
                        let online = std::io::Read::read_exact(&mut cur, &mut online_buf).is_ok() && online_buf[0] != 0;
                        parsed = Some(vec![FriendEntry { object_id, name, memo, online }]);
                    }
                }
            }
            match parsed {
                Some(entries) => {
                    server_events.write(ServerEvent::FriendUpdated { entries: entries.clone() });
                    tracing::info!(
                        "👥 好友列表: {}",
                        friend
                            .friends
                            .iter()
                            .map(|f| format!("{}{}", f.name, if f.online { "(在线)" } else { "" }))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                None => tracing::warn!("⚠️ FriendUpdate 解析失败: (len={})", payload.len()),
            }
        }

        // ---- M23: 交易 ----
        x if x == ServerPacketIds::TradeRequest as i16 => {
            use mir2_shared::binary::read_dotnet_string;
            match read_dotnet_string(&mut cur) {
                Ok(name) => {
                    server_events.write(ServerEvent::TradeRequested { name: name.clone() });
                    tracing::info!("🤝 交易请求: {}", name);
                }
                Err(e) => tracing::warn!("⚠️ TradeRequest 解析失败: {} (len={})", e, payload.len()),
            }
        }
        x if x == ServerPacketIds::TradeGold as i16 => {
            // 服务端 wire：[amount: u64 LE]
            if payload.len() >= 12 {
                let amount = u64::from_le_bytes(payload[4..12].try_into().unwrap_or([0; 8]));
                server_events.write(ServerEvent::TradeGold { amount });
                tracing::info!("💰 对方交易金币: {}", amount);
            }
        }
        x if x == ServerPacketIds::TradeConfirm as i16 => {
            // 服务端 wire：[side_a.locked u8][side_b.locked u8]（a=发起者）
            if payload.len() >= 6 {
                let a = payload[4] != 0;
                let b = payload[5] != 0;
                server_events.write(ServerEvent::TradeConfirm { a_locked: a, b_locked: b });
                tracing::info!("🔒 交易锁定状态: 我={} 对方={}", a, b);
                if a && b {
                    tracing::info!("🎉 交易完成！");
                }
            }
        }
        x if x == ServerPacketIds::TradeCancel as i16 => {
            server_events.write(ServerEvent::TradeCancelled);
            tracing::info!("🚫 交易已取消/关闭");
        }
        x if x == ServerPacketIds::TradeItem as i16 => {
            // 服务端 wire：[uid u64][grid u8][count u16][is_add u8]（对方物品更新）
            if payload.len() >= 15 {
                let uid = u64::from_le_bytes(payload[4..12].try_into().unwrap_or([0; 8]));
                let grid = payload[12] as usize;
                let count = u16::from_le_bytes(payload[13..15].try_into().unwrap_or([0; 2]));
                let is_add = payload[15] != 0;
                server_events.write(ServerEvent::TradeItemUpdate {
                    uid,
                    grid,
                    count,
                    is_add,
                });
                if is_add {
                    tracing::info!("📦 对方放入交易物品 uid={} 槽={} x{}", uid, grid, count);
                } else {
                    tracing::info!("↩️ 对方取回物品 uid={}", uid);
                }
            }
        }
        x if x == ServerPacketIds::DepositTradeItem as i16 => {
            // 服务端响应：[from_slot i32][success u8]
            if payload.len() >= 9 {
                let success = payload[8] != 0;
                let from = i32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
                server_events.write(ServerEvent::TradeDeposit { from, to: 0, success });
                if success {
                    tracing::info!("✅ 物品已放入交易槽");
                } else {
                    tracing::warn!("❌ 放入交易失败");
                }
            }
        }

        // ---- M22: 邮件 ----
        x if x == ServerPacketIds::ReceiveMail as i16 => {
            match parse_receive_mail(&payload[PacketHeader::HEADER_SIZE..]) {
                Some((entry, detail)) => {
                    server_events.write(ServerEvent::MailReceived { entry, detail });
                    tracing::info!("📧 邮件已广播");
                }
                None => tracing::warn!("⚠️ ReceiveMail 解析失败: (len={})", payload.len()),
            }
        }

        // ---- M21: 组队 ----
        x if x == ServerPacketIds::GroupMembersMap as i16 => {
            match group::GroupMembersMap::read_body(&mut cur) {
                Ok(p) => {
                    let member_count = p.members.len();
                    server_events.write(ServerEvent::GroupMembers {
                        members: p.members,
                    });
                    tracing::info!("👥 组队成员已广播: {} 人", member_count);
                }
                Err(e) => {
                    tracing::warn!("⚠️ GroupMembersMap 解析失败: {} (len={})", e, payload.len())
                }
            }
        }
        x if x == ServerPacketIds::GroupInvite as i16 => {
            match group::GroupInvite::read_body(&mut cur) {
                Ok(p) => {
                    server_events.write(ServerEvent::GroupInvite {
                        inviter_name: p.name.clone(),
                        inviter_id: p.inviter_id,
                    });
                    tracing::info!("👥 收到组队邀请: {} (id={})", p.name, p.inviter_id);
                }
                Err(e) => {
                    tracing::warn!("⚠️ GroupInvite 解析失败: {} (len={})", e, payload.len())
                }
            }
        }
        x if x == ServerPacketIds::DeleteGroup as i16 => {
            server_events.write(ServerEvent::GroupDeleted);
            tracing::info!("👥 组队已解散");
        }
        x if x == ServerPacketIds::DeleteMember as i16 => {
            if let Ok(p) = group::DeleteMember::read_body(&mut cur) {
                server_events.write(ServerEvent::GroupMemberLeft { name: p.name.clone() });
                tracing::info!("👥 成员离开: {}", p.name);
            }
        }
        x if x == ServerPacketIds::NewMagic as i16 => {
            if let Ok(p) = magic::NewMagic::read_body(&mut cur) {
                if !p.hero {
                    server_events.write(ServerEvent::MagicLearned { magic: p.magic.clone() });
                    tracing::info!(
                        "📖 学会技能: {} ({:?}) key={}",
                        p.magic.name,
                        p.magic.spell,
                        p.magic.key
                    );
                }
            }
        }
        x if x == ServerPacketIds::MagicDelay as i16 => {
            if let Ok(p) = MagicDelay::read_body(&mut cur) {
                tracing::debug!("⏳ 技能冷却: object={} spell={:?} delay={}ms", p.object_id, p.spell, p.delay);
            }
        }
        x if x == ServerPacketIds::MagicCast as i16 => {
            if let Ok(p) = MagicCast::read_body(&mut cur) {
                tracing::info!("🪄 MagicCast: spell={:?}", p.spell);
                // M38：有选中目标 → 生成魔法弹道特效
                if let Some(tid) = control.attack_target {
                    effects.write(PendingEffect::Projectile {
                        target_id: tid,
                        color: [1.0, 0.6, 0.2],
                    });
                }
            }
        }

        x if x == ServerPacketIds::KeepAlive as i16 => {
            // 服务器心跳：回一个 KeepAlive
            net.send_packet(&mir2_shared::packets::client::connection::KeepAlive { time: 0 });
        }
        _ => {}
    }
    handled
}
