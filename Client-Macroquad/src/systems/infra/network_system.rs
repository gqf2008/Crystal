use crate::{
    game::{GameContext, GameResult},
    network::handlers::NetworkEvent,
    systems::LogicSystem,
};

/// NetworkSystem - ECS 网络系统
///
/// 职责：
/// - 从 `GameContext.net` 拉取入站 NetworkEvent
/// - 写入 `EventBus.network_events`，供其他系统消费
/// - 同步 `GameContext.network.connected` 状态（Connected/Disconnected）
///
/// 设计目标：
/// - 默认“未连接”时完全 no-op，不影响 test_game_scene。
#[derive(ecs_macros::LogicSystem)]
pub struct NetworkSystem;

impl Default for NetworkSystem {
    fn default() -> Self {
        Self
    }
}

impl LogicSystem for NetworkSystem {
    fn update(&mut self, ctx: &mut GameContext, _delay_time: f32) -> GameResult {
        let Some(net) = ctx.net() else {
            return Ok(());
        };

        // 拉取所有入站事件（非阻塞）
        let events: Vec<NetworkEvent> = net.recv_all();
        if events.is_empty() {
            return Ok(());
        }

        // 写入事件总线 + 更新连接状态
        for event in events {
            match &event {
                NetworkEvent::Connected => {
                    ctx.network.connected = true;
                }
                NetworkEvent::Disconnected { .. } => {
                    ctx.network.connected = false;
                }
                _ => {}
            }
            ctx.events_mut().send_network(event);
        }

        Ok(())
    }
}
