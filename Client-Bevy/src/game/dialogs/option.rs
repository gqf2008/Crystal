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
use crate::ui::sprite_ui::{
    spawn_ui_sprite, ui_button_system, ui_image, UiButton, UiImageCache,
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
        }
    }
}

/// 设置持久化文件（与 C# Settings.cs：`.\Mir2Config.ini` 同路径同语义）
const SETTINGS_PATH: &str = "./Mir2Config.ini";

impl OptionState {
    /// 从 INI 文本解析（C# InIReader；音量按 0-100 存储 ↔ 0.0-1.0）
    pub fn from_ini(content: &str) -> Self {
        let mut s = Self::default();
        s.sound_volume = ini_percent(content, "Sound", "Volume", s.sound_volume);
        s.music_volume = ini_percent(content, "Sound", "Music", s.music_volume);
        s.skill_mode_ctrl = ini_bool(content, "Game", "SkillMode", s.skill_mode_ctrl);
        s.skill_bar = ini_bool(content, "Game", "SkillBar", s.skill_bar);
        s.effect = ini_bool(content, "Game", "Effect", s.effect);
        s.drop_view = ini_bool(content, "Game", "DropView", s.drop_view);
        s.name_view = ini_bool(content, "Game", "NameView", s.name_view);
        s.hp_view = ini_bool(content, "Game", "HPMPView", s.hp_view);
        s.allow_observe = ini_bool(content, "Game", "AllowObserve", s.allow_observe);
        s.new_move = ini_bool(content, "Game", "NewMove", s.new_move);
        s
    }

    /// 序列化为 INI 文本（对齐 C# Settings.Save 的 [Sound]/[Game] 段）
    pub fn to_ini(&self) -> String {
        let pct = |v: f32| ((v * 100.0).round() as i32).clamp(0, 100);
        format!(
            "[Sound]\nVolume={}\nMusic={}\n\n[Game]\nSkillMode={}\nSkillBar={}\nEffect={}\nDropView={}\nNameView={}\nHPMPView={}\nAllowObserve={}\nNewMove={}\n",
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
        )
    }

    /// 启动时加载（C# Settings.Load；文件不存在用默认值）
    pub fn load() -> Self {
        let content = std::fs::read_to_string(SETTINGS_PATH).unwrap_or_default();
        Self::from_ini(&content)
    }

    /// 保存（C# Settings.Save；CMain 退出/设置变更时调用）
    pub fn save(&self) {
        let _ = std::fs::write(SETTINGS_PATH, self.to_ini());
        tracing::debug!("⚙️ 设置已保存到 {}", SETTINGS_PATH);
    }
}

/// 读取 INI 布尔值（缺省/非法回退 default）
fn ini_bool(content: &str, section: &str, key: &str, default: bool) -> bool {
    ini_str(content, section, key)
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(default)
}

/// 读取 INI 百分比音量（0-100 → 0.0-1.0）
fn ini_percent(content: &str, section: &str, key: &str, default: f32) -> f32 {
    ini_str(content, section, key)
        .and_then(|v| v.parse::<f32>().ok())
        .map(|v| (v / 100.0).clamp(0.0, 1.0))
        .unwrap_or(default)
}

/// 读取 INI 某段某 key 的原始值
fn ini_str<'a>(content: &'a str, section: &str, key: &str) -> Option<&'a str> {
    let mut cur = "";
    for line in content.lines() {
        let l = line.trim();
        if l.starts_with('[') && l.ends_with(']') {
            cur = &l[1..l.len() - 1];
            continue;
        }
        if cur.eq_ignore_ascii_case(section) {
            if let Some(eq) = l.find('=') {
                if l[..eq].trim().eq_ignore_ascii_case(key) {
                    return Some(l[eq + 1..].trim());
                }
            }
        }
    }
    None
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

pub struct OptionPlugin;

impl Plugin for OptionPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(OptionState::load());
        app.add_systems(OnEnter(AppState::Game), spawn_option);
        app.add_systems(OnExit(AppState::Game), cleanup_option);
        app.add_systems(
            Update,
            (option_ui_system, ui_button_system)
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

/// 生成一个开/关按钮
/// sel_idx = 设置 ON 时的 [normal, hover, pressed]
/// unsel_idx = 设置 OFF 时的 [normal, hover, pressed]
#[allow(clippy::too_many_arguments)]
fn spawn_toggle_button(
    commands: &mut Commands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
    lib: LibraryName,
    sel_idx: [usize; 3],
    unsel_idx: [usize; 3],
    x: f32,
    y: f32,
    kind: OptionToggleKind,
    is_on: bool,
) -> Option<Entity> {
    let mut sel = [None, None, None];
    let mut unsel = [None, None, None];
    for (i, idx) in sel_idx.iter().enumerate() {
        sel[i] = ui_image(libs, images, cache, lib, *idx);
    }
    for (i, idx) in unsel_idx.iter().enumerate() {
        unsel[i] = ui_image(libs, images, cache, lib, *idx);
    }
    if sel.iter().any(|h| h.is_none()) || unsel.iter().any(|h| h.is_none()) {
        return None;
    }
    let sel = [
        sel[0].clone().unwrap(),
        sel[1].clone().unwrap(),
        sel[2].clone().unwrap(),
    ];
    let unsel = [
        unsel[0].clone().unwrap(),
        unsel[1].clone().unwrap(),
        unsel[2].clone().unwrap(),
    ];
    let normal = if is_on { sel[0].clone() } else { unsel[0].clone() };
    let e = spawn_ui_sprite(commands, normal.clone(), x, y, 7.0, 1.0);
    commands.entity(e).insert((
        UiButton {
            rect: (x, y, 36.0, 17.0),
            clicked: false,
        },
        crate::ui::sprite_ui::ButtonFrames {
            normal,
            hover: if is_on { sel[1].clone() } else { unsel[1].clone() },
            pressed: if is_on { sel[2].clone() } else { unsel[2].clone() },
        },
        OptionToggleBtn {
            kind,
            is_on,
            frames_on: sel,
            frames_off: unsel,
        },
        OptionWidget,
        DialogRoot(DialogKind::Settings),
    ));
    Some(e)
}

/// 生成音量滑条（填充 + 滑块 + 点击区域）
fn spawn_volume_bar(
    commands: &mut Commands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
    bar_x: f32,
    bar_y: f32,
    knob_y: f32,
    is_music: bool,
) {
    let Some(bar_info) = libs.0.get_image(LibraryName::Prguse2, 468) else {
        return;
    };
    let bar_w = bar_info.width.max(0) as f32;
    let bar_h = bar_info.height.max(0) as f32;
    let Some(bar_tex) = ui_image(libs, images, cache, LibraryName::Prguse2, 468) else {
        return;
    };
    let Some(knob_tex) = ui_image(libs, images, cache, LibraryName::Prguse, 20) else {
        return;
    };
    // 填充条（部分裁剪，模拟 C# Draw(section)）
    let fill = spawn_ui_sprite(commands, bar_tex, bar_x, bar_y, 7.0, 1.0);
    commands.entity(fill).insert((
        Sprite {
            rect: Some(Rect::new(0.0, 0.0, 0.0, bar_h)),
            custom_size: Some(Vec2::new(0.0, bar_h)),
            ..default()
        },
        OptionVolumeFill(is_music),
        OptionBar {
            is_music,
            rect: (bar_x, bar_y, bar_w, bar_h),
        },
        OptionWidget,
        DialogRoot(DialogKind::Settings),
    ));
    // 滑块（C#：VolumeBar at (159+fill, 218/244)）
    let knob = spawn_ui_sprite(commands, knob_tex, bar_x, knob_y, 8.0, 1.0);
    commands.entity(knob).insert((
        OptionVolumeKnob(is_music),
        OptionWidget,
        DialogRoot(DialogKind::Settings),
    ));
}

fn spawn_option(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
) {
    libs.0.ensure_initialized();

    // 面板 Title[411]（259x354），居中
    let (pw, ph) = match libs.0.get_image(LibraryName::Title, 411) {
        Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
        None => (250.0, 330.0),
    };
    let px = (1024.0 - pw) / 2.0;
    let py = (768.0 - ph) / 2.0;

    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 411) {
        let e = spawn_ui_sprite(&mut commands, h, px, py, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Settings),
            OptionWidget,
            Visibility::Hidden,
        ));
    }

    // 关闭按钮（C#：Prguse2[360/361/362] at (Width-26, 5)，纹理 24x21）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        px + pw - 26.0, py + 5.0, 7.0, 24.0, 21.0,
    ) {
        commands.entity(e).insert((
            OptionClose,
            DialogRoot(DialogKind::Settings),
            OptionWidget,
        ));
    }

    // 8 组开/关按钮（C# 布局：On at (159,y)，Off at (201,y)，36x17）
    // 每行：[ON 态 normal, OFF 态 normal, pressed] × 开按钮 / 关按钮
    // 索引来自 C# BeforeDraw 的状态切换逻辑
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
        spawn_toggle_button(
            &mut commands, &mut libs, &mut images, &mut cache,
            lib, on_btn, on_btn, px + 159.0, py + y, kind, true,
        );
        spawn_toggle_button(
            &mut commands, &mut libs, &mut images, &mut cache,
            lib, off_btn, off_btn, px + 201.0, py + y, kind, false,
        );
    }

    // 音量滑条（C#：SoundBar at (159,225)/(159,251)，滑块 at (155,218)/(155,244)）
    spawn_volume_bar(
        &mut commands, &mut libs, &mut images, &mut cache,
        px + 159.0, py + 225.0, py + 218.0, false,
    );
    spawn_volume_bar(
        &mut commands, &mut libs, &mut images, &mut cache,
        px + 159.0, py + 251.0, py + 244.0, true,
    );
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
    close: Query<&UiButton, With<OptionClose>>,
    mut toggles: Query<(&UiButton, &OptionToggleBtn, &mut crate::ui::sprite_ui::ButtonFrames)>,
    mut widgets: Query<&mut Visibility, With<OptionWidget>>,
    mut fills: Query<(&mut Sprite, &OptionVolumeFill)>,
    mut knobs: Query<(&mut Transform, &OptionVolumeKnob)>,
    bars: Query<&OptionBar>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
) {
    let open = mgr.is_open(DialogKind::Settings);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }

    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Settings);
        }
    }

    // 开关点击 + 状态帧刷新（等效 C# BeforeDraw 按状态换 Index）
    let mut changed = false;
    for (btn, tg, mut frames) in &mut toggles {
        if btn.clicked {
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
            tracing::info!(
                "⚙️ 设置切换: {:?} -> {}",
                tg.kind,
                state_value(&state, tg.kind)
            );
            changed = true;
        }
        let sel = state_value(&state, tg.kind);
        let f = if sel { &tg.frames_on } else { &tg.frames_off };
        if frames.normal != f[0] {
            frames.normal = f[0].clone();
        }
        if frames.hover != f[1] {
            frames.hover = f[1].clone();
        }
        if frames.pressed != f[2] {
            frames.pressed = f[2].clone();
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
    // 设置变更即保存（C# Settings.Save）
    if changed {
        state.save();
    }

    // 填充条 + 滑块位置（C#：fill=(Width-2)*percent，knob at (159+fill, 218/244)）
    for (mut sprite, fill) in &mut fills {
        let vol = if fill.0 {
            state.music_volume
        } else {
            state.sound_volume
        };
        let (_, _, bw, bh) = bars
            .iter()
            .find(|b| b.is_music == fill.0)
            .map(|b| b.rect)
            .unwrap_or((0.0, 0.0, 100.0, 10.0));
        let w = ((bw - 2.0) * vol).max(0.0);
        sprite.rect = Some(Rect::new(0.0, 0.0, w, bh));
        sprite.custom_size = Some(Vec2::new(w, bh));
    }
    for (mut tf, knob) in &mut knobs {
        let vol = if knob.0 {
            state.music_volume
        } else {
            state.sound_volume
        };
        let Some(bar) = bars.iter().find(|b| b.is_music == knob.0) else {
            continue;
        };
        let (bx, _, bw, _) = bar.rect;
        let fill = (bw - 2.0) * vol;
        tf.translation.x = bx + fill;
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
    }

    #[test]
    fn test_from_ini_values() {
        let content = "[Sound]\nVolume=30\nMusic=70\n\n[Game]\nSkillMode=false\nSkillBar=false\nEffect=true\nDropView=false\nNameView=false\nHPMPView=false\nAllowObserve=true\nNewMove=false\n";
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
    }

    #[test]
    fn test_to_ini_roundtrip() {
        let mut s = OptionState::default();
        s.skill_mode_ctrl = false;
        s.effect = false;
        s.sound_volume = 0.45;
        s.music_volume = 1.0;
        let parsed = OptionState::from_ini(&s.to_ini());
        assert_eq!(parsed.skill_mode_ctrl, false);
        assert_eq!(parsed.effect, false);
        assert_eq!(parsed.sound_volume, 0.45); // 45 存整数往返
        assert_eq!(parsed.music_volume, 1.0);
        assert_eq!(parsed.name_view, true);
    }

    #[test]
    fn test_ini_helpers() {
        let content = "[Sound]\nVolume=50\nMusic=100\n";
        assert_eq!(ini_bool(content, "Sound", "Missing", true), true);
        assert_eq!(ini_percent(content, "Sound", "Volume", 0.0), 0.5);
        assert_eq!(ini_percent(content, "Sound", "Music", 0.0), 1.0);
        assert_eq!(ini_percent(content, "Sound", "Missing", 0.2), 0.2);
    }
}
