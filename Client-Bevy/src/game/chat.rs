// ============================================================================
// 聊天（M8）
// 交互参考：Client/MirScenes/Dialogs/MainDialogs.cs（ChatPanel / ChatTextBox）
// 显示：聊天面板（底部左侧），最新消息向上滚动；Enter 打开输入，再次 Enter 发送
// ============================================================================

use std::collections::VecDeque;

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

use crate::game::hud::HudState;
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::scenes::AppState;
use crate::ui::pinyin_ime::{ImeFocus, PinyinIme};
use crate::resources::libraries::LibraryName;
use crate::ui::controls::spawn_checkbox;
use crate::ui::sprite_ui::{spawn_ui_text, UiButton, UiEntity, UiFont, UiImageCache};

/// 聊天频道（主话框页签，对齐 C# MainDialogs ChatPanel）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChatChannel {
    /// 全部（仅页签用，行数据不落此值）
    All,
    System,
    Nearby,
    Guild,
    Group,
    Whisper,
}

/// 服务端 ChatType → 频道
pub fn chat_channel(t: mir2_shared::enums::ChatType) -> ChatChannel {
    use mir2_shared::enums::ChatType;
    match t {
        ChatType::Guild => ChatChannel::Guild,
        ChatType::Group => ChatChannel::Group,
        ChatType::WhisperIn | ChatType::WhisperOut => ChatChannel::Whisper,
        ChatType::System
        | ChatType::System2
        | ChatType::Announcement
        | ChatType::Hint
        | ChatType::LevelUp
        | ChatType::Mentor
        | ChatType::Trainer
        | ChatType::Relationship => ChatChannel::System,
        _ => ChatChannel::Nearby,
    }
}

/// 页签列表（顺序 = 显示顺序）
pub const CHAT_TABS: [ChatChannel; 6] = [
    ChatChannel::All,
    ChatChannel::System,
    ChatChannel::Nearby,
    ChatChannel::Guild,
    ChatChannel::Group,
    ChatChannel::Whisper,
];

/// 聊天过滤项（C# ChatOptionDialog Filter 页签）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChatFilterKind {
    All,
    Normal,
    Whisper,
    Shout,
    System,
    Lover,
    Mentor,
    Group,
    Guild,
}

/// 聊天过滤设置（C# Settings.Filter*Chat）
#[derive(Resource)]
pub struct ChatFilter {
    pub normal: bool,
    pub whisper: bool,
    pub shout: bool,
    pub system: bool,
    pub lover: bool,
    pub mentor: bool,
    pub group: bool,
    pub guild: bool,
    /// 聊天面板透明（C# Settings.TransparentChat）
    pub transparent: bool,
    /// 聊天设置对话框可见
    pub visible: bool,
}

impl Default for ChatFilter {
    fn default() -> Self {
        Self {
            normal: false,
            whisper: false,
            shout: false,
            system: false,
            lover: false,
            mentor: false,
            group: false,
            guild: false,
            transparent: false,
            visible: false,
        }
    }
}

impl ChatFilter {
    pub fn get(&self, k: ChatFilterKind) -> bool {
        match k {
            ChatFilterKind::All => true,
            ChatFilterKind::Normal => self.normal,
            ChatFilterKind::Whisper => self.whisper,
            ChatFilterKind::Shout => self.shout,
            ChatFilterKind::System => self.system,
            ChatFilterKind::Lover => self.lover,
            ChatFilterKind::Mentor => self.mentor,
            ChatFilterKind::Group => self.group,
            ChatFilterKind::Guild => self.guild,
        }
    }
    pub fn set(&mut self, k: ChatFilterKind, v: bool) {
        match k {
            ChatFilterKind::All => {}
            ChatFilterKind::Normal => self.normal = v,
            ChatFilterKind::Whisper => self.whisper = v,
            ChatFilterKind::Shout => self.shout = v,
            ChatFilterKind::System => self.system = v,
            ChatFilterKind::Lover => self.lover = v,
            ChatFilterKind::Mentor => self.mentor = v,
            ChatFilterKind::Group => self.group = v,
            ChatFilterKind::Guild => self.guild = v,
        }
    }
}

/// 服务端 ChatType → 对应过滤项
pub fn chat_filter_kind(t: mir2_shared::enums::ChatType) -> ChatFilterKind {
    use mir2_shared::enums::ChatType;
    match t {
        ChatType::Guild => ChatFilterKind::Guild,
        ChatType::Group => ChatFilterKind::Group,
        ChatType::WhisperIn | ChatType::WhisperOut => ChatFilterKind::Whisper,
        ChatType::Shout | ChatType::Shout2 | ChatType::Shout3 => ChatFilterKind::Shout,
        ChatType::Mentor | ChatType::Trainer => ChatFilterKind::Mentor,
        ChatType::Relationship => ChatFilterKind::Lover,
        ChatType::System
        | ChatType::System2
        | ChatType::Announcement
        | ChatType::Hint
        | ChatType::LevelUp => ChatFilterKind::System,
        _ => ChatFilterKind::Normal,
    }
}

/// 页签显示名
pub fn chat_tab_name(tab: ChatChannel) -> &'static str {
    match tab {
        ChatChannel::All => "全部",
        ChatChannel::System => "系统",
        ChatChannel::Nearby => "附近",
        ChatChannel::Guild => "行会",
        ChatChannel::Group => "队伍",
        ChatChannel::Whisper => "私聊",
    }
}

/// 聊天状态（网络 handler 写入，显示系统读取）
#[derive(Resource)]
pub struct ChatState {
    /// (文本, 颜色, 频道, 物品链接 uid) 历史，最新在末尾；uid=None 表示无物品链接（#287）
    pub lines: VecDeque<(String, Color, ChatChannel, Option<u64>)>,
    /// 当前页签
    pub tab: ChatChannel,
    /// 输入框激活
    pub input_active: bool,
    /// 当前输入文本
    pub input_text: String,
    /// 显示行数
    pub visible_lines: usize,
    /// 历史回看滚动（0=最新；C# ChatPanel StartIndex）
    pub scroll_up: usize,
    /// 窗口尺寸档（0/1/2 → 行数 4/7/11，C# ChangeSize）
    pub size: usize,
}

impl Default for ChatState {
    fn default() -> Self {
        Self {
            lines: VecDeque::new(),
            tab: ChatChannel::All,
            input_active: false,
            input_text: String::new(),
            visible_lines: 8,
            scroll_up: 0,
            size: 1,
        }
    }
}

impl ChatState {
    pub fn add_line(&mut self, text: impl Into<String>, color: Color, channel: ChatChannel) {
        self.lines.push_back((text.into(), color, channel, None));
        while self.lines.len() > 200 {
            self.lines.pop_front();
        }
        // 新消息到达时回到最新（若已在最新位置）
        let max_scroll = self.lines.len().saturating_sub(self.visible_lines);
        self.scroll_up = self.scroll_up.min(max_scroll);
    }

    /// #287：带物品链接的行（uid 供点击 tooltip）
    pub fn add_item_line(
        &mut self,
        text: impl Into<String>,
        color: Color,
        channel: ChatChannel,
        uid: u64,
    ) {
        self.lines
            .push_back((text.into(), color, channel, Some(uid)));
        while self.lines.len() > 200 {
            self.lines.pop_front();
        }
        // 新消息到达时回到最新（若已在最新位置）
        let max_scroll = self.lines.len().saturating_sub(self.visible_lines);
        self.scroll_up = self.scroll_up.min(max_scroll);
    }
}

/// 聊天面板根标记
#[derive(Component)]
struct ChatPanel;

/// 第 i 行文本
#[derive(Component)]
struct ChatLine(usize);

/// 输入行文本
#[derive(Component)]
struct ChatInputText;

/// 频道页签按钮
#[derive(Component)]
struct ChatTabBtn(ChatChannel);

/// 发送频道快捷按钮（C# ChatControlBar）
#[derive(Component)]
struct ChatBarBtn(&'static str, &'static str);

/// 聊天设置按钮（打开 ChatOptionDialog）
#[derive(Component)]
struct ChatSettingsBtn;

/// 窗口尺寸切换按钮（C# ChatPanel SizeButton）
#[derive(Component)]
struct ChatSizeBtn;

/// 聊天设置面板背景（透明开关改 alpha）
#[derive(Component)]
struct ChatPanelBg;

/// 聊天设置面板（过滤/透明）
#[derive(Component)]
struct ChatOptionWidget;

/// 过滤勾选框
#[derive(Component)]
struct ChatFilterBox(ChatFilterKind);

/// 透明开关按钮（true=透明）
#[derive(Component)]
struct ChatTranspBtn(bool);

pub struct ChatPlugin;

impl Plugin for ChatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChatFilter>();
        app.init_resource::<ChatItemCache>();
        app.add_systems(OnEnter(AppState::Game), spawn_chat);
        app.add_systems(OnEnter(AppState::Game), spawn_chat_option_panel);
        app.add_systems(OnExit(AppState::Game), cleanup_chat);
        app.add_systems(
            Update,
            (
                chat_tab_system,
                chat_option_system.after(crate::ui::controls::checkbox_system),
                chat_size_system,
                chat_wheel_system,
                chat_key_scroll_system,
                chat_input_system,
                chat_display_system,
                chat_server_events,
                chat_item_cache_events,
                chat_item_click_system,
            )
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_chat(
    mut commands: Commands,
    roots: Query<Entity, Or<(With<ChatPanel>, With<ChatOptionWidget>)>>,
) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_chat(
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

    let panel_x = 6.0;
    let panel_y = 768.0 - 150.0 - 190.0; // 主对话框上方

    // 面板底色（半透明黑，1x1 白图着色）
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    commands.spawn((
        UiEntity,
        ChatPanel,
        ChatPanelBg,
        Sprite {
            image: white,
            custom_size: Some(Vec2::new(360.0, 172.0)),
            color: Color::srgba(0.0, 0.0, 0.0, 0.55),
            ..default()
        },
        Transform::from_xyz(panel_x + 180.0, -(panel_y + 75.0), 1.5),
        Visibility::default(),
    ));

    // 频道页签（主话框：全部/系统/附近/行会/队伍/私聊）
    let tab_w = 60.0;
    for (i, tab) in CHAT_TABS.iter().enumerate() {
        let tx = panel_x + 2.0 + i as f32 * tab_w;
        let e = spawn_ui_text(
            &mut commands, &font, chat_tab_name(*tab),
            tx, panel_y + 2.0,
            11.0, Color::srgb(0.8, 0.8, 0.8), 2.2,
        );
        commands.entity(e).insert((
            ChatTabBtn(*tab),
            UiButton {
                rect: (tx, panel_y + 2.0, tab_w - 2.0, 14.0),
                clicked: false,
            },
        ));
    }
    // 消息行（8 行）
    for i in 0..8usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            panel_x + 4.0, panel_y + 20.0 + i as f32 * 16.0,
            12.0, Color::WHITE, 2.0,
        );
        commands.entity(e).insert(ChatLine(i));
    }
    // 输入行
    let e = spawn_ui_text(
        &mut commands, &font, "",
        panel_x + 4.0, panel_y + 150.0,
        12.0, Color::srgb(0.9, 0.9, 0.4), 2.0,
    );
    commands.entity(e).insert(ChatInputText);
    // 发送频道快捷按钮（C# ChatControlBar：附近/喊话/行会/队伍/私聊）
    // 点击把指令前缀填入输入框（服务端按前缀路由频道）
    let bar: [(&str, &str); 5] = [
        ("附近", ""),
        ("喊话", "/s "),
        ("行会", "/guild "),
        ("队伍", "/g "),
        ("私聊", "/w "),
    ];
    for (i, (label, prefix)) in bar.iter().enumerate() {
        // 频道快捷栏放在面板上方（C# ChatControlBar 为独立条）
        let bx = panel_x + 4.0 + i as f32 * 72.0;
        let by = panel_y - 16.0;
        let t = spawn_ui_text(
            &mut commands, &font, label,
            bx, by,
            11.0, Color::srgb(0.7, 0.9, 1.0), 2.2,
        );
        commands.entity(t).insert((
            ChatBarBtn(label, prefix),
            UiButton {
                rect: (bx, by, 70.0, 14.0),
                clicked: false,
            },
        ));
    }
    // 聊天设置按钮（打开 C# ChatOptionDialog）
    let settings = spawn_ui_text(
        &mut commands, &font, "设置",
        panel_x + 360.0 - 34.0, panel_y + 2.0,
        11.0, Color::srgb(0.8, 0.9, 1.0), 2.2,
    );
    commands.entity(settings).insert((
        ChatSettingsBtn,
        UiButton {
            rect: (panel_x + 360.0 - 34.0, panel_y + 2.0, 32.0, 14.0),
            clicked: false,
        },
    ));
    // 窗口尺寸切换（C# ChatPanel SizeButton）
    let size_btn = spawn_ui_text(
        &mut commands, &font, "尺寸",
        panel_x + 360.0 - 66.0, panel_y + 2.0,
        11.0, Color::srgb(0.8, 0.9, 1.0), 2.2,
    );
    commands.entity(size_btn).insert((
        ChatSizeBtn,
        UiButton {
            rect: (panel_x + 360.0 - 66.0, panel_y + 2.0, 30.0, 14.0),
            clicked: false,
        },
    ));
}

/// 窗口尺寸切换（#160 C# ChatPanel ChangeSize：4/8/11 行三档）
fn chat_size_system(
    mut chat: ResMut<ChatState>,
    size_btn: Query<&UiButton, With<ChatSizeBtn>>,
    mut bg: Query<&mut Sprite, (With<ChatPanelBg>, Without<ChatOptionWidget>)>,
    mut input: Query<&mut Transform, (With<ChatInputText>, Without<ChatLine>)>,
) {
    const PANEL_Y: f32 = 428.0;
    for btn in &size_btn {
        if btn.clicked {
            chat.size = (chat.size + 1) % 3;
            let lines = [4usize, 8, 11][chat.size];
            chat.visible_lines = lines;
            let panel_h = 20.0 + lines as f32 * 16.0 + 24.0;
            for mut sp in &mut bg {
                if let Some(cs) = sp.custom_size.as_mut() {
                    *cs = Vec2::new(360.0, panel_h);
                }
            }
            let input_y = 20.0 + lines as f32 * 16.0 + 6.0;
            for mut tf in &mut input {
                tf.translation.y = -(PANEL_Y + input_y);
            }
            tracing::info!("💬 聊天窗口尺寸 -> {} 行", lines);
        }
    }
}

/// 聊天设置面板（过滤 + 透明，C# ChatOptionDialog）
fn spawn_chat_option_panel(
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
    let (dx, dy) = (400.0f32, 300.0f32);

    // 面板背景（半透明深色 + 边框感）
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    commands.spawn((
        UiEntity,
        ChatOptionWidget,
        Sprite {
            image: white.clone(),
            custom_size: Some(Vec2::new(224.0, 180.0)),
            color: Color::srgba(0.12, 0.12, 0.16, 0.95),
            ..default()
        },
        bevy::sprite::Anchor::TOP_LEFT,
        Transform::from_xyz(dx, -dy, 12.0),
        Visibility::Hidden,
    ));
    // 标题
    let title = spawn_ui_text(&mut commands, &font, "聊天设置", dx + 70.0, dy + 8.0, 13.0, Color::srgb(1.0, 0.9, 0.3), 12.2);
    commands.entity(title).insert(ChatOptionWidget);
    // 关闭
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        dx + 198.0, dy + 3.0, 12.3, 20.0, 20.0,
    ) {
        commands.entity(e).insert(ChatOptionWidget);
    }
    // 过滤勾选框（C# Prguse 2070/2071 等：偶数=勾选，奇数=未勾选）
    let items: [(ChatFilterKind, &str, [usize; 3], [usize; 3], f32, f32); 9] = [
        (ChatFilterKind::All, "全部", [2087, 2087, 2087], [2086, 2086, 2086], 74.0, 47.0),
        (ChatFilterKind::Normal, "普通", [2071, 2071, 2071], [2070, 2070, 2070], 40.0, 69.0),
        (ChatFilterKind::Whisper, "私聊", [2075, 2075, 2075], [2074, 2074, 2074], 40.0, 92.0),
        (ChatFilterKind::Shout, "喊话", [2073, 2073, 2073], [2072, 2072, 2072], 40.0, 115.0),
        (ChatFilterKind::System, "系统", [2085, 2085, 2085], [2084, 2084, 2084], 40.0, 138.0),
        (ChatFilterKind::Lover, "情侣", [2077, 2077, 2077], [2076, 2076, 2076], 135.0, 69.0),
        (ChatFilterKind::Mentor, "师徒", [2079, 2079, 2079], [2078, 2078, 2078], 135.0, 92.0),
        (ChatFilterKind::Group, "队伍", [2081, 2081, 2081], [2080, 2080, 2080], 135.0, 115.0),
        (ChatFilterKind::Guild, "行会", [2083, 2083, 2083], [2082, 2082, 2082], 135.0, 138.0),
    ];
    for (kind, label, off, on, rx, ry) in items {
        if let Some(e) = spawn_checkbox(
            &mut commands, &mut libs, &mut images, &mut cache,
            LibraryName::Prguse, off, on,
            dx + rx, dy + ry, 12.4, 16.0, 12.0,
            false,
        ) {
            commands.entity(e).insert((ChatFilterBox(kind), ChatOptionWidget));
        }
        let t = spawn_ui_text(&mut commands, &font, label, dx + rx + 20.0, dy + ry, 12.0, Color::WHITE, 12.4);
        commands.entity(t).insert(ChatOptionWidget);
    }
    // 透明开关（C# Title 471-475）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 471, 472, 470,
        dx + 45.0, dy + 90.0, 12.4, 40.0, 16.0,
    ) {
        commands.entity(e).insert((ChatTranspBtn(false), ChatOptionWidget));
    }
    let t = spawn_ui_text(&mut commands, &font, "不透明", dx + 48.0, dy + 91.0, 11.0, Color::WHITE, 12.5);
    commands.entity(t).insert(ChatOptionWidget);
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 474, 475, 473,
        dx + 115.0, dy + 90.0, 12.4, 40.0, 16.0,
    ) {
        commands.entity(e).insert((ChatTranspBtn(true), ChatOptionWidget));
    }
    let t = spawn_ui_text(&mut commands, &font, "透明", dx + 122.0, dy + 91.0, 11.0, Color::WHITE, 12.5);
    commands.entity(t).insert(ChatOptionWidget);
}


/// 聊天设置面板（#116 C# ChatOptionDialog）：打开/关闭、过滤同步、透明开关
#[allow(clippy::too_many_arguments)]
fn chat_option_system(
    mut filter: ResMut<ChatFilter>,
    settings: Query<&UiButton, With<ChatSettingsBtn>>,
    close: Query<&UiButton, (With<ChatOptionWidget>, Without<ChatFilterBox>, Without<ChatTranspBtn>)>,
    mut boxes: Query<
        (
            &UiButton,
            &mut crate::ui::controls::CheckBox,
            &mut crate::ui::sprite_ui::ButtonFrames,
            &mut Sprite,
            &ChatFilterBox,
        ),
        With<ChatOptionWidget>,
    >,
    transp: Query<(&UiButton, &ChatTranspBtn), With<ChatOptionWidget>>,
    mut widgets: Query<&mut Visibility, With<ChatOptionWidget>>,
    mut panel_bg: Query<&mut Sprite, (With<ChatPanelBg>, Without<ChatOptionWidget>)>,
) {
    // 设置按钮开/关
    for btn in &settings {
        if btn.clicked {
            filter.visible = !filter.visible;
            tracing::info!("💬 聊天设置: {}", if filter.visible { "打开" } else { "关闭" });
        }
    }
    for btn in &close {
        if btn.clicked {
            filter.visible = false;
        }
    }
    // 面板显隐
    for mut vis in &mut widgets {
        *vis = if filter.visible { Visibility::Visible } else { Visibility::Hidden };
    }
    // 勾选框 ↔ 过滤状态（checked = 屏蔽该频道，C# Filter*Chat 语义）
    // C# ChatOptionDialog：AllFiltersOff 默认 true（全部未过滤）；All 点击 = 全开/全关切换
    let mut all_clicked = false;
    let mut any_on = false;
    for (btn, cb, _, _, kind) in boxes.iter() {
        if kind.0 == ChatFilterKind::All {
            all_clicked = btn.clicked;
        } else if filter.get(kind.0) {
            any_on = true;
        }
    }
    if all_clicked {
        // All 切换：当前全关 → 全开；否则 → 全关（C# ToggleAllFilters）
        let turn_on = !any_on;
        for (_, _, _, _, kind) in &boxes {
            if kind.0 != ChatFilterKind::All {
                filter.set(kind.0, turn_on);
            }
        }
        any_on = turn_on;
    } else {
        // 单项勾选框状态 → 过滤状态（checkbox_system 已切换勾选）
        for (_, cb, _, _, kind) in &boxes {
            if kind.0 != ChatFilterKind::All && filter.get(kind.0) != cb.checked {
                filter.set(kind.0, cb.checked);
                if cb.checked {
                    any_on = true;
                }
            }
        }
    }
    // 同步勾选框显示：All = 任一频道被屏蔽（C# AllButton.Index = AllFiltersOff ? 2087 : 2086）
    for (_, mut cb, mut frames, mut sprite, kind) in &mut boxes {
        let v = if kind.0 == ChatFilterKind::All { any_on } else { filter.get(kind.0) };
        cb.checked = v;
        let f = if v { &cb.on } else { &cb.off };
        if frames.normal != f[0] {
            frames.normal = f[0].clone();
            frames.hover = f[1].clone();
            frames.pressed = f[2].clone();
            sprite.image = f[0].clone();
        }
    }
    // 透明开关
    for (btn, t) in &transp {
        if btn.clicked && filter.transparent != t.0 {
            filter.transparent = t.0;
            tracing::info!("💬 聊天面板透明: {}", t.0);
        }
    }
    // 面板底色透明度
    for mut sp in &mut panel_bg {
        let target = if filter.transparent {
            Color::srgba(0.0, 0.0, 0.0, 0.15)
        } else {
            Color::srgba(0.0, 0.0, 0.0, 0.55)
        };
        if sp.color != target {
            sp.color = target;
        }
    }
}

/// 键盘输入：Enter 激活/发送；字符输入；Backspace 删除；内置拼音 IME 中文提交
fn chat_input_system(
    mut keys: MessageReader<KeyboardInput>,
    mut ime: ResMut<PinyinIme>,
    mut focus: ResMut<ImeFocus>,
    mut chat: ResMut<ChatState>,
    net: Res<NetConnection>,
    net_mode: Res<crate::network::NetMode>,
    hud: Res<HudState>,
    filter: Res<ChatFilter>,
) {
    let key_list: Vec<KeyboardInput> = keys.read().cloned().collect();

    // 回填 IME 聚焦框（聊天输入行屏幕矩形，候选条定位 + 判定字母是否进 IME）
    // 输入行位置见 spawn_chat：panel_x+4=10, panel_y+140=568
    // 只写 Some（None 由 clear_ime_focus 每帧统一重置，避免与 Game 态其他输入框互相覆盖）
    if chat.input_active {
        focus.rect = Some((10.0, 568.0, 350.0, 16.0));
    }

    // Enter：激活/发送（组合中被 IME 接管 → 跳过，不发送）
    for key in &key_list {
        if key.state != bevy::input::ButtonState::Pressed {
            continue;
        }
        if ime.consumes_key(key) {
            continue;
        }
        if key.logical_key == Key::Enter {
            if chat.input_active {
                let msg = chat.input_text.trim().to_string();
                chat.input_text.clear();
                chat.input_active = false;
                if !msg.is_empty() {
                    net.send_packet(&mir2_shared::packets::client::chat::Chat {
                        message: msg.clone(),
                        linked_items: Vec::new(),
                    });
                    // 本地回显（C# MainDialogs 发送时本地加入聊天面板；真实服务器不回发给自己）。
                    // 指令消息（/w /g /guild /s ! 等）服务器会回发对应频道回显 → 本地不再回显避免重复。
                    // mock 服务器会回显，只在真实 TCP 模式下本地回显避免重复
                    let is_cmd = msg.starts_with('/') || msg.starts_with('!');
                    // #116 本地回显同样受“普通”过滤控制
                    if matches!(net_mode.0, crate::network::NetworkMode::Real)
                        && !is_cmd
                        && !filter.normal
                    {
                        chat.add_line(
                            format!("[{}]: {}", hud.name, msg),
                            Color::WHITE,
                            ChatChannel::Nearby,
                        );
                    }
                    tracing::info!("💬 发送聊天: {}", msg);
                }
            } else {
                chat.input_active = true;
            }
            continue;
        }
    }

    if !chat.input_active {
        return;
    }

    for key in &key_list {
        if key.state != bevy::input::ButtonState::Pressed {
            continue;
        }
        if ime.consumes_key(key) {
            continue;
        }
        if key.logical_key == Key::Backspace {
            chat.input_text.pop();
        } else if let Some(text) = &key.text {
            if !text.is_empty() {
                chat.input_text.push_str(text);
            }
        }
    }

    // 内置拼音 IME 提交的汉字
    if let Some(c) = ime.take_commit() {
        chat.input_text.push_str(&c);
    }
}

/// 聊天历史滚轮回看（#126 C# ChatPanel_MouseWheel：StartIndex）
fn chat_wheel_system(
    mut wheels: MessageReader<MouseWheel>,
    mut chat: ResMut<ChatState>,
    windows: Query<&Window>,
) {
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    const PANEL: (f32, f32, f32, f32) = (6.0, 428.0, 360.0, 172.0);
    if cursor.x < PANEL.0
        || cursor.x > PANEL.0 + PANEL.2
        || cursor.y < PANEL.1
        || cursor.y > PANEL.1 + PANEL.3
    {
        return;
    }
    let mut delta = 0.0f32;
    for ev in wheels.read() {
        match ev.unit {
            MouseScrollUnit::Line => delta += ev.y,
            MouseScrollUnit::Pixel => delta += ev.y / 20.0,
        }
    }
    if delta == 0.0 {
        return;
    }
    let max_scroll = chat.lines.len().saturating_sub(chat.visible_lines);
    chat.scroll_up = (chat.scroll_up as i32 + delta.round() as i32)
        .clamp(0, max_scroll as i32) as usize;
}

/// 键盘滚动聊天历史（#802，对齐 C# ChatPanel_KeyDown）：
/// Up/Down = 行；Home/End = 最旧/最新；PageUp/PageDown = 页（visible_lines）。
/// 门控：输入框未激活 + 鼠标悬停在聊天面板区域（与 chat_wheel_system 一致，
/// 近似 C# 面板焦点，避免与其它可滚动对话框的 Up/Down 冲突）。
fn chat_key_scroll_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut chat: ResMut<ChatState>,
    windows: Query<&Window>,
) {
    if chat.input_active {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    const PANEL: (f32, f32, f32, f32) = (6.0, 428.0, 360.0, 172.0);
    if cursor.x < PANEL.0
        || cursor.x > PANEL.0 + PANEL.2
        || cursor.y < PANEL.1
        || cursor.y > PANEL.1 + PANEL.3
    {
        return;
    }
    let max_scroll = chat.lines.len().saturating_sub(chat.visible_lines);
    let page = chat.visible_lines.max(1);
    if keys.just_pressed(KeyCode::ArrowUp) {
        chat.scroll_up = apply_key_scroll(chat.scroll_up, max_scroll, page, KeyScroll::Up);
    } else if keys.just_pressed(KeyCode::ArrowDown) {
        chat.scroll_up = apply_key_scroll(chat.scroll_up, max_scroll, page, KeyScroll::Down);
    } else if keys.just_pressed(KeyCode::Home) {
        chat.scroll_up = apply_key_scroll(chat.scroll_up, max_scroll, page, KeyScroll::Home);
    } else if keys.just_pressed(KeyCode::End) {
        chat.scroll_up = apply_key_scroll(chat.scroll_up, max_scroll, page, KeyScroll::End);
    } else if keys.just_pressed(KeyCode::PageUp) {
        chat.scroll_up = apply_key_scroll(chat.scroll_up, max_scroll, page, KeyScroll::PageUp);
    } else if keys.just_pressed(KeyCode::PageDown) {
        chat.scroll_up = apply_key_scroll(chat.scroll_up, max_scroll, page, KeyScroll::PageDown);
    }
}

/// 键盘滚动动作（#802，C# ChatPanel_KeyDown 语义；0=最新）
#[derive(Clone, Copy)]
enum KeyScroll {
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
}

/// 纯函数：根据动作计算新的 scroll_up（便于单测）
fn apply_key_scroll(scroll_up: usize, max_scroll: usize, page: usize, action: KeyScroll) -> usize {
    let page = page.max(1);
    match action {
        KeyScroll::Up => (scroll_up + 1).min(max_scroll),
        KeyScroll::Down => scroll_up.saturating_sub(1),
        KeyScroll::Home => max_scroll,
        KeyScroll::End => 0,
        KeyScroll::PageUp => (scroll_up + page).min(max_scroll),
        KeyScroll::PageDown => scroll_up.saturating_sub(page),
    }
}

/// 显示：按页签过滤聊天行 + 输入行（单查询避免 B0001）
fn chat_display_system(
    chat: Res<ChatState>,
    mut texts: Query<(
        &mut Text2d,
        &mut TextColor,
        Option<&ChatLine>,
        Option<&ChatInputText>,
        Option<&ChatTabBtn>,
    )>,
) {
    // 性能（#112）：只有聊天状态变化才重建文本（每帧重建是 CPU 热点）
    if !chat.is_changed() {
        return;
    }
    // 按页签收集可见行（All 显示全部；否则只显示对应频道）
    let mut visible: Vec<(String, Color)> = chat
        .lines
        .iter()
        .filter(|(_, _, ch, _)| chat.tab == ChatChannel::All || *ch == chat.tab)
        .map(|(m, c, _, _)| (m.clone(), *c))
        .collect();
    // #126 历史回看：从滚动起始行显示
    let start = visible
        .len()
        .saturating_sub(chat.visible_lines)
        .saturating_sub(chat.scroll_up);
    for (mut text, mut color, line, input, tab_btn) in &mut texts {
        // 变化才更新，避免每帧重排文本（ICU4X 报错 + CPU，#31）
        if let Some(line) = line {
            let (msg, c) = match visible.get(start + line.0) {
                Some((m, c)) => (m.clone(), *c),
                None => (String::new(), Color::WHITE),
            };
            if text.0 != msg {
                text.0 = msg;
            }
            if color.0 != c {
                color.0 = c;
            }
        } else if input.is_some() {
            let new = if chat.input_active {
                format!("> {}", chat.input_text)
            } else {
                String::new()
            };
            if text.0 != new {
                text.0 = new;
            }
        } else if let Some(tab_btn) = tab_btn {
            // 选中页签高亮
            let selected = tab_btn.0 == chat.tab;
            let c = if selected {
                Color::srgb(1.0, 0.9, 0.3)
            } else {
                Color::srgb(0.8, 0.8, 0.8)
            };
            if color.0 != c {
                color.0 = c;
            }
        }
    }
}

/// 页签点击切换 + 发送频道快捷按钮
fn chat_tab_system(
    mut chat: ResMut<ChatState>,
    tabs: Query<(&UiButton, &ChatTabBtn)>,
    bar: Query<(&UiButton, &ChatBarBtn)>,
) {
    for (btn, tab) in &tabs {
        if btn.clicked && chat.tab != tab.0 {
            chat.tab = tab.0;
            chat.scroll_up = 0;
            tracing::info!("💬 聊天页签 -> {:?}", tab.0);
        }
    }
    for (btn, bar_btn) in &bar {
        if btn.clicked {
            chat.input_active = true;
            chat.input_text = bar_btn.1.to_string();
            tracing::info!("💬 频道快捷: {} -> prefix", bar_btn.0);
        }
    }
}


/// 聊天颜色映射（从网络层移入：展示逻辑归 UI 模块，网络只发 ServerEvent）
pub fn chat_color(t: mir2_shared::enums::ChatType) -> Color {
    use mir2_shared::enums::ChatType;
    match t {
        ChatType::Normal => Color::WHITE,
        ChatType::Shout | ChatType::Shout2 | ChatType::Shout3 => Color::srgb(1.0, 0.75, 0.3),
        ChatType::System | ChatType::System2 | ChatType::Announcement => {
            Color::srgb(1.0, 0.95, 0.4)
        }
        ChatType::Hint => Color::srgb(0.4, 1.0, 0.4),
        ChatType::Group => Color::srgb(0.5, 0.9, 1.0),
        ChatType::WhisperIn | ChatType::WhisperOut => Color::srgb(1.0, 0.5, 1.0),
        ChatType::Guild => Color::srgb(0.8, 0.6, 1.0),
        ChatType::LevelUp => Color::srgb(1.0, 0.9, 0.2),
        ChatType::Mentor | ChatType::Trainer | ChatType::Relationship => {
            Color::srgb(0.6, 1.0, 0.8)
        }
        _ => Color::WHITE,
    }
}

/// 消费服务端聊天事件更新 ChatState（网络层只广播 ServerEvent）
fn chat_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut chat: ResMut<ChatState>,
    filter: Res<ChatFilter>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::Chat { text, chat_type } = ev {
            // #116 聊天过滤：屏蔽对应频道
            if filter.get(chat_filter_kind(*chat_type)) {
                continue;
            }
            // #285/#287：含物品链接 `%名字#uid%` 的行用蓝色并记录 uid（C# ChatLink）
            if let Some(uid) = first_item_uid(text) {
                chat.add_item_line(
                    text.clone(),
                    Color::srgb(0.3, 0.6, 1.0),
                    chat_channel(*chat_type),
                    uid,
                );
            } else {
                chat.add_line(text.clone(), chat_color(*chat_type), chat_channel(*chat_type));
            }
        }
        if let ServerEvent::ServerMessage { message, .. } = ev {
            // #258：服务端输出消息（系统提示）
            chat.add_line(
                message.clone(),
                Color::srgb(0.4, 1.0, 0.4),
                ChatChannel::System,
            );
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use mir2_shared::enums::ChatType;

    #[test]
    fn channel_mapping() {
        assert_eq!(chat_channel(ChatType::Normal), ChatChannel::Nearby);
        assert_eq!(chat_channel(ChatType::Shout), ChatChannel::Nearby);
        assert_eq!(chat_channel(ChatType::System), ChatChannel::System);
        assert_eq!(chat_channel(ChatType::Guild), ChatChannel::Guild);
        assert_eq!(chat_channel(ChatType::Group), ChatChannel::Group);
        assert_eq!(chat_channel(ChatType::WhisperIn), ChatChannel::Whisper);
        assert_eq!(chat_channel(ChatType::WhisperOut), ChatChannel::Whisper);
    }

    #[test]
    fn filter_kind_mapping() {
        assert_eq!(chat_filter_kind(ChatType::Normal), ChatFilterKind::Normal);
        assert_eq!(chat_filter_kind(ChatType::Shout), ChatFilterKind::Shout);
        assert_eq!(chat_filter_kind(ChatType::System), ChatFilterKind::System);
        assert_eq!(chat_filter_kind(ChatType::Guild), ChatFilterKind::Guild);
        assert_eq!(chat_filter_kind(ChatType::Group), ChatFilterKind::Group);
        assert_eq!(chat_filter_kind(ChatType::WhisperIn), ChatFilterKind::Whisper);
        assert_eq!(chat_filter_kind(ChatType::Mentor), ChatFilterKind::Mentor);
        assert_eq!(chat_filter_kind(ChatType::Relationship), ChatFilterKind::Lover);
    }

    #[test]
    fn parse_item_link_uid() {
        assert_eq!(first_item_uid("看看 [%金创药(小)#9005%] 这个"), Some(9005));
        assert_eq!(first_item_uid("没有链接的消息"), None);
        assert_eq!(first_item_uid("%名字#abc%"), None);
        assert_eq!(first_item_uid("前文 %a#1% 后文 %b#2%"), Some(1));
    }

    #[test]
    fn filter_get_set() {
        let mut f = ChatFilter::default();
        assert!(!f.get(ChatFilterKind::Guild));
        f.set(ChatFilterKind::Guild, true);
        assert!(f.get(ChatFilterKind::Guild));
    }
}

/// #285：聊天物品缓存（S.NewChatItem → unique_id → InvItem，供聊天链接 tooltip）
#[derive(Resource, Default)]
pub struct ChatItemCache {
    pub items: std::collections::HashMap<u64, crate::game::dialogs::inventory::InvItem>,
}

/// 解析消息中第一个 C# 聊天物品链接 `%名字#uid%` 的 uid
fn first_item_uid(text: &str) -> Option<u64> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if let Some(rel) = text[i + 1..].find('%') {
                let inner = &text[i + 1..i + 1 + rel];
                if let Some(hash) = inner.rfind('#') {
                    if let Ok(uid) = inner[hash + 1..].trim().parse::<u64>() {
                        return Some(uid);
                    }
                }
                i += rel + 2;
                continue;
            }
        }
        i += 1;
    }
    None
}

/// #285：消费 ServerEvent::ChatItemReceived → 写入 ChatItemCache
fn chat_item_cache_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut cache: ResMut<ChatItemCache>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::ChatItemReceived { item } = ev {
            cache.items.insert(item.unique_id, item.clone());
            tracing::info!("💬 聊天物品缓存: {} (uid={})", item.name, item.unique_id);
        }
    }
}

/// #287：点击聊天行（含物品链接）→ 显示物品 tooltip（C# CreateItemLabel 对齐）；
/// 未缓存则发 C.RequestChatItem 供下次解析
fn chat_item_click_system(
    chat: Res<ChatState>,
    cache: Res<ChatItemCache>,
    net: Res<NetConnection>,
    mut tooltip: ResMut<crate::ui::tooltip::TooltipState>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    // 聊天面板区域（panel_x=6, panel_y=428，行高 16，首行 y=panel_y+20）
    const PANEL_X: f32 = 6.0;
    const PANEL_Y: f32 = 428.0;
    const LINE_H: f32 = 16.0;
    if cursor.x < PANEL_X
        || cursor.x > PANEL_X + 360.0
        || cursor.y < PANEL_Y + 20.0
        || cursor.y > PANEL_Y + 20.0 + chat.visible_lines as f32 * LINE_H
    {
        return;
    }
    // 可见行 uid 列表（与 chat_display_system 一致）
    let visible: Vec<Option<u64>> = chat
        .lines
        .iter()
        .filter(|(_, _, ch, _)| chat.tab == ChatChannel::All || *ch == chat.tab)
        .map(|(_, _, _, uid)| *uid)
        .collect();
    let start = visible
        .len()
        .saturating_sub(chat.visible_lines)
        .saturating_sub(chat.scroll_up);
    let row = ((cursor.y - (PANEL_Y + 20.0)) / LINE_H) as usize;
    let Some(uid) = visible.get(start + row).copied().flatten() else {
        return;
    };
    if let Some(item) = cache.items.get(&uid) {
        let mut lines = Vec::new();
        if item.count > 1 {
            lines.push(format!("数量: {}", item.count));
        }
        lines.push(format!(
            "类型: {}",
            crate::game::dialogs::inventory::item_type_name(item.item_type)
        ));
        if item.is_equipment() {
            lines.push(format!("耐久: {}/{}", item.current_dura, item.max_dura));
        }
        tooltip.update(2, true, item.name.clone(), lines, cursor.x, cursor.y);
        tracing::info!("💬 点击聊天物品: {} (uid={})", item.name, uid);
    } else {
        net.send_packet(&mir2_shared::packets::client::misc::RequestChatItem {
            chat_item_id: uid,
        });
        tracing::info!("💬 聊天物品 uid={} 未缓存，已请求", uid);
    }
}

#[cfg(test)]
mod chat_scroll_tests {
    use super::{apply_key_scroll, KeyScroll};

    #[test]
    fn up_down_line_scroll() {
        assert_eq!(apply_key_scroll(0, 10, 8, KeyScroll::Up), 1);
        assert_eq!(apply_key_scroll(1, 10, 8, KeyScroll::Down), 0);
        // 边界
        assert_eq!(apply_key_scroll(10, 10, 8, KeyScroll::Up), 10);
        assert_eq!(apply_key_scroll(0, 10, 8, KeyScroll::Down), 0);
    }

    #[test]
    fn home_end_jump() {
        assert_eq!(apply_key_scroll(3, 10, 8, KeyScroll::Home), 10);
        assert_eq!(apply_key_scroll(3, 10, 8, KeyScroll::End), 0);
        // 空历史
        assert_eq!(apply_key_scroll(0, 0, 8, KeyScroll::Home), 0);
    }

    #[test]
    fn page_scroll() {
        assert_eq!(apply_key_scroll(0, 20, 8, KeyScroll::PageUp), 8);
        assert_eq!(apply_key_scroll(8, 20, 8, KeyScroll::PageDown), 0);
        assert_eq!(apply_key_scroll(15, 20, 8, KeyScroll::PageUp), 20);
        assert_eq!(apply_key_scroll(5, 20, 8, KeyScroll::PageDown), 0);
    }
}
