//! 41 窗「点 X 关」交互门禁 —— 把实机巡回脚本的契约搬进 `cargo test --lib`。
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

/// 巡回脚本 `$kinds` 的 A 类窗口（41 项）＋ `hero_manage`（脚本里单独走状态窗路径，
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
    // #3209：写邮件窗（状态驱动，RPC 直接切 `MailState.compose`）——同样要「点 X 关」
    ("mail_compose", DialogKind::MailCompose),
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

/// 邮件窗结构门禁的专用合成资产目录：除 `Title` 的上列帧外全是 1x1。
///
/// 声明值 = **真实资产实测**（`Data/Title.Lib`，2026-09-25 用帧头读得）：
/// `[670]=312x444`（邮件列表）、`[671]=236x300`（写信）、`[672]=236x300`（读书信）、
/// `[674]=236x384`（待寄）、`[675]=236x384`（读包裹）、`[676]=144x36`（ItemCover）。
/// 其中 `Title[675]` 的 384 正是「C# `MailReadParcelDialog.Size` 声明 236x300 与其控件
/// y 到 350 自相矛盾」时按美术落地的依据。
fn synth_mail_lib_dir() -> std::path::PathBuf {
    static DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        let dir = std::env::temp_dir().join("crystal_ui_mail_struct_libs");
        std::fs::create_dir_all(&dir).expect("建邮件结构门禁合成资产目录");
        for name in SYNTH_LIBS {
            let sized = name == &LibraryName::Title;
            write_synth_lib_sized(
                &dir.join(format!("{}.Lib", name.default_path())),
                SYNTH_FRAMES,
                |i| match (sized, i) {
                    (true, 670) => (312, 444),
                    (true, 671) | (true, 672) => (236, 300),
                    (true, 674) | (true, 675) => (236, 384),
                    (true, 676) => (144, 36),
                    _ => (1, 1),
                },
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
    write_synth_lib_sized(path, frames, |_| (1, 1));
}

/// 每个索引可自定义宽高的合成 `.Lib`。
///
/// 像素数据仍是一份 1x1 gzip（加载器会按声明的 `width*height*4` 补零，
/// 见 `mlibrary.rs::decompress_image`），**但 `ImageInfo` 里声明的尺寸是真的**——
/// 于是 Bevy 侧的 `Assets<Image>` 会带正确像素尺寸，可用于「根面板底图是不是那一帧」的判据。
fn write_synth_lib_sized(
    path: &std::path::Path,
    frames: usize,
    size_of: impl Fn(usize) -> (i16, i16),
) {
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
    for i in 0..frames {
        let (w, h) = size_of(i);
        out.extend_from_slice(&w.to_le_bytes()); // width
        out.extend_from_slice(&h.to_le_bytes()); // height
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
    sweep_app_with(synth_data_dir())
}

/// 同 [`sweep_app`]，但换成指定的合成资产目录（供「要按帧尺寸做判据」的用例使用，
/// 见 [`mail_panels_have_native_background_sprites`]）。
fn sweep_app_with(data_dir: std::path::PathBuf) -> App {
    let mut libs = Libraries::new(data_dir);
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
        // #3209：写邮件窗由 `MailState.compose` 驱动（`mail_compose_ui_system` 按它切根
        // Visibility），不进 `DialogManager`——与 hero_manage 同款的状态驱动窗。
        DialogKind::MailCompose => {
            let mut mail = app.world_mut().resource_mut::<super::mail::MailState>();
            mail.compose = true;
            mail.compose_parcel = false;
            // 同 RPC：写信窗必须连父窗（`Mail` 列表窗）一起开——
            // `mail_compose_follow_system` 的孤儿守卫否则会立刻把它关掉
            app.world_mut().resource_mut::<DialogManager>().open(DialogKind::Mail);
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
        DialogKind::MailCompose => {
            let mut mail = app.world_mut().resource_mut::<super::mail::MailState>();
            mail.compose = false;
            mail.compose_parcel = false;
            app.world_mut().resource_mut::<DialogManager>().close(DialogKind::Mail);
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
        // 状态驱动窗：判据取**状态位本身**，不能退化成 `DialogManager.is_open`
        // （那对不进 mgr 的窗恒为 false ⇒ 关窗断言会恒绿、白测）
        DialogKind::MailCompose => !app.world().resource::<super::mail::MailState>().compose,
        _ => !app.world().resource::<DialogManager>().is_open(kind),
    }
}

/// 41 窗「点 X 关」闭环：每窗开 → 定位标准关闭钮 → 按压 → 断言关栈。
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

/// 名单漂移守卫：本门禁的名单必须与**交互巡回覆盖清单**（`tools/acceptance/interact_sweep_manifest.json`）
/// 一致——否则「清单加了窗、门禁没跟上」会静默发生。
///
/// 原实现解析 `ui_interact_sweep.ps1` 里的 `$kinds = @( ... )` 字面量；该脚本已改为**读清单**
/// （报告 §6.1），脚本里不再有名单字面量——继续按字面量解析会得到空名单，故改为读清单本身，
/// 与 `control.rs` 的 `interact_sweep_manifest_covers_all_rpc_kinds` 共享同一份真源。
#[test]
fn sweep_kind_list_matches_live_manifest() {
    // include_str!：清单被删/改名 = 编译失败，而不是静默跳过一次不存在的对账
    let m: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tools/acceptance/interact_sweep_manifest.json"
    ))
    .expect("交互巡回清单必须是合法 JSON");
    let names = |key: &str| -> Vec<String> {
        m[key]
            .as_array()
            .unwrap_or_else(|| panic!("清单缺数组字段 {key}"))
            .iter()
            .map(|v| v.as_str().expect("清单项应为字符串").to_string())
            .collect()
    };

    let manifest_kinds = names("sweep");
    assert_eq!(manifest_kinds.len(), 41, "清单 sweep 应是 41 项");

    // 本模块比清单多一项 hero_manage（清单把它登记在 excluded：走状态窗专用段）
    let ours: Vec<&str> = SWEEP_KINDS
        .iter()
        .map(|(n, _)| *n)
        .filter(|n| *n != "hero_manage")
        .collect();
    assert_eq!(
        ours,
        manifest_kinds
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        "本门禁名单与清单 sweep 不一致"
    );

    let manifest_no_close = names("no_close_by_design");
    assert_eq!(
        NO_CLOSE_BY_DESIGN,
        manifest_no_close
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        "「设计无 X」名单与清单 no_close_by_design 不一致"
    );
}

/// #3103：写/读邮件**四张窗**的「真底图」结构门禁。
///
/// 上一批（#3106）留下的债：`compose_windows_match_csharp_geometry` 只断言「常量 == 字面量」，
/// 把实现改回半透明黑底它照样绿（独立复核指出）。本用例补**实现级**判据：
/// spawn 出来的四张窗根面板必须
/// ① 带 `ImageNode`（= 真底图，而不是 `BackgroundColor` 自造底）；
/// ② 底图纹理尺寸 == 该窗面板的 C#/美术尺寸（本用例用按真实尺寸声明的合成 `Title` 库，
///    故这条能钉住"用的是哪一帧"）；
/// ③ 根上不得出现自造的 60% 黑底（owner 2026-09-24 截图里的病象）。
///
/// 阳性对照（实做）：把任一面板改回 `BackgroundColor(srgba(0,0,0,0.6))` → ① 直接红
/// （没有 `ImageNode`）；把面板图换成另一帧**同尺寸**的 `Title[672]`（236x300）→ ② 不红，
/// 如实记录（同尺寸不同帧在这种判据下不可分辨，靠 `ui_alignment` 的资产尺寸表兜住）。
#[test]
fn mail_panels_have_native_background_sprites() {
    use super::mail::{ComposeWin, MailComposeRoot, MailReadRoot, ReadWin};

    let mut app = sweep_app_with(synth_mail_lib_dir());
    app.update(); // 让 OnEnter(Game) 的 spawn 命令落到世界

    let mut checked: Vec<(String, (u32, u32))> = Vec::new();
    // 先把四张窗的「标记 + 图柄 + Node + 底色」拷出来（查询要 `&mut World`，
    // 不能在同一个借用里再读 `Assets<Image>`）
    let found: Vec<(String, Handle<Image>, Node, Option<BackgroundColor>)> = {
        let world = app.world_mut();
        let mut compose_q = world.query::<(
            &MailComposeRoot,
            &ImageNode,
            &Node,
            Option<&BackgroundColor>,
        )>();
        let compose: Vec<_> = compose_q
            .iter(world)
            .map(|(root, img, node, bg)| {
                (
                    format!("{:?}", root.0),
                    img.image.clone(),
                    node.clone(),
                    bg.copied(),
                )
            })
            .collect();
        let mut read_q =
            world.query::<(&MailReadRoot, &ImageNode, &Node, Option<&BackgroundColor>)>();
        let read: Vec<_> = read_q
            .iter(world)
            .map(|(root, img, node, bg)| {
                (
                    format!("{:?}", root.0),
                    img.image.clone(),
                    node.clone(),
                    bg.copied(),
                )
            })
            .collect();
        compose.into_iter().chain(read).collect()
    };
    {
        let images = app.world().resource::<Assets<Image>>();
        for (tag, handle, node, bg) in found {
            let expect = match tag.as_str() {
                "Letter" => (236.0_f32, 300.0_f32),
                "Parcel" => (236.0, 384.0),
                other => panic!("未知邮件窗 {other}"),
            };
            let img = images.get(&handle).unwrap_or_else(|| {
                panic!("{tag}: 根面板底图句柄不在 Assets<Image> 里（自造底？）")
            });
            assert_eq!(
                (img.width() as f32, img.height() as f32),
                expect,
                "[底图] {tag}: 根面板纹理应来自该窗的 Title 帧（{}x{}）",
                expect.0,
                expect.1
            );
            assert_eq!(
                (node.width, node.height),
                (Val::Px(expect.0), Val::Px(expect.1)),
                "[几何] {tag}: 根面板 Node 尺寸应等于 C# 面板尺寸"
            );
            if let Some(bg) = bg {
                let c = bg.0.to_srgba();
                let is_selfmade_overlay = c.alpha > 0.0
                    && c.alpha < 1.0
                    && c.red == 0.0
                    && c.green == 0.0
                    && c.blue == 0.0;
                assert!(
                    !is_selfmade_overlay,
                    "[底图] {tag}: 根面板不得用自造半透明黑底（owner 截图病象）"
                );
            }
            checked.push((tag, (img.width(), img.height())));
        }
    }

    // 四张窗（写信/待寄/读书信/读包裹）都必须被查到——少一张说明它压根没建
    let tags: Vec<&str> = checked.iter().map(|(t, _)| t.as_str()).collect();
    for want in ["Letter", "Parcel"] {
        assert_eq!(
            tags.iter().filter(|t| **t == want).count(),
            2,
            "写侧/读侧各应有一张 {want} 窗（实查：{tags:?}）"
        );
    }
    eprintln!("ui 邮件窗结构门禁：{checked:?}");
}
