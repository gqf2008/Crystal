use macroquad::prelude::get_time;

use crate::{
    components::{ActiveNpc, LocalPlayer, NpcCallCooldown, RenderPass},
    game::{GameContext, GameResult},
    network::handlers::NetworkEvent,
    systems::LogicSystem,
    ui::ui_state::{UiAction, UiCommand, UiState},
};

use mir2_shared::enums::PanelType;

/// 将 Unix 时间戳（秒）格式化为中文日期字符串
fn format_mail_date(timestamp: i64) -> String {
    if timestamp <= 0 {
        return "未知时间".to_string();
    }
    // 简单按天格式化：从 epoch 开始计算
    let days = timestamp / 86400;
    let secs_of_day = (timestamp % 86400).abs();
    let hours = secs_of_day / 3600;
    let mins = (secs_of_day % 3600) / 60;
    format!("{}天 {:02}:{:02}", days, hours, mins)
}

#[derive(ecs_macros::LogicSystem)]
pub struct DialogSystem {
}

impl Default for DialogSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl DialogSystem {
    pub fn new() -> Self {
        Self {}
    }

    fn with_ui_state_mut<R>(
        ctx: &mut GameContext,
        f: impl FnOnce(&mut crate::ui::ui_state::UiStateData) -> R,
    ) -> Option<R> {
        let mut q = ctx.world.query::<&UiState>();
        let s = q.iter().next()?;
        let mut data = s.borrow_mut();
        Some(f(&mut data))
    }

    fn try_consume_npc_call_cooldown(ctx: &mut GameContext) -> bool {
        let now = get_time();

        // RenderPass 单例实体上挂了 NpcCallCooldown。
        let mut q = ctx.world.query::<&mut NpcCallCooldown>();
        let Some(cd) = q.iter().next() else {
            return true;
        };

        if now >= cd.until {
            cd.until = now + 0.35;
            return true;
        }
        false
    }

    fn active_npc_object_id(ctx: &GameContext) -> Option<u32> {
        let mut q = ctx.world.query::<&ActiveNpc>();
        q.iter().next().and_then(|a| a.npc_object_id)
    }

    fn inventory_total_free_space(
        inv: &crate::components::item::Inventory,
        item_index: i32,
        stack_size: u16,
    ) -> u32 {
        let stack_size = stack_size.max(1) as u32;
        let mut free: u32 = 0;

        for slot in inv.items.iter() {
            match slot {
                None => {
                    free = free.saturating_add(stack_size);
                }
                Some(it) => {
                    if it.item_index == item_index {
                        let current = it.count as u32;
                        if current < stack_size {
                            free = free.saturating_add(stack_size - current);
                        }
                    }
                }
            }
        }
        free
    }

    fn can_send_buy_request(
        gold: u32,
        credit: u32,
        inv_free_space: Option<u32>,
        unit_price: u32,
        count: u32,
        stack_size: u16,
        use_pearls: bool,
    ) -> Result<(), &'static str> {
        let currency = if use_pearls { credit } else { gold };

        if unit_price > 0 {
            let cost = (unit_price as u64).saturating_mul(count as u64);
            if cost > currency as u64 {
                return Err(if use_pearls {
                    "You do not have enough Pearls."
                } else {
                    "Not enough gold."
                });
            }
        }

        if let Some(free) = inv_free_space {
            let need = count.min(stack_size.max(1) as u32);
            if free < need {
                return Err("You do not have enough space.");
            }
        }

        Ok(())
    }

    fn pump_network_messages_to_ui(&mut self, ctx: &mut GameContext) {
        // 注意：ctx.events() 会对 ctx 产生不可变借用；因此先收集命令，循环结束后再写 UiState。
        let mut cmds: Vec<UiCommand> = Vec::new();

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
                        });
                        cmds.push(UiCommand::OpenInventory);
                    } else if matches!(*panel_type, PanelType::BuySub) {
                        cmds.push(UiCommand::ShowNpcGoods {
                            items: items.clone(),
                            rate: *rate,
                            panel_type: *panel_type,
                            hide_added_stats: *hide_added_stats,
                            is_sub: true,
                        });
                        cmds.push(UiCommand::OpenInventory);
                    }
                }
                NetworkEvent::SystemMessage { message } => {
                    cmds.push(UiCommand::PushSystemChatLine(message.clone()));
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
                    // 收到组队邀请，显示系统消息
                    cmds.push(UiCommand::PushSystemChatLine(format!("{} 邀请你加入队伍", inviter)));
                }
                NetworkEvent::GroupMemberAdded { name } => {
                    cmds.push(UiCommand::PushSystemChatLine(format!("{} 加入了队伍", name)));
                }
                NetworkEvent::GroupMemberRemoved { name } => {
                    cmds.push(UiCommand::PushSystemChatLine(format!("{} 离开了队伍", name)));
                }
                NetworkEvent::GroupDisbanded => {
                    cmds.push(UiCommand::PushSystemChatLine("队伍已解散".to_string()));
                }
                NetworkEvent::GroupMembersMapUpdated { player_name, player_map } => {
                    cmds.push(UiCommand::UpdateGroupMemberMap {
                        player_name: player_name.clone(),
                        player_map: player_map.clone(),
                    });
                    if !player_map.is_empty() {
                        cmds.push(UiCommand::PushSystemChatLine(
                            format!("{} 在地图: {}", player_name, player_map),
                        ));
                    }
                }
                NetworkEvent::GroupMemberLocationUpdated { name, x, y } => {
                    tracing::debug!("📍 队伍成员位置更新: {} ({}, {})", name, x, y);
                }
                NetworkEvent::GroupModeChanged { allow_group } => {
                    let allow = *allow_group == 0;
                    cmds.push(UiCommand::SetGroupAllowJoin { allow });
                    cmds.push(UiCommand::PushSystemChatLine(
                        format!("组队模式已切换为: {}", if allow { "允许组队" } else { "禁止组队" }),
                    ));
                }
                NetworkEvent::FriendUpdated { friends } => {
                    cmds.push(UiCommand::UpdateFriendList { friends: friends.clone() });
                }
                NetworkEvent::GuildInvite { inviter, guild_name } => {
                    cmds.push(UiCommand::PushSystemChatLine(format!("{} 邀请你加入行会「{}」", inviter, guild_name)));
                }
                NetworkEvent::GuildJoined { guild_name } => {
                    cmds.push(UiCommand::PushSystemChatLine(format!("你已加入行会「{}」", guild_name)));
                }
                NetworkEvent::GuildLeft => {
                    cmds.push(UiCommand::PushSystemChatLine("你已退出行会".to_string()));
                }
                NetworkEvent::GuildNoticeUpdated { notice } => {
                    cmds.push(UiCommand::PushSystemChatLine(format!("行会公告已更新：{}", notice)));
                    cmds.push(UiCommand::GuildNoticeUpdated { notice: notice.clone() });
                }
                NetworkEvent::MentorRequested2 => {
                    cmds.push(UiCommand::PushSystemChatLine("收到拜师请求".to_string()));
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
                    cmds.push(UiCommand::PushSystemChatLine(format!("{} 请求与你交易", requester)));
                }
                NetworkEvent::TradeCompleted => {
                    cmds.push(UiCommand::PushSystemChatLine("交易完成".to_string()));
                }
                NetworkEvent::TradeCancelled => {
                    cmds.push(UiCommand::PushSystemChatLine("交易已取消".to_string()));
                }
                NetworkEvent::MountUpdated { mount_type, riding_mount, .. } => {
                    cmds.push(UiCommand::UpdateMountState {
                        mount_type: *mount_type,
                        riding: *riding_mount,
                    });
                }
                NetworkEvent::HeroCreateRequested => {
                    cmds.push(UiCommand::PushHeroSystemChat("创建英雄请求".to_string()));
                }
                NetworkEvent::NewHeroCreated => {
                    cmds.push(UiCommand::PushHeroSystemChat("新英雄已创建".to_string()));
                }
                NetworkEvent::HeroInfoReceived => {
                    cmds.push(UiCommand::PushHeroSystemChat("英雄信息已接收".to_string()));
                }
                NetworkEvent::HeroSpawnStateUpdated { state } => {
                    let state_str = match *state {
                        1 => "未召唤",
                        2 => "已召唤",
                        3 => "已死亡",
                        _ => "未知",
                    };
                    cmds.push(UiCommand::PushHeroSystemChat(format!("英雄状态：{}", state_str)));
                }
                NetworkEvent::HeroBehaviourSet { behaviour } => {
                    cmds.push(UiCommand::UpdateHeroBehaviour { behaviour: *behaviour });
                }
                NetworkEvent::HeroChanged => {
                    cmds.push(UiCommand::PushHeroSystemChat("英雄已切换".to_string()));
                }
                NetworkEvent::HeroExperienceGained { amount } => {
                    cmds.push(UiCommand::PushHeroSystemChat(format!("英雄获得经验：{}", amount)));
                }
                NetworkEvent::HeroLevelUp { new_level } => {
                    cmds.push(UiCommand::PushHeroSystemChat(format!("英雄升级到 Lv.{}", new_level)));
                }
                NetworkEvent::FishingStatusUpdated { state } => {
                    cmds.push(UiCommand::UpdateFishingState {
                        state: *state,
                        chance: 0.0,
                        progress: 0.0,
                    });
                }
                NetworkEvent::NewIntelligentCreatureReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("新智能宠物已收到".to_string()));
                }
                NetworkEvent::IntelligentCreatureListUpdated => {
                    cmds.push(UiCommand::PushSystemChatLine("智能宠物列表已更新".to_string()));
                }
                NetworkEvent::BuffAdded { object_id: _, buff_id } => {
                    cmds.push(UiCommand::AddBuff {
                        buff: crate::scenes::dialogs::game::buff_dialog::BuffEntry {
                            buff_type: *buff_id,
                            icon_index: *buff_id,
                            name: format!("Buff #{}", buff_id),
                            remaining_secs: 0.0,
                            is_paused: false,
                            caster: String::new(),
                        },
                    });
                }
                NetworkEvent::BuffRemoved { object_id: _, buff_id } => {
                    cmds.push(UiCommand::RemoveBuff { buff_type: *buff_id });
                }
                NetworkEvent::HeroAutoPotUnlocked => {
                    cmds.push(UiCommand::SetHeroAutoPotUnlocked);
                }
                NetworkEvent::HeroAutoPotSet { pot_type, value } => {
                    cmds.push(UiCommand::SetHeroAutoPotValue { pot_type: *pot_type, value: *value });
                }
                NetworkEvent::HeroAutoPotItemSet { item_id } => {
                    cmds.push(UiCommand::SetHeroAutoPotItem { item_id: *item_id });
                }
                NetworkEvent::HeroManageReceived => {
                    cmds.push(UiCommand::PushHeroSystemChat("英雄管理信息已收到".to_string()));
                }
                NetworkEvent::HeroBaseStatsReceived => {
                    cmds.push(UiCommand::PushHeroSystemChat("英雄基础属性已收到".to_string()));
                }
                NetworkEvent::NewHeroInfoReceived => {
                    cmds.push(UiCommand::PushHeroSystemChat("新英雄信息已收到".to_string()));
                }
                NetworkEvent::IntelligentCreatureRenameEnabled => {
                    cmds.push(UiCommand::PushSystemChatLine("宠物重命名已启用".to_string()));
                }
                NetworkEvent::IntelligentCreaturePickupReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("宠物拾取物品".to_string()));
                }
                NetworkEvent::BuffPaused { object_id: _, buff_id, paused } => {
                    cmds.push(UiCommand::SetBuffPaused { buff_id: *buff_id, paused: *paused });
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
                NetworkEvent::TradeItemAdded => {
                    cmds.push(UiCommand::TradeItemAdded);
                }
                NetworkEvent::TradeConfirmedEvent { locked } => {
                    cmds.push(UiCommand::TradeConfirmed { locked: *locked });
                }
                NetworkEvent::TradeCancelledEvent => {
                    cmds.push(UiCommand::TradeCancelled);
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
                NetworkEvent::GuildWarRequested => {
                    cmds.push(UiCommand::GuildWarRequested);
                }
                // NPC 操作确认提示
                NetworkEvent::NPCSellReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("物品出售成功".to_string()));
                }
                NetworkEvent::NPCRepairReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("修理完成".to_string()));
                }
                NetworkEvent::NPCSRepairReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("特殊修理完成".to_string()));
                }
                NetworkEvent::NPCRefineReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("精炼操作完成".to_string()));
                }
                NetworkEvent::NPCCheckRefineReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("精炼状态已确认".to_string()));
                }
                NetworkEvent::NPCCollectRefineReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("精炼物品已提取".to_string()));
                }
                NetworkEvent::NPCReplaceWedRingReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("结婚戒指更换完成".to_string()));
                }
                NetworkEvent::NPCStorageReceived => {
                    cmds.push(UiCommand::OpenStorage);
                }
                NetworkEvent::NPCConsignReceived => {
                    cmds.push(UiCommand::OpenTrustMerchant);
                }
                NetworkEvent::NPCAwakeningReceived => {
                    // 觉醒对话框由 AwakeningNeedMaterialsReceived 触发
                }
                NetworkEvent::NPCDisassembleReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("分解操作完成".to_string()));
                }
                NetworkEvent::NPCDowngradeReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("降级操作完成".to_string()));
                }
                NetworkEvent::NPCResetReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("重置操作完成".to_string()));
                }
                NetworkEvent::NPCPearlGoodsReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("珍珠商品列表已更新".to_string()));
                }
                NetworkEvent::NPCRequestInputReceived { prompt, .. } => {
                    cmds.push(UiCommand::PushSystemChatLine(prompt.clone()));
                }
                // 公会扩展事件
                NetworkEvent::GuildStorageGoldChanged { delta } => {
                    cmds.push(UiCommand::PushSystemChatLine(format!("行会资金 {}", if *delta >= 0 { format!("+{}", delta) } else { format!("{}", delta) })));
                }
                NetworkEvent::GuildStorageItemChanged { change_type, slot } => {
                    let action = if *change_type == 0 { "存入" } else { "取出" };
                    cmds.push(UiCommand::PushSystemChatLine(format!("行会仓库物品{}: 槽位{}", action, slot)));
                }
                NetworkEvent::GuildStorageListReceived => {
                    cmds.push(UiCommand::ClearGuildStorageItems);
                    cmds.push(UiCommand::PushSystemChatLine("行会仓库列表已更新".to_string()));
                }
                NetworkEvent::GuildTerritoryPageReceived => {
                    cmds.push(UiCommand::ShowGuildTerritory);
                }
                NetworkEvent::GuildTerritoryPurchased => {
                    cmds.push(UiCommand::PushSystemChatLine("行会领地购买成功".to_string()));
                }
                NetworkEvent::GuildBuffListReceived { buff_ids } => {
                    cmds.push(UiCommand::PushSystemChatLine(format!("行会Buff列表已更新: {} 个", buff_ids.len())));
                }
                // NPC 市场/寄售事件
                NetworkEvent::NPCMarketEvent => {
                    cmds.push(UiCommand::PushSystemChatLine("NPC 市场已打开".to_string()));
                }
                NetworkEvent::NPCMarketPageEvent => {
                    // 寄售行页面刷新（具体数据由后续事件推送）
                }
                NetworkEvent::ConsignItemReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("寄售物品状态已更新".to_string()));
                }
                NetworkEvent::MarketFailedEvent { reason } => {
                    cmds.push(UiCommand::PushSystemChatLine(format!("市场操作失败: {}", reason)));
                }
                NetworkEvent::MarketSuccessEvent => {
                    cmds.push(UiCommand::PushSystemChatLine("市场操作成功".to_string()));
                }
                NetworkEvent::SellItemReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("物品出售成功".to_string()));
                }
                NetworkEvent::CraftItemReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("合成/制作完成".to_string()));
                }
                NetworkEvent::RepairItemReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("修理请求已发送".to_string()));
                }
                NetworkEvent::ItemRepairedEvent => {
                    cmds.push(UiCommand::PushSystemChatLine("物品修理完成".to_string()));
                }
                NetworkEvent::DefaultNPCReceived { message, .. } => {
                    cmds.push(UiCommand::PushSystemChatLine(message.clone()));
                }
                NetworkEvent::AwakeningNeedMaterialsReceived => {
                    // 显示觉醒对话框（材料由服务器推送）
                    cmds.push(UiCommand::ShowNPCAwake {
                        item_name: "装备".to_string(),
                        materials: Vec::new(),
                    });
                }
                NetworkEvent::AwakeningLockedItemReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("觉醒锁定物品".to_string()));
                }
                NetworkEvent::AwakeningReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("觉醒操作完成".to_string()));
                }
                NetworkEvent::MailLockedItemReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("邮件附件已锁定".to_string()));
                }
                NetworkEvent::MailSendRequestReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("收到邮件发送请求".to_string()));
                }
                NetworkEvent::MailSentEvent => {
                    cmds.push(UiCommand::PushSystemChatLine("邮件已发送".to_string()));
                }
                NetworkEvent::MailCostReceived { cost } => {
                    cmds.push(UiCommand::PushSystemChatLine(format!("邮件费用: {} 金币", cost)));
                }
                NetworkEvent::ParcelCollectedEvent => {
                    cmds.push(UiCommand::PushSystemChatLine("包裹物品已收集".to_string()));
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
                NetworkEvent::RentalItemDeposited => {
                    cmds.push(UiCommand::PushSystemChatLine("租赁物品已存入".to_string()));
                }
                NetworkEvent::RentalItemRetrieved => {
                    cmds.push(UiCommand::PushSystemChatLine("租赁物品已取回".to_string()));
                }
                NetworkEvent::RentalItemUpdated => {
                    cmds.push(UiCommand::PushSystemChatLine("租赁物品已更新".to_string()));
                }
                NetworkEvent::RentalItemsReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("租赁物品列表已收到".to_string()));
                }
                NetworkEvent::ItemRentalCancelled => {
                    cmds.push(UiCommand::CloseItemRental);
                }
                NetworkEvent::ItemRentalLocked => {
                    cmds.push(UiCommand::SetRentalLocked { locked: true });
                }
                NetworkEvent::ItemRentalPartnerLocked => {
                    cmds.push(UiCommand::SetRentalPartnerLocked { locked: true });
                }
                NetworkEvent::ItemRentalConfirmable => {
                    cmds.push(UiCommand::PushSystemChatLine("租赁可确认".to_string()));
                }
                NetworkEvent::ItemRentalConfirmed => {
                    cmds.push(UiCommand::CloseItemRental);
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
                NetworkEvent::RankingsReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("排行榜已收到（待解析）".to_string()));
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
                NetworkEvent::OutputMessageReceived { message } => {
                    cmds.push(UiCommand::PushSystemChatLine(message.clone()));
                }
                NetworkEvent::MapEffectReceived { effect } => {
                    cmds.push(UiCommand::PushSystemChatLine(format!("地图特效触发: {}", effect)));
                }
                NetworkEvent::TimeOfDayChanged { time_of_day } => {
                    let desc = match *time_of_day {
                        0 => "白天",
                        1 => "黄昏",
                        2 => "夜晚",
                        3 => "凌晨",
                        _ => "未知",
                    };
                    cmds.push(UiCommand::PushSystemChatLine(format!("时间变化: {}", desc)));
                }
                NetworkEvent::ObserveAllowed { allowed } => {
                    cmds.push(UiCommand::PushSystemChatLine(if *allowed { "允许观察".to_string() } else { "禁止观察".to_string() }));
                }
                NetworkEvent::TransformUpdated { form } => {
                    cmds.push(UiCommand::PushSystemChatLine(format!("变身状态变更: {}", form)));
                }
                NetworkEvent::BaseStatsReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("基础属性已更新".to_string()));
                }
                NetworkEvent::NewMapInfoReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("新地图信息已收到".to_string()));
                }
                NetworkEvent::WorldMapSetupReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("世界地图设置已收到".to_string()));
                }
                NetworkEvent::SearchMapResultReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("地图搜索完成".to_string()));
                }
                NetworkEvent::ConsignItemEvent => {
                    cmds.push(UiCommand::PushSystemChatLine("寄售物品状态变更".to_string()));
                }
                NetworkEvent::NPCConsignEvent => {
                    cmds.push(UiCommand::PushSystemChatLine("NPC 寄售状态变更".to_string()));
                }
                NetworkEvent::NPCMarketEvent2 => {
                    cmds.push(UiCommand::PushSystemChatLine("NPC 市场已刷新".to_string()));
                }
                NetworkEvent::NPCMarketPageEvent2 => {
                    cmds.push(UiCommand::PushSystemChatLine("NPC 市场页面已更新".to_string()));
                }
                NetworkEvent::MarketFailedEvent2 { reason } => {
                    cmds.push(UiCommand::PushSystemChatLine(format!("市场失败: {}", reason)));
                }
                NetworkEvent::MarketSuccessEvent2 => {
                    cmds.push(UiCommand::PushSystemChatLine("市场操作成功".to_string()));
                }
                // 婚姻/师徒补充
                NetworkEvent::MarriageRequested2 { requester } => {
                    cmds.push(UiCommand::SetMarriageRequester { requester: requester.clone() });
                }
                NetworkEvent::DivorceRequested2 => {
                    cmds.push(UiCommand::ClearMarriageRequester);
                }
                NetworkEvent::DoorOpened { door_id } => {
                    cmds.push(UiCommand::PushSystemChatLine(format!("门已打开 (id={})", door_id)));
                }
                NetworkEvent::BrowserOpened { url } => {
                    cmds.push(UiCommand::PushSystemChatLine(format!("浏览器已打开: {}", url)));
                }
                NetworkEvent::BindingShotSet { enabled } => {
                    cmds.push(UiCommand::PushSystemChatLine(if *enabled { "束缚箭已启用".to_string() } else { "束缚箭已禁用".to_string() }));
                }
                NetworkEvent::ConcentrationSet { enabled } => {
                    cmds.push(UiCommand::PushSystemChatLine(if *enabled { "专注已启用".to_string() } else { "专注已禁用".to_string() }));
                }
                NetworkEvent::ElementalSet { element } => {
                    cmds.push(UiCommand::PushSystemChatLine(format!("元素属性变更: element={}", element)));
                }
                NetworkEvent::DuraChanged { unique_id: _, durability } => {
                    cmds.push(UiCommand::PushSystemChatLine(format!("装备耐久度变化: {}", durability)));
                }
                NetworkEvent::DelayedExplosionRemoved => {
                    cmds.push(UiCommand::PushSystemChatLine("延迟爆炸已移除".to_string()));
                }
                NetworkEvent::ChatItemStatsReceived => {
                    cmds.push(UiCommand::PushSystemChatLine("聊天物品统计已收到".to_string()));
                }
                NetworkEvent::InventoryResized { new_size } => {
                    cmds.push(UiCommand::PushSystemChatLine(format!("背包大小调整为: {}", new_size)));
                }
                NetworkEvent::StorageResized { new_size } => {
                    cmds.push(UiCommand::PushSystemChatLine(format!("仓库大小调整为: {}", new_size)));
                }
                NetworkEvent::UserStorageReceived { items } => {
                    cmds.push(UiCommand::UpdateStorageItems { items: items.clone() });
                }
                NetworkEvent::GuildNameReceived { name } => {
                    cmds.push(UiCommand::SetGuildName { name: name.clone() });
                }
                NetworkEvent::ChangePasswordSuccess => {
                    cmds.push(UiCommand::PushSystemChatLine("密码修改成功".to_string()));
                }
                NetworkEvent::ChangePasswordFailed { reason } => {
                    cmds.push(UiCommand::PushSystemChatLine(format!("密码修改失败: {}", reason)));
                }
                NetworkEvent::ReincarnationRequested => {
                    cmds.push(UiCommand::PushSystemChatLine("转生请求已收到".to_string()));
                }
                NetworkEvent::ReincarnationCancelled => {
                    cmds.push(UiCommand::PushSystemChatLine("转生已取消".to_string()));
                }
                NetworkEvent::HeroHealthChanged { hp, mp } => {
                    tracing::debug!("🧡 英雄HP/MP更新: hp={} mp={}", hp, mp);
                }
                NetworkEvent::LogOutSuccess => {
                    cmds.push(UiCommand::PushSystemChatLine("已安全退出游戏".to_string()));
                }
                NetworkEvent::LogOutFailed => {
                    cmds.push(UiCommand::PushSystemChatLine("退出游戏失败".to_string()));
                }
                NetworkEvent::ReturnToLogin => {
                    cmds.push(UiCommand::PushSystemChatLine("返回登录界面".to_string()));
                }
                NetworkEvent::RefineItemDeposited => {
                    cmds.push(UiCommand::PushSystemChatLine("精炼物品已存入".to_string()));
                }
                NetworkEvent::RefineItemRetrieved => {
                    cmds.push(UiCommand::PushSystemChatLine("精炼物品已取回".to_string()));
                }
                NetworkEvent::RefineCancelled => {
                    cmds.push(UiCommand::PushSystemChatLine("精炼已取消".to_string()));
                }
                NetworkEvent::RefineItemCompleted => {
                    cmds.push(UiCommand::PushSystemChatLine("精炼完成！".to_string()));
                }
                NetworkEvent::NoticeUpdated { notice } => {
                    cmds.push(UiCommand::ShowNotice { text: notice.clone() });
                }
                _ => {}
            }
        }

        if !cmds.is_empty() {
            let _ = Self::with_ui_state_mut(ctx, |ui| {
                ui.pending_commands.extend(cmds);
            });
        }
    }

    fn handle_npc_goods_action(&mut self, ctx: &mut GameContext, action: crate::scenes::dialogs::game::npc_goods_dialog::NpcGoodsDialogAction) {
        use crate::network::handlers::NetworkEvent as NetEv;

        match action {
            crate::scenes::dialogs::game::npc_goods_dialog::NpcGoodsDialogAction::OpenSubGoods {
                items,
                rate,
                hide_added_stats,
            } => {
                let _ = Self::with_ui_state_mut(ctx, |ui| {
                    ui.pending_commands.push(UiCommand::ShowNpcGoods {
                        items,
                        rate,
                        panel_type: PanelType::BuySub,
                        hide_added_stats,
                        is_sub: true,
                    });
                    ui.pending_commands.push(UiCommand::OpenInventory);
                });
            }
            crate::scenes::dialogs::game::npc_goods_dialog::NpcGoodsDialogAction::OpenAmountBox {
                title,
                image_index,
                default_amount,
                unique_id,
                item_index,
                stack_size,
                unit_price,
                use_pearls,
            } => {
                let (gold, credit, free_space) = {
                    let mut gold: u32 = 0;
                    let mut credit: u32 = 0;
                    let mut free_space: Option<u32> = None;

                    {
                        let mut q = ctx.world.query::<(
                            &LocalPlayer,
                            &crate::components::combat::Currency,
                            &crate::components::item::Inventory,
                        )>();
                        if let Some((_local, cur, inv)) = q.iter().next() {
                            gold = cur.gold;
                            credit = cur.credit;
                            free_space =
                                Some(Self::inventory_total_free_space(inv, item_index, stack_size));
                            if gold == 0 {
                                gold = inv.gold;
                            }
                        }
                    }

                    (gold, credit, free_space)
                };

                let stack_max = stack_size.max(1) as u32;
                let currency = if use_pearls { credit } else { gold };

                let mut max_quantity = stack_max;
                if unit_price > 0 {
                    let full_cost = (unit_price as u64).saturating_mul(stack_max as u64);
                    if full_cost > currency as u64 {
                        max_quantity = currency / unit_price;
                    }
                }

                {
                    if max_quantity == 0 {
                        let _ = Self::with_ui_state_mut(ctx, |ui| {
                            ui.pending_commands.push(UiCommand::PushSystemChatLine(
                                (if use_pearls {
                                    "You do not have enough Pearls."
                                } else {
                                    "Not enough gold."
                                })
                                .to_string(),
                            ));
                        });
                        return;
                    }

                    if let Some(free) = free_space {
                        max_quantity = max_quantity.min(free).min(stack_max);
                    }

                    if max_quantity == 0 {
                        let _ = Self::with_ui_state_mut(ctx, |ui| {
                            ui.pending_commands.push(UiCommand::PushSystemChatLine(
                                "You do not have enough space.".to_string(),
                            ));
                        });
                        return;
                    }

                    let _ = Self::with_ui_state_mut(ctx, |ui| {
                        ui.pending_commands.push(UiCommand::ShowAmountBox {
                            title,
                            image_index,
                            max_quantity,
                            min_quantity: 0,
                            default_amount,
                            buy_uid: unique_id,
                        });
                    });
                }
            }
            crate::scenes::dialogs::game::npc_goods_dialog::NpcGoodsDialogAction::RequestBuy {
                unique_id,
                count,
                item_index,
                stack_size,
                unit_price,
                use_pearls,
            } => {
                let (gold, credit, free_space) = {
                    let mut gold: u32 = 0;
                    let mut credit: u32 = 0;
                    let mut free_space: Option<u32> = None;

                    {
                        let mut q = ctx.world.query::<(
                            &LocalPlayer,
                            &crate::components::combat::Currency,
                            &crate::components::item::Inventory,
                        )>();
                        if let Some((_local, cur, inv)) = q.iter().next() {
                            gold = cur.gold;
                            credit = cur.credit;
                            free_space =
                                Some(Self::inventory_total_free_space(inv, item_index, stack_size));
                            if gold == 0 {
                                gold = inv.gold;
                            }
                        }
                    }

                    (gold, credit, free_space)
                };

                if let Err(msg) = Self::can_send_buy_request(
                    gold,
                    credit,
                    free_space,
                    unit_price,
                    count,
                    stack_size,
                    use_pearls,
                ) {
                    let _ = Self::with_ui_state_mut(ctx, |ui| {
                        ui.pending_commands
                            .push(UiCommand::PushSystemChatLine(msg.to_string()));
                    });
                    return;
                }

                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::BuyItemRequest {
                        item_index: unique_id,
                        count,
                        panel_type: PanelType::Buy as u8,
                    });
                }
            }
            crate::scenes::dialogs::game::npc_goods_dialog::NpcGoodsDialogAction::RequestSell {
                unique_id,
                count,
            } => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::SellItemRequest { unique_id, count });
                }
            }
            crate::scenes::dialogs::game::npc_goods_dialog::NpcGoodsDialogAction::RequestRepair {
                unique_id,
            } => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::RepairItemRequest { unique_id });
                }
            }
            crate::scenes::dialogs::game::npc_goods_dialog::NpcGoodsDialogAction::RequestStorage {
                unique_id,
                deposit,
            } => {
                if let Some(net) = ctx.net.as_ref() {
                    if deposit {
                        let _ = net.send(NetEv::StoreItemRequest { unique_id });
                    } else {
                        let _ = net.send(NetEv::TakeBackItemRequest { unique_id });
                    }
                }
            }
            crate::scenes::dialogs::game::npc_goods_dialog::NpcGoodsDialogAction::RequestCraft { item } => {
                // 打开合成对话框，配方信息来自 NPC 商品列表
                let recipe = crate::scenes::dialogs::game::craft_dialog::CraftRecipe {
                    name: item.info.as_ref().map(|i| i.name.clone()).unwrap_or_else(|| "未知配方".to_string()),
                    recipe_unique_id: item.unique_id,
                    materials: Vec::new(), // TODO: 从服务器配方数据填充
                };
                let _ = Self::with_ui_state_mut(ctx, |ui| {
                    ui.pending_commands.push(UiCommand::ShowCraft { recipes: vec![recipe] });
                });
            }
        }
    }

    fn process_ui_actions(&mut self, ctx: &mut GameContext) {
        let actions = Self::with_ui_state_mut(ctx, |ui| std::mem::take(&mut ui.pending_actions))
            .unwrap_or_default();

        for action in actions {
            match action {
                UiAction::NpcDialog(a) => match a {
                    crate::scenes::dialogs::game::npc_dialog::NpcDialogAction::None => {}
                    crate::scenes::dialogs::game::npc_dialog::NpcDialogAction::Close => {
                        let _ = Self::with_ui_state_mut(ctx, |ui| {
                            ui.pending_commands.push(UiCommand::CloseNpcRelatedDialogs);
                        });
                    }
                    crate::scenes::dialogs::game::npc_dialog::NpcDialogAction::OpenLink { url } => {
                        let _ = Self::with_ui_state_mut(ctx, |ui| {
                            ui.pending_commands
                                .push(UiCommand::PushSystemChatLine(format!("链接：{}", url)));
                        });
                    }
                    crate::scenes::dialogs::game::npc_dialog::NpcDialogAction::ClickAction { action } => {
                        if !Self::try_consume_npc_call_cooldown(ctx) {
                            continue;
                        }

                        let Some(npc_object_id) = Self::active_npc_object_id(ctx) else {
                            let _ = Self::with_ui_state_mut(ctx, |ui| {
                                ui.pending_commands.push(UiCommand::PushSystemChatLine(
                                    "当前没有选中的 NPC，无法发送对话选项。".to_string(),
                                ));
                            });
                            continue;
                        };

                        if let Some(net) = ctx.net.as_ref() {
                            let key = format!("[{}]", action);
                            let _ = net.send(NetworkEvent::NPCCallRequest { npc_object_id, key });
                        }
                    }
                },
                UiAction::NpcGoods(a) | UiAction::NpcSubGoods(a) => {
                    self.handle_npc_goods_action(ctx, a);
                }
                UiAction::AmountBox(r) => {
                    match r {
                        crate::scenes::dialogs::game::amount_box::AmountBoxResult::Ok(amount) => {
                            if amount > 0 {
                                let uid = Self::with_ui_state_mut(ctx, |ui| ui.amount_box_buy_uid.take())
                                    .flatten();
                                if let Some(uid) = uid {
                                    if let Some(net) = ctx.net.as_ref() {
                                        let _ = net.send(NetworkEvent::BuyItemRequest {
                                            item_index: uid,
                                            count: amount,
                                            panel_type: PanelType::Buy as u8,
                                        });
                                    }
                                }
                            }
                        }
                        crate::scenes::dialogs::game::amount_box::AmountBoxResult::Cancel => {
                            let _ = Self::with_ui_state_mut(ctx, |ui| {
                                ui.amount_box_buy_uid = None;
                            });
                        }
                        crate::scenes::dialogs::game::amount_box::AmountBoxResult::None => {}
                    }
                }
            }
        }
    }
}

impl LogicSystem for DialogSystem {
    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        // 1) 网络消息 -> UI
        self.pump_network_messages_to_ui(ctx);

        // 2) UI action -> 发包/弹窗/窗口管理
        self.process_ui_actions(ctx);

        // 3) 读一下 RenderPass，避免未来需要做 per-pass 的 UI gating
        let _ = ctx.world.query::<&RenderPass>().iter().next();

        // 注意：EventBus 的 clear_frame 由 GameScene 在帧末处理。
        Ok(())
    }
}
