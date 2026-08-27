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
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_dropdown_ui, spawn_icon_button, spawn_image,
    spawn_label, spawn_panel, UiDropDown,
};

/// #1356：觉醒面板服务模式（C# PanelType：Awakening/Disassemble/Downgrade/Reset）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NpcAwakeService {
    #[default]
    Awaken,
    Disassemble,
    Downgrade,
    Reset,
}

impl NpcAwakeService {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Awaken => "觉醒",
            Self::Disassemble => "分解",
            Self::Downgrade => "降级",
            Self::Reset => "重置",
        }
    }
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Disassemble,
            2 => Self::Downgrade,
            3 => Self::Reset,
            _ => Self::Awaken,
        }
    }
}

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
    /// #1356：当前服务模式（觉醒/分解/降级/重置）
    pub service: NpcAwakeService,
}

#[derive(Component)]
pub struct NpcAwakeWidget;

#[derive(Component)]
pub struct NpcAwakeClose;

#[derive(Component)]
pub struct NpcAwakeUpgrade;

/// #1356：服务模式按钮（C# PanelType）
#[derive(Component)]
pub struct NpcAwakeServiceBtn(u8);

/// #1356：操作按钮文字（觉醒/分解/降级/重置）
#[derive(Component)]
pub struct NpcAwakeActionLabel;

#[derive(Component)]
pub struct NpcAwakeTypeDrop;

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
            awake_server_events.run_if(in_state(AppState::Game)),
        );
        app.add_systems(
            Update,
            (npc_awake_ui_system, npc_awake_render_system)
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

    // 面板 Title[710]（360x420）C# Location (0,0)
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 710) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, 0.0, 0.0, 360.0, 420.0, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::NpcAwake), NpcAwakeWidget));

    commands.entity(panel).with_children(|p| {
        // 关闭 Prguse2[360/361/362]（C# (284,4)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 284.0, 4.0, 24.0, 21.0, 10).insert(NpcAwakeClose);
        }
        // 升级按钮 Title[712/713/714]（C# (115,391)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 712),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 713),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 714),
        ) {
            spawn_icon_button(p, n, h, pr, 115.0, 391.0, 80.0, 25.0, 10)
                .insert((
                    NpcAwakeUpgrade,
                    // #93 通用 Tooltip：C# 升级按钮 Hint
                    crate::ui::tooltip::TooltipHint("消耗材料执行觉醒".to_string()),
                ));
        }
        // 觉醒类型下拉（C# SelectAwakeType (35,141)）
        spawn_dropdown_ui(
            p,
            &font,
            vec!["攻".to_string(), "魔".to_string(), "道".to_string()],
            None,
            (0.0, 0.0),
            35.0,
            141.0,
            72.0,
            18.0,
            3,
            9,
        )
        .insert(NpcAwakeTypeDrop);
        // #1356：服务模式按钮（C# PanelType：觉醒/分解/降级/重置）@(30+72i,26)
        for (i, label) in ["觉醒", "分解", "降级", "重置"].iter().enumerate() {
            spawn_container(p, 30.0 + i as f32 * 72.0, 26.0, 64.0, 20.0, 9)
                .insert((
                    Button,
                    NpcAwakeServiceBtn(i as u8),
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                ))
                .with_children(|b| {
                    b.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(2.0),
                            top: Val::Px(4.0),
                            ..default()
                        },
                        Text::new(*label),
                        TextFont {
                            font: FontSource::Handle(font.clone()),
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        ZIndex(1),
                    ));
                });
        }
        // #1356：操作按钮文字（升级按钮下方）
        spawn_label(p, &cjk, "觉醒", 118.0, 396.0, 12.0, Color::WHITE, 10)
            .insert(NpcAwakeActionLabel);
        // 主物品格（C# (202,91)）：图标 + 名字（白图占位，render 系统换物品图）
        let empty = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
        spawn_image(p, empty, 202.0, 91.0, 36.0, 28.0, 9).insert(NpcAwakeMainIcon);
        spawn_label(p, &cjk, "", 202.0, 122.0, 11.0, Color::WHITE, 9).insert(NpcAwakeMainName);
        // 材料需求标签（C# (67,317)/(192,317)）
        for x in [67.0, 192.0] {
            spawn_label(p, &cjk, "", x, 317.0, 11.0, Color::WHITE, 9)
                .insert(NpcAwakeMaterialText);
        }
        // 结果标签（C# GoldLabel (112,354)）
        spawn_label(p, &cjk, "", 112.0, 354.0, 11.0, Color::srgb(1.0, 0.9, 0.1), 9)
            .insert(NpcAwakeResultText);
    });
}

#[allow(clippy::too_many_arguments)]
fn npc_awake_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<NpcAwakeState>,
    net: ResMut<NetConnection>,
    inv_q: Query<&crate::game::player_state::Inventory, With<crate::actor::LocalPlayer>>,
    close: Query<(Entity, &Interaction), With<NpcAwakeClose>>,
    upgrade: Query<(Entity, &Interaction), With<NpcAwakeUpgrade>>,
    mut type_dd: Query<(&mut UiDropDown, &NpcAwakeTypeDrop)>,
    // B0001 互斥：widgets 与 action/type_vis/mat_vis 同写 Visibility——
    // widgets 侧补三对 Without（实体标记互斥，spawn 处各只挂自己的标记）
    mut widgets: Query<
        &mut Visibility,
        (
            With<NpcAwakeWidget>,
            Without<NpcAwakeActionLabel>,
            Without<NpcAwakeTypeDrop>,
            Without<NpcAwakeMaterialText>,
        ),
    >,
    service_btns: Query<(Entity, &Interaction, &NpcAwakeServiceBtn)>,
    mut action: Query<(&mut Text, &mut Visibility), With<NpcAwakeActionLabel>>,
    mut type_vis: Query<&mut Visibility, (With<NpcAwakeTypeDrop>, Without<NpcAwakeActionLabel>)>,
    mut mat_vis: Query<&mut Visibility, (With<NpcAwakeMaterialText>, Without<NpcAwakeTypeDrop>, Without<NpcAwakeActionLabel>)>,
    mouse: Res<ButtonInput<MouseButton>>,
    ui: (
        Query<&Window>,
        Query<&Node, With<NpcAwakeWidget>>,
    ),
    mut last_uid: Local<Option<u64>>,
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
    use mir2_shared::packets::client::misc::{Awakening, AwakeningNeedMaterials};

    let open = mgr.is_open(DialogKind::NpcAwake);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        // 关闭时也要隐藏下拉/材料/操作文字：它们不在 widgets 查询里，
        // 否则下拉框等会残留成屏幕上的孤按钮（觉醒类型下拉默认 Visible）
        for mut vis in &mut type_vis {
            *vis = Visibility::Hidden;
        }
        for mut vis in &mut mat_vis {
            *vis = Visibility::Hidden;
        }
        for (_, mut vis) in &mut action {
            *vis = Visibility::Hidden;
        }
        return;
    }
    // #1356：操作按钮文字 + 类型/材料显隐（非觉醒模式隐藏）
    for (mut text, mut vis) in &mut action {
        text.0 = state.service.label().to_string();
        *vis = Visibility::Visible;
    }
    for mut vis in &mut type_vis {
        *vis = if state.service == NpcAwakeService::Awaken { Visibility::Visible } else { Visibility::Hidden };
    }
    for mut vis in &mut mat_vis {
        *vis = if state.service == NpcAwakeService::Awaken { Visibility::Visible } else { Visibility::Hidden };
    }
    // #1356：服务模式切换（C# PanelType）
    for (e, inter, svc) in &service_btns {
        if edge(e, inter, &mut prev_inter) {
            let new_svc = NpcAwakeService::from_u8(svc.0);
            if new_svc != state.service {
                state.service = new_svc;
                state.selected_uid = None;
                state.selected_item = None;
                state.awake_type = None;
                state.materials.clear();
                state.result_text = String::new();
            }
        }
    }

    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::NpcAwake);
        }
    }

    // 觉醒类型下拉（#90 通用 DropDown）：选中变化 → AwakeningNeedMaterials
    const TYPES: [mir2_shared::enums::AwakeType; 3] = [
        mir2_shared::enums::AwakeType::Dc,
        mir2_shared::enums::AwakeType::Mc,
        mir2_shared::enums::AwakeType::Sc,
    ];
    if let Ok((mut dd, _)) = type_dd.single_mut() {
        // 非觉醒模式：收起下拉（防止弹出面板残留）
        if state.service != NpcAwakeService::Awaken {
            dd.open = false;
        }
        // 换了主物品 → 清空类型选择
        if *last_uid != state.selected_uid {
            *last_uid = state.selected_uid;
            dd.selected = None;
            state.awake_type = None;
        }
        let new_type = dd.selected.and_then(|i| TYPES.get(i).copied());
        if new_type != state.awake_type {
            if let (Some(uid), Some(t)) = (state.selected_uid, new_type) {
                state.awake_type = Some(t);
                net.send_packet(&AwakeningNeedMaterials {
                    unique_id: uid,
                    awake_type: t,
                });
                tracing::info!("⚒️ 选择觉醒类型 {:?}，请求材料 uid={}", t, uid);
                state.result_text = String::new();
            } else {
                state.awake_type = None;
            }
        }
    }

    // 主物品格点击：循环选择背包武器（C# 从背包拖入）
    if let Ok(window) = ui.0.single() {
        if let Some(cursor) = window.cursor_position() {
            let (ox, oy) = ui
                .1
                .single()
                .map(|n| crate::ui::theme::node_origin(n, (0.0, 0.0)))
                .unwrap_or((0.0, 0.0));
            if mouse.just_pressed(MouseButton::Left)
                && cursor.x >= ox + 202.0
                && cursor.x <= ox + 238.0
                && cursor.y >= oy + 91.0
                && cursor.y <= oy + 119.0
            {
                // #1356：觉醒模式循环武器；分解/降级/重置循环全部物品
                let items = inv_q.single().map(|inv| inv.items.as_slice()).unwrap_or(&[]);
                let pool: Vec<InvItem> = if state.service == NpcAwakeService::Awaken {
                    items.iter().flatten().filter(|it| it.item_type == 1).cloned().collect()
                } else {
                    items.iter().flatten().cloned().collect()
                };
                if !pool.is_empty() {
                    let cur = state
                        .selected_uid
                        .and_then(|u| pool.iter().position(|w| w.unique_id == u))
                        .map(|i| i + 1)
                        .unwrap_or(0);
                    let item = pool[cur % pool.len()].clone();
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

    // 操作按钮：按服务发包（C# PanelType 语义）
    for (e, inter) in &upgrade {
        if edge(e, inter, &mut prev_inter) {
            match state.service {
                NpcAwakeService::Awaken => {
                    if let (Some(uid), Some(at)) = (state.selected_uid, state.awake_type) {
                        net.send_packet(&Awakening {
                            unique_id: uid,
                            awake_type: at,
                            position_idx: 0,
                        });
                        tracing::info!("⚒️ 执行觉醒 uid={} type={:?}", uid, at);
                    }
                }
                NpcAwakeService::Disassemble => {
                    if let Some(uid) = state.selected_uid {
                        net.send_packet(&mir2_shared::packets::client::misc::DisassembleItem { unique_id: uid });
                        tracing::info!("🔧 分解物品 uid={}", uid);
                    }
                }
                NpcAwakeService::Downgrade => {
                    if let Some(uid) = state.selected_uid {
                        net.send_packet(&mir2_shared::packets::client::misc::DowngradeAwakening { unique_id: uid });
                        tracing::info!("⬇️ 觉醒降级 uid={}", uid);
                    }
                }
                NpcAwakeService::Reset => {
                    if let Some(uid) = state.selected_uid {
                        net.send_packet(&mir2_shared::packets::client::misc::ResetAddedItem { unique_id: uid });
                        tracing::info!("🔄 重置附加属性 uid={}", uid);
                    }
                }
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
    mut icon: Query<(&mut ImageNode, &NpcAwakeMainIcon), Without<NpcAwakeMainName>>,
    mut name: Query<&mut Text, (With<NpcAwakeMainName>, Without<NpcAwakeMainIcon>, Without<NpcAwakeMaterialText>, Without<NpcAwakeResultText>)>,
    mut mats: Query<&mut Text, (With<NpcAwakeMaterialText>, Without<NpcAwakeResultText>, Without<NpcAwakeMainName>, Without<NpcAwakeMainIcon>)>,
    mut res: Query<&mut Text, (With<NpcAwakeResultText>, Without<NpcAwakeMaterialText>, Without<NpcAwakeMainName>, Without<NpcAwakeMainIcon>)>,
) {
    if !mgr.is_open(crate::game::dialogs::DialogKind::NpcAwake) {
        return;
    }
    for (mut node, _) in &mut icon {
        if let Some(item) = &state.selected_item {
            if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Items, item.image as usize) {
                node.image = h;
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


/// 消费服务端觉醒事件（网络层只广播 ServerEvent）
fn awake_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut awake: ResMut<NpcAwakeState>,
    mut mgr: ResMut<DialogManager>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::AwakeningMaterials { materials } => {
                awake.materials = materials
                    .iter()
                    .map(|(item_id, count)| MaterialRow {
                        item_id: *item_id,
                        count: *count,
                    })
                    .collect();
            }
            ServerEvent::AwakeningResult { result, result_text } => {
                awake.result = *result;
                awake.result_text = result_text.clone();
            }
            ServerEvent::NpcAwakePanel { service } => {
                // #1356：C# S.NPCAwakening/S.NPCDisassemble/S.NPCDowngrade/S.NPCReset → 打开面板
                awake.service = NpcAwakeService::from_u8(*service);
                awake.selected_uid = None;
                awake.selected_item = None;
                awake.awake_type = None;
                awake.materials.clear();
                awake.result_text = String::new();
                mgr.open(DialogKind::NpcAwake);
            }
            _ => {}
        }
    }
}
