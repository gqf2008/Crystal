// ============================================================================
// 行会对话框（M9 第 3 批）
// 布局参考：macroquad guild_dialog.rs
//   - 背景 Prguse[956]，标题 Title[15]，位置 (280,80)
//   - 标签页 y=35；内容 y=60 起每 20px；按钮 y=210
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};

/// 行会状态（网络 GuildStatus/GuildMemberChange 等写入）
#[derive(Resource, Default)]
pub struct GuildState {
    pub name: String,
    pub members: Vec<String>,
}

#[derive(Component)]
pub struct GuildWidget;

#[derive(Component)]
pub struct GuildClose;

#[derive(Component)]
pub struct GuildLine(usize);

pub struct GuildPlugin;

impl Plugin for GuildPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GuildState>();
        app.add_systems(OnEnter(AppState::Game), spawn_guild);
        app.add_systems(OnExit(AppState::Game), cleanup_guild);
        app.add_systems(
            Update,
            (guild_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_guild(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_guild(
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

    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 956) {
        let e = spawn_ui_sprite(&mut commands, h, 280.0, 80.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Guild),
            GuildWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 15) {
        let e = spawn_ui_sprite(&mut commands, h, 298.0, 89.0, 6.2, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Guild),
            GuildWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 290.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            GuildClose,
            DialogRoot(DialogKind::Guild),
            GuildWidget,
        ));
    }
    for i in 0..8usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            288.0, 140.0 + i as f32 * 20.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            GuildLine(i),
            DialogRoot(DialogKind::Guild),
            GuildWidget,
        ));
    }
}

fn guild_ui_system(
    mut mgr: ResMut<DialogManager>,
    guild: Res<GuildState>,
    close: Query<&UiButton, With<GuildClose>>,
    mut widgets: Query<&mut Visibility, With<GuildWidget>>,
    mut lines: Query<(&mut Text2d, &GuildLine)>,
) {
    let open = mgr.is_open(DialogKind::Guild);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Guild);
        }
    }
    // 第一行显示行会名，其余显示成员
    for (mut text, line) in &mut lines {
        if line.0 == 0 {
            text.0 = if guild.name.is_empty() {
                "（未加入行会）".to_string()
            } else {
                format!("【{}】", guild.name)
            };
        } else {
            text.0 = guild.members.get(line.0 - 1).cloned().unwrap_or_default();
        }
    }
}
