//! mock 状态与物品构造（从 mock.rs 拆分，#1147）

use crossbeam_channel::{Receiver, Sender};
use mir2_shared::data::client_data::{ClientMagic, ClientQuestProgress, SelectInfo};
use mir2_shared::data::item::ItemInfo;
use mir2_shared::enums::{
    ChatType, ClientPacketIds, HeroBehaviour, ItemType, LevelEffects, MirClass, MirDirection,
    MirGender, PoisonType, Spell, SpellEffect, Stat,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use mir2_shared::packets::base::{serialize_packet, Packet, PacketHeader};
use mir2_shared::packets::{client, server};
use crate::network::codec;

#[derive(Clone, Copy)]
pub(crate) struct MockPlayerStats {
    pub(crate) level: u16,
    pub(crate) exp: i64,
    pub(crate) max_exp: i64,
    pub(crate) hp: u32,
    pub(crate) mp: u32,
}

impl MockPlayerStats {
    pub(crate) fn new() -> Self {
        // MOCK_START_MP 可调初始魔法值（默认 420；设小可快速验证 蓝不足拒绝）
        let mp = std::env::var("MOCK_START_MP")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(420);
        Self { level: 30, exp: 12000, max_exp: Self::max_exp_for(30), hp: 850, mp }
    }
    /// 经验上限（C# Globals.Experience 近似：level^2*100/3，30 级 = 30000）
    pub(crate) fn max_exp_for(level: u16) -> i64 {
        (level as i64 * level as i64 * 100) / 3
    }
}

/// 任务状态（#44 任务闭环：击杀 稻草人 x3）
#[derive(Default)]
pub(crate) struct MockQuest {
    pub(crate) taken: bool,
    pub(crate) kills: u32,
    pub(crate) completed: bool,
}

pub(crate) const QUEST_ID: i32 = 1;
pub(crate) const QUEST_KILL_TARGET: u32 = 3;

/// 怪物属性（#49：差异化 HP/伤害/经验/是否主动）
pub(crate) struct MonsterDef {
    pub(crate) hp_max: i32,
    pub(crate) damage: u32,
    pub(crate) exp: u32,
    pub(crate) aggressive: bool,
}

pub(crate) fn monster_def(id: u32) -> MonsterDef {
    match id {
        // 稻草人：被动挨打（首个练手怪）
        101 => MonsterDef { hp_max: 100, damage: 0, exp: 2000, aggressive: false },
        // 多钩猫：追击 + 邻接攻击
        102 => MonsterDef { hp_max: 120, damage: 40, exp: 2500, aggressive: true },
        // 半兽人：追击 + 邻接攻击（更强）
        _ => MonsterDef { hp_max: 150, damage: 60, exp: 3000, aggressive: true },
    }
}

pub(crate) fn wooden_sword_item() -> mir2_shared::data::item::UserItem {
    let mut s = mir2_shared::data::stats::Stats::new();
    s.set(Stat::MinDC, 5);
    s.set(Stat::MaxDC, 12);
    mir2_shared::data::item::UserItem {
        unique_id: 9005,
        item_index: 221,
        count: 1,
        info: Some(mir2_shared::data::item::ItemInfo {
            index: 221,
            name: "木剑".to_string(),
            image: 221,
            item_type: ItemType::Weapon,
            shape: 0,
            price: 10,
            stats: s,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// #557：带孔铁剑（2 孔，镶嵌面板验收）
/// #720：市场演示物品（uid=100/101，--market-test 寄售链路）
pub(crate) fn market_item(uid: u64) -> mir2_shared::data::item::UserItem {
    mir2_shared::data::item::UserItem {
        unique_id: uid,
        item_index: uid as i32,
        count: 1,
        info: Some(ItemInfo {
            index: uid as i32,
            name: format!("寄售物品#{}", uid),
            image: 1,
            item_type: ItemType::Weapon,
            shape: 0,
            price: 100,
            ..Default::default()
        }),
        ..Default::default()
    }
}

pub(crate) fn socketed_sword_item() -> mir2_shared::data::item::UserItem {
    let mut s = mir2_shared::data::stats::Stats::new();
    s.set(Stat::MinDC, 6);
    s.set(Stat::MaxDC, 14);
    mir2_shared::data::item::UserItem {
        unique_id: 9007,
        item_index: 222,
        count: 1,
        info: Some(mir2_shared::data::item::ItemInfo {
            index: 222,
            name: "带孔铁剑".to_string(),
            image: 222,
            item_type: ItemType::Weapon,
            shape: 0,
            price: 100,
            stats: s,
            ..Default::default()
        }),
        slots: vec![None, None],
        ..Default::default()
    }
}

pub(crate) fn potion_item(index: i32) -> mir2_shared::data::item::UserItem {
    mir2_shared::data::item::UserItem {
        unique_id: 9000 + index as u64,
        item_index: index,
        count: 1,
        info: Some(ItemInfo {
            index,
            name: match index {
                1 => "金创药(小)".to_string(),
                2 => "魔法药(小)".to_string(),
                5 => "木剑".to_string(),
                10 => "布衣".to_string(),
                _ => format!("#{}", index),
            },
            image: index as u16,
            item_type: match index {
                5 => ItemType::Weapon,
                10 => ItemType::Armour,
                _ => ItemType::Potion,
            },
            shape: 0,
            price: 10,
            stats: {
                let mut s = mir2_shared::data::stats::Stats::new();
                match index {
                    // 木剑：攻击 5-12
                    5 => {
                        s.set(Stat::MinDC, 5);
                        s.set(Stat::MaxDC, 12);
                    }
                    // 布衣：防御 2-5
                    10 => {
                        s.set(Stat::MinAC, 2);
                        s.set(Stat::MaxAC, 5);
                    }
                    _ => {}
                }
                s
            },
            ..Default::default()
        }),
        current_dura: 0,
        max_dura: 0,
        ..Default::default()
    }
}

/// 技能书物品（#212：Book 类型，shape = Spell）
pub(crate) fn book_item(spell: u8) -> mir2_shared::data::item::UserItem {
    mir2_shared::data::item::UserItem {
        unique_id: 9200 + spell as u64,
        item_index: 1000 + spell as i32,
        count: 1,
        info: Some(ItemInfo {
            index: 1000 + spell as i32,
            name: format!("技能书#{}", spell),
            image: 1,
            item_type: ItemType::Book,
            shape: spell as i16,
            price: 100,
            ..Default::default()
        }),
        ..Default::default()
    }
}



