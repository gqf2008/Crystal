// 战斗系统数据包解析
// Combat System Packet Parsing

use std::io::Cursor;
use std::io::Read;

#[derive(Debug, Clone)]
pub struct ObjectAttack {
    pub object_id: u32,
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
    pub spell: u8,
    pub level: u16,
    pub attack_type: u8,
}

#[derive(Debug, Clone)]
pub struct Struck {
    pub attacker_id: u32,
}

#[derive(Debug, Clone)]
pub struct ObjectStruck {
    pub object_id: u32,
    pub attacker_id: u32,
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
}

#[derive(Debug, Clone)]
pub struct DamageIndicator {
    pub damage: i32,
    pub damage_type: u8,
    pub object_id: u32,
}

#[derive(Debug, Clone)]
pub struct Pushed {
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
}

#[derive(Debug, Clone)]
pub struct ObjectPushed {
    pub object_id: u32,
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
}

#[derive(Debug, Clone)]
pub struct RangeAttack {
    pub target_id: u32,
    pub target_x: u32,
    pub target_y: u32,
    pub spell: u16,
    pub spell_level: u16,
}

#[derive(Debug, Clone)]
pub struct ObjectRangeAttack {
    pub object_id: u32,
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
    pub target_id: u32,
    pub target_x: u32,
    pub target_y: u32,
    pub spell: u16,
    pub spell_level: u16,
}

#[derive(Debug, Clone)]
pub struct UserDash {
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
}

#[derive(Debug, Clone)]
pub struct ObjectDash {
    pub object_id: u32,
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
}

#[derive(Debug, Clone)]
pub struct UserDashFail {
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
}

#[derive(Debug, Clone)]
pub struct ObjectDashFail {
    pub object_id: u32,
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
}

#[derive(Debug, Clone)]
pub struct Death {
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
}

#[derive(Debug, Clone)]
pub struct ObjectDied {
    pub object_id: u32,
    pub location_x: u32,
    pub location_y: u32,
    pub direction: u8,
    pub death_type: u8,
}

#[derive(Debug, Clone)]
pub struct Revived;

#[derive(Debug, Clone)]
pub struct ObjectRevived {
    pub object_id: u32,
    pub effect: u8,
}

#[derive(Debug, Clone)]
pub struct HealthChanged {
    pub hp: u32,
    pub mp: u32,
}

#[derive(Debug, Clone)]
pub struct HeroHealthChanged {
    pub hp: u32,
    pub mp: u32,
}

// ==================== 解析函数 ====================

pub(crate) fn parse_object_attack(payload: &[u8]) -> Result<ObjectAttack, String> {
    if payload.len() < 19 {
        return Err(format!(
            "ObjectAttack payload too short: {} bytes",
            payload.len()
        ));
    }

    Ok(ObjectAttack {
        object_id: u32::from_le_bytes(payload[0..4].try_into().unwrap()),
        location_x: u32::from_le_bytes(payload[4..8].try_into().unwrap()),
        location_y: u32::from_le_bytes(payload[8..12].try_into().unwrap()),
        direction: payload[12],
        spell: payload[13],
        level: u16::from_le_bytes(payload[14..16].try_into().unwrap()),
        attack_type: payload[16],
    })
}

pub(crate) fn parse_struck(payload: &[u8]) -> Result<Struck, String> {
    if payload.len() < 4 {
        return Err(format!("Struck payload too short: {} bytes", payload.len()));
    }

    Ok(Struck {
        attacker_id: u32::from_le_bytes(payload[0..4].try_into().unwrap()),
    })
}

pub(crate) fn parse_object_struck(payload: &[u8]) -> Result<ObjectStruck, String> {
    if payload.len() < 17 {
        return Err(format!(
            "ObjectStruck payload too short: {} bytes",
            payload.len()
        ));
    }

    Ok(ObjectStruck {
        object_id: u32::from_le_bytes(payload[0..4].try_into().unwrap()),
        attacker_id: u32::from_le_bytes(payload[4..8].try_into().unwrap()),
        location_x: u32::from_le_bytes(payload[8..12].try_into().unwrap()),
        location_y: u32::from_le_bytes(payload[12..16].try_into().unwrap()),
        direction: payload[16],
    })
}

pub(crate) fn parse_damage_indicator(payload: &[u8]) -> Result<DamageIndicator, String> {
    if payload.len() < 9 {
        return Err(format!(
            "DamageIndicator payload too short: {} bytes",
            payload.len()
        ));
    }

    Ok(DamageIndicator {
        damage: i32::from_le_bytes(payload[0..4].try_into().unwrap()),
        damage_type: payload[4],
        object_id: u32::from_le_bytes(payload[5..9].try_into().unwrap()),
    })
}

pub(crate) fn parse_pushed(payload: &[u8]) -> Result<Pushed, String> {
    if payload.len() < 9 {
        return Err(format!("Pushed payload too short: {} bytes", payload.len()));
    }

    Ok(Pushed {
        location_x: u32::from_le_bytes(payload[0..4].try_into().unwrap()),
        location_y: u32::from_le_bytes(payload[4..8].try_into().unwrap()),
        direction: payload[8],
    })
}

pub(crate) fn parse_object_pushed(payload: &[u8]) -> Result<ObjectPushed, String> {
    if payload.len() < 13 {
        return Err(format!(
            "ObjectPushed payload too short: {} bytes",
            payload.len()
        ));
    }

    Ok(ObjectPushed {
        object_id: u32::from_le_bytes(payload[0..4].try_into().unwrap()),
        location_x: u32::from_le_bytes(payload[4..8].try_into().unwrap()),
        location_y: u32::from_le_bytes(payload[8..12].try_into().unwrap()),
        direction: payload[12],
    })
}

pub(crate) fn parse_range_attack(payload: &[u8]) -> Result<RangeAttack, String> {
    if payload.len() < 16 {
        return Err(format!(
            "RangeAttack payload too short: {} bytes",
            payload.len()
        ));
    }

    Ok(RangeAttack {
        target_id: u32::from_le_bytes(payload[0..4].try_into().unwrap()),
        target_x: u32::from_le_bytes(payload[4..8].try_into().unwrap()),
        target_y: u32::from_le_bytes(payload[8..12].try_into().unwrap()),
        spell: u16::from_le_bytes(payload[12..14].try_into().unwrap()),
        spell_level: u16::from_le_bytes(payload[14..16].try_into().unwrap()),
    })
}

pub(crate) fn parse_object_range_attack(payload: &[u8]) -> Result<ObjectRangeAttack, String> {
    if payload.len() < 29 {
        return Err(format!(
            "ObjectRangeAttack payload too short: {} bytes",
            payload.len()
        ));
    }

    Ok(ObjectRangeAttack {
        object_id: u32::from_le_bytes(payload[0..4].try_into().unwrap()),
        location_x: u32::from_le_bytes(payload[4..8].try_into().unwrap()),
        location_y: u32::from_le_bytes(payload[8..12].try_into().unwrap()),
        direction: payload[12],
        target_id: u32::from_le_bytes(payload[13..17].try_into().unwrap()),
        target_x: u32::from_le_bytes(payload[17..21].try_into().unwrap()),
        target_y: u32::from_le_bytes(payload[21..25].try_into().unwrap()),
        spell: u16::from_le_bytes(payload[25..27].try_into().unwrap()),
        spell_level: u16::from_le_bytes(payload[27..29].try_into().unwrap()),
    })
}

pub(crate) fn parse_user_dash(payload: &[u8]) -> Result<UserDash, String> {
    if payload.len() < 9 {
        return Err(format!(
            "UserDash payload too short: {} bytes",
            payload.len()
        ));
    }

    Ok(UserDash {
        location_x: u32::from_le_bytes(payload[0..4].try_into().unwrap()),
        location_y: u32::from_le_bytes(payload[4..8].try_into().unwrap()),
        direction: payload[8],
    })
}

pub(crate) fn parse_object_dash(payload: &[u8]) -> Result<ObjectDash, String> {
    if payload.len() < 13 {
        return Err(format!(
            "ObjectDash payload too short: {} bytes",
            payload.len()
        ));
    }

    Ok(ObjectDash {
        object_id: u32::from_le_bytes(payload[0..4].try_into().unwrap()),
        location_x: u32::from_le_bytes(payload[4..8].try_into().unwrap()),
        location_y: u32::from_le_bytes(payload[8..12].try_into().unwrap()),
        direction: payload[12],
    })
}

pub(crate) fn parse_user_dash_fail(payload: &[u8]) -> Result<UserDashFail, String> {
    if payload.len() < 9 {
        return Err(format!(
            "UserDashFail payload too short: {} bytes",
            payload.len()
        ));
    }

    Ok(UserDashFail {
        location_x: u32::from_le_bytes(payload[0..4].try_into().unwrap()),
        location_y: u32::from_le_bytes(payload[4..8].try_into().unwrap()),
        direction: payload[8],
    })
}

pub(crate) fn parse_object_dash_fail(payload: &[u8]) -> Result<ObjectDashFail, String> {
    if payload.len() < 13 {
        return Err(format!(
            "ObjectDashFail payload too short: {} bytes",
            payload.len()
        ));
    }

    Ok(ObjectDashFail {
        object_id: u32::from_le_bytes(payload[0..4].try_into().unwrap()),
        location_x: u32::from_le_bytes(payload[4..8].try_into().unwrap()),
        location_y: u32::from_le_bytes(payload[8..12].try_into().unwrap()),
        direction: payload[12],
    })
}

pub(crate) fn parse_death(payload: &[u8]) -> Result<Death, String> {
    if payload.len() < 9 {
        return Err(format!("Death payload too short: {} bytes", payload.len()));
    }

    Ok(Death {
        location_x: u32::from_le_bytes(payload[0..4].try_into().unwrap()),
        location_y: u32::from_le_bytes(payload[4..8].try_into().unwrap()),
        direction: payload[8],
    })
}

pub(crate) fn parse_object_died(payload: &[u8]) -> Result<ObjectDied, String> {
    if payload.len() < 14 {
        return Err(format!(
            "ObjectDied payload too short: {} bytes",
            payload.len()
        ));
    }

    Ok(ObjectDied {
        object_id: u32::from_le_bytes(payload[0..4].try_into().unwrap()),
        location_x: u32::from_le_bytes(payload[4..8].try_into().unwrap()),
        location_y: u32::from_le_bytes(payload[8..12].try_into().unwrap()),
        direction: payload[12],
        death_type: payload[13],
    })
}

pub(crate) fn parse_revived(_payload: &[u8]) -> Result<Revived, String> {
    Ok(Revived)
}

pub(crate) fn parse_object_revived(payload: &[u8]) -> Result<ObjectRevived, String> {
    if payload.len() < 5 {
        return Err(format!(
            "ObjectRevived payload too short: {} bytes",
            payload.len()
        ));
    }

    Ok(ObjectRevived {
        object_id: u32::from_le_bytes(payload[0..4].try_into().unwrap()),
        effect: payload[4],
    })
}

pub(crate) fn parse_health_changed(payload: &[u8]) -> Result<HealthChanged, String> {
    if payload.len() < 8 {
        return Err(format!(
            "HealthChanged payload too short: {} bytes",
            payload.len()
        ));
    }

    Ok(HealthChanged {
        hp: u32::from_le_bytes(payload[0..4].try_into().unwrap()),
        mp: u32::from_le_bytes(payload[4..8].try_into().unwrap()),
    })
}

pub(crate) fn parse_hero_health_changed(payload: &[u8]) -> Result<HeroHealthChanged, String> {
    if payload.len() < 8 {
        return Err(format!(
            "HeroHealthChanged payload too short: {} bytes",
            payload.len()
        ));
    }

    Ok(HeroHealthChanged {
        hp: u32::from_le_bytes(payload[0..4].try_into().unwrap()),
        mp: u32::from_le_bytes(payload[4..8].try_into().unwrap()),
    })
}
