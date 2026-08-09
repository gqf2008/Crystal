//! auto::inventory 自动化验证系统（从 auto.rs 拆分，#1146）

use bevy::prelude::*;
use super::*;

/// --shop-test：自动 NPC 商店买卖链路（CallNPC → [@Buy] → BuyItem → SellItem）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_shop_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    npc_dialog: Res<client_bevy::game::dialogs::npc::NpcDialogState>,
    mut npc_goods: ResMut<client_bevy::game::dialogs::npc_goods::NpcGoodsState>,
    sell_panel: Res<client_bevy::game::dialogs::sell_panel::SellPanelState>,
    hud: Res<client_bevy::game::hud::HudState>,
    npcs: Query<(
        &client_bevy::actor::NetObjectId,
        &client_bevy::actor::NpcName,
    )>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut npc_oid: Local<Option<u32>>,
    mut bought_idx: Local<Option<i32>>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 6.0 {
                return;
            }
            let oid = npcs
                .iter()
                .find(|(_, n)| n.0.contains("Alchemist"))
                .or_else(|| npcs.iter().find(|(_, n)| n.0.contains("Merchant")))
                .map(|(id, _)| id.0);
            if let Some(oid) = oid {
                *npc_oid = Some(oid);
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id: oid,
                    key: "[@Main]".to_string(),
                });
                tracing::info!("[SHOPTEST] CallNPC {}", oid);
                *stage = 1;
                *t = 0.0;
            }
        }
        1 => {
            if *t < 2.0 {
                return;
            }
            // 直接发送 [@Buy]（服务端匹配该键打开商店；脚本 NPC 菜单行不包含 <购买/@Buy>）
            if let Some(oid) = *npc_oid {
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id: oid,
                    key: "[@Buy]".to_string(),
                });
                tracing::info!("[SHOPTEST] 发送购买菜单指令 [@Buy]");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 2.0 {
                return;
            }
            if npc_goods.visible && !npc_goods.goods.is_empty() {
                let g = &npc_goods.goods[0];
                net.send_packet(&mir2_shared::packets::client::npc::BuyItem {
                    item_index: g.item_index as u64,
                    count: 1,
                    panel_type: mir2_shared::enums::PanelType::Buy,
                });
                tracing::info!("[SHOPTEST] 购买 {} (idx={})", g.name, g.item_index);
                *bought_idx = Some(g.item_index);
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            if *t < 3.0 {
                return;
            }
            // 出售刚购买的物品（按 item_index 匹配，uid 每次服务端启动都会重新分配）
            if let Some(idx) = *bought_idx {
                if let Some(item) = hud
                    .inventory
                    .items
                    .iter()
                    .flatten()
                    .find(|i| i.item_index == idx)
                {
                    net.send_packet(&mir2_shared::packets::client::npc::SellItem {
                        unique_id: item.unique_id,
                        count: 1,
                    });
                    tracing::info!("[SHOPTEST] 出售 {} (uid={})", item.name, item.unique_id);
                }
            }
            *stage = 4;
            *t = 0.0;
        }
        4 => {
            if *t < 3.0 {
                return;
            }
            // 回购：标记回购面板 → 发 [@BuyBack]
            npc_goods.is_buyback = true;
            if let Some(oid) = *npc_oid {
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id: oid,
                    key: "[@BuyBack]".to_string(),
                });
                tracing::info!("[SHOPTEST] 发送回购指令 [@BuyBack]");
            }
            *stage = 5;
            *t = 0.0;
        }
        5 => {
            if *t < 2.0 {
                return;
            }
            if npc_goods.visible && !npc_goods.goods.is_empty() {
                let g = &npc_goods.goods[0];
                net.send_packet(&mir2_shared::packets::client::npc::BuyItemBack {
                    unique_id: g.unique_id,
                    count: 1,
                });
                tracing::info!("[SHOPTEST] 回购 {} (uid={})", g.name, g.unique_id);
                *stage = 6;
                *t = 0.0;
            }
        }
        6 => {
            if *t < 3.0 {
                return;
            }
            if let Some(idx) = *bought_idx {
                if hud.inventory.items.iter().flatten().any(|i| i.item_index == idx) {
                    tracing::info!("[SHOPTEST] ✅ 回购完成：物品已回背包");
                } else {
                    tracing::warn!("[SHOPTEST] ❌ 回购后背包未找到物品");
                }
            }
            *stage = 7;
            *t = 0.0;
        }
        7 => {
            if *t < 2.0 {
                return;
            }
            // 出售面板：[@Sell] → 服务端发 NPCGoods(Sell) → 客户端打开出售面板
            if let Some(oid) = *npc_oid {
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id: oid,
                    key: "[@Sell]".to_string(),
                });
                tracing::info!("[SHOPTEST] 发送出售面板指令 [@Sell]");
            }
            *stage = 8;
            *t = 0.0;
        }
        8 => {
            if *t < 2.0 {
                return;
            }
            if sell_panel.visible {
                tracing::info!("[SHOPTEST] ✅ 出售面板已打开 (mode={:?})", sell_panel.mode);
            } else {
                tracing::warn!("[SHOPTEST] ❌ 出售面板未打开");
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --storage-test：自动仓库存取链路（CallNPC → [@Storage] → StoreItem → TakeBackItem）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_storage_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    storage: Res<client_bevy::game::dialogs::storage::StorageState>,
    hud: Res<client_bevy::game::hud::HudState>,
    npcs: Query<(
        &client_bevy::actor::NetObjectId,
        &client_bevy::actor::NpcName,
        &Transform,
    )>,
    players: Query<
        &Transform,
        (With<client_bevy::actor::LocalPlayer>, With<client_bevy::actor::NetObjectId>),
    >,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut npc_oid: Local<Option<u32>>,
    mut inv_slot: Local<Option<usize>>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 6.0 {
                return;
            }
            // 名字匹配且距离最近的 NPC（真实服务器 NPC 分散，纯名字匹配会选到远处 NPC 被距离校验拒绝）
            let oid = players.single().ok().and_then(|ptf| {
                let (px, py) =
                    client_bevy::game::movement::world_to_tile(ptf.translation.x, ptf.translation.y);
                npcs.iter()
                    .filter(|(_, n, _)| n.0.contains("Alchemist") || n.0.contains("Merchant"))
                    .map(|(id, _, tf)| {
                        let (nx, ny) =
                            client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                        (id.0, (nx - px).abs() + (ny - py).abs())
                    })
                    .min_by_key(|(_, d)| *d)
                    .map(|(id, _)| id)
            });
            if let Some(oid) = oid {
                *npc_oid = Some(oid);
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id: oid,
                    key: "[@Main]".to_string(),
                });
                tracing::info!("[STORAGETEST] CallNPC {}", oid);
                *stage = 1;
                *t = 0.0;
            }
        }
        1 => {
            if *t < 2.0 {
                return;
            }
            if let Some(oid) = *npc_oid {
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id: oid,
                    key: "[@Storage]".to_string(),
                });
                tracing::info!("[STORAGETEST] 发送仓库指令 [@Storage]");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 2.0 {
                return;
            }
            // #200/#283：mock 默认有仓库密码——先解锁再存取
            if storage.unlock_panel {
                net.send_packet(&mir2_shared::packets::client::storage::UnlockStorage {
                    password: "123456".to_string(),
                });
                tracing::info!("[STORAGETEST] 仓库需解锁，发送密码");
                *t = 0.0;
                return;
            }
            if storage.visible {
                if let Some(idx) = hud.inventory.items.iter().position(|s| s.is_some()) {
                    *inv_slot = Some(idx);
                    net.send_packet(&mir2_shared::packets::client::item::StoreItem {
                        from: idx as i32,
                        to: 0,
                    });
                    tracing::info!("[STORAGETEST] 存入背包格 {} -> 仓库 0", idx);
                    *stage = 3;
                    *t = 0.0;
                }
            }
        }
        3 => {
            if *t < 2.0 {
                return;
            }
            if storage.items.get(0).and_then(|s| s.as_ref()).is_some() {
                if let Some(idx) = *inv_slot {
                    net.send_packet(&mir2_shared::packets::client::item::TakeBackItem {
                        from: 0,
                        to: idx as i32,
                    });
                    tracing::info!("[STORAGETEST] 取出仓库 0 -> 背包格 {}", idx);
                }
                *stage = 4;
            }
        }
        _ => {}
    }
}


/// --storage-equip-test：仓库格双击装备链路（#1546：Storage EquipItem → mock 处理 → 客户端装备槽更新）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_storage_equip_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    storage: Res<client_bevy::game::dialogs::storage::StorageState>,
    hud: Res<client_bevy::game::hud::HudState>,
    npcs: Query<(
        &client_bevy::actor::NetObjectId,
        &client_bevy::actor::NpcName,
        &Transform,
    )>,
    players: Query<
        &Transform,
        (With<client_bevy::actor::LocalPlayer>, With<client_bevy::actor::NetObjectId>),
    >,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut npc_oid: Local<Option<u32>>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 6.0 {
                return;
            }
            let oid = players.single().ok().and_then(|ptf| {
                let (px, py) =
                    client_bevy::game::movement::world_to_tile(ptf.translation.x, ptf.translation.y);
                npcs.iter()
                    .filter(|(_, n, _)| n.0.contains("Alchemist") || n.0.contains("Merchant"))
                    .map(|(id, _, tf)| {
                        let (nx, ny) =
                            client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                        (id.0, (nx - px).abs() + (ny - py).abs())
                    })
                    .min_by_key(|(_, d)| *d)
                    .map(|(id, _)| id)
            });
            if let Some(oid) = oid {
                *npc_oid = Some(oid);
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id: oid,
                    key: "[@Main]".to_string(),
                });
                *stage = 1;
                *t = 0.0;
            }
        }
        1 => {
            if *t < 2.0 {
                return;
            }
            if let Some(oid) = *npc_oid {
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id: oid,
                    key: "[@Storage]".to_string(),
                });
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 2.0 {
                return;
            }
            if storage.unlock_panel {
                net.send_packet(&mir2_shared::packets::client::storage::UnlockStorage {
                    password: "123456".to_string(),
                });
                tracing::info!("[STORAGEEQUIP] 仓库解锁");
                *t = 0.0;
                return;
            }
            if storage.visible {
                // 仓库格 4 = mock 木剑（#1546）；直接发 EquipItem Grid=Storage 模拟双击
                if let Some(item) = storage.items.get(4).and_then(|s| s.as_ref()) {
                    net.send_packet(&mir2_shared::packets::client::item::EquipItem {
                        grid: mir2_shared::enums::MirGridType::Storage,
                        unique_id: item.unique_id,
                        to: 0, // Weapon
                    });
                    tracing::info!("[STORAGEEQUIP] 仓库格4装备 {} uid={}", item.name, item.unique_id);
                    *stage = 3;
                    *t = 0.0;
                } else {
                    tracing::warn!("[STORAGEEQUIP] 仓库格4无木剑");
                }
            }
        }
        3 => {
            if *t < 2.0 {
                return;
            }
            let equipped = hud
                .equipment
                .get(0)
                .and_then(|s| s.as_ref())
                .map(|i| i.name.clone())
                .unwrap_or_default();
            tracing::info!("[STORAGEEQUIP] 装备槽0 = {}", equipped);
            if equipped.contains("木剑") {
                tracing::info!("[STORAGEEQUIP] ✅ 仓库双击装备链路验证通过");
            } else {
                tracing::error!("[STORAGEEQUIP] ❌ 装备槽0未更新为木剑");
            }
            *stage = 4;
        }
        _ => {}
    }
}


/// --refine-test：精炼全流程（存入 → 开始 60 秒 → 查看 → 取回）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_refine_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    chat: Res<client_bevy::game::chat::ChatState>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut uid: Local<Option<u64>>,
    mut item_index: Local<Option<i32>>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    // 聊天辅助：最近 60 条里找子串
    fn chat_has(chat: &client_bevy::game::chat::ChatState, needle: &str) -> bool {
        chat.lines.iter().rev().take(60).any(|(t, _, _, _)| t.contains(needle))
    }
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Refine) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Refine);
            }
            let first = hud
                .inventory
                .items
                .iter()
                .enumerate()
                .find_map(|(i, s)| s.as_ref().map(|it| (i, it)));
            match first {
                Some((_i, item)) => {
                    *uid = Some(item.unique_id);
                    *item_index = Some(item.item_index);
                    net.send_packet(&client_bevy::network::RefineDepositWire {
                        unique_id: item.unique_id,
                    });
                    tracing::info!(
                        "[REFINETEST] 存入精炼物品 uid={} #{}",
                        item.unique_id,
                        item.item_index
                    );
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[REFINETEST] ❌ 背包为空");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t >= 6.0 {
                tracing::warn!("[REFINETEST] ❌ 未收到存入确认");
                *stage = 9;
                return;
            }
            if chat_has(&chat, "精炼物品已存入") {
                tracing::info!("[REFINETEST] ✅ 存入成功");
                net.send_packet(&client_bevy::network::RefineItemWire {
                    item_id: item_index.unwrap_or(0) as u32,
                    materials: 1,
                });
                tracing::info!("[REFINETEST] 开始精炼");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t >= 6.0 {
                tracing::warn!("[REFINETEST] ❌ 未收到精炼开始确认");
                *stage = 9;
                return;
            }
            if chat_has(&chat, "精炼已开始") {
                tracing::info!("[REFINETEST] ✅ 精炼已开始（等待 65 秒）");
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            if *t < 65.0 {
                return;
            }
            net.send_packet(&client_bevy::network::RefineCheckWire {
                unique_id: uid.unwrap_or(0),
            });
            tracing::info!("[REFINETEST] 查看精炼结果");
            *stage = 4;
            *t = 0.0;
        }
        4 => {
            if *t >= 8.0 {
                tracing::warn!("[REFINETEST] ❌ 未收到精炼结果");
                *stage = 9;
                return;
            }
            if chat_has(&chat, "精炼成功") || chat_has(&chat, "精炼失败") || chat_has(&chat, "已完成") {
                tracing::info!("[REFINETEST] ✅ 精炼结果已返回");
                net.send_packet(&client_bevy::network::RefineRetrieveWire {
                    unique_id: uid.unwrap_or(0),
                });
                tracing::info!("[REFINETEST] 取回精炼物品");
                *stage = 5;
                *t = 0.0;
            }
        }
        5 => {
            if *t < 5.0 {
                return;
            }
            if chat_has(&chat, "精炼物品已取回") {
                tracing::info!("[REFINETEST] ✅ 取回成功，精炼全流程完成");
            } else {
                tracing::warn!("[REFINETEST] ⚠️ 取回未确认（可能已自动完成）");
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --craft-test：打开合成 → 配方1 → 合成 → 等 CraftItem 响应/聊天
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_craft_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    craft: Res<client_bevy::game::dialogs::craft::CraftState>,
    chat: Res<client_bevy::game::chat::ChatState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    fn chat_has(chat: &client_bevy::game::chat::ChatState, needle: &str) -> bool {
        chat.lines.iter().rev().take(60).any(|(t, _, _, _)| t.contains(needle))
    }
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Craft) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Craft);
            }
            net.send_packet(&client_bevy::network::CraftItemWire {
                recipe_id: 1,
                materials: 0,
            });
            tracing::info!("[CRAFTTEST] 合成配方 1（木材x3+铁矿石x2）");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 8.0 {
                tracing::warn!(
                    "[CRAFTTEST] ❌ 未收到合成结果: message={}",
                    craft.message
                );
                *stage = 9;
                return;
            }
            let ok = craft.last_result.is_some()
                || chat_has(&chat, "合成成功")
                || chat_has(&chat, "合成失败")
                || chat_has(&chat, "材料不足")
                || chat_has(&chat, "未知配方");
            if ok {
                tracing::info!(
                    "[CRAFTTEST] ✅ 合成结果: {}",
                    craft.message
                );
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --rental-test（租方）：发起租赁 → 等 UpdateRentalItem → 锁定费用 → 确认
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_rental_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    rental: Res<client_bevy::game::dialogs::item_rental::ItemRentalState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            net.send_packet(&client_bevy::network::RentalRequestWire {
                target_name: "bevy2char".to_string(),
            });
            tracing::info!("[RENTAL] 向 bevy2char 发起租赁");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 25.0 {
                tracing::warn!("[RENTAL] ❌ 未收到租赁更新（has_item={}）", rental.has_item);
                *stage = 9;
                return;
            }
            if rental.has_item {
                tracing::info!(
                    "[RENTAL] ✅ 收到租赁物品（费用={} 期限={}）",
                    rental.fee,
                    rental.period
                );
                net.send_packet(&mir2_shared::packets::client::item::ItemRentalLockFee);
                tracing::info!("[RENTAL] 锁定费用");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t >= 15.0 {
                tracing::warn!("[RENTAL] ❌ 未收到可确认");
                *stage = 9;
                return;
            }
            if rental.can_confirm {
                tracing::info!("[RENTAL] ✅ 双方已锁定，确认成交");
                net.send_packet(&mir2_shared::packets::client::item::ConfirmItemRental);
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            if *t < 5.0 {
                return;
            }
            if rental.confirmed {
                tracing::info!("[RENTAL] ✅ 租赁成交确认收到");
            } else {
                tracing::warn!("[RENTAL] ⚠️ 未收到成交确认包");
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --rental-owner（物主）：等请求 → 存入物品 → 设费/期 → 锁定物品 → 等可确认
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_rental_owner(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    rental: Res<client_bevy::game::dialogs::item_rental::ItemRentalState>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t >= 30.0 {
                tracing::warn!("[RENTALOWNER] ❌ 未收到租赁请求");
                *stage = 9;
                return;
            }
            if rental.request_received {
                tracing::info!("[RENTALOWNER] ✅ 收到租赁请求");
                // 存入第一个背包物品
                let first = hud
                    .inventory
                    .items
                    .iter()
                    .enumerate()
                    .find_map(|(i, s)| s.as_ref().map(|it| (i, it)));
                match first {
                    Some((_i, item)) => {
                        net.send_packet(&client_bevy::network::RentalDepositWire {
                            unique_id: item.unique_id,
                        });
                        tracing::info!(
                            "[RENTALOWNER] 存入物品 uid={}",
                            item.unique_id
                        );
                        *stage = 1;
                        *t = 0.0;
                    }
                    None => {
                        tracing::warn!("[RENTALOWNER] ❌ 背包为空");
                        *stage = 9;
                    }
                }
            }
        }
        1 => {
            if *t < 4.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::item::ItemRentalFee { amount: 100 });
            net.send_packet(&mir2_shared::packets::client::item::ItemRentalPeriod { days: 24 });
            tracing::info!("[RENTALOWNER] 设置费用 100 / 期限 24");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t < 4.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::item::ItemRentalLockItem);
            tracing::info!("[RENTALOWNER] 锁定物品");
            *stage = 3;
            *t = 0.0;
        }
        3 => {
            if *t >= 15.0 {
                tracing::warn!("[RENTALOWNER] ❌ 未收到可确认");
                *stage = 9;
                return;
            }
            if rental.can_confirm {
                tracing::info!("[RENTALOWNER] ✅ 双方已锁定，可确认");
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --socket-test：打开镶嵌面板 → 孔位/宝石渲染 → 关闭
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_socket_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut socket: ResMut<client_bevy::game::dialogs::socket::SocketState>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut phase: Local<f32>,
) {
    use client_bevy::scenes::AppState;
    use client_bevy::game::dialogs::DialogKind;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    if *stage == 0 {
        // 等 UserInformation 背包数据应用后再检查（首次进入 Game 帧时可能尚未应用）
        if *phase == 0.0 {
            *phase = *t;
            return;
        }
        if *t - *phase < 2.0 {
            return;
        }
        let sock = hud
            .inventory
            .items
            .iter()
            .flatten()
            .find(|it| !it.slots.is_empty())
            .cloned();
        if let Some(item) = sock {
            socket.item = Some(item.clone());
            if !mgr.is_open(DialogKind::Socket) {
                mgr.open(DialogKind::Socket);
            }
            tracing::info!(
                "[SOCKET] 打开镶嵌面板: {} ({} 孔)",
                item.name,
                item.slots.len()
            );
            *stage = 1;
        } else {
            tracing::warn!("[SOCKET] ❌ 背包中没有带孔物品");
            *stage = 9;
        }
        *phase = *t;
        return;
    }
    if *stage == 1 && *t - *phase >= 1.5 {
        let gems: Vec<String> = socket
            .item
            .as_ref()
            .map(|i| {
                i.slots
                    .iter()
                    .map(|s| {
                        s.as_ref()
                            .map(|g| format!("{}", g.name))
                            .unwrap_or_else(|| "空".to_string())
                    })
                    .collect()
            })
            .unwrap_or_default();
        tracing::info!("[SOCKET] ✅ 孔位渲染: {}", gems.join(", "));
        *stage = 2;
        *phase = *t;
        return;
    }
    if *stage == 2 && *t - *phase >= 1.0 {
        if mgr.is_open(DialogKind::Socket) {
            mgr.close(DialogKind::Socket);
            tracing::info!("[SOCKET] ✅ 关闭镶嵌面板");
        }
        *stage = 9;
    }
    if *t >= 25.0 && *stage < 9 {
        tracing::warn!("[SOCKET] ❌ 超时 stage={}", *stage);
        *stage = 9;
    }
}

/// --dura-test：打开耐久面板 → 装备耐久三态渲染 → 关闭
pub(crate) fn auto_dura_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut phase: Local<f32>,
) {
    use client_bevy::scenes::AppState;
    use client_bevy::game::dialogs::DialogKind;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    if *stage == 0 {
        if !mgr.is_open(DialogKind::DuraStatus) {
            mgr.open(DialogKind::DuraStatus);
            tracing::info!("[DURA] 打开耐久面板");
        }
        *phase = *t;
        *stage = 1;
        return;
    }
    if *stage == 1 && *t - *phase >= 1.0 {
        let equipped: Vec<String> = hud
            .equipment
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.as_ref().map(|it| format!("slot{}={}({}/{})", i, it.name, it.current_dura, it.max_dura)))
            .collect();
        tracing::info!("[DURA] ✅ 装备耐久数据: {}", if equipped.is_empty() { "无".to_string() } else { equipped.join(", ") });
        *stage = 2;
        *phase = *t;
        return;
    }
    if *stage == 2 && *t - *phase >= 1.5 {
        if mgr.is_open(DialogKind::DuraStatus) {
            mgr.close(DialogKind::DuraStatus);
            tracing::info!("[DURA] ✅ 关闭耐久面板");
        }
        *stage = 9;
    }
    if *t >= 25.0 && *stage < 9 {
        tracing::warn!("[DURA] ❌ 超时 stage={}", *stage);
        *stage = 9;
    }
}

/// --awake-test：打开觉醒 → 选武器 → 选类型/材料 → 执行觉醒（可重试）→ 关闭
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_awake_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut aw: ResMut<client_bevy::game::dialogs::npc_awake::NpcAwakeState>,
    hud: Res<client_bevy::game::hud::HudState>,
    net: ResMut<client_bevy::network::NetConnection>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut phase: Local<f32>,
    mut attempts: Local<u32>,
) {
    use client_bevy::scenes::AppState;
    use client_bevy::game::dialogs::DialogKind;
    use mir2_shared::packets::client::misc::{Awakening, AwakeningNeedMaterials};
    use mir2_shared::enums::AwakeType;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    if *stage == 0 {
        if !mgr.is_open(DialogKind::NpcAwake) {
            mgr.toggle(DialogKind::NpcAwake);
            tracing::info!("[AWAKE] 打开觉醒对话框");
        }
        *phase = *t;
        *stage = 1;
        return;
    }
    if *stage == 1 && *t - *phase >= 1.0 {
        for (i, it) in hud.inventory.items.iter().enumerate() {
            if let Some(item) = it {
                tracing::info!(
                    "[AWAKE] inv[{}] uid={} idx={} name={}",
                    i,
                    item.unique_id,
                    item.item_index,
                    item.name
                );
            }
        }
        let sword = hud
            .inventory
            .items
            .iter()
            .flatten()
            .find(|it| it.item_index == 221)
            .cloned();
        if let Some(item) = sword {
            aw.selected_uid = Some(item.unique_id);
            aw.selected_item = Some(item.clone());
            aw.awake_type = None;
            tracing::info!("[AWAKE] ✅ 选择武器: {} (uid={})", item.name, item.unique_id);
            *stage = 2;
        } else {
            tracing::warn!("[AWAKE] ❌ 背包中没有 WoodenSword");
            *stage = 9;
        }
        *phase = *t;
        return;
    }
    if *stage == 2 && *t - *phase >= 1.0 {
        if let Some(uid) = aw.selected_uid {
            aw.awake_type = Some(AwakeType::Dc);
            net.send_packet(&AwakeningNeedMaterials {
                unique_id: uid,
                awake_type: AwakeType::Dc,
            });
            tracing::info!("[AWAKE] ✅ 请求觉醒材料 uid={} type=Dc", uid);
        }
        *stage = 3;
        *phase = *t;
        return;
    }
    if *stage == 3 && *t - *phase >= 1.5 {
        tracing::info!(
            "[AWAKE] ✅ 材料需求: {}",
            if aw.materials.is_empty() {
                "无（跳过材料检查）".to_string()
            } else {
                aw.materials
                    .iter()
                    .map(|m| format!("#{}x{}", m.item_id, m.count))
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        );
        if let Some(uid) = aw.selected_uid {
            net.send_packet(&Awakening {
                unique_id: uid,
                awake_type: AwakeType::Dc,
                position_idx: 0,
            });
            tracing::info!("[AWAKE] ✅ 执行觉醒 uid={}（第 {} 次）", uid, *attempts + 1);
            *attempts += 1;
        }
        aw.result = 0;
        *stage = 4;
        *phase = *t;
        return;
    }
    if *stage == 4 && *t - *phase >= 2.5 {
        if aw.result == 1 {
            tracing::info!("[AWAKE] ✅ 觉醒成功（结果 {}）", aw.result);
            *stage = 5;
            *phase = *t;
        } else if *attempts < 6 {
            // 失败/销毁：换下一把武器重试
            tracing::warn!(
                "[AWAKE] ⚠️ 觉醒结果 {}（{}），换武器重试",
                aw.result,
                aw.result_text
            );
            let swords: Vec<_> = hud
                .inventory
                .items
                .iter()
                .flatten()
                .filter(|it| it.item_index == 221)
                .collect();
            let next = swords
                .get((*attempts) as usize % swords.len().max(1))
                .cloned();
            if let Some(item) = next {
                aw.selected_uid = Some(item.unique_id);
                aw.selected_item = Some(item.clone());
            }
            aw.materials.clear();
            aw.result_text = String::new();
            *stage = 2;
            *phase = *t;
        } else {
            tracing::warn!("[AWAKE] ❌ 多次觉醒未成功");
            *stage = 9;
        }
        return;
    }
    if *stage == 5 && *t - *phase >= 1.0 {
        if mgr.is_open(DialogKind::NpcAwake) {
            mgr.close(DialogKind::NpcAwake);
            tracing::info!("[AWAKE] ✅ 关闭觉醒对话框");
        }
        *stage = 9;
    }
    if *t >= 45.0 && *stage < 9 {
        tracing::warn!("[AWAKE] ❌ 超时 stage={}", *stage);
        *stage = 9;
    }
}

/// --item-state-test：施法 → mock 回发 DuraChanged/GainedItem/DeleteItem，
/// 断言背包耐久更新/物品获得/物品删除（#228）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_item_state_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[ITEMST] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[ITEMST] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[ITEMST] 🔥 施法触发物品状态同步");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t >= 3.0 {
                let gained = hud
                    .inventory
                    .items
                    .iter()
                    .flatten()
                    .any(|it| it.unique_id == 9002);
                let deleted = !hud
                    .inventory
                    .items
                    .iter()
                    .flatten()
                    .any(|it| it.unique_id == 9010);
                let dura = hud
                    .inventory
                    .items
                    .iter()
                    .flatten()
                    .any(|it| it.unique_id == 9005 && it.current_dura == 3);
                tracing::info!(
                    "[ITEMST] 获得={} 删除={} 耐久={}",
                    gained,
                    deleted,
                    dura
                );
                if gained && deleted && dura {
                    tracing::info!("[ITEMST] ✅ 物品状态同步全部通过");
                } else {
                    tracing::warn!("[ITEMST] ❌ 部分未通过（获得={} 删除={} 耐久={}）", gained, deleted, dura);
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --repair-test：施法 → mock 回发 ItemRepaired(9007: 12/8) + ItemSlotSizeChanged(1)，
/// 断言背包物品耐久/最大耐久/槽位数更新（#240）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_repair_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[REPAIR] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[REPAIR] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[REPAIR] 🔥 施法触发修理/槽位同步");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t >= 2.5 {
                let item = hud
                    .inventory
                    .items
                    .iter()
                    .flatten()
                    .find(|it| it.unique_id == 9007);
                let dura = item
                    .map(|it| it.current_dura == 8 && it.max_dura == 12)
                    .unwrap_or(false);
                let slots = item.map(|it| it.slots.len() == 1).unwrap_or(false);
                tracing::info!("[REPAIR] 耐久={} 槽位={}", dura, slots);
                if dura && slots {
                    tracing::info!("[REPAIR] ✅ 修理/槽位同步通过");
                } else {
                    tracing::warn!("[REPAIR] ❌ 未通过（耐久={} 槽位={}）", dura, slots);
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --resize-test：背包扩容链路（#276）
/// 流程：进游戏 → 施法（mock 回发 ResizeInventory(56)）→ 校验 items.len()==56
pub(crate) fn auto_resize_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 6.0 {
                return;
            }
            if hud.inventory.items.len() < 40 {
                return; // 等 UserInformation 完成
            }
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: mir2_shared::enums::MirDirection::Down,
                target_id: 101,
                location: mir2_shared::Point { x: 353, y: 352 },
            });
            tracing::info!("[RESIZE] 🔥 施法触发 ResizeInventory");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 2.0 {
                return;
            }
            if hud.inventory.items.len() == 56 {
                tracing::info!("[RESIZE] ✅ PASS 背包扩容 size=56");
            } else {
                tracing::error!(
                    "[RESIZE] ❌ FAIL size={} 期望 56",
                    hud.inventory.items.len()
                );
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --storage-unlock-test：仓库密码解锁链路（#200）
/// 流程：进游戏 → NPC [@Storage] → 断言解锁框出现（仓库未打开）→ 错误密码 → 提示
///       → 正确密码 → 仓库打开（StorageOpened）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_storage_unlock_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    storage: Res<client_bevy::game::dialogs::storage::StorageState>,
    npcs: Query<(
        &client_bevy::actor::NetObjectId,
        &client_bevy::actor::NpcName,
        &Transform,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut npc_oid: Local<Option<u32>>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 6.0 {
                return;
            }
            let oid = players.single().ok().and_then(|ptf| {
                let (px, py) =
                    client_bevy::game::movement::world_to_tile(ptf.translation.x, ptf.translation.y);
                npcs.iter()
                    .map(|(id, n, tf)| {
                        let (nx, ny) =
                            client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                        (id.0, n.0.clone(), (nx - px).abs() + (ny - py).abs())
                    })
                    .min_by_key(|(_, _, d)| *d)
                    .map(|(id, _, _)| id)
            });
            if let Some(oid) = oid {
                *npc_oid = Some(oid);
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id: oid,
                    key: "[@Main]".to_string(),
                });
                tracing::info!("[UNLOCK] CallNPC {}", oid);
                *stage = 1;
                *t = 0.0;
            }
        }
        1 => {
            if *t < 1.0 {
                return;
            }
            if let Some(oid) = *npc_oid {
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id: oid,
                    key: "[@Storage]".to_string(),
                });
                tracing::info!("[UNLOCK] CallNPC [@Storage]");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 1.5 {
                return;
            }
            if storage.unlock_panel && !storage.visible {
                tracing::info!("[UNLOCK] ✅ 解锁框出现（仓库未打开）");
                net.send_packet(&mir2_shared::packets::client::storage::UnlockStorage {
                    password: "wrong".to_string(),
                });
                *stage = 3;
                *t = 0.0;
            } else {
                tracing::warn!(
                    "[UNLOCK] ❌ 解锁框未出现（panel={} visible={}）",
                    storage.unlock_panel,
                    storage.visible
                );
                *stage = 9;
            }
        }
        3 => {
            if *t < 1.0 {
                return;
            }
            if !storage.unlock_msg.is_empty() && storage.unlock_panel {
                tracing::info!("[UNLOCK] ✅ 错误密码提示: {}", storage.unlock_msg);
                net.send_packet(&mir2_shared::packets::client::storage::UnlockStorage {
                    password: "123456".to_string(),
                });
                *stage = 4;
                *t = 0.0;
            } else {
                tracing::warn!(
                    "[UNLOCK] ❌ 错误密码未提示（msg={} panel={}）",
                    storage.unlock_msg,
                    storage.unlock_panel
                );
                *stage = 9;
            }
        }
        4 => {
            if *t < 1.5 {
                return;
            }
            if storage.visible && !storage.unlock_panel {
                tracing::info!("[UNLOCK] ✅ PASS 仓库解锁并打开");
            } else {
                tracing::error!(
                    "[UNLOCK] ❌ FAIL visible={} panel={}",
                    storage.visible,
                    storage.unlock_panel
                );
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --storage-resize-test：仓库扩容链路（#281）
/// 流程：进游戏 → mock 回发 ResizeStorage(80) → 断言 StorageState.items.len()==80
pub(crate) fn auto_storage_resize_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    storage: Res<client_bevy::game::dialogs::storage::StorageState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 6.0 {
                return;
            }
            // mock 的 ResizeStorage(80) 在施法演示批次里回发
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: mir2_shared::enums::MirDirection::Down,
                target_id: 101,
                location: mir2_shared::Point { x: 353, y: 352 },
            });
            tracing::info!("[SRESIZE] 🔥 施法触发演示批次");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 2.0 {
                return;
            }
            if storage.items.len() == 80 {
                tracing::info!("[SRESIZE] ✅ PASS 仓库扩容 size=80");
            } else {
                tracing::error!("[SRESIZE] ❌ FAIL size={} 期望 80", storage.items.len());
            }
            *stage = 9;
        }
        _ => {}
    }
}


