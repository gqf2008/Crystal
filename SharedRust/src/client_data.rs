use std::convert::TryFrom;
use std::io::{Read, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use crate::binary::{read_bool, read_dotnet_string, write_dotnet_string};
use crate::enums::{IntelligentCreaturePickupMode, IntelligentCreatureType, ItemGrade, MirClass, MirGender, Spell};
use crate::stats::{SharedError, SharedResult};

/// Character selection information
/// Used in login/logout for character list
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectInfo {
    pub index: i32,
    pub name: String,
    pub level: u16,
    pub class: MirClass,
    pub gender: MirGender,
    pub last_access: DateTime<Utc>,
}

impl SelectInfo {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let index = reader.read_i32::<LittleEndian>()?;
        let name = read_dotnet_string(reader)?;
        let level = reader.read_u16::<LittleEndian>()?;
        let class_value = reader.read_u8()?;
        let class = MirClass::try_from(class_value)
            .map_err(|_| SharedError::unknown_enum("MirClass", class_value.into()))?;
        let gender_value = reader.read_u8()?;
        let gender = MirGender::try_from(gender_value)
            .map_err(|_| SharedError::unknown_enum("MirGender", gender_value.into()))?;
        
        // Read .NET DateTime ticks and convert to chrono DateTime
        let ticks = reader.read_i64::<LittleEndian>()?;
        let unix_epoch_ticks = 621355968000000000i64; // .NET ticks at Unix epoch
        let unix_seconds = (ticks - unix_epoch_ticks) / 10000000;
        let last_access = Utc.timestamp_opt(unix_seconds, 0)
            .single()
            .ok_or(SharedError::InvalidDateTime)?;

        Ok(SelectInfo {
            index,
            name,
            level,
            class,
            gender,
            last_access,
        })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.index)?;
        write_dotnet_string(writer, &self.name)?;
        writer.write_u16::<LittleEndian>(self.level)?;
        writer.write_u8(self.class as u8)?;
        writer.write_u8(self.gender as u8)?;
        
        // Convert chrono DateTime to .NET ticks
        let unix_epoch_ticks = 621355968000000000i64;
        let ticks = self.last_access.timestamp() * 10000000 + unix_epoch_ticks;
        writer.write_i64::<LittleEndian>(ticks)?;
        Ok(())
    }
}

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

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.name)?;
        writer.write_u8(self.spell as u8)?;
        writer.write_u8(self.base_cost)?;
        writer.write_u8(self.level_cost)?;
        writer.write_u8(self.icon)?;
        writer.write_u8(self.level1)?;
        writer.write_u8(self.level2)?;
        writer.write_u8(self.level3)?;
        writer.write_u16::<LittleEndian>(self.need1)?;
        writer.write_u16::<LittleEndian>(self.need2)?;
        writer.write_u16::<LittleEndian>(self.need3)?;
        writer.write_u8(self.level)?;
        writer.write_u8(self.key)?;
        writer.write_u16::<LittleEndian>(self.experience)?;
        writer.write_i64::<LittleEndian>(self.delay)?;
        writer.write_u8(self.range)?;
        writer.write_i64::<LittleEndian>(self.cast_time)?;
        Ok(())
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

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.minimal_fullness)?;
        write_bool(writer, self.mouse_pickup_enabled)?;
        writer.write_i32::<LittleEndian>(self.mouse_pickup_range)?;
        write_bool(writer, self.auto_pickup_enabled)?;
        writer.write_i32::<LittleEndian>(self.auto_pickup_range)?;
        write_bool(writer, self.semi_auto_pickup_enabled)?;
        writer.write_i32::<LittleEndian>(self.semi_auto_pickup_range)?;
        write_bool(writer, self.can_produce_black_stone)?;
        Ok(())
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

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_bool(writer, self.pet_pickup_all)?;
        write_bool(writer, self.pet_pickup_gold)?;
        write_bool(writer, self.pet_pickup_weapons)?;
        write_bool(writer, self.pet_pickup_armours)?;
        write_bool(writer, self.pet_pickup_helmets)?;
        write_bool(writer, self.pet_pickup_boots)?;
        write_bool(writer, self.pet_pickup_belts)?;
        write_bool(writer, self.pet_pickup_accessories)?;
        write_bool(writer, self.pet_pickup_others)?;
        Ok(())
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

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.pet_type as u8)?;
        writer.write_i32::<LittleEndian>(self.icon)?;
        write_dotnet_string(writer, &self.custom_name)?;
        writer.write_i32::<LittleEndian>(self.fullness)?;
        writer.write_i32::<LittleEndian>(self.slot_index)?;
        writer.write_i64::<LittleEndian>(self.expire_binary)?;
        writer.write_i64::<LittleEndian>(self.blackstone_time)?;
        writer.write_u8(self.pet_mode as u8)?;
        self.creature_rules.write_to(writer)?;
        self.filter.write_to(writer)?;
        writer.write_u8(self.filter.pickup_grade as u8)?;
        writer.write_i64::<LittleEndian>(self.maintain_food_time)?;
        Ok(())
    }
}

// Hero System
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientHeroInformation {
    pub index: i32,
    pub name: String,
    pub level: u16,
    pub class: crate::enums::MirClass,
    pub gender: crate::enums::MirGender,
}

impl ClientHeroInformation {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use crate::enums::{MirClass, MirGender};
        
        let index = reader.read_i32::<LittleEndian>()?;
        let name = read_dotnet_string(reader)?;
        let level = reader.read_u16::<LittleEndian>()?;
        let class_value = reader.read_u8()?;
        let class = MirClass::try_from(class_value)
            .map_err(|_| SharedError::unknown_enum("MirClass", class_value.into()))?;
        let gender_value = reader.read_u8()?;
        let gender = MirGender::try_from(gender_value)
            .map_err(|_| SharedError::unknown_enum("MirGender", gender_value.into()))?;

        Ok(Self {
            index,
            name,
            level,
            class,
            gender,
        })
    }
}

// Quest System
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientQuestProgress {
    pub id: i32,
    pub task_list: Vec<String>,
    pub taken: bool,
    pub completed: bool,
    pub new: bool,
}

impl ClientQuestProgress {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let id = reader.read_i32::<LittleEndian>()?;
        
        let count = reader.read_i32::<LittleEndian>()?;
        let mut task_list = Vec::with_capacity(count as usize);
        for _ in 0..count {
            task_list.push(read_dotnet_string(reader)?);
        }
        
        let taken = read_bool(reader)?;
        let completed = read_bool(reader)?;
        let new = read_bool(reader)?;

        Ok(Self {
            id,
            task_list,
            taken,
            completed,
            new,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestItemReward {
    pub item_index: i32,
    pub count: u32,
}

impl QuestItemReward {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let item_index = reader.read_i32::<LittleEndian>()?;
        let count = reader.read_u32::<LittleEndian>()?;
        
        Ok(Self {
            item_index,
            count,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientQuestInfo {
    pub index: i32,
    pub npc_index: u32,
    pub name: String,
    pub group: String,
    pub description: Vec<String>,
    pub task_description: Vec<String>,
    pub return_description: Vec<String>,
    pub completion_description: Vec<String>,
    pub min_level_needed: i32,
    pub max_level_needed: i32,
    pub quest_needed: i32,
    pub class_needed: crate::enums::RequiredClass,
    pub quest_type: crate::enums::QuestType,
    pub time_limit_in_seconds: i32,
    pub reward_gold: u32,
    pub reward_exp: u32,
    pub reward_credit: u32,
    pub rewards_fixed_item: Vec<QuestItemReward>,
    pub rewards_select_item: Vec<QuestItemReward>,
    pub finish_npc_index: u32,
}

impl ClientQuestInfo {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use crate::enums::{RequiredClass, QuestType};
        
        let index = reader.read_i32::<LittleEndian>()?;
        let npc_index = reader.read_u32::<LittleEndian>()?;
        let name = read_dotnet_string(reader)?;
        let group = read_dotnet_string(reader)?;

        let count = reader.read_i32::<LittleEndian>()?;
        let mut description = Vec::with_capacity(count as usize);
        for _ in 0..count {
            description.push(read_dotnet_string(reader)?);
        }

        let count = reader.read_i32::<LittleEndian>()?;
        let mut task_description = Vec::with_capacity(count as usize);
        for _ in 0..count {
            task_description.push(read_dotnet_string(reader)?);
        }

        let count = reader.read_i32::<LittleEndian>()?;
        let mut return_description = Vec::with_capacity(count as usize);
        for _ in 0..count {
            return_description.push(read_dotnet_string(reader)?);
        }

        let count = reader.read_i32::<LittleEndian>()?;
        let mut completion_description = Vec::with_capacity(count as usize);
        for _ in 0..count {
            completion_description.push(read_dotnet_string(reader)?);
        }

        let min_level_needed = reader.read_i32::<LittleEndian>()?;
        let max_level_needed = reader.read_i32::<LittleEndian>()?;
        let quest_needed = reader.read_i32::<LittleEndian>()?;
        
        let class_needed_value = reader.read_u8()?;
        let class_needed = RequiredClass::from_bits(class_needed_value)
            .ok_or_else(|| SharedError::UnknownEnum { name: "RequiredClass", value: class_needed_value.into() })?;
        
        let quest_type_value = reader.read_u8()?;
        let quest_type = QuestType::try_from(quest_type_value)
            .map_err(|_| SharedError::unknown_enum("QuestType", quest_type_value.into()))?;
        
        let time_limit_in_seconds = reader.read_i32::<LittleEndian>()?;
        let reward_gold = reader.read_u32::<LittleEndian>()?;
        let reward_exp = reader.read_u32::<LittleEndian>()?;
        let reward_credit = reader.read_u32::<LittleEndian>()?;

        let count = reader.read_i32::<LittleEndian>()?;
        let mut rewards_fixed_item = Vec::with_capacity(count as usize);
        for _ in 0..count {
            rewards_fixed_item.push(QuestItemReward::read_from(reader)?);
        }

        let count = reader.read_i32::<LittleEndian>()?;
        let mut rewards_select_item = Vec::with_capacity(count as usize);
        for _ in 0..count {
            rewards_select_item.push(QuestItemReward::read_from(reader)?);
        }

        let finish_npc_index = reader.read_u32::<LittleEndian>()?;

        Ok(Self {
            index,
            npc_index,
            name,
            group,
            description,
            task_description,
            return_description,
            completion_description,
            min_level_needed,
            max_level_needed,
            quest_needed,
            class_needed,
            quest_type,
            time_limit_in_seconds,
            reward_gold,
            reward_exp,
            reward_credit,
            rewards_fixed_item,
            rewards_select_item,
            finish_npc_index,
        })
    }
}

// Guild System
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuildMember {
    pub name: String,
    pub rank_name: String,
    pub rank_index: u8,
    pub online: bool,
    pub last_login: i64,
}

impl GuildMember {
    pub fn read_from<R: Read>(reader: &mut R, offline: bool) -> SharedResult<Self> {
        let name = read_dotnet_string(reader)?;
        let rank_name = read_dotnet_string(reader)?;
        let rank_index = reader.read_u8()?;
        
        let (online, last_login) = if !offline {
            (read_bool(reader)?, reader.read_i64::<LittleEndian>()?)
        } else {
            (false, 0)
        };

        Ok(Self {
            name,
            rank_name,
            rank_index,
            online,
            last_login,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuildRank {
    pub name: String,
    pub index: i32,
    pub options: crate::enums::GuildRankOptions,
    pub members: Vec<GuildMember>,
}

impl GuildRank {
    pub fn read_from<R: Read>(reader: &mut R, offline: bool) -> SharedResult<Self> {
        use crate::enums::GuildRankOptions;
        
        let name = read_dotnet_string(reader)?;
        
        let options_value = reader.read_u8()?;
        let options = GuildRankOptions::from_bits(options_value)
            .ok_or_else(|| SharedError::UnknownEnum { name: "GuildRankOptions", value: options_value.into() })?;

        let index = if !offline {
            reader.read_i32::<LittleEndian>()?
        } else {
            0
        };

        let member_count = reader.read_i32::<LittleEndian>()?;
        let mut members = Vec::with_capacity(member_count as usize);
        for _ in 0..member_count {
            members.push(GuildMember::read_from(reader, offline)?);
        }

        Ok(Self {
            name,
            index,
            options,
            members,
        })
    }
}
