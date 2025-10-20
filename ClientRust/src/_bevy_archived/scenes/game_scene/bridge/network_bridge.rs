// Network Bridge - 网络线程和 Bevy ECS 之间的桥接
// 
// 功能说明:
// 在网络线程和主线程 (Bevy ECS) 之间传递包
// 使用 Bevy Observer 模式处理网络事件
//
// 复用:
// - 完全复用 SharedRust 的包定义
// - 复用 network::NetworkManager
//
// 参考:
// - ClientRust/src/network/network_manager.rs - 网络管理器
// - SharedRust/src/packets/ - 包定义

use bevy::prelude::*;
use std::sync::{Arc, Mutex};

// 复用现有的网络管理器
use crate::network::network_manager::NetworkManager;

// 复用 SharedRust 的包定义
use super::packet_types::{ServerPacket, ClientPacket, ServerPacketEvent, ClientPacketEvent};

/// Bevy 资源: 网络桥接
/// 
/// 管理网络线程和主线程之间的通信
#[derive(Resource)]
pub struct NetworkBridge {
    /// 网络管理器引用 (线程安全)
    network_manager: Option<Arc<Mutex<NetworkManager>>>,
    
    /// 服务器包缓冲区 (简化版,后续可改为 channel)
    server_packets: Vec<ServerPacket>,
}

impl NetworkBridge {
    /// 创建新的 NetworkBridge
    pub fn new() -> Self {
        Self {
            network_manager: None,
            server_packets: Vec::new(),
        }
    }
    
    /// 设置网络管理器
    pub fn set_network_manager(&mut self, manager: Arc<Mutex<NetworkManager>>) {
        self.network_manager = Some(manager);
    }
    
    /// 尝试接收一个服务器包 (非阻塞)
    fn try_recv_packet(&mut self) -> Option<ServerPacket> {
        // TODO: 实现真正的网络包接收
        // 需要从 NetworkManager 的接收队列中读取
        self.server_packets.pop()
    }
    
    /// 发送客户端包
    fn send_packet(&mut self, _packet: ClientPacket) {
        // TODO: 实现真正的网络包发送
        // 需要调用 NetworkManager 的发送方法
        warn!("TODO: 实现 send_packet");
    }
}

impl Default for NetworkBridge {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== Bevy 系统 ====================

/// Bevy 系统: 网络 → Bevy (接收服务器包)
/// 
/// 从网络桥接接收服务器包,发送为 Observer 触发器
/// 在 Update stage 运行
pub fn network_to_bevy_system(
    mut network_bridge: ResMut<NetworkBridge>,
    mut commands: Commands,
) {
    // 非阻塞地接收所有待处理的包
    let mut count = 0;
    while let Some(packet) = network_bridge.try_recv_packet() {
        commands.trigger(ServerPacketEvent { packet });
        count += 1;
        
        // 限制单帧处理数量 (防止卡顿)
        if count >= 100 {
            warn!("单帧接收包数量达到上限 (100), 剩余包将在下一帧处理");
            break;
        }
    }
    
    if count > 0 {
        debug!("本帧接收 {} 个服务器包", count);
    }
}

/// Bevy 系统: Bevy → 网络 (发送客户端包)
/// 
/// 监听 ClientPacketEvent,通过网络桥接发送到服务器
/// 在 PostUpdate stage 运行
pub fn bevy_to_network_system(
    mut network_bridge: ResMut<NetworkBridge>,
    trigger: Trigger<ClientPacketEvent>,
) {
    let event = trigger.event();
    network_bridge.send_packet(event.packet.clone());
}

/// Bevy 系统: 初始化网络桥接
/// 
/// 在 GameScene 启动时调用
pub fn setup_network_bridge(mut commands: Commands) {
    let bridge = NetworkBridge::new();
    commands.insert_resource(bridge);
    info!("✅ NetworkBridge 资源已初始化");
}

// ==================== 示例使用 ====================

/// 网络包处理示例
/// 
/// 展示如何监听和处理服务器包事件
#[allow(dead_code)]
pub fn example_packet_handler_system(
    trigger: Trigger<ServerPacketEvent>,
) {
    let event = trigger.event();
    match &event.packet {
        ServerPacket::Unknown => {
            info!("收到未知包");
        }
        _ => {
            // TODO: 实现具体的包处理
            debug!("收到服务器包: {:?}", event.packet);
        }
    }
}
