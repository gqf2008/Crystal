// ============================================================================
#![allow(clippy::type_complexity)]
// 数量输入框（M9 第 2 批，通用组件）
// 布局参考：macroquad amount_box.rs / C# MirAmountBox
//   - 背景 Prguse[238]（原生 204x109，居中 @(410,329)）；OK Title[200-202] (23,76)；
//     Cancel Title[203-205] (110,76)；关闭 Prguse2[360-362] (180,3)；标题 (19,8)；数量输入区
// 使用：AmountBoxState.ask(title, max) → 用户输入 → AmountBoxResult 事件
// bevy_ui 迁移（批 14）：全节点化，模态面板 GlobalZIndex(60)（盖过邀请弹窗 45）
// ============================================================================

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::pinyin_ime::PinyinIme;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_icon_button, spawn_label, spawn_panel,
};

/// 数量输入结果事件（OK 时携带数量）
#[derive(Message, Debug)]
pub struct AmountBoxResult(pub Option<u32>);

#[derive(Resource, Default)]
pub struct AmountBoxState {
    pub visible: bool,
    pub title: String,
    pub max: u32,
    /// 下限（C# `MinAmount`；确认值钳到 [min, max]）
    pub min: u32,
    pub value: String,
    /// 预填 max 后未编辑（首个数字整体替换，C# 预填全选语义）
    pub fresh: bool,
    /// 物品图标（C# `ImageIndex` = `Item.Info.Image`，`ItemImage` @(15,34) 38x34）；
    /// None = 不显示图标（拆分/丢弃等无图标场景）
    pub icon: Option<(LibraryName, usize)>,
}

#[derive(Component)]
pub struct AmountBoxWidget;

#[derive(Component)]
pub struct AmountOk;

#[derive(Component)]
pub struct AmountCancel;

#[derive(Component)]
pub struct AmountClose;

#[derive(Component)]
pub struct AmountTitleText;

#[derive(Component)]
pub struct AmountValueText;

/// 物品图标节点（C# `ItemImage` @(15,34) 38x34）
#[derive(Component)]
pub struct AmountIconNode;

/// 图标节点（自绘 ImageNode）
#[derive(Component)]
pub struct AmountIconImage;

/// 输入框容器（C# `InputTextBox` @(58,43) 132x19，`Border = true` → 1px 边框）
#[derive(Component)]
pub struct AmountInputBox;

pub struct AmountBoxPlugin;

impl Plugin for AmountBoxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AmountBoxState>();
        app.add_message::<AmountBoxResult>();
        app.add_systems(OnEnter(AppState::Game), spawn_amount_box);
        app.add_systems(OnExit(AppState::Game), cleanup_amount_box);
        app.add_systems(
            Update,
            amount_box_system.run_if(in_state(AppState::Game)),
        );
    }
}

impl AmountBoxState {
    /// 弹出数量输入框。默认文本 = MaxAmount（C# MirAmountBox.cs:91，
    /// Amount 初值=max——空 Enter 即以最后有效值确认，#2609）。
    /// fresh=预填未编辑态：首个数字输入整体替换（C# :92-93 预填全选语义）
    pub fn ask(&mut self, title: impl Into<String>, max: u32) {
        self.ask_with(title, None, max, 0, 1);
    }

    /// C# `MirAmountBox(title, image, max, min, defaultAmount)`：
    /// 初值 `Amount = max`，文本框取 `defaultAmount∈(0, max]` 否则 `max`（整段选中）；
    /// `image` = `Item.Info.Image`（不传则不显示图标）。
    pub fn ask_with(
        &mut self,
        title: impl Into<String>,
        icon: Option<(LibraryName, usize)>,
        max: u32,
        default_amount: u32,
        min: u32,
    ) {
        self.visible = true;
        self.title = title.into();
        self.max = max.max(1);
        self.min = min.clamp(1, self.max);
        let initial = if default_amount > 0 && default_amount <= self.max {
            default_amount
        } else {
            self.max
        };
        self.value = initial.to_string();
        self.fresh = true;
        self.icon = icon;
    }
}

/// C# `MirAmountBox` 输入框边框三态（`Border = true` + `BorderColour`）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmountBorder {
    /// 合法（`Amount >= MinAmount` 且未到上限）
    Lime,
    /// 合法且 `Amount == MaxAmount`（C# 先钳到上限再判等）
    Orange,
    /// 非法（空/非数字/`< MinAmount`）—— C# 同时隐藏 OK 键
    Red,
}

impl AmountBorder {
    /// C# `Color.Lime` / `Color.Orange` / `Color.Red`
    pub fn color(self) -> Color {
        match self {
            Self::Lime => Color::srgb_u8(0, 255, 0),
            Self::Orange => Color::srgb_u8(255, 165, 0),
            Self::Red => Color::srgb_u8(255, 0, 0),
        }
    }
}

/// C# `MirAmountBox.TextBox_TextChanged`（MirAmountBox.cs:172-194）：
/// `uint.TryParse(text) && Amount >= MinAmount` → Lime 且 **OK 可见**；
/// `Amount > MaxAmount` 时 C# 先钳到 MaxAmount 并回写文本，随后 `Amount == MaxAmount` → Orange；
/// 否则（解析失败或低于下限）→ Red 且 **OK 隐藏**。返回 `(边框态, OK 是否可见)`。
pub fn amount_border_state(value: &str, min: u32, max: u32) -> (AmountBorder, bool) {
    let Ok(amount) = value.trim().parse::<u32>() else {
        return (AmountBorder::Red, false);
    };
    if amount < min.max(1) {
        return (AmountBorder::Red, false);
    }
    if amount >= max {
        // C# 钳到 MaxAmount 后判等 → Orange（OK 可见）
        (AmountBorder::Orange, true)
    } else {
        (AmountBorder::Lime, true)
    }
}

fn cleanup_amount_box(mut commands: Commands, roots: Query<Entity, With<AmountBoxWidget>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_amount_box(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut cjk_font: ResMut<UiCjkFont>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();
    let cjk = shared_cjk_font(&mut fonts, &mut cjk_font);

    // 居中弹窗（C# MirAmountBox：Prguse[238] 原生 204x109，Screen 居中）
    let (x, y) = ((1024.0 - 204.0) / 2.0, (768.0 - 109.0) / 2.0);
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 238) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, x, y, 204.0, 109.0, 60);
    commands.entity(panel).insert(AmountBoxWidget);

    commands.entity(panel).with_children(|p| {
        // 标题（C# (19,8)）
        spawn_label(p, &cjk, "", 19.0, 8.0, 12.0, Color::WHITE, 9).insert(AmountTitleText);
        // 输入框容器（C# `InputTextBox` @(58,43) 132x19 + `Border=true` 的 1px 外框；
        // Bevy 边框画在节点内 → 容器取 (57,42) 134x21，内容区正好是 C# 的 132x19）
        p.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(57.0),
                top: Val::Px(42.0),
                width: Val::Px(134.0),
                height: Val::Px(21.0),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::BLACK),
            BorderColor::all(AmountBorder::Lime.color()),
            ZIndex(9),
            AmountInputBox,
        ))
        .with_children(|ib| {
            // 数量值（C# 文本框内文字；相对输入框 (3,2) = 绝对 (60,44)）
            spawn_label(ib, &cjk, "", 3.0, 2.0, 14.0, Color::WHITE, 10)
                .insert(AmountValueText);
        });
        // 物品图标（C# `ItemImage` @(15,34) 38x34；无图标时隐藏）
        p.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(15.0),
                top: Val::Px(34.0),
                width: Val::Px(38.0),
                height: Val::Px(34.0),
                ..default()
            },
            ImageNode::default(),
            Visibility::Hidden,
            ZIndex(9),
            AmountIconNode,
        ));
        // OK Title[200/201/202]（C# (23,76)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 200),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 201),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 202),
        ) {
            spawn_icon_button(p, n, h, pr, 23.0, 76.0, 76.0, 25.0, 10).insert(AmountOk);
        }
        // Cancel Title[203/204/205]（C# (110,76)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 203),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 204),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 205),
        ) {
            spawn_icon_button(p, n, h, pr, 110.0, 76.0, 76.0, 25.0, 10).insert(AmountCancel);
        }
        // 关闭 Prguse2[360/361/362]（C# (180,3)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 180.0, 3.0, 20.0, 20.0, 10).insert(AmountClose);
        }
    });
}

/// 确认值：解析失败/空以 MaxAmount 兜底，钳 [1, max]（C# Amount=最后有效值
/// 初值 max、MinAmount 下限；Enter/OK 两路径共用，审查 MAJOR 语义统一）
fn confirm_amount(state: &AmountBoxState) -> Option<u32> {
    state
        .value
        .parse::<u32>()
        .ok()
        .or(Some(state.max))
        .map(|v| v.clamp(state.min.max(1), state.max))
}

/// 显示/隐藏 + 数字输入 + OK/Cancel/Close
/// （pub(crate)：#2604 esc_close_dialogs_system 的 Esc 让路依赖
/// `.before(本系统)` 排序锚点——本系统同帧消费 Esc 置 visible=false，
/// 若先跑则 esc_close 读到 false 误入 Closeall 连坐关全部对话框）
pub(crate) fn amount_box_system(
    mut state: ResMut<AmountBoxState>,
    mut result: MessageWriter<AmountBoxResult>,
    mut keys: MessageReader<KeyboardInput>,
    mut ime: ResMut<PinyinIme>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<crate::ui::sprite_ui::UiImageCache>,
    mut ok: Query<
        (Entity, &Interaction, &mut Visibility),
        (With<AmountOk>, Without<AmountCancel>, Without<AmountClose>),
    >,
    cancel: Query<(Entity, &Interaction), (With<AmountCancel>, Without<AmountOk>, Without<AmountClose>)>,
    close: Query<(Entity, &Interaction), (With<AmountClose>, Without<AmountOk>, Without<AmountCancel>)>,
    mut widgets: Query<
        &mut Visibility,
        (
            With<AmountBoxWidget>,
            Without<AmountOk>,
            Without<AmountIconNode>,
        ),
    >,
    mut titles: Query<&mut Text, (With<AmountTitleText>, Without<AmountValueText>)>,
    mut values: Query<&mut Text, (With<AmountValueText>, Without<AmountTitleText>)>,
    mut icons: Query<
        (&mut ImageNode, &mut Visibility),
        (
            With<AmountIconNode>,
            Without<AmountBoxWidget>,
            Without<AmountOk>,
        ),
    >,
    mut input_box: Query<&mut BorderColor, With<AmountInputBox>>,
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
    for mut vis in widgets.iter_mut() {
        *vis = if state.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !state.visible {
        return;
    }
    // C# `ItemImage`：`ImageIndex` = `Item.Info.Image`（无图标时隐藏）
    for (mut node, mut vis) in &mut icons {
        let handle = state
            .icon
            .map(|(lib, idx)| {
                crate::ui::sprite_ui::ui_image(&mut libs, &mut images, &mut cache, lib, idx)
            })
            .flatten();
        match handle {
            Some(h) => {
                if node.image != h {
                    node.image = h;
                }
                if *vis != Visibility::Visible {
                    *vis = Visibility::Visible;
                }
            }
            None => {
                if *vis != Visibility::Hidden {
                    *vis = Visibility::Hidden;
                }
            }
        }
    }
    // C# `MirAmountBox.TextBox_TextChanged`：边框三态 + OK 键显隐
    let (border, ok_visible) = amount_border_state(&state.value, state.min, state.max);
    for mut color in &mut input_box {
        let want = BorderColor::all(border.color());
        if *color != want {
            *color = want;
        }
    }
    for (_, _, mut vis) in &mut ok {
        let want = if ok_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }

    // 数字键盘输入 + Esc/Enter（C# MirAmountBox：Esc=Cancel、Enter=OK；
    // #2604——Esc 由此消费，esc_close_dialogs_system 检查 amount.visible 让路）
    for key in keys.read() {

        if key.state != bevy::input::ButtonState::Pressed {
            continue;
        }
        // #2596-3：IME 组合/候选时接管该键（数字选候选/退格删拼音/Esc 收候选），
        // 避免与聊天输入双写；非组合时数字正常进数量框
        if ime.consumes_key(key) {
            continue;
        }
        if key.logical_key == Key::Escape {
            state.visible = false;
            result.write(AmountBoxResult(None));
            continue;
        }
        if key.logical_key == Key::Enter {
            // 空/非法文本以 MaxAmount 兜底（C# Amount 属性=最后有效值，
            // 初值即 max；#2609）；MinAmount 语义=下限 1
            let amount = confirm_amount(&state);
            state.visible = false;
            result.write(AmountBoxResult(amount));
            continue;
        }
        // 预填未编辑时首个数字整体替换（C# :92-93 预填全选，输入即覆盖）
        let digit = if let Some(text) = &key.text {
            text.chars().all(|c| c.is_ascii_digit())
                .then(|| text.clone())
        } else if let Key::Character(c) = &key.logical_key {
            // winit 注入/部分键盘事件 text=None，用 logical_key 兜底（原版 C# 任意可打印字符）
            c.chars().all(|ch| ch.is_ascii_digit()).then(|| c.clone())
        } else {
            None
        };
        if let Some(d) = digit.filter(|d| !d.is_empty()) {
            if state.value.len() < 10 || state.fresh {
                if state.fresh {
                    state.value = d.to_string();
                    state.fresh = false;
                } else {
                    state.value.push_str(&d);
                }
            }
        } else if key.logical_key == Key::Backspace {
            // 预填全选态下 Backspace 整体清空（C# 全选删除语义，复审 NIT）
            if state.fresh {
                state.value.clear();
            } else {
                state.value.pop();
            }
            state.fresh = false;
        }
    }

    for (e, inter, _) in &mut ok {
        if edge(e, inter, &mut prev_inter) {
            // C# Enter 即 OKButton.InvokeMouseClick（:204-209/:277-278）——
            // 两路径同一解析/钳制（审查 MAJOR：旧 OK 路径未同步，语义分裂）
            let amount = confirm_amount(&state);
            state.visible = false;
            result.write(AmountBoxResult(amount));
        }
    }
    for (e, inter) in &cancel {
        if edge(e, inter, &mut prev_inter) {
            state.visible = false;
            result.write(AmountBoxResult(None));
        }
    }
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            state.visible = false;
            result.write(AmountBoxResult(None));
        }
    }

    if let Ok(mut t) = titles.single_mut() {
        t.0 = state.title.clone();
    }
    if let Ok(mut v) = values.single_mut() {
        v.0 = state.value.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2747：C# `MirAmountBox.TextBox_TextChanged`（:172-194）的边框三态与 OK 显隐：
    /// `>= MinAmount` → Lime + OK 可见；`> MaxAmount` 先钳到 Max → `== MaxAmount` → Orange；
    /// 解析失败或 `< MinAmount` → Red + OK 隐藏。
    #[test]
    fn amount_border_state_matches_csharp() {
        // 合法区间 → Lime
        assert_eq!(
            amount_border_state("50", 1, 100),
            (AmountBorder::Lime, true)
        );
        // 恰好到上限 / 超过上限（C# 钳到 Max 后判等）→ Orange
        assert_eq!(
            amount_border_state("100", 1, 100),
            (AmountBorder::Orange, true)
        );
        assert_eq!(
            amount_border_state("5000", 1, 100),
            (AmountBorder::Orange, true)
        );
        // 低于下限（出价场景 min = 当前价 + 1）→ Red + OK 隐藏
        assert_eq!(
            amount_border_state("150", 151, u32::MAX),
            (AmountBorder::Red, false)
        );
        assert_eq!(
            amount_border_state("200", 151, u32::MAX),
            (AmountBorder::Lime, true)
        );
        // 空 / 非数字 → Red + OK 隐藏
        assert_eq!(amount_border_state("", 1, 100), (AmountBorder::Red, false));
        assert_eq!(
            amount_border_state("abc", 1, 100),
            (AmountBorder::Red, false)
        );
        // 颜色 = C# `Color.Lime` / `Color.Orange` / `Color.Red`
        assert_eq!(AmountBorder::Lime.color(), Color::srgb_u8(0, 255, 0));
        assert_eq!(AmountBorder::Orange.color(), Color::srgb_u8(255, 165, 0));
        assert_eq!(AmountBorder::Red.color(), Color::srgb_u8(255, 0, 0));
    }
}
