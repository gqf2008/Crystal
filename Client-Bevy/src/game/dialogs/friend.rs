// ============================================================================
// 好友对话框（M25）
// 布局参考：C# FriendDialog.cs / macroquad friend_dialog.rs
//   - 背景 Title[199]，位置 (300,100)，标题 Title[6] (18,9)
//   - 好友列表 y=40 每 20px；打开时自动请求 C.RefreshFriends
// 网络：FriendUpdate（列表 / 单个添加，同 opcode 双格式）→ 列表渲染（在线/离线）
// ============================================================================

use bevy::prelude::*;

use crate::game::chat::{ChatChannel, ChatState};
use crate::game::dialogs::mail::ComposeMail;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont};
use crate::ui::theme::{
    load_lib_image, spawn_close_button, spawn_container, spawn_icon_button, spawn_image,
    spawn_label, spawn_panel,
};

/// #2892 批B：面板精灵与 C# 原生尺寸（C# `FriendDialog.Index = 199; Library = Libraries.Title`）
pub const PANEL: (LibraryName, usize) = (LibraryName::Title, 199);
pub const PANEL_SIZE: (f32, f32) = (264.0, 272.0);
/// 关闭键 `Prguse2[360..362]` @(237,3)（`FriendDialog.cs:123-124`，无 `Size` → 原生 24x21）；
/// 曾错作 (206,3) → 偏左 31px
pub const CLOSE_POS: (f32, f32) = (237.0, 3.0);

// ---------------------------------------------------------------------------
// 好友列表 = **12 行 × 2 列**（C# `FriendDialog.Rows = new FriendRow[12]`，
// `UpdateDisplay` 里 `Location = new Point((i % 2) * 115 + 16, 55 + (i / 2) * 22)`，
// `FriendRow.Size = new Size(115, 17)`）——本端此前是**单列 10 行 @(18, 40+20i)**，
// 且没有翻页条；金标准逐窗 A/B 里 Friends 那 25.6% 的差异主要就是这一行 `◀ 1/1 ▶`
// 缺失（见 `tools/acceptance/csharp_golden/README.md` §3.2f）。
// ---------------------------------------------------------------------------
/// C# `Rows.Length` = 12（2 列 × 6 行）
pub const FRIEND_ROW_COUNT: usize = 12;
/// C# `FriendRow.Size`
pub const FRIEND_ROW_SIZE: (f32, f32) = (115.0, 17.0);
/// 第 `i` 行的面板内位置（C# `(i % 2) * 115 + 16, 55 + (i / 2) * 22`）
pub fn friend_row_pos(i: usize) -> (f32, f32) {
    (
        (i % 2) as f32 * 115.0 + 16.0,
        55.0 + (i / 2) as f32 * 22.0,
    )
}

/// 翻页条（C# `FriendDialog.cs:70-118`）：`PageNumberLabel` @(87,216) 83x17 居中、
/// 上一页 `Prguse2[240/241/242]` @(70,218) 16x16、下一页 `Prguse2[243/244/245]` @(171,218) 16x16
pub const FRIEND_PAGE_LABEL_POS: (f32, f32) = (87.0, 216.0);
pub const FRIEND_PAGE_LABEL_SIZE: (f32, f32) = (83.0, 17.0);
pub const FRIEND_PREV_POS: (f32, f32) = (70.0, 218.0);
pub const FRIEND_NEXT_POS: (f32, f32) = (171.0, 218.0);
pub const FRIEND_PAGE_BTN_SIZE: (f32, f32) = (16.0, 16.0);
/// C# 翻页按钮帧（`Prguse2`）：上一页 240/241/242、下一页 243/244/245
pub const FRIEND_PREV_FRAMES: [usize; 3] = [240, 241, 242];
pub const FRIEND_NEXT_FRAMES: [usize; 3] = [243, 244, 245];

/// C# `int maxPage = filteredFriends.Count / Rows.Length + 1; if (maxPage < 1) maxPage = 1;`
pub fn friend_page_max(count: usize) -> usize {
    (count / FRIEND_ROW_COUNT + 1).max(1)
}

/// C# `StartIndex = Rows.Length * Page;`
pub fn friend_page_start(page: usize) -> usize {
    FRIEND_ROW_COUNT * page
}

/// C# `PreviousButton`（`Page--` 且 `if (Page < 0) Page = 0`）与
/// `NextButton`（`Page++` 且 `if (Page > Count / Rows.Length) Page = Count / Rows.Length`）——
/// 注意 Next 的上界是 `Count / Rows.Length`（**不是** `maxPage - 1`，两者在整除时不同）。
pub fn friend_page_after(delta: i64, page: usize, count: usize) -> usize {
    let page = page as i64 + delta;
    page.clamp(0, (count / FRIEND_ROW_COUNT) as i64) as usize
}

/// 页签切换/列表变短后的夹紧（C# `UpdateDisplay` 里对 `StartIndex` 的夹紧等价物）
pub fn friend_page_clamped(page: usize, count: usize) -> usize {
    page.min(count / FRIEND_ROW_COUNT)
}

/// 当前页的列表切片起点对应的下标（行 `i` 显示 `filtered[start + i]`）
pub fn friend_row_index(start: usize, i: usize) -> usize {
    start + i
}

/// 好友条目
#[derive(Debug, Clone, Default)]
pub struct FriendEntry {
    pub object_id: u32,
    pub name: String,
    pub memo: String,
    /// 是否黑名单（C# ClientFriend.Blocked）
    pub blocked: bool,
    pub online: bool,
}

/// 待处理输入动作（添加/备注）
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FriendPending {
    Memo(usize),
}

/// 好友状态（网络 FriendUpdate 写入）
#[derive(Resource, Default)]
pub struct FriendState {
    pub friends: Vec<FriendEntry>,
    /// 选中的好友行（删除/备注用）
    pub selected: Option<usize>,
    /// 待处理的内嵌输入动作
    pub pending: Option<FriendPending>,
    /// 当前页签（false=好友 true=黑名单，C# _blockedTab）
    pub blocked_tab: bool,
    /// 当前页码（C# `FriendDialog.Page`，0 起；列表按 12 行/页翻）
    pub page: usize,
    /// #2892 批D：「添加好友」请求（`Some(blocked)` = 当前页签是否黑名单），
    /// 由 `friend_add_open_system` 消费 → 打开 C# `MirInputBox`（`FriendDialog.cs:143-160`）
    pub add_request: Option<bool>,
}

#[derive(Component)]
pub struct FriendWidget;

#[derive(Component)]
pub struct FriendClose;

#[derive(Component)]
pub struct FriendAdd;

#[derive(Component)]
pub struct FriendRemove;

#[derive(Component)]
pub struct FriendMemo;

/// 私聊选中好友（C# FriendDialog WhisperButton）
#[derive(Component)]
pub struct FriendWhisper;

/// 发邮件给选中好友（C# FriendDialog EmailButton）
#[derive(Component)]
pub struct FriendEmail;

/// 好友页签（C# FriendLabel）
#[derive(Component)]
pub struct FriendTabFriend;

/// 黑名单页签（C# BlacklistLabel）
#[derive(Component)]
pub struct FriendTabBlock;

#[derive(Component)]
pub struct FriendLine(usize);

/// bevy_ui 行文本子节点（父 Button 挂 FriendLine，子文本挂 FriendLineText）
#[derive(Component)]
pub struct FriendLineText(usize);

/// 上一页按钮（C# `PreviousButton`）
#[derive(Component)]
pub struct FriendPagePrev;

/// 下一页按钮（C# `NextButton`）
#[derive(Component)]
pub struct FriendPageNext;

/// 页码标签（C# `PageNumberLabel`，"N / M"）
#[derive(Component)]
pub struct FriendPageLabel;

/// friend_ui_system 的 Local 状态（合并以控制 Bevy 系统参数数 ≤16）
#[derive(Default)]
struct FriendLocal {
    prev_inter: std::collections::HashMap<Entity, Interaction>,
    requested: bool,
    /// 上一帧好友窗是否开着：**只在「刚关」那一次**清文本焦点。
    /// 修 #3260 暴露的真缺陷：原实现 `if !open { input.active = None; }` 是**每帧**执行的，
    /// 于是好友窗没开时会把**别的窗**的输入焦点一起清掉（实测：仓库密码 `MirInputBox` 弹出后
    /// 被它每帧清成 `active=None`，`type_text` 打进去的字没人接，玩家打字也没用）。
    was_open: bool,
}

/// 好友动作按钮（添加/删除/备注/邮件/私聊；bevy_ui Interaction 驱动）
#[derive(Component, Clone, Copy)]
pub struct FriendAction {
    pub is_add: bool,
    pub is_remove: bool,
    pub is_memo: bool,
    pub is_email: bool,
    pub is_whisper: bool,
}

pub struct FriendPlugin;

impl Plugin for FriendPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FriendState>();
        app.init_resource::<UiCjkFont>();
        app.add_systems(
            Update,
            friend_server_events.run_if(in_state(AppState::Game)),
        );
        app.add_systems(OnEnter(AppState::Game), spawn_friend);
        app.add_systems(OnExit(AppState::Game), cleanup_friend);
        app.add_systems(Update, friend_ui_system.run_if(in_state(AppState::Game)));
        // 页码标签（C# `PageNumberLabel.Text = (Page + 1) + " / " + maxPage`）单列一个小系统：
        // `friend_ui_system` 的参数已接近 Bevy 上限，再塞一个 `&mut Text` 查询不划算
        app.add_systems(
            Update,
            friend_page_label_system
                .after(friend_ui_system)
                .run_if(in_state(AppState::Game)),
        );
        app.add_systems(
            Update,
            friend_memo_open_system
                .after(friend_ui_system)
                .run_if(in_state(AppState::Game)),
        );
        // #2892 批D：「添加好友」走 C# `MirInputBox`（客户端发起式）
        app.add_systems(
            Update,
            friend_add_open_system
                .after(friend_ui_system)
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_friend(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_friend(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut cjk_font: ResMut<UiCjkFont>,
) {
    libs.0.ensure_initialized();
    // 整面板中文此前走 Arial 主字体 → 全是豆腐（实机截图：标签 `□□`/`□□□`、
    // 行内 `bevy2char□□□□`）。列表行是**动态写入**（`text.0 = match list.get(idx)`），
    // 必须用自带 CJK 的主字体，与其余 UI 模块一致。
    let font = shared_cjk_font(&mut fonts, &mut cjk_font);
    let white = images.add(crate::map_renderer::make_image(
        vec![255, 255, 255, 255],
        1,
        1,
    ));

    // 面板 Title[199] 原生 264x272，C# Location = Center。
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 199) else {
        return;
    };
    let (px, py) = crate::game::dialogs::center_origin(264.0, 272.0);
    let panel = spawn_panel(&mut commands, bg, px, py, PANEL_SIZE.0, PANEL_SIZE.1, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::Friend), FriendWidget));

    commands.entity(panel).with_children(|p| {
        // 标题 Title[6] @(18,9)
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 6) {
            spawn_image(p, h, 18.0, 9.0, 57.0, 15.0, 9);
        }
        // 关闭 Prguse2[360/361/362] @ CLOSE_POS
        if let Some(mut btn) =
            spawn_close_button(p, &mut libs, &mut images, CLOSE_POS.0, CLOSE_POS.1, 10)
        {
            btn.insert(FriendClose);
        }
        // 添加/删除/备注/邮件/私聊（Prguse 554-568 @(60/88/116/144/172, 241)）
        // #2771：末列为 C# `FriendDialog.cs` 的 Hint（141 AddFriend=添加、171 RemoveFriend=移除、
        // 197 FriendMemo=备注、217 FriendMail=邮件、235 FriendWhisper=悄悄话）
        let acts: [(bool, bool, bool, bool, bool, usize, f32, &str); 5] = [
            (true, false, false, false, false, 554, 60.0, "添加"),
            (false, true, false, false, false, 557, 88.0, "移除"),
            (false, false, true, false, false, 560, 116.0, "备注"),
            (false, false, false, true, false, 563, 144.0, "邮件"),
            (false, false, false, false, true, 566, 172.0, "悄悄话"),
        ];
        for (is_add, is_remove, is_memo, is_email, is_whisper, idx, x, hint) in acts {
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse, idx),
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse, idx + 1),
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse, idx + 2),
            ) {
                spawn_icon_button(p, n, h, pr, x, 241.0, 24.0, 22.0, 10).insert((
                    FriendAction {
                        is_add,
                        is_remove,
                        is_memo,
                        is_email,
                        is_whisper,
                    },
                    crate::ui::tooltip::UiHint {
                        text: hint.to_string(),
                    },
                ));
            }
        }

        // 页签 = **贴图按钮**（C# `FriendLabel` = `Title[163]` @(10,34)、
        // `BlacklistLabel` = `Title[167]` @(128,34)，精灵均为 124x24）。
        // 本端原先画的是「好友」「黑名单」两个**文字**标签 @(18,18)/(70,18)：
        // 位置既不对（C# 在 y=34），又正好压在面板美术自带的 `FRIEND` 标题上（实机截图 z_friend.png）。
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 163) {
            // C# 是 `MirImageControl`（无 hover/pressed 帧）→ 三帧同图
            spawn_icon_button(p, h.clone(), h.clone(), h, 10.0, 34.0, 124.0, 24.0, 10)
                .insert((Button, FriendTabFriend));
        }
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 167) {
            spawn_icon_button(p, h.clone(), h.clone(), h, 128.0, 34.0, 124.0, 24.0, 10)
                .insert((Button, FriendTabBlock));
        }
        // 好友列表：**12 行 × 2 列**（C# `FriendRow[12]` + `UpdateDisplay` 的格子位置）
        for i in 0..FRIEND_ROW_COUNT {
            let (rx, ry) = friend_row_pos(i);
            spawn_container(p, rx, ry, FRIEND_ROW_SIZE.0, FRIEND_ROW_SIZE.1, 9)
                .insert((Button, FriendLine(i)))
                .with_children(|rc| {
                    rc.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(0.0),
                            ..default()
                        },
                        Text::new(String::new()),
                        TextFont {
                            font: FontSource::Handle(font.clone()),
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        ZIndex(10),
                        FriendLineText(i),
                    ));
                });
        }
        // 翻页条（C# `PageNumberLabel` + `PreviousButton`/`NextButton`）
        spawn_container(
            p,
            FRIEND_PAGE_LABEL_POS.0,
            FRIEND_PAGE_LABEL_POS.1,
            FRIEND_PAGE_LABEL_SIZE.0,
            FRIEND_PAGE_LABEL_SIZE.1,
            9,
        )
        .insert(FriendPageLabel)
        .with_children(|lc| {
            lc.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(FRIEND_PAGE_LABEL_SIZE.0),
                    ..default()
                },
                Text::new("1 / 1"),
                TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                TextLayout::justify(Justify::Center),
                ZIndex(10),
            ));
        });
        for (frames, x, y, is_prev) in [
            (
                FRIEND_PREV_FRAMES,
                FRIEND_PREV_POS.0,
                FRIEND_PREV_POS.1,
                true,
            ),
            (
                FRIEND_NEXT_FRAMES,
                FRIEND_NEXT_POS.0,
                FRIEND_NEXT_POS.1,
                false,
            ),
        ] {
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, frames[0]),
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, frames[1]),
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, frames[2]),
            ) {
                let mut btn = spawn_icon_button(
                    p,
                    n,
                    h,
                    pr,
                    x,
                    y,
                    FRIEND_PAGE_BTN_SIZE.0,
                    FRIEND_PAGE_BTN_SIZE.1,
                    10,
                );
                if is_prev {
                    btn.insert(FriendPagePrev);
                } else {
                    btn.insert(FriendPageNext);
                }
            }
        }
    });
}

/// 显隐 + 列表渲染 + 打开时自动请求刷新（原版 C# FriendDialog.Show → RefreshFriends）
#[allow(clippy::too_many_arguments)]
fn friend_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut friend: ResMut<FriendState>,
    mut compose_mail: MessageWriter<ComposeMail>,
    mut chat: ResMut<ChatState>,
    net: Res<NetConnection>,
    mut wheels: MessageReader<bevy::input::mouse::MouseWheel>,
    close: Query<(Entity, &Interaction), With<FriendClose>>,
    actions: Query<(Entity, &Interaction, &FriendAction)>,
    tabs: Query<(Entity, &Interaction, Has<FriendTabBlock>)>,
    rows: Query<(Entity, &Interaction, &FriendLine), Without<FriendLineText>>,
    // 翻页条两个按钮（C# PreviousButton / NextButton）
    pager: Query<
        (
            Entity,
            &Interaction,
            Option<&FriendPagePrev>,
            Option<&FriendPageNext>,
        ),
        (Without<FriendLine>, Without<FriendClose>),
    >,
    mut line_texts: Query<(&mut Text, &mut TextColor, &FriendLineText)>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    mut widgets: Query<&mut Visibility, (With<FriendWidget>, Without<FriendLineText>)>,
    mut local: Local<FriendLocal>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }

    let open = mgr.is_open(DialogKind::Friend);
    for mut vis in widgets.iter_mut() {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        local.requested = false;
        friend.pending = None;
        friend.selected = None;
        // 只清**自己关掉时**的焦点：好友窗没开时不该动别的窗的输入焦点（#3260）
        if local.was_open {
            input.active = None;
            local.was_open = false;
        }
        friend.page = 0;
        return;
    }
    local.was_open = true;
    if !local.requested {
        local.requested = true;
        net.send_packet(&mir2_shared::packets::client::friend::RefreshFriends);
        tracing::info!("👥 请求刷新好友列表");
    }
    for (e, inter) in &close {
        if edge(e, inter, &mut local.prev_inter) {
            mgr.close(DialogKind::Friend);
        }
    }
    // 当前页签的显示列表
    let list = filter_friends(&friend.friends, friend.blocked_tab);
    // C# 好友列表是**翻页**（12 行/页），不是滚轮滚动 ⇒ 本端去掉滚轮分支，改按 `Page` 取切片。
    for ev in wheels.read() {
        let _ = ev; // 读掉滚轮消息，避免它在别的列表里被当成"这一帧还有残留输入"
    }
    friend.page = friend_page_clamped(friend.page, list.len());
    let start = friend_page_start(friend.page);
    // 列表文本（含在线标记/备注/选中高亮）
    for (mut text, mut color, line) in &mut line_texts {
        let idx = friend_row_index(start, line.0);
        let selected = friend.selected == Some(idx);
        text.0 = match list.get(idx) {
            Some(f) => {
                let mark = if f.online {
                    "（在线）"
                } else {
                    "（离线）"
                };
                let name = if f.memo.is_empty() {
                    f.name.clone()
                } else {
                    format!("{} ({})", f.name, f.memo)
                };
                format!("{}{}", name, mark)
            }
            None => String::new(),
        };
        color.0 = if selected {
            Color::srgb(1.0, 0.9, 0.3)
        } else {
            Color::WHITE
        };
    }
    // 行点击选中（#130）
    for (e, inter, line) in &rows {
        if edge(e, inter, &mut local.prev_inter) {
            let idx = friend_row_index(start, line.0);
            friend.selected = if friend.selected == Some(idx) {
                None
            } else {
                Some(idx)
            };
        }
    }
    // 翻页（C# PreviousButton / NextButton）
    for (e, inter, prev, next) in &pager {
        if !edge(e, inter, &mut local.prev_inter) {
            continue;
        }
        let delta = if prev.is_some() {
            -1
        } else if next.is_some() {
            1
        } else {
            continue;
        };
        friend.page = friend_page_after(delta, friend.page, list.len());
        friend.selected = None;
    }
    // 动作按钮 + 页签
    for (e, inter, act) in &actions {
        if !edge(e, inter, &mut local.prev_inter) {
            continue;
        }
        if act.is_add {
            // #2892 批D：C# `AddButton.Click → new MirInputBox(FriendEnterAddName/FriendEnterBlockName)`
            // （`FriendDialog.cs:143-160`）→ 记一个请求，由 `friend_add_open_system` 走通用 `MirInputBox`
            friend.add_request = Some(friend.blocked_tab);
        } else if act.is_remove {
            if let Some(idx) = friend.selected {
                if let Some(f) = list.get(idx) {
                    net.send_packet(&mir2_shared::packets::client::friend::RemoveFriend {
                        character_index: f.object_id as i32,
                    });
                    friend.selected = None;
                }
            }
        } else if act.is_memo {
            if let Some(idx) = friend.selected {
                // #2892 批D 单元①：C# `MemoButton.Click → MemoDialog.Show()`——由
                // `friend_memo_open_system` 打开独立备注窗（本系统已满 16 参，不能再加资源）
                friend.pending = Some(FriendPending::Memo(idx));
            }
        } else if act.is_email {
            if let Some(f) = friend.selected.and_then(|i| list.get(i)).cloned() {
                compose_mail.write(ComposeMail {
                    to: f.name.clone(),
                    message: None,
                    // C# `FriendDialog` 邮件钮 → `MailComposeLetterDialog.ComposeMail(Name)`
                    parcel: false,
                });
            }
        } else if act.is_whisper {
            if let Some(f) = friend.selected.and_then(|i| list.get(i)).cloned() {
                match friend_whisper_command(&f.name, f.online) {
                    Some(cmd) => {
                        chat.input_active = true;
                        chat.input_text = cmd;
                    }
                    None => {
                        chat.add_line(
                            "该玩家不在线".to_string(),
                            Color::srgb(1.0, 0.3, 0.3),
                            ChatChannel::System,
                        );
                    }
                }
            }
        }
    }
    for (e, inter, is_block) in &tabs {
        if edge(e, inter, &mut local.prev_inter) {
            let target = is_block; // 黑名单页签 → true；好友页签 → false
            if friend.blocked_tab != target {
                friend.blocked_tab = target;
                friend.selected = None;
                friend.page = 0;
            }
        }
    }
}

/// 页码标签同步（C# `PageNumberLabel.Text = (Page + 1) + " / " + maxPage`）。
/// 计数取**当前页签过滤后**的列表长度（与 C# `UpdateDisplay` 同口径）。
fn friend_page_label_system(
    friend: Res<FriendState>,
    mut labels: Query<&mut Text, With<FriendPageLabel>>,
) {
    let count = filter_friends(&friend.friends, friend.blocked_tab).len();
    let want = format!(
        "{} / {}",
        friend.page.min(friend_page_max(count).saturating_sub(1)) + 1,
        friend_page_max(count)
    );
    for mut t in &mut labels {
        if t.0 != want {
            t.0 = want.clone();
        }
    }
}

/// #2892 批D 单元①：`MemoButton.Click` → 打开独立备注窗（C# `MemoDialog.Show()`，
/// `FriendDialog.cs:551-566`：预填该好友现有备注并聚焦）。
/// 单列一个系统是因为 `friend_ui_system` 已到 Bevy 的 16 参上限。
fn friend_memo_open_system(
    mut friend: ResMut<FriendState>,
    mut memo: ResMut<crate::game::dialogs::memo::MemoState>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
) {
    let Some(FriendPending::Memo(idx)) = friend.pending else {
        return;
    };
    friend.pending = None;
    let Some(f) = friend.friends.get(idx).cloned() else {
        return;
    };
    memo.open = true;
    memo.target = Some(f.object_id as i32);
    if input.texts.len() <= crate::game::dialogs::memo::MEMO_INPUT_ID {
        input
            .texts
            .resize(crate::game::dialogs::memo::MEMO_INPUT_ID + 1, String::new());
    }
    input.texts[crate::game::dialogs::memo::MEMO_INPUT_ID] = f.memo.clone();
    input.active = Some(crate::game::dialogs::memo::MEMO_INPUT_ID);
    tracing::info!("👥 打开好友备注窗: {}", f.name);
}

/// #2892 批D：「添加好友」→ C# `MirInputBox`（`FriendDialog.cs:143-160`）：
/// 提示文案 `FriendEnterAddName`/`FriendEnterBlockName`（中文逐字取 `Chinese.json`），
/// OK 时由 `input_box.rs` 发 `C.AddFriend{Name, Blocked}`。
fn friend_add_open_system(
    mut friend: ResMut<FriendState>,
    mut box_state: ResMut<crate::game::dialogs::input_box::InputBoxState>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
) {
    let Some(blocked) = friend.add_request.take() else {
        return;
    };
    let title = if blocked {
        "请输入您想要屏蔽的人的名字。"
    } else {
        "请输入您想要添加的人的名字。"
    };
    crate::game::dialogs::input_box::open_input_box(
        &mut box_state,
        &mut input,
        crate::game::dialogs::input_box::InputPurpose::AddFriend { blocked },
        title,
    );
}

/// 消费服务端好友事件（网络层只广播 ServerEvent）
fn friend_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut friend: ResMut<FriendState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::FriendUpdated { entries } = ev {
            for e in entries {
                if let Some(existing) = friend
                    .friends
                    .iter_mut()
                    .find(|f| f.object_id == e.object_id)
                {
                    *existing = e.clone();
                } else {
                    friend.friends.push(e.clone());
                }
            }
        }
    }
}
/// 按页签过滤好友列表（false=好友 true=黑名单，C# _blockedTab）
pub fn filter_friends(friends: &[FriendEntry], blocked_tab: bool) -> Vec<FriendEntry> {
    friends
        .iter()
        .filter(|f| f.blocked == blocked_tab)
        .cloned()
        .collect()
}

/// 好友私聊命令（C# WhisperButton：离线返回 None）
pub fn friend_whisper_command(name: &str, online: bool) -> Option<String> {
    if !online {
        return None;
    }
    Some(format!("/w {} ", name))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 门禁（金标准 §3.2f）：好友列表必须是 **12 行 × 2 列**的格子 + 翻页条，逐项对齐 C#：
    /// `FriendDialog.Rows = new FriendRow[12]`、`FriendRow.Size = (115,17)`、
    /// `UpdateDisplay` 里 `Location = ((i%2)*115 + 16, 55 + (i/2)*22)`、
    /// `PageNumberLabel` @(87,216) 83x17、`PreviousButton`/`NextButton` @(70,218)/(171,218) 16x16。
    ///
    /// **阳性对照**：把 `FRIEND_ROW_COUNT` 改回 10、或把 `friend_row_pos` 改回旧的单列公式
    /// （`18, 40 + 20i`）→ 本测试立刻红（这正是金标准 A/B 抓到的那个缺陷）。
    #[test]
    fn friend_rows_match_csharp_grid() {
        assert_eq!(FRIEND_ROW_COUNT, 12, "C# FriendDialog.Rows 是 12 行");
        assert_eq!(FRIEND_ROW_SIZE, (115.0, 17.0), "C# FriendRow.Size");
        assert_eq!(friend_row_pos(0), (16.0, 55.0));
        assert_eq!(friend_row_pos(1), (131.0, 55.0), "第 2 列 x = 16 + 115");
        assert_eq!(friend_row_pos(2), (16.0, 77.0), "第 2 行 y = 55 + 22");
        // i=11 → i/2 = 5 → y = 55 + 5*22 = 165（第 6 行、第 2 列）
        assert_eq!(friend_row_pos(11), (131.0, 165.0), "最后一格（11 = 第 6 行第 2 列）");
    }

    /// 门禁：翻页条的位置/尺寸 + 翻页算式，逐项对齐 C#（`FriendDialog.cs:70-118`、`UpdateDisplay`）。
    #[test]
    fn friend_pager_matches_csharp() {
        assert_eq!(FRIEND_PAGE_LABEL_POS, (87.0, 216.0));
        assert_eq!(FRIEND_PAGE_LABEL_SIZE, (83.0, 17.0));
        assert_eq!(FRIEND_PREV_POS, (70.0, 218.0));
        assert_eq!(FRIEND_NEXT_POS, (171.0, 218.0));
        assert_eq!(FRIEND_PAGE_BTN_SIZE, (16.0, 16.0));
        assert_eq!(FRIEND_PREV_FRAMES, [240, 241, 242]);
        assert_eq!(FRIEND_NEXT_FRAMES, [243, 244, 245]);

        // C# `maxPage = count / Rows.Length + 1`（min 1）；`StartIndex = Rows.Length * Page`
        assert_eq!(friend_page_max(0), 1);
        assert_eq!(friend_page_max(1), 1);
        assert_eq!(friend_page_max(12), 2);
        assert_eq!(friend_page_max(13), 2);
        assert_eq!(friend_page_max(24), 3);
        assert_eq!(friend_page_start(0), 0);
        assert_eq!(friend_page_start(2), 24);
        assert_eq!(friend_row_index(24, 11), 35);

        // C# Previous：`Page--` 且夹到 0；Next：`Page++` 且夹到 `Count / Rows.Length`
        // （注意上界**不是** maxPage-1：整除时 `maxPage` 比它大 1）
        assert_eq!(friend_page_after(-1, 0, 30), 0, "第 0 页再按上一页仍是 0");
        assert_eq!(friend_page_after(1, 0, 30), 1);
        assert_eq!(friend_page_after(1, 2, 30), 2, "30/12 = 2 是上界");
        assert_eq!(friend_page_after(1, 2, 25), 2, "25/12 = 2");
        assert_eq!(friend_page_clamped(9, 5), 0, "列表只剩 5 条 ⇒ 只能停在第 0 页");
    }

    /// #2985 A1：好友面板文本必须用**自带 CJK 字形**的主字体。此前整面板走 Arial，
    /// 实机截图整片豆腐（标签 `□□`/`□□□`、行内 `bevy2char□□□□`）；列表行是动态写入
    /// （`text.0 = ...`），重排后同样不可能靠 Han 回退救回。修复前本测试 FAILED。
    #[test]
    fn friend_text_uses_cjk_capable_font() {
        use bevy::ecs::system::RunSystemOnce;

        // CI 无游戏资产（`Data/` 不入库）→ spawn 在取背景图处提前返回、一个文本都没有，
        // 断言会假红（本 PR 自己的 CI 就是这么红的）；按 `data_assets_present` 跳过。
        if !crate::resources::libraries::data_assets_present() {
            eprintln!("skip: 无 Data 资产（CI 只 checkout 仓库）");
            return;
        }
        let mut world = World::new();
        world.insert_resource(GameLibraries::default());
        world.insert_resource(Assets::<Image>::default());
        world.insert_resource(Assets::<Font>::default());
        world.insert_resource(UiCjkFont::default());
        world.run_system_once(spawn_friend).unwrap();

        let cjk = world.resource::<UiCjkFont>().0.clone();
        assert!(cjk.is_strong(), "CJK 字体应已被惰性加载");
        let mut n = 0usize;
        // 这些面板走 `spawn_label` → bevy_ui 的 `Text`（不是 `Text2d`），
        // 两者都用 `TextFont` 携带字体句柄，故只查后者即可全覆盖
        let mut q = world.query::<&TextFont>();
        for tf in q.iter(&world) {
            match &tf.font {
                FontSource::Handle(h) => {
                    assert_eq!(
                        *h, cjk,
                        "好友面板文本必须用自带 CJK 的主字体（Arial 会豆腐）"
                    );
                    n += 1;
                }
                // 非 Handle 源会被 `if let` 静默跳过（假绿口子）：这类实体同样
                // 渲染中文，出现即失败
                other => panic!("好友面板文本应为显式字体句柄，实得 {other:?}"),
            }
        }
        assert!(n > 0, "应至少 spawn 出若干文本实体");
    }

    #[test]
    fn friend_origin_is_csharp_center() {
        assert_eq!(
            crate::game::dialogs::center_origin(264.0, 272.0),
            (380.0, 248.0)
        );
    }

    #[test]
    fn whisper_command_online_offline() {
        assert_eq!(
            friend_whisper_command("Alice", true),
            Some("/w Alice ".to_string())
        );
        assert_eq!(friend_whisper_command("Alice", false), None);
    }

    #[test]
    fn filter_friends_by_tab() {
        let friends = vec![
            FriendEntry {
                object_id: 1,
                name: "a".into(),
                memo: String::new(),
                blocked: false,
                online: true,
            },
            FriendEntry {
                object_id: 2,
                name: "b".into(),
                memo: String::new(),
                blocked: true,
                online: false,
            },
        ];
        let ok = filter_friends(&friends, false);
        assert_eq!(ok.len(), 1);
        assert_eq!(ok[0].object_id, 1);
        let blk = filter_friends(&friends, true);
        assert_eq!(blk.len(), 1);
        assert_eq!(blk[0].object_id, 2);
    }

    /// #3260 回归：好友窗**没开**时不得清别的窗的文本焦点。
    ///
    /// 修复前 `if !open { … input.active = None; }` 每帧都执行 ⇒ 只要有别的窗开着输入框
    /// （实测：仓库密码 `MirInputBox`），焦点会被好友窗每帧清成 `None`，玩家/夹具打字全丢。
    /// 阳性对照：把 `local.was_open` 那道判断去掉（回到每帧清）→ 本测试 FAILED。
    #[test]
    fn friend_closed_does_not_steal_other_dialog_text_focus() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.init_resource::<FriendState>();
        world.init_resource::<DialogManager>(); // 好友窗默认不在栈上 = 关着
        world.init_resource::<crate::game::dialogs::text_input::TextInputState>();
        world.init_resource::<NetConnection>();
        world.init_resource::<ChatState>();
        world.init_resource::<bevy::ecs::message::Messages<ComposeMail>>();
        world.init_resource::<bevy::ecs::message::Messages<bevy::input::mouse::MouseWheel>>();
        // 别的窗（仓库密码框）聚焦着
        world
            .resource_mut::<crate::game::dialogs::text_input::TextInputState>()
            .active = Some(crate::game::dialogs::input_box::INPUT_FIELD_ID);

        world
            .run_system_once(friend_ui_system)
            .expect("friend_ui_system 应成功");

        assert_eq!(
            world
                .resource::<crate::game::dialogs::text_input::TextInputState>()
                .active,
            Some(crate::game::dialogs::input_box::INPUT_FIELD_ID),
            "好友窗关着时不得清别人的输入焦点"
        );
    }
}
