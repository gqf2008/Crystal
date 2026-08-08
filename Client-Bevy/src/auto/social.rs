//! auto::social 自动化验证系统（从 auto.rs 拆分，#1146）

use bevy::prelude::*;
use super::*;

/// --group-test：自动组队邀请链路（登录后向 bevy2char 发 AddMember，等成员列表）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_group_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    group: Res<client_bevy::game::dialogs::group::GroupState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            let invitee = std::env::args()
                .skip_while(|a| a != "--e2e-invitee")
                .nth(1)
                .unwrap_or_else(|| "bevy2char".to_string());
            net.send_packet(&mir2_shared::packets::client::group::AddMember {
                name: invitee.clone(),
            });
            tracing::info!("[GROUPTEST] 邀请组队: {}", invitee);
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 5.0 {
                return;
            }
            if group.members.len() >= 2 {
                tracing::info!(
                    "[GROUPTEST] ✅ 组队成功: {}",
                    group.members.iter().map(|m| m.name.as_str()).collect::<Vec<_>>().join(", ")
                );
            } else {
                tracing::warn!("[GROUPTEST] ❌ 组队成员不足: {:?}", group.members);
            }
            *stage = 2;
        }
        _ => {}
    }
}

/// --group-accept：自动接受组队邀请（自动化验证用）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_group_accept(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    mut group: ResMut<client_bevy::game::dialogs::group::GroupState>,
    mut accepted: Local<bool>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    if *accepted {
        return;
    }
    if let Some(inv) = group.invite.clone() {
        net.send_packet(&mir2_shared::packets::client::group::GroupInvite {
            accept_invite: true,
        });
        tracing::info!("[GROUPACCEPT] ✅ 接受邀请: {}", inv.inviter_name);
        group.invite = None;
        *accepted = true;
    }
}

/// --whisper-send：进游戏 8s 后发送一次私聊 `/w bevy2char whisper-e2e`（真实服双开验证）
pub(crate) fn auto_whisper_send(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut t: Local<f32>,
    mut sent: Local<bool>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game || *sent {
        return;
    }
    *t += time.delta_secs();
    if *t < 8.0 {
        return;
    }
    *sent = true;
    net.send_packet(&mir2_shared::packets::client::chat::Chat {
        message: "/w bevy2char whisper-e2e".to_string(),
        linked_items: Vec::new(),
    });
    tracing::info!("[WHSEND] 发送私聊 /w bevy2char whisper-e2e");
}

/// --whisper-check：轮询聊天历史是否收到 `whisper-e2e` 私聊行 + last_pm 记录（#813）
pub(crate) fn auto_whisper_check(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    chat: Res<client_bevy::game::chat::ChatState>,
    mut t: Local<f32>,
    mut done: Local<bool>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game || *done {
        return;
    }
    *t += time.delta_secs();
    let hit = chat
        .lines
        .iter()
        .rev()
        .take(100)
        .find(|(text, _, _, _)| text.contains("whisper-e2e"))
        .map(|(text, _, ch, _)| (text.clone(), *ch));
    if let Some((text, ch)) = hit {
        let pm_ok = chat.last_pm.as_deref() == Some("bevychar");
        tracing::info!(
            "[WHCHECK] ✅ 收到私聊（channel={:?}）: {}；last_pm={}",
            ch,
            text,
            chat.last_pm.clone().unwrap_or_default()
        );
        if pm_ok {
            tracing::info!("[WHCHECK] ✅ last_pm 记录正确（bevychar）");
        } else {
            tracing::warn!("[WHCHECK] ⚠️ last_pm 未记录（{:?}）", chat.last_pm);
        }
        *done = true;
        return;
    }
    if *t >= 60.0 {
        tracing::warn!("[WHCHECK] ❌ 60s 未收到私聊 whisper-e2e");
        *done = true;
    }
}

/// --mail-test：自动发邮件（登录后向 bevy2char 发 SendMail，含金币）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_mail_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut t: Local<f32>,
    mut sent: Local<bool>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    if *sent {
        return;
    }
    *t += time.delta_secs();
    if *t < 8.0 {
        return;
    }
    let receiver = std::env::args()
        .skip_while(|a| a != "--e2e-receiver")
        .nth(1)
        .unwrap_or_else(|| "bevy2char".to_string());
    net.send_packet(&mir2_shared::packets::client::mail::SendMail {
        name: receiver.clone(),
        message: "HelloSubject\n邮件正文测试 100 金币".to_string(),
        gold: 100,
        items_idx: [0; 5],
        stamped: false,
    });
    tracing::info!("[MAILTEST] 发送邮件给 {} (含 100 金币)", receiver);
    *sent = true;
}

/// --mail-read：自动读取新邮件（收到列表条目 → ReadMail → 详情）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_mail_read(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    mail: Res<client_bevy::game::dialogs::mail::MailState>,
    mut read_ids: Local<std::collections::HashSet<u64>>,
    mut done: Local<bool>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    if *done {
        return;
    }
    if let Some(d) = mail.detail.as_ref() {
        tracing::info!(
            "[MAILREAD] ✅ 已读取邮件: {} - {} 金币={} 正文={}",
            d.sender,
            d.subject,
            d.gold,
            d.body
        );
        *done = true;
        return;
    }
    for m in mail.mails.iter() {
        if m.unread && !read_ids.contains(&m.mail_id) {
            net.send_packet(&mir2_shared::packets::client::mail::ReadMail {
                mail_id: m.mail_id,
            });
            tracing::info!("[MAILREAD] 请求读取: {} ({})", m.subject, m.mail_id);
            read_ids.insert(m.mail_id);
        }
    }
}

/// --trade-test：自动交易链路（发起者：TradeRequest → 金币 500 → 放入物品 → 锁定 → 完成）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_trade_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut trade: ResMut<client_bevy::game::dialogs::trade::TradeState>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            trade.is_initiator = true;
            net.send_packet(&mir2_shared::packets::client::trade::TradeRequest);
            tracing::info!("[TRADETEST] 发起交易请求");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 3.0 {
                return;
            }
            if trade.visible {
                tracing::info!("[TRADETEST] ✅ 交易窗口已打开，对方={}", trade.partner_name);
                net.send_packet(&mir2_shared::packets::client::trade::TradeGold { amount: 500 });
                tracing::info!("[TRADETEST] 放入金币 500");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 2.0 {
                return;
            }
            // 放入背包第一个物品
            if let Some((from, _)) = hud.inventory.items.iter().enumerate().find(|(_, s)| s.is_some()) {
                if trade.pending_deposit.is_none() && trade.my_items[0].is_none() {
                    trade.pending_deposit = Some((from, 0));
                    net.send_packet(&mir2_shared::packets::client::trade::DepositTradeItem {
                        from: from as i32,
                        to: 0,
                    });
                    tracing::info!("[TRADETEST] 放入背包格 {} -> 交易槽 0", from);
                    *stage = 3;
                    *t = 0.0;
                }
            }
        }
        3 => {
            if *t < 3.0 {
                return;
            }
            if trade.my_items[0].is_some() {
                tracing::info!("[TRADETEST] ✅ 物品已入槽: {}", trade.my_items[0].as_ref().unwrap().name);
                net.send_packet(&mir2_shared::packets::client::trade::TradeConfirm { locked: true });
                tracing::info!("[TRADETEST] 锁定交易");
                *stage = 4;
                *t = 0.0;
            }
        }
        4 => {
            if *t < 5.0 {
                return;
            }
            if !trade.visible {
                tracing::info!("[TRADETEST] 🎉 交易完成（窗口已关闭）");
            } else {
                tracing::warn!("[TRADETEST] ❌ 交易未完成，locked=({},{})", trade.my_locked, trade.their_locked);
            }
            *stage = 5;
        }
        _ => {}
    }
}

/// --trade-accept：自动接受交易邀请 + 加金币 300 + 锁定
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_trade_accept(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut trade: ResMut<client_bevy::game::dialogs::trade::TradeState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if trade.invite.is_some() {
                net.send_packet(&mir2_shared::packets::client::trade::TradeReply {
                    accept_invite: true,
                });
                tracing::info!(
                    "[TRADEACCEPT] ✅ 接受邀请: {}",
                    trade.invite.as_ref().unwrap()
                );
                trade.invite = None;
                trade.visible = true;
                *stage = 1;
                *t = 0.0;
            }
        }
        1 => {
            if *t < 2.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::trade::TradeGold { amount: 300 });
            tracing::info!("[TRADEACCEPT] 放入金币 300");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t < 3.0 {
                return;
            }
            if trade.their_locked && !trade.my_locked {
                net.send_packet(&mir2_shared::packets::client::trade::TradeConfirm { locked: true });
                tracing::info!("[TRADEACCEPT] 对方已锁定，我方锁定");
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            if *t < 5.0 {
                return;
            }
            if !trade.visible {
                tracing::info!("[TRADEACCEPT] 🎉 交易完成");
            }
            *stage = 4;
        }
        _ => {}
    }
}

/// --friend-test：自动加好友（AddFriend bevy2char → 等 FriendUpdate 列表出现）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_friend_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    friend: Res<client_bevy::game::dialogs::friend::FriendState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::friend::AddFriend {
                name: "bevy2char".to_string(),
                blocked: false,
            });
            tracing::info!("[FRIENDTEST] 添加好友 bevy2char");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 3.0 {
                return;
            }
            if friend.friends.iter().any(|f| f.name == "bevy2char") {
                tracing::info!(
                    "[FRIENDTEST] ✅ 好友列表包含 bevy2char (在线={})",
                    friend.friends.iter().find(|f| f.name == "bevy2char").map(|f| f.online).unwrap_or(false)
                );
            } else {
                tracing::warn!("[FRIENDTEST] ❌ 好友列表为空或未包含 bevy2char: {:?}", friend.friends);
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --mail-compose-test：写邮件界面（输入框状态 → send_composed_mail → B 读取）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_mail_compose_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mail: ResMut<client_bevy::game::dialogs::mail::MailState>,
    mut input: ResMut<client_bevy::game::dialogs::text_input::TextInputState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            // 打开邮件对话框 + 写界面（原版 C# MailDialog 写邮件流程）
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Mail) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Mail);
            }
            mail.compose = true;
            mail.detail = None;
            input.texts = vec![
                "bevy2char".to_string(),
                "ComposeSubject".to_string(),
                "邮件正文 M26 测试".to_string(),
                "100".to_string(),
            ];
            tracing::info!("[MAILCOMPOSE] 打开写邮件界面，填写收件人/主题/正文");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 2.0 {
                return;
            }
            // 与发送按钮相同的代码路径
            client_bevy::game::dialogs::mail::send_composed_mail(&net, &input, 100, &[]);
            mail.compose = false;
            tracing::info!("[MAILCOMPOSE] 发送邮件");
            *stage = 9;
        }
        _ => {}
    }
}

/// --guild-test：创建行会（打开行会对话框 → 输入行会名 → GuildNameReturn → 等 GuildStatus 信息）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_guild_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    guild: Res<client_bevy::game::dialogs::guild::GuildState>,
    mut input: ResMut<client_bevy::game::dialogs::text_input::TextInputState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Guild) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Guild);
            }
            if input.texts.len() < 1 {
                input.texts.resize(1, String::new());
            }
            input.texts[0] = "TestGuild".to_string();
            tracing::info!("[GUILDTEST] 打开行会对话框，输入行会名 TestGuild");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 1.0 {
                return;
            }
            // 与创建按钮相同：GuildNameReturn{name}
            net.send_packet(&mir2_shared::packets::client::guild::GuildNameReturn {
                name: "TestGuild".to_string(),
            });
            tracing::info!("[GUILDTEST] 创建行会 TestGuild");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t < 4.0 {
                return;
            }
            if guild.in_guild && guild.name == "TestGuild" {
                tracing::info!(
                    "[GUILDTEST] ✅ 行会创建成功: {}（{}）成员 {}",
                    guild.name,
                    guild.leader,
                    guild.members.len()
                );
            } else {
                tracing::warn!(
                    "[GUILDTEST] ❌ 行会状态: in_guild={} name={}",
                    guild.in_guild,
                    guild.name
                );
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --guild-invite-test：创建行会 → 邀请 bevy2char → 等成员数 2
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_guild_invite_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    guild: Res<client_bevy::game::dialogs::guild::GuildState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::guild::GuildNameReturn {
                name: "TestGuild2".to_string(),
            });
            tracing::info!("[GUILDINV] 创建行会 TestGuild2");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 3.0 {
                return;
            }
            if guild.in_guild && guild.name == "TestGuild2" {
                tracing::info!("[GUILDINV] ✅ 行会已创建");
                net.send_packet(&mir2_shared::packets::client::guild::EditGuildMember {
                    change_type: 0,
                    rank_index: 0,
                    name: "bevy2char".to_string(),
                    rank_name: String::new(),
                });
                tracing::info!("[GUILDINV] 邀请 bevy2char 加入");
                *stage = 2;
                *t = 0.0;
            } else {
                tracing::warn!("[GUILDINV] ❌ 行会未创建: {}", guild.name);
                *stage = 9;
            }
        }
        2 => {
            if *t < 5.0 {
                return;
            }
            if guild.members.iter().any(|m| m.name == "bevy2char") {
                tracing::info!(
                    "[GUILDINV] ✅ 成员加入: {} 人",
                    guild.members.len()
                );
            } else {
                tracing::warn!("[GUILDINV] ❌ 成员未加入: {:?}", guild.members);
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --guild-accept：自动接受行会邀请（GuildInvite → C.GuildInvite{true} → 等 in_guild）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_guild_accept(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut guild: ResMut<client_bevy::game::dialogs::guild::GuildState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if let Some(name) = guild.invite.clone() {
                net.send_packet(&mir2_shared::packets::client::guild::GuildInvite {
                    accept_invite: true,
                });
                tracing::info!("[GUILDACCEPT] ✅ 接受行会邀请: {}", name);
                guild.invite = None;
                *stage = 1;
                *t = 0.0;
            }
        }
        1 => {
            if *t < 4.0 {
                return;
            }
            if guild.in_guild {
                tracing::info!(
                    "[GUILDACCEPT] ✅ 已加入行会: {}",
                    guild.name
                );
            } else {
                tracing::warn!("[GUILDACCEPT] ❌ 未加入行会");
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --guild-notice-test：创建行会 → 设置公告 → 等 GuildNoticeChange 回包
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_guild_notice_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    guild: Res<client_bevy::game::dialogs::guild::GuildState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::guild::GuildNameReturn {
                name: "TestGuild3".to_string(),
            });
            tracing::info!("[GUILDNOTICE] 创建行会 TestGuild3");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 3.0 {
                return;
            }
            if guild.in_guild && guild.name == "TestGuild3" {
                net.send_packet(&mir2_shared::packets::client::guild::EditGuildNotice {
                    notice_lines: vec!["TestNotice 公告内容".to_string()],
                });
                tracing::info!("[GUILDNOTICE] 设置公告");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 3.0 {
                return;
            }
            if guild.notice.iter().any(|l| l.contains("TestNotice")) {
                tracing::info!("[GUILDNOTICE] ✅ 公告已更新: {:?}", guild.notice);
            } else {
                tracing::warn!("[GUILDNOTICE] ❌ 公告未更新: {:?}", guild.notice);
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --guild-gold-test：创建行会 → 存入 100 → 取出 50 → 验证仓库金币
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_guild_gold_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    guild: Res<client_bevy::game::dialogs::guild::GuildState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::guild::GuildNameReturn {
                name: "TestGuild4".to_string(),
            });
            tracing::info!("[GUILDGOLD] 创建行会 TestGuild4");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 3.0 {
                return;
            }
            if guild.in_guild && guild.name == "TestGuild4" {
                net.send_packet(&mir2_shared::packets::client::guild::GuildStorageGoldChange {
                    change_type: 0,
                    amount: 100,
                });
                tracing::info!("[GUILDGOLD] 存入 100 金币");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 3.0 {
                return;
            }
            if guild.gold >= 100 {
                tracing::info!("[GUILDGOLD] ✅ 仓库金币: {}", guild.gold);
                net.send_packet(&mir2_shared::packets::client::guild::GuildStorageGoldChange {
                    change_type: 1,
                    amount: 50,
                });
                tracing::info!("[GUILDGOLD] 取出 50 金币");
                *stage = 3;
                *t = 0.0;
            } else {
                tracing::warn!("[GUILDGOLD] ❌ 仓库金币未更新: {}", guild.gold);
                *stage = 9;
            }
        }
        3 => {
            if *t < 3.0 {
                return;
            }
            if guild.gold >= 50 {
                tracing::info!("[GUILDGOLD] ✅ 取出后仓库金币: {}", guild.gold);
            } else {
                tracing::warn!("[GUILDGOLD] ❌ 取出后金币异常: {}", guild.gold);
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --ranking-test：打开排行榜 → 等 Rankings 数据
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_ranking_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    ranking: Res<client_bevy::game::dialogs::ranking::RankingState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Ranking) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Ranking);
            }
            tracing::info!("[RANKTEST] 打开排行榜对话框");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 4.0 {
                return;
            }
            if !ranking.entries.is_empty() {
                tracing::info!(
                    "[RANKTEST] ✅ 排行榜 {} 条，第一名: {}",
                    ranking.entries.len(),
                    ranking.entries[0].player_name
                );
            } else {
                tracing::warn!("[RANKTEST] ❌ 排行榜为空");
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --guild-item-test：行会仓库物品链路（打开仓库 → 存入背包物品 → 取出）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_guild_item_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    guild: Res<client_bevy::game::dialogs::guild::GuildState>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut deposited_uid: Local<Option<u64>>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Guild) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Guild);
            }
            tracing::info!("[GUILDITEM] 打开行会对话框");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 3.0 {
                return;
            }
            if guild.in_guild {
                tracing::info!("[GUILDITEM] 已在行会: {}", guild.name);
                *stage = 2;
                *t = 0.0;
            } else {
                net.send_packet(&mir2_shared::packets::client::guild::GuildNameReturn {
                    name: "TestGuild5".to_string(),
                });
                tracing::info!("[GUILDITEM] 创建行会 TestGuild5");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 3.0 {
                return;
            }
            if !guild.in_guild {
                return;
            }
            // 请求仓库列表（打开对话框时已自动请求，这里兜底）
            net.send_packet(&client_bevy::network::GuildStorageItemChangeWire {
                change_type: 3,
                grid: 0,
                unique_id: 0,
                count: 0,
            });
            tracing::info!("[GUILDITEM] 请求仓库列表");
            *stage = 3;
            *t = 0.0;
        }
        3 => {
            if *t < 3.0 {
                return;
            }
            if guild.storage_received {
                tracing::info!(
                    "[GUILDITEM] ✅ 仓库列表 {} 格",
                    guild.storage_items.len()
                );
            } else {
                tracing::warn!("[GUILDITEM] ❌ 仓库列表未收到");
                *stage = 9;
                return;
            }
            // 选第一个背包物品存入
            let first = hud
                .inventory
                .items
                .iter()
                .enumerate()
                .find_map(|(i, s)| s.as_ref().map(|it| (i, it)));
            match first {
                Some((i, item)) => {
                    *deposited_uid = Some(item.unique_id);
                    net.send_packet(&client_bevy::network::GuildStorageItemChangeWire {
                        change_type: 0,
                        grid: 0,
                        unique_id: item.unique_id,
                        count: item.count as u32,
                    });
                    tracing::info!(
                        "[GUILDITEM] 存入背包物品 [{}] uid={} (格 {})",
                        item.name,
                        item.unique_id,
                        i
                    );
                    *stage = 4;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[GUILDITEM] ❌ 背包为空，无法测试存入");
                    *stage = 9;
                }
            }
        }
        4 => {
            if *t < 3.0 {
                return;
            }
            let slot0 = guild.storage_items.get(0).and_then(|s| s.as_ref());
            match slot0 {
                Some(it) => {
                    tracing::info!(
                        "[GUILDITEM] ✅ 仓库格1: {} x{} (uid={})",
                        it.name,
                        it.count,
                        it.unique_id
                    );
                    net.send_packet(&client_bevy::network::GuildStorageItemChangeWire {
                        change_type: 1,
                        grid: 0,
                        unique_id: 0,
                        count: 0,
                    });
                    tracing::info!("[GUILDITEM] 取出仓库格1");
                    *stage = 5;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[GUILDITEM] ❌ 仓库格1为空，存入失败");
                    *stage = 9;
                }
            }
        }
        5 => {
            if *t < 3.0 {
                return;
            }
            let slot0_empty = guild.storage_items.get(0).and_then(|s| s.as_ref()).is_none();
            let uid_back = match *deposited_uid {
                Some(uid) => hud
                    .inventory
                    .items
                    .iter()
                    .filter_map(|s| s.as_ref())
                    .any(|it| it.unique_id == uid),
                None => false,
            };
            if slot0_empty && uid_back {
                tracing::info!("[GUILDITEM] ✅ 取出成功：仓库格1已空，物品回到背包");
            } else {
                tracing::warn!(
                    "[GUILDITEM] ❌ 取出异常: slot0_empty={} uid_back={}",
                    slot0_empty,
                    uid_back
                );
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --mentor-test：发起拜师 → 等 MentorUpdate → 解除（配合 --mentor-accept）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_mentor_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mentor: Res<client_bevy::game::dialogs::mentor::MentorState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 12.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::misc::AddMentor {
                name: "bevy2char".to_string(),
            });
            tracing::info!("[MENTORTEST] 请求拜师 bevy2char");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 15.0 {
                tracing::warn!(
                    "[MENTORTEST] ❌ 未收到师徒关系: mentor_name={}",
                    mentor.mentor_name
                );
                *stage = 9;
                return;
            }
            if mentor.mentor_name == "bevy2char" {
                tracing::info!(
                    "[MENTORTEST] ✅ 拜师成功: 师父={} Lv.{} 在线={}",
                    mentor.mentor_name,
                    mentor.mentor_level,
                    mentor.mentor_online
                );
                net.send_packet(&mir2_shared::packets::client::misc::CancelMentor);
                tracing::info!("[MENTORTEST] 解除师徒关系");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 5.0 {
                return;
            }
            if mentor.mentor_name.is_empty() {
                tracing::info!("[MENTORTEST] ✅ 解除成功");
            } else {
                tracing::warn!(
                    "[MENTORTEST] ❌ 解除失败: mentor_name={}",
                    mentor.mentor_name
                );
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --mentor-accept：允许拜师 → 接受邀请 → 等 MentorUpdate → 等解除
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_mentor_accept(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mentor: Res<client_bevy::game::dialogs::mentor::MentorState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            net.send_packet(&client_bevy::network::AllowMentorWire { allow: true });
            tracing::info!("[MENTORACCEPT] 允许拜师");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 15.0 {
                tracing::warn!("[MENTORACCEPT] ❌ 未收到拜师邀请");
                *stage = 9;
                return;
            }
            if let Some((name, level)) = mentor.invite.as_ref() {
                tracing::info!("[MENTORACCEPT] ✅ 收到拜师邀请: {} Lv.{}", name, level);
                net.send_packet(&mir2_shared::packets::client::misc::MentorReply {
                    accept_invite: true,
                });
                tracing::info!("[MENTORACCEPT] 接受拜师");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t >= 15.0 {
                tracing::warn!(
                    "[MENTORACCEPT] ❌ 未收到师徒关系: mentor_name={}",
                    mentor.mentor_name
                );
                *stage = 9;
                return;
            }
            if mentor.mentor_name == "bevychar" {
                tracing::info!(
                    "[MENTORACCEPT] ✅ 收徒成功: 徒弟={} Lv.{} 在线={}",
                    mentor.mentor_name,
                    mentor.mentor_level,
                    mentor.mentor_online
                );
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            if *t < 10.0 {
                return;
            }
            if mentor.mentor_name.is_empty() {
                tracing::info!("[MENTORACCEPT] ✅ 对方解除，师徒关系已清除");
            } else {
                tracing::warn!(
                    "[MENTORACCEPT] ❌ 未收到解除: mentor_name={}",
                    mentor.mentor_name
                );
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --market-test：寄售背包物品×2 → 取回一件 → 留一件给买家（配合 --market-buy）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_market_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    market: Res<client_bevy::game::dialogs::market::MarketState>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut consigned: Local<Vec<u32>>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 12.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Market) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Market);
            }
            net.send_packet(&mir2_shared::packets::client::market::MarketRefresh);
            tracing::info!("[MARKETTEST] 打开市场 + 刷新");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 4.0 {
                return;
            }
            // 寄售第一个背包物品（uid=100，价格 500）
            let first = hud
                .inventory
                .items
                .iter()
                .enumerate()
                .find_map(|(i, s)| s.as_ref().map(|it| (i, it)));
            match first {
                Some((_i, item)) => {
                    net.send_packet(&mir2_shared::packets::client::market::ConsignItem {
                        unique_id: item.unique_id,
                        price: 500,
                        panel_type: mir2_shared::enums::MarketPanelType::Consign,
                    });
                    tracing::info!(
                        "[MARKETTEST] 寄售 [{}] uid={} 价格500",
                        item.name,
                        item.unique_id
                    );
                    consigned.push(item.unique_id as u32);
                    *stage = 2;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[MARKETTEST] ❌ 背包为空");
                    *stage = 9;
                }
            }
        }
        2 => {
            if *t < 5.0 {
                return;
            }
            if market.consign_ok.is_some() {
                tracing::info!(
                    "[MARKETTEST] ✅ 第一件寄售成功 uid={}",
                    market.consign_ok.unwrap_or(0)
                );
            } else {
                tracing::warn!("[MARKETTEST] ❌ 第一件寄售未确认");
                *stage = 9;
                return;
            }
            // 寄售第二件（uid=101，价格 600）
            let first = hud
                .inventory
                .items
                .iter()
                .enumerate()
                .find_map(|(i, s)| s.as_ref().map(|it| (i, it)));
            match first {
                Some((_i, item)) => {
                    net.send_packet(&mir2_shared::packets::client::market::ConsignItem {
                        unique_id: item.unique_id,
                        price: 600,
                        panel_type: mir2_shared::enums::MarketPanelType::Consign,
                    });
                    tracing::info!(
                        "[MARKETTEST] 寄售第二件 [{}] uid={} 价格600",
                        item.name,
                        item.unique_id
                    );
                    consigned.push(item.unique_id as u32);
                    *stage = 3;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[MARKETTEST] ❌ 背包只剩 0 件（应剩 1 件）");
                    *stage = 9;
                }
            }
        }
        3 => {
            if *t < 5.0 {
                return;
            }
            // 刷新市场，取回第二件（uid=101）
            net.send_packet(&mir2_shared::packets::client::market::MarketRefresh);
            tracing::info!("[MARKETTEST] 刷新市场准备取回");
            *stage = 4;
            *t = 0.0;
        }
        4 => {
            if *t < 5.0 {
                return;
            }
            let mine: Vec<&client_bevy::game::dialogs::market::MarketItem> = market
                .listings
                .iter()
                .filter(|it| it.seller == "bevychar")
                .collect();
            tracing::info!("[MARKETTEST] 我的寄售: {} 件", mine.len());
            let target = mine.iter().find(|it| it.unique_id == 101).copied();
            match target {
                Some(it) => {
                    net.send_packet(&client_bevy::network::MarketGetBackWire {
                        listing_id: it.auction_id as u32,
                    });
                    tracing::info!("[MARKETTEST] 取回商品 {} uid={}", it.auction_id, it.unique_id);
                    *stage = 5;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!(
                        "[MARKETTEST] ❌ 未找到 uid=101 的寄售: {:?}",
                        mine.iter().map(|x| x.unique_id).collect::<Vec<_>>()
                    );
                    *stage = 9;
                }
            }
        }
        5 => {
            if *t < 6.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::market::MarketRefresh);
            tracing::info!("[MARKETTEST] 取回后刷新市场");
            *stage = 6;
            *t = 0.0;
        }
        6 => {
            if *t < 5.0 {
                return;
            }
            let mine: Vec<&client_bevy::game::dialogs::market::MarketItem> = market
                .listings
                .iter()
                .filter(|it| it.seller == "bevychar")
                .collect();
            if mine.len() == 1 && mine[0].unique_id == 100 {
                tracing::info!(
                    "[MARKETTEST] ✅ 取回成功：剩 1 件寄售（uid=100 价格{}）",
                    mine[0].price
                );
            } else {
                tracing::warn!(
                    "[MARKETTEST] ❌ 取回后异常: mine={:?}",
                    mine.iter().map(|x| x.unique_id).collect::<Vec<_>>()
                );
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --market-buy：刷新市场 → 买下卖家 bevychar 的商品（配合 --market-test）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_market_buy(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    market: Res<client_bevy::game::dialogs::market::MarketState>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut bought_id: Local<Option<u64>>,
    mut last_refresh: Local<f32>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 45.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Market) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Market);
            }
            net.send_packet(&mir2_shared::packets::client::market::MarketRefresh);
            tracing::info!("[MARKETBUY] 打开市场 + 刷新");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 20.0 {
                tracing::warn!("[MARKETBUY] ❌ 未找到卖家 bevychar 的商品");
                *stage = 9;
                return;
            }
            // 等待期每 4 秒刷新一次市场（卖家可能尚未上架）
            if *t - *last_refresh >= 4.0 {
                *last_refresh = *t;
                net.send_packet(&mir2_shared::packets::client::market::MarketRefresh);
                tracing::info!("[MARKETBUY] 等待中刷新市场");
            }
            let target = market
                .listings
                .iter()
                .find(|it| it.seller == "bevychar" && it.unique_id == 100)
                .cloned();
            if let Some(it) = target {
                *bought_id = Some(it.auction_id);
                net.send_packet(&mir2_shared::packets::client::market::MarketBuy {
                    auction_id: it.auction_id,
                    bid_price: 0,
                });
                tracing::info!(
                    "[MARKETBUY] 购买商品 {} [{}] {}金币",
                    it.auction_id,
                    it.name,
                    it.price
                );
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t >= 15.0 {
                tracing::warn!("[MARKETBUY] ❌ 购买未确认: message={}", market.message);
                *stage = 9;
                return;
            }
            if market.message.contains("购买成功") {
                tracing::info!("[MARKETBUY] ✅ 购买成功: {}", market.message);
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            if *t < 5.0 {
                return;
            }
            // 验证物品进入背包（item_index=853）
            let has = hud
                .inventory
                .items
                .iter()
                .filter_map(|s| s.as_ref())
                .any(|it| it.item_index == 853);
            if has {
                tracing::info!("[MARKETBUY] ✅ 购买的物品已进入背包");
            } else {
                tracing::warn!("[MARKETBUY] ❌ 背包未见购买的物品");
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --gameshop-test：打开商城 → 请求目录 → 购买第一件可负担商品 → 邮件送达
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_gameshop_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    shop: Res<client_bevy::game::dialogs::game_shop::GameShopState>,
    mail: Res<client_bevy::game::dialogs::mail::MailState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut bought_item: Local<Option<i32>>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::GameShop) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::GameShop);
            }
            tracing::info!("[SHOPTEST] 打开商城（自动请求目录）");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 10.0 {
                tracing::warn!("[SHOPTEST] ❌ 商城目录未收到");
                *stage = 9;
                return;
            }
            if !shop.items.is_empty() {
                tracing::info!(
                    "[SHOPTEST] ✅ 商城目录 {} 件，我的金币 {}",
                    shop.items.len(),
                    shop.gold
                );
                // 选第一件金币价 <= 我的金币 的商品
                let target = shop.items.iter().find(|it| it.gold_price > 0);
                match target {
                    Some(it) => {
                        *bought_item = Some(it.item_index);
                        net.send_packet(&client_bevy::network::GameshopBuyWire {
                            item_id: it.item_index as u32,
                            quantity: 1,
                        });
                        tracing::info!(
                            "[SHOPTEST] 购买 #{} {} {}金币",
                            it.item_index,
                            it.name,
                            it.gold_price
                        );
                        *stage = 2;
                        *t = 0.0;
                    }
                    None => {
                        tracing::warn!("[SHOPTEST] ❌ 目录为空或没有可购买商品");
                        *stage = 9;
                    }
                }
            }
        }
        2 => {
            if *t >= 12.0 {
                tracing::warn!("[SHOPTEST] ❌ 未收到购买邮件");
                *stage = 9;
                return;
            }
            if mail.mails.iter().any(|m| m.sender == "GameShop") {
                let ms: Vec<String> = mail
                    .mails
                    .iter()
                    .filter(|m| m.sender == "GameShop")
                    .map(|m| format!("{}: {}", m.sender, m.subject))
                    .collect();
                tracing::info!("[SHOPTEST] ✅ 购买邮件送达: {:?}", ms);
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            if *t < 3.0 {
                return;
            }
            tracing::info!(
                "[SHOPTEST] ✅ 完成（购买 #{}）",
                bought_item.unwrap_or(-1)
            );
            *stage = 9;
        }
        _ => {}
    }
}

/// --territory-test：打开行会领地 → 购买第一个无主领地 → 向 TestGuildWar 宣战
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_territory_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    territory: Res<client_bevy::game::dialogs::guild_territory::GuildTerritoryState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut bought_id: Local<Option<i32>>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::GuildTerritory) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::GuildTerritory);
            }
            tracing::info!("[TERRTEST] 打开行会领地（自动请求列表）");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 10.0 {
                tracing::warn!("[TERRTEST] ❌ 领地列表未收到");
                *stage = 9;
                return;
            }
            if !territory.rows.is_empty() {
                tracing::info!(
                    "[TERRTEST] ✅ 领地列表 {} 个",
                    territory.rows.len()
                );
                let free = territory
                    .rows
                    .iter()
                    .find(|r| r.owner.is_empty())
                    .cloned();
                match free {
                    Some(r) => {
                        *bought_id = Some(r.id);
                        net.send_packet(&client_bevy::network::PurchaseGuildTerritoryWire {
                            territory_id: r.id as u32,
                        });
                        tracing::info!("[TERRTEST] 购买领地 #{}", r.id);
                        *stage = 2;
                        *t = 0.0;
                    }
                    None => {
                        tracing::warn!("[TERRTEST] ❌ 没有无主领地");
                        *stage = 9;
                    }
                }
            }
        }
        2 => {
            if *t < 6.0 {
                return;
            }
            // 重新请求列表验证购买
            net.send_packet(&client_bevy::network::GuildTerritoryPageWire { page: 0 });
            tracing::info!("[TERRTEST] 购买后刷新领地列表");
            *stage = 3;
            *t = 0.0;
        }
        3 => {
            if *t < 6.0 {
                return;
            }
            let id = bought_id.unwrap_or(-1);
            let row = territory.rows.iter().find(|r| r.id == id);
            match row {
                Some(r) if r.owner == "TestGuild4" => {
                    tracing::info!(
                        "[TERRTEST] ✅ 购买成功：领地 #{} 归属 {}",
                        r.id,
                        r.owner
                    );
                    *stage = 4;
                    *t = 0.0;
                }
                Some(r) => {
                    tracing::warn!(
                        "[TERRTEST] ❌ 领地 #{} 归属异常: {}",
                        r.id,
                        r.owner
                    );
                    *stage = 9;
                }
                None => {
                    tracing::warn!("[TERRTEST] ❌ 领地 #{} 不存在", id);
                    *stage = 9;
                }
            }
        }
        4 => {
            if *t < 6.0 {
                return;
            }
            // 向 TestGuildWar 宣战（--territory-war 客户端先创建）
            net.send_packet(&mir2_shared::packets::client::guild::GuildWarReturn {
                guild_name: "TestGuildWar".to_string(),
            });
            tracing::info!("[TERRTEST] 向 TestGuildWar 宣战");
            *stage = 5;
            *t = 0.0;
        }
        5 => {
            if *t >= 10.0 {
                tracing::warn!("[TERRTEST] ❌ 未收到宣战确认");
                *stage = 9;
                return;
            }
            if territory.war_message.contains("TestGuildWar") {
                tracing::info!("[TERRTEST] ✅ 宣战成功: {}", territory.war_message);
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --territory-war：创建目标行会 TestGuildWar（供 --territory-test 宣战）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_territory_war(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    guild: Res<client_bevy::game::dialogs::guild::GuildState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            if guild.in_guild && guild.name == "TestGuildWar" {
                tracing::info!("[TERRWAR] ✅ 已在行会 TestGuildWar");
                *stage = 9;
                return;
            }
            net.send_packet(&mir2_shared::packets::client::guild::GuildNameReturn {
                name: "TestGuildWar".to_string(),
            });
            tracing::info!("[TERRWAR] 创建行会 TestGuildWar");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 8.0 {
                return;
            }
            if guild.in_guild && guild.name == "TestGuildWar" {
                tracing::info!("[TERRWAR] ✅ 行会创建成功");
                *stage = 9;
            } else {
                tracing::warn!("[TERRWAR] ❌ 行会创建失败");
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --marriage-test（求婚方）：求婚 → 等 LoverUpdate → 离婚
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_marriage_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    relationship: Res<client_bevy::game::dialogs::relationship::RelationshipState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            net.send_packet(&client_bevy::network::MarriageRequestWire {
                target_name: "bevy2char".to_string(),
            });
            tracing::info!("[MARRY] 向 bevy2char 求婚");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 15.0 {
                tracing::warn!("[MARRY] ❌ 未结婚（married={}）", relationship.married);
                *stage = 9;
                return;
            }
            if relationship.married {
                tracing::info!("[MARRY] ✅ 结婚成功");
                net.send_packet(&client_bevy::network::DivorceRequestWire {
                    partner_name: "bevy2char".to_string(),
                });
                tracing::info!("[MARRY] 发起离婚");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t >= 15.0 {
                tracing::warn!("[MARRY] ❌ 未离婚（married={}）", relationship.married);
                *stage = 9;
                return;
            }
            if !relationship.married {
                tracing::info!("[MARRY] ✅ 离婚成功");
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --marriage-accept（被求婚方）：接受求婚 → 等结婚 → 离婚确认
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_marriage_accept(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    relationship: Res<client_bevy::game::dialogs::relationship::RelationshipState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t >= 20.0 {
                tracing::warn!("[MARRYACC] ❌ 未收到求婚");
                *stage = 9;
                return;
            }
            if relationship.invite.is_some() {
                tracing::info!(
                    "[MARRYACC] ✅ 收到求婚: {}",
                    relationship.invite.clone().unwrap_or_default()
                );
                net.send_packet(&mir2_shared::packets::client::misc::MarriageReply {
                    accept_invite: true,
                });
                tracing::info!("[MARRYACC] 接受求婚");
                *stage = 1;
                *t = 0.0;
            }
        }
        1 => {
            if *t >= 15.0 {
                tracing::warn!("[MARRYACC] ❌ 未结婚");
                *stage = 9;
                return;
            }
            if relationship.married {
                tracing::info!("[MARRYACC] ✅ 已婚");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            // 等待对方离婚请求并确认
            if relationship.message.contains("离婚请求") {
                tracing::info!("[MARRYACC] ✅ 收到离婚请求，确认");
                net.send_packet(&mir2_shared::packets::client::misc::DivorceReply {
                    accept_invite: true,
                });
                *stage = 3;
                *t = 0.0;
            }
            if *t >= 20.0 {
                tracing::warn!("[MARRYACC] ❌ 未收到离婚请求");
                *stage = 9;
            }
        }
        3 => {
            if *t < 5.0 {
                return;
            }
            if !relationship.married {
                tracing::info!("[MARRYACC] ✅ 离婚完成");
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --member-test：施法 → mock 回发 SendMemberLocation("队友A",356,350)，断言 MemberLocations（#254）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_member_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    members: Res<client_bevy::game::dialogs::minimap::MemberLocations>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[MEMBER] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[MEMBER] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[MEMBER] 🔥 施法触发成员点位");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t >= 2.5 {
                let ok = members
                    .members
                    .iter()
                    .any(|(n, _m, x, y)| n == "队友A" && *x == 356 && *y == 350);
                tracing::info!("[MEMBER] 成员点位={}", ok);
                if ok {
                    tracing::info!("[MEMBER] ✅ 小队成员点位通过");
                } else {
                    tracing::warn!("[MEMBER] ❌ 成员点位未更新");
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --notice-test：施法 → mock 回发 UpdateNotice(["服务器公告","欢迎来到传奇2",...])，
/// 断言 NoticeState.notices 更新（#256）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_notice_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    notice: Res<client_bevy::game::dialogs::notice::NoticeState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[NOTICE] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[NOTICE] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[NOTICE] 🔥 施法触发公告");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t >= 2.5 {
                let ok = notice.title == "服务器公告" && notice.message.contains("欢迎来到传奇2");
                tracing::info!(
                    "[NOTICE] title={} message_len={} 内容={}",
                    notice.title,
                    notice.message.len(),
                    ok
                );
                if ok {
                    tracing::info!("[NOTICE] ✅ 服务器公告通过");
                } else {
                    tracing::warn!("[NOTICE] ❌ 公告未更新");
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --guild-storage-realtime-test：行会仓库实时同步（#295）
/// 流程：进游戏 → 施法触发演示批次（GuildStorageGoldChange 500 / ItemChange 存入槽0）→ 断言 guild 状态
pub(crate) fn auto_guild_storage_realtime_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    guild: Res<client_bevy::game::dialogs::guild::GuildState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 6.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: mir2_shared::enums::MirDirection::Down,
                target_id: 101,
                location: mir2_shared::Point { x: 353, y: 352 },
            });
            tracing::info!("[GSTORE] 🔥 施法触发行会仓库实时包");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 2.0 {
                return;
            }
            let gold_ok = guild.gold == 500;
            let item_ok = guild.storage_items.get(0).and_then(|s| s.as_ref()).is_some();
            tracing::info!("[GSTORE] 金币={} 仓库槽0={}", guild.gold, item_ok);
            if gold_ok && item_ok {
                tracing::info!("[GSTORE] ✅ PASS 行会仓库实时同步");
            } else {
                tracing::error!("[GSTORE] ❌ FAIL 金币={} 槽0={}", gold_ok, item_ok);
            }
            *stage = 9;
        }
        _ => {}
    }
}


