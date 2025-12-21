use macroquad::prelude::{
    is_key_pressed, is_mouse_button_down, is_mouse_button_pressed, mouse_position, mouse_wheel,
    screen_height, screen_width, vec2, Color, KeyCode, MouseButton, WHITE,
};

use crate::components::{RenderPass, RenderStage, ResourceInitState, SceneExitBlock, UiWorldInputBlock};
use crate::game::{GameContext, GameResult};
use crate::scenes::dialogs::game::{
    amount_box::AmountBoxHybrid, npc_dialog::NpcDialogHybrid,
    npc_goods_dialog::NpcGoodsDialogHybrid, MainDialog,
};
use crate::systems::RenderSystem;
use crate::ui::text_renderer::{draw_text_cn, draw_text_with_outline, measure_text_cn};
use crate::ui::ui_state::{UiAction, UiCommand, UiState};

#[derive(ecs_macros::RenderSystem)]
pub struct UIRenderSystem {
    main_dialog: MainDialog,
    npc_dialog: NpcDialogHybrid,
    npc_goods_dialog: NpcGoodsDialogHybrid,
    npc_sub_goods_dialog: NpcGoodsDialogHybrid,
    amount_box: AmountBoxHybrid,

    npc_z_order: Vec<NpcUiLayer>,

    ui_stack_top: UiStackTop,
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

impl UIRenderSystem {
    pub fn new() -> Self {
        let mut main_dialog = MainDialog::new();
        main_dialog.load_native_textures();
        Self {
            main_dialog,
            npc_dialog: NpcDialogHybrid::new(),
            npc_goods_dialog: NpcGoodsDialogHybrid::new(),
            npc_sub_goods_dialog: NpcGoodsDialogHybrid::new(),
            amount_box: AmountBoxHybrid::new(),

            // 默认：SubGoods 在最上层（如果打开）。
            npc_z_order: vec![NpcUiLayer::Dialog, NpcUiLayer::Goods, NpcUiLayer::SubGoods],

            ui_stack_top: UiStackTop::Main,
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
            .map(|(_, s)| std::mem::take(&mut s.borrow_mut().pending_commands))
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
                UiCommand::ShowNpcDialog { dialog } => {
                    self.npc_goods_dialog.hide();
                    self.npc_sub_goods_dialog.hide();
                    self.amount_box.hide();
                    if let Some((_e, s)) = ctx.world.query::<&UiState>().iter().next() {
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
                    if let Some((_e, s)) = ctx.world.query::<&UiState>().iter().next() {
                        s.borrow_mut().amount_box_buy_uid = Some(buy_uid);
                    }
                }
                UiCommand::HideAmountBox => {
                    self.amount_box.hide();
                    if let Some((_e, s)) = ctx.world.query::<&UiState>().iter().next() {
                        s.borrow_mut().amount_box_buy_uid = None;
                    }
                }
                UiCommand::HideNpcGoodsSub => {
                    self.npc_sub_goods_dialog.hide();
                }
                UiCommand::HideNpcGoods => {
                    self.npc_goods_dialog.hide();
                }
            }
        }

        // 2) 同步表现层数据 -> 具体 UI（小地图）
        let (minimap_world_size, minimap_player_pos, minimap_player_dir_radians) = {
            ctx.world
                .query::<&UiState>()
                .iter()
                .next()
                .map(|(_, s)| {
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

        // 2.5) 同步主面板红/蓝血（HP/MP）到真实 ECS 数据
        {
            use crate::components::{Health, LocalPlayer, Mana};

            let mut q = ctx.world.query::<(&LocalPlayer, &Health)>();
            if let Some((e, (_lp, hp))) = q.iter().next() {
                let (mp_cur, mp_max) = ctx
                    .world
                    .get::<&Mana>(e)
                    .map(|mp| (mp.current, mp.max))
                    .unwrap_or((0, 1));

                self.main_dialog
                    .set_vitals(hp.current, hp.max, mp_cur, mp_max);
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
                if let Some((_e, s)) = ctx.world.query::<&UiState>().iter().next() {
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
            .map(|(_, s)| s.borrow().ui_mouse_captured)
            .unwrap_or(false);
        if (left_pressed || right_pressed) && ui_over {
            ui_mouse_captured = true;
        }
        if ui_mouse_captured && !mouse_button_down {
            ui_mouse_captured = false;
        }
        if let Some((_e, s)) = ctx.world.query::<&UiState>().iter().next() {
            s.borrow_mut().ui_mouse_captured = ui_mouse_captured;
        }

        // 写入 ECS 单例：UiWorldInputBlock（与 UiState 同挂在 RenderPass 实体上）
        if let Some((pass_entity, _)) = ctx.world.query::<&UiState>().iter().next() {
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
            .map(|(_, s)| s.borrow().ui_consumed_last_frame)
            .unwrap_or(false);

        ctx.input_blocked = ui_input_active
            || ui_mouse_captured
            || (wheel_y != 0.0 && ui_over)
            || ui_consumed_last_frame
            || amount_box_visible;

        // 5) 更新 UiState 的可观察标记（供 GameScene 做 ESC 退出 gating 等）
        if let Some((_e, s)) = ctx.world.query::<&UiState>().iter().next() {
            let mut s = s.borrow_mut();
            s.ui_input_active = ui_input_active;
            s.any_modal_or_popup_open = self.main_dialog.any_popup_open()
                || self.npc_dialog.is_visible()
                || self.npc_goods_dialog.is_visible()
                || self.npc_sub_goods_dialog.is_visible()
                || self.amount_box.is_visible();
        }

        // Scene 退出 gating：避免 Scene 直接读取 UiState 内部结构。
        if let Some((pass_entity, _)) = ctx.world.query::<&UiState>().iter().next() {
            let any_modal_or_popup_open = ctx
                .world
                .get::<&UiState>(pass_entity)
                .map(|s| s.borrow().any_modal_or_popup_open)
                .unwrap_or(false);
            if let Ok(mut b) = ctx.world.get::<&mut SceneExitBlock>(pass_entity) {
                b.block_escape_exit = any_modal_or_popup_open;
            }
        }

        Ok(())
    }

    fn draw(&mut self, _world: &hecs::World) -> GameResult {
        let pass = _world
            .query::<&RenderPass>()
            .iter()
            .next()
            .map(|(_, pass)| *pass)
            .unwrap_or_default();

        if pass.stage != RenderStage::Ui {
            return Ok(());
        }

        let initialized = _world
            .query::<&ResourceInitState>()
            .iter()
            .next()
            .map(|(_, s)| s.initialized)
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
                                    if let Some((_e, s)) = _world.query::<&UiState>().iter().next() {
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
                                    if let Some((_e, s)) = _world.query::<&UiState>().iter().next() {
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

        // UI -> ECS：小地图点击自动寻路（在 show_dialogs 后取，保证同帧可用）
        if let Some(target) = self.main_dialog.take_pending_auto_path_target() {
            if let Some((_e, s)) = _world.query::<&UiState>().iter().next() {
                s.borrow_mut().pending_auto_path_target = Some(target);
            }
        }

        if let Some(action) = self.npc_goods_dialog.take_action() {
            if let Some((_e, s)) = _world.query::<&UiState>().iter().next() {
                s.borrow_mut()
                    .pending_actions
                    .push(UiAction::NpcGoods(action));
            }
        }
        if let Some(action) = self.npc_sub_goods_dialog.take_action() {
            if let Some((_e, s)) = _world.query::<&UiState>().iter().next() {
                s.borrow_mut()
                    .pending_actions
                    .push(UiAction::NpcSubGoods(action));
            }
        }

        // 数量框（modal，最上层）
        let mut amount_consumed = false;
        if self.amount_box.is_visible() {
            amount_consumed = true;
            let r = self.amount_box.update_and_draw();
            if !matches!(
                r,
                crate::scenes::dialogs::game::amount_box::AmountBoxResult::None
            ) {
                if let Some((_e, s)) = _world.query::<&UiState>().iter().next() {
                    s.borrow_mut().pending_actions.push(UiAction::AmountBox(r));
                }
            }
        }

        if let Some((_e, s)) = _world.query::<&UiState>().iter().next() {
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
            if let Some((_e, (_lp, hp, ds))) = q.iter().next() {
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
