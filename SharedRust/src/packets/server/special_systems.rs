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

    fn write_body<W: std::io::Write>(&self, _writer: &mut W) -> SharedResult<()> {
        unimplemented!("Server packets don't need write_body")
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

    fn write_body<W: std::io::Write>(&self, _writer: &mut W) -> SharedResult<()> {
        unimplemented!("Server packets don't need write_body")
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

    fn write_body<W: std::io::Write>(&self, _writer: &mut W) -> SharedResult<()> {
        unimplemented!("Server packets don't need write_body")
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

    fn write_body<W: std::io::Write>(&self, _writer: &mut W) -> SharedResult<()> {
        unimplemented!("Server packets don't need write_body")
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

    fn write_body<W: std::io::Write>(&self, _writer: &mut W) -> SharedResult<()> {
        unimplemented!("Server packets don't need write_body")
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

    fn write_body<W: std::io::Write>(&self, _writer: &mut W) -> SharedResult<()> {
        unimplemented!("Server packets don't need write_body")
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

/// NPCRequestInput - NPC请求输入 (247)
#[derive(Debug, Clone)]
pub struct NPCRequestInput {
    pub message: String,            // 提示消息
    pub max_length: i32,            // 最大长度
}

impl Packet for NPCRequestInput {
    const OPCODE: i16 = ServerPacketIds::NPCRequestInput as i16;

    fn write_body<W: std::io::Write>(&self, _writer: &mut W) -> SharedResult<()> {
        unimplemented!("Server packets don't need write_body")
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let len = reader.read_i32::<LittleEndian>()?;
        let mut bytes = vec![0u8; len as usize];
        reader.read_exact(&mut bytes)?;
        let message = String::from_utf8_lossy(&bytes).to_string();
        let max_length = reader.read_i32::<LittleEndian>()?;
        Ok(Self {
            message,
            max_length,
        })
    }
}

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

    fn write_body<W: std::io::Write>(&self, _writer: &mut W) -> SharedResult<()> {
        unimplemented!("Server packets don't need write_body")
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

    fn write_body<W: std::io::Write>(&self, _writer: &mut W) -> SharedResult<()> {
        unimplemented!("Server packets don't need write_body")
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

    fn write_body<W: std::io::Write>(&self, _writer: &mut W) -> SharedResult<()> {
        unimplemented!("Server packets don't need write_body")
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

    fn write_body<W: std::io::Write>(&self, _writer: &mut W) -> SharedResult<()> {
        unimplemented!("Server packets don't need write_body")
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

/// PurchaseGuildTerritory - 购买公会领地 (275)
#[derive(Debug, Clone)]
pub struct PurchaseGuildTerritory {
    pub success: bool,              // 是否成功
}

impl Packet for PurchaseGuildTerritory {
    const OPCODE: i16 = ServerPacketIds::PurchaseGuildTerritory as i16;

    fn write_body<W: std::io::Write>(&self, _writer: &mut W) -> SharedResult<()> {
        unimplemented!("Server packets don't need write_body")
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let success = reader.read_u8()? != 0;
        Ok(Self { success })
    }
}
