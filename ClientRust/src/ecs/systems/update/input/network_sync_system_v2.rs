// ============================================================================
// NetworkSyncSystem - 网络同步系统 (重构版)
// ============================================================================
//
// **新架构职责**:
// 1. 从 NetworkManager 接收原始数据包字节 (通过 mpsc channel)
// 2. 使用 protocol::dispatch_packet() 解析为 GameEvent
// 3. 将 GameEvent 写入 GlobalEvents.game_events
// 4. 由 GameEventSystem 从 GlobalEvents 读取并创建 ECS 实体
//
// **架构流程**:
// ```
// NetworkManager (Tokio 线程)
//     ↓ mpsc::channel<Vec<u8>>
// NetworkSyncSystem (ECS 系统, Priority 150)
//     ↓ protocol::dispatch_packet()
// GlobalEvents.game_events: Vec<GameEvent>
//     ↓
// GameEventSystem (Priority 510)
//     ↓ 创建/更新 ECS 实体
// ECS World
// ```
//
// **为什么不需要 NetworkPacketParserSystem?**
// - protocol.rs 已经提供了完整的解析功能
// - dispatch_packet() 会调用 PacketHandler trait 方法
// - GameClient 实现 PacketHandler,生成 GameEvent
// - 直接在 NetworkSyncSystem 中完成解析更高效
//
// ============================================================================

use hecs::World;
use ggez::GameResult;
use std::sync::mpsc;  // 🔄 改用 std::sync::mpsc

use crate::ecs::systems::System;
use crate::ecs::components::GlobalEvents;
use crate::network::{GameClient, GameEvent};
use crate::network::protocol::{self, PacketHeader};

/// 网络同步系统
/// 
/// 负责对接网络层和游戏场景:
/// 1. 接收原始网络数据包
/// 2. 解析为 GameEvent
/// 3. 写入 GlobalEvents
pub struct NetworkSyncSystem {
    /// 原始数据包接收器 (从 NetworkManager)
    /// 🔄 改用 std::sync::mpsc::Receiver
    packet_receiver: Option<mpsc::Receiver<Vec<u8>>>,
    
    /// GameEvent 发送器 (发送到 GlobalEvents)
    /// 可选: 如果设置,则直接发送; 否则写入 GlobalEvents 组件
    /// 🔄 改用 std::sync::mpsc::Sender
    event_sender: Option<mpsc::Sender<GameEvent>>,
    
    /// 每帧最多处理的数据包数量
    max_packets_per_frame: usize,
    
    /// 协议处理器 (GameClient 作为 PacketHandler)
    game_client: GameClient,
    
    /// 统计: 总接收包数
    total_packets: u64,
    
    /// 统计: 总解析错误数
    parse_errors: u64,
}

impl NetworkSyncSystem {
    pub fn new() -> Self {
        Self {
            packet_receiver: None,
            event_sender: None,
            max_packets_per_frame: 100,
            game_client: GameClient::new(),
            total_packets: 0,
            parse_errors: 0,
        }
    }
    
    /// 设置数据包接收器 (从 NetworkManager)
    /// 🔄 使用 std::sync::mpsc::Receiver
    pub fn set_packet_receiver(&mut self, receiver: mpsc::Receiver<Vec<u8>>) {
        self.packet_receiver = Some(receiver);
    }
    
    /// 设置事件发送器 (可选,用于直接发送而不是写入 GlobalEvents)
    /// 🔄 使用 std::sync::mpsc::Sender
    pub fn set_event_sender(&mut self, sender: mpsc::Sender<GameEvent>) {
        self.game_client.set_event_channel(sender.clone());  // 🔧 克隆一份给 GameClient
        self.event_sender = Some(sender);  // 存储原始版本
    }
    
    /// 处理单个数据包: 原始字节 → GameEvent
    fn process_packet(&mut self, payload: &[u8]) -> Result<(), String> {
        if payload.len() < 4 {
            return Err(format!("数据包太短: {} 字节", payload.len()));
        }
        
        // 解析头部
        let header = PacketHeader {
            length: u16::from_le_bytes([payload[0], payload[1]]),
            opcode: i16::from_le_bytes([payload[2], payload[3]]),
        };
        
        // 使用 protocol::dispatch_packet 解析
        // GameClient 实现了 PacketHandler trait
        // 解析成功后会自动调用 GameClient 的 on_* 方法
        // GameClient 在 on_* 方法中会发送 GameEvent
        if let Err(e) = protocol::dispatch_packet(header, payload, &mut self.game_client) {
            self.parse_errors += 1;
            return Err(format!("解析失败 (opcode={}): {}", header.opcode, e));
        }
        
        Ok(())
    }
    
    /// 从接收器拉取并处理数据包
    fn sync_and_parse_packets(&mut self, world: &mut World) -> GameResult {
        let mut packet_count = 0;
        let mut error_count = 0;
        
        // 1. 从 NetworkManager 拉取原始数据包
        let mut packets_to_process = Vec::new();
        if let Some(receiver) = &mut self.packet_receiver {
            while packet_count < self.max_packets_per_frame {
                match receiver.try_recv() {
                    Ok(data) => {
                        packets_to_process.push(data);
                        packet_count += 1;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,  // 🔄 std::sync::mpsc
                    Err(mpsc::TryRecvError::Disconnected) => {
                        tracing::error!("❌ 网络接收器已断开");
                        break;
                    }
                }
            }
        }
        
        // 2. 解析数据包 → GameEvent
        // GameClient 会自动通过 event_tx 发送 GameEvent
        for packet_data in packets_to_process {
            self.total_packets += 1;
            
            if let Err(e) = self.process_packet(&packet_data) {
                error_count += 1;
                // 只在错误超过一定比例时才警告 (避免日志刷屏)
                if error_count < 5 {
                    tracing::warn!("⚠️ {}", e);
                }
            }
        }
        
        // 3. 如果没有设置 event_sender,则需要从 GameClient 获取 GameEvent
        //    并手动写入 GlobalEvents (备用方案)
        if self.event_sender.is_none() && packet_count > 0 {
            // TODO: 实现备用方案 (直接写入 GlobalEvents.game_events)
            tracing::warn!("⚠️ NetworkSyncSystem: event_sender 未设置,GameEvent 可能丢失");
        }
        
        if packet_count > 0 {
            tracing::debug!(
                "📡 NetworkSyncSystem: 处理 {} 个数据包 ({} 错误)",
                packet_count,
                error_count
            );
        }
        
        Ok(())
    }
    
    /// 获取统计信息
    pub fn stats(&self) -> (u64, u64) {
        (self.total_packets, self.parse_errors)
    }
}

impl Default for NetworkSyncSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for NetworkSyncSystem {
    fn update(&mut self, world: &mut World, _delay_time: f32) -> GameResult {
        self.sync_and_parse_packets(world)
    }
    
    fn priority(&self) -> u32 {
        150 // Stage 1: Input & Network
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_sync_system_creation() {
        let system = NetworkSyncSystem::new();
        assert_eq!(system.max_packets_per_frame, 100);
        assert!(system.packet_receiver.is_none());
        assert_eq!(system.total_packets, 0);
    }
    
    #[test]
    fn test_packet_too_short() {
        let mut system = NetworkSyncSystem::new();
        let result = system.process_packet(&[0x01, 0x02]); // 只有2字节
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("太短"));
    }
}
