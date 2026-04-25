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

mod update;
mod draw;

#[derive(ecs_macros::RenderSystem)]
pub struct UIRenderSystem {
    pub(crate) main_dialog: MainDialog,
    pub(crate) npc_dialog: NpcDialogHybrid,
    pub(crate) npc_goods_dialog: NpcGoodsDialogHybrid,
    pub(crate) npc_sub_goods_dialog: NpcGoodsDialogHybrid,
    pub(crate) amount_box: AmountBoxHybrid,
    pub(crate) timer_dialog: crate::scenes::dialogs::game::timer_dialog::TimerDialogHybrid,
    pub(crate) chat_notice_dialog: crate::scenes::dialogs::game::chat_notice_dialog::ChatNoticeDialogHybrid,
    pub(crate) notice_dialog: crate::scenes::dialogs::game::notice_dialog::NoticeDialogHybrid,
    pub(crate) roll_dialog: crate::scenes::dialogs::game::roll_dialog::RollDialogHybrid,
    pub(crate) dura_status_dialog: crate::scenes::dialogs::game::dura_status_dialog::DuraStatusDialogHybrid,
    pub(crate) npc_drop_dialog: crate::scenes::dialogs::game::npc_drop_dialog::NPCDropDialogHybrid,
    pub(crate) guild_territory_dialog: crate::scenes::dialogs::game::guild_territory_dialog::GuildTerritoryDialogHybrid,
    pub(crate) keyboard_layout_dialog: crate::scenes::dialogs::game::keyboard_layout_dialog::KeyboardLayoutDialogHybrid,
    pub(crate) npc_awake_dialog: crate::scenes::dialogs::game::npc_awake_dialog::NPCAwakeDialogHybrid,
    pub(crate) craft_dialog: crate::scenes::dialogs::game::craft_dialog::CraftDialogHybrid,
    pub(crate) refine_dialog: crate::scenes::dialogs::game::refine_dialog::RefineDialogHybrid,
    pub(crate) item_rental_dialog: crate::scenes::dialogs::game::item_rental_dialog::ItemRentalDialogHybrid,
    pub(crate) trust_merchant_dialog: crate::scenes::dialogs::game::trust_merchant_dialog::TrustMerchantDialogHybrid,

    pub(crate) npc_z_order: Vec<NpcUiLayer>,

    pub(crate) ui_stack_top: UiStackTop,

    /// 暂存的交易动作（由 draw 阶段产出，由 update 阶段发包）
    pub(crate) pending_trade_action: Option<crate::scenes::dialogs::game::trade_dialog::TradeAction>,

    /// 暂存的文本输入结果（由 draw 阶段产出，由 update 阶段发包）
    pub(crate) pending_text_input: Option<(crate::scenes::dialogs::game::main_dialog::TextInputKind, String)>,

    /// 暂存的排行榜刷新请求（由 draw 阶段产出，由 update 阶段发包）
    pub(crate) pending_ranking_refresh_tab: Option<u8>,

    /// 暂存的装备请求（由 draw 阶段 Inventory→Character 拖拽产出，由 update 阶段发包）
    pub(crate) pending_equip_request: Option<u64>,

    /// 待显示的邀请确认弹窗
    pub(crate) pending_invite: Option<(crate::ui::ui_state::InviteKind, String, String)>,

    /// 暂存的邀请确认结果（由 draw 阶段产出，由 update 阶段发包）
    pub(crate) pending_invite_reply: Option<(crate::ui::ui_state::InviteKind, bool)>,

    /// 暂存的卸下装备请求（由 draw 阶段 Character→Inventory 拖拽产出，由 update 阶段发包）
    pub(crate) pending_unequip_request: Option<u64>,

    /// 缓存的任务信息（来自 NewQuestInfo，等待 QuestAccepted 到来时使用）
    pub(crate) cached_quest_info: std::collections::HashMap<u32, (String, String, String, u32, u64, u32)>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum NpcUiLayer {
    Dialog,
    Goods,
    SubGoods,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum UiStackTop {
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
            pending_invite: None,
            pending_invite_reply: None,
            cached_quest_info: std::collections::HashMap::new(),
        }
    }

    pub(crate) fn bring_npc_layer_to_front(&mut self, layer: NpcUiLayer) {
        if let Some(i) = self.npc_z_order.iter().position(|&x| x == layer) {
            self.npc_z_order.remove(i);
        }
        self.npc_z_order.push(layer);
    }

    pub(crate) fn npc_layer_visible(&self, layer: NpcUiLayer) -> bool {
        match layer {
            NpcUiLayer::Dialog => self.npc_dialog.is_visible(),
            NpcUiLayer::Goods => self.npc_goods_dialog.is_visible(),
            NpcUiLayer::SubGoods => self.npc_sub_goods_dialog.is_visible(),
        }
    }

    pub(crate) fn npc_layer_mouse_over(&self, layer: NpcUiLayer, mouse_pos: macroquad::prelude::Vec2) -> bool {
        match layer {
            NpcUiLayer::Dialog => self.npc_dialog.is_mouse_over(mouse_pos),
            NpcUiLayer::Goods => self.npc_goods_dialog.is_mouse_over(mouse_pos),
            NpcUiLayer::SubGoods => self.npc_sub_goods_dialog.is_mouse_over(mouse_pos),
        }
    }

    pub(crate) fn npc_mouse_over_any(&self, mouse_pos: macroquad::prelude::Vec2) -> bool {
        (self.npc_dialog.is_visible() && self.npc_dialog.is_mouse_over(mouse_pos))
            || (self.npc_goods_dialog.is_visible() && self.npc_goods_dialog.is_mouse_over(mouse_pos))
            || (self.npc_sub_goods_dialog.is_visible() && self.npc_sub_goods_dialog.is_mouse_over(mouse_pos))
    }

    pub(crate) fn close_npc_related_dialogs(&mut self) {
        self.npc_dialog.hide();
        self.npc_goods_dialog.hide();
        self.npc_sub_goods_dialog.hide();
        self.amount_box.hide();
    }

    pub(crate) fn draw_invite_confirm(&mut self, kind: &crate::ui::ui_state::InviteKind, _inviter: &str, detail: &str) {
        use macroquad::prelude::{draw_rectangle, draw_rectangle_lines, screen_height, screen_width, is_mouse_button_pressed, mouse_position, MouseButton, WHITE};

        let sw = screen_width();
        let sh = screen_height();
        let (mx, my) = mouse_position();

        let w = 320.0;
        let h = 100.0;
        let x = (sw - w) / 2.0;
        let y = (sh - h) / 2.0;

        draw_rectangle(x, y, w, h, Color::from_rgba(25, 25, 40, 240));
        draw_rectangle_lines(x, y, w, h, 1.0, Color::from_rgba(100, 100, 120, 200));

        crate::ui::text_renderer::draw_text_cn(detail, x + 15.0, y + 25.0, 14.0, WHITE);

        let btn_w = 100.0;
        let btn_h = 28.0;
        let btn_y = y + h - 40.0;
        let accept_x = x + w / 2.0 - btn_w - 10.0;
        let decline_x = x + w / 2.0 + 10.0;

        let accept_hover = mx >= accept_x && mx <= accept_x + btn_w && my >= btn_y && my <= btn_y + btn_h;
        let decline_hover = mx >= decline_x && mx <= decline_x + btn_w && my >= btn_y && my <= btn_y + btn_h;

        draw_rectangle(accept_x, btn_y, btn_w, btn_h,
            if accept_hover { Color::from_rgba(80, 160, 80, 255) } else { Color::from_rgba(60, 120, 60, 255) });
        crate::ui::text_renderer::draw_text_cn("接受", accept_x + 30.0, btn_y + 18.0, 14.0, WHITE);

        draw_rectangle(decline_x, btn_y, btn_w, btn_h,
            if decline_hover { Color::from_rgba(160, 60, 60, 255) } else { Color::from_rgba(120, 40, 40, 255) });
        crate::ui::text_renderer::draw_text_cn("拒绝", decline_x + 30.0, btn_y + 18.0, 14.0, WHITE);

        if is_mouse_button_pressed(MouseButton::Left) {
            if accept_hover {
                self.pending_invite_reply = Some((*kind, true));
                self.pending_invite = None;
            } else if decline_hover {
                self.pending_invite_reply = Some((*kind, false));
                self.pending_invite = None;
            }
        }
    }
}

impl RenderSystem for UIRenderSystem {
    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        update::update(self, ctx, _dt)
    }

    fn draw(&mut self, _world: &hecs::World) -> GameResult {
        draw::draw(self, _world)
    }
}
