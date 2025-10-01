use std::convert::TryFrom;
use std::io::Read;

use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};

use crate::binary::{read_bool, read_dotnet_string};
use crate::enums::{IntelligentCreaturePickupMode, IntelligentCreatureType, ItemGrade, Spell};
use crate::stats::{SharedError, SharedResult};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientMagic {
    pub name: String,
    pub spell: Spell,
    pub base_cost: u8,
    pub level_cost: u8,
    pub icon: u8,
    pub level1: u8,
    pub level2: u8,
    pub level3: u8,
    pub need1: u16,
    pub need2: u16,
    pub need3: u16,
    pub level: u8,
    pub key: u8,
    pub experience: u16,
    pub delay: i64,
    pub range: u8,
    pub cast_time: i64,
}

impl ClientMagic {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let name = read_dotnet_string(reader)?;
        let spell_value = reader.read_u8()?;
        let spell = Spell::try_from(spell_value)
            .map_err(|_| SharedError::unknown_enum("Spell", spell_value.into()))?;
        let base_cost = reader.read_u8()?;
        let level_cost = reader.read_u8()?;
        let icon = reader.read_u8()?;
        let level1 = reader.read_u8()?;
        let level2 = reader.read_u8()?;
        let level3 = reader.read_u8()?;
        let need1 = reader.read_u16::<LittleEndian>()?;
        let need2 = reader.read_u16::<LittleEndian>()?;
        let need3 = reader.read_u16::<LittleEndian>()?;
        let level = reader.read_u8()?;
        let key = reader.read_u8()?;
        let experience = reader.read_u16::<LittleEndian>()?;
        let delay = reader.read_i64::<LittleEndian>()?;
        let range = reader.read_u8()?;
        let cast_time = reader.read_i64::<LittleEndian>()?;

        Ok(Self {
            name,
            spell,
            base_cost,
            level_cost,
            icon,
            level1,
            level2,
            level3,
            need1,
            need2,
            need3,
            level,
            key,
            experience,
            delay,
            range,
            cast_time,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntelligentCreatureRules {
    pub minimal_fullness: i32,
    pub mouse_pickup_enabled: bool,
    pub mouse_pickup_range: i32,
    pub auto_pickup_enabled: bool,
    pub auto_pickup_range: i32,
    pub semi_auto_pickup_enabled: bool,
    pub semi_auto_pickup_range: i32,
    pub can_produce_black_stone: bool,
}

impl IntelligentCreatureRules {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(Self {
            minimal_fullness: reader.read_i32::<LittleEndian>()?,
            mouse_pickup_enabled: read_bool(reader)?,
            mouse_pickup_range: reader.read_i32::<LittleEndian>()?,
            auto_pickup_enabled: read_bool(reader)?,
            auto_pickup_range: reader.read_i32::<LittleEndian>()?,
            semi_auto_pickup_enabled: read_bool(reader)?,
            semi_auto_pickup_range: reader.read_i32::<LittleEndian>()?,
            can_produce_black_stone: read_bool(reader)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntelligentCreatureItemFilter {
    pub pet_pickup_all: bool,
    pub pet_pickup_gold: bool,
    pub pet_pickup_weapons: bool,
    pub pet_pickup_armours: bool,
    pub pet_pickup_helmets: bool,
    pub pet_pickup_boots: bool,
    pub pet_pickup_belts: bool,
    pub pet_pickup_accessories: bool,
    pub pet_pickup_others: bool,
    pub pickup_grade: ItemGrade,
}

impl IntelligentCreatureItemFilter {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(Self {
            pet_pickup_all: read_bool(reader)?,
            pet_pickup_gold: read_bool(reader)?,
            pet_pickup_weapons: read_bool(reader)?,
            pet_pickup_armours: read_bool(reader)?,
            pet_pickup_helmets: read_bool(reader)?,
            pet_pickup_boots: read_bool(reader)?,
            pet_pickup_belts: read_bool(reader)?,
            pet_pickup_accessories: read_bool(reader)?,
            pet_pickup_others: read_bool(reader)?,
            pickup_grade: ItemGrade::None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientIntelligentCreature {
    pub pet_type: IntelligentCreatureType,
    pub icon: i32,
    pub custom_name: String,
    pub fullness: i32,
    pub slot_index: i32,
    pub expire_binary: i64,
    pub blackstone_time: i64,
    pub maintain_food_time: i64,
    pub pet_mode: IntelligentCreaturePickupMode,
    pub creature_rules: IntelligentCreatureRules,
    pub filter: IntelligentCreatureItemFilter,
}

impl ClientIntelligentCreature {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let pet_type_value = reader.read_u8()?;
        let pet_type = IntelligentCreatureType::try_from(pet_type_value).map_err(|_| {
            SharedError::unknown_enum("IntelligentCreatureType", pet_type_value.into())
        })?;
        let icon = reader.read_i32::<LittleEndian>()?;
        let custom_name = read_dotnet_string(reader)?;
        let fullness = reader.read_i32::<LittleEndian>()?;
        let slot_index = reader.read_i32::<LittleEndian>()?;
        let expire_binary = reader.read_i64::<LittleEndian>()?;
        let blackstone_time = reader.read_i64::<LittleEndian>()?;
        let pet_mode_raw = reader.read_u8()?;
        let pet_mode = IntelligentCreaturePickupMode::try_from(pet_mode_raw).map_err(|_| {
            SharedError::unknown_enum("IntelligentCreaturePickupMode", pet_mode_raw.into())
        })?;
        let creature_rules = IntelligentCreatureRules::read_from(reader)?;
        let mut filter = IntelligentCreatureItemFilter::read_from(reader)?;
        let grade_raw = reader.read_u8()?;
        filter.pickup_grade = ItemGrade::try_from(grade_raw)
            .map_err(|_| SharedError::unknown_enum("ItemGrade", grade_raw.into()))?;
        let maintain_food_time = reader.read_i64::<LittleEndian>()?;

        Ok(Self {
            pet_type,
            icon,
            custom_name,
            fullness,
            slot_index,
            expire_binary,
            blackstone_time,
            maintain_food_time,
            pet_mode,
            creature_rules,
            filter,
        })
    }
}
