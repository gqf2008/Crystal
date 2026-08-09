//! mock 服务端包结构体（从 mock.rs 拆分，#1147）

use crossbeam_channel::{Receiver, Sender};
use mir2_shared::data::client_data::{ClientMagic, ClientQuestProgress, SelectInfo};
use mir2_shared::data::item::ItemInfo;
use mir2_shared::enums::{
    ChatType, ClientPacketIds, HeroBehaviour, ItemType, LevelEffects, MirClass, MirDirection,
    MirGender, PoisonType, Spell, SpellEffect, Stat,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use mir2_shared::packets::base::{serialize_packet, Packet, PacketHeader};
use mir2_shared::packets::{client, server};
use crate::network::codec;
use super::send::*;
use super::state::*;


pub(crate) struct MockGameshopMail;

impl Packet for MockGameshopMail {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::ReceiveMail as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u64::<LittleEndian>(9001)?;
        mir2_shared::binary::write_dotnet_string(writer, "GameShop")?;
        mir2_shared::binary::write_dotnet_string(writer, "商城购买成功")?;
        mir2_shared::binary::write_dotnet_string(writer, "感谢购买！物品已通过邮件送达。")?;
        writer.write_i64::<LittleEndian>(0)?; // timestamp
        writer.write_u8(0)?; // read
        writer.write_u8(0)?; // collected
        writer.write_u32::<LittleEndian>(0)?; // gold
        writer.write_u8(1)?; // item_count
        writer.write_u64::<LittleEndian>(9002)?; // uid
        writer.write_u32::<LittleEndian>(1)?; // idx
        mir2_shared::binary::write_dotnet_string(writer, "金创药(小)")?;
        writer.write_u16::<LittleEndian>(1)?; // count
        writer.write_u16::<LittleEndian>(1)?; // cd
        writer.write_u16::<LittleEndian>(1)?; // md
        Ok(())
    }
}

/// #619：AddBuff（客户端格式 [tag u8][ticks u32]）
pub(crate) struct MockAddBuff;

impl Packet for MockAddBuff {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::AddBuff as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u8(3)?; // tag=3 防御提升
        writer.write_u32::<LittleEndian>(10)?; // ticks
        Ok(())
    }
}

/// #619：宠物列表（客户端格式 [count i32][per: type u8][pickup u8][enabled u8][hunger u8][name dotnet]）
pub(crate) struct MockCreatureList;

impl Packet for MockCreatureList {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::UpdateIntelligentCreatureList as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_i32::<LittleEndian>(1)?; // count
        writer.write_u8(mir2_shared::enums::IntelligentCreatureType::BabyPig as u8)?;
        writer.write_u8(1)?; // pickup
        writer.write_u8(1)?; // enabled
        writer.write_u8(0)?; // hunger
        mir2_shared::binary::write_dotnet_string(writer, "小猪")?;
        writer.write_u8(1)?; // active
        for _ in 0..9 { writer.write_u8(0)?; } // filter（默认全部关闭）
        writer.write_u8(0)?; // grade
        Ok(())
    }
}

/// #619：查看玩家（客户端格式 [oid u32][name dotnet][guild dotnet][level u16][class u8][gender u8][count u8][per: uid u64][idx i32][dura i32][max_dura i32]）
pub(crate) struct MockPlayerInspect {
    pub(crate) object_id: u32,
}

impl Packet for MockPlayerInspect {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::PlayerInspect as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u32::<LittleEndian>(self.object_id)?;
        mir2_shared::binary::write_dotnet_string(writer, "bevy2char")?;
        mir2_shared::binary::write_dotnet_string(writer, "测试行会")?;
        writer.write_u16::<LittleEndian>(30)?; // level
        writer.write_u8(MirClass::Warrior as u8)?;
        writer.write_u8(MirGender::Male as u8)?;
        writer.write_u8(1)?; // 装备数
        writer.write_u64::<LittleEndian>(8001)?; // uid
        writer.write_i32::<LittleEndian>(221)?; // idx 木剑
        writer.write_i32::<LittleEndian>(10)?; // dura
        writer.write_i32::<LittleEndian>(12)?; // max_dura
        Ok(())
    }
}

/// #672：行会完整信息（客户端 handle_guild 双格式：name/leader dotnet + notice_count u8 + member_count u8 + gold u32）
pub(crate) struct MockGuildStatus {
    pub(crate) name: String,
    pub(crate) leader: String,
    pub(crate) rank_defs: Vec<(String, u8)>,
    pub(crate) notice: Vec<String>,
    pub(crate) members: Vec<(String, u8, u8, bool)>,
    pub(crate) gold: u32,
}

impl Packet for MockGuildStatus {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::GuildStatus as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::WriteBytesExt;
        mir2_shared::binary::write_dotnet_string(writer, &self.name)?;
        mir2_shared::binary::write_dotnet_string(writer, &self.leader)?;
        writer.write_u8(self.rank_defs.len() as u8)?;
        for (idx, (name, options)) in self.rank_defs.iter().enumerate() {
            writer.write_u8(idx as u8);
            mir2_shared::binary::write_dotnet_string(writer, name)?;
            writer.write_u8(*options)?;
        }
        writer.write_u8(self.notice.len() as u8)?;
        for line in &self.notice {
            mir2_shared::binary::write_dotnet_string(writer, line)?;
        }
        writer.write_u8(self.members.len() as u8)?;
        for (name, rank, rank_index, online) in &self.members {
            mir2_shared::binary::write_dotnet_string(writer, name)?;
            writer.write_u8(*rank)?;
            writer.write_u8(*rank_index)?;
            writer.write_u8(if *online { 1 } else { 0 })?;
        }
        writer.write_u32::<byteorder::LittleEndian>(self.gold)?;
        Ok(())
    }
}

/// #672：行会公告（客户端格式 [count u8][lines dotnet]）
pub(crate) struct MockGuildNoticeChange {
    pub(crate) lines: Vec<String>,
}

impl Packet for MockGuildNoticeChange {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::GuildNoticeChange as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_u8(self.lines.len() as u8)?;
        for line in &self.lines {
            mir2_shared::binary::write_dotnet_string(writer, line)?;
        }
        Ok(())
    }
}

/// #672：行会成员加入（客户端格式 [joined u8][name dotnet]）
pub(crate) struct MockGuildMemberJoined {
    pub(crate) name: String,
}

impl Packet for MockGuildMemberJoined {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::GuildMemberChange as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_u8(1)?; // joined
        mir2_shared::binary::write_dotnet_string(writer, &self.name)?;
        Ok(())
    }
}

/// #672：行会领地页（客户端格式 [count i32][per: id i32][map_index i32][owner dotnet][state u8]）
pub(crate) struct MockTerritoryPage {
    pub(crate) rows: Vec<(i32, i32, String, u8)>,
}

impl Packet for MockTerritoryPage {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::GuildTerritoryPage as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_i32::<LittleEndian>(self.rows.len() as i32)?;
        for (id, map_index, owner, state) in &self.rows {
            writer.write_i32::<LittleEndian>(*id)?;
            writer.write_i32::<LittleEndian>(*map_index)?;
            mir2_shared::binary::write_dotnet_string(writer, owner)?;
            writer.write_u8(*state)?;
        }
        Ok(())
    }
}

/// #672：宣战确认（客户端格式 [guild_name dotnet]）
pub(crate) struct MockGuildRequestWar {
    pub(crate) guild_name: String,
}

impl Packet for MockGuildRequestWar {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::GuildRequestWar as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        mir2_shared::binary::write_dotnet_string(writer, &self.guild_name)?;
        Ok(())
    }
}

/// #672：行会仓库列表回发
pub(crate) fn send_guild_storage_list(
    to_client: &Sender<Vec<u8>>,
    storage: &[Option<mir2_shared::data::item::UserItem>],
) {
    let items: Vec<Option<mir2_shared::data::client_data::GuildStorageItem>> = storage
        .iter()
        .map(|s| {
            s.as_ref().map(|it| mir2_shared::data::client_data::GuildStorageItem {
                item: it.clone(),
                user_id: 100,
            })
        })
        .collect();
    send(to_client, &server::guild::GuildStorageList { items });
}

/// #702：好友列表（客户端格式 [count i32][per: oid u32][name dotnet][memo dotnet][online u8]）
pub(crate) struct MockFriendList;

impl Packet for MockFriendList {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::FriendUpdate as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_i32::<LittleEndian>(1)?; // count
        writer.write_u32::<LittleEndian>(120)?; // oid bevy2char
        mir2_shared::binary::write_dotnet_string(writer, "bevy2char")?;
        mir2_shared::binary::write_dotnet_string(writer, "")?;
        writer.write_u8(0)?; // blocked=false
        writer.write_u8(1)?; // online
        Ok(())
    }
}

/// #702/#1329：婚姻状态（全量 [Name dotnet][Date i64][MapName dotnet][MarriedDays i16]，对齐 C# S.LoverUpdate）
pub(crate) struct MockLoverUpdate {
    pub(crate) married: bool,
}

impl Packet for MockLoverUpdate {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::LoverUpdate as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        mir2_shared::binary::write_dotnet_string(
            writer,
            if self.married { "bevy2char" } else { "" },
        )?;
        writer.write_i64::<LittleEndian>(if self.married { 1_700_000_000 } else { 0 })?;
        mir2_shared::binary::write_dotnet_string(
            writer,
            if self.married { "盟重省" } else { "" },
        )?;
        writer.write_i16::<LittleEndian>(if self.married { 3 } else { 0 })?;
        Ok(())
    }
}

/// #702：行会邀请（客户端格式 [guild_name dotnet]）
pub(crate) struct MockGuildInvitePush {
    pub(crate) guild_name: String,
}

impl Packet for MockGuildInvitePush {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::GuildInvite as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        mir2_shared::binary::write_dotnet_string(writer, &self.guild_name)?;
        Ok(())
    }
}

/// #720：市场页数（客户端格式 [count i32][page dotnet]）
pub(crate) struct MockNPCMarket;

impl Packet for MockNPCMarket {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::NPCMarket as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_i32::<LittleEndian>(1)?;
        mir2_shared::binary::write_dotnet_string(writer, "全部")?;
        Ok(())
    }
}

/// #720：市场列表（客户端格式 [count i32][per: auction_id u64][UserItem][seller dotnet][price u32][type u8][current_bid u32][date i64]）
pub(crate) struct MockNPCMarketPage {
    pub(crate) listings: Vec<(u64, mir2_shared::data::item::UserItem, String, u32, u8, u32)>,
}

impl Packet for MockNPCMarketPage {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::NPCMarketPage as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_i32::<LittleEndian>(self.listings.len() as i32)?;
        for (auction_id, item, seller, price, item_type, current_bid) in &self.listings {
            writer.write_u64::<LittleEndian>(*auction_id)?;
            item.write_to(writer)?;
            mir2_shared::binary::write_dotnet_string(writer, seller)?;
            writer.write_u32::<LittleEndian>(*price)?;
            writer.write_u8(*item_type)?;
            writer.write_u32::<LittleEndian>(*current_bid)?;
            writer.write_i64::<LittleEndian>(0)?; // date
        }
        Ok(())
    }
}

/// #720：寄售结果（客户端格式 [uid u64][success u8]）
pub(crate) struct MockConsignResult {
    pub(crate) uid: u64,
    pub(crate) success: bool,
}

impl Packet for MockConsignResult {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::ConsignItem as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u64::<LittleEndian>(self.uid)?;
        writer.write_u8(if self.success { 1 } else { 0 })?;
        Ok(())
    }
}

/// #755：交易请求（客户端格式 [name dotnet]）
pub(crate) struct MockTradeRequest;

impl Packet for MockTradeRequest {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::TradeRequest as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        mir2_shared::binary::write_dotnet_string(writer, "bevy2char")?;
        Ok(())
    }
}

/// #755：交易金币（客户端格式 [amount u64]）
pub(crate) struct MockTradeGold {
    pub(crate) amount: u32,
}

impl Packet for MockTradeGold {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::TradeGold as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u64::<LittleEndian>(self.amount as u64)?;
        Ok(())
    }
}

/// #755：交易物品入槽回执（客户端格式 [from i32][success u8]）
pub(crate) struct MockTradeDeposit {
    pub(crate) from: i32,
}

impl Packet for MockTradeDeposit {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::DepositTradeItem as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_i32::<LittleEndian>(self.from)?;
        writer.write_u8(1)?;
        Ok(())
    }
}

/// #755：交易锁定（客户端格式 [a u8][b u8]）
pub(crate) struct MockTradeConfirm;

impl Packet for MockTradeConfirm {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::TradeConfirm as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_u8(1)?; // a 锁定
        writer.write_u8(1)?; // b 锁定
        Ok(())
    }
}

/// #755：师徒更新（客户端格式 [name dotnet][level i32][online u8][exp i64]）
pub(crate) struct MockMentorUpdate {
    pub(crate) name: String,
    pub(crate) level: i32,
    pub(crate) online: bool,
    pub(crate) exp: i64,
}

impl Packet for MockMentorUpdate {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::MentorUpdate as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        mir2_shared::binary::write_dotnet_string(writer, &self.name)?;
        writer.write_i32::<LittleEndian>(self.level)?;
        writer.write_u8(if self.online { 1 } else { 0 })?;
        writer.write_i64::<LittleEndian>(self.exp)?;
        Ok(())
    }
}

/// #769：租赁更新（客户端格式 [hasdata u8][fee u32][period i32]）
pub(crate) struct MockUpdateRentalItem {
    pub(crate) fee: u32,
    pub(crate) period: i32,
}

impl Packet for MockUpdateRentalItem {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::UpdateRentalItem as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u8(1)?; // hasdata
        writer.write_u32::<LittleEndian>(self.fee)?;
        writer.write_i32::<LittleEndian>(self.period)?;
        Ok(())
    }
}

/// #769：租赁可确认（客户端格式 [u8]）
pub(crate) struct MockRentalCanConfirm;

impl Packet for MockRentalCanConfirm {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::CanConfirmItemRental as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_u8(1)?;
        Ok(())
    }
}

/// #769：租赁成交确认（客户端格式 [u8]）
pub(crate) struct MockRentalConfirm;

impl Packet for MockRentalConfirm {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::ConfirmItemRental as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_u8(1)?;
        Ok(())
    }
}

/// #769：市场成功消息（客户端格式 [msg dotnet]）
pub(crate) struct MockMarketSuccess {
    pub(crate) message: String,
}

impl Packet for MockMarketSuccess {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::MarketSuccess as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        mir2_shared::binary::write_dotnet_string(writer, &self.message)?;
        Ok(())
    }
}

/// #788：邮件条目（客户端 parse_receive_mail 完整格式）
pub(crate) struct MockMailEntry {
    pub(crate) sender: String,
    pub(crate) subject: String,
    pub(crate) body: String,
}

impl Packet for MockMailEntry {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::ReceiveMail as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u64::<LittleEndian>(9100)?; // mail_id
        mir2_shared::binary::write_dotnet_string(writer, &self.sender)?;
        mir2_shared::binary::write_dotnet_string(writer, &self.subject)?;
        mir2_shared::binary::write_dotnet_string(writer, &self.body)?;
        writer.write_i64::<LittleEndian>(0)?; // timestamp
        writer.write_u8(0)?; // read
        writer.write_u8(0)?; // collected
        writer.write_u32::<LittleEndian>(0)?; // gold
        writer.write_u8(1)?; // item_count
        writer.write_u64::<LittleEndian>(9101)?; // uid
        writer.write_u32::<LittleEndian>(1)?; // idx
        mir2_shared::binary::write_dotnet_string(writer, "金创药(小)")?;
        writer.write_u16::<LittleEndian>(1)?; // count
        writer.write_u16::<LittleEndian>(1)?; // cd
        writer.write_u16::<LittleEndian>(1)?; // md
        Ok(())
    }
}

/// #788：拜师邀请（客户端格式 [name dotnet][level u16]）
pub(crate) struct MockMentorRequest;

impl Packet for MockMentorRequest {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::MentorRequest as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        mir2_shared::binary::write_dotnet_string(writer, "bevychar")?;
        writer.write_u16::<LittleEndian>(30)?;
        Ok(())
    }
}

/// #788：求婚邀请（客户端格式 [name dotnet]）
pub(crate) struct MockMarriageRequest;

impl Packet for MockMarriageRequest {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::MarriageRequest as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        mir2_shared::binary::write_dotnet_string(writer, "bevychar")?;
        Ok(())
    }
}

/// #788：离婚请求（空包）
pub(crate) struct MockDivorceRequest;

impl Packet for MockDivorceRequest {
    const OPCODE: i16 = mir2_shared::enums::ServerPacketIds::DivorceRequest as i16;

    fn read_body<R: std::io::Read>(_: &mut R) -> mir2_shared::data::stats::SharedResult<Self> {
        unreachable!("mock 只发送不解析")
    }

    fn write_body<W: std::io::Write>(&self, _writer: &mut W) -> mir2_shared::data::stats::SharedResult<()> {
        Ok(())
    }
}



