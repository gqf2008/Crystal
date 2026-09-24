// ============================================================================
// 坐骑对话框（M60）
// 参考：C# MountDialog（Client/MirScenes/Dialogs/MountDialog.cs）
//   - 面板 Prguse[160/167]（按孔数 4/5 切换）@ (10,30)
//   - 名称/忠诚度标签、骑乘按钮 Prguse[155/156/157] (262,70)、关闭/帮助
//   - 坐骑装备栏 5 格（Reins/Bells/Saddle/Ribbon/Mask @ (36/90/144/198/252, 323)）
//   - 骑乘按钮 → Chat "@ride"（C# Ride()；服务端 RIDE 命令切换 + 广播外观）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_label_center, spawn_panel,
    CloseButton,
};

/// #2892 批B：面板精灵与 C# 原生尺寸/坐标（C# `MountDialog.Index = 167; Location = (10,30)`；
/// 4 孔坐骑用 `Prguse[160]`，运行时按 `slots.len()` 切换）
pub const PANEL: (LibraryName, usize) = (LibraryName::Prguse, 167);
pub const PANEL_4SLOT: (LibraryName, usize) = (LibraryName::Prguse, 160);
pub const PANEL_SIZE: (f32, f32) = (324.0, 377.0);
pub const PANEL_POS: (f32, f32) = (10.0, 30.0);

/// C# `MountDialog.cs` 5 孔档的标签几何（防漂移常量）：`MountName` @(30,10)、
/// `MountLoyalty` @(30,30)，两者 `Size(260,15)` + `DrawFormat = HCenter|VCenter`。
/// 原实现放在 (30,40)/(30,60) 且左对齐 → 名字压在标题栏边框上。
pub const LABEL_LEFT: f32 = 30.0;
pub const LABEL_WIDTH: f32 = 260.0;
pub const NAME_Y: f32 = 10.0;
pub const LOYALTY_Y: f32 = 30.0;
const PANEL_X: f32 = 10.0;
const PANEL_Y: f32 = 30.0;

/// 坐骑立绘（C# `MountDialog.MountImage`，`MountDialog.cs:80-91`）：**16 帧、100ms/帧、循环**，
/// `Index = StartIndex + MountType*20`（`DrawMountAnimation`，`:204-225`）；控件是
/// `MirAnimatedControl { UseOffSet = true }` ⇒ 绘制点 = `Location + 该帧艺术偏移`
/// （`MirImageControl.cs:7`）。`MountType` 就是**装备中坐骑物品的 Shape**
/// （`UserObject.cs:249` `MountType = realItem.Shape`），无坐骑时 -1 ⇒ `Index = 0` 且不动画。
///
/// `StartIndex` 与位置按坐骑孔数（`SwitchType`，`:163-195`）：
/// 4 孔 ⇒ 1170 @ (110,250)；5 孔 ⇒ 1330 @ (0,70)。
pub const MOUNT_PORTRAIT_FRAMES: usize = 16;
pub const MOUNT_PORTRAIT_DELAY_MS: f32 = 100.0;
pub const MOUNT_PORTRAIT_START_4SLOT: usize = 1170;
pub const MOUNT_PORTRAIT_START_5SLOT: usize = 1330;
pub const MOUNT_PORTRAIT_POS_4SLOT: (f32, f32) = (110.0, 250.0);
pub const MOUNT_PORTRAIT_POS_5SLOT: (f32, f32) = (0.0, 70.0);

/// 立绘帧号（纯函数，便于门禁）：`mount_shape < 0`（未装坐骑）⇒ `None`（原版不播动画）。
pub fn mount_portrait_frame(start_index: usize, mount_shape: i16, frame: usize) -> Option<usize> {
    if mount_shape < 0 {
        return None;
    }
    Some(start_index + mount_shape as usize * 20 + frame % MOUNT_PORTRAIT_FRAMES)
}

/// 立绘绘制点 = `Location + 艺术偏移`（C# `UseOffSet=true`；**漏这一项就会整体偏右下**，
/// 与宠物立绘 `69bf6e58` 是同一类缺陷）。
pub fn mount_portrait_draw_pos(pos: (f32, f32), offset: (i16, i16)) -> (f32, f32) {
    (pos.0 + offset.0 as f32, pos.1 + offset.1 as f32)
}

/// 当前动画帧序号（按毫秒换帧；C# `AnimationDelay = 100`）。
pub fn mount_portrait_anim_frame(elapsed_ms: f32) -> usize {
    ((elapsed_ms / MOUNT_PORTRAIT_DELAY_MS).max(0.0) as usize) % MOUNT_PORTRAIT_FRAMES
}

#[derive(Component)]
pub struct MountWidget;

#[derive(Component)]
pub struct MountClose;

#[derive(Component)]
pub struct MountRide;

#[derive(Component)]
pub struct MountPanel;

#[derive(Component)]
pub struct MountNameText;

#[derive(Component)]
pub struct MountLoyaltyText;

/// 坐骑装备格（index = Reins/Bells/Saddle/Ribbon/Mask）
#[derive(Component)]
pub struct MountGearCell(pub usize);

/// 坐骑宝石图标（格子子节点）
#[derive(Component)]
pub struct MountGearIcon(pub usize);

/// 坐骑立绘实体（C# `MountDialog.MountImage`）
#[derive(Component)]
pub struct MountPortrait;

pub struct MountPlugin;

impl Plugin for MountPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Game), spawn_mount);
        app.add_systems(OnExit(AppState::Game), cleanup_mount);
        app.add_systems(
            Update,
            (mount_ui_system,).chain().run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_mount(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_mount(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut cjk_font: ResMut<UiCjkFont>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();
    let cjk = shared_cjk_font(&mut fonts, &mut cjk_font);
    let white = images.add(crate::map_renderer::make_image(
        vec![255, 255, 255, 255],
        1,
        1,
    ));

    // 面板（默认 5 孔 167；ui_system 按孔数换 Prguse[160/167]）
    let panel = spawn_panel(
        &mut commands,
        white.clone(),
        PANEL_X,
        PANEL_Y,
        PANEL_SIZE.0,
        PANEL_SIZE.1,
        30,
    );
    commands
        .entity(panel)
        .insert((MountPanel, DialogRoot(DialogKind::Mount), MountWidget));

    commands.entity(panel).with_children(|p| {
        // 名称/忠诚度：C# `MountDialog.cs` 5 孔档 —— `MountName` @(30,10) `Size(260,15)`、
        // `MountLoyalty` @(30,30) `Size(260,15)`，两者 `DrawFormat = HCenter|VCenter`。
        // 原实现放在 (30,40)/(30,60) 且**左对齐** → 名字压在标题栏边框上、文字整体偏左偏下。
        // 居中标签的 cx = 30 + 260/2 = 160。
        let cx = LABEL_LEFT + LABEL_WIDTH / 2.0; // 260 宽居中 → cx = 160
        spawn_label_center(p, &cjk, "", cx, NAME_Y, LABEL_WIDTH, 15.0, Color::WHITE, 9)
            .insert(MountNameText);
        spawn_label_center(
            p,
            &cjk,
            "",
            cx,
            LOYALTY_Y,
            LABEL_WIDTH,
            12.0,
            Color::WHITE,
            9,
        )
        .insert(MountLoyaltyText);
        // 骑乘按钮 Prguse[155/156/157] @(262,70)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 155),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 156),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 157),
        ) {
            spawn_icon_button(p, n, h, pr, 262.0, 70.0, 36.0, 32.0, 10).insert(MountRide);
        }
        // 关闭 Prguse2[360/361/362] @(297,3)、帮助 Prguse2[257/258/259] @(274,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 297.0, 3.0, 24.0, 21.0, 10)
                .insert((MountClose, CloseButton));
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 257),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 258),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 259),
        ) {
            spawn_icon_button(p, n, h, pr, 274.0, 3.0, 24.0, 21.0, 10);
        }
        // 坐骑装备格 5 个 @(36/90/144/198/252, 323)
        // 坐骑立绘（C# `MountDialog.MountImage`）：位置/帧号在 `mount_ui_system` 里按
        // 孔数与坐骑 Shape 计算，这里先放一个占位（`ZIndex(8)` 垫在标签 z=9 / 格子 z=10 之下）
        p.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(MOUNT_PORTRAIT_POS_5SLOT.0),
                top: Val::Px(MOUNT_PORTRAIT_POS_5SLOT.1),
                ..default()
            },
            ImageNode::new(white.clone()),
            MountPortrait,
            Visibility::Hidden,
            ZIndex(8),
        ));
        for i in 0..5usize {
            let x = 36.0 + i as f32 * 54.0;
            spawn_container(p, x, 323.0, 34.0, 30.0, 9)
                .insert((
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.25)),
                    MountGearCell(i),
                ))
                .with_children(|gc| {
                    gc.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(0.0),
                            width: Val::Px(34.0),
                            height: Val::Px(30.0),
                            ..default()
                        },
                        ImageNode::new(white.clone()),
                        MountGearIcon(i),
                        Visibility::Hidden,
                        ZIndex(10),
                    ));
                });
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn mount_ui_system(
    mut mgr: ResMut<DialogManager>,
    loadout_q: Query<&crate::game::player_state::Loadout, With<crate::actor::LocalPlayer>>,
    net: ResMut<NetConnection>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    close: Query<(Entity, &Interaction), With<MountClose>>,
    ride: Query<(Entity, &Interaction), With<MountRide>>,
    mut widgets: Query<
        &mut Visibility,
        (
            With<MountWidget>,
            Without<MountGearCell>,
            Without<MountGearIcon>,
            Without<MountPortrait>,
        ),
    >,
    mut panel: Query<
        (&mut ImageNode, &mut Node, &MountPanel),
        (Without<MountGearIcon>, Without<MountPortrait>),
    >,
    mut names: Query<(&mut Text, Option<&MountNameText>, Option<&MountLoyaltyText>)>,
    mut gears: Query<(&mut Visibility, &mut ImageNode, &MountGearIcon)>,
    // 立绘：写 ImageNode/Node/Visibility——用 MountPortrait 与上面几个查询互斥（B0001 硬要求）
    mut portrait: Query<
        (&mut ImageNode, &mut Node, &mut Visibility, &MountPortrait),
        (
            Without<MountPanel>,
            Without<MountGearIcon>,
            Without<MountGearCell>,
        ),
    >,
    time: Res<Time>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
    mut logged: Local<bool>,
    mut logged_portrait: Local<bool>,
) {
    use mir2_shared::packets::client::chat::Chat;
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }

    let open = mgr.is_open(DialogKind::Mount);
    for mut vis in widgets.iter_mut() {
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
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::Mount);
        }
    }
    for (e, inter) in &ride {
        if edge(e, inter, &mut prev_inter) {
            net.send_packet(&Chat {
                message: "@ride".to_string(),
                linked_items: Vec::new(),
            });
            tracing::info!("🐴 请求骑乘/下马 (@ride)");
        }
    }

    // 坐骑物品 = 装备槽 10（Mount；#2633 批次4 步6 读 Loadout 组件）
    let mount = loadout_q
        .single()
        .ok()
        .and_then(|l| l.slots.get(10))
        .and_then(|s| s.as_ref());

    // 面板按坐骑孔数换图（4→160, 5→167）
    let slot_count = mount.map(|m| m.slots.len()).unwrap_or(0);
    if let Ok((mut node, mut layout, _)) = panel.single_mut() {
        let idx = if slot_count == 4 { 160 } else { 167 };
        // #2892 批B：C# `MirImageControl.Size` 跟随图片——`Prguse[160]`(272x378) 与
        // `Prguse[167]`(324x377) 尺寸不同，换图时必须同步节点尺寸，否则 4 孔坐骑被拉伸到 324 宽
        let (iw, ih) = libs
            .0
            .get_image(LibraryName::Prguse, idx)
            .map(|i| (i.width.max(0) as f32, i.height.max(0) as f32))
            .unwrap_or(PANEL_SIZE);
        if layout.width != Val::Px(iw) {
            layout.width = Val::Px(iw);
        }
        if layout.height != Val::Px(ih) {
            layout.height = Val::Px(ih);
        }
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, idx) {
            if node.image != h {
                node.image = h;
            }
        }
    }

    // 坐骑立绘（C# `MountDialog.MountImage` + `DrawMountAnimation`）：
    // 帧号 = StartIndex + Shape*20 + 动画帧；绘制点 = Location + 该帧艺术偏移（UseOffSet）。
    let (start_index, portrait_pos) = if slot_count == 4 {
        (MOUNT_PORTRAIT_START_4SLOT, MOUNT_PORTRAIT_POS_4SLOT)
    } else {
        (MOUNT_PORTRAIT_START_5SLOT, MOUNT_PORTRAIT_POS_5SLOT)
    };
    let anim_frame = mount_portrait_anim_frame(time.elapsed_secs() * 1000.0);
    let shape = mount.map(|m| m.shape).unwrap_or(-1);
    if let Ok((mut p_node, mut p_layout, mut p_vis, _)) = portrait.single_mut() {
        match mount_portrait_frame(start_index, shape, anim_frame) {
            Some(idx) => {
                // 艺术偏移与尺寸都取自库本身：C# `MirAnimatedControl` 按帧 `UseOffSet` 定位，
                // 且控件尺寸跟随 art（`MirImageControl.Size`）——Bevy 侧必须显式给 Node 尺寸，
                // 否则没有内容尺寸的 `ImageNode` 会以 0 尺寸渲染（第一版就踩了这条：立绘不显示）。
                let info = libs.0.get_image(LibraryName::Prguse, idx);
                let offset = info
                    .as_ref()
                    .map(|i| (i.offset_x, i.offset_y))
                    .unwrap_or((0, 0));
                let (iw, ih) = info
                    .as_ref()
                    .map(|i| (i.width.max(0) as f32, i.height.max(0) as f32))
                    .unwrap_or((1.0, 1.0));
                let (dx, dy) = mount_portrait_draw_pos(portrait_pos, offset);
                if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, idx) {
                    if p_node.image != h {
                        p_node.image = h;
                    }
                }
                if p_layout.left != Val::Px(dx) {
                    p_layout.left = Val::Px(dx);
                }
                if p_layout.top != Val::Px(dy) {
                    p_layout.top = Val::Px(dy);
                }
                if p_layout.width != Val::Px(iw) {
                    p_layout.width = Val::Px(iw);
                }
                if p_layout.height != Val::Px(ih) {
                    p_layout.height = Val::Px(ih);
                }
                *p_vis = Visibility::Visible;
            }
            // C# 无坐骑时 `Index = 0 && Animated = false`（Prguse[0] 是空白帧）→ 本端直接隐藏
            None => *p_vis = Visibility::Hidden,
        }
        if !*logged_portrait {
            *logged_portrait = true;
            tracing::info!(
                "🐴 立绘: idx={:?} shape={} frame={} offset={:?} size=({:?},{:?}) pos=({:?},{:?}) vis={:?}",
                mount_portrait_frame(start_index, shape, anim_frame),
                shape,
                anim_frame,
                libs.0
                    .get_image(
                        LibraryName::Prguse,
                        mount_portrait_frame(start_index, shape, anim_frame).unwrap_or(0)
                    )
                    .map(|i| (i.offset_x, i.offset_y)),
                p_layout.width,
                p_layout.height,
                p_layout.left,
                p_layout.top,
                *p_vis,
            );
        }
    } else {
        tracing::warn!("🐴 立绘实体缺失（MountPortrait 查询失败）");
    }

    // 名称/忠诚度
    for (mut text, name, loyalty) in &mut names {
        if name.is_some() {
            text.0 = mount.map(|m| m.name.clone()).unwrap_or_default();
        } else if loyalty.is_some() {
            text.0 = mount
                .map(|m| format!("忠诚度 {}/{}", m.current_dura, m.max_dura))
                .unwrap_or_default();
        }
    }

    // 装备格：坐骑 gem 图标（slots[0..4]）
    for (mut vis, mut node, cell) in &mut gears {
        let gem = mount
            .and_then(|m| m.slots.get(cell.0))
            .and_then(|s| s.as_ref());
        let mut show = false;
        if let Some(g) = gem {
            if let Some(h) =
                load_lib_image(&mut libs, &mut images, LibraryName::Items, g.image as usize)
            {
                node.image = h;
                show = true;
            }
        }
        *vis = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    if !*logged {
        match mount {
            Some(m) => tracing::info!(
                "🐴 坐骑: {} (耐久 {}/{}, {} 孔, 鞍={})",
                m.name,
                m.current_dura,
                m.max_dura,
                m.slots.len(),
                m.slots.get(2).and_then(|s| s.as_ref()).is_some()
            ),
            None => tracing::info!("🐴 坐骑: 未装备"),
        }
        *logged = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2985 B1：坐骑面板标签几何必须对齐 C# `MountDialog.cs` 5 孔档。原实现把名字/忠诚
    /// 放在 (30,40)/(30,60) 且左对齐，实机表现为**名字压在标题栏边框上、文字偏左偏下**。
    #[test]
    fn mount_label_geometry_matches_csharp() {
        assert_eq!(LABEL_LEFT, 30.0, "C# MountName.Location.X = 30");
        assert_eq!(LABEL_WIDTH, 260.0, "C# MountName.Size.Width = 260");
        assert_eq!(NAME_Y, 10.0, "C# MountName.Location.Y = 10");
        assert_eq!(LOYALTY_Y, 30.0, "C# MountLoyalty.Location.Y = 30");
        assert_eq!(
            LABEL_LEFT + LABEL_WIDTH / 2.0,
            160.0,
            "居中锚点 = 30 + 260/2（C# HCenter）"
        );
    }

    /// 门禁（owner 队列 `mount-image-preview`）：坐骑立绘三件事照 C#——
    /// 帧号 `StartIndex + Shape*20 + frame`（`MountDialog.cs:213`）、16 帧 100ms 循环（`:83-84`）、
    /// 绘制点 = `Location + 艺术偏移`（`UseOffSet=true`，`:90`；`MirImageControl.cs:7`）。
    ///
    /// 阳性对照：把 `mount_portrait_draw_pos` 里的 offset 去掉（直接回 `pos`）→ 第 5 条断言立即红。
    #[test]
    fn mount_portrait_matches_csharp() {
        // 帧号：5 孔档 StartIndex=1330，Shape=3 → 1330 + 60 + frame
        assert_eq!(
            mount_portrait_frame(MOUNT_PORTRAIT_START_5SLOT, 3, 0),
            Some(1390)
        );
        assert_eq!(
            mount_portrait_frame(MOUNT_PORTRAIT_START_5SLOT, 3, 15),
            Some(1405)
        );
        assert_eq!(
            mount_portrait_frame(MOUNT_PORTRAIT_START_5SLOT, 3, 16),
            Some(1390),
            "16 帧循环"
        );
        // 未装坐骑：C# `MountType < 0` ⇒ `Index = 0 && Animated = false`
        assert_eq!(
            mount_portrait_frame(MOUNT_PORTRAIT_START_5SLOT, -1, 0),
            None
        );
        // UseOffSet：绘制点必须叠艺术偏移（实测 5 孔档 offset (-86,-106) ⇒ (0,70)+偏移 = (-86,-36)）
        assert_eq!(
            mount_portrait_draw_pos(MOUNT_PORTRAIT_POS_5SLOT, (-86, -106)),
            (-86.0, -36.0),
            "C# `DisplayLocation = Location + Library.GetOffSet(Index)`"
        );
        // 4/5 孔档常量（`SwitchType`，`:167/170/182/185`）
        assert_eq!(
            (MOUNT_PORTRAIT_START_4SLOT, MOUNT_PORTRAIT_POS_4SLOT),
            (1170, (110.0, 250.0))
        );
        assert_eq!(
            (MOUNT_PORTRAIT_START_5SLOT, MOUNT_PORTRAIT_POS_5SLOT),
            (1330, (0.0, 70.0))
        );
        // 帧率：100ms/帧、16 帧循环（`AnimationDelay = 100` / `AnimationCount = 16`）
        assert_eq!(mount_portrait_anim_frame(0.0), 0);
        assert_eq!(mount_portrait_anim_frame(99.0), 0);
        assert_eq!(mount_portrait_anim_frame(150.0), 1);
        assert_eq!(mount_portrait_anim_frame(1600.0), 0);
    }
}
