// ============================================================================
// MIR2 地图查看器 - Macroquad 版本
// ============================================================================
//
// 功能：
// - 加载 .map 文件
// - 渲染三层瓦片（Back/Middle/Front）
// - 相机控制（WASD 移动，鼠标滚轮缩放）
// - 性能监控（FPS、渲染统计）
//
// 使用方法：
// cargo run --bin demo_map_viewer --no-default-features --features backend-macroquad
// ============================================================================

use byteorder::{LittleEndian, ReadBytesExt};
use macroquad::prelude::*;
use mir2_client::backends::macroquad::SpriteManager;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

// 地图瓦片信息
#[derive(Debug, Clone)]
struct Tile {
    back_index: i16,
    back_image: i32,
    middle_index: i16,
    middle_image: i32,
    front_index: i16,
    front_image: i32,
    back_blend: bool,
    middle_blend: bool,
    front_blend: bool,
    door_index: u8,
    door_offset: u8,
    front_anim_frame: u8,
    front_anim_tick: u8,
    light: u8,
}

// 地图数据
struct MapData {
    width: i32,
    height: i32,
    tiles: Vec<Vec<Tile>>,
}

// 相机
struct Camera {
    x: f32,
    y: f32,
    zoom: f32,
}

impl Camera {
    fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        }
    }

    fn update(&mut self, dt: f32) {
        let speed = 500.0 * dt / self.zoom;

        // WASD 移动
        if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
            self.y -= speed;
        }
        if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
            self.y += speed;
        }
        if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
            self.x -= speed;
        }
        if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
            self.x += speed;
        }

        // 鼠标滚轮缩放
        let (_mouse_wheel_x, mouse_wheel_y) = mouse_wheel();
        if mouse_wheel_y != 0.0 {
            let old_zoom = self.zoom;
            self.zoom *= 1.0 + mouse_wheel_y * 0.1;
            self.zoom = self.zoom.clamp(0.25, 4.0);

            // 保持鼠标位置不变
            let (mouse_x, mouse_y) = mouse_position();
            let screen_width = screen_width();
            let screen_height = screen_height();

            let world_x_before = (mouse_x - screen_width / 2.0) / old_zoom + self.x;
            let world_y_before = (mouse_y - screen_height / 2.0) / old_zoom + self.y;
            let world_x_after = (mouse_x - screen_width / 2.0) / self.zoom + self.x;
            let world_y_after = (mouse_y - screen_height / 2.0) / self.zoom + self.y;

            self.x += world_x_before - world_x_after;
            self.y += world_y_before - world_y_after;
        }
    }

    fn world_to_screen(&self, world_x: f32, world_y: f32) -> (f32, f32) {
        let screen_x = (world_x - self.x) * self.zoom + screen_width() / 2.0;
        let screen_y = (world_y - self.y) * self.zoom + screen_height() / 2.0;
        (screen_x, screen_y)
    }
}

// 加载地图文件 (简化版本，仅支持 Type 2/3)
fn load_map(path: &str) -> Result<MapData, String> {
    let bytes = fs::read(path).map_err(|e| format!("Failed to read map file: {}", e))?;

    let mut cursor = Cursor::new(&bytes);

    // 读取宽高
    let width = cursor.read_i16::<LittleEndian>().unwrap() as i32;
    let height = cursor.read_i16::<LittleEndian>().unwrap() as i32;

    println!("Map size: {}x{}", width, height);

    // 跳过头部（52 字节）
    cursor.set_position(52);

    // 检测地图类型（通过数据大小推测）
    let bytes_per_tile = if bytes.len() == (52 + width * height * 14) as usize {
        14 // Type 2
    } else if bytes.len() == (52 + width * height * 36) as usize {
        36 // Type 3
    } else {
        return Err(format!("Unknown map format, size: {}", bytes.len()));
    };

    println!("Map type: {} bytes per tile", bytes_per_tile);

    // 读取瓦片数据
    let mut tiles = vec![
        vec![
            Tile {
                back_index: 0,
                back_image: 0,
                middle_index: 1,
                middle_image: 0,
                front_index: 2,
                front_image: 0,
                back_blend: false,
                middle_blend: false,
                front_blend: false,
                door_index: 0,
                door_offset: 0,
                front_anim_frame: 0,
                front_anim_tick: 0,
                light: 0,
            };
            height as usize
        ];
        width as usize
    ];

    for x in 0..width {
        for y in 0..height {
            let back_image = cursor.read_i16::<LittleEndian>().unwrap();
            let middle_image = cursor.read_i16::<LittleEndian>().unwrap();
            let front_image = cursor.read_i16::<LittleEndian>().unwrap();

            tiles[x as usize][y as usize].back_image = (back_image & 0x7FFF) as i32;
            tiles[x as usize][y as usize].back_blend = (back_image & 0x8000u16 as i16) != 0;
            
            tiles[x as usize][y as usize].middle_image = (middle_image & 0x7FFF) as i32;
            tiles[x as usize][y as usize].middle_blend = (middle_image & 0x8000u16 as i16) != 0;
            
            tiles[x as usize][y as usize].front_image = (front_image & 0x7FFF) as i32;
            tiles[x as usize][y as usize].front_blend = (front_image & 0x8000u16 as i16) != 0;            if bytes_per_tile >= 14 {
                tiles[x as usize][y as usize].door_index = cursor.read_u8().unwrap();
                tiles[x as usize][y as usize].door_offset = cursor.read_u8().unwrap();
                tiles[x as usize][y as usize].front_anim_frame = cursor.read_u8().unwrap();
                tiles[x as usize][y as usize].front_anim_tick = cursor.read_u8().unwrap();
                cursor.read_u8().unwrap(); // front_index
                tiles[x as usize][y as usize].light = cursor.read_u8().unwrap();
                cursor.read_u8().unwrap(); // back_index
                cursor.read_u8().unwrap(); // middle_index
            }

            if bytes_per_tile == 36 {
                // Type 3 有额外的数据，跳过
                for _ in 0..22 {
                    cursor.read_u8().unwrap();
                }
            }
        }
    }

    Ok(MapData {
        width,
        height,
        tiles,
    })
}

/// macroquad 窗口配置
fn window_conf() -> Conf {
    Conf {
        window_title: "MIR2 地图查看器".to_owned(),
        window_width: 1280,
        window_height: 768,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // 配置
    let data_path = std::env::var("MIR2_DATA").unwrap_or_else(|_| "../Data".to_string());
    let map_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| format!("{}/Map/0.map", data_path));

    println!("Data path: {}", data_path);
    println!("Map path: {}", map_path);

    // 加载地图
    let map = match load_map(&map_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to load map: {}", e);
            return;
        }
    };

    println!("Map loaded: {}x{}", map.width, map.height);

    // 创建精灵管理器
    let mut sprite_manager = SpriteManager::new();
    sprite_manager.set_max_cache_size(1000);
    
    // 加载地图库 (Tiles 和 SmTiles)
    let tiles_path = PathBuf::from(&data_path).join("Map/WemadeMir2/Tiles.lib");
    let smtiles_path = PathBuf::from(&data_path).join("Map/WemadeMir2/SmTiles.lib");
    
    if tiles_path.exists() {
        println!("Loading Tiles.lib...");
        if let Err(e) = sprite_manager.load_library("tiles", &tiles_path) {
            println!("Failed to load Tiles.lib: {}", e);
        }
    }
    
    if smtiles_path.exists() {
        println!("Loading SmTiles.lib...");
        if let Err(e) = sprite_manager.load_library("smtiles", &smtiles_path) {
            println!("Failed to load SmTiles.lib: {}", e);
        }
    }    // 相机
    let mut camera = Camera::new();
    camera.x = (map.width * 48 / 2) as f32;
    camera.y = (map.height * 32 / 2) as f32;

    // 主循环
    let mut show_help = true;
    let mut frame_count = 0;
    let mut fps = 0.0;
    let mut last_fps_time = get_time();

    loop {
        let dt = get_frame_time();

        // 更新相机
        camera.update(dt);

        // 切换帮助
        if is_key_pressed(KeyCode::H) {
            show_help = !show_help;
        }

        // 渲染
        clear_background(BLACK);

        // 计算可见区域
        let cell_width = 48;
        let cell_height = 32;
        let screen_w = screen_width();
        let screen_h = screen_height();

        let visible_width = (screen_w / camera.zoom / cell_width as f32).ceil() as i32 + 2;
        let visible_height = (screen_h / camera.zoom / cell_height as f32).ceil() as i32 + 2;

        let center_grid_x = (camera.x / cell_width as f32) as i32;
        let center_grid_y = (camera.y / cell_height as f32) as i32;

        let start_x = (center_grid_x - visible_width / 2).max(0);
        let end_x = (center_grid_x + visible_width / 2).min(map.width - 1);
        let start_y = (center_grid_y - visible_height / 2).max(0);
        let end_y = (center_grid_y + visible_height / 2).min(map.height - 1);

        let mut tile_count = 0;

        // 渲染瓦片
        for layer in &["back", "middle", "front"] {
            for x in start_x..=end_x {
                for y in start_y..=end_y {
                    let tile = &map.tiles[x as usize][y as usize];

                    let (lib_name, img_idx) = match *layer {
                        "back" if tile.back_image > 0 => ("tiles", tile.back_image as usize),
                        "middle" if tile.middle_image > 0 => {
                            ("smtiles", tile.middle_image as usize)
                        }
                        "front" if tile.front_image > 0 => ("smtiles", tile.front_image as usize),
                        _ => continue,
                    };

                    let world_x = (x * cell_width) as f32;
                    let world_y = (y * cell_height) as f32;
                    let (screen_x, screen_y) = camera.world_to_screen(world_x, world_y);

                    sprite_manager.draw_sprite(
                        lib_name, img_idx, screen_x, screen_y, true, // use_offset
                    );

                    tile_count += 1;
                }
            }
        }

        // 显示信息
        frame_count += 1;
        if get_time() - last_fps_time >= 1.0 {
            fps = frame_count as f64 / (get_time() - last_fps_time);
            frame_count = 0;
            last_fps_time = get_time();
        }

        draw_text(
            &format!(
                "FPS: {:.1} | Tiles: {} | Zoom: {:.2}x",
                fps, tile_count, camera.zoom
            ),
            10.0,
            30.0,
            20.0,
            WHITE,
        );

        draw_text(
            &format!("Camera: ({:.0}, {:.0})", camera.x, camera.y),
            10.0,
            50.0,
            20.0,
            WHITE,
        );

        if show_help {
            let help_text = vec![
                "Controls:",
                "  WASD/Arrow Keys - Move camera",
                "  Mouse Wheel - Zoom in/out",
                "  H - Toggle help",
                "  ESC - Exit",
            ];

            let mut y = screen_height() - 150.0;
            for line in help_text {
                draw_text(line, 10.0, y, 20.0, GREEN);
                y += 25.0;
            }
        }

        // 退出
        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        next_frame().await
    }
}
