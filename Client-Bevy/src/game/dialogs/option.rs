// ============================================================================
// 设置对话框（M51）
// 参考：C# OptionDialog（Client/MirScenes/Dialogs/MainDialogs.cs）
//   - 背景 Title[411]（259x354，纹理自带各行标签文字），屏幕居中
//   - 关闭按钮 Prguse2[360/361/362] 右上角
//   - 8 组开/关按钮（On/Off，选中态切换纹理帧，与原版 BeforeDraw 一致）
//   - 2 条音量滑条（点击/拖动设置音量，Prguse2[468] 填充 + Prguse[20] 滑块）
// 纯客户端：所有设置仅保存在本地 OptionState
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_image, spawn_label, spawn_panel,
    CloseButton, ImageButton,
};

/// 设置状态（对应 C# Settings，纯本地）
#[derive(Resource)]
pub struct OptionState {
    /// 技能释放模式：true=Ctrl 模式，false=~ 模式
    pub skill_mode_ctrl: bool,
    /// 技能栏显示
    pub skill_bar: bool,
    /// 特效
    pub effect: bool,
    /// 掉落物显示
    pub drop_view: bool,
    /// 名称显示
    pub name_view: bool,
    /// 血条显示模式
    pub hp_view: bool,
    /// 音效音量 0.0-1.0
    pub sound_volume: f32,
    /// 音乐音量 0.0-1.0
    pub music_volume: f32,
    /// 允许观察
    pub allow_observe: bool,
    /// 新移动模式
    pub new_move: bool,
    /// 模式标签显示（C# Settings.ModeView，默认 false；仅 INI，无游戏内开关）
    pub mode_view: bool,
}

impl Default for OptionState {
    fn default() -> Self {
        Self {
            skill_mode_ctrl: true,
            skill_bar: true,
            effect: true,
            drop_view: true,
            name_view: true,
            hp_view: true,
            sound_volume: 0.8,
            music_volume: 0.6,
            allow_observe: false,
            new_move: true,
            mode_view: false,
        }
    }
}

impl OptionState {
    /// 从 INI 文本解析（C# InIReader；音量按 0-100 存储 ↔ 0.0-1.0）
    pub fn from_ini(content: &str) -> Self {
        let mut s = Self::default();
        s.sound_volume = crate::game::dialogs::settings_file::ini_percent(
            content,
            "Sound",
            "Volume",
            s.sound_volume,
        );
        s.music_volume = crate::game::dialogs::settings_file::ini_percent(
            content,
            "Sound",
            "Music",
            s.music_volume,
        );
        s.skill_mode_ctrl = crate::game::dialogs::settings_file::ini_bool(
            content,
            "Game",
            "SkillMode",
            s.skill_mode_ctrl,
        );
        s.skill_bar =
            crate::game::dialogs::settings_file::ini_bool(content, "Game", "SkillBar", s.skill_bar);
        s.effect =
            crate::game::dialogs::settings_file::ini_bool(content, "Game", "Effect", s.effect);
        s.drop_view =
            crate::game::dialogs::settings_file::ini_bool(content, "Game", "DropView", s.drop_view);
        s.name_view =
            crate::game::dialogs::settings_file::ini_bool(content, "Game", "NameView", s.name_view);
        s.hp_view =
            crate::game::dialogs::settings_file::ini_bool(content, "Game", "HPMPView", s.hp_view);
        s.allow_observe = crate::game::dialogs::settings_file::ini_bool(
            content,
            "Game",
            "AllowObserve",
            s.allow_observe,
        );
        s.new_move =
            crate::game::dialogs::settings_file::ini_bool(content, "Game", "NewMove", s.new_move);
        s.mode_view =
            crate::game::dialogs::settings_file::ini_bool(content, "Game", "ModeView", s.mode_view);
        s
    }

    /// 序列化为 INI 文本（对齐 C# Settings.Save 的 [Sound]/[Game] 段）
    pub fn to_ini(&self) -> String {
        let pct = |v: f32| ((v * 100.0).round() as i32).clamp(0, 100);
        format!(
            "[Sound]\nVolume={}\nMusic={}\n\n[Game]\nSkillMode={}\nSkillBar={}\nEffect={}\nDropView={}\nNameView={}\nHPMPView={}\nAllowObserve={}\nNewMove={}\nModeView={}\n",
            pct(self.sound_volume),
            pct(self.music_volume),
            self.skill_mode_ctrl,
            self.skill_bar,
            self.effect,
            self.drop_view,
            self.name_view,
            self.hp_view,
            self.allow_observe,
            self.new_move,
            self.mode_view,
        )
    }

    /// 启动时加载（C# Settings.Load；文件不存在用默认值）
    pub fn load() -> Self {
        let content = crate::game::dialogs::settings_file::load_ini();
        Self::from_ini(&content)
    }

    /// 保存（C# Settings.Save；merge 写回，保留 [Chat]/[Filter] 等其他 section）
    pub fn save(&self) {
        use crate::game::dialogs::settings_file::{set_ini_value, write_ini};
        let mut content = crate::game::dialogs::settings_file::load_ini();
        let pct = |v: f32| ((v * 100.0).round() as i32).clamp(0, 100).to_string();
        for (k, v) in [
            ("Volume", pct(self.sound_volume)),
            ("Music", pct(self.music_volume)),
        ] {
            content = set_ini_value(&content, "Sound", k, &v);
        }
        for (k, v) in [
            ("SkillMode", self.skill_mode_ctrl.to_string()),
            ("SkillBar", self.skill_bar.to_string()),
            ("Effect", self.effect.to_string()),
            ("DropView", self.drop_view.to_string()),
            ("NameView", self.name_view.to_string()),
            ("HPMPView", self.hp_view.to_string()),
            ("AllowObserve", self.allow_observe.to_string()),
            ("NewMove", self.new_move.to_string()),
            ("ModeView", self.mode_view.to_string()),
        ] {
            content = set_ini_value(&content, "Game", k, &v);
        }
        write_ini(&content);
        tracing::debug!("⚙️ 设置已保存到 Mir2Config.ini");
    }
}

/// 设置行类型（与 C# 各按钮组一一对应）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OptionToggleKind {
    SkillMode,
    SkillBar,
    Effect,
    DropView,
    NameView,
    HpView,
    Observe,
    NewMove,
}

/// 根标记（显隐控制）
#[derive(Component)]
pub struct OptionWidget;

/// 关闭按钮
#[derive(Component)]
pub struct OptionClose;

/// 开/关按钮：**每个按钮自带**「设置=ON 时的常态帧 / 设置=OFF 时的常态帧 / 按下帧」。
///
/// C# 依据（`MainDialogs.cs`）：`OptionPanel_BeforeDraw` 每帧按 `Settings.X` 给两颗按钮
/// **各自**赋 `Index`（如 `SkillModeOn.Index = 452/450`、`SkillModeOff.Index = 453/455`，
/// `:2917-2926`；SkillBar/Effect/DropView/NameView/Observe 同形 `:2928-3002`；
/// HPView `464/462`+`465/467`；NewMove `853/851`+`848/850`），`PressedIndex` 在构造器里定死
/// （`:2574/2589/2603/2614/…`）。行标签（"SKILL MODE"/"SKILL BAR"/…）**烘在这几张原版美术里**
/// ——C# 与 `Client/Localization/*.json` 都没有这些字面量，本端也不画文字（owner 队列
/// `settings-english-labels` 的核对结论）。
#[derive(Component)]
pub struct OptionToggleBtn {
    pub kind: OptionToggleKind,
    /// true=该排左边的钮（C# `x=159` 那颗，如 `SkillModeOn`），false=右边那颗（`x=201`）
    pub is_on: bool,
    /// 本钮自己的三帧：`[设置=ON 时的常态帧, 设置=OFF 时的常态帧, 按下帧]`
    pub own_frames: [Handle<Image>; 3],
}

/// C# `OptionPanel_BeforeDraw` 的换帧规则（纯函数，便于门禁）：
/// 返回本钮在当前设置状态下的 `(常态帧, 按下帧)`。
///
/// 阳性对照：把两分支写反 → `option_button_frame_matches_csharp` 立即红。
pub fn toggle_button_frame(own: &[usize; 3], set_on: bool) -> (usize, usize) {
    (if set_on { own[0] } else { own[1] }, own[2])
}

/// 点击某档开关时原版发的**本地化提示**（`ChatDialog.ReceiveChat(..., ChatType.Hint)`）。
///
/// C# 依据 + 文案（逐字取 `Client/Localization/Chinese.json`）：
/// - 技能模式：`GameScene.ChangeSkillMode`（`GameScene.cs:890-902`）——ON ⇒ `SkillModeCtrl`
///   `"[技能模式：Ctrl]"`（`:517`）、OFF ⇒ `SkillModeTilde` `"[技能模式：~]"`（`:516`）
/// - HP/MP 显示：`MainDialogs.cs:2693-2712`——ON ⇒ `HpMpMode1` `"[HP/MP模式 1]"`（`:528`）、
///   OFF ⇒ `HpMpMode2` `"[HP/MP模式 2]"`（`:529`）
/// - 移动方式：`MainDialogs.cs:2765-2784`——ON ⇒ `NewMovementStyle` `"[新移动方式]"`（`:530`）、
///   OFF ⇒ `OldMovementStyle` `"[旧移动方式]"`（`:531`）
/// 其余五档（技能栏/特效/掉落显示/名称显示/观察）原版**不发**提示 ⇒ `None`。
pub fn toggle_hint(kind: OptionToggleKind, set_on: bool) -> Option<&'static str> {
    match kind {
        OptionToggleKind::SkillMode => Some(if set_on {
            "[技能模式：Ctrl]"
        } else {
            "[技能模式：~]"
        }),
        OptionToggleKind::HpView => Some(if set_on {
            "[HP/MP模式 1]"
        } else {
            "[HP/MP模式 2]"
        }),
        OptionToggleKind::NewMove => Some(if set_on {
            "[新移动方式]"
        } else {
            "[旧移动方式]"
        }),
        _ => None,
    }
}

/// 音量滑条（rect 为面板内相对点击区域 x/y/w/h；命中时加面板原点——面板可拖）
#[derive(Component)]
pub struct OptionBar {
    pub is_music: bool,
    pub rect: (f32, f32, f32, f32),
}

/// #2775：音量滑条的 Hint 文案（C# `MainDialogs.cs:2844/2880` `$"{Settings.Volume}%"`，
/// 0.0-1.0 内部音量 → 整数百分比）。
fn volume_hint_text(vol: f32) -> String {
    format!("{:.0}%", (vol * 100.0).round())
}

/// 面板初始原点兜底（Title[411] 259x354 居中：与 setup 的 fallback 尺寸一致）
/// C# `OptionDialog`（`MainDialogs.cs:2545-2546`）：`Index = 411; Library = Libraries.Title`
pub const PANEL: (LibraryName, usize) = (LibraryName::Title, 411);
/// 面板 art 实测尺寸（C# 用 art 尺寸做 `Location = Center`，`:2550`）
pub const PANEL_SIZE: (f32, f32) = (259.0, 354.0);
/// 关闭键相对面板：C# `Location = new Point(Size.Width - 26, 5)`（`:2559`，无 `Size` → art 24x21）
pub const CLOSE_REL: (f32, f32) = (233.0, 5.0);
/// 8 组开/关按钮：`On @(159,y)`、`Off @(201,y)`、`Size 36x17`（`MainDialogs.cs:2566-2780`）
pub const TOGGLE_ON_X: f32 = 159.0;
pub const TOGGLE_OFF_X: f32 = 201.0;
pub const TOGGLE_SIZE: (f32, f32) = (36.0, 17.0);
/// 行 y：技能模式/技能栏/特效/掉落/名称/血条 `68+25i`，观察 `271`、新移动 `296`（`:2570/2599/2621/2643/2665/2687/2789/2759`）
pub const TOGGLE_ROWS_Y: [(OptionToggleKind, f32); 8] = [
    (OptionToggleKind::SkillMode, 68.0),
    (OptionToggleKind::SkillBar, 93.0),
    (OptionToggleKind::Effect, 118.0),
    (OptionToggleKind::DropView, 143.0),
    (OptionToggleKind::NameView, 168.0),
    (OptionToggleKind::HpView, 193.0),
    (OptionToggleKind::Observe, 271.0),
    (OptionToggleKind::NewMove, 296.0),
];
/// 音量条 `SoundBar Prguse2[468]` @(159,225)/`MusicSoundBar` @(159,251)（`:2714-2751`）
pub const VOLUME_BAR_X: f32 = 159.0;
pub const VOLUME_BAR_Y: (f32, f32) = (225.0, 251.0);
/// 音量滑块 `Prguse[20]` @(155, 218/244)（`:2726-2751`）
pub const VOLUME_KNOB_X: f32 = 155.0;

/// 8 组开关：`(kind, 帧库, y, On 帧[normal,hover,pressed], Off 帧)`。
/// C# 依据（`MainDialogs.cs`）：`SkillMode 159/201@68`（`:2567-2589`）、`SkillBar @93`（`:2596-2614`）、
/// `Effect @118`（`:2618-2636`）、`DropView @143`（`:2640-2658`）、`NameView @168`（`:2662-2680`）、
/// `HPView @193`（`:2684-2706`，帧 `Prguse2[464/462/463]`）、`Observe @271`（`:2786+`）、
/// `NewMove @296`（`:2756-2778`，帧 `Title[853/851/853]`/`[848/850/850]`）
pub const TOGGLE_ROWS: [(OptionToggleKind, LibraryName, f32, [usize; 3], [usize; 3]); 8] = [
    (
        OptionToggleKind::SkillMode,
        LibraryName::Prguse2,
        68.0,
        [452, 450, 451],
        [453, 455, 454],
    ),
    (
        OptionToggleKind::SkillBar,
        LibraryName::Prguse2,
        93.0,
        [458, 456, 457],
        [459, 461, 460],
    ),
    (
        OptionToggleKind::Effect,
        LibraryName::Prguse2,
        118.0,
        [458, 456, 457],
        [459, 461, 460],
    ),
    (
        OptionToggleKind::DropView,
        LibraryName::Prguse2,
        143.0,
        [458, 456, 457],
        [459, 461, 460],
    ),
    (
        OptionToggleKind::NameView,
        LibraryName::Prguse2,
        168.0,
        [458, 456, 457],
        [459, 461, 460],
    ),
    (
        OptionToggleKind::HpView,
        LibraryName::Prguse2,
        193.0,
        [464, 462, 463],
        [465, 467, 466],
    ),
    (
        OptionToggleKind::Observe,
        LibraryName::Prguse2,
        271.0,
        [458, 456, 457],
        [459, 461, 460],
    ),
    (
        OptionToggleKind::NewMove,
        LibraryName::Title,
        296.0,
        [853, 851, 853],
        [848, 850, 850],
    ),
];

/// C# `Location = new Point((Settings.ScreenWidth - Size.Width) / 2, (ScreenHeight - Size.Height) / 2)`
/// （`MainDialogs.cs:2550`，**整除**）：`((1024-259)/2, (768-354)/2) = (382, 207)`。
/// 与 `center_origin(PANEL_SIZE)`（floor）同值，由 `ui_alignment::settings_dialog_aligned` 钉住。
pub const OPTION_ORIGIN: (f32, f32) = (382.0, 207.0);

/// 音量填充条（Prguse2[468] 部分裁剪）
#[derive(Component)]
pub struct OptionVolumeFill(pub bool);

/// 音量滑块（Prguse[20]）
#[derive(Component)]
pub struct OptionVolumeKnob(pub bool);

/// 设置项实际生效的种类（C# OptionDialog 开关）
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OptionViewKind {
    SkillBar,
    DropView,
    NameView,
    HpView,
}

/// 开关 → 是否显示（C# Settings：SkillBar / DropView / NameView / HPMPView）
pub fn view_should_show(kind: OptionViewKind, opt: &OptionState) -> bool {
    match kind {
        OptionViewKind::SkillBar => opt.skill_bar,
        OptionViewKind::DropView => opt.drop_view,
        OptionViewKind::NameView => opt.name_view,
        OptionViewKind::HpView => opt.hp_view,
    }
}

/// 设置项实际生效：技能栏/掉落物/名字/血条 显隐跟随开关（C# OptionDialog 立即生效）
fn option_view_system(
    opt: Res<OptionState>,
    mut skill_slots: Query<
        &mut Visibility,
        (
            With<crate::game::skills::SkillBarRoot>,
            Without<crate::actor::GroundItem>,
            Without<crate::actor::ActorNameLabel>,
            Without<crate::game::combat::HpBarBg>,
            Without<crate::game::combat::HpBarFill>,
        ),
    >,
    mut ground_items: Query<
        &mut Visibility,
        (
            With<crate::actor::GroundItem>,
            Without<crate::game::skills::SkillBarRoot>,
            Without<crate::actor::ActorNameLabel>,
            Without<crate::game::combat::HpBarBg>,
            Without<crate::game::combat::HpBarFill>,
        ),
    >,
    mut name_labels: Query<
        &mut Visibility,
        (
            With<crate::actor::ActorNameLabel>,
            Without<crate::game::skills::SkillBarRoot>,
            Without<crate::actor::GroundItem>,
            Without<crate::game::combat::HpBarBg>,
            Without<crate::game::combat::HpBarFill>,
        ),
    >,
    mut hp_bars: Query<
        &mut Visibility,
        (
            Or<(
                With<crate::game::combat::HpBarBg>,
                With<crate::game::combat::HpBarFill>,
            )>,
            Without<crate::game::skills::SkillBarRoot>,
            Without<crate::actor::GroundItem>,
            Without<crate::actor::ActorNameLabel>,
        ),
    >,
) {
    // C# Settings.Volume：同步全局音量（此后播放的音效立即生效）
    crate::game::sound::SOUND_VOLUME.store(
        (opt.sound_volume * 100.0).round() as u32,
        std::sync::atomic::Ordering::Relaxed,
    );
    let target = |show: bool| {
        if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        }
    };
    // C# GameScene.DialogProcess：SkillBar 开关显隐整个 SkillBarDialog（根实体隐藏，子控件随层级联动）
    let sb = target(view_should_show(OptionViewKind::SkillBar, &opt));
    for mut vis in &mut skill_slots {
        if *vis != sb {
            *vis = sb;
        }
    }
    let dv = target(view_should_show(OptionViewKind::DropView, &opt));
    for mut vis in &mut ground_items {
        if *vis != dv {
            *vis = dv;
        }
    }
    let nv = target(view_should_show(OptionViewKind::NameView, &opt));
    for mut vis in &mut name_labels {
        if *vis != nv {
            *vis = nv;
        }
    }
    let hv = target(view_should_show(OptionViewKind::HpView, &opt));
    for mut vis in &mut hp_bars {
        if *vis != hv {
            *vis = hv;
        }
    }
}

pub struct OptionPlugin;

impl Plugin for OptionPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(OptionState::load());
        app.add_systems(OnEnter(AppState::Game), spawn_option);
        app.add_systems(OnExit(AppState::Game), cleanup_option);
        app.add_systems(
            Update,
            (option_ui_system, option_view_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_option(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_option(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 面板 Title[411]（259x354），居中
    let (pw, ph) = match libs.0.get_image(PANEL.0, PANEL.1) {
        Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
        None => PANEL_SIZE,
    };
    // C# 用整除居中（`(ScreenWidth - Size.Width) / 2`），与 `center_origin` 同口径
    let (px, py) = super::center_origin(pw, ph);

    let Some(bg) = load_lib_image(&mut libs, &mut images, PANEL.0, PANEL.1) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, px, py, pw, ph, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::Settings), OptionWidget));

    commands.entity(panel).with_children(|p| {
        // 关闭按钮（Prguse2[360/361/362] @(pw-26,5)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, CLOSE_REL.0, CLOSE_REL.1, 24.0, 21.0, 10)
                .insert((OptionClose, CloseButton));
        }
        // 8 组开/关按钮（On at (159,y)，Off at (201,y)，36x17）
        for (kind, lib, y, on_btn, off_btn) in TOGGLE_ROWS {
            let (on, off) = load_frames(&mut libs, &mut images, lib, on_btn, off_btn);
            if let (Some(on), Some(off)) = (on, off) {
                spawn_icon_button(
                    p,
                    on[0].clone(),
                    on[1].clone(),
                    on[2].clone(),
                    159.0,
                    y,
                    36.0,
                    17.0,
                    10,
                )
                .insert(OptionToggleBtn {
                    kind,
                    is_on: true,
                    own_frames: on.clone(),
                });
                spawn_icon_button(
                    p,
                    off[0].clone(),
                    off[1].clone(),
                    off[2].clone(),
                    201.0,
                    y,
                    36.0,
                    17.0,
                    10,
                )
                .insert(OptionToggleBtn {
                    kind,
                    is_on: false,
                    own_frames: off,
                });
            }
        }
        // 音量滑条（Sound @(159,225)，Music @(159,251)；滑块 y=218/244）
        spawn_volume_bar(p, &mut libs, &mut images, 159.0, 225.0, 218.0, false);
        spawn_volume_bar(p, &mut libs, &mut images, 159.0, 251.0, 244.0, true);
    });
}

/// 加载开关按钮两态帧 [normal,hover,pressed]
fn load_frames(
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    lib: LibraryName,
    on_idx: [usize; 3],
    off_idx: [usize; 3],
) -> (Option<[Handle<Image>; 3]>, Option<[Handle<Image>; 3]>) {
    let mut on = [None, None, None];
    let mut off = [None, None, None];
    for i in 0..3 {
        on[i] = load_lib_image(libs, images, lib, on_idx[i]);
        off[i] = load_lib_image(libs, images, lib, off_idx[i]);
    }
    let on = if on.iter().all(|h| h.is_some()) {
        Some([
            on[0].clone().unwrap(),
            on[1].clone().unwrap(),
            on[2].clone().unwrap(),
        ])
    } else {
        None
    };
    let off = if off.iter().all(|h| h.is_some()) {
        Some([
            off[0].clone().unwrap(),
            off[1].clone().unwrap(),
            off[2].clone().unwrap(),
        ])
    } else {
        None
    };
    (on, off)
}

/// 音量滑条（bevy_ui）：bar 容器 + fill ImageNode + knob ImageNode
fn spawn_volume_bar(
    p: &mut bevy::ecs::hierarchy::ChildSpawnerCommands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    bar_x: f32,
    bar_y: f32,
    knob_y: f32,
    is_music: bool,
) {
    let Some(bar_tex) = load_lib_image(libs, images, LibraryName::Prguse2, 468) else {
        return;
    };
    let Some(knob_tex) = load_lib_image(libs, images, LibraryName::Prguse, 20) else {
        return;
    };
    spawn_container(p, bar_x, bar_y, 76.0, 19.0, 10)
        .insert(OptionBar {
            is_music,
            rect: (bar_x, bar_y, 76.0, 19.0),
        })
        // #2775：Hint 初值随实际音量，由 option_ui_system 每帧同步（C# SoundBar.Hint = N%）
        .insert(crate::ui::tooltip::UiHint {
            text: String::new(),
        })
        .with_children(|bc| {
            spawn_image(bc, bar_tex, 0.0, 0.0, 0.0, 19.0, 11).insert(OptionVolumeFill(is_music));
        });
    spawn_container(p, bar_x, knob_y, 8.0, 22.0, 10)
        .insert((ImageNode::new(knob_tex), OptionVolumeKnob(is_music)));
}

fn state_value(state: &OptionState, kind: OptionToggleKind) -> bool {
    match kind {
        OptionToggleKind::SkillMode => state.skill_mode_ctrl,
        OptionToggleKind::SkillBar => state.skill_bar,
        OptionToggleKind::Effect => state.effect,
        OptionToggleKind::DropView => state.drop_view,
        OptionToggleKind::NameView => state.name_view,
        OptionToggleKind::HpView => state.hp_view,
        OptionToggleKind::Observe => state.allow_observe,
        OptionToggleKind::NewMove => state.new_move,
    }
}

/// 显隐 + 按钮状态帧 + 开关点击 + 音量滑条
#[allow(clippy::too_many_arguments)]
fn option_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<OptionState>,
    close: Query<(Entity, &Interaction), With<OptionClose>>,
    mut toggles: Query<
        (Entity, &mut ImageButton, &Interaction, &OptionToggleBtn),
        Without<OptionClose>,
    >,
    mut widgets: Query<
        &mut Visibility,
        (
            With<OptionWidget>,
            Without<OptionVolumeFill>,
            Without<OptionVolumeKnob>,
        ),
    >,
    // B0001 互斥：fills/knobs 同写 Node——对称补 Without（Fill/Knob 实体互斥；
    // 再与下方只读 panel(Node) 互斥，写×读同样计入冲突）
    mut fills: Query<
        (&mut Node, &OptionVolumeFill),
        (Without<OptionVolumeKnob>, Without<OptionWidget>),
    >,
    mut knobs: Query<
        (&mut Node, &OptionVolumeKnob),
        (Without<OptionVolumeFill>, Without<OptionWidget>),
    >,
    mut bars: Query<(&OptionBar, &mut crate::ui::tooltip::UiHint)>,
    panel: Query<&Node, With<OptionWidget>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    // C# 三档开关点击会往聊天区发本地化提示（`ChatType.Hint`）
    mut chat: ResMut<crate::game::chat::ChatState>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    let open = mgr.is_open(DialogKind::Settings);
    for mut vis in widgets.iter_mut() {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        return;
    }
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::Settings);
        }
    }
    // 开关点击 + 状态帧刷新
    let mut changed = false;
    for (e, mut ib, inter, tg) in &mut toggles {
        if edge(e, inter, &mut prev_inter) {
            match tg.kind {
                // 左侧那颗 = "Ctrl"（C# `SkillModeOn.Click → ChangeSkillMode(false)`，
                // 而 `ChangeSkillMode(false)` 在 SkillMode=false 时把它置 **true**，
                // `GameScene.cs:890-902`）——旧实现写成 `!tg.is_on` 把方向弄反了。
                OptionToggleKind::SkillMode => state.skill_mode_ctrl = tg.is_on,
                OptionToggleKind::SkillBar => state.skill_bar = tg.is_on,
                OptionToggleKind::Effect => state.effect = tg.is_on,
                OptionToggleKind::DropView => state.drop_view = tg.is_on,
                OptionToggleKind::NameView => state.name_view = tg.is_on,
                OptionToggleKind::HpView => state.hp_view = tg.is_on,
                OptionToggleKind::Observe => state.allow_observe = tg.is_on,
                OptionToggleKind::NewMove => state.new_move = tg.is_on,
            }
            tracing::info!(
                "⚙️ 设置切换: {:?} -> {}",
                tg.kind,
                state_value(&state, tg.kind)
            );
            // C# 点击这三档会往聊天区发本地化提示（`ChatType.Hint`）
            if let Some(hint) = toggle_hint(tg.kind, state_value(&state, tg.kind)) {
                chat.add_line(
                    hint,
                    crate::game::chat::chat_color(mir2_shared::enums::ChatType::Hint),
                    crate::game::chat::ChatChannel::System,
                );
            }
            changed = true;
        }
        // 本钮自己的两态常态帧 + 按下帧（**不是**"设置=ON 时两钮共用"——那是旧实现的错法：
        // 同排两颗钮会显示同一张图，实机看到 SKILL BAR 排显示成 [on][on]）
        let sel = state_value(&state, tg.kind);
        let base = if sel {
            &tg.own_frames[0]
        } else {
            &tg.own_frames[1]
        };
        if ib.normal != *base {
            ib.normal = base.clone();
        }
        if ib.hover != *base {
            ib.hover = base.clone();
        }
        if ib.pressed != tg.own_frames[2] {
            ib.pressed = tg.own_frames[2].clone();
        }
    }
    // #2775：音量滑条 Hint（C# `MainDialogs.cs:2844/2880` `SoundBar.Hint = $"{Settings.Volume}%"`）
    // 必须在下面两处早退**之前**写：无光标环境（控制接口探针驱动）时 `cursor_position()` 恒 None，
    // 写在早退之后会让 Hint 永远空串（实机复现：命中 rect 正确但 text=""）。
    for (bar, mut hint) in &mut bars {
        let vol = if bar.is_music {
            state.music_volume
        } else {
            state.sound_volume
        };
        let text = volume_hint_text(vol);
        if hint.text != text {
            hint.text = text;
        }
    }
    // 音量滑条：点击设置音量（rect 为面板内相对坐标，命中前取面板原点——
    // 设置面板可拖动，生成期绝对坐标在拖后即成死区）
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let (ox, oy) = panel
        .single()
        .map(|n| crate::ui::theme::node_origin(n, OPTION_ORIGIN))
        .unwrap_or(OPTION_ORIGIN);
    for (bar, _) in &mut bars {
        let (rx, ry, rw, rh) = bar.rect;
        let (bx, by) = (ox + rx, oy + ry);
        if mouse.just_pressed(MouseButton::Left)
            && cursor.x >= bx
            && cursor.x <= bx + rw
            && cursor.y >= by
            && cursor.y <= by + rh
        {
            let vol = ((cursor.x - bx) / rw).clamp(0.0, 1.0);
            if bar.is_music {
                state.music_volume = vol;
            } else {
                state.sound_volume = vol;
            }
            tracing::info!(
                "⚙️ 音量: {} -> {:.0}%",
                if bar.is_music { "音乐" } else { "音效" },
                vol * 100.0
            );
            changed = true;
        }
    }
    if changed {
        state.save();
    }
    // 填充条 + 滑块位置
    for (mut node, fill) in &mut fills {
        let vol = if fill.0 {
            state.music_volume
        } else {
            state.sound_volume
        };
        let w = ((76.0 - 2.0) * vol).max(0.0);
        node.width = Val::Px(w);
    }
    for (mut node, knob) in &mut knobs {
        let vol = if knob.0 {
            state.music_volume
        } else {
            state.sound_volume
        };
        let fill = (76.0 - 2.0) * vol;
        node.left = Val::Px(159.0 + fill);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2775：音量滑条 Hint = 整数百分比（C# `$"{Settings.Volume}%"`）
    #[test]
    fn volume_hint_formats_percent() {
        assert_eq!(volume_hint_text(0.0), "0%");
        assert_eq!(volume_hint_text(0.3), "30%");
        assert_eq!(volume_hint_text(0.999), "100%");
        assert_eq!(volume_hint_text(1.0), "100%");
    }

    /// 门禁（owner 队列 `settings-english-labels`）：换帧必须**按按钮**而不是按状态。
    ///
    /// C# `OptionPanel_BeforeDraw`（`MainDialogs.cs:2917-2926`）给同排两颗按钮**各自**赋 `Index`：
    /// `SkillModeOn.Index = 452(ON)/450(OFF)`、`SkillModeOff.Index = 453(ON)/455(OFF)`，
    /// `PressedIndex` 固定 451/454（`:2574/2589`）。旧实现把「左钮三帧」当成「设置=ON 时两钮共用」，
    /// 于是同排显示同一张图（实机 SKILL BAR 排显示成 `[on][on]`）。
    ///
    /// 阳性对照：把 [`toggle_button_frame`] 的两个分支写反 → 第 1/2 条断言立即红。
    #[test]
    fn option_button_frame_matches_csharp() {
        // 左钮（SkillModeOn）：C# 452(ON)/450(OFF)，按下 451
        let left = [452usize, 450, 451];
        assert_eq!(toggle_button_frame(&left, true), (452, 451));
        assert_eq!(toggle_button_frame(&left, false), (450, 451));
        // 右钮（SkillModeOff）：453(ON)/455(OFF)，按下 454 —— 与左钮**不同帧**
        let right = [453usize, 455, 454];
        assert_eq!(toggle_button_frame(&right, true), (453, 454));
        assert_eq!(toggle_button_frame(&right, false), (455, 454));
        assert_ne!(
            toggle_button_frame(&left, true).0,
            toggle_button_frame(&right, true).0,
            "同一状态下两颗按钮不得显示同一帧（这正是旧实现的 bug）"
        );
        // 表里 8 排的取帧与 C# 常量一致（抽 3 排核对：技能栏/HP-MP/移动方式）
        let row = |kind: OptionToggleKind| {
            TOGGLE_ROWS
                .iter()
                .find(|r| r.0 == kind)
                .expect("行必须存在")
        };
        let bar = row(OptionToggleKind::SkillBar);
        assert_eq!((bar.3, bar.4), ([458, 456, 457], [459, 461, 460]));
        let hp = row(OptionToggleKind::HpView);
        assert_eq!((hp.3, hp.4), ([464, 462, 463], [465, 467, 466]));
        let mv = row(OptionToggleKind::NewMove);
        assert_eq!((mv.3, mv.4), ([853, 851, 853], [848, 850, 850]));
    }

    /// 门禁：三档开关的本地化提示（C# `ChatType.Hint`）——文案逐字取
    /// `Client/Localization/Chinese.json:516/517/528/529/530/531`；
    /// 其余五档 C# 不发提示。
    ///
    /// 阳性对照：把 SkillMode 的两个文案对调 → 第 1 条断言立即红。
    #[test]
    fn option_toggle_hint_matches_csharp() {
        assert_eq!(
            toggle_hint(OptionToggleKind::SkillMode, true),
            Some("[技能模式：Ctrl]")
        );
        assert_eq!(
            toggle_hint(OptionToggleKind::SkillMode, false),
            Some("[技能模式：~]")
        );
        assert_eq!(
            toggle_hint(OptionToggleKind::HpView, true),
            Some("[HP/MP模式 1]")
        );
        assert_eq!(
            toggle_hint(OptionToggleKind::HpView, false),
            Some("[HP/MP模式 2]")
        );
        assert_eq!(
            toggle_hint(OptionToggleKind::NewMove, true),
            Some("[新移动方式]")
        );
        assert_eq!(
            toggle_hint(OptionToggleKind::NewMove, false),
            Some("[旧移动方式]")
        );
        for kind in [
            OptionToggleKind::SkillBar,
            OptionToggleKind::Effect,
            OptionToggleKind::DropView,
            OptionToggleKind::NameView,
            OptionToggleKind::Observe,
        ] {
            assert_eq!(toggle_hint(kind, true), None, "{kind:?} 原版不发提示");
            assert_eq!(toggle_hint(kind, false), None, "{kind:?} 原版不发提示");
        }
    }

    #[test]
    fn test_from_ini_empty_uses_defaults() {
        let s = OptionState::from_ini("");
        assert!(s.skill_mode_ctrl);
        assert!(s.skill_bar);
        assert_eq!(s.sound_volume, 0.8);
        assert_eq!(s.music_volume, 0.6);
        assert!(!s.allow_observe);
        assert!(!s.mode_view);
    }

    #[test]
    fn test_from_ini_values() {
        let content = "[Sound]\nVolume=30\nMusic=70\n\n[Game]\nSkillMode=false\nSkillBar=false\nEffect=true\nDropView=false\nNameView=false\nHPMPView=false\nAllowObserve=true\nNewMove=false\nModeView=true\n";
        let s = OptionState::from_ini(content);
        assert_eq!(s.sound_volume, 0.3);
        assert_eq!(s.music_volume, 0.7);
        assert!(!s.skill_mode_ctrl);
        assert!(!s.skill_bar);
        assert!(s.effect);
        assert!(!s.drop_view);
        assert!(!s.name_view);
        assert!(!s.hp_view);
        assert!(s.allow_observe);
        assert!(!s.new_move);
        assert!(s.mode_view);
    }

    #[test]
    fn test_to_ini_roundtrip() {
        let mut s = OptionState::default();
        s.skill_mode_ctrl = false;
        s.skill_bar = false;
        s.effect = false;
        s.drop_view = false;
        s.name_view = true;
        s.hp_view = false;
        s.allow_observe = true;
        s.new_move = false;
        s.sound_volume = 0.45;
        s.music_volume = 1.0;
        s.mode_view = true;
        let parsed = OptionState::from_ini(&s.to_ini());
        assert_eq!(parsed.skill_mode_ctrl, false);
        assert_eq!(parsed.skill_bar, false);
        assert_eq!(parsed.effect, false);
        assert_eq!(parsed.drop_view, false);
        assert_eq!(parsed.name_view, true);
        assert_eq!(parsed.hp_view, false);
        assert_eq!(parsed.allow_observe, true);
        assert_eq!(parsed.new_move, false); // 与 mode_view=true 区分，防相邻槽位/参数互换不察
        assert_eq!(parsed.sound_volume, 0.45); // 45 存整数往返
        assert_eq!(parsed.music_volume, 1.0);
        assert_eq!(parsed.mode_view, true); // C# ModeView 仅 INI 持久化往返
    }

    #[test]
    fn test_ini_helpers() {
        let content = "[Sound]\nVolume=50\nMusic=100\n";
        assert_eq!(
            crate::game::dialogs::settings_file::ini_bool(content, "Sound", "Missing", true),
            true
        );
        assert_eq!(
            crate::game::dialogs::settings_file::ini_percent(content, "Sound", "Volume", 0.0),
            0.5
        );
        assert_eq!(
            crate::game::dialogs::settings_file::ini_percent(content, "Sound", "Music", 0.0),
            1.0
        );
        assert_eq!(
            crate::game::dialogs::settings_file::ini_percent(content, "Sound", "Missing", 0.2),
            0.2
        );
    }
}
#[cfg(test)]
mod view_tests {
    use super::*;

    #[test]
    fn view_should_show_follows_option() {
        let mut opt = OptionState::default();
        assert!(view_should_show(OptionViewKind::SkillBar, &opt));
        assert!(view_should_show(OptionViewKind::DropView, &opt));
        assert!(view_should_show(OptionViewKind::NameView, &opt));
        assert!(view_should_show(OptionViewKind::HpView, &opt));
        opt.skill_bar = false;
        opt.drop_view = false;
        opt.name_view = false;
        opt.hp_view = false;
        assert!(!view_should_show(OptionViewKind::SkillBar, &opt));
        assert!(!view_should_show(OptionViewKind::DropView, &opt));
        assert!(!view_should_show(OptionViewKind::NameView, &opt));
        assert!(!view_should_show(OptionViewKind::HpView, &opt));
    }
}
