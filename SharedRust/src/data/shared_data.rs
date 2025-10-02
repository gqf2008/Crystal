//! Shared Data Structures Module
//!
//! This module contains shared data structures used by both client and server
//! for various game systems including guild territories, ranking, quests, and world maps.
//!
//! ## Overview
//! These structures are designed to be serialized and transmitted over the network,
//! maintaining compatibility with the original C# .NET binary format.
//!
//! ## Structures
//! - **Door**: Guild territory door state management
//! - **RankCharacterInfo**: Leaderboard/ranking character information
//! - **QuestItemReward**: Quest reward item definitions
//! - **WorldMapIcon**: World map marker icons
//! - **WorldMapSetup**: World map configuration
//! - **ClientGTMap**: Guild territory (conquest) map information
//!
//! ## Usage
//! ```ignore
//! use mir2_shared::data::shared_data::{Door, RankCharacterInfo, WorldMapSetup};
//!
//! // Example: Reading door state
//! let door = Door::read_from(&mut reader)?;
//! println!("Door {} is {:?}", door.index, door.door_state);
//! ```
//!
//! ## Serialization
//! All structures implement binary serialization compatible with the original
//! Legend of Mir 2 .NET format using little-endian byte order.

use std::io::{Read, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};

use crate::binary::{read_bool, read_dotnet_string, write_bool, write_dotnet_string};
use crate::enums::{DoorState, MirClass};
use crate::map::Point;
use crate::data::stats::{SharedError, SharedResult};

use std::convert::TryFrom;

/// Door state information for guild territory defense systems
///
/// Doors are used in guild territories (conquest zones) to control access
/// and defend strategic points. This structure tracks the current state,
/// position, and visual representation of a door.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Door {
    pub index: u8,
    pub door_state: DoorState,
    pub image_index: u8,
    pub last_tick: i64,
    pub location: Point,
}

impl Door {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let index = reader.read_u8()?;
        let door_state_value = reader.read_u8()?;
        let door_state = DoorState::try_from(door_state_value)
            .map_err(|_| SharedError::unknown_enum("DoorState", door_state_value.into()))?;
        let image_index = reader.read_u8()?;
        let last_tick = reader.read_i64::<LittleEndian>()?;
        let location = Point::read_from(reader)?;

        Ok(Self {
            index,
            door_state,
            image_index,
            last_tick,
            location,
        })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.index)?;
        writer.write_u8(self.door_state as u8)?;
        writer.write_u8(self.image_index)?;
        writer.write_i64::<LittleEndian>(self.last_tick)?;
        self.location.write_to(writer)?;
        Ok(())
    }
}

/// Rank character information for leaderboards and ranking systems
///
/// This structure represents a player's ranking information displayed in
/// various leaderboards (level, wealth, PvP, etc.). Contains essential info
/// for display without exposing sensitive data like experience points.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RankCharacterInfo {
    pub player_id: i64,
    pub name: String,
    pub level: i32,
    pub class: MirClass,
}

impl RankCharacterInfo {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let player_id = reader.read_i64::<LittleEndian>()?;
        let name = read_dotnet_string(reader)?;
        let level = reader.read_i32::<LittleEndian>()?;
        let class_value = reader.read_u8()?;
        let class = MirClass::try_from(class_value)
            .map_err(|_| SharedError::unknown_enum("MirClass", class_value.into()))?;

        Ok(Self {
            player_id,
            name,
            level,
            class,
        })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i64::<LittleEndian>(self.player_id)?;
        write_dotnet_string(writer, &self.name)?;
        writer.write_i32::<LittleEndian>(self.level)?;
        writer.write_u8(self.class as u8)?;
        Ok(())
    }
}

/// Quest item reward definition
///
/// Defines an item reward for completing quests. Contains the item index
/// (reference to ItemInfo database) and the quantity to be rewarded.
/// Used in both fixed rewards and selectable reward pools.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestItemReward {
    pub item_index: i32,
    pub count: u16,
}

impl QuestItemReward {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let item_index = reader.read_i32::<LittleEndian>()?;
        let count = reader.read_u16::<LittleEndian>()?;

        Ok(Self { item_index, count })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.item_index)?;
        writer.write_u16::<LittleEndian>(self.count)?;
        Ok(())
    }
}

/// World map icon marker
///
/// Represents a clickable icon on the world map UI. Players can see these
/// markers to identify important locations such as towns, dungeons, bosses,
/// or special event areas. The image_index corresponds to the icon graphic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldMapIcon {
    pub image_index: i32,
    pub title: String,
    pub map_index: i32,
}

impl WorldMapIcon {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let image_index = reader.read_i32::<LittleEndian>()?;
        let title = read_dotnet_string(reader)?;
        let map_index = reader.read_i32::<LittleEndian>()?;

        Ok(Self {
            image_index,
            title,
            map_index,
        })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.image_index)?;
        write_dotnet_string(writer, &self.title)?;
        writer.write_i32::<LittleEndian>(self.map_index)?;
        Ok(())
    }
}

/// World map setup configuration
///
/// Contains the complete world map configuration sent to clients,
/// including whether the world map feature is enabled and all icon
/// markers to display. Server sends this during login or map changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldMapSetup {
    pub enabled: bool,
    pub icons: Vec<WorldMapIcon>,
}

impl WorldMapSetup {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let enabled = read_bool(reader)?;
        let count = reader.read_i32::<LittleEndian>()?;
        if count < 0 {
            return Err(SharedError::NegativeLength {
                field: "world_map_icons",
                length: count,
            });
        }

        let mut icons = Vec::with_capacity(count as usize);
        for _ in 0..count {
            icons.push(WorldMapIcon::read_from(reader)?);
        }

        Ok(Self { enabled, icons })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_bool(writer, self.enabled)?;
        writer.write_i32::<LittleEndian>(self.icons.len() as i32)?;
        for icon in &self.icons {
            icon.write_to(writer)?;
        }
        Ok(())
    }
}

/// Guild territory (conquest) map information
///
/// Represents a conquerable territory in the guild conquest system.
/// Includes ownership information, bidding price, rental duration, and
/// leadership details. Used for territory management and war systems.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientGTMap {
    pub index: i32,
    pub name: String,
    pub owner: String,
    pub leader: String,
    pub leader2: String,
    pub price: i32,
    pub days: i32,
    pub begin: i32,
}

impl ClientGTMap {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let index = reader.read_i32::<LittleEndian>()?;
        let name = read_dotnet_string(reader)?;
        let owner = read_dotnet_string(reader)?;
        let leader = read_dotnet_string(reader)?;
        let leader2 = read_dotnet_string(reader)?;
        let price = reader.read_i32::<LittleEndian>()?;
        let days = reader.read_i32::<LittleEndian>()?;
        let begin = reader.read_i32::<LittleEndian>()?;

        Ok(Self {
            index,
            name,
            owner,
            leader,
            leader2,
            price,
            days,
            begin,
        })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.index)?;
        write_dotnet_string(writer, &self.name)?;
        write_dotnet_string(writer, &self.owner)?;
        write_dotnet_string(writer, &self.leader)?;
        write_dotnet_string(writer, &self.leader2)?;
        writer.write_i32::<LittleEndian>(self.price)?;
        writer.write_i32::<LittleEndian>(self.days)?;
        writer.write_i32::<LittleEndian>(self.begin)?;
        Ok(())
    }
}
