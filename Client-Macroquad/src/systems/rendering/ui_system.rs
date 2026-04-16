use macroquad::prelude::{
    is_key_pressed, is_mouse_button_down, is_mouse_button_pressed, mouse_position, mouse_wheel,
    screen_height, screen_width, vec2, Color, KeyCode, MouseButton, WHITE,
};

use crate::components::{MapData, RenderPass, RenderStage, ResourceInitState, SceneExitBlock, UiWorldInputBlock};
use crate::game::{GameContext, GameResult};
use crate::scenes::dialogs::game::{
    amount_box::AmountBoxHybrid, npc_dialog::NpcDialogHybrid,
    npc_goods_dialog::NpcGoodsDialogHybrid, MainDialog,
};
use crate::systems::RenderSystem;
use crate::ui::text_renderer::{draw_text_cn, draw_text_with_outline, measure_text_cn};
use crate::ui::ui_state::{UiAction, UiCommand, UiState};
use crate::network::handlers::NetworkEvent;

#[derive(ecs_macros::RenderSystem)]
pub struct UIRenderSystem {
    main_dialog: MainDialog,
    npc_dialog: NpcDialogHybrid,
    npc_goods_dialog: NpcGoodsDialogHybrid,
    npc_sub_goods_dialog: NpcGoodsDialogHybrid,
    amount_box: AmountBoxHybrid,
    timer_dialog: crate::scenes::dialogs::game::timer_dialog::TimerDialogHybrid,
    chat_notice_dialog: crate::scenes::dialogs::game::chat_notice_dialog::ChatNoticeDialogHybrid,
    notice_dialog: crate::scenes::dialogs::game::notice_dialog::NoticeDialogHybrid,
    roll_dialog: crate::scenes::dialogs::game::roll_dialog::RollDialogHybrid,
    dura_status_dialog: crate::scenes::dialogs::game::dura_status_dialog::DuraStatusDialogHybrid,
    npc_drop_dialog: crate::scenes::dialogs::game::npc_drop_dialog::NPCDropDialogHybrid,
    guild_territory_dialog: crate::scenes::dialogs::game::guild_territory_dialog::GuildTerritoryDialogHybrid,
    keyboard_layout_dialog: crate::scenes::dialogs::game::keyboard_layout_dialog::KeyboardLayoutDialogHybrid,
    npc_awake_dialog: crate::scenes::dialogs::game::npc_awake_dialog::NPCAwakeDialogHybrid,
    craft_dialog: crate::scenes::dialogs::game::craft_dialog::CraftDialogHybrid,
    refine_dialog: crate::scenes::dialogs::game::refine_dialog::RefineDialogHybrid,
    item_rental_dialog: crate::scenes::dialogs::game::item_rental_dialog::ItemRentalDialogHybrid,
    trust_merchant_dialog: crate::scenes::dialogs::game::trust_merchant_dialog::TrustMerchantDialogHybrid,

    npc_z_order: Vec<NpcUiLayer>,

    ui_stack_top: UiStackTop,

    /// 暂存的交易动作（由 draw 阶段产出，由 update 阶段发包）
    pending_trade_action: Option<crate::scenes::dialogs::game::trade_dialog::TradeAction>,

    /// 暂存的文本输入结果（由 draw 阶段产出，由 update 阶段发包）
    pending_text_input: Option<(crate::scenes::dialogs::game::main_dialog::TextInputKind, String)>,

    /// 暂存的排行榜刷新请求（由 draw 阶段产出，由 update 阶段发包）
    pending_ranking_refresh_tab: Option<u8>,

    /// 暂存的装备请求（由 draw 阶段 Inventory→Character 拖拽产出，由 update 阶段发包）
    pending_equip_request: Option<u64>,

    /// 暂存的卸下装备请求（由 draw 阶段 Character→Inventory 拖拽产出，由 update 阶段发包）
    pending_unequip_request: Option<u64>,

    /// 缓存的任务信息（来自 NewQuestInfo，等待 QuestAccepted 到来时使用）
    cached_quest_info: std::collections::HashMap<u32, (String, String, String, u32, u64, u32)>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum NpcUiLayer {
    Dialog,
    Goods,
    SubGoods,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum UiStackTop {
    Main,
    Npc,
}

impl Default for UIRenderSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl UIRenderSystem {
    pub fn new() -> Self {
        Self {
            main_dialog: MainDialog::new(),
            npc_dialog: NpcDialogHybrid::new(),
            npc_goods_dialog: NpcGoodsDialogHybrid::new(),
            npc_sub_goods_dialog: NpcGoodsDialogHybrid::new(),
            amount_box: AmountBoxHybrid::new(),
            timer_dialog: crate::scenes::dialogs::game::timer_dialog::TimerDialogHybrid::new(),
            chat_notice_dialog: crate::scenes::dialogs::game::chat_notice_dialog::ChatNoticeDialogHybrid::new(),
            notice_dialog: crate::scenes::dialogs::game::notice_dialog::NoticeDialogHybrid::new(),
            roll_dialog: crate::scenes::dialogs::game::roll_dialog::RollDialogHybrid::new(),
            dura_status_dialog: crate::scenes::dialogs::game::dura_status_dialog::DuraStatusDialogHybrid::new(),
            npc_drop_dialog: crate::scenes::dialogs::game::npc_drop_dialog::NPCDropDialogHybrid::new(),
            guild_territory_dialog: crate::scenes::dialogs::game::guild_territory_dialog::GuildTerritoryDialogHybrid::new(),
            keyboard_layout_dialog: crate::scenes::dialogs::game::keyboard_layout_dialog::KeyboardLayoutDialogHybrid::new(),
            npc_awake_dialog: crate::scenes::dialogs::game::npc_awake_dialog::NPCAwakeDialogHybrid::new(),
            craft_dialog: crate::scenes::dialogs::game::craft_dialog::CraftDialogHybrid::new(),
            refine_dialog: crate::scenes::dialogs::game::refine_dialog::RefineDialogHybrid::new(),
            item_rental_dialog: crate::scenes::dialogs::game::item_rental_dialog::ItemRentalDialogHybrid::new(),
            trust_merchant_dialog: crate::scenes::dialogs::game::trust_merchant_dialog::TrustMerchantDialogHybrid::new(),

            // 默认：SubGoods 在最上层（如果打开）。
            npc_z_order: vec![NpcUiLayer::Dialog, NpcUiLayer::Goods, NpcUiLayer::SubGoods],

            ui_stack_top: UiStackTop::Main,
            pending_trade_action: None,
            pending_text_input: None,
            pending_ranking_refresh_tab: None,
            pending_equip_request: None,
            pending_unequip_request: None,
            cached_quest_info: std::collections::HashMap::new(),
        }
    }

    fn bring_npc_layer_to_front(&mut self, layer: NpcUiLayer) {
        if let Some(i) = self.npc_z_order.iter().position(|&x| x == layer) {
            self.npc_z_order.remove(i);
        }
        self.npc_z_order.push(layer);
    }

    fn npc_layer_visible(&self, layer: NpcUiLayer) -> bool {
        match layer {
            NpcUiLayer::Dialog => self.npc_dialog.is_visible(),
            NpcUiLayer::Goods => self.npc_goods_dialog.is_visible(),
            NpcUiLayer::SubGoods => self.npc_sub_goods_dialog.is_visible(),
        }
    }

    fn npc_layer_mouse_over(&self, layer: NpcUiLayer, mouse_pos: macroquad::prelude::Vec2) -> bool {
        match layer {
            NpcUiLayer::Dialog => self.npc_dialog.is_mouse_over(mouse_pos),
            NpcUiLayer::Goods => self.npc_goods_dialog.is_mouse_over(mouse_pos),
            NpcUiLayer::SubGoods => self.npc_sub_goods_dialog.is_mouse_over(mouse_pos),
        }
    }

    fn npc_mouse_over_any(&self, mouse_pos: macroquad::prelude::Vec2) -> bool {
        (self.npc_dialog.is_visible() && self.npc_dialog.is_mouse_over(mouse_pos))
            || (self.npc_goods_dialog.is_visible() && self.npc_goods_dialog.is_mouse_over(mouse_pos))
            || (self.npc_sub_goods_dialog.is_visible() && self.npc_sub_goods_dialog.is_mouse_over(mouse_pos))
    }

    fn close_npc_related_dialogs(&mut self) {
        self.npc_dialog.hide();
        self.npc_goods_dialog.hide();
        self.npc_sub_goods_dialog.hide();
        self.amount_box.hide();
    }
}

impl RenderSystem for UIRenderSystem {
    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
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
                UiCommand::CloseNpcRelatedDialogs => self.close_npc_related_dialogs(),
                UiCommand::CloseAllPopups => self.main_dialog.close_all_popups(),
                UiCommand::OpenInventory => self.main_dialog.open_inventory(),
                UiCommand::ActivateChatInput => self.main_dialog.activate_chat_input(),
                UiCommand::ToggleMinimap => self.main_dialog.toggle_minimap(),
                UiCommand::ToggleMinimapSize => self.main_dialog.toggle_minimap_size(),
                UiCommand::PushSystemChatLine(line) => self.main_dialog.push_system_chat_line(line),
                UiCommand::PushChatLine(line) => self.main_dialog.push_chat_line(line),
                UiCommand::PushWhisperLine(line) => self.main_dialog.push_whisper_line(line),
                UiCommand::ShowNpcDialog { dialog } => {
                    self.npc_goods_dialog.hide();
                    self.npc_sub_goods_dialog.hide();
                    self.amount_box.hide();
                    if let Some(s) = ctx.world.query::<&UiState>().iter().next() {
                        s.borrow_mut().amount_box_buy_uid = None;
                    }
                    self.npc_dialog.new_dialog(dialog);
                    self.bring_npc_layer_to_front(NpcUiLayer::Dialog);
                }
                UiCommand::ShowNpcGoods {
                    items,
                    rate,
                    panel_type,
                    hide_added_stats,
                    is_sub,
                } => {
                    if is_sub {
                        self.npc_sub_goods_dialog.new_goods(
                            items,
                            rate,
                            panel_type,
                            hide_added_stats,
                        );
                        self.bring_npc_layer_to_front(NpcUiLayer::SubGoods);
                    } else {
                        self.npc_goods_dialog
                            .new_goods(items, rate, panel_type, hide_added_stats);
                        self.bring_npc_layer_to_front(NpcUiLayer::Goods);
                    }
                    self.main_dialog.open_inventory();
                }
                UiCommand::ShowAmountBox {
                    title,
                    image_index,
                    max_quantity,
                    min_quantity,
                    default_amount,
                    buy_uid,
                } => {
                    self.amount_box.show(
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
                    self.amount_box.hide();
                    if let Some(s) = ctx.world.query::<&UiState>().iter().next() {
                        s.borrow_mut().amount_box_buy_uid = None;
                    }
                }
                UiCommand::HideNpcGoodsSub => {
                    self.npc_sub_goods_dialog.hide();
                }
                UiCommand::HideNpcGoods => {
                    self.npc_goods_dialog.hide();
                }
                UiCommand::UpdateMountState { mount_type, riding } => {
                    self.main_dialog.mount_dialog_mut().update_mount_state(mount_type, riding);
                }
                UiCommand::UpdateHeroBehaviour { behaviour } => {
                    if let Ok(b) = behaviour.try_into() {
                        self.main_dialog.hero_dialog_mut().set_behaviour(b);
                    }
                }
                UiCommand::PushHeroSystemChat(msg) => {
                    self.main_dialog.push_system_chat_line(msg);
                }
                UiCommand::UpdateFishingState { state, chance, progress } => {
                    self.main_dialog.fishing_dialog_mut().update_fishing_state(state, chance, progress);
                }
                UiCommand::SetFishingAutoCast { enabled } => {
                    self.main_dialog.fishing_dialog_mut().set_auto_cast(enabled);
                }
                UiCommand::AddBuff { buff } => {
                    self.main_dialog.buff_dialog_mut().add_buff(buff.clone());
                }
                UiCommand::RemoveBuff { buff_type } => {
                    self.main_dialog.buff_dialog_mut().remove_buff(buff_type);
                }
                UiCommand::UpdateCreatureList { creatures } => {
                    self.main_dialog.intelligent_creature_dialog_mut().update_creatures(creatures.clone());
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
                    self.main_dialog.friend_dialog_mut().update_friends(friend_infos);
                }
                UiCommand::UpdateGroupMembers { members } => {
                    self.main_dialog.group_dialog_mut().update_members(members);
                }
                UiCommand::SetGroupAllowJoin { allow } => {
                    self.main_dialog.group_dialog_mut().set_allow_join(allow);
                }
                UiCommand::UpdateGroupMemberMap { player_name, player_map } => {
                    self.main_dialog.group_dialog_mut().update_member_map(&player_name, player_map);
                }
                UiCommand::SetHeroAutoPotUnlocked => {
                    tracing::debug!("Hero auto-pot unlocked");
                }
                UiCommand::SetHeroAutoPotValue { pot_type, value } => {
                    tracing::debug!("Hero auto-pot value set: type={} value={}", pot_type, value);
                }
                UiCommand::SetHeroAutoPotItem { item_id } => {
                    tracing::debug!("Hero auto-pot item set: {}", item_id);
                }
                UiCommand::SetBuffPaused { buff_id, paused } => {
                    self.main_dialog.buff_dialog_mut().set_buff_paused(buff_id, paused);
                }
                UiCommand::UpdateCompass { location } => {
                    let dir = crate::scenes::dialogs::game::compass_dialog::CompassDirection::from_location(location.0, location.1);
                    self.main_dialog.compass_dialog_mut().set_direction(dir);
                }
                UiCommand::OpenTradeDialog { partner } => {
                    self.main_dialog.open_trade_dialog(&partner);
                }
                UiCommand::TradeGoldAdded { amount } => {
                    self.main_dialog.trade_dialog_mut().add_their_gold(amount);
                }
                UiCommand::TradeItemAdded => {
                    // 由 TradeDialog 内部通过 take_action 同步，此处仅作日志
                    tracing::debug!("Trade item added (server-side sync)");
                }
                UiCommand::TradeConfirmed { locked } => {
                    self.main_dialog.trade_dialog_mut().set_partner_confirmed(locked);
                }
                UiCommand::TradeCancelled => {
                    self.main_dialog.trade_dialog_mut().reset_confirmations();
                }
                UiCommand::QuestAccepted { quest_id, name, description } => {
                    use crate::scenes::dialogs::game::quest_log_dialog::{QuestInfo, QuestRewards, QuestStatus};
                    // 优先使用缓存的真实数据（来自 NewQuestInfo），否则用传入的 stub 数据
                    let (real_name, real_group, real_desc, level_req, reward_exp, reward_gold) =
                        self.cached_quest_info.remove(&quest_id)
                            .unwrap_or_else(|| (name.clone(), String::new(), description.clone(), 0u32, 0u64, 0u32));
                    self.main_dialog.quest_log_dialog_mut().add_quest(QuestInfo {
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
                    self.main_dialog.quest_log_dialog_mut().notify_quest_complete(quest_id);
                    self.main_dialog.quest_log_dialog_mut().remove_quest(quest_id);
                }
                UiCommand::QuestProgressUpdated { quest_id, progress_text } => {
                    self.main_dialog.quest_log_dialog_mut().update_quest_progress_from_text(quest_id, progress_text.as_str());
                }
                UiCommand::QuestInfoReceived {
                    quest_id, name, group, description, level_req, reward_exp, reward_gold,
                } => {
                    // 缓存任务信息，等 QuestAccepted 到来时使用真实数据
                    self.cached_quest_info.insert(
                        quest_id,
                        (name.clone(), group.clone(), description.clone(), level_req, reward_exp, reward_gold),
                    );
                }
                // 公会扩展事件
                UiCommand::GuildMemberUpdated { name, rank, online } => {
                    self.main_dialog.guild_dialog_mut().update_member(name.clone(), rank.clone(), online);
                }
                UiCommand::GuildNoticeUpdated { notice } => {
                    self.main_dialog.guild_dialog_mut().update_notice(notice.clone());
                }
                UiCommand::GuildExpGained { amount } => {
                    self.main_dialog.push_system_chat_line(format!("行会经验 +{}", amount));
                }
                UiCommand::GuildWarRequested => {
                    self.main_dialog.push_system_chat_line("行会战请求！".to_string());
                }
                UiCommand::SetGuildName { name } => {
                    self.main_dialog.guild_dialog_mut().update_guild_info(crate::scenes::dialogs::game::guild_dialog::GuildInfo {
                        name: name.clone(),
                        ..Default::default()
                    });
                    tracing::debug!("🏰 行会名称: {}", name);
                }
                UiCommand::UpdateGuildStorageGold { gold } => {
                    self.main_dialog.guild_dialog_mut().update_storage_gold(gold);
                }
                UiCommand::UpdateGuildStorageItems { items } => {
                    for item in items {
                        self.main_dialog.guild_dialog_mut().update_storage_item(item.name.clone(), item.quantity, item.slot);
                    }
                }
                UiCommand::ClearGuildStorageItems => {
                    self.main_dialog.guild_dialog_mut().clear_storage_items();
                }
                UiCommand::OpenMailDialog => {
                    tracing::debug!("📮 打开邮件对话框");
                    self.main_dialog.mail_dialog_mut().open();
                }
                UiCommand::CloseMailDialog => {
                    self.main_dialog.mail_dialog_mut().close();
                }
                UiCommand::UpdateMailList { mails } => {
                    // 同步到对话框和 UiStateData
                    self.main_dialog.mail_dialog_mut().set_mails(mails.clone());
                    if let Some(s) = ctx.world.query::<&UiState>().iter().next() {
                        s.borrow_mut().mail_entries = mails.clone();
                    }
                }
                UiCommand::OpenBigMap => {
                    tracing::debug!("🗺️ 打开大地图");
                    self.main_dialog.big_map_dialog_mut().show();
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
                    self.npc_goods_dialog.show_storage_mode(vec![], inventory_items, 1.0);
                    self.bring_npc_layer_to_front(NpcUiLayer::Goods);
                }
                UiCommand::UpdateStorageItems { items } => {
                    self.npc_goods_dialog.update_storage_items(items);
                }
                UiCommand::UpdateStorageInventoryItems { items } => {
                    self.npc_goods_dialog.update_storage_inventory_items(items);
                }
                UiCommand::SetMarriageRequester { requester } => {
                    self.main_dialog.relationship_dialog_mut().set_marriage_requester(requester.clone());
                }
                UiCommand::ClearMarriageRequester => {
                    self.main_dialog.relationship_dialog_mut().clear_marriage_requester();
                }
                UiCommand::UpdateLover { name, date } => {
                    self.main_dialog.relationship_dialog_mut().set_lover_info(name, date);
                }
                UiCommand::UpdateMentor { name, level, online } => {
                    self.main_dialog.relationship_dialog_mut().set_mentor_info(name, level as u32, online);
                }
                UiCommand::ShowTextInput { kind, title, placeholder, max_length } => {
                    self.main_dialog.set_pending_text_input_kind(kind);
                    self.main_dialog.text_input_dialog_mut().show(&title, &placeholder, max_length);
                }
                UiCommand::HideTextInput => {
                    self.main_dialog.reset_pending_text_input_kind();
                    self.main_dialog.text_input_dialog_mut().hide();
                }
                UiCommand::UpdateRankings { tab, entries } => {
                    tracing::debug!("🏆 更新排行榜: tab={}, {} entries", tab, entries.len());
                    let ranking_dialog = self.main_dialog.ranking_dialog_mut();
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
                    self.main_dialog.game_shop_dialog_mut().update_from_server(items, credit, gold);
                }
                UiCommand::UpdateGameShopStock { item_index, stock } => {
                    self.main_dialog.game_shop_dialog_mut().update_stock(item_index, stock);
                }
                UiCommand::UpdateAttackMode { mode } => {
                    self.main_dialog.set_attack_mode(mode);
                }
                UiCommand::UpdatePetMode { mode } => {
                    self.main_dialog.set_pet_mode(mode);
                }
                UiCommand::SetTimer { timer_id, seconds } => {
                    self.timer_dialog.set_timer(timer_id, seconds);
                }
                UiCommand::TimerExpired { timer_id } => {
                    self.timer_dialog.remove_timer(timer_id);
                }
                UiCommand::PushChatNotice { text } => {
                    self.chat_notice_dialog.push_notice_default(text);
                }
                UiCommand::ShowNotice { text } => {
                    self.notice_dialog.set_notice(text);
                }
                UiCommand::CloseNotice => {
                    self.notice_dialog.close();
                }
                UiCommand::ShowRollResult { value } => {
                    self.roll_dialog.show_roll(value);
                }
                UiCommand::UpdateDuraStatus { items } => {
                    self.dura_status_dialog.update_dura(items);
                }
                UiCommand::ToggleDuraStatus => {
                    self.dura_status_dialog.toggle();
                }
                UiCommand::ShowNPCDrop { npc_name, items } => {
                    self.npc_drop_dialog.show(npc_name, items);
                }
                UiCommand::ShowGuildTerritory => {
                    self.guild_territory_dialog.show();
                }
                UiCommand::UpdateGuildTerritory { entries, page, total } => {
                    self.guild_territory_dialog.update_territories(entries, page, total);
                }
                UiCommand::ToggleKeyboardLayout => {
                    self.keyboard_layout_dialog.toggle();
                }
                UiCommand::ShowNPCAwake { item_name, materials } => {
                    self.npc_awake_dialog.show(item_name, materials);
                }
                UiCommand::SetAwakeLocked { locked } => {
                    self.npc_awake_dialog.set_locked(locked);
                }
                UiCommand::ShowCraft { recipes } => {
                    self.craft_dialog.show(recipes);
                }
                UiCommand::ShowRefine { item_name, stats, material_name, material_have, material_need } => {
                    self.refine_dialog.show(item_name, stats, material_name, material_have, material_need);
                }
                UiCommand::OpenItemRental { partner } => {
                    self.item_rental_dialog.show(partner);
                }
                UiCommand::UpdateRentalFee { fee } => {
                    self.item_rental_dialog.update_fee(fee);
                }
                UiCommand::UpdateRentalPeriod { period } => {
                    self.item_rental_dialog.update_period(period);
                }
                UiCommand::SetRentalLocked { locked } => {
                    self.item_rental_dialog.set_locked(locked);
                }
                UiCommand::SetRentalPartnerLocked { locked } => {
                    self.item_rental_dialog.set_partner_locked(locked);
                }
                UiCommand::CloseItemRental => {
                    self.item_rental_dialog.close();
                }
                UiCommand::OpenTrustMerchant => {
                    self.trust_merchant_dialog.show();
                }
                UiCommand::UpdateMerchantItems { items, page, total } => {
                    self.trust_merchant_dialog.update_items(items, page, total);
                }
                UiCommand::CloseTrustMerchant => {
                    self.trust_merchant_dialog.close();
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
            self.main_dialog.set_minimap_world_size(ws.x, ws.y);
        }
        if let Some(p) = minimap_player_pos {
            self.main_dialog
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
                self.main_dialog.big_map_dialog_mut().set_map_info(name, ws.x, ws.y);
            }
            if let Some(p) = player_pos {
                self.main_dialog.big_map_dialog_mut().set_player_position(p.x, p.y);
            }

            // 同步地图瓦片数据
            if let Some(map_data) = ctx.world.query::<&MapData>().iter().next() {
                self.main_dialog.big_map_dialog_mut().set_map_data(map_data.cells.clone(), map_data.width, map_data.height);
            }
        }

        // 2.4b) 消费小地图待处理动作
        {
            use crate::scenes::dialogs::game::minimap_dialog::MiniMapAction;
            let action = self.main_dialog.minimap_dialog_mut().take_pending_actions();
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

                self.main_dialog
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

                self.main_dialog.set_player_stats(
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
                    self.main_dialog.sync_inventory(&inv);
                }
            }
        }

        // 3) 快捷键（由 UIRenderSystem 统一处理，避免 GameScene 直连 UI 组件）
        if !self.amount_box.is_visible() && !self.main_dialog.is_any_input_active() {
            if is_key_pressed(KeyCode::Enter) {
                self.main_dialog.activate_chat_input();
            }
            if is_key_pressed(KeyCode::M) {
                self.main_dialog.toggle_minimap();
            }
            if is_key_pressed(KeyCode::Tab) {
                self.main_dialog.toggle_minimap_size();
            }
        }

        // ESC：关闭弹窗优先（AmountBox/NPCGoods/SubGoods/Popups）
        if is_key_pressed(KeyCode::Escape) {
            if self.amount_box.is_visible() {
                self.amount_box.hide();
                if let Some(s) = ctx.world.query::<&UiState>().iter().next() {
                    s.borrow_mut().amount_box_buy_uid = None;
                }
            } else if self.npc_sub_goods_dialog.is_visible() {
                self.npc_sub_goods_dialog.hide();
            } else if self.npc_goods_dialog.is_visible() {
                self.npc_goods_dialog.hide();
            } else if self.main_dialog.any_popup_open() {
                self.main_dialog.close_all_popups();
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

        let ui_over = self.main_dialog.is_mouse_over_ui(mouse_pos)
            || self.npc_goods_dialog.is_mouse_over(mouse_pos)
            || self.npc_dialog.is_mouse_over(mouse_pos)
            || self.npc_sub_goods_dialog.is_mouse_over(mouse_pos)
            || self.amount_box.is_mouse_over(mouse_pos);

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

        let ui_input_active = self.main_dialog.is_any_input_active();
        let amount_box_visible = self.amount_box.is_visible();
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
            s.any_modal_or_popup_open = self.main_dialog.any_popup_open()
                || self.npc_dialog.is_visible()
                || self.npc_goods_dialog.is_visible()
                || self.npc_sub_goods_dialog.is_visible()
                || self.amount_box.is_visible();
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
            let gd = self.main_dialog.group_dialog_mut();
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
            let fd = self.main_dialog.friend_dialog_mut();
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
            let gld = self.main_dialog.guild_dialog_mut();
            let action = gld.take_action();
            match action {
                GuildDialogAction::LeaveGuild => {
                    let player_name = self.main_dialog.character_name().to_string();
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
                    self.main_dialog.push_system_chat_line(format!("行会成员: {} | 职位: {} | 状态: {}", name, rank, status));
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
            let md = self.main_dialog.mentor_dialog_mut();
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
            let mail_action = self.main_dialog.mail_dialog_mut().take_action();
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
            let rd = self.main_dialog.relationship_dialog_mut();
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
            let md = self.main_dialog.mount_dialog_mut();
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
            let hd = self.main_dialog.hero_dialog_mut();
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
            let fd = self.main_dialog.fishing_dialog_mut();
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
            let cd = self.main_dialog.intelligent_creature_dialog_mut();
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
            let sd = self.main_dialog.socket_dialog_mut();
            let action = sd.take_action();
            match action {
                SocketAction::InsertGem { item_unique_id, position_idx } => {
                    // 当前 UI 未实现宝石选择器，AwakeType 无法确定
                    // 待实现背包宝石选择后再发包
                    tracing::debug!("💎 插入宝石: 待实现宝石选择器 (uid={}, pos={})", item_unique_id, position_idx);
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
        if self.main_dialog.take_pending_logout() {
            use crate::network::handlers::NetworkEvent as NetEv;
            if let Some(net) = ctx.net.as_ref() {
                let _ = net.send(NetEv::LogOutRequest);
            }
            tracing::info!("🚪 安全下线请求已发送");
        }

        // 交易对话框动作（由 draw 阶段产出，在此发包）
        if let Some(action) = self.pending_trade_action.take() {
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
        if let Some((kind, text)) = self.pending_text_input.take() {
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
            }
        }

        // 排行榜刷新请求（由 draw 阶段产出，在此发包）
        if let Some(tab) = self.pending_ranking_refresh_tab.take() {
            use crate::network::handlers::NetworkEvent as NetEv;
            if let Some(net) = ctx.net.as_ref() {
                let _ = net.send(NetEv::GetRankingRequest { ranking_type: tab });
            }
            tracing::debug!("🏆 请求排行榜: tab={}", tab);
        }

        // 装备物品请求（由 draw 阶段 Inventory→Character 拖拽产出，在此发包）
        if let Some(unique_id) = self.pending_equip_request.take() {
            use crate::network::handlers::NetworkEvent as NetEv;
            if let Some(net) = ctx.net.as_ref() {
                let _ = net.send(NetEv::EquipItemRequest { unique_id });
            }
            tracing::debug!("🎒 装备物品: unique_id={}", unique_id);
        }

        // 卸下装备请求（由 draw 阶段 Character→Inventory 拖拽产出，在此发包）
        if let Some(unique_id) = self.pending_unequip_request.take() {
            use crate::network::handlers::NetworkEvent as NetEv;
            if let Some(net) = ctx.net.as_ref() {
                let _ = net.send(NetEv::RemoveItemRequest { unique_id });
            }
            tracing::debug!("🎒 卸下装备: unique_id={}", unique_id);
        }

        Ok(())
    }

    fn draw(&mut self, _world: &hecs::World) -> GameResult {
        let pass = _world
            .query::<&RenderPass>()
            .iter()
            .next().copied()
            .unwrap_or_default();

        if pass.stage != RenderStage::Ui {
            return Ok(());
        }

        let initialized = _world
            .query::<&ResourceInitState>()
            .iter()
            .next()
            .map(|s| s.initialized)
            .unwrap_or(false);

        // 资源未初始化时，仅显示加载提示，避免 UI 组件访问未就绪资源。
        if !initialized {
            draw_text_cn(
                "⏳ 正在加载游戏资源...",
                screen_width() / 2.0 - 100.0,
                screen_height() / 2.0,
                24.0,
                WHITE,
            );
            return Ok(());
        }

        // 确保所有对话框纹理已加载（惰性加载，此时 data_path 一定已设置）
        self.main_dialog.ensure_textures_loaded();

        let (mx, my) = mouse_position();
        let mouse_pos = vec2(mx, my);

        // ===== 全局层级（Main vs NPC） =====
        // 目标：NPC 对话框/商品对话框点击后能像其它对话框一样置顶覆盖背包等。
        let amount_modal = self.amount_box.is_visible();
        let main_mouse_over_ui = self.main_dialog.is_mouse_over_ui(mouse_pos);
        let npc_mouse_over_ui = self.npc_mouse_over_any(mouse_pos);

        let left_clicked = is_mouse_button_pressed(MouseButton::Left);
        let right_clicked = is_mouse_button_pressed(MouseButton::Right);
        let any_clicked = left_clicked || right_clicked;

        // 点击置顶（跨栈）：仅根据“点击命中区域”切换最上层栈。
        if any_clicked && !amount_modal {
            if main_mouse_over_ui {
                self.ui_stack_top = UiStackTop::Main;
            } else if npc_mouse_over_ui {
                self.ui_stack_top = UiStackTop::Npc;
            }
        }

        // NPC 栈内部置顶：仅当 NPC 栈处于最上层时生效。
        if any_clicked && !amount_modal && self.ui_stack_top == UiStackTop::Npc {
            for &layer in self.npc_z_order.iter().rev() {
                if self.npc_layer_visible(layer) && self.npc_layer_mouse_over(layer, mouse_pos) {
                    self.bring_npc_layer_to_front(layer);
                    break;
                }
            }
        }

        // 计算 NPC 栈的“输入接收者”：仅当 NPC 栈位于最上层时，鼠标所在的最上层 NPC window 才吃输入。
        let npc_input_receiver = if amount_modal || self.ui_stack_top != UiStackTop::Npc {
            None
        } else {
            self.npc_z_order
                .iter()
                .rev()
                .copied()
                .find(|&layer| self.npc_layer_visible(layer) && self.npc_layer_mouse_over(layer, mouse_pos))
        };

        let npc_dialog_input_enabled = npc_input_receiver == Some(NpcUiLayer::Dialog);
        let npc_goods_input_enabled = npc_input_receiver == Some(NpcUiLayer::Goods);
        let npc_sub_input_enabled = npc_input_receiver == Some(NpcUiLayer::SubGoods);

        let mut npc_dialog_consumed = false;
        let mut npc_consumed = false;
        let mut npc_sub_consumed = false;

        // 下层先画，上层后画
        let ui_consumed = match self.ui_stack_top {
            UiStackTop::Main => {
                // NPC（下）
                for &layer in self.npc_z_order.clone().iter() {
                    match layer {
                        NpcUiLayer::Dialog => {
                            if self.npc_dialog.is_visible() {
                                npc_dialog_consumed = true;
                                let action = self.npc_dialog.update_and_draw_with_input(false);
                                if !matches!(
                                    action,
                                    crate::scenes::dialogs::game::npc_dialog::NpcDialogAction::None
                                ) {
                                    if let Some(s) = _world.query::<&UiState>().iter().next() {
                                        s.borrow_mut().pending_actions.push(UiAction::NpcDialog(action));
                                    }
                                }
                            }
                        }
                        NpcUiLayer::Goods => {
                            if self.npc_goods_dialog.is_visible() {
                                npc_consumed |= self
                                    .npc_goods_dialog
                                    .update_and_draw_with_input(None, false);
                            }
                        }
                        NpcUiLayer::SubGoods => {
                            if self.npc_sub_goods_dialog.is_visible() {
                                npc_sub_consumed |= self
                                    .npc_sub_goods_dialog
                                    .update_and_draw_with_input(None, false);
                            }
                        }
                    }
                }

                // Main（上）
                self.main_dialog.update_and_draw();
                self.main_dialog.show_dialogs()
            }
            UiStackTop::Npc => {
                // Main（下）
                self.main_dialog.update_and_draw();
                let consumed = self.main_dialog.show_dialogs();

                // NPC（上）
                for &layer in self.npc_z_order.clone().iter() {
                    match layer {
                        NpcUiLayer::Dialog => {
                            if self.npc_dialog.is_visible() {
                                npc_dialog_consumed = true;
                                let action = self
                                    .npc_dialog
                                    .update_and_draw_with_input(npc_dialog_input_enabled);
                                if !matches!(
                                    action,
                                    crate::scenes::dialogs::game::npc_dialog::NpcDialogAction::None
                                ) {
                                    if let Some(s) = _world.query::<&UiState>().iter().next() {
                                        s.borrow_mut().pending_actions.push(UiAction::NpcDialog(action));
                                    }
                                }
                            }
                        }
                        NpcUiLayer::Goods => {
                            if self.npc_goods_dialog.is_visible() {
                                npc_consumed |= self
                                    .npc_goods_dialog
                                    .update_and_draw_with_input(None, npc_goods_input_enabled);
                            }
                        }
                        NpcUiLayer::SubGoods => {
                            if self.npc_sub_goods_dialog.is_visible() {
                                npc_sub_consumed |= self
                                    .npc_sub_goods_dialog
                                    .update_and_draw_with_input(None, npc_sub_input_enabled);
                            }
                        }
                    }
                }

                consumed
            }
        };

        // 任务追踪面板（始终显示，不受对话框堆叠影响）
        self.main_dialog.draw_quest_tracker();

        // 任务完成通知
        self.main_dialog.draw_quest_notifications();

        // 服务器倒计时（全局 overlay，最上层绘制）
        let delta = macroquad::time::get_frame_time();
        self.timer_dialog.draw(screen_width(), screen_height(), delta);

        // 屏幕中央 transient 通知
        self.chat_notice_dialog.draw(screen_width(), screen_height(), delta);

        // 服务器公告对话框（模态弹窗，阻塞其它输入）
        let _notice_consumed = self.notice_dialog.draw(
            mouse_pos, 0.0,
            is_mouse_button_pressed(MouseButton::Left),
            false,
        );

        // 骰子结果
        self.roll_dialog.draw(screen_width(), screen_height(), delta);

        // 耐久度状态
        self.dura_status_dialog.draw(screen_width(), screen_height(), mouse_pos,
            is_mouse_button_pressed(MouseButton::Left));

        // NPC 赠送物品
        self.npc_drop_dialog.draw(screen_width(), screen_height(), mouse_pos,
            is_mouse_button_pressed(MouseButton::Left));

        // 行会领地
        let wheel_y = mouse_wheel().1;
        self.guild_territory_dialog.draw(screen_width(), screen_height(), mouse_pos,
            wheel_y, is_mouse_button_pressed(MouseButton::Left));

        // 键位设置
        let any_key = if self.keyboard_layout_dialog.is_rebinding() {
            macroquad::input::get_keys_pressed().into_iter().next()
        } else {
            None
        };
        self.keyboard_layout_dialog.draw(screen_width(), screen_height(), mouse_pos,
            wheel_y, is_mouse_button_pressed(MouseButton::Left), any_key);

        // 装备觉醒
        self.npc_awake_dialog.draw(screen_width(), screen_height(), mouse_pos,
            is_mouse_button_pressed(MouseButton::Left));

        // 合成
        if let Some(craft_data) = self.craft_dialog.draw(screen_width(), screen_height(), mouse_pos,
            wheel_y, is_mouse_button_pressed(MouseButton::Left)) {
            if let Some(s) = _world.query::<&UiState>().iter().next() {
                s.borrow_mut().pending_commands.push(UiCommand::CraftItemRequest {
                    recipe_unique_id: craft_data.recipe_unique_id,
                    count: craft_data.count,
                    slots: craft_data.slots,
                });
            }
        }

        // 精炼
        self.refine_dialog.draw(screen_width(), screen_height(), mouse_pos,
            is_mouse_button_pressed(MouseButton::Left));

        // 物品租赁
        self.item_rental_dialog.draw(screen_width(), screen_height(), mouse_pos,
            is_mouse_button_pressed(MouseButton::Left));
        if self.item_rental_dialog.confirm_clicked {
            if let Some(s) = _world.query::<&UiState>().iter().next() {
                s.borrow_mut().pending_commands.push(UiCommand::ConfirmItemRental);
            }
            self.item_rental_dialog.confirm_clicked = false;
        }

        // 寄售行
        self.trust_merchant_dialog.draw(screen_width(), screen_height(), mouse_pos,
            wheel_y, is_mouse_button_pressed(MouseButton::Left));

        // UI -> ECS：小地图点击自动寻路（在 show_dialogs 后取，保证同帧可用）
        if let Some(target) = self.main_dialog.take_pending_auto_path_target() {
            if let Some(s) = _world.query::<&UiState>().iter().next() {
                s.borrow_mut().pending_auto_path_target = Some(target);
            }
        }

        // 交易对话框动作（draw 阶段产出，存入 pending，update 阶段发包）
        if let Some(action) = self.main_dialog.take_pending_trade_action() {
            self.pending_trade_action = Some(action);
        }

        // 排行榜刷新请求（draw 阶段产出，存入 pending，update 阶段发包）
        if let Some(tab) = self.main_dialog.take_pending_ranking_refresh_tab() {
            self.pending_ranking_refresh_tab = Some(tab);
        }

        // 装备物品请求（draw 阶段 Inventory→Character 拖拽产出，存入 pending，update 阶段发包）
        if let Some(unique_id) = self.main_dialog.take_pending_equip_request() {
            self.pending_equip_request = Some(unique_id);
        }

        // 卸下装备请求（draw 阶段 Character→Inventory 拖拽产出，存入 pending，update 阶段发包）
        if let Some(unique_id) = self.main_dialog.take_pending_unequip_request() {
            self.pending_unequip_request = Some(unique_id);
        }

        if let Some(action) = self.npc_goods_dialog.take_action() {
            if let Some(s) = _world.query::<&UiState>().iter().next() {
                s.borrow_mut()
                    .pending_actions
                    .push(UiAction::NpcGoods(action));
            }
        }
        if let Some(action) = self.npc_sub_goods_dialog.take_action() {
            if let Some(s) = _world.query::<&UiState>().iter().next() {
                s.borrow_mut()
                    .pending_actions
                    .push(UiAction::NpcSubGoods(action));
            }
        }

        // 组队对话框动作已移至 update 处理（含网络发包）

        // 数量框（modal，最上层）
        let mut amount_consumed = false;
        if self.amount_box.is_visible() {
            amount_consumed = true;
            let r = self.amount_box.update_and_draw();
            if !matches!(
                r,
                crate::scenes::dialogs::game::amount_box::AmountBoxResult::None
            ) {
                if let Some(s) = _world.query::<&UiState>().iter().next() {
                    s.borrow_mut().pending_actions.push(UiAction::AmountBox(r));
                }
            }
        }

        // 大地图对话框（全屏背景，在数量框和文本输入框之下）
        if self.main_dialog.big_map_dialog_mut().is_visible() {
            self.main_dialog.big_map_dialog_mut().update_and_draw();
        }

        // 文本输入对话框（modal，最上层，覆盖数量框）
        use crate::scenes::dialogs::game::text_input_dialog::TextInputResult;
        if self.main_dialog.text_input_is_visible() {
            let result = self.main_dialog.text_input_dialog_mut().update_and_draw();
            if !matches!(result, TextInputResult::None) {
                match result {
                    TextInputResult::Ok(text) => {
                        let kind = self.main_dialog.pending_text_input_kind();
                        self.pending_text_input = Some((kind, text));
                    }
                    TextInputResult::Cancel => {
                        tracing::debug!("❌ 文本输入已取消");
                    }
                    TextInputResult::None => unreachable!(),
                }
                self.main_dialog.reset_pending_text_input_kind();
            }
        }

        if let Some(s) = _world.query::<&UiState>().iter().next() {
            s.borrow_mut().ui_consumed_last_frame = ui_consumed
                || npc_consumed
                || npc_sub_consumed
                || amount_consumed
                || npc_dialog_consumed;
        }

        // 死亡倒计时（回城复活）
        // - 仅显示给本地玩家
        // - 倒计时来源：DeathState.start_time（ObjectDied/HealthChanged->0 时挂载）
        // - 当前 mock 复活延迟为 5 秒
        {
            use crate::components::{DeathState, Health, LocalPlayer};

            const RESPAWN_DELAY_SECS: f32 = 5.0;

            let mut q = _world.query::<(&LocalPlayer, &Health, &DeathState)>();
            if let Some((_lp, hp, ds)) = q.iter().next() {
                if hp.current <= 0 {
                    let elapsed = std::time::Instant::now()
                        .duration_since(ds.start_time)
                        .as_secs_f32();
                    let remaining = (RESPAWN_DELAY_SECS - elapsed).ceil() as i32;

                    let text = if remaining > 0 {
                        format!("你已死亡，{} 秒后回城复活", remaining)
                    } else {
                        "正在回城复活...".to_string()
                    };
                    let font_size = 36.0;
                    let dims = measure_text_cn(&text, font_size);
                    let x = screen_width() / 2.0 - dims.width / 2.0;
                    let y = screen_height() * 0.28;
                    draw_text_with_outline(
                        &text,
                        x,
                        y,
                        font_size,
                        Color::from_rgba(255, 255, 255, 230),
                        Color::from_rgba(0, 0, 0, 220),
                    );
                }
            }
        }

        // 快捷键提示（覆盖在 UI 之上）
        let y = screen_height() - 25.0;
        draw_text_cn(
            "快捷键: Space+拖拽/滚轮=地图 | Enter=聊天 M=小地图 Tab=小地图大小 | ESC=返回角色选择",
            10.0,
            y,
            14.0,
            Color::from_rgba(200, 200, 200, 180),
        );
        Ok(())
    }
}
