// ============================================================================
// 游戏菜单对话框（M9 第 1 批）
// 布局参考：C# GameScene MenuDialog（macroquad menu_dialog.rs）
//   - 背景 Title[567]（实测 36x282），位置 (ScreenWidth-36, MainDialog.Y-282+15)
//     MainDialog.Y = 768 - Prguse[1]高152 = 616 → 背景绝对原点 (988, 349)
//   - 按钮（x=3，y=12..259 共 13 个）：退出/下线/帮助/键盘/排名/宠物/坐骑/钓鱼/好友/师徒/夫妻/队伍/行会
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use crate::ui::theme::{
    load_lib_image, spawn_icon_button, spawn_label, spawn_panel, spawn_image,
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
    Relationship,
    Group,
    Guild,
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

// C# MenuDialog（MainDialogs.cs:3024-3029）：Index=567 Library=Title，
//   Location = (ScreenWidth - Size.Width, MainDialog.Y - Size.Height + 15)。
//   Title[567] 实测 36x282；MainDialog 背景 Prguse[1] 实测 1024x152 → MainDialog.Y = 768-152 = 616。
/// 菜单背景宽/高 = Title[567] 实测
pub const MENU_W: f32 = 36.0;
pub const MENU_H: f32 = 282.0;
/// 主底栏高 = Prguse[1] 实测（决定 MainDialog.Y = 768-152）
pub const MAIN_DIALOG_H: f32 = 152.0;
/// 菜单背景绝对原点 X = ScreenWidth - Width（C#）
pub const MENU_X: f32 = 1024.0 - MENU_W; // 988
/// 菜单背景绝对原点 Y = MainDialog.Y - Height + 15（C#）
pub const MENU_Y: f32 = 768.0 - MAIN_DIALOG_H - MENU_H + 15.0; // 349
/// 按钮相对菜单的 x（C# 所有按钮 Location.X = 3）
pub const MENU_BTN_DX: f32 = 3.0;

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
    (MenuAction::Relationship, LibraryName::Prguse, 1988, 1989, 1990, 221.0),
    (MenuAction::Group, LibraryName::Prguse, 1991, 1992, 1993, 240.0),
    (MenuAction::Guild, LibraryName::Prguse, 1994, 1995, 1996, 259.0),
];

pub struct MenuDialogPlugin;

impl Plugin for MenuDialogPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Game), spawn_menu_dialog);
        app.add_systems(OnExit(AppState::Game), cleanup_menu_dialog);
        app.add_systems(
            Update,
            (menu_ui_system,).run_if(in_state(AppState::Game)),
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
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 背景 Title[567]（C# Location=(ScreenWidth-Width, MainDialog.Y-Height+15) → (988,349)）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 567) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, MENU_X, MENU_Y, MENU_W, MENU_H, 30);
    commands.entity(panel).insert((DialogRoot(DialogKind::Menu), MenuWidget));

    commands.entity(panel).with_children(|p| {
        // 菜单按钮（相对面板 (3, y)）
        for (action, lib, n, h, pr, y) in MENU_BUTTONS {
            if let (Some(nh), Some(hh), Some(ph)) = (
                load_lib_image(&mut libs, &mut images, *lib, *n),
                load_lib_image(&mut libs, &mut images, *lib, *h),
                load_lib_image(&mut libs, &mut images, *lib, *pr),
            ) {
                spawn_icon_button(p, nh, hh, ph, MENU_BTN_DX, *y, 38.0, 19.0, 10)
                    .insert(MenuBtn(*action));
            }
        }
    });

    // 退出确认（C# MirMessageBox）：独立根节点（不随菜单面板裁切），半透明底
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    let confirm = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(382.0),
                top: Val::Px(324.0),
                width: Val::Px(260.0),
                height: Val::Px(120.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.14, 0.96)),
            GlobalZIndex(45),
            Visibility::Hidden,
        ))
        .id();
    commands.entity(confirm).insert((DialogRoot(DialogKind::Menu), MenuExitConfirm));
    commands.entity(confirm).with_children(|p| {
        spawn_label(p, &font, "确定要退出游戏吗？", 18.0, 12.0, 14.0, Color::WHITE, 9);
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
        ) {
            spawn_icon_button(p, n, h, pr, 28.0, 56.0, 76.0, 25.0, 10)
                .insert(MenuExitYes);
        }
        spawn_label(p, &font, "确定", 50.0, 60.0, 12.0, Color::WHITE, 11);
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 210),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 211),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 212),
        ) {
            spawn_icon_button(p, n, h, pr, 118.0, 56.0, 76.0, 25.0, 10)
                .insert(MenuExitNo);
        }
        spawn_label(p, &font, "取消", 148.0, 60.0, 12.0, Color::WHITE, 11);
    });
}

/// #1330：菜单按钮 → 对应面板显隐切换（C# MenuDialog Click：已开则 Hide，未开则 Show）
fn menu_open_toggle(mgr: &mut DialogManager, kind: DialogKind, name: &str) {
    tracing::info!("🎮 打开面板: {}", name);
    if mgr.is_open(kind) {
        mgr.close(kind);
    } else {
        mgr.open(kind);
    }
    mgr.close(DialogKind::Menu);
}

fn menu_ui_system(
    mut mgr: ResMut<DialogManager>,
    net: Res<crate::network::NetConnection>,
    mut widgets: Query<&mut Visibility, (With<MenuWidget>, Without<MenuExitConfirm>)>,
    buttons: Query<(Entity, &Interaction, &MenuBtn), Without<MenuExitConfirm>>,
    mut confirm: Local<bool>,
    mut confirm_widgets: Query<&mut Visibility, (With<MenuExitConfirm>, Without<MenuWidget>)>,
    yes: Query<(Entity, &Interaction), (With<MenuExitYes>, Without<MenuExitNo>)>,
    no: Query<(Entity, &Interaction), (With<MenuExitNo>, Without<MenuExitYes>)>,
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

    let open = mgr.is_open(DialogKind::Menu);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        for mut vis in &mut confirm_widgets {
            *vis = Visibility::Hidden;
        }
        *confirm = false;
        return;
    }
    for mut vis in &mut confirm_widgets {
        *vis = if *confirm { Visibility::Visible } else { Visibility::Hidden };
    }
    for (e, inter) in &yes {
        if edge(e, inter, &mut prev_inter) {
            tracing::info!("🎮 退出游戏");
            std::process::exit(0);
        }
    }
    for (e, inter) in &no {
        if edge(e, inter, &mut prev_inter) {
            *confirm = false;
        }
    }
    for (e, inter, action) in &buttons {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        match action.0 {
            MenuAction::Exit => {
                tracing::info!("🎮 退出确认");
                *confirm = true;
            }
            MenuAction::Logout => {
                net.send_packet(&mir2_shared::packets::client::character::LogOut);
                mgr.close(DialogKind::Menu);
                tracing::info!("🎮 发送下线请求");
            }
            MenuAction::Mount => {
                tracing::info!("🐴 打开坐骑面板");
                mgr.open(DialogKind::Mount);
                mgr.close(DialogKind::Menu);
            }
            MenuAction::Help => menu_open_toggle(&mut mgr, DialogKind::Help, "帮助"),
            MenuAction::Keyboard => menu_open_toggle(&mut mgr, DialogKind::KeyboardLayout, "键盘设置"),
            MenuAction::Ranking => menu_open_toggle(&mut mgr, DialogKind::Ranking, "排行榜"),
            MenuAction::Creature => menu_open_toggle(&mut mgr, DialogKind::Creature, "宠物"),
            MenuAction::Fishing => menu_open_toggle(&mut mgr, DialogKind::Fishing, "钓鱼"),
            MenuAction::Friends => menu_open_toggle(&mut mgr, DialogKind::Friend, "好友"),
            MenuAction::Mentor => menu_open_toggle(&mut mgr, DialogKind::Mentor, "师徒"),
            MenuAction::Relationship => menu_open_toggle(&mut mgr, DialogKind::Relationship, "夫妻"),
            MenuAction::Group => menu_open_toggle(&mut mgr, DialogKind::Group, "队伍"),
            MenuAction::Guild => menu_open_toggle(&mut mgr, DialogKind::Guild, "行会"),
        }
    }
}

