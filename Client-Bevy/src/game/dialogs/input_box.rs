// ============================================================================
// 通用输入框（游戏内 C# `MirInputBox`，`Client/MirControls/MirInputBox.cs`）
//
// 布局基准（逐项对齐）：
//   - 面板 `Prguse[660]`（原生 288x156）@ 居中 = (368,306)（`MirInputBox.cs:22-25`）
//     `Modal = true`（世界输入锁）、`Movable = false`
//   - `CaptionLabel` @(25,25) 235x40（WordBreak，`:27-34`）
//   - `InputTextBox` @(23,86) 240x19，`Border=true` `BorderColour=Lime`、MaxLength 50（`:36-45`）
//   - `OKButton` `Title[200/201/202]` @(60,123)、`CancelButton` `Title[203/204/205]` @(160,123)（`:46-63`）
//   - Enter → OK、Esc → Cancel（`:64-76`）
//
// 用途（C# 服务端发起式取名）：
//   - `S.GuildNameRequest` → 输入公会名 → `C.GuildNameReturn`（`GameScene.cs:5772-5800`）
//   - `S.GuildRequestWar`  → 输入宣战目标公会名 → `C.GuildWarReturn`（`GameScene.cs:5784-5802`）
// ============================================================================

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use crate::game::dialogs::text_input::{
    TextInputDisplay, TextInputField, TextInputRect, TextInputState, TextInputSubmit,
};
use crate::game::dialogs::{
    sync_dialog_state, DialogKind, DialogManager, DialogRoot, NotDraggable,
};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont};
use crate::ui::theme::{load_lib_image, spawn_icon_button, spawn_label, spawn_panel};

/// 面板 `Prguse[660]`（原生 288x156）
pub const PANEL_INDEX: usize = 660;
pub const PANEL_W: f32 = 288.0;
pub const PANEL_H: f32 = 156.0;
/// C# `Location = ((ScreenWidth-W)/2, (ScreenHeight-H)/2)` = (368,306)
pub const PANEL_ORIGIN: (f32, f32) = (368.0, 306.0);

/// `CaptionLabel` @(25,25) 235x40
pub const CAPTION_POS: (f32, f32) = (25.0, 25.0);
pub const CAPTION_SIZE: (f32, f32) = (235.0, 40.0);

/// `InputTextBox` @(23,86) 240x19（`Border=true`、`BorderColour=Lime`、MaxLength=50）
pub const INPUT_POS: (f32, f32) = (23.0, 86.0);
pub const INPUT_SIZE: (f32, f32) = (240.0, 19.0);
pub const INPUT_MAX_LEN: usize = 50;

/// `OKButton` `Title[200..202]` @(60,123)、`CancelButton` `Title[203..205]` @(160,123)
pub const OK_POS: (f32, f32) = (60.0, 123.0);
pub const CANCEL_POS: (f32, f32) = (160.0, 123.0);
pub const OK_FRAMES: (usize, usize, usize) = (200, 201, 202);
pub const CANCEL_FRAMES: (usize, usize, usize) = (203, 204, 205);

/// 输入框 id（`TextInputState` 槽位；与 `text_input.rs` 的字段编号空间共用）
pub const INPUT_FIELD_ID: usize = 40;

/// 输入框用途（决定 OK 时回哪个包）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputPurpose {
    #[default]
    None,
    /// 宣战目标公会名 → `C.GuildWarReturn{Name}`
    GuildWarReturn,
    /// 公会名 → `C.GuildNameReturn{Name}`
    GuildNameReturn,
}

/// 输入框状态（C# 每次 `new MirInputBox(message)` 一个新窗口；本端复用同一实体）
#[derive(Resource, Default)]
pub struct InputBoxState {
    pub open: bool,
    pub title: String,
    pub purpose: InputPurpose,
}

#[derive(Component)]
pub struct InputBoxRoot;

#[derive(Component)]
pub struct InputBoxCaption;

#[derive(Component)]
pub struct InputBoxOk;

#[derive(Component)]
pub struct InputBoxCancel;

pub struct InputBoxPlugin;

impl Plugin for InputBoxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputBoxState>();
        app.add_systems(OnEnter(AppState::Game), spawn_input_box);
        app.add_systems(
            Update,
            (input_box_open_system, input_box_ui_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn spawn_input_box(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut cjk_font: ResMut<UiCjkFont>,
) {
    libs.0.ensure_initialized();
    let font = shared_cjk_font(&mut fonts, &mut cjk_font);

    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, PANEL_INDEX) else {
        return;
    };
    let (ox, oy) = PANEL_ORIGIN;
    let panel = spawn_panel(&mut commands, bg, ox, oy, PANEL_W, PANEL_H, 60);
    commands.entity(panel).insert((
        DialogRoot(DialogKind::InputBox),
        InputBoxRoot,
        // C# `MirInputBox.Movable = false`
        NotDraggable,
    ));

    let input_abs = (ox + INPUT_POS.0, oy + INPUT_POS.1);
    commands.entity(panel).with_children(|p| {
        // 标题（C# `CaptionLabel` 自动换行；文案短，单行即可）
        spawn_label(
            p,
            &font,
            "",
            CAPTION_POS.0,
            CAPTION_POS.1,
            12.0,
            Color::WHITE,
            9,
        )
        .insert(InputBoxCaption);
        // 输入框：`Border = true`、`BorderColour = Lime`（C# `:37-38`）
        p.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(INPUT_POS.0 - 1.0),
                top: Val::Px(INPUT_POS.1 - 1.0),
                width: Val::Px(INPUT_SIZE.0 + 2.0),
                height: Val::Px(INPUT_SIZE.1 + 2.0),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.85)),
            // C# `MirTextBox.Border = true; BorderColour = Color.Lime`
            BorderColor::all(Color::srgb(0.0, 1.0, 0.0)),
            ZIndex(10),
            TextInputField(INPUT_FIELD_ID),
            TextInputRect(input_abs.0, input_abs.1, INPUT_SIZE.0, INPUT_SIZE.1),
        ))
        .with_children(|ic| {
            ic.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(3.0),
                    top: Val::Px(1.0),
                    ..default()
                },
                Text::new(String::new()),
                TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                ZIndex(11),
                TextInputDisplay(INPUT_FIELD_ID),
            ));
        });
        // OK / Cancel（`Title[200..205]`）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, OK_FRAMES.0),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, OK_FRAMES.1),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, OK_FRAMES.2),
        ) {
            spawn_icon_button(p, n, h, pr, OK_POS.0, OK_POS.1, 76.0, 25.0, 10).insert(InputBoxOk);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, CANCEL_FRAMES.0),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, CANCEL_FRAMES.1),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, CANCEL_FRAMES.2),
        ) {
            spawn_icon_button(p, n, h, pr, CANCEL_POS.0, CANCEL_POS.1, 76.0, 25.0, 10)
                .insert(InputBoxCancel);
        }
    });
}

/// 服务端发起式取名 → 打开输入框（C# `GameScene.GuildNameRequest` / `GuildRequestWar`）
fn input_box_open_system(
    mut state: ResMut<InputBoxState>,
    mut input: ResMut<TextInputState>,
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
) {
    for ev in events.read() {
        let (purpose, title) = match ev {
            crate::network::server_event::ServerEvent::GuildNameRequested => (
                InputPurpose::GuildNameReturn,
                "请输入公会名称，长度必须为 3~20 个字符。",
            ),
            crate::network::server_event::ServerEvent::TerritoryWar { .. } => {
                (InputPurpose::GuildWarReturn, "请输入你想宣战的公会名称。")
            }
            _ => continue,
        };
        state.open = true;
        state.purpose = purpose;
        state.title = title.to_string();
        if input.texts.len() <= INPUT_FIELD_ID {
            input.texts.resize(INPUT_FIELD_ID + 1, String::new());
        }
        input.texts[INPUT_FIELD_ID].clear();
        input.active = Some(INPUT_FIELD_ID);
        tracing::info!("⌨️ [INPUTBOX] 打开输入框：{title}");
    }
}

/// 显隐 / 标题 / OK / Cancel / Enter / Esc
#[allow(clippy::too_many_arguments)]
fn input_box_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<InputBoxState>,
    input: Res<TextInputState>,
    net: Res<NetConnection>,
    mut keys: MessageReader<KeyboardInput>,
    ok: Query<&Interaction, (With<InputBoxOk>, Without<InputBoxCancel>)>,
    cancel: Query<&Interaction, (With<InputBoxCancel>, Without<InputBoxOk>)>,
    mut captions: Query<&mut Text, With<InputBoxCaption>>,
    mut submits: MessageReader<TextInputSubmit>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    // 状态 → 管理栈（Modal：打开即屏蔽世界输入；C# `MirInputBox.Modal = true`）
    sync_dialog_state(&mut mgr, DialogKind::InputBox, state.open);
    if !state.open {
        return;
    }
    for mut text in &mut captions {
        if text.0 != state.title {
            text.0 = state.title.clone();
        }
    }

    let mut confirm = false;
    let mut dismiss = false;
    for inter in &ok {
        if *inter == Interaction::Pressed {
            confirm = true;
        }
    }
    for inter in &cancel {
        if *inter == Interaction::Pressed {
            dismiss = true;
        }
    }
    let _ = &mut prev_inter;
    // Enter / Esc（C# `MirInputBox_KeyPress` `:64-76`）
    for ev in keys.read() {
        if !ev.state.is_pressed() {
            continue;
        }
        match &ev.logical_key {
            Key::Enter => confirm = true,
            Key::Escape => dismiss = true,
            _ => {}
        }
    }
    for TextInputSubmit(id) in submits.read() {
        if *id == INPUT_FIELD_ID {
            confirm = true;
        }
    }

    if confirm {
        let body: String = input
            .texts
            .get(INPUT_FIELD_ID)
            .cloned()
            .unwrap_or_default()
            // C# `InputTextBox.MaxLength = 50`（`MirInputBox.cs:43`）
            .chars()
            .take(INPUT_MAX_LEN)
            .collect();
        match state.purpose {
            InputPurpose::GuildWarReturn => {
                net.send_packet(&mir2_shared::packets::client::guild::GuildWarReturn {
                    guild_name: body.clone(),
                });
                tracing::info!("⌨️ [INPUTBOX] C.GuildWarReturn guild={body}");
            }
            InputPurpose::GuildNameReturn => {
                net.send_packet(&mir2_shared::packets::client::guild::GuildNameReturn {
                    name: body.clone(),
                });
                tracing::info!("⌨️ [INPUTBOX] C.GuildNameReturn name={body}");
            }
            InputPurpose::None => {}
        }
        dismiss = true;
    }
    if dismiss {
        state.open = false;
        state.purpose = InputPurpose::None;
        sync_dialog_state(&mut mgr, DialogKind::InputBox, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::server_event::ServerEvent;

    /// C# `MirInputBox`（`Client/MirControls/MirInputBox.cs:22-63`）逐项几何。
    #[test]
    fn layout_matches_csharp_mir_input_box() {
        assert_eq!(
            (PANEL_W, PANEL_H),
            (288.0, 156.0),
            "Prguse[660] 原生 288x156"
        );
        assert_eq!(PANEL_ORIGIN, (368.0, 306.0), "(1024-288)/2, (768-156)/2");
        assert_eq!(CAPTION_POS, (25.0, 25.0));
        assert_eq!(CAPTION_SIZE, (235.0, 40.0));
        assert_eq!(INPUT_POS, (23.0, 86.0));
        assert_eq!(INPUT_SIZE, (240.0, 19.0));
        assert_eq!(OK_POS, (60.0, 123.0));
        assert_eq!(CANCEL_POS, (160.0, 123.0));
        assert_eq!(OK_FRAMES, (200, 201, 202));
        assert_eq!(CANCEL_FRAMES, (203, 204, 205));
        assert_eq!(INPUT_MAX_LEN, 50, "C# `MaxLength = 50`");
    }

    fn app_with_ui_system() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<DialogManager>();
        app.init_resource::<InputBoxState>();
        app.init_resource::<TextInputState>();
        app.insert_resource(NetConnection::default());
        app.add_message::<KeyboardInput>();
        app.add_message::<TextInputSubmit>();
        app.add_systems(Update, input_box_ui_system);
        app
    }

    fn set_text(app: &mut App, text: &str) {
        let mut state = TextInputState::default();
        state.texts = vec![String::new(); INPUT_FIELD_ID + 1];
        state.texts[INPUT_FIELD_ID] = text.to_string();
        app.insert_resource(state);
    }

    /// C# `GameScene.cs:5784-5802`：`S.GuildRequestWar` → 输入框 → OK → `C.GuildWarReturn`
    #[test]
    fn ok_sends_guild_war_return_and_closes() {
        let mut app = app_with_ui_system();
        let (tx, rx) = crossbeam_channel::unbounded::<Vec<u8>>();
        app.world_mut().resource_mut::<NetConnection>().to_server = Some(tx);
        set_text(&mut app, "敌对行会");
        {
            let mut st = app.world_mut().resource_mut::<InputBoxState>();
            st.open = true;
            st.purpose = InputPurpose::GuildWarReturn;
        }
        app.world_mut().spawn((InputBoxOk, Interaction::Pressed));
        app.update();

        let raw = rx.try_recv().expect("OK 应发出 C.GuildWarReturn");
        let war: mir2_shared::packets::client::guild::GuildWarReturn =
            mir2_shared::packets::base::deserialize_packet(&mut std::io::Cursor::new(raw))
                .expect("应为 GuildWarReturn 包");
        assert_eq!(war.guild_name, "敌对行会");
        assert!(
            !app.world().resource::<InputBoxState>().open,
            "确认后应关闭"
        );
        assert!(!app
            .world()
            .resource::<DialogManager>()
            .is_open(DialogKind::InputBox));
    }

    /// C# `GameScene.cs:5772-5800`：`S.GuildNameRequest` → 输入框 → OK → `C.GuildNameReturn`
    #[test]
    fn ok_sends_guild_name_return() {
        let mut app = app_with_ui_system();
        let (tx, rx) = crossbeam_channel::unbounded::<Vec<u8>>();
        app.world_mut().resource_mut::<NetConnection>().to_server = Some(tx);
        set_text(&mut app, "NewGuild");
        {
            let mut st = app.world_mut().resource_mut::<InputBoxState>();
            st.open = true;
            st.purpose = InputPurpose::GuildNameReturn;
        }
        app.world_mut().spawn((InputBoxOk, Interaction::Pressed));
        app.update();

        let raw = rx.try_recv().expect("OK 应发出 C.GuildNameReturn");
        let name: mir2_shared::packets::client::guild::GuildNameReturn =
            mir2_shared::packets::base::deserialize_packet(&mut std::io::Cursor::new(raw))
                .expect("应为 GuildNameReturn 包");
        assert_eq!(name.name, "NewGuild");
    }

    /// C# `MirInputBox.cs:64-76`：Cancel → 关闭且不发包
    #[test]
    fn cancel_closes_without_sending() {
        let mut app = app_with_ui_system();
        let (tx, rx) = crossbeam_channel::unbounded::<Vec<u8>>();
        app.world_mut().resource_mut::<NetConnection>().to_server = Some(tx);
        set_text(&mut app, "不该发出");
        {
            let mut st = app.world_mut().resource_mut::<InputBoxState>();
            st.open = true;
            st.purpose = InputPurpose::GuildWarReturn;
        }
        app.world_mut()
            .spawn((InputBoxCancel, Interaction::Pressed));
        app.update();

        assert!(rx.try_recv().is_err(), "取消不得发包");
        assert!(!app.world().resource::<InputBoxState>().open);
    }

    /// 打开路径：`S.GuildRequestWar` → 标题取 C# `EnterGuildToWarWith` 文案、输入框聚焦
    #[test]
    fn server_request_opens_box_with_csharp_caption() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<InputBoxState>();
        app.init_resource::<TextInputState>();
        app.add_message::<ServerEvent>();
        app.add_systems(Update, input_box_open_system);

        app.world_mut().write_message(ServerEvent::TerritoryWar {
            guild_name: String::new(),
        });
        app.update();
        {
            let st = app.world().resource::<InputBoxState>();
            assert!(st.open, "收到 GuildRequestWar 应打开输入框");
            assert_eq!(st.purpose, InputPurpose::GuildWarReturn);
            assert_eq!(
                st.title, "请输入你想宣战的公会名称。",
                "C# `ClientTextKeys.EnterGuildToWarWith`"
            );
        }
        assert_eq!(
            app.world().resource::<TextInputState>().active,
            Some(INPUT_FIELD_ID),
            "打开即聚焦（C# `InputTextBox.SetFocus()`）"
        );

        // 公会名请求（`S.GuildNameRequest`）走另一条文案
        app.world_mut()
            .write_message(ServerEvent::GuildNameRequested);
        app.update();
        let st = app.world().resource::<InputBoxState>();
        assert_eq!(st.purpose, InputPurpose::GuildNameReturn);
        assert_eq!(
            st.title, "请输入公会名称，长度必须为 3~20 个字符。",
            "C# `ClientTextKeys.EnterGuildNameLengthLimit`"
        );
        // 每次打开清空上次输入
        assert_eq!(
            app.world().resource::<TextInputState>().texts[INPUT_FIELD_ID],
            ""
        );
    }

    /// 打开的输入框必须进管理栈（C# `Modal = true` → 屏蔽世界输入）
    #[test]
    fn open_box_blocks_world_click() {
        let mut app = app_with_ui_system();
        {
            let mut st = app.world_mut().resource_mut::<InputBoxState>();
            st.open = true;
        }
        app.update();
        assert!(
            app.world().resource::<DialogManager>().blocks_world_click(),
            "MirInputBox 是模态框，应屏蔽世界点击"
        );
    }
}
