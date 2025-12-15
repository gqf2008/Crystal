use macroquad::prelude::{
    is_key_pressed, is_mouse_button_down, is_mouse_button_pressed, mouse_position, mouse_wheel,
    vec2, KeyCode, MouseButton,
};

use crate::components::{RenderPass, RenderStage, UiWorldInputBlock};
use crate::game::{GameContext, GameResult};
use crate::scenes::dialogs::game::{
    amount_box::AmountBoxHybrid, npc_dialog::NpcDialogHybrid,
    npc_goods_dialog::NpcGoodsDialogHybrid, MainDialog,
};
use crate::systems::RenderSystem;
use crate::ui::ui_state::{UiAction, UiCommand, UiState};

#[derive(ecs_macros::RenderSystem)]
pub struct UIRenderSystem {
    main_dialog: MainDialog,
    npc_dialog: NpcDialogHybrid,
    npc_goods_dialog: NpcGoodsDialogHybrid,
    npc_sub_goods_dialog: NpcGoodsDialogHybrid,
    amount_box: AmountBoxHybrid,
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
        }
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
                    } else {
                        self.npc_goods_dialog
                            .new_goods(items, rate, panel_type, hide_added_stats);
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

        // 绘制完整 UI（复用现有 hybrid UI 代码）
        self.main_dialog.update_and_draw();
        let ui_consumed = self.main_dialog.show_dialogs();

        // UI -> ECS：小地图点击自动寻路
        if let Some(target) = self.main_dialog.take_pending_auto_path_target() {
            if let Some((_e, s)) = _world.query::<&UiState>().iter().next() {
                s.borrow_mut().pending_auto_path_target = Some(target);
            }
        }

        // NPC 对话框（非 modal，位于主 UI 之上）
        let mut npc_dialog_consumed = false;
        if self.npc_dialog.is_visible() {
            npc_dialog_consumed = true;
            let action = self.npc_dialog.update_and_draw();
            if !matches!(
                action,
                crate::scenes::dialogs::game::npc_dialog::NpcDialogAction::None
            ) {
                if let Some((_e, s)) = _world.query::<&UiState>().iter().next() {
                    s.borrow_mut()
                        .pending_actions
                        .push(UiAction::NpcDialog(action));
                }
            }
        }

        // NPC 商店窗口：放在更上层
        let input_enabled = !self.amount_box.is_visible();
        let npc_consumed = self
            .npc_goods_dialog
            .update_and_draw_with_input(None, input_enabled);
        let npc_sub_consumed = self
            .npc_sub_goods_dialog
            .update_and_draw_with_input(None, input_enabled);

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
        Ok(())
    }
}
