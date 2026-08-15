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

use crate::game::dialogs::inventory::{InvItem, InventoryOrigin};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{spawn_ui_sprite, ui_button_system, ui_image, UiButton, UiImageCache};

/// 背包背景 Title[196] 缺失时的兜底尺寸（真实值运行时从库读取）
const INV_W_FALLBACK: f32 = 316.0;
const INV_H_FALLBACK: f32 = 236.0;

/// C# SocketDialog.Show(Inventory) 定位公式（SocketDialog.cs:108-110）：
/// x = inv.X + (inv.W - sock.W)/2，y = inv.Y + inv.H + 5 —— 全部用背包**真实**尺寸；
/// C# Point 是 int，除法整除截断（floor 复刻）。
/// 原点由调用方传入（背包**当前**位置——C# 动态读 InventoryDialog.Location，
/// 仓库/交易推位或拖动后跟随；初始位 = InventoryOrigin 默认 (0,0)）。
fn socket_origin(inv: (f32, f32), inv_w: f32, inv_h: f32, sock_w: f32) -> (f32, f32) {
    (
        inv.0 + ((inv_w - sock_w) / 2.0).floor(),
        inv.1 + inv_h + 5.0,
    )
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
    inv_origin: Res<InventoryOrigin>,
) {
    libs.0.ensure_initialized();

    // 面板（初始 1 孔，打开时按孔数换图并按背包真实尺寸重定位）
    let (inv_w, inv_h) = inventory_real_size(&mut libs);
    let (pw, ph) = match libs.0.get_image(LibraryName::Prguse3, 20) {
        Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
        None => (81.0, 62.0),
    };
    let (px, py) = socket_origin((inv_origin.0, inv_origin.1), inv_w, inv_h, pw);

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
    inv_origin: Res<InventoryOrigin>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    close: Query<&UiButton, With<SocketClose>>,
    // B0001：与 panel 双写 Transform，filter 必须可证互斥（Without<SocketPanel>）——
    // run_if 不拦 schedule 初始化期的访问集检查（详见 LESSON_Bevy同系统多查询双写组件）
    mut close_tf: Query<
        &mut Transform,
        (With<SocketClose>, Without<SocketCell>, Without<SocketPanel>),
    >,
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
        .unwrap_or(81.0); // Prguse3 缺失兜底：1 孔面板宽（最小情形）
    let (px, py) = socket_origin((inv_origin.0, inv_origin.1), inv_w, inv_h, w);
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
    /// 背包 Title[196] 实测 316x236、原点 (0,0) → 面板 y=241（inv.Y+inv.H+5）、x 随宽度居中
    /// **整除**（C# Point 是 int：(316-81)/2=117 非 117.5）。
    /// 旧实现 y=207（格子底 +5）、x 用硬编码 280 —— 均与 C# 不符。
    #[test]
    fn socket_origin_matches_csharp_show() {
        // 原点 (0,0)（默认背包原点，InventoryOrigin 初始值）
        // 1 孔面板宽 81（Prguse3[20] 实测）
        assert_eq!(
            socket_origin((0.0, 0.0), 316.0, 236.0, 81.0),
            (117.0, 241.0)
        );
        // 12 孔面板宽 268（Prguse3[31]）
        assert_eq!(
            socket_origin((0.0, 0.0), 316.0, 236.0, 268.0),
            (24.0, 241.0)
        );
        // 关闭钮跟随实际宽度：w-23（spawn 与运行时同步该公式）
        assert_eq!(
            socket_origin((0.0, 0.0), 316.0, 236.0, 81.0).0 + 81.0 - 23.0,
            175.0
        );
        // 背包被推位后（仓库推位 STORAGE_W+5=393 或交易推位 1024-316=708）面板跟随：
        // 仓库推位 x=393+117=510；交易推位 x=708+24=732（12 孔面板）
        assert_eq!(socket_origin((393.0, 0.0), 316.0, 236.0, 81.0).0, 510.0);
        assert_eq!(socket_origin((708.0, 0.0), 316.0, 236.0, 268.0).0, 732.0);
        // 拖动背包 (100,50) 后 y=50+236+5=291
        assert_eq!(socket_origin((100.0, 50.0), 316.0, 236.0, 81.0).1, 291.0);
    }

    /// B0001 冒烟（PR #2553 审查实证：close_tf 与 panel 双写 Transform 若 filter 不互斥，
    /// schedule 初始化期即 panic，run_if 不拦、单元测试全绿是盲区）：
    /// 注册 SocketPlugin 的 App 必须能 update 而不 panic。
    #[test]
    fn socket_plugin_updates_without_b0001() {
        let mut app = bevy::app::App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin::default(),
            bevy::input::InputPlugin,
            bevy::state::app::StatesPlugin,
        ));
        app.init_state::<crate::scenes::AppState>();
        app.init_asset::<Image>();
        app.init_resource::<crate::ui::sprite_ui::UiImageCache>();
        app.insert_resource(crate::map_renderer::GameLibraries(
            crate::resources::libraries::Libraries::new(
                crate::resources::libraries::resolve_data_path(),
            ),
        ));
        app.init_resource::<DialogManager>();
        // socket_ui_system/spawn_socket 读 InventoryOrigin（背包推位/拖动原点）
        app.init_resource::<crate::game::dialogs::inventory::InventoryOrigin>();
        app.add_plugins(SocketPlugin);
        // 非 Game 状态 + 切到 Game 各跑一帧（两阶段都做：B0001 检查发生在
        // schedule 初始化，与 run_if 是否命中无关）
        app.update();
        app.world_mut()
            .resource_mut::<NextState<crate::scenes::AppState>>()
            .set(crate::scenes::AppState::Game);
        app.update();
        app.update();
    }
}
