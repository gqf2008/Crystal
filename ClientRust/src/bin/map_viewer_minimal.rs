// 最小化地图查看器 - 逐步验证每个功能
// 只加载一个库，只画 Back 层，不翻转坐标

use macroquad::prelude::*;
use mir2_client::backends::macroquad::SpriteManager;
use mir2_client::objects::MapReader;

const WINDOW_WIDTH: i32 = 1024;
const WINDOW_HEIGHT: i32 = 768;
const TILE_WIDTH: f32 = 96.0;   // Back 层瓦片宽度
const TILE_HEIGHT: f32 = 64.0;  // Back 层瓦片高度

fn window_conf() -> Conf {
    Conf {
        window_title: "最小化地图查看器 - 调试版".to_owned(),
        window_width: WINDOW_WIDTH,
        window_height: WINDOW_HEIGHT,
        window_resizable: false,
        ..Default::default()
    }
}

struct MinimalViewer {
    sprite_manager: SpriteManager,
    map_reader: Option<MapReader>,
    camera_x: f32,
    camera_y: f32,
    dragging: bool,
    last_mouse_pos: Vec2,
}

impl MinimalViewer {
    fn new() -> Result<Self, String> {
        println!("🗺️ 最小化地图查看器启动");
        
        // 加载地图
        let map_reader = match MapReader::new("Map/n0.map") {
            Ok(reader) => {
                println!("✅ 地图加载成功: {}x{}", reader.width, reader.height);
                Some(reader)
            }
            Err(e) => {
                println!("⚠️ 地图加载失败: {}", e);
                return Err(format!("地图加载失败: {}", e));
            }
        };
        
        // 创建精灵管理器，加载所有需要的库
        let mut sprite_manager = SpriteManager::new();
        
        // 加载 WeMade 原始库
        if let Err(e) = sprite_manager.load_library("MapLib_0", "Data/Map/WemadeMir2/Tiles.Lib") {
            println!("⚠️ 加载 MapLib_0 失败: {}", e);
        } else {
            println!("✅ 加载库: MapLib_0 (WeMade Tiles)");
        }
        
        if let Err(e) = sprite_manager.load_library("MapLib_1", "Data/Map/WemadeMir2/SmTiles.Lib") {
            println!("⚠️ 加载 MapLib_1 失败: {}", e);
        } else {
            println!("✅ 加载库: MapLib_1 (WeMade SmTiles)");
        }
        
        // 加载 ShandaMir2 扩展库 (102-108)
        for i in 102..=108 {
            let lib_name = format!("MapLib_{}", i);
            let file_path = format!("Data/Map/ShandaMir2/Tiles{}.Lib", i - 100);
            if let Err(e) = sprite_manager.load_library(&lib_name, &file_path) {
                println!("⚠️ 加载 {} 失败: {}", lib_name, e);
            } else {
                println!("✅ 加载库: {} (ShandaMir2 Tiles{})", lib_name, i - 100);
            }
        }
        
        Ok(Self {
            sprite_manager,
            map_reader,
            camera_x: 0.0,
            camera_y: 0.0,
            dragging: false,
            last_mouse_pos: vec2(0.0, 0.0),
        })
    }
    
    fn update(&mut self) {
        // 鼠标拖拽
        if is_mouse_button_pressed(MouseButton::Left) {
            self.dragging = true;
            self.last_mouse_pos = vec2(mouse_position().0, mouse_position().1);
        }
        if is_mouse_button_released(MouseButton::Left) {
            self.dragging = false;
        }
        if self.dragging {
            let current_pos = vec2(mouse_position().0, mouse_position().1);
            let delta = current_pos - self.last_mouse_pos;
            self.camera_x -= delta.x;
            self.camera_y -= delta.y;
            self.last_mouse_pos = current_pos;
            
            println!("📍 相机移动到: ({:.0}, {:.0})", self.camera_x, self.camera_y);
        }
    }
    
    fn draw(&mut self) {
        clear_background(Color::from_rgba(40, 40, 50, 255));
        
        let map = match &self.map_reader {
            Some(m) => m,
            None => {
                draw_text("地图未加载", 400.0, 400.0, 40.0, RED);
                return;
            }
        };
        
        // 计算要绘制的格子范围（只画屏幕可见区域）
        let start_x = ((self.camera_x / TILE_WIDTH).floor() as i32).max(0);
        let start_y = ((self.camera_y / TILE_HEIGHT).floor() as i32).max(0);
        let end_x = (((self.camera_x + WINDOW_WIDTH as f32) / TILE_WIDTH).ceil() as i32).min(map.width as i32);
        let end_y = (((self.camera_y + WINDOW_HEIGHT as f32) / TILE_HEIGHT).ceil() as i32).min(map.height as i32);
        
        let mut tile_count = 0;
        let mut drawn_count = 0;
        let mut non_zero_tiles = 0;
        let mut other_lib_tiles = 0;
        
        // 遍历格子（Back 层每2x2格子一个瓦片）
        for y in (start_y..end_y).step_by(2) {
            for x in (start_x..end_x).step_by(2) {
                // 检查边界
                if x >= map.width as i32 || y >= map.height as i32 {
                    continue;
                }
                
                // 直接访问，不翻转 Y（先用最简单的方式）
                let cell = &map.map_cells[x as usize][y as usize];
                
                // 获取 Back 层瓦片信息
                if let Some((file_index, image_index)) = cell.back_tile() {
                    tile_count += 1;
                    non_zero_tiles += 1;
                    
                    // 调试：打印前几个找到的瓦片信息
                    if non_zero_tiles <= 10 {
                        println!("🔍 找到瓦片 #{}: 格子({}, {}) file_index={} img={}", 
                            non_zero_tiles, x, y, file_index, image_index);
                    }
                    
                    // 尝试从对应的库中获取精灵
                    let lib_name = format!("MapLib_{}", file_index);
                    if let Some(sprite) = self.sprite_manager.get_or_create_sprite(&lib_name, image_index as usize) {
                        // 计算屏幕坐标（直接用格子坐标 * 瓦片尺寸 - 相机偏移）
                        let screen_x = x as f32 * TILE_WIDTH - self.camera_x;
                        let screen_y = y as f32 * TILE_HEIGHT - self.camera_y;
                        
                        // 绘制
                        draw_texture(&sprite.texture, screen_x, screen_y, WHITE);
                        drawn_count += 1;
                        
                        // 调试：打印前几个绘制的瓦片
                        if drawn_count <= 5 {
                            println!("🎨 绘制瓦片 #{}: 格子({}, {}) file_index={} img={} 屏幕位置({:.0}, {:.0})", 
                                drawn_count, x, y, file_index, image_index, screen_x, screen_y);
                        }
                    }
                }
            }
        }
        
        // UI 信息
        draw_text(
            &format!("相机: ({:.0}, {:.0})", self.camera_x, self.camera_y),
            10.0, 30.0, 30.0, WHITE
        );
        draw_text(
            &format!("格子范围: [{}, {}] x [{}, {}]", start_x, end_x, start_y, end_y),
            10.0, 60.0, 20.0, WHITE
        );
        draw_text(
            &format!("Tiles found: {} | drawn: {}", non_zero_tiles, drawn_count),
            10.0, 90.0, 20.0, if drawn_count > 0 { GREEN } else { RED }
        );
        draw_text("Drag mouse | ESC quit", 10.0, screen_height() - 20.0, 20.0, YELLOW);
        
        // 如果没有绘制任何瓦片，显示提示
        if drawn_count == 0 && non_zero_tiles > 0 {
            draw_text("Tiles found but failed to load sprites", 300.0, 400.0, 30.0, YELLOW);
            draw_text("Check if all MapLib files are loaded", 250.0, 440.0, 20.0, YELLOW);
        } else if non_zero_tiles == 0 {
            draw_text("No tiles found in this area", 300.0, 400.0, 30.0, RED);
            draw_text("Try dragging to explore the map", 250.0, 440.0, 20.0, YELLOW);
        }
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut viewer = match MinimalViewer::new() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("❌ 初始化失败: {}", e);
            return;
        }
    };
    
    println!("✅ 启动成功");
    println!("🎮 控制:");
    println!("  - 拖拽鼠标移动相机");
    println!("  - ESC 退出");
    
    loop {
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        
        viewer.update();
        viewer.draw();
        
        next_frame().await;
    }
    
    println!("✅ 程序退出");
}
