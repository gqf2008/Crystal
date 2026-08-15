use mir2_shared::packets::base::Packet;


/// 商城购买（对齐 SharedRust GameshopBuy / C# C.GameshopBuy：[g_index i32][quantity u8][p_type i32]）
#[derive(Debug, Clone, Copy)]
pub struct GameshopBuyWire {
    pub g_index: i32,
    pub quantity: u8,
    pub p_type: i32,
}

impl Packet for GameshopBuyWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::GameshopBuy as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(Self {
            g_index: reader.read_i32::<LittleEndian>()?,
            quantity: reader.read_u8()?,
            p_type: reader.read_i32::<LittleEndian>()?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_i32::<LittleEndian>(self.g_index)?;
        writer.write_u8(self.quantity)?;
        writer.write_i32::<LittleEndian>(self.p_type)?;
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

/// 精炼存入（对齐 SharedRust DepositRefineItem / C# C.DepositRefineItem：[from i32][to i32]）
#[derive(Debug, Clone, Copy)]
pub struct RefineDepositWire {
    pub from: i32,
    pub to: i32,
}

impl Packet for RefineDepositWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::DepositRefineItem as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(Self {
            from: reader.read_i32::<LittleEndian>()?,
            to: reader.read_i32::<LittleEndian>()?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_i32::<LittleEndian>(self.from)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        Ok(())
    }
}

/// 合成请求（#2573：对齐 C# C.CraftItem wire
/// [UniqueID u64][Count u16][slots_len i32][slots i32×N]）
#[derive(Debug, Clone)]
pub struct CraftItemWire {
    pub unique_id: u64,
    pub count: u16,
    pub slots: Vec<i32>,
}

impl Packet for CraftItemWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::CraftItem as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let count = reader.read_u16::<LittleEndian>()?;
        let slots_len = reader.read_i32::<LittleEndian>()?;
        let mut slots = Vec::new();
        for _ in 0..slots_len.max(0).min(64) {
            slots.push(reader.read_i32::<LittleEndian>()?);
        }
        Ok(Self {
            unique_id,
            count,
            slots,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u16::<LittleEndian>(self.count)?;
        writer.write_i32::<LittleEndian>(self.slots.len() as i32)?;
        for s in &self.slots {
            writer.write_i32::<LittleEndian>(*s)?;
        }
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

/// 精炼取回（对齐 SharedRust RetrieveRefineItem / C# C.RetrieveRefineItem：[from i32][to i32]）
#[derive(Debug, Clone, Copy)]
pub struct RefineRetrieveWire {
    pub from: i32,
    pub to: i32,
}

impl Packet for RefineRetrieveWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::RetrieveRefineItem as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(Self {
            from: reader.read_i32::<LittleEndian>()?,
            to: reader.read_i32::<LittleEndian>()?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_i32::<LittleEndian>(self.from)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RefineItemWire {
    pub unique_id: u64,
}

impl Packet for RefineItemWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::RefineItem as i16;

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
/// 观察玩家（gate 解析 [name DotNetString]，对齐 SharedRust Observe / C# C.Observe）
#[derive(Debug, Clone, Default)]
pub struct ObserveWire {
    pub name: String,
}

impl Packet for ObserveWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::Observe as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        Ok(Self {
            name: mir2_shared::binary::read_dotnet_string(reader)?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        mir2_shared::binary::write_dotnet_string(writer, &self.name)?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_wire_roundtrip() {
        let w = ObserveWire { name: "测试玩家".to_string() };
        let mut buf = Vec::new();
        w.write_body(&mut buf).unwrap();
        // gate 解析 [name DotNetString]（对齐 SharedRust Observe / C# C.Observe）
        let mut cur = std::io::Cursor::new(buf);
        let r = ObserveWire::read_body(&mut cur).unwrap();
        assert_eq!(r.name, "测试玩家");
    }

    #[test]
    fn refine_deposit_wire_roundtrip() {
        let w = RefineDepositWire { from: 3, to: 0 };
        let mut buf = Vec::new();
        w.write_body(&mut buf).unwrap();
        // gate 解析 [from i32][to i32]（对齐 SharedRust DepositRefineItem / C#）
        assert_eq!(&buf[..4], 3i32.to_le_bytes());
        assert_eq!(&buf[4..], 0i32.to_le_bytes());
        let mut cur = std::io::Cursor::new(buf);
        let r = RefineDepositWire::read_body(&mut cur).unwrap();
        assert_eq!(r.from, 3);
        assert_eq!(r.to, 0);
    }

    #[test]
    fn refine_retrieve_wire_roundtrip() {
        let w = RefineRetrieveWire { from: 0, to: 5 };
        let mut buf = Vec::new();
        w.write_body(&mut buf).unwrap();
        let mut cur = std::io::Cursor::new(buf);
        let r = RefineRetrieveWire::read_body(&mut cur).unwrap();
        assert_eq!(r.from, 0);
        assert_eq!(r.to, 5);
    }
}
/// 大地图 NPC 搜索（gate 解析 [keyword: u16 len + bytes]，对齐 C# BigMapDialog SearchButton → C.SearchMap）
#[derive(Debug, Clone, Default)]
pub struct SearchMapWire {
    pub keyword: String,
}

impl Packet for SearchMapWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::SearchMap as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::ReadBytesExt;
        let len = reader.read_u16::<byteorder::LittleEndian>()? as usize;
        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf)?;
        Ok(Self {
            keyword: String::from_utf8_lossy(&buf).to_string(),
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::WriteBytesExt;
        let bytes = self.keyword.as_bytes();
        writer.write_u16::<byteorder::LittleEndian>(bytes.len() as u16)?;
        writer.write_all(bytes)?;
        Ok(())
    }
}
#[cfg(test)]
mod search_tests {
    use super::*;

    #[test]
    fn search_map_wire_roundtrip() {
        let w = SearchMapWire { keyword: "比奇".to_string() };
        let mut buf = Vec::new();
        w.write_body(&mut buf).unwrap();
        // gate 解析 [keyword: u16 len + bytes]
        let len = u16::from_le_bytes([buf[0], buf[1]]) as usize;
        assert_eq!(len, "比奇".len());
        assert_eq!(&buf[2..], "比奇".as_bytes());
        let mut cur = std::io::Cursor::new(buf);
        let r = SearchMapWire::read_body(&mut cur).unwrap();
        assert_eq!(r.keyword, "比奇");
    }

    #[test]
    fn search_map_wire_empty_keyword() {
        let w = SearchMapWire::default();
        let mut buf = Vec::new();
        w.write_body(&mut buf).unwrap();
        assert_eq!(buf, vec![0, 0]);
    }
}
