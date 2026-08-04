// ============================================================================
// NPC 觉醒对话框（M54）
// 参考：C# NPCAwakeDialog（Client/MirScenes/Dialogs/NPCDialogs.cs）
//   - 面板 Title[710]（360x420）位于 (0,0)
//   - 升级按钮 Title[712/713/714] (115,391)；关闭 Prguse2[360/361/362] (284,4)
//   - 主物品格 (202,91)、材料标签 (67,317)/(192,317)、结果 (112,354)
//   - 觉醒类型选择（武器：攻/魔/道）
// 网络：AwakeningNeedMaterials → 材料需求；Awakening → 觉醒结果（服务端全链路已支持）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::inventory::InvItem;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetworkContext;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};

/// 材料需求行
#[derive(Clone, Default)]
pub struct MaterialRow {
    pub item_id: i32,
    pub count: i32,
}

/// 觉醒状态
#[derive(Resource, Default)]
pub struct NpcAwakeState {
    /// 主物品 unique_id
    pub selected_uid: Option<u64>,
    /// 主物品（显示用）
    pub selected_item: Option<InvItem>,
    /// 觉醒类型（mir2_shared AwakeType；None=3）
    pub awake_type: Option<mir2_shared::enums::AwakeType>,
    /// 服务端返回的材料需求
    pub materials: Vec<MaterialRow>,
    /// 最近觉醒结果（1=成功 0=销毁 -1=失败 -2=满级 -3=金币不足 -4=材料不足）
    pub result: i32,
    pub result_text: String,
}

#[derive(Component)]
pub struct NpcAwakeWidget;

#[derive(Component)]
pub struct NpcAwakeClose;

#[derive(Component)]
pub struct NpcAwakeUpgrade;

#[derive(Component)]
pub struct NpcAwakeTypeBtn(pub mir2_shared::enums::AwakeType);

#[derive(Component)]
pub struct NpcAwakeMainIcon;

#[derive(Component)]
pub struct NpcAwakeMainName;

#[derive(Component)]
pub struct NpcAwakeMaterialText;

#[derive(Component)]
pub struct NpcAwakeResultText;

pub struct NpcAwakePlugin;

impl Plugin for NpcAwakePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NpcAwakeState>();
        app.add_systems(OnEnter(AppState::Game), spawn_npc_awake);
        app.add_systems(OnExit(AppState::Game), cleanup_npc_awake);
        app.add_systems(
            Update,
            (npc_awake_ui_system, npc_awake_render_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_npc_awake(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_npc_awake(
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

    // 面板 Title[710]（360x420）C# Location (0,0)
    let (pw, ph) = match libs.0.get_image(LibraryName::Title, 710) {
        Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
        None => (360.0, 420.0),
    };
    let px = 0.0;
    let py = 0.0;

    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 710) {
        let e = spawn_ui_sprite(&mut commands, h, px, py, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::NpcAwake),
            NpcAwakeWidget,
            Visibility::Hidden,
        ));
    }

    // 关闭 (284,4)
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        px + 284.0, py + 4.0, 7.0, 24.0, 21.0,
    ) {
        commands.entity(e).insert((
            NpcAwakeClose,
            DialogRoot(DialogKind::NpcAwake),
            NpcAwakeWidget,
        ));
    }

    // 升级按钮 Title[712/713/714] (115,391) 80x25
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 712, 713, 714,
        px + 115.0, py + 391.0, 7.0, 80.0, 25.0,
    ) {
        commands.entity(e).insert((
            NpcAwakeUpgrade,
            DialogRoot(DialogKind::NpcAwake),
            NpcAwakeWidget,
        ));
    }

    // 觉醒类型按钮（C# SelectAwakeType 下拉 (35,141)；此处简化为一排文本按钮）
    let types = [
        (mir2_shared::enums::AwakeType::Dc, "攻"),
        (mir2_shared::enums::AwakeType::Mc, "魔"),
        (mir2_shared::enums::AwakeType::Sc, "道"),
    ];
    for (i, (t, label)) in types.iter().enumerate() {
        let e = spawn_ui_text(
            &mut commands, &font, label,
            px + 35.0 + i as f32 * 34.0, py + 141.0,
            12.0, Color::srgb(1.0, 0.9, 0.1), 8.0,
        );
        commands.entity(e).insert((
            NpcAwakeTypeBtn(*t),
            DialogRoot(DialogKind::NpcAwake),
            NpcAwakeWidget,
        ));
        let _ = (e, px, py);
    }

    // 主物品格 (202,91)：图标 + 名字
    let empty = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    let icon = spawn_ui_sprite(&mut commands, empty.clone(), px + 202.0, py + 91.0, 7.0, 1.0);
    commands.entity(icon).insert((
        Sprite {
            image: empty,
            custom_size: Some(Vec2::new(36.0, 28.0)),
            ..default()
        },
        NpcAwakeMainIcon,
        DialogRoot(DialogKind::NpcAwake),
        NpcAwakeWidget,
    ));
    let name = spawn_ui_text(
        &mut commands, &font, "",
        px + 202.0, py + 122.0, 11.0, Color::WHITE, 8.0,
    );
    commands.entity(name).insert((
        NpcAwakeMainName,
        DialogRoot(DialogKind::NpcAwake),
        NpcAwakeWidget,
    ));

    // 材料需求标签 (67,317)/(192,317)
    for (x, _) in [(67.0, 0usize), (192.0, 1usize)] {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            px + x, py + 317.0, 11.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            NpcAwakeMaterialText,
            DialogRoot(DialogKind::NpcAwake),
            NpcAwakeWidget,
        ));
    }

    // 结果标签 (112,354)
    let res = spawn_ui_text(
        &mut commands, &font, "",
        px + 112.0, py + 354.0, 11.0, Color::srgb(1.0, 0.9, 0.1), 8.0,
    );
    commands.entity(res).insert((
        NpcAwakeResultText,
        DialogRoot(DialogKind::NpcAwake),
        NpcAwakeWidget,
    ));

    let _ = (pw, ph);
}

#[allow(clippy::too_many_arguments)]
fn npc_awake_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<NpcAwakeState>,
    net: ResMut<NetworkContext>,
    hud: Res<crate::game::hud::HudState>,
    close: Query<&UiButton, With<NpcAwakeClose>>,
    upgrade: Query<&UiButton, With<NpcAwakeUpgrade>>,
    type_btns: Query<&NpcAwakeTypeBtn>,
    mut widgets: Query<&mut Visibility, With<NpcAwakeWidget>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
) {
    use mir2_shared::packets::client::misc::{Awakening, AwakeningNeedMaterials};

    let open = mgr.is_open(DialogKind::NpcAwake);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }

    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::NpcAwake);
        }
    }

    // 类型按钮点击（C# SelectAwakeType.ValueChanged → AwakeningNeedMaterials）
    if let Ok(window) = windows.single() {
        if let Some(cursor) = window.cursor_position() {
            if mouse.just_pressed(MouseButton::Left) {
                for (i, t) in type_btns.iter().enumerate() {
                    let x = 35.0 + i as f32 * 34.0;
                    if cursor.x >= x && cursor.x <= x + 30.0 && cursor.y >= 141.0 && cursor.y <= 155.0 {
                        if let Some(uid) = state.selected_uid {
                            state.awake_type = Some(t.0);
                            net.send_packet(&AwakeningNeedMaterials {
                                unique_id: uid,
                                awake_type: t.0,
                            });
                            tracing::info!("⚒️ 选择觉醒类型 {:?}，请求材料 uid={}", t.0, uid);
                            state.result_text = String::new();
                        }
                        break;
                    }
                }
            }
        }
    }

    // 主物品格点击：循环选择背包武器（C# 从背包拖入）
    if let Ok(window) = windows.single() {
        if let Some(cursor) = window.cursor_position() {
            if mouse.just_pressed(MouseButton::Left)
                && cursor.x >= 202.0
                && cursor.x <= 238.0
                && cursor.y >= 91.0
                && cursor.y <= 119.0
            {
                let weapons: Vec<InvItem> = hud
                    .inventory
                    .items
                    .iter()
                    .flatten()
                    .filter(|it| it.item_type == 1)
                    .cloned()
                    .collect();
                if !weapons.is_empty() {
                    let cur = state
                        .selected_uid
                        .and_then(|u| weapons.iter().position(|w| w.unique_id == u))
                        .map(|i| i + 1)
                        .unwrap_or(0);
                    let item = weapons[cur % weapons.len()].clone();
                    state.selected_uid = Some(item.unique_id);
                    state.selected_item = Some(item.clone());
                    state.awake_type = None;
                    state.materials.clear();
                    state.result_text = String::new();
                    tracing::info!("⚒️ 选择觉醒物品: {} (uid={})", item.name, item.unique_id);
                }
            }
        }
    }

    // 升级按钮
    for btn in &upgrade {
        if btn.clicked {
            if let (Some(uid), Some(at)) = (state.selected_uid, state.awake_type) {
                net.send_packet(&Awakening {
                    unique_id: uid,
                    awake_type: at,
                    position_idx: 0,
                });
                tracing::info!("⚒️ 执行觉醒 uid={} type={:?}", uid, at);
            }
        }
    }
}

/// 渲染：主物品图标/名字 + 材料/结果标签
#[allow(clippy::too_many_arguments)]
fn npc_awake_render_system(
    mgr: Res<crate::game::dialogs::DialogManager>,
    state: Res<NpcAwakeState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut icon: Query<(&mut Sprite, &NpcAwakeMainIcon), Without<NpcAwakeMainName>>,
    mut name: Query<&mut Text2d, (With<NpcAwakeMainName>, Without<NpcAwakeMainIcon>, Without<NpcAwakeMaterialText>, Without<NpcAwakeResultText>)>,
    mut mats: Query<&mut Text2d, (With<NpcAwakeMaterialText>, Without<NpcAwakeResultText>, Without<NpcAwakeMainName>, Without<NpcAwakeMainIcon>)>,
    mut res: Query<&mut Text2d, (With<NpcAwakeResultText>, Without<NpcAwakeMaterialText>, Without<NpcAwakeMainName>, Without<NpcAwakeMainIcon>)>,
) {
    if !mgr.is_open(crate::game::dialogs::DialogKind::NpcAwake) {
        return;
    }
    for (mut sprite, _) in &mut icon {
        if let Some(item) = &state.selected_item {
            if let Some(h) = ui_image(
                &mut libs,
                &mut images,
                &mut cache,
                LibraryName::Items,
                item.image as usize,
            ) {
                sprite.image = h;
                sprite.custom_size = None;
            }
        }
    }
    for mut text in &mut name {
        text.0 = state
            .selected_item
            .as_ref()
            .map(|i| i.name.clone())
            .unwrap_or_default();
    }
    let mat_text: Vec<String> = state
        .materials
        .iter()
        .map(|m| format!("材料#{} x{}", m.item_id, m.count))
        .collect();
    for (i, mut text) in mats.iter_mut().enumerate() {
        text.0 = mat_text.get(i).cloned().unwrap_or_default();
    }
    for mut text in &mut res {
        text.0 = state.result_text.clone();
    }
}
