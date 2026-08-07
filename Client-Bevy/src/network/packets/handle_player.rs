use bevy::prelude::*;
use mir2_shared::packets::base::{Packet, PacketHeader};
use crate::network::*;
use crate::ui::login::AuthFeedback;
use super::*;

// 网络包解码分派（#72 拆分；#1148 再按域拆分）：handle_player 处理服务端包 玩家属性/觉醒/信用 分支。
// 由 packets.rs::handle_packet 调度器按 opcode 调用；返回 true 表示已处理。

#[allow(clippy::too_many_arguments, unused_variables)]
pub(crate) fn handle_player(    net: &mut NetConnection,
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
    const HANDLED: &[i16] = &[ServerPacketIds::AwakeningNeedMaterials as i16, ServerPacketIds::AwakeningLockedItem as i16, ServerPacketIds::Awakening as i16, ServerPacketIds::ChatItemStats as i16, ServerPacketIds::GainedCredit as i16, ServerPacketIds::LoseCredit as i16, ServerPacketIds::UserInformation as i16, ServerPacketIds::HealthChanged as i16, ServerPacketIds::UserLocation as i16, ServerPacketIds::GainedGold as i16, ServerPacketIds::GainExperience as i16, ServerPacketIds::LoseGold as i16, ServerPacketIds::ChangeAMode as i16, ServerPacketIds::ChangePMode as i16, ServerPacketIds::ObjectLeveled as i16, ServerPacketIds::LevelChanged as i16, ServerPacketIds::MountUpdate as i16];
    let handled = HANDLED.contains(&opcode);
    match opcode {
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
        x if x == ServerPacketIds::ChatItemStats as i16 => {
            if let Ok(p) = miscellaneous::ChatItemStats::read_body(&mut cur) {
                tracing::debug!("📊 聊天物品属性 uid={}", p.unique_id);
            }
        }
        x if x == ServerPacketIds::GainedCredit as i16 => {
            if let Ok(p) = drops::GainedCredit::read_body(&mut cur) {
                server_events.write(ServerEvent::CreditGained { credit: p.credit });
                tracing::info!("🏅 获得声望 +{}", p.credit);
            }
        }
        x if x == ServerPacketIds::LoseCredit as i16 => {
            if let Ok(p) = drops::LoseCredit::read_body(&mut cur) {
                server_events.write(ServerEvent::CreditLost { amount: p.credit });
                tracing::info!("🏅 失去声望 -{}", p.credit);
            }
        }
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
        x if x == ServerPacketIds::ObjectLeveled as i16 => {
            if let Ok(p) = experience::ObjectLeveled::read_body(&mut cur) {
                server_events.write(ServerEvent::ObjectLeveled {
                    object_id: p.object_id,
                    level: p.level,
                });
                tracing::info!("⬆️ 对象升级 id={} Lv.{}", p.object_id, p.level);
            }
        }
        x if x == ServerPacketIds::LevelChanged as i16 => {
            if let Ok(p) = experience::LevelChanged::read_body(&mut cur) {
                server_events.write(server_event::from_packet::level_changed(&p));
                tracing::info!("⬆️ 升级 Lv.{} exp={}/{}", p.level, p.experience, p.max_experience);
            }
        }
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
        _ => {}
    }
    handled
}
