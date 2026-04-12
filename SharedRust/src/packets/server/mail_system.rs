// 邮件系统相关数据包
use super::super::base::Packet;
use crate::binary::read_dotnet_string;
use crate::data::item::UserItem;
use crate::data::stats::SharedResult;
use crate::enums::ServerPacketIds;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Read;

/// ReceiveMail - 接收邮件 (229)
#[derive(Debug, Clone)]
pub struct ReceiveMail {
    pub mail_list: Vec<MailInfo>,  // 邮件列表
}

#[derive(Debug, Clone)]
pub struct MailInfo {
    pub mail_id: u64,
    pub sender_name: String,
    pub message: String,
    pub opened: bool,
    pub locked: bool,
    pub can_reply: bool,
    pub collected: bool,
    pub send_date: i64,
    pub gold: u32,
    pub items: Vec<UserItem>,
}

impl Packet for ReceiveMail {
    const OPCODE: i16 = ServerPacketIds::ReceiveMail as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        use crate::binary::write_dotnet_string;

        writer.write_i32::<LittleEndian>(self.mail_list.len() as i32)?;

        for mail in &self.mail_list {
            writer.write_u64::<LittleEndian>(mail.mail_id)?;
            write_dotnet_string(writer, &mail.sender_name)?;
            write_dotnet_string(writer, &mail.message)?;
            writer.write_u8(mail.opened as u8)?;
            writer.write_u8(mail.locked as u8)?;
            writer.write_u8(mail.can_reply as u8)?;
            writer.write_u8(mail.collected as u8)?;
            writer.write_i64::<LittleEndian>(mail.send_date)?;
            writer.write_u32::<LittleEndian>(mail.gold)?;
            writer.write_i32::<LittleEndian>(mail.items.len() as i32)?;
            for item in &mail.items {
                item.write_to(writer)?;
            }
        }

        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()?;
        let mut mail_list = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let mail_id = reader.read_u64::<LittleEndian>()?;
            let sender_name = read_dotnet_string(reader)?;
            let message = read_dotnet_string(reader)?;
            let opened = reader.read_u8()? != 0;
            let locked = reader.read_u8()? != 0;
            let can_reply = reader.read_u8()? != 0;
            let collected = reader.read_u8()? != 0;
            let send_date = reader.read_i64::<LittleEndian>()?;
            let gold = reader.read_u32::<LittleEndian>()?;
            let item_count = reader.read_i32::<LittleEndian>()?;
            let mut items = Vec::with_capacity(item_count as usize);
            for _ in 0..item_count {
                items.push(UserItem::read_from(reader, i32::MAX, i32::MAX)?);
            }

            mail_list.push(MailInfo {
                mail_id,
                sender_name,
                message,
                opened,
                locked,
                can_reply,
                collected,
                send_date,
                gold,
                items,
            });
        }

        Ok(Self { mail_list })
    }
}

/// MailLockedItem - 邮件锁定物品 (230)
/// C# sends: UniqueID(u64), Locked(bool) — no index field.
#[derive(Debug, Clone)]
pub struct MailLockedItem {
    pub unique_id: u64,
    pub locked: bool,
}

impl Packet for MailLockedItem {
    const OPCODE: i16 = ServerPacketIds::MailLockedItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u8(self.locked as u8)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let locked = reader.read_u8()? != 0;
        Ok(Self { unique_id, locked })
    }
}

/// MailSendRequest - 邮件发送请求 (231)
#[derive(Debug, Clone)]
pub struct MailSendRequest {
    pub mail_id: u64,              // 邮件ID
}

impl Packet for MailSendRequest {
    const OPCODE: i16 = ServerPacketIds::MailSendRequest as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        writer.write_u64::<LittleEndian>(self.mail_id)?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let mail_id = reader.read_u64::<LittleEndian>()?;
        Ok(Self { mail_id })
    }
}

/// MailSent - 邮件已发送 (232)
/// C# sends only result(sbyte), no mail_id.
#[derive(Debug, Clone)]
pub struct MailSent {
    pub result: i8,
}

impl Packet for MailSent {
    const OPCODE: i16 = ServerPacketIds::MailSent as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_i8(self.result)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let result = reader.read_i8()?;
        Ok(Self { result })
    }
}

/// ParcelCollected - 包裹已收取 (233)
/// C# sends only success(bool), no mail_id.
#[derive(Debug, Clone)]
pub struct ParcelCollected {
    pub success: bool,
}

impl Packet for ParcelCollected {
    const OPCODE: i16 = ServerPacketIds::ParcelCollected as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_u8(self.success as u8)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let success = reader.read_u8()? != 0;
        Ok(Self { success })
    }
}

/// MailCost - 邮件费用 (234)
#[derive(Debug, Clone)]
pub struct MailCost {
    pub cost: u32,                  // 邮寄费用
}

impl Packet for MailCost {
    const OPCODE: i16 = ServerPacketIds::MailCost as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        writer.write_u32::<LittleEndian>(self.cost)?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let cost = reader.read_u32::<LittleEndian>()?;
        Ok(Self { cost })
    }
}
