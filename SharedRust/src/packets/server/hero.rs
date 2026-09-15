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
    /// #2892 批C：英雄最大 HP/MP —— C# 客户端 `HeroInfoPanel` 的百分比条需要
    /// `Stats[Stat.HP]/Stats[Stat.MP]`（`HeroDialogs.cs:668-695`）；C# 从英雄对象读，
    /// Rust 由服务端随包下发（原 wire 只有当前值）。
    pub max_hp: i32,
    pub max_mp: i32,
    pub experience: i64,
    pub max_experience: i64,
    // ===== #2892 批57：C# `CharacterDialog` 状态页/状态二页所需属性 =====
    // C# `HeroInformation : UserInformation` 携带全部 `Stats`；本端按两页用到的字段显式下发
    // （`Client/MirScenes/Dialogs/CharacterDialog.cs:94-133`）。
    /// 最小 AC（C# `Stat.MinAC`）
    /// 最大 AC（`Stat.MaxAC`）
    /// 最小 MAC（`Stat.MinMAC`）
    /// 最大 MAC（`Stat.MaxMAC`）
    /// 最小 DC（`Stat.MinDC`）
    /// 最大 DC（`Stat.MaxDC`）
    /// 最小 MC（`Stat.MinMC`）
    /// 最大 MC（`Stat.MaxMC`）
    /// 最小 SC（`Stat.MinSC`）
    /// 最大 SC（`Stat.MaxSC`）
    /// 暴击率 %（`Stat.CriticalRate`）
    /// 暴击伤害（`Stat.CriticalDamage`）
    /// 攻击速度（`Stat.AttackSpeed`）
    /// 命中（`Stat.Accuracy`）
    /// 敏捷（`Stat.Agility`）
    /// 幸运（`Stat.Luck`）
    /// 魔法躲避（`Stat.MagicResist`）
    /// 毒躲避（`Stat.PoisonResist`）
    /// 体力恢复（`Stat.HealthRecovery`）
    /// 魔法恢复（`Stat.SpellRecovery`）
    /// 毒恢复（`Stat.PoisonRecovery`）
    /// 神圣（`Stat.Holy`）
    /// 冰冻（`Stat.Freezing`）
    /// 毒攻击（`Stat.PoisonAttack`）
    // ===== #2892 批57：C# `CharacterDialog` 状态页/状态二页所需属性 =====
    // C# `HeroInformation : UserInformation` 携带全部 `Stats`；本端按两页用到的字段显式下发
    // （`Client/MirScenes/Dialogs/CharacterDialog.cs:94-133`）。
    /// 最小 AC（C# `Stat.MinAC`）
    pub min_ac: i32,
    /// 最大 AC（`Stat.MaxAC`）
    pub max_ac: i32,
    /// 最小 MAC（`Stat.MinMAC`）
    pub min_mac: i32,
    /// 最大 MAC（`Stat.MaxMAC`）
    pub max_mac: i32,
    /// 最小 DC（`Stat.MinDC`）
    pub min_dc: i32,
    /// 最大 DC（`Stat.MaxDC`）
    pub max_dc: i32,
    /// 最小 MC（`Stat.MinMC`）
    pub min_mc: i32,
    /// 最大 MC（`Stat.MaxMC`）
    pub max_mc: i32,
    /// 最小 SC（`Stat.MinSC`）
    pub min_sc: i32,
    /// 最大 SC（`Stat.MaxSC`）
    pub max_sc: i32,
    /// 暴击率 %（`Stat.CriticalRate`）
    pub critical_rate: i32,
    /// 暴击伤害（`Stat.CriticalDamage`）
    pub critical_damage: i32,
    /// 攻击速度（`Stat.AttackSpeed`）
    pub attack_speed: i32,
    /// 命中（`Stat.Accuracy`）
    pub accuracy: i32,
    /// 敏捷（`Stat.Agility`）
    pub agility: i32,
    /// 幸运（`Stat.Luck`）
    pub luck: i32,
    /// 魔法躲避（`Stat.MagicResist`）
    pub magic_resist: i32,
    /// 毒躲避（`Stat.PoisonResist`）
    pub poison_resist: i32,
    /// 体力恢复（`Stat.HealthRecovery`）
    pub health_recovery: i32,
    /// 魔法恢复（`Stat.SpellRecovery`）
    pub spell_recovery: i32,
    /// 毒恢复（`Stat.PoisonRecovery`）
    pub poison_recovery: i32,
    /// 神圣（`Stat.Holy`）
    pub holy: i32,
    /// 冰冻（`Stat.Freezing`）
    pub freezing: i32,
    /// 毒攻击（`Stat.PoisonAttack`）
    pub poison_attack: i32,
    /// 当前背包重量（C# `CurrentBagWeight`）
    pub current_bag_weight: i32,
    /// 当前穿戴重量（C# `CurrentWearWeight`）
    pub current_wear_weight: i32,
    /// 当前手持重量（C# `CurrentHandWeight`）
    pub current_hand_weight: i32,
    /// 负重上限（C# `Stats[Stat.BagWeight]`）
    pub max_bag_weight: i32,
    /// 穿戴上限（`Stats[Stat.WearWeight]`）
    pub max_wear_weight: i32,
    /// 手持上限（`Stats[Stat.HandWeight]`）
    pub max_hand_weight: i32,
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
        let max_hp = reader.read_i32::<LittleEndian>()?;
        let max_mp = reader.read_i32::<LittleEndian>()?;
        let experience = reader.read_i64::<LittleEndian>()?;
        let max_experience = reader.read_i64::<LittleEndian>()?;
        let min_ac = reader.read_i32::<LittleEndian>()?;
        let max_ac = reader.read_i32::<LittleEndian>()?;
        let min_mac = reader.read_i32::<LittleEndian>()?;
        let max_mac = reader.read_i32::<LittleEndian>()?;
        let min_dc = reader.read_i32::<LittleEndian>()?;
        let max_dc = reader.read_i32::<LittleEndian>()?;
        let min_mc = reader.read_i32::<LittleEndian>()?;
        let max_mc = reader.read_i32::<LittleEndian>()?;
        let min_sc = reader.read_i32::<LittleEndian>()?;
        let max_sc = reader.read_i32::<LittleEndian>()?;
        let critical_rate = reader.read_i32::<LittleEndian>()?;
        let critical_damage = reader.read_i32::<LittleEndian>()?;
        let attack_speed = reader.read_i32::<LittleEndian>()?;
        let accuracy = reader.read_i32::<LittleEndian>()?;
        let agility = reader.read_i32::<LittleEndian>()?;
        let luck = reader.read_i32::<LittleEndian>()?;
        let magic_resist = reader.read_i32::<LittleEndian>()?;
        let poison_resist = reader.read_i32::<LittleEndian>()?;
        let health_recovery = reader.read_i32::<LittleEndian>()?;
        let spell_recovery = reader.read_i32::<LittleEndian>()?;
        let poison_recovery = reader.read_i32::<LittleEndian>()?;
        let holy = reader.read_i32::<LittleEndian>()?;
        let freezing = reader.read_i32::<LittleEndian>()?;
        let poison_attack = reader.read_i32::<LittleEndian>()?;
        let current_bag_weight = reader.read_i32::<LittleEndian>()?;
        let current_wear_weight = reader.read_i32::<LittleEndian>()?;
        let current_hand_weight = reader.read_i32::<LittleEndian>()?;
        let max_bag_weight = reader.read_i32::<LittleEndian>()?;
        let max_wear_weight = reader.read_i32::<LittleEndian>()?;
        let max_hand_weight = reader.read_i32::<LittleEndian>()?;

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
            max_hp,
            max_mp,
            experience,
            max_experience,
            min_ac,
            max_ac,
            min_mac,
            max_mac,
            min_dc,
            max_dc,
            min_mc,
            max_mc,
            min_sc,
            max_sc,
            critical_rate,
            critical_damage,
            attack_speed,
            accuracy,
            agility,
            luck,
            magic_resist,
            poison_resist,
            health_recovery,
            spell_recovery,
            poison_recovery,
            holy,
            freezing,
            poison_attack,
            current_bag_weight,
            current_wear_weight,
            current_hand_weight,
            max_bag_weight,
            max_wear_weight,
            max_hand_weight,
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
        writer.write_i32::<LittleEndian>(self.max_hp)?;
        writer.write_i32::<LittleEndian>(self.max_mp)?;
        writer.write_i64::<LittleEndian>(self.experience)?;
        writer.write_i64::<LittleEndian>(self.max_experience)?;
        writer.write_i32::<LittleEndian>(self.min_ac)?;
        writer.write_i32::<LittleEndian>(self.max_ac)?;
        writer.write_i32::<LittleEndian>(self.min_mac)?;
        writer.write_i32::<LittleEndian>(self.max_mac)?;
        writer.write_i32::<LittleEndian>(self.min_dc)?;
        writer.write_i32::<LittleEndian>(self.max_dc)?;
        writer.write_i32::<LittleEndian>(self.min_mc)?;
        writer.write_i32::<LittleEndian>(self.max_mc)?;
        writer.write_i32::<LittleEndian>(self.min_sc)?;
        writer.write_i32::<LittleEndian>(self.max_sc)?;
        writer.write_i32::<LittleEndian>(self.critical_rate)?;
        writer.write_i32::<LittleEndian>(self.critical_damage)?;
        writer.write_i32::<LittleEndian>(self.attack_speed)?;
        writer.write_i32::<LittleEndian>(self.accuracy)?;
        writer.write_i32::<LittleEndian>(self.agility)?;
        writer.write_i32::<LittleEndian>(self.luck)?;
        writer.write_i32::<LittleEndian>(self.magic_resist)?;
        writer.write_i32::<LittleEndian>(self.poison_resist)?;
        writer.write_i32::<LittleEndian>(self.health_recovery)?;
        writer.write_i32::<LittleEndian>(self.spell_recovery)?;
        writer.write_i32::<LittleEndian>(self.poison_recovery)?;
        writer.write_i32::<LittleEndian>(self.holy)?;
        writer.write_i32::<LittleEndian>(self.freezing)?;
        writer.write_i32::<LittleEndian>(self.poison_attack)?;
        writer.write_i32::<LittleEndian>(self.current_bag_weight)?;
        writer.write_i32::<LittleEndian>(self.current_wear_weight)?;
        writer.write_i32::<LittleEndian>(self.current_hand_weight)?;
        writer.write_i32::<LittleEndian>(self.max_bag_weight)?;
        writer.write_i32::<LittleEndian>(self.max_wear_weight)?;
        writer.write_i32::<LittleEndian>(self.max_hand_weight)?;

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
            max_hp: 800,
            max_mp: 400,
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
            min_ac: 11,
            max_ac: 22,
            min_mac: 33,
            max_mac: 44,
            min_dc: 55,
            max_dc: 66,
            min_mc: 77,
            max_mc: 88,
            min_sc: 99,
            max_sc: 111,
            critical_rate: 12,
            critical_damage: 150,
            attack_speed: 700,
            accuracy: 13,
            agility: 14,
            luck: 15,
            magic_resist: 16,
            poison_resist: 17,
            health_recovery: 18,
            spell_recovery: 19,
            poison_recovery: 20,
            holy: 21,
            freezing: 22,
            poison_attack: 23,
            current_bag_weight: 120,
            current_wear_weight: 80,
            current_hand_weight: 30,
            max_bag_weight: 400,
            max_wear_weight: 200,
            max_hand_weight: 100,
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
        // #2892 批C：最大 HP/MP 随包往返（HUD 百分比条用）
        assert_eq!((read.max_hp, read.max_mp), (800, 400));
        // #2892 批57：英雄属性块（状态页/状态二页）逐字段往返——首/尾 + 两个中段
        assert_eq!(read.min_ac, 11);
        assert_eq!(read.max_sc, 111);
        assert_eq!(read.critical_damage, 150);
        assert_eq!(read.poison_attack, 23);
        assert_eq!(read.current_bag_weight, 120);
        assert_eq!(read.max_hand_weight, 100);
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
