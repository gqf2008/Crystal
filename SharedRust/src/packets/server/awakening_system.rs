// 觉醒系统相关数据包
use super::super::base::Packet;
use crate::data::stats::SharedResult;
use crate::enums::ServerPacketIds;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Read;

/// NPCAwakening - NPC觉醒 (222)
#[derive(Debug, Clone)]
pub struct NPCAwakening {
    // C# 原实现为空
}

impl Packet for NPCAwakening {
    const OPCODE: i16 = ServerPacketIds::NPCAwakening as i16;

    fn write_body<W: std::io::Write>(&self, _writer: &mut W) -> SharedResult<()> {
        // Empty packet - no data to write
        Ok(())
    }

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self {})
    }
}

/// NPCDisassemble - NPC分解 (223)
#[derive(Debug, Clone)]
pub struct NPCDisassemble {
    // C# 原实现为空
}

impl Packet for NPCDisassemble {
    const OPCODE: i16 = ServerPacketIds::NPCDisassemble as i16;

    fn write_body<W: std::io::Write>(&self, _writer: &mut W) -> SharedResult<()> {
        // Empty packet - no data to write
        Ok(())
    }

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self {})
    }
}

/// NPCDowngrade - NPC降级 (224)
#[derive(Debug, Clone)]
pub struct NPCDowngrade {
    // C# 原实现为空
}

impl Packet for NPCDowngrade {
    const OPCODE: i16 = ServerPacketIds::NPCDowngrade as i16;

    fn write_body<W: std::io::Write>(&self, _writer: &mut W) -> SharedResult<()> {
        // Empty packet - no data to write
        Ok(())
    }

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self {})
    }
}

/// NPCReset - NPC重置 (225)
#[derive(Debug, Clone)]
pub struct NPCReset {
    // C# 原实现为空
}

impl Packet for NPCReset {
    const OPCODE: i16 = ServerPacketIds::NPCReset as i16;

    fn write_body<W: std::io::Write>(&self, _writer: &mut W) -> SharedResult<()> {
        // Empty packet - no data to write
        Ok(())
    }

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self {})
    }
}

/// AwakeningNeedMaterials - 觉醒所需材料 (226)
#[derive(Debug, Clone)]
pub struct AwakeningNeedMaterials {
    pub item_id: i32,               // 物品ID
    pub materials: Vec<MaterialInfo>, // 材料列表
}

#[derive(Debug, Clone)]
pub struct MaterialInfo {
    pub item_id: i32,               // 材料物品ID
    pub count: i32,                 // 所需数量
}

impl Packet for AwakeningNeedMaterials {
    const OPCODE: i16 = ServerPacketIds::AwakeningNeedMaterials as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        writer.write_i32::<LittleEndian>(self.item_id)?;
        writer.write_i32::<LittleEndian>(self.materials.len() as i32)?;
        
        for material in &self.materials {
            writer.write_i32::<LittleEndian>(material.item_id)?;
            writer.write_i32::<LittleEndian>(material.count)?;
        }
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let item_id = reader.read_i32::<LittleEndian>()?;
        let count = reader.read_i32::<LittleEndian>()?;
        let mut materials = Vec::with_capacity(count as usize);
        
        for _ in 0..count {
            let mat_id = reader.read_i32::<LittleEndian>()?;
            let mat_count = reader.read_i32::<LittleEndian>()?;
            materials.push(MaterialInfo {
                item_id: mat_id,
                count: mat_count,
            });
        }
        
        Ok(Self { item_id, materials })
    }
}

/// AwakeningLockedItem - 觉醒锁定物品 (227)
#[derive(Debug, Clone)]
pub struct AwakeningLockedItem {
    pub unique_id: u64,             // 物品唯一ID
    pub locked: bool,               // 是否锁定
}

impl Packet for AwakeningLockedItem {
    const OPCODE: i16 = ServerPacketIds::AwakeningLockedItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u8(if self.locked { 1 } else { 0 })?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let locked = reader.read_u8()? != 0;
        Ok(Self { unique_id, locked })
    }
}

/// Awakening result codes
pub const AWAKE_RESULT_SUCCESS: i32 = 1;
pub const AWAKE_RESULT_FAIL: i32 = -1;
pub const AWAKE_RESULT_DESTROYED: i32 = 0;
pub const AWAKE_RESULT_MAX_LEVEL: i32 = -2;
pub const AWAKE_RESULT_NO_GOLD: i32 = -3;
pub const AWAKE_RESULT_NO_MATERIALS: i32 = -4;

/// Awakening - 觉醒结果 (228)
/// result: 1=success, 0=item destroyed (remove_id set), -1=fail, -2=max level, -3=no gold, -4=no materials
#[derive(Debug, Clone)]
pub struct Awakening {
    pub result: i32,
    pub remove_id: i64,
}

impl Packet for Awakening {
    const OPCODE: i16 = ServerPacketIds::Awakening as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_i32::<LittleEndian>(self.result)?;
        writer.write_i64::<LittleEndian>(self.remove_id)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let result = reader.read_i32::<LittleEndian>()?;
        let remove_id = reader.read_i64::<LittleEndian>()?;
        Ok(Self { result, remove_id })
    }
}
