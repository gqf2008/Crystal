// ============================================================================
// NPC 出售/修理面板（M20）
// 布局参考：C# NPCDialogs.cs NPCDropDialog
//   - 背景 Prguse[392]，位置 (264,224)
//   - 确认按钮 Title[290-292] (114,62)；物品格 (38,72)
//   - 交互（原版 C# MirItemCell 拖放语义）：
//       点背包物品选中（SelectedCell）→ 点面板拖放区放入 TargetItem
//       → 点确认：Sell 卖整叠（C.SellItem{uid, count=整叠数量}）/ Repair 修理（C.RepairItem{uid}）
// 网络：NPCGoods(panel_type=Sell/Repair) → 打开面板（C# GameScene.NPCSell/NPCRepair）
// ============================================================================

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::actor::LocalPlayer;
use crate::game::dialogs::inventory::{InvClickState, InvItem};
use crate::game::dialogs::refine::RefineWeaponRequest;
use crate::game::dialogs::{DialogKind, DialogRoot};
use crate::game::player_state::Inventory;
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_image, spawn_label, spawn_panel,
};
use mir2_shared::enums::PanelType;

/// 出售/修理面板状态
#[derive(Resource, Default)]
pub struct SellPanelState {
    pub visible: bool,
    /// 当前模式（Sell / Repair / SpecialRepair）
    pub mode: Option<PanelType>,
    /// 面板中的目标物品（原版 C# NPCDropDialog.TargetItem）
    pub target: Option<InvItem>,
    /// C# `NPCDropDialog.Hold`（按住/自动确认开关）：把物品放进面板后**立即确认**
    /// （`NPCDialogs.cs:1734` `if (Hold) Confirm();`）
    pub hold: bool,
    /// 本帧是否要按 `hold` 语义自动确认（放进物品那一帧置位，确认逻辑复用同一段）
    pub auto_confirm: bool,
}

const DIALOG_X: f32 = 264.0;
const DIALOG_Y: f32 = 224.0;

/// 面板精灵（**不是** C# 构造期写的 `Prguse[392]`）：
/// C# `NPCDropDialog` 构造时 `Index = 392; Location = (264,224)`，但 `BeforeDraw`
/// （`NPCDialogs.cs:1743-1745`）会改写为 `Index = 351; Library = Prguse2;
/// Location = new Point(264, GameScene.Scene.NPCDialog.Size.Height)` —— 实际画的是
/// **`Prguse2[351]`（实测 176x147）**；`Prguse[392]` 在这套数据里是**空帧（0x0）**。
/// NPC 窗底图 `Prguse[995]` 实测 440x224 ⇒ `NPCDialog.Size.Height = 224`，与本端 (264,224) 一致。
pub const PANEL: (LibraryName, usize) = (LibraryName::Prguse2, 351);
pub const PANEL_SIZE: (f32, f32) = (176.0, 147.0);
/// C# `HoldButton` `Title[293/294/295]` @(114,36)（无显式 `Size` → 图头 48x25）
pub const HOLD_BTN_POS: (f32, f32) = (114.0, 36.0);
pub const HOLD_FRAMES: (usize, usize, usize) = (293, 294, 295);
/// C# `ConfirmButton` `Title[290/291/292]` @(114,62)（图头 48x25）
pub const CONFIRM_BTN_POS: (f32, f32) = (114.0, 62.0);
pub const CONFIRM_FRAMES: (usize, usize, usize) = (290, 291, 292);

/// C# `HoldButton.Visible`：`BeforeDraw` 先置 true，再按 `PanelType` 关闭
/// （`NPCDialogs.cs:1741/1772/1785/1789/1793/1799` —— 分解/降级/重置/精炼/查看精炼不显示）
pub fn hold_button_visible(mode: Option<PanelType>) -> bool {
    !matches!(
        mode,
        Some(PanelType::Disassemble)
            | Some(PanelType::Downgrade)
            | Some(PanelType::Reset)
            | Some(PanelType::Refine)
            | Some(PanelType::CheckRefine)
    )
}

#[derive(Component)]
pub struct SellPanelWidget;

#[derive(Component)]
pub struct SellPanelConfirm;

/// C# `HoldButton`（按住/自动确认开关）+ 它在 `AfterDraw` 里叠画的高亮帧
#[derive(Component)]
pub struct SellPanelHold;
#[derive(Component)]
pub struct SellPanelHoldOn;

/// 拖放区（原版 C# ItemCell / NPCDropPanel_Click 的 (20,55,75,75) 区域）
#[derive(Component)]
pub struct SellPanelDrop;

/// 面板提示文案（C# `NPCDropDialog` 各 PanelType 的 `text`，NPCDialogs.cs:1760-1805）
pub fn sell_panel_prompt(mode: Option<PanelType>) -> &'static str {
    match mode {
        Some(PanelType::Repair) | Some(PanelType::SpecialRepair) => "放入物品后点确认修理",
        Some(PanelType::Refine) => "放入武器后点确认精炼",
        Some(PanelType::CheckRefine) => "放入物品后点确认查看精炼",
        _ => "放入物品后点确认出售",
    }
}

/// 目标物品图标（拖放区子实体）
#[derive(Component)]
pub struct SellPanelIcon;

/// 提示文本（InfoLabel）
#[derive(Component)]
pub struct SellPanelInfo;

pub struct SellPanelPlugin;

impl Plugin for SellPanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SellPanelState>();
        app.add_systems(
            Update,
            sell_panel_server_events.run_if(in_state(AppState::Game)),
        );
        app.add_systems(OnEnter(AppState::Game), spawn_sell_panel);
        app.add_systems(OnExit(AppState::Game), cleanup_sell_panel);
        app.add_systems(
            Update,
            (sell_panel_ui_system, sell_panel_action_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_sell_panel(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_sell_panel(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
    mut cjk_font: ResMut<UiCjkFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        crate::ui::sprite_ui::ensure_ui_font(&mut fonts, &mut ui_font);
    }
    let font = ui_font.0.clone();
    let cjk = shared_cjk_font(&mut fonts, &mut cjk_font);

    // 背景 Prguse2[351] 176x147（见 `PANEL` 注释：C# 构造期写 392，实际画 351）
    let Some(bg) = load_lib_image(&mut libs, &mut images, PANEL.0, PANEL.1) else {
        return;
    };
    let panel = spawn_panel(
        &mut commands,
        bg,
        DIALOG_X,
        DIALOG_Y,
        PANEL_SIZE.0,
        PANEL_SIZE.1,
        30,
    );
    commands
        .entity(panel)
        .insert((SellPanelWidget, DialogRoot(DialogKind::Npc)));

    commands.entity(panel).with_children(|p| {
        // 确认按钮 Title[290/291/292]（C# ConfirmButton (114,62)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, CONFIRM_FRAMES.0),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, CONFIRM_FRAMES.1),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, CONFIRM_FRAMES.2),
        ) {
            spawn_icon_button(
                p,
                n,
                h,
                pr,
                CONFIRM_BTN_POS.0,
                CONFIRM_BTN_POS.1,
                48.0,
                25.0,
                10,
            )
            .insert(SellPanelConfirm);
        }
        // 按住/自动确认（C# HoldButton Title[293/294/295] @(114,36)；开启时 `AfterDraw` 叠画 295）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, HOLD_FRAMES.0),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, HOLD_FRAMES.1),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, HOLD_FRAMES.2),
        ) {
            spawn_icon_button(
                p,
                n,
                h,
                pr,
                HOLD_BTN_POS.0,
                HOLD_BTN_POS.1,
                48.0,
                25.0,
                10,
            )
            .insert(SellPanelHold);
            let on = load_lib_image(&mut libs, &mut images, LibraryName::Title, HOLD_FRAMES.2);
            if let Some(on) = on {
                spawn_image(p, on, HOLD_BTN_POS.0, HOLD_BTN_POS.1, 48.0, 25.0, 11)
                    .insert((SellPanelHoldOn, Visibility::Hidden));
            }
        }
        // 提示文本（C# InfoLabel (30,10)）——中文走 CJK 主字体（拉丁字体出豆腐块）
        spawn_label(
            p,
            &cjk,
            "把物品放入面板后点确认",
            30.0,
            10.0,
            12.0,
            Color::WHITE,
            9,
        )
        .insert(SellPanelInfo);
        // 拖放区（C# ItemCell (38,72) 区域 (20,55,75,75)）+ 目标图标
        spawn_container(p, 20.0, 55.0, 75.0, 75.0, 9)
            .insert((
                SellPanelDrop,
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
            ))
            .with_children(|c| {
                let white = images.add(crate::map_renderer::make_image(
                    vec![255, 255, 255, 255],
                    1,
                    1,
                ));
                spawn_image(c, white, 3.0, 3.0, 68.0, 68.0, 10).insert(SellPanelIcon);
            });
    });
}

/// 显示/隐藏 + 提示文本 + 目标物品图标
fn sell_panel_ui_system(
    state: Res<SellPanelState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    // 三组 `Visibility` 查询必须**可证不相交**（B0001）：互相补 `Without`，同 `LESSON_..._B0001` 口径
    mut widgets: Query<
        &mut Visibility,
        (
            With<SellPanelWidget>,
            Without<SellPanelIcon>,
            Without<SellPanelHold>,
            Without<SellPanelHoldOn>,
        ),
    >,
    mut icons: Query<
        (&mut ImageNode, &mut Visibility),
        (
            With<SellPanelIcon>,
            Without<SellPanelWidget>,
            Without<SellPanelHold>,
            Without<SellPanelHoldOn>,
        ),
    >,
    mut info_texts: Query<(&mut Text, &SellPanelInfo)>,
    // #3265：Hold 开关本体 + 「按住」高亮帧（C# `HoldButton.Visible` / `AfterDraw` 叠画 295）
    mut hold_btns: Query<
        &mut Visibility,
        (
            With<SellPanelHold>,
            Without<SellPanelHoldOn>,
            Without<SellPanelWidget>,
            Without<SellPanelIcon>,
        ),
    >,
    mut hold_on: Query<
        &mut Visibility,
        (
            With<SellPanelHoldOn>,
            Without<SellPanelHold>,
            Without<SellPanelWidget>,
            Without<SellPanelIcon>,
        ),
    >,
) {
    for mut vis in &mut widgets {
        *vis = if state.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (mut text, _) in &mut info_texts {
        let new = sell_panel_prompt(state.mode).to_string();
        if text.0 != new {
            text.0 = new;
        }
    }
    // Hold 开关：按面板类型决定是否显示（C# `HoldButton.Visible`），开启时叠画高亮帧 295
    let hold_visible = state.visible && hold_button_visible(state.mode);
    for mut vis in &mut hold_btns {
        let want = if hold_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    for mut vis in &mut hold_on {
        let want = if hold_visible && state.hold {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }

    // 目标物品图标
    for (mut node, mut vis) in &mut icons {
        match state.target.as_ref() {
            Some(item) => {
                let handle = load_lib_image(
                    &mut libs,
                    &mut images,
                    LibraryName::Items,
                    item.image as usize,
                );
                match handle {
                    Some(h) if node.image != h => node.image = h,
                    None => *vis = Visibility::Hidden,
                    _ => {}
                }
                if node.image.is_strong() {
                    *vis = Visibility::Visible;
                }
            }
            None => *vis = Visibility::Hidden,
        }
    }
}

/// 交互：点拖放区放入选中物品；点确认出售/修理
#[allow(clippy::too_many_arguments)]
fn sell_panel_action_system(
    mut state: ResMut<SellPanelState>,
    mut inv_click: ResMut<InvClickState>,
    inv_q: Query<&Inventory, With<LocalPlayer>>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    // 面板原点（拖后跟随；挂在 Npc kind 组随 NPC 对话框联合拖动/置顶）
    panel_origin: Query<&Node, With<SellPanelWidget>>,
    confirm_btns: Query<(Entity, &Interaction), With<SellPanelConfirm>>,
    hold_btns: Query<(Entity, &Interaction), With<SellPanelHold>>,
    mut weapon_req: MessageWriter<RefineWeaponRequest>,
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
    if !state.visible {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    // 点拖放区（C# NPCDropPanel_Click 区域 (20,55,75,75) 相对面板）：放入选中物品
    // （面板原点动态取——面板挂 Npc kind 可被联合拖动，固定 DIALOG_X/Y 会成死区）
    if mouse.just_pressed(MouseButton::Left) {
        let (ox, oy) = panel_origin
            .single()
            .map(|n| crate::ui::theme::node_origin(n, (DIALOG_X, DIALOG_Y)))
            .unwrap_or((DIALOG_X, DIALOG_Y));
        let dx = ox + 20.0;
        let dy = oy + 55.0;
        if cursor.x >= dx && cursor.x <= dx + 75.0 && cursor.y >= dy && cursor.y <= dy + 75.0 {
            // #2631：选中态归 inventory 所有，经接口访问。严格对齐旧码：仅当物品确实存在
            // 才放入并清除选中；陈旧选中（物品已被移除）保留选中态，不用 take_selected。
            if let Some(sel) = inv_click.selected() {
                if let Some(item) = inv_q
                    .single()
                    .ok()
                    .and_then(|inv| inv.items.get(sel).and_then(|s| s.as_ref()))
                {
                    state.target = Some(item.clone());
                    // C# `ItemCell_Click` 末尾（`NPCDialogs.cs:1734`）：`if (Hold) Confirm();`
                    if state.hold {
                        state.auto_confirm = true;
                    }
                    tracing::info!(
                        "🎯 放入面板: {} (uid={}) x{}",
                        item.name,
                        item.unique_id,
                        item.count
                    );
                    inv_click.clear_selected();
                }
            }
        }
    }

    // 点背包物品时若面板已打开且无选中 → 交给背包系统选中（原版 C# SelectedCell）
    // （这里只负责面板拖放区与确认）

    // 按住/自动确认开关（C# `HoldButton.Click += (o,e) => Hold = !Hold;`，`NPCDialogs.cs:1465`）
    for (e, inter) in &hold_btns {
        if edge(e, inter, &mut prev_inter) && hold_button_visible(state.mode) {
            state.hold = !state.hold;
            tracing::info!("🔁 出售面板 Hold={}", state.hold);
        }
    }
    // 确认：按钮按下 **或** `hold` 语义下的自动确认（C# `if (Hold) Confirm();`）
    let mut pressed = false;
    for (e, inter) in &confirm_btns {
        if edge(e, inter, &mut prev_inter) {
            pressed = true;
        }
    }
    if pressed || state.auto_confirm {
        state.auto_confirm = false;
        if let Some(item) = state.target.take() {
            match state.mode {
            Some(PanelType::Sell) => {
                // 原版 C# Confirm：C.SellItem{UniqueID, Count=TargetItem.Count}（卖整叠）
                net.send_packet(&mir2_shared::packets::client::npc::SellItem {
                    unique_id: item.unique_id,
                    count: item.count.max(1),
                });
                tracing::info!(
                    "💰 面板出售 {} (uid={}) x{}",
                    item.name,
                    item.unique_id,
                    item.count
                );
            }
            Some(PanelType::Repair) | Some(PanelType::SpecialRepair) => {
                net.send_packet(&mir2_shared::packets::client::npc::RepairItem {
                    unique_id: item.unique_id,
                });
                tracing::info!("🔧 面板修理 {} (uid={})", item.name, item.unique_id);
            }
            // C# Confirm（NPCDialogs.cs:1606）：`C.RefineItem{UniqueID}`；
            // Rust 服务端语义为「先存入武器(to=0)再按 uid 发起」，交给 refine 模块两步执行
            Some(PanelType::Refine) => {
                let slot = inv_q.single().ok().and_then(|inv| {
                    inv.items
                        .iter()
                        .position(|s| s.as_ref().is_some_and(|it| it.unique_id == item.unique_id))
                });
                match slot {
                    Some(inv_slot) => {
                        weapon_req.write(RefineWeaponRequest {
                            unique_id: item.unique_id,
                            inv_slot,
                        });
                        tracing::info!("🔨 面板精炼 {} (uid={})", item.name, item.unique_id);
                    }
                    None => tracing::warn!("🔨 精炼目标已不在背包 uid={}", item.unique_id),
                }
            }
            // C# CheckRefine：`C.CheckRefine{UniqueID}`（NPCDialogs.cs:1624）
            Some(PanelType::CheckRefine) => {
                net.send_packet(&crate::network::RefineCheckWire {
                    unique_id: item.unique_id,
                });
                tracing::info!("🔨 面板查看精炼 {} (uid={})", item.name, item.unique_id);
            }
                _ => {}
            }
        }
    }
}

/// 消费服务端出售/修理面板事件（网络层只广播 ServerEvent）
fn sell_panel_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut sell_panel: ResMut<SellPanelState>,
    mut mgr: ResMut<crate::game::dialogs::DialogManager>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::NpcSellPanel { panel_type } = ev {
            sell_panel.mode = Some(*panel_type);
            sell_panel.target = None;
            sell_panel.auto_confirm = false;
            sell_panel.visible = true;
            // C# NPCDropDialog.Show() 同时打开背包
            if !mgr.is_open(crate::game::dialogs::DialogKind::Inventory) {
                mgr.open.push(crate::game::dialogs::DialogKind::Inventory);
            }
        }
        // #2720：C# `GameScene.NPCRefine`（NPCDialogs.cs:1791 `RefineDialog.Show()`）
        if let ServerEvent::NpcRefinePanel { refining, .. } = ev {
            if *refining {
                // C#：精炼进行中 → 收起投放窗与材料窗（GameScene.cs:4291-4295）
                sell_panel.visible = false;
                sell_panel.target = None;
                mgr.close(crate::game::dialogs::DialogKind::Refine);
            } else {
                sell_panel.mode = Some(PanelType::Refine);
                sell_panel.target = None;
                sell_panel.visible = true;
                if !mgr.is_open(crate::game::dialogs::DialogKind::Inventory) {
                    mgr.open.push(crate::game::dialogs::DialogKind::Inventory);
                }
                mgr.open(crate::game::dialogs::DialogKind::Refine);
            }
        }
        // #2720：C# `GameScene.NPCCheckRefine`（只开投放窗）
        if let ServerEvent::NpcCheckRefinePanel = ev {
            sell_panel.mode = Some(PanelType::CheckRefine);
            sell_panel.target = None;
            sell_panel.visible = true;
            if !mgr.is_open(crate::game::dialogs::DialogKind::Inventory) {
                mgr.open.push(crate::game::dialogs::DialogKind::Inventory);
            }
        }
        // #2720：C# `GameScene.NPCCollectRefine`（NPCDialog.Hide）→ 全部收起
        if let ServerEvent::NpcCollectRefine = ev {
            sell_panel.visible = false;
            sell_panel.target = None;
            mgr.close(crate::game::dialogs::DialogKind::Refine);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2720：投放窗提示随 PanelType 切换（C# `NPCDropDialog` 各分支 text）
    #[test]
    fn refine_panel_prompt_matches_csharp() {
        assert_eq!(
            sell_panel_prompt(Some(PanelType::Refine)),
            "放入武器后点确认精炼"
        );
        assert_eq!(
            sell_panel_prompt(Some(PanelType::CheckRefine)),
            "放入物品后点确认查看精炼"
        );
        assert_eq!(
            sell_panel_prompt(Some(PanelType::Repair)),
            "放入物品后点确认修理"
        );
        assert_eq!(
            sell_panel_prompt(Some(PanelType::Sell)),
            "放入物品后点确认出售"
        );
        assert_eq!(sell_panel_prompt(None), "放入物品后点确认出售");
    }

    /// #3265 门禁：面板精灵与 Hold 开关必须按 C# `NPCDropDialog` 的**实际绘制值**（`BeforeDraw`）对齐。
    /// 阳性对照：把 `PANEL` 改回 `Prguse[392]`（构造期那个**空帧 0x0**）⇒ 本测试 FAILED。
    #[test]
    fn panel_skin_and_hold_button_match_csharp_beforedraw() {
        assert_eq!(
            PANEL,
            (LibraryName::Prguse2, 351),
            "C# BeforeDraw 改写为 Index=351/Library=Prguse2（NPCDialogs.cs:1743-1745）；Prguse[392] 是空帧"
        );
        assert_eq!(PANEL_SIZE, (176.0, 147.0), "Prguse2[351] 图头实测 176x147");
        assert_eq!(HOLD_BTN_POS, (114.0, 36.0), "C# HoldButton @(114,36)");
        assert_eq!(HOLD_FRAMES, (293, 294, 295), "C# HoldButton 三帧");
        assert_eq!(CONFIRM_BTN_POS, (114.0, 62.0), "C# ConfirmButton @(114,62)");
        assert_eq!(CONFIRM_FRAMES, (290, 291, 292), "C# ConfirmButton 三帧");
        // C# `HoldButton.Visible`：分解/降级/重置/精炼/查看精炼不显示
        assert!(hold_button_visible(Some(PanelType::Sell)));
        assert!(hold_button_visible(Some(PanelType::Repair)));
        assert!(hold_button_visible(Some(PanelType::SpecialRepair)));
        assert!(!hold_button_visible(Some(PanelType::Refine)));
        assert!(!hold_button_visible(Some(PanelType::CheckRefine)));
        assert!(!hold_button_visible(Some(PanelType::Disassemble)));
        assert!(!hold_button_visible(Some(PanelType::Downgrade)));
        assert!(!hold_button_visible(Some(PanelType::Reset)));
    }
}
