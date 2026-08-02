// ============================================================================
// Mock 服务器（本地模拟，打通 登录→选角→进游戏→对象 协议流程）
// ============================================================================
// 与真实服务端同构：客户端内层包(PacketHeader+body) → mock 处理 →
// mock 回包（codec 外帧编码）。仅实现里程碑所需的最小闭环。

use crossbeam_channel::{Receiver, Sender};
use mir2_shared::data::client_data::SelectInfo;
use mir2_shared::enums::{
    ClientPacketIds, LevelEffects, MirClass, MirDirection, MirGender, PoisonType, SpellEffect,
};
use mir2_shared::packets::base::{serialize_packet, Packet, PacketHeader};
use mir2_shared::packets::{client, server};

use crate::network::codec;

pub fn spawn_mock(to_client: Sender<Vec<u8>>, from_client: Receiver<Vec<u8>>) {
    std::thread::Builder::new()
        .name("mock-server".into())
        .spawn(move || {
            let mut in_game = false;
            let mut characters: Vec<SelectInfo> = Vec::new();
            let mut last_ping = std::time::Instant::now();
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
                                        send_map_and_objects(&to_client, p.character_index);
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
                                            "[@CLOSE]" => vec![],
                                            _ => vec![
                                                "欢迎来到传奇 2（MOCK NPC）".to_string(),
                                                "[@SHOP] 商店".to_string(),
                                                "[@CLOSE] 关闭".to_string(),
                                            ],
                                        };
                                        if !page.is_empty() {
                                            send(&to_client, &server::npc_interaction::NPCResponse { page });
                                        }
                                    }
                                }
                                x if x == ClientPacketIds::Attack as i16 => {
                                    // 攻击反馈：怪物受击动画 + 伤害飘字
                                    if let Ok(p) = client::Attack::read_body(&mut cur) {
                                        tracing::info!("[MOCK] 攻击 dir={:?}", p.direction);
                                        let target = 101u32; // 第一个怪物
                                        send(
                                            &to_client,
                                            &server::combat::ObjectStruck {
                                                object_id: target,
                                                attacker_id: 100,
                                                location_x: 344,
                                                location_y: 352,
                                                direction: p.direction as u8,
                                            },
                                        );
                                        send(
                                            &to_client,
                                            &server::combat::DamageIndicator {
                                                damage: 15,
                                                damage_type: 0,
                                                object_id: target,
                                            },
                                        );
                                    }
                                }
                                x if x == ClientPacketIds::KeepAlive as i16 => {
                                    // 客户端心跳回应，无需处理
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
                    }
                    Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
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

/// 进图：MapChanged(n0) + 本地玩家 + 怪物/NPC
fn send_map_and_objects(to_client: &Sender<Vec<u8>>, char_index: i32) {
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
            location_x: 350,
            location_y: 350,
            direction: 0,
            map_dark_light: 0,
            music: 0,
            weather: 0,
        },
    );

    // 本地玩家（职业/性别随所选角色）
    let (class, gender) = match char_index {
        1 => (MirClass::Wizard, MirGender::Female),
        _ => (MirClass::Warrior, MirGender::Male),
    };
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
            level: 30,
            location_x: 350,
            location_y: 350,
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
    for (id, img, x, y) in [(101u32, 1u16, 344i32, 352i32), (102, 5, 356, 347), (103, 9, 345, 345)] {
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
            location_x: 353,
            location_y: 353,
            direction: MirDirection::Up,
            quest_ids: vec![],
        },
    );
}

