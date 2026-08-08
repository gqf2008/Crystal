//! mock 发送/奖励/信息构建（从 mock.rs 拆分，#1147）

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
use super::state::*;

pub(crate) fn send<P: Packet>(to_client: &Sender<Vec<u8>>, packet: &P) {
    let mut inner = Vec::new();
    if serialize_packet(&mut inner, packet).is_ok() {
        let mut framed = Vec::new();
        codec::encode(&inner, &mut framed);
        let _ = to_client.send(framed);
    }
}

/// 发放经验：GainExperience + 升级检测（LevelChanged/ObjectLeveled/聊天）
pub(crate) fn grant_exp(to_client: &Sender<Vec<u8>>, stats: &mut MockPlayerStats, amount: u32) {
    stats.exp += amount as i64;
    send(to_client, &server::experience::GainExperience { amount });
    send(
        to_client,
        &server::chat::Chat {
            message: format!("获得 {} 经验", amount),
            chat_type: ChatType::System,
        },
    );
    tracing::info!("⭐ [MOCK] 获得经验 {}，当前 {}/{}", amount, stats.exp, stats.max_exp);

    while stats.exp >= stats.max_exp {
        stats.exp -= stats.max_exp;
        stats.level += 1;
        stats.max_exp = MockPlayerStats::max_exp_for(stats.level);
        stats.hp = 1000;
        stats.mp = 500;
        send(
            to_client,
            &server::experience::LevelChanged {
                level: stats.level,
                experience: stats.exp,
                max_experience: stats.max_exp,
            },
        );
        send(to_client, &server::experience::ObjectLeveled { object_id: 100, level: stats.level });
        send(
            to_client,
            &server::chat::Chat {
                message: format!("🎉 恭喜升级！当前等级 {}", stats.level),
                chat_type: ChatType::System,
            },
        );
        tracing::info!("⬆️ [MOCK] 升级到 {} 级", stats.level);
    }
}

/// 击杀奖励：经验（按怪物 #49）+ 任务计数/完成（#43/#44）
pub(crate) fn on_kill_reward(
    to_client: &Sender<Vec<u8>>,
    target: u32,
    stats: &mut MockPlayerStats,
    quest: &mut MockQuest,
) {
    grant_exp(to_client, stats, monster_def(target).exp);
    if !quest.taken || quest.completed {
        return;
    }
    quest.kills += 1;
    tracing::info!("📜 [MOCK] 任务击杀计数 {}/{}", quest.kills, QUEST_KILL_TARGET);
    let task = format!("击杀 稻草人 {}/{}", quest.kills, QUEST_KILL_TARGET);
    if quest.kills >= QUEST_KILL_TARGET {
        quest.completed = true;
        send(
            to_client,
            &server::quest::ChangeQuest {
                quest: ClientQuestProgress {
                    id: QUEST_ID,
                    task_list: vec![task],
                    taken: true,
                    completed: true,
                    new: false,
                },
            },
        );
        send(
            to_client,
            &server::chat::Chat {
                message: "任务完成！找 NPC 交任务领取奖励".into(),
                chat_type: ChatType::System,
            },
        );
        tracing::info!("🎯 [MOCK] 任务 {} 完成（等待交任务）", QUEST_ID);
    } else {
        send(
            to_client,
            &server::quest::ChangeQuest {
                quest: ClientQuestProgress {
                    id: QUEST_ID,
                    task_list: vec![task],
                    taken: true,
                    completed: false,
                    new: false,
                },
            },
        );
    }
}

/// 玩家攻击伤害：基础 15 + 武器槽 MinDC..MaxDC 中值（#47）
pub(crate) fn player_attack_damage(equipment: &[Option<mir2_shared::data::item::UserItem>]) -> u32 {
    let weapon = equipment.get(0).and_then(|s| s.as_ref());
    let (min_dc, max_dc) = weapon
        .and_then(|w| w.info.as_ref())
        .map(|i| (i.stats.get(Stat::MinDC).max(0) as u32, i.stats.get(Stat::MaxDC).max(0) as u32))
        .unwrap_or((0, 0));
    15 + if max_dc > 0 { min_dc + (max_dc - min_dc) / 2 } else { 0 }
}

/// 玩家防御：护甲槽 MaxAC（#47）
pub(crate) fn player_defence(equipment: &[Option<mir2_shared::data::item::UserItem>]) -> u32 {
    equipment
        .get(1)
        .and_then(|s| s.as_ref())
        .and_then(|i| i.info.as_ref())
        .map(|i| i.stats.get(Stat::MaxAC).max(0) as u32)
        .unwrap_or(0)
}

/// 复活玩家：清死亡状态 + Revived/ObjectRevived + 满血 UserInformation + 回安全区
#[allow(clippy::too_many_arguments)]
pub(crate) fn revive_player(
    to_client: &Sender<Vec<u8>>,
    char_index: i32,
    inventory: &[Option<mir2_shared::data::item::UserItem>],
    equipment: &[Option<mir2_shared::data::item::UserItem>],
    gold: u32,
    stats: &mut MockPlayerStats,
    dead: &mut bool,
    dead_since: &mut Option<std::time::Instant>,
) {
    *dead = false;
    *dead_since = None;
    stats.hp = 1000;
    stats.mp = 500;
    send(to_client, &server::combat::Revived);
    send(to_client, &server::combat::ObjectRevived { object_id: 100, effect: 1 });
    send_user_information(to_client, char_index, inventory, equipment, gold, *stats);
    send(
        to_client,
        &server::user::UserLocation {
            location_x: 354,
            location_y: 352,
            direction: MirDirection::Up,
        },
    );
    tracing::info!("💚 [MOCK] 玩家复活（满血）");
}

/// 构造可拾取/背包物品（金创药等）
/// 木剑（WoodenSword，index 221；觉醒测试用，uid 9005 与修理/槽位演示一致）
/// 发送玩家属性（UserInformation + HealthChanged）——购买/拾取后刷新用
pub(crate) fn send_user_information(
    to_client: &Sender<Vec<u8>>,
    char_index: i32,
    inventory: &[Option<mir2_shared::data::item::UserItem>],
    equipment: &[Option<mir2_shared::data::item::UserItem>],
    gold: u32,
    stats: MockPlayerStats,
) {
    let (class, gender) = match char_index {
        1 => (MirClass::Wizard, MirGender::Female),
        _ => (MirClass::Warrior, MirGender::Male),
    };
    send(
        to_client,
        &server::user::UserInformation {
            object_id: 100,
            real_id: 100,
            name: match char_index {
                1 => "法师".to_string(),
                2 => "道士".to_string(),
                3 => "刺客".to_string(),
                _ => "刀客".to_string(),
            },
            guild_name: String::new(),
            guild_rank: String::new(),
            name_colour: 0,
            class,
            gender,
            level: stats.level,
            location_x: 354,
            location_y: 352,
            direction: MirDirection::Up,
            hair: 0,
            hp: stats.hp as i32,
            mp: stats.mp as i32,
            experience: stats.exp,
            max_experience: stats.max_exp,
            level_effects: LevelEffects::NONE,
            has_hero: false,
            hero_behaviour: HeroBehaviour::Follow,
            inventory: Some(inventory.to_vec()),
            equipment: Some(equipment.to_vec()),
            quest_inventory: Some(vec![]),
            gold,
            credit: 0,
            has_expanded_storage: false,
            has_storage_password: false,
            require_storage_password: false,
            storage_password_last_set: 0,
            expanded_storage_expiry_time: 0,
            magics: vec![
                ClientMagic {
                    name: "攻杀剑术".to_string(),
                    spell: Spell::Slaying,
                    base_cost: 3,
                    level_cost: 1,
                    icon: 0,
                    level1: 1,
                    level2: 2,
                    level3: 3,
                    need1: 0,
                    need2: 0,
                    need3: 0,
                    level: 1,
                    key: 1,
                    experience: 0,
                    delay: 0,
                    range: 1,
                    cast_time: 0,
                },
                ClientMagic {
                    name: "刺杀剑术".to_string(),
                    spell: Spell::Fencing,
                    base_cost: 3,
                    level_cost: 1,
                    icon: 0,
                    level1: 1,
                    level2: 2,
                    level3: 3,
                    need1: 0,
                    need2: 0,
                    need3: 0,
                    level: 1,
                    key: 2,
                    experience: 0,
                    delay: 0,
                    range: 1,
                    cast_time: 0,
                },
            ],
            summoned_creature_type: 0,
            creature_summoned: false,
            allow_observe: false,
            observer: false,
            max_hp: stats.hp as i32,
            max_mp: stats.mp as i32,
            ac: [5, 10],
            mac: [2, 5],
            dc: [10, 18],
            mc: [0, 0],
            sc: [0, 0],
            critical_rate: 5,
            critical_damage: 10,
            attack_speed: 0,
            accuracy: 3,
            agility: 2,
            luck: 0,
            bag_weight: 120,
            wear_weight: 45,
            hand_weight: 0,
            magic_resist: 5,
            poison_resist: 2,
            health_recovery: 3,
            spell_recovery: 1,
            poison_recovery: 4,
            holy: 0,
            freezing: 0,
            poison_atk: 0,
        },
    );
    send(to_client, &server::combat::HealthChanged { hp: stats.hp, mp: stats.mp });
}

/// #203：发送 S.HeroInformation（mock 英雄完整信息）
pub(crate) fn send_hero_information(
    to_client: &Sender<Vec<u8>>,
    inventory: &[Option<mir2_shared::data::item::UserItem>],
    equipment: &[Option<mir2_shared::data::item::UserItem>],
    magics: &[ClientMagic],
    auto_pot: bool,
    auto_hp_percent: u8,
    auto_mp_percent: u8,
    hp_item_index: i32,
    mp_item_index: i32,
) {
    send(
        to_client,
        &server::hero::HeroInformation {
            object_id: 0x1000_0100,
            name: "英雄小刀".to_string(),
            class: MirClass::Warrior,
            gender: MirGender::Male,
            level: 30,
            hair: 0,
            hp: 600,
            mp: 100,
            experience: 8000,
            max_experience: 30000,
            inventory: Some(inventory.to_vec()),
            equipment: Some(equipment.to_vec()),
            magics: magics.to_vec(),
            auto_pot,
            auto_hp_percent,
            auto_mp_percent,
            hp_item_index,
            mp_item_index,
        },
    );
}

/// 进图：MapChanged(n0) + 本地玩家 + 怪物/NPC
pub(crate) fn send_map_and_objects(
    to_client: &Sender<Vec<u8>>,
    char_index: i32,
    inventory: &[Option<mir2_shared::data::item::UserItem>],
    equipment: &[Option<mir2_shared::data::item::UserItem>],
    gold: u32,
    stats: MockPlayerStats,
) {
    // 地图：新手村 n0，出生点附近
    send(
        to_client,
        &server::map::MapChanged {
            map_index: 0,
            file_name: "n0".to_string(),
            title: "新手村".to_string(),
            minimap: 0,
            big_map: 0,
            lights: 0,
            location_x: 354,
            location_y: 352,
            direction: 0,
            map_dark_light: 0,
            music: 0,
            weather: 1,
        },
    );

    // 大地图信息（#300：--bigmap-test / --worldmap-test 需要 NewMapInfo）
    send(
        to_client,
        &server::map::NewMapInfo {
            map_index: 0,
            title: "新手村".to_string(),
            width: 400,
            height: 400,
            big_map: 1,
            movements: vec![server::map::MovementInfo {
                destination: 1,
                title: "比奇省".to_string(),
                location_x: 320,
                location_y: 300,
                icon: 0,
            }],
            npcs: vec![
                server::map::NpcMapInfo { object_id: 2001, name: "仓库管理员".to_string(), location_x: 352, location_y: 353, icon: 0, can_teleport_to: true },
                server::map::NpcMapInfo { object_id: 2002, name: "武器店老板".to_string(), location_x: 356, location_y: 352, icon: 0, can_teleport_to: true },
                server::map::NpcMapInfo { object_id: 2003, name: "药店老板".to_string(), location_x: 352, location_y: 355, icon: 0, can_teleport_to: false },
                server::map::NpcMapInfo { object_id: 2004, name: "Merchant".to_string(), location_x: 355, location_y: 353, icon: 0, can_teleport_to: true },
            ],
        },
    );
    // 本地玩家（职业/性别随所选角色）
    let (class, gender) = match char_index {
        1 => (MirClass::Wizard, MirGender::Female),
        _ => (MirClass::Warrior, MirGender::Male),
    };
    // MOCK_SHOWCASE=1：表现层验收模式（装备/武器/翅膀/坐骑/光照，供 #28 截图验收）
    let showcase = std::env::var("MOCK_SHOWCASE").as_deref() == Ok("1");
    send(
        to_client,
        &server::objects::ObjectPlayer {
            object_id: 100,
            name: match char_index {
                1 => "法师".to_string(),
                2 => "道士".to_string(),
                3 => "刺客".to_string(),
                _ => "刀客".to_string(),
            },
            guild_name: String::new(),
            guild_rank_name: String::new(),
            name_colour: 0,
            class,
            gender,
            level: stats.level,
            location_x: 354,
            location_y: 352,
            direction: MirDirection::Up,
            hair: 0,
            light: if showcase { 2 } else { 0 },
            weapon: if showcase { 20 } else { 0 },
            weapon_effect: if showcase { 5 } else { 0 },
            armour: if showcase { 10 } else { 0 },
            poison: PoisonType::empty(),
            dead: false,
            hidden: false,
            effect: SpellEffect::None,
            wing_effect: if showcase { 3 } else { 0 },
            extra: false,
            mount_type: if showcase { 1 } else { 0 },
            riding_mount: showcase,
            fishing: false,
            transform_type: 0,
            element_orb_effect: 0,
            element_orb_lvl: 0,
            element_orb_max: 0,
            buffs: vec![],
            level_effects: LevelEffects::NONE,
        },
    );

    // #619：远端玩家 bevy2char（--inspect-test 目标）
    send(
        to_client,
        &server::objects::ObjectPlayer {
            object_id: 120,
            name: "bevy2char".to_string(),
            guild_name: "测试行会".to_string(),
            guild_rank_name: "成员".to_string(),
            name_colour: 0,
            class: MirClass::Warrior,
            gender: MirGender::Male,
            level: 30,
            location_x: 355,
            location_y: 352,
            direction: MirDirection::Up,
            hair: 0,
            light: 0,
            weapon: 0,
            weapon_effect: 0,
            armour: 0,
            poison: PoisonType::empty(),
            dead: false,
            hidden: false,
            effect: SpellEffect::None,
            wing_effect: 0,
            extra: false,
            mount_type: 0,
            riding_mount: false,
            fishing: false,
            transform_type: 0,
            element_orb_effect: 0,
            element_orb_lvl: 0,
            element_orb_max: 0,
            buffs: vec![],
            level_effects: LevelEffects::NONE,
        },
    );

    // 怪物
    for (id, img, x, y) in [(101u32, 1u16, 353i32, 352i32), (102, 5, 354, 351), (103, 9, 353, 353)] {
        send(
            to_client,
            &server::objects::ObjectMonster {
                object_id: id,
                name: format!("怪物{}", img),
                name_colour: 0,
                location_x: x,
                location_y: y,
                image: img,
                direction: MirDirection::Up,
                effect: 0,
                ai: 0,
                light: 0,
                dead: false,
                skeleton: false,
                poison: PoisonType::empty(),
                hidden: false,
                shock_time: 0,
                binding_shot_center: false,
                extra: false,
                extra_byte: 0,
                buffs: vec![],
            },
        );
    }

    // NPC
    send(
        to_client,
        &server::objects::ObjectNpc {
            object_id: 110,
            name: "仓库管理员".to_string(),
            name_colour: 0,
            image: 0,
            colour: 0,
            location_x: 352,
            location_y: 353,
            direction: MirDirection::Up,
            quest_ids: vec![],
        },
    );

    // Merchant NPC（--storage-test / --shop-test / --storage-unlock-test 按 Alchemist/Merchant 查找）
    send(
        to_client,
        &server::objects::ObjectNpc {
            object_id: 111,
            name: "Merchant".to_string(),
            name_colour: 0,
            image: 0,
            colour: 0,
            location_x: 355,
            location_y: 353,
            direction: MirDirection::Up,
            quest_ids: vec![],
        },
    );

    send_user_information(to_client, char_index, inventory, equipment, gold, stats);

    // #643：排行榜推送（--ranking-test）
    send(
        to_client,
        &server::special_systems::Rankings {
            my_rank: 0,
            rankings: vec![
                server::special_systems::RankInfo {
                    rank: 1,
                    player_id: 0,
                    player_name: "刀客".to_string(),
                    class: MirClass::Warrior as u8,
                    level: 30,
                    experience: 12000,
                },
                server::special_systems::RankInfo {
                    rank: 2,
                    player_id: 0,
                    player_name: "bevy2char".to_string(),
                    class: MirClass::Warrior as u8,
                    level: 30,
                    experience: 10000,
                },
            ],
        },
    );

    // 初始地面物品（拾取验收用）：金创药 @ (353,352)（玩家左侧 1 格）
    send(
        to_client,
        &server::drops::ObjectItem {
            object_id: 300,
            item: potion_item(1),
            location_x: 353,
            location_y: 352,
        },
    );

    // 世界地图配置（#300：C# S.WorldMapSetupInfo，进图下发一次）
    send(
        to_client,
        &server::map::WorldMapSetupInfo {
            enabled: true,
            world_maps: vec![
                server::map::WorldMapIcon { image_index: 0, title: "新手村".to_string(), map_index: 0 },
                server::map::WorldMapIcon { image_index: 1, title: "比奇省".to_string(), map_index: 1 },
                server::map::WorldMapIcon { image_index: 2, title: "盟重省".to_string(), map_index: 2 },
                server::map::WorldMapIcon { image_index: 3, title: "沙漠".to_string(), map_index: 3 },
            ],
            teleport_cost: 1000,
        },
    );
}
/// 按地图返回大地图信息（#300：--worldmap-test 多地图切换演示）
pub(crate) fn mock_map_info(map_index: i32) -> server::map::NewMapInfo {
    match map_index {
        0 => server::map::NewMapInfo {
            map_index: 0,
            title: "新手村".to_string(),
            width: 400,
            height: 400,
            big_map: 1,
            movements: vec![server::map::MovementInfo {
                destination: 1,
                title: "比奇省".to_string(),
                location_x: 320,
                location_y: 300,
                icon: 0,
            }],
            npcs: vec![
                server::map::NpcMapInfo { object_id: 2001, name: "仓库管理员".to_string(), location_x: 352, location_y: 353, icon: 0, can_teleport_to: true },
                server::map::NpcMapInfo { object_id: 2002, name: "武器店老板".to_string(), location_x: 356, location_y: 352, icon: 0, can_teleport_to: true },
                server::map::NpcMapInfo { object_id: 2003, name: "药店老板".to_string(), location_x: 352, location_y: 355, icon: 0, can_teleport_to: false },
                server::map::NpcMapInfo { object_id: 2004, name: "Merchant".to_string(), location_x: 355, location_y: 353, icon: 0, can_teleport_to: true },
            ],
        },
        1 => server::map::NewMapInfo {
            map_index: 1,
            title: "比奇省".to_string(),
            width: 800,
            height: 600,
            big_map: 1,
            movements: vec![server::map::MovementInfo {
                destination: 2,
                title: "盟重省".to_string(),
                location_x: 500,
                location_y: 200,
                icon: 0,
            }],
            npcs: vec![
                server::map::NpcMapInfo { object_id: 2101, name: "比奇城主".to_string(), location_x: 400, location_y: 300, icon: 0, can_teleport_to: true },
                server::map::NpcMapInfo { object_id: 2102, name: "比奇铁匠".to_string(), location_x: 410, location_y: 310, icon: 0, can_teleport_to: true },
            ],
        },
        2 => server::map::NewMapInfo {
            map_index: 2,
            title: "盟重省".to_string(),
            width: 900,
            height: 700,
            big_map: 1,
            movements: vec![],
            npcs: vec![
                server::map::NpcMapInfo { object_id: 2201, name: "盟重城主".to_string(), location_x: 450, location_y: 350, icon: 0, can_teleport_to: true },
                server::map::NpcMapInfo { object_id: 2202, name: "盟重药店".to_string(), location_x: 460, location_y: 360, icon: 0, can_teleport_to: false },
            ],
        },
        3 => server::map::NewMapInfo {
            map_index: 3,
            title: "沙漠".to_string(),
            width: 700,
            height: 500,
            big_map: 1,
            movements: vec![],
            npcs: vec![
                server::map::NpcMapInfo { object_id: 2301, name: "沙漠商队".to_string(), location_x: 350, location_y: 250, icon: 0, can_teleport_to: true },
            ],
        },
        _ => server::map::NewMapInfo {
            map_index,
            title: "未知地图".to_string(),
            width: 400,
            height: 400,
            big_map: 0,
            movements: vec![],
            npcs: vec![],
        },
    }
}





