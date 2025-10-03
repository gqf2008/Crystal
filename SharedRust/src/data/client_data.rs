use std::convert::TryFrom;
use std::io::{Read, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use crate::binary::{read_bool, read_dotnet_string, write_bool, write_dotnet_string};
use crate::enums::{IntelligentCreaturePickupMode, IntelligentCreatureType, ItemGrade, MirClass, MirGender, Spell};
use crate::data::stats::{SharedError, SharedResult};

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

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.id)?;
        
        writer.write_i32::<LittleEndian>(self.task_list.len() as i32)?;
        for task in &self.task_list {
            write_dotnet_string(writer, task)?;
        }
        
        writer.write_u8(if self.taken { 1 } else { 0 })?;
        writer.write_u8(if self.completed { 1 } else { 0 })?;
        writer.write_u8(if self.new { 1 } else { 0 })?;
        
        Ok(())
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
    pub rewards_fixed_item: Vec<crate::data::shared_data::QuestItemReward>,
    pub rewards_select_item: Vec<crate::data::shared_data::QuestItemReward>,
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
            rewards_fixed_item.push(crate::data::shared_data::QuestItemReward::read_from(reader)?);
        }

        let count = reader.read_i32::<LittleEndian>()?;
        let mut rewards_select_item = Vec::with_capacity(count as usize);
        for _ in 0..count {
            rewards_select_item.push(crate::data::shared_data::QuestItemReward::read_from(reader)?);
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

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.index)?;
        writer.write_u32::<LittleEndian>(self.npc_index)?;
        write_dotnet_string(writer, &self.name)?;
        write_dotnet_string(writer, &self.group)?;

        writer.write_i32::<LittleEndian>(self.description.len() as i32)?;
        for desc in &self.description {
            write_dotnet_string(writer, desc)?;
        }

        writer.write_i32::<LittleEndian>(self.task_description.len() as i32)?;
        for desc in &self.task_description {
            write_dotnet_string(writer, desc)?;
        }

        writer.write_i32::<LittleEndian>(self.return_description.len() as i32)?;
        for desc in &self.return_description {
            write_dotnet_string(writer, desc)?;
        }

        writer.write_i32::<LittleEndian>(self.completion_description.len() as i32)?;
        for desc in &self.completion_description {
            write_dotnet_string(writer, desc)?;
        }

        writer.write_i32::<LittleEndian>(self.min_level_needed)?;
        writer.write_i32::<LittleEndian>(self.max_level_needed)?;
        writer.write_i32::<LittleEndian>(self.quest_needed)?;
        writer.write_u8(self.class_needed.bits())?;
        writer.write_u8(self.quest_type as u8)?;
        writer.write_i32::<LittleEndian>(self.time_limit_in_seconds)?;
        writer.write_u32::<LittleEndian>(self.reward_gold)?;
        writer.write_u32::<LittleEndian>(self.reward_exp)?;
        writer.write_u32::<LittleEndian>(self.reward_credit)?;

        writer.write_i32::<LittleEndian>(self.rewards_fixed_item.len() as i32)?;
        for reward in &self.rewards_fixed_item {
            reward.write_to(writer)?;
        }

        writer.write_i32::<LittleEndian>(self.rewards_select_item.len() as i32)?;
        for reward in &self.rewards_select_item {
            reward.write_to(writer)?;
        }

        writer.write_u32::<LittleEndian>(self.finish_npc_index)?;

        Ok(())
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

/// Client movement information (teleport points)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientMovementInfo {
    pub destination: i32,
    pub title: String,
    pub location: crate::map::Point,
    pub icon: i32,
}

impl ClientMovementInfo {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let destination = reader.read_i32::<LittleEndian>()?;
        let title = read_dotnet_string(reader)?;
        let x = reader.read_i32::<LittleEndian>()?;
        let y = reader.read_i32::<LittleEndian>()?;
        let location = crate::map::Point { x, y };
        let icon = reader.read_i32::<LittleEndian>()?;

        Ok(ClientMovementInfo {
            destination,
            title,
            location,
            icon,
        })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.destination)?;
        write_dotnet_string(writer, &self.title)?;
        writer.write_i32::<LittleEndian>(self.location.x)?;
        writer.write_i32::<LittleEndian>(self.location.y)?;
        writer.write_i32::<LittleEndian>(self.icon)?;
        Ok(())
    }
}

/// Client NPC information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientNPCInfo {
    pub object_id: u32,
    pub name: String,
    pub location: crate::map::Point,
    pub icon: i32,
    pub can_teleport_to: bool,
}

impl ClientNPCInfo {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let name = read_dotnet_string(reader)?;
        let x = reader.read_i32::<LittleEndian>()?;
        let y = reader.read_i32::<LittleEndian>()?;
        let location = crate::map::Point { x, y };
        let icon = reader.read_i32::<LittleEndian>()?;
        let can_teleport_to = read_bool(reader)?;

        Ok(ClientNPCInfo {
            object_id,
            name,
            location,
            icon,
            can_teleport_to,
        })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        write_dotnet_string(writer, &self.name)?;
        writer.write_i32::<LittleEndian>(self.location.x)?;
        writer.write_i32::<LittleEndian>(self.location.y)?;
        writer.write_i32::<LittleEndian>(self.icon)?;
        write_bool(writer, self.can_teleport_to)?;
        Ok(())
    }
}

/// Client map information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientMapInfo {
    pub width: i32,
    pub height: i32,
    pub big_map: i32,
    pub title: String,
    pub movements: Vec<ClientMovementInfo>,
    pub npcs: Vec<ClientNPCInfo>,
}

impl ClientMapInfo {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let title = read_dotnet_string(reader)?;
        let width = reader.read_i32::<LittleEndian>()?;
        let height = reader.read_i32::<LittleEndian>()?;
        let big_map = reader.read_i32::<LittleEndian>()?;
        
        let movement_count = reader.read_i32::<LittleEndian>()?;
        let mut movements = Vec::with_capacity(movement_count as usize);
        for _ in 0..movement_count {
            movements.push(ClientMovementInfo::read_from(reader)?);
        }
        
        let npc_count = reader.read_i32::<LittleEndian>()?;
        let mut npcs = Vec::with_capacity(npc_count as usize);
        for _ in 0..npc_count {
            npcs.push(ClientNPCInfo::read_from(reader)?);
        }

        Ok(ClientMapInfo {
            width,
            height,
            big_map,
            title,
            movements,
            npcs,
        })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.title)?;
        writer.write_i32::<LittleEndian>(self.width)?;
        writer.write_i32::<LittleEndian>(self.height)?;
        writer.write_i32::<LittleEndian>(self.big_map)?;
        
        writer.write_i32::<LittleEndian>(self.movements.len() as i32)?;
        for movement in &self.movements {
            movement.write_to(writer)?;
        }
        
        writer.write_i32::<LittleEndian>(self.npcs.len() as i32)?;
        for npc in &self.npcs {
            npc.write_to(writer)?;
        }
        
        Ok(())
    }
}

/// Client buff information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientBuff {
    pub buff_type: crate::enums::BuffType,
    pub caster: Option<String>,
    pub visible: bool,
    pub object_id: u32,
    pub expire_time: i64,
    pub infinite: bool,
    pub stats: crate::data::stats::Stats,
    pub paused: bool,
    pub values: Vec<i32>,
}

impl ClientBuff {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let buff_type_value = reader.read_u8()?;
        let buff_type = crate::enums::BuffType::try_from(buff_type_value)
            .map_err(|_| SharedError::unknown_enum("BuffType", buff_type_value.into()))?;
        let visible = read_bool(reader)?;
        let object_id = reader.read_u32::<LittleEndian>()?;
        let expire_time = reader.read_i64::<LittleEndian>()?;
        let infinite = read_bool(reader)?;
        let paused = read_bool(reader)?;
        
        let stats = crate::data::stats::Stats::read_from(reader)?;
        
        let value_count = reader.read_i32::<LittleEndian>()?;
        let mut values = Vec::with_capacity(value_count as usize);
        for _ in 0..value_count {
            values.push(reader.read_i32::<LittleEndian>()?);
        }

        Ok(ClientBuff {
            buff_type,
            caster: None, // Not serialized
            visible,
            object_id,
            expire_time,
            infinite,
            stats,
            paused,
            values,
        })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.buff_type as u8)?;
        write_bool(writer, self.visible)?;
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_i64::<LittleEndian>(self.expire_time)?;
        write_bool(writer, self.infinite)?;
        write_bool(writer, self.paused)?;
        
        self.stats.write_to(writer)?;
        
        writer.write_i32::<LittleEndian>(self.values.len() as i32)?;
        for value in &self.values {
            writer.write_i32::<LittleEndian>(*value)?;
        }
        
        Ok(())
    }
}

/// Client recipe information (crafting system)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientRecipeInfo {
    pub gold: u32,
    pub chance: u8,
    pub item: crate::data::item::UserItem,
    pub tools: Vec<crate::data::item::UserItem>,
    pub ingredients: Vec<crate::data::item::UserItem>,
}

impl ClientRecipeInfo {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let gold = reader.read_u32::<LittleEndian>()?;
        let chance = reader.read_u8()?;
        let item = crate::data::item::UserItem::read_from(reader, i32::MAX, i32::MAX)?;
        
        let tool_count = reader.read_i32::<LittleEndian>()?;
        let mut tools = Vec::with_capacity(tool_count as usize);
        for _ in 0..tool_count {
            tools.push(crate::data::item::UserItem::read_from(reader, i32::MAX, i32::MAX)?);
        }
        
        let ingredient_count = reader.read_i32::<LittleEndian>()?;
        let mut ingredients = Vec::with_capacity(ingredient_count as usize);
        for _ in 0..ingredient_count {
            ingredients.push(crate::data::item::UserItem::read_from(reader, i32::MAX, i32::MAX)?);
        }

        Ok(Self {
            gold,
            chance,
            item,
            tools,
            ingredients,
        })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.gold)?;
        writer.write_u8(self.chance)?;
        self.item.write_to(writer)?;
        
        writer.write_i32::<LittleEndian>(self.tools.len() as i32)?;
        for tool in &self.tools {
            tool.write_to(writer)?;
        }
        
        writer.write_i32::<LittleEndian>(self.ingredients.len() as i32)?;
        for ingredient in &self.ingredients {
            ingredient.write_to(writer)?;
        }
        
        Ok(())
    }
}

/// Client friend information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientFriend {
    pub index: i32,
    pub name: String,
    pub memo: String,
    pub blocked: bool,
    pub online: bool,
}

impl ClientFriend {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let index = reader.read_i32::<LittleEndian>()?;
        let name = read_dotnet_string(reader)?;
        let memo = read_dotnet_string(reader)?;
        let blocked = read_bool(reader)?;
        let online = read_bool(reader)?;

        Ok(Self {
            index,
            name,
            memo,
            blocked,
            online,
        })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.index)?;
        write_dotnet_string(writer, &self.name)?;
        write_dotnet_string(writer, &self.memo)?;
        write_bool(writer, self.blocked)?;
        write_bool(writer, self.online)?;
        Ok(())
    }
}

/// Client mail information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientMail {
    pub mail_id: u64,
    pub sender_name: String,
    pub message: String,
    pub opened: bool,
    pub locked: bool,
    pub can_reply: bool,
    pub collected: bool,
    pub date_sent: DateTime<Utc>,
    pub gold: u32,
    pub items: Vec<crate::data::item::UserItem>,
}

impl ClientMail {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let mail_id = reader.read_u64::<LittleEndian>()?;
        let sender_name = read_dotnet_string(reader)?;
        let message = read_dotnet_string(reader)?;
        let opened = read_bool(reader)?;
        let locked = read_bool(reader)?;
        let can_reply = read_bool(reader)?;
        let collected = read_bool(reader)?;
        
        // Read .NET DateTime
        let ticks = reader.read_i64::<LittleEndian>()?;
        let unix_epoch_ticks = 621355968000000000i64;
        let unix_seconds = (ticks - unix_epoch_ticks) / 10000000;
        let date_sent = Utc.timestamp_opt(unix_seconds, 0)
            .single()
            .ok_or(SharedError::InvalidDateTime)?;
        
        let gold = reader.read_u32::<LittleEndian>()?;
        
        let item_count = reader.read_i32::<LittleEndian>()?;
        let mut items = Vec::with_capacity(item_count as usize);
        for _ in 0..item_count {
            items.push(crate::data::item::UserItem::read_from(reader, i32::MAX, i32::MAX)?);
        }

        Ok(Self {
            mail_id,
            sender_name,
            message,
            opened,
            locked,
            can_reply,
            collected,
            date_sent,
            gold,
            items,
        })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.mail_id)?;
        write_dotnet_string(writer, &self.sender_name)?;
        write_dotnet_string(writer, &self.message)?;
        write_bool(writer, self.opened)?;
        write_bool(writer, self.locked)?;
        write_bool(writer, self.can_reply)?;
        write_bool(writer, self.collected)?;
        
        // Write .NET DateTime
        let unix_epoch_ticks = 621355968000000000i64;
        let ticks = self.date_sent.timestamp() * 10000000 + unix_epoch_ticks;
        writer.write_i64::<LittleEndian>(ticks)?;
        
        writer.write_u32::<LittleEndian>(self.gold)?;
        
        writer.write_i32::<LittleEndian>(self.items.len() as i32)?;
        for item in &self.items {
            item.write_to(writer)?;
        }
        
        Ok(())
    }
}

/// Client auction information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientAuction {
    pub auction_id: u64,
    pub item: crate::data::item::UserItem,
    pub seller: String,
    pub price: u32,
    pub consignment_date: DateTime<Utc>,
    pub item_type: crate::enums::MarketItemType,
}

impl ClientAuction {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let auction_id = reader.read_u64::<LittleEndian>()?;
        let item = crate::data::item::UserItem::read_from(reader, i32::MAX, i32::MAX)?;
        let seller = read_dotnet_string(reader)?;
        let price = reader.read_u32::<LittleEndian>()?;
        
        // Read .NET DateTime
        let ticks = reader.read_i64::<LittleEndian>()?;
        let unix_epoch_ticks = 621355968000000000i64;
        let unix_seconds = (ticks - unix_epoch_ticks) / 10000000;
        let consignment_date = Utc.timestamp_opt(unix_seconds, 0)
            .single()
            .ok_or(SharedError::InvalidDateTime)?;
        
        let item_type_value = reader.read_u8()?;
        let item_type = crate::enums::MarketItemType::try_from(item_type_value)
            .map_err(|_| SharedError::unknown_enum("MarketItemType", item_type_value.into()))?;

        Ok(Self {
            auction_id,
            item,
            seller,
            price,
            consignment_date,
            item_type,
        })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.auction_id)?;
        self.item.write_to(writer)?;
        write_dotnet_string(writer, &self.seller)?;
        writer.write_u32::<LittleEndian>(self.price)?;
        
        // Write .NET DateTime
        let unix_epoch_ticks = 621355968000000000i64;
        let ticks = self.consignment_date.timestamp() * 10000000 + unix_epoch_ticks;
        writer.write_i64::<LittleEndian>(ticks)?;
        
        writer.write_u8(self.item_type as u8)?;
        
        Ok(())
    }
}

/// Guild storage item
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuildStorageItem {
    pub item: crate::data::item::UserItem,
    pub user_id: i64,
}

impl GuildStorageItem {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let item = crate::data::item::UserItem::read_from(reader, i32::MAX, i32::MAX)?;
        let user_id = reader.read_i64::<LittleEndian>()?;

        Ok(Self { item, user_id })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        self.item.write_to(writer)?;
        writer.write_i64::<LittleEndian>(self.user_id)?;
        Ok(())
    }
}


