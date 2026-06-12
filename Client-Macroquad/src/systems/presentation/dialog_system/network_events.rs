use super::*;

pub fn pump_network_messages_to_ui(ctx: &mut GameContext) {
    // 注意：ctx.events() 会对 ctx 产生不可变借用；因此先收集命令，循环结束后再写 UiState。
    let mut cmds: Vec<UiCommand> = Vec::new();

    // 获取本地玩家的 object_id 用于过滤 buff 等事件
    let local_object_id: Option<u32> = ctx.world.iter()
        .find_map(|e| e.get::<&crate::components::PlayerData>().map(|pd| pd.object_id));

    // 缓存当前时间（用于 buff 过期计算，避免循环内重复 syscall）
    let now_ms = (get_time() * 1000.0) as i64;

    for ev in ctx.events().network_events() {
        match ev {
            NetworkEvent::NpcDialog { npc_id, dialog } => {
                let _ = npc_id;

                // 收到新 NPC 对话时，关闭残留窗口
                cmds.push(UiCommand::CloseNpcRelatedDialogs);
                cmds.push(UiCommand::ShowNpcDialog {
                    dialog: dialog.clone(),
                });
            }
            NetworkEvent::NPCGoods {
                items,
                rate,
                panel_type,
                hide_added_stats,
            } => {
                if matches!(*panel_type, PanelType::Buy | PanelType::Craft) {
                    cmds.push(UiCommand::ShowNpcGoods {
                        items: items.clone(),
                        rate: *rate,
                        panel_type: *panel_type,
                        hide_added_stats: *hide_added_stats,
                        is_sub: false,
                        use_pearls: false,
                    });
                    cmds.push(UiCommand::OpenInventory);
                } else if matches!(*panel_type, PanelType::BuySub) {
                    cmds.push(UiCommand::ShowNpcGoods {
                        items: items.clone(),
                        rate: *rate,
                        panel_type: *panel_type,
                        hide_added_stats: *hide_added_stats,
                        is_sub: true,
                        use_pearls: false,
                    });
                    cmds.push(UiCommand::OpenInventory);
                }
            }
            NetworkEvent::SystemMessage { message } => {
                sys_chat(&mut cmds, message.clone());
            }
            NetworkEvent::ChatMessage { sender, message, chat_type } => {
                use mir2_shared::enums::ChatType;
                let line = format!("{}: {}", sender, message);
                if matches!(chat_type, ChatType::WhisperIn) {
                    cmds.push(UiCommand::PushWhisperLine(line));
                } else {
                    cmds.push(UiCommand::PushChatLine(line));
                }
            }
            NetworkEvent::GroupInvite { inviter } => {
                cmds.push(UiCommand::ShowInviteConfirm {
                    kind: crate::ui::ui_state::InviteKind::Group,
                    inviter: inviter.clone(),
                    detail: format!("{} 邀请你加入队伍", inviter),
                });
            }
            NetworkEvent::GroupMemberAdded { name } => {
                cmds.push(UiCommand::AddGroupMember { name: name.clone() });
                sys_chat(&mut cmds, format!("{} 加入了队伍", name));
            }
            NetworkEvent::GroupMemberRemoved { name } => {
                cmds.push(UiCommand::RemoveGroupMember { name: name.clone() });
                sys_chat(&mut cmds, format!("{} 离开了队伍", name));
            }
            NetworkEvent::GroupDisbanded => {
                cmds.push(UiCommand::ClearGroupMembers);
                sys_chat(&mut cmds, "队伍已解散");
            }
            NetworkEvent::GroupMembersMapUpdated { player_name, player_map } => {
                cmds.push(UiCommand::UpdateGroupMemberMap {
                    player_name: player_name.clone(),
                    player_map: player_map.clone(),
                });
                if !player_map.is_empty() {
                    sys_chat(&mut cmds, format!("{} 在地图: {}", player_name, player_map));
                }
            }
            NetworkEvent::GroupMemberLocationUpdated { name, x, y } => {
                cmds.push(UiCommand::UpdateGroupMemberLocation {
                    player_name: name.clone(),
                    x: *x,
                    y: *y,
                });
            }
            NetworkEvent::GroupModeChanged { allow_group } => {
                let allow = *allow_group == 0;
                cmds.push(UiCommand::SetGroupAllowJoin { allow });
                sys_chat(&mut cmds, format!("组队模式已切换为: {}", if allow { "允许组队" } else { "禁止组队" }));
            }
            NetworkEvent::FriendUpdated { friends } => {
                cmds.push(UiCommand::UpdateFriendList { friends: friends.clone() });
            }
            NetworkEvent::GuildInvite { inviter, guild_name } => {
                cmds.push(UiCommand::ShowInviteConfirm {
                    kind: crate::ui::ui_state::InviteKind::Guild,
                    inviter: inviter.clone(),
                    detail: format!("{} 邀请你加入行会「{}」", inviter, guild_name),
                });
            }
            NetworkEvent::GuildJoined { guild_name, rank_name, level, experience, max_experience, gold, spare_points, member_count, max_members, voting: _, item_count: _, buff_count: _, my_options: _, my_rank_id } => {
                cmds.push(UiCommand::SetGuildName { name: guild_name.clone() });
                cmds.push(UiCommand::UpdateGuildStatus {
                    rank_name: rank_name.clone(),
                    level: *level,
                    experience: *experience,
                    max_experience: *max_experience,
                    gold: *gold,
                    spare_points: *spare_points,
                    member_count: *member_count,
                    max_members: *max_members,
                    my_rank_id: *my_rank_id,
                });
                sys_chat(&mut cmds, format!("你已加入行会「{}」({}) Lv.{} {}/{}人", guild_name, rank_name, level, member_count, max_members));
            }
            NetworkEvent::GuildLeft => {
                cmds.push(UiCommand::SetGuildName { name: String::new() });
                sys_chat(&mut cmds, "你已退出行会");
            }
            NetworkEvent::GuildNoticeUpdated { notice } => {
                sys_chat(&mut cmds, format!("行会公告已更新：{}", notice));
                cmds.push(UiCommand::GuildNoticeUpdated { notice: notice.clone() });
            }
            NetworkEvent::MentorRequested2 { mentor_name } => {
                cmds.push(UiCommand::ShowInviteConfirm {
                    kind: crate::ui::ui_state::InviteKind::Mentor,
                    inviter: mentor_name.clone(),
                    detail: format!("{} 请求收你为徒", mentor_name),
                });
            }
            NetworkEvent::MentorUpdated { mentor_name, mentor_level, mentor_online } => {
                cmds.push(UiCommand::UpdateMentor {
                    name: mentor_name.clone(),
                    level: *mentor_level,
                    online: *mentor_online,
                });
            }
            NetworkEvent::LoverUpdated { lover_name, date } => {
                cmds.push(UiCommand::UpdateLover {
                    name: lover_name.clone(),
                    date: *date,
                });
            }
            NetworkEvent::TradeRequested { requester } => {
                cmds.push(UiCommand::ShowInviteConfirm {
                    kind: crate::ui::ui_state::InviteKind::Trade,
                    inviter: requester.clone(),
                    detail: format!("{} 请求与你交易", requester),
                });
            }
            NetworkEvent::TradeCompleted => {
                cmds.push(UiCommand::TradeCompleted);
                sys_chat(&mut cmds, "交易完成");
            }
            NetworkEvent::MountUpdated { object_id, mount_type, riding_mount } => {
                if local_object_id.is_none_or(|loid| loid == *object_id) {
                    cmds.push(UiCommand::UpdateMountState {
                        mount_type: *mount_type,
                        riding: *riding_mount,
                    });
                }
            }
            NetworkEvent::HeroCreateRequested { can_create_class } => {
                let classes: Vec<&str> = can_create_class.iter().enumerate()
                    .filter(|(_, &c)| c)
                    .map(|(i, _)| match i { 0 => "战士", 1 => "法师", 2 => "道士", _ => "未知" })
                    .collect();
                cmds.push(UiCommand::PushHeroSystemChat(format!("可选职业: {}", classes.join("/"))));
            }
            NetworkEvent::NewHeroCreated { hero_info } => {
                cmds.push(UiCommand::PushHeroSystemChat(format!("新英雄已创建: {}", hero_info)));
            }
            NetworkEvent::HeroInfoReceived { hero_id } => {
                cmds.push(UiCommand::HeroInfoReceived { hero_id: *hero_id });
            }
            NetworkEvent::HeroSpawnStateUpdated { state } => {
                cmds.push(UiCommand::UpdateHeroSpawnState { state: *state });
            }
            NetworkEvent::HeroBehaviourSet { behaviour, pet_mode: _ } => {
                cmds.push(UiCommand::UpdateHeroBehaviour { behaviour: *behaviour });
            }
            NetworkEvent::HeroChanged { success } => {
                cmds.push(UiCommand::HeroChanged);
                if !success {
                    cmds.push(UiCommand::PushHeroSystemChat("切换英雄失败".to_string()));
                }
            }
            NetworkEvent::ExperienceGained { amount } => {
                cmds.push(UiCommand::ExperienceGained { amount: *amount });
            }
            NetworkEvent::LevelUp { new_level } => {
                cmds.push(UiCommand::PlayerLevelUp { new_level: *new_level });
            }
            NetworkEvent::HeroExperienceGained { amount } => {
                cmds.push(UiCommand::HeroExperienceGained { amount: *amount });
            }
            NetworkEvent::HeroLevelUp { new_level } => {
                cmds.push(UiCommand::HeroLevelUp { new_level: *new_level });
                cmds.push(UiCommand::PushHeroSystemChat(format!("英雄升级到 Lv.{}", new_level)));
            }
            NetworkEvent::FishingStatusUpdated { state, success } => {
                if *state == 5 {
                    // Autocast toggle signal
                    cmds.push(UiCommand::SetFishingAutoCast { enabled: *success });
                } else {
                    cmds.push(UiCommand::UpdateFishingState {
                        state: *state,
                        chance: 0.0,
                        progress: 0.0,
                    });
                }
            }
            NetworkEvent::NewIntelligentCreatureReceived { creature_type } => {
                sys_chat(&mut cmds, format!("获得新宠物: type={}", creature_type));
            }
            NetworkEvent::IntelligentCreatureListUpdated { creatures } => {
                let entries: Vec<crate::scenes::dialogs::game::intelligent_creature_dialog::CreatureEntry> = creatures.iter().map(|c| {
                    crate::scenes::dialogs::game::intelligent_creature_dialog::CreatureEntry {
                        name: if c.custom_name.is_empty() { format!("{:?}", c.creature_type) } else { c.custom_name.clone() },
                        creature_type: c.creature_type as u8,
                        fullness: 100,
                        max_fullness: 100,
                        is_summoned: false,
                        pearl_count: 0,
                        deadline_days: 0,
                    }
                }).collect();
                cmds.push(UiCommand::UpdateCreatureList { creatures: entries });
            }
            NetworkEvent::BuffAdded { object_id, buff_id, visible, expire_time, infinite, paused } => {
                if !*visible {
                    // 不可见 buff 不显示图标
                    continue;
                }
                if local_object_id.is_none_or(|loid| loid == *object_id) {
                    let remaining_secs = if *infinite {
                        0.0
                    } else {
                        let expiry_ms = crate::utils::dotnet_ticks_to_unix_ms(*expire_time);
                        ((expiry_ms - now_ms) as f32 / 1000.0).max(0.0)
                    };
                    cmds.push(UiCommand::AddBuff {
                        buff: crate::scenes::dialogs::game::buff_dialog::BuffEntry {
                            buff_type: *buff_id,
                            icon_index: *buff_id,
                            name: format!("Buff #{}", buff_id),
                            remaining_secs,
                            is_paused: *paused,
                            caster: String::new(),
                        },
                    });
                }
            }
            NetworkEvent::BuffRemoved { object_id, buff_id } => {
                if local_object_id.is_none_or(|loid| loid == *object_id) {
                    cmds.push(UiCommand::RemoveBuff { buff_type: *buff_id });
                }
            }
            NetworkEvent::HeroAutoPotUnlocked { unlocked } => {
                if *unlocked {
                    cmds.push(UiCommand::SetHeroAutoPotUnlocked);
                }
            }
            NetworkEvent::HeroAutoPotSet { pot_type, value } => {
                cmds.push(UiCommand::SetHeroAutoPotValue { pot_type: *pot_type, value: *value });
            }
            NetworkEvent::HeroAutoPotItemSet { slot, item_id } => {
                cmds.push(UiCommand::SetHeroAutoPotItem { slot: *slot, item_id: *item_id });
            }
            NetworkEvent::HeroManageReceived { heroes } => {
                let entries: Vec<_> = heroes.iter().map(|h| {
                    crate::scenes::dialogs::game::hero_dialog::ManageHeroEntry {
                        index: h.index,
                        name: h.name.clone(),
                        level: h.level,
                        class: h.class as u8,
                        gender: h.gender as u8,
                    }
                }).collect();
                cmds.push(UiCommand::UpdateHeroManageList { heroes: entries });
            }
            NetworkEvent::HeroBaseStatsReceived { stats } => {
                cmds.push(UiCommand::SetHeroBaseStats { stats: stats.clone() });
            }
            NetworkEvent::NewHeroInfoReceived { info } => {
                cmds.push(UiCommand::PushHeroSystemChat(format!("英雄信息: {}", info)));
            }
            NetworkEvent::IntelligentCreatureRenameEnabled { can_rename } => {
                cmds.push(UiCommand::SetCreatureCanRename { can_rename: *can_rename });
            }
            NetworkEvent::IntelligentCreaturePickupReceived { enabled } => {
                cmds.push(UiCommand::SetCreatureAutoPickup { enabled: *enabled });
            }
            NetworkEvent::BuffPaused { object_id, buff_id, paused } => {
                if local_object_id.is_none_or(|loid| loid == *object_id) {
                    cmds.push(UiCommand::SetBuffPaused { buff_id: *buff_id, paused: *paused });
                }
            }
            NetworkEvent::CompassUpdated { location } => {
                cmds.push(UiCommand::UpdateCompass { location: *location });
            }
            // 交易进度事件
            NetworkEvent::TradeStarted { partner } => {
                cmds.push(UiCommand::OpenTradeDialog { partner: partner.clone() });
            }
            NetworkEvent::TradeGoldAdded { amount } => {
                cmds.push(UiCommand::TradeGoldAdded { amount: *amount });
            }
            NetworkEvent::TradeItemAdded { items } => {
                cmds.push(UiCommand::TradeItemAdded { items: items.clone() });
            }
            NetworkEvent::TradeItemDeposited { from_slot, success } => {
                cmds.push(UiCommand::TradeItemDeposited { from_slot: *from_slot, success: *success });
            }
            NetworkEvent::TradeItemRetrieved { from_slot, success } => {
                cmds.push(UiCommand::TradeItemRetrieved { from_slot: *from_slot, success: *success });
            }
            NetworkEvent::TradeConfirmedEvent { locked } => {
                cmds.push(UiCommand::TradeConfirmed { locked: *locked });
            }
            NetworkEvent::TradeCancelledEvent { unlock } => {
                cmds.push(UiCommand::TradeCancelled { unlock: *unlock });
            }
            // 邮件事件
            NetworkEvent::MailReceived { mails } => {
                let entries: Vec<_> = mails.iter().map(|m| crate::ui::ui_state::MailEntry {
                    mail_id: m.mail_id,
                    sender: m.sender_name.clone(),
                    subject: m.message.lines().next().unwrap_or("(无主题)").to_string(),
                    body: m.message.clone(),
                    date: format_mail_date(m.send_date),
                    has_parcel: !m.items.is_empty(),
                    is_read: m.collected,
                }).collect();
                cmds.push(UiCommand::UpdateMailList { mails: entries });
            }
            // 任务事件
            NetworkEvent::QuestAccepted { quest_id } => {
                cmds.push(UiCommand::QuestAccepted {
                    quest_id: *quest_id,
                    name: format!("任务 #{}", quest_id),
                    description: "新任务".to_string(),
                });
            }
            NetworkEvent::QuestCompleted { quest_id } => {
                cmds.push(UiCommand::QuestCompleted { quest_id: *quest_id });
            }
            NetworkEvent::QuestShared { quest_id } => {
                sys_chat(&mut cmds, format!("任务 #{} 已分享", quest_id));
            }
            NetworkEvent::QuestProgressUpdated { quest_id, progress } => {
                cmds.push(UiCommand::QuestProgressUpdated {
                    quest_id: *quest_id,
                    progress_text: progress.clone(),
                });
            }
            NetworkEvent::QuestInfoReceived {
                quest_id, name, group, description, level_req, reward_exp, reward_gold,
            } => {
                cmds.push(UiCommand::QuestInfoReceived {
                    quest_id: *quest_id,
                    name: name.clone(),
                    group: group.clone(),
                    description: description.clone(),
                    level_req: *level_req,
                    reward_exp: *reward_exp,
                    reward_gold: *reward_gold,
                });
            }
            // 公会扩展事件
            NetworkEvent::GuildMemberUpdated { name, rank, online } => {
                cmds.push(UiCommand::GuildMemberUpdated {
                    name: name.clone(),
                    rank: format!("Rank {}", rank),
                    online: *online,
                });
            }
            NetworkEvent::GuildExpGained { amount } => {
                cmds.push(UiCommand::GuildExpGained { amount: *amount });
            }
            NetworkEvent::GuildWarRequested { guild_name } => {
                cmds.push(UiCommand::GuildWarRequested { guild_name: guild_name.clone() });
            }
            // NPC 操作确认提示
            NetworkEvent::NPCSellReceived => {
                cmds.push(UiCommand::HideNpcGoods);
                sys_chat(&mut cmds, "物品出售成功");
            }
            NetworkEvent::NPCRepairReceived { rate } => {
                sys_chat(&mut cmds, format!("修理完成 (费率={:.1})", rate));
            }
            NetworkEvent::NPCSRepairReceived { rate } => {
                sys_chat(&mut cmds, format!("特殊修理完成 (费率={:.1})", rate));
            }
            NetworkEvent::NPCRefineReceived { rate, refining } => {
                let state = if *refining { "精炼中" } else { "待精炼" };
                sys_chat(&mut cmds, format!("精炼操作完成 (费率={:.1}, {})", rate, state));
            }
            NetworkEvent::NPCCheckRefineReceived => {
                sys_chat(&mut cmds, "精炼状态已确认");
            }
            NetworkEvent::NPCCollectRefineReceived { success } => {
                cmds.push(UiCommand::CloseNpcRelatedDialogs);
                if *success {
                    sys_chat(&mut cmds, "精炼物品已提取");
                } else {
                    sys_chat(&mut cmds, "精炼物品提取失败");
                }
            }
            NetworkEvent::NPCReplaceWedRingReceived { rate } => {
                cmds.push(UiCommand::CloseNpcRelatedDialogs);
                sys_chat(&mut cmds, format!("结婚戒指更换完成 (费率={:.1})", rate));
            }
            NetworkEvent::NPCStorageReceived => {
                cmds.push(UiCommand::OpenStorage);
            }
            NetworkEvent::NPCConsignReceived => {
                cmds.push(UiCommand::OpenTrustMerchant);
            }
            NetworkEvent::NPCConsignEvent => {
                cmds.push(UiCommand::OpenTrustMerchant);
            }
            NetworkEvent::NPCMarketEvent2 { pages } => {
                let total = pages.len().max(1) as i32;
                cmds.push(UiCommand::UpdateMerchantItems { items: vec![], page: 1, total });
            }
            NetworkEvent::NPCMarketPageEvent2 { listings } => {
                let now_secs = get_time() as i64;
                let seven_days_secs: i64 = 7 * 24 * 3600;
                let items: Vec<MerchantItem> = listings.iter().map(|l| {
                    let remaining_hours = ((l.consignment_date + seven_days_secs - now_secs) / 3600).max(0) as u32;
                    MerchantItem {
                        item: l.item.clone(),
                        price: l.price,
                        seller: l.seller_name.clone(),
                        remaining_hours,
                    }
                }).collect();
                cmds.push(UiCommand::UpdateMerchantItems { items, page: 1, total: 1 });
            }
            NetworkEvent::ConsignItemEvent { success, .. } => {
                let msg = if *success { "寄售成功" } else { "寄售失败" };
                sys_chat(&mut cmds, msg);
            }
            NetworkEvent::MarketFailedEvent2 { reason } => {
                sys_chat(&mut cmds, format!("市场操作失败: {}", reason));
            }
            NetworkEvent::MarketSuccessEvent2 { message } => {
                sys_chat(&mut cmds, message.clone());
            }
            NetworkEvent::NPCAwakeningReceived => {
                // 觉醒对话框由 AwakeningNeedMaterialsReceived 触发
            }
            NetworkEvent::NPCDisassembleReceived => {
                cmds.push(UiCommand::CloseNpcRelatedDialogs);
                sys_chat(&mut cmds, "分解操作完成");
            }
            NetworkEvent::NPCDowngradeReceived => {
                cmds.push(UiCommand::CloseNpcRelatedDialogs);
                sys_chat(&mut cmds, "降级操作完成");
            }
            NetworkEvent::NPCResetReceived => {
                cmds.push(UiCommand::CloseNpcRelatedDialogs);
                sys_chat(&mut cmds, "重置操作完成");
            }
            NetworkEvent::NPCPearlGoodsReceived { rate, item_list } => {
                let pearl_items: Vec<mir2_shared::data::item::UserItem> = item_list.iter()
                    .map(|&item_id| mir2_shared::data::item::UserItem {
                        item_index: item_id,
                        ..Default::default()
                    })
                    .collect();
                cmds.push(UiCommand::ShowNpcGoods {
                    items: pearl_items,
                    rate: *rate as f32,
                    panel_type: PanelType::Buy,
                    hide_added_stats: false,
                    is_sub: false,
                    use_pearls: true,
                });
                cmds.push(UiCommand::OpenInventory);
            }
            NetworkEvent::NPCRequestInputReceived { npc_id, prompt, max_length } => {
                cmds.push(UiCommand::ShowTextInput {
                    kind: crate::scenes::dialogs::game::main_dialog::TextInputKind::NPCInput { npc_id: *npc_id },
                    title: "NPC 输入".to_string(),
                    placeholder: prompt.clone(),
                    max_length: *max_length as usize,
                });
            }
            // 公会扩展事件
            NetworkEvent::GuildStorageGoldChanged { delta, total } => {
                cmds.push(UiCommand::UpdateGuildStorageGold { gold: *total });
                sys_chat(&mut cmds, format!("行会资金 {} (总计: {})", if *delta >= 0 { format!("+{}", delta) } else { format!("{}", delta) }, total));
            }
            NetworkEvent::GuildStorageItemChanged { change_type, slot } => {
                let action = if *change_type == 0 { "存入" } else { "取出" };
                sys_chat(&mut cmds, format!("行会仓库物品{}: 槽位{}", action, slot));
                // 服务器当前不发送 item details(只 slot + change_type)。
                // UI 端的 `UpdateGuildStorageItem` 需要 slot+name+quantity,
                // 但服务器不提供 name/quantity。workaround 是:
                //   1) 玩家需重新打开行会仓库触发 GuildStorageListReceived
                //   2) 一旦服务器协议扩展(在 GuildStorageItemChanged 中携带 item 数据),
                //      这里可改为直接调用 `cmds.push(UiCommand::UpdateGuildStorageItem { slot, ... })`。
                // 当前只记录日志,等用户重新打开仓库时再刷新 UI。
                tracing::debug!("🔄 行会仓库变更: 等用户重开仓库刷新 (slot={}, type={})", slot, change_type);
            }
            NetworkEvent::GuildStorageListReceived { items } => {
                cmds.push(UiCommand::ClearGuildStorageItems);
                for (slot, item_opt) in items.iter().enumerate() {
                    if let Some(item) = item_opt {
                        let name = item.item.info.as_ref()
                            .map(|i| i.friendly_name())
                            .unwrap_or_else(|| format!("Item#{}", item.item.item_index));
                        cmds.push(UiCommand::UpdateGuildStorageItem {
                            slot: slot as i32,
                            name,
                            quantity: item.item.count as i32,
                        });
                    }
                }
            }
            NetworkEvent::GuildTerritoryPageReceived { territories } => {
                cmds.push(UiCommand::ShowGuildTerritory);
                if !territories.is_empty() {
                    sys_chat(&mut cmds, format!("领地列表: {} 条记录", territories.len()));
                }
            }
            NetworkEvent::GuildTerritoryPurchased { success } => {
                cmds.push(UiCommand::ShowGuildTerritory);
                if *success {
                    sys_chat(&mut cmds, "行会领地购买成功");
                } else {
                    sys_chat(&mut cmds, "行会领地购买失败");
                }
            }
            NetworkEvent::GuildBuffListReceived { buff_ids } => {
                cmds.push(UiCommand::UpdateGuildBuffs { buff_ids: buff_ids.clone() });
            }
            // NPC 市场/寄售事件
            NetworkEvent::SellItemReceived { unique_id, count, success } => {
                if *success {
                    cmds.push(UiCommand::HideNpcGoods);
                    sys_chat(&mut cmds, format!("物品出售成功 (id={}, 数量={})", unique_id, count));
                } else {
                    sys_chat(&mut cmds, format!("物品出售失败 (id={})", unique_id));
                }
            }
            NetworkEvent::CraftItemReceived { unique_id, count, success } => {
                if *success {
                    cmds.push(UiCommand::CloseNpcRelatedDialogs);
                    sys_chat(&mut cmds, format!("合成/制作完成 (id={}, 数量={})", unique_id, count));
                } else {
                    sys_chat(&mut cmds, format!("合成/制作失败 (id={})", unique_id));
                }
            }
            NetworkEvent::RepairItemReceived { unique_id } => {
                sys_chat(&mut cmds, format!("修理请求已发送 (id={})", unique_id));
            }
            NetworkEvent::ItemRepairedEvent { unique_id, max_dura, current_dura } => {
                cmds.push(UiCommand::ItemDuraChanged { unique_id: *unique_id, current_dura: *current_dura as i32 });
                sys_chat(&mut cmds, format!("物品修理完成 (id={}, 耐久={}/{})", unique_id, current_dura, max_dura));
            }
            NetworkEvent::DefaultNPCReceived { message, .. } => {
                sys_chat(&mut cmds, message.clone());
            }
            NetworkEvent::AwakeningNeedMaterialsReceived { item_id, materials } => {
                // 显示觉醒对话框（材料由服务器推送）
                let mat_entries: Vec<crate::scenes::dialogs::game::npc_awake_dialog::AwakeningMaterial> =
                    materials.iter()
                        .map(|(id, count)| crate::scenes::dialogs::game::npc_awake_dialog::AwakeningMaterial {
                            name: format!("物品 #{}", id),
                            required: *count as u32,
                            have: 0,
                        })
                        .collect();
                cmds.push(UiCommand::ShowNPCAwake {
                    item_name: format!("物品 #{}", item_id),
                    materials: mat_entries,
                });
            }
            NetworkEvent::AwakeningLockedItemReceived { unique_id: _, locked } => {
                cmds.push(UiCommand::SetAwakeLocked { locked: *locked });
            }
            NetworkEvent::AwakeningReceived { result, remove_id: _ } => {
                let msg = match result {
                    1 => "觉醒成功！",
                    0 => "觉醒失败，物品已损坏",
                    -1 => "觉醒失败",
                    -2 => "已达最高觉醒等级",
                    -3 => "金币不足",
                    -4 => "材料不足",
                    _ => "觉醒结果未知",
                };
                sys_chat(&mut cmds, msg);
            }
            NetworkEvent::MailLockedItemReceived { unique_id, locked } => {
                let state = if *locked { "已锁定" } else { "已解锁" };
                sys_chat(&mut cmds, format!("邮件附件{} (unique_id={})", state, unique_id));
            }
            NetworkEvent::MailSendRequestReceived { mail_id } => {
                sys_chat(&mut cmds, format!("请输入收件人和邮件内容 (mail_id={})", mail_id));
            }
            NetworkEvent::MailSentEvent { result } => {
                cmds.push(UiCommand::CloseMailDialog);
                let msg = if *result >= 0 { "邮件已发送" } else { "邮件发送失败" };
                sys_chat(&mut cmds, msg);
            }
            NetworkEvent::MailCostReceived { cost } => {
                sys_chat(&mut cmds, format!("邮件费用: {} 金币", cost));
            }
            NetworkEvent::ParcelCollectedEvent { success } => {
                if *success {
                    sys_chat(&mut cmds, "包裹物品已收集");
                } else {
                    sys_chat(&mut cmds, "包裹物品收集失败");
                }
            }
            // 物品租赁事件
            NetworkEvent::ItemRentalRequested => {
                cmds.push(UiCommand::OpenItemRental { partner: "未知".to_string() });
            }
            NetworkEvent::ItemRentalFeeReceived { fee } => {
                cmds.push(UiCommand::UpdateRentalFee { fee: *fee });
            }
            NetworkEvent::ItemRentalPeriodReceived { period } => {
                cmds.push(UiCommand::UpdateRentalPeriod { period: *period });
            }
            NetworkEvent::RentalItemDeposited { unique_id, success } => {
                if *success {
                    sys_chat(&mut cmds, format!("租赁物品已存入 (unique_id={})", unique_id));
                } else {
                    sys_chat(&mut cmds, "租赁物品存入失败");
                }
            }
            NetworkEvent::RentalItemRetrieved { unique_id, success } => {
                if *success {
                    cmds.push(UiCommand::CloseItemRental);
                    sys_chat(&mut cmds, format!("租赁物品已取回 (unique_id={})", unique_id));
                } else {
                    sys_chat(&mut cmds, "租赁物品取回失败");
                }
            }
            NetworkEvent::RentalItemUpdated { fee, period } => {
                sys_chat(&mut cmds, format!("租赁物品已更新 (费用={}, 周期={})", fee, period));
            }
            NetworkEvent::RentalItemsReceived { items } => {
                cmds.push(UiCommand::UpdateRentalItemList { items: items.clone() });
                sys_chat(&mut cmds, format!("收到{}件租赁物品", items.len()));
            }
            NetworkEvent::ItemRentalCancelled { success } => {
                if *success {
                    cmds.push(UiCommand::CloseItemRental);
                    sys_chat(&mut cmds, "租赁已取消");
                } else {
                    sys_chat(&mut cmds, "取消租赁失败");
                }
            }
            NetworkEvent::ItemRentalLocked { locked } => {
                cmds.push(UiCommand::SetRentalLocked { locked: *locked });
            }
            NetworkEvent::ItemRentalPartnerLocked { locked } => {
                cmds.push(UiCommand::SetRentalPartnerLocked { locked: *locked });
            }
            NetworkEvent::ItemRentalConfirmed { success } => {
                if *success {
                    cmds.push(UiCommand::CloseItemRental);
                    sys_chat(&mut cmds, "租赁交易已完成");
                } else {
                    sys_chat(&mut cmds, "租赁确认失败");
                }
            }
            // 游戏商店
            NetworkEvent::GameShopInfoReceived { items, credit, gold } => {
                cmds.push(UiCommand::UpdateGameShopItems {
                    items: items.clone(),
                    credit: *credit,
                    gold: *gold,
                });
            }
            NetworkEvent::GameShopStockReceived { item_index, stock } => {
                cmds.push(UiCommand::UpdateGameShopStock {
                    item_index: *item_index,
                    stock: *stock,
                });
            }
            NetworkEvent::AttackModeChanged { mode } => {
                cmds.push(UiCommand::UpdateAttackMode { mode: *mode });
            }
            NetworkEvent::PetModeChanged { mode } => {
                cmds.push(UiCommand::UpdatePetMode { mode: *mode });
            }
            NetworkEvent::TimerSet { timer_id, seconds } => {
                cmds.push(UiCommand::SetTimer {
                    timer_id: *timer_id,
                    seconds: *seconds,
                });
            }
            NetworkEvent::TimerExpired { timer_id } => {
                cmds.push(UiCommand::TimerExpired { timer_id: *timer_id });
            }
            // 排行榜
            NetworkEvent::RankingsReceived { rankings } => {
                let entries: Vec<_> = rankings.iter().map(|r| {
                    (r.rank as u32, r.player_name.clone(), format!("Lv.{}", r.level))
                }).collect();
                cmds.push(UiCommand::UpdateRankings { tab: 0, entries });
            }
            NetworkEvent::RankingsReceivedWithEntries { tab, entries } => {
                let entries_clone: Vec<_> = entries.clone();
                cmds.push(UiCommand::UpdateRankings {
                    tab: *tab,
                    entries: entries_clone,
                });
            }
            // 其他重要事件
            NetworkEvent::RollReceivedEvent { value } => {
                cmds.push(UiCommand::ShowRollResult { value: *value });
            }
            NetworkEvent::PlayerRevived => {
                cmds.push(UiCommand::PushChatNotice { text: "你已复活".to_string() });
            }
            NetworkEvent::PlayerPoisoned { object_id: _, poison_type } => {
                cmds.push(UiCommand::PushChatNotice { text: format!("你中毒了！(类型={})", poison_type) });
            }
            NetworkEvent::ObjectPoisonedEvent { object_id: _, poison_type } => {
                cmds.push(UiCommand::PushChatNotice { text: format!("目标中毒(类型={})", poison_type) });
            }
            NetworkEvent::OutputMessageReceived { message, message_type: _ } => {
                sys_chat(&mut cmds, message.clone());
            }
            NetworkEvent::QuestItemGained { item_id } => {
                sys_chat(&mut cmds, format!("获得任务物品 (ID={})", item_id));
            }
            NetworkEvent::MapEffectReceived { effect, location_x: _, location_y: _, value: _ } => {
                cmds.push(UiCommand::TriggerMapEffect { effect: *effect });
            }
            NetworkEvent::TimeOfDayChanged { time_of_day } => {
                cmds.push(UiCommand::SetTimeOfDay { time: *time_of_day });
                let desc = match *time_of_day {
                    0 => "白天",
                    1 => "黄昏",
                    2 => "夜晚",
                    3 => "凌晨",
                    _ => "未知",
                };
                sys_chat(&mut cmds, format!("时间变化: {}", desc));
            }
            NetworkEvent::ObserveAllowed { allowed } => {
                cmds.push(UiCommand::SetObserveAllowed { allowed: *allowed });
            }
            NetworkEvent::TransformUpdated { form } => {
                cmds.push(UiCommand::SetTransformForm { form: *form });
            }
            NetworkEvent::BaseStatsReceived { stats } => {
                cmds.push(UiCommand::SetBaseStats { stats: stats.clone() });
            }
            NetworkEvent::NewMapInfoReceived { packet } => {
                cmds.push(UiCommand::UpdateBigMapInfo {
                    map_index: packet.map_index,
                    title: packet.title.clone(),
                    width: packet.width,
                    height: packet.height,
                });
            }
            NetworkEvent::WorldMapSetupReceived { icons } => {
                cmds.push(UiCommand::UpdateWorldMapIcons { icons: icons.clone() });
            }
            NetworkEvent::SearchMapResultReceived { map_index, location_x, location_y } => {
                cmds.push(UiCommand::NavigateToMapLocation { map_index: *map_index, x: *location_x, y: *location_y });
            }
            // 婚姻/师徒补充
            NetworkEvent::MarriageRequested2 { requester } => {
                cmds.push(UiCommand::SetMarriageRequester { requester: requester.clone() });
            }
            NetworkEvent::DivorceRequested2 { lover_name } => {
                cmds.push(UiCommand::ShowInviteConfirm {
                    kind: crate::ui::ui_state::InviteKind::Divorce,
                    inviter: lover_name.clone(),
                    detail: format!("{} 请求与你离婚", lover_name),
                });
            }
            NetworkEvent::DoorOpened { door_id, close: _ } => {
                cmds.push(UiCommand::OpenDoor { door_id: *door_id });
            }
            NetworkEvent::BrowserOpened { url } => {
                sys_chat(&mut cmds, format!("浏览器已打开: {}", url));
            }
            NetworkEvent::BindingShotSet { enabled } => {
                cmds.push(UiCommand::SetBindingShot { enabled: *enabled });
            }
            NetworkEvent::ConcentrationSet { object_id: _, enabled, interrupted: _ } => {
                cmds.push(UiCommand::SetConcentration { enabled: *enabled });
            }
            NetworkEvent::ElementalSet { object_id: _, enabled: _, value: _, element, expire_time: _ } => {
                cmds.push(UiCommand::SetElement { element: *element });
            }
            NetworkEvent::DuraChanged { unique_id, durability } => {
                cmds.push(UiCommand::ItemDuraChanged { unique_id: *unique_id, current_dura: *durability });
            }
            NetworkEvent::DelayedExplosionRemoved { object_id } => {
                sys_chat(&mut cmds, format!("延迟爆炸已移除 (object_id={})", object_id));
            }
            NetworkEvent::ChatItemStatsReceived { stats, .. } => {
                sys_chat(&mut cmds, stats.clone());
            }
            NetworkEvent::InventoryResized { new_size } => {
                cmds.push(UiCommand::SetInventorySize { size: *new_size });
            }
            NetworkEvent::StorageResized { new_size } => {
                cmds.push(UiCommand::SetStorageSize { size: *new_size });
            }
            NetworkEvent::UserStorageReceived { items } => {
                cmds.push(UiCommand::UpdateStorageItems { items: items.clone() });
            }
            NetworkEvent::GuildNameReceived { .. } => {
                cmds.push(UiCommand::ShowTextInput {
                    kind: crate::scenes::dialogs::game::main_dialog::TextInputKind::GuildName,
                    title: "请输入公会名称".to_string(),
                    placeholder: "3~20个字符".to_string(),
                    max_length: 20,
                });
            }
            NetworkEvent::ChangePasswordSuccess => {
                sys_chat(&mut cmds, "密码修改成功");
            }
            NetworkEvent::ChangePasswordFailed { reason } => {
                sys_chat(&mut cmds, format!("密码修改失败: {}", reason));
            }
            NetworkEvent::ReincarnationRequested => {
                sys_chat(&mut cmds, "转生请求已收到");
            }
            NetworkEvent::ReincarnationCancelled => {
                sys_chat(&mut cmds, "转生已取消");
            }
            NetworkEvent::HeroHealthChanged { hp, mp } => {
                cmds.push(UiCommand::UpdateHeroHealth { hp: *hp, mp: *mp });
            }
            NetworkEvent::LogOutSuccess { characters } => {
                let count = characters.len();
                cmds.push(UiCommand::RequestSceneTransition {
                    target: crate::scenes::SceneTransition::CharacterSelect,
                });
                tracing::info!("🚪 LogOutSuccess → CharacterSelect with {} characters", count);
            }
            NetworkEvent::LogOutFailed => {
                sys_chat(&mut cmds, "退出游戏失败，请稍后再试");
            }
            NetworkEvent::ReturnToLogin => {
                cmds.push(UiCommand::RequestSceneTransition {
                    target: crate::scenes::SceneTransition::Login,
                });
            }
            NetworkEvent::RefineItemDeposited { from, to, success } => {
                if *success {
                    sys_chat(&mut cmds, format!("精炼物品已存入 ({}→{})", from, to));
                } else {
                    sys_chat(&mut cmds, "精炼物品存入失败");
                }
            }
            NetworkEvent::RefineItemRetrieved { from, to, success } => {
                if *success {
                    cmds.push(UiCommand::CloseNpcRelatedDialogs);
                    sys_chat(&mut cmds, format!("精炼物品已取回 ({}→{})", from, to));
                } else {
                    sys_chat(&mut cmds, "精炼物品取回失败");
                }
            }
            NetworkEvent::RefineCancelled { unlock } => {
                cmds.push(UiCommand::CloseNpcRelatedDialogs);
                sys_chat(&mut cmds, if *unlock { "精炼已取消，物品已解锁" } else { "精炼已取消" }.to_string());
            }
            NetworkEvent::RefineItemCompleted { unique_id } => {
                cmds.push(UiCommand::CloseNpcRelatedDialogs);
                sys_chat(&mut cmds, format!("精炼完成！ (unique_id={})", unique_id));
            }
            NetworkEvent::NoticeUpdated { notice } => {
                cmds.push(UiCommand::ShowNotice { text: notice.clone() });
            }
            // 魔法/技能事件
            NetworkEvent::MagicLearned { magic, hero } => {
                cmds.push(UiCommand::MagicLearned {
                    spell: magic.spell as u8,
                    name: magic.name.clone(),
                    level: magic.level,
                    icon: magic.icon,
                    hero: *hero,
                });
                sys_chat(&mut cmds, format!("学会技能：{} Lv.{}", magic.name, magic.level));
            }
            NetworkEvent::MagicLeveledUp { spell, level, hero } => {
                cmds.push(UiCommand::MagicLeveledUp { spell: *spell as u8, level: *level, hero: *hero });
                sys_chat(&mut cmds, format!("技能升级：{:?} Lv.{}", spell, level));
            }
            NetworkEvent::MagicRemoved { spell, hero } => {
                cmds.push(UiCommand::MagicRemoved { spell: *spell as u8, hero: *hero });
                sys_chat(&mut cmds, format!("技能遗忘：{:?}", spell));
            }
            NetworkEvent::SpellToggled { spell, can_use, hero } => {
                cmds.push(UiCommand::SpellToggled { spell: *spell as u8, can_use: *can_use, hero: *hero });
                let state = if *can_use { "启用" } else { "禁用" };
                sys_chat(&mut cmds, format!("技能{:?}{}", spell, state));
            }
            // 租赁确认
            NetworkEvent::ItemRentalConfirmable { can_confirm } => {
                if *can_confirm {
                    sys_chat(&mut cmds, "租赁交易可确认，双方均已锁定");
                } else {
                    sys_chat(&mut cmds, "租赁交易尚不可确认");
                }
            }
            NetworkEvent::ItemRemoved { unique_id, .. }
            | NetworkEvent::ItemLost { unique_id, .. } => {
                cmds.push(UiCommand::RemoveDuraEntry { unique_id: *unique_id });
            }
            NetworkEvent::ItemGained { item } => {
                let name = item.info.as_ref().map(|i| i.name.as_str()).unwrap_or("未知物品");
                sys_chat(&mut cmds, format!("获得物品: {} x{} (UID={})", name, item.count, item.unique_id));
            }
            NetworkEvent::ItemDropped { unique_id, count, success } => {
                cmds.push(UiCommand::RemoveDuraEntry { unique_id: *unique_id });
                if !success {
                    sys_chat(&mut cmds, format!("丢弃物品失败 (UID={})", unique_id));
                } else {
                    sys_chat(&mut cmds, format!("丢弃物品 x{} (UID={})", count, unique_id));
                }
            }
            NetworkEvent::NewRecipeInfoReceived { recipe_id } => {
                sys_chat(&mut cmds, format!("获得新配方: ID={}", recipe_id));
            }
            NetworkEvent::NPCUpdated { npc_id } => {
                sys_chat(&mut cmds, format!("NPC 更新: ID={}", npc_id));
            }
            NetworkEvent::ItemSealed { unique_id, expiry_date } => {
                sys_chat(&mut cmds, format!("物品封印: UID={} 到期={}", unique_id, expiry_date));
            }
            NetworkEvent::MagicCastEvent { spell } => {
                sys_chat(&mut cmds, format!("施法: {:?}", spell));
            }
            NetworkEvent::MagicListReceived { spell, target_id, target_x, target_y, cast, level } => {
                sys_chat(&mut cmds, format!(
                    "法术: {:?} 目标={} 位置=({},{}) 施法={} 等级={}",
                    spell, target_id, target_x, target_y, cast, level
                ));
            }
            NetworkEvent::ItemUsed { unique_id: _ } => {
                sys_chat(&mut cmds, "使用了物品");
            }
            NetworkEvent::QuestListUpdated => {
                sys_chat(&mut cmds, "任务列表已更新");
            }
            NetworkEvent::GoldChanged { delta } => {
                let msg = if *delta >= 0 {
                    format!("获得 {} 金币", delta)
                } else {
                    format!("失去 {} 金币", -delta)
                };
                sys_chat(&mut cmds, msg);
            }
            NetworkEvent::CreditChanged { delta } => {
                let msg = if *delta >= 0 {
                    format!("获得 {} 元宝", delta)
                } else {
                    format!("消耗 {} 元宝", -delta)
                };
                sys_chat(&mut cmds, msg);
            }
            NetworkEvent::ItemSlotEquipped { slot, unique_id, success, .. } => {
                let msg = if *success {
                    format!("装备成功: 槽位={} UID={}", slot, unique_id)
                } else {
                    format!("装备失败: 槽位={} UID={}", slot, unique_id)
                };
                sys_chat(&mut cmds, msg);
            }
            NetworkEvent::ItemCombined { id_from, id_to, success, destroy, .. } => {
                let msg = if *success {
                    if *destroy {
                        format!("合成成功: {} → {} (副材料已销毁)", id_from, id_to)
                    } else {
                        format!("合成成功: {} → {}", id_from, id_to)
                    }
                } else {
                    format!("合成失败: {} → {}", id_from, id_to)
                };
                sys_chat(&mut cmds, msg);
            }
            NetworkEvent::ObjectHeroSpawned { packet } => {
                sys_chat(&mut cmds, format!("{} 的英雄已召唤", packet.owner_name));
            }
            NetworkEvent::ObjectLeveled { object_id, level } => {
                sys_chat(&mut cmds, format!("玩家 {} 升级到 Lv.{}", object_id, level));
            }
            NetworkEvent::RangeAttacked { target_id, target_x, target_y, spell, spell_level } => {
                sys_chat(&mut cmds, format!(
                    "远程攻击: 目标={} 位置=({},{}) 法术={} 等级={}",
                    target_id, target_x, target_y, spell, spell_level
                ));
            }
            NetworkEvent::HeroItemTakenBack { from, to, success } => {
                let msg = if *success {
                    format!("英雄物品取回成功: {} → {}", from, to)
                } else {
                    format!("英雄物品取回失败: {} → {}", from, to)
                };
                sys_chat(&mut cmds, msg);
            }
            NetworkEvent::HeroItemTransferred { from, to, success } => {
                let msg = if *success {
                    format!("英雄物品转移成功: {} → {}", from, to)
                } else {
                    format!("英雄物品转移失败: {} → {}", from, to)
                };
                sys_chat(&mut cmds, msg);
            }
            NetworkEvent::NewItemInfoReceived { item_index, item_name } => {
                sys_chat(&mut cmds, format!("新物品信息: {} (ID={})", item_name, item_index));
            }
            NetworkEvent::NewChatItemReceived { item_id } => {
                sys_chat(&mut cmds, format!("聊天物品: ID={}", item_id));
            }
            NetworkEvent::ItemEquipped { unique_id, slot, success, .. } => {
                let msg = if *success {
                    format!("装备成功: UID={} 槽位={}", unique_id, slot)
                } else {
                    format!("装备失败: UID={} 槽位={}", unique_id, slot)
                };
                sys_chat(&mut cmds, msg);
            }
            NetworkEvent::ItemMerged { id_from, id_to, success, .. } => {
                let msg = if *success {
                    format!("物品合并成功: {} → {}", id_from, id_to)
                } else {
                    format!("物品合并失败: {} → {}", id_from, id_to)
                };
                sys_chat(&mut cmds, msg);
            }
            NetworkEvent::ItemStored { from, to, success } => {
                let msg = if *success {
                    format!("物品存入仓库成功: {} → {}", from, to)
                } else {
                    format!("物品存入仓库失败: {} → {}", from, to)
                };
                sys_chat(&mut cmds, msg);
            }
            NetworkEvent::ItemTakenBack { from, to, success } => {
                let msg = if *success {
                    format!("物品取回成功: {} → {}", from, to)
                } else {
                    format!("物品取回失败: {} → {}", from, to)
                };
                sys_chat(&mut cmds, msg);
            }
            NetworkEvent::QuestItemLost { unique_id } => {
                sys_chat(&mut cmds, format!("任务物品丢失: UID={}", unique_id));
            }
            NetworkEvent::ItemUpgraded { item } => {
                let name = item.info.as_ref().map(|i| i.name.as_str()).unwrap_or("未知物品");
                sys_chat(&mut cmds, format!("物品升级成功: {} (UID={})", name, item.unique_id));
            }
            NetworkEvent::ItemSplit { unique_id, count, .. } => {
                sys_chat(&mut cmds, format!("物品拆分: UID={} 数量={}", unique_id, count));
            }
            NetworkEvent::ItemSlotRemoved { slot, unique_id, success, .. } => {
                let msg = if *success {
                    format!("槽位移除成功: 槽位={} UID={}", slot, unique_id)
                } else {
                    format!("槽位移除失败: 槽位={} UID={}", slot, unique_id)
                };
                sys_chat(&mut cmds, msg);
            }
            NetworkEvent::ObjectHarvested { object_id, .. } => {
                sys_chat(&mut cmds, format!("采集完成: ObjectID={}", object_id));
            }
            NetworkEvent::PlayerDied { x, y, .. } => {
                sys_chat(&mut cmds, format!("你已死亡！位置: ({}, {})", x, y));
            }
            NetworkEvent::PlayerStruck { attacker_id, damage } => {
                if *damage > 0 {
                    sys_chat(&mut cmds, format!("受到 {} 点伤害 (来自 ObjectID={})", damage, attacker_id));
                }
            }
            _ => {}
        }
    }

    if !cmds.is_empty() {
        let _ = UiState::with_mut_in_world(&mut ctx.world, |ui| {
            ui.pending_commands.extend(cmds);
        });
    }
}
