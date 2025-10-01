use std::io::Read;

use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};

use crate::binary::{read_bool, read_dotnet_string};
use crate::stats::{SharedError, SharedResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let x = reader.read_i32::<LittleEndian>()?;
        let y = reader.read_i32::<LittleEndian>()?;
        Ok(Self { x, y })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientMovementInfo {
    pub destination: i32,
    pub title: String,
    pub location: Point,
    pub icon: i32,
}

impl ClientMovementInfo {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let destination = reader.read_i32::<LittleEndian>()?;
        let title = read_dotnet_string(reader)?;
        let location = Point::read_from(reader)?;
        let icon = reader.read_i32::<LittleEndian>()?;

        Ok(Self {
            destination,
            title,
            location,
            icon,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientNpcInfo {
    pub object_id: u32,
    pub name: String,
    pub location: Point,
    pub icon: i32,
    pub can_teleport_to: bool,
}

impl ClientNpcInfo {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let name = read_dotnet_string(reader)?;
        let location = Point::read_from(reader)?;
        let icon = reader.read_i32::<LittleEndian>()?;
        let can_teleport_to = read_bool(reader)?;

        Ok(Self {
            object_id,
            name,
            location,
            icon,
            can_teleport_to,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientMapInfo {
    pub title: String,
    pub width: i32,
    pub height: i32,
    pub big_map: i32,
    pub movements: Vec<ClientMovementInfo>,
    pub npcs: Vec<ClientNpcInfo>,
}

impl ClientMapInfo {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let title = read_dotnet_string(reader)?;
        let width = reader.read_i32::<LittleEndian>()?;
        let height = reader.read_i32::<LittleEndian>()?;
        let big_map = reader.read_i32::<LittleEndian>()?;

        let movement_count = reader.read_i32::<LittleEndian>()?;
        if movement_count < 0 {
            return Err(SharedError::NegativeLength {
                field: "movements",
                length: movement_count,
            });
        }
        let mut movements = Vec::with_capacity(movement_count as usize);
        for _ in 0..movement_count {
            movements.push(ClientMovementInfo::read_from(reader)?);
        }

        let npc_count = reader.read_i32::<LittleEndian>()?;
        if npc_count < 0 {
            return Err(SharedError::NegativeLength {
                field: "npcs",
                length: npc_count,
            });
        }
        let mut npcs = Vec::with_capacity(npc_count as usize);
        for _ in 0..npc_count {
            npcs.push(ClientNpcInfo::read_from(reader)?);
        }

        Ok(Self {
            title,
            width,
            height,
            big_map,
            movements,
            npcs,
        })
    }
}
