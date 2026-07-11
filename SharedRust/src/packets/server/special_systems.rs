// 特殊系统相关数据包（智能生物、游戏商店、排名、公会领地等）
use super::super::base::Packet;
use crate::data::stats::SharedResult;
use crate::enums::{IntelligentCreatureType, ServerPacketIds};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Read;

/// NewIntelligentCreature - 新智能生物 (237)
#[derive(Debug, Clone)]
pub struct NewIntelligentCreature {
    pub creature_type: IntelligentCreatureType, // 生物类型
}

impl Packet for NewIntelligentCreature {
    const OPCODE: i16 = ServerPacketIds::NewIntelligentCreature as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        // C# saves entire ClientIntelligentCreature, we only have type
        writer.write_u8(self.creature_type as u8)?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let creature_type = IntelligentCreatureType::try_from(reader.read_u8()?)?;
        Ok(Self { creature_type })
    }
}

/// UpdateIntelligentCreatureList - 更新智能生物列表 (238)
#[derive(Debug, Clone)]
pub struct UpdateIntelligentCreatureList {
    pub creatures: Vec<IntelligentCreatureInfo>,
}

#[derive(Debug, Clone)]
pub struct IntelligentCreatureInfo {
    pub creature_id: i32,           // 生物ID
    pub creature_type: IntelligentCreatureType, // 生物类型
    pub custom_name: String,        // 自定义名称
    pub petmode: u8,                // 宠物模式
    pub exp: i64,                   // 经验值
    pub level: i32,                 // 等级
    pub slot_index: i32,            // 槽位索引
}

impl Packet for UpdateIntelligentCreatureList {
    const OPCODE: i16 = ServerPacketIds::UpdateIntelligentCreatureList as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        use crate::binary::write_dotnet_string;
        
        writer.write_i32::<LittleEndian>(self.creatures.len() as i32)?;
        
        for creature in &self.creatures {
            writer.write_i32::<LittleEndian>(creature.creature_id)?;
            writer.write_u8(creature.creature_type as u8)?;
            write_dotnet_string(writer, &creature.custom_name)?;
            writer.write_u8(creature.petmode)?;
            writer.write_i64::<LittleEndian>(creature.exp)?;
            writer.write_i32::<LittleEndian>(creature.level)?;
            writer.write_i32::<LittleEndian>(creature.slot_index)?;
        }
        
        // C# also writes: CreatureSummoned, SummonedCreatureType, PearlCount
        // But Rust struct doesn't have these, so we skip them or write defaults
        writer.write_u8(0)?; // CreatureSummoned = false
        writer.write_u8(0)?; // SummonedCreatureType = None
        writer.write_i32::<LittleEndian>(0)?; // PearlCount = 0
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()?;
        let mut creatures = Vec::with_capacity(count as usize);
        
        for _ in 0..count {
            let creature_id = reader.read_i32::<LittleEndian>()?;
            let creature_type = IntelligentCreatureType::try_from(reader.read_u8()?)?;
            
            let name_len = reader.read_i32::<LittleEndian>()?;
            let mut name_bytes = vec![0u8; name_len as usize];
            reader.read_exact(&mut name_bytes)?;
            let custom_name = String::from_utf8_lossy(&name_bytes).to_string();
            
            let petmode = reader.read_u8()?;
            let exp = reader.read_i64::<LittleEndian>()?;
            let level = reader.read_i32::<LittleEndian>()?;
            let slot_index = reader.read_i32::<LittleEndian>()?;
            
            creatures.push(IntelligentCreatureInfo {
                creature_id,
                creature_type,
                custom_name,
                petmode,
                exp,
                level,
                slot_index,
            });
        }
        
        Ok(Self { creatures })
    }
}

/// IntelligentCreatureEnableRename - 智能生物启用重命名 (239)
#[derive(Debug, Clone)]
pub struct IntelligentCreatureEnableRename {
    pub can_rename: bool,           // 是否可以重命名
}

impl Packet for IntelligentCreatureEnableRename {
    const OPCODE: i16 = ServerPacketIds::IntelligentCreatureEnableRename as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        // Note: C# is empty, but Rust has can_rename field
        writer.write_u8(if self.can_rename { 1 } else { 0 })?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let can_rename = reader.read_u8()? != 0;
        Ok(Self { can_rename })
    }
}

/// IntelligentCreaturePickup - 智能生物拾取 (240)
#[derive(Debug, Clone)]
pub struct IntelligentCreaturePickup {
    pub enabled: bool,              // 是否启用
}

impl Packet for IntelligentCreaturePickup {
    const OPCODE: i16 = ServerPacketIds::IntelligentCreaturePickup as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        // Note: C# uses ObjectID(u32), Rust uses enabled(bool)
        writer.write_u8(if self.enabled { 1 } else { 0 })?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let enabled = reader.read_u8()? != 0;
        Ok(Self { enabled })
    }
}

/// NPCPearlGoods - NPC珍珠商品 (241)
#[derive(Debug, Clone)]
pub struct NPCPearlGoods {
    pub rate: i32,                  // 汇率
    pub item_list: Vec<i32>,        // 物品列表
}

impl Packet for NPCPearlGoods {
    const OPCODE: i16 = ServerPacketIds::NPCPearlGoods as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        // Note: C# writes List<UserItem> + Rate(f32) + Type(u8)
        // Rust has rate(i32) + item_list(Vec<i32>)
        writer.write_i32::<LittleEndian>(self.item_list.len() as i32)?;
        
        for &item_id in &self.item_list {
            writer.write_i32::<LittleEndian>(item_id)?;
        }
        
        writer.write_i32::<LittleEndian>(self.rate)?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let rate = reader.read_i32::<LittleEndian>()?;
        let count = reader.read_i32::<LittleEndian>()?;
        let mut item_list = Vec::with_capacity(count as usize);
        
        for _ in 0..count {
            item_list.push(reader.read_i32::<LittleEndian>()?);
        }
        
        Ok(Self { rate, item_list })
    }
}

/// GuildBuffList - 公会Buff列表 (246)
#[derive(Debug, Clone)]
pub struct GuildBuffList {
    pub active_buffs: Vec<i32>,     // 激活的Buff列表
}

impl Packet for GuildBuffList {
    const OPCODE: i16 = ServerPacketIds::GuildBuffList as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        // Note: C# has Remove(u8) + ActiveBuffs + GuildBuffs
        // Rust only has active_buffs(Vec<i32>)
        writer.write_u8(0)?; // Remove = 0
        writer.write_i32::<LittleEndian>(self.active_buffs.len() as i32)?;
        
        for &buff_id in &self.active_buffs {
            writer.write_i32::<LittleEndian>(buff_id)?;
        }
        
        // GuildBuffs list (empty in Rust)
        writer.write_i32::<LittleEndian>(0)?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()?;
        let mut active_buffs = Vec::with_capacity(count as usize);
        
        for _ in 0..count {
            active_buffs.push(reader.read_i32::<LittleEndian>()?);
        }
        
        Ok(Self { active_buffs })
    }
}

// Note: NPCRequestInput has been moved to npc.rs to avoid duplication

/// GameShopInfo - 游戏商店信息 (248)
#[derive(Debug, Clone)]
pub struct GameShopInfo {
    pub items: Vec<GameShopItem>,   // 商品列表
    pub credit: u32,                // 点券
    pub gold: u32,                  // 金币
}

#[derive(Debug, Clone)]
pub struct GameShopItem {
    pub item_index: i32,            // 物品索引
    pub gold_price: u32,            // 金币价格
    pub credit_price: u32,          // 点券价格
    pub count: i32,                 // 数量
    pub class: u8,                  // 职业
    pub category: String,           // 分类
    pub stock: i32,                 // 库存
    pub is_bought: bool,            // 是否已购买
    pub deal: bool,                 // 是否特价
}

impl Packet for GameShopInfo {
    const OPCODE: i16 = ServerPacketIds::GameShopInfo as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        use crate::binary::write_dotnet_string;
        
        // Note: C# has Item(GameShopItem) + StockLevel(i32)
        // Rust has items(Vec<GameShopItem>) + credit + gold
        writer.write_i32::<LittleEndian>(self.items.len() as i32)?;
        
        for item in &self.items {
            writer.write_i32::<LittleEndian>(item.item_index)?;
            writer.write_u32::<LittleEndian>(item.gold_price)?;
            writer.write_u32::<LittleEndian>(item.credit_price)?;
            writer.write_i32::<LittleEndian>(item.count)?;
            writer.write_u8(item.class)?;
            write_dotnet_string(writer, &item.category)?;
            writer.write_i32::<LittleEndian>(item.stock)?;
            writer.write_u8(if item.is_bought { 1 } else { 0 })?;
            writer.write_u8(if item.deal { 1 } else { 0 })?;
        }
        
        writer.write_u32::<LittleEndian>(self.credit)?;
        writer.write_u32::<LittleEndian>(self.gold)?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()?;
        let mut items = Vec::with_capacity(count as usize);
        
        for _ in 0..count {
            let item_index = reader.read_i32::<LittleEndian>()?;
            let gold_price = reader.read_u32::<LittleEndian>()?;
            let credit_price = reader.read_u32::<LittleEndian>()?;
            let item_count = reader.read_i32::<LittleEndian>()?;
            let class = reader.read_u8()?;
            
            let cat_len = reader.read_i32::<LittleEndian>()?;
            let mut cat_bytes = vec![0u8; cat_len as usize];
            reader.read_exact(&mut cat_bytes)?;
            let category = String::from_utf8_lossy(&cat_bytes).to_string();
            
            let stock = reader.read_i32::<LittleEndian>()?;
            let is_bought = reader.read_u8()? != 0;
            let deal = reader.read_u8()? != 0;
            
            items.push(GameShopItem {
                item_index,
                gold_price,
                credit_price,
                count: item_count,
                class,
                category,
                stock,
                is_bought,
                deal,
            });
        }
        
        let credit = reader.read_u32::<LittleEndian>()?;
        let gold = reader.read_u32::<LittleEndian>()?;
        
        Ok(Self {
            items,
            credit,
            gold,
        })
    }
}

/// GameShopStock - 游戏商店库存 (249)
#[derive(Debug, Clone)]
pub struct GameShopStock {
    pub item_index: i32,            // 物品索引
    pub stock: i32,                 // 库存数量
}

impl Packet for GameShopStock {
    const OPCODE: i16 = ServerPacketIds::GameShopStock as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        writer.write_i32::<LittleEndian>(self.item_index)?;
        writer.write_i32::<LittleEndian>(self.stock)?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let item_index = reader.read_i32::<LittleEndian>()?;
        let stock = reader.read_i32::<LittleEndian>()?;
        Ok(Self { item_index, stock })
    }
}

/// Rankings - 排名榜 (250)
#[derive(Debug, Clone)]
pub struct Rankings {
    pub rankings: Vec<RankInfo>,    // 排名列表
}

#[derive(Debug, Clone)]
pub struct RankInfo {
    pub rank: i32,                  // 排名
    pub player_name: String,        // 玩家名称
    pub class: u8,                  // 职业
    pub level: i32,                 // 等级
    pub experience: i64,            // 经验值
}

impl Packet for Rankings {
    const OPCODE: i16 = ServerPacketIds::Rankings as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        use crate::binary::write_dotnet_string;
        
        // Note: C# has RankType(u8) + MyRank(i32) + ListingDetails + Listings(Vec<i64>) + Count
        // Rust only has rankings(Vec<RankInfo>)
        writer.write_u8(0)?; // RankType = 0
        writer.write_i32::<LittleEndian>(0)?; // MyRank = 0
        
        writer.write_i32::<LittleEndian>(self.rankings.len() as i32)?;
        
        for rank in &self.rankings {
            writer.write_i32::<LittleEndian>(rank.rank)?;
            write_dotnet_string(writer, &rank.player_name)?;
            writer.write_u8(rank.class)?;
            writer.write_i32::<LittleEndian>(rank.level)?;
            writer.write_i64::<LittleEndian>(rank.experience)?;
        }
        
        // Listings(Vec<i64>)
        writer.write_i32::<LittleEndian>(0)?;
        
        // Count
        writer.write_i32::<LittleEndian>(self.rankings.len() as i32)?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()?;
        let mut rankings = Vec::with_capacity(count as usize);
        
        for _ in 0..count {
            let rank = reader.read_i32::<LittleEndian>()?;
            
            let name_len = reader.read_i32::<LittleEndian>()?;
            let mut name_bytes = vec![0u8; name_len as usize];
            reader.read_exact(&mut name_bytes)?;
            let player_name = String::from_utf8_lossy(&name_bytes).to_string();
            
            let class = reader.read_u8()?;
            let level = reader.read_i32::<LittleEndian>()?;
            let experience = reader.read_i64::<LittleEndian>()?;
            
            rankings.push(RankInfo {
                rank,
                player_name,
                class,
                level,
                experience,
            });
        }
        
        Ok(Self { rankings })
    }
}

/// GuildTerritoryPage - 公会领地页面 (274)
#[derive(Debug, Clone)]
pub struct GuildTerritoryPage {
    pub territories: Vec<TerritoryInfo>,
}

#[derive(Debug, Clone)]
pub struct TerritoryInfo {
    pub index: i32,                 // 索引
    pub name: String,               // 名称
    pub owner_guild: String,        // 拥有公会
    pub start_time: i64,            // 开始时间
    pub end_time: i64,              // 结束时间
    pub war_fee: u32,               // 战争费用
}

impl Packet for GuildTerritoryPage {
    const OPCODE: i16 = ServerPacketIds::GuildTerritoryPage as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        use crate::binary::write_dotnet_string;
        
        // C# writes: length(i32) + Count + List[i].Save()
        writer.write_i32::<LittleEndian>(self.territories.len() as i32)?; // length
        writer.write_i32::<LittleEndian>(self.territories.len() as i32)?; // count
        
        for territory in &self.territories {
            writer.write_i32::<LittleEndian>(territory.index)?;
            write_dotnet_string(writer, &territory.name)?;
            write_dotnet_string(writer, &territory.owner_guild)?;
            writer.write_i64::<LittleEndian>(territory.start_time)?;
            writer.write_i64::<LittleEndian>(territory.end_time)?;
            writer.write_u32::<LittleEndian>(territory.war_fee)?;
        }
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()?;
        let mut territories = Vec::with_capacity(count as usize);
        
        for _ in 0..count {
            let index = reader.read_i32::<LittleEndian>()?;
            
            let name_len = reader.read_i32::<LittleEndian>()?;
            let mut name_bytes = vec![0u8; name_len as usize];
            reader.read_exact(&mut name_bytes)?;
            let name = String::from_utf8_lossy(&name_bytes).to_string();
            
            let guild_len = reader.read_i32::<LittleEndian>()?;
            let mut guild_bytes = vec![0u8; guild_len as usize];
            reader.read_exact(&mut guild_bytes)?;
            let owner_guild = String::from_utf8_lossy(&guild_bytes).to_string();
            
            let start_time = reader.read_i64::<LittleEndian>()?;
            let end_time = reader.read_i64::<LittleEndian>()?;
            let war_fee = reader.read_u32::<LittleEndian>()?;
            
            territories.push(TerritoryInfo {
                index,
                name,
                owner_guild,
                start_time,
                end_time,
                war_fee,
            });
        }
        
        Ok(Self { territories })
    }
}

// 注：`PurchaseGuildTerritory` 在 C# 中仅作为 ClientPacket 存在（客户端→服务端请求），
// 服务端从不回送此 opcode。Rust 端此前凭空定义了一个 server::PurchaseGuildTerritory
// 并占用了 opcode 277，挤掉了本应是 277 的 StorageUnlockResult。现已移除该伪服务端包，
// StorageUnlockResult/StoragePasswordResult 恢复为 277/278。
// 客户端发起购买仍走 client::guild::PurchaseGuildTerritory（ClientPacketIds）。
