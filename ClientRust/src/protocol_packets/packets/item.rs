//! Item System Packets
//!
//! This module contains all item-related packet definitions and parsers.

use byteorder::{LittleEndian, ReadBytesExt};
use mir2_shared::{enums::MirGridType, item::ItemInfo, UserItem};
use std::io::Cursor;

// ============================================================================
// Packet Structures
// ============================================================================

/// Item sold to NPC
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SellItem {
    pub unique_id: u64,
    pub count: u16,
    pub success: bool,
}

/// Item sent for repair
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairItem {
    pub unique_id: u64,
}

/// Item repair completed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemRepaired {
    pub unique_id: u64,
    pub max_dura: u16,
    pub current_dura: u16,
}

/// Split item stack
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitItem {
    pub grid: MirGridType,
    pub unique_id: u64,
    pub count: u16,
}

/// Split item stack (variant 1)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitItem1 {
    pub grid: MirGridType,
    pub unique_id: u64,
    pub count: u16,
}

/// Refresh item data
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshItem {
    pub item: UserItem,
}

/// Item slot size changed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemSlotSizeChanged {
    pub grid_type: MirGridType,
    pub unique_id: u64,
    pub slot_size: u8,
}

/// Item seal status changed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemSealChanged {
    pub grid_type: MirGridType,
    pub unique_id: u64,
    pub expiry_date: i64,
}

/// Item crafting result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CraftItem {
    pub unique_id: u64,
    pub count: u16,
    pub success: bool,
}

/// New item information received
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewItemInfo {
    pub info: ItemInfo,
}

// ============================================================================
// Parser Functions
// ============================================================================

pub(crate) fn parse_sell_item(payload: &[u8]) -> Result<SellItem, String> {
    let mut cursor = Cursor::new(payload);
    let unique_id = cursor
        .read_u64::<LittleEndian>()
        .map_err(|e| format!("Failed to read unique_id: {}", e))?;
    let count = cursor
        .read_u16::<LittleEndian>()
        .map_err(|e| format!("Failed to read count: {}", e))?;
    let success = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read success: {}", e))?
        != 0;
    Ok(SellItem {
        unique_id,
        count,
        success,
    })
}

pub(crate) fn parse_repair_item(payload: &[u8]) -> Result<RepairItem, String> {
    let mut cursor = Cursor::new(payload);
    let unique_id = cursor
        .read_u64::<LittleEndian>()
        .map_err(|e| format!("Failed to read unique_id: {}", e))?;
    Ok(RepairItem { unique_id })
}

pub(crate) fn parse_item_repaired(payload: &[u8]) -> Result<ItemRepaired, String> {
    let mut cursor = Cursor::new(payload);
    let unique_id = cursor
        .read_u64::<LittleEndian>()
        .map_err(|e| format!("Failed to read unique_id: {}", e))?;
    let max_dura = cursor
        .read_u16::<LittleEndian>()
        .map_err(|e| format!("Failed to read max_dura: {}", e))?;
    let current_dura = cursor
        .read_u16::<LittleEndian>()
        .map_err(|e| format!("Failed to read current_dura: {}", e))?;
    Ok(ItemRepaired {
        unique_id,
        max_dura,
        current_dura,
    })
}

pub(crate) fn parse_split_item(payload: &[u8]) -> Result<SplitItem, String> {
    let mut cursor = Cursor::new(payload);
    let grid_byte = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read grid: {}", e))?;
    let grid = MirGridType::try_from(grid_byte)
        .map_err(|_| format!("Unknown grid type: {}", grid_byte))?;
    let unique_id = cursor
        .read_u64::<LittleEndian>()
        .map_err(|e| format!("Failed to read unique_id: {}", e))?;
    let count = cursor
        .read_u16::<LittleEndian>()
        .map_err(|e| format!("Failed to read count: {}", e))?;
    Ok(SplitItem {
        grid,
        unique_id,
        count,
    })
}

pub(crate) fn parse_split_item1(payload: &[u8]) -> Result<SplitItem1, String> {
    let mut cursor = Cursor::new(payload);
    let grid_byte = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read grid: {}", e))?;
    let grid = MirGridType::try_from(grid_byte)
        .map_err(|_| format!("Unknown grid type: {}", grid_byte))?;
    let unique_id = cursor
        .read_u64::<LittleEndian>()
        .map_err(|e| format!("Failed to read unique_id: {}", e))?;
    let count = cursor
        .read_u16::<LittleEndian>()
        .map_err(|e| format!("Failed to read count: {}", e))?;
    Ok(SplitItem1 {
        grid,
        unique_id,
        count,
    })
}

pub(crate) fn parse_refresh_item(payload: &[u8]) -> Result<RefreshItem, String> {
    let mut cursor = Cursor::new(payload);
    let item = UserItem::read_from(&mut cursor)?;
    Ok(RefreshItem { item })
}

pub(crate) fn parse_item_slot_size_changed(payload: &[u8]) -> Result<ItemSlotSizeChanged, String> {
    let mut cursor = Cursor::new(payload);
    let grid_type_byte = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read grid_type: {}", e))?;
    let grid_type = MirGridType::try_from(grid_type_byte)
        .map_err(|_| format!("Unknown grid type: {}", grid_type_byte))?;
    let unique_id = cursor
        .read_u64::<LittleEndian>()
        .map_err(|e| format!("Failed to read unique_id: {}", e))?;
    let slot_size = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read slot_size: {}", e))?;
    Ok(ItemSlotSizeChanged {
        grid_type,
        unique_id,
        slot_size,
    })
}

pub(crate) fn parse_item_seal_changed(payload: &[u8]) -> Result<ItemSealChanged, String> {
    let mut cursor = Cursor::new(payload);
    let grid_type_byte = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read grid_type: {}", e))?;
    let grid_type = MirGridType::try_from(grid_type_byte)
        .map_err(|_| format!("Unknown grid type: {}", grid_type_byte))?;
    let unique_id = cursor
        .read_u64::<LittleEndian>()
        .map_err(|e| format!("Failed to read unique_id: {}", e))?;
    let expiry_date = cursor
        .read_i64::<LittleEndian>()
        .map_err(|e| format!("Failed to read expiry_date: {}", e))?;
    Ok(ItemSealChanged {
        grid_type,
        unique_id,
        expiry_date,
    })
}

pub(crate) fn parse_craft_item(payload: &[u8]) -> Result<CraftItem, String> {
    let mut cursor = Cursor::new(payload);
    let unique_id = cursor
        .read_u64::<LittleEndian>()
        .map_err(|e| format!("Failed to read unique_id: {}", e))?;
    let count = cursor
        .read_u16::<LittleEndian>()
        .map_err(|e| format!("Failed to read count: {}", e))?;
    let success = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read success: {}", e))?
        != 0;
    Ok(CraftItem {
        unique_id,
        count,
        success,
    })
}

pub(crate) fn parse_new_item_info(payload: &[u8]) -> Result<NewItemInfo, String> {
    let mut cursor = Cursor::new(payload);
    let info = ItemInfo::read_from(&mut cursor, &[], &[])?;
    Ok(NewItemInfo { info })
}
