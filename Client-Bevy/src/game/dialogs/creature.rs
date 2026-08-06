// ============================================================================
// 宠物对话框（M47）
// 参考：C# IntelligentCreatureDialog + ServerRust hero.rs 宠物系统
// 网络（ServerRust 实际 wire）：
//   C: RequestIntelligentCreatureUpdates[bool u8] / UpdateIntelligentCreature[type u8][pickup u8]
//   S: UpdateIntelligentCreatureList[count i32][per: type u8][pickup u8][enabled u8][hunger u8][name dotnet]
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};

/// 宠物条目
#[derive(Debug, Clone, Default)]
pub struct CreatureEntry {
    pub creature_type: u8,
    pub pickup_mode: u8,
    pub enabled: bool,
    pub hunger: u8,
    pub name: String,
}

/// 宠物状态
#[derive(Resource, Default)]
pub struct CreatureState {
    pub creatures: Vec<CreatureEntry>,
    pub message: String,
}

#[derive(Component)]
pub struct CreatureWidget;

#[derive(Component)]
pub struct CreatureClose;

#[derive(Component)]
pub struct CreatureRefresh;

#[derive(Component)]
pub struct CreatureLine(usize);

pub struct CreaturePlugin;

impl Plugin for CreaturePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CreatureState>();
                app.add_systems(
            Update,
            creature_server_events.run_if(in_state(AppState::Game)),
        );
app.add_systems(OnEnter(AppState::Game), spawn_creature);
        app.add_systems(OnExit(AppState::Game), cleanup_creature);
        app.add_systems(
            Update,
            (creature_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_creature(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_creature(
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
            DialogRoot(DialogKind::Creature),
            CreatureWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 300.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            CreatureClose,
            DialogRoot(DialogKind::Creature),
            CreatureWidget,
        ));
    }
    // 8 行宠物 + 2 状态行
    for i in 0..10usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            298.0, 120.0 + i as f32 * 22.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            CreatureLine(i),
            DialogRoot(DialogKind::Creature),
            CreatureWidget,
        ));
    }
    // 刷新按钮
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        480.0, 345.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            CreatureRefresh,
            DialogRoot(DialogKind::Creature),
            CreatureWidget,
        ));
    }
}

/// 显隐 + 渲染 + 刷新
#[allow(clippy::too_many_arguments)]
fn creature_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<CreatureState>,
    net: Res<NetConnection>,
    close: Query<&UiButton, With<CreatureClose>>,
    refresh_btn: Query<&UiButton, With<CreatureRefresh>>,
    mut widgets: Query<&mut Visibility, With<CreatureWidget>>,
    mut lines: Query<(&mut Text2d, &CreatureLine)>,
    mut requested: Local<bool>,
) {
    let open = mgr.is_open(DialogKind::Creature);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        *requested = false;
        return;
    }
    // 打开瞬间请求宠物列表
    if !*requested {
        *requested = true;
        net.send_packet(&crate::network::CreatureRequestWire { request: true });
        tracing::info!("🐾 请求宠物列表");
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Creature);
        }
    }
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            i if i < 8 => match state.creatures.get(i) {
                Some(c) => format!(
                    "{}（类型 {}）拾取:{} 饥饿:{} {}",
                    if c.name.is_empty() { format!("#{}", c.creature_type) } else { c.name.clone() },
                    c.creature_type,
                    if c.pickup_mode != 0 { "开" } else { "关" },
                    c.hunger,
                    if c.enabled { "启用" } else { "" }
                ),
                None => String::new(),
            },
            8 => format!("宠物: {} 个", state.creatures.len()),
            9 => state.message.clone(),
            _ => String::new(),
        };
    }
    for btn in &refresh_btn {
        if btn.clicked {
            net.send_packet(&crate::network::CreatureRequestWire { request: true });
            state.message = "已请求刷新".to_string();
            tracing::info!("🐾 刷新宠物列表");
        }
    }
}


/// 消费服务端宠物列表事件（网络层只广播 ServerEvent）
fn creature_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut creature: ResMut<CreatureState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::CreatureList { creatures } = ev {
            creature.creatures = creatures.clone();
        }
        if let ServerEvent::CreatureAcquired { creature_type } = ev {
            // #274：获得新宠物
            if !creature
                .creatures
                .iter()
                .any(|c| c.creature_type == *creature_type)
            {
                creature.creatures.push(CreatureEntry {
                    creature_type: *creature_type,
                    ..Default::default()
                });
            }
            creature.message = format!("获得新宠物（type {}）", creature_type);
        }
        if let ServerEvent::CreatureRenameEnabled { can_rename } = ev {
            creature.message = format!("宠物{}重命名", if *can_rename { "可以" } else { "不可" });
        }
        if let ServerEvent::CreaturePickupToggled { enabled } = ev {
            creature.message = format!("宠物拾取模式: {}", if *enabled { "开启" } else { "关闭" });
        }
    }
}
