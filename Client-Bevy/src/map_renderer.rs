// ============================================================================
// MapRenderPlugin - Bevy 地图渲染（里程碑 1）
// ============================================================================
//
// 把 Client-Macroquad 的 MeshMapRenderer 移植为 Bevy 渲染：
// - 每 32x32 格合成一张块纹理（1536x1024），按 Back/Middle/Front 三层分层
// - 每个块生成一个 Sprite，Bevy 自动做视锥剔除
// - 坐标约定与 macroquad 一致：世界 x 向右、y 向下（屏幕空间），
//   sprite 位置做 y 取反以适配 Bevy 的 y 向上坐标系

use bevy::prelude::*;
use bevy::render::camera::{OrthographicProjection, Projection};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::resources::libraries::{resolve_data_path, Libraries};
use crate::resources::map_reader::{resolve_map_path, CellInfo, MapReader};
use crate::resources::mlibrary::ImageInfo;

/// 瓦片尺寸（与 macroquad 版一致）
pub const TILE_WIDTH: f32 = 48.0;
pub const TILE_HEIGHT: f32 = 32.0;
/// 每个块包含的瓦片数
pub const CHUNK_TILES: u32 = 32;
/// 块纹理尺寸
pub const CHUNK_PIXEL_W: u32 = CHUNK_TILES * TILE_WIDTH as u32; // 1536
pub const CHUNK_PIXEL_H: u32 = CHUNK_TILES * TILE_HEIGHT as u32; // 1024

/// 游戏数据资源：库 + 地图
#[derive(Resource, Default)]
pub struct GameData {
    pub libraries: Option<Libraries>,
    pub map: Option<LoadedMap>,
}

/// 已加载地图
pub struct LoadedMap {
    pub name: String,
    pub width: i32,
    pub height: i32,
}

/// 图层
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layer {
    Back,
    Middle,
    Front,
}

impl Layer {
    fn z(self) -> f32 {
        match self {
            Layer::Back => 0.0,
            Layer::Middle => 0.1,
            Layer::Front => 0.2,
        }
    }

    fn tile(self, cell: &CellInfo) -> Option<(i16, i32)> {
        match self {
            Layer::Back => cell.back_tile(),
            Layer::Middle => cell.middle_tile(),
            Layer::Front => cell.front_tile(),
        }
    }
}

pub struct MapRenderPlugin;

impl Plugin for MapRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameData>();
        app.add_systems(Startup, setup_world);
        app.add_systems(Update, camera_control);
    }
}

/// 命令行参数：--map <name>，默认 n0（新手村，macroquad map_viewer 同款地图）
fn map_arg() -> String {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == "--map")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| "n0".to_string())
}

/// i32 向上取整除法（避免依赖不稳定的 int_roundings）
fn div_ceil_i32(a: i32, b: i32) -> i32 {
    if a % b == 0 {
        a / b
    } else {
        a / b + 1
    }
}

fn setup_world(
    mut commands: Commands,
    mut assets: ResMut<Assets<Image>>,
    mut game_data: ResMut<GameData>,
) {
    // 1. 加载图像库（MapLibs）
    let data_path = resolve_data_path();
    tracing::info!("📁 数据目录: {}", data_path.display());
    let mut libraries = Libraries::new(data_path.clone());
    libraries.init_map_libraries();
    let (single, map_libs) = libraries.stats();
    tracing::info!(
        "📚 库加载完成: 单体 {} 个, MapLibs {} 个",
        single,
        map_libs
    );

    // 2. 加载地图
    let map_name = map_arg();
    let map_path = resolve_map_path(&map_name);
    let map = match MapReader::new(&map_path) {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("❌ 地图加载失败 {}: {}", map_path, e);
            commands.spawn(Camera2d);
            return;
        }
    };
    tracing::info!(
        "🗺️ 地图 {} 加载成功: {}x{}",
        map_path,
        map.width,
        map.height
    );

    // 3. 按块生成纹理
    let mut spawned = 0usize;
    let chunks_x = div_ceil_i32(map.width, CHUNK_TILES as i32);
    let chunks_y = div_ceil_i32(map.height, CHUNK_TILES as i32);

    for layer in [Layer::Back, Layer::Middle, Layer::Front] {
        for cy in 0..chunks_y {
            for cx in 0..chunks_x {
                if let Some(handle) =
                    build_chunk(&mut libraries, &map, layer, cx, cy, &mut assets)
                {
                    // 块中心（世界坐标，y 取反适配 Bevy）
                    let rect_x = (cx * CHUNK_TILES as i32) as f32 * TILE_WIDTH;
                    let rect_y = (cy * CHUNK_TILES as i32) as f32 * TILE_HEIGHT;
                    let px = rect_x + CHUNK_PIXEL_W as f32 / 2.0;
                    let py = -(rect_y + CHUNK_PIXEL_H as f32 / 2.0);
                    commands.spawn((
                        Sprite::from_image(handle),
                        Transform::from_xyz(px, py, layer.z()),
                        Visibility::default(),
                    ));
                    spawned += 1;
                }
            }
        }
    }
    tracing::info!("🧩 地图块生成完成: {} 个 Sprite", spawned);

    // 4. 相机（对准地图中心，默认看到约 18x13 格）
    let center_x = map.width as f32 * TILE_WIDTH / 2.0;
    let center_y = -(map.height as f32 * TILE_HEIGHT / 2.0);
    commands.spawn((
        Camera2d,
        Transform::from_xyz(center_x, center_y, 10.0),
        Projection::Orthographic(OrthographicProjection {
            scale: 1.0,
            ..OrthographicProjection::default_2d()
        }),
    ));

    game_data.libraries = Some(libraries);
    game_data.map = Some(LoadedMap {
        name: map_name,
        width: map.width,
        height: map.height,
    });
}

/// 把指定块的三层之一合成一张纹理。块内无任何瓦片时返回 None。
fn build_chunk(
    libraries: &mut Libraries,
    map: &MapReader,
    layer: Layer,
    cx: i32,
    cy: i32,
    assets: &mut Assets<Image>,
) -> Option<Handle<Image>> {
    let mut canvas = vec![0u8; (CHUNK_PIXEL_W * CHUNK_PIXEL_H * 4) as usize];
    let mut any_drawn = false;

    let start_x = cx * CHUNK_TILES as i32;
    let start_y = cy * CHUNK_TILES as i32;
    let end_x = (start_x + CHUNK_TILES as i32).min(map.width);
    let end_y = (start_y + CHUNK_TILES as i32).min(map.height);

    for x in start_x..end_x {
        for y in start_y..end_y {
            let cell = &map.map_cells[x as usize][y as usize];
            let Some((file_index, image_index)) = layer.tile(cell) else {
                continue;
            };
            let Some(info) = libraries.get_map_image(file_index, image_index) else {
                continue;
            };
            let Some(rgba) = info.rgba.as_ref() else {
                continue;
            };
            // 块内相对位置（macroquad 的 offset 规则：图片底边对齐格子底边）
            let dx = (x - start_x) * TILE_WIDTH as i32;
            let dy = (y - start_y) * TILE_HEIGHT as i32 + TILE_HEIGHT as i32 - info.height as i32;
            if blit(&mut canvas, dx, dy, &info, rgba) {
                any_drawn = true;
            }
        }
    }

    if !any_drawn {
        return None;
    }

    let image = make_image(canvas, CHUNK_PIXEL_W, CHUNK_PIXEL_H);
    Some(assets.add(image))
}

/// 把图像 RGBA 拷贝到画布，返回是否有像素被写入
fn blit(
    canvas: &mut [u8],
    dx: i32,
    dy: i32,
    img: &ImageInfo,
    rgba: &[u8],
) -> bool {
    let w = img.width as i32;
    let h = img.height as i32;
    if w <= 0 || h <= 0 {
        return false;
    }
    let mut drawn = false;
    for yy in 0..h {
        let sy = dy + yy;
        if sy < 0 || sy >= CHUNK_PIXEL_H as i32 {
            continue;
        }
        for xx in 0..w {
            let sx = dx + xx;
            if sx < 0 || sx >= CHUNK_PIXEL_W as i32 {
                continue;
            }
            let src = ((yy * w + xx) * 4) as usize;
            if rgba[src + 3] == 0 {
                continue;
            }
            let dst = ((sy * CHUNK_PIXEL_W as i32 + sx) * 4) as usize;
            canvas[dst..dst + 4].copy_from_slice(&rgba[src..src + 4]);
            drawn = true;
        }
    }
    drawn
}

/// 用原始 RGBA 数据构造 Bevy Image 资产
fn make_image(rgba: Vec<u8>, width: u32, height: u32) -> Image {
    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// 相机控制：WASD/方向键平移，+/- 缩放
fn camera_control(
    mut camera: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let Ok((mut transform, mut projection)) = camera.single_mut() else {
        return;
    };
    let Projection::Orthographic(ortho) = projection.as_mut() else {
        return;
    };
    let dt = time.delta_secs();

    let mut pan = Vec3::ZERO;
    let speed = 480.0 * dt;
    if keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::KeyA) {
        pan.x -= speed;
    }
    if keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::KeyD) {
        pan.x += speed;
    }
    if keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::KeyW) {
        pan.y += speed;
    }
    if keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::KeyS) {
        pan.y -= speed;
    }
    transform.translation += pan;

    // 缩放：1.0 = 1 世界单位 ≈ 1 像素
    if keys.pressed(KeyCode::Equal) || keys.pressed(KeyCode::NumpadAdd) {
        ortho.scale = (ortho.scale / 1.02).max(0.02);
    }
    if keys.pressed(KeyCode::Minus) || keys.pressed(KeyCode::NumpadSubtract) {
        ortho.scale = (ortho.scale * 1.02).min(4.0);
    }
}
