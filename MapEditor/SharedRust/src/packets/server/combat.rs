//! Combat System Packets
//!
//! This module contains all combat-related packet definitions.

use std::io::{Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::{
    enums::ServerPacketIds,
    data::stats::SharedResult,
};
use super::super::base::Packet;


#[derive(Debug, Clone)]
pub struct ObjectAttack {
    pub object_id: u32,
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
    pub spell: u8,
    pub level: u16,
    pub attack_type: u8,
}

#[derive(Debug, Clone)]
pub struct Struck {
    pub attacker_id: u32,
}

#[derive(Debug, Clone)]
pub struct ObjectStruck {
    pub object_id: u32,
    pub attacker_id: u32,
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
}

#[derive(Debug, Clone)]
pub struct DamageIndicator {
    pub damage: i32,
    pub damage_type: u8,
    pub object_id: u32,
}

#[derive(Debug, Clone)]
pub struct Pushed {
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
}

#[derive(Debug, Clone)]
pub struct ObjectPushed {
    pub object_id: u32,
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
}

#[derive(Debug, Clone)]
pub struct RangeAttack {
    pub target_id: u32,
    pub target_x: u32,
    pub target_y: u32,
    pub spell: u16,
    pub spell_level: u16,
}

#[derive(Debug, Clone)]
pub struct ObjectRangeAttack {
    pub object_id: u32,
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
    pub target_id: u32,
    pub target_x: u32,
    pub target_y: u32,
    /// C# S.ObjectRangeAttack.Type
    pub attack_type: u8,
    pub spell: u8,
    pub spell_level: u8,
}

#[derive(Debug, Clone)]
pub struct UserDash {
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
}

#[derive(Debug, Clone)]
pub struct ObjectDash {
    pub object_id: u32,
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
}

#[derive(Debug, Clone)]
pub struct UserDashFail {
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
}

#[derive(Debug, Clone)]
pub struct ObjectDashFail {
    pub object_id: u32,
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
}

#[derive(Debug, Clone)]
pub struct Death {
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
}

#[derive(Debug, Clone)]
pub struct ObjectDied {
    pub object_id: u32,
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
    pub death_type: u8,
}

#[derive(Debug, Clone)]
pub struct Revived;

#[derive(Debug, Clone)]
pub struct ObjectRevived {
    pub object_id: u32,
    pub effect: u8,
}

#[derive(Debug, Clone)]
pub struct HealthChanged {
    pub hp: u32,
    pub mp: u32,
}

#[derive(Debug, Clone)]
pub struct HeroHealthChanged {
    pub hp: u32,
    pub mp: u32,
}

// ==================== 解析函数 ====================

impl Packet for ObjectAttack {
    const OPCODE: i16 = ServerPacketIds::ObjectAttack as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(ObjectAttack {
            object_id: reader.read_u32::<LittleEndian>()?,
            location_x: reader.read_u32::<LittleEndian>()?,
            location_y: reader.read_u32::<LittleEndian>()?,
            direction: reader.read_u8()?,
            spell: reader.read_u8()?,
            level: reader.read_u16::<LittleEndian>()?,
            attack_type: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u32::<LittleEndian>(self.location_x)?;
        writer.write_u32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction)?;
        writer.write_u8(self.spell)?;
        writer.write_u16::<LittleEndian>(self.level)?;
        writer.write_u8(self.attack_type)?;
        Ok(())
    }
}

impl Packet for Struck {
    const OPCODE: i16 = ServerPacketIds::Struck as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(Struck {
            attacker_id: reader.read_u32::<LittleEndian>()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.attacker_id)?;
        Ok(())
    }
}

impl Packet for ObjectStruck {
    const OPCODE: i16 = ServerPacketIds::ObjectStruck as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(ObjectStruck {
            object_id: reader.read_u32::<LittleEndian>()?,
            attacker_id: reader.read_u32::<LittleEndian>()?,
            location_x: reader.read_u32::<LittleEndian>()?,
            location_y: reader.read_u32::<LittleEndian>()?,
            direction: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u32::<LittleEndian>(self.attacker_id)?;
        writer.write_u32::<LittleEndian>(self.location_x)?;
        writer.write_u32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction)?;
        Ok(())
    }
}

impl Packet for DamageIndicator {
    const OPCODE: i16 = ServerPacketIds::DamageIndicator as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(DamageIndicator {
            damage: reader.read_i32::<LittleEndian>()?,
            damage_type: reader.read_u8()?,
            object_id: reader.read_u32::<LittleEndian>()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.damage)?;
        writer.write_u8(self.damage_type)?;
        writer.write_u32::<LittleEndian>(self.object_id)?;
        Ok(())
    }
}

impl Packet for Pushed {
    const OPCODE: i16 = ServerPacketIds::Pushed as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(Pushed {
            location_x: reader.read_u32::<LittleEndian>()?,
            location_y: reader.read_u32::<LittleEndian>()?,
            direction: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.location_x)?;
        writer.write_u32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction)?;
        Ok(())
    }
}

impl Packet for ObjectPushed {
    const OPCODE: i16 = ServerPacketIds::ObjectPushed as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(ObjectPushed {
            object_id: reader.read_u32::<LittleEndian>()?,
            location_x: reader.read_u32::<LittleEndian>()?,
            location_y: reader.read_u32::<LittleEndian>()?,
            direction: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u32::<LittleEndian>(self.location_x)?;
        writer.write_u32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction)?;
        Ok(())
    }
}

impl Packet for RangeAttack {
    const OPCODE: i16 = ServerPacketIds::RangeAttack as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(RangeAttack {
            target_id: reader.read_u32::<LittleEndian>()?,
            target_x: reader.read_u32::<LittleEndian>()?,
            target_y: reader.read_u32::<LittleEndian>()?,
            spell: reader.read_u16::<LittleEndian>()?,
            spell_level: reader.read_u16::<LittleEndian>()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.target_id)?;
        writer.write_u32::<LittleEndian>(self.target_x)?;
        writer.write_u32::<LittleEndian>(self.target_y)?;
        writer.write_u16::<LittleEndian>(self.spell)?;
        writer.write_u16::<LittleEndian>(self.spell_level)?;
        Ok(())
    }
}

impl Packet for ObjectRangeAttack {
    const OPCODE: i16 = ServerPacketIds::ObjectRangeAttack as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(ObjectRangeAttack {
            object_id: reader.read_u32::<LittleEndian>()?,
            location_x: reader.read_u32::<LittleEndian>()?,
            location_y: reader.read_u32::<LittleEndian>()?,
            direction: reader.read_u8()?,
            target_id: reader.read_u32::<LittleEndian>()?,
            target_x: reader.read_u32::<LittleEndian>()?,
            target_y: reader.read_u32::<LittleEndian>()?,
            attack_type: reader.read_u8()?,
            spell: reader.read_u8()?,
            spell_level: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u32::<LittleEndian>(self.location_x)?;
        writer.write_u32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction)?;
        writer.write_u32::<LittleEndian>(self.target_id)?;
        writer.write_u32::<LittleEndian>(self.target_x)?;
        writer.write_u32::<LittleEndian>(self.target_y)?;
        writer.write_u8(self.attack_type)?;
        writer.write_u8(self.spell)?;
        writer.write_u8(self.spell_level)?;
        Ok(())
    }
}

impl Packet for UserDash {
    const OPCODE: i16 = ServerPacketIds::UserDash as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(UserDash {
            location_x: reader.read_u32::<LittleEndian>()?,
            location_y: reader.read_u32::<LittleEndian>()?,
            direction: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.location_x)?;
        writer.write_u32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction)?;
        Ok(())
    }
}

impl Packet for ObjectDash {
    const OPCODE: i16 = ServerPacketIds::ObjectDash as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(ObjectDash {
            object_id: reader.read_u32::<LittleEndian>()?,
            location_x: reader.read_u32::<LittleEndian>()?,
            location_y: reader.read_u32::<LittleEndian>()?,
            direction: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u32::<LittleEndian>(self.location_x)?;
        writer.write_u32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction)?;
        Ok(())
    }
}

impl Packet for UserDashFail {
    const OPCODE: i16 = ServerPacketIds::UserDashFail as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(UserDashFail {
            location_x: reader.read_u32::<LittleEndian>()?,
            location_y: reader.read_u32::<LittleEndian>()?,
            direction: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.location_x)?;
        writer.write_u32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction)?;
        Ok(())
    }
}

impl Packet for ObjectDashFail {
    const OPCODE: i16 = ServerPacketIds::ObjectDashFail as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(ObjectDashFail {
            object_id: reader.read_u32::<LittleEndian>()?,
            location_x: reader.read_u32::<LittleEndian>()?,
            location_y: reader.read_u32::<LittleEndian>()?,
            direction: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u32::<LittleEndian>(self.location_x)?;
        writer.write_u32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction)?;
        Ok(())
    }
}

impl Packet for Death {
    const OPCODE: i16 = ServerPacketIds::Death as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(Death {
            location_x: reader.read_u32::<LittleEndian>()?,
            location_y: reader.read_u32::<LittleEndian>()?,
            direction: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.location_x)?;
        writer.write_u32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction)?;
        Ok(())
    }
}

impl Packet for ObjectDied {
    const OPCODE: i16 = ServerPacketIds::ObjectDied as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(ObjectDied {
            object_id: reader.read_u32::<LittleEndian>()?,
            location_x: reader.read_u32::<LittleEndian>()?,
            location_y: reader.read_u32::<LittleEndian>()?,
            direction: reader.read_u8()?,
            death_type: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u32::<LittleEndian>(self.location_x)?;
        writer.write_u32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction)?;
        writer.write_u8(self.death_type)?;
        Ok(())
    }
}

impl Packet for Revived {
    const OPCODE: i16 = ServerPacketIds::Revived as i16;

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Revived)
    }

    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

impl Packet for ObjectRevived {
    const OPCODE: i16 = ServerPacketIds::ObjectRevived as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(ObjectRevived {
            object_id: reader.read_u32::<LittleEndian>()?,
            effect: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u8(self.effect)?;
        Ok(())
    }
}

impl Packet for HealthChanged {
    const OPCODE: i16 = ServerPacketIds::HealthChanged as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(HealthChanged {
            hp: reader.read_u32::<LittleEndian>()?,
            mp: reader.read_u32::<LittleEndian>()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.hp)?;
        writer.write_u32::<LittleEndian>(self.mp)?;
        Ok(())
    }
}

impl Packet for HeroHealthChanged {
    const OPCODE: i16 = ServerPacketIds::HeroHealthChanged as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(HeroHealthChanged {
            hp: reader.read_u32::<LittleEndian>()?,
            mp: reader.read_u32::<LittleEndian>()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.hp)?;
        writer.write_u32::<LittleEndian>(self.mp)?;
        Ok(())
    }
}
