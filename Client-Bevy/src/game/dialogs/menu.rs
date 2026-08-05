// ============================================================================
// 游戏菜单对话框（M9 第 1 批）
// 布局参考：C# GameScene MenuDialog（macroquad menu_dialog.rs）
//   - 背景 Title[567]，位置 (ScreenWidth-44, MainDialog.top - 224 + 15)
//   - 按钮（x=3，y=12..202 每 19px）：退出/下线/帮助/键盘/排名/宠物/坐骑/钓鱼/好友/师徒
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiEntity, UiFont,
    UiImageCache,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuAction {
    Exit,
    Logout,
    Help,
    Keyboard,
    Ranking,
    Creature,
    Mount,
    Fishing,
    Friends,
    Mentor,
}

#[derive(Component)]
pub struct MenuWidget;

#[derive(Component)]
pub struct MenuBtn(pub MenuAction);

/// 退出确认面板（C# MirMessageBox 对齐）
#[derive(Component)]
pub struct MenuExitConfirm;

#[derive(Component)]
pub struct MenuExitYes;

#[derive(Component)]
pub struct MenuExitNo;

/// 菜单按钮定义（纹理索引 + y 偏移）
const MENU_BUTTONS: &[(MenuAction, LibraryName, usize, usize, usize, f32)] = &[
    (MenuAction::Exit, LibraryName::Title, 633, 634, 635, 12.0),
    (MenuAction::Logout, LibraryName::Title, 636, 637, 638, 31.0),
    (MenuAction::Help, LibraryName::Prguse, 1970, 1971, 1972, 50.0),
    (MenuAction::Keyboard, LibraryName::Prguse, 1973, 1974, 1975, 69.0),
    (MenuAction::Ranking, LibraryName::Prguse, 2000, 2001, 2002, 88.0),
    (MenuAction::Creature, LibraryName::Prguse2, 431, 432, 433, 126.0),
    (MenuAction::Mount, LibraryName::Prguse, 1976, 1977, 1978, 145.0),
    (MenuAction::Fishing, LibraryName::Prguse, 1979, 1980, 1981, 164.0),
    (MenuAction::Friends, LibraryName::Prguse, 1982, 1983, 1984, 183.0),
    (MenuAction::Mentor, LibraryName::Prguse, 1985, 1986, 1987, 202.0),
];

pub struct MenuDialogPlugin;

impl Plugin for MenuDialogPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Game), spawn_menu_dialog);
        app.add_systems(OnExit(AppState::Game), cleanup_menu_dialog);
        app.add_systems(
            Update,
            (menu_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_menu_dialog(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_menu_dialog(
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

    // 背景 Title[567]（位置：主对话框上方右侧）
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 567) {
        let e = spawn_ui_sprite(&mut commands, h, 1024.0 - 44.0, 768.0 - 150.0 - 224.0 + 15.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Menu),
            MenuWidget,
            Visibility::Hidden,
        ));
    }

    // 按钮
    for (action, lib, n, h, p, y) in MENU_BUTTONS {
        if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
            &mut commands, &mut libs, &mut images, &mut cache,
            *lib, *n, *h, *p,
            1024.0 - 44.0 + 3.0, 768.0 - 150.0 - 224.0 + 15.0 + y, 7.0, 38.0, 19.0,
        ) {
            commands.entity(e).insert((
                MenuBtn(*action),
                DialogRoot(DialogKind::Menu),
                MenuWidget,
            ));
        }
    }
    // 退出确认（C# MirMessageBox）
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    commands.spawn((
        UiEntity,
        DialogRoot(DialogKind::Menu),
        MenuExitConfirm,
        Sprite {
            image: white.clone(),
            color: Color::srgba(0.1, 0.1, 0.14, 0.96),
            custom_size: Some(Vec2::new(260.0, 120.0)),
            ..default()
        },
        bevy::sprite::Anchor::TOP_LEFT,
        Transform::from_xyz(382.0, -324.0, 9.0),
        Visibility::Hidden,
    ));
    let t = spawn_ui_text(&mut commands, &font, "确定要退出游戏吗？", 400.0, 336.0, 14.0, Color::WHITE, 9.2);
    commands.entity(t).insert((MenuExitConfirm, DialogRoot(DialogKind::Menu)));
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        410.0, 380.0, 9.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((MenuExitYes, DialogRoot(DialogKind::Menu)));
    }
    let t = spawn_ui_text(&mut commands, &font, "确定", 432.0, 384.0, 12.0, Color::WHITE, 9.4);
    commands.entity(t).insert((MenuExitConfirm, DialogRoot(DialogKind::Menu)));
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        500.0, 380.0, 9.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((MenuExitNo, DialogRoot(DialogKind::Menu)));
    }
    let t = spawn_ui_text(&mut commands, &font, "取消", 530.0, 384.0, 12.0, Color::WHITE, 9.4);
    commands.entity(t).insert((MenuExitConfirm, DialogRoot(DialogKind::Menu)));
}

fn menu_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut widgets: Query<&mut Visibility, With<MenuWidget>>,
    buttons: Query<(&UiButton, &MenuBtn)>,
    mut confirm: Local<bool>,
    mut confirm_widgets: Query<&mut Visibility, (With<MenuExitConfirm>, Without<MenuWidget>)>,
    yes: Query<&UiButton, With<MenuExitYes>>,
    no: Query<&UiButton, With<MenuExitNo>>,
) {
    let open = mgr.is_open(DialogKind::Menu);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        *confirm = false;
        return;
    }
    for mut vis in &mut confirm_widgets {
        *vis = if open && *confirm { Visibility::Visible } else { Visibility::Hidden };
    }
    for btn in &yes {
        if btn.clicked {
            tracing::info!("🎮 退出游戏");
            std::process::exit(0);
        }
    }
    for btn in &no {
        if btn.clicked {
            *confirm = false;
        }
    }
    for (btn, action) in &buttons {
        if btn.clicked {
            match action.0 {
                MenuAction::Exit => {
                    tracing::info!("🎮 退出确认");
                    *confirm = true;
                }
                MenuAction::Logout => {
                    tracing::info!("🎮 下线（待接入 LogoutRequest）");
                    mgr.close(DialogKind::Menu);
                }
                MenuAction::Mount => {
                    tracing::info!("🐴 打开坐骑面板");
                    mgr.open(DialogKind::Mount);
                    mgr.close(DialogKind::Menu);
                }
                other => {
                    tracing::info!("🎮 菜单: {:?}", other);
                    mgr.close(DialogKind::Menu);
                }
            }
        }
    }
}
