// 测试 GameState + Sprite 是否正常工作

use bevy::prelude::*;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
enum TestState {
    #[default]
    Loading,
    Game,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "GameState测试".to_string(),
                resolution: (800, 600).into(),
                ..default()
            }),
            ..default()
        }))
        .init_state::<TestState>()
        .add_systems(Startup, go_to_game)
        .add_systems(OnEnter(TestState::Game), setup_game)
        .run();
}

fn go_to_game(mut next_state: ResMut<NextState<TestState>>) {
    info!("🔄 切换到Game状态");
    next_state.set(TestState::Game);
}

fn setup_game(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut clear_color: ResMut<ClearColor>,
) {
    info!("🎨 初始化Game场景");
    
    // 设置深蓝色背景
    *clear_color = ClearColor(Color::srgb(0.1, 0.1, 0.15));
    info!("✅ 背景颜色已设置");
    
    // 创建摄像机
    commands.spawn((Camera2d, Name::new("GameCamera")));
    info!("✅ 摄像机已创建");
    
    // 创建红色纹理
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    
    let size = 200;
    let mut red_data = Vec::with_capacity(size * size * 4);
    for _ in 0..(size * size) {
        red_data.push(255); // R
        red_data.push(0);   // G
        red_data.push(0);   // B
        red_data.push(255); // A
    }
    
    let red_image = Image::new(
        Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        red_data,
        TextureFormat::Rgba8UnormSrgb,
        Default::default(),
    );
    
    let red_texture = images.add(red_image);
    
    // 创建红色方块
    commands.spawn((
        Sprite::from_image(red_texture),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Name::new("RED-SQUARE"),
    ));
    info!("✅ 红色方块已创建");
    
    info!("========================================");
    info!("这个测试模拟了游戏的State切换");
    info!("如果能看到红色方块，说明State系统没问题");
    info!("========================================");
}
