// ============================================================================
// 钓鱼对话框（M39）
// 参考：C# FishingDialog（Prguse[1340]）+ ServerRust tick_fishing / FishingCast
// 网络：
//   C: FishingCast[fishing_type u8] / FishingChangeAutocast[enabled u8]
//   S: FishingUpdate(198)[progress i32][success u8]
// 收获结果通过系统聊天消息返回（C# ReceiveChat 语义）
// ============================================================================

use bevy::prelude::*;

use crate::actor::LocalPlayer;
use crate::game::dialogs::inventory::{InvClickState, InvItem};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::game::player_state::{Inventory, Loadout};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_animated_icon_button, spawn_container, spawn_icon_button, spawn_image,
    spawn_label, spawn_label_center, spawn_panel,
};

/// #2892 批B：面板精灵与 C# 原生尺寸（C# `FishingDialog.Index = 1340; Location = Center`）
pub const PANEL: (LibraryName, usize) = (LibraryName::Prguse, 1340);
pub const PANEL_SIZE: (f32, f32) = (200.0, 287.0);
/// C# `FishingStatusDialog`（`FishingDialog.cs:159-179`）：`Prguse[1341]` 244x128 @(390,300)、`Movable`
pub const STATUS_PANEL: (LibraryName, usize) = (LibraryName::Prguse, 1341);
pub const STATUS_SIZE: (f32, f32) = (244.0, 128.0);
pub const STATUS_X: f32 = 390.0;
pub const STATUS_Y: f32 = 300.0;

/// 钓鱼状态（`S.FishingUpdate` 写入）
/// #2892：字段对齐 C# `S.FishingUpdate`（`ObjectID + Fishing + ProgressPercent + ChancePercent + FoundFish`）
#[derive(Resource, Default)]
pub struct FishingState {
    /// C# `Fishing`：是否在钓鱼（状态窗显隐，`GameScene.cs:3056-3059`）
    pub fishing: bool,
    /// C# `ProgressPercent`（0..100；进度条 `Prguse[1349]` 裁绘宽度 = `2.16 * percent`）
    pub progress_percent: i32,
    /// C# `ChancePercent`（0..100；机会条 `Prguse[1342]` 同理 + `ChanceLabel` 文本）
    pub chance_percent: i32,
    /// C# `FoundFish`：是否咬钩（抛竿按钮 `Title[170..179]` 显隐，`PlayerObject.cs:2591-2597`）
    pub found_fish: bool,
    /// 自动钓鱼开关（C# `_autoCast`；点按钮本地切换 `AutoCastBox.Index`）
    pub autocast: bool,
    /// ESC 退出勾选（C# `bEscExit`：勾选后 ESC 才取消钓鱼，`GameScene.cs:698`）
    pub esc_exit: bool,
    pub message: String,
}

/// C# `FishingDialog.cs:353/367`：`width = (int)(2.16 * percent)`，钳制 `0..=216`
pub fn fishing_bar_width(percent: i32) -> f32 {
    ((2.16 * percent as f32) as i32).clamp(0, 216) as f32
}

/// 状态窗内控件的显隐角色（`fishing_ui_system` 的 `vis_q` 与测试共用同一套规则）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FishingVisRole {
    /// 主窗面板（跟 `DialogKind::Fishing`，C# `FishingDialog.Hide()` 手动窗）
    MainPanel,
    /// 状态窗面板（跟 `S.FishingUpdate.Fishing`，C# `GameScene.cs:3056-3059`）
    StatusPanel,
    /// 抛竿动画钮（C# `PlayerObject.cs:2591-2597`：咬钩才显示）
    CastButton,
    /// 抛竿禁用帧（C# `FishDisableButton` = `Title[149]`，在钓但未咬钩）
    CastDisabled,
    /// 自动钓鱼勾选图（C# `AutoCastBox.Index = _autoCast ? 1344 : 1343`，`:267`）
    AutocastBox,
    /// ESC 勾选图（C# `ESCTick.Visible = bEscExit`，`:294`）
    EscTick,
}

/// #2892：各控件显隐（逐条对应上面的 C# 出处）
pub fn fishing_visible(role: FishingVisRole, main_open: bool, st: &FishingState) -> bool {
    match role {
        FishingVisRole::MainPanel => main_open,
        FishingVisRole::StatusPanel => st.fishing,
        FishingVisRole::CastButton => st.fishing && st.found_fish,
        FishingVisRole::CastDisabled => st.fishing && !st.found_fish,
        FishingVisRole::AutocastBox => st.autocast,
        FishingVisRole::EscTick => st.esc_exit,
    }
}

#[derive(Component)]
pub struct FishingWidget;

#[derive(Component)]
pub struct FishingClose;

#[derive(Component)]
pub struct FishingCast;

#[derive(Component)]
pub struct FishingAutocast;

#[derive(Component)]
pub struct FishingLine(usize);

/// 主窗标题 `TitleLabel`（C# @(10,4) 180x20）
#[derive(Component)]
pub struct FishingTitle;
/// 状态窗根（C# `FishingStatusDialog`，与主窗分开）
#[derive(Component)]
pub struct FishingStatusRoot;
#[derive(Component)]
pub struct FishingStatusClose;
/// `ChanceLabel`（C# @(14,79)）
#[derive(Component)]
pub struct FishingChanceLabel;
/// 机会条精灵（C# `ChanceBar_BeforeDraw`：裁绘 `Prguse[1342]` 的 `(0,0,2.16*ChancePercent,12)` @(14,64)）
#[derive(Component)]
pub struct FishingChanceBar;
/// 进度条精灵（C# `ProgressBar_BeforeDraw`：裁绘 `Prguse[1349]` 的 `(0,0,2.16*ProgressPercent,8)` @(14,79)）
#[derive(Component)]
pub struct FishingProgressBar;
/// ESC 退出勾选框按钮（C# `ESCExitButton` = `Prguse[1346]` @(135,41)，三态同帧）
#[derive(Component)]
pub struct FishingEscButton;
/// 抛竿键禁用帧（C# `Title[149]`）
#[derive(Component)]
pub struct FishingCastDisabled;
/// 自动钓鱼勾选框（`Prguse[1343]/[1344]`）
#[derive(Component)]
pub struct FishingAutocastBox;
/// ESC 退出勾选框（`Prguse[1346]/[1347]`）
#[derive(Component)]
pub struct FishingEscTick;

/// 钓具槽（0=Hook 1=Float 2=Bait 3=Finder 4=Reel，C# FishingSlot）
#[derive(Component)]
struct FishingGearSlot(usize);

pub struct FishingPlugin;

impl Plugin for FishingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FishingState>();
        app.add_systems(
            Update,
            fishing_server_events.run_if(in_state(AppState::Game)),
        );
        app.add_systems(OnEnter(AppState::Game), spawn_fishing);
        app.add_systems(OnExit(AppState::Game), cleanup_fishing);
        app.add_systems(
            Update,
            (fishing_ui_system, fishing_gear_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_fishing(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

/// 钓具槽位置（C# FishingDialog Grid：Hook@(17,203) Float@(17,241) Bait@(57,241) Finder@(97,241) Reel@(137,241)，34x30）
const GEAR_POS: [(f32, f32); 5] = [
    (17.0, 203.0),
    (17.0, 241.0),
    (57.0, 241.0),
    (97.0, 241.0),
    (137.0, 241.0),
];

/// 找背包第一个空格（钓具卸下目标；无空格返回 None）
fn free_inventory_index(items: &[Option<InvItem>]) -> Option<usize> {
    items.iter().position(|s| s.is_none())
}

fn spawn_fishing(
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

    // 背景 Prguse[1340] 原生 200x287，C# Location = Center。
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1340) else {
        return;
    };
    let (px, py) = crate::game::dialogs::center_origin(200.0, 287.0);
    let panel = spawn_panel(&mut commands, bg, px, py, 200.0, 287.0, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::Fishing), FishingWidget));

    // ---- FishingStatusDialog（C# `FishingDialog.cs:159-320`）：`Prguse[1341]` 244x128
    //      @ ((1024-244)/2, 300) = (390,300)，`Movable = true`；与主窗**分开**（C# 两个独立窗）。
    if let Some(bg2) = load_lib_image(&mut libs, &mut images, STATUS_PANEL.0, STATUS_PANEL.1) {
        let status = spawn_panel(
            &mut commands,
            bg2,
            STATUS_X,
            STATUS_Y,
            STATUS_SIZE.0,
            STATUS_SIZE.1,
            31,
        );
        commands.entity(status).insert((
            // 独立 kind：C# `FishingDialog` 与 `FishingStatusDialog` 都 `Movable` 且各自拖动
            DialogRoot(DialogKind::FishingStatus),
            FishingWidget,
            FishingStatusRoot,
        ));
        commands.entity(status).with_children(|p| {
            // #2892：机会条 `Prguse[1342]` @(14,64)（C# `ChanceBar_BeforeDraw`，
            // `FishingDialog.cs:347-359`：裁绘 `(0,0,2.16*ChancePercent,12)`）
            if let Some(chance_bar) =
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1342)
            {
                spawn_image(p, chance_bar, 14.0, 64.0, 216.0, 12.0, 8).insert(FishingChanceBar);
            }
            // `ChanceLabel` @(14,62) 216x12 居中（C# `:190-197`，文本 `"{ChancePercent}%"`）
            spawn_label_center(
                p,
                &cjk,
                "0%",
                14.0 + 108.0,
                62.0,
                216.0,
                12.0,
                Color::WHITE,
                9,
            )
            .insert(FishingChanceLabel);
            // #2892：进度条 `Prguse[1349]` @(14,79)（C# `ProgressBar_BeforeDraw`，
            // `FishingDialog.cs:361-374`：裁绘 `(0,0,2.16*ProgressPercent,8)`）
            if let Some(progress_bar) =
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1349)
            {
                spawn_image(p, progress_bar, 14.0, 79.0, 216.0, 8.0, 8).insert(FishingProgressBar);
            }
            // ESC 退出：按钮 `ESCExitButton` `Prguse[1346]` @(135,41)（C# 三态同帧，`:281-295`）
            // + 选中图 `ESCTick` `Prguse[1347]`（默认隐藏）+ 文字 `ESCExit` `Title[45]` @(150,40)（`:307-314`）
            if let Some(btn) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1346) {
                spawn_icon_button(p, btn.clone(), btn.clone(), btn, 135.0, 41.0, 12.0, 12.0, 8)
                    .insert(FishingEscButton);
            }
            if let Some(on) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1347) {
                spawn_image(p, on, 135.0, 41.0, 16.0, 12.0, 9)
                    .insert((FishingEscTick, Visibility::Hidden));
            }
            if let Some((w, h)) = libs
                .0
                .get_image(LibraryName::Title, 45)
                .map(|i| (i.width as f32, i.height as f32))
            {
                if let Some(esc_text) =
                    load_lib_image(&mut libs, &mut images, LibraryName::Title, 45)
                {
                    spawn_image(p, esc_text, 150.0, 40.0, w, h, 8);
                }
            }
            // 关闭 `Prguse2[360/361/362]` @(216,4) 24x21
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
            ) {
                spawn_icon_button(p, n, h, pr, 216.0, 4.0, 24.0, 21.0, 10)
                    .insert(FishingStatusClose);
            }
            // 抛竿 `FishButton`：禁用帧 `Title[149]`；可抛时 10 帧动画 + 按下帧 `142` @(47,95)
            if let Some(disabled) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 149)
            {
                spawn_icon_button(
                    p,
                    disabled.clone(),
                    disabled.clone(),
                    disabled,
                    47.0,
                    95.0,
                    60.0,
                    25.0,
                    10,
                )
                .insert(FishingCastDisabled);
            }
            let frames: Vec<Handle<Image>> = (0..10usize)
                .filter_map(|i| load_lib_image(&mut libs, &mut images, LibraryName::Title, 170 + i))
                .collect();
            if frames.len() < 10 {
                tracing::warn!("抛竿按钮动画帧缺失：{}/10（Title[170..179]）", frames.len());
            }
            if !frames.is_empty() {
                let pressed = load_lib_image(&mut libs, &mut images, LibraryName::Title, 142);
                spawn_animated_icon_button(
                    p, frames, None, pressed, 47.0, 95.0, 60.0, 25.0, 10, 0.13, true,
                )
                .insert(FishingCast);
            }
            // 自动钓鱼开关 `Title[180/181/182]` @(110,95) 48x25
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 180),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 181),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 182),
            ) {
                spawn_icon_button(p, n, h, pr, 110.0, 95.0, 48.0, 25.0, 10).insert(FishingAutocast);
            }
            // 自动钓鱼勾选框 `Prguse[1343]`（未）/`[1344]`（勾）@(172,95) 28x25
            if let Some(off) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1343) {
                spawn_image(p, off, 172.0, 95.0, 28.0, 25.0, 8);
            }
            if let Some(on) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1344) {
                spawn_image(p, on, 172.0, 95.0, 28.0, 25.0, 9)
                    .insert((FishingAutocastBox, Visibility::Hidden));
            }
        });
    }

    commands.entity(panel).with_children(|p| {
        // 关闭 Prguse2[360/361/362]：旧 sprite 在 rel(220,3) 悬空面板外（200 宽），
        // 移到面板右上角 (176,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 176.0, 3.0, 20.0, 20.0, 10).insert(FishingClose);
        }
        // 钓具槽（C# FishingDialog Grid：Hook/Float/Bait/Finder/Reel，34x30）
        for (i, (rx, ry)) in GEAR_POS.iter().enumerate() {
            spawn_container(p, *rx, *ry, 34.0, 30.0, 9)
                .insert(BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)))
                .with_children(|c| {
                    spawn_label(c, &font, "—", 4.0, 8.0, 10.0, Color::WHITE, 10)
                        .insert(FishingGearSlot(i));
                });
        }
    });
}

/// 显隐 + 渲染 + 抛竿/自动钓鱼
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn fishing_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<FishingState>,
    net: Res<NetConnection>,
    close: Query<(Entity, &Interaction), With<FishingClose>>,
    status_close: Query<(Entity, &Interaction), With<FishingStatusClose>>,
    cast_btn: Query<(Entity, &Interaction), With<FishingCast>>,
    autocast_btn: Query<(Entity, &Interaction), With<FishingAutocast>>,
    esc_btn: Query<(Entity, &Interaction), With<FishingEscButton>>,
    // #2892：单个 `Visibility` 查询按标记分发（多个同写 `Visibility` 的查询需要两两 `Without`，
    // 每加一个按钮就要补一圈过滤器，容易漏成运行期 B0001）
    mut vis_q: Query<(
        &mut Visibility,
        Option<&FishingWidget>,
        Option<&FishingStatusRoot>,
        Option<&FishingCast>,
        Option<&FishingCastDisabled>,
        Option<&FishingAutocastBox>,
        Option<&FishingEscTick>,
    )>,
    // C# `ChanceLabel` @(14,62) 216x12
    mut chance: Query<&mut Text, With<FishingChanceLabel>>,
    // 两条 `BeforeDraw` 自绘的裁绘宽度（C# `FishingDialog.cs:347-374`）
    mut chance_bar: Query<
        (&mut Node, &mut ImageNode),
        (With<FishingChanceBar>, Without<FishingProgressBar>),
    >,
    mut progress_bar: Query<
        (&mut Node, &mut ImageNode),
        (With<FishingProgressBar>, Without<FishingChanceBar>),
    >,
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
    let open = mgr.is_open(DialogKind::Fishing);
    // #2892：C# `GameScene.cs:3056-3059` —— 状态窗由 `S.FishingUpdate.Fishing` 驱动（与主窗独立；
    // 主窗是手动窗，ESC/热键开关），不再「状态窗跟主窗」。
    crate::game::dialogs::sync_dialog_state(&mut mgr, DialogKind::FishingStatus, state.fishing);
    // 显隐分发（同一 `Visibility` 查询按标记取角色，规则走 `fishing_visible`）
    for (mut vis, widget, status_root, cast, disabled, auto_box, esc_tick) in &mut vis_q {
        let role = if widget.is_some() {
            FishingVisRole::MainPanel
        } else if status_root.is_some() {
            FishingVisRole::StatusPanel
        } else if cast.is_some() {
            FishingVisRole::CastButton
        } else if disabled.is_some() {
            FishingVisRole::CastDisabled
        } else if auto_box.is_some() {
            FishingVisRole::AutocastBox
        } else if esc_tick.is_some() {
            FishingVisRole::EscTick
        } else {
            continue;
        };
        let want = if fishing_visible(role, open, &state) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    if !open && !state.fishing {
        return;
    }
    // 主窗关闭键（C# `FishingDialog.CloseButton.Click → Hide()`）
    if open {
        for (e, inter) in &close {
            if edge(e, inter, &mut prev_inter) {
                mgr.close(DialogKind::Fishing);
            }
        }
    }
    // 状态窗关闭键 / ESC 勾选后 ESC 键：C# `Cancel()`（发 `C.FishingCast{CastOut=false}` 并关状态窗）
    for (e, inter) in &status_close {
        if edge(e, inter, &mut prev_inter) {
            cancel_fishing(&net, &mut mgr);
        }
    }
    // 机会百分比 `ChanceLabel`（C# `GameScene.cs:3054`：`string.Format("{0}%", ChancePercent)`）
    let want_text = format!("{}%", state.chance_percent.clamp(0, 100));
    for mut text in &mut chance {
        if text.0 != want_text {
            text.0 = want_text.clone();
        }
    }
    // 机会条 `Prguse[1342]` 裁绘宽度（C# `ChanceBar_BeforeDraw`，高 12）
    let chance_w = fishing_bar_width(state.chance_percent);
    for (mut node, mut image) in &mut chance_bar {
        let w = Val::Px(chance_w);
        if node.width != w {
            node.width = w;
        }
        image.rect = Some(Rect::new(0.0, 0.0, chance_w, 12.0));
    }
    // 进度条 `Prguse[1349]` 裁绘宽度（C# `ProgressBar_BeforeDraw`，高 8）
    let progress_w = fishing_bar_width(state.progress_percent);
    for (mut node, mut image) in &mut progress_bar {
        let w = Val::Px(progress_w);
        if node.width != w {
            node.width = w;
        }
        image.rect = Some(Rect::new(0.0, 0.0, progress_w, 8.0));
    }
    // 抛竿（C# `FishButton.Click → C.FishingCast { CastOut = false }`）
    for (e, inter) in &cast_btn {
        if edge(e, inter, &mut prev_inter) {
            net.send_packet(&crate::network::FishingCastWire { fishing_type: 0 });
            state.message = "已抛竿，等待鱼上钩…".to_string();
            tracing::info!("🎣 抛竿");
        }
    }
    // 自动钓鱼开关（C# `:260-271`：本地切 `_autoCast` 并发 `C.FishingChangeAutocast`）
    for (e, inter) in &autocast_btn {
        if edge(e, inter, &mut prev_inter) {
            state.autocast = !state.autocast;
            net.send_packet(&crate::network::FishingChangeAutocastWire {
                enabled: state.autocast,
            });
            state.message = format!("自动钓鱼: {}", if state.autocast { "开" } else { "关" });
            tracing::info!("🎣 自动钓鱼: {}", state.autocast);
        }
    }
    // ESC 退出勾选（C# `:291-295`：`bEscExit = !bEscExit`）
    for (e, inter) in &esc_btn {
        if edge(e, inter, &mut prev_inter) {
            state.esc_exit = !state.esc_exit;
            tracing::info!("🎣 ESC 退出: {}", state.esc_exit);
        }
    }
}

/// C# `FishingStatusDialog.Cancel()`（`FishingDialog.cs:376-382`）：
/// 发 `C.FishingCast { CastOut = false }` 并关状态窗（服务端随后回 `Fishing=false`）
pub fn cancel_fishing(net: &NetConnection, mgr: &mut DialogManager) {
    net.send_packet(&crate::network::FishingCastWire { fishing_type: 1 });
    mgr.close(DialogKind::FishingStatus);
}

/// #1313：钓具槽显示 + 点击穿戴/卸下（C# FishingDialog Grid + EquipSlotItem/RemoveSlotItem）
#[allow(clippy::too_many_arguments)]
fn fishing_gear_system(
    mgr: Res<DialogManager>,
    inv_q: Query<(&Inventory, &Loadout), With<LocalPlayer>>,
    mut inv_click: ResMut<InvClickState>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut slots: Query<(&mut Text, &FishingGearSlot)>,
) {
    let open = mgr.is_open(DialogKind::Fishing);
    let player = inv_q.single().ok();
    let rod = player
        .and_then(|(_, l)| l.slots.get(0))
        .and_then(|e| e.as_ref());
    for (mut text, slot) in &mut slots {
        let name = rod
            .and_then(|r| r.slots.get(slot.0))
            .and_then(|s| s.as_ref())
            .map(|g| g.name.clone())
            .unwrap_or_else(|| "—".to_string());
        if text.0 != name {
            text.0 = name;
        }
    }
    if !open || !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let items = player.map(|(inv, _)| inv.items.as_slice()).unwrap_or(&[]);
    for (slot, (rx, ry)) in GEAR_POS.iter().enumerate() {
        let x = 280.0 + rx;
        let y = 80.0 + ry;
        if cursor.x < x || cursor.x > x + 34.0 || cursor.y < y || cursor.y > y + 30.0 {
            continue;
        }
        let Some(rod) = rod else { return };
        // 已占用 → 卸下回背包（C# RemoveSlotItem Grid=Fishing）
        if let Some(gear) = rod.slots.get(slot).and_then(|s| s.as_ref()) {
            let Some(to) = free_inventory_index(items) else {
                tracing::warn!("🎣 背包已满，无法卸下钓具");
                return;
            };
            net.send_packet(&crate::network::RemoveSlotItemWire {
                grid: mir2_shared::enums::MirGridType::Fishing as u8,
                grid_to: mir2_shared::enums::MirGridType::Inventory as u8,
                unique_id: gear.unique_id,
                to: to as i32,
                from_unique_id: rod.unique_id,
            });
            tracing::info!("🎣 卸下钓具 {} -> 背包{}", gear.name, to);
            return;
        }
        // 空槽 + 背包已选中物品 → 穿戴（C# EquipSlotItem GridTo=Fishing）
        // #2631：选中态归 inventory 所有，经接口访问。严格对齐旧码：仅当物品确实存在
        // 才发送并清除选中；陈旧选中（物品已被移除）保留选中态，不用 take_selected。
        if let Some(sel) = inv_click.selected() {
            if let Some(item) = items.get(sel).and_then(|s| s.as_ref()) {
                net.send_packet(&mir2_shared::packets::client::misc::EquipSlotItem {
                    grid: mir2_shared::enums::MirGridType::Inventory,
                    unique_id: item.unique_id,
                    to_slot: slot as i32,
                    grid_to: mir2_shared::enums::MirGridType::Fishing,
                });
                tracing::info!("🎣 穿戴钓具 {} -> 槽{}", item.name, slot);
                inv_click.clear_selected();
            }
        }
        return;
    }
}

/// 消费服务端钓鱼事件（网络层只广播 ServerEvent；文案在此构造）
fn fishing_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut fishing: ResMut<FishingState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::FishingUpdate {
            fishing: is_fishing,
            progress_percent,
            chance_percent,
            found_fish,
            ..
        } = ev
        {
            fishing.fishing = *is_fishing;
            fishing.progress_percent = *progress_percent;
            fishing.chance_percent = *chance_percent;
            fishing.found_fish = *found_fish;
            fishing.message = if *found_fish {
                "上钩了！".to_string()
            } else if *is_fishing {
                format!("等待中…（进度 {progress_percent}%）")
            } else {
                "未钓鱼".to_string()
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2892：两条 `BeforeDraw` 条的裁绘宽度（C# `FishingDialog.cs:353/367`：
    /// `width = (int)(2.16 * percent)`，`< 0` 钳 0、`> 216` 钳 216）。
    ///
    /// 阳性对照：把 `fishing_bar_width` 改成 `216.0 * percent / 100.0`（浮点比例，非 C# 口径）
    /// → `fishing_bar_width(13)` 得 28.08 ≠ 28（截断），断言 FAILED。
    #[test]
    fn fishing_bar_width_matches_csharp() {
        assert_eq!(fishing_bar_width(0), 0.0);
        assert_eq!(fishing_bar_width(50), 108.0);
        assert_eq!(fishing_bar_width(100), 216.0);
        assert_eq!(fishing_bar_width(13), 28.0, "2.16*13 = 28.08 → 截断 28");
        assert_eq!(fishing_bar_width(-5), 0.0, "负值钳到 0");
        assert_eq!(fishing_bar_width(150), 216.0, "超过 216 钳到 216");
    }

    /// #2892：状态窗控件显隐（C# 出处见 `FishingVisRole` 注释）
    #[test]
    fn fishing_visibility_matches_csharp() {
        use FishingVisRole as R;
        let mut st = FishingState::default();
        // 未在钓：主窗跟手动开关、状态窗隐藏、两个抛竿图都不显示
        assert!(!fishing_visible(R::StatusPanel, false, &st));
        assert!(fishing_visible(R::MainPanel, true, &st));
        assert!(!fishing_visible(R::MainPanel, false, &st));
        assert!(!fishing_visible(R::CastButton, false, &st));
        assert!(!fishing_visible(R::CastDisabled, false, &st));
        // 在钓未咬钩：禁用帧；咬钩后换成动画钮
        st.fishing = true;
        assert!(fishing_visible(R::StatusPanel, false, &st));
        assert!(fishing_visible(R::CastDisabled, true, &st));
        assert!(!fishing_visible(R::CastButton, true, &st));
        st.found_fish = true;
        assert!(fishing_visible(R::CastButton, true, &st));
        assert!(!fishing_visible(R::CastDisabled, true, &st));
        // 勾选框
        assert!(!fishing_visible(R::AutocastBox, true, &st));
        assert!(!fishing_visible(R::EscTick, true, &st));
        st.autocast = true;
        st.esc_exit = true;
        assert!(fishing_visible(R::AutocastBox, true, &st));
        assert!(fishing_visible(R::EscTick, true, &st));
    }

    #[test]
    fn fishing_origin_is_csharp_center() {
        assert_eq!(
            crate::game::dialogs::center_origin(200.0, 287.0),
            (412.0, 240.0)
        );
    }

    #[test]
    fn free_inventory_index_finds_first_empty() {
        let items = vec![
            Some(InvItem::default()),
            None,
            Some(InvItem::default()),
            None,
        ];
        assert_eq!(free_inventory_index(&items), Some(1));
        let full = vec![Some(InvItem::default()); 3];
        assert_eq!(free_inventory_index(&full), None);
        let empty: Vec<Option<InvItem>> = vec![];
        assert_eq!(free_inventory_index(&empty), None);
    }
}
