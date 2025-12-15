use macroquad::prelude::get_time;

use crate::{
    components::{ActiveNpc, LocalPlayer, NpcCallCooldown, RenderPass},
    game::{GameContext, GameResult},
    network::handlers::NetworkEvent,
    systems::LogicSystem,
    ui::ui_state::{UiAction, UiCommand, UiState},
};

use mir2_shared::enums::PanelType;

#[derive(ecs_macros::LogicSystem)]
pub struct DialogSystem {
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
        let (_e, s) = q.iter().next()?;
        let mut data = s.borrow_mut();
        Some(f(&mut data))
    }

    fn try_consume_npc_call_cooldown(ctx: &mut GameContext) -> bool {
        let now = get_time();

        // RenderPass 单例实体上挂了 NpcCallCooldown。
        let mut q = ctx.world.query::<&mut NpcCallCooldown>();
        let Some((_e, cd)) = q.iter().next() else {
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
        q.iter().next().and_then(|(_, a)| a.npc_object_id)
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
                NetworkEvent::ChatMessage { sender, message, .. } => {
                    cmds.push(UiCommand::PushChatLine(format!("{}: {}", sender, message)));
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
                        if let Some((_e, (_local, cur, inv))) = q.iter().next() {
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
                        if let Some((_e, (_local, cur, inv))) = q.iter().next() {
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
