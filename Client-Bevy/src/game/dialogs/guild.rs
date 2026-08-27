// ============================================================================
// 行会对话框（M27 → 批 47 bevy_ui 迁移）
// 布局参考：C# GuildDialog.cs / macroquad guild_dialog.rs
//   - 背景 Prguse[180]（实测 590x432），标题 Title[15]，位置 (280,80)
//   - 行会名/会长/金币、成员列表（职务+在线）、公告、创建输入框
// 网络：GuildStatus（1 字节 in_guild / 完整信息，同 opcode 双格式）、GuildNoticeChange、GuildMemberChange
// 迁移说明：
//   - 原版 C# GuildDialog 是分页窗口（Member/Buff/Rank/Storage 各一页，590x432）。
//     本移植为单窗垂直堆叠：根容器 590x740 @ (280,80)，背景图以自然尺寸作子图，
//     下方 432..740 为深色延伸区容纳职务/仓库/金币区块（原 sprite 版这些区块
//     溢出面板裸奔，正是"UI 堆屏幕"病灶之一）。
//   - 邀请提示 = 独立覆盖层 Prguse[360]（456x190）@ (284,289)。
// ============================================================================

use bevy::prelude::*;
use std::collections::HashMap;

use crate::game::dialogs::text_input::{
    TextInputDisplay, TextInputField, TextInputRect, TextInputState,
};
use crate::game::dialogs::{AlwaysVisible, DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_dropdown_ui, spawn_icon_button, spawn_image,
    spawn_label, spawn_panel, spawn_scroll_bar_ui, UiDropDown, UiScrollList,
};

/// 根容器尺寸（容纳堆叠的成员/职务/仓库/金币区块；背景图保持自然尺寸）
pub const GUILD_X: f32 = 280.0;
pub const GUILD_Y: f32 = 80.0;
pub const GUILD_W: f32 = 590.0;
pub const GUILD_H: f32 = 740.0;
/// 背景图 Prguse[180] 自然尺寸
pub const BG_W: f32 = 590.0;
pub const BG_H: f32 = 432.0;

/// 行会成员
#[derive(Debug, Clone, Default)]
pub struct GuildMember {
    pub name: String,
    pub rank: u8,
    /// #1395：职务定义索引（C# rank_index）
    pub rank_index: u8,
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
    /// #1395：职务定义（name, options；服务端 GuildStatus 下发，C# GuildObject.Ranks）
    pub rank_defs: Vec<(String, u8)>,
    /// #2537：Buff 定义目录（S.GuildBuffList 第三段，服务端 ini GuildSettings 全量）
    pub buff_catalog: Vec<mir2_shared::data::client_data::GuildBuffInfo>,
    /// #2537：已激活 Buff id（S.GuildBuffList ActiveBuffs）
    pub active_buffs: Vec<i32>,
    /// #2537：Buff 页显示开关（C# BuffButton/BuffPage 切换）
    pub show_buff_page: bool,
    /// #2537：Buff 页滚动起点（C# StartIndex，8 行/页）
    pub buff_start: usize,
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

    /// #2537 Buff 是否已激活
    pub fn buff_active(&self, buff_id: i32) -> bool {
        self.active_buffs.contains(&buff_id)
    }
}

/// #2537 Buff 行文本（C# GuildBuffButton：名称 + 等级/点数/费用 + 状态；纯函数便于头测）
pub fn buff_row_text(info: &mir2_shared::data::client_data::GuildBuffInfo, active: bool) -> String {
    format!(
        "{}  Lv{} 点{} 金{}{}",
        info.name,
        info.level_requirement,
        info.points_requirement,
        info.activation_cost,
        if active { " [已激活]" } else { "" }
    )
}

/// #2537 Buff 页数（C# 8 个 GuildBuffButton/页；空目录仍算 1 页，C# Count<8 不翻页）
pub fn buff_page_count(catalog_len: usize) -> usize {
    catalog_len.div_ceil(8).max(1)
}

#[derive(Component)]
pub struct GuildWidget;


/// 创建行会输入框（TextInputState id 0）
#[derive(Component)]
pub struct GuildNameField;

#[derive(Component)]
pub struct GuildCreateBtn;

/// 邀请玩家输入框（TextInput id 1）
#[derive(Component)]
pub struct GuildInviteField;


/// #1362：职务改名下拉（C# RanksSelectBox）
#[derive(Component)]
pub struct GuildRankDrop;
/// #1362：职务改名输入框（TextInput id 4）
#[derive(Component)]
pub struct GuildRankRenameField;
/// #1362：职务改名保存按钮（C# RanksSaveName）
#[derive(Component)]
pub struct GuildRankSaveBtn;
/// #1395 子批2：加职务输入框（TextInput id 7）/ 按钮（C# AddRank）
#[derive(Component)]
pub struct GuildAddRankField;
#[derive(Component)]
pub struct GuildAddRankBtn;
/// #1395 子批2：职务权限位按钮（C# RanksOptionsButtons[8]，bit 0..7）
#[derive(Component)]
pub struct GuildRankPermBtn(u8);
#[derive(Component)]
pub struct GuildRankPermText;
/// #1395 子批2：调职按钮（C# 升职 ChangeType=2，把选中成员调到下拉职务）
#[derive(Component)]
pub struct GuildPromoteBtn;

/// 公告输入框（TextInput id 2）
#[derive(Component)]
pub struct GuildNoticeField;

/// 仓库金币输入框（TextInput id 3）
#[derive(Component)]
pub struct GuildGoldField;

/// #1348：显示离线成员切换（C# MembersShowOfflineButton）
#[derive(Component)]
pub struct GuildShowOfflineBtn;

/// #2537：Buff 页开关（C# BuffButton）
#[derive(Component)]
pub struct GuildBuffToggleBtn;

/// #2537：Buff 页翻页（C# UpButton/DownButton，8 行/页）
#[derive(Component)]
pub struct GuildBuffUp;

#[derive(Component)]
pub struct GuildBuffDown;




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

/// 行会窗口主按钮（单查询分发，避免多 With<marker> 查询超 SystemParam 上限）
#[derive(Component, Clone, Copy)]
pub enum GuildBtnKind {
    Close,
    Invite,
    Kick,
    Notice,
    GoldDeposit,
    GoldWithdraw,
}
#[derive(Component)]
pub struct GuildBtn(pub GuildBtnKind);

/// #1348：显示离线按钮文本子节点
#[derive(Component)]
pub struct GuildShowOfflineText;

pub struct GuildPlugin;

impl Plugin for GuildPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GuildState>();
        app.add_systems(OnEnter(AppState::Game), spawn_guild);
        app.add_systems(OnExit(AppState::Game), cleanup_guild);
        app.add_systems(Update, guild_server_events.run_if(in_state(AppState::Game)));
        app.add_systems(
            Update,
            (
                guild_ui_system,
                guild_buff_system,
                guild_storage_system,
                guild_invite_system,
                guild_show_offline_system,
                guild_rank_rename_system,
                guild_rank_manage_system,
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
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 根容器（bevy_ui Node + Overflow::clip）：590x740 @ (280,80)
    // 背景 Prguse[180]（实测 590x432）以自然尺寸作子图，下方 432..740 为深色延伸区，
    // 容纳移植版堆叠的职务/仓库/金币区块（原 sprite 版这些区块溢出面板裸奔）。
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(GUILD_X),
                top: Val::Px(GUILD_Y),
                width: Val::Px(GUILD_W),
                height: Val::Px(GUILD_H),
                overflow: Overflow::clip(),
                ..default()
            },
            DialogRoot(DialogKind::Guild),
            GuildWidget,
            GlobalZIndex(30),
            Visibility::Hidden,
            // #89 可滚动成员列表：10 行 × 20px（滚动条/滚轮区域相对根容器）
            UiScrollList {
                rect_rel: (18.0, 60.0, 200.0, 200.0),
                row_h: 20.0,
                visible: 10,
                total: 0,
                offset: 0,
                step: 3,
                track_rel: (218.0, 60.0, 4.0, 200.0),
                thumb: None,
                z: 8,
            },
        ))
        .id();

    commands.entity(root).with_children(|p| {
        // 背景图（自然尺寸）+ 下方深色延伸区
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 180) {
            spawn_image(p, h, 0.0, 0.0, BG_W, BG_H, 0);
        }
        spawn_container(p, 0.0, BG_H, GUILD_W, GUILD_H - BG_H, 0)
            .insert(BackgroundColor(crate::ui::theme::colors::PANEL_BG));
        // 标题 Title[15] @(18,8)
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 15) {
            spawn_image(p, h, 18.0, 8.0, 103.0, 17.0, 1);
        }
        // 关闭 Prguse2[360-362] @(340,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 340.0, 3.0, 20.0, 20.0, 9)
                .insert(GuildBtn(GuildBtnKind::Close));
        }
        // 滚动条（轨道 + 滑块）
        spawn_scroll_bar_ui(p, (218.0, 60.0, 4.0, 200.0), 8);
        // 行会名/会长文本（GuildLine 0 占位显示头部）@(18,40)
        spawn_label(p, &font, "", 18.0, 40.0, 12.0, Color::srgb(1.0, 0.9, 0.5), 8)
            .insert(GuildLine(0));
        // 成员列表（10 行，1..=10）@(18,60+20i)
        for i in 1..=10usize {
            spawn_label(p, &font, "", 18.0, 60.0 + (i - 1) as f32 * 20.0, 12.0, Color::WHITE, 8)
                .insert(GuildLine(i));
        }
        // #1348：显示离线成员切换（C# MembersShowOfflineButton）@(265,310) 70x20
        spawn_container(p, 265.0, 310.0, 70.0, 20.0, 8)
            .insert((Button, GuildShowOfflineBtn))
            .with_children(|b| {
                spawn_label(b, &font, "显示离线", 0.0, 0.0, 12.0, Color::WHITE, 1)
                    .insert(GuildShowOfflineText);
            });
        // #2537：Buff 页开关（C# BuffButton）+ 翻页（C# UpButton/DownButton）
        spawn_container(p, 190.0, 310.0, 60.0, 20.0, 8)
            .insert((Button, GuildBuffToggleBtn))
            .with_children(|b| {
                spawn_label(b, &font, "技能", 0.0, 0.0, 12.0, Color::WHITE, 1);
            });
        spawn_container(p, 225.0, 60.0, 16.0, 14.0, 8)
            .insert((Button, GuildBuffUp))
            .with_children(|b| {
                spawn_label(b, &font, "▲", 0.0, 0.0, 11.0, Color::WHITE, 1);
            });
        spawn_container(p, 225.0, 244.0, 16.0, 14.0, 8)
            .insert((Button, GuildBuffDown))
            .with_children(|b| {
                spawn_label(b, &font, "▼", 0.0, 0.0, 11.0, Color::WHITE, 1);
            });

        // #1362：职务改名（C# RanksSelectBox + RanksName + RanksSaveName @(18,340)）
        spawn_dropdown_ui(
            p,
            &font,
            vec!["会长".to_string(), "副会长".to_string(), "成员".to_string()],
            Some(0),
            (GUILD_X, GUILD_Y),
            18.0,
            340.0,
            64.0,
            18.0,
            3,
            8,
        )
        .insert(GuildRankDrop);
        spawn_container(p, 90.0, 340.0, 120.0, 20.0, 8)
            .insert((
                BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                GuildRankRenameField,
                TextInputField(4),
                TextInputRect(370.0, 420.0, 120.0, 20.0),
                Visibility::Hidden,
            ))
            .with_children(|ic| {
                ic.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(4.0),
                        top: Val::Px(2.0),
                        ..default()
                    },
                    Text::new(String::new()),
                    TextFont {
                        font: FontSource::Handle(font.clone()),
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    ZIndex(9),
                    TextInputDisplay(4),
                ));
            });
        spawn_container(p, 220.0, 340.0, 40.0, 20.0, 8)
            .insert((Button, GuildRankSaveBtn))
            .with_children(|b| {
                spawn_label(b, &font, "改名", 0.0, 0.0, 12.0, Color::WHITE, 1);
            });

        // #1395 子批2：加职务（TextInput id 7 @(60,368)）+ 按钮
        spawn_label(p, &font, "加职务", 18.0, 368.0, 11.0, Color::WHITE, 8);
        spawn_container(p, 60.0, 368.0, 100.0, 20.0, 8)
            .insert((
                BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                GuildAddRankField,
                TextInputField(7),
                TextInputRect(340.0, 448.0, 100.0, 20.0),
                Visibility::Hidden,
            ))
            .with_children(|ic| {
                ic.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(4.0),
                        top: Val::Px(2.0),
                        ..default()
                    },
                    Text::new(String::new()),
                    TextFont {
                        font: FontSource::Handle(font.clone()),
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    ZIndex(9),
                    TextInputDisplay(7),
                ));
            });
        spawn_container(p, 170.0, 368.0, 36.0, 18.0, 8)
            .insert((Button, GuildAddRankBtn))
            .with_children(|b| {
                spawn_label(b, &font, "添加", 0.0, 0.0, 11.0, Color::WHITE, 1);
            });
        // #1395 子批2：权限位（C# RanksOptionsButtons[8]：改/招/踢/存/取/盟/告/益）
        spawn_label(p, &font, "权限", 18.0, 392.0, 11.0, Color::WHITE, 8);
        for (i, label) in ["改", "招", "踢", "存", "取", "盟", "告", "益"].iter().enumerate() {
            spawn_container(p, 50.0 + i as f32 * 30.0, 392.0, 24.0, 16.0, 8)
                .insert((Button, GuildRankPermBtn(i as u8)))
                .with_children(|b| {
                    spawn_label(b, &font, label, 0.0, 0.0, 11.0, Color::WHITE, 1);
                });
        }
        spawn_label(p, &font, "权限:00000000", 18.0, 412.0, 10.0, Color::srgb(0.8, 0.9, 0.6), 8)
            .insert(GuildRankPermText);
        spawn_container(p, 160.0, 412.0, 100.0, 18.0, 8)
            .insert((Button, GuildPromoteBtn))
            .with_children(|b| {
                spawn_label(b, &font, "调职到下拉职务", 0.0, 0.0, 11.0, Color::WHITE, 1);
            });

        // 创建行会：输入框（TextInput id 0）+ 创建按钮（原版 C# GuildDialog 创建流程）
        spawn_container(p, 60.0, 250.0, 200.0, 20.0, 8)
            .insert((
                BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                GuildNameField,
                TextInputField(0),
                TextInputRect(340.0, 330.0, 200.0, 20.0),
                Visibility::Hidden,
            ))
            .with_children(|ic| {
                ic.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(4.0),
                        top: Val::Px(2.0),
                        ..default()
                    },
                    Text::new(String::new()),
                    TextFont {
                        font: FontSource::Handle(font.clone()),
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 1.0, 1.0)),
                    ZIndex(9),
                    TextInputDisplay(0),
                ));
            });
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
        ) {
            spawn_icon_button(p, n, h, pr, 20.0, 280.0, 76.0, 25.0, 9).insert(GuildCreateBtn);
        }
        // 邀请玩家：输入框（TextInput id 1）+ 邀请按钮
        spawn_container(p, 60.0, 310.0, 200.0, 20.0, 8)
            .insert((
                BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                GuildInviteField,
                TextInputField(1),
                TextInputRect(340.0, 390.0, 200.0, 20.0),
                Visibility::Hidden,
            ))
            .with_children(|ic| {
                ic.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(4.0),
                        top: Val::Px(2.0),
                        ..default()
                    },
                    Text::new(String::new()),
                    TextFont {
                        font: FontSource::Handle(font.clone()),
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 1.0, 1.0)),
                    ZIndex(9),
                    TextInputDisplay(1),
                ));
            });
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
        ) {
            spawn_icon_button(p, n, h, pr, 20.0, 340.0, 76.0, 25.0, 9)
                .insert(GuildBtn(GuildBtnKind::Invite));
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 210),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 211),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 212),
        ) {
            spawn_icon_button(p, n, h, pr, 110.0, 340.0, 76.0, 25.0, 9)
                .insert(GuildBtn(GuildBtnKind::Kick));
        }
        // 公告输入框（TextInput id 2）+ 设置按钮
        spawn_container(p, 60.0, 380.0, 200.0, 20.0, 8)
            .insert((
                BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                GuildNoticeField,
                TextInputField(2),
                TextInputRect(340.0, 460.0, 200.0, 20.0),
                Visibility::Hidden,
            ))
            .with_children(|ic| {
                ic.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(4.0),
                        top: Val::Px(2.0),
                        ..default()
                    },
                    Text::new(String::new()),
                    TextFont {
                        font: FontSource::Handle(font.clone()),
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 1.0, 1.0)),
                    ZIndex(9),
                    TextInputDisplay(2),
                ));
            });
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
        ) {
            spawn_icon_button(p, n, h, pr, 20.0, 410.0, 76.0, 25.0, 9)
                .insert(GuildBtn(GuildBtnKind::Notice));
        }
        // 仓库金币：输入框（TextInput id 3）+ 存入/取出
        spawn_container(p, 60.0, 450.0, 200.0, 20.0, 8)
            .insert((
                BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                GuildGoldField,
                TextInputField(3),
                TextInputRect(340.0, 530.0, 200.0, 20.0),
                Visibility::Hidden,
            ))
            .with_children(|ic| {
                ic.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(4.0),
                        top: Val::Px(2.0),
                        ..default()
                    },
                    Text::new(String::new()),
                    TextFont {
                        font: FontSource::Handle(font.clone()),
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 1.0, 1.0)),
                    ZIndex(9),
                    TextInputDisplay(3),
                ));
            });
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
        ) {
            spawn_icon_button(p, n, h, pr, 20.0, 480.0, 76.0, 25.0, 9)
                .insert(GuildBtn(GuildBtnKind::GoldDeposit));
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 210),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 211),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 212),
        ) {
            spawn_icon_button(p, n, h, pr, 110.0, 480.0, 76.0, 25.0, 9)
                .insert(GuildBtn(GuildBtnKind::GoldWithdraw));
        }

        // 仓库物品（M32）：8 行列表 + 页签 + 存入/取出/翻页 @(18,515+18i)
        for i in 0..8usize {
            spawn_label(p, &font, "", 18.0, 515.0 + i as f32 * 18.0, 12.0, Color::WHITE, 8)
                .insert(GuildLine(11 + i));
        }
        spawn_label(p, &font, "", 18.0, 665.0, 12.0, Color::srgb(1.0, 0.9, 0.5), 8)
            .insert(GuildLine(19));
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
        ) {
            spawn_icon_button(p, n, h, pr, 20.0, 690.0, 76.0, 25.0, 9).insert(GuildItemDeposit);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 210),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 211),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 212),
        ) {
            spawn_icon_button(p, n, h, pr, 110.0, 690.0, 76.0, 25.0, 9).insert(GuildItemWithdraw);
        }
        // 翻页（原版 C# Prguse2 197/198/199 上、207/208/209 下）@(20/40,722)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 197),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 198),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 199),
        ) {
            spawn_icon_button(p, n, h, pr, 20.0, 722.0, 16.0, 14.0, 9).insert(GuildStorageUp);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 208),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 209),
        ) {
            spawn_icon_button(p, n, h, pr, 40.0, 722.0, 16.0, 14.0, 9).insert(GuildStorageDown);
        }
    });

    // 邀请提示（MirMessageBox，独立覆盖层 Prguse[360] 456x190 @ (284,289)）
    let (bx, by) = (284.0, 289.0);
    if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 360) {
        let popup = spawn_panel(&mut commands, h, bx, by, 456.0, 190.0, 45);
        commands
            .entity(popup)
            .insert((
                DialogRoot(DialogKind::Guild),
                // 独立弹窗不随 Guild 开关门控；挂 DialogRoot 仅为 OnExit 时随行会窗口一起清理
                // （否则重进 Game 会重复生成弹窗）
                AlwaysVisible,
                GuildInviteWidget,
                Visibility::Hidden,
            ));
        commands.entity(popup).with_children(|p| {
            spawn_label(p, &font, "", 35.0, 40.0, 12.0, Color::WHITE, 9).insert(GuildInviteText);
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
            ) {
                spawn_icon_button(p, n, h, pr, 240.0, 150.0, 76.0, 25.0, 10)
                    .insert(GuildInviteYes);
            }
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 210),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 211),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 212),
            ) {
                spawn_icon_button(p, n, h, pr, 340.0, 150.0, 76.0, 25.0, 10)
                    .insert(GuildInviteNo);
            }
        });
    }
}

/// 显隐 + 渲染 + 打开时请求行会信息 + 创建按钮
#[allow(clippy::too_many_arguments)]
fn guild_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut guild: ResMut<GuildState>,
    net: Res<NetConnection>,
    mut input: ResMut<TextInputState>,
    mut create_btns: Query<(Entity, &Interaction, &mut Visibility), With<GuildCreateBtn>>,
    btns: Query<(Entity, &Interaction, &GuildBtn)>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut widgets: Query<(&mut Visibility, Option<&mut UiScrollList>), (With<GuildWidget>, Without<GuildCreateBtn>)>,
    mut lines: Query<(&mut Text, &mut TextColor, &GuildLine)>,
    mut prev_inter: Local<HashMap<Entity, Interaction>>,
    mut requested: Local<bool>,
    panel_origin: Query<&Node, With<GuildWidget>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    let open = mgr.is_open(DialogKind::Guild);
    for (mut vis, sl) in &mut widgets {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if let Some(mut sl) = sl {
            // #89 成员列表行数（滚动夹紧）
            sl.set_total(if guild.in_guild {
                guild.visible_member_indices().len()
            } else {
                0
            });
        }
    }
    // 创建行会按钮：仅对话框打开且未入会时显示（此前完全没管理显隐，一直残留屏幕）；
    // 点击动作在下方"创建按钮 → GuildNameReturn"统一处理
    for (_, _, mut vis) in &mut create_btns {
        *vis = if open && !guild.in_guild {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        *requested = false;
        return;
    }
    // 打开瞬间请求行会信息（原版 C# GuildDialog.Show → RequestGuildInfo）
    if !*requested {
        *requested = true;
        net.send_packet(&mir2_shared::packets::client::guild::RequestGuildInfo { info_type: 0 });
        // #2537：Buff 列表（C.GuildBuffUpdate action=0，C# RequestGuildBuffList；每次打开刷新）
        net.send_packet(&mir2_shared::packets::client::guild::GuildBuffUpdate {
            action: 0,
            buff_id: 0,
        });
        tracing::info!("🏰 请求行会信息 + 行会技能列表");
    }
    // 关闭（bevy_ui Interaction 边沿）
    for (e, inter, k) in &btns {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        match k.0 {
            GuildBtnKind::Close => {
                mgr.close(DialogKind::Guild);
            }
            GuildBtnKind::Invite => {
                // 邀请按钮 → EditGuildMember{0=add member}（C# GuildDialog 邀请）
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
            GuildBtnKind::Kick => {
                // 踢出按钮 → EditGuildMember{1=delete member}（对选中的成员）
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
            GuildBtnKind::Notice => {
                // 公告按钮 → EditGuildNotice（C# GuildDialog 公告编辑）
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
            GuildBtnKind::GoldDeposit => {
                // 仓库金币：存入（C# GuildDialog 仓库语义：GuildStorageGoldChange）
                if guild.in_guild {
                    let amount = input
                        .texts
                        .get(3)
                        .cloned()
                        .unwrap_or_default()
                        .trim()
                        .parse::<u32>()
                        .unwrap_or(0);
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
            GuildBtnKind::GoldWithdraw => {
                if guild.in_guild {
                    let amount = input
                        .texts
                        .get(3)
                        .cloned()
                        .unwrap_or_default()
                        .trim()
                        .parse::<u32>()
                        .unwrap_or(0);
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
        }
    }
    // 渲染（#89 成员列表支持滚轮滚动）
    let scroll_offset = widgets
        .iter()
        .find_map(|(_, sl)| sl.map(|s| s.offset))
        .unwrap_or(0);
    // #1348：可见成员下标（过滤离线）
    let visible = guild.visible_member_indices();
    for (mut text, mut color, line) in &mut lines {
        text.0 = match line.0 {
            0 => {
                if guild.show_buff_page {
                    // #2537：Buff 页头（C# BuffPage + PointsLeft；行会等级/剩余点数服务端未同步，显示激活计数）
                    format!(
                        "行会技能（已激活 {}/{}）",
                        guild.active_buffs.len(),
                        guild.buff_catalog.len()
                    )
                } else if guild.in_guild {
                    let notice = guild.notice.first().cloned().unwrap_or_default();
                    if notice.is_empty() {
                        format!("{}（{}）金币:{}", guild.name, guild.leader, guild.gold)
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
                // #2537：Buff 页模式——行 1-8 Buff 目录（buff_start 起 8 项）、行 9 页码
                if guild.show_buff_page {
                    if i <= 8 {
                        let idx = guild.buff_start + (i - 1);
                        match guild.buff_catalog.get(idx) {
                            Some(info) => buff_row_text(info, guild.buff_active(info.id)),
                            None => String::new(),
                        }
                    } else if i == 9 {
                        format!(
                            "技能 第{}/{}页",
                            guild.buff_start / 8 + 1,
                            buff_page_count(guild.buff_catalog.len())
                        )
                    } else {
                        String::new()
                    }
                } else {
                    let idx = scroll_offset + i - 1;
                    // #1348：按 show_offline 过滤后的可见成员映射
                    match visible.get(idx).and_then(|&mi| guild.members.get(mi)) {
                        Some(m) => {
                            // #1395：按 rank_index 显示职务名（C# 按职务分组）
                            let rank = guild
                                .rank_defs
                                .get(m.rank_index as usize)
                                .map(|(n, _)| n.clone())
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
                }
            }
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
        // #140 成员选中行高亮（踢出目标可见）；#2537 Buff 页已激活行绿色
        let c = if guild.show_buff_page {
            let active = matches!(line.0, 1..=8)
                && guild
                    .buff_catalog
                    .get(guild.buff_start + (line.0 - 1))
                    .map(|info| guild.buff_active(info.id))
                    .unwrap_or(false);
            if active {
                Color::srgb(0.5, 1.0, 0.5)
            } else {
                Color::WHITE
            }
        } else if matches!(line.0, 1..=10)
            && guild.selected_member == Some(scroll_offset + line.0 - 1)
        {
            Color::srgb(1.0, 0.9, 0.3)
        } else {
            Color::WHITE
        };
        if color.0 != c {
            color.0 = c;
        }
    }
    // 创建按钮 → GuildNameReturn（原版 C#：输入行会名 → 创建）
    for (e, inter, _) in &create_btns {
        if edge(e, inter, &mut prev_inter) {
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
    // 点击成员行选中（踢出目标）；Buff 页模式下由 guild_buff_system 处理点击
    if mouse.just_pressed(MouseButton::Left) {
        if let Ok(window) = windows.single() {
            if let Some(cursor) = window.cursor_position() {
                let (ox, oy) = panel_origin
                    .single()
                    .map(|n| crate::ui::theme::node_origin(n, (GUILD_X, GUILD_Y)))
                    .unwrap_or((GUILD_X, GUILD_Y));
                if !guild.show_buff_page {
                    let visible = guild.visible_member_indices();
                    for i in 1..=10usize {
                        let (rx, ry, rw, rh) = guild_member_row_rect(i, ox, oy);
                        if cursor.x >= rx
                            && cursor.x <= rx + rw
                            && cursor.y >= ry
                            && cursor.y <= ry + rh
                        {
                            let idx = scroll_offset + i - 1;
                            if let Some(&mi) = visible.get(idx) {
                                guild.selected_member = Some(idx);
                                tracing::info!("🏰 选中行会成员: {}", guild.members[mi].name);
                            }
                            break;
                        }
                    }
                }
                // 仓库格子点击选中（取出目标，原版 C# StorageGrid 点击语义）
                for i in 11..=18usize {
                    let (rx, ry, rw, rh) = guild_storage_row_rect(i, ox, oy);
                    if cursor.x >= rx
                        && cursor.x <= rx + rw
                        && cursor.y >= ry
                        && cursor.y <= ry + rh
                    {
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

/// 成员行命中矩形（面板原点 ox/oy + 相对坐标；i 1..=10）
fn guild_member_row_rect(i: usize, ox: f32, oy: f32) -> (f32, f32, f32, f32) {
    (ox + 18.0, oy + 60.0 + (i - 1) as f32 * 20.0, 320.0, 18.0)
}

/// 仓库格命中矩形（i 11..=18）
fn guild_storage_row_rect(i: usize, ox: f32, oy: f32) -> (f32, f32, f32, f32) {
    (ox + 18.0, oy + 515.0 + (i - 11) as f32 * 18.0, 320.0, 16.0)
}

/// Buff 行命中矩形（i 1..=8）
fn guild_buff_row_rect(i: usize, ox: f32, oy: f32) -> (f32, f32, f32, f32) {
    (ox + 18.0, oy + 60.0 + (i - 1) as f32 * 20.0, 218.0, 18.0)
}

/// #2537 Buff 页交互（独立系统：guild_ui_system 已满 16 参 Bevy SystemParam 上限）
/// 开关/翻页 + 行点击（C# BuffButton/RequestBuff/UpButton/DownButton）
fn guild_buff_system(
    mut guild: ResMut<GuildState>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    buff_toggle_btn: Query<(Entity, &Interaction), With<GuildBuffToggleBtn>>,
    buff_up_btn: Query<(Entity, &Interaction), With<GuildBuffUp>>,
    buff_down_btn: Query<(Entity, &Interaction), With<GuildBuffDown>>,
    mut prev_inter: Local<HashMap<Entity, Interaction>>,
    panel_origin: Query<&Node, With<GuildWidget>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    // Buff 页开关（C# BuffButton 切换 BuffPage）
    for (e, inter) in &buff_toggle_btn {
        if edge(e, inter, &mut prev_inter) {
            guild.show_buff_page = !guild.show_buff_page;
            tracing::info!(
                "🏴 行会技能页: {}",
                if guild.show_buff_page { "开" } else { "关" }
            );
        }
    }
    for (e, inter) in &buff_up_btn {
        if edge(e, inter, &mut prev_inter) {
            guild.buff_start = guild.buff_start.saturating_sub(8);
        }
    }
    for (e, inter) in &buff_down_btn {
        if edge(e, inter, &mut prev_inter) && guild.buff_start + 8 < guild.buff_catalog.len() {
            guild.buff_start += 8;
        }
    }
    if !guild.show_buff_page || !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    // 行点击 → C.GuildBuffUpdate；服务端 toggle 语义（未激活→激活收费校验，已激活→停用），结果走系统消息
    let (ox, oy) = panel_origin
        .single()
        .map(|n| crate::ui::theme::node_origin(n, (GUILD_X, GUILD_Y)))
        .unwrap_or((GUILD_X, GUILD_Y));
    for i in 1..=8usize {
        let (rx, ry, rw, rh) = guild_buff_row_rect(i, ox, oy);
        if cursor.x >= rx && cursor.x <= rx + rw && cursor.y >= ry && cursor.y <= ry + rh {
            if let Some(info) = guild.buff_catalog.get(guild.buff_start + i - 1) {
                net.send_packet(&mir2_shared::packets::client::guild::GuildBuffUpdate {
                    action: 2,
                    buff_id: info.id,
                });
                tracing::info!(
                    "🏴 行会技能: {} #{}（服务端 toggle）",
                    if guild.buff_active(info.id) {
                        "停用"
                    } else {
                        "激活"
                    },
                    info.id
                );
            }
            break;
        }
    }
}

/// #1362：职务改名（C# RanksSelectBox + RanksName + RanksSaveName → EditGuildMember ChangeType=6）
fn guild_rank_rename_system(
    guild: Res<GuildState>,
    net: Res<NetConnection>,
    mut input: ResMut<TextInputState>,
    mut rank_dd: Query<(&mut UiDropDown, &GuildRankDrop)>,
    save_btn: Query<(Entity, &Interaction), With<GuildRankSaveBtn>>,
    mut prev_inter: Local<HashMap<Entity, Interaction>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    // #1395：下拉同步服务端职务定义（顺序即索引）
    let defs = guild.rank_defs.clone();
    let idx = if let Ok((mut dd, _)) = rank_dd.single_mut() {
        if dd.items != defs.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>() {
            dd.items = defs.iter().map(|(n, _)| n.clone()).collect();
            dd.selected = dd.selected.filter(|&s| s < dd.items.len());
        }
        dd.selected.unwrap_or(0)
    } else {
        0
    };
    for (e, inter) in &save_btn {
        if edge(e, inter, &mut prev_inter) && guild.in_guild {
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

/// #1395 子批2：加职务/权限勾选/调职（C# EditGuildMember 4/5/2）
fn guild_rank_manage_system(
    guild: Res<GuildState>,
    net: Res<NetConnection>,
    mut input: ResMut<TextInputState>,
    mut rank_dd: Query<(&mut UiDropDown, &GuildRankDrop)>,
    add_btn: Query<(Entity, &Interaction), With<GuildAddRankBtn>>,
    promote_btn: Query<(Entity, &Interaction), With<GuildPromoteBtn>>,
    perm_btns: Query<(Entity, &Interaction, &GuildRankPermBtn)>,
    mut perm_text: Query<&mut Text, With<GuildRankPermText>>,
    mut prev_inter: Local<HashMap<Entity, Interaction>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    let idx = rank_dd
        .single_mut()
        .map(|(dd, _)| dd.selected.unwrap_or(0))
        .unwrap_or(0);
    let options = guild.rank_defs.get(idx).map(|(_, o)| *o).unwrap_or(0);
    for mut t in &mut perm_text {
        let s = format!("权限:{:08b}", options);
        if t.0 != s {
            t.0 = s;
        }
    }
    if !guild.in_guild {
        return;
    }
    for (e, inter) in &add_btn {
        if edge(e, inter, &mut prev_inter) {
            let name = input.texts.get(7).cloned().unwrap_or_default();
            let name = name.trim().to_string();
            if !name.is_empty() {
                net.send_packet(&mir2_shared::packets::client::guild::EditGuildMember {
                    change_type: 4,
                    rank_index: 0,
                    name: String::new(),
                    rank_name: name.clone(),
                });
                tracing::info!("🏰 添加职务: {}", name);
                if input.texts.len() > 7 {
                    input.texts[7].clear();
                }
                input.active = None;
            }
        }
    }
    for (e, inter, p) in &perm_btns {
        if edge(e, inter, &mut prev_inter) {
            let bit = p.0;
            let on = options & (1 << bit) == 0;
            net.send_packet(&mir2_shared::packets::client::guild::EditGuildMember {
                change_type: 5,
                rank_index: idx as u8,
                name: if on { "true".to_string() } else { "false".to_string() },
                rank_name: bit.to_string(),
            });
            tracing::info!("🏰 职务 #{} 权限位 {} -> {}", idx, bit, on);
        }
    }
    for (e, inter) in &promote_btn {
        if edge(e, inter, &mut prev_inter) {
            if let Some(si) = guild.selected_member {
                if let Some(m) = guild.members.get(si) {
                    net.send_packet(&mir2_shared::packets::client::guild::EditGuildMember {
                        change_type: 2,
                        rank_index: idx as u8,
                        name: m.name.clone(),
                        rank_name: String::new(),
                    });
                    tracing::info!("🏰 调职 {} -> 职务 #{}", m.name, idx);
                }
            }
        }
    }
}

/// #1348：显示离线成员切换（C# MembersShowOfflineButton/Status，纯本地过滤）
fn guild_show_offline_system(
    mut guild: ResMut<GuildState>,
    btn: Query<(Entity, &Interaction), With<GuildShowOfflineBtn>>,
    mut texts: Query<&mut Text, With<GuildShowOfflineText>>,
    mut prev_inter: Local<HashMap<Entity, Interaction>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    for (e, inter) in &btn {
        if edge(e, inter, &mut prev_inter) {
            guild.show_offline = !guild.show_offline;
            if !guild.show_offline {
                guild.selected_member = None;
            }
        }
    }
    for mut t in &mut texts {
        t.0 = if guild.show_offline {
            "✓显示离线".to_string()
        } else {
            "显示离线".to_string()
        };
    }
}

/// 仓库物品交互（M32）：打开时请求列表 + 存入/取出/翻页
/// 原版 C# GuildDialog：StorageGrid 点击选中 → 拖拽/按钮存入取出；列表由
/// S.GuildStorageList 推送（C# GuildStorageItemChange type=3 请求）
#[allow(clippy::too_many_arguments)]
fn guild_storage_system(
    mgr: ResMut<DialogManager>,
    mut guild: ResMut<GuildState>,
    net: Res<NetConnection>,
    inv_q: Query<&crate::game::player_state::Inventory, With<crate::actor::LocalPlayer>>,
    inv_click: Res<crate::game::dialogs::inventory::InvClickState>,
    deposit_btn: Query<(Entity, &Interaction), With<GuildItemDeposit>>,
    withdraw_btn: Query<(Entity, &Interaction), With<GuildItemWithdraw>>,
    up_btn: Query<(Entity, &Interaction), With<GuildStorageUp>>,
    down_btn: Query<(Entity, &Interaction), With<GuildStorageDown>>,
    mut prev_inter: Local<HashMap<Entity, Interaction>>,
    mut requested: Local<bool>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
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
    for (e, inter) in &up_btn {
        if edge(e, inter, &mut prev_inter) {
            guild.storage_page = guild.storage_page.saturating_sub(1);
        }
    }
    for (e, inter) in &down_btn {
        if edge(e, inter, &mut prev_inter) && guild.storage_page + 1 < 13 {
            guild.storage_page += 1;
        }
    }
    for (e, inter) in &deposit_btn {
        if edge(e, inter, &mut prev_inter) && guild.in_guild {
            // 选中背包物品 → 存入（原版 C#：选中物品 → GuildStorageItemChange type=0）
            let items = inv_q.single().map(|inv| inv.items.as_slice()).unwrap_or(&[]);
            let idx = inv_click
                .selected
                .filter(|i| items.get(*i).and_then(|s| s.as_ref()).is_some())
                .or_else(|| items.iter().position(|s| s.is_some()));
            if let Some(i) = idx {
                if let Some(item) = items.get(i).and_then(|s| s.as_ref()) {
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
    for (e, inter) in &withdraw_btn {
        if edge(e, inter, &mut prev_inter) && guild.in_guild {
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
    yes: Query<(Entity, &Interaction), With<GuildInviteYes>>,
    no: Query<(Entity, &Interaction), With<GuildInviteNo>>,
    mut widgets: Query<&mut Visibility, With<GuildInviteWidget>>,
    mut texts: Query<&mut Text, With<GuildInviteText>>,
    mut prev_inter: Local<HashMap<Entity, Interaction>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    let has_invite = guild.invite.is_some();
    for mut vis in &mut widgets {
        *vis = if has_invite {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut text in &mut texts {
        text.0 = match guild.invite.as_ref() {
            Some(name) => format!("{} 邀请你加入行会", name),
            None => String::new(),
        };
    }
    if guild.invite.is_none() {
        return;
    }
    let mut accept: Option<bool> = None;
    for (e, inter) in &yes {
        if edge(e, inter, &mut prev_inter) {
            accept = Some(true);
        }
    }
    for (e, inter) in &no {
        if edge(e, inter, &mut prev_inter) {
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
            ServerEvent::GuildData { name, leader, rank_defs, notice, members, gold } => {
                guild.in_guild = true;
                guild.name = name.clone();
                guild.leader = leader.clone();
                guild.rank_defs = rank_defs.clone();
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
                            rank_index: *rank,
                            online: *online,
                        });
                    }
                } else if let Some(m) = guild.members.iter_mut().find(|m| m.name == *name) {
                    m.rank = *rank;
                    m.rank_index = *rank;
                    m.online = *online;
                }
            }
            ServerEvent::GuildInvited { name } => {
                guild.invite = Some(name.clone());
            }
            ServerEvent::GuildBuffList { active, catalog } => {
                // #2537：行会技能同步（目录 + 激活列表；打开对话框/他人变更时刷新）
                guild.active_buffs = active.clone();
                guild.buff_catalog = catalog.clone();
                // 目录变短时夹紧到最后一页起点（8 行/页）
                let max_start = guild.buff_catalog.len().saturating_sub(1) / 8 * 8;
                if guild.buff_start > max_start {
                    guild.buff_start = max_start;
                }
                tracing::info!(
                    "🏴 行会技能已同步: 目录 {} 项（激活 {}）",
                    catalog.len(),
                    active.len()
                );
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

#[cfg(test)]
mod tests {
    /// 成员行命中：初始原点等价于原固定坐标，拖动后跟随面板
    #[test]
    fn member_row_rect_origin_and_drag() {
        // 初始 (GUILD_X=280, GUILD_Y=80)：首行 y=140（=80+60），x 起 298（=280+18）
        let (rx, ry, rw, rh) = guild_member_row_rect(1, GUILD_X, GUILD_Y);
        assert_eq!((rx, ry, rw, rh), (298.0, 140.0, 320.0, 18.0));
        assert_eq!(guild_member_row_rect(10, GUILD_X, GUILD_Y).1, 140.0 + 9.0 * 20.0);
        // 拖动到 (330,100)：同一相对位置命中跟随（原始坐标 + delta(50,20)）
        let (rx2, ry2, _, _) = guild_member_row_rect(1, 330.0, 100.0);
        assert_eq!((rx2, ry2), (348.0, 160.0));
    }

    /// 仓库格命中：初始等价 + 拖动跟随
    #[test]
    fn storage_row_rect_origin_and_drag() {
        let (rx, ry, _, _) = guild_storage_row_rect(11, GUILD_X, GUILD_Y);
        assert_eq!((rx, ry), (298.0, 595.0), "初始 11 格 y=595");
        let (rx2, ry2, _, _) = guild_storage_row_rect(11, 330.0, 100.0);
        assert_eq!((rx2, ry2), (348.0, 615.0), "拖动后跟随");
    }

    /// Buff 行命中：初始等价 + 拖动跟随
    #[test]
    fn buff_row_rect_origin_and_drag() {
        let (rx, ry, rw, _) = guild_buff_row_rect(1, GUILD_X, GUILD_Y);
        assert_eq!((rx, ry, rw), (298.0, 140.0, 218.0));
        let (rx2, ry2, _, _) = guild_buff_row_rect(1, 330.0, 100.0);
        assert_eq!((rx2, ry2), (348.0, 160.0));
    }


    use super::*;

    fn buff_info(id: i32, name: &str) -> mir2_shared::data::client_data::GuildBuffInfo {
        mir2_shared::data::client_data::GuildBuffInfo {
            id,
            icon: 24,
            name: name.to_string(),
            level_requirement: 3,
            points_requirement: 2,
            time_limit: 60,
            activation_cost: 500,
            stats: mir2_shared::data::stats::Stats::new(),
        }
    }

    /// #2537 Buff 行文本（激活态标记，C# GuildBuffButton Name/Info）
    #[test]
    fn buff_row_text_marks_active() {
        let info = buff_info(1, "经验加成");
        assert!(buff_row_text(&info, true).contains("[已激活]"));
        assert!(!buff_row_text(&info, false).contains("[已激活]"));
        assert!(buff_row_text(&info, false).contains("Lv3"));
        assert!(buff_row_text(&info, false).contains("点2"));
        assert!(buff_row_text(&info, false).contains("金500"));
    }

    /// #2537 页数（C# 8 GuildBuffButton/页）：0/8 → 1 页，9/16 → 2 页
    #[test]
    fn buff_page_count_rounds_up() {
        assert_eq!(buff_page_count(0), 1);
        assert_eq!(buff_page_count(8), 1);
        assert_eq!(buff_page_count(9), 2);
        assert_eq!(buff_page_count(16), 2);
    }

    /// #2537 目录同步夹紧：buff_start 超出末页回夹（8 行/页）
    #[test]
    fn buff_start_clamped_on_sync() {
        let mut guild = GuildState::default();
        guild.buff_catalog = vec![buff_info(1, "a"); 16];
        guild.buff_start = 8;
        guild.active_buffs = vec![1];
        // 复现 GuildBuffList arm 的夹紧逻辑
        let max_start = guild.buff_catalog.len().saturating_sub(1) / 8 * 8;
        if guild.buff_start > max_start {
            guild.buff_start = max_start;
        }
        assert_eq!(guild.buff_start, 8);
        assert!(guild.buff_active(1));
        assert!(!guild.buff_active(2));
        // 目录缩到 9 项 → 末页起点 0…wait 9 项末页起点 = 8/8*8 = 8? (9-1)/8*8 = 8
        guild.buff_catalog.truncate(9);
        let max_start = guild.buff_catalog.len().saturating_sub(1) / 8 * 8;
        if guild.buff_start > max_start {
            guild.buff_start = max_start;
        }
        assert_eq!(guild.buff_start, 8);
        // 目录缩到 8 项 → 末页起点 0
        guild.buff_catalog.truncate(8);
        let max_start = guild.buff_catalog.len().saturating_sub(1) / 8 * 8;
        if guild.buff_start > max_start {
            guild.buff_start = max_start;
        }
        assert_eq!(guild.buff_start, 0);
    }
}
