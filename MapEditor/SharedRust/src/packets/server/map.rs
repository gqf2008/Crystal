//! Map System Packets
//!
//! This module contains all map-related packet definitions and parsers.

use std::io::{Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::{
    enums::ServerPacketIds,
    binary::{read_dotnet_string, write_dotnet_string},
};
use super::super::base::Packet;
use crate::data::stats::SharedResult;

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
    pub file_name: String,
    pub title: String,
    pub minimap: u16,
    pub big_map: u16,
    pub music: u16,
    pub lights: u8,
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
    pub map_dark_light: u8,
}

#[derive(Debug, Clone)]
pub struct MapChanged {
    pub file_name: String,
    pub title: String,
    pub minimap: u16,
    pub big_map: u16,
    pub music: u16,
    pub lights: u8,
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
    pub map_dark_light: u8,
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
    /// Rust 扩展：旧位置
    pub location_x: u32,
    pub location_y: u32,
}

#[derive(Debug, Clone)]
pub struct ObjectTeleportIn {
    pub object_id: u32,
    pub teleport_type: u8,
    /// Rust 扩展：新位置
    pub location_x: u32,
    pub location_y: u32,
}

#[derive(Debug, Clone)]
pub struct TeleportIn;

#[derive(Debug, Clone)]
pub struct WorldMapSetupInfo {
    pub world_maps: Vec<WorldMapIcon>,
}

#[derive(Debug, Clone)]
pub struct WorldMapIcon {
    pub icon: u16,
    pub title: String,
    pub map_index: i32,
    pub location_x: u32,
    pub location_y: u32,
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
        if self.lightning { bools |= 0x01; }
        if self.fire { bools |= 0x02; }
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
        Ok(NewMapInfo {
            map_index: reader.read_i32::<LittleEndian>()?,
            file_name: read_dotnet_string(reader)?,
            title: read_dotnet_string(reader)?,
            minimap: reader.read_u16::<LittleEndian>()?,
            big_map: reader.read_u16::<LittleEndian>()?,
            music: reader.read_u16::<LittleEndian>()?,
            lights: reader.read_u8()?,
            location_x: reader.read_u32::<LittleEndian>()?,
            location_y: reader.read_u32::<LittleEndian>()?,
            direction: reader.read_u8()?,
            map_dark_light: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.map_index)?;
        write_dotnet_string(writer, &self.file_name)?;
        write_dotnet_string(writer, &self.title)?;
        writer.write_u16::<LittleEndian>(self.minimap)?;
        writer.write_u16::<LittleEndian>(self.big_map)?;
        writer.write_u16::<LittleEndian>(self.music)?;
        writer.write_u8(self.lights)?;
        writer.write_u32::<LittleEndian>(self.location_x)?;
        writer.write_u32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction)?;
        writer.write_u8(self.map_dark_light)?;
        Ok(())
    }
}

impl Packet for MapChanged {
    const OPCODE: i16 = ServerPacketIds::MapChanged as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(MapChanged {
            file_name: read_dotnet_string(reader)?,
            title: read_dotnet_string(reader)?,
            minimap: reader.read_u16::<LittleEndian>()?,
            big_map: reader.read_u16::<LittleEndian>()?,
            music: reader.read_u16::<LittleEndian>()?,
            lights: reader.read_u8()?,
            location_x: reader.read_u32::<LittleEndian>()?,
            location_y: reader.read_u32::<LittleEndian>()?,
            direction: reader.read_u8()?,
            map_dark_light: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.file_name)?;
        write_dotnet_string(writer, &self.title)?;
        writer.write_u16::<LittleEndian>(self.minimap)?;
        writer.write_u16::<LittleEndian>(self.big_map)?;
        writer.write_u16::<LittleEndian>(self.music)?;
        writer.write_u8(self.lights)?;
        writer.write_u32::<LittleEndian>(self.location_x)?;
        writer.write_u32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction)?;
        writer.write_u8(self.map_dark_light)?;
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
        let count = reader.read_i32::<LittleEndian>()? as usize;
        let mut world_maps = Vec::with_capacity(count);
        
        for _ in 0..count {
            world_maps.push(WorldMapIcon {
                icon: reader.read_u16::<LittleEndian>()?,
                title: read_dotnet_string(reader)?,
                map_index: reader.read_i32::<LittleEndian>()?,
                location_x: reader.read_u32::<LittleEndian>()?,
                location_y: reader.read_u32::<LittleEndian>()?,
            });
        }

        Ok(WorldMapSetupInfo { world_maps })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.world_maps.len() as i32)?;
        
        for icon in &self.world_maps {
            writer.write_u16::<LittleEndian>(icon.icon)?;
            write_dotnet_string(writer, &icon.title)?;
            writer.write_i32::<LittleEndian>(icon.map_index)?;
            writer.write_u32::<LittleEndian>(icon.location_x)?;
            writer.write_u32::<LittleEndian>(icon.location_y)?;
        }
        
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
