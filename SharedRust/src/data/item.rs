use std::convert::TryFrom;
use std::io::{Read, Write};
use std::sync::Mutex;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::binary::{read_bool, read_dotnet_string, write_bool, write_dotnet_string};
use crate::enums::{
    AwakeType, BindMode, ItemGrade, ItemSet, ItemType, MirGridType, RefinedValue, RequiredClass,
    RequiredGender, RequiredType, SpecialItemMode, Stat,
};
use crate::data::stats::{SharedError, SharedResult, Stats};

const FISHING_ROD_SHAPES: [i16; 2] = [49, 50];

pub static AWAKE_MATERIALS: Lazy<Mutex<Vec<Vec<Vec<u8>>>>> = Lazy::new(|| Mutex::new(Vec::new()));

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemInfo {
    pub index: i32,
    pub name: String,
    pub item_type: ItemType,
    pub grade: ItemGrade,
    pub required_type: RequiredType,
    pub required_class: RequiredClass,
    pub required_gender: RequiredGender,
    pub set: ItemSet,
    pub shape: i16,
    pub weight: u8,
    pub light: u8,
    pub required_amount: u8,
    pub image: u16,
    pub durability: u16,
    pub price: u32,
    pub stack_size: u16,
    pub start_item: bool,
    pub effect: u8,
    pub need_identify: bool,
    pub show_group_pickup: bool,
    pub global_drop_notify: bool,
    pub class_based: bool,
    pub level_based: bool,
    pub can_mine: bool,
    pub can_fast_run: bool,
    pub can_awakening: bool,
    pub bind: BindMode,
    pub unique: SpecialItemMode,
    pub random_stats_id: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub random_stats: Option<RandomItemStat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_tip: Option<String>,
    pub slots: u8,
    pub stats: Stats,
}

impl Default for ItemInfo {
    fn default() -> Self {
        Self {
            index: 0,
            name: String::new(),
            item_type: ItemType::Nothing,
            grade: ItemGrade::None,
            required_type: RequiredType::Level,
            required_class: RequiredClass::NONE,
            required_gender: RequiredGender::NONE,
            set: ItemSet::None,
            shape: 0,
            weight: 0,
            light: 0,
            required_amount: 0,
            image: 0,
            durability: 0,
            price: 0,
            stack_size: 1,
            start_item: false,
            effect: 0,
            need_identify: false,
            show_group_pickup: false,
            global_drop_notify: false,
            class_based: false,
            level_based: false,
            can_mine: false,
            can_fast_run: false,
            can_awakening: false,
            bind: BindMode::NONE,
            unique: SpecialItemMode::NONE,
            random_stats_id: 0,
            random_stats: None,
            tool_tip: None,
            slots: 0,
            stats: Stats::new(),
        }
    }
}

impl ItemInfo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read_from<R: Read>(
        reader: &mut R,
        version: i32,
        _custom_version: i32,
    ) -> SharedResult<Self> {
        let index = reader.read_i32::<LittleEndian>()?;
        let name = read_dotnet_string(reader)?;
        let type_value = reader.read_u8()?;
        let item_type = ItemType::try_from(type_value)
            .map_err(|_| SharedError::unknown_enum("ItemType", type_value.into()))?;
        let grade_value = reader.read_u8()?;
        let grade = ItemGrade::try_from(grade_value)
            .map_err(|_| SharedError::unknown_enum("ItemGrade", grade_value.into()))?;
        let required_type_value = reader.read_u8()?;
        let required_type = RequiredType::try_from(required_type_value)
            .map_err(|_| SharedError::unknown_enum("RequiredType", required_type_value.into()))?;
        let required_class = RequiredClass::from_bits_truncate(reader.read_u8()?);
        let required_gender = RequiredGender::from_bits_truncate(reader.read_u8()?);
        let set_value = reader.read_u8()?;
        let set = ItemSet::try_from(set_value)
            .map_err(|_| SharedError::unknown_enum("ItemSet", set_value.into()))?;

        let shape = reader.read_i16::<LittleEndian>()?;
        let weight = reader.read_u8()?;
        let light = reader.read_u8()?;
        let required_amount = reader.read_u8()?;

        let image = reader.read_u16::<LittleEndian>()?;
        let durability = reader.read_u16::<LittleEndian>()?;

        let stack_size = if version <= 84 {
            reader.read_u32::<LittleEndian>()? as u16
        } else {
            reader.read_u16::<LittleEndian>()?
        };

        let price = reader.read_u32::<LittleEndian>()?;

        let mut stats = Stats::new();
        if version <= 84 {
            stats.set(Stat::MinAC, reader.read_u8()? as i32);
            stats.set(Stat::MaxAC, reader.read_u8()? as i32);
            stats.set(Stat::MinMAC, reader.read_u8()? as i32);
            stats.set(Stat::MaxMAC, reader.read_u8()? as i32);
            stats.set(Stat::MinDC, reader.read_u8()? as i32);
            stats.set(Stat::MaxDC, reader.read_u8()? as i32);
            stats.set(Stat::MinMC, reader.read_u8()? as i32);
            stats.set(Stat::MaxMC, reader.read_u8()? as i32);
            stats.set(Stat::MinSC, reader.read_u8()? as i32);
            stats.set(Stat::MaxSC, reader.read_u8()? as i32);
            stats.set(Stat::HP, reader.read_u16::<LittleEndian>()? as i32);
            stats.set(Stat::MP, reader.read_u16::<LittleEndian>()? as i32);
            stats.set(Stat::Accuracy, reader.read_u8()? as i32);
            stats.set(Stat::Agility, reader.read_u8()? as i32);
            stats.set(Stat::Luck, reader.read_i8()? as i32);
            stats.set(Stat::AttackSpeed, reader.read_i8()? as i32);
        }

        let start_item = read_bool(reader)?;

        if version <= 84 {
            stats.set(Stat::BagWeight, reader.read_u8()? as i32);
            stats.set(Stat::HandWeight, reader.read_u8()? as i32);
            stats.set(Stat::WearWeight, reader.read_u8()? as i32);
        }

        let effect = reader.read_u8()?;

        if version <= 84 {
            stats.set(Stat::Strong, reader.read_u8()? as i32);
            stats.set(Stat::MagicResist, reader.read_u8()? as i32);
            stats.set(Stat::PoisonResist, reader.read_u8()? as i32);
            stats.set(Stat::HealthRecovery, reader.read_u8()? as i32);
            stats.set(Stat::SpellRecovery, reader.read_u8()? as i32);
            stats.set(Stat::PoisonRecovery, reader.read_u8()? as i32);
            stats.set(Stat::HPRatePercent, reader.read_u8()? as i32);
            stats.set(Stat::MPRatePercent, reader.read_u8()? as i32);
            stats.set(Stat::CriticalRate, reader.read_u8()? as i32);
            stats.set(Stat::CriticalDamage, reader.read_u8()? as i32);
        }

        let bools = reader.read_u8()?;
        let need_identify = (bools & 0x01) != 0;
        let show_group_pickup = (bools & 0x02) != 0;
        let class_based = (bools & 0x04) != 0;
        let level_based = (bools & 0x08) != 0;
        let can_mine = (bools & 0x10) != 0;
        let global_drop_notify = if version >= 77 {
            (bools & 0x20) != 0
        } else {
            false
        };

        if version <= 84 {
            stats.set(Stat::MaxACRatePercent, reader.read_u8()? as i32);
            stats.set(Stat::MaxMACRatePercent, reader.read_u8()? as i32);
            stats.set(Stat::Holy, reader.read_u8()? as i32);
            stats.set(Stat::Freezing, reader.read_u8()? as i32);
            stats.set(Stat::PoisonAttack, reader.read_u8()? as i32);
        }

        let mut bind = BindMode::from_bits_truncate(reader.read_i16::<LittleEndian>()? as u16);

        if version <= 84 {
            stats.set(Stat::Reflect, reader.read_u8()? as i32);
            stats.set(Stat::HPDrainRatePercent, reader.read_u8()? as i32);
        }

        let unique = SpecialItemMode::from_bits_truncate(reader.read_i16::<LittleEndian>()? as u16);
        let random_stats_id = reader.read_u8()?;

        let can_fast_run = read_bool(reader)?;
        let can_awakening = read_bool(reader)?;

        let slots = if version > 83 { reader.read_u8()? } else { 0 };

        if version > 84 {
            stats = Stats::read_from(reader)?;
        }

        let tool_tip = if read_bool(reader)? {
            Some(read_dotnet_string(reader)?)
        } else {
            None
        };

        if version < 70 && matches!(item_type, ItemType::Ring) && !unique.is_empty() {
            bind.insert(BindMode::NO_WEDDING_RING);
        }

        Ok(Self {
            index,
            name,
            item_type,
            grade,
            required_type,
            required_class,
            required_gender,
            set,
            shape,
            weight,
            light,
            required_amount,
            image,
            durability,
            price,
            stack_size,
            start_item,
            effect,
            need_identify,
            show_group_pickup,
            global_drop_notify,
            class_based,
            level_based,
            can_mine,
            can_fast_run,
            can_awakening,
            bind,
            unique,
            random_stats_id,
            random_stats: None,
            tool_tip,
            slots,
            stats,
        })
    }

    pub fn read_default<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Self::read_from(reader, i32::MAX, i32::MAX)
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.index)?;
        write_dotnet_string(writer, &self.name)?;
        writer.write_u8(u8::from(self.item_type))?;
        writer.write_u8(u8::from(self.grade))?;
        writer.write_u8(u8::from(self.required_type))?;
        writer.write_u8(self.required_class.bits())?;
        writer.write_u8(self.required_gender.bits())?;
        writer.write_u8(u8::from(self.set))?;
        writer.write_i16::<LittleEndian>(self.shape)?;
        writer.write_u8(self.weight)?;
        writer.write_u8(self.light)?;
        writer.write_u8(self.required_amount)?;
        writer.write_u16::<LittleEndian>(self.image)?;
        writer.write_u16::<LittleEndian>(self.durability)?;
        writer.write_u16::<LittleEndian>(self.stack_size)?;
        writer.write_u32::<LittleEndian>(self.price)?;
        write_bool(writer, self.start_item)?;
        writer.write_u8(self.effect)?;

        let mut bools = 0u8;
        if self.need_identify {
            bools |= 0x01;
        }
        if self.show_group_pickup {
            bools |= 0x02;
        }
        if self.class_based {
            bools |= 0x04;
        }
        if self.level_based {
            bools |= 0x08;
        }
        if self.can_mine {
            bools |= 0x10;
        }
        if self.global_drop_notify {
            bools |= 0x20;
        }
        writer.write_u8(bools)?;

        writer.write_i16::<LittleEndian>(self.bind.bits() as i16)?;
        writer.write_i16::<LittleEndian>(self.unique.bits() as i16)?;
        writer.write_u8(self.random_stats_id)?;
        write_bool(writer, self.can_fast_run)?;
        write_bool(writer, self.can_awakening)?;
        writer.write_u8(self.slots)?;
        self.stats.write_to(writer)?;
        write_bool(writer, self.tool_tip.is_some())?;
        if let Some(t) = &self.tool_tip {
            write_dotnet_string(writer, t)?;
        }
        Ok(())
    }

    pub fn is_consumable(&self) -> bool {
        matches!(
            self.item_type,
            ItemType::Potion
                | ItemType::Scroll
                | ItemType::Food
                | ItemType::Transform
                | ItemType::Script
                | ItemType::SealedHero
        )
    }

    pub fn is_fishing_rod(&self) -> bool {
        FISHING_ROD_SHAPES.contains(&self.shape)
    }

    pub fn friendly_name(&self) -> String {
        let mut temp = self
            .name
            .trim_end_matches(|c: char| c.is_ascii_digit())
            .to_string();
        let mut result = String::with_capacity(temp.len());
        let mut depth = 0u32;
        for ch in temp.drain(..) {
            match ch {
                '[' => depth += 1,
                ']' => {
                    if depth > 0 {
                        depth -= 1;
                    }
                }
                _ => {
                    if depth == 0 {
                        result.push(ch);
                    }
                }
            }
        }
        result
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserItem {
    pub unique_id: u64,
    pub item_index: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<ItemInfo>,
    pub current_dura: u16,
    pub max_dura: u16,
    pub count: u16,
    pub gem_count: u16,
    pub refined_value: RefinedValue,
    pub refine_added: u8,
    pub refine_success_chance: i32,
    pub dura_changed: bool,
    pub soul_bound_id: i32,
    pub identified: bool,
    pub cursed: bool,
    pub wedding_ring: i32,
    pub buyback_expiry_date_binary: i64,
    pub slots: Vec<Option<UserItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_info: Option<ExpireInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rental_information: Option<RentalInformation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sealed_info: Option<SealedInfo>,
    pub is_shop_item: bool,
    pub awake: Awake,
    pub added_stats: Stats,
    pub is_gm_made: bool,
}

impl Default for UserItem {
    fn default() -> Self {
        Self {
            unique_id: 0,
            item_index: 0,
            info: None,
            current_dura: 0,
            max_dura: 0,
            count: 1,
            gem_count: 0,
            refined_value: RefinedValue::None,
            refine_added: 0,
            refine_success_chance: 0,
            dura_changed: false,
            soul_bound_id: -1,
            identified: false,
            cursed: false,
            wedding_ring: -1,
            buyback_expiry_date_binary: 0,
            slots: Vec::new(),
            expire_info: None,
            rental_information: None,
            sealed_info: None,
            is_shop_item: false,
            awake: Awake::default(),
            added_stats: Stats::new(),
            is_gm_made: false,
        }
    }
}

impl UserItem {
    pub fn new(item_index: i32) -> Self {
        Self {
            item_index,
            ..Self::default()
        }
    }

    pub fn with_info(info: ItemInfo) -> Self {
        let item_index = info.index;
        Self {
            item_index,
            info: Some(info),
            ..Self::default()
        }
    }

    pub fn set_info(&mut self, info: ItemInfo) {
        self.item_index = info.index;
        self.info = Some(info);
    }

    pub fn read_from<R: Read>(
        reader: &mut R,
        version: i32,
        custom_version: i32,
    ) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let item_index = reader.read_i32::<LittleEndian>()?;
        let current_dura = reader.read_u16::<LittleEndian>()?;
        let max_dura = reader.read_u16::<LittleEndian>()?;

        let count = if version <= 84 {
            reader.read_u32::<LittleEndian>()? as u16
        } else {
            reader.read_u16::<LittleEndian>()?
        };

        let mut added_stats = Stats::new();
        if version <= 84 {
            added_stats.set(Stat::MaxAC, reader.read_u8()? as i32);
            added_stats.set(Stat::MaxMAC, reader.read_u8()? as i32);
            added_stats.set(Stat::MaxDC, reader.read_u8()? as i32);
            added_stats.set(Stat::MaxMC, reader.read_u8()? as i32);
            added_stats.set(Stat::MaxSC, reader.read_u8()? as i32);
            added_stats.set(Stat::Accuracy, reader.read_u8()? as i32);
            added_stats.set(Stat::Agility, reader.read_u8()? as i32);
            added_stats.set(Stat::HP, reader.read_u8()? as i32);
            added_stats.set(Stat::MP, reader.read_u8()? as i32);
            added_stats.set(Stat::AttackSpeed, reader.read_i8()? as i32);
            added_stats.set(Stat::Luck, reader.read_i8()? as i32);
        }

        let soul_bound_id = reader.read_i32::<LittleEndian>()?;
        let bools = reader.read_u8()?;
        let identified = (bools & 0x01) != 0;
        let cursed = (bools & 0x02) != 0;

        if version <= 84 {
            added_stats.set(Stat::Strong, reader.read_u8()? as i32);
            added_stats.set(Stat::MagicResist, reader.read_u8()? as i32);
            added_stats.set(Stat::PoisonResist, reader.read_u8()? as i32);
            added_stats.set(Stat::HealthRecovery, reader.read_u8()? as i32);
            added_stats.set(Stat::SpellRecovery, reader.read_u8()? as i32);
            added_stats.set(Stat::PoisonRecovery, reader.read_u8()? as i32);
            added_stats.set(Stat::CriticalRate, reader.read_u8()? as i32);
            added_stats.set(Stat::CriticalDamage, reader.read_u8()? as i32);
            added_stats.set(Stat::Freezing, reader.read_u8()? as i32);
            added_stats.set(Stat::PoisonAttack, reader.read_u8()? as i32);
        }

        let slot_count = reader.read_i32::<LittleEndian>()?;
        let mut slots = Vec::with_capacity(slot_count as usize);
        for _ in 0..slot_count {
            let is_null = read_bool(reader)?;
            if is_null {
                slots.push(None);
            } else {
                let item = UserItem::read_from(reader, version, custom_version)?;
                slots.push(Some(item));
            }
        }

        let gem_count = if version <= 84 {
            reader.read_u32::<LittleEndian>()? as u16
        } else {
            reader.read_u16::<LittleEndian>()?
        };

        if version > 84 {
            added_stats = Stats::read_from(reader)?;
        }

        let awake = Awake::read_from(reader)?;

        let refined_value_raw = reader.read_u8()?;
        let refined_value = RefinedValue::try_from(refined_value_raw)
            .map_err(|_| SharedError::unknown_enum("RefinedValue", refined_value_raw.into()))?;
        let refine_added = reader.read_u8()?;
        let refine_success_chance = if version > 85 {
            reader.read_i32::<LittleEndian>()?
        } else {
            0
        };

        let wedding_ring = reader.read_i32::<LittleEndian>()?;

        let mut expire_info = None;
        let mut rental_information = None;
        let mut sealed_info = None;
        let mut is_shop_item = false;
        let mut is_gm_made = false;

        if version >= 65 {
            if read_bool(reader)? {
                expire_info = Some(ExpireInfo::read_from(reader, version, custom_version)?);
            }

            if version >= 76 {
                if read_bool(reader)? {
                    rental_information = Some(RentalInformation::read_from(
                        reader,
                        version,
                        custom_version,
                    )?);
                }

                if version >= 83 {
                    is_shop_item = read_bool(reader)?;

                    if version >= 92 {
                        if read_bool(reader)? {
                            sealed_info =
                                Some(SealedInfo::read_from(reader, version, custom_version)?);
                        }

                        if version > 107 {
                            is_gm_made = read_bool(reader)?;
                        }
                    }
                }
            }
        }

        Ok(Self {
            unique_id,
            item_index,
            info: None,
            current_dura,
            max_dura,
            count,
            gem_count,
            refined_value,
            refine_added,
            refine_success_chance,
            dura_changed: false,
            soul_bound_id,
            identified,
            cursed,
            wedding_ring,
            buyback_expiry_date_binary: 0,
            slots,
            expire_info,
            rental_information,
            sealed_info,
            is_shop_item,
            awake,
            added_stats,
            is_gm_made,
        })
    }

    pub fn read_default<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Self::read_from(reader, i32::MAX, i32::MAX)
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_i32::<LittleEndian>(self.item_index)?;
        writer.write_u16::<LittleEndian>(self.current_dura)?;
        writer.write_u16::<LittleEndian>(self.max_dura)?;
        writer.write_u16::<LittleEndian>(self.count)?;
        writer.write_i32::<LittleEndian>(self.soul_bound_id)?;
        let mut bools = 0u8;
        if self.identified {
            bools |= 0x01;
        }
        if self.cursed {
            bools |= 0x02;
        }
        writer.write_u8(bools)?;

        writer.write_i32::<LittleEndian>(self.slots.len() as i32)?;
        for slot in &self.slots {
            write_bool(writer, slot.is_none())?;
            if let Some(item) = slot {
                item.write_to(writer)?;
            }
        }

        writer.write_u16::<LittleEndian>(self.gem_count)?;
        self.added_stats.write_to(writer)?;
        self.awake.write_to(writer)?;
        writer.write_u8(u8::from(self.refined_value))?;
        writer.write_u8(self.refine_added)?;
        writer.write_i32::<LittleEndian>(self.refine_success_chance)?;
        writer.write_i32::<LittleEndian>(self.wedding_ring)?;

        write_bool(writer, self.expire_info.is_some())?;
        if let Some(info) = &self.expire_info {
            info.write_to(writer)?;
        }

        write_bool(writer, self.rental_information.is_some())?;
        if let Some(info) = &self.rental_information {
            info.write_to(writer)?;
        }

        write_bool(writer, self.is_shop_item)?;

        write_bool(writer, self.sealed_info.is_some())?;
        if let Some(info) = &self.sealed_info {
            info.write_to(writer)?;
        }

        write_bool(writer, self.is_gm_made)?;
        Ok(())
    }

    pub fn is_added(&self, info: Option<&ItemInfo>) -> bool {
        let base_info = info.or(self.info.as_ref());
        let slot_limit = base_info
            .map(|i| i.slots as usize)
            .unwrap_or(self.slots.len());
        self.added_stats.total_magnitude() > 0 || self.slots.len() > slot_limit
    }

    pub fn weight(&self, info: Option<&ItemInfo>) -> i32 {
        let base_info = info.or(self.info.as_ref());
        if let Some(info) = base_info {
            match info.item_type {
                ItemType::Amulet | ItemType::Bait => info.weight as i32,
                _ => info.weight as i32 * i32::from(self.count),
            }
        } else {
            0
        }
    }

    pub fn set_slot_size(&mut self, info: Option<&ItemInfo>, size: Option<usize>) {
        let info = info.or(self.info.as_ref());
        let mut target = size;
        if target.is_none() {
            if let Some(info) = info {
                match info.item_type {
                    ItemType::Mount => {
                        target = if info.shape < 7 {
                            Some(4)
                        } else if info.shape < 12 {
                            Some(5)
                        } else {
                            None
                        };
                    }
                    ItemType::Weapon => {
                        if info.shape == 49 || info.shape == 50 {
                            target = Some(5);
                        }
                    }
                    _ => {}
                }
            }
        }

        let desired = target
            .or_else(|| info.map(|i| usize::from(i.slots)))
            .unwrap_or_else(|| self.slots.len());

        if desired != self.slots.len() {
            self.slots.resize_with(desired, || None);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpireInfo {
    pub expiry_date_binary: i64,
}

impl ExpireInfo {
    pub fn new(expiry_date_binary: i64) -> Self {
        Self { expiry_date_binary }
    }

    pub fn read_from<R: Read>(
        reader: &mut R,
        _version: i32,
        _custom_version: i32,
    ) -> SharedResult<Self> {
        let expiry_date_binary = reader.read_i64::<LittleEndian>()?;
        Ok(Self { expiry_date_binary })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i64::<LittleEndian>(self.expiry_date_binary)?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedInfo {
    pub expiry_date_binary: i64,
    pub next_seal_date_binary: i64,
}

impl SealedInfo {
    pub fn read_from<R: Read>(
        reader: &mut R,
        version: i32,
        _custom_version: i32,
    ) -> SharedResult<Self> {
        let expiry_date_binary = reader.read_i64::<LittleEndian>()?;
        let next_seal_date_binary = if version > 92 {
            reader.read_i64::<LittleEndian>()?
        } else {
            0
        };
        Ok(Self {
            expiry_date_binary,
            next_seal_date_binary,
        })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i64::<LittleEndian>(self.expiry_date_binary)?;
        writer.write_i64::<LittleEndian>(self.next_seal_date_binary)?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RentalInformation {
    pub owner_name: String,
    pub binding_flags: BindMode,
    pub expiry_date_binary: i64,
    pub rental_locked: bool,
}

impl RentalInformation {
    pub fn read_from<R: Read>(
        reader: &mut R,
        _version: i32,
        _custom_version: i32,
    ) -> SharedResult<Self> {
        let owner_name = read_dotnet_string(reader)?;
        let binding_flags = BindMode::from_bits_truncate(reader.read_i16::<LittleEndian>()? as u16);
        let expiry_date_binary = reader.read_i64::<LittleEndian>()?;
        let rental_locked = read_bool(reader)?;
        Ok(Self {
            owner_name,
            binding_flags,
            expiry_date_binary,
            rental_locked,
        })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.owner_name)?;
        writer.write_i16::<LittleEndian>(self.binding_flags.bits() as i16)?;
        writer.write_i64::<LittleEndian>(self.expiry_date_binary)?;
        write_bool(writer, self.rental_locked)?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Awake {
    pub awake_type: AwakeType,
    pub levels: Vec<u8>,
}

impl Default for Awake {
    fn default() -> Self {
        Self {
            awake_type: AwakeType::None,
            levels: Vec::new(),
        }
    }
}

impl Awake {
    pub const SUCCESS_RATE: u8 = 70;
    pub const HIT_RATE: u8 = 70;
    pub const MAX_AWAKE_LEVEL: usize = 5;
    pub const WEAPON_RATE: u8 = 1;
    pub const HELMET_RATE: u8 = 1;
    pub const ARMOUR_RATE: u8 = 5;
    pub const CHANCE_MIN: u8 = 1;
    pub const MATERIAL_RATE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
    pub const CHANCE_MAX: [u8; 4] = [1, 2, 3, 4];

    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let type_value = reader.read_u8()?;
        let awake_type = AwakeType::try_from(type_value)
            .map_err(|_| SharedError::unknown_enum("AwakeType", type_value.into()))?;
        let count = reader.read_i32::<LittleEndian>()?;
        let mut levels = Vec::with_capacity(count as usize);
        for _ in 0..count {
            levels.push(reader.read_u8()?);
        }
        Ok(Self { awake_type, levels })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(u8::from(self.awake_type))?;
        writer.write_i32::<LittleEndian>(self.levels.len() as i32)?;
        for level in &self.levels {
            writer.write_u8(*level)?;
        }
        Ok(())
    }

    pub fn is_max_level(&self) -> bool {
        self.levels.len() >= Self::MAX_AWAKE_LEVEL
    }

    pub fn awake_level(&self) -> usize {
        self.levels.len()
    }

    pub fn awake_value(&self) -> u8 {
        self.levels.iter().copied().sum()
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RandomItemStat {
    pub max_dura_chance: u8,
    pub max_dura_stat_chance: u8,
    pub max_dura_max_stat: u8,
    pub max_ac_chance: u8,
    pub max_ac_stat_chance: u8,
    pub max_ac_max_stat: u8,
    pub max_mac_chance: u8,
    pub max_mac_stat_chance: u8,
    pub max_mac_max_stat: u8,
    pub max_dc_chance: u8,
    pub max_dc_stat_chance: u8,
    pub max_dc_max_stat: u8,
    pub max_mc_chance: u8,
    pub max_mc_stat_chance: u8,
    pub max_mc_max_stat: u8,
    pub max_sc_chance: u8,
    pub max_sc_stat_chance: u8,
    pub max_sc_max_stat: u8,
    pub accuracy_chance: u8,
    pub accuracy_stat_chance: u8,
    pub accuracy_max_stat: u8,
    pub agility_chance: u8,
    pub agility_stat_chance: u8,
    pub agility_max_stat: u8,
    pub hp_chance: u8,
    pub hp_stat_chance: u8,
    pub hp_max_stat: u8,
    pub mp_chance: u8,
    pub mp_stat_chance: u8,
    pub mp_max_stat: u8,
    pub strong_chance: u8,
    pub strong_stat_chance: u8,
    pub strong_max_stat: u8,
    pub magic_resist_chance: u8,
    pub magic_resist_stat_chance: u8,
    pub magic_resist_max_stat: u8,
    pub poison_resist_chance: u8,
    pub poison_resist_stat_chance: u8,
    pub poison_resist_max_stat: u8,
    pub hp_recovery_chance: u8,
    pub hp_recovery_stat_chance: u8,
    pub hp_recovery_max_stat: u8,
    pub mp_recovery_chance: u8,
    pub mp_recovery_stat_chance: u8,
    pub mp_recovery_max_stat: u8,
    pub poison_recovery_chance: u8,
    pub poison_recovery_stat_chance: u8,
    pub poison_recovery_max_stat: u8,
    pub critical_rate_chance: u8,
    pub critical_rate_stat_chance: u8,
    pub critical_rate_max_stat: u8,
    pub critical_damage_chance: u8,
    pub critical_damage_stat_chance: u8,
    pub critical_damage_max_stat: u8,
    pub freeze_chance: u8,
    pub freeze_stat_chance: u8,
    pub freeze_max_stat: u8,
    pub poison_attack_chance: u8,
    pub poison_attack_stat_chance: u8,
    pub poison_attack_max_stat: u8,
    pub attack_speed_chance: u8,
    pub attack_speed_stat_chance: u8,
    pub attack_speed_max_stat: u8,
    pub luck_chance: u8,
    pub luck_stat_chance: u8,
    pub luck_max_stat: u8,
    pub curse_chance: u8,
    pub slot_chance: u8,
    pub slot_stat_chance: u8,
    pub slot_max_stat: u8,
}

impl RandomItemStat {
    pub fn new(item_type: ItemType) -> Self {
        let mut stat = Self::default();
        match item_type {
            ItemType::Weapon => stat.set_weapon(),
            ItemType::Armour => stat.set_armour(),
            ItemType::Helmet => stat.set_helmet(),
            ItemType::Belt | ItemType::Boots => stat.set_belt_boots(),
            ItemType::Necklace => stat.set_necklace(),
            ItemType::Bracelet => stat.set_bracelet(),
            ItemType::Ring => stat.set_ring(),
            ItemType::Mount => stat.set_mount(),
            _ => {}
        }
        stat
    }

    pub fn set_weapon(&mut self) {
        self.max_dura_chance = 2;
        self.max_dura_stat_chance = 13;
        self.max_dura_max_stat = 13;

        self.max_dc_chance = 15;
        self.max_dc_stat_chance = 15;
        self.max_dc_max_stat = 13;

        self.max_mc_chance = 20;
        self.max_mc_stat_chance = 15;
        self.max_mc_max_stat = 13;

        self.max_sc_chance = 20;
        self.max_sc_stat_chance = 15;
        self.max_sc_max_stat = 13;

        self.attack_speed_chance = 60;
        self.attack_speed_stat_chance = 30;
        self.attack_speed_max_stat = 3;

        self.strong_chance = 24;
        self.strong_stat_chance = 20;
        self.strong_max_stat = 2;

        self.accuracy_chance = 30;
        self.accuracy_stat_chance = 20;
        self.accuracy_max_stat = 2;
    }

    pub fn set_armour(&mut self) {
        self.max_dura_chance = 2;
        self.max_dura_stat_chance = 10;
        self.max_dura_max_stat = 3;

        self.max_ac_chance = 30;
        self.max_ac_stat_chance = 15;
        self.max_ac_max_stat = 7;

        self.max_mac_chance = 30;
        self.max_mac_stat_chance = 15;
        self.max_mac_max_stat = 7;

        self.max_dc_chance = 40;
        self.max_dc_stat_chance = 20;
        self.max_dc_max_stat = 7;

        self.max_mc_chance = 40;
        self.max_mc_stat_chance = 20;
        self.max_mc_max_stat = 7;

        self.max_sc_chance = 40;
        self.max_sc_stat_chance = 20;
        self.max_sc_max_stat = 7;
    }

    pub fn set_helmet(&mut self) {
        self.max_dura_chance = 2;
        self.max_dura_stat_chance = 10;
        self.max_dura_max_stat = 3;

        self.max_ac_chance = 30;
        self.max_ac_stat_chance = 15;
        self.max_ac_max_stat = 7;

        self.max_mac_chance = 30;
        self.max_mac_stat_chance = 15;
        self.max_mac_max_stat = 7;

        self.max_dc_chance = 40;
        self.max_dc_stat_chance = 20;
        self.max_dc_max_stat = 7;

        self.max_mc_chance = 40;
        self.max_mc_stat_chance = 20;
        self.max_mc_max_stat = 7;

        self.max_sc_chance = 40;
        self.max_sc_stat_chance = 20;
        self.max_sc_max_stat = 7;
    }

    pub fn set_belt_boots(&mut self) {
        self.max_dura_chance = 2;
        self.max_dura_stat_chance = 10;
        self.max_dura_max_stat = 3;

        self.max_ac_chance = 30;
        self.max_ac_stat_chance = 30;
        self.max_ac_max_stat = 3;

        self.max_mac_chance = 30;
        self.max_mac_stat_chance = 30;
        self.max_mac_max_stat = 3;

        self.max_dc_chance = 30;
        self.max_dc_stat_chance = 30;
        self.max_dc_max_stat = 3;

        self.max_mc_chance = 30;
        self.max_mc_stat_chance = 30;
        self.max_mc_max_stat = 3;

        self.max_sc_chance = 30;
        self.max_sc_stat_chance = 30;
        self.max_sc_max_stat = 3;

        self.agility_chance = 60;
        self.agility_stat_chance = 30;
        self.agility_max_stat = 3;
    }

    pub fn set_necklace(&mut self) {
        self.max_dura_chance = 2;
        self.max_dura_stat_chance = 10;
        self.max_dura_max_stat = 3;

        self.max_dc_chance = 15;
        self.max_dc_stat_chance = 30;
        self.max_dc_max_stat = 7;

        self.max_mc_chance = 15;
        self.max_mc_stat_chance = 30;
        self.max_mc_max_stat = 7;

        self.max_sc_chance = 15;
        self.max_sc_stat_chance = 30;
        self.max_sc_max_stat = 7;

        self.accuracy_chance = 60;
        self.accuracy_stat_chance = 30;
        self.accuracy_max_stat = 7;

        self.agility_chance = 60;
        self.agility_stat_chance = 30;
        self.agility_max_stat = 7;
    }

    pub fn set_bracelet(&mut self) {
        self.max_dura_chance = 2;
        self.max_dura_stat_chance = 10;
        self.max_dura_max_stat = 3;

        self.max_ac_chance = 20;
        self.max_ac_stat_chance = 30;
        self.max_ac_max_stat = 6;

        self.max_mac_chance = 20;
        self.max_mac_stat_chance = 30;
        self.max_mac_max_stat = 6;

        self.max_dc_chance = 30;
        self.max_dc_stat_chance = 30;
        self.max_dc_max_stat = 6;

        self.max_mc_chance = 30;
        self.max_mc_stat_chance = 30;
        self.max_mc_max_stat = 6;

        self.max_sc_chance = 30;
        self.max_sc_stat_chance = 30;
        self.max_sc_max_stat = 6;
    }

    pub fn set_ring(&mut self) {
        self.max_dura_chance = 2;
        self.max_dura_stat_chance = 10;
        self.max_dura_max_stat = 3;

        self.max_ac_chance = 25;
        self.max_ac_stat_chance = 20;
        self.max_ac_max_stat = 6;

        self.max_mac_chance = 25;
        self.max_mac_stat_chance = 20;
        self.max_mac_max_stat = 6;

        self.max_dc_chance = 15;
        self.max_dc_stat_chance = 30;
        self.max_dc_max_stat = 6;

        self.max_mc_chance = 15;
        self.max_mc_stat_chance = 30;
        self.max_mc_max_stat = 6;

        self.max_sc_chance = 15;
        self.max_sc_stat_chance = 30;
        self.max_sc_max_stat = 6;
    }

    pub fn set_mount(&mut self) {
        self.set_ring();
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GameShopItem {
    pub item_index: i32,
    pub g_index: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<ItemInfo>,
    pub gold_price: u32,
    pub credit_price: u32,
    pub count: u16,
    pub class_name: String,
    pub category: String,
    pub stock: i32,
    pub in_stock: bool,
    pub deal: bool,
    pub top_item: bool,
    pub date_binary: i64,
    pub can_buy_gold: bool,
    pub can_buy_credit: bool,
}

impl GameShopItem {
    pub fn read_from<R: Read>(
        reader: &mut R,
        version: i32,
        _custom_version: i32,
    ) -> SharedResult<Self> {
        let item_index = reader.read_i32::<LittleEndian>()?;
        let g_index = reader.read_i32::<LittleEndian>()?;
        let gold_price = reader.read_u32::<LittleEndian>()?;
        let credit_price = reader.read_u32::<LittleEndian>()?;

        let count = if version <= 84 {
            reader.read_u32::<LittleEndian>()? as u16
        } else {
            reader.read_u16::<LittleEndian>()?
        };

        let class_name = read_dotnet_string(reader)?;
        let category = read_dotnet_string(reader)?;
        let stock = reader.read_i32::<LittleEndian>()?;
        let in_stock = read_bool(reader)?;
        let deal = read_bool(reader)?;
        let top_item = read_bool(reader)?;
        let date_binary = reader.read_i64::<LittleEndian>()?;

        let (can_buy_gold, can_buy_credit) = if version > 105 {
            let can_buy_gold = read_bool(reader)?;
            let can_buy_credit = read_bool(reader)?;
            (can_buy_gold, can_buy_credit)
        } else {
            (false, false)
        };

        Ok(Self {
            item_index,
            g_index,
            info: None,
            gold_price,
            credit_price,
            count,
            class_name,
            category,
            stock,
            in_stock,
            deal,
            top_item,
            date_binary,
            can_buy_gold,
            can_buy_credit,
        })
    }

    pub fn read_from_packet<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let item_index = reader.read_i32::<LittleEndian>()?;
        let g_index = reader.read_i32::<LittleEndian>()?;
        let info = ItemInfo::read_default(reader)?;
        let gold_price = reader.read_u32::<LittleEndian>()?;
        let credit_price = reader.read_u32::<LittleEndian>()?;
        let count = reader.read_u16::<LittleEndian>()?;
        let class_name = read_dotnet_string(reader)?;
        let category = read_dotnet_string(reader)?;
        let stock = reader.read_i32::<LittleEndian>()?;
        let in_stock = read_bool(reader)?;
        let deal = read_bool(reader)?;
        let top_item = read_bool(reader)?;
        let date_binary = reader.read_i64::<LittleEndian>()?;
        let can_buy_credit = read_bool(reader)?;
        let can_buy_gold = read_bool(reader)?;

        Ok(Self {
            item_index,
            g_index,
            info: Some(info),
            gold_price,
            credit_price,
            count,
            class_name,
            category,
            stock,
            in_stock,
            deal,
            top_item,
            date_binary,
            can_buy_gold,
            can_buy_credit,
        })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W, packet: bool) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.item_index)?;
        writer.write_i32::<LittleEndian>(self.g_index)?;

        if packet {
            let info = self
                .info
                .as_ref()
                .ok_or_else(|| SharedError::missing_field("info"))?;
            info.write_to(writer)?;
        }

        writer.write_u32::<LittleEndian>(self.gold_price)?;
        writer.write_u32::<LittleEndian>(self.credit_price)?;
        writer.write_u16::<LittleEndian>(self.count)?;
        write_dotnet_string(writer, &self.class_name)?;
        write_dotnet_string(writer, &self.category)?;
        writer.write_i32::<LittleEndian>(self.stock)?;
        write_bool(writer, self.in_stock)?;
        write_bool(writer, self.deal)?;
        write_bool(writer, self.top_item)?;
        writer.write_i64::<LittleEndian>(self.date_binary)?;
        write_bool(writer, self.can_buy_credit)?;
        write_bool(writer, self.can_buy_gold)?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatItem {
    pub unique_id: u64,
    pub title: String,
    pub grid: MirGridType,
}

impl ChatItem {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let title = read_dotnet_string(reader)?;
        let grid_value = reader.read_u8()?;
        let grid = MirGridType::try_from(grid_value)
            .map_err(|_| SharedError::unknown_enum("MirGridType", grid_value.into()))?;

        Ok(Self {
            unique_id,
            title,
            grid,
        })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        write_dotnet_string(writer, &self.title)?;
        writer.write_u8(u8::from(self.grid))?;
        Ok(())
    }

    pub fn regex_internal_name(&self) -> String {
        let escaped = self.title.replace('(', "\\(").replace(')', "\\)");
        format!("<{}>", escaped)
    }

    pub fn internal_name(&self) -> String {
        format!("<{}/{}>", self.title, self.unique_id)
    }
}

impl Default for ChatItem {
    fn default() -> Self {
        Self {
            unique_id: 0,
            title: String::new(),
            grid: MirGridType::None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemSets {
    pub set: ItemSet,
    pub types: Vec<ItemType>,
    pub count: u8,
}

impl ItemSets {
    pub fn required_amount(&self) -> u8 {
        match self.set {
            ItemSet::Mundane
            | ItemSet::NokChi
            | ItemSet::TaoProtect
            | ItemSet::Whisker1
            | ItemSet::Whisker2
            | ItemSet::Whisker3
            | ItemSet::Whisker4
            | ItemSet::Whisker5 => 2,
            ItemSet::RedOrchid
            | ItemSet::RedFlower
            | ItemSet::Smash
            | ItemSet::HwanDevil
            | ItemSet::Purity
            | ItemSet::FiveString
            | ItemSet::Bone
            | ItemSet::Bug
            | ItemSet::DarkGhost => 3,
            ItemSet::Recall => 4,
            ItemSet::Spirit
            | ItemSet::WhiteGold
            | ItemSet::WhiteGoldH
            | ItemSet::RedJade
            | ItemSet::RedJadeH
            | ItemSet::Nephrite
            | ItemSet::NephriteH
            | ItemSet::Hyeolryong
            | ItemSet::Monitor
            | ItemSet::Oppressive
            | ItemSet::Paeok
            | ItemSet::Sulgwan
            | ItemSet::BlueFrostH
            | ItemSet::BlueFrost => 5,
            _ => 0,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.count >= self.required_amount()
    }
}

impl Default for ItemSets {
    fn default() -> Self {
        Self {
            set: ItemSet::None,
            types: Vec::new(),
            count: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ItemRentalInformation {
    pub item_id: u64,
    pub item_name: String,
    pub renting_player_name: String,
    pub item_return_date_binary: i64,
}

impl ItemRentalInformation {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let item_id = reader.read_u64::<LittleEndian>()?;
        let item_name = read_dotnet_string(reader)?;
        let renting_player_name = read_dotnet_string(reader)?;
        let item_return_date_binary = reader.read_i64::<LittleEndian>()?;
        Ok(Self {
            item_id,
            item_name,
            renting_player_name,
            item_return_date_binary,
        })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.item_id)?;
        write_dotnet_string(writer, &self.item_name)?;
        write_dotnet_string(writer, &self.renting_player_name)?;
        writer.write_i64::<LittleEndian>(self.item_return_date_binary)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::{LittleEndian, WriteBytesExt};
    use std::io::Cursor;

    #[test]
    fn item_info_roundtrip_latest_version() -> SharedResult<()> {
        let mut item = ItemInfo::new();
        item.index = 123;
        item.name = "CrystalSword[+3]99".to_string();
        item.item_type = ItemType::Weapon;
        item.grade = ItemGrade::Legendary;
        item.required_class = RequiredClass::WAR_WIZ_TAO;
        item.required_gender = RequiredGender::MALE;
        item.set = ItemSet::Spirit;
        item.shape = 49;
        item.weight = 12;
        item.light = 3;
        item.required_amount = 1;
        item.image = 42;
        item.durability = 100;
        item.price = 123_456;
        item.stack_size = 5;
        item.start_item = true;
        item.effect = 2;
        item.need_identify = true;
        item.show_group_pickup = true;
        item.global_drop_notify = true;
        item.class_based = true;
        item.level_based = true;
        item.can_mine = true;
        item.can_fast_run = true;
        item.can_awakening = true;
        item.bind = BindMode::BIND_ON_EQUIP;
        item.unique = SpecialItemMode::PARALIZE;
        item.random_stats_id = 7;
        item.tool_tip = Some("Legendary weapon".to_string());
        item.slots = 2;
        item.stats.set(Stat::MaxDC, 15);
        item.stats.set(Stat::Luck, 2);

        let mut buffer = Vec::new();
        item.write_to(&mut buffer)?;
        let mut cursor = Cursor::new(buffer);
        let decoded = ItemInfo::read_from(&mut cursor, i32::MAX, i32::MAX)?;
        assert_eq!(decoded.index, item.index);
        assert_eq!(decoded.name, item.name);
        assert_eq!(decoded.item_type, item.item_type);
        assert_eq!(decoded.grade, item.grade);
        assert_eq!(decoded.required_class, item.required_class);
        assert_eq!(decoded.required_gender, item.required_gender);
        assert_eq!(decoded.set, item.set);
        assert_eq!(decoded.shape, item.shape);
        assert_eq!(decoded.weight, item.weight);
        assert_eq!(decoded.light, item.light);
        assert_eq!(decoded.required_amount, item.required_amount);
        assert_eq!(decoded.image, item.image);
        assert_eq!(decoded.durability, item.durability);
        assert_eq!(decoded.price, item.price);
        assert_eq!(decoded.stack_size, item.stack_size);
        assert_eq!(decoded.start_item, item.start_item);
        assert_eq!(decoded.tool_tip, item.tool_tip);
        assert_eq!(decoded.stats, item.stats);
        Ok(())
    }

    #[test]
    fn user_item_roundtrip_latest_version() -> SharedResult<()> {
        let mut item = UserItem::new(456);
        item.unique_id = 0xDEADBEEFDEADBEEF;
        item.current_dura = 50;
        item.max_dura = 100;
        item.count = 3;
        item.gem_count = 2;
        item.refined_value = RefinedValue::Dc;
        item.refine_added = 5;
        item.refine_success_chance = 25;
        item.soul_bound_id = 42;
        item.identified = true;
        item.cursed = true;
        item.wedding_ring = 77;
        item.is_shop_item = true;
        item.is_gm_made = true;
        item.awake.awake_type = AwakeType::Dc;
        item.awake.levels = vec![1, 2, 3];
        item.added_stats.set(Stat::MaxDC, 7);
        item.added_stats.set(Stat::Luck, 1);
        item.slots = vec![None, None];
        item.expire_info = Some(ExpireInfo::new(1_234_567));
        item.rental_information = Some(RentalInformation {
            owner_name: "Tester".to_string(),
            binding_flags: BindMode::DONT_DROP,
            expiry_date_binary: 8_765_432,
            rental_locked: true,
        });
        item.sealed_info = Some(SealedInfo {
            expiry_date_binary: 9_876_543,
            next_seal_date_binary: 9_999_999,
        });

        let mut buffer = Vec::new();
        item.write_to(&mut buffer)?;
        let mut cursor = Cursor::new(buffer);
        let decoded = UserItem::read_from(&mut cursor, i32::MAX, i32::MAX)?;
        assert_eq!(decoded.unique_id, item.unique_id);
        assert_eq!(decoded.item_index, item.item_index);
        assert_eq!(decoded.current_dura, item.current_dura);
        assert_eq!(decoded.max_dura, item.max_dura);
        assert_eq!(decoded.count, item.count);
        assert_eq!(decoded.gem_count, item.gem_count);
        assert_eq!(decoded.refined_value, item.refined_value);
        assert_eq!(decoded.refine_added, item.refine_added);
        assert_eq!(decoded.refine_success_chance, item.refine_success_chance);
        assert_eq!(decoded.soul_bound_id, item.soul_bound_id);
        assert_eq!(decoded.identified, item.identified);
        assert_eq!(decoded.cursed, item.cursed);
        assert_eq!(decoded.wedding_ring, item.wedding_ring);
        assert_eq!(decoded.is_shop_item, item.is_shop_item);
        assert_eq!(decoded.is_gm_made, item.is_gm_made);
        assert_eq!(decoded.awake, item.awake);
        assert_eq!(decoded.added_stats, item.added_stats);
        assert_eq!(decoded.expire_info, item.expire_info);
        assert_eq!(decoded.rental_information, item.rental_information);
        assert_eq!(decoded.sealed_info, item.sealed_info);
        Ok(())
    }

    #[test]
    fn game_shop_item_roundtrip_packet() -> SharedResult<()> {
        let mut info = ItemInfo::new();
        info.index = 200;
        info.name = "Potion(Deluxe)".to_string();
        info.item_type = ItemType::Potion;
        info.stack_size = 20;

        let mut item = GameShopItem::default();
        item.item_index = info.index;
        item.g_index = 5;
        item.info = Some(info.clone());
        item.gold_price = 1_000;
        item.credit_price = 250;
        item.count = 3;
        item.class_name = "All".to_string();
        item.category = "Consumables".to_string();
        item.stock = 99;
        item.in_stock = true;
        item.deal = true;
        item.top_item = false;
        item.date_binary = 1_234_567_890;
        item.can_buy_gold = true;
        item.can_buy_credit = false;

        let mut buffer = Vec::new();
        item.write_to(&mut buffer, true)?;
        let mut cursor = Cursor::new(buffer);
        let decoded = GameShopItem::read_from_packet(&mut cursor)?;

        assert_eq!(decoded.item_index, item.item_index);
        assert_eq!(decoded.g_index, item.g_index);
        assert_eq!(decoded.gold_price, item.gold_price);
        assert_eq!(decoded.credit_price, item.credit_price);
        assert_eq!(decoded.count, item.count);
        assert_eq!(decoded.class_name, item.class_name);
        assert_eq!(decoded.category, item.category);
        assert_eq!(decoded.stock, item.stock);
        assert_eq!(decoded.in_stock, item.in_stock);
        assert_eq!(decoded.deal, item.deal);
        assert_eq!(decoded.top_item, item.top_item);
        assert_eq!(decoded.date_binary, item.date_binary);
        assert_eq!(decoded.can_buy_gold, item.can_buy_gold);
        assert_eq!(decoded.can_buy_credit, item.can_buy_credit);
        assert_eq!(
            decoded.info.as_ref().map(|i| i.name.clone()),
            item.info.as_ref().map(|i| i.name.clone())
        );
        Ok(())
    }

    #[test]
    fn game_shop_item_read_database_format() -> SharedResult<()> {
        let mut buffer = Vec::new();
        buffer.write_i32::<LittleEndian>(7)?;
        buffer.write_i32::<LittleEndian>(9)?;
        buffer.write_u32::<LittleEndian>(50_000)?;
        buffer.write_u32::<LittleEndian>(15_000)?;
        buffer.write_u16::<LittleEndian>(1)?;
        write_dotnet_string(&mut buffer, "Wizard")?;
        write_dotnet_string(&mut buffer, "Scrolls")?;
        buffer.write_i32::<LittleEndian>(10)?;
        write_bool(&mut buffer, false)?;
        write_bool(&mut buffer, false)?;
        write_bool(&mut buffer, true)?;
        buffer.write_i64::<LittleEndian>(-123_456)?;
        write_bool(&mut buffer, true)?; // CanBuyGold
        write_bool(&mut buffer, false)?; // CanBuyCredit

        let mut cursor = Cursor::new(buffer);
        let decoded = GameShopItem::read_from(&mut cursor, 106, i32::MAX)?;

        assert_eq!(decoded.item_index, 7);
        assert_eq!(decoded.g_index, 9);
        assert_eq!(decoded.gold_price, 50_000);
        assert_eq!(decoded.credit_price, 15_000);
        assert_eq!(decoded.count, 1);
        assert_eq!(decoded.class_name, "Wizard");
        assert_eq!(decoded.category, "Scrolls");
        assert_eq!(decoded.stock, 10);
        assert!(!decoded.in_stock);
        assert!(!decoded.deal);
        assert!(decoded.top_item);
        assert_eq!(decoded.date_binary, -123_456);
        assert!(decoded.can_buy_gold);
        assert!(!decoded.can_buy_credit);
        Ok(())
    }

    #[test]
    fn game_shop_item_write_order_matches_csharp() -> SharedResult<()> {
        let mut item = GameShopItem::default();
        item.item_index = 1;
        item.g_index = 2;
        item.class_name = "All".to_string();
        item.category = "General".to_string();
        item.date_binary = 42;
        item.can_buy_gold = true;
        item.can_buy_credit = false;

        let mut buffer = Vec::new();
        item.write_to(&mut buffer, false)?;
        let len = buffer.len();
        assert_eq!(buffer[len - 2], 0); // CanBuyCredit
        assert_eq!(buffer[len - 1], 1); // CanBuyGold
        Ok(())
    }

    #[test]
    fn chat_item_roundtrip() -> SharedResult<()> {
        let item = ChatItem {
            unique_id: 42,
            title: "Legend (Hero)".to_string(),
            grid: MirGridType::Inventory,
        };

        assert_eq!(item.regex_internal_name(), "<Legend \\(Hero\\)>");
        assert_eq!(item.internal_name(), "<Legend (Hero)/42>");

        let mut buffer = Vec::new();
        item.write_to(&mut buffer)?;
        let mut cursor = Cursor::new(buffer);
        let decoded = ChatItem::read_from(&mut cursor)?;
        assert_eq!(decoded, item);
        Ok(())
    }

    #[test]
    fn item_set_status_completion() {
        let status = ItemSets {
            set: ItemSet::Spirit,
            types: vec![ItemType::Weapon, ItemType::Armour],
            count: 5,
        };
        assert_eq!(status.required_amount(), 5);
        assert!(status.is_complete());

        let status2 = ItemSets {
            set: ItemSet::RedOrchid,
            types: vec![ItemType::Bracelet],
            count: 2,
        };
        assert_eq!(status2.required_amount(), 3);
        assert!(!status2.is_complete());
    }

    #[test]
    fn item_rental_information_roundtrip() -> SharedResult<()> {
        let info = ItemRentalInformation {
            item_id: 123,
            item_name: "DragonBlade".to_string(),
            renting_player_name: "PlayerOne".to_string(),
            item_return_date_binary: 9_876_543,
        };

        let mut buffer = Vec::new();
        info.write_to(&mut buffer)?;
        let mut cursor = Cursor::new(buffer);
        let decoded = ItemRentalInformation::read_from(&mut cursor)?;
        assert_eq!(decoded, info);
        Ok(())
    }
}
