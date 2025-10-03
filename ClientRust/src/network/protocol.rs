// Protocol module - Packet handling and dispatching
//
// 这个模块提供了基于 SharedRust 的数据包处理架构:
// - 直接使用 SharedRust 的 273 个服务器数据包和 146 个客户端数据包
// - 基于 opcode 的数据包分发系统
// - 类型安全的数据包处理接口
//
// 设计原则:
// 1. 不创建中间抽象层 (如 ServerMessage 枚举)
// 2. 直接使用 SharedRust 的 Packet trait
// 3. 通过 handler trait 实现多态处理

use anyhow::{anyhow, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Cursor, Write};

// ============================================================================
// 重新导出 SharedRust 的核心类型
// ============================================================================

// 重新导出 Packet trait
pub use mir2_shared::packets::base::Packet;

// 重新导出常用枚举
pub use mir2_shared::enums::{ClientPacketIds, ServerPacketIds};

// 重新导出常用数据类型
pub use mir2_shared::data::UserItem;

// 为了兼容性，重新导出所有 SharedRust 模块
pub use mir2_shared::{packets, data, enums};

// ============================================================================
// 类型别名 - 为了兼容现有代码
// ============================================================================

// 这些类型别名映射到 SharedRust 的实际类型
pub type PlayerObject = mir2_shared::packets::ObjectPlayer;
pub type HeroObject = mir2_shared::packets::ObjectHero;
pub type ObjectNpc = mir2_shared::packets::ObjectNpc;
pub type ObjectMonster = mir2_shared::packets::ObjectMonster;
pub type ObjectItem = mir2_shared::packets::ObjectItem;
pub type UserInformation = mir2_shared::packets::UserInformation;
pub type CharacterSummary = mir2_shared::packets::CharacterSummary;

// 占位类型 - 需要进一步确定映射
pub type HeroInformation = ();
pub type ServerMessage = ();

// ============================================================================
// 客户端数据包序列化
// ============================================================================

/// 序列化客户端数据包为字节流
///
/// 数据包格式:
/// - 2字节: 总长度 (包括这2字节)
/// - 2字节: Opcode (小端序)
/// - N字节: 数据包体
///
/// # 示例
/// ```ignore
/// use mir2_shared::packets::client::ClientVersion;
/// 
/// let packet = ClientVersion { version_hash: vec![1, 2, 3, 4] };
/// let bytes = serialize_client_packet(&packet)?;
/// // bytes 现在可以发送到服务器
/// ```
pub fn serialize_client_packet<P: Packet>(packet: &P) -> Result<Vec<u8>> {
    let mut buffer = Vec::new();
    
    // 1. 写入长度占位符 (2字节)
    buffer.write_u16::<LittleEndian>(0)?;
    
    // 2. 写入opcode (2字节)
    buffer.write_i16::<LittleEndian>(P::OPCODE)?;
    
    // 3. 写入数据包体
    packet.write_body(&mut buffer)?;
    
    // 4. 回填实际长度
    let length = buffer.len() as u16;
    LittleEndian::write_u16(&mut buffer[0..2], length);
    
    Ok(buffer)
}

// ============================================================================
// 服务器数据包解析辅助函数
// ============================================================================

/// 数据包头部信息
#[derive(Debug, Clone, Copy)]
pub struct PacketHeader {
    pub length: u16,
    pub opcode: i16,
}

/// 从字节流中解析数据包头部
///
/// # 参数
/// - `data`: 至少包含4字节的数据 (2字节长度 + 2字节opcode)
///
/// # 返回
/// - `Ok(PacketHeader)`: 成功解析的头部
/// - `Err`: 数据不足或解析失败
pub fn parse_packet_header(data: &[u8]) -> Result<PacketHeader> {
    if data.len() < 4 {
        return Err(anyhow!("数据不足: 需要至少4字节，实际{}字节", data.len()));
    }
    
    let mut cursor = Cursor::new(data);
    let length = cursor.read_u16::<LittleEndian>()?;
    let opcode = cursor.read_i16::<LittleEndian>()?;
    
    Ok(PacketHeader { length, opcode })
}

/// 获取数据包体的切片 (跳过4字节的头部)
///
/// # 参数
/// - `data`: 完整的数据包数据 (包括头部)
///
/// # 返回
/// - `Ok(&[u8])`: 数据包体的切片
/// - `Err`: 数据不足
pub fn get_packet_body(data: &[u8]) -> Result<&[u8]> {
    if data.len() < 4 {
        return Err(anyhow!("数据不足: 需要至少4字节"));
    }
    Ok(&data[4..])
}

// ============================================================================
// 数据包处理器 Trait
// ============================================================================

/// 服务器数据包处理器接口
///
/// 实现这个 trait 来处理来自服务器的数据包。
/// 每个方法对应一个服务器数据包类型。
///
/// # 设计说明
/// 
/// 我们不使用一个大的 ServerMessage 枚举，而是通过这个 trait 来处理不同的数据包。
/// 好处:
/// 1. 类型安全 - 每个处理函数接收特定的数据包类型
/// 2. 可扩展 - 只需实现需要的方法
/// 3. 清晰 - 避免大量的 match 分支
///
/// # 示例
/// ```ignore
/// struct MyHandler;
/// 
/// impl PacketHandler for MyHandler {
///     fn on_connected(&mut self, packet: packets::Connected) {
///         println!("已连接到服务器");
///     }
///     
///     fn on_user_location(&mut self, packet: packets::UserLocation) {
///         println!("玩家位置: {:?}", packet.location);
///     }
/// }
/// ```
pub trait PacketHandler {
    // 连接相关
    fn on_connected(&mut self, _packet: packets::Connected) {}
    fn on_disconnect(&mut self, _packet: packets::Disconnect) {}
    
    // 用户信息
    fn on_user_information(&mut self, _packet: packets::UserInformation) {}
    fn on_user_location(&mut self, _packet: packets::UserLocation) {}
    
    // 地图相关
    fn on_map_information(&mut self, _packet: packets::MapInformation) {}
    fn on_new_map_info(&mut self, _packet: packets::NewMapInfo) {}
    
    // 对象相关
    fn on_object_player(&mut self, _packet: packets::ObjectPlayer) {}
    fn on_object_hero(&mut self, _packet: packets::ObjectHero) {}
    fn on_object_monster(&mut self, _packet: packets::ObjectMonster) {}
    fn on_object_npc(&mut self, _packet: packets::ObjectNpc) {}
    fn on_object_item(&mut self, _packet: packets::ObjectItem) {}
    
    // 默认处理 - 当收到未知或未实现的数据包时调用
    fn on_unknown_packet(&mut self, opcode: i16, _data: &[u8]) {
        tracing::warn!("收到未处理的数据包: opcode={}", opcode);
    }
}

// ============================================================================
// 数据包分发器
// ============================================================================

/// 数据包分发器 - 将原始字节流分发到对应的处理函数
///
/// 这个函数根据 opcode 解析数据包并调用 handler 的相应方法。
///
/// # 参数
/// - `header`: 数据包头部信息
/// - `data`: 完整的数据包数据 (包括头部)
/// - `handler`: 实现了 PacketHandler trait 的处理器
///
/// # 返回
/// - `Ok(())`: 成功处理
/// - `Err`: 解析或处理失败
///
/// # 示例
/// ```ignore
/// let header = parse_packet_header(&data)?;
/// dispatch_packet(header, &data, &mut my_handler)?;
/// ```
pub fn dispatch_packet<H: PacketHandler>(
    header: PacketHeader,
    data: &[u8],
    handler: &mut H,
) -> Result<()> {
    // 获取数据包体
    let body = get_packet_body(data)?;
    let mut cursor = Cursor::new(body);
    
    // 根据 opcode 分发到对应的处理函数
    match header.opcode as u16 {
        // 连接相关
        x if x == ServerPacketIds::Connected as u16 => {
            let packet = packets::Connected::read_body(&mut cursor)?;
            handler.on_connected(packet);
        }
        x if x == ServerPacketIds::Disconnect as u16 => {
            let packet = packets::Disconnect::read_body(&mut cursor)?;
            handler.on_disconnect(packet);
        }
        
        // 用户信息
        x if x == ServerPacketIds::UserInformation as u16 => {
            let packet = packets::UserInformation::read_body(&mut cursor)?;
            handler.on_user_information(packet);
        }
        x if x == ServerPacketIds::UserLocation as u16 => {
            let packet = packets::UserLocation::read_body(&mut cursor)?;
            handler.on_user_location(packet);
        }
        
        // 地图相关
        x if x == ServerPacketIds::MapInformation as u16 => {
            let packet = packets::MapInformation::read_body(&mut cursor)?;
            handler.on_map_information(packet);
        }
        x if x == ServerPacketIds::NewMapInfo as u16 => {
            let packet = packets::NewMapInfo::read_body(&mut cursor)?;
            handler.on_new_map_info(packet);
        }
        
        // 对象相关
        x if x == ServerPacketIds::ObjectPlayer as u16 => {
            let packet = packets::ObjectPlayer::read_body(&mut cursor)?;
            handler.on_object_player(packet);
        }
        x if x == ServerPacketIds::ObjectHero as u16 => {
            let packet = packets::ObjectHero::read_body(&mut cursor)?;
            handler.on_object_hero(packet);
        }
        x if x == ServerPacketIds::ObjectMonster as u16 => {
            let packet = packets::ObjectMonster::read_body(&mut cursor)?;
            handler.on_object_monster(packet);
        }
        x if x == ServerPacketIds::ObjectNpc as u16 => {
            let packet = packets::ObjectNpc::read_body(&mut cursor)?;
            handler.on_object_npc(packet);
        }
        x if x == ServerPacketIds::ObjectItem as u16 => {
            let packet = packets::ObjectItem::read_body(&mut cursor)?;
            handler.on_object_item(packet);
        }
        
        // 未知数据包
        _ => {
            handler.on_unknown_packet(header.opcode, body);
        }
    }
    
    Ok(())
}

// ============================================================================
// 测试和示例
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    struct TestHandler {
        connected_called: bool,
    }
    
    impl PacketHandler for TestHandler {
        fn on_connected(&mut self, _packet: packets::Connected) {
            self.connected_called = true;
        }
    }
    
    #[test]
    fn test_packet_header_parsing() {
        let data = vec![0x10, 0x00, 0x01, 0x00]; // length=16, opcode=1
        let header = parse_packet_header(&data).unwrap();
        assert_eq!(header.length, 16);
        assert_eq!(header.opcode, 1);
    }
    
    #[test]
    fn test_serialize_client_packet() {
        use mir2_shared::packets::client::ClientVersion;
        
        let packet = ClientVersion {
            version_hash: vec![1, 2, 3, 4],
        };
        
        let bytes = serialize_client_packet(&packet).unwrap();
        
        // 检查长度和opcode
        assert!(bytes.len() >= 4);
        let header = parse_packet_header(&bytes).unwrap();
        assert_eq!(header.length, bytes.len() as u16);
        assert_eq!(header.opcode, ClientVersion::OPCODE);
    }
}
