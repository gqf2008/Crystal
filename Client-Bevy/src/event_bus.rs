// 游戏事件总线（Bevy Events 实现，对应 macroquad 的 EventBus 5 队列）
use bevy::prelude::*;

#[derive(Event, Debug, Clone)]
pub enum GameEvent {
    /// 登录成功（mock 模式直接触发）
    LoginSuccess { account: String },
}

pub struct EventBusPlugin;

impl Plugin for EventBusPlugin {
    fn build(&self, app: &mut App) {
        // Bevy 0.19 事件系统：Observer + trigger
        app.add_observer(handle_game_events);
    }
}

fn handle_game_events(on: On<GameEvent>) {
    match on.event() {
        GameEvent::LoginSuccess { account } => {
            tracing::info!("🎮 登录成功: {}", account);
        }
    }
}
