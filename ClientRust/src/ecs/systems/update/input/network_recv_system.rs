// ============================================================================use crate::ecs::systems::{System};

// Network Receive System - 网络接收系统use ggez::GameResult;

// ============================================================================

//pub struct NetworkRecvSystem;

// 职责（Layer 1: 输入与网络层）：

// - 从网络层接收数据包impl System for NetworkRecvSystem {

// - 解包并转换为游戏事件    fn update(&mut self, world: &mut hecs::World, _delay_time: f32) -> GameResult {

// - 将事件放入事件队列供其他系统处理       

//        Ok(())

// 特点：    }

// - 异步IO，不阻塞游戏主循环}
// - 优先级最高(50)，确保网络数据及时处理
//
// 数据流：
// NetworkManager → GameEvent → NetworkRecvSystem → GameEventSystem → 其他系统
//
// ============================================================================

use crate::ecs::systems::System;
use crate::network::GameEvent;
use ggez::GameResult;
use hecs::World;
use tokio::sync::mpsc;
use std::collections::VecDeque;

/// 网络接收系统
pub struct NetworkRecvSystem {
    /// 网络事件接收器（从 NetworkManager 接收）
    event_receiver: Option<mpsc::UnboundedReceiver<GameEvent>>,
    
    /// 缓冲的事件队列（避免每帧处理过多事件）
    event_buffer: VecDeque<GameEvent>,
    
    /// 每帧最多处理的事件数量
    max_events_per_frame: usize,
}

impl NetworkRecvSystem {
    pub fn new() -> Self {
        Self {
            event_receiver: None,
            event_buffer: VecDeque::new(),
            max_events_per_frame: 100,
        }
    }
    
    /// 设置网络事件接收器
    pub fn set_receiver(&mut self, receiver: mpsc::UnboundedReceiver<GameEvent>) {
        self.event_receiver = Some(receiver);
    }
    
    /// 从接收器拉取新事件到缓冲区
    fn pull_events(&mut self) {
        if let Some(receiver) = &mut self.event_receiver {
            while let Ok(event) = receiver.try_recv() {
                self.event_buffer.push_back(event);
                
                if self.event_buffer.len() > 1000 {
                    tracing::warn!("⚠️ 网络事件缓冲区溢出，丢弃旧事件");
                    self.event_buffer.pop_front();
                }
            }
        }
    }
    
    /// 处理缓冲的事件
    fn process_events(&mut self, world: &mut World) {
        let mut processed = 0;
        
        while let Some(event) = self.event_buffer.pop_front() {
            self.handle_event(world, event);
            
            processed += 1;
            if processed >= self.max_events_per_frame {
                tracing::debug!("📦 本帧已处理 {} 个网络事件，剩余 {} 个待处理",
                    processed, self.event_buffer.len());
                break;
            }
        }
    }
    
    /// 处理单个游戏事件
    fn handle_event(&mut self, _world: &mut World, event: GameEvent) {
        match &event {
            GameEvent::Connected => {
                tracing::info!("🌐 已连接到服务器");
            }
            GameEvent::Disconnected { reason } => {
                tracing::warn!("🔌 与服务器断开连接: {}", reason);
            }
            GameEvent::PlayerMoved { location } => {
                tracing::debug!("🚶 玩家移动: ({}, {})", location.x, location.y);
            }
            GameEvent::ObjectSpawned { object } => {
                use crate::network::GameObject;
                let name = match object {
                    GameObject::Player { name, .. } => name,
                    GameObject::Monster { name, .. } => name,
                    GameObject::Npc { name, .. } => name,
                    GameObject::Item { .. } => "Item",
                };
                tracing::debug!("👤 对象生成: {}", name);
            }
            GameEvent::ObjectRemoved { object_id } => {
                tracing::debug!("🗑️ 对象移除: {}", object_id);
            }
            GameEvent::ChatReceived { message } => {
                tracing::debug!("💬 收到聊天: {} - {}", message.sender, message.text);
            }
            _ => {}
        }
    }

}

impl Default for NetworkRecvSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for NetworkRecvSystem {
    
    fn priority(&self) -> u32 {
        crate::ecs::systems::priority::NETWORK_RECV
    }

    fn update(&mut self, world: &mut hecs::World, _delay_time: f32) -> GameResult {
        self.pull_events();
        self.process_events(world);
        Ok(())
    }
}
