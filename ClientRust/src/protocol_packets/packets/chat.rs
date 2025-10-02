// 聊天系统数据包解析
// Chat System Packet Parsing

use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

// ChatType 枚举需要从主模块导入,或在这里定义
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatType {
    Normal = 0,
    Whisper = 1,
    Group = 2,
    Guild = 3,
    Shout = 4,
    System = 5,
    Announcement = 6,
    LineMessage = 7,
    Mentor = 8,
}

impl TryFrom<u8> for ChatType {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ChatType::Normal),
            1 => Ok(ChatType::Whisper),
            2 => Ok(ChatType::Group),
            3 => Ok(ChatType::Guild),
            4 => Ok(ChatType::Shout),
            5 => Ok(ChatType::System),
            6 => Ok(ChatType::Announcement),
            7 => Ok(ChatType::LineMessage),
            8 => Ok(ChatType::Mentor),
            _ => Err(format!("Unknown ChatType: {}", value)),
        }
    }
}

// 辅助函数
fn read_dotnet_string(cursor: &mut Cursor<&[u8]>) -> Result<String, std::io::Error> {
    let len = cursor.read_u16::<LittleEndian>()? as usize;
    let mut buf = vec![0u8; len];
    std::io::Read::read_exact(cursor, &mut buf)?;
    String::from_utf8(buf).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

// ==================== 数据结构 ====================

#[derive(Debug, Clone)]
pub struct Chat {
    pub message: String,
    pub chat_type: ChatType,
}

#[derive(Debug, Clone)]
pub struct ObjectChat {
    pub object_id: u32,
    pub text: String,
    pub chat_type: ChatType,
}

// ==================== 解析函数 ====================

pub(crate) fn parse_chat(payload: &[u8]) -> Result<Chat, String> {
    let mut cursor = Cursor::new(payload);
    let message = read_dotnet_string(&mut cursor)
        .map_err(|err| format!("failed to read chat message: {err}"))?;
    let chat_type_byte = cursor
        .read_u8()
        .map_err(|err| format!("failed to read chat type: {err}"))?;
    let chat_type = ChatType::try_from(chat_type_byte)
        .map_err(|_| format!("unknown chat type {chat_type_byte}"))?;

    Ok(Chat { message, chat_type })
}

pub(crate) fn parse_object_chat(payload: &[u8]) -> Result<ObjectChat, String> {
    let mut cursor = Cursor::new(payload);
    let object_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read object chat object id: {err}"))?;
    let text = read_dotnet_string(&mut cursor)
        .map_err(|err| format!("failed to read object chat text: {err}"))?;
    let chat_type_byte = cursor
        .read_u8()
        .map_err(|err| format!("failed to read object chat type: {err}"))?;
    let chat_type = ChatType::try_from(chat_type_byte)
        .map_err(|_| format!("unknown object chat type {chat_type_byte}"))?;

    Ok(ObjectChat {
        object_id,
        text,
        chat_type,
    })
}
