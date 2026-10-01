// ============================================================================
// 关系/婚姻对话框（M49）
// 参考：C# RelationshipDialog + ServerRust social.rs 婚姻系统
// 网络：
//   C: MarriageRequest[target dotnet] / MarriageReply[bool] / ChangeMarriage(空)
//      DivorceRequest[partner dotnet] / DivorceReply[bool]
//   S: MarriageRequest[lover dotnet] / LoverUpdate[Name dotnet][Date i64][MapName dotnet][MarriedDays i16] / DivorceRequest[lover dotnet]
// bevy_ui 迁移（批 14）：面板 **`Prguse[583]`**（图头 284x194，C# `Index = 583`、
//   `Location = Center` → `center_origin(284,194)` = (370,287)），全节点化。
//   （旧注释写「Prguse[170] @(280,80) 320x262」是错的：`Prguse[170]` 图头 244x207，
//     且代码里从来没用过它 —— 2026-09-27 长尾窗核对时改正。）
//   邀请弹窗 = C# MirMessageBox（Prguse[360] 原生 456x190 居中 @(284,289)，
//   Label(35,35)、Yes Title[206/207/208] (260,157)、No Title[210/211/212] (360,157)）
// ============================================================================

use bevy::prelude::*;

use crate::game::chat::{ChatChannel, ChatState};
use crate::game::dialogs::mail::ComposeMail;
use crate::game::dialogs::{AlwaysVisible, DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_close_button, spawn_container, spawn_icon_button, spawn_image,
    spawn_label, spawn_panel,
};

/// #2892 批B：面板精灵与 C# 原生尺寸（C# `RelationshipDialog.Index = 583; Library = Libraries.Prguse`）
pub const PANEL: (LibraryName, usize) = (LibraryName::Prguse, 583);
pub const PANEL_SIZE: (f32, f32) = (284.0, 194.0);
/// 关闭键 `Prguse2[360..362]` @(260,3)（`RelationshipDialog.cs:38-47`，无 `Size` → 原生 24x21）
pub const CLOSE_POS: (f32, f32) = (260.0, 3.0);
/// 标题图 `Title[52]` @(18,8)（`RelationshipDialog.cs:30-36`）。
///
/// 2026-09-28 金标准 A/B（`tools/acceptance/csharp_golden/README.md` §3.2g）实测：
/// 该带（面板内 y 8..23）原版帧有 439 个亮像素、本端只有 13 —— C# 有这张标题图、
/// 本端整个控件漏了。图头 `Title[52]` = 109x15（C# 未写 `Size` ⇒ 原生尺寸）。
pub const TITLE_POS: (f32, f32) = (18.0, 8.0);
pub const TITLE_SIZE: (f32, f32) = (109.0, 15.0);
/// 五个操作按钮的**原生**精灵尺寸（`Prguse[610/600/616/437/566]` 图头 = 28x25）。
///
/// C# 五个 `MirButton` 都不写 `Size` ⇒ `MirControl.Size = Library.GetTrueSize(Index)`
/// = 28x25。本端旧值 24x22 会把精灵拉伸（A/B 逐像素看是插值色，且每颗钮多出
/// 32x25 的差异块）。
pub const ACTION_SIZE: (f32, f32) = (28.0, 25.0);
/// 五个操作按钮的 Y（C# `RelationshipDialog.cs:54/67/89/111/133` 都是 164）
pub const ACTION_Y: f32 = 164.0;
/// 五颗操作钮的（X, normal, hover, pressed）：`RelationshipDialog.cs:50-139`——
/// 600=求婚、610=切换婚配、616=离婚、437=邮件、566=私聊（Hint 见 `relationship_hint`）。
pub const ACTION_BUTTONS: [(f32, usize, usize, usize); 5] = [
    (50.0, 610, 611, 612),
    (85.0, 600, 601, 602),
    (120.0, 616, 617, 618),
    (155.0, 437, 438, 439),
    (190.0, 566, 567, 568),
];
/// `ACTION_BUTTONS` 的顺序（**判据用**：门禁按它钉住「哪颗钮在哪」，
/// 免得以后调换精灵号/坐标时无人发现）
const ACTION_ORDER: [RelationshipAction; 5] = [
    RelationshipAction::Allow,
    RelationshipAction::Propose,
    RelationshipAction::Divorce,
    RelationshipAction::Mail,
    RelationshipAction::Whisper,
];
/// C# 信息行 `MirLabel` 是 `Location`(左上) + `Size.(200,30)` + `DrawFormat.VerticalCenter`
/// ⇒ 文本**垂直居中在 30px 高的盒子里**，即文本中心 = `y + 15`。
///
/// §3.2dj：字号取 C# 的 **10F**（`RelationshipDialog.cs:170/182/194/206`），
/// `1pt = 4/3 px`（96 DPI）⇒ **13px**（本端此前写 12px，实机帧里字形带 11px，而原版 **14px**）。
/// 本端 `spawn_label` 是左上锚点、无垂直居中：13px 字体的字形高约 12px
/// ⇒ 顶边补偿 `(30 - 12) / 2 = 9`，保证文本中心仍落在 C# 的 `y + 15`。
pub const LINE_PAD_Y: f32 = 9.0;
/// C# 四行信息标签的 `ForeColour = Color.LightGray`（`RelationshipDialog.cs:166/178/190/202`）
/// = **#D3D3D3 / (211,211,211)**；实机帧核实：原版那四行的亮像素**全部恰为 (211,211,211)**、
/// 无一例外（719/719），而本端此前用 `Color::WHITE`。
pub const LINE_COLOUR: Color = Color::srgb(211.0 / 255.0, 211.0 / 255.0, 211.0 / 255.0);

/// 婚姻状态
#[derive(Resource, Default)]
pub struct RelationshipState {
    pub married: bool,
    /// 配偶名（#1329：LoverUpdate 全量）
    pub lover_name: String,
    /// 结婚日期（unix 秒）
    pub date: i64,
    /// 配偶当前地图标题
    pub map_name: String,
    /// 结婚天数
    pub married_days: i16,
    /// 收到结婚邀请（对方名字）
    pub invite: Option<String>,
    /// 收到离婚请求（对方名字）——C# `GameScene.cs:6212-6220` 的 YesNo 框；
    /// 与 `invite` 分开存，避免自动 e2e 里「看到 invite 就回 MarriageReply」那条路径误判。
    pub divorce_invite: Option<String>,
    pub message: String,
}

/// 关系确认框的两种语义（决定回 `MarriageReply` 还是 `DivorceReply`）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RelationshipPrompt {
    Marriage,
    Divorce,
}

/// 关系确认框正文（逐字取 C# 键值：`PlayerAskedForMarriage` / `PlayerRequestedDivorce`）
pub fn relationship_prompt_text(prompt: RelationshipPrompt, name: &str) -> String {
    match prompt {
        RelationshipPrompt::Marriage => format!("{name} 向你求婚。"),
        RelationshipPrompt::Divorce => format!("{name} 请求离婚。"),
    }
}

/// C# `Date < new DateTime(2000)` 对应的 unix 秒（2000-01-01T00:00:00Z）：
/// `RelationshipDialog.UpdateInterface` 用它把「刚结束的关系」与「已离婚」分成两支文案。
const RELATIONSHIP_EARLY_EPOCH: i64 = 946_684_800;

/// C# `RelationshipDialog.UpdateInterface:221` 的「离婚支」判据：`(LoverName == "") && (Date != default)`。
fn relationship_divorced_branch(state: &RelationshipState) -> bool {
    state.lover_name.is_empty() && state.date != 0
}

/// C# `DateTime.ToShortDateString()` 等价（中西文 culture 都是 `yyyy/M/d`、**月日不补零**；
/// 沙箱原版帧实测渲染 `0001/1/1`）。
///
/// unix 秒 → (y,m,d)：Howard Hinnant 的 `civil_from_days`，纯整数、无日期库依赖。
pub fn relationship_short_date(unix: i64) -> String {
    let z = unix.div_euclid(86_400) + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y}/{m}/{d}")
}

/// C# `RelationshipDialog.UpdateInterface:212-244` 的四行信息文本，**逐键照抄**
/// `Client/Localization/Chinese.json`（`LoverName` / `LoverDate` / `LoverLength` /
/// `LoverLocation{,Offline,Title}` / `MarriageDate` / `LengthDays` / `DivorcedDate` / `TimeSinceDays`）。
///
/// 抽成纯函数是为了单测能钉住四行 + 三种状态分支（已婚 / 未婚 / 已离），
/// 免得以后又被"关系（婚姻）"这类自造文案覆盖（2026-09-30 §3.2ce）。
pub fn relationship_line_text(i: usize, state: &RelationshipState) -> String {
    let divorced = relationship_divorced_branch(state);
    let early = state.date < RELATIONSHIP_EARLY_EPOCH;
    match i {
        0 => format!("伴侣：{}", state.lover_name),
        1 => {
            if divorced {
                if early {
                    "日期：".to_string()
                } else {
                    format!("离婚日期：{}", relationship_short_date(state.date))
                }
            } else if state.date == 0 {
                "结婚日期：".to_string()
            } else {
                format!("结婚日期：{}", relationship_short_date(state.date))
            }
        }
        2 => {
            if divorced {
                if early {
                    "持续时间：".to_string()
                } else {
                    format!("已过去：{}天", state.married_days)
                }
            } else {
                format!("持续：{}天", state.married_days)
            }
        }
        3 => {
            if divorced {
                "位置：".to_string()
            } else if state.map_name.is_empty() {
                "位置：离线".to_string()
            } else {
                format!("位置：{}", state.map_name)
            }
        }
        _ => String::new(),
    }
}

#[derive(Component)]
pub struct RelationshipWidget;

#[derive(Component)]
pub struct RelationshipClose;

#[derive(Component)]
pub struct RelationshipAllow;

#[derive(Component)]
pub struct RelationshipPropose;

#[derive(Component)]
pub struct RelationshipDivorce;

#[derive(Component)]
pub struct RelationshipMail;

#[derive(Component)]
pub struct RelationshipWhisper;

#[derive(Clone, Copy)]
enum RelationshipAction {
    Allow,
    Propose,
    Divorce,
    Mail,
    Whisper,
}

/// #2775：C# `RelationshipDialog.cs:59/72/94/116/138` 五个按钮的 Hint 文案。
/// （AllowButton 的 Hint 在 C# 里还会随婚配状态改写：已婚=允许/阻止结婚（`:237`）、
/// 未婚=允许/禁止传送（`:243`）——#2786 已接 `relationship_allow_hint_system` 动态改写。）
fn relationship_hint(action: RelationshipAction) -> &'static str {
    match action {
        RelationshipAction::Allow => "允许/阻止结婚",
        RelationshipAction::Propose => "请求结婚",
        RelationshipAction::Divorce => "请求离婚",
        RelationshipAction::Mail => "发送邮件给伴侣",
        RelationshipAction::Whisper => "发送悄悄话给伴侣",
    }
}

/// #2786：伴侣钮 `AllowButton` 的**动态** Hint（C# `RelationshipDialog.cs:237/243`）：
/// 已婚（`LoverName != ""`）→ `SwitchMarriage`「允许/阻止结婚」；
/// 未婚 → `AllowBlockRecall`「允许/禁止传送」。
pub fn allow_button_hint(lover_name: &str) -> &'static str {
    if lover_name.is_empty() {
        "允许/禁止传送"
    } else {
        "允许/阻止结婚"
    }
}

/// #2786：伴侣钮 Hint 随婚配状态刷新（独立系统：`relationship_ui_system` 参数已满）。
fn relationship_allow_hint_system(
    state: Res<RelationshipState>,
    mut allow: Query<&mut crate::ui::tooltip::UiHint, With<RelationshipAllow>>,
) {
    let text = allow_button_hint(&state.lover_name);
    for mut hint in &mut allow {
        if hint.text != text {
            hint.text = text.to_string();
        }
    }
}

#[derive(Component)]
pub struct RelationshipLine(usize);

/// 邀请弹窗
#[derive(Component)]
pub struct MarriageInviteWidget;

#[derive(Component)]
pub struct MarriageInviteText;

#[derive(Component)]
pub struct MarriageInviteYes;

#[derive(Component)]
pub struct MarriageInviteNo;

/// 目标名输入框（TextInput 13）
#[derive(Component)]
pub struct RelationshipTargetField;

pub struct RelationshipPlugin;

impl Plugin for RelationshipPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiCjkFont>();
        app.init_resource::<RelationshipState>();
        app.add_systems(
            Update,
            relationship_server_events.run_if(in_state(AppState::Game)),
        );
        app.add_systems(OnEnter(AppState::Game), spawn_relationship);
        app.add_systems(OnExit(AppState::Game), cleanup_relationship);
        app.add_systems(
            Update,
            (
                relationship_ui_system,
                relationship_prompt_system,
                // #2786：伴侣钮动态 Hint（已婚/未婚两态）
                relationship_allow_hint_system,
            )
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_relationship(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_relationship(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut cjk_font: ResMut<UiCjkFont>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        crate::ui::sprite_ui::ensure_ui_font(&mut fonts, &mut ui_font);
    }
    let font = ui_font.0.clone();
    let cjk = shared_cjk_font(&mut fonts, &mut cjk_font);

    // C# RelationshipDialog: Prguse[583] 原生 284x194，Location = Center。
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 583) else {
        return;
    };
    let (px, py) = crate::game::dialogs::center_origin(284.0, 194.0);
    let panel = spawn_panel(&mut commands, bg, px, py, PANEL_SIZE.0, PANEL_SIZE.1, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::Relationship), RelationshipWidget));

    commands.entity(panel).with_children(|p| {
        // 关闭 Prguse2[360/361/362] @(260,3)
        if let Some(mut btn) =
            spawn_close_button(p, &mut libs, &mut images, CLOSE_POS.0, CLOSE_POS.1, 10)
        {
            btn.insert(RelationshipClose);
        }
        // 标题图 Title[52] @(18,8)（`RelationshipDialog.cs:30-36` 的 `TitleLabel`）
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 52) {
            spawn_image(
                p,
                h,
                TITLE_POS.0,
                TITLE_POS.1,
                TITLE_SIZE.0,
                TITLE_SIZE.1,
                9,
            );
        }
        // C# 信息行 4 @(30,40/65/90/115)：Size (200,30) + VerticalCenter + `Font(...,10F)` + LightGray
        // ⇒ 文本中心 = y+15、字号 ≈13px、颜色 (211,211,211)；本端左上锚点、顶边补 `LINE_PAD_Y`
        for (i, y) in [40.0, 65.0, 90.0, 115.0].into_iter().enumerate() {
            spawn_label(p, &cjk, "", 30.0, y + LINE_PAD_Y, 13.0, LINE_COLOUR, 9)
                .insert(RelationshipLine(i));
        }
        // 目标名输入框（TextInput id 13）@(30,140)，保留简化版求婚目标输入。
        spawn_container(p, 30.0, 140.0, 160.0, 20.0, 10)
            .insert((
                RelationshipTargetField,
                BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                crate::game::dialogs::text_input::TextInputField(13),
                crate::game::dialogs::text_input::TextInputRect(400.0, 427.0, 160.0, 20.0),
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
                        font: FontSource::Handle(cjk.clone()),
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    ZIndex(11),
                    crate::game::dialogs::text_input::TextInputDisplay(13),
                ));
            });
        // C# 五个操作按钮：切换/求婚/离婚/邮件/私聊（`ACTION_BUTTONS`），y 恒 164。
        // #2775：Hint 取 C# `RelationshipDialog.cs:59/72/94/116/138`（精灵号与坐标一一对应）
        for (i, &(x, normal, hover, pressed)) in ACTION_BUTTONS.iter().enumerate() {
            let action = ACTION_ORDER[i];
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse, normal),
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse, hover),
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse, pressed),
            ) {
                let mut e =
                    spawn_icon_button(p, n, h, pr, x, ACTION_Y, ACTION_SIZE.0, ACTION_SIZE.1, 10);
                e.insert(crate::ui::tooltip::UiHint {
                    text: relationship_hint(action).to_string(),
                });
                match action {
                    RelationshipAction::Allow => {
                        e.insert(RelationshipAllow);
                    }
                    RelationshipAction::Propose => {
                        e.insert(RelationshipPropose);
                    }
                    RelationshipAction::Divorce => {
                        e.insert(RelationshipDivorce);
                    }
                    RelationshipAction::Mail => {
                        e.insert(RelationshipMail);
                    }
                    RelationshipAction::Whisper => {
                        e.insert(RelationshipWhisper);
                    }
                }
            }
        }
    });

    // 婚姻邀请弹窗（C# MirMessageBox：Prguse[360] 原生 456x190 居中 @(284,289)）
    let (bx, by) = (284.0, 289.0);
    if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 360) {
        let inv = spawn_panel(&mut commands, h, bx, by, 456.0, 190.0, 45);
        commands.entity(inv).insert((
            DialogRoot(DialogKind::Relationship),
            // 独立弹窗不随 Relationship 开关门控；挂 DialogRoot 仅为 OnExit 时随婚姻窗口一起清理
            AlwaysVisible,
            MarriageInviteWidget,
        ));
        commands.entity(inv).with_children(|ip| {
            // Label（C# (35,35)，390x110）
            spawn_label(ip, &cjk, "", 35.0, 35.0, 12.0, Color::WHITE, 9).insert(MarriageInviteText);
            // Yes Title[206/207/208]（C# (260,157)）
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
            ) {
                spawn_icon_button(ip, n, h, pr, 260.0, 157.0, 76.0, 25.0, 10)
                    .insert(MarriageInviteYes);
            }
            // No Title[210/211/212]（C# (360,157)）
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 210),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 211),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 212),
            ) {
                spawn_icon_button(ip, n, h, pr, 360.0, 157.0, 76.0, 25.0, 10)
                    .insert(MarriageInviteNo);
            }
        });
    }
}

/// 显隐 + 渲染 + 求婚/离婚
#[allow(clippy::too_many_arguments)]
fn relationship_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<RelationshipState>,
    net: Res<NetConnection>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    mut compose_mail: MessageWriter<ComposeMail>,
    mut chat: ResMut<ChatState>,
    close: Query<(Entity, &Interaction), With<RelationshipClose>>,
    allow_btn: Query<(Entity, &Interaction), With<RelationshipAllow>>,
    propose_btn: Query<(Entity, &Interaction), With<RelationshipPropose>>,
    divorce_btn: Query<(Entity, &Interaction), With<RelationshipDivorce>>,
    mail_btn: Query<(Entity, &Interaction), With<RelationshipMail>>,
    whisper_btn: Query<(Entity, &Interaction), With<RelationshipWhisper>>,
    mut widgets: Query<&mut Visibility, With<RelationshipWidget>>,
    mut lines: Query<(&mut Text, &RelationshipLine)>,
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
    let open = mgr.is_open(DialogKind::Relationship);
    for mut vis in widgets.iter_mut() {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        return;
    }
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::Relationship);
        }
    }
    for (mut text, line) in &mut lines {
        // C# `UpdateInterface` 的四行（文案/分支见 `relationship_line_text`）
        text.0 = relationship_line_text(line.0, &state);
    }
    for (e, inter) in &allow_btn {
        if edge(e, inter, &mut prev_inter) {
            // C# `RelationshipDialog.cs:61`：只发包，不提示
            net.send_packet(&mir2_shared::packets::client::misc::ChangeMarriage);
        }
    }

    for (e, inter) in &mail_btn {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        // C# `RelationshipDialog.cs:120-122`：未婚 → 系统聊天提示后 return
        if state.lover_name.is_empty() {
            chat.add_line(
                "你尚未结婚。".to_string(),
                Color::srgb(1.0, 0.3, 0.3),
                ChatChannel::System,
            );
            continue;
        }
        compose_mail.write(ComposeMail {
            to: state.lover_name.clone(),
            message: None,
            // C# `RelationshipDialog.cs:126` → 写信窗
            parcel: false,
        });
    }

    for (e, inter) in &whisper_btn {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        // C# `RelationshipDialog.cs:140-152`：未婚 → `YouAreNotMarried`；名字有但 `MapName == ""` → `LoverIsNotOnline`
        if state.lover_name.is_empty() {
            chat.add_line(
                "你尚未结婚。".to_string(),
                Color::srgb(1.0, 0.3, 0.3),
                ChatChannel::System,
            );
        } else if state.map_name.is_empty() {
            chat.add_line(
                "伴侣未在线".to_string(),
                Color::srgb(1.0, 0.3, 0.3),
                ChatChannel::System,
            );
        } else {
            chat.input_active = true;
            chat.input_text = format!("/w {} ", state.lover_name);
        }
    }

    for (e, inter) in &propose_btn {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        // C# `RelationshipDialog.cs:76-80`：已婚 → `YouAreAlreadyMarried` 后 return
        if !state.lover_name.is_empty() {
            chat.add_line(
                "你已经结婚了。".to_string(),
                Color::srgb(1.0, 0.3, 0.3),
                ChatChannel::System,
            );
            continue;
        }
        // 本端协议差异：`ServerRust` 的 `MarriageRequest` 带 `target_name`
        // （C# `C.MarriageRequest` 无字段、由服务端定目标）⇒ 保留面板内的目标名输入框。
        let name = input.texts.get(13).cloned().unwrap_or_default();
        let name = name.trim().to_string();
        if !name.is_empty() {
            net.send_packet(&crate::network::MarriageRequestWire {
                target_name: name.clone(),
            });
            tracing::info!("💍 求婚 → {}", name);
            input.texts[13].clear();
            input.active = None;
        }
    }
    for (e, inter) in &divorce_btn {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        // C# `RelationshipDialog.cs:98-102`：未婚 → `YouAreNotMarried` 后 return
        if state.lover_name.is_empty() {
            chat.add_line(
                "你尚未结婚。".to_string(),
                Color::srgb(1.0, 0.3, 0.3),
                ChatChannel::System,
            );
            continue;
        }
        // 服务端离婚流程：发起离婚请求 → 对方确认
        net.send_packet(&crate::network::DivorceRequestWire {
            partner_name: String::new(),
        });
        tracing::info!("💔 发起离婚");
    }
}

/// 关系确认弹窗（求婚 / 离婚共用同一块 `Prguse[360]` 框）：Yes/No → `MarriageReply` 或 `DivorceReply`。
///
/// C# 两处都是 `MirMessageBox(…, YesNo)`：
/// * 求婚 `GameScene.cs:6204`（`PlayerAskedForMarriage` = 「{0} 向你求婚。」）→ `C.MarriageReply`
/// * 离婚 `GameScene.cs:6212-6220`（`PlayerRequestedDivorce` = 「{0} 请求离婚。」）→ `C.DivorceReply`
fn relationship_prompt_system(
    mut state: ResMut<RelationshipState>,
    net: Res<NetConnection>,
    yes: Query<(Entity, &Interaction), With<MarriageInviteYes>>,
    no: Query<(Entity, &Interaction), With<MarriageInviteNo>>,
    mut widgets: Query<&mut Visibility, With<MarriageInviteWidget>>,
    mut texts: Query<(&mut Text, &MarriageInviteText)>,
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
    // 两个提示同屏时以「求婚」优先（C# 里两者各自弹独立消息框；本端复用一块框）
    let prompt: Option<(RelationshipPrompt, String)> = match (&state.invite, &state.divorce_invite)
    {
        (Some(n), _) => Some((RelationshipPrompt::Marriage, n.clone())),
        (None, Some(n)) => Some((RelationshipPrompt::Divorce, n.clone())),
        (None, None) => None,
    };
    for mut vis in widgets.iter_mut() {
        *vis = if prompt.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (mut text, _) in &mut texts {
        text.0 = match &prompt {
            Some((kind, name)) => relationship_prompt_text(*kind, name),
            None => String::new(),
        };
    }
    let Some((kind, _)) = prompt else {
        return;
    };
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
        match kind {
            RelationshipPrompt::Marriage => {
                net.send_packet(&mir2_shared::packets::client::misc::MarriageReply {
                    accept_invite: a,
                });
                tracing::info!("💍 婚姻邀请回复: accept={}", a);
                state.invite = None;
            }
            RelationshipPrompt::Divorce => {
                net.send_packet(&mir2_shared::packets::client::misc::DivorceReply {
                    accept_invite: a,
                });
                tracing::info!("💔 离婚请求回复: accept={}", a);
                state.divorce_invite = None;
            }
        }
    }
}

/// 消费服务端婚姻/关系事件（网络层只广播 ServerEvent）
fn relationship_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut relationship: ResMut<RelationshipState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::MarriageInvite { name } => {
                relationship.invite = Some(name.clone());
                relationship.message = format!("收到 {} 的求婚", name);
            }
            ServerEvent::LoverUpdate {
                lover_name,
                date,
                map_name,
                married_days,
            } => {
                relationship.lover_name = lover_name.clone();
                relationship.date = *date;
                relationship.map_name = map_name.clone();
                relationship.married_days = *married_days;
                relationship.married = !lover_name.is_empty();
                relationship.message = if relationship.married {
                    format!("婚姻关系已建立：{}（结婚 {} 天）", lover_name, married_days)
                } else {
                    "婚姻关系已解除".to_string()
                };
            }
            ServerEvent::DivorceRequest { name } => {
                // C# 一定带名字（弹「{0} 请求离婚。」的 YesNo 框）；空名只降级成聊天提示，不弹框。
                if name.is_empty() {
                    relationship.message = "收到离婚请求".to_string();
                } else {
                    relationship.divorce_invite = Some(name.clone());
                    relationship.message = format!("收到 {name} 的离婚请求");
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2786：伴侣钮两态 Hint（C# `RelationshipDialog.cs:237/243`）
    #[test]
    fn allow_button_hint_switches_with_marriage_state() {
        assert_eq!(
            allow_button_hint("老婆大人"),
            "允许/阻止结婚",
            "已婚（LoverName 非空）→ SwitchMarriage"
        );
        assert_eq!(
            allow_button_hint(""),
            "允许/禁止传送",
            "未婚 → AllowBlockRecall"
        );
        // 静态默认（关系表里的 Allow 项）保持已婚文案：未收到 LoverUpdate 前与 C# 构造默认一致
        assert_eq!(
            relationship_hint(RelationshipAction::Allow),
            "允许/阻止结婚"
        );
    }

    #[test]
    fn relationship_layout_matches_csharp() {
        assert_eq!(
            crate::game::dialogs::center_origin(284.0, 194.0),
            (370.0, 287.0)
        );
        // 面板 `Prguse[583]` 图头 284x194（C# `Index = 583` 无 Size ⇒ 原生）
        assert_eq!(PANEL_SIZE, (284.0, 194.0));
    }

    /// 2026-09-28 金标准 A/B（README §3.2g）捞出的两处真缺口：
    /// ① C# `TitleLabel`（`Title[52]` @(18,8)）整张漏画；② 五颗操作钮写死 24x22
    /// 而美术原生是 28x25（`Prguse[610/600/616/437/566]`，C# 不写 `Size`）。
    #[test]
    fn relationship_title_and_action_buttons_match_csharp() {
        assert_eq!(TITLE_POS, (18.0, 8.0), "C# `RelationshipDialog.cs:30-36`");
        assert_eq!(
            TITLE_SIZE,
            (109.0, 15.0),
            "`Title[52]` 图头（libextract 实测）"
        );
        assert_eq!(
            ACTION_SIZE,
            (28.0, 25.0),
            "`Prguse[610]` 图头；旧值 24x22 会拉伸精灵"
        );
        assert_eq!(ACTION_Y, 164.0);
        // 五颗钮的 X + 精灵号（C# `:50-139`）；顺序 = 切换/求婚/离婚/邮件/私聊
        assert_eq!(
            ACTION_BUTTONS,
            [
                (50.0, 610, 611, 612),
                (85.0, 600, 601, 602),
                (120.0, 616, 617, 618),
                (155.0, 437, 438, 439),
                (190.0, 566, 567, 568),
            ]
        );
        assert_eq!(ACTION_ORDER.len(), ACTION_BUTTONS.len());
    }

    /// C# 四行信息是 `Location` + `Size(200,30)` + `VerticalCenter` + `Font(...,10F)` ⇒ 文本中心 = `y + 15`；
    /// 本端左上锚点 + **13px** 字体（字形高约 12px）⇒ 顶边补 `(30 - 12) / 2 = 9`。
    #[test]
    fn relationship_line_vertical_centering_matches_csharp() {
        assert_eq!(LINE_PAD_Y, 9.0);
        // 逐行中心（本端）≈ C# 的 y + 15；容差 ±1px（字体行高取整）
        for y in [40.0, 65.0, 90.0, 115.0] {
            let ours_center = y + LINE_PAD_Y + 12.0 / 2.0;
            assert!(
                (ours_center - (y + 15.0)).abs() <= 1.0,
                "y={y}: 本端中心 {ours_center} vs C# {}",
                y + 15.0
            );
        }
        // 颜色：C# `Color.LightGray` = (211,211,211)（实机帧里原版四行亮像素全是这个值）
        assert_eq!(
            LINE_COLOUR.to_srgba().to_u8_array()[0],
            211,
            "四行文字色 = C# Color.LightGray（211,211,211）"
        );
    }

    /// 2026-09-30（§3.2ce）：四行文案必须**逐字**等于 C# `UpdateInterface` 用的
    /// `Client/Localization/Chinese.json` 键值——旧实现是自造的「关系（婚姻）/婚姻状态/输入目标名」。
    #[test]
    fn relationship_lines_match_csharp_localization() {
        // 未婚：名字空、无日期、无地图
        let single = RelationshipState::default();
        assert_eq!(relationship_line_text(0, &single), "伴侣：");
        assert_eq!(relationship_line_text(1, &single), "结婚日期：");
        assert_eq!(relationship_line_text(2, &single), "持续：0天");
        assert_eq!(relationship_line_text(3, &single), "位置：离线");

        // 已婚：名字 + 日期 + 天数 + 在线地图
        let married = RelationshipState {
            married: true,
            lover_name: "老婆大人".to_string(),
            date: 1_700_000_000, // 2023/11/14
            map_name: "比奇省".to_string(),
            married_days: 12,
            ..Default::default()
        };
        assert_eq!(relationship_line_text(0, &married), "伴侣：老婆大人");
        assert_eq!(
            relationship_line_text(1, &married),
            "结婚日期：2023/11/14",
            "C# `MarriageDate` = 「结婚日期：{{0}}」，日期取 ToShortDateString（月日不补零）"
        );
        assert_eq!(relationship_line_text(2, &married), "持续：12天");
        assert_eq!(relationship_line_text(3, &married), "位置：比奇省");

        // 已婚但配偶离线
        let offline = RelationshipState {
            married: true,
            lover_name: "老婆大人".to_string(),
            date: 1_700_000_000,
            map_name: String::new(),
            married_days: 12,
            ..Default::default()
        };
        assert_eq!(relationship_line_text(3, &offline), "位置：离线");

        // 关系刚结束（名字空 + 早于 2000 的日期）→ `LoverDate`/`LoverLength`/`LoverLocationTitle`
        let early = RelationshipState {
            date: 1, // 1970/1/1
            ..Default::default()
        };
        assert!(relationship_divorced_branch(&early));
        assert_eq!(relationship_line_text(1, &early), "日期：");
        assert_eq!(relationship_line_text(2, &early), "持续时间：");
        assert_eq!(relationship_line_text(3, &early), "位置：");

        // 已离婚（名字空 + 2000 之后）→ `DivorcedDate`/`TimeSinceDays`
        let divorced = RelationshipState {
            date: 1_700_000_000,
            married_days: 30,
            ..Default::default()
        };
        assert_eq!(relationship_line_text(1, &divorced), "离婚日期：2023/11/14");
        assert_eq!(relationship_line_text(2, &divorced), "已过去：30天");
    }

    /// `ToShortDateString` 等价的纯整数换算：月/日不补零，闰年/世纪边界不能漂。
    #[test]
    fn relationship_short_date_matches_csharp_to_short_date_string() {
        assert_eq!(relationship_short_date(0), "1970/1/1");
        assert_eq!(relationship_short_date(1_700_000_000), "2023/11/14");
        // 2000-02-29（闰年 + 世纪闰）与 2024-02-29
        assert_eq!(relationship_short_date(951_782_400), "2000/2/29");
        assert_eq!(relationship_short_date(1_709_164_800), "2024/2/29");
        // 2000-01-01 00:00:00Z = 早/晚分界
        assert_eq!(
            relationship_short_date(RELATIONSHIP_EARLY_EPOCH),
            "2000/1/1"
        );
        assert_eq!(
            relationship_short_date(RELATIONSHIP_EARLY_EPOCH - 86_400),
            "1999/12/31"
        );
    }

    /// 2026-09-30（§3.2cf）：离婚请求确认框的文案（C# `GameScene.cs:6204/6214` 两个键）。
    #[test]
    fn relationship_prompt_text_matches_csharp_keys() {
        assert_eq!(
            relationship_prompt_text(RelationshipPrompt::Marriage, "bevychar"),
            "bevychar 向你求婚。"
        );
        assert_eq!(
            relationship_prompt_text(RelationshipPrompt::Divorce, "bevychar"),
            "bevychar 请求离婚。"
        );
    }

    /// 收到 `S.DivorceRequest{Name}` → 记下待确认的离婚请求（并保留含「离婚请求」字样的状态文案，
    /// 自动 e2e `[MARRYACC]` 阶段 2 就是按这条文案触发 `DivorceReply` 的）；
    /// 空名（历史/异常形状）→ 只提示、不弹框。
    #[test]
    fn divorce_request_event_sets_prompt_and_keeps_e2e_marker() {
        use crate::network::server_event::ServerEvent;
        use bevy::ecs::system::RunSystemOnce;

        // `run_system_once` 每次都新建系统实例（`MessageReader` 游标从 0 起）⇒ 两种输入各起一个新 App，
        // 否则第二次会把第一条消息再读一遍（本轮就踩了这个，断言在"空名"那条挂）。
        fn run(name: &str) -> (Option<String>, String) {
            let mut app = App::new();
            app.add_message::<ServerEvent>();
            app.init_resource::<RelationshipState>();
            app.add_systems(Update, relationship_server_events);
            app.world_mut()
                .resource_mut::<bevy::ecs::message::Messages<ServerEvent>>()
                .write(ServerEvent::DivorceRequest {
                    name: name.to_string(),
                });
            app.world_mut()
                .run_system_once(relationship_server_events)
                .expect("系统应成功");
            let st = app.world().resource::<RelationshipState>();
            (st.divorce_invite.clone(), st.message.clone())
        }

        let (prompt, message) = run("bevychar");
        assert_eq!(prompt.as_deref(), Some("bevychar"));
        assert!(
            message.contains("离婚请求"),
            "[MARRYACC] 阶段 2 依赖这条文案：{}",
            message
        );

        // 空名 → 不弹框
        let (prompt, _) = run("");
        assert!(prompt.is_none(), "空名不应弹确认框");
    }
}
