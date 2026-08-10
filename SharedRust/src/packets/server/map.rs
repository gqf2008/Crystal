//! Map System Packets
//!
//! This module contains all map-related packet definitions and parsers.

use super::super::base::Packet;
use crate::data::stats::SharedResult;
use crate::{
    binary::{read_dotnet_string, write_dotnet_string},
    enums::ServerPacketIds,
};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

#[derive(Debug, Clone)]
pub struct MapInformation {
    pub map_index: i32,
    pub file_name: String,
    pub title: String,
    pub minimap: u16,
    pub big_map: u16,
    pub lights: u8,
    pub lightning: bool,
    pub fire: bool,
    pub map_dark_light: u8,
    pub music: u16,
    pub weather_particles: u16,
}

#[derive(Debug, Clone)]
pub struct NewMapInfo {
    pub map_index: i32,
    pub title: String,
    pub width: i32,
    pub height: i32,
    pub big_map: i32,
    pub movements: Vec<MovementInfo>,
    pub npcs: Vec<NpcMapInfo>,
}

#[derive(Debug, Clone)]
pub struct MovementInfo {
    pub destination: i32,
    pub title: String,
    pub location_x: i32,
    pub location_y: i32,
    pub icon: i32,
}

#[derive(Debug, Clone)]
pub struct NpcMapInfo {
    pub object_id: u32,
    pub name: String,
    pub location_x: i32,
    pub location_y: i32,
    pub icon: i32,
    pub can_teleport_to: bool,
}

#[derive(Debug, Clone)]
pub struct MapChanged {
    pub map_index: i32,
    pub file_name: String,
    pub title: String,
    pub minimap: u16,
    pub big_map: u16,
    pub lights: u8,
    pub location_x: i32,
    pub location_y: i32,
    pub direction: u8,
    pub map_dark_light: u8,
    pub music: u16,
    pub weather: u16,
}

#[derive(Debug, Clone)]
pub struct ObjectHide {
    pub object_id: u32,
}

#[derive(Debug, Clone)]
pub struct ObjectShow {
    pub object_id: u32,
}

#[derive(Debug, Clone)]
pub struct ObjectTeleportOut {
    pub object_id: u32,
    pub teleport_type: u8,
    /// Rust 扩展：旧位置（客户端瞬移用；C# 原包无位置，靠重生广播）
    pub location_x: u32,
    pub location_y: u32,
}

#[derive(Debug, Clone)]
pub struct ObjectTeleportIn {
    pub object_id: u32,
    pub teleport_type: u8,
    /// Rust 扩展：新位置（客户端瞬移用）
    pub location_x: u32,
    pub location_y: u32,
}

#[derive(Debug, Clone)]
pub struct TeleportIn;

#[derive(Debug, Clone)]
pub struct WorldMapSetupInfo {
    /// C# 线格式：[enabled u8][count i32][(image_index i32)(title dotnet)(map_index i32)]... [teleport_cost i32]
    pub enabled: bool,
    pub world_maps: Vec<WorldMapIcon>,
    pub teleport_cost: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 世界地图图标（C# SharedData.WorldMapIcon：ImageIndex/Title/MapIndex）
pub struct WorldMapIcon {
    pub image_index: i32,
    pub title: String,
    pub map_index: i32,
}

#[derive(Debug, Clone)]
pub struct SearchMapResult {
    pub map_index: i32,
    pub location_x: u32,
    pub location_y: u32,
}

// ==================== 解析函数 ====================

impl Packet for MapInformation {
    const OPCODE: i16 = ServerPacketIds::MapInformation as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let map_index = reader.read_i32::<LittleEndian>()?;
        let file_name = read_dotnet_string(reader)?;
        let title = read_dotnet_string(reader)?;
        let minimap = reader.read_u16::<LittleEndian>()?;
        let big_map = reader.read_u16::<LittleEndian>()?;
        let lights = reader.read_u8()?;

        // 读取 Lightning 和 Fire 布尔标志位 (打包在一个字节中)
        let bools = reader.read_u8()?;
        let lightning = (bools & 0x01) == 0x01;
        let fire = (bools & 0x02) == 0x02;

        let map_dark_light = reader.read_u8()?;
        let music = reader.read_u16::<LittleEndian>()?;
        let weather_particles = reader.read_u16::<LittleEndian>()?;

        Ok(MapInformation {
            map_index,
            file_name,
            title,
            minimap,
            big_map,
            lights,
            lightning,
            fire,
            map_dark_light,
            music,
            weather_particles,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.map_index)?;
        write_dotnet_string(writer, &self.file_name)?;
        write_dotnet_string(writer, &self.title)?;
        writer.write_u16::<LittleEndian>(self.minimap)?;
        writer.write_u16::<LittleEndian>(self.big_map)?;
        writer.write_u8(self.lights)?;

        // 写入 Lightning 和 Fire 布尔标志位 (打包在一个字节中)
        let mut bools: u8 = 0;
        if self.lightning {
            bools |= 0x01;
        }
        if self.fire {
            bools |= 0x02;
        }
        writer.write_u8(bools)?;

        writer.write_u8(self.map_dark_light)?;
        writer.write_u16::<LittleEndian>(self.music)?;
        writer.write_u16::<LittleEndian>(self.weather_particles)?;
        Ok(())
    }
}

impl Packet for NewMapInfo {
    const OPCODE: i16 = ServerPacketIds::NewMapInfo as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let map_index = reader.read_i32::<LittleEndian>()?;
        let title = read_dotnet_string(reader)?;
        let width = reader.read_i32::<LittleEndian>()?;
        let height = reader.read_i32::<LittleEndian>()?;
        let big_map = reader.read_i32::<LittleEndian>()?;

        let mov_count = reader.read_i32::<LittleEndian>()?;
        let mut movements = Vec::with_capacity(mov_count as usize);
        for _ in 0..mov_count {
            movements.push(MovementInfo {
                destination: reader.read_i32::<LittleEndian>()?,
                title: read_dotnet_string(reader)?,
                location_x: reader.read_i32::<LittleEndian>()?,
                location_y: reader.read_i32::<LittleEndian>()?,
                icon: reader.read_i32::<LittleEndian>()?,
            });
        }

        let npc_count = reader.read_i32::<LittleEndian>()?;
        let mut npcs = Vec::with_capacity(npc_count as usize);
        for _ in 0..npc_count {
            npcs.push(NpcMapInfo {
                object_id: reader.read_u32::<LittleEndian>()?,
                name: read_dotnet_string(reader)?,
                location_x: reader.read_i32::<LittleEndian>()?,
                location_y: reader.read_i32::<LittleEndian>()?,
                icon: reader.read_i32::<LittleEndian>()?,
                can_teleport_to: reader.read_u8()? != 0,
            });
        }

        Ok(Self {
            map_index,
            title,
            width,
            height,
            big_map,
            movements,
            npcs,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.map_index)?;
        write_dotnet_string(writer, &self.title)?;
        writer.write_i32::<LittleEndian>(self.width)?;
        writer.write_i32::<LittleEndian>(self.height)?;
        writer.write_i32::<LittleEndian>(self.big_map)?;

        writer.write_i32::<LittleEndian>(self.movements.len() as i32)?;
        for m in &self.movements {
            writer.write_i32::<LittleEndian>(m.destination)?;
            write_dotnet_string(writer, &m.title)?;
            writer.write_i32::<LittleEndian>(m.location_x)?;
            writer.write_i32::<LittleEndian>(m.location_y)?;
            writer.write_i32::<LittleEndian>(m.icon)?;
        }

        writer.write_i32::<LittleEndian>(self.npcs.len() as i32)?;
        for n in &self.npcs {
            writer.write_u32::<LittleEndian>(n.object_id)?;
            write_dotnet_string(writer, &n.name)?;
            writer.write_i32::<LittleEndian>(n.location_x)?;
            writer.write_i32::<LittleEndian>(n.location_y)?;
            writer.write_i32::<LittleEndian>(n.icon)?;
            writer.write_u8(n.can_teleport_to as u8)?;
        }

        Ok(())
    }
}

impl Packet for MapChanged {
    const OPCODE: i16 = ServerPacketIds::MapChanged as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(MapChanged {
            map_index: reader.read_i32::<LittleEndian>()?,
            file_name: read_dotnet_string(reader)?,
            title: read_dotnet_string(reader)?,
            minimap: reader.read_u16::<LittleEndian>()?,
            big_map: reader.read_u16::<LittleEndian>()?,
            lights: reader.read_u8()?,
            location_x: reader.read_i32::<LittleEndian>()?,
            location_y: reader.read_i32::<LittleEndian>()?,
            direction: reader.read_u8()?,
            map_dark_light: reader.read_u8()?,
            music: reader.read_u16::<LittleEndian>()?,
            weather: reader.read_u16::<LittleEndian>()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.map_index)?;
        write_dotnet_string(writer, &self.file_name)?;
        write_dotnet_string(writer, &self.title)?;
        writer.write_u16::<LittleEndian>(self.minimap)?;
        writer.write_u16::<LittleEndian>(self.big_map)?;
        writer.write_u8(self.lights)?;
        writer.write_i32::<LittleEndian>(self.location_x)?;
        writer.write_i32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction)?;
        writer.write_u8(self.map_dark_light)?;
        writer.write_u16::<LittleEndian>(self.music)?;
        writer.write_u16::<LittleEndian>(self.weather)?;
        Ok(())
    }
}

impl Packet for ObjectHide {
    const OPCODE: i16 = ServerPacketIds::ObjectHide as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(ObjectHide {
            object_id: reader.read_u32::<LittleEndian>()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        Ok(())
    }
}

impl Packet for ObjectShow {
    const OPCODE: i16 = ServerPacketIds::ObjectShow as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(ObjectShow {
            object_id: reader.read_u32::<LittleEndian>()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        Ok(())
    }
}

impl Packet for ObjectTeleportOut {
    const OPCODE: i16 = ServerPacketIds::ObjectTeleportOut as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(ObjectTeleportOut {
            object_id: reader.read_u32::<LittleEndian>()?,
            teleport_type: reader.read_u8()?,
            location_x: reader.read_u32::<LittleEndian>()?,
            location_y: reader.read_u32::<LittleEndian>()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u8(self.teleport_type)?;
        writer.write_u32::<LittleEndian>(self.location_x)?;
        writer.write_u32::<LittleEndian>(self.location_y)?;
        Ok(())
    }
}

impl Packet for ObjectTeleportIn {
    const OPCODE: i16 = ServerPacketIds::ObjectTeleportIn as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(ObjectTeleportIn {
            object_id: reader.read_u32::<LittleEndian>()?,
            teleport_type: reader.read_u8()?,
            location_x: reader.read_u32::<LittleEndian>()?,
            location_y: reader.read_u32::<LittleEndian>()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u8(self.teleport_type)?;
        writer.write_u32::<LittleEndian>(self.location_x)?;
        writer.write_u32::<LittleEndian>(self.location_y)?;
        Ok(())
    }
}

impl Packet for TeleportIn {
    const OPCODE: i16 = ServerPacketIds::TeleportIn as i16;

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(TeleportIn)
    }

    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

impl Packet for WorldMapSetupInfo {
    const OPCODE: i16 = ServerPacketIds::WorldMapSetup as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let enabled = reader.read_u8()? != 0;
        let count = reader.read_i32::<LittleEndian>()? as usize;
        let mut world_maps = Vec::with_capacity(count);
        for _ in 0..count {
            world_maps.push(WorldMapIcon {
                image_index: reader.read_i32::<LittleEndian>()?,
                title: read_dotnet_string(reader)?,
                map_index: reader.read_i32::<LittleEndian>()?,
            });
        }
        let teleport_cost = reader.read_i32::<LittleEndian>()?;
        Ok(WorldMapSetupInfo {
            enabled,
            world_maps,
            teleport_cost,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_all(&[if self.enabled { 1 } else { 0 }])?;
        writer.write_i32::<LittleEndian>(self.world_maps.len() as i32)?;
        for icon in &self.world_maps {
            writer.write_i32::<LittleEndian>(icon.image_index)?;
            write_dotnet_string(writer, &icon.title)?;
            writer.write_i32::<LittleEndian>(icon.map_index)?;
        }
        writer.write_i32::<LittleEndian>(self.teleport_cost)?;
        Ok(())
    }
}

impl Packet for SearchMapResult {
    const OPCODE: i16 = ServerPacketIds::SearchMapResult as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(SearchMapResult {
            map_index: reader.read_i32::<LittleEndian>()?,
            location_x: reader.read_u32::<LittleEndian>()?,
            location_y: reader.read_u32::<LittleEndian>()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.map_index)?;
        writer.write_u32::<LittleEndian>(self.location_x)?;
        writer.write_u32::<LittleEndian>(self.location_y)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn world_map_setup_info_roundtrip() {
        let pkt = WorldMapSetupInfo {
            enabled: true,
            world_maps: vec![
                WorldMapIcon {
                    image_index: 1,
                    title: "比奇省".to_string(),
                    map_index: 0,
                },
                WorldMapIcon {
                    image_index: 2,
                    title: "盟重省".to_string(),
                    map_index: 1,
                },
            ],
            teleport_cost: 1000,
        };
        let mut buf = Vec::new();
        pkt.write_body(&mut buf).unwrap();
        let mut cur = Cursor::new(&buf);
        let read = WorldMapSetupInfo::read_body(&mut cur).unwrap();
        assert_eq!(read.enabled, pkt.enabled);
        assert_eq!(read.world_maps.len(), 2);
        assert_eq!(read.world_maps[0].image_index, 1);
        assert_eq!(read.world_maps[0].title, "比奇省");
        assert_eq!(read.world_maps[0].map_index, 0);
        assert_eq!(read.world_maps[1].image_index, 2);
        assert_eq!(read.world_maps[1].map_index, 1);
        assert_eq!(read.teleport_cost, 1000);
    }
}
