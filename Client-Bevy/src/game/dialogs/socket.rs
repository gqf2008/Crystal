// ============================================================================
// 镶嵌（宝石槽）对话框（M56）
// 参考：C# SocketDialog（Client/MirScenes/Dialogs/SocketDialog.cs）
//   - 面板 Prguse3[20 + 孔数-1]（1-6 孔 81-268x62；7-12 孔 268x95）
//   - 12 个镶嵌格（6x2，C# 位置 x*36+23+x, y*33+15+y），显示孔内宝石图标
//   - 关闭按钮 Prguse2[360/361/362]（W-23, 3）
//   - 打开方式：背包/装备 Ctrl+右键（C# MirItemCell.OpenItem）
// 纯客户端：数据来自物品 slots（UserInformation 下发）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::inventory::{
    InvItem, DIALOG_X as INV_DIALOG_X, DIALOG_Y as INV_DIALOG_Y,
};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{spawn_ui_sprite, ui_button_system, ui_image, UiButton, UiImageCache};

/// 背包对话框原点（用于定位镶嵌面板，跟随背包原点常量防漂移）
const INV_X: f32 = INV_DIALOG_X;
const INV_Y: f32 = INV_DIALOG_Y;
/// 背包背景 Title[196] 缺失时的兜底尺寸（真实值运行时从库读取）
const INV_W_FALLBACK: f32 = 316.0;
const INV_H_FALLBACK: f32 = 236.0;

/// C# SocketDialog.Show(Inventory) 定位公式（SocketDialog.cs:108-110）：
/// x = inv.X + (inv.W - sock.W)/2，y = inv.Y + inv.H + 5 —— 全部用背包**真实**尺寸。
fn socket_origin(inv_w: f32, inv_h: f32, sock_w: f32) -> (f32, f32) {
    (INV_X + (inv_w - sock_w) / 2.0, INV_Y + inv_h + 5.0)
}

/// 背包背景 Title[196] 真实尺寸（缺失回退 316x236 实测值）
fn inventory_real_size(libs: &mut GameLibraries) -> (f32, f32) {
    match libs.0.get_image(LibraryName::Title, 196) {
        Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
        None => (INV_W_FALLBACK, INV_H_FALLBACK),
    }
}

/// 镶嵌状态（当前展示的物品）
#[derive(Resource, Default)]
pub struct SocketState {
    pub item: Option<InvItem>,
}

#[derive(Component)]
pub struct SocketWidget;

#[derive(Component)]
pub struct SocketClose;

/// 面板背景（按孔数换 Prguse3 索引）
#[derive(Component)]
pub struct SocketPanel;

/// 镶嵌格（index = 槽位）
#[derive(Component)]
pub struct SocketCell(pub usize);

pub struct SocketPlugin;

impl Plugin for SocketPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SocketState>();
        app.add_systems(OnEnter(AppState::Game), spawn_socket);
        app.add_systems(OnExit(AppState::Game), cleanup_socket);
        app.add_systems(
            Update,
            (socket_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_socket(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_socket(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
) {
    libs.0.ensure_initialized();

    // 面板（初始 1 孔，打开时按孔数换图并按背包真实尺寸重定位）
    let (inv_w, inv_h) = inventory_real_size(&mut libs);
    let (pw, ph) = match libs.0.get_image(LibraryName::Prguse3, 20) {
        Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
        None => (81.0, 62.0),
    };
    let (px, py) = socket_origin(inv_w, inv_h, pw);

    let white = images.add(crate::map_renderer::make_image(
        vec![255, 255, 255, 255],
        1,
        1,
    ));
    let panel = spawn_ui_sprite(&mut commands, white.clone(), px, py, 6.0, 1.0);
    commands.entity(panel).insert((
        Sprite {
            image: white.clone(),
            custom_size: Some(Vec2::new(pw, ph)),
            ..default()
        },
        SocketPanel,
        DialogRoot(DialogKind::Socket),
        SocketWidget,
        Visibility::Hidden,
    ));

    // 关闭按钮（W-23, 3）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Prguse2,
        360,
        361,
        362,
        px + pw - 23.0,
        py + 3.0,
        7.0,
        24.0,
        21.0,
    ) {
        commands.entity(e).insert((
            SocketClose,
            DialogRoot(DialogKind::Socket),
            SocketWidget,
            Visibility::Hidden,
        ));
    }

    // 12 个镶嵌格（6x2；C# x*36+23+x, y*33+15+y）
    for idx in 0..12usize {
        let x = (idx % 6) as f32;
        let y = (idx / 6) as f32;
        let cell_x = x * 36.0 + 23.0 + x;
        let cell_y = y * 33.0 + 15.0 + y;
        let e = spawn_ui_sprite(
            &mut commands,
            white.clone(),
            px + cell_x,
            py + cell_y,
            6.1,
            1.0,
        );
        commands.entity(e).insert((
            Sprite {
                image: white.clone(),
                custom_size: Some(Vec2::new(30.0, 30.0)),
                ..default()
            },
            SocketCell(idx),
            DialogRoot(DialogKind::Socket),
            SocketWidget,
            Visibility::Hidden,
        ));
    }
}

fn socket_ui_system(
    mut mgr: ResMut<DialogManager>,
    state: Res<SocketState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    close: Query<&UiButton, With<SocketClose>>,
    mut close_tf: Query<&mut Transform, (With<SocketClose>, Without<SocketCell>)>,
    mut widgets: Query<&mut Visibility, (With<SocketWidget>, Without<SocketCell>)>,
    mut cells: Query<(&mut Visibility, &mut Sprite, &mut Transform, &SocketCell)>,
    mut panel: Query<(&mut Sprite, &mut Transform), (With<SocketPanel>, Without<SocketCell>)>,
    mut logged: Local<bool>,
) {
    let open = mgr.is_open(DialogKind::Socket);
    for mut vis in &mut widgets {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        *logged = false;
        return;
    }

    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Socket);
        }
    }

    let slots = state.item.as_ref().map(|i| i.slots.len()).unwrap_or(0);
    let slot_count = slots.clamp(1, 12);

    // 面板按孔数换图 + 按背包真实尺寸重定位（C# SocketDialog.Show：
    // x = inv.X+(inv.W-w)/2、y = inv.Y+inv.H+5、CloseButton = w-23 —— 关闭钮随实际宽度）
    let (inv_w, inv_h) = inventory_real_size(&mut libs);
    let idx = 20 + slot_count - 1;
    let w = libs
        .0
        .get_image(LibraryName::Prguse3, idx)
        .map(|i| i.width.max(0) as f32)
        .unwrap_or(INV_W_FALLBACK);
    let (px, py) = socket_origin(inv_w, inv_h, w);
    if let Ok((mut sprite, mut tf)) = panel.single_mut() {
        if let Some(h) = ui_image(
            &mut libs,
            &mut images,
            &mut cache,
            LibraryName::Prguse3,
            idx,
        ) {
            if sprite.image != h {
                sprite.image = h.clone();
                sprite.custom_size = None;
            }
        }
        tf.translation.x = px;
        tf.translation.y = -py;
    }
    for mut tf in &mut close_tf {
        tf.translation.x = px + w - 23.0;
        tf.translation.y = -(py + 3.0);
    }

    // 镶嵌格：idx < 孔数 且 有宝石 → 显示宝石图标；否则隐藏；位置随面板原点
    for (mut vis, mut sprite, mut tf, cell) in &mut cells {
        let gx = (cell.0 % 6) as f32;
        let gy = (cell.0 / 6) as f32;
        tf.translation.x = px + gx * 36.0 + 23.0 + gx;
        tf.translation.y = -(py + gy * 33.0 + 15.0 + gy);
        let gem = state
            .item
            .as_ref()
            .and_then(|i| i.slots.get(cell.0))
            .and_then(|s| s.as_ref());
        let mut show = false;
        if cell.0 < slot_count {
            if let Some(g) = gem {
                if let Some(h) = ui_image(
                    &mut libs,
                    &mut images,
                    &mut cache,
                    LibraryName::Items,
                    g.image as usize,
                ) {
                    sprite.image = h;
                    sprite.custom_size = None;
                    show = true;
                }
            }
        }
        *vis = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // 打开时日志（E2E 证据）
    if !*logged {
        if let Some(item) = &state.item {
            let gems: Vec<String> = item
                .slots
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    s.as_ref()
                        .map(|g| format!("{}#{}={}", i, g.item_index, g.name))
                        .unwrap_or_else(|| format!("{}#空", i))
                })
                .collect();
            tracing::info!(
                "💎 镶嵌面板: {} ({} 孔) {}",
                item.name,
                item.slots.len(),
                gems.join(", ")
            );
        }
        *logged = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// C# SocketDialog.Show(Inventory)（SocketDialog.cs:108-110）公式锚点：
    /// 背包 Title[196] 实测 316x236、原点 (0,0) → 面板 y=241（inv.Y+inv.H+5）、x 随宽度居中。
    /// 旧实现 y=207（格子底 +5）、x 用硬编码 280 —— 均与 C# 不符。
    #[test]
    fn socket_origin_matches_csharp_show() {
        // 1 孔面板宽 81（Prguse3[20] 实测）
        assert_eq!(socket_origin(316.0, 236.0, 81.0), (117.5, 241.0));
        // 12 孔面板宽 268（Prguse3[31]）
        assert_eq!(socket_origin(316.0, 236.0, 268.0), (24.0, 241.0));
        // 关闭钮跟随实际宽度：w-23（spawn 与运行时同步该公式）
        assert_eq!(socket_origin(316.0, 236.0, 81.0).0 + 81.0 - 23.0, 175.5);
    }
}
