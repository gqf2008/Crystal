// ============================================================================
// mir2 bevy - 主入口
// ============================================================================
// 传奇2 (Legend of Mir 2) 客户端 Bevy 移植版
//
// 用法:
//   cargo run --bin client_bevy                     # 默认地图 n0 + 演示角色
//   cargo run --bin client_bevy -- --map n0
//   cargo run --bin client_bevy -- --map 11yearvilliage
//   cargo run --bin client_bevy -- --no-actors      # 只渲染地图（截图验证用）

use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::render::RenderPlugin;
use client_bevy::actor::ActorPlugin;
use client_bevy::event_bus::EventBusPlugin;
use client_bevy::map_renderer::MapRenderPlugin;
use client_bevy::network::NetworkPlugin;
use client_bevy::scenes::AppState;
use client_bevy::ui::intro::IntroPlugin;
use client_bevy::ui::login::LoginPlugin;
use client_bevy::ui::modal_box::ModalBoxPlugin;
use client_bevy::ui::new_character::NewCharacterPlugin;
use client_bevy::ui::select::SelectPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            // 使用 DX12 后端（Vulkan 的 swapchain present 在此机器上会冻结）
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                    backends: Some(Backends::DX12),
                    ..default()
                })),
                ..default()
            })
            .set(LogPlugin {
                filter:
                    "info,bevy_render=warn,bevy_asset=warn,bevy_log=warn,bevy_diagnostic=warn,wgpu_hal=warn,naga=warn"
                        .into(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Mir2 (Bevy) — 传奇2 客户端移植".to_string(),
                    resolution: (1024u32, 768u32).into(),
                    // 启用 IME：支持中文输入法（角色名/账号密码等）
                    ime_enabled: true,
                    // 无 vsync：避免会话中 vblank 缺失导致 present 永久阻塞（画面冻结）
                    present_mode: bevy::window::PresentMode::Immediate,
                    ..default()
                }),
                ..default()
            }),
    );
    app.insert_resource(ClearColor(Color::srgb(0.07, 0.08, 0.12)));
    app.init_state::<AppState>();
    // --skip-login: 直接从登录界面进入游戏（诊断呈现问题用）
    if std::env::args().any(|a| a == "--skip-login") {
        app.add_systems(Update, |mut next: ResMut<NextState<AppState>>| {
            next.set(AppState::Game)
        });
    }
    app.add_plugins(EventBusPlugin);
    app.add_plugins((
        NetworkPlugin,
        IntroPlugin,
        LoginPlugin,
        SelectPlugin,
        NewCharacterPlugin,
        ModalBoxPlugin,
        client_bevy::game::GamePlugin,
    ));
    // --auto-attack: 进游戏后每 1.5s 自动攻击（M10 战斗链路调试）
    if std::env::args().any(|a| a == "--auto-attack") {
        app.add_systems(Update, auto_attack_debug);
    }
    // --auto-inv / --auto-char: 进游戏 3 秒后自动打开背包/角色对话框（M9 调试）
    if std::env::args().any(|a| a == "--auto-inv") {
        app.add_systems(Update, auto_open_inventory);
    }
    if std::env::args().any(|a| a == "--auto-char") {
        app.add_systems(Update, auto_open_character);
    }
    // --shop-test: 自动 NPC 商店买卖链路（自动化验证用）
    if std::env::args().any(|a| a == "--shop-test") {
        app.add_systems(Update, auto_shop_test);
    }

    // --storage-test: 自动仓库存取链路（自动化验证用）
    if std::env::args().any(|a| a == "--storage-test") {
        app.add_systems(Update, auto_storage_test);
    }    // --shop-test: 自动 NPC 商店买卖链路（自动化验证用）
    if std::env::args().any(|a| a == "--shop-test") {
        app.add_systems(Update, auto_shop_test);
    }
    // --group-test: 自动组队邀请链路（自动化验证用，配合 --group-accept）
    if std::env::args().any(|a| a == "--group-test") {
        app.add_systems(Update, auto_group_test);
    }
    // --group-accept: 自动接受组队邀请（自动化验证用）
    if std::env::args().any(|a| a == "--group-accept") {
        app.add_systems(Update, auto_group_accept);
    }
    // --shop-test: 自动 NPC 商店买卖链路（自动化验证用）
    if std::env::args().any(|a| a == "--shop-test") {
        app.add_systems(Update, auto_shop_test);
    }
    // --mail-test: 自动发邮件链路（自动化验证用，配合 --mail-read）
    if std::env::args().any(|a| a == "--mail-test") {
        app.add_systems(Update, auto_mail_test);
    }
    // --mail-read: 自动读取新邮件（自动化验证用）
    if std::env::args().any(|a| a == "--mail-read") {
        app.add_systems(Update, auto_mail_read);
    }
    // --shop-test: 自动 NPC 商店买卖链路（自动化验证用）
    if std::env::args().any(|a| a == "--shop-test") {
        app.add_systems(Update, auto_shop_test);
    }
    // --auto-enter: 自动从登录界面进入游戏（自动化验证用）
    if std::env::args().any(|a| a == "--auto-enter") {
        // auto_enter 需要覆盖 Login 和 Select 两个状态（内部自行判断）
        app.add_systems(Update, auto_enter);
    }
    // BEVY_DEMO_DELETE=1: 自动登录→进选角→打开删除询问框（截图验证用）
    if std::env::var("BEVY_DEMO_DELETE").as_deref() == Ok("1") {
        app.add_systems(Update, demo_delete_flow);
    }
    // F12: 保存当前帧截图到 ../../tools/bevy_shot_N.png（开发调试用）
    app.add_systems(Update, debug_screenshot);
    // 窗口获得焦点时强制激活 winit IME（见 ime_focus_activation）
    app.init_resource::<ImePulse>();
    app.add_systems(Update, ime_focus_activation);
    // --no-actors: 只渲染地图（用于纯地图截图验证）
    if std::env::args().any(|a| a == "--no-actors") {
        app.add_plugins(MapRenderPlugin);
    } else {
        app.add_plugins((MapRenderPlugin, ActorPlugin));
    }
    app.run();
}

/// F12 截图（保存到工作区 tools/ 目录）
/// F12 截图；设置 BEVY_AUTO_SHOT=1 时按 BEVY_SHOT_INTERVAL（默认 2 秒）自动截一张
/// （保存到工作区 tools/ 目录，开发调试用）
/// --shop-test：自动 NPC 商店买卖链路（CallNPC → [@Buy] → BuyItem → SellItem）
#[allow(clippy::too_many_arguments)]
fn auto_shop_test(
    net: ResMut<client_bevy::network::NetworkContext>,
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
fn auto_storage_test(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    storage: Res<client_bevy::game::dialogs::storage::StorageState>,
    hud: Res<client_bevy::game::hud::HudState>,
    npcs: Query<(
        &client_bevy::actor::NetObjectId,
        &client_bevy::actor::NpcName,
    )>,
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

fn debug_screenshot(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut counter: Local<u32>,
    time: Res<Time>,
    mut acc: Local<f32>,
) {
    if std::env::var("BEVY_AUTO_SHOT").is_ok() {
        let interval: f32 = std::env::var("BEVY_SHOT_INTERVAL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2.0);
        *acc += time.delta_secs();
        if *acc >= interval {
            *acc = 0.0;
            capture_shot(&mut commands, &mut counter);
        }
    }
    if keys.just_pressed(KeyCode::F12) {
        capture_shot(&mut commands, &mut counter);
    }
}

fn capture_shot(commands: &mut Commands, counter: &mut u32) {
    let path = format!("../tools/bevy_shot_{}.png", *counter);
    *counter += 1;
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
}

/// --auto-attack：自动攻击（验证 攻击→受击→飘字 链路）
fn auto_attack_debug(
    net: Res<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut timer: Local<f32>,
) {
    if *state != client_bevy::scenes::AppState::Game {
        return;
    }
    *timer += time.delta_secs();
    if *timer >= 1.5 {
        *timer = 0.0;
        net.send_packet(&mir2_shared::packets::client::combat::Attack {
            direction: mir2_shared::enums::MirDirection::Up,
            spell: mir2_shared::enums::Spell::None,
        });
        tracing::info!("⚔️ --auto-attack 自动攻击");
    }
}

/// --auto-char：进游戏 3 秒后自动打开角色对话框
fn auto_open_character(
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut timer: Local<f32>,
) {
    if *state != client_bevy::scenes::AppState::Game {
        return;
    }
    *timer += time.delta_secs();
    if *timer >= 3.0 && !mgr.is_open(client_bevy::game::dialogs::DialogKind::Character) {
        mgr.toggle(client_bevy::game::dialogs::DialogKind::Character);
        tracing::info!("🎛️ --auto-char 自动打开角色对话框");
    }
}

/// --auto-inv：进游戏 3 秒后自动打开背包
fn auto_open_inventory(
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut timer: Local<f32>,
) {
    if *state != client_bevy::scenes::AppState::Game {
        return;
    }
    *timer += time.delta_secs();
    if *timer >= 3.0 && !mgr.is_open(client_bevy::game::dialogs::DialogKind::Inventory) {
        mgr.toggle(client_bevy::game::dialogs::DialogKind::Inventory);
        tracing::info!("🎛️ --auto-inv 自动打开背包");
    }
}

/// 强制激活 winit IME。
/// 根因：winit 创建窗口时强制 set_ime_allowed(false) 断开 IMM 上下文；
/// bevy_winit 创建时不同步 ime_enabled（仅后续 Changed<Window> 脏检测才 set_ime_allowed），
/// 缓存初值=true 导致 winit 的 IME 永远停在 false。
/// 这里在窗口首次报告 focused 后做一次 false→true 两帧脉冲，借脏检测触发
/// winit set_ime_allowed(true) 重连 IMM。不依赖 WindowFocused 事件（启动即聚焦时不会发）。
#[derive(Resource, Default)]
struct ImePulse(u8); // 0=待触发 1=已置false待回true 2=已完成

fn ime_focus_activation(mut windows: Query<&mut Window>, mut pulse: ResMut<ImePulse>) {
    match pulse.0 {
        0 => {
            // 等窗口报告已聚焦（启动即聚焦或用户点击后）
            let focused = windows.iter().any(|w| w.focused);
            if focused {
                for mut w in windows.iter_mut() {
                    if w.ime_enabled {
                        w.ime_enabled = false;
                    }
                }
                pulse.0 = 1;
            }
        }
        1 => {
            for mut w in windows.iter_mut() {
                w.ime_enabled = true;
            }
            pulse.0 = 2;
            tracing::debug!("[IME] 已激活 winit IME（set_ime_allowed(true)）");
        }
        _ => {}
    }
}
/// --group-test：自动组队邀请链路（登录后向 bevy2char 发 AddMember，等成员列表）
#[allow(clippy::too_many_arguments)]
fn auto_group_test(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    group: Res<client_bevy::game::dialogs::group::GroupState>,
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
            if *t < 8.0 {
                return;
            }
            let invitee = std::env::args()
                .skip_while(|a| a != "--e2e-invitee")
                .nth(1)
                .unwrap_or_else(|| "bevy2char".to_string());
            net.send_packet(&mir2_shared::packets::client::group::AddMember {
                name: invitee.clone(),
            });
            tracing::info!("[GROUPTEST] 邀请组队: {}", invitee);
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 5.0 {
                return;
            }
            if group.members.len() >= 2 {
                tracing::info!(
                    "[GROUPTEST] ✅ 组队成功: {}",
                    group.members.iter().map(|m| m.name.as_str()).collect::<Vec<_>>().join(", ")
                );
            } else {
                tracing::warn!("[GROUPTEST] ❌ 组队成员不足: {:?}", group.members);
            }
            *stage = 2;
        }
        _ => {}
    }
}

/// --group-accept：自动接受组队邀请（自动化验证用）
#[allow(clippy::too_many_arguments)]
fn auto_group_accept(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    mut group: ResMut<client_bevy::game::dialogs::group::GroupState>,
    mut accepted: Local<bool>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    if *accepted {
        return;
    }
    if let Some(inv) = group.invite.clone() {
        net.send_packet(&mir2_shared::packets::client::group::GroupInvite {
            accept_invite: true,
        });
        tracing::info!("[GROUPACCEPT] ✅ 接受邀请: {}", inv.inviter_name);
        group.invite = None;
        *accepted = true;
    }
}

/// --mail-test：自动发邮件（登录后向 bevy2char 发 SendMail，含金币）
#[allow(clippy::too_many_arguments)]
fn auto_mail_test(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut t: Local<f32>,
    mut sent: Local<bool>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    if *sent {
        return;
    }
    *t += time.delta_secs();
    if *t < 8.0 {
        return;
    }
    let receiver = std::env::args()
        .skip_while(|a| a != "--e2e-receiver")
        .nth(1)
        .unwrap_or_else(|| "bevy2char".to_string());
    net.send_packet(&mir2_shared::packets::client::mail::SendMail {
        name: receiver.clone(),
        message: "HelloSubject\n邮件正文测试 100 金币".to_string(),
        gold: 100,
        items_idx: [0; 5],
        stamped: false,
    });
    tracing::info!("[MAILTEST] 发送邮件给 {} (含 100 金币)", receiver);
    *sent = true;
}

/// --mail-read：自动读取新邮件（收到列表条目 → ReadMail → 详情）
#[allow(clippy::too_many_arguments)]
fn auto_mail_read(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    mail: Res<client_bevy::game::dialogs::mail::MailState>,
    mut read_ids: Local<std::collections::HashSet<u64>>,
    mut done: Local<bool>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    if *done {
        return;
    }
    if let Some(d) = mail.detail.as_ref() {
        tracing::info!(
            "[MAILREAD] ✅ 已读取邮件: {} - {} 金币={} 正文={}",
            d.sender,
            d.subject,
            d.gold,
            d.body
        );
        *done = true;
        return;
    }
    for m in mail.mails.iter() {
        if m.unread && !read_ids.contains(&m.mail_id) {
            net.send_packet(&mir2_shared::packets::client::mail::ReadMail {
                mail_id: m.mail_id,
            });
            tracing::info!("[MAILREAD] 请求读取: {} ({})", m.subject, m.mail_id);
            read_ids.insert(m.mail_id);
        }
    }
}

/// --auto-enter：自动驱动 mock 登录流程（Login→Select→Game，验证网络管道）
fn auto_enter(
    mut net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<AppState>>,
    time: Res<Time>,
    mut login_sent: Local<bool>,
    mut select_timer: Local<f32>,
) {
    use mir2_shared::packets::client::account::{Login, StartGame};
    if *state == AppState::Login && !*login_sent {
        *login_sent = true;
        net.state = client_bevy::network::NetState::LoggingIn;
        net.send_packet(&Login {
            account_id: {
            let user = std::env::args()
                .skip_while(|a| a != "--e2e-user")
                .nth(1)
                .unwrap_or_else(|| "test".to_string());
            user
        },
        password: {
            let pass = std::env::args()
                .skip_while(|a| a != "--e2e-pass")
                .nth(1)
                .unwrap_or_else(|| "123456".to_string());
            pass
        },
        });
    }
    // 在选角界面停留 3 秒再进游戏（便于 live 截屏验证选角界面）
    if *state == AppState::Select && net.selected_index.is_none() {
        *select_timer += time.delta_secs();
        if *select_timer >= 3.0 {
            let first_index = net.characters.first().map(|c| c.index);
            if let Some(idx) = first_index {
                net.selected_index = Some(idx);
                net.send_packet(&StartGame {
                    character_index: idx,
                });
            }
        }
    }
}

/// BEVY_DEMO_DELETE=1：自动登录→进选角→选中角色→打开删除询问框（截图验证用）
fn demo_delete_flow(
    mut net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<AppState>>,
    mut modal: ResMut<client_bevy::ui::modal_box::ModalState>,
    time: Res<Time>,
    mut login_sent: Local<bool>,
    mut select_timer: Local<f32>,
    mut opened: Local<bool>,
) {
    use mir2_shared::packets::client::account::Login;
    if *state == AppState::Login && !*login_sent {
        *login_sent = true;
        net.state = client_bevy::network::NetState::LoggingIn;
        net.send_packet(&Login {
            account_id: "test".to_string(),
            password: "123456".to_string(),
        });
    }
    if *state == AppState::Select && !*opened {
        *select_timer += time.delta_secs();
        if *select_timer >= 1.0 {
            *opened = true;
            if net.selected_index.is_none() {
                net.selected_index = net.characters.first().map(|c| c.index);
            }
            modal.kind = client_bevy::ui::modal_box::ModalKind::DeleteAsk;
            tracing::info!("[DEMO] 打开删除询问框, selected={:?}", net.selected_index);
        }
    }
}

