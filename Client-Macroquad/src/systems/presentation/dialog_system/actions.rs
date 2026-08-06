use super::*;

pub fn handle_npc_goods_action(
    ctx: &mut GameContext,
    action: crate::scenes::dialogs::game::npc_goods_dialog::NpcGoodsDialogAction,
) {
    use crate::network::handlers::NetworkEvent as NetEv;

    match action {
        crate::scenes::dialogs::game::npc_goods_dialog::NpcGoodsDialogAction::OpenSubGoods {
            items,
            rate,
            hide_added_stats,
        } => {
            let _ = UiState::with_mut_in_world(&mut ctx.world, |ui| {
                ui.pending_commands.push(UiCommand::ShowNpcGoods {
                    items,
                    rate,
                    panel_type: PanelType::BuySub,
                    hide_added_stats,
                    is_sub: true,
                    use_pearls: false,
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
                        free_space = Some(DialogSystem::inventory_total_free_space(
                            inv, item_index, stack_size,
                        ));
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
                    let _ = UiState::with_mut_in_world(&mut ctx.world, |ui| {
                        sys_chat(
                            &mut ui.pending_commands,
                            if use_pearls {
                                "You do not have enough Pearls."
                            } else {
                                "Not enough gold."
                            },
                        );
                    });
                    return;
                }

                if let Some(free) = free_space {
                    max_quantity = max_quantity.min(free).min(stack_max);
                }

                if max_quantity == 0 {
                    let _ = UiState::with_mut_in_world(&mut ctx.world, |ui| {
                        sys_chat(&mut ui.pending_commands, "You do not have enough space.");
                    });
                    return;
                }

                let _ = UiState::with_mut_in_world(&mut ctx.world, |ui| {
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
                        free_space = Some(DialogSystem::inventory_total_free_space(
                            inv, item_index, stack_size,
                        ));
                    }
                }

                (gold, credit, free_space)
            };

            if let Err(msg) = DialogSystem::can_send_buy_request(
                gold, credit, free_space, unit_price, count, stack_size, use_pearls,
            ) {
                let _ = UiState::with_mut_in_world(&mut ctx.world, |ui| {
                    sys_chat(&mut ui.pending_commands, msg);
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
        crate::scenes::dialogs::game::npc_goods_dialog::NpcGoodsDialogAction::RequestCraft {
            item,
        } => {
            // 打开合成对话框，配方信息来自 NPC 商品列表
            let recipe = crate::scenes::dialogs::game::craft_dialog::CraftRecipe {
                name: item
                    .info
                    .as_ref()
                    .map(|i| i.name.clone())
                    .unwrap_or_else(|| "未知配方".to_string()),
                recipe_unique_id: item.unique_id,
                materials: Vec::new(), // 协议不携带配方材料数据，由客户端仅展示结果物品
                gold: item.info.as_ref().map(|i| i.price).unwrap_or(0), // 用商品价格作为合成 Gold 成本
            };
            let _ = UiState::with_mut_in_world(&mut ctx.world, |ui| {
                ui.pending_commands.push(UiCommand::ShowCraft {
                    recipes: vec![recipe],
                });
            });
        }
    }
}

pub fn process_ui_actions(ctx: &mut GameContext) {
    let actions =
        UiState::with_mut_in_world(&mut ctx.world, |ui| std::mem::take(&mut ui.pending_actions))
            .unwrap_or_default();

    for action in actions {
        match action {
            UiAction::NpcDialog(a) => match a {
                crate::scenes::dialogs::game::npc_dialog::NpcDialogAction::None => {}
                crate::scenes::dialogs::game::npc_dialog::NpcDialogAction::Close => {
                    let _ = UiState::with_mut_in_world(&mut ctx.world, |ui| {
                        ui.pending_commands.push(UiCommand::CloseNpcRelatedDialogs);
                    });
                }
                crate::scenes::dialogs::game::npc_dialog::NpcDialogAction::OpenLink { url } => {
                    let _ = UiState::with_mut_in_world(&mut ctx.world, |ui| {
                        sys_chat(&mut ui.pending_commands, format!("链接：{}", url));
                    });
                }
                crate::scenes::dialogs::game::npc_dialog::NpcDialogAction::ClickAction { action } => {
                    if !DialogSystem::try_consume_npc_call_cooldown(ctx) {
                        continue;
                    }

                    let Some(npc_object_id) = DialogSystem::active_npc_object_id(ctx) else {
                        let _ = UiState::with_mut_in_world(&mut ctx.world, |ui| {
                            sys_chat(&mut ui.pending_commands, "当前没有选中的 NPC，无法发送对话选项。");
                        });
                        continue;
                    };

                    if let Some(net) = ctx.net.as_ref() {
                        let key = format!("[{}]", action);
                        let _ = net.send(NetworkEvent::NPCCallRequest { npc_object_id, key });
                    }
                }
                // ===== PR #1169: Warehouse password UI triggers =====
                crate::scenes::dialogs::game::npc_dialog::NpcDialogAction::StorageUnlock => {
                    let _ = UiState::with_mut_in_world(&mut ctx.world, |ui| {
                        ui.pending_commands.push(UiCommand::ShowTextInput {
                            kind: crate::scenes::dialogs::game::main_dialog::TextInputKind::UnlockStorage,
                            title: "解锁仓库".to_string(),
                            placeholder: "输入仓库密码".to_string(),
                            max_length: 32,
                        });
                    });
                }
                crate::scenes::dialogs::game::npc_dialog::NpcDialogAction::StorageRemovePassword => {
                    let _ = UiState::with_mut_in_world(&mut ctx.world, |ui| {
                        ui.pending_commands.push(UiCommand::ShowTextInput {
                            kind: crate::scenes::dialogs::game::main_dialog::TextInputKind::RemoveStoragePassword,
                            title: "删除仓库密码".to_string(),
                            placeholder: "输入当前密码以确认".to_string(),
                            max_length: 32,
                        });
                    });
                }
            },
            UiAction::NpcGoods(a) | UiAction::NpcSubGoods(a) => {
                handle_npc_goods_action(ctx, a);
            }
            UiAction::AmountBox(r) => {
                match r {
                    crate::scenes::dialogs::game::amount_box::AmountBoxResult::Ok(amount) => {
                        if amount > 0 {
                            let uid = UiState::with_mut_in_world(&mut ctx.world, |ui| ui.amount_box_buy_uid.take())
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
                        let _ = UiState::with_mut_in_world(&mut ctx.world, |ui| {
                            ui.amount_box_buy_uid = None;
                        });
                    }
                    crate::scenes::dialogs::game::amount_box::AmountBoxResult::None => {}
                }
            }
        }
    }
}
