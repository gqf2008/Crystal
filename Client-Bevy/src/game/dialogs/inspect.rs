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
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiEntity, UiFont,
    UiImageCache,
};

/// 背景 Prguse[430] 左贴右缘（C# :2153-2155 Location(536,0)）
pub const BG_X: f32 = 536.0;
pub const BG_Y: f32 = 0.0;
/// 角色页 Prguse[340] @(8,70)（C# :2159-2165）
pub const PAGE_REL: (f32, f32) = (8.0, 70.0);

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
            (inspect_ui_system, inspect_icon_system, ui_button_system)
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
    mut cache: ResMut<UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 背景 + 角色页（C# :2153-2165）
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 430) {
        let e = spawn_ui_sprite(&mut commands, h, BG_X, BG_Y, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Inspect),
            InspectWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 340) {
        let e = spawn_ui_sprite(
            &mut commands,
            h,
            BG_X + PAGE_REL.0,
            BG_Y + PAGE_REL.1,
            6.1,
            1.0,
        );
        commands.entity(e).insert((
            InspectPage,
            DialogRoot(DialogKind::Inspect),
            InspectWidget,
            Visibility::Hidden,
        ));
    }
    // 关闭 @(241,3)（C# MainDialogs.cs:2213；509 是 HelpDialog 宽面板坐标，审查 MAJOR）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Prguse2,
        360,
        361,
        362,
        BG_X + 241.0,
        BG_Y + 3.0,
        7.0,
        20.0,
        20.0,
    ) {
        commands.entity(e).insert((
            InspectClose,
            DialogRoot(DialogKind::Inspect),
            InspectWidget,
        ));
    }
    // 名字（8F HCenter|VCenter 190x20 @(50,12) → 框心 (145,22)，C# :2317-2324；
    // C# 文本=Name 纯名字，等级/职业不进此标签——对齐）
    // 行会（190x30 @(50,33) → 框心 (145,48)，C# :2343-2350；C# 文本=GuildName+" "
    // +GuildRank，无 rank 数据只显示名——有意偏差附 #2607）
    for (is_name, cx, cy) in [(true, 145.0, 22.0), (false, 145.0, 48.0)] {
        let t = spawn_ui_text(
            &mut commands,
            &font,
            "",
            BG_X + cx,
            BG_Y + cy,
            8.0,
            Color::WHITE,
            8.0,
        );
        let mut ec = commands.entity(t);
        if is_name {
            ec.insert((
                InspectNameText,
                bevy::sprite::Anchor::CENTER,
                DialogRoot(DialogKind::Inspect),
                InspectWidget,
            ));
        } else {
            ec.insert((
                InspectGuildText,
                bevy::sprite::Anchor::CENTER,
                DialogRoot(DialogKind::Inspect),
                InspectWidget,
            ));
        }
    }
    // 14 格装备图标（格位坐标 = character::EQUIP_SLOTS，页内 @(8,70) 偏移；
    // C# :2362-2469 MirItemCell GridType=Inspect）
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    // 纸娃娃层（C# :2166-2206：StateItems 画护甲→武器→头盔。锚点=**对话框**
    // 原点+(0,-20)=(536,-20)——λ 里的 DisplayLocation 词法上是 InspectDialog
    // 自身而非 CharacterPage（审查 MAJOR 修正误读；交叉验证 CharacterDialog
    // 纸娃娃相对页同为 (-8,-90)）；offSet:true 的图库内偏移在渲染时叠加）
    // z 按层叠 7.3/7.35/7.4 在角色页(6.1)之上、格图标(7.5)之下
    for (slot, z) in [(1u8, 7.3), (0u8, 7.35), (2u8, 7.4)] {
        commands.spawn((
            UiEntity,
            InspectDoll(slot),
            DialogRoot(DialogKind::Inspect),
            InspectWidget,
            Sprite {
                image: white.clone(),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(BG_X, -(BG_Y - 20.0), z),
            Visibility::Hidden,
        ));
    }
    // 职业图标 Prguse[100]@(15,33)（按 class 换帧在 icon_system）
    {
        let e = spawn_ui_sprite(
            &mut commands,
            white.clone(),
            BG_X + 15.0,
            BG_Y + 33.0,
            7.2,
            1.0,
        );
        commands.entity(e).insert((
            InspectClassImage,
            DialogRoot(DialogKind::Inspect),
            InspectWidget,
            Visibility::Hidden,
        ));
    }
    // 五动作按钮（C# :2223-2315；库/帧/坐标逐字：Group Prguse[431-433]@(55,357)、
    // Friend [434-436]@(85,357)、Mail [437-439]@(115,357)、Trade [523-525]@(145,357)、
    // Observe Title[854-856]@(175,357)）
    let buttons: [(InspectBtnKind, LibraryName, usize, usize, usize, f32); 5] = [
        (InspectBtnKind::Group, LibraryName::Prguse, 431, 432, 433, 55.0),
        (InspectBtnKind::Friend, LibraryName::Prguse, 434, 435, 436, 85.0),
        (InspectBtnKind::Mail, LibraryName::Prguse, 437, 438, 439, 115.0),
        (InspectBtnKind::Trade, LibraryName::Prguse, 523, 524, 525, 145.0),
        (InspectBtnKind::Observe, LibraryName::Title, 854, 855, 856, 175.0),
    ];
    for (kind, lib, n, h, p, rx) in buttons {
        if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
            &mut commands,
            &mut libs,
            &mut images,
            &mut cache,
            lib,
            n,
            h,
            p,
            BG_X + rx,
            BG_Y + 357.0,
            7.0,
            28.0,
            24.0,
        ) {
            commands.entity(e).insert((
                InspectBtn(kind),
                DialogRoot(DialogKind::Inspect),
                InspectWidget,
            ));
        }
    }
    for (pos, (cx, cy)) in EQUIP_SLOTS.iter().enumerate() {
        commands.spawn((
            UiEntity,
            InspectCellIcon(pos),
            DialogRoot(DialogKind::Inspect),
            InspectWidget,
            Sprite {
                image: white.clone(),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(BG_X + PAGE_REL.0 + cx, -(BG_Y + PAGE_REL.1 + cy), 7.5),
            Visibility::Hidden,
        ));
    }
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
    close: Query<&UiButton, (With<InspectClose>, Without<InspectBtn>)>,
    // 单查询分发（多 With<marker> 查询有 B0001 风险）；接线同 player_menu 既有实现
    btns: Query<(&UiButton, &InspectBtn)>,
    mut widgets: Query<&mut Visibility, (With<InspectWidget>, Without<InspectCellIcon>)>,
    mut names: Query<
        &mut Text2d,
        (With<InspectNameText>, Without<InspectGuildText>),
    >,
    mut guilds: Query<
        &mut Text2d,
        (With<InspectGuildText>, Without<InspectNameText>),
    >,
) {
    let open = mgr.is_open(DialogKind::Inspect);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Inspect);
        }
    }
    for (b, k) in btns.iter() {
        if !b.clicked {
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
    mut cache: ResMut<UiImageCache>,
    mut icons: Query<(&mut Sprite, &mut Visibility, &InspectCellIcon)>,
    // 角色页性别换帧（C# RefreshInferface :2474-2476：340 男 / 341 女）
    mut page: Query<&mut Sprite, (With<InspectPage>, Without<InspectCellIcon>, Without<InspectClassImage>, Without<InspectDoll>)>,
    // 职业图标
    mut class_img: Query<
        &mut Sprite,
        (
            With<InspectClassImage>,
            Without<InspectCellIcon>,
            Without<InspectPage>,
            Without<InspectDoll>,
        ),
    >,
    // 纸娃娃层
    mut dolls: Query<
        (&mut Sprite, &mut Transform, &mut Visibility, &InspectDoll),
        (
            Without<InspectCellIcon>,
            Without<InspectPage>,
            Without<InspectClassImage>,
        ),
    >,
) {
    if !mgr.is_open(DialogKind::Inspect) {
        for (_, mut vis, _) in &mut icons {
            *vis = Visibility::Hidden;
        }
        for (_, _, mut vis, _) in &mut dolls {
            *vis = Visibility::Hidden;
        }
        return;
    }
    let page_idx = if state.gender == 1 { 341 } else { 340 };
    for mut sp in &mut page {
        if let Some(h) = ui_image(
            &mut libs,
            &mut images,
            &mut cache,
            LibraryName::Prguse,
            page_idx,
        ) {
            sp.image = h;
            sp.custom_size = None;
        }
    }
    // 职业图标（C# :2480-2494：Index = 100 + Class）
    for mut sp in &mut class_img {
        if let Some(h) = ui_image(
            &mut libs,
            &mut images,
            &mut cache,
            LibraryName::Prguse,
            100 + (state.class as usize).min(4),
        ) {
            sp.image = h;
            sp.custom_size = None;
        }
    }
    // 纸娃娃（C# :2166-2206：StateItems[RealItem.Image]，锚点=对话框原点+(0,-20)
    // + 图库内偏移 offSet:true（MLibrary.cs:732）；偏差附 #2609——GetRealItem
    // 按等级/职业换 image 与翅膀 Effect/发型不在包内，直接用 image）
    for (mut sp, mut tf, mut vis, doll) in &mut dolls {
        let it = state.items.iter().find(|i| i.slot == doll.0 && i.image > 0);
        match it {
            Some(it) => {
                if let Some(h) = ui_image(
                    &mut libs,
                    &mut images,
                    &mut cache,
                    LibraryName::StateItems,
                    it.image as usize,
                ) {
                    sp.image = h;
                    sp.custom_size = None;
                    // 图库内偏移并入位置（C# offSet:true：DisplayLocation += mi 偏移）
                    let (ox, oy) = libs
                        .0
                        .get_image(LibraryName::StateItems, it.image as usize)
                        .map(|i| (i.offset_x as f32, i.offset_y as f32))
                        .unwrap_or((0.0, 0.0));
                    tf.translation.x = BG_X + ox;
                    tf.translation.y = -(BG_Y - 20.0) - oy;
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
    for (mut sprite, mut vis, cell) in &mut icons {
        match by_pos.get(cell.0).copied().flatten() {
            Some(it) if it.image > 0 => {
                if let Some(h) = ui_image(
                    &mut libs,
                    &mut images,
                    &mut cache,
                    LibraryName::Items,
                    it.image as usize,
                ) {
                    sprite.image = h;
                    sprite.custom_size = None;
                    *vis = Visibility::Visible;
                    continue;
                }
                *vis = Visibility::Hidden;
            }
            _ => *vis = Visibility::Hidden,
        }
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
