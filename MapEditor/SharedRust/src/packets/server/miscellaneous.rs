// 杂项数据包（任务、重生、其他）
use super::super::base::Packet;
use crate::data::stats::SharedResult;
use crate::enums::ServerPacketIds;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Read;

/// CompleteQuest - 完成任务 (200)
#[derive(Debug, Clone)]
pub struct CompleteQuest {
    pub quest_id: i32,              // 任务ID
}

impl Packet for CompleteQuest {
    const OPCODE: i16 = ServerPacketIds::CompleteQuest as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_i32::<LittleEndian>(self.quest_id)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let quest_id = reader.read_i32::<LittleEndian>()?;
        Ok(Self { quest_id })
    }
}

/// ShareQuest - 分享任务 (201)
#[derive(Debug, Clone)]
pub struct ShareQuest {
    pub quest_id: i32,              // 任务ID
}

impl Packet for ShareQuest {
    const OPCODE: i16 = ServerPacketIds::ShareQuest as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_i32::<LittleEndian>(self.quest_id)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let quest_id = reader.read_i32::<LittleEndian>()?;
        Ok(Self { quest_id })
    }
}

/// GainedQuestItem - 获得任务物品 (203)
#[derive(Debug, Clone)]
pub struct GainedQuestItem {
    pub item_id: i32,               // 物品ID
}

impl Packet for GainedQuestItem {
    const OPCODE: i16 = ServerPacketIds::GainedQuestItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_i32::<LittleEndian>(self.item_id)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let item_id = reader.read_i32::<LittleEndian>()?;
        Ok(Self { item_id })
    }
}

/// DeleteQuestItem - 删除任务物品 (204)
#[derive(Debug, Clone)]
pub struct DeleteQuestItem {
    pub item_id: i32,               // 物品ID
}

impl Packet for DeleteQuestItem {
    const OPCODE: i16 = ServerPacketIds::DeleteQuestItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_i32::<LittleEndian>(self.item_id)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let item_id = reader.read_i32::<LittleEndian>()?;
        Ok(Self { item_id })
    }
}

/// CancelReincarnation - 取消重生 (205)
#[derive(Debug, Clone)]
pub struct CancelReincarnation {
    // C# 原实现为空
}

impl Packet for CancelReincarnation {
    const OPCODE: i16 = ServerPacketIds::CancelReincarnation as i16;

    fn write_body<W: std::io::Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self {})
    }
}

/// RequestReincarnation - 请求重生 (206)
#[derive(Debug, Clone)]
pub struct RequestReincarnation {
    // C# 原实现为空
}

impl Packet for RequestReincarnation {
    const OPCODE: i16 = ServerPacketIds::RequestReincarnation as i16;

    fn write_body<W: std::io::Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self {})
    }
}

/// MountUpdate - 坐骑更新 (196)
#[derive(Debug, Clone)]
pub struct MountUpdate {
    pub object_id: u32,
    pub mount_type: i16,            // 坐骑类型
    pub riding_mount: bool,
}

impl Packet for MountUpdate {
    const OPCODE: i16 = ServerPacketIds::MountUpdate as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_i16::<LittleEndian>(self.mount_type)?;
        writer.write_u8(self.riding_mount as u8)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let mount_type = reader.read_i16::<LittleEndian>()?;
        let riding_mount = reader.read_u8()? != 0;
        Ok(Self {
            object_id,
            mount_type,
            riding_mount,
        })
    }
}

/// FishingUpdate - 钓鱼更新 (198)
#[derive(Debug, Clone)]
pub struct FishingUpdate {
    pub fishing_progress: i32,      // 钓鱼进度
    pub fishing_success: bool,      // 是否成功
}

impl Packet for FishingUpdate {
    const OPCODE: i16 = ServerPacketIds::FishingUpdate as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_i32::<LittleEndian>(self.fishing_progress)?;
        writer.write_u8(if self.fishing_success { 1 } else { 0 })?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let fishing_progress = reader.read_i32::<LittleEndian>()?;
        let fishing_success = reader.read_u8()? != 0;
        Ok(Self {
            fishing_progress,
            fishing_success,
        })
    }
}

/// ObjectSitDown - 对象坐下 (158)
#[derive(Debug, Clone)]
pub struct ObjectSitDown {
    pub object_id: u32,             // 对象ID
    pub direction: u8,              // 方向
    pub location: (i32, i32),       // 位置
}

impl Packet for ObjectSitDown {
    const OPCODE: i16 = ServerPacketIds::ObjectSitDown as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u8(self.direction)?;
        writer.write_i32::<LittleEndian>(self.location.0)?;
        writer.write_i32::<LittleEndian>(self.location.1)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let direction = reader.read_u8()?;
        let location_x = reader.read_i32::<LittleEndian>()?;
        let location_y = reader.read_i32::<LittleEndian>()?;
        Ok(Self {
            object_id,
            direction,
            location: (location_x, location_y),
        })
    }
}

/// InTrapRock - 在陷阱岩石中 (159)
#[derive(Debug, Clone)]
pub struct InTrapRock {
    pub in_trap: bool,              // 是否在陷阱中
}

impl Packet for InTrapRock {
    const OPCODE: i16 = ServerPacketIds::InTrapRock as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_u8(if self.in_trap { 1 } else { 0 })?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let in_trap = reader.read_u8()? != 0;
        Ok(Self { in_trap })
    }
}

/// BaseStatsInfo - 基础属性信息 (160)
#[derive(Debug, Clone)]
pub struct BaseStatsInfo {
    pub stats: Vec<i32>,            // 属性值列表
}

impl Packet for BaseStatsInfo {
    const OPCODE: i16 = ServerPacketIds::BaseStatsInfo as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_i32::<LittleEndian>(self.stats.len() as i32)?;
        for &stat in &self.stats {
            writer.write_i32::<LittleEndian>(stat)?;
        }
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()?;
        let mut stats = Vec::with_capacity(count as usize);
        for _ in 0..count {
            stats.push(reader.read_i32::<LittleEndian>()?);
        }
        Ok(Self { stats })
    }
}

/// HeroBaseStatsInfo - 英雄基础属性信息 (161)
#[derive(Debug, Clone)]
pub struct HeroBaseStatsInfo {
    pub stats: Vec<i32>,            // 属性值列表
}

impl Packet for HeroBaseStatsInfo {
    const OPCODE: i16 = ServerPacketIds::HeroBaseStatsInfo as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_i32::<LittleEndian>(self.stats.len() as i32)?;
        for &stat in &self.stats {
            writer.write_i32::<LittleEndian>(stat)?;
        }
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()?;
        let mut stats = Vec::with_capacity(count as usize);
        for _ in 0..count {
            stats.push(reader.read_i32::<LittleEndian>()?);
        }
        Ok(Self { stats })
    }
}

/// UserName - 用户名称 (162)
#[derive(Debug, Clone)]
pub struct UserName {
    pub object_id: u32,             // 对象ID
    pub name: String,               // 名称
}

impl Packet for UserName {
    const OPCODE: i16 = ServerPacketIds::UserName as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        use crate::binary::write_dotnet_string;
        writer.write_u32::<LittleEndian>(self.object_id)?;
        write_dotnet_string(writer, &self.name)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let len = reader.read_i32::<LittleEndian>()?;
        let mut bytes = vec![0u8; len as usize];
        reader.read_exact(&mut bytes)?;
        let name = String::from_utf8_lossy(&bytes).to_string();
        Ok(Self { object_id, name })
    }
}

/// ChatItemStats - 聊天物品属性 (163)
#[derive(Debug, Clone)]
pub struct ChatItemStats {
    pub unique_id: u64,             // 物品唯一ID
    pub stats: String,              // 属性字符串
}

impl Packet for ChatItemStats {
    const OPCODE: i16 = ServerPacketIds::ChatItemStats as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        use crate::binary::write_dotnet_string;
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        write_dotnet_string(writer, &self.stats)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let len = reader.read_i32::<LittleEndian>()?;
        let mut bytes = vec![0u8; len as usize];
        reader.read_exact(&mut bytes)?;
        let stats = String::from_utf8_lossy(&bytes).to_string();
        Ok(Self { unique_id, stats })
    }
}

/// GuildStatus - 公会状态 (166)
#[derive(Debug, Clone)]
pub struct GuildStatus {
    pub guild_name: String,         // 公会名称
    pub rank_name: String,          // 等级名称
}

impl Packet for GuildStatus {
    const OPCODE: i16 = ServerPacketIds::GuildStatus as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use crate::binary::write_dotnet_string;
        write_dotnet_string(writer, &self.guild_name)?;
        write_dotnet_string(writer, &self.rank_name)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let guild_len = reader.read_i32::<LittleEndian>()?;
        let mut guild_bytes = vec![0u8; guild_len as usize];
        reader.read_exact(&mut guild_bytes)?;
        let guild_name = String::from_utf8_lossy(&guild_bytes).to_string();
        
        let rank_len = reader.read_i32::<LittleEndian>()?;
        let mut rank_bytes = vec![0u8; rank_len as usize];
        reader.read_exact(&mut rank_bytes)?;
        let rank_name = String::from_utf8_lossy(&rank_bytes).to_string();
        
        Ok(Self {
            guild_name,
            rank_name,
        })
    }
}

/// GuildInvite - 公会邀请 (167)
#[derive(Debug, Clone)]
pub struct GuildInvite {
    pub guild_name: String,         // 公会名称
}

impl Packet for GuildInvite {
    const OPCODE: i16 = ServerPacketIds::GuildInvite as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use crate::binary::write_dotnet_string;
        write_dotnet_string(writer, &self.guild_name)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let len = reader.read_i32::<LittleEndian>()?;
        let mut bytes = vec![0u8; len as usize];
        reader.read_exact(&mut bytes)?;
        let guild_name = String::from_utf8_lossy(&bytes).to_string();
        Ok(Self { guild_name })
    }
}

/// GuildExpGain - 公会经验获得 (168)
#[derive(Debug, Clone)]
pub struct GuildExpGain {
    pub amount: u32,                // 经验数量
}

impl Packet for GuildExpGain {
    const OPCODE: i16 = ServerPacketIds::GuildExpGain as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_u32::<LittleEndian>(self.amount)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let amount = reader.read_u32::<LittleEndian>()?;
        Ok(Self { amount })
    }
}

/// GuildNameRequest - 公会名称请求 (169)
#[derive(Debug, Clone)]
pub struct GuildNameRequest {
    // C# 原实现为空
}

impl Packet for GuildNameRequest {
    const OPCODE: i16 = ServerPacketIds::GuildNameRequest as i16;

    fn write_body<W: std::io::Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self {})
    }
}

/// GuildStorageGoldChange - 公会仓库金币变化 (170)
#[derive(Debug, Clone)]
pub struct GuildStorageGoldChange {
    pub change: i32,                // 变化量
    pub total: u32,                 // 总金币
}

impl Packet for GuildStorageGoldChange {
    const OPCODE: i16 = ServerPacketIds::GuildStorageGoldChange as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_i32::<LittleEndian>(self.change)?;
        writer.write_u32::<LittleEndian>(self.total)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let change = reader.read_i32::<LittleEndian>()?;
        let total = reader.read_u32::<LittleEndian>()?;
        Ok(Self { change, total })
    }
}

/// GuildStorageItemChange - 公会仓库物品变化 (171)
#[derive(Debug, Clone)]
pub struct GuildStorageItemChange {
    pub change_type: u8,            // 变化类型 (add/remove)
    pub slot: i32,                  // 槽位
}

impl Packet for GuildStorageItemChange {
    const OPCODE: i16 = ServerPacketIds::GuildStorageItemChange as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_u8(self.change_type)?;
        writer.write_i32::<LittleEndian>(self.slot)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let change_type = reader.read_u8()?;
        let slot = reader.read_i32::<LittleEndian>()?;
        Ok(Self { change_type, slot })
    }
}

/// GuildRequestWar - 公会请求战争 (173)
#[derive(Debug, Clone)]
pub struct GuildRequestWar {
    pub guild_name: String,         // 公会名称
}

impl Packet for GuildRequestWar {
    const OPCODE: i16 = ServerPacketIds::GuildRequestWar as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use crate::binary::write_dotnet_string;
        write_dotnet_string(writer, &self.guild_name)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let len = reader.read_i32::<LittleEndian>()?;
        let mut bytes = vec![0u8; len as usize];
        reader.read_exact(&mut bytes)?;
        let guild_name = String::from_utf8_lossy(&bytes).to_string();
        Ok(Self { guild_name })
    }
}

/// NewHero - 新英雄 (175)
#[derive(Debug, Clone)]
pub struct NewHero {
    pub hero_info: String,          // 英雄信息
}

impl Packet for NewHero {
    const OPCODE: i16 = ServerPacketIds::NewHero as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use crate::binary::write_dotnet_string;
        write_dotnet_string(writer, &self.hero_info)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let len = reader.read_i32::<LittleEndian>()?;
        let mut bytes = vec![0u8; len as usize];
        reader.read_exact(&mut bytes)?;
        let hero_info = String::from_utf8_lossy(&bytes).to_string();
        Ok(Self { hero_info })
    }
}

/// HeroInformation - 英雄信息 (176)
#[derive(Debug, Clone)]
pub struct HeroInformation {
    pub hero_id: u32,               // 英雄ID
}

impl Packet for HeroInformation {
    const OPCODE: i16 = ServerPacketIds::HeroInformation as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_u32::<LittleEndian>(self.hero_id)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let hero_id = reader.read_u32::<LittleEndian>()?;
        Ok(Self { hero_id })
    }
}

/// UnlockHeroAutoPot - 解锁英雄自动喝药 (178)
#[derive(Debug, Clone)]
pub struct UnlockHeroAutoPot {
    pub unlocked: bool,             // 是否解锁
}

impl Packet for UnlockHeroAutoPot {
    const OPCODE: i16 = ServerPacketIds::UnlockHeroAutoPot as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_u8(if self.unlocked { 1 } else { 0 })?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unlocked = reader.read_u8()? != 0;
        Ok(Self { unlocked })
    }
}

/// SetAutoPotItem - 设置自动喝药物品 (180)
#[derive(Debug, Clone)]
pub struct SetAutoPotItem {
    pub slot: i32,                  // 槽位
    pub item_id: i32,               // 物品ID
}

impl Packet for SetAutoPotItem {
    const OPCODE: i16 = ServerPacketIds::SetAutoPotItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_i32::<LittleEndian>(self.slot)?;
        writer.write_i32::<LittleEndian>(self.item_id)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let slot = reader.read_i32::<LittleEndian>()?;
        let item_id = reader.read_i32::<LittleEndian>()?;
        Ok(Self { slot, item_id })
    }
}

/// ChangeHero - 切换英雄 (183)
#[derive(Debug, Clone)]
pub struct ChangeHero {
    pub success: bool,              // 是否成功
}

impl Packet for ChangeHero {
    const OPCODE: i16 = ServerPacketIds::ChangeHero as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_u8(if self.success { 1 } else { 0 })?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let success = reader.read_u8()? != 0;
        Ok(Self { success })
    }
}

/// TakeBackHeroItem - 取回英雄物品 (52)
#[derive(Debug, Clone)]
pub struct TakeBackHeroItem {
    pub from: i32,                  // 来源槽位
    pub to: i32,                    // 目标槽位
    pub success: bool,              // 是否成功
}

impl Packet for TakeBackHeroItem {
    const OPCODE: i16 = ServerPacketIds::TakeBackHeroItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_i32::<LittleEndian>(self.from)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        writer.write_u8(if self.success { 1 } else { 0 })?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let from = reader.read_i32::<LittleEndian>()?;
        let to = reader.read_i32::<LittleEndian>()?;
        let success = reader.read_u8()? != 0;
        Ok(Self { from, to, success })
    }
}

/// TransferHeroItem - 转移英雄物品 (53)
#[derive(Debug, Clone)]
pub struct TransferHeroItem {
    pub from: i32,                  // 来源槽位
    pub to: i32,                    // 目标槽位
    pub success: bool,              // 是否成功
}

impl Packet for TransferHeroItem {
    const OPCODE: i16 = ServerPacketIds::TransferHeroItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_i32::<LittleEndian>(self.from)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        writer.write_u8(if self.success { 1 } else { 0 })?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let from = reader.read_i32::<LittleEndian>()?;
        let to = reader.read_i32::<LittleEndian>()?;
        let success = reader.read_u8()? != 0;
        Ok(Self { from, to, success })
    }
}

/// NewChatItem - 新聊天物品 (34)
#[derive(Debug, Clone)]
pub struct NewChatItem {
    pub item_id: i32,               // 物品ID
}

impl Packet for NewChatItem {
    const OPCODE: i16 = ServerPacketIds::NewChatItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_i32::<LittleEndian>(self.item_id)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let item_id = reader.read_i32::<LittleEndian>()?;
        Ok(Self { item_id })
    }
}

/// NewHeroInfo - 新英雄信息 (33)
#[derive(Debug, Clone)]
pub struct NewHeroInfo {
    pub info: String,               // 信息
}

impl Packet for NewHeroInfo {
    const OPCODE: i16 = ServerPacketIds::NewHeroInfo as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use crate::binary::write_dotnet_string;
        write_dotnet_string(writer, &self.info)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let len = reader.read_i32::<LittleEndian>()?;
        let mut bytes = vec![0u8; len as usize];
        reader.read_exact(&mut bytes)?;
        let info = String::from_utf8_lossy(&bytes).to_string();
        Ok(Self { info })
    }
}

/// AllowObserve - 允许观察 (140)
#[derive(Debug, Clone)]
pub struct AllowObserve {
    pub allowed: bool,              // 是否允许
}

impl Packet for AllowObserve {
    const OPCODE: i16 = ServerPacketIds::AllowObserve as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_u8(if self.allowed { 1 } else { 0 })?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let allowed = reader.read_u8()? != 0;
        Ok(Self { allowed })
    }
}

/// DepositTradeItem - 存入交易物品 (48)
#[derive(Debug, Clone)]
pub struct DepositTradeItem {
    pub from_slot: i32,             // 来源槽位
    pub success: bool,              // 是否成功
}

impl Packet for DepositTradeItem {
    const OPCODE: i16 = ServerPacketIds::DepositTradeItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_i32::<LittleEndian>(self.from_slot)?;
        writer.write_u8(if self.success { 1 } else { 0 })?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let from_slot = reader.read_i32::<LittleEndian>()?;
        let success = reader.read_u8()? != 0;
        Ok(Self {
            from_slot,
            success,
        })
    }
}

/// RetrieveTradeItem - 取回交易物品 (49)
#[derive(Debug, Clone)]
pub struct RetrieveTradeItem {
    pub from_slot: i32,             // 来源槽位
    pub success: bool,              // 是否成功
}

impl Packet for RetrieveTradeItem {
    const OPCODE: i16 = ServerPacketIds::RetrieveTradeItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_i32::<LittleEndian>(self.from_slot)?;
        writer.write_u8(if self.success { 1 } else { 0 })?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let from_slot = reader.read_i32::<LittleEndian>()?;
        let success = reader.read_u8()? != 0;
        Ok(Self {
            from_slot,
            success,
        })
    }
}
