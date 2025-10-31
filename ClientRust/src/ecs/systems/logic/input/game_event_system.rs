// ============================================================================

// Game Event System - 游戏事件系统

// ============================================================================use crate::ecs::systems::{System};

//use ggez::GameResult;

// 职责（Layer 1: 输入与网络层）：

// - 统一管理系统间事件通信pub struct GameEventSystem;

// - 分发游戏事件到各个系统

// - 事件队列管理impl System for GameEventSystem {

//    fn update(&mut self, world: &mut hecs::World, _delay_time: f32) -> GameResult {

// 设计理念：       

// - 作为事件总线，避免系统间直接耦合        Ok(())

// - 其他系统通过发布事件进行通信    }

// - 订阅者按需接收感兴趣的事件}
//
// ============================================================================

use crate::ecs::systems::System;
use crate::network::GameEvent;
use ggez::GameResult;
use hecs::World;
use std::collections::VecDeque;

/// 游戏内部事件类型
#[derive(Debug, Clone)]
pub enum InternalEvent {
    PlayerStateChanged { entity: hecs::Entity },
    MonsterStateChanged { entity: hecs::Entity },
    UIEvent { event_type: String, data: String },
    SoundTriggered { sound_id: u32, position: Option<(f32, f32)> },
    ParticleTriggered { effect_id: u32, position: (f32, f32) },
    CameraShake { intensity: f32, duration: f32 },
}

/// 游戏事件系统
pub struct GameEventSystem {
    network_events: VecDeque<GameEvent>,
    internal_events: VecDeque<InternalEvent>,
    events_processed_this_frame: usize,
}

impl GameEventSystem {
    pub fn new() -> Self {
        Self {
            network_events: VecDeque::new(),
            internal_events: VecDeque::new(),
            events_processed_this_frame: 0,
        }
    }
    
    pub fn publish_network_event(&mut self, event: GameEvent) {
        self.network_events.push_back(event);
    }
    
    pub fn publish_internal_event(&mut self, event: InternalEvent) {
        self.internal_events.push_back(event);
    }
    
    fn process_all_events(&mut self, world: &mut World) {
        self.events_processed_this_frame = 0;
        
        while let Some(event) = self.network_events.pop_front() {
            self.handle_network_event(world, event);
            self.events_processed_this_frame += 1;
        }
        
        while let Some(event) = self.internal_events.pop_front() {
            self.handle_internal_event(world, event);
            self.events_processed_this_frame += 1;
        }
    }
    
    fn handle_network_event(&mut self, _world: &mut World, event: GameEvent) {
        match event {
            GameEvent::Connected => {
                tracing::info!("📡 事件总线: 已连接到服务器");
            }
            GameEvent::Disconnected { reason } => {
                tracing::warn!("📡 事件总线: 断开连接 - {}", reason);
            }
            GameEvent::ChatMessage { sender, message, chat_type } => {
                tracing::info!("📡 聊天({:?}): {} - {}", chat_type, sender, message);
            }
            _ => {}
        }
    }
    
    fn handle_internal_event(&mut self, _world: &mut World, event: InternalEvent) {
        match event {
            InternalEvent::SoundTriggered { sound_id, .. } => {
                tracing::debug!("📡 音效: {}", sound_id);
            }
            _ => {}
        }
    }

    pub fn update(_world: &mut World, _delta_time: f32) {}
}

impl Default for GameEventSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for GameEventSystem {
   
    fn priority(&self) -> u32 {
        crate::ecs::systems::priority::GAME_EVENT
    }

    fn update(&mut self, world: &mut hecs::World, _delay_time: f32) -> GameResult {
        self.process_all_events(world);
        Ok(())
    }
}
