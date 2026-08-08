// 社交系统相关数据包（好友、恋人、导师等）
use super::super::base::Packet;
use crate::data::stats::SharedResult;
use crate::enums::ServerPacketIds;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Read;

/// TransformUpdate - 变身更新 (242)
#[derive(Debug, Clone)]
pub struct TransformUpdate {
    pub object_id: u32,     // 对象ID
    pub transform_type: u8, // 变身类型
}

impl Packet for TransformUpdate {
    const OPCODE: i16 = ServerPacketIds::TransformUpdate as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;

        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u8(self.transform_type)?;

        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let transform_type = reader.read_u8()?;
        Ok(Self {
            object_id,
            transform_type,
        })
    }
}

/// FriendUpdate - 好友更新 (243)
#[derive(Debug, Clone)]
pub struct FriendUpdate {
    pub friends: Vec<FriendInfo>, // 好友列表
}

#[derive(Debug, Clone)]
pub struct FriendInfo {
    pub object_id: u32, // 好友ID
    pub name: String,   // 好友名称
    pub memo: String,   // 备注
    /// 是否黑名单（C# ClientFriend.Blocked）
    pub blocked: bool,
    pub online: bool,   // 是否在线
}

impl Packet for FriendUpdate {
    const OPCODE: i16 = ServerPacketIds::FriendUpdate as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use crate::binary::write_dotnet_string;
        use byteorder::WriteBytesExt;

        writer.write_i32::<LittleEndian>(self.friends.len() as i32)?;

        for friend in &self.friends {
            writer.write_u32::<LittleEndian>(friend.object_id)?;
            write_dotnet_string(writer, &friend.name)?;
            write_dotnet_string(writer, &friend.memo)?;
            writer.write_u8(if friend.blocked { 1 } else { 0 })?;
            writer.write_u8(if friend.online { 1 } else { 0 })?;
        }

        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use crate::binary::read_dotnet_string;
        let count = reader.read_i32::<LittleEndian>()?;
        let mut friends = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let object_id = reader.read_u32::<LittleEndian>()?;

            let name = read_dotnet_string(reader)?;

            let memo = read_dotnet_string(reader)?;
            let blocked = reader.read_u8()? != 0;

            let online = reader.read_u8()? != 0;

            friends.push(FriendInfo {
                object_id,
                name,
                memo,
                blocked,
                online,
            });
        }

        Ok(Self { friends })
    }
}

/// LoverUpdate - 恋人更新 (246)
/// #1329：对齐 C# `S.LoverUpdate`（ServerPackets.cs）：[Name dotnet][Date i64 ToBinary][MapName dotnet][MarriedDays i16]
#[derive(Debug, Clone, PartialEq)]
pub struct LoverUpdate {
    pub lover_name: String, // 恋人名称（未结婚/离婚后为空串）
    pub date: i64,          // 结婚日期（C# DateTime.ToBinary；Rust 存 unix 秒，客户端仅用于计算天数）
    pub map_name: String,   // 配偶当前地图标题（离线为空串）
    pub married_days: i16,  // 结婚天数
}

impl Packet for LoverUpdate {
    const OPCODE: i16 = ServerPacketIds::LoverUpdate as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use crate::binary::write_dotnet_string;
        use byteorder::WriteBytesExt;

        write_dotnet_string(writer, &self.lover_name)?;
        writer.write_i64::<LittleEndian>(self.date)?;
        write_dotnet_string(writer, &self.map_name)?;
        writer.write_i16::<LittleEndian>(self.married_days)?;

        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use crate::binary::read_dotnet_string;
        let lover_name = read_dotnet_string(reader)?;

        let date = reader.read_i64::<LittleEndian>()?;

        let map_name = read_dotnet_string(reader)?;

        let married_days = reader.read_i16::<LittleEndian>()?;

        Ok(Self {
            lover_name,
            date,
            map_name,
            married_days,
        })
    }
}

/// MentorUpdate - 导师更新 (245)
#[derive(Debug, Clone)]
pub struct MentorUpdate {
    pub mentor_name: String, // 导师名称
    pub mentor_level: i32,   // 导师等级
    pub mentor_online: bool, // 导师是否在线
}

impl Packet for MentorUpdate {
    const OPCODE: i16 = ServerPacketIds::MentorUpdate as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use crate::binary::write_dotnet_string;
        use byteorder::WriteBytesExt;

        write_dotnet_string(writer, &self.mentor_name)?;
        writer.write_i32::<LittleEndian>(self.mentor_level)?;
        writer.write_u8(if self.mentor_online { 1 } else { 0 })?;

        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use crate::binary::read_dotnet_string;
        let mentor_name = read_dotnet_string(reader)?;

        let mentor_level = reader.read_i32::<LittleEndian>()?;
        let mentor_online = reader.read_u8()? != 0;

        Ok(Self {
            mentor_name,
            mentor_level,
            mentor_online,
        })
    }
}

/// MarriageRequest - 结婚请求 (187)
#[derive(Debug, Clone)]
pub struct MarriageRequest {
    pub lover_name: String, // 恋人名称
}

impl Packet for MarriageRequest {
    const OPCODE: i16 = ServerPacketIds::MarriageRequest as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use crate::binary::write_dotnet_string;

        write_dotnet_string(writer, &self.lover_name)?;

        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use crate::binary::read_dotnet_string;
        let lover_name = read_dotnet_string(reader)?;
        Ok(Self { lover_name })
    }
}

/// DivorceRequest - 离婚请求 (188)
#[derive(Debug, Clone)]
pub struct DivorceRequest {
    pub lover_name: String, // 恋人名称
}

impl Packet for DivorceRequest {
    const OPCODE: i16 = ServerPacketIds::DivorceRequest as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use crate::binary::write_dotnet_string;

        write_dotnet_string(writer, &self.lover_name)?;

        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use crate::binary::read_dotnet_string;
        let lover_name = read_dotnet_string(reader)?;
        Ok(Self { lover_name })
    }
}

/// MentorRequest - 导师请求 (189)
#[derive(Debug, Clone)]
pub struct MentorRequest {
    pub mentor_name: String, // 导师名称
}

impl Packet for MentorRequest {
    const OPCODE: i16 = ServerPacketIds::MentorRequest as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use crate::binary::write_dotnet_string;

        write_dotnet_string(writer, &self.mentor_name)?;

        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use crate::binary::read_dotnet_string;
        let mentor_name = read_dotnet_string(reader)?;
        Ok(Self { mentor_name })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packets::base::{deserialize_packet, serialize_packet};
    use std::io::Cursor;

    #[test]
    fn lover_update_roundtrip() {
        let pkt = LoverUpdate {
            lover_name: "bevy2char".to_string(),
            date: 1_700_000_000,
            map_name: "盟重省".to_string(),
            married_days: 3,
        };
        let mut buf = Vec::new();
        serialize_packet(&mut buf, &pkt).unwrap();
        let mut cur = Cursor::new(&buf);
        let got = deserialize_packet::<_, LoverUpdate>(&mut cur).unwrap();
        assert_eq!(got, pkt);
    }

    #[test]
    fn lover_update_empty_roundtrip() {
        let pkt = LoverUpdate {
            lover_name: String::new(),
            date: 0,
            map_name: String::new(),
            married_days: 0,
        };
        let mut buf = Vec::new();
        serialize_packet(&mut buf, &pkt).unwrap();
        let mut cur = Cursor::new(&buf);
        let got = deserialize_packet::<_, LoverUpdate>(&mut cur).unwrap();
        assert_eq!(got, pkt);
    }
}
