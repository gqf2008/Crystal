use super::*;

pub fn update(sys: &mut UIRenderSystem, ctx: &mut GameContext, _dt: f32) -> GameResult {
    // 1) 应用表现层写入的 UI 命令（驱动具体 UI 组件）
    let commands = ctx
        .world
        .query::<&UiState>()
        .iter()
        .next()
        .map(|s| std::mem::take(&mut s.borrow_mut().pending_commands))
        .unwrap_or_default();
    for cmd in commands {
        match cmd {
            UiCommand::CloseNpcRelatedDialogs => sys.close_npc_related_dialogs(),
            UiCommand::CloseAllPopups => sys.main_dialog.close_all_popups(),
            UiCommand::OpenInventory => sys.main_dialog.open_inventory(),
            UiCommand::ActivateChatInput => sys.main_dialog.activate_chat_input(),
            UiCommand::ToggleMinimap => sys.main_dialog.toggle_minimap(),
            UiCommand::ToggleMinimapSize => sys.main_dialog.toggle_minimap_size(),
            UiCommand::PushSystemChatLine(line) => sys.main_dialog.push_system_chat_line(line),
            UiCommand::PushChatLine(line) => sys.main_dialog.push_chat_line(line),
            UiCommand::PushWhisperLine(line) => sys.main_dialog.push_whisper_line(line),
            UiCommand::ShowNpcDialog { dialog } => {
                sys.npc_goods_dialog.hide();
                sys.npc_sub_goods_dialog.hide();
                sys.amount_box.hide();
                if let Some(s) = ctx.world.query::<&UiState>().iter().next() {
                    s.borrow_mut().amount_box_buy_uid = None;
                }
                sys.npc_dialog.new_dialog(dialog);
                sys.bring_npc_layer_to_front(NpcUiLayer::Dialog);
            }
            UiCommand::ShowNpcGoods {
                items,
                rate,
                panel_type,
                hide_added_stats,
                is_sub,
                use_pearls,
            } => {
                if is_sub {
                    sys.npc_sub_goods_dialog.new_goods(
                        items,
                        rate,
                        panel_type,
                        hide_added_stats,
                        use_pearls,
                    );
                    sys.bring_npc_layer_to_front(NpcUiLayer::SubGoods);
                } else {
                    sys.npc_goods_dialog
                        .new_goods(items, rate, panel_type, hide_added_stats, use_pearls);
                    sys.bring_npc_layer_to_front(NpcUiLayer::Goods);
                }
                sys.main_dialog.open_inventory();
            }
            UiCommand::ShowAmountBox {
                title,
                image_index,
                max_quantity,
                min_quantity,
                default_amount,
                buy_uid,
            } => {
                sys.amount_box.show(
                    title,
                    image_index,
                    max_quantity,
                    min_quantity,
                    default_amount,
                );
                if let Some(s) = ctx.world.query::<&UiState>().iter().next() {
                    s.borrow_mut().amount_box_buy_uid = Some(buy_uid);
                }
            }
            UiCommand::HideAmountBox => {
                sys.amount_box.hide();
                if let Some(s) = ctx.world.query::<&UiState>().iter().next() {
                    s.borrow_mut().amount_box_buy_uid = None;
                }
            }
            UiCommand::HideNpcGoodsSub => {
                sys.npc_sub_goods_dialog.hide();
            }
            UiCommand::HideNpcGoods => {
                sys.npc_goods_dialog.hide();
            }
            UiCommand::UpdateMountState { mount_type, riding } => {
                sys.main_dialog.mount_dialog_mut().update_mount_state(mount_type, riding);
            }
            UiCommand::UpdateHeroBehaviour { behaviour } => {
                if let Ok(b) = behaviour.try_into() {
                    sys.main_dialog.hero_dialog_mut().set_behaviour(b);
                }
            }
            UiCommand::PushHeroSystemChat(msg) => {
                sys.main_dialog.push_system_chat_line(msg);
            }
            UiCommand::UpdateHeroHealth { hp, mp } => {
                sys.main_dialog.hero_dialog_mut().update_health(hp, mp);
            }
            UiCommand::UpdateHeroSpawnState { state } => {
                sys.main_dialog.hero_dialog_mut().set_spawn_state(state);
            }
            UiCommand::HeroChanged => {
                sys.main_dialog.push_system_chat_line("英雄已切换");
            }
            UiCommand::PlayerLevelUp { new_level } => {
                sys.main_dialog.push_system_chat_line(format!("🎉 升级到 Lv.{}！", new_level));
            }
            UiCommand::UpdateFishingState { state, chance, progress } => {
                sys.main_dialog.fishing_dialog_mut().update_fishing_state(state, chance, progress);
            }
            UiCommand::SetFishingAutoCast { enabled } => {
                sys.main_dialog.fishing_dialog_mut().set_auto_cast(enabled);
            }
            UiCommand::AddBuff { buff } => {
                sys.main_dialog.buff_dialog_mut().add_buff(buff.clone());
            }
            UiCommand::RemoveBuff { buff_type } => {
                sys.main_dialog.buff_dialog_mut().remove_buff(buff_type);
            }
            UiCommand::UpdateCreatureList { creatures } => {
                sys.main_dialog.intelligent_creature_dialog_mut().update_creatures(creatures.clone());
            }
            UiCommand::UpdateFriendList { friends } => {
                let friend_infos: Vec<crate::scenes::dialogs::game::friend_dialog::FriendInfo> = friends
                    .iter()
                    .map(|f| crate::scenes::dialogs::game::friend_dialog::FriendInfo {
                        object_id: f.object_id,
                        name: f.name.clone(),
                        memo: f.memo.clone(),
                        online: f.online,
                    })
                    .collect();
                sys.main_dialog.friend_dialog_mut().update_friends(friend_infos);
            }
            UiCommand::UpdateGroupMembers { members } => {
                sys.main_dialog.group_dialog_mut().update_members(members);
            }
            UiCommand::SetGroupAllowJoin { allow } => {
                sys.main_dialog.group_dialog_mut().set_allow_join(allow);
            }
            UiCommand::UpdateGroupMemberMap { player_name, player_map } => {
                sys.main_dialog.group_dialog_mut().update_member_map(&player_name, player_map);
            }
            UiCommand::UpdateGroupMemberLocation { player_name, x, y } => {
                sys.main_dialog.group_dialog_mut().update_member_location(&player_name, x, y);
            }
            UiCommand::AddGroupMember { name } => {
                sys.main_dialog.group_dialog_mut().add_member(
                    crate::scenes::dialogs::game::group_dialog::GroupMember {
                        name: name.clone(),
                        hp_percent: 1.0,
                        online: true,
                        is_leader: false,
                        map_name: String::new(),
                        x: 0,
                        y: 0,
                    },
                );
            }
            UiCommand::RemoveGroupMember { name } => {
                sys.main_dialog.group_dialog_mut().remove_member(&name);
            }
            UiCommand::ClearGroupMembers => {
                sys.main_dialog.group_dialog_mut().update_members(Vec::new());
            }
            UiCommand::SetHeroAutoPotUnlocked => {
                sys.main_dialog.hero_dialog_mut().set_auto_pot_unlocked(true);
            }
            UiCommand::SetHeroAutoPotValue { pot_type, value } => {
                sys.main_dialog.hero_dialog_mut().set_auto_pot_value(pot_type, value);
            }
            UiCommand::SetHeroAutoPotItem { slot, item_id } => {
                sys.main_dialog.hero_dialog_mut().set_auto_pot_item(slot, item_id);
            }
            UiCommand::SetBuffPaused { buff_id, paused } => {
                sys.main_dialog.buff_dialog_mut().set_buff_paused(buff_id, paused);
            }
            UiCommand::UpdateCompass { location } => {
                let dir = crate::scenes::dialogs::game::compass_dialog::CompassDirection::from_location(location.0, location.1);
                sys.main_dialog.compass_dialog_mut().set_direction(dir);
            }
            UiCommand::OpenTradeDialog { partner } => {
                sys.main_dialog.open_trade_dialog(&partner);
            }
            UiCommand::TradeGoldAdded { amount } => {
                sys.main_dialog.trade_dialog_mut().add_their_gold(amount);
            }
            UiCommand::TradeItemAdded { items } => {
                let count = items.iter().filter(|i| i.is_some()).count();
                tracing::debug!("Trade item added: {} items from partner", count);
            }
            UiCommand::TradeItemDeposited { from_slot, success } => {
                if !success {
                    sys.main_dialog.push_system_chat_line(format!("存入交易物品失败 (槽位{})", from_slot));
                }
            }
            UiCommand::TradeItemRetrieved { from_slot, success } => {
                if !success {
                    sys.main_dialog.push_system_chat_line(format!("取回交易物品失败 (槽位{})", from_slot));
                }
            }
            UiCommand::TradeConfirmed { locked } => {
                sys.main_dialog.trade_dialog_mut().set_partner_confirmed(locked);
            }
            UiCommand::TradeCancelled { unlock: _ } => {
                sys.main_dialog.trade_dialog_mut().reset_confirmations();
            }
            UiCommand::TradeCompleted => {
                sys.main_dialog.trade_dialog_mut().reset_confirmations();
            }
            UiCommand::QuestAccepted { quest_id, name, description } => {
                use crate::scenes::dialogs::game::quest_log_dialog::{QuestInfo, QuestRewards, QuestStatus};
                // 优先使用缓存的真实数据（来自 NewQuestInfo），否则用传入的 stub 数据
                let (real_name, real_group, real_desc, level_req, reward_exp, reward_gold) =
                    sys.cached_quest_info.remove(&quest_id)
                        .unwrap_or_else(|| (name.clone(), String::new(), description.clone(), 0u32, 0u64, 0u32));
                sys.main_dialog.quest_log_dialog_mut().add_quest(QuestInfo {
                    id: quest_id,
                    name: real_name,
                    description: real_desc,
                    npc_name: String::new(),
                    status: QuestStatus::Accepted,
                    progress: 0,
                    max_progress: 1,
                    level_required: level_req,
                    rewards: QuestRewards {
                        experience: reward_exp,
                        gold: reward_gold,
                        items: Vec::new(),
                    },
                    group: real_group,
                });
            }
            UiCommand::QuestCompleted { quest_id } => {
                sys.main_dialog.quest_log_dialog_mut().notify_quest_complete(quest_id);
                sys.main_dialog.quest_log_dialog_mut().remove_quest(quest_id);
            }
            UiCommand::QuestProgressUpdated { quest_id, progress_text } => {
                sys.main_dialog.quest_log_dialog_mut().update_quest_progress_from_text(quest_id, progress_text.as_str());
            }
            UiCommand::QuestInfoReceived {
                quest_id, name, group, description, level_req, reward_exp, reward_gold,
            } => {
                // 缓存任务信息，等 QuestAccepted 到来时使用真实数据
                sys.cached_quest_info.insert(
                    quest_id,
                    (name.clone(), group.clone(), description.clone(), level_req, reward_exp, reward_gold),
                );
            }
            // 公会扩展事件
            UiCommand::GuildMemberUpdated { name, rank, online } => {
                sys.main_dialog.guild_dialog_mut().update_member(name.clone(), rank.clone(), online);
            }
            UiCommand::GuildNoticeUpdated { notice } => {
                sys.main_dialog.guild_dialog_mut().update_notice(notice.clone());
            }
            UiCommand::GuildExpGained { amount } => {
                sys.main_dialog.push_system_chat_line(format!("行会经验 +{}", amount));
            }
            UiCommand::GuildWarRequested { guild_name } => {
                sys.main_dialog.push_system_chat_line(format!("行会战请求！「{}」向你方宣战", guild_name));
            }
            UiCommand::SetGuildName { name } => {
                sys.main_dialog.guild_dialog_mut().update_guild_info(crate::scenes::dialogs::game::guild_dialog::GuildInfo {
                    name: name.clone(),
                    ..Default::default()
                });
                tracing::debug!("🏰 行会名称: {}", name);
            }
            UiCommand::UpdateGuildStatus { rank_name, level, experience, max_experience, gold, spare_points, member_count, max_members, my_rank_id } => {
                let dialog = sys.main_dialog.guild_dialog_mut();
                dialog.update_guild_info(crate::scenes::dialogs::game::guild_dialog::GuildInfo {
                    rank_name: rank_name.clone(),
                    level,
                    experience,
                    max_experience,
                    gold,
                    spare_points,
                    member_count: member_count as u32,
                    max_members: max_members as u32,
                    my_rank_id,
                    ..Default::default()
                });
            }
            UiCommand::UpdateGuildStorageGold { gold } => {
                sys.main_dialog.guild_dialog_mut().update_storage_gold(gold);
            }
            UiCommand::UpdateGuildStorageItems { items } => {
                for item in items {
                    sys.main_dialog.guild_dialog_mut().update_storage_item(item.name.clone(), item.quantity, item.slot);
                }
            }
            UiCommand::UpdateGuildStorageItem { slot, name, quantity } => {
                sys.main_dialog.guild_dialog_mut().update_storage_item(name, quantity, slot);
            }
            UiCommand::ClearGuildStorageItems => {
                sys.main_dialog.guild_dialog_mut().clear_storage_items();
            }
            UiCommand::UpdateGuildBuffs { buff_ids } => {
                tracing::debug!("🏛️ Guild buffs updated: {:?}", buff_ids);
            }
            UiCommand::OpenMailDialog => {
                tracing::debug!("📮 打开邮件对话框");
                sys.main_dialog.mail_dialog_mut().open();
            }
            UiCommand::CloseMailDialog => {
                sys.main_dialog.mail_dialog_mut().close();
            }
            UiCommand::UpdateMailList { mails } => {
                // 同步到对话框和 UiStateData
                sys.main_dialog.mail_dialog_mut().set_mails(mails.clone());
                if let Some(s) = ctx.world.query::<&UiState>().iter().next() {
                    s.borrow_mut().mail_entries = mails.clone();
                }
            }
            UiCommand::OpenBigMap => {
                tracing::debug!("🗺️ 打开大地图");
                sys.main_dialog.big_map_dialog_mut().show();
            }
            UiCommand::OpenStorage => {
                tracing::debug!("📦 打开仓库对话框");
                // 查询本地背包物品传入，实现双面板布局
                use crate::components::{Inventory, LocalPlayer};
                let inventory_items: Vec<mir2_shared::data::item::UserItem> = ctx
                    .world
                    .iter()
                    .find_map(|e| {
                        let _lp = e.get::<&LocalPlayer>()?;
                        let inv = e.get::<&Inventory>()?;
                        Some(inv.items.iter().filter_map(|s: &Option<mir2_shared::data::item::UserItem>| s.clone()).collect())
                    })
                    .unwrap_or_default();
                sys.npc_goods_dialog.show_storage_mode(vec![], inventory_items, 1.0);
                sys.bring_npc_layer_to_front(NpcUiLayer::Goods);
            }
            UiCommand::UpdateStorageItems { items } => {
                sys.npc_goods_dialog.update_storage_items(items);
            }
            UiCommand::UpdateStorageInventoryItems { items } => {
                sys.npc_goods_dialog.update_storage_inventory_items(items);
            }
            UiCommand::SetMarriageRequester { requester } => {
                sys.main_dialog.relationship_dialog_mut().set_marriage_requester(requester.clone());
            }
            UiCommand::ClearMarriageRequester => {
                sys.main_dialog.relationship_dialog_mut().clear_marriage_requester();
            }
            UiCommand::UpdateLover { name, date } => {
                sys.main_dialog.relationship_dialog_mut().set_lover_info(name, date);
            }
            UiCommand::UpdateMentor { name, level, online } => {
                sys.main_dialog.relationship_dialog_mut().set_mentor_info(name, level as u32, online);
            }
            UiCommand::ShowTextInput { kind, title, placeholder, max_length } => {
                sys.main_dialog.set_pending_text_input_kind(kind);
                sys.main_dialog.text_input_dialog_mut().show(&title, &placeholder, max_length);
            }
            UiCommand::HideTextInput => {
                sys.main_dialog.reset_pending_text_input_kind();
                sys.main_dialog.text_input_dialog_mut().hide();
            }
            UiCommand::ShowInviteConfirm { kind, inviter, detail } => {
                sys.pending_invite = Some((kind, inviter.clone(), detail.clone()));
            }
            UiCommand::HideInviteConfirm => {
                sys.pending_invite = None;
            }
            UiCommand::UpdateRankings { tab, entries } => {
                tracing::debug!("🏆 更新排行榜: tab={}, {} entries", tab, entries.len());
                let ranking_dialog = sys.main_dialog.ranking_dialog_mut();
                let tab_enum = crate::scenes::dialogs::game::RankingTab::from_index(tab as usize);
                let mapped: Vec<_> = entries.iter().map(|(rank, name, value)| {
                    crate::scenes::dialogs::game::RankingEntry {
                        rank: *rank,
                        name: name.clone(),
                        value: value.clone(),
                    }
                }).collect();
                ranking_dialog.set_rankings(tab_enum, mapped);
            }
            UiCommand::UpdateGameShopItems { items, credit, gold } => {
                tracing::debug!("🛒 更新商城商品列表: {} items", items.len());
                sys.main_dialog.game_shop_dialog_mut().update_from_server(items, credit, gold);
            }
            UiCommand::UpdateGameShopStock { item_index, stock } => {
                sys.main_dialog.game_shop_dialog_mut().update_stock(item_index, stock);
            }
            UiCommand::UpdateAttackMode { mode } => {
                sys.main_dialog.set_attack_mode(mode);
            }
            UiCommand::UpdatePetMode { mode } => {
                sys.main_dialog.set_pet_mode(mode);
            }
            UiCommand::SetTimer { timer_id, seconds } => {
                sys.timer_dialog.set_timer(timer_id, seconds);
            }
            UiCommand::TimerExpired { timer_id } => {
                sys.timer_dialog.remove_timer(timer_id);
            }
            UiCommand::PushChatNotice { text } => {
                sys.chat_notice_dialog.push_notice_default(text);
            }
            UiCommand::ShowNotice { text } => {
                sys.notice_dialog.set_notice(text);
            }
            UiCommand::CloseNotice => {
                sys.notice_dialog.close();
            }
            UiCommand::ShowRollResult { value } => {
                sys.roll_dialog.show_roll(value);
            }
            UiCommand::UpdateDuraStatus { items } => {
                sys.dura_status_dialog.update_dura(items);
            }
            UiCommand::ToggleDuraStatus => {
                sys.dura_status_dialog.toggle();
            }
            UiCommand::ShowNPCDrop { npc_name, items } => {
                sys.npc_drop_dialog.show(npc_name, items);
            }
            UiCommand::ShowGuildTerritory => {
                sys.guild_territory_dialog.show();
            }
            UiCommand::UpdateGuildTerritory { entries, page, total } => {
                sys.guild_territory_dialog.update_territories(entries, page, total);
            }
            UiCommand::ToggleKeyboardLayout => {
                sys.keyboard_layout_dialog.toggle();
            }
            UiCommand::ShowNPCAwake { item_name, materials } => {
                sys.npc_awake_dialog.show(item_name, materials);
            }
            UiCommand::SetAwakeLocked { locked } => {
                sys.npc_awake_dialog.set_locked(locked);
            }
            UiCommand::ShowCraft { recipes } => {
                sys.craft_dialog.show(recipes);
            }
            UiCommand::ShowRefine { item_name, stats, material_name, material_have, material_need } => {
                sys.refine_dialog.show(item_name, stats, material_name, material_have, material_need);
            }
            UiCommand::OpenItemRental { partner } => {
                sys.item_rental_dialog.show(partner);
            }
            UiCommand::UpdateRentalFee { fee } => {
                sys.item_rental_dialog.update_fee(fee);
            }
            UiCommand::UpdateRentalPeriod { period } => {
                sys.item_rental_dialog.update_period(period);
            }
            UiCommand::SetRentalLocked { locked } => {
                sys.item_rental_dialog.set_locked(locked);
            }
            UiCommand::SetRentalPartnerLocked { locked } => {
                sys.item_rental_dialog.set_partner_locked(locked);
            }
            UiCommand::CloseItemRental => {
                sys.item_rental_dialog.close();
            }
            UiCommand::UpdateRentalItemList { items } => {
                tracing::debug!("📋 Rental item list updated: {} items", items.len());
            }
            UiCommand::OpenTrustMerchant => {
                sys.trust_merchant_dialog.show();
            }
            UiCommand::UpdateMerchantItems { items, page, total } => {
                sys.trust_merchant_dialog.update_items(items, page, total);
            }
            UiCommand::CloseTrustMerchant => {
                sys.trust_merchant_dialog.close();
            }
            UiCommand::CraftItemRequest { recipe_unique_id, count, slots } => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetworkEvent::CraftItemRequest { recipe_unique_id, count, slots });
                }
            }
            UiCommand::ConfirmItemRental => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetworkEvent::ItemRentalConfirm);
                }
            }
            UiCommand::RequestSceneTransition { target } => {
                UiState::with_in_world(&ctx.world, |s| s.request_scene_transition = Some(target));
            }
            UiCommand::HeroLevelUp { new_level } => {
                sys.main_dialog.hero_dialog_mut().update_hero_level(new_level);
            }
            UiCommand::UpdateHeroManageList { heroes } => {
                sys.main_dialog.hero_dialog_mut().update_manage_list(heroes);
            }
            UiCommand::HeroInfoReceived { hero_id } => {
                sys.main_dialog.hero_dialog_mut().set_hero_id(hero_id);
            }
            UiCommand::ItemDuraChanged { unique_id, current_dura } => {
                sys.dura_status_dialog.update_item_dura(unique_id, current_dura);
            }
            UiCommand::RemoveDuraEntry { unique_id } => {
                sys.dura_status_dialog.remove_dura_entry(unique_id);
            }
            UiCommand::SetInventorySize { size } => {
                UiState::with_in_world(&ctx.world, |s| s.inventory_size = size);
            }
            UiCommand::SetStorageSize { size } => {
                UiState::with_in_world(&ctx.world, |s| s.storage_size = size);
            }
            UiCommand::SetTimeOfDay { time } => {
                UiState::with_in_world(&ctx.world, |s| s.time_of_day = time);
            }
            UiCommand::SetBindingShot { enabled } => {
                UiState::with_in_world(&ctx.world, |s| s.binding_shot_enabled = enabled);
            }
            UiCommand::SetConcentration { enabled } => {
                UiState::with_in_world(&ctx.world, |s| s.concentration_enabled = enabled);
            }
            UiCommand::SetElement { element } => {
                UiState::with_in_world(&ctx.world, |s| s.element_type = element);
            }
            UiCommand::SetObserveAllowed { allowed } => {
                UiState::with_in_world(&ctx.world, |s| s.observe_allowed = allowed);
            }
            UiCommand::SetHeroBaseStats { stats } => {
                sys.main_dialog.hero_dialog_mut().set_base_stats(stats);
            }
            UiCommand::UpdateBigMapInfo { map_index: _, title, width, height } => {
                sys.main_dialog.big_map_dialog_mut().set_map_info(title.clone(), width as f32, height as f32);
            }
            UiCommand::UpdateWorldMapIcons { icons } => {
                tracing::debug!("🌍 世界地图图标更新: {} icons", icons.len());
            }
            UiCommand::NavigateToMapLocation { map_index: _, x, y } => {
                sys.main_dialog.big_map_dialog_mut().set_player_position(x as f32, y as f32);
                sys.main_dialog.big_map_dialog_mut().show();
            }
            UiCommand::MagicLearned { spell, name, level, icon, hero } => {
                sys.main_dialog.skill_dialog_mut(hero).learn_skill(spell, name, level, icon);
            }
            UiCommand::MagicLeveledUp { spell, level, hero } => {
                sys.main_dialog.skill_dialog_mut(hero).level_up_skill(spell, level);
            }
            UiCommand::MagicRemoved { spell, hero } => {
                sys.main_dialog.skill_dialog_mut(hero).remove_skill(spell);
            }
            UiCommand::SpellToggled { spell, can_use, hero } => {
                sys.main_dialog.skill_dialog_mut(hero).toggle_skill(spell, can_use);
            }
            UiCommand::ExperienceGained { amount } => {
                sys.main_dialog.push_system_chat_line(format!("+{} 经验", amount));
            }
            UiCommand::HeroExperienceGained { amount } => {
                sys.main_dialog.push_system_chat_line(format!("英雄 +{} 经验", amount));
            }
            UiCommand::SetTransformForm { form } => {
                UiState::with_in_world(&ctx.world, |s| s.transform_form = form);
            }
            UiCommand::TriggerMapEffect { effect } => {
                UiState::with_in_world(&ctx.world, |s| s.pending_map_effect = effect);
            }
            UiCommand::SetBaseStats { stats } => {
                if stats.len() >= 10 {
                    let cd = sys.main_dialog.character_dialog_mut();
                    cd.stats.ac = (stats[0] as u32, stats[1] as u32);
                    cd.stats.mac = (stats[2] as u32, stats[3] as u32);
                    cd.stats.dc = (stats[4] as u32, stats[5] as u32);
                    cd.stats.mc = (stats[6] as u32, stats[7] as u32);
                    cd.stats.sc = (stats[8] as u32, stats[9] as u32);
                }
            }
            UiCommand::SetCreatureCanRename { can_rename } => {
                sys.main_dialog.intelligent_creature_dialog_mut().set_can_rename(can_rename);
            }
            UiCommand::SetCreatureAutoPickup { enabled } => {
                sys.main_dialog.intelligent_creature_dialog_mut().set_auto_pickup(enabled);
            }
            UiCommand::OpenDoor { door_id } => {
                UiState::with_in_world(&ctx.world, |s| { s.open_doors.insert(door_id); });
            }
        }
    }

    // 1.5) 检查 ChatDialog 的待发送消息，转发为网络事件
    if let Some(message) = crate::scenes::dialogs::game::chat_dialog::take_pending_chat_message() {
        if let Some(net) = ctx.net.as_ref() {
            let _ = net.send(NetworkEvent::ChatRequest {
                message,
                linked_items: Vec::new(),
            });
        }
    }

    // 2) 同步表现层数据 -> 具体 UI（小地图）
    let (minimap_world_size, minimap_player_pos, minimap_player_dir_radians) = {
        ctx.world
            .query::<&UiState>()
            .iter()
            .next()
            .map(|s| {
                let s = s.borrow();
                (
                    s.minimap_world_size,
                    s.minimap_player_pos,
                    s.minimap_player_dir_radians,
                )
            })
            .unwrap_or((None, None, 0.0))
    };
    if let Some(ws) = minimap_world_size {
        sys.main_dialog.set_minimap_world_size(ws.x, ws.y);
    }
    if let Some(p) = minimap_player_pos {
        sys.main_dialog
            .update_minimap_player_position(p.x, p.y, minimap_player_dir_radians);
    }

    // 2.4a) 同步大地图数据
    {
        let (map_name, world_size, player_pos) = {
            if let Some(s) = ctx.world.query::<&UiState>().iter().next() {
                let s = s.borrow();
                (s.big_map_map_name.clone(), s.minimap_world_size, s.minimap_player_pos)
            } else {
                (None, None, None)
            }
        };

        if let Some(name) = map_name {
            let ws = world_size.unwrap_or(macroquad::prelude::Vec2::ZERO);
            sys.main_dialog.big_map_dialog_mut().set_map_info(name, ws.x, ws.y);
        }
        if let Some(p) = player_pos {
            sys.main_dialog.big_map_dialog_mut().set_player_position(p.x, p.y);
        }

        // 同步地图瓦片数据（仅当地图尺寸变化时才克隆）
        if let Some(map_data) = ctx.world.query::<&MapData>().iter().next() {
            let (bm_w, bm_h) = {
                let big_map = sys.main_dialog.big_map_dialog();
                (big_map.map_width(), big_map.map_height())
            };
            if bm_w != map_data.width || bm_h != map_data.height {
                sys.main_dialog.big_map_dialog_mut().set_map_data(map_data.cells.clone(), map_data.width, map_data.height);
            }
        }
    }

    // 2.4b) 消费小地图待处理动作
    {
        use crate::scenes::dialogs::game::minimap_dialog::MiniMapAction;
        let action = sys.main_dialog.minimap_dialog_mut().take_pending_actions();
        match action {
            MiniMapAction::OpenMail => {
                tracing::debug!("📮 小地图：打开邮件对话框");
                if let Some(s) = ctx.world.query::<&UiState>().iter().next() {
                    s.borrow_mut().pending_commands.push(UiCommand::OpenMailDialog);
                }
            }
            MiniMapAction::OpenBigMap => {
                tracing::debug!("🗺️ 小地图：打开大地图");
                if let Some(s) = ctx.world.query::<&UiState>().iter().next() {
                    s.borrow_mut().pending_commands.push(UiCommand::OpenBigMap);
                }
            }
            MiniMapAction::None => {}
        }
    }

    // 2.5) 同步主面板红/蓝血（HP/MP）到真实 ECS 数据
    {
        use crate::components::{Health, LocalPlayer, Mana};

        if let Some((e, _lp, hp)) = ctx.world.iter().find_map(|e| {
            let lp = e.get::<&LocalPlayer>()?;
            let hp = e.get::<&Health>()?;
            Some((e.entity(), lp, hp))
        }) {
            let (mp_cur, mp_max) = ctx
                .world
                .get::<&Mana>(e)
                .map(|mp| (mp.current, mp.max))
                .unwrap_or((0, 1));

            sys.main_dialog
                .set_vitals(hp.current, hp.max, mp_cur, mp_max);
        }
    }

    // 2.6) 同步玩家属性（等级/金币/经验/负重/背包）
    {
        use crate::components::{CombatStats, Currency, Experience, Inventory, LocalPlayer};

        if let Some((e, _lp, stats)) = ctx.world.iter().find_map(|e| {
            let lp = e.get::<&LocalPlayer>()?;
            let stats = e.get::<&CombatStats>()?;
            Some((e.entity(), lp, stats))
        }) {
            let exp = ctx.world.get::<&Experience>(e).ok().map(|e| e.percent()).unwrap_or(0.0);
            let currency = ctx.world.get::<&Currency>(e)
                .map(|c| c.gold)
                .unwrap_or(0);
            let (weight, max_weight, bag_space, bag_capacity) = ctx.world.get::<&Inventory>(e)
                .map(|inv| (
                    inv.current_weight,
                    inv.max_weight,
                    inv.items.iter().filter(|s| s.is_some()).count() as u32,
                    inv.items.len() as u32,
                ))
                .unwrap_or((0, 100, 0, 40));

            sys.main_dialog.set_player_stats(
                stats.level,
                currency,
                exp,
                weight,
                max_weight,
                bag_space,
                bag_capacity,
                None, // character name 从 ECS 暂不可用
            );

            // 同步背包 InventoryDialog
            if let Ok(inv) = ctx.world.get::<&Inventory>(e) {
                sys.main_dialog.sync_inventory(&inv, currency);
            }
        }
    }

    // 3) 快捷键（由 UIRenderSystem 统一处理，避免 GameScene 直连 UI 组件）
    if !sys.amount_box.is_visible() && !sys.main_dialog.is_any_input_active() {
        if is_key_pressed(KeyCode::Enter) {
            sys.main_dialog.activate_chat_input();
        }
        if is_key_pressed(KeyCode::M) {
            sys.main_dialog.toggle_minimap();
        }
        if is_key_pressed(KeyCode::Tab) {
            sys.main_dialog.toggle_minimap_size();
        }
    }

    // ESC：关闭弹窗优先（AmountBox/NPCGoods/SubGoods/Popups）
    if is_key_pressed(KeyCode::Escape) {
        if sys.amount_box.is_visible() {
            sys.amount_box.hide();
            if let Some(s) = ctx.world.query::<&UiState>().iter().next() {
                s.borrow_mut().amount_box_buy_uid = None;
            }
        } else if sys.npc_sub_goods_dialog.is_visible() {
            sys.npc_sub_goods_dialog.hide();
        } else if sys.npc_goods_dialog.is_visible() {
            sys.npc_goods_dialog.hide();
        } else if sys.main_dialog.any_popup_open() {
            sys.main_dialog.close_all_popups();
        }
    }

    // 4) 输入阻塞：UI 命中/捕获时阻止世界输入（写入 UiWorldInputBlock + ctx.input_blocked）
    let (mx, my) = mouse_position();
    let mouse_pos = vec2(mx, my);
    let left_pressed = is_mouse_button_pressed(MouseButton::Left);
    let right_pressed = is_mouse_button_pressed(MouseButton::Right);
    let left_down = is_mouse_button_down(MouseButton::Left);
    let right_down = is_mouse_button_down(MouseButton::Right);
    let mouse_button_down = left_down || right_down;
    let wheel_y = mouse_wheel().1;

    let ui_over = sys.main_dialog.is_mouse_over_ui(mouse_pos)
        || sys.npc_goods_dialog.is_mouse_over(mouse_pos)
        || sys.npc_dialog.is_mouse_over(mouse_pos)
        || sys.npc_sub_goods_dialog.is_mouse_over(mouse_pos)
        || sys.amount_box.is_mouse_over(mouse_pos);

    // UI 鼠标捕获：在 UI 上按下鼠标后，直到松开都阻止 ECS 读取输入。
    let mut ui_mouse_captured = ctx
        .world
        .query::<&UiState>()
        .iter()
        .next()
        .map(|s| s.borrow().ui_mouse_captured)
        .unwrap_or(false);
    if (left_pressed || right_pressed) && ui_over {
        ui_mouse_captured = true;
    }
    if ui_mouse_captured && !mouse_button_down {
        ui_mouse_captured = false;
    }
    if let Some(s) = ctx.world.query::<&UiState>().iter().next() {
        s.borrow_mut().ui_mouse_captured = ui_mouse_captured;
    }

    // 写入 ECS 单例：UiWorldInputBlock（与 UiState 同挂在 RenderPass 实体上）
    if let Some((pass_entity, _)) = ctx.world.iter().find_map(|e| e.get::<&UiState>().map(|ui| (e.entity(), ui))) {
        if let Ok(mut block) = ctx.world.get::<&mut UiWorldInputBlock>(pass_entity) {
            block.mouse_over_ui = ui_over;
            block.mouse_captured = ui_mouse_captured;
        }
    }

    let ui_input_active = sys.main_dialog.is_any_input_active();
    let amount_box_visible = sys.amount_box.is_visible();
    let ui_consumed_last_frame = ctx
        .world
        .query::<&UiState>()
        .iter()
        .next()
        .map(|s| s.borrow().ui_consumed_last_frame)
        .unwrap_or(false);

    ctx.input_blocked = ui_input_active
        || ui_mouse_captured
        || (wheel_y != 0.0 && ui_over)
        || ui_consumed_last_frame
        || amount_box_visible;

    // 5) 更新 UiState 的可观察标记（供 GameScene 做 ESC 退出 gating 等）
    if let Some(s) = ctx.world.query::<&UiState>().iter().next() {
        let mut s = s.borrow_mut();
        s.ui_input_active = ui_input_active;
        s.any_modal_or_popup_open = sys.main_dialog.any_popup_open()
            || sys.npc_dialog.is_visible()
            || sys.npc_goods_dialog.is_visible()
            || sys.npc_sub_goods_dialog.is_visible()
            || sys.amount_box.is_visible();
    }

    // Scene 退出 gating：避免 Scene 直接读取 UiState 内部结构。
    if let Some((pass_entity, _)) = ctx.world.iter().find_map(|e| e.get::<&UiState>().map(|ui| (e.entity(), ui))) {
        let any_modal_or_popup_open = ctx
            .world
            .get::<&UiState>(pass_entity)
            .map(|s| s.borrow().any_modal_or_popup_open)
            .unwrap_or(false);
        if let Ok(mut b) = ctx.world.get::<&mut SceneExitBlock>(pass_entity) {
            b.block_escape_exit = any_modal_or_popup_open;
        }
    }

    // 组队对话框动作（本地切换 + 网络发包）
    {
        use crate::network::handlers::NetworkEvent as NetEv;
        use crate::scenes::dialogs::game::group_dialog::GroupDialogAction;
        let gd = sys.main_dialog.group_dialog_mut();
        let action = gd.take_action();
        match action {
            GroupDialogAction::AllowJoinToggle => {
                if gd.is_leader() {
                    let current = gd.allow_join();
                    gd.set_allow_join(!current);
                }
            }
            GroupDialogAction::Invite => {
                if let Some(s) = ctx.world.query::<&UiState>().iter().next() {
                    use crate::scenes::dialogs::game::main_dialog::TextInputKind;
                    s.borrow_mut().pending_commands.push(UiCommand::ShowTextInput {
                        kind: TextInputKind::GroupInvite,
                        title: "组队邀请".to_string(),
                        placeholder: "输入玩家名称".to_string(),
                        max_length: 32,
                    });
                }
            }
            GroupDialogAction::Leave => {
                let player_name = gd.get_local_player_name();
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::GroupLeaveRequest { player_name });
                }
            }
            GroupDialogAction::KickSelected => {
                if let Some(name) = gd.get_selected_member_name() {
                    if let Some(net) = ctx.net.as_ref() {
                        let _ = net.send(NetEv::GroupKickRequest { player_name: name });
                    }
                }
            }
            GroupDialogAction::ViewMemberDetail { name, hp_percent, is_leader } => {
                let hp_display = (hp_percent * 100.0).max(0.0) as u32;
                let role_tag = if is_leader { "队长" } else { "队员" };
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::SystemMessage {
                        message: format!("{} - {} | HP: {}%", name, role_tag, hp_display),
                    });
                }
            }
            GroupDialogAction::None => {}
        }
    }

    // 好友对话框动作
    {
        use crate::network::handlers::NetworkEvent as NetEv;
        use crate::scenes::dialogs::game::friend_dialog::FriendDialogAction;
        let fd = sys.main_dialog.friend_dialog_mut();
        let action = fd.take_action();
        match action {
            FriendDialogAction::AddFriend => {
                if let Some(s) = ctx.world.query::<&UiState>().iter().next() {
                    use crate::scenes::dialogs::game::main_dialog::TextInputKind;
                    s.borrow_mut().pending_commands.push(UiCommand::ShowTextInput {
                        kind: TextInputKind::AddFriend,
                        title: "添加好友".to_string(),
                        placeholder: "输入玩家名称".to_string(),
                        max_length: 32,
                    });
                }
            }
            FriendDialogAction::RemoveSelected => {
                if let Some(object_id) = fd.get_selected_friend_object_id() {
                    if let Some(net) = ctx.net.as_ref() {
                        let _ = net.send(NetEv::RemoveFriendRequest { object_id });
                    }
                }
            }
            FriendDialogAction::RefreshList => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::RefreshFriendsRequest);
                }
            }
            FriendDialogAction::PrivateChatSelected => {
                if let Some(name) = fd.get_selected_friend_name() {
                    use crate::scenes::dialogs::game::main_dialog::TextInputKind;
                    if let Some(s) = ctx.world.query::<&UiState>().iter().next() {
                        s.borrow_mut().pending_commands.push(UiCommand::ShowTextInput {
                            kind: TextInputKind::WhisperChat { target: name.clone() },
                            title: format!("私聊 - {}", name),
                            placeholder: "输入消息".to_string(),
                            max_length: 256,
                        });
                    }
                }
            }
            FriendDialogAction::None => {}
        }
    }

    // 行会对话框动作
    {
        use crate::network::handlers::NetworkEvent as NetEv;
        use crate::scenes::dialogs::game::guild_dialog::GuildDialogAction;
        let gld = sys.main_dialog.guild_dialog_mut();
        let action = gld.take_action();
        match action {
            GuildDialogAction::LeaveGuild => {
                let player_name = sys.main_dialog.character_name().to_string();
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::GuildLeaveRequest { player_name });
                }
            }
            GuildDialogAction::RequestGuildInfo => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::RequestGuildInfo);
                }
            }
            GuildDialogAction::EditNotice(_) => {
                if let Some(s) = ctx.world.query::<&UiState>().iter().next() {
                    use crate::scenes::dialogs::game::main_dialog::TextInputKind;
                    s.borrow_mut().pending_commands.push(UiCommand::ShowTextInput {
                        kind: TextInputKind::GuildNotice,
                        title: "编辑行会公告".to_string(),
                        placeholder: "输入行会公告内容".to_string(),
                        max_length: 200,
                    });
                }
            }
            GuildDialogAction::EditMemberRank { ref name, ref rank } => {
                if name.is_empty() {
                    tracing::info!("👤 管理行会成员待实现（需双击成员弹出输入框）");
                } else if let Ok(rank_num) = rank.parse::<u8>() {
                    if let Some(net) = ctx.net.as_ref() {
                        let _ = net.send(NetEv::EditGuildMember { member_name: name.clone(), rank: rank_num });
                    }
                } else {
                    tracing::warn!("⚠️ 行会成员 rank 不是有效数字: {}", rank);
                }
            }
            GuildDialogAction::ViewMemberDetail { ref name, ref rank, ref online } => {
                let status = if *online { "在线" } else { "离线" };
                sys.main_dialog.push_system_chat_line(format!("行会成员: {} | 职位: {} | 状态: {}", name, rank, status));
            }
            GuildDialogAction::RequestGuildWar => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::GuildWarReturn);
                }
            }
            GuildDialogAction::None => {}
        }
    }

    // 师徒对话框动作
    {
        use crate::network::handlers::NetworkEvent as NetEv;
        use crate::scenes::dialogs::game::mentor_dialog::MentorDialogAction;
        let md = sys.main_dialog.mentor_dialog_mut();
        let action = md.take_action();
        match action {
            MentorDialogAction::AddMentor => {
                if let Some(s) = ctx.world.query::<&UiState>().iter().next() {
                    use crate::scenes::dialogs::game::main_dialog::TextInputKind;
                    s.borrow_mut().pending_commands.push(UiCommand::ShowTextInput {
                        kind: TextInputKind::AddMentor,
                        title: "拜师".to_string(),
                        placeholder: "输入师傅名称".to_string(),
                        max_length: 32,
                    });
                }
            }
            MentorDialogAction::CancelMentor => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::CancelMentorRequest);
                }
            }
            MentorDialogAction::ToggleAllowRequest => {
                md.set_allow_request(!md.allow_request());
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::AllowMentorRequest { enabled: md.allow_request() });
                }
            }
            MentorDialogAction::AcceptMentor => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::MentorReply { accept: true });
                }
            }
            MentorDialogAction::DeclineMentor => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::MentorReply { accept: false });
                }
            }
            MentorDialogAction::None => {}
        }
    }

    // 邮件对话框动作
    {
        use crate::network::handlers::NetworkEvent as NetEv;
        use crate::scenes::dialogs::game::mail_dialog::MailDialogAction;
        let mail_action = sys.main_dialog.mail_dialog_mut().take_action();
        match mail_action {
            MailDialogAction::ReadMail { mail_id } => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::ReadMailRequest { mail_id });
                }
            }
            MailDialogAction::CollectParcel { mail_id } => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::CollectParcelRequest { mail_id });
                }
            }
            MailDialogAction::DeleteMail { mail_id } => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::DeleteMailRequest { mail_id });
                }
            }
            MailDialogAction::SendMail { .. } => {
                tracing::info!("📬 写信功能需要文本输入支持");
            }
            MailDialogAction::None => {}
        }
    }

    // 婚姻对话框动作
    {
        use crate::network::handlers::NetworkEvent as NetEv;
        use crate::scenes::dialogs::game::relationship_dialog::RelationshipDialogAction;
        let rd = sys.main_dialog.relationship_dialog_mut();
        let action = rd.take_action();
        match action {
            RelationshipDialogAction::RequestDivorce => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::DivorceRequestSend);
                }
                tracing::debug!("💔 发送离婚请求");
            }
            RelationshipDialogAction::RequestMarriage => {
                if let Some(net) = ctx.net.as_ref() {
                    // 求婚目标由服务器根据亲密度/位置等逻辑判定
                    let _ = net.send(NetEv::MarriageRequestSend { target: String::new() });
                }
                tracing::debug!("💍 发送求婚请求");
            }
            RelationshipDialogAction::AcceptMarriage => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::MarriageReply { accept: true });
                }
                tracing::debug!("💍 接受求婚");
            }
            RelationshipDialogAction::DeclineMarriage => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::MarriageReply { accept: false });
                }
                tracing::debug!("💍 拒绝求婚");
            }
            RelationshipDialogAction::None => {}
        }
    }

    // 坐骑对话框动作
    {
        use crate::network::handlers::NetworkEvent as NetEv;
        use crate::scenes::dialogs::game::mount_dialog::MountDialogAction;
        let md = sys.main_dialog.mount_dialog_mut();
        let action = md.take_action();
        match action {
            MountDialogAction::Ride => {
                if let Some(mount_type) = md.get_selected_mount_type() {
                    if let Some(net) = ctx.net.as_ref() {
                        let _ = net.send(NetEv::MountRideRequest { mount_type });
                    }
                }
            }
            MountDialogAction::Dismount => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::MountDismountRequest);
                }
            }
            MountDialogAction::SelectMount(idx) => {
                tracing::debug!("Mount selected: index={}", idx);
            }
            MountDialogAction::None => {}
        }
    }

    // 英雄对话框动作
    {
        use crate::network::handlers::NetworkEvent as NetEv;
        use crate::scenes::dialogs::game::hero_dialog::HeroDialogAction;
        let hd = sys.main_dialog.hero_dialog_mut();
        let action = hd.take_action();
        match action {
            HeroDialogAction::SetBehaviour(behaviour) => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::SetHeroBehaviourRequest { behaviour: behaviour as u8 });
                }
            }
            HeroDialogAction::ChangeHero(hero_index) => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::ChangeHeroRequest { hero_index });
                }
            }
            HeroDialogAction::SetAutoHpPot { value } => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::SetHeroAutoPotValue { pot_type: 0, value });
                }
            }
            HeroDialogAction::SetAutoMpPot { value } => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::SetHeroAutoPotValue { pot_type: 1, value });
                }
            }
            HeroDialogAction::None => {}
        }
    }

    // 钓鱼对话框动作
    {
        use crate::network::handlers::NetworkEvent as NetEv;
        let fd = sys.main_dialog.fishing_dialog_mut();
        if let Some(enabled) = fd.take_pending_autocast() {
            if let Some(net) = ctx.net.as_ref() {
                let _ = net.send(NetEv::FishingAutocastToggle { enabled });
            }
        }
    }

    // 智能宠物对话框动作
    {
        use crate::network::handlers::NetworkEvent as NetEv;
        use crate::scenes::dialogs::game::intelligent_creature_dialog::CreatureDialogAction;
        let cd = sys.main_dialog.intelligent_creature_dialog_mut();
        let action = cd.take_action();
        match action {
            CreatureDialogAction::SummonCreature(idx) => {
                tracing::debug!("Summon creature: index={}", idx);
            }
            CreatureDialogAction::DismissCreature => {
                tracing::debug!("Dismiss creature");
            }
            CreatureDialogAction::ReleaseCreature(idx) => {
                tracing::debug!("Release creature: index={}", idx);
            }
            CreatureDialogAction::ToggleMode => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::UpdateIntelligentCreatureRequest);
                }
            }
            CreatureDialogAction::OpenOptions => {
                tracing::debug!("Open creature options (not implemented)");
            }
            CreatureDialogAction::None => {}
        }
    }

    // 宝石镶嵌对话框动作
    {
        use crate::network::handlers::NetworkEvent as NetEv;
        use crate::scenes::dialogs::game::socket_dialog::SocketAction;
        use mir2_shared::enums::AwakeType;
        let sd = sys.main_dialog.socket_dialog_mut();
        let action = sd.take_action();
        match action {
            SocketAction::InsertGem { item_unique_id, position_idx, awake_type } => {
                // AwakeType 由用户通过嵌入式 gem picker 选择(见 SocketDialogHybrid::draw_gem_picker)
                // 服务端需要的 op 协议: `Awakening { unique_id, awake_type, position_idx }`
                // (PR #1126 之前是 `Awakening` 单个位置选择;现在带 awake_type)
                if awake_type == AwakeType::None {
                    tracing::warn!("💎 InsertGem refused: AwakeType::None is not selectable");
                } else if let Some(net) = ctx.net.as_ref() {
                    let pos_u32: u32 = position_idx.try_into().unwrap_or(0);
                    let _ = net.send(NetEv::AwakeningRequest {
                        unique_id: item_unique_id,
                        awake_type,
                        position_idx: pos_u32,
                    });
                    tracing::info!(
                        "💎 插入宝石: uid={} pos={} awake_type={:?}",
                        item_unique_id, position_idx, awake_type
                    );
                }
            }
            SocketAction::RemoveGem { item_unique_id, position_idx } => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::DisassembleItemRequest { unique_id: item_unique_id });
                    tracing::debug!("💎 取出宝石: uid={} pos={}", item_unique_id, position_idx);
                }
            }
            SocketAction::Close => {
                // 本地关闭，无需发包
            }
            SocketAction::None => {}
        }
    }

    // 安全下线请求
    if sys.main_dialog.take_pending_logout() {
        use crate::network::handlers::NetworkEvent as NetEv;
        if let Some(net) = ctx.net.as_ref() {
            let _ = net.send(NetEv::LogOutRequest);
        }
        tracing::info!("🚪 安全下线请求已发送");
    }

    // 交易对话框动作（由 draw 阶段产出，在此发包）
    if let Some(action) = sys.pending_trade_action.take() {
        use crate::network::handlers::NetworkEvent as NetEv;
        use crate::scenes::dialogs::game::trade_dialog::TradeAction;
        match action {
            TradeAction::Confirm => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::TradeConfirmRequest { locked: true });
                }
            }
            TradeAction::Cancel => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::TradeCancelRequest);
                }
            }
            TradeAction::SetGold { amount } => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::TradeGoldRequest { amount });
                }
            }
            TradeAction::AddItem { item_index } => {
                let _ = item_index;
                tracing::info!("📦 添加物品到交易栏（需背包联动）");
            }
            TradeAction::RemoveItem { slot_index } => {
                let _ = slot_index;
                tracing::info!("📤 从交易栏移除物品");
            }
            TradeAction::None => {}
        }
    }

    // 文本输入结果（由 draw 阶段产出，在此发包）
    if let Some((kind, text)) = sys.pending_text_input.take() {
        use crate::network::handlers::NetworkEvent as NetEv;
        use crate::scenes::dialogs::game::main_dialog::TextInputKind;
        match kind {
            TextInputKind::GroupInvite => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::GroupInviteRequest { player_name: text.clone() });
                }
                tracing::debug!("👥 发送组队邀请: {}", text);
            }
            TextInputKind::AddFriend => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::AddFriendRequest { name: text.clone() });
                }
                tracing::debug!("👤 发送添加好友请求: {}", text);
            }
            TextInputKind::AddMentor => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::AddMentorRequest { name: text.clone() });
                }
                tracing::debug!("🎓 发送拜师请求: {}", text);
            }
            TextInputKind::GuildNotice => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::EditGuildNotice { notice: text.clone() });
                }
                tracing::debug!("📝 编辑行会公告: {}", text);
            }
            TextInputKind::WhisperChat { target } => {
                if let Some(net) = ctx.net.as_ref() {
                    // 服务器通过 `/目标 消息` 格式识别私聊（非专用 opcode）
                    let whisper_message = format!("/{} {}", target, text);
                    let _ = net.send(NetEv::ChatRequest { message: whisper_message.clone(), linked_items: vec![] });
                }
                tracing::debug!("💬 私聊 → {}: {}", target, text);
            }
            TextInputKind::None => {
                tracing::warn!("⚠️ 收到文本输入结果但 kind 为 None");
            }
            TextInputKind::NPCInput { npc_id } => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::NPCConfirmInput { npc_id, input: text.clone() });
                }
                tracing::debug!("📝 NPC 输入确认: npc_id={} input={}", npc_id, text);
            }
            TextInputKind::GuildName => {
                if let Some(net) = ctx.net.as_ref() {
                    let _ = net.send(NetEv::GuildNameReturn { name: text.clone() });
                }
                tracing::debug!("🏛️ 公会名称输入: {}", text);
            }
        }
    }

    // 排行榜刷新请求（由 draw 阶段产出，在此发包）
    if let Some(tab) = sys.pending_ranking_refresh_tab.take() {
        use crate::network::handlers::NetworkEvent as NetEv;
        if let Some(net) = ctx.net.as_ref() {
            let _ = net.send(NetEv::GetRankingRequest { ranking_type: tab });
        }
        tracing::debug!("🏆 请求排行榜: tab={}", tab);
    }

    // 装备物品请求（由 draw 阶段 Inventory→Character 拖拽产出，在此发包）
    if let Some(unique_id) = sys.pending_equip_request.take() {
        use crate::network::handlers::NetworkEvent as NetEv;
        if let Some(net) = ctx.net.as_ref() {
            let _ = net.send(NetEv::EquipItemRequest { unique_id });
        }
        tracing::debug!("🎒 装备物品: unique_id={}", unique_id);
    }

    // 卸下装备请求（由 draw 阶段 Character→Inventory 拖拽产出，在此发包）
    if let Some(unique_id) = sys.pending_unequip_request.take() {
        use crate::network::handlers::NetworkEvent as NetEv;
        if let Some(net) = ctx.net.as_ref() {
            let _ = net.send(NetEv::RemoveItemRequest { unique_id });
        }
        tracing::debug!("🎒 卸下装备: unique_id={}", unique_id);
    }

    // 邀请确认回复（由 draw 阶段邀请弹窗产出，在此发包）
    if let Some((kind, accept)) = sys.pending_invite_reply.take() {
        use crate::network::handlers::NetworkEvent as NetEv;
        if let Some(net) = ctx.net.as_ref() {
            match kind {
                crate::ui::ui_state::InviteKind::Group => {
                    let _ = net.send(if accept { NetEv::GroupAcceptRequest } else { NetEv::GroupDeclineRequest });
                }
                crate::ui::ui_state::InviteKind::Guild => {
                    let _ = net.send(if accept { NetEv::GuildAcceptRequest } else { NetEv::GuildDeclineRequest });
                }
                crate::ui::ui_state::InviteKind::Trade => {
                    let _ = net.send(NetEv::TradeReplyRequest { accept });
                }
                crate::ui::ui_state::InviteKind::Mentor => {
                    let _ = net.send(NetEv::MentorReply { accept });
                }
                crate::ui::ui_state::InviteKind::Divorce => {
                    let _ = net.send(NetEv::DivorceReply { accept });
                }
            }
        }
    }

    Ok(())
}
