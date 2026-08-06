//! Game Objects Packets
//!
//! Packets for spawning and managing game objects (players, monsters, NPCs).

use super::super::base::Packet;
use crate::binary::{read_dotnet_string, write_dotnet_string};
use crate::data::stats::SharedResult;
use crate::enums::{
    BuffType, LevelEffects, MirClass, MirDirection, MirGender, PoisonType, ServerPacketIds,
    SpellEffect,
};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

/// ObjectPlayer packet - spawns a player object
#[derive(Debug, Clone)]
pub struct ObjectPlayer {
    pub object_id: u32,
    pub name: String,
    pub guild_name: String,
    pub guild_rank_name: String,
    pub name_colour: i32,
    pub class: MirClass,
    pub gender: MirGender,
    pub level: u16,
    pub location_x: i32,
    pub location_y: i32,
    pub direction: MirDirection,
    pub hair: u8,
    pub light: u8,
    pub weapon: i16,
    pub weapon_effect: i16,
    pub armour: i16,
    pub poison: PoisonType,
    pub dead: bool,
    pub hidden: bool,
    pub effect: SpellEffect,
    pub wing_effect: u8,
    pub extra: bool,
    pub mount_type: i16,
    pub riding_mount: bool,
    pub fishing: bool,
    pub transform_type: i16,
    pub element_orb_effect: u32,
    pub element_orb_lvl: u32,
    pub element_orb_max: u32,
    pub buffs: Vec<BuffType>,
    pub level_effects: LevelEffects,
}

impl Packet for ObjectPlayer {
    const OPCODE: i16 = ServerPacketIds::ObjectPlayer as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let name = read_dotnet_string(reader)?;
        let guild_name = read_dotnet_string(reader)?;
        let guild_rank_name = read_dotnet_string(reader)?;
        let name_colour = reader.read_i32::<LittleEndian>()?;
        let class = MirClass::try_from(reader.read_u8()?)?;
        let gender = MirGender::try_from(reader.read_u8()?)?;
        let level = reader.read_u16::<LittleEndian>()?;
        let location_x = reader.read_i32::<LittleEndian>()?;
        let location_y = reader.read_i32::<LittleEndian>()?;
        let direction = MirDirection::try_from(reader.read_u8()?)?;
        let hair = reader.read_u8()?;
        let light = reader.read_u8()?;
        let weapon = reader.read_i16::<LittleEndian>()?;
        let weapon_effect = reader.read_i16::<LittleEndian>()?;
        let armour = reader.read_i16::<LittleEndian>()?;
        let poison = PoisonType::from_bits_truncate(reader.read_u16::<LittleEndian>()?);
        let dead = reader.read_u8()? != 0;
        let hidden = reader.read_u8()? != 0;
        let effect = SpellEffect::try_from(reader.read_u8()?)?;
        let wing_effect = reader.read_u8()?;
        let extra = reader.read_u8()? != 0;
        let mount_type = reader.read_i16::<LittleEndian>()?;
        let riding_mount = reader.read_u8()? != 0;
        let fishing = reader.read_u8()? != 0;
        let transform_type = reader.read_i16::<LittleEndian>()?;
        let element_orb_effect = reader.read_u32::<LittleEndian>()?;
        let element_orb_lvl = reader.read_u32::<LittleEndian>()?;
        let element_orb_max = reader.read_u32::<LittleEndian>()?;

        let buff_count = reader.read_i32::<LittleEndian>()?;
        let mut buffs = Vec::with_capacity(buff_count as usize);
        for _ in 0..buff_count {
            buffs.push(BuffType::try_from(reader.read_u8()?)?);
        }

        let level_effects = LevelEffects::from_bits_truncate(reader.read_u16::<LittleEndian>()?);

        Ok(Self {
            object_id,
            name,
            guild_name,
            guild_rank_name,
            name_colour,
            class,
            gender,
            level,
            location_x,
            location_y,
            direction,
            hair,
            light,
            weapon,
            weapon_effect,
            armour,
            poison,
            dead,
            hidden,
            effect,
            wing_effect,
            extra,
            mount_type,
            riding_mount,
            fishing,
            transform_type,
            element_orb_effect,
            element_orb_lvl,
            element_orb_max,
            buffs,
            level_effects,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        write_dotnet_string(writer, &self.name)?;
        write_dotnet_string(writer, &self.guild_name)?;
        write_dotnet_string(writer, &self.guild_rank_name)?;
        writer.write_i32::<LittleEndian>(self.name_colour)?;
        writer.write_u8(self.class as u8)?;
        writer.write_u8(self.gender as u8)?;
        writer.write_u16::<LittleEndian>(self.level)?;
        writer.write_i32::<LittleEndian>(self.location_x)?;
        writer.write_i32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction as u8)?;
        writer.write_u8(self.hair)?;
        writer.write_u8(self.light)?;
        writer.write_i16::<LittleEndian>(self.weapon)?;
        writer.write_i16::<LittleEndian>(self.weapon_effect)?;
        writer.write_i16::<LittleEndian>(self.armour)?;
        writer.write_u16::<LittleEndian>(self.poison.bits())?;
        writer.write_u8(self.dead as u8)?;
        writer.write_u8(self.hidden as u8)?;
        writer.write_u8(self.effect as u8)?;
        writer.write_u8(self.wing_effect)?;
        writer.write_u8(self.extra as u8)?;
        writer.write_i16::<LittleEndian>(self.mount_type)?;
        writer.write_u8(self.riding_mount as u8)?;
        writer.write_u8(self.fishing as u8)?;
        writer.write_i16::<LittleEndian>(self.transform_type)?;
        writer.write_u32::<LittleEndian>(self.element_orb_effect)?;
        writer.write_u32::<LittleEndian>(self.element_orb_lvl)?;
        writer.write_u32::<LittleEndian>(self.element_orb_max)?;

        writer.write_i32::<LittleEndian>(self.buffs.len() as i32)?;
        for buff in &self.buffs {
            writer.write_u8(*buff as u8)?;
        }

        writer.write_u16::<LittleEndian>(self.level_effects.bits())?;

        Ok(())
    }
}

/// ObjectHero packet - spawns a hero object (extends ObjectPlayer)
#[derive(Debug, Clone)]
pub struct ObjectHero {
    pub player: ObjectPlayer,
    pub owner_name: String,
}

impl Packet for ObjectHero {
    const OPCODE: i16 = ServerPacketIds::ObjectHero as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let player = ObjectPlayer::read_body(reader)?;
        let owner_name = read_dotnet_string(reader)?;
        Ok(Self { player, owner_name })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        self.player.write_body(writer)?;
        write_dotnet_string(writer, &self.owner_name)?;
        Ok(())
    }
}

/// ObjectMonster packet - spawns a monster object
#[derive(Debug, Clone)]
pub struct ObjectMonster {
    pub object_id: u32,
    pub name: String,
    pub name_colour: i32,
    pub location_x: i32,
    pub location_y: i32,
    pub image: u16, // Monster enum
    pub direction: MirDirection,
    pub effect: u8,
    pub ai: u8,
    pub light: u8,
    pub dead: bool,
    pub skeleton: bool,
    pub poison: PoisonType,
    pub hidden: bool,
    pub shock_time: i64,
    pub binding_shot_center: bool,
    pub extra: bool,
    pub extra_byte: u8,
    pub buffs: Vec<BuffType>,
}

impl Packet for ObjectMonster {
    const OPCODE: i16 = ServerPacketIds::ObjectMonster as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let name = read_dotnet_string(reader)?;
        let name_colour = reader.read_i32::<LittleEndian>()?;
        let location_x = reader.read_i32::<LittleEndian>()?;
        let location_y = reader.read_i32::<LittleEndian>()?;
        let image = reader.read_u16::<LittleEndian>()?;
        let direction = MirDirection::try_from(reader.read_u8()?)?;
        let effect = reader.read_u8()?;
        let ai = reader.read_u8()?;
        let light = reader.read_u8()?;
        let dead = reader.read_u8()? != 0;
        let skeleton = reader.read_u8()? != 0;
        let poison = PoisonType::from_bits_truncate(reader.read_u16::<LittleEndian>()?);
        let hidden = reader.read_u8()? != 0;
        let shock_time = reader.read_i64::<LittleEndian>()?;
        let binding_shot_center = reader.read_u8()? != 0;
        let extra = reader.read_u8()? != 0;
        let extra_byte = reader.read_u8()?;

        let buff_count = reader.read_i32::<LittleEndian>()?;
        let mut buffs = Vec::with_capacity(buff_count as usize);
        for _ in 0..buff_count {
            buffs.push(BuffType::try_from(reader.read_u8()?)?);
        }

        Ok(Self {
            object_id,
            name,
            name_colour,
            location_x,
            location_y,
            image,
            direction,
            effect,
            ai,
            light,
            dead,
            skeleton,
            poison,
            hidden,
            shock_time,
            binding_shot_center,
            extra,
            extra_byte,
            buffs,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        write_dotnet_string(writer, &self.name)?;
        writer.write_i32::<LittleEndian>(self.name_colour)?;
        writer.write_i32::<LittleEndian>(self.location_x)?;
        writer.write_i32::<LittleEndian>(self.location_y)?;
        writer.write_u16::<LittleEndian>(self.image)?;
        writer.write_u8(self.direction as u8)?;
        writer.write_u8(self.effect)?;
        writer.write_u8(self.ai)?;
        writer.write_u8(self.light)?;
        writer.write_u8(self.dead as u8)?;
        writer.write_u8(self.skeleton as u8)?;
        writer.write_u16::<LittleEndian>(self.poison.bits())?;
        writer.write_u8(self.hidden as u8)?;
        writer.write_i64::<LittleEndian>(self.shock_time)?;
        writer.write_u8(self.binding_shot_center as u8)?;
        writer.write_u8(self.extra as u8)?;
        writer.write_u8(self.extra_byte)?;

        writer.write_i32::<LittleEndian>(self.buffs.len() as i32)?;
        for buff in &self.buffs {
            writer.write_u8(*buff as u8)?;
        }

        Ok(())
    }
}

/// ObjectNpc packet - spawns an NPC object
#[derive(Debug, Clone)]
pub struct ObjectNpc {
    pub object_id: u32,
    pub name: String,
    pub name_colour: i32,
    pub image: u16, // NPC enum
    pub colour: i32,
    pub location_x: i32,
    pub location_y: i32,
    pub direction: MirDirection,
    pub quest_ids: Vec<i32>,
}

impl Packet for ObjectNpc {
    const OPCODE: i16 = ServerPacketIds::ObjectNpc as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let name = read_dotnet_string(reader)?;
        let name_colour = reader.read_i32::<LittleEndian>()?;
        let image = reader.read_u16::<LittleEndian>()?;
        let colour = reader.read_i32::<LittleEndian>()?;
        let location_x = reader.read_i32::<LittleEndian>()?;
        let location_y = reader.read_i32::<LittleEndian>()?;
        let direction = MirDirection::try_from(reader.read_u8()?)?;

        let count = reader.read_i32::<LittleEndian>()?;
        let mut quest_ids = Vec::with_capacity(count as usize);
        for _ in 0..count {
            quest_ids.push(reader.read_i32::<LittleEndian>()?);
        }

        Ok(Self {
            object_id,
            name,
            name_colour,
            image,
            colour,
            location_x,
            location_y,
            direction,
            quest_ids,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        write_dotnet_string(writer, &self.name)?;
        writer.write_i32::<LittleEndian>(self.name_colour)?;
        writer.write_u16::<LittleEndian>(self.image)?;
        writer.write_i32::<LittleEndian>(self.colour)?;
        writer.write_i32::<LittleEndian>(self.location_x)?;
        writer.write_i32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction as u8)?;

        writer.write_i32::<LittleEndian>(self.quest_ids.len() as i32)?;
        for quest_id in &self.quest_ids {
            writer.write_i32::<LittleEndian>(*quest_id)?;
        }

        Ok(())
    }
}

/// ObjectRemove packet - removes an object from the map
#[derive(Debug, Clone)]
pub struct ObjectRemove {
    pub object_id: u32,
}

impl Packet for ObjectRemove {
    const OPCODE: i16 = ServerPacketIds::ObjectRemove as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        Ok(Self { object_id })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        Ok(())
    }
}

/// ObjectTurn packet - object turns to face a direction
#[derive(Debug, Clone)]
pub struct ObjectTurn {
    pub object_id: u32,
    pub location_x: i32,
    pub location_y: i32,
    pub direction: MirDirection,
}

impl Packet for ObjectTurn {
    const OPCODE: i16 = ServerPacketIds::ObjectTurn as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let location_x = reader.read_i32::<LittleEndian>()?;
        let location_y = reader.read_i32::<LittleEndian>()?;
        let direction = MirDirection::try_from(reader.read_u8()?)?;
        Ok(Self {
            object_id,
            location_x,
            location_y,
            direction,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_i32::<LittleEndian>(self.location_x)?;
        writer.write_i32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction as u8)?;
        Ok(())
    }
}

/// ObjectWalk packet - object walks to a new location
#[derive(Debug, Clone)]
pub struct ObjectWalk {
    pub object_id: u32,
    pub location_x: i32,
    pub location_y: i32,
    pub direction: MirDirection,
}

impl Packet for ObjectWalk {
    const OPCODE: i16 = ServerPacketIds::ObjectWalk as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let location_x = reader.read_i32::<LittleEndian>()?;
        let location_y = reader.read_i32::<LittleEndian>()?;
        let direction = MirDirection::try_from(reader.read_u8()?)?;
        Ok(Self {
            object_id,
            location_x,
            location_y,
            direction,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_i32::<LittleEndian>(self.location_x)?;
        writer.write_i32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction as u8)?;
        Ok(())
    }
}

/// ObjectRun packet - object runs to a new location
#[derive(Debug, Clone)]
pub struct ObjectRun {
    pub object_id: u32,
    pub location_x: i32,
    pub location_y: i32,
    pub direction: MirDirection,
}

impl Packet for ObjectRun {
    const OPCODE: i16 = ServerPacketIds::ObjectRun as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let location_x = reader.read_i32::<LittleEndian>()?;
        let location_y = reader.read_i32::<LittleEndian>()?;
        let direction = MirDirection::try_from(reader.read_u8()?)?;
        Ok(Self {
            object_id,
            location_x,
            location_y,
            direction,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_i32::<LittleEndian>(self.location_x)?;
        writer.write_i32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction as u8)?;
        Ok(())
    }
}

/// ObjectHarvest packet - object harvests something
#[derive(Debug, Clone)]
pub struct ObjectHarvest {
    pub object_id: u32,
    pub location_x: i32,
    pub location_y: i32,
    pub direction: MirDirection,
}

impl Packet for ObjectHarvest {
    const OPCODE: i16 = ServerPacketIds::ObjectHarvest as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let location_x = reader.read_i32::<LittleEndian>()?;
        let location_y = reader.read_i32::<LittleEndian>()?;
        let direction = MirDirection::try_from(reader.read_u8()?)?;
        Ok(Self {
            object_id,
            location_x,
            location_y,
            direction,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_i32::<LittleEndian>(self.location_x)?;
        writer.write_i32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction as u8)?;
        Ok(())
    }
}

/// ObjectHarvested packet - object finishes harvesting
#[derive(Debug, Clone)]
pub struct ObjectHarvested {
    pub object_id: u32,
    pub location_x: i32,
    pub location_y: i32,
    pub direction: MirDirection,
}

impl Packet for ObjectHarvested {
    const OPCODE: i16 = ServerPacketIds::ObjectHarvested as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let location_x = reader.read_i32::<LittleEndian>()?;
        let location_y = reader.read_i32::<LittleEndian>()?;
        let direction = MirDirection::try_from(reader.read_u8()?)?;
        Ok(Self {
            object_id,
            location_x,
            location_y,
            direction,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_i32::<LittleEndian>(self.location_x)?;
        writer.write_i32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction as u8)?;
        Ok(())
    }
}
