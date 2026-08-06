// 魔法战斗相关的服务器数据包

use super::super::base::Packet;
use crate::data::stats::SharedResult;
use crate::enums::{MirDirection, ServerPacketIds, Spell, SpellEffect};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

/// 玩家施放魔法
#[derive(Debug, Clone)]
pub struct Magic {
    pub spell: Spell,                   // 魔法类型
    pub target_id: u32,                 // 目标 ID
    pub target_x: i32,                  // 目标 X 坐标
    pub target_y: i32,                  // 目标 Y 坐标
    pub cast: bool,                     // 是否施放
    pub level: u8,                      // 魔法等级
    pub secondary_target_ids: Vec<u32>, // 次要目标 ID 列表（多目标魔法）
}

impl Packet for Magic {
    const OPCODE: i16 = ServerPacketIds::Magic as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let spell = Spell::try_from(reader.read_u8()?)?;
        let target_id = reader.read_u32::<LittleEndian>()?;
        let target_x = reader.read_i32::<LittleEndian>()?;
        let target_y = reader.read_i32::<LittleEndian>()?;
        let cast = reader.read_u8()? != 0;
        let level = reader.read_u8()?;

        let count = reader.read_i32::<LittleEndian>()? as usize;
        let mut secondary_target_ids = Vec::with_capacity(count);
        for _ in 0..count {
            secondary_target_ids.push(reader.read_u32::<LittleEndian>()?);
        }

        Ok(Self {
            spell,
            target_id,
            target_x,
            target_y,
            cast,
            level,
            secondary_target_ids,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.spell as u8)?;
        writer.write_u32::<LittleEndian>(self.target_id)?;
        writer.write_i32::<LittleEndian>(self.target_x)?;
        writer.write_i32::<LittleEndian>(self.target_y)?;
        writer.write_u8(if self.cast { 1 } else { 0 })?;
        writer.write_u8(self.level)?;

        writer.write_i32::<LittleEndian>(self.secondary_target_ids.len() as i32)?;
        for &target_id in &self.secondary_target_ids {
            writer.write_u32::<LittleEndian>(target_id)?;
        }

        Ok(())
    }
}

/// 魔法延迟（冷却时间）
#[derive(Debug, Clone)]
pub struct MagicDelay {
    pub object_id: u32, // 对象 ID
    pub spell: Spell,   // 魔法类型
    pub delay: i64,     // 延迟时间（毫秒）
}

impl Packet for MagicDelay {
    const OPCODE: i16 = ServerPacketIds::MagicDelay as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let spell = Spell::try_from(reader.read_u8()?)?;
        let delay = reader.read_i64::<LittleEndian>()?;
        Ok(Self {
            object_id,
            spell,
            delay,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u8(self.spell as u8)?;
        writer.write_i64::<LittleEndian>(self.delay)?;
        Ok(())
    }
}

/// 魔法施放确认
#[derive(Debug, Clone)]
pub struct MagicCast {
    pub spell: Spell, // 魔法类型
}

impl Packet for MagicCast {
    const OPCODE: i16 = ServerPacketIds::MagicCast as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let spell = Spell::try_from(reader.read_u8()?)?;
        Ok(Self { spell })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.spell as u8)?;
        Ok(())
    }
}

/// 对象施放魔法（其他玩家或怪物）
#[derive(Debug, Clone)]
pub struct ObjectMagic {
    pub object_id: u32,          // 施放者 ID
    pub location_x: i32,         // 施放者位置 X
    pub location_y: i32,         // 施放者位置 Y
    pub direction: MirDirection, // 施放者朝向

    pub spell: Spell,                   // 魔法类型
    pub target_id: u32,                 // 目标 ID
    pub target_x: i32,                  // 目标 X 坐标
    pub target_y: i32,                  // 目标 Y 坐标
    pub cast: bool,                     // 是否施放
    pub level: u8,                      // 魔法等级
    pub self_broadcast: bool,           // 是否广播给自己
    pub secondary_target_ids: Vec<u32>, // 次要目标 ID 列表
}

impl Packet for ObjectMagic {
    const OPCODE: i16 = ServerPacketIds::ObjectMagic as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let location_x = reader.read_i32::<LittleEndian>()?;
        let location_y = reader.read_i32::<LittleEndian>()?;
        let direction = MirDirection::try_from(reader.read_u8()?)?;

        let spell = Spell::try_from(reader.read_u8()?)?;
        let target_id = reader.read_u32::<LittleEndian>()?;

        let target_x = reader.read_i32::<LittleEndian>()?;
        let target_y = reader.read_i32::<LittleEndian>()?;
        let cast = reader.read_u8()? != 0;
        let level = reader.read_u8()?;
        let self_broadcast = reader.read_u8()? != 0;

        let count = reader.read_i32::<LittleEndian>()? as usize;
        let mut secondary_target_ids = Vec::with_capacity(count);
        for _ in 0..count {
            secondary_target_ids.push(reader.read_u32::<LittleEndian>()?);
        }

        Ok(Self {
            object_id,
            location_x,
            location_y,
            direction,
            spell,
            target_id,
            target_x,
            target_y,
            cast,
            level,
            self_broadcast,
            secondary_target_ids,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_i32::<LittleEndian>(self.location_x)?;
        writer.write_i32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction as u8)?;

        writer.write_u8(self.spell as u8)?;
        writer.write_u32::<LittleEndian>(self.target_id)?;

        writer.write_i32::<LittleEndian>(self.target_x)?;
        writer.write_i32::<LittleEndian>(self.target_y)?;
        writer.write_u8(if self.cast { 1 } else { 0 })?;
        writer.write_u8(self.level)?;
        writer.write_u8(if self.self_broadcast { 1 } else { 0 })?;

        writer.write_i32::<LittleEndian>(self.secondary_target_ids.len() as i32)?;
        for &target_id in &self.secondary_target_ids {
            writer.write_u32::<LittleEndian>(target_id)?;
        }

        Ok(())
    }
}

/// 对象特效
#[derive(Debug, Clone)]
pub struct ObjectEffect {
    pub object_id: u32,      // 对象 ID
    pub effect: SpellEffect, // 特效类型
    pub effect_type: u32,    // 特效类型编号
    pub delay_time: u32,     // 延迟时间
    pub time: u32,           // 持续时间
}

impl Packet for ObjectEffect {
    const OPCODE: i16 = ServerPacketIds::ObjectEffect as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let effect = SpellEffect::try_from(reader.read_u8()?)?;
        let effect_type = reader.read_u32::<LittleEndian>()?;
        let delay_time = reader.read_u32::<LittleEndian>()?;
        let time = reader.read_u32::<LittleEndian>()?;
        Ok(Self {
            object_id,
            effect,
            effect_type,
            delay_time,
            time,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u8(self.effect as u8)?;
        writer.write_u32::<LittleEndian>(self.effect_type)?;
        writer.write_u32::<LittleEndian>(self.delay_time)?;
        writer.write_u32::<LittleEndian>(self.time)?;
        Ok(())
    }
}

/// 对象投射物（箭矢、火球等）
#[derive(Debug, Clone)]
pub struct ObjectProjectile {
    pub spell: Spell,     // 魔法类型
    pub source: u32,      // 来源对象 ID
    pub destination: u32, // 目标对象 ID
}

impl Packet for ObjectProjectile {
    const OPCODE: i16 = ServerPacketIds::ObjectProjectile as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let spell = Spell::try_from(reader.read_u8()?)?;
        let source = reader.read_u32::<LittleEndian>()?;
        let destination = reader.read_u32::<LittleEndian>()?;
        Ok(Self {
            spell,
            source,
            destination,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.spell as u8)?;
        writer.write_u32::<LittleEndian>(self.source)?;
        writer.write_u32::<LittleEndian>(self.destination)?;
        Ok(())
    }
}

/// 对象魔法效果（buff/debuff显示）
#[derive(Debug, Clone)]
pub struct ObjectSpell {
    pub object_id: u32,  // 对象 ID
    pub location_x: i32, // 位置 X
    pub location_y: i32, // 位置 Y
    pub spell: Spell,    // 魔法类型
}

impl Packet for ObjectSpell {
    const OPCODE: i16 = ServerPacketIds::ObjectSpell as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let location_x = reader.read_i32::<LittleEndian>()?;
        let location_y = reader.read_i32::<LittleEndian>()?;
        let spell = Spell::try_from(reader.read_u8()?)?;
        Ok(Self {
            object_id,
            location_x,
            location_y,
            spell,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_i32::<LittleEndian>(self.location_x)?;
        writer.write_i32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.spell as u8)?;
        Ok(())
    }
}
