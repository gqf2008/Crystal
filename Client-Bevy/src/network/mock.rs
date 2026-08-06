// ============================================================================
// Mock 服务器（本地模拟，打通 登录→选角→进游戏→对象 协议流程）
// ============================================================================
// 与真实服务端同构：客户端内层包(PacketHeader+body) → mock 处理 →
// mock 回包（codec 外帧编码）。仅实现里程碑所需的最小闭环。

use crossbeam_channel::{Receiver, Sender};
use mir2_shared::data::client_data::{ClientMagic, ClientQuestProgress, SelectInfo};
use mir2_shared::data::item::ItemInfo;
use mir2_shared::enums::{
    ChatType, ClientPacketIds, HeroBehaviour, ItemType, LevelEffects, MirClass, MirDirection,
    MirGender, PoisonType, Spell, SpellEffect, Stat,
};
use std::collections::HashMap;
use mir2_shared::packets::base::{serialize_packet, Packet, PacketHeader};
use mir2_shared::packets::{client, server};

use crate::network::codec;

/// 玩家成长状态（#43 经验/升级闭环）
#[derive(Clone, Copy)]
struct MockPlayerStats {
    level: u16,
    exp: i64,
    max_exp: i64,
    hp: u32,
    mp: u32,
}

impl MockPlayerStats {
    fn new() -> Self {
        // MOCK_START_MP 可调初始魔法值（默认 420；设小可快速验证 蓝不足拒绝）
        let mp = std::env::var("MOCK_START_MP")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(420);
        Self { level: 30, exp: 12000, max_exp: Self::max_exp_for(30), hp: 850, mp }
    }
    /// 经验上限（C# Globals.Experience 近似：level^2*100/3，30 级 = 30000）
    fn max_exp_for(level: u16) -> i64 {
        (level as i64 * level as i64 * 100) / 3
    }
}

/// 任务状态（#44 任务闭环：击杀 稻草人 x3）
#[derive(Default)]
struct MockQuest {
    taken: bool,
    kills: u32,
    completed: bool,
}

const QUEST_ID: i32 = 1;
const QUEST_KILL_TARGET: u32 = 3;

/// 怪物属性（#49：差异化 HP/伤害/经验/是否主动）
struct MonsterDef {
    hp_max: i32,
    damage: u32,
    exp: u32,
    aggressive: bool,
}

fn monster_def(id: u32) -> MonsterDef {
    match id {
        // 稻草人：被动挨打（首个练手怪）
        101 => MonsterDef { hp_max: 100, damage: 0, exp: 2000, aggressive: false },
        // 多钩猫：追击 + 邻接攻击
        102 => MonsterDef { hp_max: 120, damage: 40, exp: 2500, aggressive: true },
        // 半兽人：追击 + 邻接攻击（更强）
        _ => MonsterDef { hp_max: 150, damage: 60, exp: 3000, aggressive: true },
    }
}

pub fn spawn_mock(to_client: Sender<Vec<u8>>, from_client: Receiver<Vec<u8>>) {
    std::thread::Builder::new()
        .name("mock-server".into())
        .spawn(move || {
            let mut in_game = false;
            let mut characters: Vec<SelectInfo> = Vec::new();
            let mut last_ping = std::time::Instant::now();
            // ---- 玩法闭环状态（#next：战斗/掉落/拾取/技能/怪物AI/装备）----
            let mut monster_hp: HashMap<u32, i32> = [101u32, 102, 103]
                .into_iter()
                .map(|id| (id, monster_def(id).hp_max))
                .collect();
            let mut monster_pos: HashMap<u32, (i32, i32)> = [
                (101u32, (353, 352)),
                (102, (356, 350)),
                (103, (351, 355)),
            ]
            .into_iter()
            .collect();
            /// 怪物最后受击时刻（#49 脱战回血）
            let mut monster_last_hit: HashMap<u32, std::time::Instant> = HashMap::new();
            /// 地面掉落物品（id, x, y）
            let mut ground_items: Vec<(u32, i32, i32, mir2_shared::data::item::UserItem)> = Vec::new();
            let mut next_item_id = 200u32;
            /// 死亡怪物重生计时（3 秒后）
            let mut respawn: HashMap<u32, std::time::Instant> = HashMap::new();
            let mut last_monster_ai = std::time::Instant::now();
            // 玩家背包/金币（购买/使用/拾取后更新）
            let mut player_inventory: Vec<Option<mir2_shared::data::item::UserItem>> = {
                let mut inv: Vec<Option<mir2_shared::data::item::UserItem>> = vec![None; 40];
                inv[0] = Some(wooden_sword_item()); // 木剑(221)
                inv[1] = Some(potion_item(10)); // 布衣
                inv[2] = Some(potion_item(1)); // 金创药（可喝）
                inv[3] = Some(book_item(34)); // 技能书：FireBall（#212）
                inv
            };
            let mut player_gold: u32 = 10000;
            let mut active_char_index: i32 = 0;
            // 玩家成长/任务状态（#43 经验升级 / #44 任务闭环）
            let mut player_stats = MockPlayerStats::new();
            let mut quest = MockQuest::default();
            // 装备（12 槽，Weapon=0/Armour=1）与死亡状态（#46/#47）
            let mut player_equipment: Vec<Option<mir2_shared::data::item::UserItem>> = vec![None; 12];
            let mut player_dead = false;
            // 英雄背包/出战状态（#203：供 HeroInventoryDialog mock 验证）
            let mut mock_hero_inventory: Vec<Option<mir2_shared::data::item::UserItem>> = vec![None; 40];
            mock_hero_inventory[0] = Some(potion_item(2)); // 金创药(中)
            mock_hero_inventory[1] = Some(potion_item(10)); // 布衣
            mock_hero_inventory[2] = Some(book_item(37)); // 英雄技能书：GreatFireBall（#218，SharedRust=37，uid 与玩家书区分）
            // 英雄装备（12 槽，服务端 EquipmentSlot::COUNT；#206）
            let mut mock_hero_equipment: Vec<Option<mir2_shared::data::item::UserItem>> = vec![None; 12];
            let mut mock_hero_active = false;
            // #212：技能书学习的技能（UserInformation 已带初始技能，这里追加）
            let mut mock_learned_magics: Vec<ClientMagic> = Vec::new();
            // #218：英雄技能书学习的技能
            let mut mock_hero_learned_magics: Vec<ClientMagic> = Vec::new();
            let mut player_dead_since: Option<std::time::Instant> = None;
            // #200：仓库密码（MOCK 默认 123456，解锁后仓库才打开）
            let mut mock_storage_password: Option<String> = Some("123456".to_string());
            // #283：首次击杀触发本地升级演示（LevelChanged + ObjectLeveled）
            let mut mock_leveled_up = false;
            // #222：轮回术复活请求状态
            let mut mock_reincarnation_offered = false;
            // #226：对象状态演示（隐藏/显形/坐下/击退/传送）状态机
            let mut object_state_stage: u8 = 0;
            let mut object_state_timer: Option<std::time::Instant> = None;
            // 怪物攻击伤害覆盖（默认 0 = 用怪物自身伤害；MOCK_PLAYER_DAMAGE 可调大测死亡/复活闭环）
            let player_damage = std::env::var("MOCK_PLAYER_DAMAGE")
                .ok()
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(0);
            loop {
                match from_client.recv_timeout(std::time::Duration::from_millis(100)) {
                    Ok(payload) => {
                        let mut cur = std::io::Cursor::new(&payload);
                        if let Ok(header) = PacketHeader::read_from(&mut cur) {
                            match header.opcode {
                                x if x == ClientPacketIds::Login as i16 => {
                                    if let Ok(p) = client::account::Login::read_body(&mut cur) {
                                        tracing::info!("[MOCK] 登录请求: {}", p.account_id);
                                        // 回 4 个角色（战士/法师/道士/刺客，对应 4 个选角槽位）
                                        characters = vec![
                                            SelectInfo {
                                                index: 0,
                                                name: "刀客".to_string(),
                                                level: 35,
                                                class: MirClass::Warrior,
                                                gender: MirGender::Male,
                                                last_access: chrono::Utc::now(),
                                            },
                                            SelectInfo {
                                                index: 1,
                                                name: "法师".to_string(),
                                                level: 28,
                                                class: MirClass::Wizard,
                                                gender: MirGender::Female,
                                                 last_access: chrono::Utc::now(),
                                             },
                                            SelectInfo {
                                                index: 2,
                                                name: "道士".to_string(),
                                                level: 30,
                                                class: MirClass::Taoist,
                                                gender: MirGender::Male,
                                                last_access: chrono::Utc::now(),
                                            },
                                            SelectInfo {
                                                index: 3,
                                                name: "刺客".to_string(),
                                                level: 26,
                                                class: MirClass::Assassin,
                                                gender: MirGender::Female,
                                                last_access: chrono::Utc::now(),
                                            },
                                        ];
                                        send(&to_client, &server::login::LoginSuccess { characters: characters.clone() });
                                    }
                                }
                                x if x == ClientPacketIds::StartGame as i16 => {
                                    if let Ok(p) = client::account::StartGame::read_body(&mut cur) {
                                        tracing::info!("[MOCK] 开始游戏 char={}", p.character_index);
                                        send(
                                            &to_client,
                                            &server::login::StartGame {
                                                result: 4,
                                                resolution: 0,
                                            },
                                        );
                                        active_char_index = p.character_index;
                        send_map_and_objects(&to_client, p.character_index, &player_inventory, &player_equipment, player_gold, player_stats);
                                        // #203：mock 英雄列表（1 个英雄，供 HeroInventoryDialog 验证）
                                        let hero_info = mir2_shared::data::client_data::ClientHeroInformation {
                                            index: 1,
                                            name: "英雄小刀".to_string(),
                                            level: 30,
                                            class: MirClass::Warrior,
                                            gender: MirGender::Male,
                                        };
                                        send(
                                            &to_client,
                                            &server::hero::ManageHeroes {
                                                max_count: 2,
                                                current_hero: None,
                                                heroes: vec![hero_info],
                                            },
                                        );
                                        in_game = true;
                                    }
                                }
                                x if x == ClientPacketIds::NewCharacter as i16 => {
                                    if let Ok(p) = client::NewCharacter::read_body(&mut cur) {
                                        tracing::info!("[MOCK] 新建角色: {} {:?} {:?}", p.name, p.class, p.gender);
                                        // 对齐原版：最多 4 个角色（Globals.MaxCharacterCount）
                                        if characters.len() >= 4 {
                                            send(&to_client, &server::NewCharacter { result: 4 });
                                            continue;
                                        }
                                        let idx = characters.len() as i32;
                                        let info = SelectInfo {
                                            index: idx,
                                            name: p.name.clone(),
                                            level: 1,
                                            class: p.class,
                                            gender: p.gender,
                                            last_access: chrono::Utc::now(),
                                        };
                                        characters.push(info.clone());
                                        send(
                                            &to_client,
                                            &server::NewCharacterSuccess {
                                                character: mir2_shared::packets::CharacterSummary {
                                                    index: idx,
                                                    name: p.name,
                                                    level: 1,
                                                    class: p.class,
                                                    gender: p.gender,
                                                    last_access: chrono::Utc::now(),
                                                },
                                            },
                                        );
                                    }
                                }
                                x if x == ClientPacketIds::DeleteCharacter as i16 => {
                                    if let Ok(p) = client::DeleteCharacter::read_body(&mut cur) {
                                        tracing::info!("[MOCK] 删除角色 idx={}", p.character_index);
                                        characters.retain(|c| c.index != p.character_index);
                                        send(
                                            &to_client,
                                            &server::DeleteCharacterSuccess {
                                                character_index: p.character_index,
                                            },
                                        );
                                    }
                                }
                                x if x == ClientPacketIds::CallNPC as i16 => {
                                    if let Ok(p) = client::CallNPC::read_body(&mut cur) {
                                        tracing::info!("[MOCK] NPC 对话: id={} key={}", p.object_id, p.key);
                                        // 简单对话页：欢迎 + 选项
                                        let key = p.key.to_uppercase();
                                        if key == "[@SHOP]" {
                                            // 商店商品（带 ItemInfo）
                                            use mir2_shared::data::item::ItemInfo;
                                            let mk = |index: i32, name: &str, price: u32| {
                                                mir2_shared::data::item::UserItem {
                                                    item_index: index,
                                                    count: 1,
                                                    info: Some(ItemInfo {
                                                        index,
                                                        name: name.to_string(),
                                                        price,
                                                        // 与背包物品一致：Items 库帧 = index
                                                        image: index as u16,
                                                        tool_tip: Some(format!("{}（商店演示物品）", name)),
                                                        ..Default::default()
                                                    }),
                                                    ..Default::default()
                                                }
                                            };
                                            send(
                                                &to_client,
                                                &server::npc_interaction::NPCGoods {
                                                    list: vec![
                                                        mk(1, "金创药(小)", 10),
                                                        mk(2, "魔法药(小)", 10),
                                                        mk(3, "随机传送卷", 100),
                                                    ],
                                                    rate: 1.0,
                                                    panel_type: mir2_shared::enums::PanelType::Buy,
                                                    hide_added_stats: false,
                                                },
                                            );
                                        }
                                        let page: Vec<String> = match key.as_str() {
                                            "[@SHOP]" => vec![
                                                "这里是商店（MOCK）".to_string(),
                                                "[@BUY] 购买".to_string(),
                                                "[@MAIN] 返回".to_string(),
                                            ],
                                            "[@QUEST]" => vec![
                                                "任务：击杀 3 只稻草人（怪物 101）".to_string(),
                                                "完成后回来交任务，奖励 100 金币 + 4000 经验".to_string(),
                                                "[@MAIN] 返回".to_string(),
                                            ],
                                            "[@STORAGE]" => vec![
                                                "这里是仓库（MOCK）".to_string(),
                                                "[@CLOSE] 关闭".to_string(),
                                            ],
                                            "[@CLOSE]" => vec![],
                                            _ => vec![
                                                "欢迎来到传奇 2（MOCK NPC）".to_string(),
                                                "[@SHOP] 商店".to_string(),
                                                "[@QUEST] 任务".to_string(),
                                                "[@STORAGE] 仓库".to_string(),
                                                "[@CLOSE] 关闭".to_string(),
                                            ],
                                        };
                                        // #200：仓库密码保护——有密码先弹解锁框，不泄露仓库内容
                                        if key == "[@STORAGE]" {
                                            if mock_storage_password.is_some() {
                                                send(&to_client, &server::npc::NPCStorage);
                                            } else {
                                                let storage_items: Vec<Option<mir2_shared::data::item::UserItem>> =
                                                    (0..80)
                                                        .map(|i| {
                                                            if i == 3 {
                                                                Some(mir2_shared::data::item::UserItem {
                                                                    item_index: 1,
                                                                    count: 1,
                                                                    info: Some(mir2_shared::data::item::ItemInfo {
                                                                        index: 1,
                                                                        name: "仓库演示物品".to_string(),
                                                                        price: 10,
                                                                        image: 1,
                                                                        tool_tip: Some("仓库演示物品".to_string()),
                                                                        ..Default::default()
                                                                    }),
                                                                    ..Default::default()
                                                                })
                                                            } else {
                                                                None
                                                            }
                                                        })
                                                        .collect();
                                                send(&to_client, &server::player::UserStorage { storage: storage_items });
                                            }
                                        }
                                        if !page.is_empty() {
                                            send(&to_client, &server::npc_interaction::NPCResponse { page });
                                        }
                                    }
                                }
                                // #285：聊天物品请求 → 回发 NewChatItem
                                x if x == ClientPacketIds::RequestChatItem as i16 => {
                                    if let Ok(p) = client::misc::RequestChatItem::read_body(&mut cur) {
                                        let uid = if p.chat_item_id == 0 { 9005 } else { p.chat_item_id };
                                        send(
                                            &to_client,
                                            &server::NewChatItem {
                                                item: mir2_shared::data::item::UserItem {
                                                    unique_id: uid,
                                                    item_index: 1,
                                                    count: 1,
                                                    info: Some(mir2_shared::data::item::ItemInfo {
                                                        index: 1,
                                                        name: "金创药(小)".to_string(),
                                                        price: 10,
                                                        image: 1,
                                                        tool_tip: Some("金创药(小)：恢复少量生命。".to_string()),
                                                        ..Default::default()
                                                    }),
                                                    ..Default::default()
                                                },
                                            },
                                        );
                                        tracing::info!("[MOCK] 聊天物品请求 uid={}", uid);
                                    }
                                }
                                // #222：觉醒材料/执行（MOCK 回发成功，供 --awake-test）
                                x if x == ClientPacketIds::AwakeningNeedMaterials as i16 => {
                                    if let Ok(p) = client::misc::AwakeningNeedMaterials::read_body(&mut cur) {
                                        send(
                                            &to_client,
                                            &server::awakening_system::AwakeningNeedMaterials {
                                                item_id: 221,
                                                materials: vec![
                                                    mir2_shared::packets::server::awakening_system::MaterialInfo {
                                                        item_id: 1,
                                                        count: 2,
                                                    },
                                                ],
                                            },
                                        );
                                        tracing::info!(
                                            "⚒️ [MOCK] 觉醒材料请求 uid={} type={:?}",
                                            p.unique_id,
                                            p.awake_type
                                        );
                                    }
                                }
                                x if x == ClientPacketIds::Awakening as i16 => {
                                    if let Ok(p) = client::misc::Awakening::read_body(&mut cur) {
                                        send(
                                            &to_client,
                                            &server::awakening_system::Awakening { result: 1, remove_id: 0 },
                                        );
                                        tracing::info!("⚒️ [MOCK] 执行觉醒 uid={} -> 成功", p.unique_id);
                                    }
                                }
                                // #200：仓库密码解锁 / 设置 / 移除（MOCK）
                                x if x == ClientPacketIds::UnlockStorage as i16 => {
                                    if let Ok(p) = client::storage::UnlockStorage::read_body(&mut cur) {
                                        tracing::info!("[MOCK] 仓库解锁请求 pwd_len={}", p.password.len());
                                        let ok = mock_storage_password
                                            .as_ref()
                                            .map(|pwd| *pwd == p.password)
                                            .unwrap_or(false);
                                        send(
                                            &to_client,
                                            &server::StorageUnlockResult {
                                                result: if ok { 0 } else { 2 },
                                                has_password: mock_storage_password.is_some(),
                                            },
                                        );
                                        if ok {
                                            let storage_items: Vec<Option<mir2_shared::data::item::UserItem>> =
                                                (0..80).map(|_| None).collect();
                                            send(&to_client, &server::player::UserStorage { storage: storage_items });
                                        }
                                    }
                                }
                                x if x == ClientPacketIds::SetStoragePassword as i16 => {
                                    if let Ok(p) = client::storage::SetStoragePassword::read_body(&mut cur) {
                                        let ok = match &mock_storage_password {
                                            None => true,
                                            Some(cur) => *cur == p.current_password,
                                        };
                                        if ok {
                                            mock_storage_password = Some(p.new_password.clone());
                                        }
                                        send(
                                            &to_client,
                                            &server::StoragePasswordResult {
                                                result: if ok { 4 } else { 2 },
                                                removing: false,
                                                has_password: mock_storage_password.is_some(),
                                                last_set_time: 0,
                                            },
                                        );
                                    }
                                }
                                x if x == ClientPacketIds::RemoveStoragePassword as i16 => {
                                    if let Ok(p) = client::storage::RemoveStoragePassword::read_body(&mut cur) {
                                        let ok = mock_storage_password
                                            .as_ref()
                                            .map(|cur| *cur == p.current_password)
                                            .unwrap_or(false);
                                        if ok {
                                            mock_storage_password = None;
                                        }
                                        send(
                                            &to_client,
                                            &server::StoragePasswordResult {
                                                result: if ok { 4 } else { 2 },
                                                removing: true,
                                                has_password: false,
                                                last_set_time: 0,
                                            },
                                        );
                                    }
                                }
                                x if x == ClientPacketIds::Attack as i16 => {
                                    // 攻击反馈：怪物受击动画 + 伤害飘字 + 血量/死亡/掉落
                                    if player_dead {
                                        tracing::debug!("[MOCK] 死亡中忽略攻击");
                                        continue;
                                    }
                                    if let Ok(p) = client::Attack::read_body(&mut cur) {
                                        tracing::info!("[MOCK] 攻击 dir={:?}", p.direction);
                                        let target = 101u32; // 第一个怪物
                                        let hp = monster_hp.entry(target).or_insert(monster_def(target).hp_max);
                                        let damage = player_attack_damage(&player_equipment);
                                        *hp -= damage as i32;
                                        monster_last_hit.insert(target, std::time::Instant::now());
                                        send(
                                            &to_client,
                                            &server::combat::ObjectStruck {
                                                object_id: target,
                                                attacker_id: 100,
                                                location_x: monster_pos.get(&target).map(|v| v.0 as u32).unwrap_or(353),
                                                location_y: monster_pos.get(&target).map(|v| v.1 as u32).unwrap_or(352),
                                                direction: p.direction as u8,
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::combat::DamageIndicator {
                                                damage: damage as i32,
                                                damage_type: 0,
                                                object_id: target,
                                            },
                                        );
                                        if *hp <= 0 && !respawn.contains_key(&target) {
                                            let (ix, iy) = monster_pos.get(&target).copied().unwrap_or((353, 352));
                                            send(
                                                &to_client,
                                                &server::combat::ObjectDied {
                                                    object_id: target,
                                                    location_x: ix as u32,
                                                    location_y: iy as u32,
                                                    direction: 0,
                                                    death_type: 0,
                                                },
                                            );
                                            // #283：首次击杀触发本地升级（LevelChanged + ObjectLeveled 演示）
                                            if !mock_leveled_up {
                                                mock_leveled_up = true;
                                                player_stats.level += 1;
                                                let max_exp = MockPlayerStats::max_exp_for(player_stats.level);
                                                send(
                                                    &to_client,
                                                    &server::experience::LevelChanged {
                                                        level: player_stats.level,
                                                        experience: player_stats.exp,
                                                        max_experience: max_exp,
                                                    },
                                                );
                                                send(
                                                    &to_client,
                                                    &server::experience::ObjectLeveled {
                                                        object_id: 100,
                                                        level: player_stats.level,
                                                    },
                                                );
                                                tracing::info!("⬆️ [MOCK] 玩家升级到 Lv.{}", player_stats.level);
                                            }
                                            // 掉落：40% 金币 / 30% 药水 / 20% 装备 / 10% 无（#50）
                                            // 伪随机：时间微秒 + 击杀序号混合（毫秒级时间戳下 subsec_micros 末位仍有变化）
                                            let roll = (std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .map(|d| d.subsec_micros())
                                                .unwrap_or(0)
                                                + next_item_id)
                                                % 10;
                                            next_item_id += 1;
                                            if roll < 4 {
                                                let g = 20 + (next_item_id % 5) * 20;
                                                player_gold += g;
                                                send(&to_client, &server::drops::GainedGold { gold: g });
                                                send(
                                                    &to_client,
                                                    &server::chat::Chat {
                                                        message: format!("获得 {} 金币", g),
                                                        chat_type: ChatType::System,
                                                    },
                                                );
                                                tracing::info!("💰 [MOCK] 怪物 {} 掉落金币 +{}（余额 {}）", target, g, player_gold);
                                            } else if roll < 7 {
                                                let item_id = next_item_id;
                                                next_item_id += 1;
                                                ground_items.push((item_id, ix, iy, potion_item(1)));
                                                send(
                                                    &to_client,
                                                    &server::drops::ObjectItem {
                                                        object_id: item_id,
                                                        item: potion_item(1),
                                                        location_x: ix,
                                                        location_y: iy,
                                                    },
                                                );
                                                tracing::info!("💊 [MOCK] 怪物 {} 掉落药水 #{}", target, item_id);
                                            } else if roll < 9 {
                                                let item_id = next_item_id;
                                                next_item_id += 1;
                                                let equip = if next_item_id % 2 == 0 { 5 } else { 10 };
                                                ground_items.push((item_id, ix, iy, potion_item(equip)));
                                                send(
                                                    &to_client,
                                                    &server::drops::ObjectItem {
                                                        object_id: item_id,
                                                        item: potion_item(equip),
                                                        location_x: ix,
                                                        location_y: iy,
                                                    },
                                                );
                                                tracing::info!("⚔️ [MOCK] 怪物 {} 掉落装备 #{} (index {})", target, item_id, equip);
                                            } else {
                                                tracing::info!("🍃 [MOCK] 怪物 {} 无掉落", target);
                                            }
                                            send(&to_client, &server::objects::ObjectRemove { object_id: target });
                                            respawn.insert(target, std::time::Instant::now());
                                            on_kill_reward(&to_client, target, &mut player_stats, &mut quest);
                                            tracing::info!("💀 怪物 {} 死亡", target);
                                        }
                                    }
                                }
                                x if x == ClientPacketIds::Magic as i16 => {
                                    // 技能施放：回显魔法 + 对目标造成伤害
                                    if player_dead {
                                        tracing::debug!("[MOCK] 死亡中忽略技能");
                                        continue;
                                    }
                                    if let Ok(p) = client::combat::Magic::read_body(&mut cur) {
                                        tracing::info!("[MOCK] 魔法 spell={:?}", p.spell);
                                        // 耗蓝：施法扣 5 MP，不足拒绝（#51）
                                        const MAGIC_COST: u32 = 5;
                                        if player_stats.mp < MAGIC_COST {
                                            send(
                                                &to_client,
                                                &server::chat::Chat {
                                                    message: "魔法值不足".into(),
                                                    chat_type: ChatType::System,
                                                },
                                            );
                                            tracing::info!("🔮 [MOCK] 施法被拒：MP 不足（{}/{}）", player_stats.mp, MAGIC_COST);
                                            continue;
                                        }
                                        player_stats.mp -= MAGIC_COST;
                                        send(
                                            &to_client,
                                            &server::combat::HealthChanged {
                                                hp: player_stats.hp,
                                                mp: player_stats.mp,
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::magic_combat::MagicCast {
                                                spell: p.spell,
                                            },
                                        );
                                        // #276：施法后触发背包扩容（验证 ResizeInventory 解码/动态格子）
                                        send(
                                            &to_client,
                                            &server::ui_events::ResizeInventory { size: 56 },
                                        );
                                        let target = if p.target_id != 0 { p.target_id } else { 101u32 };
                                        let hp = monster_hp.entry(target).or_insert(monster_def(target).hp_max);
                                        *hp -= 20;
                                        monster_last_hit.insert(target, std::time::Instant::now());
                                        send(
                                            &to_client,
                                            &server::combat::DamageIndicator {
                                                damage: 20,
                                                damage_type: 0,
                                                object_id: target,
                                            },
                                        );
                                        // #224：战斗表现层——怪物反击施法/远程攻击/命中特效（验证
                                        // ObjectMagic/ObjectProjectile/ObjectEffect/ObjectRangeAttack 解码渲染）
                                        let (mx, my) = monster_pos.get(&target).copied().unwrap_or((353, 352));
                                        let (px, py): (i32, i32) = (354, 352);
                                        send(
                                            &to_client,
                                            &server::magic_combat::ObjectMagic {
                                                object_id: target,
                                                location_x: mx,
                                                location_y: my,
                                                direction: MirDirection::Down,
                                                spell: Spell::FireBall,
                                                target_id: 100,
                                                target_x: px,
                                                target_y: py,
                                                cast: true,
                                                level: 1,
                                                self_broadcast: false,
                                                secondary_target_ids: Vec::new(),
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::magic_combat::ObjectProjectile {
                                                spell: Spell::FireBall,
                                                source: target,
                                                destination: 100,
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::magic_combat::ObjectEffect {
                                                object_id: 100,
                                                effect: mir2_shared::enums::SpellEffect::Critical,
                                                effect_type: 0,
                                                delay_time: 0,
                                                time: 500,
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::combat::ObjectRangeAttack {
                                                object_id: target,
                                                location_x: mx as u32,
                                                location_y: my as u32,
                                                direction: 2,
                                                target_id: 100,
                                                target_x: px as u32,
                                                target_y: py as u32,
                                                spell: 0,
                                                spell_level: 0,
                                            },
                                        );
                                        // #226：触发对象状态演示（2s 后开始隐藏/击退/坐下）
                                        object_state_stage = 1;
                                        object_state_timer = Some(std::time::Instant::now());
                                        // #228：物品状态同步——耐久变化 + 获得物品入包 + 删除物品
                                        send(
                                            &to_client,
                                            &server::experience::DuraChanged {
                                                unique_id: 9005,
                                                current_dura: 3,
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::drops::GainedItem {
                                                item: potion_item(2),
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::experience::DeleteItem {
                                                unique_id: 9010,
                                                count: 1,
                                            },
                                        );
                                        // #230：地图特效 + 服务端音效 + 计时器
                                        send(
                                            &to_client,
                                            &server::object::MapEffect {
                                                location: mir2_shared::Point { x: mx, y: my },
                                                effect: mir2_shared::enums::SpellEffect::Critical,
                                                value: 0,
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::ui_events::PlaySound { sound_id: 10060 },
                                        );
                                        send(
                                            &to_client,
                                            &server::ui_events::SetTimer {
                                                timer_id: 1,
                                                seconds: 5,
                                            },
                                        );
                                        // #232：上马（本地玩家）
                                        send(
                                            &to_client,
                                            &server::miscellaneous::MountUpdate {
                                                object_id: 100,
                                                mount_type: 1,
                                                riding_mount: true,
                                            },
                                        );
                                        // #234：邻接怪物立即反击（近战攻击动作）
                                        send(
                                            &to_client,
                                            &server::combat::ObjectAttack {
                                                object_id: 101,
                                                location_x: 353,
                                                location_y: 352,
                                                direction: 4,
                                                spell: 0,
                                                level: 0,
                                                attack_type: 0,
                                            },
                                        );
                                        // #236：玩家中毒（绿毒）
                                        send(
                                            &to_client,
                                            &server::buff::ObjectPoisoned {
                                                object_id: 100,
                                                poison: PoisonType::GREEN,
                                            },
                                        );
                                        // #238：对象蓝条（邻接怪物 80%）
                                        send(
                                            &to_client,
                                            &server::object::ObjectMana {
                                                object_id: 101,
                                                percent: 80,
                                            },
                                        );
                                        // #242：服务端同步开关技能状态（Slaying 开）
                                        send(
                                            &to_client,
                                            &server::magic::SpellToggle {
                                                spell: Spell::Slaying,
                                                can_use: true,
                                                hero: false,
                                            },
                                        );
                                        // #244：怪物位置掉金币
                                        send(
                                            &to_client,
                                            &server::drops::ObjectGold {
                                                object_id: 7001,
                                                gold: 150,
                                                location_x: mx,
                                                location_y: my,
                                            },
                                        );
                                        // #246：采集（怪物 101 位移 + Harvest 动作）
                                        send(
                                            &to_client,
                                            &server::objects::ObjectHarvest {
                                                object_id: 101,
                                                location_x: 352,
                                                location_y: 352,
                                                direction: MirDirection::Right,
                                            },
                                        );
                                        // #248：NPC 形象更新 + 声望增加
                                        send(
                                            &to_client,
                                            &server::npc_interaction::NPCImageUpdate {
                                                npc_id: 110,
                                                image: 2,
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::drops::GainedCredit { credit: 50 },
                                        );
                                        // #250：罗盘目标
                                        send(
                                            &to_client,
                                            &server::ui_events::SetCompass {
                                                location: (354, 350),
                                            },
                                        );
                                        // #252：潜行 / 等级特效 / 装饰
                                        send(
                                            &to_client,
                                            &server::movement::ObjectSneaking {
                                                object_id: 102,
                                                sneaking: true,
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::movement::ObjectLevelEffects {
                                                object_id: 103,
                                                level_effects: 1,
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::movement::ObjectDeco {
                                                object_id: 101,
                                                deco: 3,
                                                remove: false,
                                            },
                                        );
                                        // #254：小队成员位置
                                        send(
                                            &to_client,
                                            &server::group::SendMemberLocation {
                                                member_name: "队友A".to_string(),
                                                location: mir2_shared::Point { x: 356, y: 350 },
                                            },
                                        );
                                        // #258：物品升级 / 合成 / 技能删除 / 对象魔法 / 服务端消息
                                        let mut upgraded = potion_item(5);
                                        upgraded.unique_id = 9005;
                                        upgraded.item_index = 6; // 升级后物品索引变化
                                        send(
                                            &to_client,
                                            &server::item_operations::ItemUpgraded {
                                                item: upgraded,
                                            },
                                        );
                                        // #240：修理结果 + 镶嵌槽位（木剑 9005）——放在升级之后，避免覆盖升级后的物品
                                        send(
                                            &to_client,
                                            &server::item::ItemRepaired {
                                                unique_id: 9005,
                                                max_dura: 12,
                                                current_dura: 8,
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::item::ItemSlotSizeChanged {
                                                unique_id: 9005,
                                                slot_size: 1,
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::item_operations::CombineItem {
                                                grid: mir2_shared::enums::MirGridType::Inventory,
                                                id_from: 9010,
                                                id_to: 9005,
                                                success: true,
                                                destroy: true,
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::magic::RemoveMagic {
                                                spell: Spell::Fencing,
                                                hero: false,
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::magic_combat::ObjectSpell {
                                                object_id: 103,
                                                location_x: 353,
                                                location_y: 353,
                                                spell: Spell::FireBall,
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::ui_events::SendOutputMessage {
                                                message: "测试服务端消息".to_string(),
                                                message_type: 0,
                                            },
                                        );
                                        // #260：任务数据包
                                        send(
                                            &to_client,
                                            &server::quest::NewQuestInfo {
                                                quest: mir2_shared::data::client_data::ClientQuestInfo {
                                                    index: 1,
                                                    npc_index: 0,
                                                    name: "消灭稻草人".to_string(),
                                                    group: String::new(),
                                                    description: vec![],
                                                    task_description: vec!["击杀 稻草人 3/3".to_string()],
                                                    return_description: vec![],
                                                    completion_description: vec![],
                                                    min_level_needed: 1,
                                                    max_level_needed: 99,
                                                    quest_needed: 0,
                                                    class_needed: mir2_shared::enums::RequiredClass::WAR_WIZ_TAO,
                                                    quest_type: mir2_shared::enums::QuestType::General,
                                                    time_limit_in_seconds: 0,
                                                    reward_gold: 100,
                                                    reward_exp: 50,
                                                    reward_credit: 0,
                                                    rewards_fixed_item: vec![],
                                                    rewards_select_item: vec![],
                                                    finish_npc_index: 0,
                                                },
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::miscellaneous::ShareQuest { quest_id: 1 },
                                        );
                                        send(
                                            &to_client,
                                            &server::miscellaneous::GainedQuestItem { item_id: 1001 },
                                        );
                                        send(
                                            &to_client,
                                            &server::miscellaneous::DeleteQuestItem { item_id: 1002 },
                                        );
                                        // #262：配方 / Buff 暂停 / 杂项
                                        send(
                                            &to_client,
                                            &server::ui_events::NewRecipeInfo { recipe_id: 1 },
                                        );
                                        send(
                                            &to_client,
                                            &server::buff::PauseBuff {
                                                buff_type: mir2_shared::enums::BuffType::Haste,
                                                object_id: 100,
                                                paused: true,
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::item::RefreshItem { item: potion_item(1) },
                                        );
                                        send(
                                            &to_client,
                                            &server::ui_events::SetBindingShot { enabled: true },
                                        );
                                        // #264：改名 / 杂项
                                        send(
                                            &to_client,
                                            &server::player::ObjectName {
                                                object_id: 101,
                                                name: "稻草人·改".to_string(),
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::miscellaneous::UserName {
                                                object_id: 100,
                                                name: "刀客·改名".to_string(),
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::miscellaneous::ChatItemStats {
                                                unique_id: 9005,
                                                stats: "攻击 10-20".to_string(),
                                            },
                                        );
                                        // #285：聊天物品信息（聊天链接解析用）
                                        send(
                                            &to_client,
                                            &server::NewChatItem {
                                                item: mir2_shared::data::item::UserItem {
                                                    unique_id: 9005,
                                                    item_index: 1,
                                                    count: 1,
                                                    info: Some(mir2_shared::data::item::ItemInfo {
                                                        index: 1,
                                                        name: "金创药(小)".to_string(),
                                                        price: 10,
                                                        image: 1,
                                                        tool_tip: Some("金创药(小)：恢复少量生命。".to_string()),
                                                        ..Default::default()
                                                    }),
                                                    ..Default::default()
                                                },
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::miscellaneous::InTrapRock { in_trap: true },
                                        );
                                        send(
                                            &to_client,
                                            &server::awakening_system::NPCAwakening {},
                                        );
                                        // #268：杂项协议
                                        send(
                                            &to_client,
                                            &server::miscellaneous::BaseStatsInfo {
                                                stats: vec![10, 20, 30],
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::rental_system::GetRentedItems { items: vec![] },
                                        );
                                        send(
                                            &to_client,
                                            &server::npc::NPCRequestInput {
                                                npc_id: 110,
                                                page_name: "Amount".to_string(),
                                            },
                                        );
                                        // #270：冲刺攻击 / 传送 / 杂项
                                        send(
                                            &to_client,
                                            &server::movement::UserDashAttack {
                                                location_x: 353,
                                                location_y: 352,
                                                direction: MirDirection::Down,
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::movement::ObjectDashAttack {
                                                object_id: target,
                                                location_x: 353,
                                                location_y: 352,
                                                direction: MirDirection::Down,
                                                distance: 1,
                                            },
                                        );
                                        send(&to_client, &server::map::TeleportIn {});
                                        send(
                                            &to_client,
                                            &server::trade::TradeAccept {
                                                name: "队友A".to_string(),
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::mail_system::MailSent { result: 1 },
                                        );
                                        send(
                                            &to_client,
                                            &server::experience::HeroLevelChanged {
                                                level: 5,
                                                experience: 0,
                                                max_experience: 100,
                                            },
                                        );
                                        // #274：智能宠物协议
                                        send(
                                            &to_client,
                                            &server::special_systems::NewIntelligentCreature {
                                                creature_type: mir2_shared::enums::IntelligentCreatureType::BabyPig,
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::special_systems::IntelligentCreatureEnableRename {
                                                can_rename: true,
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::special_systems::IntelligentCreaturePickup {
                                                enabled: true,
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::ui_events::ResizeStorage { size: 80 },
                                        );
                                        // #256：公告 + 杂项协议
                                        send(
                                            &to_client,
                                            &server::ui_events::UpdateNotice {
                                                notice: mir2_shared::data::notice::Notice {
                                                    title: "服务器公告".to_string(),
                                                    message: "欢迎来到传奇2
本次为 Bevy 迁移测试".to_string(),
                                                },
                                            },
                                        );
                                        tracing::info!("[MOCK] 已发送 UpdateNotice");
                                        send(
                                            &to_client,
                                            &server::item::ItemSealChanged {
                                                grid_type: mir2_shared::enums::MirGridType::Inventory,
                                                unique_id: 9005,
                                                expiry_date: 0,
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::social_system::TransformUpdate {
                                                object_id: 103,
                                                transform_type: 2,
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::ui_events::Opendoor {
                                                door_index: 1,
                                                close: false,
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::ui_events::OpenBrowser {
                                                url: "https://github.com/gqf2008/Crystal".to_string(),
                                            },
                                        );
                                        if *hp <= 0 && !respawn.contains_key(&target) {
                                            let (ix, iy) = monster_pos.get(&target).copied().unwrap_or((353, 352));
                                            send(
                                                &to_client,
                                                &server::combat::ObjectDied {
                                                    object_id: target,
                                                    location_x: ix as u32,
                                                    location_y: iy as u32,
                                                    direction: 0,
                                                    death_type: 0,
                                                },
                                            );
                                            // 掉落：40% 金币 / 30% 药水 / 20% 装备 / 10% 无（#50）
                                            // 伪随机：时间微秒 + 击杀序号混合（毫秒级时间戳下 subsec_micros 末位仍有变化）
                                            let roll = (std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .map(|d| d.subsec_micros())
                                                .unwrap_or(0)
                                                + next_item_id)
                                                % 10;
                                            next_item_id += 1;
                                            if roll < 4 {
                                                let g = 20 + (next_item_id % 5) * 20;
                                                player_gold += g;
                                                send(&to_client, &server::drops::GainedGold { gold: g });
                                                send(
                                                    &to_client,
                                                    &server::chat::Chat {
                                                        message: format!("获得 {} 金币", g),
                                                        chat_type: ChatType::System,
                                                    },
                                                );
                                                tracing::info!("💰 [MOCK] 怪物 {} 掉落金币 +{}（余额 {}）", target, g, player_gold);
                                            } else if roll < 7 {
                                                let item_id = next_item_id;
                                                next_item_id += 1;
                                                ground_items.push((item_id, ix, iy, potion_item(1)));
                                                send(
                                                    &to_client,
                                                    &server::drops::ObjectItem {
                                                        object_id: item_id,
                                                        item: potion_item(1),
                                                        location_x: ix,
                                                        location_y: iy,
                                                    },
                                                );
                                                tracing::info!("💊 [MOCK] 怪物 {} 掉落药水 #{}", target, item_id);
                                            } else if roll < 9 {
                                                let item_id = next_item_id;
                                                next_item_id += 1;
                                                let equip = if next_item_id % 2 == 0 { 5 } else { 10 };
                                                ground_items.push((item_id, ix, iy, potion_item(equip)));
                                                send(
                                                    &to_client,
                                                    &server::drops::ObjectItem {
                                                        object_id: item_id,
                                                        item: potion_item(equip),
                                                        location_x: ix,
                                                        location_y: iy,
                                                    },
                                                );
                                                tracing::info!("⚔️ [MOCK] 怪物 {} 掉落装备 #{} (index {})", target, item_id, equip);
                                            } else {
                                                tracing::info!("🍃 [MOCK] 怪物 {} 无掉落", target);
                                            }
                                            send(&to_client, &server::objects::ObjectRemove { object_id: target });
                                            respawn.insert(target, std::time::Instant::now());
                                            on_kill_reward(&to_client, target, &mut player_stats, &mut quest);
                                        }
                                    }
                                }
                                x if x == ClientPacketIds::PickUp as i16 => {
                                    // 拾取第一个地面物品 → 移除
                                    if client::item::PickUp::read_body(&mut cur).is_ok() {
                                        if let Some((id, _, _, item)) = ground_items.first().cloned() {
                                            ground_items.retain(|(i, _, _, _)| *i != id);
                                            send(&to_client, &server::objects::ObjectRemove { object_id: id });
                                            // 放入背包并同步客户端（#293 修复：拾取真正入包）
                                            if let Some(empty) = player_inventory.iter_mut().find(|s| s.is_none()) {
                                                *empty = Some(item.clone());
                                            }
                                            send(&to_client, &server::drops::GainedItem { item });
                                            send_user_information(
                                                &to_client,
                                                active_char_index,
                                                &player_inventory,
                                                &player_equipment,
                                                player_gold,
                                                player_stats,
                                            );
                                            tracing::info!("🎒 拾取地面物品 #{} 入包", id);
                                        } else {
                                            // 初始物品 300（无战斗掉落时）——同样入包
                                            send(&to_client, &server::objects::ObjectRemove { object_id: 300 });
                                            let item = potion_item(1);
                                            if let Some(empty) = player_inventory.iter_mut().find(|s| s.is_none()) {
                                                *empty = Some(item.clone());
                                            }
                                            send(&to_client, &server::drops::GainedItem { item });
                                            send_user_information(
                                                &to_client,
                                                active_char_index,
                                                &player_inventory,
                                                &player_equipment,
                                                player_gold,
                                                player_stats,
                                            );
                                            tracing::info!("🎒 拾取初始地面物品 #300 入包");
                                        }
                                    }
                                }
                                x if x == ClientPacketIds::EquipItem as i16 => {
                                    // #206：英雄装备（英雄背包 → 英雄装备槽）
                                    if let Ok(p) = client::item::EquipItem::read_body(&mut cur) {
                                        if p.grid == mir2_shared::enums::MirGridType::HeroInventory {
                                            let to = p.to as usize;
                                            let from = mock_hero_inventory
                                                .iter()
                                                .position(|s| s.as_ref().map(|i| i.unique_id) == Some(p.unique_id));
                                            let ok = if let Some(from) = from {
                                                if to < mock_hero_equipment.len() {
                                                    let item = mock_hero_inventory[from].take();
                                                    let old = mock_hero_equipment[to].take();
                                                    match (item, old) {
                                                        (Some(item), None) => {
                                                            mock_hero_equipment[to] = Some(item);
                                                            true
                                                        }
                                                        (Some(item), Some(old)) => {
                                                            if let Some(empty) = mock_hero_inventory.iter_mut().find(|s| s.is_none()) {
                                                                *empty = Some(old);
                                                                mock_hero_equipment[to] = Some(item);
                                                                true
                                                            } else {
                                                                mock_hero_equipment[to] = Some(old);
                                                                mock_hero_inventory[from] = Some(item);
                                                                false
                                                            }
                                                        }
                                                        (None, _) => false,
                                                    }
                                                } else {
                                                    false
                                                }
                                            } else {
                                                false
                                            };
                                            if ok {
                                                send_hero_information(&to_client, &mock_hero_inventory, &mock_hero_equipment, &mock_hero_learned_magics, true, 30, 20, 1, 2);
                                                tracing::info!("🦸 [MOCK] 英雄装备成功 uid={} -> 槽 {}", p.unique_id, p.to);
                                            } else {
                                                tracing::warn!("⚠️ [MOCK] 英雄装备失败 uid={} -> 槽 {}", p.unique_id, p.to);
                                            }
                                            continue;
                                        }
                                        // 玩家装备：背包 → 装备槽（服务端记录，供伤害/防御计算）
                                        let to = p.to as usize;
                                        let from = player_inventory
                                            .iter()
                                            .position(|s| s.as_ref().map(|i| i.unique_id) == Some(p.unique_id));
                                        let ok = if let Some(from) = from {
                                            if to < player_equipment.len() {
                                                let item = player_inventory[from].take();
                                                let old = player_equipment[to].take();
                                                if let Some(old) = old {
                                                    if let Some(empty) = player_inventory.iter_mut().find(|s| s.is_none()) {
                                                        *empty = Some(old);
                                                    }
                                                }
                                                player_equipment[to] = item;
                                                true
                                            } else {
                                                false
                                            }
                                        } else {
                                            false
                                        };
                                        send(
                                            &to_client,
                                            &server::item_operations::EquipItem {
                                                grid: p.grid,
                                                unique_id: p.unique_id,
                                                to: p.to,
                                                success: ok,
                                            },
                                        );
                                        if ok {
                                            send_user_information(&to_client, active_char_index, &player_inventory, &player_equipment, player_gold, player_stats);
                                            tracing::info!("⚔️ 装备成功 uid={} -> 槽 {}", p.unique_id, p.to);
                                        } else {
                                            tracing::warn!("⚠️ 装备失败 uid={} -> 槽 {}", p.unique_id, p.to);
                                        }
                                    }
                                }
                                x if x == ClientPacketIds::RemoveItem as i16 => {
                                    // #206：英雄卸下（英雄装备槽 → 英雄背包）
                                    if let Ok(p) = client::item::RemoveItem::read_body(&mut cur) {
                                        if p.grid == mir2_shared::enums::MirGridType::HeroEquipment {
                                            let slot = mock_hero_equipment
                                                .iter()
                                                .position(|s| s.as_ref().map(|i| i.unique_id) == Some(p.unique_id));
                                            let ok = if let Some(slot) = slot {
                                                let item = mock_hero_equipment[slot].take();
                                                if let Some(item) = item {
                                                    if let Some(empty) = mock_hero_inventory.iter_mut().find(|s| s.is_none()) {
                                                        *empty = Some(item);
                                                        true
                                                    } else {
                                                        mock_hero_equipment[slot] = Some(item);
                                                        false
                                                    }
                                                } else {
                                                    false
                                                }
                                            } else {
                                                false
                                            };
                                            if ok {
                                                send_hero_information(&to_client, &mock_hero_inventory, &mock_hero_equipment, &mock_hero_learned_magics, true, 30, 20, 1, 2);
                                                tracing::info!("🦸 [MOCK] 英雄卸下成功 uid={}", p.unique_id);
                                            } else {
                                                tracing::warn!("⚠️ [MOCK] 英雄卸下失败 uid={}", p.unique_id);
                                            }
                                            continue;
                                        }
                                        // 玩家卸下装备：装备槽 → 背包（服务端记录）
                                        let slot = player_equipment
                                            .iter()
                                            .position(|s| s.as_ref().map(|i| i.unique_id) == Some(p.unique_id));
                                        let ok = if let Some(slot) = slot {
                                            let item = player_equipment[slot].take();
                                            if let Some(item) = item {
                                                if let Some(empty) = player_inventory.iter_mut().find(|x| x.is_none()) {
                                                    *empty = Some(item);
                                                    true
                                                } else {
                                                    player_equipment[slot] = Some(item);
                                                    false
                                                }
                                            } else {
                                                false
                                            }
                                        } else {
                                            false
                                        };
                                        send(
                                            &to_client,
                                            &server::item_operations::RemoveItem {
                                                grid: p.grid,
                                                unique_id: p.unique_id,
                                                to: p.to,
                                                success: ok,
                                            },
                                        );
                                        if ok {
                                            send_user_information(&to_client, active_char_index, &player_inventory, &player_equipment, player_gold, player_stats);
                                            tracing::info!("🛡️ 卸下装备 uid={}（防御/伤害回落）", p.unique_id);
                                        }
                                    }
                                }
                                x if x == ClientPacketIds::Chat as i16 => {
                                    // 聊天：服务器回显（广播）
                                    if let Ok(p) = client::chat::Chat::read_body(&mut cur) {
                                        // #289：测试命令 → 服务端要求返回登录界面
                                        if p.message.trim().eq_ignore_ascii_case("@RETURNLOGIN") {
                                            send(&to_client, &server::ReturnToLogin);
                                            tracing::info!("🚪 [MOCK] 触发返回登录");
                                            continue;
                                        }
                                        let name = match active_char_index {
                                            1 => "法师",
                                            2 => "道士",
                                            3 => "刺客",
                                            _ => "刀客",
                                        };
                                        send(
                                            &to_client,
                                            &server::chat::Chat {
                                                message: format!("[{}] {}", name, p.message),
                                                chat_type: ChatType::Normal,
                                            },
                                        );
                                        tracing::info!("💬 [MOCK] 聊天: {}", p.message);
                                    }
                                }
                                x if x == ClientPacketIds::UseItem as i16 => {
                                    // 使用物品：金创药回血
                                    if let Ok(p) = client::item::UseItem::read_body(&mut cur) {
                                        let idx = player_inventory
                                            .iter()
                                            .position(|s| s.as_ref().map(|i| i.unique_id) == Some(p.unique_id));
                                        if let Some(idx) = idx {
                                            if let Some(item) = player_inventory[idx].take() {
                                                send(&to_client, &server::item_operations::UseItem { unique_id: p.unique_id });
                                                // #212：技能书 → 学习（shape = Spell）
                                                if item.info.as_ref().map(|i| i.item_type) == Some(ItemType::Book) {
                                                    let spell = item.info.as_ref().map(|i| i.shape).unwrap_or(0) as u8;
                                                    if !mock_learned_magics.iter().any(|m| m.spell as u8 == spell) {
                                                        let cm = ClientMagic {
                                                            name: format!("技能#{}", spell),
                                                            spell: mir2_shared::enums::Spell::try_from(spell).unwrap_or(Spell::None),
                                                            base_cost: 3,
                                                            level_cost: 1,
                                                            icon: 0,
                                                            level1: 1,
                                                            level2: 2,
                                                            level3: 3,
                                                            need1: 0,
                                                            need2: 0,
                                                            need3: 0,
                                                            level: 0,
                                                            key: 0,
                                                            experience: 0,
                                                            delay: 0,
                                                            range: 1,
                                                            cast_time: 0,
                                                        };
                                                        mock_learned_magics.push(cm.clone());
                                                        send(&to_client, &server::magic::NewMagic { magic: cm, hero: false });
                                                        send(
                                                            &to_client,
                                                            &server::chat::Chat {
                                                                message: "你学会了技能！".into(),
                                                                chat_type: ChatType::System,
                                                            },
                                                        );
                                                        tracing::info!("📖 [MOCK] 学会技能 spell={}", spell);
                                                    } else {
                                                        send(
                                                            &to_client,
                                                            &server::chat::Chat {
                                                                message: "你已经学会这个技能".into(),
                                                                chat_type: ChatType::System,
                                                            },
                                                        );
                                                    }
                                                    continue;
                                                }
                                                // 魔法药(小) index=2 回蓝，其余回血（#51）
                                                if item.item_index == 2 {
                                                    player_stats.mp = (player_stats.mp + 200).min(600);
                                                } else {
                                                    player_stats.hp = 1000;
                                                }
                                                send(&to_client, &server::combat::HealthChanged { hp: player_stats.hp, mp: player_stats.mp });
                                                tracing::info!("💊 [MOCK] 使用物品: {} (uid={}) hp={} mp={}", item.info.as_ref().map(|i| i.name.clone()).unwrap_or_default(), p.unique_id, player_stats.hp, player_stats.mp);
                                            }
                                        } else {
                                            // #218：英雄背包技能书学习
                                            let hidx = mock_hero_inventory
                                                .iter()
                                                .position(|s| s.as_ref().map(|i| i.unique_id) == Some(p.unique_id));
                                            if let Some(hidx) = hidx {
                                                if let Some(item) = mock_hero_inventory[hidx].take() {
                                                    if item.info.as_ref().map(|i| i.item_type) == Some(ItemType::Book) {
                                                        let spell = item.info.as_ref().map(|i| i.shape).unwrap_or(0) as u8;
                                                        if !mock_hero_learned_magics.iter().any(|m| m.spell as u8 == spell) {
                                                            let cm = ClientMagic {
                                                                name: format!("英雄技能#{}", spell),
                                                                spell: mir2_shared::enums::Spell::try_from(spell).unwrap_or(Spell::None),
                                                                base_cost: 3,
                                                                level_cost: 1,
                                                                icon: 0,
                                                                level1: 1,
                                                                level2: 2,
                                                                level3: 3,
                                                                need1: 0,
                                                                need2: 0,
                                                                need3: 0,
                                                                level: 0,
                                                                key: 0,
                                                                experience: 0,
                                                                delay: 0,
                                                                range: 1,
                                                                cast_time: 0,
                                                            };
                                                            mock_hero_learned_magics.push(cm.clone());
                                                            send(&to_client, &server::magic::NewMagic { magic: cm, hero: true });
                                                            send(
                                                                &to_client,
                                                                &server::chat::Chat {
                                                                    message: "英雄学会了技能！".into(),
                                                                    chat_type: ChatType::System,
                                                                },
                                                            );
                                                            // #220：模拟英雄施法升级（验证 MagicLeveled 路由到英雄技能面板）
                                                            send(&to_client, &server::magic::MagicLeveled {
                                                                object_id: 0x1000_0100,
                                                                spell: mir2_shared::enums::Spell::try_from(spell).unwrap_or(Spell::None),
                                                                level: 1,
                                                                experience: 0,
                                                            });
                                                            send_hero_information(&to_client, &mock_hero_inventory, &mock_hero_equipment, &mock_hero_learned_magics, true, 30, 20, 1, 2);
                                                            tracing::info!("🦸 [MOCK] 英雄学会技能 spell={}", spell);
                                                        } else {
                                                            send(
                                                                &to_client,
                                                                &server::chat::Chat {
                                                                    message: "英雄已经学会这个技能".into(),
                                                                    chat_type: ChatType::System,
                                                                },
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                x if x == ClientPacketIds::BuyItem as i16 => {
                                    // 商店购买：扣金币 + 背包加物品 + 重发 UserInformation
                                    if let Ok(p) = client::npc::BuyItem::read_body(&mut cur) {
                                        let price = match p.item_index { 1 => 10, 2 => 10, 3 => 100, _ => 10 };
                                        let total = price * p.count as u32;
                                        if player_gold >= total {
                                            player_gold -= total;
                                            // 放入背包第一个空位
                                            let mut placed = false;
                                            for slot in player_inventory.iter_mut() {
                                                if slot.is_none() {
                                                    *slot = Some(potion_item(p.item_index as i32));
                                                    placed = true;
                                                    break;
                                                }
                                            }
                                            if placed {
                                                send_user_information(&to_client, active_char_index, &player_inventory, &player_equipment, player_gold, player_stats);
                                                tracing::info!("🛒 [MOCK] 购买 item={} x{} 花费 {}，剩余金币 {}", p.item_index, p.count, total, player_gold);
                                            } else {
                                                send(&to_client, &server::chat::Chat { message: "背包已满".into(), chat_type: ChatType::System });
                                            }
                                        } else {
                                            send(&to_client, &server::chat::Chat { message: "金币不足".into(), chat_type: ChatType::System });
                                        }
                                    }
                                }
                                x if x == ClientPacketIds::AcceptQuest as i16 => {
                                    // 接受任务：回 ChangeQuest（任务：击杀 稻草人 x3）
                                    if let Ok(p) = client::quest::AcceptQuest::read_body(&mut cur) {
                                        quest.taken = true;
                                        quest.kills = 0;
                                        quest.completed = false;
                                        send(
                                            &to_client,
                                            &server::quest::ChangeQuest {
                                                quest: ClientQuestProgress {
                                                    id: QUEST_ID,
                                                    task_list: vec![format!("击杀 稻草人 0/{}", QUEST_KILL_TARGET)],
                                                    taken: true,
                                                    completed: false,
                                                    new: true,
                                                },
                                            },
                                        );
                                        tracing::info!("📜 [MOCK] 接受任务 {} (npc={})", p.quest_index, p.npc_index);
                                    }
                                }
                                x if x == ClientPacketIds::AbandonQuest as i16 => {
                                    // 放弃任务：清空任务状态
                                    if let Ok(p) = client::quest::AbandonQuest::read_body(&mut cur) {
                                        quest.taken = false;
                                        quest.kills = 0;
                                        quest.completed = false;
                                        tracing::info!("📜 [MOCK] 放弃任务 {}", p.quest_index);
                                    }
                                }
                                x if x == ClientPacketIds::FinishQuest as i16 => {
                                    // 交任务：完成后发 CompleteQuest + 奖励（金币/经验）
                                    if let Ok(p) = client::quest::FinishQuest::read_body(&mut cur) {
                                        if quest.taken && quest.completed {
                                            quest.taken = false;
                                            quest.completed = false;
                                            quest.kills = 0;
                                            send(&to_client, &server::miscellaneous::CompleteQuest { quest_id: QUEST_ID });
                                            player_gold += 100;
                                            grant_exp(&to_client, &mut player_stats, 4000);
                                            send(
                                                &to_client,
                                                &server::chat::Chat {
                                                    message: "🎁 任务完成奖励：100 金币 + 4000 经验".into(),
                                                    chat_type: ChatType::System,
                                                },
                                            );
                                            send_user_information(&to_client, active_char_index, &player_inventory, &player_equipment, player_gold, player_stats);
                                            tracing::info!("🎁 [MOCK] 交任务 {} 完成，奖励 100 金币 + 4000 经验", p.quest_index);
                                        } else {
                                            send(
                                                &to_client,
                                                &server::chat::Chat {
                                                    message: "任务尚未完成".into(),
                                                    chat_type: ChatType::System,
                                                },
                                            );
                                            tracing::info!("📜 [MOCK] 交任务 {} 被拒（未完成）", p.quest_index);
                                        }
                                    }
                                }
                                x if x == ClientPacketIds::TownRevive as i16 => {
                                    // 城镇复活（死亡 UI 按钮 / --auto-revive）
                                    if client::misc::TownRevive::read_body(&mut cur).is_ok() {
                                        if player_dead {
                                            revive_player(
                                                &to_client,
                                                active_char_index,
                                                &player_inventory,
                                                &player_equipment,
                                                player_gold,
                                                &mut player_stats,
                                                &mut player_dead,
                                                &mut player_dead_since,
                                            );
                                            tracing::info!("⛪ [MOCK] 城镇复活");
                                        } else {
                                            tracing::debug!("[MOCK] 未死亡忽略 TownRevive");
                                        }
                                    }
                                }
                                x if x == ClientPacketIds::ChangeHero as i16 => {
                                    // #203：切换英雄（0=主角色，1=英雄）
                                    let body = &payload[PacketHeader::HEADER_SIZE.min(payload.len())..];
                                    let index = body.first().copied().unwrap_or(0);
                                    mock_hero_active = index != 0;
                                    tracing::info!("[MOCK] 切换英雄 index={}", index);
                                    send(&to_client, &server::miscellaneous::ChangeHero { success: mock_hero_active });
                                    if mock_hero_active {
                                        send_hero_information(&to_client, &mock_hero_inventory, &mock_hero_equipment, &mock_hero_learned_magics, true, 30, 20, 1, 2);
                                    }
                                }
                                x if x == ClientPacketIds::TakeBackHeroItem as i16 => {
                                    // 英雄→主背包（C# [from i32][to i32]）
                                    let body = &payload[PacketHeader::HEADER_SIZE.min(payload.len())..];
                                    if body.len() >= 8 {
                                        let from = i32::from_le_bytes(body[0..4].try_into().unwrap_or([0; 4])) as usize;
                                        let to = i32::from_le_bytes(body[4..8].try_into().unwrap_or([0; 4])) as usize;
                                        if from < mock_hero_inventory.len() && to < player_inventory.len() {
                                            if let Some(item) = mock_hero_inventory[from].take() {
                                                if player_inventory[to].is_none() {
                                                    player_inventory[to] = Some(item);
                                                } else if let Some(empty) = player_inventory.iter_mut().find(|s| s.is_none()) {
                                                    *empty = Some(item);
                                                } else {
                                                    mock_hero_inventory[from] = Some(item);
                                                }
                                                tracing::info!("[MOCK] 英雄取回 {} -> 主背包 {}", from, to);
                                                send_user_information(&to_client, active_char_index, &player_inventory, &player_equipment, player_gold, player_stats);
                                                send_hero_information(&to_client, &mock_hero_inventory, &mock_hero_equipment, &mock_hero_learned_magics, true, 30, 20, 1, 2);
                                            }
                                        }
                                    }
                                }
                                x if x == ClientPacketIds::TransferHeroItem as i16 => {
                                    // 主背包→英雄（C# [from i32][to i32]）
                                    let body = &payload[PacketHeader::HEADER_SIZE.min(payload.len())..];
                                    if body.len() >= 8 {
                                        let from = i32::from_le_bytes(body[0..4].try_into().unwrap_or([0; 4])) as usize;
                                        let to = i32::from_le_bytes(body[4..8].try_into().unwrap_or([0; 4])) as usize;
                                        if from < player_inventory.len() && to < mock_hero_inventory.len() {
                                            if let Some(item) = player_inventory[from].take() {
                                                if mock_hero_inventory[to].is_none() {
                                                    mock_hero_inventory[to] = Some(item);
                                                } else if let Some(empty) = mock_hero_inventory.iter_mut().find(|s| s.is_none()) {
                                                    *empty = Some(item);
                                                } else {
                                                    player_inventory[from] = Some(item);
                                                }
                                                tracing::info!("[MOCK] 转移 主背包{} -> 英雄 {}", from, to);
                                                send_user_information(&to_client, active_char_index, &player_inventory, &player_equipment, player_gold, player_stats);
                                                send_hero_information(&to_client, &mock_hero_inventory, &mock_hero_equipment, &mock_hero_learned_magics, true, 30, 20, 1, 2);
                                            }
                                        }
                                    }
                                }
                                x if x == ClientPacketIds::AcceptReincarnation as i16 => {
                                    // #222：接受轮回术复活
                                    if player_dead && mock_reincarnation_offered {
                                        mock_reincarnation_offered = false;
                                        revive_player(
                                            &to_client,
                                            active_char_index,
                                            &player_inventory,
                                            &player_equipment,
                                            player_gold,
                                            &mut player_stats,
                                            &mut player_dead,
                                            &mut player_dead_since,
                                        );
                                        tracing::info!("🌀 [MOCK] 接受轮回术复活");
                                    }
                                }
                                x if x == ClientPacketIds::CancelReincarnation as i16 => {
                                    mock_reincarnation_offered = false;
                                    tracing::info!("🌀 [MOCK] 拒绝轮回术复活");
                                }
                                x if x == ClientPacketIds::KeepAlive as i16 => {
                                    // 客户端心跳回应，无需处理
                                }
                                x if x == ClientPacketIds::SpellToggle as i16 => {
                                    // #242：客户端切换开关技能 → 服务端回显 S.SpellToggle
                                    if let Ok(p) = client::combat::SpellToggle::read_body(&mut cur) {
                                        send(
                                            &to_client,
                                            &server::magic::SpellToggle {
                                                spell: p.spell,
                                                can_use: p.can_use,
                                                hero: false,
                                            },
                                        );
                                        tracing::info!(
                                            "🔄 [MOCK] 技能开关回显 {:?} can_use={}",
                                            p.spell,
                                            p.can_use
                                        );
                                    }
                                }
                                x if x == ClientPacketIds::NPCConfirmInput as i16 => {
                                    // #272：NPC 输入回执
                                    if let Ok(p) = client::npc::NPCConfirmInput::read_body(&mut cur) {
                                        tracing::info!(
                                            "⌨️ [MOCK] NPC 输入回执 npc={} page={} value={}",
                                            p.npc_id,
                                            p.page_name,
                                            p.value
                                        );
                                    }
                                }
                                x if x == ClientPacketIds::Turn as i16 => {}
                                x if x == ClientPacketIds::Walk as i16
                                    || x == ClientPacketIds::Run as i16 => {}
                                other => tracing::debug!("[MOCK] 未处理客户端包 {:04X}", other),
                            }
                        }
                    }
                    Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                        // 每 3s 发服务器心跳（客户端会回 KeepAlive）
                        if in_game && last_ping.elapsed() >= std::time::Duration::from_secs(3) {
                            last_ping = std::time::Instant::now();
                            send(&to_client, &server::connection::KeepAlive { time: 0 });
                        }
                        // 怪物 AI（每 3s）：死亡怪物重生 + 怪物 102 移动 + 怪物 103 攻击玩家
                        if in_game && last_monster_ai.elapsed() >= std::time::Duration::from_secs(3) {
                            last_monster_ai = std::time::Instant::now();
                            // 玩家死亡超 10s 自动复活（防卡死；--auto-revive 会更快触发 TownRevive）
                            if player_dead {
                                if let Some(since) = player_dead_since {
                                    if since.elapsed() >= std::time::Duration::from_secs(10) {
                                        revive_player(
                                            &to_client,
                                            active_char_index,
                                            &player_inventory,
                                            &player_equipment,
                                            player_gold,
                                            &mut player_stats,
                                            &mut player_dead,
                                            &mut player_dead_since,
                                        );
                                        tracing::info!("💚 [MOCK] 超时自动复活");
                                    }
                                }
                            }
                            // 重生
                            let due: Vec<u32> = respawn
                                .iter()
                                .filter(|(_, t)| t.elapsed() >= std::time::Duration::from_secs(3))
                                .map(|(id, _)| *id)
                                .collect();
                            for id in due {
                                respawn.remove(&id);
                                monster_hp.insert(id, monster_def(id).hp_max);
                                let (x, y) = monster_pos.get(&id).copied().unwrap_or((353, 352));
                                let img = match id { 101 => 1u16, 102 => 5, _ => 9 };
                                send(
                                    &to_client,
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
                                // #293：重生后重发名字（避免综合冒烟 NAME 测试因怪物死亡/重生丢失名字）
                                if id == 101 {
                                    send(
                                        &to_client,
                                        &server::player::ObjectName {
                                            object_id: 101,
                                            name: "稻草人·改".to_string(),
                                        },
                                    );
                                }
                                tracing::info!("♻️ 怪物 {} 重生", id);
                            }
                            // 怪物 AI（#49）：脱战回血 + 追击 + 邻接攻击
                            for id in [101u32, 102, 103] {
                                if respawn.contains_key(&id) {
                                    continue;
                                }
                                let def = monster_def(id);
                                // 脱战回血：8s 未受击 → 回满
                                if let Some(last) = monster_last_hit.get(&id) {
                                    if last.elapsed() >= std::time::Duration::from_secs(8) {
                                        if let Some(hp) = monster_hp.get_mut(&id) {
                                            if *hp < def.hp_max {
                                                *hp = def.hp_max;
                                                tracing::info!("💚 [MOCK] 怪物 {} 脱战回血（8s 未受击）", id);
                                            }
                                        }
                                    }
                                }
                                if !def.aggressive || player_dead {
                                    continue;
                                }
                                let (mx, my) = monster_pos.get(&id).copied().unwrap_or((353, 352));
                                let (px, py): (i32, i32) = (354, 352);
                                // 邻接攻击（切比雪夫距离 <=1）
                                if (mx - px).abs().max((my - py).abs()) <= 1 {
                                    let defence = player_defence(&player_equipment);
                                    let base = if player_damage > 0 { player_damage } else { def.damage };
                                    let dmg = base.saturating_sub(defence).max(1);
                                    player_stats.hp = player_stats.hp.saturating_sub(dmg);
                                    send(
                                        &to_client,
                                        &server::combat::ObjectStruck {
                                            object_id: 100,
                                            attacker_id: id,
                                            location_x: px as u32,
                                            location_y: py as u32,
                                            direction: 4,
                                        },
                                    );
                                    // #234：怪物近战攻击动作（攻击者挥砍动画）
                                    send(
                                        &to_client,
                                        &server::combat::ObjectAttack {
                                            object_id: id,
                                            location_x: mx as u32,
                                            location_y: my as u32,
                                            direction: 4,
                                            spell: 0,
                                            level: 0,
                                            attack_type: 0,
                                        },
                                    );
                                    send(
                                        &to_client,
                                        &server::combat::DamageIndicator {
                                            damage: dmg as i32,
                                            damage_type: 0,
                                            object_id: 100,
                                        },
                                    );
                                    send(
                                        &to_client,
                                        &server::combat::HealthChanged {
                                            hp: player_stats.hp,
                                            mp: player_stats.mp,
                                        },
                                    );
                                    tracing::info!("🗡️ 怪物 {} 攻击玩家 -{}（防御 {}）hp={}", id, dmg, defence, player_stats.hp);
                                    if player_stats.hp == 0 {
                                        player_dead = true;
                                        player_dead_since = Some(std::time::Instant::now());
                                        send(
                                            &to_client,
                                            &server::combat::Death {
                                                location_x: px as u32,
                                                location_y: py as u32,
                                                direction: 0,
                                            },
                                        );
                                        tracing::info!("💀 [MOCK] 玩家死亡，等待复活（10s 自动）");
                                        // #222：模拟道士轮回术 offer（S.RequestReincarnation）
                                        mock_reincarnation_offered = true;
                                        send(&to_client, &server::miscellaneous::RequestReincarnation {});
                                    }
                                } else {
                                    // 追击 1 格（8 方向）
                                    let (dx, dy) = ((px - mx).signum(), (py - my).signum());
                                    let (nx, ny) = (mx + dx, my + dy);
                                    monster_pos.insert(id, (nx, ny));
                                    let dir = match (dx, dy) {
                                        (-1, -1) => MirDirection::UpLeft,
                                        (0, -1) => MirDirection::Up,
                                        (1, -1) => MirDirection::UpRight,
                                        (-1, 0) => MirDirection::Left,
                                        (1, 0) => MirDirection::Right,
                                        (-1, 1) => MirDirection::DownLeft,
                                        (0, 1) => MirDirection::Down,
                                        _ => MirDirection::DownRight,
                                    };
                                    tracing::info!("🚶 怪物 {} 追击玩家 ({},{})->({},{}) {:?}", id, mx, my, nx, ny, dir);
                                    send(
                                        &to_client,
                                        &server::objects::ObjectWalk {
                                            object_id: id,
                                            location_x: nx,
                                            location_y: ny,
                                            direction: dir,
                                        },
                                    );
                                }
                            }
                        }
                    }
                    Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                }
                // #226：对象状态演示状态机（施法触发后每 2s 推进）
                if in_game && object_state_stage > 0 {
                    if let Some(t) = object_state_timer {
                        if t.elapsed() >= std::time::Duration::from_secs(2) {
                            object_state_timer = Some(std::time::Instant::now());
                            match object_state_stage {
                                1 => {
                                    send(&to_client, &server::map::ObjectHide { object_id: 102 });
                                    send(
                                        &to_client,
                                        &server::combat::ObjectPushed {
                                            object_id: 103,
                                            location_x: 352,
                                            location_y: 354,
                                            direction: 2,
                                        },
                                    );
                                    send(
                                        &to_client,
                                        &server::miscellaneous::ObjectSitDown {
                                            object_id: 101,
                                            direction: 2,
                                            location: (353, 352),
                                        },
                                    );
                                    // #234：对象冲刺/后跳动作
                                    send(
                                        &to_client,
                                        &server::combat::ObjectDash {
                                            object_id: 103,
                                            location_x: 353,
                                            location_y: 354,
                                            direction: 2,
                                        },
                                    );
                                    send(
                                        &to_client,
                                        &server::movement::ObjectBackStep {
                                            object_id: 101,
                                            location_x: 352,
                                            location_y: 352,
                                            direction: mir2_shared::enums::MirDirection::Down,
                                            distance: 1,
                                        },
                                    );
                                    tracing::info!("🌀 [MOCK] 对象状态: 隐藏102/击退103/坐下101 + 冲刺/后跳");
                                    object_state_stage = 2;
                                }
                                2 => {
                                    send(&to_client, &server::map::ObjectShow { object_id: 102 });
                                    send(
                                        &to_client,
                                        &server::map::ObjectTeleportOut {
                                            object_id: 103,
                                            teleport_type: 0,
                                        },
                                    );
                                    // #230：计时器到期（SetTimer 5s 后服务端主动关闭）
                                    send(
                                        &to_client,
                                        &server::ui_events::ExpireTimer { timer_id: 1 },
                                    );
                                    // #232：下马（本地玩家）
                                    send(
                                        &to_client,
                                        &server::miscellaneous::MountUpdate {
                                            object_id: 100,
                                            mount_type: 1,
                                            riding_mount: false,
                                        },
                                    );
                                    // #236：解毒
                                    send(
                                        &to_client,
                                        &server::buff::ObjectPoisoned {
                                            object_id: 100,
                                            poison: PoisonType::empty(),
                                        },
                                    );
                                    tracing::info!("🌀 [MOCK] 对象状态: 显形102/传送消失103 + 计时器到期 + 下马 + 解毒");
                                    object_state_stage = 3;
                                }
                                3 => {
                                    send(
                                        &to_client,
                                        &server::map::ObjectTeleportIn {
                                            object_id: 103,
                                            teleport_type: 0,
                                        },
                                    );
                                    tracing::info!("🌀 [MOCK] 对象状态: 传送出现103");
                                    object_state_stage = 4;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        })
        .expect("spawn mock thread");
}

/// 发送服务器包（serialize 内层 → codec 外帧编码）
fn send<P: Packet>(to_client: &Sender<Vec<u8>>, packet: &P) {
    let mut inner = Vec::new();
    if serialize_packet(&mut inner, packet).is_ok() {
        let mut framed = Vec::new();
        codec::encode(&inner, &mut framed);
        let _ = to_client.send(framed);
    }
}

/// 发放经验：GainExperience + 升级检测（LevelChanged/ObjectLeveled/聊天）
fn grant_exp(to_client: &Sender<Vec<u8>>, stats: &mut MockPlayerStats, amount: u32) {
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
fn on_kill_reward(
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
fn player_attack_damage(equipment: &[Option<mir2_shared::data::item::UserItem>]) -> u32 {
    let weapon = equipment.get(0).and_then(|s| s.as_ref());
    let (min_dc, max_dc) = weapon
        .and_then(|w| w.info.as_ref())
        .map(|i| (i.stats.get(Stat::MinDC).max(0) as u32, i.stats.get(Stat::MaxDC).max(0) as u32))
        .unwrap_or((0, 0));
    15 + if max_dc > 0 { min_dc + (max_dc - min_dc) / 2 } else { 0 }
}

/// 玩家防御：护甲槽 MaxAC（#47）
fn player_defence(equipment: &[Option<mir2_shared::data::item::UserItem>]) -> u32 {
    equipment
        .get(1)
        .and_then(|s| s.as_ref())
        .and_then(|i| i.info.as_ref())
        .map(|i| i.stats.get(Stat::MaxAC).max(0) as u32)
        .unwrap_or(0)
}

/// 复活玩家：清死亡状态 + Revived/ObjectRevived + 满血 UserInformation + 回安全区
#[allow(clippy::too_many_arguments)]
fn revive_player(
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
fn wooden_sword_item() -> mir2_shared::data::item::UserItem {
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

fn potion_item(index: i32) -> mir2_shared::data::item::UserItem {
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
fn book_item(spell: u8) -> mir2_shared::data::item::UserItem {
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

/// 发送玩家属性（UserInformation + HealthChanged）——购买/拾取后刷新用
fn send_user_information(
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
fn send_hero_information(
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
fn send_map_and_objects(
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

    send_user_information(to_client, char_index, inventory, equipment, gold, stats);


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
}


#[cfg(test)]
mod roundtrip_tests {
    use super::*;
    use std::io::Cursor;

    fn rt(index: i32) {
        let item = potion_item(index);
        let mut buf = Vec::new();
        item.write_to_with_info(&mut buf).unwrap();
        let mut cur = Cursor::new(&buf);
        let read = mir2_shared::data::item::UserItem::read_from_with_info(&mut cur).unwrap();
        let info = read.info.as_ref().expect("info");
        assert_eq!(info.item_type, item.info.as_ref().unwrap().item_type, "item_type mismatch for index {}", index);
    }

    #[test]
    fn test_potion_item_roundtrip() {
        rt(1);
        rt(2);
        rt(5);
        rt(10);
    }

    #[test]
    fn test_user_information_roundtrip_with_inventory() {
        use mir2_shared::enums::{MirClass, MirGender, MirDirection, HeroBehaviour, LevelEffects};
        let info = server::user::UserInformation {
            object_id: 100,
            real_id: 100,
            name: "刀客".to_string(),
            guild_name: String::new(),
            guild_rank: String::new(),
            name_colour: 0,
            class: MirClass::Warrior,
            gender: MirGender::Male,
            level: 30,
            location_x: 354,
            location_y: 352,
            direction: MirDirection::Up,
            hair: 0,
            hp: 850,
            mp: 420,
            experience: 12000,
            max_experience: 30000,
            level_effects: LevelEffects::NONE,
            has_hero: false,
            hero_behaviour: HeroBehaviour::Follow,
            inventory: Some({
                let mut inv: Vec<Option<mir2_shared::data::item::UserItem>> = vec![None; 40];
                inv[0] = Some(potion_item(5));
                inv[1] = Some(potion_item(10));
                inv
            }),
            equipment: Some(vec![None; 12]),
            quest_inventory: Some(vec![]),
            gold: 10000,
            credit: 0,
            has_expanded_storage: false,
            expanded_storage_expiry_time: 0,
            magics: vec![],
            summoned_creature_type: 0,
            creature_summoned: false,
            allow_observe: false,
            observer: false,
            max_hp: 850,
            max_mp: 420,
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
        };
        let mut buf = Vec::new();
        info.write_body(&mut buf).unwrap();
        let mut cur = Cursor::new(&buf);
        let read = server::user::UserInformation::read_body(&mut cur).unwrap();
        let inv = read.inventory.unwrap();
        assert!(inv[0].is_some(), "slot0 should have item");
        assert!(inv[1].is_some(), "slot1 should have item");
    }
}
