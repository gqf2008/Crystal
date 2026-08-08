//! Miscellaneous Game System Packets (Client → Server)
//! Contains: Marriage, Mentor, Modes, Fishing, Reincarnation, Combine, Awakening,
//! Intelligent Creature, Item Rental, and other systems

use super::super::base::Packet;
use crate::binary::{read_dotnet_string, write_dotnet_string};
use crate::data::stats::SharedResult;
use crate::enums::{AttackMode, AwakeType, ClientPacketIds, MirGridType, PetMode};
use crate::map::Point;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

// ==================== Mode Change Packets ====================

/// Change attack mode (Peace, Group, Guild, EnemyGuild, All, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeAMode {
    pub mode: AttackMode,
}

impl Packet for ChangeAMode {
    const OPCODE: i16 = ClientPacketIds::ChangeAMode as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let mode = AttackMode::try_from(reader.read_u8()?)?;
        Ok(Self { mode })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.mode as u8)?;
        Ok(())
    }
}

/// Change pet mode (Both, MoveOnly, AttackOnly, None, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangePMode {
    pub mode: PetMode,
}

impl Packet for ChangePMode {
    const OPCODE: i16 = ClientPacketIds::ChangePMode as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let mode = PetMode::try_from(reader.read_u8()?)?;
        Ok(Self { mode })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.mode as u8)?;
        Ok(())
    }
}

/// Toggle trading allowance
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeTrade {
    pub allow_trade: bool,
}

impl Packet for ChangeTrade {
    const OPCODE: i16 = ClientPacketIds::ChangeTrade as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let allow_trade = reader.read_u8()? != 0;
        Ok(Self { allow_trade })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(if self.allow_trade { 1 } else { 0 })?;
        Ok(())
    }
}

// ==================== Marriage System Packets ====================

/// Request marriage with another player
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarriageRequest;

impl Packet for MarriageRequest {
    const OPCODE: i16 = ClientPacketIds::MarriageRequest as i16;

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

/// Reply to marriage proposal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarriageReply {
    pub accept_invite: bool,
}

impl Packet for MarriageReply {
    const OPCODE: i16 = ClientPacketIds::MarriageReply as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let accept_invite = reader.read_u8()? != 0;
        Ok(Self { accept_invite })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(if self.accept_invite { 1 } else { 0 })?;
        Ok(())
    }
}

/// Change marriage settings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeMarriage;

impl Packet for ChangeMarriage {
    const OPCODE: i16 = ClientPacketIds::ChangeMarriage as i16;

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

/// Request divorce
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DivorceRequest;

impl Packet for DivorceRequest {
    const OPCODE: i16 = ClientPacketIds::DivorceRequest as i16;

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

/// Reply to divorce request
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DivorceReply {
    pub accept_invite: bool,
}

impl Packet for DivorceReply {
    const OPCODE: i16 = ClientPacketIds::DivorceReply as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let accept_invite = reader.read_u8()? != 0;
        Ok(Self { accept_invite })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(if self.accept_invite { 1 } else { 0 })?;
        Ok(())
    }
}

// ==================== Mentor System Packets ====================

/// Request to add mentor
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddMentor {
    pub name: String,
}

impl Packet for AddMentor {
    const OPCODE: i16 = ClientPacketIds::AddMentor as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let name = read_dotnet_string(reader)?;
        Ok(Self { name })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.name)?;
        Ok(())
    }
}

/// Reply to mentor request
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MentorReply {
    pub accept_invite: bool,
}

impl Packet for MentorReply {
    const OPCODE: i16 = ClientPacketIds::MentorReply as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let accept_invite = reader.read_u8()? != 0;
        Ok(Self { accept_invite })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(if self.accept_invite { 1 } else { 0 })?;
        Ok(())
    }
}

/// Toggle allow mentor requests
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllowMentor;

impl Packet for AllowMentor {
    const OPCODE: i16 = ClientPacketIds::AllowMentor as i16;

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

/// Cancel mentor relationship
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CancelMentor;

impl Packet for CancelMentor {
    const OPCODE: i16 = ClientPacketIds::CancelMentor as i16;

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

// ==================== Resurrection Packets ====================

/// Revive at town
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TownRevive;

impl Packet for TownRevive {
    const OPCODE: i16 = ClientPacketIds::TownRevive as i16;

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

// ==================== Equipment Packets ====================

/// Equip item to equipment slot
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipSlotItem {
    pub grid: MirGridType,
    pub unique_id: u64,
    pub to_slot: i32,
    pub grid_to: MirGridType,
}

impl Packet for EquipSlotItem {
    const OPCODE: i16 = ClientPacketIds::EquipSlotItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let grid = MirGridType::try_from(reader.read_u8()?)?;
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let to_slot = reader.read_i32::<LittleEndian>()?;
        let grid_to = MirGridType::try_from(reader.read_u8()?)?;
        Ok(Self {
            grid,
            unique_id,
            to_slot,
            grid_to,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.grid as u8)?;
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_i32::<LittleEndian>(self.to_slot)?;
        writer.write_u8(self.grid_to as u8)?;
        Ok(())
    }
}

// ==================== Fishing System Packets ====================

/// Cast fishing rod
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FishingCast {
    pub cast_out: bool,
}

impl Packet for FishingCast {
    const OPCODE: i16 = ClientPacketIds::FishingCast as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let cast_out = reader.read_u8()? != 0;
        Ok(Self { cast_out })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(if self.cast_out { 1 } else { 0 })?;
        Ok(())
    }
}

/// Toggle autocast fishing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FishingChangeAutocast {
    pub auto_cast: bool,
}

impl Packet for FishingChangeAutocast {
    const OPCODE: i16 = ClientPacketIds::FishingChangeAutocast as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let auto_cast = reader.read_u8()? != 0;
        Ok(Self { auto_cast })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(if self.auto_cast { 1 } else { 0 })?;
        Ok(())
    }
}

// ==================== Reincarnation Packets ====================

/// Accept reincarnation offer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcceptReincarnation;

impl Packet for AcceptReincarnation {
    const OPCODE: i16 = ClientPacketIds::AcceptReincarnation as i16;

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

/// Cancel reincarnation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CancelReincarnation;

impl Packet for CancelReincarnation {
    const OPCODE: i16 = ClientPacketIds::CancelReincarnation as i16;

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

// ==================== Item Crafting Packets ====================

/// Combine items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CombineItem {
    pub grid: MirGridType,
    pub id_from: u64,
    pub id_to: u64,
}

impl Packet for CombineItem {
    const OPCODE: i16 = ClientPacketIds::CombineItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let grid = MirGridType::try_from(reader.read_u8()?)?;
        let id_from = reader.read_u64::<LittleEndian>()?;
        let id_to = reader.read_u64::<LittleEndian>()?;
        Ok(Self {
            grid,
            id_from,
            id_to,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.grid as u8)?;
        writer.write_u64::<LittleEndian>(self.id_from)?;
        writer.write_u64::<LittleEndian>(self.id_to)?;
        Ok(())
    }
}

// ==================== Awakening System Packets ====================

/// Get materials needed for awakening
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AwakeningNeedMaterials {
    pub unique_id: u64,
    pub awake_type: AwakeType,
}

impl Packet for AwakeningNeedMaterials {
    const OPCODE: i16 = ClientPacketIds::AwakeningNeedMaterials as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let awake_type = AwakeType::try_from(reader.read_u8()?)?;
        Ok(Self {
            unique_id,
            awake_type,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u8(self.awake_type as u8)?;
        Ok(())
    }
}

/// Lock/unlock item for awakening
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AwakeningLockedItem {
    pub unique_id: u64,
    pub locked: bool,
}

impl Packet for AwakeningLockedItem {
    const OPCODE: i16 = ClientPacketIds::AwakeningLockedItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let locked = reader.read_u8()? != 0;
        Ok(Self { unique_id, locked })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u8(if self.locked { 1 } else { 0 })?;
        Ok(())
    }
}

/// Perform awakening on item
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Awakening {
    pub unique_id: u64,
    pub awake_type: AwakeType,
    pub position_idx: u32,
}

impl Packet for Awakening {
    const OPCODE: i16 = ClientPacketIds::Awakening as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let awake_type = AwakeType::try_from(reader.read_u8()?)?;
        let position_idx = reader.read_u32::<LittleEndian>()?;
        Ok(Self {
            unique_id,
            awake_type,
            position_idx,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u8(self.awake_type as u8)?;
        writer.write_u32::<LittleEndian>(self.position_idx)?;
        Ok(())
    }
}

/// Disassemble item into materials
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisassembleItem {
    pub unique_id: u64,
}

impl Packet for DisassembleItem {
    const OPCODE: i16 = ClientPacketIds::DisassembleItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        Ok(Self { unique_id })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        Ok(())
    }
}

/// Downgrade awakening level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DowngradeAwakening {
    pub unique_id: u64,
}

impl Packet for DowngradeAwakening {
    const OPCODE: i16 = ClientPacketIds::DowngradeAwakening as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        Ok(Self { unique_id })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        Ok(())
    }
}

/// Reset added item stats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResetAddedItem {
    pub unique_id: u64,
}

impl Packet for ResetAddedItem {
    const OPCODE: i16 = ClientPacketIds::ResetAddedItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        Ok(Self { unique_id })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        Ok(())
    }
}

// ==================== Intelligent Creature Packets ====================

/// Request intelligent creature updates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestIntelligentCreatureUpdates {
    pub update: bool,
}

impl Packet for RequestIntelligentCreatureUpdates {
    const OPCODE: i16 = ClientPacketIds::RequestIntelligentCreatureUpdates as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let update = reader.read_u8()? != 0;
        Ok(Self { update })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(if self.update { 1 } else { 0 })?;
        Ok(())
    }
}

/// Update intelligent creature（简化 wire：[type u8][pet_mode u8][custom_name dotnet][summon u8][unsummon u8][release u8]）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateIntelligentCreature {
    pub creature_type: u8,
    /// 拾取模式（C# IntelligentCreaturePickupMode：0=自动 1=半自动）
    pub pet_mode: u8,
    /// 自定义名称（改名时携带）
    pub custom_name: String,
    pub summon_me: bool,
    pub unsummon_me: bool,
    pub release_me: bool,
}

impl Packet for UpdateIntelligentCreature {
    const OPCODE: i16 = ClientPacketIds::UpdateIntelligentCreature as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let creature_type = reader.read_u8()?;
        let pet_mode = reader.read_u8()?;
        let custom_name = read_dotnet_string(reader)?;
        let summon_me = reader.read_u8()? != 0;
        let unsummon_me = reader.read_u8()? != 0;
        let release_me = reader.read_u8()? != 0;
        Ok(Self {
            creature_type,
            pet_mode,
            custom_name,
            summon_me,
            unsummon_me,
            release_me,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.creature_type)?;
        writer.write_u8(self.pet_mode)?;
        write_dotnet_string(writer, &self.custom_name)?;
        writer.write_u8(if self.summon_me { 1 } else { 0 })?;
        writer.write_u8(if self.unsummon_me { 1 } else { 0 })?;
        writer.write_u8(if self.release_me { 1 } else { 0 })?;
        Ok(())
    }
}

/// Intelligent creature pickup item
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntelligentCreaturePickup {
    pub mouse_mode: bool,
    pub location: Point,
}

impl Packet for IntelligentCreaturePickup {
    const OPCODE: i16 = ClientPacketIds::IntelligentCreaturePickup as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let mouse_mode = reader.read_u8()? != 0;
        let x = reader.read_i32::<LittleEndian>()?;
        let y = reader.read_i32::<LittleEndian>()?;
        Ok(Self {
            mouse_mode,
            location: Point::new(x, y),
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(if self.mouse_mode { 1 } else { 0 })?;
        writer.write_i32::<LittleEndian>(self.location.x)?;
        writer.write_i32::<LittleEndian>(self.location.y)?;
        Ok(())
    }
}

// Note: Item Rental System Packets have been moved to item.rs to avoid duplication

// ==================== Other Packets ====================

/// Buy from game shop
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameshopBuy {
    pub g_index: i32,
    pub quantity: u8,
    pub p_type: i32,
}

impl Packet for GameshopBuy {
    const OPCODE: i16 = ClientPacketIds::GameshopBuy as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let g_index = reader.read_i32::<LittleEndian>()?;
        let quantity = reader.read_u8()?;
        let p_type = reader.read_i32::<LittleEndian>()?;
        Ok(Self {
            g_index,
            quantity,
            p_type,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.g_index)?;
        writer.write_u8(self.quantity)?;
        writer.write_i32::<LittleEndian>(self.p_type)?;
        Ok(())
    }
}

/// Report issue to GM (simplified - full version has image data)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportIssue {
    pub message: String,
}

impl Packet for ReportIssue {
    const OPCODE: i16 = ClientPacketIds::ReportIssue as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        // Skip image data for simplified version
        let message = read_dotnet_string(reader)?;
        Ok(Self { message })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        // Skip image data
        write_dotnet_string(writer, &self.message)?;
        Ok(())
    }
}

/// Get ranking information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetRanking {
    pub rank_index: u8,
    /// 仅在线（C# RankingDialog OnlineOnly）
    pub online_only: bool,
}

impl Packet for GetRanking {
    const OPCODE: i16 = ClientPacketIds::GetRanking as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let rank_index = reader.read_u8()?;
        let online_only = reader.read_u8()? != 0;
        Ok(Self {
            rank_index,
            online_only,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.rank_index)?;
        writer.write_u8(self.online_only as u8)?;
        Ok(())
    }
}

/// Open door
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Opendoor {
    pub door_index: u8,
}

impl Packet for Opendoor {
    const OPCODE: i16 = ClientPacketIds::Opendoor as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let door_index = reader.read_u8()?;
        Ok(Self { door_index })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.door_index)?;
        Ok(())
    }
}

/// Request user name by ID
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestUserName {
    pub user_id: u32,
}

impl Packet for RequestUserName {
    const OPCODE: i16 = ClientPacketIds::RequestUserName as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let user_id = reader.read_u32::<LittleEndian>()?;
        Ok(Self { user_id })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.user_id)?;
        Ok(())
    }
}

/// Request chat item information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestChatItem {
    pub chat_item_id: u64,
}

impl Packet for RequestChatItem {
    const OPCODE: i16 = ClientPacketIds::RequestChatItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let chat_item_id = reader.read_u64::<LittleEndian>()?;
        Ok(Self { chat_item_id })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.chat_item_id)?;
        Ok(())
    }
}
