use super::*;
use crate::network::*;
use crate::ui::login::AuthFeedback;
use bevy::prelude::*;
use mir2_shared::packets::base::{Packet, PacketHeader};

// 网络包解码分派（#72 拆分；#1148 再按域拆分）：handle_player 处理服务端包 玩家属性/觉醒/信用 分支。
// 由 packets.rs::handle_packet 调度器按 opcode 调用；返回 true 表示已处理。

/// `S.UserLocation` 到达时的会话状态更新（纯函数：门禁与阳性对照都在这里钉）。
///
/// **关键口径**（2026-09-27 owner「跑一段被拉回来」的根因修复）：服务端**每成功走一步也回一发**
/// `UserLocation`（`PlayerActor::MoveRequest` 成功分支），那一发对客户端是「回显 ACK」，
/// 天生落后本地预测一个 RTT。旧实现把它当权威校正写进 `self_position`，
/// `apply_self_position` 就会把刚跑出去的玩家往回拉 —— 用户看到的正是「跑一段被拉回来」。
///
/// 所以：
/// * `correction == false`（ACK）→ **只**更新 `last_server_position`（夹具 `in_sync` /
///   `state.server_tile_*` 读它），**不写** `self_position` ⇒ 不会挪玩家；
/// * `correction == true`（走位被拒 / 传送 / 复活 / 召回）→ 写 `self_position`，下一帧被
///   `apply_self_position` 无条件采纳（C# `GameScene.UserLocation` 同款语义）。
pub(crate) fn apply_user_location(
    session: &mut SessionState,
    x: i32,
    y: i32,
    direction: u8,
    correction: bool,
) {
    session.last_server_position = Some((x, y));
    if correction {
        session.self_position = Some((x, y, direction));
    }
}

/// `S.UserInformation` 的背包段 → 本端 `Inventory::items`。
///
/// **不要截断到 40**（2026-09-27 修复）：C# `UserObject.Inventory = new UserItem[46]`
/// （`Client/MirObjects/UserObject.cs:37`），`InventoryDialog.Grid = new MirItemCell[8*10]`
/// （`InventoryDialog.cs:148`）——背包窗本来就有**第二页**（C# `ItemSlot = 6 + idx` 把网格映射到
/// 46 格里的 `6..45`）。服务端 `BACKPACK_SIZE = 46`（`ServerRust/src/actors/inventory.rs:61`）
/// 与本端口径一致：本端腰带在装备槽，`Inventory::items` 只存背包那一段，46 格要**照单全收**。
///
/// 此前这里 `.take(40)`：第 41–46 格被整段丢弃 —— 实机数据里那 6 格**确实有物品**
/// （只读查 `inventory_backpack` 的 `grid` 40..45 共 6 行），玩家在客户端**看不见也用不了**，
/// 同时背包还显示"满"。上限取 `MAX_INV_SLOTS = 80`（= C# 的 8×10 网格），扩容后同样不丢。
pub(crate) fn client_inventory_slots(
    inv: &Option<Vec<Option<mir2_shared::data::item::UserItem>>>,
) -> Vec<Option<crate::game::dialogs::inventory::InvItem>> {
    inv.as_ref()
        .map(|v| {
            v.iter()
                .take(crate::game::dialogs::inventory::MAX_INV_SLOTS)
                .map(|slot| slot.as_ref().map(to_inv_item))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mir2_shared::data::item::UserItem;

    /// 门禁：`S.UserInformation` 的背包段必须**整段收下**（46 格口径），不许截断到 40。
    ///
    /// 依据：C# `UserObject.Inventory = new UserItem[46]`（`UserObject.cs:37`）+
    /// `InventoryDialog.Grid = new MirItemCell[8*10]`（`:148`，背包窗有第二页）；
    /// 服务端 `BACKPACK_SIZE = 46`（`inventory.rs:61`）。实机里第 41–46 格确实有物品，
    /// 截断会让玩家"看不见也用不了"，同时背包显示"满"。
    ///
    /// 阳性对照（落地时实做）：把 `client_inventory_slots` 里的 `take(MAX_INV_SLOTS)`
    /// 改回 `take(40)` → 本测试立即红（末位物品消失、长度 40）。
    #[test]
    fn user_information_keeps_all_46_backpack_slots() {
        let mut slots: Vec<Option<UserItem>> = vec![None; 46];
        slots[45] = Some(UserItem {
            unique_id: 777_045,
            item_index: 782,
            ..Default::default()
        });
        let out = client_inventory_slots(&Some(slots));
        assert_eq!(out.len(), 46, "46 格背包必须整段保留（不许 take(40)）");
        assert_eq!(
            out[45].as_ref().map(|i| i.unique_id),
            Some(777_045),
            "第 46 格（index 45）的物品必须还在 —— 截断会让它对玩家不可见"
        );
    }

    /// 门禁：服务端未携带背包段（轻量 UserInformation）时不得凭空造格子。
    /// 该语义由 `apply_slots` 的空 Vec 守卫负责（#2870），这里只钉住转换本身返回空。
    #[test]
    fn user_information_without_inventory_yields_empty() {
        assert!(client_inventory_slots(&None).is_empty());
        assert!(client_inventory_slots(&Some(Vec::new())).is_empty());
    }

    /// 门禁（owner「跑一段被拉回来」）：`S.UserLocation` 的**回显 ACK** 绝不能写 `self_position`，
    /// 否则 `apply_self_position` 会把刚跑出去的玩家拉回服务端那个（落后一个 RTT 的）坐标。
    ///
    /// 阳性对照（落地时实做）：把 `if correction` 去掉、改成无条件写 `self_position`
    /// → 第二条断言立即红（这就是修复前的行为）。
    #[test]
    fn user_location_ack_does_not_move_local_player() {
        let mut s = SessionState::default();
        // ACK：只更新「服务端已知位置」（夹具 in_sync 读它），权威位保持空 ⇒ 不产生位置校正
        apply_user_location(&mut s, 300, 400, 2, false);
        assert_eq!(s.last_server_position, Some((300, 400)));
        assert_eq!(
            s.self_position, None,
            "ACK 不许写权威位（否则玩家每跑一段被拉回一段）"
        );
        // 校正：走位被拒 / 传送 / 复活 —— 必须写权威位，下一帧被无条件采纳
        apply_user_location(&mut s, 301, 401, 3, true);
        assert_eq!(s.last_server_position, Some((301, 401)));
        assert_eq!(s.self_position, Some((301, 401, 3)));
    }
}

#[allow(clippy::too_many_arguments, unused_variables)]
pub(crate) fn handle_player(
    net: &mut NetConnection,
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
    payload: &[u8],
) -> bool {
    use mir2_shared::packets::server::*;

    let mut cur = std::io::Cursor::new(payload);
    let Ok(header) = PacketHeader::read_from(&mut cur) else {
        return false;
    };
    let opcode = header.opcode;
    const HANDLED: &[i16] = &[
        ServerPacketIds::AwakeningNeedMaterials as i16,
        ServerPacketIds::AwakeningLockedItem as i16,
        ServerPacketIds::Awakening as i16,
        ServerPacketIds::ChatItemStats as i16,
        ServerPacketIds::GainedCredit as i16,
        ServerPacketIds::LoseCredit as i16,
        ServerPacketIds::UserInformation as i16,
        ServerPacketIds::HealthChanged as i16,
        ServerPacketIds::UserLocation as i16,
        ServerPacketIds::GainedGold as i16,
        ServerPacketIds::GainExperience as i16,
        ServerPacketIds::LoseGold as i16,
        ServerPacketIds::ChangeAMode as i16,
        ServerPacketIds::ChangePMode as i16,
        ServerPacketIds::ObjectLeveled as i16,
        ServerPacketIds::LevelChanged as i16,
        ServerPacketIds::MountUpdate as i16,
    ];
    let handled = HANDLED.contains(&opcode);
    match opcode {
        x if x == ServerPacketIds::AwakeningNeedMaterials as i16 => {
            if let Ok(p) =
                mir2_shared::packets::server::awakening_system::AwakeningNeedMaterials::read_body(
                    &mut cur,
                )
            {
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
                mir2_shared::packets::server::awakening_system::AwakeningLockedItem::read_body(
                    &mut cur,
                )
            {
                tracing::info!("⚒️ 觉醒锁定: uid={} locked={}", p.unique_id, p.locked);
            }
        }
        x if x == ServerPacketIds::Awakening as i16 => {
            if let Ok(p) =
                mir2_shared::packets::server::awakening_system::Awakening::read_body(&mut cur)
            {
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
                    session.last_server_position = Some((p.location_x, p.location_y));

                    // ---- UI 数据：广播 ServerEvent，由各模块消费 ----
                    let magics: Vec<mir2_shared::data::client_data::ClientMagic> = p.magics.clone();
                    let inventory: Vec<Option<InvItem>> = client_inventory_slots(&p.inventory);
                    let equipment: Vec<Option<InvItem>> = p
                        .equipment
                        .as_ref()
                        .map(|eq| {
                            eq.iter()
                                .map(|slot| slot.as_ref().map(to_inv_item))
                                .collect()
                        })
                        .unwrap_or_default();
                    // #1342：任务物品格（C# QuestInventory 40 格）
                    let quest_inventory: Vec<Option<InvItem>> = p
                        .quest_inventory
                        .as_ref()
                        .map(|qi| {
                            qi.iter()
                                .take(40)
                                .map(|slot| slot.as_ref().map(to_inv_item))
                                .collect()
                        })
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
                        gender: p.gender as u8,
                        object_id: p.object_id,
                        magics,
                        inventory,
                        equipment,
                        quest_inventory,
                        item_names,
                        // #3258：仓库密码三件套（C# `UserInformation.*`）→ 密码流程判据
                        has_storage_password: p.has_storage_password,
                        require_storage_password: p.require_storage_password,
                        storage_password_last_set: p.storage_password_last_set,
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
                    tracing::info!(
                        "📍 UserLocation: ({},{}) dir={:?} correction={}",
                        p.location_x,
                        p.location_y,
                        p.direction,
                        p.correction
                    );
                    apply_user_location(
                        session,
                        p.location_x,
                        p.location_y,
                        p.direction as u8,
                        p.correction,
                    );
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
                // #1388：HUD 宠物模式标签
                server_events.write(ServerEvent::PetModeChanged { mode: p.mode });
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
                tracing::info!(
                    "⬆️ 升级 Lv.{} exp={}/{}",
                    p.level,
                    p.experience,
                    p.max_experience
                );
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
