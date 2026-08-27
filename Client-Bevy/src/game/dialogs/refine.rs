// ============================================================================
// 精炼对话框（M40）
// 参考：C# RefineDialog + ServerRust awakening.rs 精炼流程
// 网络（对齐 SharedRust / ServerRust gate wire）：
//   C: DepositRefineItem[from i32][to i32] / RetrieveRefineItem[from i32][to i32] / RefineCancel(空)
//      RefineItem[unique_id u64] / CheckRefine[unique_id u64]
// 结果通过系统聊天消息返回（精炼 60 秒 / 材料槽未实现 → 成功率 0）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{load_lib_image, spawn_icon_button, spawn_label, spawn_panel};

/// 精炼状态
#[derive(Resource, Default)]
pub struct RefineState {
    pub message: String,
    /// 最近存入的精炼物品 uid
    pub deposited_uid: Option<u64>,
    /// 最近存入的物品 index（精炼用）
    pub deposited_index: Option<i32>,
}

#[derive(Component)]
pub struct RefineWidget;

#[derive(Component)]
pub struct RefineClose;

#[derive(Component)]
pub struct RefineDeposit;

#[derive(Component)]
pub struct RefineRetrieve;

#[derive(Component)]
pub struct RefineStart;

#[derive(Component)]
pub struct RefineCheck;

#[derive(Component)]
pub struct RefineCancel;

#[derive(Component)]
pub struct RefineLine(usize);

pub struct RefinePlugin;

impl Plugin for RefinePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RefineState>();
        app.add_systems(OnEnter(AppState::Game), spawn_refine);
        app.add_systems(OnExit(AppState::Game), cleanup_refine);
        app.add_systems(
            Update,
            refine_ui_system.run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_refine(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_refine(
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

    // 面板 Prguse[170] @ (280,80)。加宽到 320x207：3 列按钮 + 关闭按钮都在面板内
    // （旧 sprite 布局"开始精炼"按钮右缘超出 244 面板宽，悬空面板外）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 170) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, 280.0, 80.0, 320.0, 207.0, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::Refine), RefineWidget));

    commands.entity(panel).with_children(|p| {
        // 关闭 Prguse2[360/361/362] @(300,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 300.0, 3.0, 20.0, 20.0, 10).insert(RefineClose);
        }
        // 状态行 4 @(18,40+22i)
        for i in 0..4usize {
            spawn_label(p, &cjk, "", 18.0, 40.0 + i as f32 * 22.0, 12.0, Color::WHITE, 9)
                .insert(RefineLine(i));
        }
        // 按钮：存入/取回/开始精炼 @(20/110/200,135)；查看/取消 @(20/110,170)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
        ) {
            spawn_icon_button(p, n.clone(), h.clone(), pr.clone(), 20.0, 135.0, 76.0, 25.0, 10)
                .insert(RefineDeposit);
            spawn_icon_button(p, n.clone(), h.clone(), pr.clone(), 200.0, 135.0, 76.0, 25.0, 10)
                .insert(RefineStart);
            spawn_icon_button(p, n, h, pr, 110.0, 170.0, 76.0, 25.0, 10).insert(RefineCancel);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 210),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 211),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 212),
        ) {
            spawn_icon_button(p, n.clone(), h.clone(), pr.clone(), 110.0, 135.0, 76.0, 25.0, 10)
                .insert(RefineRetrieve);
            spawn_icon_button(p, n, h, pr, 20.0, 170.0, 76.0, 25.0, 10).insert(RefineCheck);
        }
    });
}

/// 显隐 + 渲染 + 按钮
#[allow(clippy::too_many_arguments)]
fn refine_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<RefineState>,
    net: Res<NetConnection>,
    inv_q: Query<&crate::game::player_state::Inventory, With<crate::actor::LocalPlayer>>,
    inv_click: Res<crate::game::dialogs::inventory::InvClickState>,
    close: Query<(Entity, &Interaction), With<RefineClose>>,
    deposit_btn: Query<(Entity, &Interaction), With<RefineDeposit>>,
    retrieve_btn: Query<(Entity, &Interaction), With<RefineRetrieve>>,
    start_btn: Query<(Entity, &Interaction), With<RefineStart>>,
    check_btn: Query<(Entity, &Interaction), With<RefineCheck>>,
    cancel_btn: Query<(Entity, &Interaction), With<RefineCancel>>,
    mut widgets: Query<&mut Visibility, With<RefineWidget>>,
    mut lines: Query<(&mut Text, &RefineLine)>,
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
    let open = mgr.is_open(DialogKind::Refine);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::Refine);
        }
    }
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            0 => "精炼（NPC 铁匠）".to_string(),
            1 => format!(
                "存入物品: {}",
                state
                    .deposited_uid
                    .map(|u| format!("uid={}", u))
                    .unwrap_or_else(|| "无".to_string())
            ),
            2 => state.message.clone(),
            3 => "流程：存入 → 开始精炼（60秒）→ 查看 → 取回".to_string(),
            _ => String::new(),
        };
    }
    // 存入：选中背包物品 → DepositRefineItem[uid]
    let items = inv_q.single().map(|inv| inv.items.as_slice()).unwrap_or(&[]);
    for (e, inter) in &deposit_btn {
        if edge(e, inter, &mut prev_inter) {
            let idx = inv_click
                .selected
                .filter(|i| items.get(*i).and_then(|s| s.as_ref()).is_some())
                .or_else(|| items.iter().position(|s| s.is_some()));
            if let Some(i) = idx {
                if let Some(item) = items.get(i).and_then(|s| s.as_ref()) {
                    net.send_packet(&crate::network::RefineDepositWire {
                        from: i as i32,
                        to: 0,
                    });
                    state.deposited_uid = Some(item.unique_id);
                    state.deposited_index = Some(item.item_index);
                    state.message = format!("存入精炼 #{}", item.item_index);
                    tracing::info!("🔨 存入精炼物品 uid={} #{}", item.unique_id, item.item_index);
                }
            } else {
                state.message = "背包没有可存入的物品".to_string();
            }
        }
    }
    // 取回
    for (e, inter) in &retrieve_btn {
        if edge(e, inter, &mut prev_inter) {
            if let Some(uid) = state.deposited_uid {
                // C# RetrieveRefineItem：[from 精炼栏格][to 背包格]；Rust 单槽 from=0，to 需为空格
                match items.iter().position(|s| s.is_none()) {
                    Some(grid) => {
                        net.send_packet(&crate::network::RefineRetrieveWire {
                            from: 0,
                            to: grid as i32,
                        });
                        state.deposited_uid = None;
                        state.message = "已请求取回".to_string();
                        tracing::info!("🔨 取回精炼物品 uid={} 到背包格 {}", uid, grid);
                    }
                    None => state.message = "背包已满，无法取回".to_string(),
                }
            }
        }
    }
    // 开始精炼
    for (e, inter) in &start_btn {
        if edge(e, inter, &mut prev_inter) {
            if let Some(uid) = state.deposited_uid {
                net.send_packet(&crate::network::RefineItemWire { unique_id: uid });
                state.message = "精炼已开始（60 秒）".to_string();
                tracing::info!("🔨 开始精炼 uid={}", uid);
            } else {
                state.message = "请先存入物品".to_string();
            }
        }
    }
    // 查看
    for (e, inter) in &check_btn {
        if edge(e, inter, &mut prev_inter) {
            if let Some(uid) = state.deposited_uid {
                net.send_packet(&crate::network::RefineCheckWire { unique_id: uid });
                state.message = "已请求查看精炼状态".to_string();
                tracing::info!("🔨 查看精炼 uid={}", uid);
            }
        }
    }
    // 取消
    for (e, inter) in &cancel_btn {
        if edge(e, inter, &mut prev_inter) {
            net.send_packet(&mir2_shared::packets::client::refine::RefineCancel {});
            state.message = "已请求取消精炼".to_string();
            tracing::info!("🔨 取消精炼");
        }
    }
}
