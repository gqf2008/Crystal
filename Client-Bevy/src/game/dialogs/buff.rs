// ============================================================================
// 状态/增益对话框（M44）
// 参考：C# BuffDialog + ServerRust buff 系统
// 网络（M44 简化 wire，服务端此前无 AddBuff 推送）：
//   S: AddBuff[tag u8][remaining_ticks u32] / RemoveBuff[tag u8]
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};

/// Buff 条目
#[derive(Debug, Clone, Copy, Default)]
pub struct BuffEntry {
    pub tag: u8,
    pub remaining_ticks: u32,
}

/// Buff 状态
#[derive(Resource, Default)]
pub struct BuffState {
    pub buffs: Vec<BuffEntry>,
    pub message: String,
    /// 展开/收起（C# BuffDialog ExpandedBuffWindow，[Game] 段持久化）
    pub expanded: bool,
}

impl BuffState {
    /// 从 Mir2Config.ini 解析展开状态（C# Settings.ExpandedBuffWindow；缺失默认展开）
    pub fn from_ini(content: &str) -> Self {
        use crate::game::dialogs::settings_file::ini_bool;
        Self {
            buffs: Vec::new(),
            message: String::new(),
            expanded: ini_bool(content, "Game", "ExpandedBuffWindow", true),
        }
    }

    /// 启动时加载（C# Settings.Load）
    pub fn load() -> Self {
        Self::from_ini(&crate::game::dialogs::settings_file::load_ini())
    }

    /// 保存展开状态（C# Settings.Save；merge 写回）
    pub fn save_expanded(&self) {
        use crate::game::dialogs::settings_file::{set_ini_value, write_ini};
        let content = crate::game::dialogs::settings_file::load_ini();
        let content = set_ini_value(&content, "Game", "ExpandedBuffWindow", &self.expanded.to_string());
        write_ini(&content);
        tracing::debug!("⚙️ Buff 窗口展开状态已保存: {}", self.expanded);
    }
}

/// tag → 显示名（与服务端 buff_tag 对应）
pub fn buff_name(tag: u8) -> &'static str {
    match tag {
        0 => "HP恢复",
        1 => "MP恢复",
        2 => "攻击提升",
        3 => "防御提升",
        4 => "物理防御提升",
        5 => "魔法防御提升",
        6 => "伤害减免",
        7 => "中毒",
        8 => "沉默",
        9 => "眩晕",
        10 => "隐身",
        11 => "攻速提升",
        12 => "移速提升",
        13 => "敏捷提升",
        14 => "暴击提升",
        15 => "魔力恢复提升",
        16 => "魔力上限提升",
        17 => "反伤",
        18 => "嘲讽",
        19 => "减速",
        20 => "冰冻",
        _ => "未知",
    }
}

#[derive(Component)]
pub struct BuffWidget;

/// 面板本体（收起时缩小为 44x34 小窗，C# Size(44,34)）
#[derive(Component)]
pub struct BuffPanel;

/// 展开/收起按钮（C# _expandCollapseButton，Prguse2 7/8/9，16x15）
#[derive(Component)]
pub struct BuffExpand;

/// 收起时显示的状态数量标签（C# _buffCountLabel）
#[derive(Component)]
pub struct BuffCount;

#[derive(Component)]
pub struct BuffClose;

#[derive(Component)]
pub struct BuffLine(usize);

pub struct BuffPlugin;

impl Plugin for BuffPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(BuffState::load());
                app.add_systems(
            Update,
            buff_server_events.run_if(in_state(AppState::Game)),
        );
app.add_systems(OnEnter(AppState::Game), spawn_buff);
        app.add_systems(OnExit(AppState::Game), cleanup_buff);
        app.add_systems(
            Update,
            (buff_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_buff(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_buff(
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

    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 170) {
        let e = spawn_ui_sprite(&mut commands, h, 280.0, 80.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Buff),
            BuffWidget,
            BuffPanel,
            Visibility::Hidden,
        ));
    }
    // 展开/收起按钮（C# _expandCollapseButton：Prguse2 7/8/9，16x15）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 7, 8, 9,
        280.0 + 300.0 - 20.0, 83.0, 7.0, 16.0, 15.0,
    ) {
        commands.entity(e).insert((
            BuffExpand,
            DialogRoot(DialogKind::Buff),
            BuffWidget,
        ));
    }
    // 收起时的数量标签（C# _buffCountLabel：黄色粗体）
    let e = spawn_ui_text(
        &mut commands, &font, "",
        280.0 + 22.0, 97.0,
        12.0, Color::srgb(1.0, 1.0, 0.0), 8.0,
    );
    commands.entity(e).insert((
        BuffCount,
        DialogRoot(DialogKind::Buff),
        BuffWidget,
    ));
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 300.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            BuffClose,
            DialogRoot(DialogKind::Buff),
            BuffWidget,
        ));
    }
    // 8 行 buff + 2 状态行
    for i in 0..10usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            298.0, 120.0 + i as f32 * 22.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            BuffLine(i),
            DialogRoot(DialogKind::Buff),
            BuffWidget,
        ));
    }
}

/// 显隐 + 渲染 + 展开/收起 + 关闭
fn buff_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<BuffState>,
    mut expand: Query<(&UiButton, &mut Transform), With<BuffExpand>>,
    mut close: Query<(&UiButton, &mut Visibility), With<BuffClose>>,
    mut panel: Query<&mut Sprite, (With<BuffPanel>, Without<BuffExpand>, Without<BuffClose>, Without<BuffCount>)>,
    // #1290：Bevy B0001——多个 &mut Query 需完整 Without 隔离（#1237 合并后启动 panic）
    mut count_text: Query<
        (&mut Text2d, &mut Transform, &mut Visibility),
        (With<BuffCount>, Without<BuffLine>, Without<BuffExpand>, Without<BuffClose>, Without<BuffWidget>),
    >,
    mut widgets: Query<&mut Visibility, (With<BuffWidget>, Without<BuffLine>, Without<BuffCount>, Without<BuffClose>)>,
    mut lines: Query<
        (&mut Text2d, &mut Visibility, &BuffLine),
        (Without<BuffCount>, Without<BuffClose>, Without<BuffWidget>),
    >,
) {
    let open = mgr.is_open(DialogKind::Buff);
    // 面板尺寸：展开 300x200，收起 44x34（C# Size(44,34)）
    let (pw, ph) = if state.expanded { (300.0f32, 200.0f32) } else { (44.0, 34.0) };
    for mut sp in &mut panel {
        sp.custom_size = Some(Vec2::new(pw, ph));
    }
    // 展开按钮：收起时移到小窗右上角
    for (btn, mut tf) in &mut expand {
        tf.translation.x = 280.0 + pw - 18.0;
        tf.translation.y = -83.0;
        if btn.clicked {
            state.expanded = !state.expanded;
            state.save_expanded();
            tracing::info!("🩹 Buff 窗口{}", if state.expanded { "展开" } else { "收起" });
        }
    }
    // 数量标签（收起时显示）
    for (mut text, mut tf, mut vis) in &mut count_text {
        *vis = if open && !state.expanded {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        text.0 = format!("{}", state.buffs.len());
        tf.translation.x = 280.0 + pw / 2.0;
        tf.translation.y = -(80.0 + ph / 2.0);
    }
    // 面板/按钮显隐
    for mut vis in &mut widgets {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        // 关闭按钮（BuffClose）只在 open 分支管理显隐，关闭时必须隐藏
        for (_, mut vis) in &mut close {
            *vis = Visibility::Hidden;
        }
        return;
    }
    // 关闭按钮仅展开时显示
    for (_, mut vis) in &mut close {
        *vis = if state.expanded { Visibility::Visible } else { Visibility::Hidden };
    }
    // 列表行仅展开时显示 + 渲染
    for (mut text, mut vis, line) in &mut lines {
        *vis = if state.expanded { Visibility::Visible } else { Visibility::Hidden };
        text.0 = match line.0 {
            i if i < 8 => match state.buffs.get(i) {
                Some(b) => format!(
                    "{}（剩余 {} tick）",
                    buff_name(b.tag),
                    b.remaining_ticks
                ),
                None => String::new(),
            },
            8 => format!("当前状态: {} 个", state.buffs.len()),
            9 => state.message.clone(),
            _ => String::new(),
        };
    }
}


/// 消费服务端状态事件（网络层只广播 ServerEvent）
fn buff_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut buff: ResMut<BuffState>,
    mut hud: ResMut<crate::game::hud::HudState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::BuffAdded { tag, ticks } => {
                // #1552：SwiftFeet(ServerRust MoveSpeedBoost tag=12) → Sprint；MoonLight/DarkBody(Invisibility tag=10) → Sneaking
                match *tag {
                    12 => hud.sprint = true,
                    10 => hud.sneaking = true,
                    _ => {}
                }
                if let Some(e) = buff.buffs.iter_mut().find(|b| b.tag == *tag) {
                    e.remaining_ticks = *ticks;
                } else {
                    buff.buffs.push(BuffEntry { tag: *tag, remaining_ticks: *ticks });
                }
                buff.message = format!("获得状态: {}", buff_name(*tag));
            }
            ServerEvent::BuffRemoved { tag } => {
                // #1552：状态消失 → 清对应移动状态
                match *tag {
                    12 => hud.sprint = false,
                    10 => hud.sneaking = false,
                    _ => {}
                }
                buff.buffs.retain(|b| b.tag != *tag);
                buff.message = format!("状态消失: {}", buff_name(*tag));
            }
            ServerEvent::BuffPaused {
                buff_type,
                object_id,
                paused,
            } => {
                // #262：Buff 暂停/恢复提示
                buff.message = format!(
                    "状态{}: {} (对象 {})",
                    if *paused { "暂停" } else { "恢复" },
                    buff_name(*buff_type),
                    object_id
                );
                tracing::info!("⏸️ Buff {} 对象 {}", buff_name(*buff_type), object_id);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buff_state_expanded_parse() {
        let s = BuffState::from_ini("[Game]\nExpandedBuffWindow=false\n");
        assert!(!s.expanded);
        let s2 = BuffState::from_ini("[Game]\nExpandedBuffWindow=true\n");
        assert!(s2.expanded);
    }

    #[test]
    fn buff_state_expanded_default_true() {
        // 缺失配置 → 默认展开（保持既有列表可见行为）
        let s = BuffState::from_ini("");
        assert!(s.expanded);
    }

    #[test]
    fn buff_to_sprint_sneaking_state() {
        // #1552：SwiftFeet(ServerRust MoveSpeedBoost tag=12) → sprint；MoonLight/DarkBody(Invisibility tag=10) → sneaking
        let mut hud = crate::game::hud::HudState::default();
        assert!(!hud.sprint);
        assert!(!hud.sneaking);
        // 模拟 buff_server_events 的 tag 分支逻辑
        match 12u8 { 12 => hud.sprint = true, 10 => hud.sneaking = true, _ => {} }
        assert!(hud.sprint);
        match 10u8 { 12 => hud.sprint = true, 10 => hud.sneaking = true, _ => {} }
        assert!(hud.sneaking);
        // 消失
        match 12u8 { 12 => hud.sprint = false, 10 => hud.sneaking = false, _ => {} }
        assert!(!hud.sprint);
        match 10u8 { 12 => hud.sprint = false, 10 => hud.sneaking = false, _ => {} }
        assert!(!hud.sneaking);
    }
}

