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
    // ---- #208 角色面板属性（服务端最终值 = 基础 + 装备加成）----
    pub max_hp: i32,
    pub max_mp: i32,
    /// [min, max] AC / MAC / DC / MC / SC
    pub ac: [i32; 2],
    pub mac: [i32; 2],
    pub dc: [i32; 2],
    pub mc: [i32; 2],
    pub sc: [i32; 2],
    pub critical_rate: i32,
    pub critical_damage: i32,
    pub attack_speed: i32,
    pub accuracy: i32,
    pub agility: i32,
    pub luck: i32,
    // ---- #210 角色面板 State 页 ----
    pub bag_weight: i32,
    pub wear_weight: i32,
    pub hand_weight: i32,
    pub magic_resist: i32,
    pub poison_resist: i32,
    pub health_recovery: i32,
    pub spell_recovery: i32,
    pub poison_recovery: i32,
    pub holy: i32,
    pub freezing: i32,
    pub poison_atk: i32,
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
                    items.push(Some(UserItem::read_from_with_info(reader)?));
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
                    items.push(Some(UserItem::read_from_with_info(reader)?));
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
                    items.push(Some(UserItem::read_from_with_info(reader)?));
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

        // #208：角色属性段（18 x i32）
        let max_hp = reader.read_i32::<LittleEndian>()?;
        let max_mp = reader.read_i32::<LittleEndian>()?;
        let ac = [reader.read_i32::<LittleEndian>()?, reader.read_i32::<LittleEndian>()?];
        let mac = [reader.read_i32::<LittleEndian>()?, reader.read_i32::<LittleEndian>()?];
        let dc = [reader.read_i32::<LittleEndian>()?, reader.read_i32::<LittleEndian>()?];
        let mc = [reader.read_i32::<LittleEndian>()?, reader.read_i32::<LittleEndian>()?];
        let sc = [reader.read_i32::<LittleEndian>()?, reader.read_i32::<LittleEndian>()?];
        let critical_rate = reader.read_i32::<LittleEndian>()?;
        let critical_damage = reader.read_i32::<LittleEndian>()?;
        let attack_speed = reader.read_i32::<LittleEndian>()?;
        let accuracy = reader.read_i32::<LittleEndian>()?;
        let agility = reader.read_i32::<LittleEndian>()?;
        let luck = reader.read_i32::<LittleEndian>()?;

        // #210：State 页段（11 x i32）
        let bag_weight = reader.read_i32::<LittleEndian>()?;
        let wear_weight = reader.read_i32::<LittleEndian>()?;
        let hand_weight = reader.read_i32::<LittleEndian>()?;
        let magic_resist = reader.read_i32::<LittleEndian>()?;
        let poison_resist = reader.read_i32::<LittleEndian>()?;
        let health_recovery = reader.read_i32::<LittleEndian>()?;
        let spell_recovery = reader.read_i32::<LittleEndian>()?;
        let poison_recovery = reader.read_i32::<LittleEndian>()?;
        let holy = reader.read_i32::<LittleEndian>()?;
        let freezing = reader.read_i32::<LittleEndian>()?;
        let poison_atk = reader.read_i32::<LittleEndian>()?;

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
            max_hp,
            max_mp,
            ac,
            mac,
            dc,
            mc,
            sc,
            critical_rate,
            critical_damage,
            attack_speed,
            accuracy,
            agility,
            luck,
            bag_weight,
            wear_weight,
            hand_weight,
            magic_resist,
            poison_resist,
            health_recovery,
            spell_recovery,
            poison_recovery,
            holy,
            freezing,
            poison_atk,
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

        // Write inventory（注意：read_body 用 read_from_with_info 读 ItemInfo，必须 write_to_with_info 对称）
        if let Some(ref inventory) = self.inventory {
            writer.write_u8(1)?;
            writer.write_i32::<LittleEndian>(inventory.len() as i32)?;
            for item in inventory {
                if let Some(ref item) = item {
                    writer.write_u8(1)?;
                    item.write_to_with_info(writer)?;
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
                    item.write_to_with_info(writer)?;
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
                    item.write_to_with_info(writer)?;
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

        // #208：角色属性段
        writer.write_i32::<LittleEndian>(self.max_hp)?;
        writer.write_i32::<LittleEndian>(self.max_mp)?;
        for v in self.ac {
            writer.write_i32::<LittleEndian>(v)?;
        }
        for v in self.mac {
            writer.write_i32::<LittleEndian>(v)?;
        }
        for v in self.dc {
            writer.write_i32::<LittleEndian>(v)?;
        }
        for v in self.mc {
            writer.write_i32::<LittleEndian>(v)?;
        }
        for v in self.sc {
            writer.write_i32::<LittleEndian>(v)?;
        }
        writer.write_i32::<LittleEndian>(self.critical_rate)?;
        writer.write_i32::<LittleEndian>(self.critical_damage)?;
        writer.write_i32::<LittleEndian>(self.attack_speed)?;
        writer.write_i32::<LittleEndian>(self.accuracy)?;
        writer.write_i32::<LittleEndian>(self.agility)?;
        writer.write_i32::<LittleEndian>(self.luck)?;

        // #210：State 页段
        writer.write_i32::<LittleEndian>(self.bag_weight)?;
        writer.write_i32::<LittleEndian>(self.wear_weight)?;
        writer.write_i32::<LittleEndian>(self.hand_weight)?;
        writer.write_i32::<LittleEndian>(self.magic_resist)?;
        writer.write_i32::<LittleEndian>(self.poison_resist)?;
        writer.write_i32::<LittleEndian>(self.health_recovery)?;
        writer.write_i32::<LittleEndian>(self.spell_recovery)?;
        writer.write_i32::<LittleEndian>(self.poison_recovery)?;
        writer.write_i32::<LittleEndian>(self.holy)?;
        writer.write_i32::<LittleEndian>(self.freezing)?;
        writer.write_i32::<LittleEndian>(self.poison_atk)?;

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
                    items.push(Some(UserItem::read_from_with_info(reader)?));
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
                    items.push(Some(UserItem::read_from_with_info(reader)?));
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
