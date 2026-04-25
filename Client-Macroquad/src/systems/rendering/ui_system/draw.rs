use super::*;

pub fn draw(sys: &mut UIRenderSystem, _world: &hecs::World) -> GameResult {
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
    sys.main_dialog.ensure_textures_loaded();

    let (mx, my) = mouse_position();
    let mouse_pos = vec2(mx, my);

    // ===== 全局层级（Main vs NPC） =====
    // 目标：NPC 对话框/商品对话框点击后能像其它对话框一样置顶覆盖背包等。
    let amount_modal = sys.amount_box.is_visible();
    let main_mouse_over_ui = sys.main_dialog.is_mouse_over_ui(mouse_pos);
    let npc_mouse_over_ui = sys.npc_mouse_over_any(mouse_pos);

    let left_clicked = is_mouse_button_pressed(MouseButton::Left);
    let right_clicked = is_mouse_button_pressed(MouseButton::Right);
    let any_clicked = left_clicked || right_clicked;

    // 点击置顶（跨栈）：仅根据"点击命中区域"切换最上层栈。
    if any_clicked && !amount_modal {
        if main_mouse_over_ui {
            sys.ui_stack_top = UiStackTop::Main;
        } else if npc_mouse_over_ui {
            sys.ui_stack_top = UiStackTop::Npc;
        }
    }

    // NPC 栈内部置顶：仅当 NPC 栈处于最上层时生效。
    if any_clicked && !amount_modal && sys.ui_stack_top == UiStackTop::Npc {
        for &layer in sys.npc_z_order.iter().rev() {
            if sys.npc_layer_visible(layer) && sys.npc_layer_mouse_over(layer, mouse_pos) {
                sys.bring_npc_layer_to_front(layer);
                break;
            }
        }
    }

    // 计算 NPC 栈的"输入接收者"：仅当 NPC 栈位于最上层时，鼠标所在的最上层 NPC window 才吃输入。
    let npc_input_receiver = if amount_modal || sys.ui_stack_top != UiStackTop::Npc {
        None
    } else {
        sys.npc_z_order
            .iter()
            .rev()
            .copied()
            .find(|&layer| sys.npc_layer_visible(layer) && sys.npc_layer_mouse_over(layer, mouse_pos))
    };

    let npc_dialog_input_enabled = npc_input_receiver == Some(NpcUiLayer::Dialog);
    let npc_goods_input_enabled = npc_input_receiver == Some(NpcUiLayer::Goods);
    let npc_sub_input_enabled = npc_input_receiver == Some(NpcUiLayer::SubGoods);

    let mut npc_dialog_consumed = false;
    let mut npc_consumed = false;
    let mut npc_sub_consumed = false;

    // 下层先画，上层后画
    let ui_consumed = match sys.ui_stack_top {
        UiStackTop::Main => {
            // NPC（下）
            for i in 0..sys.npc_z_order.len() {
                let layer = sys.npc_z_order[i];
                match layer {
                    NpcUiLayer::Dialog => {
                        if sys.npc_dialog.is_visible() {
                            npc_dialog_consumed = true;
                            let action = sys.npc_dialog.update_and_draw_with_input(false);
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
                        if sys.npc_goods_dialog.is_visible() {
                            npc_consumed |= sys
                                .npc_goods_dialog
                                .update_and_draw_with_input(None, false);
                        }
                    }
                    NpcUiLayer::SubGoods => {
                        if sys.npc_sub_goods_dialog.is_visible() {
                            npc_sub_consumed |= sys
                                .npc_sub_goods_dialog
                                .update_and_draw_with_input(None, false);
                        }
                    }
                }
            }

            // Main（上）
            sys.main_dialog.update_and_draw();
            sys.main_dialog.show_dialogs()
        }
        UiStackTop::Npc => {
            // Main（下）
            sys.main_dialog.update_and_draw();
            let consumed = sys.main_dialog.show_dialogs();

            // NPC（上）
            for i in 0..sys.npc_z_order.len() {
                let layer = sys.npc_z_order[i];
                match layer {
                    NpcUiLayer::Dialog => {
                        if sys.npc_dialog.is_visible() {
                            npc_dialog_consumed = true;
                            let action = sys
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
                        if sys.npc_goods_dialog.is_visible() {
                            npc_consumed |= sys
                                .npc_goods_dialog
                                .update_and_draw_with_input(None, npc_goods_input_enabled);
                        }
                    }
                    NpcUiLayer::SubGoods => {
                        if sys.npc_sub_goods_dialog.is_visible() {
                            npc_sub_consumed |= sys
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
    sys.main_dialog.draw_quest_tracker();

    // 任务完成通知
    sys.main_dialog.draw_quest_notifications();

    // 服务器倒计时（全局 overlay，最上层绘制）
    let delta = macroquad::time::get_frame_time();
    sys.timer_dialog.draw(screen_width(), screen_height(), delta);

    // 屏幕中央 transient 通知
    sys.chat_notice_dialog.draw(screen_width(), screen_height(), delta);

    // 服务器公告对话框（模态弹窗，阻塞其它输入）
    let _notice_consumed = sys.notice_dialog.draw(
        mouse_pos, 0.0,
        is_mouse_button_pressed(MouseButton::Left),
        false,
    );

    // 骰子结果
    sys.roll_dialog.draw(screen_width(), screen_height(), delta);

    // 耐久度状态
    sys.dura_status_dialog.draw(screen_width(), screen_height(), mouse_pos,
        is_mouse_button_pressed(MouseButton::Left));

    // NPC 赠送物品
    sys.npc_drop_dialog.draw(screen_width(), screen_height(), mouse_pos,
        is_mouse_button_pressed(MouseButton::Left));

    // 行会领地
    let wheel_y = mouse_wheel().1;
    sys.guild_territory_dialog.draw(screen_width(), screen_height(), mouse_pos,
        wheel_y, is_mouse_button_pressed(MouseButton::Left));

    // 键位设置
    let any_key = if sys.keyboard_layout_dialog.is_rebinding() {
        macroquad::input::get_keys_pressed().into_iter().next()
    } else {
        None
    };
    sys.keyboard_layout_dialog.draw(screen_width(), screen_height(), mouse_pos,
        wheel_y, is_mouse_button_pressed(MouseButton::Left), any_key);

    // 装备觉醒
    sys.npc_awake_dialog.draw(screen_width(), screen_height(), mouse_pos,
        is_mouse_button_pressed(MouseButton::Left));

    // 合成
    if let Some(craft_data) = sys.craft_dialog.draw(screen_width(), screen_height(), mouse_pos,
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
    sys.refine_dialog.draw(screen_width(), screen_height(), mouse_pos,
        is_mouse_button_pressed(MouseButton::Left));

    // 物品租赁
    sys.item_rental_dialog.draw(screen_width(), screen_height(), mouse_pos,
        is_mouse_button_pressed(MouseButton::Left));
    if sys.item_rental_dialog.confirm_clicked {
        if let Some(s) = _world.query::<&UiState>().iter().next() {
            s.borrow_mut().pending_commands.push(UiCommand::ConfirmItemRental);
        }
        sys.item_rental_dialog.confirm_clicked = false;
    }

    // 寄售行
    sys.trust_merchant_dialog.draw(screen_width(), screen_height(), mouse_pos,
        wheel_y, is_mouse_button_pressed(MouseButton::Left));

    // UI -> ECS：小地图点击自动寻路（在 show_dialogs 后取，保证同帧可用）
    if let Some(target) = sys.main_dialog.take_pending_auto_path_target() {
        if let Some(s) = _world.query::<&UiState>().iter().next() {
            s.borrow_mut().pending_auto_path_target = Some(target);
        }
    }

    // 交易对话框动作（draw 阶段产出，存入 pending，update 阶段发包）
    if let Some(action) = sys.main_dialog.take_pending_trade_action() {
        sys.pending_trade_action = Some(action);
    }

    // 排行榜刷新请求（draw 阶段产出，存入 pending，update 阶段发包）
    if let Some(tab) = sys.main_dialog.take_pending_ranking_refresh_tab() {
        sys.pending_ranking_refresh_tab = Some(tab);
    }

    // 装备物品请求（draw 阶段 Inventory→Character 拖拽产出，存入 pending，update 阶段发包）
    if let Some(unique_id) = sys.main_dialog.take_pending_equip_request() {
        sys.pending_equip_request = Some(unique_id);
    }

    // 卸下装备请求（draw 阶段 Character→Inventory 拖拽产出，存入 pending，update 阶段发包）
    if let Some(unique_id) = sys.main_dialog.take_pending_unequip_request() {
        sys.pending_unequip_request = Some(unique_id);
    }

    if let Some(action) = sys.npc_goods_dialog.take_action() {
        if let Some(s) = _world.query::<&UiState>().iter().next() {
            s.borrow_mut()
                .pending_actions
                .push(UiAction::NpcGoods(action));
        }
    }
    if let Some(action) = sys.npc_sub_goods_dialog.take_action() {
        if let Some(s) = _world.query::<&UiState>().iter().next() {
            s.borrow_mut()
                .pending_actions
                .push(UiAction::NpcSubGoods(action));
        }
    }

    // 组队对话框动作已移至 update 处理（含网络发包）

    // 数量框（modal，最上层）
    let mut amount_consumed = false;
    if sys.amount_box.is_visible() {
        amount_consumed = true;
        let r = sys.amount_box.update_and_draw();
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
    if sys.main_dialog.big_map_dialog_mut().is_visible() {
        sys.main_dialog.big_map_dialog_mut().update_and_draw();
    }

    // 文本输入对话框（modal，最上层，覆盖数量框）
    use crate::scenes::dialogs::game::text_input_dialog::TextInputResult;
    if sys.main_dialog.text_input_is_visible() {
        let result = sys.main_dialog.text_input_dialog_mut().update_and_draw();
        if !matches!(result, TextInputResult::None) {
            match result {
                TextInputResult::Ok(text) => {
                    let kind = sys.main_dialog.pending_text_input_kind();
                    sys.pending_text_input = Some((kind, text));
                }
                TextInputResult::Cancel => {
                    tracing::debug!("❌ 文本输入已取消");
                }
                TextInputResult::None => unreachable!(),
            }
            sys.main_dialog.reset_pending_text_input_kind();
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

    // 邀请确认弹窗（clone needed: draw_invite_confirm borrows &mut self）
    if let Some((kind, inviter, detail)) = sys.pending_invite.clone() {
        sys.draw_invite_confirm(&kind, &inviter, &detail);
    }

    Ok(())
}
