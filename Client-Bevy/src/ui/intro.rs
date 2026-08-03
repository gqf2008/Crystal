// ============================================================================
// IntroPlugin - 启动画面（logo 展示 ~2.5s 后进入登录）
// ============================================================================

use bevy::prelude::*;

use crate::map_renderer::{make_image, GameLibraries};
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::theme::{colors, load_cn_font};

pub struct IntroPlugin;

impl Plugin for IntroPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Intro), setup_intro);
        app.add_systems(OnExit(AppState::Intro), cleanup_intro);
        app.add_systems(Update, intro_timer.run_if(in_state(AppState::Intro)));
    }
}

#[derive(Component)]
struct IntroRoot;

fn setup_intro(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut libs: ResMut<GameLibraries>,
    mut fonts: ResMut<Assets<Font>>,
) {
    libs.0.ensure_initialized();
    let font = FontSource::Handle(load_cn_font(&mut fonts));

    commands
        .spawn((
            IntroRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.02, 0.02, 0.03)),
        ))
        .with_children(|root| {
            // Logo 图（Title.Lib[30]）
            let mut logo_added = false;
            if let Some(info) = libs.0.get_image(LibraryName::Title, 30) {
                if let Some(rgba) = info.rgba.clone() {
                    let w = info.width.max(0) as u32;
                    let h = info.height.max(0) as u32;
                    if w > 0 && h > 0 {
                        let handle = images.add(make_image(rgba, w, h));
                        root.spawn((
                            ImageNode { image: handle, ..default() },
                            Node {
                                width: Val::Px(w as f32),
                                height: Val::Px(h as f32),
                                ..default()
                            },
                        ));
                        logo_added = true;
                    }
                }
            }
            // 兜底：文字标题
            if !logo_added {
                root.spawn((
                    Text::new("传 奇 2"),
                    TextFont {
                        font: font.clone(),
                        font_size: FontSize::Px(64.0),
                        ..default()
                    },
                    TextColor(colors::TITLE_GOLD),
                ));
            }
            root.spawn((
                Text::new("Loading…"),
                TextFont {
                    font: font.clone(),
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(colors::GRAY),
            ));
        });
}

fn cleanup_intro(mut commands: Commands, root: Query<Entity, With<IntroRoot>>) {
    for e in root.iter() {
        commands.entity(e).despawn();
    }
}

fn intro_timer(
    mut next: ResMut<NextState<AppState>>,
    time: Res<Time>,
    mut elapsed: Local<f32>,
) {
    *elapsed += time.delta_secs();
    if *elapsed > 2.5 {
        next.set(AppState::Login);
    }
}
