// ============================================================================
// 排名对话框（M9 第 3 批）
// 布局参考：macroquad ranking_dialog.rs
//   - 320x380 面板，(200,150)，10 行 28px
// 网络：Ranking 请求 → 服务器回排名 → 显示
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::network::NetConnection;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{spawn_ui_text, UiEntity, UiFont};

/// 排名条目（服务端 Rankings 包）
#[derive(Debug, Clone, Default)]
pub struct RankEntry {
    pub rank: i32,
    pub player_name: String,
    pub class: u8,
    pub level: i32,
    pub experience: i64,
}

#[derive(Resource, Default)]
pub struct RankingState {
    pub visible: bool,
    pub entries: Vec<RankEntry>,
}

#[derive(Component)]
pub struct RankingWidget;

#[derive(Component)]
pub struct RankingClose;

#[derive(Component)]
pub struct RankingLine(usize);

pub struct RankingPlugin;

impl Plugin for RankingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RankingState>();
        app.add_systems(OnEnter(AppState::Game), spawn_ranking);
        app.add_systems(OnExit(AppState::Game), cleanup_ranking);
        app.add_systems(
            Update,
            (ranking_ui_system,).run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_ranking(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_ranking(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 面板
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    commands.spawn((
        UiEntity,
        DialogRoot(DialogKind::Ranking),
        RankingWidget,
        Sprite {
            image: white.clone(),
            color: Color::srgba(0.12, 0.12, 0.16, 0.95),
            custom_size: Some(Vec2::new(320.0, 380.0)),
            ..default()
        },
        bevy::sprite::Anchor::TOP_LEFT,
        Transform::from_xyz(200.0, -150.0, 8.0),
        Visibility::Hidden,
    ));

    // 标题
    let t = spawn_ui_text(&mut commands, &font, "排行榜", 330.0, 158.0, 16.0, Color::srgb(1.0, 1.0, 0.3), 8.2);
    commands.entity(t).insert((DialogRoot(DialogKind::Ranking), RankingWidget));

    // 10 行
    for i in 0..10usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            210.0, 190.0 + i as f32 * 28.0,
            13.0, Color::WHITE, 8.2,
        );
        commands.entity(e).insert((
            RankingLine(i),
            DialogRoot(DialogKind::Ranking),
            RankingWidget,
        ));
    }
}

fn ranking_ui_system(
    mut mgr: ResMut<DialogManager>,
    ranking: Res<RankingState>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut widgets: Query<&mut Visibility, With<RankingWidget>>,
    mut lines: Query<(&mut Text2d, &RankingLine)>,
    mut requested: Local<bool>,
) {
    let open = ranking.visible || mgr.is_open(DialogKind::Ranking);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        *requested = false;
        return;
    }
    // 打开瞬间请求排行榜（原版 C# RankingDialog.Show → GetRanking）
    if !*requested {
        *requested = true;
        net.send_packet(&mir2_shared::packets::client::misc::GetRanking { rank_index: 0 });
        tracing::info!("🏅 请求排行榜");
    }
    for (mut text, line) in &mut lines {
        text.0 = match ranking.entries.get(line.0) {
            Some(e) => {
                let class = match e.class {
                    0 => "战士",
                    1 => "法师",
                    2 => "道士",
                    _ => "未知",
                };
                format!("#{} {} ({} Lv.{})", e.rank, e.player_name, class, e.level)
            }
            None => String::new(),
        };
    }
    // 点击右上角关闭
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    if mouse.just_pressed(MouseButton::Left) {
        let x = 200.0 + 320.0 - 24.0;
        let y = 150.0 + 4.0;
        if cursor.x >= x && cursor.x <= x + 20.0 && cursor.y >= y && cursor.y <= y + 20.0 {
            mgr.close(DialogKind::Ranking);
        }
    }
}
