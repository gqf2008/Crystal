use crate::mlibrary::MLibrary;
use crate::map_code::MapReader;
use crate::renderer::Renderer;
use ggez::event::MouseButton;
use ggez::graphics::Canvas;
use ggez::input::keyboard::{KeyCode, KeyInput};
use ggez::{Context, GameResult};
use std::time::{Duration, Instant};

pub struct EditorState {
    pub map: Option<MapReader>,
    pub renderer: Renderer,
    pub libraries: Vec<MLibrary>,
    
    // 编辑器状态
    pub current_cell_x: i32,
    pub current_cell_y: i32,
    pub selected_layer: Layer,
    
    // 性能统计
    fps: u32,
    frame_count: u32,
    last_fps_update: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    None,
    BackImage,
    MiddleImage,
    FrontImage,
    BackLimit,
    FrontLimit,
    PlaceObjects,
}

impl EditorState {
    pub fn new(_ctx: &mut Context) -> GameResult<Self> {
        let map = None; // 初始不加载地图
        let renderer = Renderer::new();
        let libraries = Vec::new(); // 稍后加载库
        
        Ok(EditorState {
            map,
            renderer,
            libraries,
            current_cell_x: 0,
            current_cell_y: 0,
            selected_layer: Layer::None,
            fps: 0,
            frame_count: 0,
            last_fps_update: Instant::now(),
        })
    }

    pub fn update(&mut self, ctx: &mut Context) -> GameResult {
        // 更新 FPS
        self.frame_count += 1;
        let now = Instant::now();
        if now.duration_since(self.last_fps_update) >= Duration::from_secs(1) {
            self.fps = self.frame_count;
            self.frame_count = 0;
            self.last_fps_update = now;
            
            // 更新窗口标题
            let (map_w, map_h) = if let Some(ref map) = self.map {
                (map.width, map.height)
            } else {
                (0, 0)
            };
            
            ctx.gfx.set_window_title(&format!(
                "Crystal Map Editor - FPS: {} - Map: {}x{} - Cell: ({}, {})",
                self.fps,
                map_w,
                map_h,
                self.current_cell_x,
                self.current_cell_y
            ));
        }

        // 更新渲染器
        self.renderer.update(ctx);
        
        Ok(())
    }

    pub fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        if let Some(ref map) = self.map {
            self.renderer.render(ctx, canvas, map, &self.libraries)?;
        }
        Ok(())
    }

    pub fn handle_mouse_move(&mut self, x: f32, y: f32) {
        let (cell_x, cell_y) = self.renderer.screen_to_cell(x, y);
        self.current_cell_x = cell_x;
        self.current_cell_y = cell_y;
    }

    pub fn handle_mouse_down(&mut self, button: MouseButton, x: f32, y: f32) {
        let (cell_x, cell_y) = self.renderer.screen_to_cell(x, y);
        
        match button {
            MouseButton::Left => {
                // TODO: 放置图块
                println!("Left click at cell ({}, {})", cell_x, cell_y);
            }
            MouseButton::Right => {
                // TODO: 删除图块
                println!("Right click at cell ({}, {})", cell_x, cell_y);
            }
            _ => {}
        }
    }

    pub fn handle_key_down(&mut self, _ctx: &mut Context, input: KeyInput) {
        if let Some(keycode) = input.keycode {
            match keycode {
                // 移动视口
                KeyCode::W => self.renderer.map_point_y -= 5,
                KeyCode::S => self.renderer.map_point_y += 5,
                KeyCode::A => self.renderer.map_point_x -= 5,
                KeyCode::D => self.renderer.map_point_x += 5,
                
                // 缩放
                KeyCode::Plus | KeyCode::Equals => {
                    self.renderer.zoom = (self.renderer.zoom + 0.1).min(3.0);
                }
                KeyCode::Minus => {
                    self.renderer.zoom = (self.renderer.zoom - 0.1).max(0.3);
                }
                
                // 切换显示选项
                KeyCode::Key1 => self.renderer.show_back = !self.renderer.show_back,
                KeyCode::Key2 => self.renderer.show_middle = !self.renderer.show_middle,
                KeyCode::Key3 => self.renderer.show_front = !self.renderer.show_front,
                KeyCode::G => self.renderer.show_grid = !self.renderer.show_grid,
                KeyCode::F => self.renderer.show_front_tag = !self.renderer.show_front_tag,
                
                _ => {}
            }
        }
    }

    pub fn load_map(&mut self, _ctx: &mut Context, path: &str) -> GameResult {
        match MapReader::new(path) {
            Ok(map) => {
                println!("Map loaded: {}x{}", map.width, map.height);
                self.map = Some(map);
                Ok(())
            }
            Err(e) => {
                eprintln!("Failed to load map: {:?}", e);
                Err(ggez::GameError::FilesystemError(format!("Load failed: {:?}", e)))
            }
        }
    }

    pub fn save_map(&self, _path: &str) -> GameResult {
        if let Some(ref _map) = self.map {
            // TODO: 实现地图保存
            println!("Map save not yet implemented");
            Ok(())
        } else {
            Err(ggez::GameError::FilesystemError("No map to save".to_string()))
        }
    }
}
