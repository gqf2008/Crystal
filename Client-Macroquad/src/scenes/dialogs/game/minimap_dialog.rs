// ============================================================================
// MiniMapDialog - 小地图对话框
// ============================================================================
// 
// 【功能说明】
// 1. 显示当前地图的缩略图
// 2. 显示玩家位置和朝向
// 3. 显示其他玩家、NPC、怪物位置
// 4. 支持地图缩放和拖拽查看
// 5. 快速传送功能（如果有传送权限）
// 
// ============================================================================

use egui_macroquad::egui;
use crate::resources::LibraryName;
use crate::scenes::dialogs::Dialog;

/// 地图上的对象类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MapObjectType {
    Player,     // 玩家
    OtherPlayer, // 其他玩家
    NPC,        // NPC
    Monster,    // 怪物
    Item,       // 掉落物品
    Portal,     // 传送点
}

/// 地图对象
#[derive(Debug, Clone)]
pub struct MapObject {
    pub obj_type: MapObjectType,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub level: Option<u32>,  // 等级（玩家和怪物）
    pub hostile: bool,       // 是否敌对
}

/// 小地图对话框
pub struct MiniMapDialog {
    /// 是否可见
    visible: bool,
    /// 窗口位置
    position: egui::Pos2,
    /// 是否正在拖拽
    dragging: bool,
    /// 拖拽偏移
    drag_offset: egui::Vec2,
    /// 地图名称
    map_name: String,
    /// 地图尺寸（像素）
    map_size: (f32, f32),
    /// 小地图显示区域尺寸
    display_size: egui::Vec2,
    /// 当前缩放级别
    zoom_level: f32,
    /// 地图视图偏移
    view_offset: egui::Vec2,
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
}

impl MiniMapDialog {
    pub fn new() -> Self {
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
                obj_type: MapObjectType::OtherPlayer,
                name: "法师123".to_string(),
                x: 450.0,
                y: 300.0,
                level: Some(42),
                hostile: true,
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
                obj_type: MapObjectType::NPC,
                name: "药店老板".to_string(),
                x: 550.0,
                y: 320.0,
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
                obj_type: MapObjectType::Monster,
                name: "骷髅".to_string(),
                x: 650.0,
                y: 350.0,
                level: Some(30),
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
            MapObject {
                obj_type: MapObjectType::Item,
                name: "金币".to_string(),
                x: 520.0,
                y: 420.0,
                level: None,
                hostile: false,
            },
        ];

        Self {
            visible: true,   // 默认显示，符合原版逻辑
            position: egui::pos2(macroquad::prelude::screen_width() - 126.0, 0.0),  // 右上角位置，符合原版
            dragging: false,
            drag_offset: egui::Vec2::ZERO,
            map_name: "比奇城".to_string(),
            map_size: (1024.0, 768.0),
            display_size: egui::vec2(200.0, 150.0),
            zoom_level: 1.0,
            view_offset: egui::Vec2::ZERO,
            player_pos: (512.0, 384.0),
            player_direction: 0.0,
            map_objects,
            show_player_names: true,
            show_monsters: true,
            show_npcs: true,
            big_mode: true,  // 默认大模式
        }
    }

    /// 切换可见性（原工程逻辑：在大模式和隐藏之间切换）
    pub fn toggle(&mut self) {
        if !self.visible {
            // 隐藏状态 -> 显示大模式
            self.visible = true;
            self.big_mode = true;
            println!("🗺️ 小地图: 显示大模式");
        } else {
            // 显示状态 -> 隐藏
            self.visible = false;
            println!("🗺️ 小地图: 隐藏");
        }
    }

    /// 获取可见性
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// 设置可见性
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// 切换大小模式（TAB键功能）
    pub fn toggle_size(&mut self) {
        if self.visible {
            self.big_mode = !self.big_mode;
            println!("🗺️ 小地图模式: {}", if self.big_mode { "大模式" } else { "小模式" });
        }
    }

    /// 获取当前模式
    pub fn is_big_mode(&self) -> bool {
        self.big_mode
    }

    /// 世界坐标转换为小地图坐标
    fn world_to_minimap(&self, world_x: f32, world_y: f32, map_rect: &egui::Rect) -> egui::Pos2 {
        let scale_x = map_rect.width() / self.map_size.0 * self.zoom_level;
        let scale_y = map_rect.height() / self.map_size.1 * self.zoom_level;
        
        let mini_x = map_rect.min.x + world_x * scale_x - self.view_offset.x;
        let mini_y = map_rect.min.y + world_y * scale_y - self.view_offset.y;
        
        egui::pos2(mini_x, mini_y)
    }

    /// 绘制对话框背景
    fn draw_background(&self, ui: &mut egui::Ui, ctx: &egui::Context) -> egui::Rect {
        // 根据模式选择纹理：大模式2090、小模式2091
        let texture_index = if self.big_mode { 2090 } else { 2091 };
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, texture_index) {
            if let Some(bg_texture) = info.egui_texture {
                let bg_size = bg_texture.size_vec2();
                let bg_rect = egui::Rect::from_min_size(self.position, bg_size);
                
                ui.painter().image(
                    bg_texture.id(),
                    bg_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                
                return bg_rect;
            }
        }
        
        // 降级：使用自定义背景
        let bg_size = egui::vec2(250.0, 280.0);
        let bg_rect = egui::Rect::from_min_size(self.position, bg_size);
        
        ui.painter().rect_filled(
            bg_rect,
            5.0,
            egui::Color32::from_rgba_premultiplied(40, 45, 50, 250),
        );
        ui.painter().rect_stroke(
            bg_rect,
            5.0,
            egui::Stroke::new(2.0, egui::Color32::from_rgb(150, 150, 150)),
            egui::epaint::StrokeKind::Outside,
        );

        // 绘制标题
        ui.painter().text(
            egui::pos2(bg_rect.min.x + 15.0, bg_rect.min.y + 12.0),
            egui::Align2::LEFT_CENTER,
            &format!("🗺️ {}", self.map_name),
            egui::FontId::proportional(14.0),
            egui::Color32::from_rgb(255, 215, 0),
        );

        bg_rect
    }

    /// 绘制地图区域
    fn draw_map(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 根据原版小地图布局，地图显示区域应该适配纹理尺寸
        // 纹理2090大概是124x150，实际地图区域更小
        let map_area = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 3.0, bg_rect.min.y + 22.0),  // 左边距3px，顶部22px（留给标题栏）
            egui::vec2(bg_rect.width() - 6.0, 108.0)  // 宽度留6px边距，高度108px（原版地图显示区域）
        );

        // 地图背景
        ui.painter().rect_filled(
            map_area,
            3.0,
            egui::Color32::from_rgb(30, 40, 30),
        );
        ui.painter().rect_stroke(
            map_area,
            3.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 100, 80)),
            egui::epaint::StrokeKind::Outside,
        );

        // 绘制网格
        self.draw_grid(ui, &map_area);

        // 绘制地图对象
        self.draw_map_objects(ui, &map_area);

        // 处理地图交互（拖拽、缩放）
        self.handle_map_interaction(ui, &map_area);
        
        // 绘制Toggle按钮（位置109,3）
        self.draw_toggle_button(ui, ctx, bg_rect);
    }

    /// 绘制网格
    fn draw_grid(&self, ui: &mut egui::Ui, map_rect: &egui::Rect) {
        let grid_size = 20.0 * self.zoom_level;
        let grid_color = egui::Color32::from_rgba_premultiplied(100, 100, 100, 60);

        // 垂直线
        let mut x = map_rect.min.x;
        while x < map_rect.max.x {
            ui.painter().line_segment(
                [egui::pos2(x, map_rect.min.y), egui::pos2(x, map_rect.max.y)],
                egui::Stroke::new(0.5, grid_color),
            );
            x += grid_size;
        }

        // 水平线
        let mut y = map_rect.min.y;
        while y < map_rect.max.y {
            ui.painter().line_segment(
                [egui::pos2(map_rect.min.x, y), egui::pos2(map_rect.max.x, y)],
                egui::Stroke::new(0.5, grid_color),
            );
            y += grid_size;
        }
    }

    /// 绘制地图对象
    fn draw_map_objects(&self, ui: &mut egui::Ui, map_rect: &egui::Rect) {
        for obj in &self.map_objects {
            // 根据设置过滤对象
            match obj.obj_type {
                MapObjectType::Monster if !self.show_monsters => continue,
                MapObjectType::NPC if !self.show_npcs => continue,
                _ => {}
            }

            let pos = self.world_to_minimap(obj.x, obj.y, map_rect);
            
            // 检查是否在可见区域内
            if !map_rect.contains(pos) {
                continue;
            }

            let (color, size, symbol) = match obj.obj_type {
                MapObjectType::Player => (egui::Color32::from_rgb(0, 255, 0), 6.0, "●"),
                MapObjectType::OtherPlayer => {
                    if obj.hostile {
                        (egui::Color32::from_rgb(255, 100, 100), 5.0, "●")
                    } else {
                        (egui::Color32::from_rgb(100, 100, 255), 5.0, "●")
                    }
                },
                MapObjectType::NPC => (egui::Color32::from_rgb(255, 255, 0), 4.0, "■"),
                MapObjectType::Monster => (egui::Color32::from_rgb(255, 0, 0), 4.0, "▲"),
                MapObjectType::Item => (egui::Color32::from_rgb(255, 215, 0), 3.0, "♦"),
                MapObjectType::Portal => (egui::Color32::from_rgb(255, 0, 255), 5.0, "◆"),
            };

            // 绘制对象
            ui.painter().text(
                pos,
                egui::Align2::CENTER_CENTER,
                symbol,
                egui::FontId::proportional(size),
                color,
            );

            // 绘制玩家朝向
            if obj.obj_type == MapObjectType::Player {
                let direction_end = pos + egui::vec2(
                    self.player_direction.cos() * 8.0,
                    self.player_direction.sin() * 8.0,
                );
                ui.painter().line_segment(
                    [pos, direction_end],
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 255, 0)),
                );
            }

            // 显示名称（如果启用）
            if self.show_player_names {
                let name_pos = pos + egui::vec2(0.0, -15.0);
                ui.painter().text(
                    name_pos,
                    egui::Align2::CENTER_BOTTOM,
                    &obj.name,
                    egui::FontId::proportional(8.0),
                    egui::Color32::WHITE,
                );

                // 显示等级
                if let Some(level) = obj.level {
                    let level_pos = pos + egui::vec2(0.0, 12.0);
                    ui.painter().text(
                        level_pos,
                        egui::Align2::CENTER_TOP,
                        &format!("Lv.{}", level),
                        egui::FontId::proportional(7.0),
                        egui::Color32::from_rgb(200, 200, 200),
                    );
                }
            }
        }
    }

    /// 绘制Toggle按钮
    fn draw_toggle_button(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // Toggle按钮位置：右上角(109,3)
        let button_pos = egui::pos2(bg_rect.min.x + 109.0, bg_rect.min.y + 3.0);
        
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 2102) {
            if let Some(texture) = info.egui_texture {
                let button_size = texture.size_vec2();
                let button_rect = egui::Rect::from_min_size(button_pos, button_size);
                
                let response = ui.interact(button_rect, egui::Id::new("minimap_toggle"), egui::Sense::click());
                
                // 根据状态选择纹理
                let texture_idx = if response.is_pointer_button_down_on() {
                    2104  // 按下状态
                } else if response.hovered() {
                    2103  // 悬停状态
                } else {
                    2102  // 正常状态
                };
                
                if let Some(btn_info) = LibraryName::Prguse.get_egui_texture(ctx, texture_idx) {
                    if let Some(btn_texture) = btn_info.egui_texture {
                        ui.painter().image(
                            btn_texture.id(),
                            button_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
                
                // 处理点击事件
                if response.clicked() {
                    self.toggle();
                }
                
                response.on_hover_text("小地图 (M)");
            }
        }
    }

    /// 处理地图交互
    fn handle_map_interaction(&mut self, ui: &mut egui::Ui, map_rect: &egui::Rect) {
        let response = ui.interact(*map_rect, egui::Id::new("minimap_area"), egui::Sense::click_and_drag());
        
        // 拖拽地图
        if response.dragged() {
            self.view_offset += response.drag_delta();
        }

        // 滚轮缩放
        if response.hovered() {
            let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
            if scroll_delta != 0.0 {
                let zoom_factor = 1.0 + scroll_delta * 0.001;
                self.zoom_level = (self.zoom_level * zoom_factor).clamp(0.5, 3.0);
            }
        }

        // 双击重置视图
        if response.double_clicked() {
            self.zoom_level = 1.0;
            self.view_offset = egui::Vec2::ZERO;
            println!("🗺️ 重置地图视图");
        }
    }

    /// 绘制控制面板
    fn draw_controls(&mut self, ui: &mut egui::Ui, bg_rect: &egui::Rect) {
        let controls_area = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 10.0, bg_rect.min.y + 225.0),
            egui::vec2(230.0, 45.0)
        );

        // 缩放控制
        let zoom_text = format!("缩放: {:.1}x", self.zoom_level);
        ui.painter().text(
            egui::pos2(controls_area.min.x + 5.0, controls_area.min.y + 5.0),
            egui::Align2::LEFT_TOP,
            &zoom_text,
            egui::FontId::proportional(10.0),
            egui::Color32::WHITE,
        );

        // 坐标显示
        let coord_text = format!("坐标: ({:.0}, {:.0})", self.player_pos.0, self.player_pos.1);
        ui.painter().text(
            egui::pos2(controls_area.min.x + 5.0, controls_area.min.y + 18.0),
            egui::Align2::LEFT_TOP,
            &coord_text,
            egui::FontId::proportional(10.0),
            egui::Color32::from_rgb(200, 200, 200),
        );

        // 显示选项按钮
        let toggle_names_rect = egui::Rect::from_min_size(
            egui::pos2(controls_area.min.x + 120.0, controls_area.min.y),
            egui::vec2(50.0, 15.0)
        );
        let toggle_monsters_rect = egui::Rect::from_min_size(
            egui::pos2(controls_area.min.x + 175.0, controls_area.min.y),
            egui::vec2(50.0, 15.0)
        );

        // 名称显示切换
        let names_color = if self.show_player_names {
            egui::Color32::from_rgb(100, 150, 100)
        } else {
            egui::Color32::from_rgb(100, 100, 100)
        };
        ui.painter().rect_filled(toggle_names_rect, 2.0, names_color);
        ui.painter().text(
            toggle_names_rect.center(),
            egui::Align2::CENTER_CENTER,
            "名称",
            egui::FontId::proportional(9.0),
            egui::Color32::WHITE,
        );

        let names_response = ui.interact(toggle_names_rect, egui::Id::new("toggle_names"), egui::Sense::click());
        if names_response.clicked() {
            self.show_player_names = !self.show_player_names;
        }

        // 怪物显示切换
        let monsters_color = if self.show_monsters {
            egui::Color32::from_rgb(150, 100, 100)
        } else {
            egui::Color32::from_rgb(100, 100, 100)
        };
        ui.painter().rect_filled(toggle_monsters_rect, 2.0, monsters_color);
        ui.painter().text(
            toggle_monsters_rect.center(),
            egui::Align2::CENTER_CENTER,
            "怪物",
            egui::FontId::proportional(9.0),
            egui::Color32::WHITE,
        );

        let monsters_response = ui.interact(toggle_monsters_rect, egui::Id::new("toggle_monsters"), egui::Sense::click());
        if monsters_response.clicked() {
            self.show_monsters = !self.show_monsters;
        }
    }

    /// 绘制关闭按钮
    fn draw_close_button(&self, ui: &mut egui::Ui, _ctx: &egui::Context, bg_rect: &egui::Rect) -> bool {
        // 关闭按钮位置（右上角）
        let close_size = egui::vec2(15.0, 15.0);
        let close_rect = egui::Rect::from_min_size(
            egui::pos2(bg_rect.max.x - 20.0, bg_rect.min.y + 5.0),
            close_size
        );

        // 绘制关闭按钮背景
        ui.painter().rect_filled(close_rect, 2.0, egui::Color32::from_rgb(150, 50, 50));
        ui.painter().rect_stroke(
            close_rect,
            2.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 100, 100)),
            egui::epaint::StrokeKind::Outside,
        );

        // 绘制关闭符号 "×"
        ui.painter().text(
            close_rect.center(),
            egui::Align2::CENTER_CENTER,
            "×",
            egui::FontId::proportional(10.0),
            egui::Color32::WHITE,
        );

        let response = ui.interact(close_rect, egui::Id::new("minimap_close"), egui::Sense::click());
        let is_clicked = response.clicked();
        if response.hovered() {
            response.on_hover_text("关闭");
        }

        is_clicked
    }

    /// 处理窗口拖拽
    fn handle_window_dragging(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 标题栏区域作为拖拽区域
        let title_area = egui::Rect::from_min_size(
            bg_rect.min,
            egui::vec2(bg_rect.width(), 25.0),
        );
        
        let drag_response = ui.interact(title_area, egui::Id::new("minimap_drag"), egui::Sense::drag());
        
        if drag_response.drag_started() {
            self.dragging = true;
            if let Some(pointer_pos) = ctx.pointer_interact_pos() {
                self.drag_offset = self.position.to_vec2() - pointer_pos.to_vec2();
            }
        }
        
        if self.dragging {
            if let Some(pointer_pos) = ctx.pointer_interact_pos() {
                self.position = (pointer_pos.to_vec2() + self.drag_offset).to_pos2();
            }
        }
        
        if drag_response.drag_stopped() {
            self.dragging = false;
        }
    }

    /// 更新玩家位置（应该由游戏逻辑调用）
    pub fn update_player_position(&mut self, x: f32, y: f32, direction: f32) {
        self.player_pos = (x, y);
        self.player_direction = direction;
        
        // 更新玩家对象在列表中的位置
        if let Some(player_obj) = self.map_objects.iter_mut().find(|obj| obj.obj_type == MapObjectType::Player) {
            player_obj.x = x;
            player_obj.y = y;
        }
    }
}

impl Dialog for MiniMapDialog {
    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        if !self.visible {
            *open = false;
            return;
        }
        
        // 使用 Area 创建自由浮动窗口
        egui::Area::new(egui::Id::new("minimap_dialog"))
            .fixed_pos(self.position)
            .movable(false)  // 使用自定义拖拽
            .show(ctx, |ui| {
                // 绘制背景
                let bg_rect = self.draw_background(ui, ctx);
                
                // 处理窗口拖拽
                self.handle_window_dragging(ui, ctx, &bg_rect);
                
                // 绘制地图
                self.draw_map(ui, ctx, &bg_rect);
                
                // 绘制控制面板
                self.draw_controls(ui, &bg_rect);
                
                // 绘制关闭按钮
                if self.draw_close_button(ui, ctx, &bg_rect) {
                    self.visible = false;
                    *open = false;
                }
            });
        
        *open = self.visible;
    }
}