//! NPC Interaction Packets (Client → Server)

use std::io::{Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::binary::{read_dotnet_string, write_dotnet_string};
use crate::enums::{ClientPacketIds, PanelType};
use super::super::base::Packet;
use crate::data::stats::SharedResult;

/// Call NPC to open dialog
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallNPC {
    pub object_id: u32,
    pub key: String,
}

impl Packet for CallNPC {
    const OPCODE: i16 = ClientPacketIds::CallNPC as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let key = read_dotnet_string(reader)?;
        Ok(Self { object_id, key })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        write_dotnet_string(writer, &self.key)?;
        Ok(())
    }
}

/// Buy item from NPC
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuyItem {
    pub item_index: u64,
    pub count: u16,
    pub panel_type: PanelType,
}

impl Packet for BuyItem {
    const OPCODE: i16 = ClientPacketIds::BuyItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let item_index = reader.read_u64::<LittleEndian>()?;
        let count = reader.read_u16::<LittleEndian>()?;
        let panel_type = PanelType::try_from(reader.read_u8()?).unwrap_or(PanelType::Buy);
        Ok(Self {
            item_index,
            count,
            panel_type,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.item_index)?;
        writer.write_u16::<LittleEndian>(self.count)?;
        writer.write_u8(self.panel_type as u8)?;
        Ok(())
    }
}

/// Sell item to NPC
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SellItem {
    pub unique_id: u64,
    pub count: u16,
}

impl Packet for SellItem {
    const OPCODE: i16 = ClientPacketIds::SellItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let count = reader.read_u16::<LittleEndian>()?;
        Ok(Self { unique_id, count })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u16::<LittleEndian>(self.count)?;
        Ok(())
    }
}

/// Craft item at NPC
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CraftItem {
    pub unique_id: u64,
    pub count: u16,
    pub slots: Vec<i32>,
}

impl Packet for CraftItem {
    const OPCODE: i16 = ClientPacketIds::CraftItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let count = reader.read_u16::<LittleEndian>()?;
        let slot_count = reader.read_i32::<LittleEndian>()?;
        let mut slots = Vec::with_capacity(slot_count as usize);
        for _ in 0..slot_count {
            slots.push(reader.read_i32::<LittleEndian>()?);
        }
        Ok(Self {
            unique_id,
            count,
            slots,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u16::<LittleEndian>(self.count)?;
        writer.write_i32::<LittleEndian>(self.slots.len() as i32)?;
        for slot in &self.slots {
            writer.write_i32::<LittleEndian>(*slot)?;
        }
        Ok(())
    }
}

/// Repair item at NPC
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairItem {
    pub unique_id: u64,
}

impl Packet for RepairItem {
    const OPCODE: i16 = ClientPacketIds::RepairItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        Ok(Self { unique_id })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        Ok(())
    }
}

/// Buy back previously sold item
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuyItemBack {
    pub unique_id: u64,
    pub count: u16,
}

impl Packet for BuyItemBack {
    const OPCODE: i16 = ClientPacketIds::BuyItemBack as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let count = reader.read_u16::<LittleEndian>()?;
        Ok(Self { unique_id, count })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u16::<LittleEndian>(self.count)?;
        Ok(())
    }
}

/// Special repair item
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SRepairItem {
    pub unique_id: u64,
}

impl Packet for SRepairItem {
    const OPCODE: i16 = ClientPacketIds::SRepairItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        Ok(Self { unique_id })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        Ok(())
    }
}

/// Request map info from NPC
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestMapInfo {
    pub map_index: i32,
}

impl Packet for RequestMapInfo {
    const OPCODE: i16 = ClientPacketIds::RequestMapInfo as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let map_index = reader.read_i32::<LittleEndian>()?;
        Ok(Self { map_index })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.map_index)?;
        Ok(())
    }
}

/// Teleport to NPC
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeleportToNPC {
    pub object_id: u32,
}

impl Packet for TeleportToNPC {
    const OPCODE: i16 = ClientPacketIds::TeleportToNPC as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        Ok(Self { object_id })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        Ok(())
    }
}

/// Search map
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMap {
    pub text: String,
}

impl Packet for SearchMap {
    const OPCODE: i16 = ClientPacketIds::SearchMap as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let text = read_dotnet_string(reader)?;
        Ok(Self { text })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.text)?;
        Ok(())
    }
}

/// NPC confirm input
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NPCConfirmInput {
    pub npc_id: u32,
    pub page_name: String,
    pub value: String,
}

impl Packet for NPCConfirmInput {
    const OPCODE: i16 = ClientPacketIds::NPCConfirmInput as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let npc_id = reader.read_u32::<LittleEndian>()?;
        let page_name = read_dotnet_string(reader)?;
        let value = read_dotnet_string(reader)?;
        Ok(Self {
            npc_id,
            page_name,
            value,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.npc_id)?;
        write_dotnet_string(writer, &self.page_name)?;
        write_dotnet_string(writer, &self.value)?;
        Ok(())
    }
}
