//! User Information and Location Packets
//!
//! Packets related to user state, inventory, and location.

use std::io::{Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::data::stats::{SharedResult, SharedError};
use crate::binary::{read_dotnet_string, write_dotnet_string};
use crate::data::item::UserItem;
use crate::data::client_data::ClientMagic;
use crate::enums::{ServerPacketIds, MirClass, MirGender, MirDirection, HeroBehaviour, LevelEffects};
use super::super::base::Packet;

/// UserInformation packet - complete user state information
#[derive(Debug, Clone)]
pub struct UserInformation {
    pub object_id: u32,
    pub real_id: u32,
    pub name: String,
    pub guild_name: String,
    pub guild_rank: String,
    pub name_colour: i32,
    pub class: MirClass,
    pub gender: MirGender,
    pub level: u16,
    pub location_x: i32,
    pub location_y: i32,
    pub direction: MirDirection,
    pub hair: u8,
    pub hp: i32,
    pub mp: i32,
    pub experience: i64,
    pub max_experience: i64,
    pub level_effects: LevelEffects,
    pub has_hero: bool,
    pub hero_behaviour: HeroBehaviour,
    pub inventory: Option<Vec<Option<UserItem>>>,
    pub equipment: Option<Vec<Option<UserItem>>>,
    pub quest_inventory: Option<Vec<Option<UserItem>>>,
    pub gold: u32,
    pub credit: u32,
    pub has_expanded_storage: bool,
    pub expanded_storage_expiry_time: i64,
    pub magics: Vec<ClientMagic>,
    pub summoned_creature_type: u8,
    pub creature_summoned: bool,
    pub allow_observe: bool,
    pub observer: bool,
}

impl Packet for UserInformation {
    const OPCODE: i16 = ServerPacketIds::UserInformation as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let real_id = reader.read_u32::<LittleEndian>()?;
        let name = read_dotnet_string(reader)?;
        let guild_name = read_dotnet_string(reader)?;
        let guild_rank = read_dotnet_string(reader)?;
        let name_colour = reader.read_i32::<LittleEndian>()?;
        let class = MirClass::try_from(reader.read_u8()?)?;
        let gender = MirGender::try_from(reader.read_u8()?)?;
        let level = reader.read_u16::<LittleEndian>()?;
        let location_x = reader.read_i32::<LittleEndian>()?;
        let location_y = reader.read_i32::<LittleEndian>()?;
        let direction = MirDirection::try_from(reader.read_u8()?)?;
        let hair = reader.read_u8()?;
        let hp = reader.read_i32::<LittleEndian>()?;
        let mp = reader.read_i32::<LittleEndian>()?;
        let experience = reader.read_i64::<LittleEndian>()?;
        let max_experience = reader.read_i64::<LittleEndian>()?;
        let level_effects = LevelEffects::from_bits_truncate(reader.read_u16::<LittleEndian>()?);
        let has_hero = reader.read_u8()? != 0;
        let hero_behaviour = HeroBehaviour::try_from(reader.read_u8()?)?;

        // Read inventory
        let inventory = if reader.read_u8()? != 0 {
            let count = reader.read_i32::<LittleEndian>()?;
            const MAX_INVENTORY_SIZE: i32 = 1000; // 合理上限
            if !(0..=MAX_INVENTORY_SIZE).contains(&count) {
                eprintln!("[UserInformation] Invalid inventory count: {}", count);
                return Err(SharedError::PacketTooLarge(count as usize));
            }
            let count = count as usize;
            let mut items = Vec::with_capacity(count);
            for _ in 0..count {
                if reader.read_u8()? != 0 {
                    items.push(Some(UserItem::read_from(reader, i32::MAX, i32::MAX)?));
                } else {
                    items.push(None);
                }
            }
            Some(items)
        } else {
            None
        };

        // Read equipment
        let equipment = if reader.read_u8()? != 0 {
            let count = reader.read_i32::<LittleEndian>()?;
            const MAX_EQUIPMENT_SIZE: i32 = 100; // 合理上限
            if !(0..=MAX_EQUIPMENT_SIZE).contains(&count) {
                eprintln!("[UserInformation] Invalid equipment count: {}", count);
                return Err(SharedError::PacketTooLarge(count as usize));
            }
            let count = count as usize;
            let mut items = Vec::with_capacity(count);
            for _ in 0..count {
                if reader.read_u8()? != 0 {
                    items.push(Some(UserItem::read_from(reader, i32::MAX, i32::MAX)?));
                } else {
                    items.push(None);
                }
            }
            Some(items)
        } else {
            None
        };

        // Read quest inventory
        let quest_inventory = if reader.read_u8()? != 0 {
            let count = reader.read_i32::<LittleEndian>()?;
            const MAX_QUEST_INVENTORY_SIZE: i32 = 500; // 合理上限
            if !(0..=MAX_QUEST_INVENTORY_SIZE).contains(&count) {
                eprintln!("[UserInformation] Invalid quest inventory count: {}", count);
                return Err(SharedError::PacketTooLarge(count as usize));
            }
            let count = count as usize;
            let mut items = Vec::with_capacity(count);
            for _ in 0..count {
                if reader.read_u8()? != 0 {
                    items.push(Some(UserItem::read_from(reader, i32::MAX, i32::MAX)?));
                } else {
                    items.push(None);
                }
            }
            Some(items)
        } else {
            None
        };

        let gold = reader.read_u32::<LittleEndian>()?;
        let credit = reader.read_u32::<LittleEndian>()?;
        let has_expanded_storage = reader.read_u8()? != 0;
        let expanded_storage_expiry_time = reader.read_i64::<LittleEndian>()?;

        // Read magics
        let magic_count = reader.read_i32::<LittleEndian>()?;
        const MAX_MAGIC_COUNT: i32 = 500;
        if !(0..=MAX_MAGIC_COUNT).contains(&magic_count) {
            eprintln!("[UserInformation] Invalid magic_count: {}", magic_count);
            return Err(SharedError::PacketTooLarge(magic_count as usize));
        }
        let magic_count = magic_count as usize;
        let mut magics = Vec::with_capacity(magic_count);
        for _ in 0..magic_count {
            magics.push(ClientMagic::read_from(reader)?);
        }

        // Skip intelligent creatures for now
        let creature_count = reader.read_i32::<LittleEndian>()?;
        const MAX_CREATURE_COUNT: i32 = 100;
        if !(0..=MAX_CREATURE_COUNT).contains(&creature_count) {
            eprintln!("[UserInformation] Invalid creature_count: {}", creature_count);
            return Err(SharedError::PacketTooLarge(creature_count as usize));
        }
        for _ in 0..creature_count {
            // Skip creature data - not implemented yet
            let _ = reader.read_u8()?; // Placeholder
        }

        let summoned_creature_type = reader.read_u8()?;
        let creature_summoned = reader.read_u8()? != 0;
        let allow_observe = reader.read_u8()? != 0;
        let observer = reader.read_u8()? != 0;

        Ok(Self {
            object_id,
            real_id,
            name,
            guild_name,
            guild_rank,
            name_colour,
            class,
            gender,
            level,
            location_x,
            location_y,
            direction,
            hair,
            hp,
            mp,
            experience,
            max_experience,
            level_effects,
            has_hero,
            hero_behaviour,
            inventory,
            equipment,
            quest_inventory,
            gold,
            credit,
            has_expanded_storage,
            expanded_storage_expiry_time,
            magics,
            summoned_creature_type,
            creature_summoned,
            allow_observe,
            observer,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u32::<LittleEndian>(self.real_id)?;
        write_dotnet_string(writer, &self.name)?;
        write_dotnet_string(writer, &self.guild_name)?;
        write_dotnet_string(writer, &self.guild_rank)?;
        writer.write_i32::<LittleEndian>(self.name_colour)?;
        writer.write_u8(self.class as u8)?;
        writer.write_u8(self.gender as u8)?;
        writer.write_u16::<LittleEndian>(self.level)?;
        writer.write_i32::<LittleEndian>(self.location_x)?;
        writer.write_i32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction as u8)?;
        writer.write_u8(self.hair)?;
        writer.write_i32::<LittleEndian>(self.hp)?;
        writer.write_i32::<LittleEndian>(self.mp)?;
        writer.write_i64::<LittleEndian>(self.experience)?;
        writer.write_i64::<LittleEndian>(self.max_experience)?;
        writer.write_u16::<LittleEndian>(self.level_effects.bits())?;
        writer.write_u8(self.has_hero as u8)?;
        writer.write_u8(self.hero_behaviour as u8)?;

        // Write inventory
        if let Some(ref inventory) = self.inventory {
            writer.write_u8(1)?;
            writer.write_i32::<LittleEndian>(inventory.len() as i32)?;
            for item in inventory {
                if let Some(ref item) = item {
                    writer.write_u8(1)?;
                    item.write_to(writer)?;
                } else {
                    writer.write_u8(0)?;
                }
            }
        } else {
            writer.write_u8(0)?;
        }

        // Write equipment
        if let Some(ref equipment) = self.equipment {
            writer.write_u8(1)?;
            writer.write_i32::<LittleEndian>(equipment.len() as i32)?;
            for item in equipment {
                if let Some(ref item) = item {
                    writer.write_u8(1)?;
                    item.write_to(writer)?;
                } else {
                    writer.write_u8(0)?;
                }
            }
        } else {
            writer.write_u8(0)?;
        }

        // Write quest inventory
        if let Some(ref quest_inventory) = self.quest_inventory {
            writer.write_u8(1)?;
            writer.write_i32::<LittleEndian>(quest_inventory.len() as i32)?;
            for item in quest_inventory {
                if let Some(ref item) = item {
                    writer.write_u8(1)?;
                    item.write_to(writer)?;
                } else {
                    writer.write_u8(0)?;
                }
            }
        } else {
            writer.write_u8(0)?;
        }

        writer.write_u32::<LittleEndian>(self.gold)?;
        writer.write_u32::<LittleEndian>(self.credit)?;
        writer.write_u8(self.has_expanded_storage as u8)?;
        writer.write_i64::<LittleEndian>(self.expanded_storage_expiry_time)?;

        // Write magics
        writer.write_i32::<LittleEndian>(self.magics.len() as i32)?;
        for magic in &self.magics {
            magic.write_to(writer)?;
        }

        // Write intelligent creatures (empty for now)
        writer.write_i32::<LittleEndian>(0)?;

        writer.write_u8(self.summoned_creature_type)?;
        writer.write_u8(self.creature_summoned as u8)?;
        writer.write_u8(self.allow_observe as u8)?;
        writer.write_u8(self.observer as u8)?;

        Ok(())
    }
}

/// UserLocation packet - player location and direction
#[derive(Debug, Clone)]
pub struct UserLocation {
    pub location_x: i32,
    pub location_y: i32,
    pub direction: MirDirection,
}

impl Packet for UserLocation {
    const OPCODE: i16 = ServerPacketIds::UserLocation as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let location_x = reader.read_i32::<LittleEndian>()?;
        let location_y = reader.read_i32::<LittleEndian>()?;
        let direction = MirDirection::try_from(reader.read_u8()?)?;
        Ok(Self { location_x, location_y, direction })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.location_x)?;
        writer.write_i32::<LittleEndian>(self.location_y)?;
        writer.write_u8(self.direction as u8)?;
        Ok(())
    }
}

/// UserSlotsRefresh packet - refreshes inventory and equipment slots
#[derive(Debug, Clone)]
pub struct UserSlotsRefresh {
    pub inventory: Option<Vec<Option<UserItem>>>,
    pub equipment: Option<Vec<Option<UserItem>>>,
}

impl Packet for UserSlotsRefresh {
    const OPCODE: i16 = ServerPacketIds::UserSlotsRefresh as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let inventory = if reader.read_u8()? != 0 {
            let count = reader.read_i32::<LittleEndian>()? as usize;
            let mut items = Vec::with_capacity(count);
            for _ in 0..count {
                if reader.read_u8()? != 0 {
                    items.push(Some(UserItem::read_from(reader, i32::MAX, i32::MAX)?));
                } else {
                    items.push(None);
                }
            }
            Some(items)
        } else {
            None
        };

        let equipment = if reader.read_u8()? != 0 {
            let count = reader.read_i32::<LittleEndian>()? as usize;
            let mut items = Vec::with_capacity(count);
            for _ in 0..count {
                if reader.read_u8()? != 0 {
                    items.push(Some(UserItem::read_from(reader, i32::MAX, i32::MAX)?));
                } else {
                    items.push(None);
                }
            }
            Some(items)
        } else {
            None
        };

        Ok(Self { inventory, equipment })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        if let Some(ref inventory) = self.inventory {
            writer.write_u8(1)?;
            writer.write_i32::<LittleEndian>(inventory.len() as i32)?;
            for item in inventory {
                if let Some(ref item) = item {
                    writer.write_u8(1)?;
                    item.write_to(writer)?;
                } else {
                    writer.write_u8(0)?;
                }
            }
        } else {
            writer.write_u8(0)?;
        }

        if let Some(ref equipment) = self.equipment {
            writer.write_u8(1)?;
            writer.write_i32::<LittleEndian>(equipment.len() as i32)?;
            for item in equipment {
                if let Some(ref item) = item {
                    writer.write_u8(1)?;
                    item.write_to(writer)?;
                } else {
                    writer.write_u8(0)?;
                }
            }
        } else {
            writer.write_u8(0)?;
        }

        Ok(())
    }
}
