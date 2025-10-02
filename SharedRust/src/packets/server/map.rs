// 地图系统数据包解析
// Map System Packet Parsing

use std::io::{Cursor, Read};

#[derive(Debug, Clone)]
pub struct MapInformation {
    pub map_index: i32,
    pub file_name: String,
    pub title: String,
    pub minimap: u16,
    pub big_map: u16,
    pub lights: u8,
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
    pub map_dark_light: u8,
    pub music: u16,
}

#[derive(Debug, Clone)]
pub struct NewMapInfo {
    pub map_index: i32,
    pub file_name: String,
    pub title: String,
    pub minimap: u16,
    pub big_map: u16,
    pub music: u16,
    pub lights: u8,
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
    pub map_dark_light: u8,
}

#[derive(Debug, Clone)]
pub struct MapChanged {
    pub file_name: String,
    pub title: String,
    pub minimap: u16,
    pub big_map: u16,
    pub music: u16,
    pub lights: u8,
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
    pub map_dark_light: u8,
}

#[derive(Debug, Clone)]
pub struct ObjectHide {
    pub object_id: u32,
}

#[derive(Debug, Clone)]
pub struct ObjectShow {
    pub object_id: u32,
}

#[derive(Debug, Clone)]
pub struct ObjectTeleportOut {
    pub object_id: u32,
    pub teleport_type: u8,
}

#[derive(Debug, Clone)]
pub struct ObjectTeleportIn {
    pub object_id: u32,
    pub teleport_type: u8,
}

#[derive(Debug, Clone)]
pub struct TeleportIn;

#[derive(Debug, Clone)]
pub struct WorldMapSetupInfo {
    pub world_maps: Vec<WorldMapIcon>,
}

#[derive(Debug, Clone)]
pub struct WorldMapIcon {
    pub icon: u16,
    pub title: String,
    pub map_index: i32,
    pub location_x: u32,
    pub location_y: u32,
}

#[derive(Debug, Clone)]
pub struct SearchMapResult {
    pub map_index: i32,
    pub location_x: u32,
    pub location_y: u32,
}

// ==================== 解析函数 ====================

#[cfg(feature = "client-parse")]
pub(crate) fn parse_map_information(payload: &[u8]) -> Result<MapInformation, String> {
    let mut cursor = Cursor::new(payload);

    let map_index = read_i32(&mut cursor)?;
    let file_name = read_string(&mut cursor)?;
    let title = read_string(&mut cursor)?;
    let minimap = read_u16(&mut cursor)?;
    let big_map = read_u16(&mut cursor)?;
    let lights = read_u8(&mut cursor)?;
    let location_x = read_u32(&mut cursor)?;
    let location_y = read_u32(&mut cursor)?;
    let direction = read_u8(&mut cursor)?;
    let map_dark_light = read_u8(&mut cursor)?;
    let music = read_u16(&mut cursor)?;

    Ok(MapInformation {
        map_index,
        file_name,
        title,
        minimap,
        big_map,
        lights,
        location_x,
        location_y,
        direction,
        map_dark_light,
        music,
    })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_new_map_info(payload: &[u8]) -> Result<NewMapInfo, String> {
    let mut cursor = Cursor::new(payload);

    Ok(NewMapInfo {
        map_index: read_i32(&mut cursor)?,
        file_name: read_string(&mut cursor)?,
        title: read_string(&mut cursor)?,
        minimap: read_u16(&mut cursor)?,
        big_map: read_u16(&mut cursor)?,
        music: read_u16(&mut cursor)?,
        lights: read_u8(&mut cursor)?,
        location_x: read_u32(&mut cursor)?,
        location_y: read_u32(&mut cursor)?,
        direction: read_u8(&mut cursor)?,
        map_dark_light: read_u8(&mut cursor)?,
    })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_map_changed(payload: &[u8]) -> Result<MapChanged, String> {
    let mut cursor = Cursor::new(payload);

    Ok(MapChanged {
        file_name: read_string(&mut cursor)?,
        title: read_string(&mut cursor)?,
        minimap: read_u16(&mut cursor)?,
        big_map: read_u16(&mut cursor)?,
        music: read_u16(&mut cursor)?,
        lights: read_u8(&mut cursor)?,
        location_x: read_u32(&mut cursor)?,
        location_y: read_u32(&mut cursor)?,
        direction: read_u8(&mut cursor)?,
        map_dark_light: read_u8(&mut cursor)?,
    })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_object_hide(payload: &[u8]) -> Result<ObjectHide, String> {
    if payload.len() < 4 {
        return Err(format!(
            "ObjectHide payload too short: {} bytes",
            payload.len()
        ));
    }

    Ok(ObjectHide {
        object_id: u32::from_le_bytes(payload[0..4].try_into().unwrap()),
    })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_object_show(payload: &[u8]) -> Result<ObjectShow, String> {
    if payload.len() < 4 {
        return Err(format!(
            "ObjectShow payload too short: {} bytes",
            payload.len()
        ));
    }

    Ok(ObjectShow {
        object_id: u32::from_le_bytes(payload[0..4].try_into().unwrap()),
    })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_object_teleport_out(payload: &[u8]) -> Result<ObjectTeleportOut, String> {
    if payload.len() < 5 {
        return Err(format!(
            "ObjectTeleportOut payload too short: {} bytes",
            payload.len()
        ));
    }

    Ok(ObjectTeleportOut {
        object_id: u32::from_le_bytes(payload[0..4].try_into().unwrap()),
        teleport_type: payload[4],
    })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_object_teleport_in(payload: &[u8]) -> Result<ObjectTeleportIn, String> {
    if payload.len() < 5 {
        return Err(format!(
            "ObjectTeleportIn payload too short: {} bytes",
            payload.len()
        ));
    }

    Ok(ObjectTeleportIn {
        object_id: u32::from_le_bytes(payload[0..4].try_into().unwrap()),
        teleport_type: payload[4],
    })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_teleport_in(_payload: &[u8]) -> Result<TeleportIn, String> {
    Ok(TeleportIn)
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_world_map_setup(payload: &[u8]) -> Result<WorldMapSetupInfo, String> {
    let mut cursor = Cursor::new(payload);
    let count = read_i32(&mut cursor)? as usize;

    let mut world_maps = Vec::with_capacity(count);
    for _ in 0..count {
        world_maps.push(WorldMapIcon {
            icon: read_u16(&mut cursor)?,
            title: read_string(&mut cursor)?,
            map_index: read_i32(&mut cursor)?,
            location_x: read_u32(&mut cursor)?,
            location_y: read_u32(&mut cursor)?,
        });
    }

    Ok(WorldMapSetupInfo { world_maps })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_search_map_result(payload: &[u8]) -> Result<SearchMapResult, String> {
    if payload.len() < 12 {
        return Err(format!(
            "SearchMapResult payload too short: {} bytes",
            payload.len()
        ));
    }

    Ok(SearchMapResult {
        map_index: i32::from_le_bytes(payload[0..4].try_into().unwrap()),
        location_x: u32::from_le_bytes(payload[4..8].try_into().unwrap()),
        location_y: u32::from_le_bytes(payload[8..12].try_into().unwrap()),
    })
}

// ==================== 辅助函数 ====================

fn read_u8(cursor: &mut Cursor<&[u8]>) -> Result<u8, String> {
    let mut buf = [0u8; 1];
    cursor
        .read_exact(&mut buf)
        .map_err(|e| format!("Failed to read u8: {}", e))?;
    Ok(buf[0])
}

fn read_u16(cursor: &mut Cursor<&[u8]>) -> Result<u16, String> {
    let mut buf = [0u8; 2];
    cursor
        .read_exact(&mut buf)
        .map_err(|e| format!("Failed to read u16: {}", e))?;
    Ok(u16::from_le_bytes(buf))
}

fn read_u32(cursor: &mut Cursor<&[u8]>) -> Result<u32, String> {
    let mut buf = [0u8; 4];
    cursor
        .read_exact(&mut buf)
        .map_err(|e| format!("Failed to read u32: {}", e))?;
    Ok(u32::from_le_bytes(buf))
}

fn read_i32(cursor: &mut Cursor<&[u8]>) -> Result<i32, String> {
    let mut buf = [0u8; 4];
    cursor
        .read_exact(&mut buf)
        .map_err(|e| format!("Failed to read i32: {}", e))?;
    Ok(i32::from_le_bytes(buf))
}

fn read_string(cursor: &mut Cursor<&[u8]>) -> Result<String, String> {
    let length = read_u16(cursor)? as usize;
    let mut buf = vec![0u8; length];
    cursor
        .read_exact(&mut buf)
        .map_err(|e| format!("Failed to read string: {}", e))?;
    String::from_utf8(buf).map_err(|e| format!("Invalid UTF-8: {}", e))
}
