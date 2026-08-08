// ============================================================================
// 行会对话框（M27）
// 布局参考：C# GuildDialog.cs / macroquad guild_dialog.rs
//   - 背景 Prguse[956]，标题 Title[15]，位置 (280,80)
//   - 行会名/会长/金币、成员列表（职务+在线）、公告、创建输入框
// 网络：GuildStatus（1 字节 in_guild / 完整信息，同 opcode 双格式）、GuildNoticeChange、GuildMemberChange
// ============================================================================

use bevy::prelude::*;
use std::collections::HashMap;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};
use crate::ui::controls::{spawn_dropdown, DropDown};
use crate::ui::scroll_list::{spawn_scroll_bar, ScrollList};

/// 行会成员
#[derive(Debug, Clone, Default)]
pub struct GuildMember {
    pub name: String,
    pub rank: u8,
    pub online: bool,
}

/// 行会仓库物品条目（GuildStorageList，M32）
#[derive(Debug, Clone, Default)]
pub struct StorageItem {
    pub unique_id: u64,
    pub item_index: i32,
    pub name: String,
    pub count: u16,
}

/// 行会状态
#[derive(Resource, Default)]
pub struct GuildState {
    pub in_guild: bool,
    pub name: String,
    pub leader: String,
    pub notice: Vec<String>,
    pub members: Vec<GuildMember>,
    pub gold: u32,
    /// 行会仓库物品（100 格，GuildStorageList 写入）
    pub storage_items: Vec<Option<StorageItem>>,
    /// 仓库列表是否已收到（E2E/UI 等待标记）
    pub storage_received: bool,
    /// 仓库翻页（每页 8 格，共 13 页）
    pub storage_page: usize,
    /// 选中的仓库格子（取出用）
    pub selected_storage: Option<usize>,
    /// 物品名缓存（item_index → name，来自 UserInformation 内嵌 ItemInfo）
    pub item_names: HashMap<i32, String>,
    /// 待处理行会邀请（行会名）
    pub invite: Option<String>,
    /// 选中的成员行（踢出用）
    pub selected_member: Option<usize>,
    /// #1348：是否显示离线成员（C# MembersShowOfflinesetting，默认 true）
    pub show_offline: bool,
    /// #1362：职务名（3 个，C# 自定义职务名简化；服务端 GuildStatus 下发）
    pub rank_names: [String; 3],
}

impl GuildState {
    /// #1348：可见成员下标（show_offline=false 时过滤离线；C# MembersShowOfflineSwitch）
    pub fn visible_member_indices(&self) -> Vec<usize> {
        self.members
            .iter()
            .enumerate()
            .filter(|(_, m)| self.show_offline || m.online)
            .map(|(i, _)| i)
            .collect()
    }

    /// 物品显示名：优先缓存名，回退 #index
    pub fn item_name(&self, index: i32) -> String {
        self.item_names
            .get(&index)
            .cloned()
            .unwrap_or_else(|| format!("#{}", index))
    }
}

#[derive(Component)]
pub struct GuildWidget;

#[derive(Component)]
pub struct GuildClose;

/// 创建行会输入框（TextInputState id 0）
#[derive(Component)]
pub struct GuildNameField;

#[derive(Component)]
pub struct GuildCreateBtn;

/// 邀请玩家输入框（TextInput id 1）
#[derive(Component)]
pub struct GuildInviteField;

#[derive(Component)]
pub struct GuildInviteBtn;

/// #1362：职务改名下拉（C# RanksSelectBox）
#[derive(Component)]
pub struct GuildRankDrop;
/// #1362：职务改名输入框（TextInput id 4）
#[derive(Component)]
pub struct GuildRankRenameField;
/// #1362：职务改名保存按钮（C# RanksSaveName）
#[derive(Component)]
pub struct GuildRankSaveBtn;

/// #1348：显示离线成员切换（C# MembersShowOfflineButton）
#[derive(Component)]
pub struct GuildShowOfflineBtn;

/// 踢出选中成员
#[derive(Component)]
pub struct GuildKickBtn;

/// 公告输入框（TextInput id 2）
#[derive(Component)]
pub struct GuildNoticeField;

#[derive(Component)]
pub struct GuildNoticeBtn;

/// 仓库金币输入框（TextInput id 3）
#[derive(Component)]
pub struct GuildGoldField;

#[derive(Component)]
pub struct GuildGoldDeposit;

#[derive(Component)]
pub struct GuildGoldWithdraw;

#[derive(Component)]
pub struct GuildItemDeposit;

#[derive(Component)]
pub struct GuildItemWithdraw;

#[derive(Component)]
pub struct GuildStorageUp;

#[derive(Component)]
pub struct GuildStorageDown;

// 邀请提示
#[derive(Component)]
pub struct GuildInviteWidget;

#[derive(Component)]
pub struct GuildInviteText;

#[derive(Component)]
pub struct GuildInviteYes;

#[derive(Component)]
pub struct GuildInviteNo;

#[derive(Component)]
pub struct GuildLine(usize);

pub struct GuildPlugin;

impl Plugin for GuildPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GuildState>();
        app.add_systems(OnEnter(AppState::Game), spawn_guild);
        app.add_systems(OnExit(AppState::Game), cleanup_guild);
        app.add_systems(
            Update,
            guild_server_events.run_if(in_state(AppState::Game)),
        );
        app.add_systems(
            Update,
            (
                guild_ui_system,
                guild_storage_system,
                guild_invite_system,
                guild_show_offline_system,
                guild_rank_rename_system,
                ui_button_system,
            )
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_guild(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_guild(
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

    // 背景 Prguse[956]
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 956) {
        let e = spawn_ui_sprite(&mut commands, h, 280.0, 80.0, 6.0, 1.0);
        // #89 可滚动成员列表：10 行 × 20px
        let (track, thumb) = spawn_scroll_bar(&mut commands, &mut images, (498.0, 140.0, 4.0, 200.0), 6.3);
        commands.entity(track).insert((DialogRoot(DialogKind::Guild), GuildWidget, Visibility::Visible));
        commands.entity(thumb).insert((
            DialogRoot(DialogKind::Guild),
            GuildWidget,
            Visibility::Visible,
        ));
        commands.entity(e).insert((
            DialogRoot(DialogKind::Guild),
            GuildWidget,
            Visibility::Hidden,
            ScrollList {
                rect_rel: (18.0, 60.0, 200.0, 200.0),
                row_h: 20.0,
                visible: 10,
                total: 0,
                offset: 0,
                step: 3,
                track_rel: (218.0, 60.0, 4.0, 200.0),
                thumb: Some(thumb),
                z: 8.0,
            },
        ));
    }
    // 标题 Title[15]
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 15) {
        let e = spawn_ui_sprite(&mut commands, h, 298.0, 88.0, 6.2, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Guild),
            GuildWidget,
            Visibility::Hidden,
        ));
    }
    // 关闭
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 340.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            GuildClose,
            DialogRoot(DialogKind::Guild),
            GuildWidget,
        ));
    }
    // 行会名/会长文本（GuildLine 0 占位显示头部）
    let head = spawn_ui_text(
        &mut commands, &font, "",
        298.0, 120.0, 12.0, Color::srgb(1.0, 0.9, 0.5), 8.0,
    );
    commands.entity(head).insert((
        GuildLine(0),
        DialogRoot(DialogKind::Guild),
        GuildWidget,
    ));
    // 成员列表（10 行，1..=10）
    for i in 1..=10usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            298.0, 140.0 + (i - 1) as f32 * 20.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            GuildLine(i),
            DialogRoot(DialogKind::Guild),
            GuildWidget,
        ));
    }
    // #1348：显示离线成员切换（C# MembersShowOfflineButton/Status @(230,310)，纯本地过滤）
    let show_offline_btn = spawn_ui_text(
        &mut commands, &font, "显示离线",
        545.0, 390.0, 12.0, Color::WHITE, 8.0,
    );
    commands.entity(show_offline_btn).insert((
        GuildShowOfflineBtn,
        UiButton {
            rect: (545.0, 390.0, 70.0, 20.0),
            clicked: false,
        },
        DialogRoot(DialogKind::Guild),
        GuildWidget,
    ));

    // #1362：职务改名（C# RanksSelectBox + RanksName + RanksSaveName @(298,420)）
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    let rank_dd = spawn_dropdown(
        &mut commands, &mut images, &font,
        vec!["会长".to_string(), "副会长".to_string(), "成员".to_string()],
        Some(0),
        298.0, 420.0, 64.0, 18.0,
        3, 8.0,
    );
    commands.entity(rank_dd).insert((GuildRankDrop, DialogRoot(DialogKind::Guild), GuildWidget));
    let rank_input = commands
        .spawn((
            crate::ui::sprite_ui::UiEntity,
            DialogRoot(DialogKind::Guild),
            GuildWidget,
            GuildRankRenameField,
            crate::game::dialogs::text_input::TextInputField(4),
            crate::game::dialogs::text_input::TextInputRect(370.0, 420.0, 120.0, 20.0),
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.2, 0.2, 0.25, 0.9),
                custom_size: Some(Vec2::new(120.0, 20.0)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(370.0, -420.0, 8.1),
            Visibility::Hidden,
        ))
        .id();
    commands.entity(rank_input).with_children(|p| {
        p.spawn((
            crate::game::dialogs::text_input::TextInputDisplay(4),
            Text2d::new(String::new()),
            bevy::sprite::Anchor::TOP_LEFT,
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::WHITE),
            Transform::from_xyz(4.0, -2.0, 8.2),
        ));
    });
    let rank_save = spawn_ui_text(
        &mut commands, &font, "改名",
        500.0, 420.0, 12.0, Color::WHITE, 8.0,
    );
    commands.entity(rank_save).insert((
        GuildRankSaveBtn,
        UiButton { rect: (500.0, 420.0, 40.0, 20.0), clicked: false },
        DialogRoot(DialogKind::Guild),
        GuildWidget,
    ));

    // 创建行会：输入框 + 按钮（原版 C# GuildDialog 创建流程）
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    let name_box = commands
        .spawn((
            crate::ui::sprite_ui::UiEntity,
            DialogRoot(DialogKind::Guild),
            GuildWidget,
            GuildNameField,
            crate::game::dialogs::text_input::TextInputField(0),
            crate::game::dialogs::text_input::TextInputRect(340.0, 330.0, 200.0, 20.0),
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.2, 0.2, 0.25, 0.9),
                custom_size: Some(Vec2::new(200.0, 20.0)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(340.0, -330.0, 8.1),
            Visibility::Hidden,
        ))
        .id();
    commands.entity(name_box).with_children(|p| {
        p.spawn((
            crate::game::dialogs::text_input::TextInputDisplay(0),
            Text2d::new(String::new()),
            bevy::sprite::Anchor::TOP_LEFT,
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 1.0, 1.0)),
            Transform::from_xyz(4.0, -2.0, 8.2),
        ));
    });
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        300.0, 360.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            GuildCreateBtn,
            DialogRoot(DialogKind::Guild),
            GuildWidget,
        ));
    }
    // 邀请玩家：输入框（TextInput id 1）+ 邀请按钮
    let inv_box = commands
        .spawn((
            crate::ui::sprite_ui::UiEntity,
            DialogRoot(DialogKind::Guild),
            GuildWidget,
            GuildInviteField,
            crate::game::dialogs::text_input::TextInputField(1),
            crate::game::dialogs::text_input::TextInputRect(340.0, 390.0, 200.0, 20.0),
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.2, 0.2, 0.25, 0.9),
                custom_size: Some(Vec2::new(200.0, 20.0)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(340.0, -390.0, 8.1),
            Visibility::Hidden,
        ))
        .id();
    commands.entity(inv_box).with_children(|p| {
        p.spawn((
            crate::game::dialogs::text_input::TextInputDisplay(1),
            Text2d::new(String::new()),
            bevy::sprite::Anchor::TOP_LEFT,
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 1.0, 1.0)),
            Transform::from_xyz(4.0, -2.0, 8.2),
        ));
    });
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        300.0, 420.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            GuildInviteBtn,
            DialogRoot(DialogKind::Guild),
            GuildWidget,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        390.0, 420.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            GuildKickBtn,
            DialogRoot(DialogKind::Guild),
            GuildWidget,
        ));
    }
    // 公告输入框（TextInput id 2）+ 设置按钮（C# GuildDialog 公告编辑）
    let notice_box = commands
        .spawn((
            crate::ui::sprite_ui::UiEntity,
            DialogRoot(DialogKind::Guild),
            GuildWidget,
            GuildNoticeField,
            crate::game::dialogs::text_input::TextInputField(2),
            crate::game::dialogs::text_input::TextInputRect(340.0, 460.0, 200.0, 20.0),
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.2, 0.2, 0.25, 0.9),
                custom_size: Some(Vec2::new(200.0, 20.0)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(340.0, -460.0, 8.1),
            Visibility::Hidden,
        ))
        .id();
    commands.entity(notice_box).with_children(|p| {
        p.spawn((
            crate::game::dialogs::text_input::TextInputDisplay(2),
            Text2d::new(String::new()),
            bevy::sprite::Anchor::TOP_LEFT,
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 1.0, 1.0)),
            Transform::from_xyz(4.0, -2.0, 8.2),
        ));
    });
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        300.0, 490.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            GuildNoticeBtn,
            DialogRoot(DialogKind::Guild),
            GuildWidget,
        ));
    }
    // 仓库金币：输入框（TextInput id 3）+ 存入/取出（C# GuildDialog 仓库语义）
    let gold_box = commands
        .spawn((
            crate::ui::sprite_ui::UiEntity,
            DialogRoot(DialogKind::Guild),
            GuildWidget,
            GuildGoldField,
            crate::game::dialogs::text_input::TextInputField(3),
            crate::game::dialogs::text_input::TextInputRect(340.0, 530.0, 200.0, 20.0),
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.2, 0.2, 0.25, 0.9),
                custom_size: Some(Vec2::new(200.0, 20.0)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(340.0, -530.0, 8.1),
            Visibility::Hidden,
        ))
        .id();
    commands.entity(gold_box).with_children(|p| {
        p.spawn((
            crate::game::dialogs::text_input::TextInputDisplay(3),
            Text2d::new(String::new()),
            bevy::sprite::Anchor::TOP_LEFT,
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 1.0, 1.0)),
            Transform::from_xyz(4.0, -2.0, 8.2),
        ));
    });
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        300.0, 560.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            GuildGoldDeposit,
            DialogRoot(DialogKind::Guild),
            GuildWidget,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        390.0, 560.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            GuildGoldWithdraw,
            DialogRoot(DialogKind::Guild),
            GuildWidget,
        ));
    }

    // 仓库物品（M32）：8 行列表 + 页签 + 存入/取出/翻页
    // 原版 C# GuildDialog.StorageGrid 8x14（这里分页显示 8 格/页）
    for i in 0..8usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            298.0, 595.0 + i as f32 * 18.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            GuildLine(11 + i),
            DialogRoot(DialogKind::Guild),
            GuildWidget,
        ));
    }
    let page = spawn_ui_text(
        &mut commands, &font, "",
        298.0, 745.0, 12.0, Color::srgb(1.0, 0.9, 0.5), 8.0,
    );
    commands.entity(page).insert((
        GuildLine(19),
        DialogRoot(DialogKind::Guild),
        GuildWidget,
    ));
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        300.0, 770.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            GuildItemDeposit,
            DialogRoot(DialogKind::Guild),
            GuildWidget,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        390.0, 770.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            GuildItemWithdraw,
            DialogRoot(DialogKind::Guild),
            GuildWidget,
        ));
    }
    // 翻页（原版 C# Prguse2 197/198/199 上、207/208/209 下）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 197, 198, 199,
        300.0, 802.0, 8.3, 16.0, 14.0,
    ) {
        commands.entity(e).insert((
            GuildStorageUp,
            DialogRoot(DialogKind::Guild),
            GuildWidget,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 207, 208, 209,
        320.0, 802.0, 8.3, 16.0, 14.0,
    ) {
        commands.entity(e).insert((
            GuildStorageDown,
            DialogRoot(DialogKind::Guild),
            GuildWidget,
        ));
    }

    // 邀请提示（MirMessageBox）
    let (bx, by) = (284.0, 289.0);
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 360) {
        let e = spawn_ui_sprite(&mut commands, h, bx, by, 9.5, 1.0);
        commands
            .entity(e)
            .insert((GuildInviteWidget, Visibility::Hidden));
    }
    let t = spawn_ui_text(
        &mut commands, &font, "", bx + 35.0, by + 40.0, 12.0, Color::WHITE, 9.6,
    );
    commands.entity(t).insert((GuildInviteText, GuildInviteWidget));
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        bx + 240.0, by + 150.0, 9.7, 76.0, 25.0,
    ) {
        commands.entity(e).insert((GuildInviteYes, GuildInviteWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        bx + 340.0, by + 150.0, 9.7, 76.0, 25.0,
    ) {
        commands.entity(e).insert((GuildInviteNo, GuildInviteWidget));
    }
}

/// 显隐 + 渲染 + 打开时请求行会信息 + 创建按钮
#[allow(clippy::too_many_arguments)]
fn guild_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut guild: ResMut<GuildState>,
    net: Res<NetConnection>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    create_btn: Query<&UiButton, With<GuildCreateBtn>>,
    invite_btn: Query<&UiButton, With<GuildInviteBtn>>,
    kick_btn: Query<&UiButton, With<GuildKickBtn>>,
    notice_btn: Query<&UiButton, With<GuildNoticeBtn>>,
    gold_deposit: Query<&UiButton, With<GuildGoldDeposit>>,
    gold_withdraw: Query<&UiButton, With<GuildGoldWithdraw>>,
    close: Query<&UiButton, With<GuildClose>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut widgets: Query<
        (
            &mut Visibility,
            Option<&GuildLine>,
            Option<&GuildNameField>,
            Option<&mut ScrollList>,
        ),
        (With<GuildWidget>, Without<GuildCreateBtn>),
    >,
    mut lines: Query<(&mut Text2d, &mut TextColor, &GuildLine)>,
    mut requested: Local<bool>,
) {
    let open = mgr.is_open(DialogKind::Guild);
    for (mut vis, _line, _field, sl) in &mut widgets {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
        if let Some(mut sl) = sl {
            // #89 成员列表行数（滚动夹紧）
            sl.set_total(if guild.in_guild { guild.visible_member_indices().len() } else { 0 });
        }
    }
    if !open {
        *requested = false;
        return;
    }
    // 打开瞬间请求行会信息（原版 C# GuildDialog.Show → RequestGuildInfo）
    if !*requested {
        *requested = true;
        net.send_packet(&mir2_shared::packets::client::guild::RequestGuildInfo {
            info_type: 0,
        });
        tracing::info!("🏰 请求行会信息");
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Guild);
        }
    }
    // 渲染（#89 成员列表支持滚轮滚动）
    let scroll_offset = widgets
        .iter()
        .find_map(|(_, _, _, sl)| sl.map(|s| s.offset))
        .unwrap_or(0);
    // #1348：可见成员下标（过滤离线）
    let visible = guild.visible_member_indices();
    for (mut text, mut color, line) in &mut lines {
        text.0 = match line.0 {
            0 => {
                if guild.in_guild {
                    let notice = guild.notice.first().cloned().unwrap_or_default();
                    if notice.is_empty() {
                        format!(
                            "{}（{}）金币:{}",
                            guild.name,
                            guild.leader,
                            guild.gold
                        )
                    } else {
                        format!(
                            "{}（{}）金币:{} 公告:{}",
                            guild.name,
                            guild.leader,
                            guild.gold,
                            notice
                        )
                    }
                } else {
                    "未加入行会".to_string()
                }
            }
            i if (1..=10).contains(&i) => {
                let idx = scroll_offset + i - 1;
                // #1348：按 show_offline 过滤后的可见成员映射
                match visible.get(idx).and_then(|&mi| guild.members.get(mi)) {
                Some(m) => {
                    // #1362：显示服务端职务名（C# 自定义职务名简化）
                    let rank = guild
                        .rank_names
                        .get(m.rank as usize)
                        .cloned()
                        .unwrap_or_else(|| "成员".to_string());
                    format!(
                        "{}{} ({})",
                        m.name,
                        if m.online { "" } else { "（离线）" },
                        rank
                    )
                }
                None => String::new(),
            }
            },
            i if (11..=18).contains(&i) => {
                let slot = guild.storage_page * 8 + (i - 11);
                match guild.storage_items.get(slot).and_then(|s| s.as_ref()) {
                    Some(it) => format!(
                        "{:02}: {} x{}",
                        slot + 1,
                        guild.item_name(it.item_index),
                        it.count
                    ),
                    None => format!("{:02}: 空", slot + 1),
                }
            }
            19 => format!("仓库 第{}/13页", guild.storage_page + 1),
            _ => String::new(),
        };
        // #140 成员选中行高亮（踢出目标可见）
        let selected = matches!(line.0, 1..=10)
            && guild.selected_member == Some(scroll_offset + line.0 - 1);
        let c = if selected {
            Color::srgb(1.0, 0.9, 0.3)
        } else {
            Color::WHITE
        };
        if color.0 != c {
            color.0 = c;
        }
    }
    // 创建按钮 → GuildNameReturn（原版 C#：输入行会名 → 创建）
    for btn in &create_btn {
        if btn.clicked {
            let name = input.texts.get(0).cloned().unwrap_or_default();
            let name = name.trim().to_string();
            if !name.is_empty() {
                net.send_packet(&mir2_shared::packets::client::guild::GuildNameReturn {
                    name: name.clone(),
                });
                tracing::info!("🏰 创建行会: {}", name);
                input.texts[0].clear();
                input.active = None;
            }
        }
    }
    // 邀请按钮 → EditGuildMember{0=add member}（C# GuildDialog 邀请）
    for btn in &invite_btn {
        if btn.clicked {
            let name = input.texts.get(1).cloned().unwrap_or_default();
            let name = name.trim().to_string();
            if !name.is_empty() && guild.in_guild {
                net.send_packet(&mir2_shared::packets::client::guild::EditGuildMember {
                    change_type: 0,
                    rank_index: 0,
                    name: name.clone(),
                    rank_name: String::new(),
                });
                tracing::info!("🏰 邀请玩家加入行会: {}", name);
                input.texts[1].clear();
                input.active = None;
            }
        }
    }
    // 踢出按钮 → EditGuildMember{1=delete member}（对选中的成员）
    for btn in &kick_btn {
        if btn.clicked {
            if let Some(idx) = guild.selected_member {
                let visible = guild.visible_member_indices();
                if let Some(&mi) = visible.get(idx) {
                    if let Some(m) = guild.members.get(mi) {
                        net.send_packet(&mir2_shared::packets::client::guild::EditGuildMember {
                            change_type: 1,
                            rank_index: 0,
                            name: m.name.clone(),
                            rank_name: String::new(),
                        });
                        tracing::info!("🏰 踢出行会成员: {}", m.name);
                        guild.selected_member = None;
                    }
                }
            }
        }
    }
    // 公告按钮 → EditGuildNotice（C# GuildDialog 公告编辑）
    for btn in &notice_btn {
        if btn.clicked {
            let notice = input.texts.get(2).cloned().unwrap_or_default();
            let notice = notice.trim().to_string();
            if !notice.is_empty() && guild.in_guild {
                net.send_packet(&mir2_shared::packets::client::guild::EditGuildNotice {
                    notice_lines: vec![notice.clone()],
                });
                tracing::info!("🏰 更新行会公告: {}", notice);
                input.texts[2].clear();
                input.active = None;
            }
        }
    }
    // 仓库金币：存入/取出（C# GuildDialog 仓库语义：GuildStorageGoldChange）
    for btn in &gold_deposit {
        if btn.clicked && guild.in_guild {
            let amount = input.texts.get(3).cloned().unwrap_or_default().trim().parse::<u32>().unwrap_or(0);
            if amount > 0 {
                net.send_packet(&mir2_shared::packets::client::guild::GuildStorageGoldChange {
                    change_type: 0,
                    amount,
                });
                tracing::info!("🏰 存入行会仓库 {} 金币", amount);
                input.texts[3].clear();
                input.active = None;
            }
        }
    }
    for btn in &gold_withdraw {
        if btn.clicked && guild.in_guild {
            let amount = input.texts.get(3).cloned().unwrap_or_default().trim().parse::<u32>().unwrap_or(0);
            if amount > 0 {
                net.send_packet(&mir2_shared::packets::client::guild::GuildStorageGoldChange {
                    change_type: 1,
                    amount,
                });
                tracing::info!("🏰 取出行会仓库 {} 金币", amount);
                input.texts[3].clear();
                input.active = None;
            }
        }
    }
    // 点击成员行选中（踢出目标）
    if mouse.just_pressed(MouseButton::Left) {
        if let Ok(window) = windows.single() {
            if let Some(cursor) = window.cursor_position() {
                let visible = guild.visible_member_indices();
                for i in 1..=10usize {
                    let y = 140.0 + (i - 1) as f32 * 20.0;
                    if cursor.x >= 298.0 && cursor.x <= 600.0 && cursor.y >= y && cursor.y <= y + 18.0 {
                        let idx = scroll_offset + i - 1;
                        if let Some(&mi) = visible.get(idx) {
                            guild.selected_member = Some(idx);
                            tracing::info!("🏰 选中行会成员: {}", guild.members[mi].name);
                        }
                        break;
                    }
                }
                // 仓库格子点击选中（取出目标，原版 C# StorageGrid 点击语义）
                for i in 11..=18usize {
                    let y = 595.0 + (i - 11) as f32 * 18.0;
                    if cursor.x >= 298.0 && cursor.x <= 600.0 && cursor.y >= y && cursor.y <= y + 16.0 {
                        let slot = guild.storage_page * 8 + (i - 11);
                        if slot < guild.storage_items.len() {
                            guild.selected_storage = Some(slot);
                            tracing::info!("🏰 选中仓库格子 {}", slot);
                        }
                        break;
                    }
                }
            }
        }
    }
}

/// #1362：职务改名（C# RanksSelectBox + RanksName + RanksSaveName → EditGuildMember ChangeType=6）
fn guild_rank_rename_system(
    guild: Res<GuildState>,
    net: Res<NetConnection>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    rank_dd: Query<(&DropDown, &GuildRankDrop)>,
    save_btn: Query<&UiButton, With<GuildRankSaveBtn>>,
) {
    let idx = rank_dd
        .single()
        .map(|(dd, _)| dd.selected.unwrap_or(0))
        .unwrap_or(0);
    for btn in &save_btn {
        if btn.clicked && guild.in_guild {
            let name = input.texts.get(4).cloned().unwrap_or_default();
            let name = name.trim().to_string();
            if !name.is_empty() {
                net.send_packet(&mir2_shared::packets::client::guild::EditGuildMember {
                    change_type: 6,
                    rank_index: idx as u8,
                    name: String::new(),
                    rank_name: name.clone(),
                });
                tracing::info!("🏰 职务改名: {} -> {}", idx, name);
                if input.texts.len() > 4 {
                    input.texts[4].clear();
                }
                input.active = None;
            }
        }
    }
}

/// #1348：显示离线成员切换（C# MembersShowOfflineButton/Status，纯本地过滤）
fn guild_show_offline_system(
    mut guild: ResMut<GuildState>,
    mut btn: Query<(&UiButton, &mut Text2d), With<GuildShowOfflineBtn>>,
) {
    for (b, mut text) in btn.iter_mut() {
        text.0 = if guild.show_offline { "✓显示离线".to_string() } else { "显示离线".to_string() };
        if b.clicked {
            guild.show_offline = !guild.show_offline;
            if !guild.show_offline {
                guild.selected_member = None;
            }
        }
    }
}

/// 仓库物品交互（M32）：打开时请求列表 + 存入/取出/翻页
/// 原版 C# GuildDialog：StorageGrid 点击选中 → 拖拽/按钮存入取出；列表由
/// S.GuildStorageList 推送（C# GuildStorageItemChange type=3 请求）
#[allow(clippy::too_many_arguments)]
fn guild_storage_system(
    mut mgr: ResMut<DialogManager>,
    mut guild: ResMut<GuildState>,
    net: Res<NetConnection>,
    hud: Res<crate::game::hud::HudState>,
    inv_click: Res<crate::game::dialogs::inventory::InvClickState>,
    deposit_btn: Query<&UiButton, With<GuildItemDeposit>>,
    withdraw_btn: Query<&UiButton, With<GuildItemWithdraw>>,
    up_btn: Query<&UiButton, With<GuildStorageUp>>,
    down_btn: Query<&UiButton, With<GuildStorageDown>>,
    mut requested: Local<bool>,
) {
    let open = mgr.is_open(DialogKind::Guild);
    if !open {
        *requested = false;
        return;
    }
    // 打开瞬间请求仓库物品列表（原版 C# GuildStorageItemChange type=3 语义）
    if !*requested {
        *requested = true;
        net.send_packet(&crate::network::GuildStorageItemChangeWire {
            change_type: 3,
            grid: 0,
            unique_id: 0,
            count: 0,
        });
        tracing::info!("🏰 请求仓库物品列表");
    }
    for btn in &up_btn {
        if btn.clicked {
            guild.storage_page = guild.storage_page.saturating_sub(1);
        }
    }
    for btn in &down_btn {
        if btn.clicked && guild.storage_page + 1 < 13 {
            guild.storage_page += 1;
        }
    }
    for btn in &deposit_btn {
        if btn.clicked && guild.in_guild {
            // 选中背包物品 → 存入（原版 C#：选中物品 → GuildStorageItemChange type=0）
            let idx = inv_click
                .selected
                .filter(|i| hud.inventory.items.get(*i).and_then(|s| s.as_ref()).is_some())
                .or_else(|| hud.inventory.items.iter().position(|s| s.is_some()));
            if let Some(i) = idx {
                if let Some(item) = hud.inventory.items.get(i).and_then(|s| s.as_ref()) {
                    net.send_packet(&crate::network::GuildStorageItemChangeWire {
                        change_type: 0,
                        grid: 0,
                        unique_id: item.unique_id,
                        count: item.count as u32,
                    });
                    tracing::info!(
                        "🏰 存入背包物品 [{}] uid={} x{}",
                        item.name,
                        item.unique_id,
                        item.count
                    );
                }
            } else {
                tracing::warn!("🏰 背包没有可存入的物品");
            }
        }
    }
    for btn in &withdraw_btn {
        if btn.clicked && guild.in_guild {
            if let Some(slot) = guild.selected_storage {
                if slot < guild.storage_items.len() && guild.storage_items[slot].is_some() {
                    net.send_packet(&crate::network::GuildStorageItemChangeWire {
                        change_type: 1,
                        grid: slot as u8,
                        unique_id: 0,
                        count: 0,
                    });
                    tracing::info!("🏰 取出仓库格子 {}", slot);
                }
            } else {
                tracing::warn!("🏰 请先点击选中一个仓库格子");
            }
        }
    }
}

/// 行会邀请提示：Yes/No → C.GuildInvite{accept}
fn guild_invite_system(
    mut guild: ResMut<GuildState>,
    net: Res<NetConnection>,
    yes: Query<&UiButton, With<GuildInviteYes>>,
    no: Query<&UiButton, With<GuildInviteNo>>,
    mut widgets: Query<
        &mut Visibility,
        (With<GuildInviteWidget>, Without<GuildWidget>),
    >,
    mut texts: Query<(&mut Text2d, &GuildInviteText)>,
) {
    let has_invite = guild.invite.is_some();
    for mut vis in &mut widgets {
        *vis = if has_invite {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (mut text, _) in &mut texts {
        text.0 = match guild.invite.as_ref() {
            Some(name) => format!("{} 邀请你加入行会", name),
            None => String::new(),
        };
    }
    if guild.invite.is_none() {
        return;
    }
    let mut accept: Option<bool> = None;
    for btn in &yes {
        if btn.clicked {
            accept = Some(true);
        }
    }
    for btn in &no {
        if btn.clicked {
            accept = Some(false);
        }
    }
    if let Some(a) = accept {
        net.send_packet(&mir2_shared::packets::client::guild::GuildInvite {
            accept_invite: a,
        });
        tracing::info!("🏰 行会邀请回复: accept={}", a);
        guild.invite = None;
    }
}


/// 消费服务端行会事件（网络层只广播 ServerEvent）
fn guild_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut guild: ResMut<GuildState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::GuildInGuild { in_guild } => {
                guild.in_guild = *in_guild;
                if !guild.in_guild {
                    guild.name.clear();
                    guild.leader.clear();
                    guild.members.clear();
                    guild.notice.clear();
                    guild.gold = 0;
                    guild.storage_items.clear();
                    guild.storage_received = false;
                }
            }
            ServerEvent::GuildData { name, leader, rank_names, notice, members, gold } => {
                guild.in_guild = true;
                guild.name = name.clone();
                guild.leader = leader.clone();
                guild.rank_names = rank_names.clone();
                guild.notice = notice.clone();
                guild.members = members.clone();
                guild.gold = *gold;
            }
            ServerEvent::GuildStorageGoldChanged {
                amount,
                change_type,
                name,
            } => {
                // #295：行会仓库金币实时同步（C# GuildDialog.Gold +/-）
                if *change_type == 0 {
                    guild.gold = guild.gold.saturating_add(*amount);
                } else {
                    guild.gold = guild.gold.saturating_sub(*amount);
                }
                tracing::info!(
                    "💰 行会仓库金币 {} {}（by {}）",
                    if *change_type == 0 { "存入" } else { "取出" },
                    amount,
                    name
                );
            }
            ServerEvent::GuildStorageItemChanged {
                change_type,
                to,
                from,
                item,
            } => {
                // #295：行会仓库物品实时同步（C# 0=存入 1=取出 2=移动）
                match *change_type {
                    0 => {
                        if let Some(item) = item {
                            if *to >= 0 {
                                // 未收到全量列表时先扩容（C# StorageGrid 固定 100 格）
                                let need = (*to as usize).saturating_add(1);
                                if guild.storage_items.len() < need {
                                    guild.storage_items.resize(need, None);
                                }
                                guild.storage_items[*to as usize] = Some(StorageItem {
                                    unique_id: item.unique_id,
                                    item_index: item.item_index,
                                    name: item.name.clone(),
                                    count: item.count,
                                });
                            }
                        }
                    }
                    1 => {
                        if *from >= 0 && (*from as usize) < guild.storage_items.len() {
                            guild.storage_items[*from as usize] = None;
                        }
                    }
                    2 => {
                        if *from >= 0
                            && *to >= 0
                            && (*from as usize) < guild.storage_items.len()
                            && (*to as usize) < guild.storage_items.len()
                        {
                            let moved = guild.storage_items[*from as usize].take();
                            if let Some(item) = item {
                                guild.storage_items[*to as usize] = Some(StorageItem {
                                    unique_id: item.unique_id,
                                    item_index: item.item_index,
                                    name: item.name.clone(),
                                    count: item.count,
                                });
                            } else {
                                guild.storage_items[*to as usize] = moved;
                            }
                        }
                    }
                    _ => {}
                }
                tracing::info!("📦 行会仓库物品变化 type={} to={} from={}", change_type, to, from);
            }
            ServerEvent::GuildStorage { items } => {
                guild.storage_items = items
                    .iter()
                    .map(|(unique_id, item_index, count, info_name)| {
                        let name = if !info_name.is_empty() {
                            info_name.clone()
                        } else {
                            guild
                                .item_names
                                .get(item_index)
                                .cloned()
                                .unwrap_or_default()
                        };
                        Some(StorageItem {
                            unique_id: *unique_id,
                            item_index: *item_index,
                            name,
                            count: *count,
                        })
                    })
                    .collect();
                guild.storage_received = true;
            }
            ServerEvent::GuildNotice { notice } => {
                guild.notice = notice.clone();
            }
            ServerEvent::GuildMemberChanged { name, rank, online, joined, removed } => {
                if *removed {
                    guild.members.retain(|m| m.name != *name);
                } else if *joined {
                    if !guild.members.iter().any(|m| m.name == *name) {
                        guild.members.push(GuildMember {
                            name: name.clone(),
                            rank: *rank,
                            online: *online,
                        });
                    }
                } else if let Some(m) = guild.members.iter_mut().find(|m| m.name == *name) {
                    m.rank = *rank;
                    m.online = *online;
                }
            }
            ServerEvent::GuildInvited { name } => {
                guild.invite = Some(name.clone());
            }
            ServerEvent::UserInformation { item_names, .. } => {
                for (idx, name) in item_names {
                    guild.item_names.insert(*idx, name.clone());
                }
            }
            _ => {}
        }
    }
}
