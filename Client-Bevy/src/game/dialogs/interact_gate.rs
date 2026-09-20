//! 40 窗「点 X 关」交互门禁 —— 把实机巡回脚本的契约搬进 `cargo test --lib`。
//!
//! 背景：`tools/acceptance/ui_interact_sweep.ps1` 需要「服务端 + 真实 `Data/` 资产 +
//! 桌面窗口」，只能在开发机人工跑，进不了任何门禁；于是「窗口有没有标准关闭钮、
//! 按了关不关」这类回归长期无人自动守——#2955 的 `storage`（钮缺 `StorageWidget`）与
//! `hero_manage`（钮缺 `CloseButton`）两处就是这样一路漏到实机巡回才被抓出来的。
//! 本模块以**无窗口、无服务端、无真实资产**的方式复刻巡回的核心判据。
//!
//! 与实机脚本的**有意差异**（越界使用会得出错误结论，故写明）：
//! - 资产用**合成最小 `.Lib`**（每库 [`SYNTH_FRAMES`] 帧 1x1，见 [`write_synth_lib`]）：
//!   `Data/` 不入库、CI 没有真实精灵，而各窗 spawn 里普遍是
//!   `let Some(bg) = load_lib_image(..) else { return }`——没有帧就整窗不建。
//!   因此本门禁守的是「关闭钮存在 + 按压→关闭」的**接线**，**不守**像素级命中区/遮挡
//!   （那类缺陷见 #2953，仍需实机脚本）。
//! - 按压注入直接写 `Interaction::Pressed` 后**只跑 `Update` 调度**：bevy_ui 的
//!   `ui_focus_system` 在 `PreUpdate` 会按真实光标把 `Interaction` 复位成 `None`，
//!   而本环境没有窗口与相机，走不了真实命中链路（`click` RPC 那条路要窗口 + 相机）。
//!   换言之：本门禁等价于「光标确实落在关闭钮上」之后的那一段。
//!
//! 开窗语义逐条对齐 `control.rs` 的 `ControlCommand::Dialog` 分支（含状态驱动窗的
//! `HeroState.managing` / `InputBoxState.open` / `StorageState.visible` 特例），
//! 判据沿用 `dialog_rect` 的「CloseButton + 最近祖先 `DialogRoot` 且其 Visible」。

use super::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::{Libraries, LibraryName};
use crate::scenes::AppState;
use bevy::prelude::*;

/// 巡回脚本 `$kinds` 的 A 类窗口（40 项）＋ `hero_manage`（脚本里单独走状态窗路径，
/// 同样要「点 X 关」，故并入本门禁）。
const SWEEP_KINDS: &[(&str, DialogKind)] = &[
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
    ("hero_manage", DialogKind::HeroManage),
];

/// 脚本 `$noCloseByDesign`：C# 原版即无 X 的窗口（只验证开/关往返）。
const NO_CLOSE_BY_DESIGN: &[&str] = &["menu", "minimap", "buff", "refine", "timer", "chat_notice"];

/// 合成库帧数：需覆盖各窗引用的最大精灵索引（`Prguse[2443]` 是全表最大值），
/// 索引越界会让 `load_lib_image` 返回 `None` → 对应窗直接不建。
const SYNTH_FRAMES: usize = 2600;

/// 需要合成的单体库（= `Libraries::init_single_libraries` 的全表；新增库要同步）。
const SYNTH_LIBS: &[LibraryName] = &[
    LibraryName::ChrSel,
    LibraryName::Prguse,
    LibraryName::Prguse2,
    LibraryName::Prguse3,
    LibraryName::BuffIcon,
    LibraryName::Help,
    LibraryName::MiniMap,
    LibraryName::MapLinkIcon,
    LibraryName::Title,
    LibraryName::MagIcon,
    LibraryName::MagIcon2,
    LibraryName::Magic,
    LibraryName::Magic2,
    LibraryName::Magic3,
    LibraryName::Effect,
    LibraryName::MagicC,
    LibraryName::GuildSkill,
    LibraryName::Weather,
    LibraryName::Background,
    LibraryName::Dragon,
    LibraryName::Items,
    LibraryName::StateItems,
    LibraryName::FloorItems,
    LibraryName::Deco,
];

/// 合成资产目录（进程内只建一次）。
fn synth_data_dir() -> std::path::PathBuf {
    static DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        let dir = std::env::temp_dir().join("crystal_ui_interact_gate_libs");
        std::fs::create_dir_all(&dir).expect("建合成资产目录");
        for name in SYNTH_LIBS {
            write_synth_lib(
                &dir.join(format!("{}.Lib", name.default_path())),
                SYNTH_FRAMES,
            );
        }
        dir
    })
    .clone()
}

/// 写一个最小合法 `.Lib`（version 2）：文件头 + 索引表 + 每帧
/// 「17 字节 `ImageInfo` 头 + gzip(BGRA)」（见 `resources/mlibrary.rs` 的解析）。
/// 所有帧共用同一份 1x1 压缩数据——门禁只关心「精灵存在 ⇒ 控件被创建」，图案无意义。
fn write_synth_lib(path: &std::path::Path, frames: usize) {
    use std::io::Write;
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    enc.write_all(&[0x40, 0x80, 0xC0, 0xFF]).expect("gzip 写入");
    let payload = enc.finish().expect("gzip 收尾");

    let header_len = 8 + frames * 4;
    let frame_len = 17 + payload.len();
    let mut out = Vec::with_capacity(header_len + frames * frame_len);
    out.extend_from_slice(&2i32.to_le_bytes()); // version < 3 → 无 frame_seek 字段
    out.extend_from_slice(&(frames as i32).to_le_bytes());
    for i in 0..frames {
        out.extend_from_slice(&((header_len + i * frame_len) as i32).to_le_bytes());
    }
    for _ in 0..frames {
        out.extend_from_slice(&1i16.to_le_bytes()); // width
        out.extend_from_slice(&1i16.to_le_bytes()); // height
        for _ in 0..4 {
            // offset_x / offset_y / shadow_x / shadow_y
            out.extend_from_slice(&0i16.to_le_bytes());
        }
        out.push(0u8); // shadow：bit7=0 → 无遮罩层
        out.extend_from_slice(&(payload.len() as i32).to_le_bytes());
        out.extend_from_slice(&payload);
    }
    std::fs::write(path, out).expect("写合成 .Lib");
}

/// 最小可跑的 headless App：与生产同构地进 `AppState::Game`（各窗在 `OnEnter` 里
/// 建自己的 UI），但不含窗口/渲染/网络。
fn sweep_app() -> App {
    let mut libs = Libraries::new(synth_data_dir());
    libs.init_single_libraries();
    // 合成库已装载：置 initialized 阻止各窗 spawn 里的 `ensure_initialized()` 把
    // data_path 改回 `resolve_data_path()`（CI 无 `Data/` → 每窗 return，门禁空转）
    libs.initialized = true;

    let mut app = App::new();
    // 最小插件组合（同 `ui/outlined_text.rs` 的 UiStack 测试）：全量 DefaultPlugins
    // 要窗口事件循环，headless CI 会炸
    app.add_plugins((
        bevy::asset::AssetPlugin::default(),
        bevy::time::TimePlugin,
        bevy::input::InputPlugin,
        bevy::picking::InteractionPlugin,
        bevy::picking::PickingPlugin,
        bevy::image::ImagePlugin::default(),
        bevy::sprite::SpritePlugin,
        bevy::mesh::MeshPlugin,
        bevy::text::TextPlugin,
        bevy::ui::UiPlugin,
        bevy::state::app::StatesPlugin,
    ));
    app.init_state::<AppState>();
    app.insert_resource(GameLibraries(libs));
    app.insert_resource(crate::network::NetConnection::default());
    app.add_plugins(super::DialogsPlugin);
    provision_foreign_resources(&mut app);
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Game);
    app.update(); // OnEnter(Game)：各窗 spawn 自身 UI
    app
}

/// 补上「对话框子系统之外」的资源：生产里由 `GamePlugin` 的各子插件（chat/skills/
/// sound/day_night/input_gate…）与 `MapRenderPlugin` 提供，这里不需要它们的行为，
/// 只要资源在位（缺任一 → 对应系统一跑就 panic）。新增对话框若读了新的外部资源，
/// 这里会以「Resource does not exist」报出来，补一行即可。
fn provision_foreign_resources(app: &mut App) {
    app.init_resource::<crate::map_renderer::GameData>();
    app.init_resource::<crate::game::chat::ChatState>();
    app.init_resource::<crate::game::sound::SoundBank>();
    app.init_resource::<crate::game::day_night::DayNight>();
    app.init_resource::<crate::game::skills::MagicsState>();
    app.init_resource::<crate::game::skills::MagicCooldowns>();
    app.init_resource::<crate::game::input_gate::TextInputGate>();
    app.init_resource::<crate::ui::sprite_ui::UiFont>();
    app.init_resource::<crate::ui::sprite_ui::UiCjkFont>();
    app.init_resource::<crate::control::CursorProbe>();
    app.init_resource::<crate::game::dialogs::minimap::CurrentMapIndex>();
    app.init_resource::<crate::game::object_state::InfoCache>();
    app.init_resource::<crate::game::player_menu::PlayerMenuState>();
    app.init_resource::<crate::game::dialogs::quest_tracking::QuestTrackingState>();
    app.init_resource::<crate::ui::pinyin_ime::ImeFocus>();
    // 音频资产：`inv_sound_system` 等按 `Assets<AudioSource>` 判存在（不装 AudioPlugin）
    app.init_resource::<Assets<bevy::audio::AudioSource>>();
    app.init_resource::<crate::game::sound::SoundCache>();
    app.init_resource::<crate::ui::new_character::NewCharState>();
    // 各窗的 ServerEvent 读取端（38 个系统）：生产由 NetworkPlugin 注册
    app.add_message::<crate::network::server_event::ServerEvent>();
    app.add_message::<crate::game::dialogs::quest_tracking::ToggleQuestTracking>();
    // `PinyinIme` 无 Default：`new()` 在缺 libpinyin 数据时降级为「禁用内置 IME」
    // （不 panic，见 src/ui/pinyin_ime.rs），CI 无数据也安全
    app.insert_resource(crate::ui::pinyin_ime::PinyinIme::new());
}

/// 开窗（逐条对齐 `control.rs` 的 `ControlCommand::Dialog { action: Open }`）。
fn open_window(app: &mut App, kind: DialogKind) {
    match kind {
        DialogKind::HeroManage => {
            app.world_mut()
                .resource_mut::<super::hero::HeroState>()
                .managing = true;
        }
        DialogKind::InputBox => {
            app.world_mut()
                .resource_mut::<super::input_box::InputBoxState>()
                .open = true;
        }
        DialogKind::Storage => {
            app.world_mut()
                .resource_mut::<super::storage::StorageState>()
                .visible = true;
            app.world_mut().resource_mut::<DialogManager>().open(kind);
        }
        _ => app.world_mut().resource_mut::<DialogManager>().open(kind),
    }
}

/// 关窗（对齐 `ControlCommand::Dialog { action: Close }`）。
fn close_window(app: &mut App, kind: DialogKind) {
    match kind {
        DialogKind::HeroManage => {
            let mut hero = app.world_mut().resource_mut::<super::hero::HeroState>();
            hero.managing = false;
            hero.confirm_slot = None;
        }
        DialogKind::InputBox => {
            app.world_mut()
                .resource_mut::<super::input_box::InputBoxState>()
                .open = false;
        }
        DialogKind::Storage => {
            app.world_mut()
                .resource_mut::<super::storage::StorageState>()
                .visible = false;
            app.world_mut().resource_mut::<DialogManager>().close(kind);
        }
        _ => app.world_mut().resource_mut::<DialogManager>().close(kind),
    }
}

/// 该 kind 的根实体（`DialogRoot(kind)`）及其 `Visibility`（无 `Visibility` 视为不可见）。
/// `World::iter_entities` 走 `&World`（`query()` 需要 `&mut World`，与调用点借阅冲突）。
fn roots_of(world: &World, kind: DialogKind) -> Vec<(Entity, Visibility)> {
    world
        .iter_entities()
        .filter(|e| e.get::<DialogRoot>().is_some_and(|r| r.0 == kind))
        .map(|e| {
            (
                e.id(),
                e.get::<Visibility>().copied().unwrap_or(Visibility::Hidden),
            )
        })
        .collect()
}

/// 从关闭钮沿 `ChildOf` 上溯找**最近**的 `DialogRoot`（先查自身——`dura_status` 的
/// 常驻切换钮就挂在钮自己身上），限 32 层防环；与 `dialog_rect` RPC 同一判据。
fn owner_root(world: &World, btn: Entity) -> Option<Entity> {
    let mut cur = btn;
    for _ in 0..32 {
        if world.get::<DialogRoot>(cur).is_some() {
            return Some(cur);
        }
        let Some(co) = world.get::<ChildOf>(cur) else {
            return None;
        };
        cur = co.parent();
    }
    None
}

/// 归属该窗的标准关闭钮。`require_visible_root` = `dialog_rect` 的判据
/// （根 `Visibility::Visible` 才算找到）；关掉它可区分「没钮」与「钮在但窗没打开」。
fn close_buttons_of(world: &World, kind: DialogKind, require_visible_root: bool) -> Vec<Entity> {
    world
        .iter_entities()
        .filter(|e| e.contains::<crate::ui::theme::CloseButton>())
        .map(|e| e.id())
        .filter(|btn| {
            owner_root(world, *btn).is_some_and(|root| {
                world.get::<DialogRoot>(root).is_some_and(|r| r.0 == kind)
                    && (!require_visible_root
                        || world.get::<Visibility>(root) == Some(&Visibility::Visible))
            })
        })
        .collect()
}

/// 合成按压：写 `Interaction::Pressed` 后只跑 `Update` 调度（理由见模块头注释）。
fn press(app: &mut App, btn: Entity) {
    app.world_mut().entity_mut(btn).insert(Interaction::Pressed);
    app.world_mut().run_schedule(Update);
}

/// 关窗判据。默认看 `DialogManager` 栈（= 脚本 `dialogs` RPC 的判据）；
/// `hero_manage` 是纯状态驱动窗（`AlwaysVisible` + `HeroState.managing`，**从不进栈**）
/// → 看状态位（脚本对它的判据正是 `visible` RPC）。
fn is_closed(app: &App, kind: DialogKind) -> bool {
    match kind {
        DialogKind::HeroManage => !app.world().resource::<super::hero::HeroState>().managing,
        _ => !app.world().resource::<DialogManager>().is_open(kind),
    }
}

/// 40 窗「点 X 关」闭环：每窗开 → 定位标准关闭钮 → 按压 → 断言关栈。
#[test]
fn sweep_windows_close_via_standard_close_button() {
    let mut app = sweep_app();
    let mut fails: Vec<String> = Vec::new();
    let mut passed = 0usize;
    // 走「真实标准关闭钮按压」的窗数（其余是「设计无 X」的开关往返）——打印出来
    // 是为了让「41/41」看得出不是空转：没有钮就算失败，所以通过的必是按压过的
    let mut by_button = 0usize;

    for (name, kind) in SWEEP_KINDS {
        let (name, kind) = (*name, *kind);
        open_window(&mut app, kind);
        app.update();
        app.update();

        if NO_CLOSE_BY_DESIGN.contains(&name) {
            // 脚本对这类窗口的判据：不该有可见的标准关闭钮 + 开/关往返后 mgr 不留残留
            // （`chat_notice` 是纯状态覆盖层，既无 DialogRoot 也不进 mgr，故不做根断言）
            let btns = close_buttons_of(app.world(), kind, true);
            if !btns.is_empty() {
                fails.push(format!(
                    "{name}: 脚本按「设计无 X」处理，却存在可见的标准关闭钮——名单或实现漂了"
                ));
            }
            close_window(&mut app, kind);
            app.update();
            if is_closed(&app, kind) {
                passed += 1;
            } else {
                fails.push(format!("{name}: 关闭后仍在 DialogManager.open 栈"));
            }
            continue;
        }

        if roots_of(app.world(), kind).is_empty() {
            fails.push(format!("{name}: 没有 DialogRoot({kind:?})——窗口未建立"));
            close_window(&mut app, kind);
            app.update();
            continue;
        }

        let btns = close_buttons_of(app.world(), kind, true);

        if btns.is_empty() {
            let all = close_buttons_of(app.world(), kind, false);
            let detail = if all.is_empty() {
                "无 theme::CloseButton（同 dialog_rect 的 close button not found）"
            } else {
                "有 CloseButton，但其 DialogRoot 不是 Visible（窗口没打开）"
            };
            fails.push(format!("{name}: {detail}"));
            close_window(&mut app, kind);
            app.update();
            continue;
        }

        for btn in &btns {
            press(&mut app, *btn);
        }
        app.update(); // PostUpdate 的 enforce_dialog_visibility 按关栈结果隐藏根

        if !is_closed(&app, kind) {
            let visible = roots_of(app.world(), kind)
                .iter()
                .filter(|(_, v)| *v == Visibility::Visible)
                .count();
            fails.push(format!(
                "{name}: 按压标准关闭钮后仍在 open 栈（该窗可见根 {visible} 个）"
            ));
            close_window(&mut app, kind);
            app.update();
            continue;
        }
        passed += 1;
        by_button += 1;
    }

    eprintln!(
        "ui 交互门禁（headless 巡回）：{passed}/{} 窗通过（{by_button} 窗按压真实标准关闭钮，其余为「设计无 X」的开关往返）",
        SWEEP_KINDS.len()
    );
    assert!(
        fails.is_empty(),
        "headless 交互门禁失败 {} 项（实机脚本：tools/acceptance/ui_interact_sweep.ps1）:\n{}",
        fails.len(),
        fails.join("\n")
    );
}

/// 从 PowerShell 数组字面量里取单引号项（`$var = @( 'a','b' )`）。
fn ps_array(src: &str, var: &str) -> Vec<String> {
    let head = format!("${var} = @(");
    let start = src
        .find(&head)
        .unwrap_or_else(|| panic!("脚本里找不到 {head}"));
    let rest = &src[start + head.len()..];
    let end = rest.find(')').expect("数组字面量未闭合");
    rest[..end]
        .split('\'')
        .skip(1)
        .step_by(2)
        .map(|s| s.trim().to_string())
        .collect()
}

/// 名单漂移守卫：本门禁的名单必须与实机脚本一致——否则「脚本加了窗、门禁没跟上」
/// 会静默发生，而脚本自身不在任何门禁里（正是本模块要解决的问题）。
#[test]
fn sweep_kind_list_matches_live_script() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tools/acceptance/ui_interact_sweep.ps1");
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("读不到实机巡回脚本 {}: {e}", path.display()));

    let script_kinds = ps_array(&src, "kinds");
    assert_eq!(script_kinds.len(), 40, "脚本 $kinds 应是 40 项");

    // 本模块比脚本多一项 hero_manage（脚本把它放在单独的状态窗段落里）
    let ours: Vec<&str> = SWEEP_KINDS
        .iter()
        .map(|(n, _)| *n)
        .filter(|n| *n != "hero_manage")
        .collect();
    assert_eq!(
        ours,
        script_kinds.iter().map(String::as_str).collect::<Vec<_>>(),
        "本门禁名单与脚本 $kinds 不一致"
    );

    let script_no_close = ps_array(&src, "noCloseByDesign");
    assert_eq!(
        NO_CLOSE_BY_DESIGN,
        script_no_close
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        "「设计无 X」名单与脚本 $noCloseByDesign 不一致"
    );
}
