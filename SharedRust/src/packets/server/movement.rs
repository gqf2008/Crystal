// 高级移动相关的服务器数据包

use super::super::base::Packet;
use crate::data::stats::SharedResult;
use crate::enums::{MirDirection, ServerPacketIds};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

/// 玩家后退
#[derive(Debug, Clone)]
pub struct UserBackStep {
    pub location_x: i32,         // 位置 X
    pub location_y: i32,         // 位置 Y
    pub direction: MirDirection, // 朝向
}

impl Packet for UserBackStep {
    const OPCODE: i16 = ServerPacketIds::UserBackStep as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let location_x = reader.read_i32::<LittleEndian>()?;
        let location_y = reader.read_i32::<LittleEndian>()?;
        let direction = MirDirection::try_from(reader.read_u8()?)?;
        Ok(Self {
            location_x,
            location_y,
            direction,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.location_x)?;
        writer.write_i32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction as u8)?;
        Ok(())
    }
}

/// 对象后退
#[derive(Debug, Clone)]
pub struct ObjectBackStep {
    pub object_id: u32,          // 对象 ID
    pub location_x: i32,         // 位置 X
    pub location_y: i32,         // 位置 Y
    pub direction: MirDirection, // 朝向
    pub distance: i32,           // 后退距离
}

impl Packet for ObjectBackStep {
    const OPCODE: i16 = ServerPacketIds::ObjectBackStep as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let location_x = reader.read_i32::<LittleEndian>()?;
        let location_y = reader.read_i32::<LittleEndian>()?;
        let direction = MirDirection::try_from(reader.read_u8()?)?;
        let distance = reader.read_i32::<LittleEndian>()?;
        Ok(Self {
            object_id,
            location_x,
            location_y,
            direction,
            distance,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_i32::<LittleEndian>(self.location_x)?;
        writer.write_i32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction as u8)?;
        writer.write_i32::<LittleEndian>(self.distance)?;
        Ok(())
    }
}

/// 玩家冲刺攻击
#[derive(Debug, Clone)]
pub struct UserDashAttack {
    pub location_x: i32,         // 位置 X
    pub location_y: i32,         // 位置 Y
    pub direction: MirDirection, // 朝向
}

impl Packet for UserDashAttack {
    const OPCODE: i16 = ServerPacketIds::UserDashAttack as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let location_x = reader.read_i32::<LittleEndian>()?;
        let location_y = reader.read_i32::<LittleEndian>()?;
        let direction = MirDirection::try_from(reader.read_u8()?)?;
        Ok(Self {
            location_x,
            location_y,
            direction,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.location_x)?;
        writer.write_i32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction as u8)?;
        Ok(())
    }
}

/// 对象冲刺攻击
#[derive(Debug, Clone)]
pub struct ObjectDashAttack {
    pub object_id: u32,          // 对象 ID
    pub location_x: i32,         // 位置 X
    pub location_y: i32,         // 位置 Y
    pub direction: MirDirection, // 朝向
    pub distance: i32,           // 冲刺距离
}

impl Packet for ObjectDashAttack {
    const OPCODE: i16 = ServerPacketIds::ObjectDashAttack as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let location_x = reader.read_i32::<LittleEndian>()?;
        let location_y = reader.read_i32::<LittleEndian>()?;
        let direction = MirDirection::try_from(reader.read_u8()?)?;
        let distance = reader.read_i32::<LittleEndian>()?;
        Ok(Self {
            object_id,
            location_x,
            location_y,
            direction,
            distance,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_i32::<LittleEndian>(self.location_x)?;
        writer.write_i32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction as u8)?;
        writer.write_i32::<LittleEndian>(self.distance)?;
        Ok(())
    }
}

/// 玩家攻击移动（战士技能 - 斩击爆发）
#[derive(Debug, Clone)]
pub struct UserAttackMove {
    pub location_x: i32,         // 位置 X
    pub location_y: i32,         // 位置 Y
    pub direction: MirDirection, // 朝向
}

impl Packet for UserAttackMove {
    const OPCODE: i16 = ServerPacketIds::UserAttackMove as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let location_x = reader.read_i32::<LittleEndian>()?;
        let location_y = reader.read_i32::<LittleEndian>()?;
        let direction = MirDirection::try_from(reader.read_u8()?)?;
        Ok(Self {
            location_x,
            location_y,
            direction,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.location_x)?;
        writer.write_i32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction as u8)?;
        Ok(())
    }
}

/// 设置专注（技能状态）
#[derive(Debug, Clone)]
pub struct SetConcentration {
    pub object_id: u32,    // 对象 ID
    pub enabled: bool,     // 是否启用
    pub interrupted: bool, // 是否被打断
}

impl Packet for SetConcentration {
    const OPCODE: i16 = ServerPacketIds::SetConcentration as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let enabled = reader.read_u8()? != 0;
        let interrupted = reader.read_u8()? != 0;
        Ok(Self {
            object_id,
            enabled,
            interrupted,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u8(if self.enabled { 1 } else { 0 })?;
        writer.write_u8(if self.interrupted { 1 } else { 0 })?;
        Ok(())
    }
}

/// 设置元素属性（魔法师技能）
#[derive(Debug, Clone)]
pub struct SetElemental {
    pub object_id: u32,   // 对象 ID
    pub enabled: bool,    // 是否启用
    pub value: u32,       // 元素值
    pub element: u8,      // 元素类型
    pub expire_time: i64, // 过期时间
}

impl Packet for SetElemental {
    const OPCODE: i16 = ServerPacketIds::SetElemental as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let enabled = reader.read_u8()? != 0;
        let value = reader.read_u32::<LittleEndian>()?;
        let element = reader.read_u8()?;
        let expire_time = reader.read_i64::<LittleEndian>()?;
        Ok(Self {
            object_id,
            enabled,
            value,
            element,
            expire_time,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u8(if self.enabled { 1 } else { 0 })?;
        writer.write_u32::<LittleEndian>(self.value)?;
        writer.write_u8(self.element)?;
        writer.write_i64::<LittleEndian>(self.expire_time)?;
        Ok(())
    }
}

/// 对象装饰（外观效果）
#[derive(Debug, Clone)]
pub struct ObjectDeco {
    pub object_id: u32, // 对象 ID
    pub deco: u16,      // 装饰编号
    pub remove: bool,   // 是否移除
}

impl Packet for ObjectDeco {
    const OPCODE: i16 = ServerPacketIds::ObjectDeco as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let deco = reader.read_u16::<LittleEndian>()?;
        let remove = reader.read_u8()? != 0;
        Ok(Self {
            object_id,
            deco,
            remove,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u16::<LittleEndian>(self.deco)?;
        writer.write_u8(if self.remove { 1 } else { 0 })?;
        Ok(())
    }
}

/// 对象潜行状态
#[derive(Debug, Clone)]
pub struct ObjectSneaking {
    pub object_id: u32, // 对象 ID
    pub sneaking: bool, // 是否潜行
}

impl Packet for ObjectSneaking {
    const OPCODE: i16 = ServerPacketIds::ObjectSneaking as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let sneaking = reader.read_u8()? != 0;
        Ok(Self {
            object_id,
            sneaking,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u8(if self.sneaking { 1 } else { 0 })?;
        Ok(())
    }
}

/// 对象等级特效
#[derive(Debug, Clone)]
pub struct ObjectLevelEffects {
    pub object_id: u32,     // 对象 ID
    pub level_effects: u16, // 等级特效标志
}

impl Packet for ObjectLevelEffects {
    const OPCODE: i16 = ServerPacketIds::ObjectLevelEffects as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let level_effects = reader.read_u16::<LittleEndian>()?;
        Ok(Self {
            object_id,
            level_effects,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u16::<LittleEndian>(self.level_effects)?;
        Ok(())
    }
}
