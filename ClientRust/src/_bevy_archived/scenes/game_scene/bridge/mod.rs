// GameScene 桥接层
// 
// 功能说明:
// 在 Bevy ECS 和传统对象系统之间建立桥接
// 
// 子模块:
// - object_sync.rs - MapObject ↔ Bevy Entity 同步
// - network_bridge.rs - 网络线程 ↔ ECS 通信
// - packet_types.rs - 统一的包类型定义 (复用 SharedRust)

pub mod object_sync;
pub mod network_bridge;
pub mod packet_types;

pub use object_sync::{MapObjectRef, sync_objects_to_entities, sync_entities_to_objects};
pub use network_bridge::{
    NetworkBridge, 
    network_to_bevy_system, 
    bevy_to_network_system
};
pub use packet_types::{
    ServerPacket,
    ClientPacket,
    ServerPacketEvent, 
    ClientPacketEvent,
};
