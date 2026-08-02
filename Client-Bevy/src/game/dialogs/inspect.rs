// ============================================================================
// 查看对话框（M9 第 2 批收尾）
// 布局参考：macroquad inspect_dialog.rs / C# MainDialogs.cs InspectDialog
//   - 280x380 面板，位置 (250,140)，标题"查看装备"
//   - 目标名 + 14 装备槽（C# Items[14]）
// 网络：Inspect 请求 → 服务器回 UserInformation/装备 → 显示
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::scenes::AppState;
use crate::ui::sprite_ui::{spawn_ui_text, UiEntity, UiFont};

#[derive(Resource, Default)]
pub struct InspectState {
    pub visible: bool,
    pub target_name: String,
    /// 14 个装备槽（unique_id）
    pub items: [Option<u32>; 14],
}

#[derive(Component)]
pub struct InspectWidget;

#[derive(Component)]
pub struct InspectClose;

#[derive(Component)]
pub struct InspectName;

#[derive(Component)]
pub struct InspectItemLine(usize);

pub struct InspectPlugin;

impl Plugin for InspectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InspectState>();
        app.add_systems(OnEnter(AppState::Game), spawn_inspect);
        app.add_systems(OnExit(AppState::Game), cleanup_inspect);
        app.add_systems(
            Update,
            (inspect_ui_system,).run_if(in_state(AppState::Game)),
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
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 面板（深色半透明）
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    commands.spawn((
        UiEntity,
        DialogRoot(DialogKind::Inspect),
        InspectWidget,
        Sprite {
            image: white.clone(),
            color: Color::srgba(0.12, 0.12, 0.16, 0.95),
            custom_size: Some(Vec2::new(280.0, 380.0)),
            ..default()
        },
        bevy::sprite::Anchor::TOP_LEFT,
        Transform::from_xyz(250.0, -140.0, 8.0),
        Visibility::Hidden,
    ));

    // 标题 + 目标名
    let t = spawn_ui_text(&mut commands, &font, "查看装备", 350.0, 148.0, 16.0, Color::srgb(1.0, 1.0, 0.3), 8.2);
    commands.entity(t).insert((DialogRoot(DialogKind::Inspect), InspectWidget));
    let n = spawn_ui_text(&mut commands, &font, "", 270.0, 172.0, 14.0, Color::srgb(1.0, 1.0, 0.3), 8.2);
    commands.entity(n).insert((
        InspectName,
        DialogRoot(DialogKind::Inspect),
        InspectWidget,
    ));

    // 14 个装备槽行（占位文本）
    let names = ["武器", "护甲", "头盔", "火把", "项链", "左手镯", "右手镯", "左戒指", "右戒指", "护身符", "腰带", "靴子", "宝石", "坐骑"];
    for (i, n) in names.iter().enumerate() {
        let e = spawn_ui_text(
            &mut commands, &font, &format!("{}：", n),
            258.0, 200.0 + i as f32 * 22.0,
            12.0, Color::WHITE, 8.2,
        );
        commands.entity(e).insert((
            InspectItemLine(i),
            DialogRoot(DialogKind::Inspect),
            InspectWidget,
        ));
    }
}

fn inspect_ui_system(
    mut mgr: ResMut<DialogManager>,
    inspect: Res<InspectState>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut widgets: Query<&mut Visibility, With<InspectWidget>>,
    mut names: Query<&mut Text2d, With<InspectName>>,
) {
    let open = inspect.visible || mgr.is_open(DialogKind::Inspect);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    if let Ok(mut t) = names.single_mut() {
        t.0 = inspect.target_name.clone();
    }
    // 关闭：点击面板外（简化：点击右上角 X 区域）
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    if mouse.just_pressed(MouseButton::Left) {
        let x = 250.0 + 280.0 - 24.0;
        let y = 140.0 + 4.0;
        if cursor.x >= x && cursor.x <= x + 20.0 && cursor.y >= y && cursor.y <= y + 20.0 {
            mgr.close(DialogKind::Inspect);
        }
    }
}
