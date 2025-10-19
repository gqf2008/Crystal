// 最简单的Bevy Sprite测试
// 用于验证基础渲染功能

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut clear_color: ResMut<ClearColor>, // Bevy 0.17使用全局ClearColor资源
) {
    // 设置全局背景颜色为深蓝色
    *clear_color = ClearColor(Color::srgb(0.1, 0.1, 0.15));
    
    // 创建摄像机
    commands.spawn(Camera2d);
    
    println!("📷 摄像机已创建，背景设置为深蓝色");
    
    // 创建红色纹理
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    
    let mut red_data = vec![255u8; 200 * 200 * 4];
    for i in 0..(200 * 200) {
        red_data[i * 4] = 255;     // R
        red_data[i * 4 + 1] = 0;   // G
        red_data[i * 4 + 2] = 0;   // B
        red_data[i * 4 + 3] = 255; // A
    }
    
    let red_image = Image::new(
        Extent3d {
            width: 200,
            height: 200,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        red_data,
        TextureFormat::Rgba8UnormSrgb,
        Default::default(),
    );
    
    let red_texture = images.add(red_image);
    
    // 创建红色精灵
    commands.spawn((
        Sprite::from_image(red_texture),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
    
    println!("🔴 红色精灵已创建 (200x200, 位置: 0,0)");
}
