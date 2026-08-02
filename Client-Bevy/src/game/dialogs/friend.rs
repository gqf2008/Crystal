// ============================================================================
// 好友对话框（M25）
// 布局参考：C# FriendDialog.cs / macroquad friend_dialog.rs
//   - 背景 Title[199]，位置 (300,100)，标题 Title[6] (18,9)
//   - 好友列表 y=40 每 20px；打开时自动请求 C.RefreshFriends
// 网络：FriendUpdate（列表 / 单个添加，同 opcode 双格式）→ 列表渲染（在线/离线）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetworkContext;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};

/// 好友条目
#[derive(Debug, Clone, Default)]
pub struct FriendEntry {
    pub object_id: u32,
    pub name: String,
    pub memo: String,
    pub online: bool,
}

/// 好友状态（网络 FriendUpdate 写入）
#[derive(Resource, Default)]
pub struct FriendState {
    pub friends: Vec<FriendEntry>,
}

#[derive(Component)]
pub struct FriendWidget;

#[derive(Component)]
pub struct FriendClose;

#[derive(Component)]
pub struct FriendLine(usize);

pub struct FriendPlugin;

impl Plugin for FriendPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FriendState>();
        app.add_systems(OnEnter(AppState::Game), spawn_friend);
        app.add_systems(OnExit(AppState::Game), cleanup_friend);
        app.add_systems(
            Update,
            (friend_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_friend(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_friend(
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

    // 背景 Title[199]
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 199) {
        let e = spawn_ui_sprite(&mut commands, h, 300.0, 100.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Friend),
            FriendWidget,
            Visibility::Hidden,
        ));
    }
    // 标题 Title[6]
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 6) {
        let e = spawn_ui_sprite(&mut commands, h, 318.0, 109.0, 6.2, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Friend),
            FriendWidget,
            Visibility::Hidden,
        ));
    }
    // 关闭
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        300.0 + 206.0, 103.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            FriendClose,
            DialogRoot(DialogKind::Friend),
            FriendWidget,
        ));
    }
    // 好友列表（10 行）
    for i in 0..10usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            318.0, 140.0 + i as f32 * 20.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            FriendLine(i),
            DialogRoot(DialogKind::Friend),
            FriendWidget,
        ));
    }
}

/// 显隐 + 列表渲染 + 打开时自动请求刷新（原版 C# FriendDialog.Show → RefreshFriends）
fn friend_ui_system(
    mut mgr: ResMut<DialogManager>,
    friend: Res<FriendState>,
    net: Res<NetworkContext>,
    close: Query<&UiButton, With<FriendClose>>,
    mut widgets: Query<(&mut Visibility, Option<&FriendLine>), With<FriendWidget>>,
    mut lines: Query<(&mut Text2d, &FriendLine)>,
    mut requested: Local<bool>,
) {
    let open = mgr.is_open(DialogKind::Friend);
    for (mut vis, _line) in &mut widgets {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        *requested = false;
        return;
    }
    // 打开瞬间请求一次好友列表
    if !*requested {
        *requested = true;
        net.send_packet(&mir2_shared::packets::client::friend::RefreshFriends);
        tracing::info!("👥 请求刷新好友列表");
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Friend);
        }
    }
    // 列表（在线标记，原版 C# 语义）
    for (mut text, line) in &mut lines {
        text.0 = match friend.friends.get(line.0) {
            Some(f) => {
                let mark = if f.online { "（在线）" } else { "（离线）" };
                let name = if f.memo.is_empty() {
                    f.name.clone()
                } else {
                    format!("{} ({})", f.name, f.memo)
                };
                format!("{}{}", name, mark)
            }
            None => String::new(),
        };
    }
}