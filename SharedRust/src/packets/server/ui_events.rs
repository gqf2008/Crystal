// UI事件和通知相关数据包
use super::super::base::Packet;
use crate::data::stats::SharedResult;
use crate::enums::ServerPacketIds;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Read;

/// ResizeInventory - 调整背包大小 (237)
#[derive(Debug, Clone)]
pub struct ResizeInventory {
    pub size: i32,                  // 新大小
}

impl Packet for ResizeInventory {
    const OPCODE: i16 = ServerPacketIds::ResizeInventory as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        writer.write_i32::<LittleEndian>(self.size)?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let size = reader.read_i32::<LittleEndian>()?;
        Ok(Self { size })
    }
}

/// ResizeStorage - 调整仓库大小 (236)
#[derive(Debug, Clone)]
pub struct ResizeStorage {
    pub size: i32,                  // 新大小
}

impl Packet for ResizeStorage {
    const OPCODE: i16 = ServerPacketIds::ResizeStorage as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        writer.write_i32::<LittleEndian>(self.size)?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let size = reader.read_i32::<LittleEndian>()?;
        Ok(Self { size })
    }
}

/// NewRecipeInfo - 新配方信息 (264)
#[derive(Debug, Clone)]
pub struct NewRecipeInfo {
    pub recipe_id: i32,             // 配方ID
}

impl Packet for NewRecipeInfo {
    const OPCODE: i16 = ServerPacketIds::NewRecipeInfo as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        writer.write_i32::<LittleEndian>(self.recipe_id)?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let recipe_id = reader.read_i32::<LittleEndian>()?;
        Ok(Self { recipe_id })
    }
}

/// OpenBrowser - 打开浏览器 (265)
#[derive(Debug, Clone)]
pub struct OpenBrowser {
    pub url: String,                // URL地址
}

impl Packet for OpenBrowser {
    const OPCODE: i16 = ServerPacketIds::OpenBrowser as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use crate::binary::write_dotnet_string;
        
        write_dotnet_string(writer, &self.url)?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use crate::binary::read_dotnet_string;
        let url = read_dotnet_string(reader)?;
        Ok(Self { url })
    }
}

/// PlaySound - 播放声音 (266)
#[derive(Debug, Clone)]
pub struct PlaySound {
    pub sound_id: i32,              // 声音ID
}

impl Packet for PlaySound {
    const OPCODE: i16 = ServerPacketIds::PlaySound as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        writer.write_i32::<LittleEndian>(self.sound_id)?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let sound_id = reader.read_i32::<LittleEndian>()?;
        Ok(Self { sound_id })
    }
}

/// SetTimer - 设置计时器 (267)
#[derive(Debug, Clone)]
pub struct SetTimer {
    pub timer_id: i32,              // 计时器ID
    pub seconds: i32,               // 秒数
}

impl Packet for SetTimer {
    const OPCODE: i16 = ServerPacketIds::SetTimer as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        writer.write_i32::<LittleEndian>(self.timer_id)?;
        writer.write_i32::<LittleEndian>(self.seconds)?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let timer_id = reader.read_i32::<LittleEndian>()?;
        let seconds = reader.read_i32::<LittleEndian>()?;
        Ok(Self { timer_id, seconds })
    }
}

/// ExpireTimer - 计时器过期 (268)
#[derive(Debug, Clone)]
pub struct ExpireTimer {
    pub timer_id: i32,              // 计时器ID
}

impl Packet for ExpireTimer {
    const OPCODE: i16 = ServerPacketIds::ExpireTimer as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        writer.write_i32::<LittleEndian>(self.timer_id)?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let timer_id = reader.read_i32::<LittleEndian>()?;
        Ok(Self { timer_id })
    }
}

/// UpdateNotice - 服务器公告 (271)（#256：C# S.UpdateNotice = Notice(Title, Message)）
#[derive(Debug, Clone)]
pub struct UpdateNotice {
    pub notice: crate::data::notice::Notice,
}

impl Packet for UpdateNotice {
    const OPCODE: i16 = ServerPacketIds::UpdateNotice as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        self.notice.write_to(writer)
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let notice = crate::data::notice::Notice::read_from(reader)?;
        Ok(Self { notice })
    }
}

/// Roll - 掷骰子 (270)
/// wire 对齐 C# ServerPackets.Roll：Type/Page/Result/AutoRoll
#[derive(Debug, Clone)]
pub struct Roll {
    /// 0=骰子（Prguse 281+result），1=尤茨（Items 2587+result）
    pub r#type: i32,
    /// 掷完后客户端回调的 NPC 页（C# RollDialog.ReturnResult → CallNPC "[page]"）
    pub page: String,
    /// 结果 1-6
    pub result: i32,
    /// 是否自动掷骰（true 到达即掷，false 需点击）
    pub auto_roll: bool,
}

impl Packet for Roll {
    const OPCODE: i16 = ServerPacketIds::Roll as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        use crate::binary::write_dotnet_string;

        writer.write_i32::<LittleEndian>(self.r#type)?;
        write_dotnet_string(writer, &self.page)?;
        writer.write_i32::<LittleEndian>(self.result)?;
        writer.write_u8(if self.auto_roll { 1 } else { 0 })?;

        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use crate::binary::read_dotnet_string;

        let r#type = reader.read_i32::<LittleEndian>()?;
        let page = read_dotnet_string(reader)?;
        let result = reader.read_i32::<LittleEndian>()?;
        let auto_roll = reader.read_u8()? != 0;
        Ok(Self { r#type, page, result, auto_roll })
    }
}


/// SetCompass - 设置指南针 (271)
#[derive(Debug, Clone)]
pub struct SetCompass {
    pub location: (i32, i32),       // 位置
}

impl Packet for SetCompass {
    const OPCODE: i16 = ServerPacketIds::SetCompass as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        writer.write_i32::<LittleEndian>(self.location.0)?;
        writer.write_i32::<LittleEndian>(self.location.1)?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let x = reader.read_i32::<LittleEndian>()?;
        let y = reader.read_i32::<LittleEndian>()?;
        Ok(Self { location: (x, y) })
    }
}

/// Opendoor - 打开门 (251)
#[derive(Debug, Clone)]
pub struct Opendoor {
    pub door_index: u8,             // 门索引
    pub close: bool,                // 是否关闭
}

impl Packet for Opendoor {
    const OPCODE: i16 = ServerPacketIds::Opendoor as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        writer.write_u8(self.door_index)?;
        writer.write_u8(if self.close { 1 } else { 0 })?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let door_index = reader.read_u8()?;
        let close = reader.read_u8()? != 0;
        Ok(Self { door_index, close })
    }
}

/// SendOutputMessage - 发送输出消息 (221)
#[derive(Debug, Clone)]
pub struct SendOutputMessage {
    pub message: String,            // 消息内容
    pub message_type: u8,           // 消息类型
}

impl Packet for SendOutputMessage {
    const OPCODE: i16 = ServerPacketIds::SendOutputMessage as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use crate::binary::write_dotnet_string;
        use byteorder::WriteBytesExt;

        write_dotnet_string(writer, &self.message)?;
        writer.write_u8(self.message_type)?;

        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use crate::binary::read_dotnet_string;

        let message = read_dotnet_string(reader)?;
        let message_type = reader.read_u8()?;
        Ok(Self {
            message,
            message_type,
        })
    }
}

/// SetBindingShot - 设置捆绑射击 (220)
#[derive(Debug, Clone)]
pub struct SetBindingShot {
    pub enabled: bool,              // 是否启用
}

impl Packet for SetBindingShot {
    const OPCODE: i16 = ServerPacketIds::SetBindingShot as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        writer.write_u8(if self.enabled { 1 } else { 0 })?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let enabled = reader.read_u8()? != 0;
        Ok(Self { enabled })
    }
}

/// RemoveDelayedExplosion - 移除延迟爆炸 (216)
#[derive(Debug, Clone)]
pub struct RemoveDelayedExplosion {
    pub object_id: u32,             // 对象ID
}

impl Packet for RemoveDelayedExplosion {
    const OPCODE: i16 = ServerPacketIds::RemoveDelayedExplosion as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        writer.write_u32::<LittleEndian>(self.object_id)?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        Ok(Self { object_id })
    }
}
