// ============================================================================
// 精炼对话框（M40）
// 参考：C# RefineDialog + ServerRust awakening.rs 精炼流程
// 网络（ServerRust gate 实际 wire，与 SharedRust 客户端结构不一致，手动构造）：
//   C: DepositRefineItem[uid u64] / RetrieveRefineItem[uid u64] / RefineCancel(空)
//      RefineItem[item_id u32][materials u32] / CheckRefine[uid u64]
// 结果通过系统聊天消息返回（精炼 60 秒 / 80% 成功率）
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
            (refine_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
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
            DialogRoot(DialogKind::Refine),
            RefineWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 300.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            RefineClose,
            DialogRoot(DialogKind::Refine),
            RefineWidget,
        ));
    }
    // 状态行
    for i in 0..4usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            298.0, 120.0 + i as f32 * 22.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            RefineLine(i),
            DialogRoot(DialogKind::Refine),
            RefineWidget,
        ));
    }
    // 按钮：存入 / 取回 / 开始精炼 / 查看 / 取消
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        300.0, 215.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((RefineDeposit, DialogRoot(DialogKind::Refine), RefineWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        390.0, 215.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((RefineRetrieve, DialogRoot(DialogKind::Refine), RefineWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        480.0, 215.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((RefineStart, DialogRoot(DialogKind::Refine), RefineWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        300.0, 250.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((RefineCheck, DialogRoot(DialogKind::Refine), RefineWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        390.0, 250.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((RefineCancel, DialogRoot(DialogKind::Refine), RefineWidget));
    }
}

/// 显隐 + 渲染 + 按钮
#[allow(clippy::too_many_arguments)]
fn refine_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<RefineState>,
    net: Res<NetConnection>,
    hud: Res<crate::game::hud::HudState>,
    inv_click: Res<crate::game::dialogs::inventory::InvClickState>,
    close: Query<&UiButton, With<RefineClose>>,
    deposit_btn: Query<&UiButton, With<RefineDeposit>>,
    retrieve_btn: Query<&UiButton, With<RefineRetrieve>>,
    start_btn: Query<&UiButton, With<RefineStart>>,
    check_btn: Query<&UiButton, With<RefineCheck>>,
    cancel_btn: Query<&UiButton, With<RefineCancel>>,
    mut widgets: Query<&mut Visibility, With<RefineWidget>>,
    mut lines: Query<(&mut Text2d, &RefineLine)>,
) {
    let open = mgr.is_open(DialogKind::Refine);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
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
    for btn in &deposit_btn {
        if btn.clicked {
            let idx = inv_click
                .selected
                .filter(|i| hud.inventory.items.get(*i).and_then(|s| s.as_ref()).is_some())
                .or_else(|| hud.inventory.items.iter().position(|s| s.is_some()));
            if let Some(i) = idx {
                if let Some(item) = hud.inventory.items.get(i).and_then(|s| s.as_ref()) {
                    net.send_packet(&crate::network::RefineDepositWire {
                        unique_id: item.unique_id,
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
    for btn in &retrieve_btn {
        if btn.clicked {
            if let Some(uid) = state.deposited_uid {
                net.send_packet(&crate::network::RefineRetrieveWire { unique_id: uid });
                state.deposited_uid = None;
                state.message = "已请求取回".to_string();
                tracing::info!("🔨 取回精炼物品 uid={}", uid);
            }
        }
    }
    // 开始精炼
    for btn in &start_btn {
        if btn.clicked {
            let item_id = state.deposited_index.unwrap_or(0) as u32;
            if item_id > 0 {
                net.send_packet(&crate::network::RefineItemWire {
                    item_id,
                    materials: 1,
                });
                state.message = "精炼已开始（60 秒）".to_string();
                tracing::info!("🔨 开始精炼 #{}", item_id);
            } else {
                state.message = "请先存入物品".to_string();
            }
        }
    }
    // 查看
    for btn in &check_btn {
        if btn.clicked {
            if let Some(uid) = state.deposited_uid {
                net.send_packet(&crate::network::RefineCheckWire { unique_id: uid });
                state.message = "已请求查看精炼状态".to_string();
                tracing::info!("🔨 查看精炼 uid={}", uid);
            }
        }
    }
    // 取消
    for btn in &cancel_btn {
        if btn.clicked {
            net.send_packet(&mir2_shared::packets::client::refine::RefineCancel {});
            state.message = "已请求取消精炼".to_string();
            tracing::info!("🔨 取消精炼");
        }
    }
}
