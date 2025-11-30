// ============================================================================
// MiniMapDialogHybrid - 小地图对话框（混合版本）
// ============================================================================
//
// 【实现方式】
// - 使用 macroquad 原生 draw_* 函数绘制
// - 使用 DragHelper 实现拖拽功能
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::DragHelper;

/// 地图上的对象类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MapObjectType {
    Player,      // 玩家
    OtherPlayer, // 其他玩家
    NPC,         // NPC
    Monster,     // 怪物
    Item,        // 掉落物品
    Portal,      // 传送点
}

/// 地图对象
#[derive(Debug, Clone)]
pub struct MapObject {
    pub obj_type: MapObjectType,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub level: Option<u32>,
    pub hostile: bool,
}

/// 小地图对话框（混合版本）
pub struct MiniMapDialogHybrid {
    /// 窗口位置
    position: Vec2,
    /// 是否可见
    visible: bool,
    /// 地图名称
    map_name: String,
    /// 地图尺寸（像素）
    map_size: (f32, f32),
    /// 当前缩放级别
    zoom_level: f32,
    /// 地图视图偏移
    view_offset: Vec2,
    /// 玩家位置
    player_pos: (f32, f32),
    /// 玩家朝向（角度）
    player_direction: f32,
    /// 地图上的对象列表
    map_objects: Vec<MapObject>,
    /// 是否显示玩家名称
    show_player_names: bool,
    /// 是否显示怪物
    show_monsters: bool,
    /// 是否显示NPC
    show_npcs: bool,
    /// 是否为大模式（true=大模式2090、false=小模式2091）
    big_mode: bool,
    /// 背景纹理
    bg_texture: Option<Texture2D>,
    /// 当前对话框尺寸
    current_size: Vec2,
    /// 拖拽辅助器
    drag_helper: DragHelper,
}

impl MiniMapDialogHybrid {
    pub fn new() -> Self {
        let screen_w = screen_width() / screen_dpi_scale();
        
        // 创建一些示例地图对象
        let map_objects = vec![
            MapObject {
                obj_type: MapObjectType::Player,
                name: "我".to_string(),
                x: 512.0,
                y: 384.0,
                level: Some(45),
                hostile: false,
            },
            MapObject {
                obj_type: MapObjectType::OtherPlayer,
                name: "战士001".to_string(),
                x: 600.0,
                y: 400.0,
                level: Some(38),
                hostile: false,
            },
            MapObject {
                obj_type: MapObjectType::NPC,
                name: "武器店老板".to_string(),
                x: 480.0,
                y: 350.0,
                level: None,
                hostile: false,
            },
            MapObject {
                obj_type: MapObjectType::Monster,
                name: "僵尸".to_string(),
                x: 400.0,
                y: 450.0,
                level: Some(25),
                hostile: true,
            },
            MapObject {
                obj_type: MapObjectType::Portal,
                name: "传送点".to_string(),
                x: 300.0,
                y: 200.0,
                level: None,
                hostile: false,
            },
        ];

        Self {
            position: vec2(screen_w - 126.0, 0.0),
            visible: true,
            map_name: "比奇城".to_string(),
            map_size: (1024.0, 768.0),
            zoom_level: 1.0,
            view_offset: Vec2::ZERO,
            player_pos: (512.0, 384.0),
            player_direction: 0.0,
            map_objects,
            show_player_names: true,
            show_monsters: true,
            show_npcs: true,
            big_mode: true,
            bg_texture: None,
            current_size: vec2(124.0, 150.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 显示对话框
    pub fn open(&mut self) {
        self.visible = true;
    }

    /// 关闭对话框
    pub fn close(&mut self) {
        self.visible = false;
    }

    /// 切换显示状态
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// 是否可见
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// 设置位置
    pub fn set_position(&mut self, pos: Vec2) {
        self.position = pos;
    }

    /// 获取位置
    pub fn get_position(&self) -> Vec2 {
        self.position
    }

    /// 检查点是否在对话框内
    pub fn contains(&self, point: Vec2) -> bool {
        if !self.visible {
            return false;
        }
        point.x >= self.position.x
            && point.x <= self.position.x + self.current_size.x
            && point.y >= self.position.y
            && point.y <= self.position.y + self.current_size.y
    }

    /// 切换大小模式
    pub fn toggle_size(&mut self) {
        self.big_mode = !self.big_mode;
        self.bg_texture = None; // 重新加载纹理
        println!(
            "🗺️ 小地图模式: {}",
            if self.big_mode { "大模式" } else { "小模式" }
        );
    }

    /// 是否为大模式
    pub fn is_big_mode(&self) -> bool {
        self.big_mode
    }

    /// 异步加载纹理
    pub async fn load_textures(&mut self) {
        // 预加载小地图纹理
        for idx in [2090, 2091, 2102, 2103, 2104] {
            let _ = LibraryName::Prguse.get_texture(idx);
        }
    }

    /// 世界坐标转换为小地图坐标
    fn world_to_minimap(&self, world_x: f32, world_y: f32, map_rect: Rect) -> Vec2 {
        let scale_x = map_rect.w / self.map_size.0 * self.zoom_level;
        let scale_y = map_rect.h / self.map_size.1 * self.zoom_level;

        let mini_x = map_rect.x + world_x * scale_x - self.view_offset.x;
        let mini_y = map_rect.y + world_y * scale_y - self.view_offset.y;

        vec2(mini_x, mini_y)
    }

    /// 更新和绘制
    pub fn update_and_draw(&mut self) {
        if !self.visible {
            return;
        }

        let mouse_pos = vec2(mouse_position().0, mouse_position().1);

        // 获取背景纹理尺寸
        let texture_index = if self.big_mode { 2090 } else { 2091 };
        if let Some(texture) = LibraryName::Prguse.get_texture(texture_index) {
            self.current_size = vec2(texture.width as f32, texture.height as f32);
            if let Some(tex) = texture.image {
                self.bg_texture = Some(tex);
            }
        }

        // 使用 DragHelper 实现拖拽
        let drag_area = Rect::new(self.position.x, self.position.y, self.current_size.x, 20.0);
        self.drag_helper.apply(drag_area, &mut self.position);

        // 绘制背景
        self.draw_background();

        // 绘制地图区域
        self.draw_map(mouse_pos);

        // 绘制Toggle按钮
        self.draw_toggle_button(mouse_pos);

        // 绘制关闭按钮
        if self.draw_close_button(mouse_pos) {
            self.close();
        }
    }

    /// 绘制背景
    fn draw_background(&self) {
        if let Some(ref texture) = self.bg_texture {
            draw_texture_ex(
                texture,
                self.position.x,
                self.position.y,
                WHITE,
                DrawTextureParams::default(),
            );
        } else {
            // 降级：使用纯色背景
            draw_rectangle(
                self.position.x,
                self.position.y,
                self.current_size.x,
                self.current_size.y,
                Color::from_rgba(40, 45, 50, 250),
            );
            draw_rectangle_lines(
                self.position.x,
                self.position.y,
                self.current_size.x,
                self.current_size.y,
                2.0,
                Color::from_rgba(150, 150, 150, 255),
            );
        }
    }

    /// 绘制地图区域
    fn draw_map(&mut self, _mouse_pos: Vec2) {
        // 地图显示区域
        let map_rect = Rect::new(
            self.position.x + 3.0,
            self.position.y + 22.0,
            self.current_size.x - 6.0,
            108.0,
        );

        // 地图背景
        draw_rectangle(
            map_rect.x,
            map_rect.y,
            map_rect.w,
            map_rect.h,
            Color::from_rgba(30, 40, 30, 255),
        );

        // 绘制网格
        self.draw_grid(map_rect);

        // 绘制地图对象
        self.draw_map_objects(map_rect);
    }

    /// 绘制网格
    fn draw_grid(&self, map_rect: Rect) {
        let grid_size = 20.0 * self.zoom_level;
        let grid_color = Color::from_rgba(100, 100, 100, 60);

        // 垂直线
        let mut x = map_rect.x;
        while x < map_rect.x + map_rect.w {
            draw_line(x, map_rect.y, x, map_rect.y + map_rect.h, 0.5, grid_color);
            x += grid_size;
        }

        // 水平线
        let mut y = map_rect.y;
        while y < map_rect.y + map_rect.h {
            draw_line(map_rect.x, y, map_rect.x + map_rect.w, y, 0.5, grid_color);
            y += grid_size;
        }
    }

    /// 绘制地图对象
    fn draw_map_objects(&self, map_rect: Rect) {
        for obj in &self.map_objects {
            // 根据设置过滤对象
            match obj.obj_type {
                MapObjectType::Monster if !self.show_monsters => continue,
                MapObjectType::NPC if !self.show_npcs => continue,
                _ => {}
            }

            let pos = self.world_to_minimap(obj.x, obj.y, map_rect);

            // 检查是否在可见区域内
            if pos.x < map_rect.x || pos.x > map_rect.x + map_rect.w ||
               pos.y < map_rect.y || pos.y > map_rect.y + map_rect.h {
                continue;
            }

            let (color, size) = match obj.obj_type {
                MapObjectType::Player => (Color::from_rgba(0, 255, 0, 255), 6.0),
                MapObjectType::OtherPlayer => {
                    if obj.hostile {
                        (Color::from_rgba(255, 100, 100, 255), 5.0)
                    } else {
                        (Color::from_rgba(100, 100, 255, 255), 5.0)
                    }
                }
                MapObjectType::NPC => (Color::from_rgba(255, 255, 0, 255), 4.0),
                MapObjectType::Monster => (Color::from_rgba(255, 0, 0, 255), 4.0),
                MapObjectType::Item => (Color::from_rgba(255, 215, 0, 255), 3.0),
                MapObjectType::Portal => (Color::from_rgba(255, 0, 255, 255), 5.0),
            };

            // 绘制对象（圆点）
            draw_circle(pos.x, pos.y, size / 2.0, color);

            // 绘制玩家朝向
            if obj.obj_type == MapObjectType::Player {
                let dir_end = pos + vec2(
                    self.player_direction.cos() * 8.0,
                    self.player_direction.sin() * 8.0,
                );
                draw_line(pos.x, pos.y, dir_end.x, dir_end.y, 2.0, Color::from_rgba(0, 255, 0, 255));
            }

            // 显示名称
            if self.show_player_names {
                draw_text_cn(&obj.name, pos.x - 10.0, pos.y - 8.0, 10.0, WHITE);
            }
        }
    }

    /// 绘制Toggle按钮
    fn draw_toggle_button(&mut self, mouse_pos: Vec2) {
        let button_pos = vec2(self.position.x + 109.0, self.position.y + 3.0);
        let button_size = vec2(12.0, 12.0);
        let button_rect = Rect::new(button_pos.x, button_pos.y, button_size.x, button_size.y);

        let is_hovered = button_rect.contains(mouse_pos);

        // 尝试使用纹理
        let texture_idx = if is_mouse_button_down(MouseButton::Left) && is_hovered {
            2104
        } else if is_hovered {
            2103
        } else {
            2102
        };

        if let Some(texture) = LibraryName::Prguse.get_texture(texture_idx) {
            if let Some(ref tex) = texture.image {
                draw_texture_ex(
                    tex,
                    button_pos.x,
                    button_pos.y,
                    WHITE,
                    DrawTextureParams::default(),
                );
            }
        } else {
            // 降级
            let color = if is_hovered {
                Color::from_rgba(100, 100, 150, 255)
            } else {
                Color::from_rgba(80, 80, 100, 255)
            };
            draw_rectangle(button_pos.x, button_pos.y, button_size.x, button_size.y, color);
        }

        // 点击切换大小
        if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
            self.toggle_size();
        }
    }

    /// 绘制关闭按钮（返回是否点击）
    fn draw_close_button(&self, mouse_pos: Vec2) -> bool {
        let close_size = 15.0;
        let close_x = self.position.x + self.current_size.x - 20.0;
        let close_y = self.position.y + 5.0;
        let close_rect = Rect::new(close_x, close_y, close_size, close_size);

        let is_hovered = close_rect.contains(mouse_pos);

        let bg_color = if is_hovered {
            Color::from_rgba(200, 70, 70, 255)
        } else {
            Color::from_rgba(150, 50, 50, 255)
        };
        draw_rectangle(close_x, close_y, close_size, close_size, bg_color);

        draw_text("×", close_x + 3.0, close_y + 12.0, 14.0, WHITE);

        is_hovered && is_mouse_button_pressed(MouseButton::Left)
    }

    /// 更新玩家位置
    pub fn update_player_position(&mut self, x: f32, y: f32, direction: f32) {
        self.player_pos = (x, y);
        self.player_direction = direction;

        if let Some(player_obj) = self
            .map_objects
            .iter_mut()
            .find(|obj| obj.obj_type == MapObjectType::Player)
        {
            player_obj.x = x;
            player_obj.y = y;
        }
    }
}
