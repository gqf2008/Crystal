// ============================================================================
// 状态/增益对话框（M44）
// 参考：C# BuffDialog + ServerRust buff 系统
// 网络（M44 简化 wire，服务端此前无 AddBuff 推送）：
//   S: AddBuff[tag u8][remaining_ticks u32] / RemoveBuff[tag u8]
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetworkContext;
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

#[derive(Component)]
pub struct BuffClose;

#[derive(Component)]
pub struct BuffLine(usize);

pub struct BuffPlugin;

impl Plugin for BuffPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BuffState>();
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
            Visibility::Hidden,
        ));
    }
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

/// 显隐 + 渲染 + 关闭
fn buff_ui_system(
    mut mgr: ResMut<DialogManager>,
    state: Res<BuffState>,
    close: Query<&UiButton, With<BuffClose>>,
    mut widgets: Query<&mut Visibility, With<BuffWidget>>,
    mut lines: Query<(&mut Text2d, &BuffLine)>,
) {
    let open = mgr.is_open(DialogKind::Buff);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Buff);
        }
    }
    for (mut text, line) in &mut lines {
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
