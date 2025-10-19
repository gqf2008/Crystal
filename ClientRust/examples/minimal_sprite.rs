// 最简单的Sprite测试 - 完全独立于游戏代码

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "最简Sprite测试".to_string(),
                resolution: (800, 600).into(), // 使用u32
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut clear_color: ResMut<ClearColor>,
) {
    // 设置深蓝色背景
    *clear_color = ClearColor(Color::srgb(0.1, 0.1, 0.15));
    info!("✅ 背景颜色已设置为深蓝色");
    
    // 创建摄像机
    commands.spawn(Camera2d);
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
    info!("✅ 红色方块已创建 (200x200, 位置(0,0))");
    
    info!("========================================");
    info!("如果你看到深蓝色背景和红色方块, Bevy渲染正常!");
    info!("如果只看到深蓝色没有红色方块, 说明Sprite渲染有问题!");
    info!("如果什么都看不到, 说明窗口或显卡有问题!");
    info!("========================================");
}
