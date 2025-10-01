use std::io::Read;

use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};

use crate::binary::{read_bool, read_dotnet_string};
use crate::stats::{SharedError, SharedResult};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
}
