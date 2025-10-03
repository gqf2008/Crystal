// 邮件系统相关数据包
use super::super::base::Packet;
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
    pub mail_id: u64,              // 邮件ID
    pub sender_name: String,        // 发件人
    pub mail_subject: String,       // 邮件主题
    pub message: String,            // 邮件内容
    pub gold: u32,                  // 金币数量
    pub items: Vec<UserItem>,       // 附件物品列表
    pub locked: bool,               // 是否锁定
    pub collected: bool,            // 是否已收取
    pub send_date: i64,             // 发送日期
}

impl Packet for ReceiveMail {
    const OPCODE: i16 = ServerPacketIds::ReceiveMail as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        use crate::binary::write_dotnet_string;
        
        writer.write_i32::<LittleEndian>(self.mail_list.len() as i32)?;
        
        for mail in &self.mail_list {
            writer.write_u64::<LittleEndian>(mail.mail_id)?;
            write_dotnet_string(writer, &mail.sender_name)?;
            write_dotnet_string(writer, &mail.mail_subject)?;
            write_dotnet_string(writer, &mail.message)?;
            writer.write_u32::<LittleEndian>(mail.gold)?;
            
            writer.write_i32::<LittleEndian>(mail.items.len() as i32)?;
            for item in &mail.items {
                item.write_to(writer)?;
            }
            
            writer.write_u8(if mail.locked { 1 } else { 0 })?;
            writer.write_u8(if mail.collected { 1 } else { 0 })?;
            writer.write_i64::<LittleEndian>(mail.send_date)?;
        }
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()?;
        let mut mail_list = Vec::with_capacity(count as usize);
        
        for _ in 0..count {
            let mail_id = reader.read_u64::<LittleEndian>()?;
            
            let sender_len = reader.read_i32::<LittleEndian>()?;
            let mut sender_bytes = vec![0u8; sender_len as usize];
            reader.read_exact(&mut sender_bytes)?;
            let sender_name = String::from_utf8_lossy(&sender_bytes).to_string();
            
            let subject_len = reader.read_i32::<LittleEndian>()?;
            let mut subject_bytes = vec![0u8; subject_len as usize];
            reader.read_exact(&mut subject_bytes)?;
            let mail_subject = String::from_utf8_lossy(&subject_bytes).to_string();
            
            let message_len = reader.read_i32::<LittleEndian>()?;
            let mut message_bytes = vec![0u8; message_len as usize];
            reader.read_exact(&mut message_bytes)?;
            let message = String::from_utf8_lossy(&message_bytes).to_string();
            
            let gold = reader.read_u32::<LittleEndian>()?;
            
            let item_count = reader.read_i32::<LittleEndian>()?;
            let mut items = Vec::with_capacity(item_count as usize);
            for _ in 0..item_count {
                items.push(UserItem::read_from(reader, i32::MAX, i32::MAX)?);
            }
            
            let locked = reader.read_u8()? != 0;
            let collected = reader.read_u8()? != 0;
            let send_date = reader.read_i64::<LittleEndian>()?;
            
            mail_list.push(MailInfo {
                mail_id,
                sender_name,
                mail_subject,
                message,
                gold,
                items,
                locked,
                collected,
                send_date,
            });
        }
        
        Ok(Self { mail_list })
    }
}

/// MailLockedItem - 邮件锁定物品 (230)
#[derive(Debug, Clone)]
pub struct MailLockedItem {
    pub mail_id: u64,              // 邮件ID
    pub index: i32,                 // 物品索引
    pub locked: bool,               // 是否锁定
}

impl Packet for MailLockedItem {
    const OPCODE: i16 = ServerPacketIds::MailLockedItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        writer.write_u64::<LittleEndian>(self.mail_id)?;
        writer.write_i32::<LittleEndian>(self.index)?;
        writer.write_u8(if self.locked { 1 } else { 0 })?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let mail_id = reader.read_u64::<LittleEndian>()?;
        let index = reader.read_i32::<LittleEndian>()?;
        let locked = reader.read_u8()? != 0;
        Ok(Self {
            mail_id,
            index,
            locked,
        })
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
#[derive(Debug, Clone)]
pub struct MailSent {
    pub mail_id: u64,              // 邮件ID
    pub result: u8,                 // 发送结果
}

impl Packet for MailSent {
    const OPCODE: i16 = ServerPacketIds::MailSent as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        writer.write_u64::<LittleEndian>(self.mail_id)?;
        writer.write_u8(self.result)?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let mail_id = reader.read_u64::<LittleEndian>()?;
        let result = reader.read_u8()?;
        Ok(Self { mail_id, result })
    }
}

/// ParcelCollected - 包裹已收取 (233)
#[derive(Debug, Clone)]
pub struct ParcelCollected {
    pub mail_id: u64,              // 邮件ID
    pub success: bool,              // 是否成功
}

impl Packet for ParcelCollected {
    const OPCODE: i16 = ServerPacketIds::ParcelCollected as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        writer.write_u64::<LittleEndian>(self.mail_id)?;
        writer.write_u8(if self.success { 1 } else { 0 })?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let mail_id = reader.read_u64::<LittleEndian>()?;
        let success = reader.read_u8()? != 0;
        Ok(Self { mail_id, success })
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
