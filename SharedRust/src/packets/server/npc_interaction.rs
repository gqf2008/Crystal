// NPC 交互相关的服务器数据包

use super::super::base::Packet;
use crate::binary::{read_dotnet_string, write_dotnet_string};
use crate::data::item::UserItem;
use crate::data::stats::SharedResult;
use crate::enums::{PanelType, ServerPacketIds};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

/// NPC 响应（对话页面）
#[derive(Debug, Clone)]
pub struct NPCResponse {
    pub page: Vec<String>, // NPC 对话页面内容列表
}

impl Packet for NPCResponse {
    const OPCODE: i16 = ServerPacketIds::NPCResponse as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()? as usize;
        let mut page = Vec::with_capacity(count);
        for _ in 0..count {
            page.push(read_dotnet_string(reader)?);
        }
        Ok(Self { page })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.page.len() as i32)?;
        for line in &self.page {
            write_dotnet_string(writer, line)?;
        }
        Ok(())
    }
}

/// NPC 商品列表
#[derive(Debug, Clone)]
pub struct NPCGoods {
    pub list: Vec<UserItem>,    // 商品列表
    pub rate: f32,              // 价格倍率
    pub panel_type: PanelType,  // 面板类型（购买、出售、修理等）
    pub hide_added_stats: bool, // 是否隐藏附加属性
}

impl Packet for NPCGoods {
    const OPCODE: i16 = ServerPacketIds::NPCGoods as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()? as usize;
        let mut list = Vec::with_capacity(count);
        for _ in 0..count {
            list.push(UserItem::read_from_with_info(reader)?);
        }

        let rate = reader.read_f32::<LittleEndian>()?;
        let panel_type = PanelType::try_from(reader.read_u8()?)?;
        let hide_added_stats = reader.read_u8()? != 0;

        Ok(Self {
            list,
            rate,
            panel_type,
            hide_added_stats,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.list.len() as i32)?;
        for item in &self.list {
            item.write_to_with_info(writer)?;
        }

        writer.write_f32::<LittleEndian>(self.rate)?;
        writer.write_u8(self.panel_type as u8)?;
        writer.write_u8(if self.hide_added_stats { 1 } else { 0 })?;

        Ok(())
    }

    fn is_compressed() -> bool {
        true // NPCGoods 使用压缩
    }
}

/// NPC 更新
#[derive(Debug, Clone)]
pub struct NPCUpdate {
    pub npc_id: u32, // NPC ID
}

impl Packet for NPCUpdate {
    const OPCODE: i16 = ServerPacketIds::NPCUpdate as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let npc_id = reader.read_u32::<LittleEndian>()?;
        Ok(Self { npc_id })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.npc_id)?;
        Ok(())
    }
}

/// NPC 图像更新（C# ServerPackets.cs：ObjectID + Image + Colour）
#[derive(Debug, Clone)]
pub struct NPCImageUpdate {
    pub npc_id: u32, // NPC ID
    pub image: u16,  // 新图像
    pub colour: i32, // 新染色（C# Color.ToArgb；征服旗子 FlagColour）
}

impl Packet for NPCImageUpdate {
    const OPCODE: i16 = ServerPacketIds::NPCImageUpdate as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let npc_id = reader.read_u32::<LittleEndian>()?;
        let image = reader.read_u16::<LittleEndian>()?;
        let colour = reader.read_i32::<LittleEndian>()?;
        Ok(Self {
            npc_id,
            image,
            colour,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.npc_id)?;
        writer.write_u16::<LittleEndian>(self.image)?;
        writer.write_i32::<LittleEndian>(self.colour)?;
        Ok(())
    }
}

/// 默认 NPC 触发
#[derive(Debug, Clone)]
pub struct DefaultNPC {
    pub object_id: u32,    // 对象 ID
    pub page: Vec<String>, // NPC 页面内容
}

impl Packet for DefaultNPC {
    const OPCODE: i16 = ServerPacketIds::DefaultNPC as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let count = reader.read_i32::<LittleEndian>()? as usize;
        let mut page = Vec::with_capacity(count);
        for _ in 0..count {
            page.push(read_dotnet_string(reader)?);
        }
        Ok(Self { object_id, page })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_i32::<LittleEndian>(self.page.len() as i32)?;
        for line in &self.page {
            write_dotnet_string(writer, line)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn npc_image_update_roundtrip_with_colour() {
        // C# ServerPackets.cs：ObjectID + Image + Colour（Color.ToArgb）
        let pkt = NPCImageUpdate {
            npc_id: 42,
            image: 1000,
            colour: 0x00FF0000,
        };
        let mut buf = Vec::new();
        pkt.write_body(&mut buf).unwrap();
        // u32 + u16 + i32 = 10 bytes
        assert_eq!(buf.len(), 10);
        let mut cursor = Cursor::new(&buf);
        let parsed = NPCImageUpdate::read_body(&mut cursor).unwrap();
        assert_eq!(parsed.npc_id, 42);
        assert_eq!(parsed.image, 1000);
        assert_eq!(parsed.colour, 0x00FF0000);
    }
}
