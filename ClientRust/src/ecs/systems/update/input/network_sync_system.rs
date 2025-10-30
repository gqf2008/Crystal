// ============================================================================
// Layer 1: Input & Network - NetworkSyncSystem
// Priority: 5
// ============================================================================
//
// **职责**：
// - 从网络层接收原始数据包
// - 将数据包写入 GlobalEvents.network_packets
// - 不处理游戏逻辑，只做数据转发
//
// **设计理念**：
// - NetworkSyncSystem 只负责 TCP  GlobalEvents 的数据搬运
// - GameClient 只负责数据包的序列化/反序列化
// - 游戏逻辑由其他系统（如 PlayerControlSystem）处理
//
// **数据流**：
// ```
// 网络线程  NetworkManager  mpsc::Receiver
//     
// NetworkSyncSystem::update() (读取 Receiver)
//     
// GlobalEvents.network_packets (写入 Vec)
//     
// 其他系统读取并处理数据包
//     
// EventCleanupSystem 清理
// ```
//
// ============================================================================

use hecs::World;
use ggez::GameResult;
use tokio::sync::mpsc;
use crate::ecs::systems::System;
use crate::ecs::components::{GlobalEvents, NetworkPacket};

/// 网络同步系统
/// 
/// 从网络管理器接收原始数据包并写入 GlobalEvents
pub struct NetworkSyncSystem {
    /// 网络数据包接收器（从 NetworkManager 接收）
    packet_receiver: Option<mpsc::UnboundedReceiver<Vec<u8>>>,
    
    /// 每帧最多接收的数据包数量（防止阻塞）
    max_packets_per_frame: usize,
}

impl NetworkSyncSystem {
    pub fn new() -> Self {
        Self {
            packet_receiver: None,
            max_packets_per_frame: 100,
        }
    }
    
    /// 设置数据包接收器
    pub fn set_receiver(&mut self, receiver: mpsc::UnboundedReceiver<Vec<u8>>) {
        self.packet_receiver = Some(receiver);
    }
    
    /// 从接收器拉取数据包并写入 GlobalEvents
    fn sync_packets(&mut self, world: &mut World) -> GameResult {
        let mut packet_count = 0;
        
        // 查询 GlobalEvents 组件
        let mut global_events = world.query::<&mut GlobalEvents>();
        if let Some((_, events)) = global_events.iter().next() {
            // 从网络接收器拉取数据包
            if let Some(receiver) = &mut self.packet_receiver {
                while packet_count < self.max_packets_per_frame {
                    match receiver.try_recv() {
                        Ok(data) => {
                            // 创建 NetworkPacket 并添加到 GlobalEvents
                            events.network_incoming.push(NetworkPacket {
                                packet_type: "RAW".to_string(), // 后续由解析系统识别
                                data,
                            });
                            packet_count += 1;
                        }
                        Err(mpsc::error::TryRecvError::Empty) => {
                            // 没有更多数据包
                            break;
                        }
                        Err(mpsc::error::TryRecvError::Disconnected) => {
                            tracing::error!(" 网络接收器已断开连接");
                            break;
                        }
                    }
                }
            }
            
            if packet_count > 0 {
                tracing::trace!(" NetworkSyncSystem: 本帧接收 {} 个数据包", packet_count);
            }
        } else {
            tracing::warn!(" GlobalEvents 组件未找到，无法同步网络数据包");
        }
        
        Ok(())
    }
}

impl Default for NetworkSyncSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for NetworkSyncSystem {
    fn name(&self) -> &'static str {
        "NetworkSyncSystem"
    }
    
    fn priority(&self) -> u32 {
        crate::ecs::systems::priority::NETWORK_RECV
    }

    fn update(&mut self, world: &mut hecs::World, _delay_time: f32) -> GameResult {
        self.sync_packets(world)
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
    }
}
