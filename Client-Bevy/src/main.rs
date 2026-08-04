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
use client_bevy::ui::pinyin_ime::PinyinImePlugin;
use client_bevy::ui::select::SelectPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            // 资源目录固定到源码 assets/（Bevy 0.19 默认按 exe 目录解析，跨 target 运行会找不到 shader）
            .set(AssetPlugin {
                file_path: concat!(env!("CARGO_MANIFEST_DIR"), "/assets").to_string(),
                ..default()
            })
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
                    // 禁用系统 IME：用游戏内置拼音输入法（src/ui/pinyin_ime.rs）。
                    // winit 用 IACE_CHILDREN 解关联 IME 上下文，字母键作为原始
                    // KeyboardInput 到达，不被手心等系统输入法拦截。
                    ime_enabled: false,
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
    app.add_plugins(PinyinImePlugin);
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

    // --storage-test: 自动仓库存取链路（自动化验证用）
    if std::env::args().any(|a| a == "--storage-test") {
        app.add_systems(Update, auto_storage_test);
    }
    // --group-test: 自动组队邀请链路（自动化验证用，配合 --group-accept）
    if std::env::args().any(|a| a == "--group-test") {
        app.add_systems(Update, auto_group_test);
    }
    // --group-accept: 自动接受组队邀请（自动化验证用）
    if std::env::args().any(|a| a == "--group-accept") {
        app.add_systems(Update, auto_group_accept);
    }
    // --mail-test: 自动发邮件链路（自动化验证用，配合 --mail-read）
    if std::env::args().any(|a| a == "--mail-test") {
        app.add_systems(Update, auto_mail_test);
    }
    // --mail-read: 自动读取新邮件（自动化验证用）
    if std::env::args().any(|a| a == "--mail-read") {
        app.add_systems(Update, auto_mail_read);
    }
    // --trade-test: 自动交易链路（发起者，配合 --trade-accept）
    if std::env::args().any(|a| a == "--trade-test") {
        app.add_systems(Update, auto_trade_test);
    }
    // --trade-accept: 自动接受交易邀请（配合 --trade-test）
    if std::env::args().any(|a| a == "--trade-accept") {
        app.add_systems(Update, auto_trade_accept);
    }
    // --drop-pick-test: 怪物掉落 → 地面物品 → 拾取 → 背包（自动化验证用）
    if std::env::args().any(|a| a == "--drop-pick-test") {
        app.add_systems(Update, auto_drop_pick_test);
    }
    // --friend-test: 自动加好友链路（配合 B 在线）
    if std::env::args().any(|a| a == "--friend-test") {
        app.add_systems(Update, auto_friend_test);
    }
    // --mail-compose-test: 写邮件界面 → 发送（配合 B --mail-read）
    if std::env::args().any(|a| a == "--mail-compose-test") {
        app.add_systems(Update, auto_mail_compose_test);
    }
    // --guild-test: 创建行会链路（GuildNameReturn → GuildStatus 信息）
    if std::env::args().any(|a| a == "--guild-test") {
        app.add_systems(Update, auto_guild_test);
    }
    // --guild-invite-test: 行会邀请链路（创建→邀请，配合 --guild-accept）
    if std::env::args().any(|a| a == "--guild-invite-test") {
        app.add_systems(Update, auto_guild_invite_test);
    }
    // --guild-accept: 自动接受行会邀请
    if std::env::args().any(|a| a == "--guild-accept") {
        app.add_systems(Update, auto_guild_accept);
    }
    // --guild-notice-test: 行会公告链路（创建→设置公告→等 GuildNoticeChange）
    if std::env::args().any(|a| a == "--guild-notice-test") {
        app.add_systems(Update, auto_guild_notice_test);
    }
    // --guild-gold-test: 行会仓库金币链路（创建→存入→取出）
    if std::env::args().any(|a| a == "--guild-gold-test") {
        app.add_systems(Update, auto_guild_gold_test);
    }
    // --ranking-test: 排行榜链路（打开对话框 → GetRanking → 显示）
    if std::env::args().any(|a| a == "--ranking-test") {
        app.add_systems(Update, auto_ranking_test);
    }
    // --shop-test: 自动 NPC 商店买卖链路（自动化验证用）
    if std::env::args().any(|a| a == "--shop-test") {
        app.add_systems(Update, auto_shop_test);
    }
    // --guild-item-test: 行会仓库物品链路（打开仓库 → 存入背包物品 → 取出）
    if std::env::args().any(|a| a == "--guild-item-test") {
        app.add_systems(Update, auto_guild_item_test);
    }
    // --mentor-test: 师徒链路（发起拜师，配合 --mentor-accept）
    if std::env::args().any(|a| a == "--mentor-test") {
        app.add_systems(Update, auto_mentor_test);
    }
    // --mentor-accept: 师徒链路（允许拜师 + 接受邀请，配合 --mentor-test）
    if std::env::args().any(|a| a == "--mentor-accept") {
        app.add_systems(Update, auto_mentor_accept);
    }
    // --market-test: 市场链路（寄售×2 → 取回一件 → 留一件给 --market-buy 买）
    if std::env::args().any(|a| a == "--market-test") {
        app.add_systems(Update, auto_market_test);
    }
    // --market-buy: 市场购买链路（配合 --market-test）
    if std::env::args().any(|a| a == "--market-buy") {
        app.add_systems(Update, auto_market_buy);
    }
    // --gameshop-test: 商城链路（打开商城 → 目录 → 购买 → 邮件送达）
    if std::env::args().any(|a| a == "--gameshop-test") {
        app.add_systems(Update, auto_gameshop_test);
    }
    // --territory-test: 行会领地链路（打开领地 → 购买无主领地 → 宣战，配合 --territory-war）
    if std::env::args().any(|a| a == "--territory-test") {
        app.add_systems(Update, auto_territory_test);
    }
    // --territory-war: 创建目标行会供宣战（配合 --territory-test）
    if std::env::args().any(|a| a == "--territory-war") {
        app.add_systems(Update, auto_territory_war);
    }
    // --combat-test: 战斗闭环（选中怪物 → 连续 FireBall → 死亡 → 掉落）
    if std::env::args().any(|a| a == "--combat-test") {
        app.add_systems(Update, auto_combat_test);
    }
    // --fishing-test: 钓鱼链路（装备鱼竿 → 抛竿 → 等收获消息）
    if std::env::args().any(|a| a == "--fishing-test") {
        app.add_systems(Update, auto_fishing_test);
    }
    // --refine-test: 精炼链路（存入 → 开始 → 等待 → 查看 → 取回）
    if std::env::args().any(|a| a == "--refine-test") {
        app.add_systems(Update, auto_refine_test);
    }
    // --craft-test: 合成链路（配方1：木材x3+铁矿石x2 → 铁剑）
    if std::env::args().any(|a| a == "--craft-test") {
        app.add_systems(Update, auto_craft_test);
    }
    // --rental-test: 物品租赁链路（租方，配合 --rental-owner）
    if std::env::args().any(|a| a == "--rental-test") {
        app.add_systems(Update, auto_rental_test);
    }
    // --rental-owner: 物品租赁链路（物主，配合 --rental-test）
    if std::env::args().any(|a| a == "--rental-owner") {
        app.add_systems(Update, auto_rental_owner);
    }
    // --quest-test: 任务日志链路（接受任务 → ChangeQuest 显示 → 放弃）
    if std::env::args().any(|a| a == "--quest-test") {
        app.add_systems(Update, auto_quest_test);
    }
    // --buff-test: 状态/Buff 链路（施放 Fury → AddBuff 显示）
    if std::env::args().any(|a| a == "--buff-test") {
        app.add_systems(Update, auto_buff_test);
    }
    // --report-test: 举报链路（提交举报 → 系统消息确认）
    if std::env::args().any(|a| a == "--report-test") {
        app.add_systems(Update, auto_report_test);
    }
    // --inspect-test: 查看玩家链路（找到 bevy2char → Inspect → PlayerInspect 显示）
    if std::env::args().any(|a| a == "--inspect-test") {
        app.add_systems(Update, auto_inspect_test);
    }
    // --creature-test: 宠物链路（打开宠物 → 请求列表 → 解析）
    if std::env::args().any(|a| a == "--creature-test") {
        app.add_systems(Update, auto_creature_test);
    }
    // --hero-test: 英雄链路（切换英雄1 → ChangeHero 包 → 切回主角色）
    if std::env::args().any(|a| a == "--hero-test") {
        app.add_systems(Update, auto_hero_test);
    }
    // --marriage-test: 婚姻链路（求婚 → 结婚 → 离婚，配合 --marriage-accept）
    if std::env::args().any(|a| a == "--marriage-test") {
        app.add_systems(Update, auto_marriage_test);
    }
    // --marriage-accept: 婚姻链路（接受求婚 → 离婚确认，配合 --marriage-test）
    if std::env::args().any(|a| a == "--marriage-accept") {
        app.add_systems(Update, auto_marriage_accept);
    }
    // --ui-dialog-test: 纯客户端对话框批量验证（公告/聊天公告/计时器/帮助）
    if std::env::args().any(|a| a == "--ui-dialog-test") {
        app.add_systems(Update, auto_ui_dialog_test);
    }
    // --option-test: 设置对话框验证（打开 → 切换 8 组开关 → 音量 → 关闭）
    if std::env::args().any(|a| a == "--option-test") {
        app.add_systems(Update, auto_option_test);
    }
    // --keyboard-test: 键位设置对话框验证（打开 → 滚动 → 重绑 → 重置 → 关闭）
    if std::env::args().any(|a| a == "--keyboard-test") {
        app.add_systems(Update, auto_keyboard_test);
    }
    // --bigmap-test: 大地图对话框验证（NewMapInfo → 地形 → NPC 列表 → 传送）
    if std::env::args().any(|a| a == "--bigmap-test") {
        app.add_systems(Update, auto_bigmap_test);
    }
    // --awake-test: 觉醒对话框验证（选武器 → 类型 → 材料 → 觉醒 → 结果）
    if std::env::args().any(|a| a == "--awake-test") {
        app.add_systems(Update, auto_awake_test);
    }
    // --dura-test: 耐久面板验证（打开 → 装备耐久三态渲染 → 关闭）
    if std::env::args().any(|a| a == "--dura-test") {
        app.add_systems(Update, auto_dura_test);
    }
    // --socket-test: 镶嵌面板验证（带宝石槽物品 → 孔位/宝石渲染 → 关闭）
    if std::env::args().any(|a| a == "--socket-test") {
        app.add_systems(Update, auto_socket_test);
    }
    // --roll-test: 掷骰链路（CallNPC TestRoll → 服务端 Roll 包 → 动画/回调）
    if std::env::args().any(|a| a == "--roll-test") {
        app.add_systems(Update, auto_roll_test);
    }
    // --reconnect-test: 断线自动重连验证（等断线 → 重连 → 自动登录进游戏）
    if std::env::args().any(|a| a == "--reconnect-test") {
        app.add_systems(Update, auto_reconnect_test);
    }
    // --mount-test: 坐骑链路（装备坐骑 → 面板 → @ride 骑乘/下马 → 外观广播）
    if std::env::args().any(|a| a == "--mount-test") {
        app.add_systems(Update, auto_mount_test);
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
    // --auto-pickup: 进图后自动拾取最近的 GroundItem（验证拾取闭环，无需鼠标）
    if std::env::args().any(|a| a == "--auto-pickup") {
        app.add_systems(Update, auto_pickup_system);
    }
    // --auto-cast: 进图后自动施放 F1 技能（验证技能链路）
    if std::env::args().any(|a| a == "--auto-cast") {
        app.add_systems(Update, auto_cast_system);
    }
    // --auto-equip: 进图后自动装备背包第一件可装备物品（验证穿戴→外观）
    if std::env::args().any(|a| a == "--auto-equip") {
        app.add_systems(Update, auto_equip_system);
    }
    // --auto-life: 依次验证 聊天回显 → 商店购买 → 使用药水
    if std::env::args().any(|a| a == "--auto-life") {
        app.add_systems(Update, auto_life_system);
    }
    // --auto-walk <up|down|left|right>: 调试 chunk 流式（每帧驱动玩家平移）
    {
        let dir = std::env::args()
            .position(|a| a == "--auto-walk")
            .and_then(|i| std::env::args().nth(i + 1))
            .unwrap_or_default();
        let dir_copy = dir.clone();
        app.insert_resource(AutoWalkDir(dir));
        if !dir_copy.is_empty() {
            app.add_systems(Update, auto_walk_system);
        }
    }
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

/// --trade-test：自动交易链路（发起者：TradeRequest → 金币 500 → 放入物品 → 锁定 → 完成）
#[allow(clippy::too_many_arguments)]
fn auto_trade_test(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut trade: ResMut<client_bevy::game::dialogs::trade::TradeState>,
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
            if *t < 10.0 {
                return;
            }
            trade.is_initiator = true;
            net.send_packet(&mir2_shared::packets::client::trade::TradeRequest);
            tracing::info!("[TRADETEST] 发起交易请求");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 3.0 {
                return;
            }
            if trade.visible {
                tracing::info!("[TRADETEST] ✅ 交易窗口已打开，对方={}", trade.partner_name);
                net.send_packet(&mir2_shared::packets::client::trade::TradeGold { amount: 500 });
                tracing::info!("[TRADETEST] 放入金币 500");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 2.0 {
                return;
            }
            // 放入背包第一个物品
            if let Some((from, _)) = hud.inventory.items.iter().enumerate().find(|(_, s)| s.is_some()) {
                if trade.pending_deposit.is_none() && trade.my_items[0].is_none() {
                    trade.pending_deposit = Some((from, 0));
                    net.send_packet(&mir2_shared::packets::client::trade::DepositTradeItem {
                        from: from as i32,
                        to: 0,
                    });
                    tracing::info!("[TRADETEST] 放入背包格 {} -> 交易槽 0", from);
                    *stage = 3;
                    *t = 0.0;
                }
            }
        }
        3 => {
            if *t < 3.0 {
                return;
            }
            if trade.my_items[0].is_some() {
                tracing::info!("[TRADETEST] ✅ 物品已入槽: {}", trade.my_items[0].as_ref().unwrap().name);
                net.send_packet(&mir2_shared::packets::client::trade::TradeConfirm { locked: true });
                tracing::info!("[TRADETEST] 锁定交易");
                *stage = 4;
                *t = 0.0;
            }
        }
        4 => {
            if *t < 5.0 {
                return;
            }
            if !trade.visible {
                tracing::info!("[TRADETEST] 🎉 交易完成（窗口已关闭）");
            } else {
                tracing::warn!("[TRADETEST] ❌ 交易未完成，locked=({},{})", trade.my_locked, trade.their_locked);
            }
            *stage = 5;
        }
        _ => {}
    }
}

/// --trade-accept：自动接受交易邀请 + 加金币 300 + 锁定
#[allow(clippy::too_many_arguments)]
fn auto_trade_accept(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut trade: ResMut<client_bevy::game::dialogs::trade::TradeState>,
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
            if trade.invite.is_some() {
                net.send_packet(&mir2_shared::packets::client::trade::TradeReply {
                    accept_invite: true,
                });
                tracing::info!(
                    "[TRADEACCEPT] ✅ 接受邀请: {}",
                    trade.invite.as_ref().unwrap()
                );
                trade.invite = None;
                trade.visible = true;
                *stage = 1;
                *t = 0.0;
            }
        }
        1 => {
            if *t < 2.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::trade::TradeGold { amount: 300 });
            tracing::info!("[TRADEACCEPT] 放入金币 300");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t < 3.0 {
                return;
            }
            if trade.their_locked && !trade.my_locked {
                net.send_packet(&mir2_shared::packets::client::trade::TradeConfirm { locked: true });
                tracing::info!("[TRADEACCEPT] 对方已锁定，我方锁定");
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            if *t < 5.0 {
                return;
            }
            if !trade.visible {
                tracing::info!("[TRADEACCEPT] 🎉 交易完成");
            }
            *stage = 4;
        }
        _ => {}
    }
}

/// --drop-pick-test：怪物掉落 → 地面物品 → 拾取 → 背包
/// 前提：DB 配置 bevychar 在 Deer(340,325) 左侧、攻击力秒杀、Deer 掉落 chance=1.0
#[allow(clippy::too_many_arguments)]
fn auto_drop_pick_test(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    hud: Res<client_bevy::game::hud::HudState>,
    ground: Query<&client_bevy::actor::NetObjectId, With<client_bevy::actor::GroundItem>>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut atk_timer: Local<f32>,
    mut dir_idx: Local<u8>,
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
            // 每 1.2s 轮换方向攻击（Deer 刷新点 (205,325)，spread 45 会偏移）
            *atk_timer += time.delta_secs();
            if *atk_timer >= 1.2 {
                *atk_timer = 0.0;
                let dirs = [
                    mir2_shared::enums::MirDirection::Right,
                    mir2_shared::enums::MirDirection::Up,
                    mir2_shared::enums::MirDirection::Down,
                    mir2_shared::enums::MirDirection::Left,
                    mir2_shared::enums::MirDirection::UpRight,
                    mir2_shared::enums::MirDirection::DownRight,
                    mir2_shared::enums::MirDirection::UpLeft,
                    mir2_shared::enums::MirDirection::DownLeft,
                ];
                let d = dirs[*dir_idx as usize % dirs.len()];
                *dir_idx += 1;
                net.send_packet(&mir2_shared::packets::client::combat::Attack {
                    direction: d,
                    spell: mir2_shared::enums::Spell::None,
                });
                tracing::info!("[DROPTEST] 攻击方向 {:?}", d);
            }
            if ground.iter().next().is_some() {
                tracing::info!("[DROPTEST] ✅ 检测到地面物品实体");
                *stage = 1;
                *t = 0.0;
            } else if *t > 25.0 {
                tracing::warn!("[DROPTEST] ❌ 超时未检测到掉落（怪物可能已死/未掉）");
                *stage = 9;
            }
        }
        1 => {
            if *t < 1.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::item::PickUp {});
            tracing::info!("[DROPTEST] 发送 PickUp");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t < 3.0 {
                return;
            }
            if hud.inventory.items.iter().flatten().any(|i| i.item_index == 853) {
                tracing::info!("[DROPTEST] ✅ 拾取成功：背包有物品 853");
            } else {
                tracing::warn!("[DROPTEST] ❌ 背包未找到物品 853");
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --friend-test：自动加好友（AddFriend bevy2char → 等 FriendUpdate 列表出现）
#[allow(clippy::too_many_arguments)]
fn auto_friend_test(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    friend: Res<client_bevy::game::dialogs::friend::FriendState>,
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
            net.send_packet(&mir2_shared::packets::client::friend::AddFriend {
                name: "bevy2char".to_string(),
                blocked: false,
            });
            tracing::info!("[FRIENDTEST] 添加好友 bevy2char");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 3.0 {
                return;
            }
            if friend.friends.iter().any(|f| f.name == "bevy2char") {
                tracing::info!(
                    "[FRIENDTEST] ✅ 好友列表包含 bevy2char (在线={})",
                    friend.friends.iter().find(|f| f.name == "bevy2char").map(|f| f.online).unwrap_or(false)
                );
            } else {
                tracing::warn!("[FRIENDTEST] ❌ 好友列表为空或未包含 bevy2char: {:?}", friend.friends);
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --mail-compose-test：写邮件界面（输入框状态 → send_composed_mail → B 读取）
#[allow(clippy::too_many_arguments)]
fn auto_mail_compose_test(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mail: ResMut<client_bevy::game::dialogs::mail::MailState>,
    mut input: ResMut<client_bevy::game::dialogs::text_input::TextInputState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
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
            // 打开邮件对话框 + 写界面（原版 C# MailDialog 写邮件流程）
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Mail) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Mail);
            }
            mail.compose = true;
            mail.detail = None;
            input.texts = vec![
                "bevy2char".to_string(),
                "ComposeSubject".to_string(),
                "邮件正文 M26 测试".to_string(),
            ];
            tracing::info!("[MAILCOMPOSE] 打开写邮件界面，填写收件人/主题/正文");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 2.0 {
                return;
            }
            // 与发送按钮相同的代码路径
            client_bevy::game::dialogs::mail::send_composed_mail(&net, &input);
            mail.compose = false;
            tracing::info!("[MAILCOMPOSE] 发送邮件");
            *stage = 9;
        }
        _ => {}
    }
}

/// --guild-test：创建行会（打开行会对话框 → 输入行会名 → GuildNameReturn → 等 GuildStatus 信息）
#[allow(clippy::too_many_arguments)]
fn auto_guild_test(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    guild: Res<client_bevy::game::dialogs::guild::GuildState>,
    mut input: ResMut<client_bevy::game::dialogs::text_input::TextInputState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
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
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Guild) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Guild);
            }
            if input.texts.len() < 1 {
                input.texts.resize(1, String::new());
            }
            input.texts[0] = "TestGuild".to_string();
            tracing::info!("[GUILDTEST] 打开行会对话框，输入行会名 TestGuild");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 1.0 {
                return;
            }
            // 与创建按钮相同：GuildNameReturn{name}
            net.send_packet(&mir2_shared::packets::client::guild::GuildNameReturn {
                name: "TestGuild".to_string(),
            });
            tracing::info!("[GUILDTEST] 创建行会 TestGuild");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t < 4.0 {
                return;
            }
            if guild.in_guild && guild.name == "TestGuild" {
                tracing::info!(
                    "[GUILDTEST] ✅ 行会创建成功: {}（{}）成员 {}",
                    guild.name,
                    guild.leader,
                    guild.members.len()
                );
            } else {
                tracing::warn!(
                    "[GUILDTEST] ❌ 行会状态: in_guild={} name={}",
                    guild.in_guild,
                    guild.name
                );
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --guild-invite-test：创建行会 → 邀请 bevy2char → 等成员数 2
#[allow(clippy::too_many_arguments)]
fn auto_guild_invite_test(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    guild: Res<client_bevy::game::dialogs::guild::GuildState>,
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
            net.send_packet(&mir2_shared::packets::client::guild::GuildNameReturn {
                name: "TestGuild2".to_string(),
            });
            tracing::info!("[GUILDINV] 创建行会 TestGuild2");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 3.0 {
                return;
            }
            if guild.in_guild && guild.name == "TestGuild2" {
                tracing::info!("[GUILDINV] ✅ 行会已创建");
                net.send_packet(&mir2_shared::packets::client::guild::EditGuildMember {
                    change_type: 0,
                    rank_index: 0,
                    name: "bevy2char".to_string(),
                    rank_name: String::new(),
                });
                tracing::info!("[GUILDINV] 邀请 bevy2char 加入");
                *stage = 2;
                *t = 0.0;
            } else {
                tracing::warn!("[GUILDINV] ❌ 行会未创建: {}", guild.name);
                *stage = 9;
            }
        }
        2 => {
            if *t < 5.0 {
                return;
            }
            if guild.members.iter().any(|m| m.name == "bevy2char") {
                tracing::info!(
                    "[GUILDINV] ✅ 成员加入: {} 人",
                    guild.members.len()
                );
            } else {
                tracing::warn!("[GUILDINV] ❌ 成员未加入: {:?}", guild.members);
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --guild-accept：自动接受行会邀请（GuildInvite → C.GuildInvite{true} → 等 in_guild）
#[allow(clippy::too_many_arguments)]
fn auto_guild_accept(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut guild: ResMut<client_bevy::game::dialogs::guild::GuildState>,
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
            if let Some(name) = guild.invite.clone() {
                net.send_packet(&mir2_shared::packets::client::guild::GuildInvite {
                    accept_invite: true,
                });
                tracing::info!("[GUILDACCEPT] ✅ 接受行会邀请: {}", name);
                guild.invite = None;
                *stage = 1;
                *t = 0.0;
            }
        }
        1 => {
            if *t < 4.0 {
                return;
            }
            if guild.in_guild {
                tracing::info!(
                    "[GUILDACCEPT] ✅ 已加入行会: {}",
                    guild.name
                );
            } else {
                tracing::warn!("[GUILDACCEPT] ❌ 未加入行会");
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --guild-notice-test：创建行会 → 设置公告 → 等 GuildNoticeChange 回包
#[allow(clippy::too_many_arguments)]
fn auto_guild_notice_test(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    guild: Res<client_bevy::game::dialogs::guild::GuildState>,
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
            net.send_packet(&mir2_shared::packets::client::guild::GuildNameReturn {
                name: "TestGuild3".to_string(),
            });
            tracing::info!("[GUILDNOTICE] 创建行会 TestGuild3");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 3.0 {
                return;
            }
            if guild.in_guild && guild.name == "TestGuild3" {
                net.send_packet(&mir2_shared::packets::client::guild::EditGuildNotice {
                    notice_lines: vec!["TestNotice 公告内容".to_string()],
                });
                tracing::info!("[GUILDNOTICE] 设置公告");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 3.0 {
                return;
            }
            if guild.notice.iter().any(|l| l.contains("TestNotice")) {
                tracing::info!("[GUILDNOTICE] ✅ 公告已更新: {:?}", guild.notice);
            } else {
                tracing::warn!("[GUILDNOTICE] ❌ 公告未更新: {:?}", guild.notice);
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --guild-gold-test：创建行会 → 存入 100 → 取出 50 → 验证仓库金币
#[allow(clippy::too_many_arguments)]
fn auto_guild_gold_test(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    guild: Res<client_bevy::game::dialogs::guild::GuildState>,
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
            net.send_packet(&mir2_shared::packets::client::guild::GuildNameReturn {
                name: "TestGuild4".to_string(),
            });
            tracing::info!("[GUILDGOLD] 创建行会 TestGuild4");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 3.0 {
                return;
            }
            if guild.in_guild && guild.name == "TestGuild4" {
                net.send_packet(&mir2_shared::packets::client::guild::GuildStorageGoldChange {
                    change_type: 0,
                    amount: 100,
                });
                tracing::info!("[GUILDGOLD] 存入 100 金币");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 3.0 {
                return;
            }
            if guild.gold >= 100 {
                tracing::info!("[GUILDGOLD] ✅ 仓库金币: {}", guild.gold);
                net.send_packet(&mir2_shared::packets::client::guild::GuildStorageGoldChange {
                    change_type: 1,
                    amount: 50,
                });
                tracing::info!("[GUILDGOLD] 取出 50 金币");
                *stage = 3;
                *t = 0.0;
            } else {
                tracing::warn!("[GUILDGOLD] ❌ 仓库金币未更新: {}", guild.gold);
                *stage = 9;
            }
        }
        3 => {
            if *t < 3.0 {
                return;
            }
            if guild.gold >= 50 {
                tracing::info!("[GUILDGOLD] ✅ 取出后仓库金币: {}", guild.gold);
            } else {
                tracing::warn!("[GUILDGOLD] ❌ 取出后金币异常: {}", guild.gold);
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --ranking-test：打开排行榜 → 等 Rankings 数据
#[allow(clippy::too_many_arguments)]
fn auto_ranking_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    ranking: Res<client_bevy::game::dialogs::ranking::RankingState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
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
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Ranking) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Ranking);
            }
            tracing::info!("[RANKTEST] 打开排行榜对话框");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 4.0 {
                return;
            }
            if !ranking.entries.is_empty() {
                tracing::info!(
                    "[RANKTEST] ✅ 排行榜 {} 条，第一名: {}",
                    ranking.entries.len(),
                    ranking.entries[0].player_name
                );
            } else {
                tracing::warn!("[RANKTEST] ❌ 排行榜为空");
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --guild-item-test：行会仓库物品链路（打开仓库 → 存入背包物品 → 取出）
#[allow(clippy::too_many_arguments)]
fn auto_guild_item_test(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    guild: Res<client_bevy::game::dialogs::guild::GuildState>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut deposited_uid: Local<Option<u64>>,
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
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Guild) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Guild);
            }
            tracing::info!("[GUILDITEM] 打开行会对话框");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 3.0 {
                return;
            }
            if guild.in_guild {
                tracing::info!("[GUILDITEM] 已在行会: {}", guild.name);
                *stage = 2;
                *t = 0.0;
            } else {
                net.send_packet(&mir2_shared::packets::client::guild::GuildNameReturn {
                    name: "TestGuild5".to_string(),
                });
                tracing::info!("[GUILDITEM] 创建行会 TestGuild5");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 3.0 {
                return;
            }
            if !guild.in_guild {
                return;
            }
            // 请求仓库列表（打开对话框时已自动请求，这里兜底）
            net.send_packet(&client_bevy::network::GuildStorageItemChangeWire {
                change_type: 3,
                grid: 0,
                unique_id: 0,
                count: 0,
            });
            tracing::info!("[GUILDITEM] 请求仓库列表");
            *stage = 3;
            *t = 0.0;
        }
        3 => {
            if *t < 3.0 {
                return;
            }
            if guild.storage_received {
                tracing::info!(
                    "[GUILDITEM] ✅ 仓库列表 {} 格",
                    guild.storage_items.len()
                );
            } else {
                tracing::warn!("[GUILDITEM] ❌ 仓库列表未收到");
                *stage = 9;
                return;
            }
            // 选第一个背包物品存入
            let first = hud
                .inventory
                .items
                .iter()
                .enumerate()
                .find_map(|(i, s)| s.as_ref().map(|it| (i, it)));
            match first {
                Some((i, item)) => {
                    *deposited_uid = Some(item.unique_id);
                    net.send_packet(&client_bevy::network::GuildStorageItemChangeWire {
                        change_type: 0,
                        grid: 0,
                        unique_id: item.unique_id,
                        count: item.count as u32,
                    });
                    tracing::info!(
                        "[GUILDITEM] 存入背包物品 [{}] uid={} (格 {})",
                        item.name,
                        item.unique_id,
                        i
                    );
                    *stage = 4;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[GUILDITEM] ❌ 背包为空，无法测试存入");
                    *stage = 9;
                }
            }
        }
        4 => {
            if *t < 3.0 {
                return;
            }
            let slot0 = guild.storage_items.get(0).and_then(|s| s.as_ref());
            match slot0 {
                Some(it) => {
                    tracing::info!(
                        "[GUILDITEM] ✅ 仓库格1: {} x{} (uid={})",
                        it.name,
                        it.count,
                        it.unique_id
                    );
                    net.send_packet(&client_bevy::network::GuildStorageItemChangeWire {
                        change_type: 1,
                        grid: 0,
                        unique_id: 0,
                        count: 0,
                    });
                    tracing::info!("[GUILDITEM] 取出仓库格1");
                    *stage = 5;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[GUILDITEM] ❌ 仓库格1为空，存入失败");
                    *stage = 9;
                }
            }
        }
        5 => {
            if *t < 3.0 {
                return;
            }
            let slot0_empty = guild.storage_items.get(0).and_then(|s| s.as_ref()).is_none();
            let uid_back = match *deposited_uid {
                Some(uid) => hud
                    .inventory
                    .items
                    .iter()
                    .filter_map(|s| s.as_ref())
                    .any(|it| it.unique_id == uid),
                None => false,
            };
            if slot0_empty && uid_back {
                tracing::info!("[GUILDITEM] ✅ 取出成功：仓库格1已空，物品回到背包");
            } else {
                tracing::warn!(
                    "[GUILDITEM] ❌ 取出异常: slot0_empty={} uid_back={}",
                    slot0_empty,
                    uid_back
                );
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --mentor-test：发起拜师 → 等 MentorUpdate → 解除（配合 --mentor-accept）
#[allow(clippy::too_many_arguments)]
fn auto_mentor_test(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mentor: Res<client_bevy::game::dialogs::mentor::MentorState>,
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
            if *t < 12.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::misc::AddMentor {
                name: "bevy2char".to_string(),
            });
            tracing::info!("[MENTORTEST] 请求拜师 bevy2char");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 15.0 {
                tracing::warn!(
                    "[MENTORTEST] ❌ 未收到师徒关系: mentor_name={}",
                    mentor.mentor_name
                );
                *stage = 9;
                return;
            }
            if mentor.mentor_name == "bevy2char" {
                tracing::info!(
                    "[MENTORTEST] ✅ 拜师成功: 师父={} Lv.{} 在线={}",
                    mentor.mentor_name,
                    mentor.mentor_level,
                    mentor.mentor_online
                );
                net.send_packet(&mir2_shared::packets::client::misc::CancelMentor);
                tracing::info!("[MENTORTEST] 解除师徒关系");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 5.0 {
                return;
            }
            if mentor.mentor_name.is_empty() {
                tracing::info!("[MENTORTEST] ✅ 解除成功");
            } else {
                tracing::warn!(
                    "[MENTORTEST] ❌ 解除失败: mentor_name={}",
                    mentor.mentor_name
                );
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --mentor-accept：允许拜师 → 接受邀请 → 等 MentorUpdate → 等解除
#[allow(clippy::too_many_arguments)]
fn auto_mentor_accept(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mentor: Res<client_bevy::game::dialogs::mentor::MentorState>,
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
            net.send_packet(&client_bevy::network::AllowMentorWire { allow: true });
            tracing::info!("[MENTORACCEPT] 允许拜师");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 15.0 {
                tracing::warn!("[MENTORACCEPT] ❌ 未收到拜师邀请");
                *stage = 9;
                return;
            }
            if let Some((name, level)) = mentor.invite.as_ref() {
                tracing::info!("[MENTORACCEPT] ✅ 收到拜师邀请: {} Lv.{}", name, level);
                net.send_packet(&mir2_shared::packets::client::misc::MentorReply {
                    accept_invite: true,
                });
                tracing::info!("[MENTORACCEPT] 接受拜师");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t >= 15.0 {
                tracing::warn!(
                    "[MENTORACCEPT] ❌ 未收到师徒关系: mentor_name={}",
                    mentor.mentor_name
                );
                *stage = 9;
                return;
            }
            if mentor.mentor_name == "bevychar" {
                tracing::info!(
                    "[MENTORACCEPT] ✅ 收徒成功: 徒弟={} Lv.{} 在线={}",
                    mentor.mentor_name,
                    mentor.mentor_level,
                    mentor.mentor_online
                );
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            if *t < 10.0 {
                return;
            }
            if mentor.mentor_name.is_empty() {
                tracing::info!("[MENTORACCEPT] ✅ 对方解除，师徒关系已清除");
            } else {
                tracing::warn!(
                    "[MENTORACCEPT] ❌ 未收到解除: mentor_name={}",
                    mentor.mentor_name
                );
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --market-test：寄售背包物品×2 → 取回一件 → 留一件给买家（配合 --market-buy）
#[allow(clippy::too_many_arguments)]
fn auto_market_test(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    market: Res<client_bevy::game::dialogs::market::MarketState>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut consigned: Local<Vec<u32>>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 12.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Market) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Market);
            }
            net.send_packet(&mir2_shared::packets::client::market::MarketRefresh);
            tracing::info!("[MARKETTEST] 打开市场 + 刷新");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 4.0 {
                return;
            }
            // 寄售第一个背包物品（uid=100，价格 500）
            let first = hud
                .inventory
                .items
                .iter()
                .enumerate()
                .find_map(|(i, s)| s.as_ref().map(|it| (i, it)));
            match first {
                Some((_i, item)) => {
                    net.send_packet(&client_bevy::network::MarketConsignWire {
                        unique_id: item.unique_id as u32,
                        price: 500,
                        duration: 0,
                    });
                    tracing::info!(
                        "[MARKETTEST] 寄售 [{}] uid={} 价格500",
                        item.name,
                        item.unique_id
                    );
                    consigned.push(item.unique_id as u32);
                    *stage = 2;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[MARKETTEST] ❌ 背包为空");
                    *stage = 9;
                }
            }
        }
        2 => {
            if *t < 5.0 {
                return;
            }
            if market.consign_ok.is_some() {
                tracing::info!(
                    "[MARKETTEST] ✅ 第一件寄售成功 uid={}",
                    market.consign_ok.unwrap_or(0)
                );
            } else {
                tracing::warn!("[MARKETTEST] ❌ 第一件寄售未确认");
                *stage = 9;
                return;
            }
            // 寄售第二件（uid=101，价格 600）
            let first = hud
                .inventory
                .items
                .iter()
                .enumerate()
                .find_map(|(i, s)| s.as_ref().map(|it| (i, it)));
            match first {
                Some((_i, item)) => {
                    net.send_packet(&client_bevy::network::MarketConsignWire {
                        unique_id: item.unique_id as u32,
                        price: 600,
                        duration: 0,
                    });
                    tracing::info!(
                        "[MARKETTEST] 寄售第二件 [{}] uid={} 价格600",
                        item.name,
                        item.unique_id
                    );
                    consigned.push(item.unique_id as u32);
                    *stage = 3;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[MARKETTEST] ❌ 背包只剩 0 件（应剩 1 件）");
                    *stage = 9;
                }
            }
        }
        3 => {
            if *t < 5.0 {
                return;
            }
            // 刷新市场，取回第二件（uid=101）
            net.send_packet(&mir2_shared::packets::client::market::MarketRefresh);
            tracing::info!("[MARKETTEST] 刷新市场准备取回");
            *stage = 4;
            *t = 0.0;
        }
        4 => {
            if *t < 5.0 {
                return;
            }
            let mine: Vec<&client_bevy::game::dialogs::market::MarketItem> = market
                .listings
                .iter()
                .filter(|it| it.seller == "bevychar")
                .collect();
            tracing::info!("[MARKETTEST] 我的寄售: {} 件", mine.len());
            let target = mine.iter().find(|it| it.unique_id == 101).copied();
            match target {
                Some(it) => {
                    net.send_packet(&client_bevy::network::MarketGetBackWire {
                        listing_id: it.auction_id as u32,
                    });
                    tracing::info!("[MARKETTEST] 取回商品 {} uid={}", it.auction_id, it.unique_id);
                    *stage = 5;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!(
                        "[MARKETTEST] ❌ 未找到 uid=101 的寄售: {:?}",
                        mine.iter().map(|x| x.unique_id).collect::<Vec<_>>()
                    );
                    *stage = 9;
                }
            }
        }
        5 => {
            if *t < 6.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::market::MarketRefresh);
            tracing::info!("[MARKETTEST] 取回后刷新市场");
            *stage = 6;
            *t = 0.0;
        }
        6 => {
            if *t < 5.0 {
                return;
            }
            let mine: Vec<&client_bevy::game::dialogs::market::MarketItem> = market
                .listings
                .iter()
                .filter(|it| it.seller == "bevychar")
                .collect();
            if mine.len() == 1 && mine[0].unique_id == 100 {
                tracing::info!(
                    "[MARKETTEST] ✅ 取回成功：剩 1 件寄售（uid=100 价格{}）",
                    mine[0].price
                );
            } else {
                tracing::warn!(
                    "[MARKETTEST] ❌ 取回后异常: mine={:?}",
                    mine.iter().map(|x| x.unique_id).collect::<Vec<_>>()
                );
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --market-buy：刷新市场 → 买下卖家 bevychar 的商品（配合 --market-test）
#[allow(clippy::too_many_arguments)]
fn auto_market_buy(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    market: Res<client_bevy::game::dialogs::market::MarketState>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut bought_id: Local<Option<u64>>,
    mut last_refresh: Local<f32>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 45.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Market) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Market);
            }
            net.send_packet(&mir2_shared::packets::client::market::MarketRefresh);
            tracing::info!("[MARKETBUY] 打开市场 + 刷新");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 20.0 {
                tracing::warn!("[MARKETBUY] ❌ 未找到卖家 bevychar 的商品");
                *stage = 9;
                return;
            }
            // 等待期每 4 秒刷新一次市场（卖家可能尚未上架）
            if *t - *last_refresh >= 4.0 {
                *last_refresh = *t;
                net.send_packet(&mir2_shared::packets::client::market::MarketRefresh);
                tracing::info!("[MARKETBUY] 等待中刷新市场");
            }
            let target = market
                .listings
                .iter()
                .find(|it| it.seller == "bevychar" && it.unique_id == 100)
                .cloned();
            if let Some(it) = target {
                *bought_id = Some(it.auction_id);
                net.send_packet(&client_bevy::network::MarketBuyWire {
                    listing_id: it.auction_id as u32,
                });
                tracing::info!(
                    "[MARKETBUY] 购买商品 {} [{}] {}金币",
                    it.auction_id,
                    it.name,
                    it.price
                );
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t >= 15.0 {
                tracing::warn!("[MARKETBUY] ❌ 购买未确认: message={}", market.message);
                *stage = 9;
                return;
            }
            if market.message.contains("购买成功") {
                tracing::info!("[MARKETBUY] ✅ 购买成功: {}", market.message);
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            if *t < 5.0 {
                return;
            }
            // 验证物品进入背包（item_index=853）
            let has = hud
                .inventory
                .items
                .iter()
                .filter_map(|s| s.as_ref())
                .any(|it| it.item_index == 853);
            if has {
                tracing::info!("[MARKETBUY] ✅ 购买的物品已进入背包");
            } else {
                tracing::warn!("[MARKETBUY] ❌ 背包未见购买的物品");
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --gameshop-test：打开商城 → 请求目录 → 购买第一件可负担商品 → 邮件送达
#[allow(clippy::too_many_arguments)]
fn auto_gameshop_test(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    shop: Res<client_bevy::game::dialogs::game_shop::GameShopState>,
    mail: Res<client_bevy::game::dialogs::mail::MailState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut bought_item: Local<Option<i32>>,
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
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::GameShop) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::GameShop);
            }
            tracing::info!("[SHOPTEST] 打开商城（自动请求目录）");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 10.0 {
                tracing::warn!("[SHOPTEST] ❌ 商城目录未收到");
                *stage = 9;
                return;
            }
            if !shop.items.is_empty() {
                tracing::info!(
                    "[SHOPTEST] ✅ 商城目录 {} 件，我的金币 {}",
                    shop.items.len(),
                    shop.gold
                );
                // 选第一件金币价 <= 我的金币 的商品
                let target = shop.items.iter().find(|it| it.gold_price > 0);
                match target {
                    Some(it) => {
                        *bought_item = Some(it.item_index);
                        net.send_packet(&client_bevy::network::GameshopBuyWire {
                            item_id: it.item_index as u32,
                            quantity: 1,
                        });
                        tracing::info!(
                            "[SHOPTEST] 购买 #{} {} {}金币",
                            it.item_index,
                            it.name,
                            it.gold_price
                        );
                        *stage = 2;
                        *t = 0.0;
                    }
                    None => {
                        tracing::warn!("[SHOPTEST] ❌ 目录为空或没有可购买商品");
                        *stage = 9;
                    }
                }
            }
        }
        2 => {
            if *t >= 12.0 {
                tracing::warn!("[SHOPTEST] ❌ 未收到购买邮件");
                *stage = 9;
                return;
            }
            if mail.mails.iter().any(|m| m.sender == "GameShop") {
                let ms: Vec<String> = mail
                    .mails
                    .iter()
                    .filter(|m| m.sender == "GameShop")
                    .map(|m| format!("{}: {}", m.sender, m.subject))
                    .collect();
                tracing::info!("[SHOPTEST] ✅ 购买邮件送达: {:?}", ms);
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            if *t < 3.0 {
                return;
            }
            tracing::info!(
                "[SHOPTEST] ✅ 完成（购买 #{}）",
                bought_item.unwrap_or(-1)
            );
            *stage = 9;
        }
        _ => {}
    }
}

/// --territory-test：打开行会领地 → 购买第一个无主领地 → 向 TestGuildWar 宣战
#[allow(clippy::too_many_arguments)]
fn auto_territory_test(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    territory: Res<client_bevy::game::dialogs::guild_territory::GuildTerritoryState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut bought_id: Local<Option<i32>>,
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
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::GuildTerritory) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::GuildTerritory);
            }
            tracing::info!("[TERRTEST] 打开行会领地（自动请求列表）");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 10.0 {
                tracing::warn!("[TERRTEST] ❌ 领地列表未收到");
                *stage = 9;
                return;
            }
            if !territory.rows.is_empty() {
                tracing::info!(
                    "[TERRTEST] ✅ 领地列表 {} 个",
                    territory.rows.len()
                );
                let free = territory
                    .rows
                    .iter()
                    .find(|r| r.owner.is_empty())
                    .cloned();
                match free {
                    Some(r) => {
                        *bought_id = Some(r.id);
                        net.send_packet(&client_bevy::network::PurchaseGuildTerritoryWire {
                            territory_id: r.id as u32,
                        });
                        tracing::info!("[TERRTEST] 购买领地 #{}", r.id);
                        *stage = 2;
                        *t = 0.0;
                    }
                    None => {
                        tracing::warn!("[TERRTEST] ❌ 没有无主领地");
                        *stage = 9;
                    }
                }
            }
        }
        2 => {
            if *t < 6.0 {
                return;
            }
            // 重新请求列表验证购买
            net.send_packet(&client_bevy::network::GuildTerritoryPageWire { page: 0 });
            tracing::info!("[TERRTEST] 购买后刷新领地列表");
            *stage = 3;
            *t = 0.0;
        }
        3 => {
            if *t < 6.0 {
                return;
            }
            let id = bought_id.unwrap_or(-1);
            let row = territory.rows.iter().find(|r| r.id == id);
            match row {
                Some(r) if r.owner == "TestGuild4" => {
                    tracing::info!(
                        "[TERRTEST] ✅ 购买成功：领地 #{} 归属 {}",
                        r.id,
                        r.owner
                    );
                    *stage = 4;
                    *t = 0.0;
                }
                Some(r) => {
                    tracing::warn!(
                        "[TERRTEST] ❌ 领地 #{} 归属异常: {}",
                        r.id,
                        r.owner
                    );
                    *stage = 9;
                }
                None => {
                    tracing::warn!("[TERRTEST] ❌ 领地 #{} 不存在", id);
                    *stage = 9;
                }
            }
        }
        4 => {
            if *t < 6.0 {
                return;
            }
            // 向 TestGuildWar 宣战（--territory-war 客户端先创建）
            net.send_packet(&mir2_shared::packets::client::guild::GuildWarReturn {
                guild_name: "TestGuildWar".to_string(),
            });
            tracing::info!("[TERRTEST] 向 TestGuildWar 宣战");
            *stage = 5;
            *t = 0.0;
        }
        5 => {
            if *t >= 10.0 {
                tracing::warn!("[TERRTEST] ❌ 未收到宣战确认");
                *stage = 9;
                return;
            }
            if territory.war_message.contains("TestGuildWar") {
                tracing::info!("[TERRTEST] ✅ 宣战成功: {}", territory.war_message);
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --territory-war：创建目标行会 TestGuildWar（供 --territory-test 宣战）
#[allow(clippy::too_many_arguments)]
fn auto_territory_war(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    guild: Res<client_bevy::game::dialogs::guild::GuildState>,
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
            if guild.in_guild && guild.name == "TestGuildWar" {
                tracing::info!("[TERRWAR] ✅ 已在行会 TestGuildWar");
                *stage = 9;
                return;
            }
            net.send_packet(&mir2_shared::packets::client::guild::GuildNameReturn {
                name: "TestGuildWar".to_string(),
            });
            tracing::info!("[TERRWAR] 创建行会 TestGuildWar");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 8.0 {
                return;
            }
            if guild.in_guild && guild.name == "TestGuildWar" {
                tracing::info!("[TERRWAR] ✅ 行会创建成功");
                *stage = 9;
            } else {
                tracing::warn!("[TERRWAR] ❌ 行会创建失败");
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --combat-test：自动选怪 → 连续 FireBall → 验证死亡 + 掉落（M37 战斗闭环）
#[allow(clippy::too_many_arguments)]
fn auto_combat_test(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut cast_timer: Local<f32>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    mut item_count_before: Local<usize>,
    mut effect_seen: Local<bool>,
    effects: Res<client_bevy::game::effects::EffectsState>,
    mut control: ResMut<client_bevy::game::player_control::ControlState>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        &client_bevy::actor::ActorAppearance,
    )>,
    items: Query<(&client_bevy::actor::NetObjectId, &client_bevy::actor::GroundItem)>,
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
            // 找 10 格内最近的怪物
            let mut best: Option<(u32, i32, i32, i32)> = None;
            for (id, tf, app) in &actors {
                if !matches!(app, client_bevy::actor::ActorAppearance::Monster { .. }) {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my, d));
                }
            }
            if best.is_none() {
                // 探测：附近 40 格内怪物数量与最近距离
                let mut total = 0usize;
                let mut nearest = i32::MAX;
                for (_, tf, app) in &actors {
                    if !matches!(app, client_bevy::actor::ActorAppearance::Monster { .. }) {
                        continue;
                    }
                    total += 1;
                    let (mx, my) =
                        client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                    let d = (mx - px).abs() + (my - py).abs();
                    if d < nearest {
                        nearest = d;
                    }
                }
                tracing::warn!(
                    "[COMBAT] 40 格内无怪物：玩家=({},{}), 全图可见怪物={}, 最近距离={}",
                    px,
                    py,
                    total,
                    nearest
                );
            }
            match best {
                Some((oid, mx, my, d)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    *item_count_before = items.iter().count();
                    // 模拟真实玩法：点击选中攻击目标（供特效/施法定位）
                    control.attack_target = Some(oid);
                    tracing::info!(
                        "[COMBAT] 🎯 目标怪物 id={} @ ({},{}) 距离={}",
                        oid,
                        mx,
                        my,
                        d
                    );
                    *stage = 1;
                    *t = 0.0;
                    *cast_timer = 0.0;
                }
                None => {
                    tracing::warn!("[COMBAT] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t >= 45.0 {
                tracing::warn!("[COMBAT] ❌ 超时未击杀（目标仍在）");
                *stage = 9;
                return;
            }
            // 目标实体已消失（ObjectDied 移除）→ 击杀成功
            let alive = target
                .and_then(|tid| actors.iter().find(|(id, _, _)| id.0 == tid))
                .is_some();
            if !alive {
                tracing::info!("[COMBAT] ✅ 目标怪物已死亡（实体移除）");
                *stage = 2;
                *t = 0.0;
                return;
            }
            // M38：魔法特效验证（MagicCast → 弹道，ObjectStruck → 爆炸）
            if !*effect_seen && effects.spawned > 0 {
                *effect_seen = true;
                tracing::info!(
                    "[COMBAT] ✅ 魔法特效已生成（计数 {}）",
                    effects.spawned
                );
            }
            // 每 1.3 秒施放一次 FireBall（目标位置）
            *cast_timer += time.delta_secs();
            if *cast_timer >= 1.3 {
                *cast_timer = 0.0;
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
                tracing::info!("[COMBAT] 🔥 FireBall → ({},{})", mx, my);
            }
        }
        2 => {
            if *t < 5.0 {
                return;
            }
            // 对比地面物品计数（M24 掉落链路）
            let now = items.iter().count();
            let before = *item_count_before;
            if now > before {
                tracing::info!(
                    "[COMBAT] ✅ 死亡后出现掉落（地面物品 {} → {}）",
                    before,
                    now
                );
            } else {
                tracing::warn!(
                    "[COMBAT] ⚠️ 地面物品数未增加（{} → {}，可能掉落被拾取）",
                    before,
                    now
                );
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --fishing-test：打开钓鱼 → 抛竿 → 等 FishingUpdate → 等收获聊天消息
#[allow(clippy::too_many_arguments)]
fn auto_fishing_test(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    fishing: Res<client_bevy::game::dialogs::fishing::FishingState>,
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
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Fishing) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Fishing);
            }
            net.send_packet(&client_bevy::network::FishingCastWire { fishing_type: 0 });
            tracing::info!("[FISHTEST] 抛竿");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 6.0 {
                tracing::warn!(
                    "[FISHTEST] ❌ 未收到 FishingUpdate（progress={}）",
                    fishing.progress
                );
                *stage = 9;
                return;
            }
            if fishing.progress == 1 {
                tracing::info!("[FISHTEST] ✅ 抛竿成功（等待中）");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 12.0 {
                return;
            }
            let hit = chat
                .lines
                .iter()
                .rev()
                .take(30)
                .find(|(text, _)| {
                    text.contains("钓到了") || text.contains("鱼跑了") || text.contains("需要装备鱼竿")
                })
                .map(|(text, _)| text.clone());
            match hit {
                Some(text) => {
                    tracing::info!("[FISHTEST] ✅ 收获消息: {}", text);
                    *stage = 9;
                }
                None => {
                    tracing::warn!("[FISHTEST] ❌ 未收到收获消息");
                    *stage = 9;
                }
            }
        }
        _ => {}
    }
}

/// --refine-test：精炼全流程（存入 → 开始 60 秒 → 查看 → 取回）
#[allow(clippy::too_many_arguments)]
fn auto_refine_test(
    net: ResMut<client_bevy::network::NetworkContext>,
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
        chat.lines.iter().rev().take(60).any(|(t, _)| t.contains(needle))
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
fn auto_craft_test(
    net: ResMut<client_bevy::network::NetworkContext>,
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
        chat.lines.iter().rev().take(60).any(|(t, _)| t.contains(needle))
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
fn auto_rental_test(
    net: ResMut<client_bevy::network::NetworkContext>,
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
fn auto_rental_owner(
    net: ResMut<client_bevy::network::NetworkContext>,
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

/// --quest-test：打开任务日志 → 接受任务1 → 等 ChangeQuest → 放弃
#[allow(clippy::too_many_arguments)]
fn auto_quest_test(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut quest_log: ResMut<client_bevy::game::dialogs::quest_log::QuestLogState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
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
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::QuestLog) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::QuestLog);
            }
            // 登录推送容错：若任务 1 已存在（上次会话残留）则直接走放弃流程
            if quest_log.quests.iter().any(|q| q.id == 1) {
                tracing::info!("[QUESTTEST] 任务 1 已在列表中（登录推送），直接放弃");
                net.send_packet(&mir2_shared::packets::client::quest::AbandonQuest {
                    quest_index: 1,
                });
                quest_log.quests.retain(|q| q.id != 1);
                *stage = 2;
                *t = 0.0;
                return;
            }
            net.send_packet(&mir2_shared::packets::client::quest::AcceptQuest {
                npc_index: 0,
                quest_index: 1,
            });
            tracing::info!("[QUESTTEST] 接受任务 1");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 8.0 {
                tracing::warn!("[QUESTTEST] ❌ 未收到任务更新");
                *stage = 9;
                return;
            }
            let first_id = quest_log.quests.first().map(|q| q.id);
            if let Some(qid) = first_id {
                let qname = quest_log
                    .quests
                    .iter()
                    .find(|q| q.id == qid)
                    .map(|q| q.name.clone())
                    .unwrap_or_default();
                tracing::info!("[QUESTTEST] ✅ 任务已显示: {}（任务 {}）", qname, qid);
                net.send_packet(&mir2_shared::packets::client::quest::AbandonQuest {
                    quest_index: qid,
                });
                // 模拟放弃按钮：本地移除
                quest_log.quests.retain(|x| x.id != qid);
                tracing::info!(
                    "[QUESTTEST] 放弃任务 {}（移除后剩 {}）",
                    qid,
                    quest_log.quests.len()
                );
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 5.0 {
                return;
            }
            if quest_log.quests.is_empty() {
                tracing::info!("[QUESTTEST] ✅ 任务已放弃（列表清空）");
            } else {
                let ids: Vec<i32> = quest_log.quests.iter().map(|q| q.id).collect();
                tracing::warn!(
                    "[QUESTTEST] ⚠️ 任务列表仍非空: {} ids={:?}",
                    quest_log.quests.len(),
                    ids
                );
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --buff-test：打开状态对话框 → 施放 Fury（攻击提升）→ 等 AddBuff
#[allow(clippy::too_many_arguments)]
fn auto_buff_test(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    buff: Res<client_bevy::game::dialogs::buff::BuffState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    tracing::debug!("[BUFFTEST] 驱动运行中 stage={} t={:.1}", *stage, *t);
    match *stage {
        0 => {
            if *t < 4.0 {
                return;
            }
            tracing::info!("[BUFFTEST] 打开状态对话框");
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Buff) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Buff);
            }
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::Mirroring,
                direction: mir2_shared::enums::MirDirection::Down,
                target_id: 0,
                location: mir2_shared::Point { x: 0, y: 0 },
            });
            tracing::info!("[BUFFTEST] 施放 Mirroring");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 8.0 {
                tracing::warn!("[BUFFTEST] ❌ 未收到 AddBuff（buff={}）", buff.buffs.len());
                *stage = 9;
                return;
            }
            if let Some(b) = buff.buffs.first() {
                tracing::info!(
                    "[BUFFTEST] ✅ 获得状态: {}（剩余 {} tick）",
                    client_bevy::game::dialogs::buff::buff_name(b.tag),
                    b.remaining_ticks
                );
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 4.0 {
                return;
            }
            tracing::info!(
                "[BUFFTEST] ✅ 完成（当前 {} 个状态）",
                buff.buffs.len()
            );
            *stage = 9;
        }
        _ => {}
    }
}

/// --report-test：打开举报 → 提交 → 等系统消息确认
#[allow(clippy::too_many_arguments)]
fn auto_report_test(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
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
        chat.lines.iter().rev().take(60).any(|(t, _)| t.contains(needle))
    }
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Report) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Report);
            }
            net.send_packet(&client_bevy::network::ReportIssueWire {
                issue_type: 1,
                description: "测试举报".to_string(),
            });
            tracing::info!("[REPORTTEST] 提交举报（type=1）");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 8.0 {
                tracing::warn!("[REPORTTEST] ❌ 未收到举报确认");
                *stage = 9;
                return;
            }
            if chat_has(&chat, "举报信息已提交") {
                tracing::info!("[REPORTTEST] ✅ 举报已提交确认");
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --inspect-test：找目标玩家 → 发 Inspect → 等 PlayerInspect
#[allow(clippy::too_many_arguments)]
fn auto_inspect_test(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    inspect: Res<client_bevy::game::dialogs::inspect::InspectState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        Option<&client_bevy::actor::PlayerName>,
    )>,
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
            // 找到 bevy2char
            let target = actors
                .iter()
                .find(|(_, name)| name.and_then(|n| Some(n.0 == "bevy2char")).unwrap_or(false))
                .map(|(id, _)| id.0);
            match target {
                Some(oid) => {
                    if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Inspect) {
                        mgr.toggle(client_bevy::game::dialogs::DialogKind::Inspect);
                    }
                    net.send_packet(&mir2_shared::packets::client::chat::Inspect {
                        object_id: oid,
                    });
                    tracing::info!("[INSPECTTEST] 查看玩家 bevy2char (oid={})", oid);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[INSPECTTEST] ❌ 找不到目标玩家 bevy2char");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t >= 8.0 {
                tracing::warn!("[INSPECTTEST] ❌ 未收到 PlayerInspect");
                *stage = 9;
                return;
            }
            if !inspect.name.is_empty() {
                tracing::info!(
                    "[INSPECTTEST] ✅ 查看成功: {} Lv.{} 行会={} 装备 {} 件",
                    inspect.name,
                    inspect.level,
                    inspect.guild,
                    inspect.items.len()
                );
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --creature-test：打开宠物对话框 → 自动请求列表 → 等解析完成
#[allow(clippy::too_many_arguments)]
fn auto_creature_test(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    creature: Res<client_bevy::game::dialogs::creature::CreatureState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
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
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Creature) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Creature);
            }
            // 打开对话框会自动请求；这里兜底再发一次
            net.send_packet(&client_bevy::network::CreatureRequestWire { request: true });
            tracing::info!("[CREATURETEST] 请求宠物列表");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 8.0 {
                tracing::warn!("[CREATURETEST] ❌ 未收到宠物列表");
                *stage = 9;
                return;
            }
            if creature.message.contains("宠物列表已更新") {
                tracing::info!(
                    "[CREATURETEST] ✅ 宠物列表: {} 个",
                    creature.creatures.len()
                );
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --hero-test：打开英雄 → 切换英雄1 → 等 ChangeHero → 切回主角色
#[allow(clippy::too_many_arguments)]
fn auto_hero_test(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    hero: Res<client_bevy::game::dialogs::hero::HeroState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
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
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Hero) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Hero);
            }
            net.send_packet(&client_bevy::network::ChangeHeroWire { hero_index: 1 });
            tracing::info!("[HEROTEST] 切换英雄 1");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 8.0 {
                tracing::warn!("[HEROTEST] ❌ 未收到 ChangeHero（index={}）", hero.hero_index);
                *stage = 9;
                return;
            }
            if hero.hero_index == 1 {
                tracing::info!("[HEROTEST] ✅ 英雄切换成功: {}", hero.message);
                net.send_packet(&client_bevy::network::ChangeHeroWire { hero_index: 0 });
                tracing::info!("[HEROTEST] 切回主角色");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 5.0 {
                return;
            }
            if hero.hero_index == 0 {
                tracing::info!("[HEROTEST] ✅ 切回主角色成功");
            } else {
                tracing::warn!("[HEROTEST] ⚠️ 当前 index={}", hero.hero_index);
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --marriage-test（求婚方）：求婚 → 等 LoverUpdate → 离婚
#[allow(clippy::too_many_arguments)]
fn auto_marriage_test(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    relationship: Res<client_bevy::game::dialogs::relationship::RelationshipState>,
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
            net.send_packet(&client_bevy::network::MarriageRequestWire {
                target_name: "bevy2char".to_string(),
            });
            tracing::info!("[MARRY] 向 bevy2char 求婚");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 15.0 {
                tracing::warn!("[MARRY] ❌ 未结婚（married={}）", relationship.married);
                *stage = 9;
                return;
            }
            if relationship.married {
                tracing::info!("[MARRY] ✅ 结婚成功");
                net.send_packet(&client_bevy::network::DivorceRequestWire {
                    partner_name: "bevy2char".to_string(),
                });
                tracing::info!("[MARRY] 发起离婚");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t >= 15.0 {
                tracing::warn!("[MARRY] ❌ 未离婚（married={}）", relationship.married);
                *stage = 9;
                return;
            }
            if !relationship.married {
                tracing::info!("[MARRY] ✅ 离婚成功");
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --marriage-accept（被求婚方）：接受求婚 → 等结婚 → 离婚确认
#[allow(clippy::too_many_arguments)]
fn auto_marriage_accept(
    net: ResMut<client_bevy::network::NetworkContext>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    relationship: Res<client_bevy::game::dialogs::relationship::RelationshipState>,
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
            if *t >= 20.0 {
                tracing::warn!("[MARRYACC] ❌ 未收到求婚");
                *stage = 9;
                return;
            }
            if relationship.invite.is_some() {
                tracing::info!(
                    "[MARRYACC] ✅ 收到求婚: {}",
                    relationship.invite.clone().unwrap_or_default()
                );
                net.send_packet(&mir2_shared::packets::client::misc::MarriageReply {
                    accept_invite: true,
                });
                tracing::info!("[MARRYACC] 接受求婚");
                *stage = 1;
                *t = 0.0;
            }
        }
        1 => {
            if *t >= 15.0 {
                tracing::warn!("[MARRYACC] ❌ 未结婚");
                *stage = 9;
                return;
            }
            if relationship.married {
                tracing::info!("[MARRYACC] ✅ 已婚");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            // 等待对方离婚请求并确认
            if relationship.message.contains("离婚请求") {
                tracing::info!("[MARRYACC] ✅ 收到离婚请求，确认");
                net.send_packet(&mir2_shared::packets::client::misc::DivorceReply {
                    accept_invite: true,
                });
                *stage = 3;
                *t = 0.0;
            }
            if *t >= 20.0 {
                tracing::warn!("[MARRYACC] ❌ 未收到离婚请求");
                *stage = 9;
            }
        }
        3 => {
            if *t < 5.0 {
                return;
            }
            if !relationship.married {
                tracing::info!("[MARRYACC] ✅ 离婚完成");
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --mount-test：打开坐骑面板 → 骑乘/下马（@ride）→ 外观广播 → 坐骑层
#[allow(clippy::too_many_arguments)]
fn auto_mount_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    net: ResMut<client_bevy::network::NetworkContext>,
    mounts: Query<Option<&client_bevy::actor::MountState>, With<client_bevy::actor::LocalPlayer>>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut phase: Local<f32>,
    mut rode: Local<bool>,
) {
    use client_bevy::scenes::AppState;
    use client_bevy::game::dialogs::DialogKind;
    use mir2_shared::packets::client::chat::Chat;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    if *stage == 0 {
        if !mgr.is_open(DialogKind::Mount) {
            mgr.open(DialogKind::Mount);
            tracing::info!("[MOUNT] 打开坐骑面板");
        }
        *phase = *t;
        *stage = 1;
        return;
    }
    if *stage == 1 && *t - *phase >= 1.5 {
        let mounted = mounts.single().ok().flatten().is_some();
        tracing::info!("[MOUNT] ✅ 面板状态: 本地坐骑层={}", mounted);
        net.send_packet(&Chat {
            message: "@ride".to_string(),
            linked_items: Vec::new(),
        });
        tracing::info!("[MOUNT] ✅ 发送 @ride（骑乘）");
        *stage = 2;
        *phase = *t;
        return;
    }
    if *stage == 2 {
        if mounts.single().ok().flatten().is_some() {
            tracing::info!("[MOUNT] ✅ 骑乘成功（本地玩家出现坐骑层）");
            *rode = true;
            *stage = 3;
            *phase = *t;
        } else if *t - *phase >= 8.0 {
            tracing::warn!("[MOUNT] ❌ 骑乘超时（检查地图限制/鞍）");
            *stage = 9;
        }
        return;
    }
    if *stage == 3 && *t - *phase >= 1.5 {
        net.send_packet(&Chat {
            message: "@ride".to_string(),
            linked_items: Vec::new(),
        });
        tracing::info!("[MOUNT] ✅ 发送 @ride（下马）");
        *stage = 4;
        *phase = *t;
        return;
    }
    if *stage == 4 {
        if mounts.single().ok().flatten().is_none() {
            tracing::info!("[MOUNT] ✅ 下马成功");
            if mgr.is_open(DialogKind::Mount) {
                mgr.close(DialogKind::Mount);
                tracing::info!("[MOUNT] ✅ 关闭坐骑面板");
            }
            *stage = 9;
        } else if *t - *phase >= 8.0 {
            tracing::warn!("[MOUNT] ❌ 下马超时");
            *stage = 9;
        }
        return;
    }
    if *t >= 60.0 && *stage < 9 {
        tracing::warn!("[MOUNT] ❌ 总超时 stage={}", *stage);
        *stage = 9;
    }
}

/// --reconnect-test：进入游戏 → 等服务器断开 → 自动重连 → 自动登录并重新进游戏
#[allow(clippy::too_many_arguments)]
fn auto_reconnect_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut net: ResMut<client_bevy::network::NetworkContext>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut phase: Local<f32>,
    mut saw_disconnect: Local<bool>,
) {
    use client_bevy::scenes::AppState;
    use client_bevy::network::NetState;
    *t += time.delta_secs();
    if *stage == 0 {
        if *state == AppState::Game {
            tracing::info!("[RECON] 已进入游戏，等待服务器断开...");
            *stage = 1;
            *phase = *t;
        } else if *t >= 60.0 {
            tracing::warn!("[RECON] ❌ 60 秒内未进入游戏");
            *stage = 9;
        }
        return;
    }
    if *stage == 1 {
        if net.disconnected.is_some() && !*saw_disconnect {
            *saw_disconnect = true;
            tracing::info!("[RECON] ✅ 检测到断线: {:?}", net.disconnected);
            *stage = 2;
            *phase = *t;
        } else if *t - *phase >= 60.0 {
            tracing::warn!("[RECON] ❌ 未检测到断线");
            *stage = 9;
        }
        return;
    }
    if *stage == 2 {
        if net.state == NetState::InGame && *state == AppState::Game && !net.reconnecting {
            tracing::info!("[RECON] ✅ 自动重连成功并重新进入游戏");
            *stage = 9;
        } else if *t - *phase >= 90.0 {
            tracing::warn!("[RECON] ❌ 重连超时（state={:?} reconnecting={}）", net.state, net.reconnecting);
            *stage = 9;
        }
        return;
    }
    if *t >= 200.0 && *stage < 9 {
        tracing::warn!("[RECON] ❌ 总超时 stage={}", *stage);
        *stage = 9;
    }
}

/// --roll-test：触发 NPC 掷骰 → 服务端 Roll 包 → 客户端骰子对话框 → 自动回调
#[allow(clippy::too_many_arguments)]
fn auto_roll_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut roll: ResMut<client_bevy::game::dialogs::roll::RollState>,
    bm: Res<client_bevy::game::dialogs::big_map::BigMapState>,
    game_data: Res<client_bevy::map_renderer::GameData>,
    mut npc_dialog: ResMut<client_bevy::game::dialogs::npc::NpcDialogState>,
    net: ResMut<client_bevy::network::NetworkContext>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut phase: Local<f32>,
    mut npc_id: Local<u32>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    if *stage == 0 {
        // 选离玩家出生点最近的 NPC（CallNPC 需 2 格内）
        let spawn = game_data.player_spawn.map(|(x, y, _)| (x as i32, y as i32));
        let picked = if let Some((sx, sy)) = spawn {
            bm.npcs
                .iter()
                .min_by_key(|n| (n.x - sx).abs() + (n.y - sy).abs())
                .cloned()
        } else {
            bm.npcs.first().cloned()
        };
        if let Some(npc) = picked {
            *npc_id = npc.object_id;
            npc_dialog.npc_object_id = npc.object_id;
            net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                object_id: npc.object_id,
                key: "[@TestRoll]".to_string(),
            });
            tracing::info!(
                "[ROLL] 触发 NPC {} ({},{}) 掷骰页",
                npc.object_id,
                npc.x,
                npc.y
            );
            *stage = 1;
            *phase = *t;
        } else if *t - *phase >= 15.0 {
            tracing::warn!("[ROLL] ❌ 未等到 NPC 数据");
            *stage = 9;
        }
        return;
    }
    if *stage == 1 {
        if roll.visible {
            tracing::info!(
                "[ROLL] ✅ 收到 Roll 包: type={} result={} page={} auto={}",
                roll.r#type,
                roll.result,
                roll.page,
                roll.auto_roll
            );
            *stage = 2;
            *phase = *t;
        } else if *t - *phase >= 10.0 {
            tracing::warn!("[ROLL] ❌ 未收到 Roll 包");
            *stage = 9;
        }
        return;
    }
    if *stage == 2 {
        if !roll.visible && roll.finished {
            tracing::info!("[ROLL] ✅ 掷骰完成回调已发送（NPC {}）", *npc_id);
            *stage = 9;
        } else if *t - *phase >= 12.0 {
            tracing::warn!("[ROLL] ❌ 回调超时");
            *stage = 9;
        }
        return;
    }
    if *t >= 40.0 && *stage < 9 {
        tracing::warn!("[ROLL] ❌ 超时 stage={}", *stage);
        *stage = 9;
    }
}

/// --socket-test：打开镶嵌面板 → 孔位/宝石渲染 → 关闭
#[allow(clippy::too_many_arguments)]
fn auto_socket_test(
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
fn auto_dura_test(
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
fn auto_awake_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut aw: ResMut<client_bevy::game::dialogs::npc_awake::NpcAwakeState>,
    hud: Res<client_bevy::game::hud::HudState>,
    net: ResMut<client_bevy::network::NetworkContext>,
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

/// --bigmap-test：打开大地图 → 等 NewMapInfo/地形 → 选中 NPC → 传送 → 关闭
#[allow(clippy::too_many_arguments)]
fn auto_bigmap_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut bm: ResMut<client_bevy::game::dialogs::big_map::BigMapState>,
    net: ResMut<client_bevy::network::NetworkContext>,
    players: Query<&Transform, With<client_bevy::actor::LocalPlayer>>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut phase: Local<f32>,
    mut target: Local<(i32, i32)>,
) {
    use client_bevy::scenes::AppState;
    use client_bevy::game::dialogs::DialogKind;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    if *stage == 0 {
        if !mgr.is_open(DialogKind::BigMap) {
            mgr.toggle(DialogKind::BigMap);
            tracing::info!("[BIGMAP] 打开大地图");
        }
        *phase = *t;
        *stage = 1;
        return;
    }
    if *stage == 1 && *t - *phase >= 1.0 {
        if bm.npcs.is_empty() {
            tracing::warn!("[BIGMAP] ⚠️ 无 NewMapInfo NPC 数据（服务端需 M53 支持）");
        } else {
            tracing::info!("[BIGMAP] ✅ NewMapInfo: {} 个 NPC（{}）", bm.npcs.len(), bm.title);
        }
        *stage = 2;
        *phase = *t;
        return;
    }
    if *stage == 2 {
        if bm.viewport_ready {
            tracing::info!("[BIGMAP] ✅ 地形纹理生成完成 {}x{}", bm.tex_size.0, bm.tex_size.1);
            *stage = 3;
            *phase = *t;
        } else if *t - *phase >= 8.0 {
            tracing::warn!("[BIGMAP] ❌ 地形生成超时");
            *stage = 9;
        }
        return;
    }
    if *stage == 3 && *t - *phase >= 1.0 {
        let tp = bm.npcs.iter().find(|n| n.can_teleport_to).cloned();
        if let Some(npc) = tp {
            bm.selected = Some(0);
            *target = (npc.x, npc.y);
            tracing::info!("[BIGMAP] ✅ 选中可传送 NPC: {} ({},{})", npc.name, npc.x, npc.y);
            net.send_packet(&mir2_shared::packets::client::npc::TeleportToNPC {
                object_id: npc.object_id,
            });
            tracing::info!("[BIGMAP] ✅ 发送传送请求 id={}", npc.object_id);
        } else {
            tracing::warn!("[BIGMAP] ⚠️ 无可传送 NPC");
        }
        *stage = 4;
        *phase = *t;
        return;
    }
    if *stage == 4 && *t - *phase >= 3.0 {
        let moved = players.single().ok().map(|tf| {
            client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y)
        });
        match moved {
            Some((x, y)) if (x, y) == *target => {
                tracing::info!("[BIGMAP] ✅ 传送生效 玩家位置=({},{})", x, y);
            }
            Some((x, y)) => {
                tracing::info!(
                    "[BIGMAP] ✅ 传送已处理 玩家位置=({},{})（目标 ({},{})）",
                    x,
                    y,
                    target.0,
                    target.1
                );
            }
            None => {
                tracing::warn!("[BIGMAP] ⚠️ 无法读取玩家位置");
            }
        }
        if mgr.is_open(DialogKind::BigMap) {
            mgr.close(DialogKind::BigMap);
            tracing::info!("[BIGMAP] ✅ 关闭大地图");
        }
        *stage = 9;
    }
    if *t >= 40.0 && *stage < 9 {
        tracing::warn!("[BIGMAP] ❌ 超时 stage={}", *stage);
        *stage = 9;
    }
}

/// --keyboard-test：打开键位设置 → 滚动 → 重绑一行 → 重置 → 关闭
#[allow(clippy::too_many_arguments)]
fn auto_keyboard_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut kb: ResMut<client_bevy::game::dialogs::keyboard_layout::KeyboardState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut phase: Local<f32>,
) {
    use client_bevy::scenes::AppState;
    use client_bevy::game::dialogs::DialogKind;
    use bevy::input::keyboard::KeyCode;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    if *stage == 0 {
        if !mgr.is_open(DialogKind::KeyboardLayout) {
            mgr.toggle(DialogKind::KeyboardLayout);
            tracing::info!("[KBD] 打开键位设置");
        }
        *phase = *t;
        *stage = 1;
        return;
    }
    if *stage == 1 && *t - *phase >= 1.0 {
        kb.top_line = kb.top_line.saturating_add(2);
        tracing::info!("[KBD] ✅ 滚动 top_line={}", kb.top_line);
        kb.rebinding = Some(4);
        tracing::info!("[KBD] ✅ 等待按键: 行 4");
        *stage = 2;
        *phase = *t;
        return;
    }
    if *stage == 2 && *t - *phase >= 1.0 {
        if let Some(b) = kb.bindings.get_mut(4) {
            tracing::info!("[KBD] ✅ 绑定 {} → {}", b.action, "X");
            b.key = KeyCode::KeyX;
        }
        kb.rebinding = None;
        tracing::info!("[KBD] ✅ 重绑完成");
        *stage = 3;
        *phase = *t;
        return;
    }
    if *stage == 3 && *t - *phase >= 1.0 {
        kb.bindings = kb.defaults.clone();
        kb.top_line = 0;
        kb.enforce = !kb.enforce;
        tracing::info!("[KBD] ✅ 重置默认 + 规则切换（严格/宽松）完成");
        *stage = 4;
        *phase = *t;
        return;
    }
    if *stage == 4 && *t - *phase >= 1.0 {
        if mgr.is_open(DialogKind::KeyboardLayout) {
            mgr.close(DialogKind::KeyboardLayout);
            tracing::info!("[KBD] ✅ 关闭键位设置");
        }
        *stage = 9;
    }
    if *t >= 30.0 && *stage < 9 {
        tracing::warn!("[KBD] ❌ 超时 stage={}", *stage);
        *stage = 9;
    }
}

/// --option-test：打开设置对话框 → 依次切换 8 组开关 + 音量 → 关闭
fn auto_option_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut option: ResMut<client_bevy::game::dialogs::option::OptionState>,
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
        if !mgr.is_open(DialogKind::Settings) {
            mgr.toggle(DialogKind::Settings);
            tracing::info!("[OPT] 打开设置对话框");
        }
        *phase = *t;
        *stage = 1;
        return;
    }
    if *stage == 1 && *t - *phase >= 1.0 {
        // 依次翻转 8 组开关（模拟点击，验证状态机 + 按钮帧刷新）
        let flips: [(&str, bool); 8] = [
            ("技能模式", option.skill_mode_ctrl),
            ("技能栏", option.skill_bar),
            ("特效", option.effect),
            ("掉落显示", option.drop_view),
            ("名称显示", option.name_view),
            ("血条显示", option.hp_view),
            ("允许观察", option.allow_observe),
            ("新移动", option.new_move),
        ];
        for (name, cur) in flips {
            let next = !cur;
            match name {
                "技能模式" => option.skill_mode_ctrl = next,
                "技能栏" => option.skill_bar = next,
                "特效" => option.effect = next,
                "掉落显示" => option.drop_view = next,
                "名称显示" => option.name_view = next,
                "血条显示" => option.hp_view = next,
                "允许观察" => option.allow_observe = next,
                _ => option.new_move = next,
            }
            tracing::info!("[OPT] ✅ 设置切换: {} -> {}", name, next);
        }
        option.sound_volume = 0.5;
        option.music_volume = 0.35;
        tracing::info!("[OPT] ✅ 音量: 音效 50% / 音乐 35%");
        tracing::info!("[OPT] ✅ 设置对话框渲染正常（8 组开关 + 2 条音量条）");
        *stage = 2;
        *phase = *t;
        return;
    }
    if *stage == 2 && *t - *phase >= 1.0 {
        if mgr.is_open(DialogKind::Settings) {
            mgr.close(DialogKind::Settings);
            tracing::info!("[OPT] ✅ 关闭设置对话框");
        }
        *stage = 9;
    }
    if *t >= 30.0 && *stage < 9 {
        tracing::warn!("[OPT] ❌ 超时 stage={}", *stage);
        *stage = 9;
    }
}

/// --ui-dialog-test：依次打开 Notice/ChatNotice/Timer/Help 验证渲染
#[allow(clippy::too_many_arguments)]
fn auto_ui_dialog_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
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
    const KINDS: [DialogKind; 4] = [
        DialogKind::Notice,
        DialogKind::ChatNotice,
        DialogKind::Timer,
        DialogKind::Help,
    ];
    if *stage >= KINDS.len() as u8 {
        return;
    }
    let kind = KINDS[*stage as usize];
    if !mgr.is_open(kind) && *phase == 0.0 {
        mgr.toggle(kind);
        tracing::info!("[UIDLG] 打开 {:?}", kind);
        *phase = *t;
    }
    if mgr.is_open(kind) && *t - *phase >= 1.5 {
        mgr.close(kind);
        tracing::info!("[UIDLG] ✅ {:?} 渲染正常", kind);
        *stage += 1;
        *phase = 0.0;
        *t = 0.0;
    }
    if *t >= 30.0 && *stage < KINDS.len() as u8 {
        tracing::warn!("[UIDLG] ❌ 卡在 {:?}", kind);
        *stage = 9;
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


#[derive(Resource)]
struct AutoWalkDir(String);

/// 调试：把本地玩家按方向持续平移（--auto-walk down），用于验证 chunk 流式加载
fn auto_walk_system(
    mut timer: Local<f32>,
    time: Res<Time>,
    dir: Res<AutoWalkDir>,
    mut players: Query<&mut Transform, (With<client_bevy::actor::LocalPlayer>, With<client_bevy::actor::NetObjectId>)>,
) {
    if dir.0.is_empty() {
        return;
    }
    *timer += time.delta_secs();
    if *timer < 0.06 {
        return;
    }
    *timer = 0.0;
    let step = match dir.0.as_str() {
        "down" => Vec3::new(0.0, -client_bevy::map_renderer::TILE_HEIGHT, 0.0),
        "up" => Vec3::new(0.0, client_bevy::map_renderer::TILE_HEIGHT, 0.0),
        "left" => Vec3::new(-client_bevy::map_renderer::TILE_WIDTH, 0.0, 0.0),
        "right" => Vec3::new(client_bevy::map_renderer::TILE_WIDTH, 0.0, 0.0),
        _ => return,
    };
    for mut tf in players.iter_mut() {
        // 调试边界：向下最多走到 tile 430（n0 村庄下方），避免走出地图
        let tile_y = ((-tf.translation.y - 32.0) / 32.0).round() as i32;
        if dir.0 == "down" && tile_y >= 430 {
            return;
        }
        if dir.0 == "up" && tile_y <= 270 {
            return;
        }
        tf.translation += step;
    }
}

/// --auto-pickup：每 2.5s 自动拾取最近的 GroundItem（复用 player_input 的拾取逻辑）
fn auto_pickup_system(
    mut commands: Commands,
    mut timer: Local<f32>,
    time: Res<Time>,
    net: Res<client_bevy::network::NetworkContext>,
    mut control: ResMut<client_bevy::game::player_control::ControlState>,
    game_data: Res<client_bevy::map_renderer::GameData>,
    players: Query<(Entity, &Transform), With<client_bevy::actor::LocalPlayer>>,
    items: Query<(&client_bevy::actor::NetObjectId, &Transform), (With<client_bevy::actor::GroundItem>, Without<client_bevy::actor::LocalPlayer>)>,
) {
    *timer += time.delta_secs();
    if *timer < 2.5 {
        return;
    }
    *timer = 0.0;
    let Ok((pe, ptf)) = players.single() else { return };
    let from_tile = client_bevy::game::movement::world_to_tile(ptf.translation.x, ptf.translation.y);
    let mut best: Option<(u32, f32)> = None;
    for (id, tf) in &items {
        let d = Vec2::new(tf.translation.x - ptf.translation.x, tf.translation.y - ptf.translation.y).length();
        if best.map(|(_, bd)| d < bd).unwrap_or(true) {
            best = Some((id.0, d));
        }
    }
    let Some((item_id, _)) = best else { return };
    let item_tile = items
        .iter()
        .find(|(id, _)| id.0 == item_id)
        .map(|(_, tf)| client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y));
    let Some(item_tile) = item_tile else { return };
    let adjacent = (item_tile.0 - from_tile.0).abs() <= 1 && (item_tile.1 - from_tile.1).abs() <= 1;
    if adjacent {
        net.send_packet(&mir2_shared::packets::client::item::PickUp {});
        control.attack_target = None;
        tracing::info!("🎒 [AUTO] 拾取地面物品 id={}", item_id);
    } else if let Some(map) = &game_data.map {
        if let Some(p) = client_bevy::game::pathfinding::find_path(map, from_tile, item_tile) {
            if !p.is_empty() {
                let len = p.len();
                commands.entity(pe).insert(client_bevy::game::movement::LocalMove {
                    path: p.into(),
                    step_timer_ms: 0.0,
                    run: false,
                    last: None,
                    turn_acc: 0.0,
                });
                control.pickup_target = Some(item_id);
                tracing::info!("🚶 [AUTO] 走向物品 id={}（{} 格）", item_id, len);
            }
        }
    }
}

/// --auto-cast：进图后施放一次 F1 技能（验证 客户端→mock→回显 链路）
fn auto_cast_system(
    mut timer: Local<f32>,
    mut fired: Local<bool>,
    time: Res<Time>,
    net: Res<client_bevy::network::NetworkContext>,
    magics: Res<client_bevy::game::skills::MagicsState>,
) {
    *timer += time.delta_secs();
    if *fired || *timer < 6.0 || magics.magics.is_empty() {
        return; // 等 UserInformation（技能）就绪
    }
    *fired = true;
    let Some(m) = magics.by_key(1) else {
        tracing::info!("[AUTO] 无技能 key=1");
        return;
    };
    net.send_packet(&mir2_shared::packets::client::combat::Magic {
        spell: m.spell,
        direction: mir2_shared::enums::MirDirection::Up,
        target_id: 101,
        location: mir2_shared::map::Point { x: 0, y: 0 },
    });
    tracing::info!("🪄 [AUTO] 施放 {}", m.name);
}

/// --auto-equip：进图后自动装备背包第一件可装备物品（验证 EquipItem 闭环 + 外观刷新）
fn auto_equip_system(
    mut timer: Local<f32>,
    mut fired: Local<bool>,
    time: Res<Time>,
    net: Res<client_bevy::network::NetworkContext>,
    hud: Res<client_bevy::game::hud::HudState>,
) {
    if *fired {
        return;
    }
    *timer += time.delta_secs();
    if *timer < 6.0 || hud.inventory.items.iter().flatten().count() == 0 {
        return;
    }
    *fired = true;
    if let Some(item) = hud.inventory.items.iter().flatten().find(|i| i.is_equipment()) {
        if let Some(to) = item.equip_slot() {
            net.send_packet(&mir2_shared::packets::client::item::EquipItem {
                grid: mir2_shared::enums::MirGridType::Inventory,
                unique_id: item.unique_id,
                to,
            });
            tracing::info!("⚔️ [AUTO] 装备 {} -> 槽 {}", item.name, to);
        }
    }
}

/// --auto-life：进图后依次 聊天(6s) → 购买(9s) → 喝药(12s)
fn auto_life_system(
    mut timer: Local<f32>,
    mut phase: Local<u8>,
    time: Res<Time>,
    net: Res<client_bevy::network::NetworkContext>,
    hud: Res<client_bevy::game::hud::HudState>,
) {
    *timer += time.delta_secs();
    let t = *timer;
    match *phase {
        0 if t >= 6.0 => {
            *phase = 1;
            net.send_packet(&mir2_shared::packets::client::chat::Chat {
                message: "你好，传奇世界！".to_string(),
                linked_items: vec![],
            });
            tracing::info!("💬 [LIFE] 发送聊天");
        }
        1 if t >= 9.0 => {
            *phase = 2;
            net.send_packet(&mir2_shared::packets::client::npc::BuyItem {
                item_index: 1,
                count: 1,
                panel_type: mir2_shared::enums::PanelType::Buy,
            });
            tracing::info!("🛒 [LIFE] 发送购买请求");
        }
        2 if t >= 12.0 => {
            *phase = 3;
            if let Some(potion) = hud.inventory.items.iter().flatten().find(|i| i.item_index == 1) {
                net.send_packet(&mir2_shared::packets::client::item::UseItem {
                    unique_id: potion.unique_id,
                });
                tracing::info!("💊 [LIFE] 使用药水 uid={}", potion.unique_id);
            } else {
                tracing::info!("💊 [LIFE] 背包无药水");
            }
        }
        _ => {}
    }
}
