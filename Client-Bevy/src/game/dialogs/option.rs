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
    ImageButton,
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
        s.sound_volume = crate::game::dialogs::settings_file::ini_percent(content, "Sound", "Volume", s.sound_volume);
        s.music_volume = crate::game::dialogs::settings_file::ini_percent(content, "Sound", "Music", s.music_volume);
        s.skill_mode_ctrl = crate::game::dialogs::settings_file::ini_bool(content, "Game", "SkillMode", s.skill_mode_ctrl);
        s.skill_bar = crate::game::dialogs::settings_file::ini_bool(content, "Game", "SkillBar", s.skill_bar);
        s.effect = crate::game::dialogs::settings_file::ini_bool(content, "Game", "Effect", s.effect);
        s.drop_view = crate::game::dialogs::settings_file::ini_bool(content, "Game", "DropView", s.drop_view);
        s.name_view = crate::game::dialogs::settings_file::ini_bool(content, "Game", "NameView", s.name_view);
        s.hp_view = crate::game::dialogs::settings_file::ini_bool(content, "Game", "HPMPView", s.hp_view);
        s.allow_observe = crate::game::dialogs::settings_file::ini_bool(content, "Game", "AllowObserve", s.allow_observe);
        s.new_move = crate::game::dialogs::settings_file::ini_bool(content, "Game", "NewMove", s.new_move);
        s.mode_view = crate::game::dialogs::settings_file::ini_bool(content, "Game", "ModeView", s.mode_view);
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

/// 开/关按钮：持有“设置开/关”两套三态帧，每帧按当前状态切换
#[derive(Component)]
pub struct OptionToggleBtn {
    pub kind: OptionToggleKind,
    /// true=“开”按钮，false=“关”按钮
    pub is_on: bool,
    /// 设置=ON 时的帧 [normal, hover, pressed]
    pub frames_on: [Handle<Image>; 3],
    /// 设置=OFF 时的帧 [normal, hover, pressed]
    pub frames_off: [Handle<Image>; 3],
}

/// 音量滑条（rect 为点击区域，x/y/w/h 屏幕坐标）
#[derive(Component)]
pub struct OptionBar {
    pub is_music: bool,
    pub rect: (f32, f32, f32, f32),
}

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
            Or<(With<crate::game::combat::HpBarBg>, With<crate::game::combat::HpBarFill>)>,
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
    let target = |show: bool| if show { Visibility::Visible } else { Visibility::Hidden };
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
    let (pw, ph) = match libs.0.get_image(LibraryName::Title, 411) {
        Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
        None => (250.0, 330.0),
    };
    let px = (1024.0 - pw) / 2.0;
    let py = (768.0 - ph) / 2.0;

    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 411) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, px, py, pw, ph, 30);
    commands.entity(panel).insert((DialogRoot(DialogKind::Settings), OptionWidget));

    commands.entity(panel).with_children(|p| {
        // 关闭按钮（Prguse2[360/361/362] @(pw-26,5)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, pw - 26.0, 5.0, 24.0, 21.0, 10).insert(OptionClose);
        }
        // 8 组开/关按钮（On at (159,y)，Off at (201,y)，36x17）
        let rows: [(OptionToggleKind, LibraryName, f32, [usize; 3], [usize; 3]); 8] = [
            (OptionToggleKind::SkillMode, LibraryName::Prguse2, 68.0, [452, 450, 451], [453, 455, 454]),
            (OptionToggleKind::SkillBar, LibraryName::Prguse2, 93.0, [458, 456, 457], [459, 461, 460]),
            (OptionToggleKind::Effect, LibraryName::Prguse2, 118.0, [458, 456, 457], [459, 461, 460]),
            (OptionToggleKind::DropView, LibraryName::Prguse2, 143.0, [458, 456, 457], [459, 461, 460]),
            (OptionToggleKind::NameView, LibraryName::Prguse2, 168.0, [458, 456, 457], [459, 461, 460]),
            (OptionToggleKind::HpView, LibraryName::Prguse2, 193.0, [464, 462, 463], [465, 467, 466]),
            (OptionToggleKind::Observe, LibraryName::Prguse2, 271.0, [458, 456, 457], [459, 461, 460]),
            (OptionToggleKind::NewMove, LibraryName::Title, 296.0, [853, 851, 853], [848, 850, 850]),
        ];
        for (kind, lib, y, on_btn, off_btn) in rows {
            let (on, off) = load_frames(&mut libs, &mut images, lib, on_btn, off_btn);
            if let (Some(on), Some(off)) = (on, off) {
                spawn_icon_button(p, on[0].clone(), on[1].clone(), on[2].clone(), 159.0, y, 36.0, 17.0, 10)
                    .insert(OptionToggleBtn { kind, is_on: true, frames_on: on.clone(), frames_off: off.clone() });
                spawn_icon_button(p, off[0].clone(), off[1].clone(), off[2].clone(), 201.0, y, 36.0, 17.0, 10)
                    .insert(OptionToggleBtn { kind, is_on: false, frames_on: on, frames_off: off });
            }
        }
        // 音量滑条（Sound @(159,225)，Music @(159,251)；滑块 y=218/244）
        spawn_volume_bar(p, &mut libs, &mut images, px, py, 159.0, 225.0, 218.0, false);
        spawn_volume_bar(p, &mut libs, &mut images, px, py, 159.0, 251.0, 244.0, true);
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
        Some([on[0].clone().unwrap(), on[1].clone().unwrap(), on[2].clone().unwrap()])
    } else {
        None
    };
    let off = if off.iter().all(|h| h.is_some()) {
        Some([off[0].clone().unwrap(), off[1].clone().unwrap(), off[2].clone().unwrap()])
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
    px: f32,
    py: f32,
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
            rect: (px + bar_x, py + bar_y, 76.0, 19.0),
        })
        .with_children(|bc| {
            spawn_image(bc, bar_tex, 0.0, 0.0, 0.0, 19.0, 11)
                .insert(OptionVolumeFill(is_music));
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
    mut toggles: Query<(Entity, &mut ImageButton, &Interaction, &OptionToggleBtn), Without<OptionClose>>,
    mut widgets: Query<&mut Visibility, (With<OptionWidget>, Without<OptionVolumeFill>, Without<OptionVolumeKnob>)>,
    // B0001 互斥：fills/knobs 同写 Node——对称补 Without（Fill/Knob 实体互斥）
    mut fills: Query<(&mut Node, &OptionVolumeFill), Without<OptionVolumeKnob>>,
    mut knobs: Query<(&mut Node, &OptionVolumeKnob), Without<OptionVolumeFill>>,
    bars: Query<&OptionBar>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
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
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
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
                OptionToggleKind::SkillMode => state.skill_mode_ctrl = !tg.is_on,
                OptionToggleKind::SkillBar => state.skill_bar = tg.is_on,
                OptionToggleKind::Effect => state.effect = tg.is_on,
                OptionToggleKind::DropView => state.drop_view = tg.is_on,
                OptionToggleKind::NameView => state.name_view = tg.is_on,
                OptionToggleKind::HpView => state.hp_view = tg.is_on,
                OptionToggleKind::Observe => state.allow_observe = tg.is_on,
                OptionToggleKind::NewMove => state.new_move = tg.is_on,
            }
            tracing::info!("⚙️ 设置切换: {:?} -> {}", tg.kind, state_value(&state, tg.kind));
            changed = true;
        }
        let sel = state_value(&state, tg.kind);
        let f = if sel { &tg.frames_on } else { &tg.frames_off };
        if ib.normal != f[0] {
            ib.normal = f[0].clone();
        }
        if ib.hover != f[1] {
            ib.hover = f[1].clone();
        }
        if ib.pressed != f[2] {
            ib.pressed = f[2].clone();
        }
    }
    // 音量滑条：点击设置音量
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    for bar in &bars {
        let (bx, by, bw, bh) = bar.rect;
        if mouse.just_pressed(MouseButton::Left)
            && cursor.x >= bx && cursor.x <= bx + bw
            && cursor.y >= by && cursor.y <= by + bh
        {
            let vol = ((cursor.x - bx) / bw).clamp(0.0, 1.0);
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
        assert_eq!(crate::game::dialogs::settings_file::ini_bool(content, "Sound", "Missing", true), true);
        assert_eq!(crate::game::dialogs::settings_file::ini_percent(content, "Sound", "Volume", 0.0), 0.5);
        assert_eq!(crate::game::dialogs::settings_file::ini_percent(content, "Sound", "Music", 0.0), 1.0);
        assert_eq!(crate::game::dialogs::settings_file::ini_percent(content, "Sound", "Missing", 0.2), 0.2);
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



