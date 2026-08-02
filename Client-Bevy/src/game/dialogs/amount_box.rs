// ============================================================================
#![allow(clippy::type_complexity)]
// 数量输入框（M9 第 2 批，通用组件）
// 布局参考：macroquad amount_box.rs / C# MirAmountBox
//   - 背景 Prguse[238]；OK Title[200-202] (23,76)；Cancel Title[203-205] (110,76)
//   - 关闭 Prguse2[360-362] (180,3)；标题 (19,8)；数量输入区
// 使用：AmountBoxState.ask(title, max) → 用户输入 → AmountBoxResult 事件
// ============================================================================

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};

/// 数量输入结果事件（OK 时携带数量）
#[derive(Message, Debug)]
pub struct AmountBoxResult(pub Option<u32>);

#[derive(Resource, Default)]
pub struct AmountBoxState {
    pub visible: bool,
    pub title: String,
    pub max: u32,
    pub value: String,
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

pub struct AmountBoxPlugin;

impl Plugin for AmountBoxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AmountBoxState>();
        app.add_systems(OnEnter(AppState::Game), spawn_amount_box);
        app.add_systems(OnExit(AppState::Game), cleanup_amount_box);
        app.add_systems(
            Update,
            (amount_box_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

impl AmountBoxState {
    /// 弹出数量输入框
    pub fn ask(&mut self, title: impl Into<String>, max: u32) {
        self.visible = true;
        self.title = title.into();
        self.max = max.max(1);
        self.value = String::new();
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
    mut cache: ResMut<UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 居中弹窗
    let x = (1024.0 - 210.0) / 2.0;
    let y = (768.0 - 110.0) / 2.0;

    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 238) {
        let e = spawn_ui_sprite(&mut commands, h, x, y, 9.0, 1.0);
        commands.entity(e).insert((AmountBoxWidget, Visibility::Hidden));
    }

    // 标题
    let t = spawn_ui_text(&mut commands, &font, "", x + 19.0, y + 8.0, 12.0, Color::WHITE, 9.2);
    commands.entity(t).insert((AmountTitleText, AmountBoxWidget));

    // 数量值
    let v = spawn_ui_text(&mut commands, &font, "", x + 60.0, y + 40.0, 14.0, Color::WHITE, 9.2);
    commands.entity(v).insert((AmountValueText, AmountBoxWidget));

    // OK / Cancel / Close
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 200, 201, 202,
        x + 23.0, y + 76.0, 9.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((AmountOk, AmountBoxWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 203, 204, 205,
        x + 110.0, y + 76.0, 9.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((AmountCancel, AmountBoxWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        x + 180.0, y + 3.0, 9.3, 20.0, 20.0,
    ) {
        commands.entity(e).insert((AmountClose, AmountBoxWidget));
    }
}

/// 显示/隐藏 + 数字输入 + OK/Cancel/Close
fn amount_box_system(
    mut state: ResMut<AmountBoxState>,
    mut result: MessageWriter<AmountBoxResult>,
    mut keys: MessageReader<KeyboardInput>,
    ok: Query<&UiButton, (With<AmountOk>, Without<AmountCancel>, Without<AmountClose>)>,
    cancel: Query<&UiButton, (With<AmountCancel>, Without<AmountOk>, Without<AmountClose>)>,
    close: Query<&UiButton, (With<AmountClose>, Without<AmountOk>, Without<AmountCancel>)>,
    mut widgets: Query<&mut Visibility, With<AmountBoxWidget>>,
    mut titles: Query<&mut Text2d, With<AmountTitleText>>,
    mut values: Query<&mut Text2d, With<AmountValueText>>,
) {
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

    // 数字键盘输入
    for key in keys.read() {
        if key.state != bevy::input::ButtonState::Pressed {
            continue;
        }
        if key.logical_key == Key::Backspace {
            state.value.pop();
        } else if let Some(text) = &key.text {
            if text.chars().all(|c| c.is_ascii_digit()) && state.value.len() < 10 {
                state.value.push_str(text);
            }
        }
    }

    for btn in &ok {
        if btn.clicked {
            let amount = state.value.parse::<u32>().ok().map(|v| v.min(state.max));
            state.visible = false;
            result.write(AmountBoxResult(amount));
        }
    }
    for btn in &cancel {
        if btn.clicked {
            state.visible = false;
            result.write(AmountBoxResult(None));
        }
    }
    for btn in &close {
        if btn.clicked {
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
