use mir2_shared::packets::base::Packet;


/// 商城购买（ServerRust gate 解析 [item_id u32][quantity u32]，与 SharedRust 结构不一致）
#[derive(Debug, Clone, Copy)]
pub struct GameshopBuyWire {
    pub item_id: u32,
    pub quantity: u32,
}

impl Packet for GameshopBuyWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::GameshopBuy as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(Self {
            item_id: reader.read_u32::<LittleEndian>()?,
            quantity: reader.read_u32::<LittleEndian>()?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u32::<LittleEndian>(self.item_id)?;
        writer.write_u32::<LittleEndian>(self.quantity)?;
        Ok(())
    }
}

/// 行会领地页请求（M36：gate 解析 [page u32]）
#[derive(Debug, Clone, Copy)]
pub struct GuildTerritoryPageWire {
    pub page: u32,
}

impl Packet for GuildTerritoryPageWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::GuildTerritoryPage as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(Self {
            page: reader.read_u32::<LittleEndian>()?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u32::<LittleEndian>(self.page)?;
        Ok(())
    }
}

/// 购买行会领地（M36：gate 解析 [territory_id u32]）
#[derive(Debug, Clone, Copy)]
pub struct PurchaseGuildTerritoryWire {
    pub territory_id: u32,
}

impl Packet for PurchaseGuildTerritoryWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::PurchaseGuildTerritory as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(Self {
            territory_id: reader.read_u32::<LittleEndian>()?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u32::<LittleEndian>(self.territory_id)?;
        Ok(())
    }
}

/// 钓鱼抛竿（M39：gate 解析 [fishing_type u8]）
#[derive(Debug, Clone, Copy)]
pub struct FishingCastWire {
    pub fishing_type: u8,
}

impl Packet for FishingCastWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::FishingCast as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::ReadBytesExt;
        Ok(Self {
            fishing_type: reader.read_u8()?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_u8(self.fishing_type)?;
        Ok(())
    }
}

/// 自动钓鱼开关（M39：gate 解析 [enabled u8]）
#[derive(Debug, Clone, Copy)]
pub struct FishingChangeAutocastWire {
    pub enabled: bool,
}

impl Packet for FishingChangeAutocastWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::FishingChangeAutocast as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::ReadBytesExt;
        Ok(Self {
            enabled: reader.read_u8()? != 0,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_u8(if self.enabled { 1 } else { 0 })?;
        Ok(())
    }
}

/// 精炼客户端包（M40：gate 实际 wire 与 SharedRust 结构不一致，手动构造）
#[derive(Debug, Clone, Copy)]
pub struct RefineDepositWire {
    pub unique_id: u64,
}

impl Packet for RefineDepositWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::DepositRefineItem as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(Self {
            unique_id: reader.read_u64::<LittleEndian>()?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        Ok(())
    }
}

/// 合成请求（M41：gate 解析 [recipe_id u32][materials_count u32]）
#[derive(Debug, Clone, Copy)]
pub struct CraftItemWire {
    pub recipe_id: u32,
    pub materials: u32,
}

impl Packet for CraftItemWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::CraftItem as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(Self {
            recipe_id: reader.read_u32::<LittleEndian>()?,
            materials: reader.read_u32::<LittleEndian>()?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u32::<LittleEndian>(self.recipe_id)?;
        writer.write_u32::<LittleEndian>(self.materials)?;
        Ok(())
    }
}

/// 物品租赁客户端包（M42：gate wire 与 SharedRust 不一致的手动构造）
#[derive(Debug, Clone)]
pub struct RentalRequestWire {
    pub target_name: String,
}

impl Packet for RentalRequestWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::ItemRentalRequest as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        Ok(Self {
            target_name: mir2_shared::binary::read_dotnet_string(reader)?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        mir2_shared::binary::write_dotnet_string(writer, &self.target_name)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RentalDepositWire {
    pub unique_id: u64,
}

impl Packet for RentalDepositWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::DepositRentalItem as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(Self {
            unique_id: reader.read_u64::<LittleEndian>()?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RentalRetrieveWire {
    pub unique_id: u64,
}

impl Packet for RentalRetrieveWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::RetrieveRentalItem as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(Self {
            unique_id: reader.read_u64::<LittleEndian>()?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RefineRetrieveWire {
    pub unique_id: u64,
}

impl Packet for RefineRetrieveWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::RetrieveRefineItem as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(Self {
            unique_id: reader.read_u64::<LittleEndian>()?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RefineItemWire {
    pub item_id: u32,
    pub materials: u32,
}

impl Packet for RefineItemWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::RefineItem as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(Self {
            item_id: reader.read_u32::<LittleEndian>()?,
            materials: reader.read_u32::<LittleEndian>()?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u32::<LittleEndian>(self.item_id)?;
        writer.write_u32::<LittleEndian>(self.materials)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RefineCheckWire {
    pub unique_id: u64,
}

impl Packet for RefineCheckWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::CheckRefine as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(Self {
            unique_id: reader.read_u64::<LittleEndian>()?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        Ok(())
    }

}
