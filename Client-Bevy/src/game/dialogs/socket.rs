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
use crate::ui::theme::{load_lib_image, spawn_icon_button, spawn_image, CloseButton, ImageButton};

/// #2892 批B：面板精灵（C# `SocketDialog.Index = 20; Library = Libraries.Prguse3`）
pub const PANEL: (LibraryName, usize) = (LibraryName::Prguse3, 20);

/// 背包背景 Title[196] 缺失时的兜底尺寸（真实值运行时从库读取）
const INV_W_FALLBACK: f32 = 316.0;
const INV_H_FALLBACK: f32 = 236.0;

/// 人窗（C# `CharacterDialog.Index = 504`）的面板图号与兜底尺寸
/// （`CharacterDialog.cs:32-34`：`Title[504]` @ `(ScreenWidth-264, 0)`；实测 264x380）
pub const CHAR_PANEL_INDEX: usize = 504;
pub const CHAR_SIZE_FALLBACK: (f32, f32) = (264.0, 380.0);

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

/// 镶嵌面板的**来源格**（C# `SocketDialog.Show(MirGridType grid, UserItem item)`，
/// `SocketDialog.cs:88-124`）——两种来源**落点不同**：
///
/// | 来源 | C# 分支 | 公式（`:108-118`） |
/// |---|---|---|
/// | 背包 | `case MirGridType.Inventory` | `x = inv.X + (inv.W - w)/2`，`y = inv.Y + inv.H + 5` |
/// | 装备 | `case MirGridType.Equipment` | `x = char.X + (char.W - w)/2`，`y = char.Y + char.H + 5` |
///
/// 两扇宿主窗 C# 都是 `Movable = true`（`InventoryDialog` / `CharacterDialog.cs:35`），
/// 所以公式读的是**当前**位置——本端两处都从运行期 `Node.left/top` 取（见 `socket_ui_system`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SocketSource {
    #[default]
    Inventory,
    Equipment,
}

/// 纯函数：按来源算面板落点（C# 两条公式的逐值复刻；`Point` 是 int ⇒ 整除截断用 `floor`）。
/// `host` = 宿主窗当前原点，`host_size` = 宿主窗真实尺寸，`sock_w` = 面板当前宽度。
pub fn socket_origin_for(
    source: SocketSource,
    inv_origin: (f32, f32),
    inv_size: (f32, f32),
    char_origin: (f32, f32),
    char_size: (f32, f32),
    sock_w: f32,
) -> (f32, f32) {
    let (origin, size) = match source {
        SocketSource::Inventory => (inv_origin, inv_size),
        SocketSource::Equipment => (char_origin, char_size),
    };
    socket_origin(origin, size.0, size.1, sock_w)
}

/// 背包背景 Title[196] 真实尺寸（缺失回退 316x236 实测值）
fn inventory_real_size(libs: &mut GameLibraries) -> (f32, f32) {
    match libs.0.get_image(LibraryName::Title, 196) {
        Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
        None => (INV_W_FALLBACK, INV_H_FALLBACK),
    }
}

/// 镶嵌状态（当前展示的物品 + 来源格）
#[derive(Resource, Default)]
pub struct SocketState {
    pub item: Option<InvItem>,
    /// C# `SocketDialog.Show(grid, …)` 的 `grid`：决定面板贴在**背包**下还是**人窗**下
    pub source: SocketSource,
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
        app.add_systems(Update, socket_ui_system.run_if(in_state(AppState::Game)));
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
    inv_origin: Res<InventoryOrigin>,
) {
    libs.0.ensure_initialized();

    // 面板（初始 1 孔，打开时按孔数换图并按背包真实尺寸重定位；不加 Overflow::clip，
    // 关闭按钮 left=w-23 时右缘与面板齐平）
    let (inv_w, inv_h) = inventory_real_size(&mut libs);
    let (pw, ph) = match libs.0.get_image(LibraryName::Prguse3, 20) {
        Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
        None => (81.0, 62.0),
    };
    let (px, py) = socket_origin((inv_origin.0, inv_origin.1), inv_w, inv_h, pw);

    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse3, 20) else {
        return;
    };
    let panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(px),
                top: Val::Px(py),
                width: Val::Px(pw),
                height: Val::Px(ph),
                ..default()
            },
            ImageNode::new(bg),
            SocketPanel,
            DialogRoot(DialogKind::Socket),
            SocketWidget,
            GlobalZIndex(30),
            Visibility::Hidden,
        ))
        .id();

    commands.entity(panel).with_children(|p| {
        // 关闭按钮（C# W-23, 3）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, pw - 23.0, 3.0, 24.0, 21.0, 10)
                .insert((SocketClose, CloseButton));
        }
        // 12 个镶嵌格（6x2；C# x*36+23+x, y*33+15+y；白图占位，ui_system 换宝石图）
        let white = images.add(crate::map_renderer::make_image(
            vec![255, 255, 255, 255],
            1,
            1,
        ));
        for idx in 0..12usize {
            let x = (idx % 6) as f32;
            let y = (idx / 6) as f32;
            let cell_x = x * 36.0 + 23.0 + x;
            let cell_y = y * 33.0 + 15.0 + y;
            spawn_image(p, white.clone(), cell_x, cell_y, 30.0, 30.0, 9).insert(SocketCell(idx));
        }
    });
}

fn socket_ui_system(
    mut mgr: ResMut<DialogManager>,
    state: Res<SocketState>,
    inv_origin: Res<InventoryOrigin>,
    // C# `case MirGridType.Equipment`：面板贴 `CharacterDialog.Location`（该窗 `Movable = true`，
    // 所以取运行期 `Node.left/top` 而不是常量）
    // `Without<SocketClose>/Without<SocketPanel>`：本系统另有两处 `&mut Node`（关闭钮/面板），
    // 不加这两个过滤 Bevy 报 B0001（同一系统内 Node 的读/写访问无法证不相交）
    roots: Query<
        (&Node, &crate::game::dialogs::DialogRoot),
        (Without<SocketClose>, Without<SocketPanel>),
    >,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut close: Query<
        (Entity, &Interaction, &mut Node),
        (With<SocketClose>, Without<SocketCell>, Without<SocketPanel>),
    >,
    mut widgets: Query<&mut Visibility, (With<SocketWidget>, Without<SocketCell>)>,
    mut cells: Query<(&mut Visibility, &mut ImageNode, &SocketCell), Without<SocketPanel>>,
    mut panel: Query<
        (&mut Node, &mut ImageNode),
        (With<SocketPanel>, Without<SocketCell>, Without<SocketClose>),
    >,
    mut logged: Local<bool>,
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

    for (e, inter, _) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::Socket);
        }
    }

    let slots = state.item.as_ref().map(|i| i.slots.len()).unwrap_or(0);
    let slot_count = slots.clamp(1, 12);

    // 面板按孔数换图 + 按背包真实尺寸重定位（C# SocketDialog.Show：
    // x = inv.X+(inv.W-w)/2、y = inv.Y+inv.H+5、CloseButton = w-23 —— 关闭钮随实际宽度）
    let (inv_w, inv_h) = inventory_real_size(&mut libs);
    // 人窗（C# `CharacterDialog`）：原点取运行期，尺寸取 `Title[504]` 真实值（兜底 264x380）
    let char_origin = roots
        .iter()
        .find(|(_, r)| r.0 == DialogKind::Character)
        .map(|(n, _)| {
            (
                match n.left {
                    Val::Px(v) => v,
                    _ => crate::game::dialogs::character::DIALOG_X,
                },
                match n.top {
                    Val::Px(v) => v,
                    _ => crate::game::dialogs::character::DIALOG_Y,
                },
            )
        })
        .unwrap_or((
            crate::game::dialogs::character::DIALOG_X,
            crate::game::dialogs::character::DIALOG_Y,
        ));
    let char_size = match libs.0.get_image(LibraryName::Title, CHAR_PANEL_INDEX) {
        Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
        None => CHAR_SIZE_FALLBACK,
    };
    let idx = 20 + slot_count - 1;
    let w = libs
        .0
        .get_image(LibraryName::Prguse3, idx)
        .map(|i| i.width.max(0) as f32)
        .unwrap_or(81.0); // Prguse3 缺失兜底：1 孔面板宽（最小情形）
    let (px, py) = socket_origin_for(
        state.source,
        (inv_origin.0, inv_origin.1),
        (inv_w, inv_h),
        char_origin,
        char_size,
        w,
    );
    if let Ok((mut node, mut img)) = panel.single_mut() {
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse3, idx) {
            if img.image != h {
                img.image = h.clone();
            }
        }
        node.left = Val::Px(px);
        node.top = Val::Px(py);
        node.width = Val::Px(w);
        node.height = Val::Px(
            libs.0
                .get_image(LibraryName::Prguse3, idx)
                .map(|i| i.height.max(0) as f32)
                .unwrap_or(62.0),
        );
    }
    for (_, _, mut node) in &mut close {
        node.left = Val::Px(w - 23.0);
    }

    // 镶嵌格：idx < 孔数 且 有宝石 → 显示宝石图标；否则隐藏（相对面板子节点）
    for (mut vis, mut node, cell) in &mut cells {
        let gem = state
            .item
            .as_ref()
            .and_then(|i| i.slots.get(cell.0))
            .and_then(|s| s.as_ref());
        let mut show = false;
        if cell.0 < slot_count {
            if let Some(g) = gem {
                if let Some(h) =
                    load_lib_image(&mut libs, &mut images, LibraryName::Items, g.image as usize)
                {
                    node.image = h;
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

    /// 来源格决定宿主窗（C# `SocketDialog.Show(grid, …)`，`SocketDialog.cs:108-118`）：
    /// 背包贴 `InventoryDialog`、**装备贴 `CharacterDialog`**（`Location=(ScreenWidth-264,0)=(760,0)`、
    /// `Title[504]` 实测 264x380 ⇒ y=380+5=385）。C# `Point` 是 int ⇒ 整除截断。
    #[test]
    fn socket_origin_follows_source_grid_like_csharp_show() {
        let inv = ((0.0, 0.0), (316.0, 236.0));
        let ch = ((760.0, 0.0), (264.0, 380.0));
        // 背包来源：与旧口径逐值一致（y = 0+236+5 = 241）
        assert_eq!(
            socket_origin_for(SocketSource::Inventory, inv.0, inv.1, ch.0, ch.1, 81.0),
            (117.0, 241.0)
        );
        // 装备来源：x = 760 + (264-81)/2 = 760+91 = 851；y = 0+380+5 = 385
        assert_eq!(
            socket_origin_for(SocketSource::Equipment, inv.0, inv.1, ch.0, ch.1, 81.0),
            (851.0, 385.0)
        );
        // 12 孔面板 268 比人窗宽：x = 760 + floor((264-268)/2) = 760-2 = 758
        assert_eq!(
            socket_origin_for(SocketSource::Equipment, inv.0, inv.1, ch.0, ch.1, 268.0).0,
            758.0
        );
        // 宿主窗被拖动/推位后公式跟随（装备：人窗被拖到 (700,40)）
        assert_eq!(
            socket_origin_for(
                SocketSource::Equipment,
                inv.0,
                inv.1,
                (700.0, 40.0),
                ch.1,
                81.0
            ),
            (791.0, 425.0)
        );
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
