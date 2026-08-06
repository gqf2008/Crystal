use bevy::prelude::*;
use mir2_shared::packets::base::{Packet, PacketHeader};
use crate::network::*;
use crate::ui::login::AuthFeedback;
use super::*;

// 网络包解码分派（#72 拆分）：handle_auth 处理 arms_auth.rs 的服务端包分支。
// 由 packets.rs::handle_packet 调度器按 opcode 调用；返回 true 表示已处理。

#[allow(clippy::too_many_arguments, unused_variables)]
pub(crate) fn handle_auth(    net: &mut NetConnection,
    session: &mut SessionState,
    auth: &mut AuthFeedback,
    game_data: &mut GameData,
    net_objects: &mut MessageWriter<NetObject>,
    net_removals: &mut MessageWriter<NetObjectRemoved>,
    motions: &mut MessageWriter<NetMotion>,
    combat_evt: &mut MessageWriter<CombatEvent>,
    effects: &mut MessageWriter<PendingEffect>,
    server_events: &mut MessageWriter<ServerEvent>,
    control: &mut ControlState,
    next: &mut NextState<AppState>,
    payload: &[u8],) -> bool {
    use mir2_shared::packets::server::*;

    let mut cur = std::io::Cursor::new(payload);
    let Ok(header) = PacketHeader::read_from(&mut cur) else {
        return false;
    };
    let opcode = header.opcode;
    const HANDLED: &[i16] = &[ServerPacketIds::Connected as i16, ServerPacketIds::Disconnect as i16, ServerPacketIds::ClientVersion as i16, ServerPacketIds::NewAccount as i16, ServerPacketIds::ChangePassword as i16, ServerPacketIds::Login as i16, ServerPacketIds::LoginSuccess as i16, ServerPacketIds::StartGame as i16, ServerPacketIds::NewCharacter as i16, ServerPacketIds::NewCharacterSuccess as i16, ServerPacketIds::DeleteCharacter as i16, ServerPacketIds::DeleteCharacterSuccess as i16, ServerPacketIds::MapChanged as i16, ServerPacketIds::NewMapInfo as i16, ServerPacketIds::AwakeningNeedMaterials as i16, ServerPacketIds::AwakeningLockedItem as i16, ServerPacketIds::Awakening as i16, ServerPacketIds::Roll as i16, ServerPacketIds::ObjectPlayer as i16, ServerPacketIds::ObjectMonster as i16, ServerPacketIds::ObjectNpc as i16, ServerPacketIds::ObjectRemove as i16, ServerPacketIds::ObjectItem as i16, ServerPacketIds::UserInformation as i16, ServerPacketIds::HealthChanged as i16, ServerPacketIds::UserLocation as i16, ServerPacketIds::GainedGold as i16, ServerPacketIds::GainExperience as i16, ServerPacketIds::LoseGold as i16, ServerPacketIds::TimeOfDay as i16, ServerPacketIds::LogOutSuccess as i16, ServerPacketIds::ChangeAMode as i16, ServerPacketIds::ChangePMode as i16, ServerPacketIds::LevelChanged as i16, ServerPacketIds::ObjectTurn as i16, ServerPacketIds::ObjectWalk as i16, ServerPacketIds::ObjectRun as i16, ServerPacketIds::ObjectHide as i16, ServerPacketIds::ObjectShow as i16, ServerPacketIds::ObjectSitDown as i16, ServerPacketIds::Pushed as i16, ServerPacketIds::ObjectPushed as i16, ServerPacketIds::ObjectTeleportOut as i16, ServerPacketIds::ObjectTeleportIn as i16, ServerPacketIds::MountUpdate as i16, ServerPacketIds::ObjectAttack as i16, ServerPacketIds::UserDash as i16, ServerPacketIds::ObjectDash as i16, ServerPacketIds::UserDashFail as i16, ServerPacketIds::ObjectDashFail as i16, ServerPacketIds::UserBackStep as i16, ServerPacketIds::ObjectBackStep as i16, ServerPacketIds::UserAttackMove as i16, ServerPacketIds::Poisoned as i16, ServerPacketIds::ObjectPoisoned as i16, ServerPacketIds::Chat as i16, ServerPacketIds::ObjectChat as i16];
    let handled = HANDLED.contains(&opcode);
    match opcode {
        // ---- M7: 握手 ----
        x if x == ServerPacketIds::Connected as i16 => {
            tracing::info!("🔌 服务器已连接（Connected），发送 ClientVersion");
            if !net.client_version_sent {
                net.client_version_sent = true;
                net.send_packet(&mir2_shared::packets::client::connection::ClientVersion {
                    version_hash: net.client_version_hash.to_vec(),
                });
            }
        }
        x if x == ServerPacketIds::Disconnect as i16 => {
            // C# S.Disconnect.Reason：0=服务器关闭 1=顶号 2=包错误 3=崩溃
            let reason = match connection::Disconnect::read_body(&mut cur) {
                Ok(p) => p.reason,
                Err(_) => 0, // 空 body 容错（早期 ServerRust）
            };
            let msg = match reason {
                1 => "账号已在其他地方登录".to_string(),
                2 => "网络数据包错误".to_string(),
                3 => "服务器崩溃".to_string(),
                _ => "服务器已关闭连接".to_string(),
            };
            auth.login_error = Some(msg);
            net.auto_reconnect = false;
            net.disconnected = Some(format!("server-disconnect:{}", reason));
            next.set(AppState::Login);
            tracing::warn!("🚪 服务端断开: reason={}", reason);
        }
        x if x == ServerPacketIds::ClientVersion as i16 => {
            if let Ok(p) = connection::ClientVersion::read_body(&mut cur) {
                tracing::info!("🔑 ClientVersion 校验结果: {}", p.result);
            }
        }

        // ---- M7: 认证 ----
        x if x == ServerPacketIds::NewAccount as i16 => {
            if let Ok(p) = login::NewAccount::read_body(&mut cur) {
                let msg = match p.result {
                    8 => {
                        auth.new_account_success = true;
                        auth.new_account_error = None;
                        "注册成功，请登录".to_string()
                    }
                    0 => "服务器暂时关闭注册".to_string(),
                    1 => "账号格式错误（3-15位字母数字）".to_string(),
                    2 => "密码格式错误（5-15位字母数字）".to_string(),
                    3 => "邮箱格式错误".to_string(),
                    4 => "用户名过长".to_string(),
                    5 => "密保问题过长".to_string(),
                    6 => "密保答案过长".to_string(),
                    7 => "账号已存在".to_string(),
                    _ => format!("注册失败（{}）", p.result),
                };
                tracing::info!("📝 NewAccount result={} {}", p.result, msg);
                if p.result != 8 {
                    auth.new_account_error = Some(msg);
                }
            }
        }
        x if x == ServerPacketIds::ChangePassword as i16 => {
            if let Ok(p) = login::ChangePassword::read_body(&mut cur) {
                let msg = match p.result {
                    6 => {
                        auth.change_password_success = true;
                        auth.change_password_error = None;
                        "密码修改成功".to_string()
                    }
                    0 => "服务器关闭修改密码".to_string(),
                    1 => "账号格式错误".to_string(),
                    2 => "当前密码格式错误".to_string(),
                    3 => "新密码格式错误".to_string(),
                    4 => "账号不存在".to_string(),
                    5 => "当前密码错误".to_string(),
                    _ => format!("修改密码失败（{}）", p.result),
                };
                tracing::info!("🔑 ChangePassword result={} {}", p.result, msg);
                if p.result != 6 {
                    auth.change_password_error = Some(msg);
                }
            }
        }
        x if x == ServerPacketIds::Login as i16 => {
            if let Ok(p) = login::Login::read_body(&mut cur) {
                let msg = match p.result {
                    0 => "服务器禁止登录".to_string(),
                    1 => "账号格式错误".to_string(),
                    2 => "密码格式错误".to_string(),
                    3 => "账号不存在".to_string(),
                    4 => "密码错误".to_string(),
                    _ => format!("登录失败（{}）", p.result),
                };
                tracing::warn!("⛔ 登录失败 result={} {}", p.result, msg);
                net.state = NetState::Offline;
                auth.login_error = Some(msg);
                net.reconnecting = false;
            }
        }
        x if x == ServerPacketIds::LoginSuccess as i16 => {
            if let Ok(p) = login::LoginSuccess::read_body(&mut cur) {
                tracing::info!("✅ 登录成功，角色 {} 个", p.characters.len());
                session.characters = p.characters;
                session.select_reload = false;
                net.state = NetState::Select;
                auth.login_error = None;
                auth.login_success = true;
                // M58：重连成功后自动进入之前的角色
                if net.reconnecting {
                    net.reconnecting = false;
                    let saved = net.saved_character.lock().ok().map(|g| g.clone()).flatten();
                    if let Some(idx) = saved {
                        net.send_packet(&mir2_shared::packets::client::account::StartGame {
                            character_index: idx,
                        });
                        tracing::info!("🔌 自动重连成功，自动进入角色 idx={}", idx);
                    }
                }
            }
        }

        // ---- 角色管理 ----
        x if x == ServerPacketIds::StartGame as i16 => {
            if let Ok(p) = login::StartGame::read_body(&mut cur) {
                tracing::info!("✅ 开始游戏 result={}", p.result);
                net.state = NetState::InGame;
            }
        }
        x if x == ServerPacketIds::NewCharacter as i16 => {
            if let Ok(p) = account::NewCharacter::read_body(&mut cur) {
                tracing::info!("⛔ 新建角色被拒绝 result={}", p.result);
                session.character_error = Some(match p.result {
                    4 => "最多只能创建4个角色！".to_string(),
                    _ => "创建角色失败！".to_string(),
                });
            }
        }
        x if x == ServerPacketIds::NewCharacterSuccess as i16 => {
            if let Ok(p) = account::NewCharacterSuccess::read_body(&mut cur) {
                tracing::info!("✅ 新建角色成功: {}", p.character.name);
                session.characters.push(SelectInfo {
                    index: p.character.index,
                    name: p.character.name.clone(),
                    level: p.character.level,
                    class: p.character.class,
                    gender: p.character.gender,
                    last_access: p.character.last_access,
                });
                session.select_reload = true;
            }
        }
        x if x == ServerPacketIds::DeleteCharacter as i16 => {
            if let Ok(p) = account::DeleteCharacter::read_body(&mut cur) {
                tracing::warn!("⛔ 删除角色被拒绝 result={}", p.result);
                session.character_error = Some(match p.result {
                    0 => "删除失败：不能删除当前在线角色".to_string(),
                    1 => "删除失败：角色不存在".to_string(),
                    _ => format!("删除角色失败（{}）", p.result),
                });
            }
        }
        x if x == ServerPacketIds::DeleteCharacterSuccess as i16 => {
            if let Ok(p) = account::DeleteCharacterSuccess::read_body(&mut cur) {
                tracing::info!("🗑️ 删除角色成功 idx={}", p.character_index);
                session.characters.retain(|c| c.index != p.character_index);
                session.selected_index = None;
                session.select_reload = true;
            }
        }

        // ---- 地图与对象 ----
        x if x == ServerPacketIds::MapChanged as i16 => {
            if let Ok(p) = map::MapChanged::read_body(&mut cur) {
                tracing::info!(
                    "🗺️ MapChanged: {} ({},{})",
                    p.file_name,
                    p.location_x,
                    p.location_y
                );
                game_data.desired_map = Some(p.file_name);
                game_data.player_spawn =
                    Some((p.location_x as f32, p.location_y as f32, p.direction));
                server_events.write(ServerEvent::WeatherChanged { code: p.weather });
                next.set(AppState::Game);
            }
        }
        x if x == ServerPacketIds::NewMapInfo as i16 => {
            if let Ok(p) = map::NewMapInfo::read_body(&mut cur) {
                tracing::info!(
                    "🗺️ NewMapInfo: map={} title={} npcs={}",
                    p.map_index,
                    p.title,
                    p.npcs.len()
                );
                let npcs: Vec<crate::game::dialogs::big_map::NpcRow> = p
                    .npcs
                    .into_iter()
                    .map(|n| crate::game::dialogs::big_map::NpcRow {
                        object_id: n.object_id,
                        name: n.name,
                        x: n.location_x,
                        y: n.location_y,
                        icon: n.icon,
                        can_teleport_to: n.can_teleport_to,
                    })
                    .collect();
                server_events.write(ServerEvent::MapInfo {
                    map_index: p.map_index,
                    title: p.title.clone(),
                    npcs,
                });
            }
        }
        x if x == ServerPacketIds::AwakeningNeedMaterials as i16 => {
            if let Ok(p) = mir2_shared::packets::server::awakening_system::AwakeningNeedMaterials::read_body(
                &mut cur
            ) {
                tracing::info!(
                    "⚒️ 觉醒材料: item={} materials={:?}",
                    p.item_id,
                    p.materials
                        .iter()
                        .map(|m| format!("#{}x{}", m.item_id, m.count))
                        .collect::<Vec<_>>()
                );
                server_events.write(ServerEvent::AwakeningMaterials {
                    materials: p
                        .materials
                        .into_iter()
                        .map(|m| (m.item_id as i32, m.count))
                        .collect(),
                });
            }
        }
        x if x == ServerPacketIds::AwakeningLockedItem as i16 => {
            if let Ok(p) =
                mir2_shared::packets::server::awakening_system::AwakeningLockedItem::read_body(&mut cur)
            {
                tracing::info!("⚒️ 觉醒锁定: uid={} locked={}", p.unique_id, p.locked);
            }
        }
        x if x == ServerPacketIds::Awakening as i16 => {
            if let Ok(p) = mir2_shared::packets::server::awakening_system::Awakening::read_body(&mut cur) {
                let msg = match p.result {
                    1 => "觉醒成功".to_string(),
                    0 => format!("觉醒失败，物品已损毁 (uid={})", p.remove_id),
                    -1 => "觉醒失败".to_string(),
                    -2 => "已达最大觉醒等级".to_string(),
                    -3 => "金币不足".to_string(),
                    -4 => "材料不足".to_string(),
                    _ => format!("未知结果 {}", p.result),
                };
                tracing::info!("⚒️ 觉醒结果: {} -> {}", p.result, msg);
                server_events.write(ServerEvent::AwakeningResult {
                    result: p.result,
                    result_text: msg,
                });
            }
        }
        x if x == ServerPacketIds::Roll as i16 => {
            if let Ok(p) = mir2_shared::packets::server::ui_events::Roll::read_body(&mut cur) {
                tracing::info!(
                    "🎲 Roll: type={} result={} page={} auto={}",
                    p.r#type,
                    p.result,
                    p.page,
                    p.auto_roll
                );
                server_events.write(ServerEvent::Roll {
                    r#type: p.r#type,
                    page: p.page,
                    result: p.result,
                    auto_roll: p.auto_roll,
                    visible: true,
                    started_at: 0.0,
                    finished: false,
                });
            }
        }
        x if x == ServerPacketIds::ObjectPlayer as i16 => {
            match objects::ObjectPlayer::read_body(&mut cur) {
            Ok(p) => {
                net_objects.write(NetObject::Player {
                    object_id: p.object_id,
                    name: p.name,
                    class: p.class,
                    gender: p.gender,
                    location_x: p.location_x,
                    location_y: p.location_y,
                    direction: p.direction as u8,
                    hair: p.hair,
                    weapon: p.weapon,
                    weapon_effect: p.weapon_effect,
                    armour: p.armour,
                    wing_effect: p.wing_effect,
                    mount_type: p.mount_type,
                    is_mounted: p.riding_mount,
                });
            }
            Err(e) => {
                tracing::warn!("⚠️ ObjectPlayer 解析失败: {} (len={})", e, payload.len());
            }
            }
        }
        x if x == ServerPacketIds::ObjectMonster as i16 => {
            if let Ok(p) = objects::ObjectMonster::read_body(&mut cur) {
                net_objects.write(NetObject::Monster {
                    object_id: p.object_id,
                    name: p.name,
                    location_x: p.location_x,
                    location_y: p.location_y,
                    image: p.image,
                    direction: p.direction as u8,
                });
            }
        }
        x if x == ServerPacketIds::ObjectNpc as i16 => {
            if let Ok(p) = objects::ObjectNpc::read_body(&mut cur) {
                net_objects.write(NetObject::Npc {
                    object_id: p.object_id,
                    name: p.name,
                    image: p.image,
                    location_x: p.location_x,
                    location_y: p.location_y,
                    direction: p.direction as u8,
                });
            }
        }
        x if x == ServerPacketIds::ObjectRemove as i16 => {
            if let Ok(p) = objects::ObjectRemove::read_body(&mut cur) {
                tracing::debug!("🗑️ ObjectRemove id={}", p.object_id);
                net_removals.write(NetObjectRemoved(p.object_id));
            }
        }
        x if x == ServerPacketIds::ObjectItem as i16 => {
            match drops::ObjectItem::read_body(&mut cur) {
            Ok(p) => {
                let name = p
                    .item
                    .info
                    .as_ref()
                    .map(|i| i.name.clone())
                    .unwrap_or_else(|| format!("#{}", p.item.item_index));
                tracing::info!(
                    "📦 地面物品: {} (uid={}) @ ({},{})",
                    name,
                    p.item.unique_id,
                    p.location_x,
                    p.location_y
                );
                net_objects.write(NetObject::GroundItem {
                    object_id: p.object_id,
                    item: to_inv_item(&p.item),
                    location_x: p.location_x,
                    location_y: p.location_y,
                });
            }
            Err(e) => tracing::warn!("⚠️ ObjectItem 解析失败: {} (len={})", e, payload.len()),
            }
        }
        // ---- M8: 玩家状态 ----
        x if x == ServerPacketIds::UserInformation as i16 => {
            match user::UserInformation::read_body(&mut cur) {
                Ok(p) => {
                    tracing::info!(
                        "👤 UserInformation: {} Lv.{} hp={} mp={} exp={}/{} gold={}",
                        p.name,
                        p.level,
                        p.hp,
                        p.mp,
                        p.experience,
                        p.max_experience,
                        p.gold
                    );
                    // ---- 会话状态（网络层保留直写） ----
                    session.local_player_id = Some(p.object_id);
                    session.self_position = Some((p.location_x, p.location_y, p.direction as u8));

                    // ---- UI 数据：广播 ServerEvent，由各模块消费 ----
                    let magics: Vec<mir2_shared::data::client_data::ClientMagic> = p.magics.clone();
                    let inventory: Vec<Option<InvItem>> = p
                        .inventory
                        .as_ref()
                        .map(|inv| {
                            inv.iter()
                                .take(40)
                                .map(|slot| slot.as_ref().map(to_inv_item))
                                .collect()
                        })
                        .unwrap_or_default();
                    let equipment: Vec<Option<InvItem>> = p
                        .equipment
                        .as_ref()
                        .map(|eq| eq.iter().map(|slot| slot.as_ref().map(to_inv_item)).collect())
                        .unwrap_or_default();
                    let mut item_names: Vec<(i32, String)> = Vec::new();
                    for slot in p
                        .inventory
                        .iter()
                        .flat_map(|inv| inv.iter())
                        .chain(p.equipment.iter().flat_map(|eq| eq.iter()))
                    {
                        if let Some(slot) = slot {
                            if let Some(info) = &slot.info {
                                item_names.push((slot.item_index, info.name.clone()));
                            }
                        }
                    }
                    server_events.write(ServerEvent::UserInformation {
                        name: p.name.clone(),
                        level: p.level,
                        hp: p.hp,
                        mp: p.mp,
                        exp: p.experience,
                        max_exp: p.max_experience.max(1),
                        gold: p.gold,
                        class: p.class as u8,
                        object_id: p.object_id,
                        magics,
                        inventory,
                        equipment,
                        item_names,
                        max_hp: p.max_hp,
                        max_mp: p.max_mp,
                        ac: p.ac,
                        mac: p.mac,
                        dc: p.dc,
                        mc: p.mc,
                        sc: p.sc,
                        critical_rate: p.critical_rate,
                        critical_damage: p.critical_damage,
                        attack_speed: p.attack_speed,
                        accuracy: p.accuracy,
                        agility: p.agility,
                        luck: p.luck,
                        bag_weight: p.bag_weight,
                        wear_weight: p.wear_weight,
                        hand_weight: p.hand_weight,
                        magic_resist: p.magic_resist,
                        poison_resist: p.poison_resist,
                        health_recovery: p.health_recovery,
                        spell_recovery: p.spell_recovery,
                        poison_recovery: p.poison_recovery,
                        holy: p.holy,
                        freezing: p.freezing,
                        poison_atk: p.poison_atk,
                    });
                }
                Err(e) => {
                    tracing::warn!("⚠️ UserInformation 解析失败: {} (len={})", e, payload.len())
                }
            }
        }
        x if x == ServerPacketIds::HealthChanged as i16 => {
            if let Ok(p) = combat::HealthChanged::read_body(&mut cur) {
                server_events.write(server_event::from_packet::health_changed(&p));
            }
        }
        x if x == ServerPacketIds::UserLocation as i16 => {
            match user::UserLocation::read_body(&mut cur) {
                Ok(p) => {
                    tracing::info!("📍 UserLocation: ({},{}) dir={:?}", p.location_x, p.location_y, p.direction);
                    session.self_position = Some((p.location_x, p.location_y, p.direction as u8));
                }
                Err(e) => {
                    tracing::warn!("⚠️ UserLocation 解析失败: {}", e);
                }
            }
        }
        x if x == ServerPacketIds::GainedGold as i16 => {
            // GainedGold 是增量（击杀掉落），累加到余额
            if let Ok(p) = drops::GainedGold::read_body(&mut cur) {
                server_events.write(server_event::from_packet::gold_gained(&p));
                tracing::info!("💰 获得金币 +{}", p.gold);
            }
        }
        x if x == ServerPacketIds::GainExperience as i16 => {
            if let Ok(p) = experience::GainExperience::read_body(&mut cur) {
                server_events.write(server_event::from_packet::experience_gained(&p));
                tracing::info!("✨ 获得经验 +{}", p.amount);
            }
        }
        x if x == ServerPacketIds::LoseGold as i16 => {
            // C# S.LoseGold.Gold = 扣减金额，余额扣减
            if let Ok(p) = drops::LoseGold::read_body(&mut cur) {
                server_events.write(server_event::from_packet::gold_lost(&p));
                tracing::info!("💸 失去金币 -{}", p.gold);
            }
        }
        x if x == ServerPacketIds::TimeOfDay as i16 => {
            // C# S.TimeOfDay.Lights（SharedRust LightSetting 值 3..7）
            if let Ok(p) = TimeOfDay::read_body(&mut cur) {
                if let Ok(light) = mir2_shared::enums::LightSetting::try_from(p.lights) {
                    server_events.write(ServerEvent::TimeOfDay { light });
                    tracing::info!("🌗 服务端昼夜: {:?}", light);
                }
            }
        }
        x if x == ServerPacketIds::LogOutSuccess as i16 => {
            // C# S.LogOutSuccess：登出成功，返回选角界面
            if let Ok(_p) = player::LogOutSuccess::read_body(&mut cur) {
                server_events.write(ServerEvent::LogOutSuccess);
                next.set(AppState::Select);
                tracing::info!("🚪 登出成功，返回选角");
            }
        }
        x if x == ServerPacketIds::ChangeAMode as i16 => {
            // C# S.ChangeAMode：攻击模式确认
            if let Ok(p) = player::ChangeAMode::read_body(&mut cur) {
                server_events.write(ServerEvent::AttackModeChanged { mode: p.mode });
                let name = crate::game::combat::attack_mode_name(p.mode);
                server_events.write(ServerEvent::Chat {
                    text: format!("攻击模式：{}", name),
                    chat_type: mir2_shared::enums::ChatType::System,
                });
                tracing::info!("⚔️ 攻击模式确认: {:?}", p.mode);
            }
        }
        x if x == ServerPacketIds::ChangePMode as i16 => {
            // C# S.ChangePMode：宠物模式确认
            if let Ok(p) = player::ChangePMode::read_body(&mut cur) {
                let name = match p.mode {
                    mir2_shared::enums::PetMode::Both => "攻击和跟随",
                    mir2_shared::enums::PetMode::MoveOnly => "仅跟随",
                    mir2_shared::enums::PetMode::AttackOnly => "仅攻击",
                    mir2_shared::enums::PetMode::None => "不行动",
                    mir2_shared::enums::PetMode::FocusMasterTarget => "跟随目标",
                    _ => "未知",
                };
                server_events.write(ServerEvent::Chat {
                    text: format!("宠物模式：{}", name),
                    chat_type: mir2_shared::enums::ChatType::System,
                });
                tracing::info!("🐾 宠物模式确认: {:?}", p.mode);
            }
        }
        x if x == ServerPacketIds::LevelChanged as i16 => {
            if let Ok(p) = experience::LevelChanged::read_body(&mut cur) {
                server_events.write(server_event::from_packet::level_changed(&p));
                tracing::info!("⬆️ 升级 Lv.{} exp={}/{}", p.level, p.experience, p.max_experience);
            }
        }
        // ---- M8: 对象移动与聊天 ----
        x if x == ServerPacketIds::ObjectTurn as i16 => {
            if let Ok(p) = objects::ObjectTurn::read_body(&mut cur) {
                motions.write(NetMotion::Turn {
                    object_id: p.object_id,
                    x: p.location_x,
                    y: p.location_y,
                    dir: p.direction as u8,
                });
            }
        }
        x if x == ServerPacketIds::ObjectWalk as i16 => {
            if let Ok(p) = objects::ObjectWalk::read_body(&mut cur) {
                tracing::debug!("🚶 ObjectWalk id={} -> ({},{})", p.object_id, p.location_x, p.location_y);
                motions.write(NetMotion::Walk {
                    object_id: p.object_id,
                    x: p.location_x,
                    y: p.location_y,
                    dir: p.direction as u8,
                });
            }
        }
        x if x == ServerPacketIds::ObjectRun as i16 => {
            if let Ok(p) = objects::ObjectRun::read_body(&mut cur) {
                motions.write(NetMotion::Run {
                    object_id: p.object_id,
                    x: p.location_x,
                    y: p.location_y,
                    dir: p.direction as u8,
                });
            }
        }
        // #226：对象状态（隐藏/显形/坐下/击退/传送进出）
        x if x == ServerPacketIds::ObjectHide as i16 => {
            if let Ok(p) = map::ObjectHide::read_body(&mut cur) {
                server_events.write(ServerEvent::ObjectHidden {
                    object_id: p.object_id,
                });
                tracing::debug!("🙈 对象隐藏 id={}", p.object_id);
            }
        }
        x if x == ServerPacketIds::ObjectShow as i16 => {
            if let Ok(p) = map::ObjectShow::read_body(&mut cur) {
                server_events.write(ServerEvent::ObjectShown {
                    object_id: p.object_id,
                });
                tracing::debug!("🙉 对象显形 id={}", p.object_id);
            }
        }
        x if x == ServerPacketIds::ObjectSitDown as i16 => {
            if let Ok(p) = miscellaneous::ObjectSitDown::read_body(&mut cur) {
                server_events.write(ServerEvent::ObjectSitDown {
                    object_id: p.object_id,
                    direction: p.direction,
                });
                tracing::debug!("🪑 对象坐下 id={}", p.object_id);
            }
        }
        x if x == ServerPacketIds::Pushed as i16 => {
            if let Ok(p) = combat::Pushed::read_body(&mut cur) {
                let pid = session.local_player_id.unwrap_or(100);
                server_events.write(ServerEvent::ObjectPushed {
                    object_id: pid,
                    x: p.location_x as i32,
                    y: p.location_y as i32,
                    direction: p.direction,
                });
                tracing::debug!("💨 玩家被击退 ({},{})", p.location_x, p.location_y);
            }
        }
        x if x == ServerPacketIds::ObjectPushed as i16 => {
            if let Ok(p) = combat::ObjectPushed::read_body(&mut cur) {
                server_events.write(ServerEvent::ObjectPushed {
                    object_id: p.object_id,
                    x: p.location_x as i32,
                    y: p.location_y as i32,
                    direction: p.direction,
                });
                tracing::debug!(
                    "💨 对象被击退 id={} ({},{})",
                    p.object_id,
                    p.location_x,
                    p.location_y
                );
            }
        }
        x if x == ServerPacketIds::ObjectTeleportOut as i16 => {
            if let Ok(p) = map::ObjectTeleportOut::read_body(&mut cur) {
                server_events.write(ServerEvent::ObjectTeleportOut {
                    object_id: p.object_id,
                });
                tracing::debug!("🌀 对象传送消失 id={}", p.object_id);
            }
        }
        x if x == ServerPacketIds::ObjectTeleportIn as i16 => {
            if let Ok(p) = map::ObjectTeleportIn::read_body(&mut cur) {
                server_events.write(ServerEvent::ObjectTeleportIn {
                    object_id: p.object_id,
                });
                tracing::debug!("🌀 对象传送出现 id={}", p.object_id);
            }
        }

        // #232：坐骑上/下马（S.MountUpdate）
        x if x == ServerPacketIds::MountUpdate as i16 => {
            if let Ok(p) = miscellaneous::MountUpdate::read_body(&mut cur) {
                server_events.write(ServerEvent::MountUpdated {
                    object_id: p.object_id,
                    mount_type: p.mount_type,
                    is_mounted: p.riding_mount,
                });
                tracing::info!(
                    "🐴 坐骑更新: id={} type={} riding={}",
                    p.object_id,
                    p.mount_type,
                    p.riding_mount
                );
            }
        }

        // #234：对象动作（近战攻击/冲刺/后跳/攻击移动）
        x if x == ServerPacketIds::ObjectAttack as i16 => {
            if let Ok(p) = combat::ObjectAttack::read_body(&mut cur) {
                combat_evt.write(CombatEvent::Attack {
                    object_id: p.object_id,
                    direction: p.direction,
                });
                tracing::debug!("⚔️ 对象攻击 id={} dir={}", p.object_id, p.direction);
            }
        }
        x if x == ServerPacketIds::UserDash as i16 => {
            if let Ok(p) = combat::UserDash::read_body(&mut cur) {
                let pid = session.local_player_id.unwrap_or(100);
                server_events.write(ServerEvent::ObjectPushed {
                    object_id: pid,
                    x: p.location_x as i32,
                    y: p.location_y as i32,
                    direction: p.direction,
                });
                tracing::debug!("💨 玩家冲刺 ({},{})", p.location_x, p.location_y);
            }
        }
        x if x == ServerPacketIds::ObjectDash as i16 => {
            if let Ok(p) = combat::ObjectDash::read_body(&mut cur) {
                server_events.write(ServerEvent::ObjectPushed {
                    object_id: p.object_id,
                    x: p.location_x as i32,
                    y: p.location_y as i32,
                    direction: p.direction,
                });
                tracing::debug!(
                    "💨 对象冲刺 id={} ({},{})",
                    p.object_id,
                    p.location_x,
                    p.location_y
                );
            }
        }
        x if x == ServerPacketIds::UserDashFail as i16 => {
            if let Ok(p) = combat::UserDashFail::read_body(&mut cur) {
                tracing::debug!("🚫 玩家冲刺失败 ({},{})", p.location_x, p.location_y);
            }
        }
        x if x == ServerPacketIds::ObjectDashFail as i16 => {
            if let Ok(p) = combat::ObjectDashFail::read_body(&mut cur) {
                tracing::debug!("🚫 对象冲刺失败 id={}", p.object_id);
            }
        }
        x if x == ServerPacketIds::UserBackStep as i16 => {
            if let Ok(p) = movement::UserBackStep::read_body(&mut cur) {
                let pid = session.local_player_id.unwrap_or(100);
                server_events.write(ServerEvent::ObjectPushed {
                    object_id: pid,
                    x: p.location_x,
                    y: p.location_y,
                    direction: p.direction as u8,
                });
                tracing::debug!("↩️ 玩家后跳 ({},{})", p.location_x, p.location_y);
            }
        }
        x if x == ServerPacketIds::ObjectBackStep as i16 => {
            if let Ok(p) = movement::ObjectBackStep::read_body(&mut cur) {
                server_events.write(ServerEvent::ObjectPushed {
                    object_id: p.object_id,
                    x: p.location_x,
                    y: p.location_y,
                    direction: p.direction as u8,
                });
                tracing::debug!(
                    "↩️ 对象后跳 id={} ({},{})",
                    p.object_id,
                    p.location_x,
                    p.location_y
                );
            }
        }
        x if x == ServerPacketIds::UserAttackMove as i16 => {
            if let Ok(p) = movement::UserAttackMove::read_body(&mut cur) {
                let pid = session.local_player_id.unwrap_or(100);
                server_events.write(ServerEvent::ObjectPushed {
                    object_id: pid,
                    x: p.location_x,
                    y: p.location_y,
                    direction: p.direction as u8,
                });
                tracing::debug!("🏃 玩家攻击移动 ({},{})", p.location_x, p.location_y);
            }
        }

        // #236：中毒状态（S.Poisoned 本地 / S.ObjectPoisoned 对象）
        x if x == ServerPacketIds::Poisoned as i16 => {
            if let Ok(p) = buff::Poisoned::read_body(&mut cur) {
                let pid = session.local_player_id.unwrap_or(100);
                server_events.write(ServerEvent::ObjectPoisoned {
                    object_id: pid,
                    poisoned: !p.poison.is_empty(),
                });
                tracing::debug!("☠️ 玩家中毒: {:?}", p.poison);
            }
        }
        x if x == ServerPacketIds::ObjectPoisoned as i16 => {
            if let Ok(p) = buff::ObjectPoisoned::read_body(&mut cur) {
                server_events.write(ServerEvent::ObjectPoisoned {
                    object_id: p.object_id,
                    poisoned: !p.poison.is_empty(),
                });
                tracing::debug!("☠️ 对象中毒 id={}: {:?}", p.object_id, p.poison);
            }
        }

        x if x == ServerPacketIds::Chat as i16 => {
            if let Ok(p) = chat::Chat::read_body(&mut cur) {
                server_events.write(crate::network::server_event::from_packet::chat(&p));
            }
        }
        x if x == ServerPacketIds::ObjectChat as i16 => {
            if let Ok(p) = chat::ObjectChat::read_body(&mut cur) {
                server_events.write(crate::network::server_event::from_packet::object_chat(&p));
            }
        }

        _ => {}
    }
    handled
}
