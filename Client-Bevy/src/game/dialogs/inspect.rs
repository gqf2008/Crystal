// ============================================================================
// 查看玩家对话框（M46 → #2607 批T 对齐 C# InspectDialog 14 格装备网格）
// C# MainDialogs.cs:2113-2526：背景 Prguse[430]@(536,0)、角色页 Prguse[340/341]@(8,70)
//   （男/女换帧）、14 格装备网格（EQUIP_SLOTS 同角色对话框坐标）、名字/行会标签
//   （框心 (145,22)/(145,48)）、Close@(241,3)。
// 网络（#2607 协议加 slot+image）：
//   C: Inspect[object_id u32]
//   S: PlayerInspect[oid u32][name dotnet][guild dotnet][level u16][class u8]
//      [gender u8][count u8][per: slot u8][uid u64][index i32][image i32][dura i32][max_dura i32]
// 有意偏差（附 #2607/#2609 记录）：纸娃娃 GetRealItem 等级/职业换图、翅膀
//   Effect、发型（Hair 不在 InspectPlayer 协议内）；Observe 无 AllowObserve
//   守卫（协议未携带该字段，服务端兜底）；Group 无队伍满员/队长预检
//   （C# 满员不发包、非队长仅警告后仍发包——移植直接发包由服务端裁决）；
//   行会标签无 rank 数据只显示名
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::character::EQUIP_SLOTS;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use crate::ui::theme::{
    load_lib_image, spawn_icon_button, spawn_image, spawn_item_cell_ui, spawn_label_center,
    UiItemCellData,
};

/// 背景 Prguse[430] 左贴右缘（C# :2153-2155 Location(536,0)）
pub const BG_X: f32 = 536.0;
pub const BG_Y: f32 = 0.0;
/// 角色页 Prguse[340] @(8,70)（C# :2159-2165）
pub const PAGE_REL: (f32, f32) = (8.0, 70.0);
/// 装备格尺寸（C# 36x32，同角色对话框）
pub const SLOT_W: f32 = 36.0;
pub const SLOT_H: f32 = 32.0;

/// 装备条目（#2607：slot=服务端旧序槽位，image=Items 库图标索引）
#[derive(Debug, Clone, Default)]
pub struct InspectItem {
    pub slot: u8,
    pub unique_id: u64,
    pub item_index: i32,
    pub image: i32,
    pub current_dura: i32,
    pub max_dura: i32,
}

/// 查看状态（PlayerInspect 写入）
#[derive(Resource, Default)]
pub struct InspectState {
    pub name: String,
    pub guild: String,
    pub level: u16,
    pub class: u8,
    pub gender: u8,
    /// 允许观察（#2611：Observe 按钮门控，服务端下发）
    pub allow_observe: bool,
    pub items: Vec<InspectItem>,
}

#[derive(Component)]
pub struct InspectWidget;

#[derive(Component)]
pub struct InspectClose;

/// 角色页精灵（性别换帧 340/341 用，C# RefreshInferface :2474-2476）
#[derive(Component)]
pub struct InspectPage;

/// 职业图标（Prguse[100+class]@(15,33)，C# :2352-2359/:2480-2494）
#[derive(Component)]
pub struct InspectClassImage;

/// 纸娃娃层（C# :2166-2206 StateItems 画装备，锚点=对话框原点+(0,-20)+
/// 图库内偏移；.0 = 服务端旧序槽位（0 武器/1 护甲/2 头盔），绘制顺序=枚举序）
#[derive(Component)]
pub struct InspectDoll(pub u8);

/// 五动作按钮（C# :2223-2315）：组队/加友/邮件/交易/观察
#[derive(Component)]
pub struct InspectBtn(pub InspectBtnKind);

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum InspectBtnKind {
    Group,
    Friend,
    Mail,
    Trade,
    Observe,
}

/// 名字/行会标签（C# NameLabel/GuildLabel）
#[derive(Component)]
pub struct InspectNameText;

#[derive(Component)]
pub struct InspectGuildText;

/// 装备图标格（.0 = C# 格位 0..13，非服务端槽位）
#[derive(Component)]
pub struct InspectCellIcon(usize);

pub struct InspectPlugin;

impl Plugin for InspectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InspectState>();
        app.add_systems(OnEnter(AppState::Game), spawn_inspect);
        app.add_systems(OnExit(AppState::Game), cleanup_inspect);
        app.add_systems(
            Update,
            inspect_server_events.run_if(in_state(AppState::Game)),
        );
        app.add_systems(
            Update,
            (inspect_ui_system, inspect_icon_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_inspect(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_inspect(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 背景 Prguse[430]（C# MainDialogs.cs:2153，264x408 @ (536,0)）。
    // 不加 Overflow::clip：纸娃娃锚点在面板上方 y=-20（C# 语义）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 430) else {
        return;
    };
    let panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(BG_X),
                top: Val::Px(BG_Y),
                width: Val::Px(264.0),
                height: Val::Px(408.0),
                ..default()
            },
            ImageNode::new(bg),
            DialogRoot(DialogKind::Inspect),
            InspectWidget,
            GlobalZIndex(30),
            Visibility::Hidden,
        ))
        .id();

    commands.entity(panel).with_children(|p| {
        // 角色页 Prguse[340/341] @(8,70)（性别换帧由 icon_system）
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 340) {
            spawn_image(p, h, PAGE_REL.0, PAGE_REL.1, 248.0, 284.0, 8).insert(InspectPage);
        }
        // 关闭 @(241,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 241.0, 3.0, 20.0, 20.0, 10).insert(InspectClose);
        }
        // 名字（8F 居中 @ 框心 (145,22)）/ 行会（@ 框心 (145,48)）
        spawn_label_center(p, &font, "", 145.0, 18.0, 190.0, 8.0, Color::WHITE, 9)
            .insert(InspectNameText);
        spawn_label_center(p, &font, "", 145.0, 44.0, 190.0, 8.0, Color::WHITE, 9)
            .insert(InspectGuildText);
        // 纸娃娃层（C# :2166-2206 锚点=对话框原点+(0,-20)，z 层叠）
        for (slot, z) in [(1u8, 9u8), (0u8, 10u8), (2u8, 11u8)] {
            let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
            spawn_image(p, white, 0.0, -20.0, 1.0, 1.0, z as i32).insert(InspectDoll(slot));
        }
        // 职业图标 Prguse[100]@(15,33)（按 class 换帧在 icon_system）
        let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
        spawn_image(p, white, 15.0, 33.0, 1.0, 1.0, 9).insert(InspectClassImage);
        // 五动作按钮（C# :2223-2315；Group Prguse[431-433]@(55,357)、Friend [434-436]@(85,357)、
        // Mail [437-439]@(115,357)、Trade [523-525]@(145,357)、Observe Title[854-856]@(175,357)）
        let buttons: [(InspectBtnKind, LibraryName, usize, usize, usize, f32); 5] = [
            (InspectBtnKind::Group, LibraryName::Prguse, 431, 432, 433, 55.0),
            (InspectBtnKind::Friend, LibraryName::Prguse, 434, 435, 436, 85.0),
            (InspectBtnKind::Mail, LibraryName::Prguse, 437, 438, 439, 115.0),
            (InspectBtnKind::Trade, LibraryName::Prguse, 523, 524, 525, 145.0),
            (InspectBtnKind::Observe, LibraryName::Title, 854, 855, 856, 175.0),
        ];
        for (kind, lib, n, h, pidx, rx) in buttons {
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, lib, n),
                load_lib_image(&mut libs, &mut images, lib, h),
                load_lib_image(&mut libs, &mut images, lib, pidx),
            ) {
                spawn_icon_button(p, n, h, pr, rx, 357.0, 28.0, 24.0, 10)
                    .insert(InspectBtn(kind));
            }
        }
        // 14 格装备图标（格位坐标 = character::EQUIP_SLOTS，页内 @(8,70) 偏移；
        // 数据写 UiItemCellData 由 item_cell_ui_system 渲染）
        for (pos, (cx, cy)) in EQUIP_SLOTS.iter().enumerate() {
            spawn_item_cell_ui(
                p,
                &mut images,
                &font,
                PAGE_REL.0 + cx,
                PAGE_REL.1 + cy,
                SLOT_W,
                SLOT_H,
                9,
                pos,
            )
            .insert(InspectCellIcon(pos));
        }
    });
}

/// 显隐 + 标签 + 关闭 + 五动作按钮（图标显隐完全由 inspect_icon_system 管理——
/// 审查 MAJOR：两系统都写图标 Visibility 会在关闭后每帧互相打架，图标悬浮不消失）
fn inspect_ui_system(
    mut mgr: ResMut<DialogManager>,
    state: Res<InspectState>,
    net: Res<crate::network::NetConnection>,
    mut mail: ResMut<crate::game::dialogs::mail::MailState>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    mut chat: ResMut<crate::game::chat::ChatState>,
    close: Query<(Entity, &Interaction), (With<InspectClose>, Without<InspectBtn>)>,
    // 单查询分发（多 With<marker> 查询有 B0001 风险）；接线同 player_menu 既有实现
    btns: Query<(Entity, &Interaction, &InspectBtn)>,
    mut widgets: Query<&mut Visibility, (With<InspectWidget>, Without<InspectCellIcon>)>,
    mut names: Query<
        &mut Text,
        (With<InspectNameText>, Without<InspectGuildText>),
    >,
    mut guilds: Query<
        &mut Text,
        (With<InspectGuildText>, Without<InspectNameText>),
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
    let open = mgr.is_open(DialogKind::Inspect);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::Inspect);
        }
    }
    for (e, inter, k) in btns.iter() {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        match k.0 {
            // C# :2234-2250：C.AddMember{Name}
            InspectBtnKind::Group => {
                net.send_packet(&mir2_shared::packets::client::group::AddMember {
                    name: state.name.clone(),
                });
                tracing::info!("👥 [查看] 邀请组队: {}", state.name);
            }
            // C# :2263-2266：C.AddFriend{Name, Blocked=false}
            InspectBtnKind::Friend => {
                net.send_packet(&mir2_shared::packets::client::friend::AddFriend {
                    name: state.name.clone(),
                    blocked: false,
                });
                tracing::info!("👥 [查看] 添加好友 {}", state.name);
            }
            // C# :2279：MailComposeLetterDialog.ComposeMail(Name)（同 player_menu 邮件路径）
            InspectBtnKind::Mail => {
                mgr.open.push(DialogKind::Mail);
                mail.compose = true;
                mail.detail = None;
                mail.attach = vec![None; 5];
                mail.compose_gold = 0;
                if input.texts.len() < 4 {
                    input.texts.resize(4, String::new());
                }
                input.texts[0] = state.name.clone();
                input.active = None;
                tracing::info!("✉️ [查看] 给 {} 写邮件", state.name);
            }
            // C# :2292：C.TradeRequest
            InspectBtnKind::Trade => {
                net.send_packet(&mir2_shared::packets::client::trade::TradeRequest);
                tracing::info!("🤝 [查看] 请求交易");
            }
            // C# :2305-2315：C.Observe{Name}；AllowObserve=false 时不发包、
            // 聊天系统消息提示（#2611：协议已带该字段，守卫落地）
            InspectBtnKind::Observe => {
                if state.allow_observe {
                    net.send_packet(&crate::network::ObserveWire {
                        name: state.name.clone(),
                    });
                    tracing::info!("👁️ [查看] 观察玩家 {}", state.name);
                } else {
                    chat.add_line(
                        format!("{} 禁用了观察", state.name),
                        Color::srgb(1.0, 0.85, 0.3),
                        crate::game::chat::ChatChannel::System,
                    );
                }
            }
        }
    }
    for mut t in &mut names {
        // C# NameLabel.Text = Name（纯名字，:2317-2324；等级/职业不进此标签）
        t.0 = state.name.clone();
    }
    for mut t in &mut guilds {
        // C# = GuildName+" "+GuildRank（:2343-2350）；无 rank 数据只显示名（偏差附 #2607）
        t.0 = if state.guild.is_empty() {
            String::new()
        } else {
            state.guild.clone()
        };
    }
}

/// 图标渲染：服务端槽位（旧序）→ C# 格位（SERVER_SLOT_TO_POS）→ Items[image]。
/// 关闭时整体隐藏（审查 MAJOR：缺 is_open 门控则关闭后已装备图标悬浮不消失）
fn inspect_icon_system(
    state: Res<InspectState>,
    mgr: Res<DialogManager>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cells: Query<(&InspectCellIcon, &mut UiItemCellData)>,
    // 角色页性别换帧（C# RefreshInferface :2474-2476：340 男 / 341 女）
    mut page: Query<&mut ImageNode, (With<InspectPage>, Without<InspectCellIcon>, Without<InspectClassImage>, Without<InspectDoll>)>,
    // 职业图标
    mut class_img: Query<
        &mut ImageNode,
        (
            With<InspectClassImage>,
            Without<InspectCellIcon>,
            Without<InspectPage>,
            Without<InspectDoll>,
        ),
    >,
    // 纸娃娃层
    mut dolls: Query<
        (&mut ImageNode, &mut Node, &mut Visibility, &InspectDoll),
        (
            Without<InspectCellIcon>,
            Without<InspectPage>,
            Without<InspectClassImage>,
        ),
    >,
) {
    if !mgr.is_open(DialogKind::Inspect) {
        for (_, mut data) in &mut cells {
            data.icon = None;
        }
        for (_, _, mut vis, _) in &mut dolls {
            *vis = Visibility::Hidden;
        }
        return;
    }
    let page_idx = if state.gender == 1 { 341 } else { 340 };
    for mut node in &mut page {
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, page_idx) {
            node.image = h;
        }
    }
    // 职业图标（C# :2480-2494：Index = 100 + Class）
    for mut node in &mut class_img {
        if let Some(h) = load_lib_image(
            &mut libs,
            &mut images,
            LibraryName::Prguse,
            100 + (state.class as usize).min(4),
        ) {
            node.image = h;
        }
    }
    // 纸娃娃（C# :2166-2206：StateItems[RealItem.Image]，锚点=面板原点+(0,-20)
    // + 图库内偏移 offSet:true（MLibrary.cs:732）；偏差附 #2609）
    for (mut node, mut tf, mut vis, doll) in &mut dolls {
        let it = state.items.iter().find(|i| i.slot == doll.0 && i.image > 0);
        match it {
            Some(it) => {
                if let Some(h) = load_lib_image(
                    &mut libs,
                    &mut images,
                    LibraryName::StateItems,
                    it.image as usize,
                ) {
                    node.image = h;
                    // 图库内偏移并入位置（C# offSet:true：DisplayLocation += mi 偏移）
                    let (ox, oy) = libs
                        .0
                        .get_image(LibraryName::StateItems, it.image as usize)
                        .map(|i| (i.offset_x as f32, i.offset_y as f32))
                        .unwrap_or((0.0, 0.0));
                    tf.left = Val::Px(ox);
                    tf.top = Val::Px(-20.0 - oy);
                    *vis = Visibility::Visible;
                    continue;
                }
                *vis = Visibility::Hidden;
            }
            None => *vis = Visibility::Hidden,
        }
    }
    let mut by_pos = [None::<&InspectItem>; 14];
    for it in &state.items {
        let pos = crate::game::dialogs::character::SERVER_SLOT_TO_POS
            .get(it.slot as usize)
            .copied();
        if let Some(pos) = pos {
            by_pos[pos] = Some(it);
        }
    }
    for (cell, mut data) in &mut cells {
        data.icon = match by_pos.get(cell.0).copied().flatten() {
            Some(it) if it.image > 0 => load_lib_image(
                &mut libs,
                &mut images,
                LibraryName::Items,
                it.image as usize,
            ),
            _ => None,
        };
    }
}

/// 消费服务端查看事件（网络层只广播 ServerEvent）
fn inspect_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut mgr: ResMut<DialogManager>,
    mut inspect: ResMut<InspectState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::InspectPlayer { name, guild, level, class, gender, allow_observe, items } = ev {
            inspect.name = name.clone();
            inspect.guild = guild.clone();
            inspect.level = *level;
            inspect.class = *class;
            inspect.gender = *gender;
            inspect.allow_observe = *allow_observe;
            inspect.items = items.clone();
            mgr.open(DialogKind::Inspect);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 布局常量对齐 C#（MainDialogs.cs:2153-2165）
    #[test]
    fn layout_matches_csharp() {
        assert_eq!((BG_X, BG_Y), (536.0, 0.0));
        assert_eq!(PAGE_REL, (8.0, 70.0));
        // 首格 = Weapon(123,7)（C# :2362-2369，格坐标与角色对话框 EQUIP_SLOTS 同源）
        assert_eq!(EQUIP_SLOTS[0], (123.0, 7.0));
        assert_eq!(EQUIP_SLOTS.len(), 14);
    }
}
