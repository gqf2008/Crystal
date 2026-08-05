//! Hero System Packets
//!
//! This module contains all hero-related packet definitions and parsers.
//! 包格式对齐 C# Shared/ServerPackets.cs + Shared/Data/ClientData.cs。

use super::super::base::Packet;
use crate::data::stats::SharedResult;
use crate::{
    binary::write_dotnet_string,
    data::client_data::{ClientHeroInformation, ClientMagic},
    data::item::UserItem,
    enums::{HeroBehaviour, HeroSpawnState, MirClass, MirGender, ServerPacketIds},
};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

// ============================================================================
// Packet Structures
// ============================================================================

/// Update hero spawn state (C# S.UpdateHeroSpawnState: 1 byte state)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateHeroSpawnState {
    pub state: HeroSpawnState,
}

/// Set auto potion value (C# S.SetAutoPotValue: stat u8 + value u32)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetAutoPotValue {
    pub stat: u8,
    pub value: u32,
}

/// Set hero behaviour (C# S.SetHeroBehaviour: 1 byte HeroBehaviour)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetHeroBehaviour {
    pub behaviour: HeroBehaviour,
}

/// Manage heroes list (C# S.ManageHeroes:
///   MaximumCount i32 + CurrentHero(bool+info) + Heroes(bool+count+每项 bool+info))
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManageHeroes {
    pub max_count: i32,
    pub current_hero: Option<ClientHeroInformation>,
    /// 展开后的英雄列表（只含存在项，wire 上的 null 项被跳过）
    pub heroes: Vec<ClientHeroInformation>,
}

/// Hero creation request response (C# S.HeroCreateRequest)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeroCreateRequest {
    pub can_create_class: Vec<bool>,
}

// ============================================================================
// PacketMessage Implementations
// ============================================================================

impl Packet for UpdateHeroSpawnState {
    const OPCODE: i16 = ServerPacketIds::UpdateHeroSpawnState as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let state = HeroSpawnState::try_from(reader.read_u8()?)?;
        Ok(Self { state })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.state as u8)?;
        Ok(())
    }
}

impl Packet for SetAutoPotValue {
    const OPCODE: i16 = ServerPacketIds::SetAutoPotValue as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let stat = reader.read_u8()?;
        let value = reader.read_u32::<LittleEndian>()?;
        Ok(Self { stat, value })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.stat)?;
        writer.write_u32::<LittleEndian>(self.value)?;
        Ok(())
    }
}

impl Packet for SetHeroBehaviour {
    const OPCODE: i16 = ServerPacketIds::SetHeroBehaviour as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let behaviour = HeroBehaviour::try_from(reader.read_u8()?)?;
        Ok(Self { behaviour })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.behaviour as u8)?;
        Ok(())
    }
}

fn write_hero_info<W: Write>(writer: &mut W, hero: &ClientHeroInformation) -> SharedResult<()> {
    // C# ClientHeroInformation.Save: Index i32 + Name string + Level u16 + Class u8 + Gender u8
    writer.write_i32::<LittleEndian>(hero.index)?;
    write_dotnet_string(writer, &hero.name)?;
    writer.write_u16::<LittleEndian>(hero.level)?;
    writer.write_u8(hero.class as u8)?;
    writer.write_u8(hero.gender as u8)?;
    Ok(())
}

impl Packet for ManageHeroes {
    const OPCODE: i16 = ServerPacketIds::ManageHeroes as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let max_count = reader.read_i32::<LittleEndian>()?;
        let current_hero = if reader.read_u8()? != 0 {
            Some(ClientHeroInformation::read_from(reader)?)
        } else {
            None
        };
        let mut heroes = Vec::new();
        if reader.read_u8()? != 0 {
            let count = reader.read_i32::<LittleEndian>()? as usize;
            for _ in 0..count {
                if reader.read_u8()? != 0 {
                    heroes.push(ClientHeroInformation::read_from(reader)?);
                }
            }
        }
        Ok(Self {
            max_count,
            current_hero,
            heroes,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.max_count)?;
        writer.write_u8(if self.current_hero.is_some() { 1 } else { 0 })?;
        if let Some(hero) = &self.current_hero {
            write_hero_info(writer, hero)?;
        }
        writer.write_u8(1)?; // Heroes != null
        writer.write_i32::<LittleEndian>(self.heroes.len() as i32)?;
        for hero in &self.heroes {
            writer.write_u8(1)?;
            write_hero_info(writer, hero)?;
        }
        Ok(())
    }
}

impl Packet for HeroCreateRequest {
    const OPCODE: i16 = ServerPacketIds::HeroCreateRequest as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()? as usize;
        let mut can_create_class = Vec::with_capacity(count);
        for _ in 0..count {
            can_create_class.push(reader.read_u8()? != 0);
        }
        Ok(Self { can_create_class })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.can_create_class.len() as i32)?;
        for &can_create in &self.can_create_class {
            writer.write_u8(if can_create { 1 } else { 0 })?;
        }
        Ok(())
    }
}

/// Full hero information (C# S.HeroInformation : UserInformation + autopot)
/// 顺序对齐 C# HeroInformation.ReadPacket：
///   ObjectID u32 / Name string / Class u8 / Gender u8 / Level u16 / Hair u8
///   HP i32 / MP i32 / Experience i64 / MaxExperience i64
///   Inventory(bool + count + 每项 bool+UserItem) / Equipment(同)
///   Magics count i32 + ClientMagic[]
///   AutoPot bool / AutoHPPercent u8 / AutoMPPercent u8 / HPItemIndex i32 / MPItemIndex i32
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeroInformation {
    pub object_id: u32,
    pub name: String,
    pub class: MirClass,
    pub gender: MirGender,
    pub level: u16,
    pub hair: u8,
    pub hp: i32,
    pub mp: i32,
    pub experience: i64,
    pub max_experience: i64,
    /// 英雄背包（None = 无数据，与 C# bool 标志对应）
    pub inventory: Option<Vec<Option<UserItem>>>,
    /// 英雄装备（None = 无数据）
    pub equipment: Option<Vec<Option<UserItem>>>,
    pub magics: Vec<ClientMagic>,
    pub auto_pot: bool,
    pub auto_hp_percent: u8,
    pub auto_mp_percent: u8,
    pub hp_item_index: i32,
    pub mp_item_index: i32,
}

impl Packet for HeroInformation {
    const OPCODE: i16 = ServerPacketIds::HeroInformation as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let name = crate::binary::read_dotnet_string(reader)?;
        let class = MirClass::try_from(reader.read_u8()?)?;
        let gender = MirGender::try_from(reader.read_u8()?)?;
        let level = reader.read_u16::<LittleEndian>()?;
        let hair = reader.read_u8()?;
        let hp = reader.read_i32::<LittleEndian>()?;
        let mp = reader.read_i32::<LittleEndian>()?;
        let experience = reader.read_i64::<LittleEndian>()?;
        let max_experience = reader.read_i64::<LittleEndian>()?;

        let inventory = if reader.read_u8()? != 0 {
            let count = reader.read_i32::<LittleEndian>()? as usize;
            let mut items = Vec::with_capacity(count.min(1000));
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
            let mut items = Vec::with_capacity(count.min(100));
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

        let magic_count = reader.read_i32::<LittleEndian>()? as usize;
        let mut magics = Vec::with_capacity(magic_count.min(100));
        for _ in 0..magic_count {
            magics.push(ClientMagic::read_from(reader)?);
        }

        let auto_pot = reader.read_u8()? != 0;
        let auto_hp_percent = reader.read_u8()?;
        let auto_mp_percent = reader.read_u8()?;
        let hp_item_index = reader.read_i32::<LittleEndian>()?;
        let mp_item_index = reader.read_i32::<LittleEndian>()?;

        Ok(Self {
            object_id,
            name,
            class,
            gender,
            level,
            hair,
            hp,
            mp,
            experience,
            max_experience,
            inventory,
            equipment,
            magics,
            auto_pot,
            auto_hp_percent,
            auto_mp_percent,
            hp_item_index,
            mp_item_index,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        write_dotnet_string(writer, &self.name)?;
        writer.write_u8(self.class as u8)?;
        writer.write_u8(self.gender as u8)?;
        writer.write_u16::<LittleEndian>(self.level)?;
        writer.write_u8(self.hair)?;
        writer.write_i32::<LittleEndian>(self.hp)?;
        writer.write_i32::<LittleEndian>(self.mp)?;
        writer.write_i64::<LittleEndian>(self.experience)?;
        writer.write_i64::<LittleEndian>(self.max_experience)?;

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

        writer.write_i32::<LittleEndian>(self.magics.len() as i32)?;
        for magic in &self.magics {
            magic.write_to(writer)?;
        }

        writer.write_u8(self.auto_pot as u8)?;
        writer.write_u8(self.auto_hp_percent)?;
        writer.write_u8(self.auto_mp_percent)?;
        writer.write_i32::<LittleEndian>(self.hp_item_index)?;
        writer.write_i32::<LittleEndian>(self.mp_item_index)?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::enums::{MirClass, MirGender};
    use std::io::Cursor;

    fn hero_info(index: i32) -> ClientHeroInformation {
        ClientHeroInformation {
            index,
            name: format!("Hero{index}"),
            level: 30,
            class: MirClass::Warrior,
            gender: MirGender::Male,
        }
    }

    #[test]
    fn set_hero_behaviour_roundtrip() {
        for b in [
            HeroBehaviour::Attack,
            HeroBehaviour::CounterAttack,
            HeroBehaviour::Follow,
            HeroBehaviour::Custom,
        ] {
            let pkt = SetHeroBehaviour { behaviour: b };
            let mut buf = Vec::new();
            pkt.write_body(&mut buf).unwrap();
            assert_eq!(buf.len(), 1, "C# S.SetHeroBehaviour 应为 1 字节");
            let mut cur = Cursor::new(&buf);
            let read = SetHeroBehaviour::read_body(&mut cur).unwrap();
            assert_eq!(read, pkt);
        }
    }

    #[test]
    fn manage_heroes_roundtrip() {
        let pkt = ManageHeroes {
            max_count: 2,
            current_hero: Some(hero_info(1)),
            heroes: vec![hero_info(1), hero_info(2)],
        };
        let mut buf = Vec::new();
        pkt.write_body(&mut buf).unwrap();
        let mut cur = Cursor::new(&buf);
        let read = ManageHeroes::read_body(&mut cur).unwrap();
        assert_eq!(read, pkt);
        assert_eq!(read.max_count, 2);
        assert_eq!(read.heroes.len(), 2);
    }

    #[test]
    fn manage_heroes_empty_roundtrip() {
        let pkt = ManageHeroes {
            max_count: 1,
            current_hero: None,
            heroes: vec![],
        };
        let mut buf = Vec::new();
        pkt.write_body(&mut buf).unwrap();
        let mut cur = Cursor::new(&buf);
        let read = ManageHeroes::read_body(&mut cur).unwrap();
        assert_eq!(read, pkt);
        assert!(read.current_hero.is_none());
        assert!(read.heroes.is_empty());
    }

    #[test]
    fn hero_information_roundtrip() {
        let mut item = UserItem::new(1001);
        item.unique_id = 9001;
        item.count = 5;
        item.current_dura = 10;
        item.max_dura = 20;
        let pkt = HeroInformation {
            object_id: 0x1000_1234,
            name: "HeroOne".to_string(),
            class: MirClass::Warrior,
            gender: MirGender::Male,
            level: 32,
            hair: 3,
            hp: 500,
            mp: 200,
            experience: 12345,
            max_experience: 99999,
            inventory: Some(vec![Some(item.clone()), None, Some(item.clone())]),
            equipment: Some(vec![None, Some(item)]),
            magics: vec![ClientMagic {
                name: "FireBall".to_string(),
                spell: crate::enums::Spell::FireBall,
                base_cost: 2,
                level_cost: 3,
                icon: 4,
                level1: 5,
                level2: 6,
                level3: 7,
                need1: 8,
                need2: 9,
                need3: 10,
                level: 1,
                key: 0,
                experience: 100,
                delay: 11,
                range: 3,
                cast_time: 12,
            }],
            auto_pot: true,
            auto_hp_percent: 30,
            auto_mp_percent: 20,
            hp_item_index: 42,
            mp_item_index: -1,
        };
        let mut buf = Vec::new();
        pkt.write_body(&mut buf).unwrap();
        let mut cur = Cursor::new(&buf);
        let read = HeroInformation::read_body(&mut cur).unwrap();
        assert_eq!(read, pkt);
        assert_eq!(read.inventory.as_ref().unwrap().len(), 3);
        assert_eq!(read.equipment.as_ref().unwrap().len(), 2);
        assert_eq!(read.magics.len(), 1);
        assert!(read.auto_pot);
    }

    #[test]
    fn new_hero_result_roundtrip() {
        // C# S.NewHero: 1 byte Result
        use crate::packets::server::miscellaneous::NewHero;
        for result in [0u8, 1, 5, 10] {
            let pkt = NewHero { result };
            let mut buf = Vec::new();
            pkt.write_body(&mut buf).unwrap();
            assert_eq!(buf.len(), 1, "C# S.NewHero 应为 1 字节");
            let mut cur = Cursor::new(&buf);
            let read = NewHero::read_body(&mut cur).unwrap();
            assert_eq!(read.result, result);
        }
    }
}
