// ============================================================================
// 查看玩家对话框（M46）
// 参考：C# MainDialogs.InspectDialog + ServerRust send_inspect_packet
// 网络：
//   C: Inspect[object_id u32]（SharedRust，与 gate 兼容）
//   S: PlayerInspect[object_id u32][name dotnet][guild dotnet][level u16][class u8]
//      [gender u8][equip_count u8][per: uid u64][index i32][dura i32][max_dura i32]
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

/// 装备条目
#[derive(Debug, Clone, Default)]
pub struct InspectItem {
    pub unique_id: u64,
    pub item_index: i32,
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
    pub items: Vec<InspectItem>,
    pub message: String,
}

#[derive(Component)]
pub struct InspectWidget;

#[derive(Component)]
pub struct InspectClose;

#[derive(Component)]
pub struct InspectLine(usize);

pub struct InspectPlugin;

impl Plugin for InspectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InspectState>();
        app.add_systems(OnEnter(AppState::Game), spawn_inspect);
        app.add_systems(OnExit(AppState::Game), cleanup_inspect);
        app.add_systems(
            Update,
            (inspect_ui_system, ui_button_system)
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

    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 170) {
        let e = spawn_ui_sprite(&mut commands, h, 280.0, 80.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Inspect),
            InspectWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 300.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            InspectClose,
            DialogRoot(DialogKind::Inspect),
            InspectWidget,
        ));
    }
    // 信息行 10
    for i in 0..10usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            298.0, 120.0 + i as f32 * 22.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            InspectLine(i),
            DialogRoot(DialogKind::Inspect),
            InspectWidget,
        ));
    }
}

/// 显隐 + 渲染 + 关闭
fn inspect_ui_system(
    mut mgr: ResMut<DialogManager>,
    state: Res<InspectState>,
    close: Query<&UiButton, With<InspectClose>>,
    mut widgets: Query<&mut Visibility, With<InspectWidget>>,
    mut lines: Query<(&mut Text2d, &InspectLine)>,
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
    let class = match state.class {
        0 => "战士",
        1 => "法师",
        2 => "道士",
        _ => "未知",
    };
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            0 => format!("{}（{}）Lv.{}", state.name, class, state.level),
            1 => format!("行会: {}", if state.guild.is_empty() { "无" } else { &state.guild }),
            2 => "装备:".to_string(),
            i if (3..9).contains(&i) => match state.items.get(i - 3) {
                Some(it) => format!(
                    "#{}（耐久 {}/{}）",
                    it.item_index, it.current_dura, it.max_dura
                ),
                None => String::new(),
            },
            9 => state.message.clone(),
            _ => String::new(),
        };
    }
}
