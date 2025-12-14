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
use crate::coord::Coord;
use super::native_ui_utils::{
    DragHelper, draw_library_button_with_offset, draw_library_image_with_offset,
};

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
    /// 地图尺寸（格子数）
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
    /// 是否有新邮件提示（对应 C# NewMail.Visible）
    has_new_mail: bool,
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
            has_new_mail: false,
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

        // 对齐 C#：切换 Index(2090/2091) 会改变 Size；这里立即刷新纹理与尺寸，
        // 避免出现“切换后仍显示旧纹理/旧尺寸”的一帧延迟或缓存残留。
        let old_size = self.current_size;
        let right = self.position.x + old_size.x;

        let texture_index = if self.big_mode { 2090 } else { 2091 };
        if let Some(texture) = LibraryName::Prguse.get_texture(texture_index) {
            self.current_size = vec2(texture.width as f32, texture.height as f32);
            self.bg_texture = texture.image;
        } else {
            self.bg_texture = None;
        }

        // 维持右侧对齐（更接近 C# 右上角固定的视觉效果）
        self.position.x = right - self.current_size.x;
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
        // map_size 以“格子尺寸(宽高=地图格数)”为单位；这里先把世界像素换算成格子坐标再映射。
        let scale_x = map_rect.w / self.map_size.0.max(1.0) * self.zoom_level;
        let scale_y = map_rect.h / self.map_size.1.max(1.0) * self.zoom_level;

        let grid_x = world_x / crate::coord::CELL_WIDTH as f32;
        let grid_y = world_y / crate::coord::CELL_HEIGHT as f32;

        let mini_x = map_rect.x + grid_x * scale_x - self.view_offset.x;
        let mini_y = map_rect.y + grid_y * scale_y - self.view_offset.y;

        vec2(mini_x, mini_y)
    }

    /// 设置地图尺寸（单位：格子数 width/height），用于小地图坐标映射/点击反算
    pub fn set_world_size(&mut self, grid_w: f32, grid_h: f32) {
        // 防御：避免除零
        self.map_size = (grid_w.max(1.0), grid_h.max(1.0));
    }

    /// 若鼠标点击在小地图“地图区域”内，返回目标世界坐标（像素）
    pub fn pick_world_target_from_mouse(&self, mouse_pos: Vec2) -> Option<(f32, f32)> {
        if !self.visible || !self.big_mode {
            return None;
        }

        // 与 draw_map 使用的区域保持一致
        let map_rect = Rect::new(
            self.position.x + 3.0,
            self.position.y + 22.0,
            120.0,
            108.0,
        );

        if mouse_pos.x < map_rect.x
            || mouse_pos.x > map_rect.x + map_rect.w
            || mouse_pos.y < map_rect.y
            || mouse_pos.y > map_rect.y + map_rect.h
        {
            return None;
        }

        let zoom = self.zoom_level.max(0.0001);
        let scale_x = map_rect.w / self.map_size.0 * zoom;
        let scale_y = map_rect.h / self.map_size.1 * zoom;

        // 反算得到“格子坐标”（浮点），再转回世界像素坐标
        let grid_x = (mouse_pos.x - map_rect.x + self.view_offset.x) / scale_x;
        let grid_y = (mouse_pos.y - map_rect.y + self.view_offset.y) / scale_y;

        let gx = grid_x.floor().clamp(0.0, self.map_size.0 - 1.0) as i32;
        let gy = grid_y.floor().clamp(0.0, self.map_size.1 - 1.0) as i32;
        let (wx, wy) = Coord::grid_to_world_center(gx, gy);
        Some((wx, wy))
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

        // 绘制标题/标签（对齐 C# MapNameLabel / LocationLabel）
        self.draw_labels();

        // 绘制地图区域
        // 对齐 C#：小模式(Index=2091)不绘制 MiniMap 内容（仅显示小框/按钮）。
        if self.big_mode {
            self.draw_map(mouse_pos);
        }

        // 绘制底部控件（对齐 C# MailButton / BigMapButton / LightSetting / ToggleButton）
        self.draw_bottom_controls(mouse_pos);
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
            120.0,
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

    fn draw_labels(&self) {
        // MapNameLabel: Location (2,2), Size(120,18), 居中
        let name_rect = Rect::new(self.position.x + 2.0, self.position.y + 2.0, 120.0, 18.0);
        let name = self.map_name.as_str();
        let name_x = name_rect.x + name_rect.w / 2.0 - (name.chars().count() as f32) * 6.0 / 2.0;
        draw_text_cn(name, name_x, name_rect.y + 14.0, 12.0, WHITE);

        // LocationLabel: Location (46, y), Size(56,18), 居中
        // 对齐 C#：y = Size.Height - 23
        let bottom_y = (self.current_size.y - 23.0).max(0.0);
        let loc_rect = Rect::new(self.position.x + 46.0, self.position.y + bottom_y, 56.0, 18.0);
        let loc_text = format!("{},{}", self.player_pos.0 as i32, self.player_pos.1 as i32);
        let loc_x = loc_rect.x + loc_rect.w / 2.0 - (loc_text.chars().count() as f32) * 6.0 / 2.0;
        draw_text_cn(&loc_text, loc_x, loc_rect.y + 14.0, 12.0, WHITE);
    }

    fn draw_bottom_controls(&mut self, mouse_pos: Vec2) {
        // 对齐 C#：y = Size.Height - 23
        let bottom_y = (self.current_size.y - 23.0).max(0.0);

        // MailButton: (4,y) Prguse[2099/2100/2101]
        if draw_library_button_with_offset(
            LibraryName::Prguse,
            [2099, 2100, 2101],
            vec2(self.position.x + 4.0, self.position.y + bottom_y),
            mouse_pos,
        ) {
            println!("📮 MiniMap: MailButton clicked (stub)");
        }

        // NewMail icon: Prguse[544] at (5,y+1) when visible
        if self.has_new_mail {
            let _ = draw_library_image_with_offset(
                LibraryName::Prguse,
                544,
                vec2(self.position.x + 5.0, self.position.y + bottom_y + 1.0),
                WHITE,
            );
        }

        // BigMapButton: (25,y) Prguse[2096/2097/2098]
        if draw_library_button_with_offset(
            LibraryName::Prguse,
            [2096, 2097, 2098],
            vec2(self.position.x + 25.0, self.position.y + bottom_y),
            mouse_pos,
        ) {
            println!("🗺️ MiniMap: BigMapButton clicked (stub)");
        }

        // LightSetting image: Prguse[2093] at (102,y)
        let _ = draw_library_image_with_offset(
            LibraryName::Prguse,
            2093,
            vec2(self.position.x + 102.0, self.position.y + bottom_y),
            WHITE,
        );

        // ToggleButton: (109,3) Prguse[2102/2103/2104]
        if draw_library_button_with_offset(
            LibraryName::Prguse,
            [2102, 2103, 2104],
            vec2(self.position.x + 109.0, self.position.y + 3.0),
            mouse_pos,
        ) {
            self.toggle_size();
        }
    }

    /// 绘制网格
    fn draw_grid(&self, map_rect: Rect) {
        let grid_color = Color::from_rgba(50, 80, 50, 120);
        let grid_size = 12.0;

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
