// ============================================================================
// 键位设置对话框（M52）
// 参考：C# KeyboardLayoutDialog（Client/MirScenes/Dialogs/KeyboardLayoutDialog.cs）
//   - 面板 Title[119]（512x430）居中；标题“键位设置” (135,34)
//   - 关闭按钮 Prguse2[360/361/362] (489,3)
//   - 滚动：Prguse2[197/198/199] 上 (491,88)、Prguse2[207/208/209] 下 (491,363)、
//     位置条 Prguse2[205/206] (491,101)
//   - 重置按钮 Title[120/121/122] (30,400)；严格规则复选框 Prguse[1346/1347] (105,406)
//   - 行区 x=20 起：组标题（30px）+ 绑定行（18px），点击绑定行 → 等待按键 → 重绑
// 纯客户端：绑定仅保存在本地 KeyboardState（无服务端依赖）
// ============================================================================

use bevy::prelude::*;

use std::fs;

use crate::game::dialogs::settings_file;
use crate::network::NetConnection;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};
use crate::ui::controls::{spawn_checkbox, CheckBox};

/// 单个键位绑定（动作 + 组 + 当前键）
#[derive(Clone)]
pub struct KeyBinding {
    pub action: &'static str,
    pub group: &'static str,
    pub key: KeyCode,
}

impl KeyBinding {
    fn new(action: &'static str, group: &'static str, key: KeyCode) -> Self {
        Self { action, group, key }
    }
}

/// 键位显示名
pub fn key_name(key: KeyCode) -> String {
    match key {
        KeyCode::Space => "空格".to_string(),
        KeyCode::Enter => "回车".to_string(),
        KeyCode::Escape => "Esc".to_string(),
        KeyCode::AltLeft => "左Alt".to_string(),
        KeyCode::AltRight => "右Alt".to_string(),
        KeyCode::ControlLeft => "左Ctrl".to_string(),
        KeyCode::ControlRight => "右Ctrl".to_string(),
        KeyCode::ShiftLeft => "左Shift".to_string(),
        KeyCode::ShiftRight => "右Shift".to_string(),
        KeyCode::KeyW => "W".to_string(),
        KeyCode::KeyA => "A".to_string(),
        KeyCode::KeyS => "S".to_string(),
        KeyCode::KeyD => "D".to_string(),
        KeyCode::KeyQ => "Q".to_string(),
        KeyCode::KeyB => "B".to_string(),
        KeyCode::KeyC => "C".to_string(),
        KeyCode::KeyK => "K".to_string(),
        KeyCode::KeyG => "G".to_string(),
        KeyCode::KeyM => "M".to_string(),
        KeyCode::KeyH => "H".to_string(),
        KeyCode::KeyO => "O".to_string(),
        _ => format!("{:?}", key).trim_start_matches("Key").to_string(),
    }
}

/// KeyBinds.ini 路径（对齐 C# KeyBindSettings：.\KeyBinds.ini）
const KEYBINDS_PATH: &str = "./KeyBinds.ini";

/// KeyCode Debug 名 → KeyCode（覆盖默认键位 + 常用键；未知返回 None）
fn key_code_from_name(name: &str) -> Option<KeyCode> {
    use KeyCode::*;
    Some(match name {
        "Space" => Space,
        "Enter" => Enter,
        "Escape" => Escape,
        "Tab" => Tab,
        "Backspace" => Backspace,
        "AltLeft" => AltLeft,
        "AltRight" => AltRight,
        "ControlLeft" => ControlLeft,
        "ControlRight" => ControlRight,
        "ShiftLeft" => ShiftLeft,
        "ShiftRight" => ShiftRight,
        "KeyW" => KeyW,
        "KeyA" => KeyA,
        "KeyS" => KeyS,
        "KeyD" => KeyD,
        "KeyQ" => KeyQ,
        "KeyB" => KeyB,
        "KeyC" => KeyC,
        "KeyK" => KeyK,
        "KeyG" => KeyG,
        "KeyM" => KeyM,
        "KeyH" => KeyH,
        "KeyO" => KeyO,
        "KeyI" => KeyI,
        "KeyV" => KeyV,
        "F1" => F1,
        "F2" => F2,
        "F3" => F3,
        "F4" => F4,
        "F5" => F5,
        "F6" => F6,
        "F7" => F7,
        "F8" => F8,
        "F9" => F9,
        "F10" => F10,
        "F11" => F11,
        "F12" => F12,
        "ArrowUp" => ArrowUp,
        "ArrowDown" => ArrowDown,
        "ArrowLeft" => ArrowLeft,
        "ArrowRight" => ArrowRight,
        _ => return None,
    })
}

/// 序列化绑定 → KeyBinds.ini 内容（[Bindings] action=KeyCode Debug 名）
fn bindings_to_ini(bindings: &[KeyBinding]) -> String {
    let mut s = String::from("[Bindings]\n");
    for b in bindings {
        s.push_str(&format!("{}={}\n", b.action, format!("{:?}", b.key)));
    }
    s
}

/// 解析 KeyBinds.ini 内容 → 绑定列表（缺失/非法回退默认键）
fn bindings_from_ini(content: &str, defaults: &[KeyBinding]) -> Vec<KeyBinding> {
    defaults
        .iter()
        .map(|b| {
            let key = settings_file::ini_str(content, "Bindings", b.action)
                .and_then(key_code_from_name)
                .unwrap_or(b.key);
            KeyBinding::new(b.action, b.group, key)
        })
        .collect()
}

/// 从 KeyBinds.ini 加载（不存在返回默认）
fn load_bindings(defaults: &[KeyBinding]) -> Vec<KeyBinding> {
    bindings_from_ini(&fs::read_to_string(KEYBINDS_PATH).unwrap_or_default(), defaults)
}

/// 保存绑定到 KeyBinds.ini
fn save_bindings(bindings: &[KeyBinding]) {
    let _ = fs::write(KEYBINDS_PATH, bindings_to_ini(bindings));
}

/// 默认键位（对齐 C# KeyBindSettings.New()：背包 F9/I、角色 F10/C、技能 F11/S、
/// 任务 Q、小地图 V、设置 F12/O、拾取 Tab；攻击模式切换 C# Ctrl+H 已在 combat.rs 硬编码）
pub fn default_bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("向上移动", "移动", KeyCode::KeyW),
        KeyBinding::new("向左移动", "移动", KeyCode::KeyA),
        KeyBinding::new("向下移动", "移动", KeyCode::KeyS),
        KeyBinding::new("向右移动", "移动", KeyCode::KeyD),
        KeyBinding::new("拾取", "交互", KeyCode::Tab),
        KeyBinding::new("拾取2", "交互", KeyCode::Space),
        KeyBinding::new("聊天", "交互", KeyCode::Enter),
        KeyBinding::new("背包", "界面", KeyCode::F9),
        KeyBinding::new("背包2", "界面", KeyCode::KeyI),
        KeyBinding::new("角色", "界面", KeyCode::F10),
        KeyBinding::new("角色2", "界面", KeyCode::KeyC),
        KeyBinding::new("技能", "界面", KeyCode::F11),
        KeyBinding::new("技能2", "界面", KeyCode::KeyS),
        KeyBinding::new("好友", "界面", KeyCode::KeyF),
        KeyBinding::new("宠物", "界面", KeyCode::KeyE),
        KeyBinding::new("坐骑", "界面", KeyCode::KeyJ),
        KeyBinding::new("钓鱼", "界面", KeyCode::KeyN),
        KeyBinding::new("夫妻", "界面", KeyCode::KeyL),
        KeyBinding::new("队伍", "界面", KeyCode::KeyP),
        KeyBinding::new("商城", "界面", KeyCode::KeyY),
        KeyBinding::new("大地图", "界面", KeyCode::KeyB),
        KeyBinding::new("排行", "界面", KeyCode::KeyK),
        KeyBinding::new("键位", "界面", KeyCode::KeyU),
        KeyBinding::new("技能栏显隐", "界面", KeyCode::KeyR),
        KeyBinding::new("腰带", "界面", KeyCode::KeyZ),
        KeyBinding::new("行会", "界面", KeyCode::KeyG),
        KeyBinding::new("小地图", "界面", KeyCode::KeyV),
        KeyBinding::new("任务", "界面", KeyCode::KeyQ),
        KeyBinding::new("设置", "系统", KeyCode::F12),
        KeyBinding::new("设置2", "系统", KeyCode::KeyO),
        KeyBinding::new("技能栏1", "技能", KeyCode::F1),
        KeyBinding::new("技能栏2", "技能", KeyCode::F2),
        KeyBinding::new("技能栏3", "技能", KeyCode::F3),
        KeyBinding::new("技能栏4", "技能", KeyCode::F4),
        KeyBinding::new("技能栏5", "技能", KeyCode::F5),
        KeyBinding::new("技能栏6", "技能", KeyCode::F6),
        KeyBinding::new("技能栏7", "技能", KeyCode::F7),
        KeyBinding::new("技能栏8", "技能", KeyCode::F8),
        KeyBinding::new("帮助", "系统", KeyCode::KeyH),
        KeyBinding::new("关闭全部", "系统", KeyCode::Escape),
    ]
}

/// 面板尺寸（Title[119] 实测 512x430）
const PANEL_W: f32 = 512.0;
const PANEL_H: f32 = 430.0;

/// 键位设置状态
#[derive(Resource)]
pub struct KeyboardState {
    pub bindings: Vec<KeyBinding>,
    pub defaults: Vec<KeyBinding>,
    /// 当前滚动偏移（按条目计，含组标题）
    pub top_line: usize,
    /// 正在等待重新绑定的绑定下标（bindings 下标）
    pub rebinding: Option<usize>,
    /// 严格规则（true=严格，false=宽松）
    pub enforce: bool,
}

impl Default for KeyboardState {
    fn default() -> Self {
        let defaults = default_bindings();
        // #1301：启动从 KeyBinds.ini 加载（缺失/非法回退默认）
        let bindings = load_bindings(&defaults);
        Self {
            bindings,
            defaults,
            top_line: 0,
            rebinding: None,
            enforce: true,
        }
    }
}

/// 可见行（y 为行区内的相对偏移，组标题 30px、绑定行 18px）
enum RowSpec {
    Group { y: f32, text: String },
    Bind { y: f32, text: String, index: usize, waiting: bool },
}

/// 按 C# UpdateText 规则生成可见行
fn build_rows(state: &KeyboardState) -> Vec<RowSpec> {
    let mut rows = Vec::new();
    let mut current_group = "";
    let mut group_count = 0usize;
    let mut skip = state.top_line;
    for (i, b) in state.bindings.iter().enumerate() {
        if b.group != current_group {
            current_group = b.group;
            if skip > 0 {
                skip -= 1;
            } else {
                let y = 18.0 * (i as f32 - state.top_line as f32) + group_count as f32 * 30.0;
                if y > 260.0 {
                    break;
                }
                rows.push(RowSpec::Group {
                    y,
                    text: b.group.to_string(),
                });
                group_count += 1;
            }
        }
        let y = 18.0 * (i as f32 - state.top_line as f32) + group_count as f32 * 30.0;
        if skip > 0 {
            skip -= 1;
            continue;
        }
        if y > 260.0 {
            break;
        }
        let waiting = state.rebinding == Some(i);
        let key_txt = if waiting {
            "按新按键...".to_string()
        } else {
            key_name(b.key)
        };
        rows.push(RowSpec::Bind {
            y,
            text: format!("{}  [{}]", b.action, key_txt),
            index: i,
            waiting,
        });
    }
    rows
}

/// 条目总数（绑定 + 组标题），用于滚动范围
fn total_rows(state: &KeyboardState) -> usize {
    let mut groups = 0;
    let mut last = "";
    for b in &state.bindings {
        if b.group != last {
            groups += 1;
            last = b.group;
        }
    }
    state.bindings.len() + groups
}

#[derive(Component)]
pub struct KeyboardWidget;

#[derive(Component)]
pub struct KeyboardClose;

#[derive(Component)]
pub struct KeyboardScrollUp;

#[derive(Component)]
pub struct KeyboardScrollDown;

#[derive(Component)]
pub struct KeyboardReset;

#[derive(Component)]
pub struct KeyboardEnforce;

#[derive(Component)]
pub struct KeyboardPositionBar(pub f32);

#[derive(Component)]
pub struct KeyboardRow(pub usize, pub f32);

pub struct KeyboardPlugin;

impl Plugin for KeyboardPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KeyboardState>();
        app.add_systems(OnEnter(AppState::Game), spawn_keyboard_layout);
        app.add_systems(OnExit(AppState::Game), cleanup_keyboard_layout);
        app.add_systems(
            Update,
            (dialog_hotkey_system, secondary_hotkey_system, keyboard_layout_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_keyboard_layout(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_keyboard_layout(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 面板 Title[119]（512x430），居中
    let (pw, ph) = match libs.0.get_image(LibraryName::Title, 119) {
        Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
        None => (PANEL_W, PANEL_H),
    };
    let px = (1024.0 - pw) / 2.0;
    let py = (768.0 - ph) / 2.0;

    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 119) {
        let e = spawn_ui_sprite(&mut commands, h, px, py, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::KeyboardLayout),
            KeyboardWidget,
            Visibility::Hidden,
        ));
    }

    // 标题“键位设置”（C# PageLabel (135,34)）
    let t = spawn_ui_text(
        &mut commands, &font, "键位设置",
        px + 135.0, py + 34.0, 15.0, Color::WHITE, 8.0,
    );
    commands.entity(t).insert((
        KeyboardWidget,
        DialogRoot(DialogKind::KeyboardLayout),
    ));

    // 关闭按钮 (489,3)
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        px + 489.0, py + 3.0, 7.0, 16.0, 14.0,
    ) {
        commands.entity(e).insert((
            KeyboardClose,
            DialogRoot(DialogKind::KeyboardLayout),
            KeyboardWidget,
        ));
    }

    // 上滚 (491,88) / 下滚 (491,363)
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 197, 198, 199,
        px + 491.0, py + 88.0, 7.0, 16.0, 14.0,
    ) {
        commands.entity(e).insert((
            KeyboardScrollUp,
            DialogRoot(DialogKind::KeyboardLayout),
            KeyboardWidget,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 207, 208, 209,
        px + 491.0, py + 363.0, 7.0, 16.0, 14.0,
    ) {
        commands.entity(e).insert((
            KeyboardScrollDown,
            DialogRoot(DialogKind::KeyboardLayout),
            KeyboardWidget,
        ));
    }

    // 位置条 (491,101)
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse2, 205) {
        let e = spawn_ui_sprite(&mut commands, h, px + 491.0, py + 101.0, 7.0, 1.0);
        commands.entity(e).insert((
            KeyboardPositionBar(py + 101.0),
            DialogRoot(DialogKind::KeyboardLayout),
            KeyboardWidget,
        ));
    }

    // 重置按钮 Title[120/121/122] (30,400) 72x25
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 120, 121, 122,
        px + 30.0, py + 400.0, 7.0, 72.0, 25.0,
    ) {
        commands.entity(e).insert((
            KeyboardReset,
            DialogRoot(DialogKind::KeyboardLayout),
            KeyboardWidget,
            // #93 通用 Tooltip：C# 重置按钮 Hint
            crate::ui::tooltip::TooltipHint("重置为默认键位".to_string()),
        ));
    }

    // 严格规则复选框（#90 通用 CheckBox：Prguse[1346] 未勾 / [1347] 勾选）
    if let Some(e) = spawn_checkbox(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse,
        [1346, 1346, 1346],
        [1347, 1347, 1347],
        px + 105.0, py + 406.0, 7.0, 16.0, 14.0,
        false,
    ) {
        commands.entity(e).insert((
            KeyboardEnforce,
            DialogRoot(DialogKind::KeyboardLayout),
            KeyboardWidget,
        ));
    }
    let e = spawn_ui_text(
        &mut commands, &font, "严格规则",
        px + 125.0, py + 405.0, 12.0, Color::WHITE, 8.0,
    );
    commands.entity(e).insert((
        KeyboardWidget,
        DialogRoot(DialogKind::KeyboardLayout),
    ));

    // 行区文字实体（16 个槽，位置每帧按 build_rows 更新）
    for i in 0..16usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            px + 20.0, py + 90.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            KeyboardRow(i, py + 90.0),
            DialogRoot(DialogKind::KeyboardLayout),
            KeyboardWidget,
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn keyboard_layout_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<KeyboardState>,
    close: Query<&UiButton, With<KeyboardClose>>,
    scroll_up: Query<&UiButton, With<KeyboardScrollUp>>,
    scroll_down: Query<&UiButton, With<KeyboardScrollDown>>,
    reset: Query<&UiButton, With<KeyboardReset>>,
    enforce: Query<&CheckBox, With<KeyboardEnforce>>,
    mut widgets: Query<&mut Visibility, With<KeyboardWidget>>,
    mut pos_bar: Query<(&mut Transform, &KeyboardPositionBar), Without<KeyboardRow>>,
    mut rows: Query<(&mut Text2d, &mut Transform, &KeyboardRow)>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
) {
    let open = mgr.is_open(DialogKind::KeyboardLayout);
    for mut vis in &mut widgets {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        state.rebinding = None;
        return;
    }

    for btn in &close {
        if btn.clicked {
            state.rebinding = None;
            mgr.close(DialogKind::KeyboardLayout);
        }
    }
    for btn in &scroll_up {
        if btn.clicked && state.top_line > 0 {
            state.top_line -= 1;
            tracing::info!("🎹 上滚 top_line={}", state.top_line);
        }
    }
    let max_scroll = total_rows(&state).saturating_sub(12).max(1);
    for btn in &scroll_down {
        if btn.clicked && state.top_line < max_scroll - 1 {
            state.top_line += 1;
            tracing::info!("🎹 下滚 top_line={}", state.top_line);
        }
    }
    for btn in &reset {
        if btn.clicked {
            state.bindings = state.defaults.clone();
            state.top_line = 0;
            state.rebinding = None;
            save_bindings(&state.bindings);
            tracing::info!("🎹 键位已重置为默认");
        }
    }
    // #90 通用 CheckBox：点击切换由 checkbox_system 处理，这里同步状态
    if let Ok(cb) = enforce.single() {
        state.enforce = cb.checked;
    }

    // 点击绑定行 → 进入等待重绑（C# KeybindRow.Click）
    if state.rebinding.is_none() {
        if let Ok(window) = windows.single() {
            if let Some(cursor) = window.cursor_position() {
                if mouse.just_pressed(MouseButton::Left) {
                    let base = 90.0;
                    for spec in build_rows(&state) {
                        if let RowSpec::Bind { y, index, .. } = spec {
                            let ry = (768.0 - PANEL_H) / 2.0 + base + y;
                            if cursor.x >= (1024.0 - PANEL_W) / 2.0 + 20.0
                                && cursor.x <= (1024.0 - PANEL_W) / 2.0 + 480.0
                                && cursor.y >= ry
                                && cursor.y <= ry + 15.0
                            {
                                state.rebinding = Some(index);
                                tracing::info!("🎹 等待按键: 行 {}", index);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    // 等待按键：任意键重绑（Esc 取消）
    if let Some(idx) = state.rebinding {
        let mut changed = false;
        for key in keys.get_just_pressed() {
            let k = *key;
            if k == KeyCode::Escape {
                state.rebinding = None;
                tracing::info!("🎹 取消重绑");
            } else if let Some(b) = state.bindings.get_mut(idx) {
                tracing::info!("🎹 绑定 {} → {}", b.action, key_name(k));
                b.key = k;
                state.rebinding = None;
                changed = true;
            }
            break;
        }
        // #1301：重绑后立即持久化
        if changed {
            save_bindings(&state.bindings);
        }
    }


    // 位置条
    for (mut tf, bar) in &mut pos_bar {
        let pct = state.top_line as f32 / max_scroll as f32;
        tf.translation.y = -(bar.0 + 13.0 + pct * 262.0);
    }

    // 行文字 + 位置
    let specs = build_rows(&state);
    for (mut text, mut tf, row) in &mut rows {
        match specs.get(row.0) {
            Some(RowSpec::Group { y, text: t }) => {
                text.0 = format!("◆ {}", t);
                tf.translation.y = -(row.1 + *y);
            }
            Some(RowSpec::Bind { y, text: t, .. }) => {
                text.0 = t.clone();
                tf.translation.y = -(row.1 + *y);
            }
            None => {
                text.0 = String::new();
            }
        }
    }
}


/// 快捷键打开/关闭窗口（#148/#1370，C# KeybindOptions 对齐；随键位设置可重绑）
/// 覆盖：背包/角色/技能/好友/宠物/坐骑/钓鱼/夫妻/队伍/商城/大地图/排行/键位/帮助/行会/小地图/任务/设置
fn dialog_hotkey_system(
    keys: Res<ButtonInput<KeyCode>>,
    kb: Res<KeyboardState>,
    mut mgr: ResMut<DialogManager>,
    mut belt_visible: ResMut<crate::game::dialogs::belt::BeltVisible>,
    mut potion_belt_visible: ResMut<crate::game::dialogs::potion_belt::PotionBeltVisible>,
) {
    // #795：主/次绑定（对齐 C# KeyBindSettings 主键 + 备用键）
    let map: [(&str, DialogKind); 22] = [
        ("背包", DialogKind::Inventory),
        ("背包2", DialogKind::Inventory),
        ("角色", DialogKind::Character),
        ("角色2", DialogKind::Character),
        ("技能", DialogKind::Skills),
        ("技能2", DialogKind::Skills),
        ("好友", DialogKind::Friend),
        ("宠物", DialogKind::Creature),
        ("坐骑", DialogKind::Mount),
        ("钓鱼", DialogKind::Fishing),
        ("夫妻", DialogKind::Relationship),
        ("队伍", DialogKind::Group),
        ("商城", DialogKind::GameShop),
        ("大地图", DialogKind::BigMap),
        ("排行", DialogKind::Ranking),
        ("键位", DialogKind::KeyboardLayout),
        ("帮助", DialogKind::Help),
        ("行会", DialogKind::Guild),
        ("小地图", DialogKind::Minimap),
        ("任务", DialogKind::QuestLog),
        ("设置", DialogKind::Settings),
        ("设置2", DialogKind::Settings),
    ];
    for (action, kind) in map {
        let Some(b) = kb.bindings.iter().find(|b| b.action == action) else { continue };
        if keys.just_pressed(b.key) {
            mgr.toggle(kind);
        }
    }
    // #1370：技能栏显隐（R）/ 腰带（Z）——非对话框，走显隐资源
    if let Some(b) = kb.bindings.iter().find(|b| b.action == "技能栏显隐") {
        if keys.just_pressed(b.key) {
            belt_visible.0 = !belt_visible.0;
        }
    }
    if let Some(b) = kb.bindings.iter().find(|b| b.action == "腰带") {
        if keys.just_pressed(b.key) {
            potion_belt_visible.0 = !potion_belt_visible.0;
        }
    }
}

/// #1373：次级快捷键（C# KeyBindSettings 默认，含修饰键；修饰键暂不可重绑为简化）
/// 英雄背包 Ctrl+I / 英雄装备 Ctrl+C / 英雄技能 Ctrl+S / 坐骑 M(@ride) /
/// 退出 Alt+Q / 下线 Alt+X / 腰带 1-8（使用药水，C# Belt1..8）
fn secondary_hotkey_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut mgr: ResMut<DialogManager>,
    net: Res<NetConnection>,
    belt: Res<crate::game::dialogs::potion_belt::PotionBeltState>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let alt = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
    if ctrl && keys.just_pressed(KeyCode::KeyI) {
        mgr.toggle(DialogKind::HeroInventory);
    }
    if ctrl && keys.just_pressed(KeyCode::KeyC) {
        mgr.toggle(DialogKind::HeroEquipment);
    }
    if ctrl && keys.just_pressed(KeyCode::KeyS) {
        mgr.toggle(DialogKind::HeroSkill);
    }
    if keys.just_pressed(KeyCode::KeyM) {
        net.send_packet(&mir2_shared::packets::client::chat::Chat {
            message: "@ride".to_string(),
            linked_items: Vec::new(),
        });
        tracing::info!("🐴 M 请求骑乘/下马 (@ride)");
    }
    if alt && keys.just_pressed(KeyCode::KeyQ) {
        tracing::info!("🎮 Alt+Q 退出游戏");
        std::process::exit(0);
    }
    if alt && keys.just_pressed(KeyCode::KeyX) {
        net.send_packet(&mir2_shared::packets::client::character::LogOut);
        tracing::info!("🎮 Alt+X 下线");
    }
    // 腰带 1-8（C# Belt1..8：D1..D8 / NumPad1..8）
    let digits = [
        KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4,
        KeyCode::Digit5, KeyCode::Digit6, KeyCode::Digit7, KeyCode::Digit8,
    ];
    let numpads = [
        KeyCode::Numpad1, KeyCode::Numpad2, KeyCode::Numpad3, KeyCode::Numpad4,
        KeyCode::Numpad5, KeyCode::Numpad6, KeyCode::Numpad7, KeyCode::Numpad8,
    ];
    for i in 0..8usize {
        let pressed = keys.just_pressed(digits[i]) || keys.just_pressed(numpads[i]);
        if pressed {
            if let Some(uid) = belt.slots.get(i).and_then(|u| u.as_ref()).copied() {
                net.send_packet(&mir2_shared::packets::client::item::UseItem { unique_id: uid });
                tracing::info!("🧪 腰带 {} 使用 uid={}", i + 1, uid);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_code_name_roundtrip() {
        for k in [KeyCode::KeyW, KeyCode::F1, KeyCode::Tab, KeyCode::Space, KeyCode::Escape, KeyCode::ArrowUp] {
            let name = format!("{:?}", k);
            assert_eq!(key_code_from_name(&name), Some(k), "{}", name);
        }
        assert_eq!(key_code_from_name("NotAKey"), None);
    }

    #[test]
    fn bindings_roundtrip_ini() {
        let defaults = default_bindings();
        // 改两个键位（背包→KeyB、技能栏1→KeyQ）
        let mut bindings = defaults.clone();
        bindings[7].key = KeyCode::KeyB;
        bindings[18].key = KeyCode::KeyQ;
        let ini = bindings_to_ini(&bindings);
        let loaded = bindings_from_ini(&ini, &defaults);
        assert_eq!(loaded.len(), defaults.len());
        assert_eq!(loaded[7].key, KeyCode::KeyB);
        assert_eq!(loaded[18].key, KeyCode::KeyQ);
        // 未改动的回退默认
        assert_eq!(loaded[0].key, KeyCode::KeyW);
    }

    #[test]
    fn bindings_from_ini_invalid_falls_back() {
        let defaults = default_bindings();
        let ini = "[Bindings]\n背包=NotAKey\n";
        let loaded = bindings_from_ini(ini, &defaults);
        assert_eq!(loaded[7].key, defaults[7].key);
    }
}
