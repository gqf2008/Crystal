use macroquad::prelude::get_time;

use crate::{
    components::{ActiveNpc, LocalPlayer, NpcCallCooldown, RenderPass},
    game::{GameContext, GameResult},
    network::handlers::NetworkEvent,
    scenes::dialogs::game::trust_merchant_dialog::MerchantItem,
    systems::LogicSystem,
    ui::ui_state::{UiAction, UiCommand, UiState},
};

use mir2_shared::enums::PanelType;

mod actions;
mod network_events;

/// 将 Unix 时间戳（秒）格式化为中文日期字符串
fn format_mail_date(timestamp: i64) -> String {
    if timestamp <= 0 {
        return "未知时间".to_string();
    }
    let days = timestamp / 86400;
    let secs_of_day = (timestamp % 86400).abs();
    let hours = secs_of_day / 3600;
    let mins = (secs_of_day % 3600) / 60;
    format!("{}天 {:02}:{:02}", days, hours, mins)
}

fn sys_chat(cmds: &mut Vec<UiCommand>, msg: impl Into<String>) {
    cmds.push(UiCommand::PushSystemChatLine(msg.into()));
}

#[derive(ecs_macros::LogicSystem)]
pub struct DialogSystem {}

impl Default for DialogSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl DialogSystem {
    pub fn new() -> Self {
        Self {}
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
}

impl LogicSystem for DialogSystem {
    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        network_events::pump_network_messages_to_ui(ctx);
        actions::process_ui_actions(ctx);

        let _ = ctx.world.query::<&RenderPass>().iter().next();

        Ok(())
    }
}
