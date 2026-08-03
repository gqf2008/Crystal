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
    spawn_ui_sprite, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
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
    _fonts: ResMut<Assets<Font>>,
    _ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();

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
}

fn menu_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut widgets: Query<&mut Visibility, With<MenuWidget>>,
    buttons: Query<(&UiButton, &MenuBtn)>,
) {
    let open = mgr.is_open(DialogKind::Menu);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for (btn, action) in &buttons {
        if btn.clicked {
            match action.0 {
                MenuAction::Exit => {
                    tracing::info!("🎮 退出游戏");
                    std::process::exit(0);
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
