// Buff/状态系统数据包解析
// Buff/Status System Packet Parsing

use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

// 辅助函数
fn read_bool(cursor: &mut Cursor<&[u8]>) -> Result<bool, std::io::Error> {
    Ok(cursor.read_u8()? != 0)
}

fn read_dotnet_string(cursor: &mut Cursor<&[u8]>) -> Result<String, std::io::Error> {
    let len = cursor.read_u16::<LittleEndian>()? as usize;
    let mut buf = vec![0u8; len];
    std::io::Read::read_exact(cursor, &mut buf)?;
    String::from_utf8(buf).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

// BuffType 枚举 (简化版,实际应该从共享库导入)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BuffType {
    None = 0,
    TemporarySkilBoost = 1,
    Hiding = 2,
    Haste = 3,
    // ... 更多类型
}

impl TryFrom<u8> for BuffType {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(BuffType::None),
            1 => Ok(BuffType::TemporarySkilBoost),
            2 => Ok(BuffType::Hiding),
            3 => Ok(BuffType::Haste),
            _ => Ok(BuffType::None), // 默认处理
        }
    }
}

// PoisonType 枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum PoisonType {
    None = 0,
    Green = 1,
    Red = 2,
    Slow = 3,
    Frozen = 4,
    Stun = 5,
    Paralysis = 6,
    DelayedExplosion = 7,
    Bleeding = 8,
    LRParalysis = 9,
    Blindness = 10,
}

// ClientBuff 结构体
#[derive(Debug, Clone)]
pub struct ClientBuff {
    pub buff_type: BuffType,
    pub visible: bool,
    pub object_id: u32,
    pub expire_time: i64,
    pub infinite: bool,
    pub paused: bool,
    pub stats: crate::data::stats::Stats,
    pub values: Vec<i32>,
}

// ==================== 数据结构 ====================

#[derive(Debug, Clone)]
pub struct AddBuff {
    pub buff: ClientBuff,
}

#[derive(Debug, Clone)]
pub struct RemoveBuff {
    pub buff_type: BuffType,
    pub object_id: u32,
}

#[derive(Debug, Clone)]
pub struct PauseBuff {
    pub buff_type: BuffType,
    pub object_id: u32,
    pub paused: bool,
}

#[derive(Debug, Clone)]
pub struct ColourChanged {
    pub name_colour_argb: i32,
}

#[derive(Debug, Clone)]
pub struct ObjectColourChanged {
    pub object_id: u32,
    pub name_colour_argb: i32,
}

#[derive(Debug, Clone)]
pub struct ObjectGuildNameChanged {
    pub object_id: u32,
    pub guild_name: String,
}

#[derive(Debug, Clone)]
pub struct Poisoned {
    pub poison: PoisonType,
}

#[derive(Debug, Clone)]
pub struct ObjectPoisoned {
    pub object_id: u32,
    pub poison: PoisonType,
}

// ==================== 解析函数 ====================

#[cfg(feature = "client-parse")]
pub(crate) fn parse_add_buff(payload: &[u8]) -> Result<AddBuff, String> {
    let mut cursor = Cursor::new(payload);
    let buff_type_byte = cursor
        .read_u8()
        .map_err(|err| format!("failed to read buff type: {err}"))?;
    let buff_type = BuffType::try_from(buff_type_byte)
        .map_err(|_| format!("unknown buff type {buff_type_byte}"))?;
    let visible =
        read_bool(&mut cursor).map_err(|err| format!("failed to read buff visible: {err}"))?;
    let object_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read buff object id: {err}"))?;
    let expire_time = cursor
        .read_i64::<LittleEndian>()
        .map_err(|err| format!("failed to read buff expire time: {err}"))?;
    let infinite =
        read_bool(&mut cursor).map_err(|err| format!("failed to read buff infinite: {err}"))?;
    let paused =
        read_bool(&mut cursor).map_err(|err| format!("failed to read buff paused: {err}"))?;

    let stats = crate::data::stats::Stats::read_from(&mut cursor)
        .map_err(|err| format!("failed to read buff stats: {err}"))?;

    let values_count = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read buff values count: {err}"))?;

    let mut values = Vec::with_capacity(values_count as usize);
    for _ in 0..values_count {
        let value = cursor
            .read_i32::<LittleEndian>()
            .map_err(|err| format!("failed to read buff value: {err}"))?;
        values.push(value);
    }

    Ok(AddBuff {
        buff: ClientBuff {
            buff_type,
            visible,
            object_id,
            expire_time,
            infinite,
            paused,
            stats,
            values,
        },
    })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_remove_buff(payload: &[u8]) -> Result<RemoveBuff, String> {
    let mut cursor = Cursor::new(payload);
    let buff_type_byte = cursor
        .read_u8()
        .map_err(|err| format!("failed to read remove buff type: {err}"))?;
    let buff_type = BuffType::try_from(buff_type_byte)
        .map_err(|_| format!("unknown remove buff type {buff_type_byte}"))?;
    let object_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read remove buff object id: {err}"))?;

    Ok(RemoveBuff {
        buff_type,
        object_id,
    })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_pause_buff(payload: &[u8]) -> Result<PauseBuff, String> {
    let mut cursor = Cursor::new(payload);
    let buff_type_byte = cursor
        .read_u8()
        .map_err(|err| format!("failed to read pause buff type: {err}"))?;
    let buff_type = BuffType::try_from(buff_type_byte)
        .map_err(|_| format!("unknown pause buff type {buff_type_byte}"))?;
    let object_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read pause buff object id: {err}"))?;
    let paused =
        read_bool(&mut cursor).map_err(|err| format!("failed to read pause buff paused: {err}"))?;

    Ok(PauseBuff {
        buff_type,
        object_id,
        paused,
    })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_colour_changed(payload: &[u8]) -> Result<ColourChanged, String> {
    let mut cursor = Cursor::new(payload);
    let name_colour_argb = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read name colour value: {err}"))?;
    Ok(ColourChanged { name_colour_argb })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_object_colour_changed(payload: &[u8]) -> Result<ObjectColourChanged, String> {
    let mut cursor = Cursor::new(payload);
    let object_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read colour change object id: {err}"))?;
    let name_colour_argb = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read object name colour value: {err}"))?;
    Ok(ObjectColourChanged {
        object_id,
        name_colour_argb,
    })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_object_guild_name_changed(
    payload: &[u8],
) -> Result<ObjectGuildNameChanged, String> {
    let mut cursor = Cursor::new(payload);
    let object_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read guild name change object id: {err}"))?;
    let guild_name = read_dotnet_string(&mut cursor)
        .map_err(|err| format!("failed to read guild name: {err}"))?;
    Ok(ObjectGuildNameChanged {
        object_id,
        guild_name,
    })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_poisoned(payload: &[u8]) -> Result<Poisoned, String> {
    let mut cursor = Cursor::new(payload);
    let poison_value = cursor
        .read_u16::<LittleEndian>()
        .map_err(|err| format!("failed to read poisoned type: {err}"))?;
    let poison = unsafe { std::mem::transmute::<u16, PoisonType>(poison_value) };
    Ok(Poisoned { poison })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_object_poisoned(payload: &[u8]) -> Result<ObjectPoisoned, String> {
    let mut cursor = Cursor::new(payload);
    let object_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read object poisoned id: {err}"))?;
    let poison_value = cursor
        .read_u16::<LittleEndian>()
        .map_err(|err| format!("failed to read object poisoned type: {err}"))?;
    let poison = unsafe { std::mem::transmute::<u16, PoisonType>(poison_value) };
    Ok(ObjectPoisoned { object_id, poison })
}
