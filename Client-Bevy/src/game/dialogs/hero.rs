// ============================================================================
// 英雄对话框（M48）
// 参考：C# HeroDialog + ServerRust hero.rs
// 网络：
//   C: ChangeHero[hero_index u8]
//   S: ChangeHero[hero_index u8]（send_hero_update_packet）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};

/// 英雄状态
#[derive(Resource, Default)]
pub struct HeroState {
    pub hero_index: u8,
    pub message: String,
}

#[derive(Component)]
pub struct HeroWidget;

#[derive(Component)]
pub struct HeroClose;

#[derive(Component)]
pub struct HeroSwitchMain;

#[derive(Component)]
pub struct HeroSwitch1;

#[derive(Component)]
pub struct HeroLine(usize);

pub struct HeroPlugin;

impl Plugin for HeroPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeroState>();
        app.add_systems(OnEnter(AppState::Game), spawn_hero);
        app.add_systems(OnExit(AppState::Game), cleanup_hero);
        app.add_systems(
            Update,
            (hero_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_hero(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_hero(
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

    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 170) {
        let e = spawn_ui_sprite(&mut commands, h, 280.0, 80.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Hero),
            HeroWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 300.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            HeroClose,
            DialogRoot(DialogKind::Hero),
            HeroWidget,
        ));
    }
    for i in 0..4usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            298.0, 120.0 + i as f32 * 24.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            HeroLine(i),
            DialogRoot(DialogKind::Hero),
            HeroWidget,
        ));
    }
    // 切换主角色 / 英雄 1
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        300.0, 230.0, 8.3, 90.0, 25.0,
    ) {
        commands.entity(e).insert((
            HeroSwitchMain,
            DialogRoot(DialogKind::Hero),
            HeroWidget,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        410.0, 230.0, 8.3, 90.0, 25.0,
    ) {
        commands.entity(e).insert((
            HeroSwitch1,
            DialogRoot(DialogKind::Hero),
            HeroWidget,
        ));
    }
}

/// 显隐 + 渲染 + 切换
#[allow(clippy::too_many_arguments)]
fn hero_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<HeroState>,
    net: Res<NetConnection>,
    close: Query<&UiButton, With<HeroClose>>,
    main_btn: Query<&UiButton, With<HeroSwitchMain>>,
    hero1_btn: Query<&UiButton, With<HeroSwitch1>>,
    mut widgets: Query<&mut Visibility, With<HeroWidget>>,
    mut lines: Query<(&mut Text2d, &HeroLine)>,
) {
    let open = mgr.is_open(DialogKind::Hero);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Hero);
        }
    }
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            0 => "英雄".to_string(),
            1 => format!(
                "当前: {}",
                if state.hero_index == 0 {
                    "主角色".to_string()
                } else {
                    format!("英雄 {}", state.hero_index)
                }
            ),
            2 => state.message.clone(),
            3 => "切换英雄（服务端响应 ChangeHero）".to_string(),
            _ => String::new(),
        };
    }
    for btn in &main_btn {
        if btn.clicked {
            net.send_packet(&crate::network::ChangeHeroWire { hero_index: 0 });
            state.message = "切换主角色…".to_string();
            tracing::info!("🦸 切换主角色");
        }
    }
    for btn in &hero1_btn {
        if btn.clicked {
            net.send_packet(&crate::network::ChangeHeroWire { hero_index: 1 });
            state.message = "切换英雄 1…".to_string();
            tracing::info!("🦸 切换英雄 1");
        }
    }
}
