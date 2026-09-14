// ============================================================================
// 好友备注窗（C# `MemoDialog`，`Client/MirScenes/Dialogs/FriendDialog.cs:480-568`）
//   - 面板 `Title[209]`（实测 196x166），`Movable = true`、`Location = Center` → (414,301)
//   - `MemoTextBox` @(15,30) 165x100（C# `MultiLine()`）
//   - OK `Title[382/383/384]` @(30,133) 48x25 → `C.AddMemo{CharacterIndex, Memo}` → `Hide()`
//   - Cancel `Title[385/386/387]` @(115,133) 48x25 → `Hide()`
//   - Close `Prguse2[360/361/362]` @(168,3) 24x21 → `Hide()`
//   - `Show()`：`MemoTextBox.Text = Friend.Memo` 并聚焦（`:563-566`）
// 本端：`DialogRoot(DialogKind::Memo)` 可直接吃通用 `dialog_drag_system` 的拖动（C# `Movable = true`）。
// 差异：C# 是多行文本框（`MirTextBox.MultiLine()`），本端文本输入基建是单行 → 先用单行输入占位，
// 与行会公告页的多行编辑缺口同源（见 `docs/UI_COMPONENTS.md` §7）。
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::text_input::{
    TextInputDisplay, TextInputField, TextInputMultiline, TextInputRect, TextInputState,
};
use crate::game::dialogs::{sync_dialog_state, DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont};
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_label, spawn_panel,
};

/// 面板（C# `MemoDialog.Index = 209; Library = Libraries.Title`，实测 196x166）
pub const PANEL: (LibraryName, usize) = (LibraryName::Title, 209);
pub const PANEL_SIZE: (f32, f32) = (196.0, 166.0);
/// 文本区（C# `MemoTextBox` @(15,30) 165x100）
pub const FIELD_POS: (f32, f32) = (15.0, 30.0);
pub const FIELD_SIZE: (f32, f32) = (165.0, 100.0);
/// OK / Cancel（`Title[382..384]` / `Title[385..387]`，48x25）
pub const OK_SPRITES: [usize; 3] = [382, 383, 384];
pub const CANCEL_SPRITES: [usize; 3] = [385, 386, 387];
pub const OK_POS: (f32, f32) = (30.0, 133.0);
pub const CANCEL_POS: (f32, f32) = (115.0, 133.0);
pub const BTN_SIZE: (f32, f32) = (48.0, 25.0);
/// 关闭（`Prguse2[360..362]` @(168,3)）
pub const CLOSE_SPRITES: [usize; 3] = [360, 361, 362];
pub const CLOSE_POS: (f32, f32) = (168.0, 3.0);
pub const CLOSE_SIZE: (f32, f32) = (24.0, 21.0);
/// 文本输入 id（避开其它窗口已用 id）
pub const MEMO_INPUT_ID: usize = 41;

/// 备注窗状态：`target` = 目标好友的 `object_id`
#[derive(Resource, Default)]
pub struct MemoState {
    pub open: bool,
    pub target: Option<i32>,
}

#[derive(Component)]
pub struct MemoWidget;
#[derive(Component)]
pub struct MemoOk;
#[derive(Component)]
pub struct MemoCancel;
#[derive(Component)]
pub struct MemoClose;

pub struct MemoPlugin;

impl Plugin for MemoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MemoState>();
        app.add_systems(OnEnter(AppState::Game), spawn_memo);
        app.add_systems(OnExit(AppState::Game), cleanup_memo);
        app.add_systems(Update, memo_ui_system.run_if(in_state(AppState::Game)));
    }
}

fn cleanup_memo(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_memo(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut cjk_font: ResMut<UiCjkFont>,
) {
    libs.0.ensure_initialized();
    let cjk = shared_cjk_font(&mut fonts, &mut cjk_font);

    let Some(bg) = load_lib_image(&mut libs, &mut images, PANEL.0, PANEL.1) else {
        return;
    };
    // C# `Location = Center`
    let (ox, oy) = crate::game::dialogs::center_origin(PANEL_SIZE.0, PANEL_SIZE.1);
    let panel = spawn_panel(&mut commands, bg, ox, oy, PANEL_SIZE.0, PANEL_SIZE.1, 45);
    // C# `Movable = true` → 挂 `DialogRoot` 即可拖（通用 `dialog_drag_system`），不加 `NotDraggable`
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::Memo), MemoWidget));

    let input_abs = (ox + FIELD_POS.0, oy + FIELD_POS.1);
    commands.entity(panel).with_children(|p| {
        // 文本区（C# `MemoTextBox` @(15,30) 165x100，`MultiLine()` → 本端多行输入）
        spawn_container(p, FIELD_POS.0, FIELD_POS.1, FIELD_SIZE.0, FIELD_SIZE.1, 2)
            .insert((
                BackgroundColor(Color::srgba(0.10, 0.10, 0.13, 0.95)),
                TextInputField(MEMO_INPUT_ID),
                // #2892：C# `MemoTextBox.MultiLine()` → Enter 换行、按宽度折行
                TextInputMultiline,
                TextInputRect(input_abs.0, input_abs.1, FIELD_SIZE.0, FIELD_SIZE.1),
                Visibility::Hidden,
            ))
            .with_children(|ic| {
                ic.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(3.0),
                        top: Val::Px(2.0),
                        // 定宽 → bevy_ui 文本按容器宽度自动折行（多行框）
                        width: Val::Px(FIELD_SIZE.0 - 6.0),
                        ..default()
                    },
                    Text::new(String::new()),
                    TextFont {
                        font: FontSource::Handle(cjk.clone()),
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    ZIndex(9),
                    TextInputDisplay(MEMO_INPUT_ID),
                ));
            });
        // OK / Cancel
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, OK_SPRITES[0]),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, OK_SPRITES[1]),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, OK_SPRITES[2]),
        ) {
            spawn_icon_button(p, n, h, pr, OK_POS.0, OK_POS.1, BTN_SIZE.0, BTN_SIZE.1, 9)
                .insert(MemoOk);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(
                &mut libs,
                &mut images,
                LibraryName::Title,
                CANCEL_SPRITES[0],
            ),
            load_lib_image(
                &mut libs,
                &mut images,
                LibraryName::Title,
                CANCEL_SPRITES[1],
            ),
            load_lib_image(
                &mut libs,
                &mut images,
                LibraryName::Title,
                CANCEL_SPRITES[2],
            ),
        ) {
            spawn_icon_button(
                p,
                n,
                h,
                pr,
                CANCEL_POS.0,
                CANCEL_POS.1,
                BTN_SIZE.0,
                BTN_SIZE.1,
                9,
            )
            .insert(MemoCancel);
        }
        // 关闭
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(
                &mut libs,
                &mut images,
                LibraryName::Prguse2,
                CLOSE_SPRITES[0],
            ),
            load_lib_image(
                &mut libs,
                &mut images,
                LibraryName::Prguse2,
                CLOSE_SPRITES[1],
            ),
            load_lib_image(
                &mut libs,
                &mut images,
                LibraryName::Prguse2,
                CLOSE_SPRITES[2],
            ),
        ) {
            spawn_icon_button(
                p,
                n,
                h,
                pr,
                CLOSE_POS.0,
                CLOSE_POS.1,
                CLOSE_SIZE.0,
                CLOSE_SIZE.1,
                9,
            )
            .insert(MemoClose);
        }
        // 标题（C# 的 `TitleLabel` 被注释掉，但面板 `Title[209]` 自带标题图样 → 不加文字标签）
        let _ = &cjk;
    });
}

/// 显隐 + OK/Cancel/Close（C# `MemoDialog` 交互）
fn memo_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<MemoState>,
    mut input: ResMut<TextInputState>,
    net: Res<NetConnection>,
    ok: Query<(Entity, &Interaction), With<MemoOk>>,
    cancel: Query<(Entity, &Interaction), (With<MemoCancel>, Without<MemoOk>, Without<MemoClose>)>,
    close: Query<(Entity, &Interaction), (With<MemoClose>, Without<MemoOk>, Without<MemoCancel>)>,
    mut widgets: Query<&mut Visibility, With<MemoWidget>>,
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
    sync_dialog_state(&mut mgr, DialogKind::Memo, state.open);
    for mut vis in &mut widgets {
        *vis = if state.open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !state.open {
        return;
    }
    // OK → `C.AddMemo`（C# `:516-520`）
    for (e, inter) in &ok {
        if edge(e, inter, &mut prev_inter) {
            let memo = input.texts.get(MEMO_INPUT_ID).cloned().unwrap_or_default();
            if let Some(target) = state.target {
                net.send_packet(&mir2_shared::packets::client::friend::AddMemo {
                    character_index: target,
                    memo: memo.clone(),
                });
                tracing::info!("👥 好友备注提交: {memo}");
            }
            state.open = false;
            input.active = None;
        }
    }
    // Cancel / Close → `Hide()`（C# `:532/546`）
    for (e, inter) in cancel.iter().chain(close.iter()) {
        if edge(e, inter, &mut prev_inter) {
            state.open = false;
            input.active = None;
        }
    }
}
