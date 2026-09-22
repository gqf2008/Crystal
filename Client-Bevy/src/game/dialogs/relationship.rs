// ============================================================================
// 关系/婚姻对话框（M49）
// 参考：C# RelationshipDialog + ServerRust social.rs 婚姻系统
// 网络：
//   C: MarriageRequest[target dotnet] / MarriageReply[bool] / ChangeMarriage(空)
//      DivorceRequest[partner dotnet] / DivorceReply[bool]
//   S: MarriageRequest[lover dotnet] / LoverUpdate[Name dotnet][Date i64][MapName dotnet][MarriedDays i16] / DivorceRequest[lover dotnet]
// bevy_ui 迁移（批 14）：面板 Prguse[170] @(280,80) 320x262，全节点化；
//   邀请弹窗 = C# MirMessageBox（Prguse[360] 原生 456x190 居中 @(284,289)，
//   Label(35,35)、Yes Title[206/207/208] (260,157)、No Title[210/211/212] (360,157)）
// ============================================================================

use bevy::prelude::*;

use crate::game::chat::ChatState;
use crate::game::dialogs::mail::ComposeMail;
use crate::game::dialogs::{AlwaysVisible, DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_close_button, spawn_container, spawn_icon_button, spawn_label,
    spawn_panel,
};

/// #2892 批B：面板精灵与 C# 原生尺寸（C# `RelationshipDialog.Index = 583; Library = Libraries.Prguse`）
pub const PANEL: (LibraryName, usize) = (LibraryName::Prguse, 583);
pub const PANEL_SIZE: (f32, f32) = (284.0, 194.0);
/// 关闭键 `Prguse2[360..362]` @(260,3)（`RelationshipDialog.cs:38-47`，无 `Size` → 原生 24x21）
pub const CLOSE_POS: (f32, f32) = (260.0, 3.0);

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
    pub message: String,
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
                marriage_invite_system,
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
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
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
        // C# 信息行 4 @(30,40/65/90/115)
        for (i, y) in [40.0, 65.0, 90.0, 115.0].into_iter().enumerate() {
            spawn_label(p, &cjk, "", 30.0, y, 12.0, Color::WHITE, 9).insert(RelationshipLine(i));
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
        // C# 五个操作按钮：切换/求婚/离婚/邮件/私聊 @ x=50/85/120/155/190, y=164。
        // #2775：Hint 取 C# `RelationshipDialog.cs:59/72/94/116/138`（精灵号与坐标一一对应）
        let buttons = [
            (
                50.0,
                610usize,
                611usize,
                612usize,
                RelationshipAction::Allow,
            ),
            (85.0, 600, 601, 602, RelationshipAction::Propose),
            (120.0, 616, 617, 618, RelationshipAction::Divorce),
            (155.0, 437, 438, 439, RelationshipAction::Mail),
            (190.0, 566, 567, 568, RelationshipAction::Whisper),
        ];
        for (x, normal, hover, pressed, action) in buttons {
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse, normal),
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse, hover),
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse, pressed),
            ) {
                let mut e = spawn_icon_button(p, n, h, pr, x, 164.0, 24.0, 22.0, 10);
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
        text.0 = match line.0 {
            0 => "关系（婚姻）".to_string(),
            1 => {
                if state.married {
                    format!(
                        "婚姻状态: 已婚（{}，{} 天）",
                        state.lover_name, state.married_days
                    )
                } else {
                    "婚姻状态: 未婚".to_string()
                }
            }
            2 => state.message.clone(),
            3 => {
                if state.married {
                    format!(
                        "配偶位置: {}",
                        if state.map_name.is_empty() {
                            "未知"
                        } else {
                            state.map_name.as_str()
                        }
                    )
                } else {
                    "输入目标名 → 求婚；已婚可离婚".to_string()
                }
            }
            _ => String::new(),
        };
    }
    for (e, inter) in &allow_btn {
        if edge(e, inter, &mut prev_inter) {
            net.send_packet(&mir2_shared::packets::client::misc::ChangeMarriage);
            state.message = "已切换求婚/结婚模式".to_string();
        }
    }

    for (e, inter) in &mail_btn {
        if edge(e, inter, &mut prev_inter) && !state.lover_name.is_empty() {
            compose_mail.write(ComposeMail {
                to: state.lover_name.clone(),
                message: None,
            });
            state.message = format!("写信给 {}", state.lover_name);
        }
    }

    for (e, inter) in &whisper_btn {
        if edge(e, inter, &mut prev_inter) && !state.lover_name.is_empty() {
            chat.input_active = true;
            chat.input_text = format!("/w {} ", state.lover_name);
            state.message = format!("私聊 {}", state.lover_name);
        }
    }

    for (e, inter) in &propose_btn {
        if edge(e, inter, &mut prev_inter) {
            let name = input.texts.get(13).cloned().unwrap_or_default();
            let name = name.trim().to_string();
            if !name.is_empty() && !state.married {
                net.send_packet(&crate::network::MarriageRequestWire {
                    target_name: name.clone(),
                });
                state.message = format!("已向 {} 求婚", name);
                tracing::info!("💍 求婚 → {}", name);
                input.texts[13].clear();
                input.active = None;
            }
        }
    }
    for (e, inter) in &divorce_btn {
        if edge(e, inter, &mut prev_inter) && state.married {
            // 服务端离婚流程：发起离婚请求 → 对方确认
            net.send_packet(&crate::network::DivorceRequestWire {
                partner_name: String::new(),
            });
            state.message = "已发起离婚请求".to_string();
            tracing::info!("💔 发起离婚");
        }
    }
}

/// 婚姻邀请弹窗：Yes/No → MarriageReply
fn marriage_invite_system(
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
    let has_invite = state.invite.is_some();
    for mut vis in widgets.iter_mut() {
        *vis = if has_invite {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (mut text, _) in &mut texts {
        text.0 = match state.invite.as_ref() {
            Some(name) => format!("{} 向你求婚！", name),
            None => String::new(),
        };
    }
    if state.invite.is_none() {
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
        net.send_packet(&mir2_shared::packets::client::misc::MarriageReply { accept_invite: a });
        tracing::info!("💍 婚姻邀请回复: accept={}", a);
        state.invite = None;
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
            ServerEvent::DivorceRequest => {
                relationship.message = "收到离婚请求".to_string();
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
    }
}
