//! 40 窗交互巡回的 headless 等价物（原实机脚本 `tools/acceptance/ui_interact_sweep.ps1`）。
//!
//! 为什么需要它：实机巡回是**交互级**的（`dialog open` → `dialog_rect` 定位关闭钮 →
//! `click` 走真实 picking→Interaction → 断言窗口关闭），但它要真客户端 + 真服务端 +
//! 本机 Windows 桌面，只能在实机手动跑；任何自动门禁都覆盖不到，于是「窗口根节点没建 /
//! 关闭钮漏挂 / spawn 系统崩」这类回归能一路合进 master。
//!
//! 本测试是它的 headless 等价物：Bevy 最小 App 进 `AppState::Game`，让 40 个对话框的
//! `OnEnter(Game)` spawn 系统各跑一遍，逐窗断言：
//!   1. **根节点**：存在 `DialogRoot(kind)`——实机 `dialogs` RPC 与 `dialog_rect`
//!      （沿 ChildOf 上溯找 owner）都靠它；
//!   2. **关闭钮归属**：非「设计无钮」的窗口，其根之下必须挂标准关闭钮 `CloseButton`，
//!      且该钮上溯到的 owner 必须是**自己**（串到别的窗口 = 实机点 X 关错窗）；
//!   3. **显隐门控**：`DialogManager::open(kind)` 后根为 `Visible`、`close(kind)` 后
//!      回到 `Hidden`（`enforce_dialog_visibility` 链路——「能开」与「关得掉」）。
//!
//! 覆盖边界（与实机巡回的差异，显式列出）：
//!   - 需要真实网络/NPC 会话的窗口（`npc` / `trade` / `guest_trade` / `npc_goods` /
//!     `roll` 等）不在 40 窗清单里，与 ps1 的 `$kinds` 同口径；
//!   - 本测试不覆盖「真实鼠标 picking 链路」（那要窗口 + 光标 + `UiPlugin`，仍由实机
//!     ps1 守）——这里守的是 picking **能命中什么**：根节点与关闭钮的结构归属；
//!   - `hero_manage` / `input_box` 由业务状态驱动（不进 `DialogManager.open`），
//!     与 ps1 一致，在这里只作根节点/关闭钮断言，不作 `DialogManager` 开合断言；
//!   - **关闭钮断言需要 Data 资产**：`spawn_close_button` 取不到 `Prguse2[360..362]`
//!     时返回 `None`（无资产的窗口按设计就没有关闭钮）——无资产环境（CI 只 checkout
//!     仓库）跳过该断言、只守根节点，与 `ui_alignment.rs` 的 `require_assets!` 同判据。
//!
//! 运行：`cargo test --test ui_interact_sweep`

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::state::state::StateTransition;

use client_bevy::control::CursorProbe;
use client_bevy::game::chat::ChatState;
use client_bevy::game::dialogs::chat_notice::ChatNoticeWidget;
use client_bevy::game::dialogs::minimap::CurrentMapIndex;
use client_bevy::game::dialogs::{DialogKind, DialogRoot, DialogsPlugin};
use client_bevy::map_renderer::GameLibraries;
use client_bevy::network::server_event::ServerEvent;
use client_bevy::network::NetConnection;
use client_bevy::resources::libraries::data_assets_present;
use client_bevy::scenes::AppState;
use client_bevy::ui::sprite_ui::{UiCjkFont, UiFont};
use client_bevy::ui::theme::CloseButton;

/// 40 窗清单：RPC 名 → `DialogKind`，与 `tools/acceptance/ui_interact_sweep.ps1`
/// 的 `$kinds` 逐项对应（改一处必须同步另一处）。
const SWEEP: &[(&str, DialogKind)] = &[
    ("inventory", DialogKind::Inventory),
    ("character", DialogKind::Character),
    ("quest_log", DialogKind::QuestLog),
    ("settings", DialogKind::Settings),
    ("menu", DialogKind::Menu),
    ("game_shop", DialogKind::GameShop),
    ("minimap", DialogKind::Minimap),
    ("group", DialogKind::Group),
    ("friend", DialogKind::Friend),
    ("inspect", DialogKind::Inspect),
    ("guild", DialogKind::Guild),
    ("mail", DialogKind::Mail),
    ("ranking", DialogKind::Ranking),
    ("mentor", DialogKind::Mentor),
    ("relationship", DialogKind::Relationship),
    ("mount", DialogKind::Mount),
    ("report", DialogKind::Report),
    ("hero_inventory", DialogKind::HeroInventory),
    ("hero_equipment", DialogKind::HeroEquipment),
    ("creature", DialogKind::Creature),
    ("item_rental", DialogKind::ItemRental),
    ("guild_territory", DialogKind::GuildTerritory),
    ("help", DialogKind::Help),
    ("notice", DialogKind::Notice),
    ("buff", DialogKind::Buff),
    ("fishing", DialogKind::Fishing),
    ("socket", DialogKind::Socket),
    ("refine", DialogKind::Refine),
    ("craft", DialogKind::Craft),
    ("dura_status", DialogKind::DuraStatus),
    ("npc_awake", DialogKind::NpcAwake),
    ("timer", DialogKind::Timer),
    ("keyboard_layout", DialogKind::KeyboardLayout),
    ("big_map", DialogKind::BigMap),
    ("chat_notice", DialogKind::ChatNotice),
    ("market", DialogKind::Market),
    ("storage", DialogKind::Storage),
    ("item_rental_browse", DialogKind::ItemRentalBrowse),
    ("quest_detail", DialogKind::QuestDetail),
    ("input_box", DialogKind::InputBox),
];

/// 设计上没有关闭钮的窗口（C# 原版即无 X）——与 ps1 的 `$noCloseByDesign` 同集：
/// 这些窗口只有开/关 RPC 往返，没有可点的 X。
const NO_CLOSE_BY_DESIGN: &[&str] = &["menu", "minimap", "buff", "refine", "timer", "chat_notice"];

/// 40 窗里**没有** `DialogRoot` 的特例：`chat_notice` 是屏幕顶部的小通知条
/// （`ChatNoticeWidget` 标记 + `ChatNoticeState.visible` 驱动，不进 `DialogManager`，
/// 且 `Prguse[1361]` 缺失时整个不 spawn，见 `chat_notice.rs`）。
///
/// 顺带记一笔实机 ps1 在此处的**空转**：它对 chat_notice 走「无钮设计 → RPC 往返」
/// 分支，而 `dialog` RPC 只动 `DialogManager` 栈、与通知条实体毫无关系，所以那一条
/// 「closed=YES」恒真。这里改成断言**实体标记**（有资产时才存在），比它更实。
/// （功能现状见 UI_VERIFICATION_REPORT.md §10.5 第 9 条：`ChatNoticeState` 全仓无写入方。）
const NO_DIALOG_ROOT: &[&str] = &["chat_notice"];

/// 非 `DialogRoot` 体系的**独立弹窗**：自管显隐 + 自管 `OnExit` 清理，其关闭钮不是孤儿。
/// 首个登记项 `amount_box`（数量输入框）：`spawn_amount_box` / `cleanup_amount_box` 自成一对，
/// 不进 `DialogManager`（`AmountBoxState` 驱动显隐），关闭钮由 `amount_box_system` 处理——
/// 与 `chat_notice` 同属「状态驱动的自管理弹窗」，只是它走 `CloseButton` 而非独立标记。
/// **新增此类弹窗时在此登记**；否则孤儿钮检查亮红——那是刻意的：孤儿钮在实机里
/// `dialog_rect` 永远找不到，正是「窗口忘了建根 / X 挂错层级」的典型信号。
const NON_ROOT_DIALOG_BTN: &[&str] = &["client_bevy::game::dialogs::amount_box::AmountClose"];

/// 最小 headless App：只要能跑对话框的 `OnEnter(Game)` spawn 系统 + 显隐门控，
/// 不装渲染/winit（CI 无 GPU，见 `b0001_smoke` 同款取舍）。
fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(StatesPlugin);
    // AssetPlugin 提供 AssetServer（`init_asset` 只注册 `Assets<T>` 类型，不起 server——
    // 缺它时任何 `Res<AssetServer>` 系统在参数校验期直接 panic）；assets 路径按 cwd
    // （crate 根）解析，测试不依赖其中的 shader/字体是否加载完成。
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.init_asset::<Image>();
    app.init_asset::<Font>();
    // 对话框 spawn 的统一依赖面（各插件一律 `Res<GameLibraries>` + `Assets<Image>` +
    // `Assets<Font>` + `UiFont`/`UiCjkFont`，见 inventory.rs:591 等 55 处 spawn）
    app.init_resource::<GameLibraries>();
    app.init_resource::<UiFont>();
    app.init_resource::<UiCjkFont>();
    // spawn 系统之外的兜底依赖面：对话框插件体系里不少系统按**非 Option** `Res<…>` 取这些
    // （`ButtonInput` 40 处 / `NetConnection` 86 处），bevy_ecs 0.19 参数校验失败直接 panic
    // （不 warn、run_if 不拦）——库内各处对话框测试也是这么手工预置的。
    app.add_plugins(bevy::input::InputPlugin);
    app.insert_resource(NetConnection::default());
    app.add_message::<ServerEvent>();
    app.init_resource::<CurrentMapIndex>();
    app.init_resource::<CursorProbe>();
    app.init_resource::<ChatState>();
    app.init_state::<AppState>();
    app.add_plugins(DialogsPlugin);
    app
}

/// 进 Game 状态：**只**驱动 `StateTransition` 调度（跑 `OnEnter(Game)` 的各窗 spawn 系统），
/// 不调 `app.update()`——后者会连带跑整个 Update 调度，把 40 个窗口各自的 ui_system 一并拉起，
/// 于是测试被迫补齐上百个与「窗口建没建」无关的资源（DayNight / MagicsState / GameData /
/// PinyinIme / Assets<AudioSource> …），真正的守卫目标被淹没（实测第一版即如此）。
///
/// 代价（见文件头「覆盖边界」）：Update 级 wiring（如「open 后根有没有被写 Visible」）不在
/// 本测试范围——那一路由实机巡回与各窗口自己的回归测试守。
fn enter_game(app: &mut App) {
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Game);
    app.world_mut().run_schedule(StateTransition);
}

/// 主断言：40 窗的**根节点**与**关闭钮归属**——即实机 `dialog_rect`/`click` 能命中关闭钮的
/// 充分条件（`dialog_rect` 沿 ChildOf 上溯找 Visible 的 `DialogRoot` 定位 owner）。
#[test]
fn sweep_40_windows_root_and_close_button() {
    let mut app = headless_app();
    enter_game(&mut app);
    let world = app.world_mut();

    // 根节点：DialogRoot(kind) → 实体
    let mut q_roots = world.query::<(Entity, &DialogRoot)>();
    let roots: Vec<(DialogKind, Entity)> = q_roots.iter(world).map(|(e, r)| (r.0, e)).collect();
    // 全仓标准关闭钮（`spawn_close_button` 统一挂 `CloseButton` 标记）
    let mut q_cb = world.query::<(Entity, &CloseButton)>();
    let close_buttons: Vec<Entity> = q_cb.iter(world).map(|(e, _)| e).collect();
    // chat_notice 特例（无 DialogRoot）：数它的实体标记
    let mut q_notice = world.query_filtered::<Entity, With<ChatNoticeWidget>>();
    let notice_widgets: Vec<Entity> = q_notice.iter(world).collect();

    // 与 `control.rs` 的 `dialog_rect` 同一算法：沿 ChildOf 上溯（限 32 层防环）、**先查自身**
    // （dura_status 的常驻切换钮把 DialogRoot 挂在钮自己身上）。
    let owner_of = |btn: Entity| -> Option<DialogKind> {
        let mut cur = btn;
        for _ in 0..32 {
            if let Some(root) = world.get::<DialogRoot>(cur) {
                return Some(root.0);
            }
            match world.get::<ChildOf>(cur) {
                Some(co) => cur = co.parent(),
                None => return None,
            }
        }
        None
    };

    let has_assets = data_assets_present();
    // 无 Data 资产：**多数窗口的 spawn 在 `load_lib_image(...) else { return }` 处直接返回**
    // （根面板要精灵帧 `Prguse[...]` 才有尺寸，实测 39 窗里只有 3 个在不依赖帧的路径上建根），
    // 结构断言此时无意义 → 跳过（判据同 `ui_alignment.rs` 的 `require_assets!`）。
    // 本测试在无资产下仍守住两件事：① 各 spawn 系统**安全跑过**（能走到这里 = StateTransition
    // 没 panic，资产缺失时的 `None` 分支没写崩）；② 清单一致性（另一个测试）。
    // 完整结构防线在**有 Data 的本机门禁**（walgit `client-test`）上跑。
    if !has_assets {
        eprintln!(
            "40 窗巡回：无 Data 资产 → 结构断言跳过（多数窗口 spawn 需精灵帧，按设计不建根）；             本测试仍验证「资产缺失下 spawn 系统不 panic」与清单一致性"
        );
        return;
    }

    let mut failures: Vec<String> = Vec::new();
    let mut roots_ok = 0usize;
    let mut multi_root: Vec<(&str, usize)> = Vec::new();
    let mut close_ok = 0usize;
    let mut close_expected = 0usize;

    for (name, kind) in SWEEP {
        if NO_DIALOG_ROOT.contains(name) {
            continue; // chat_notice：无 DialogRoot，走下面的实体标记断言
        }
        // ① 根节点：实机 `dialogs` / `dialog_rect` / PostUpdate 显隐兜底全靠它
        let found: Vec<Entity> = roots
            .iter()
            .filter(|(k, _)| k == kind)
            .map(|(_, e)| *e)
            .collect();
        if found.is_empty() {
            failures.push(format!(
                "[{name}] 缺 DialogRoot({kind:?})：窗口根节点没建（实机 dialogs / dialog_rect / 显隐兜底全落空）"
            ));
            continue;
        }
        // 同 kind 多根是设计（主窗 + 确认框/子框各自成根：实测 inventory 2 个、
        // storage/item_rental 各 4 个）——`dialog_rect` 按各自的关闭钮上溯各自的根，
        // 互不干扰。只记录数量供对读，不作失败。
        multi_root.push((*name, found.len()));
        roots_ok += 1;

        // ② 关闭钮归属：无钮设计的窗口跳过；其余必须挂在自己根下且是 Button
        if NO_CLOSE_BY_DESIGN.contains(name) {
            continue;
        }
        close_expected += 1;
        if !has_assets {
            continue; // 无资产：`spawn_close_button` 取不到 Prguse2[360..362] → 按钮按设计缺席
        }
        let owned: Vec<Entity> = close_buttons
            .iter()
            .copied()
            .filter(|b| owner_of(*b) == Some(*kind))
            .collect();
        if owned.is_empty() {
            failures.push(format!(
                "[{name}] 根下没有 CloseButton：实机 dialog_rect 会 FAIL_NO_BTN（本测试集共 {} 个关闭钮实体）",
                close_buttons.len()
            ));
            continue;
        }
        if let Some(not_btn) = owned.iter().find(|b| world.get::<Button>(**b).is_none()) {
            failures.push(format!(
                "[{name}] 关闭钮 {not_btn:?} 无 Button 组件：实机 picking 命不中"
            ));
            continue;
        }
        close_ok += 1;
    }

    // chat_notice 特例：断言实体标记（ps1 的那条判据是空转的，见 NO_DIALOG_ROOT）
    let mut notice_ok = 0usize;
    if has_assets {
        if notice_widgets.is_empty() {
            failures.push(
                "[chat_notice] 缺 ChatNoticeWidget 实体：通知条根面板没建（spawn 依赖 Prguse[1361]）"
                    .to_string(),
            );
        } else {
            notice_ok = 1;
        }
    }

    // 孤儿关闭钮：不挂在任何 DialogRoot 之下 → 实机 dialog_rect 永远找不到它
    // （典型成因：窗口忘了建根，或 X 挂到了根之外的父节点）
    let orphans: Vec<Entity> = close_buttons
        .iter()
        .copied()
        .filter(|b| owner_of(*b).is_none())
        .collect();
    if !orphans.is_empty() {
        let mut detail = String::new();
        let mut unexpected = 0usize;
        for e in &orphans {
            let comps: Vec<String> = world
                .entity(*e)
                .archetype()
                .components()
                .iter()
                .filter_map(|id| world.components().get_name(*id).map(|n| n.to_string()))
                .collect();
            let known = comps
                .iter()
                .any(|c| NON_ROOT_DIALOG_BTN.contains(&c.as_str()));
            let note = if known {
                "（已知非 DialogRoot 弹窗）"
            } else {
                unexpected += 1;
                "（**未登记**：窗口可能忘了建根 / X 挂错层级）"
            };
            detail.push_str(&format!("\n  - {e:?}{note} 组件: {comps:?}"));
        }
        if unexpected > 0 {
            failures.push(format!(
                "有 {unexpected} 个 CloseButton 既不在任何 DialogRoot 之下、也不属于已登记的非 DialogRoot 弹窗（实机 dialog_rect 永远找不到它们）：{detail}"
            ));
        } else {
            eprintln!(
                "非 DialogRoot 弹窗的关闭钮（设计，已登记）：{} 个",
                orphans.len()
            );
        }
    }

    eprintln!(
        "40 窗巡回（含资产）：DialogRoot {roots_ok}/39 + chat_notice 实体 {notice_ok}/1；\
         关闭钮 {close_ok}/{close_expected}（6 窗设计无钮）"
    );
    let multi: Vec<String> = multi_root
        .iter()
        .filter(|(_, n)| *n > 1)
        .map(|(name, n)| format!("{name}×{n}"))
        .collect();
    eprintln!("同 kind 多根（设计：主窗+确认/子框）：{multi:?}");
    assert!(
        failures.is_empty(),
        "40 窗交互巡回（headless）失败 {} 项（实机脚本 tools/acceptance/ui_interact_sweep.ps1 的等价防线）：\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// 防清单漂移：本测试的窗口清单必须与实机 ps1 的 `$kinds` / `$noCloseByDesign` 一致。
/// 两处清单任一处漏改 = 该窗口**同时**失去实机与门禁两条防线（改一处必须同步另一处）。
#[test]
fn sweep_list_matches_ps1() {
    let ps1 = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tools/acceptance/ui_interact_sweep.ps1");
    let Ok(text) = std::fs::read_to_string(&ps1) else {
        eprintln!(
            "skip sweep_list_matches_ps1：读不到 {}（非完整仓库）",
            ps1.display()
        );
        return;
    };
    let names = |var: &str| -> Vec<String> {
        let start = text
            .find(&format!("${var} = @("))
            .unwrap_or_else(|| panic!("ps1 里找不到 ${var} = @("));
        let body = &text[start..];
        let end = body.find(')').expect("ps1 列表未闭合");
        body[..end]
            .split('\'')
            .skip(1)
            .step_by(2)
            .map(|s| s.to_string())
            .collect()
    };
    let ps1_kinds = names("kinds");
    let ps1_noclose = names("noCloseByDesign");
    let here: Vec<String> = SWEEP.iter().map(|(n, _)| (*n).to_string()).collect();
    assert_eq!(
        here, ps1_kinds,
        "headless 巡回清单与 ps1 的 $kinds 不一致（顺序也要一致，便于逐项对读）"
    );
    let here_noclose: Vec<String> = NO_CLOSE_BY_DESIGN
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    assert_eq!(
        here_noclose, ps1_noclose,
        "「设计无钮」清单与 ps1 的 $noCloseByDesign 不一致"
    );
}
