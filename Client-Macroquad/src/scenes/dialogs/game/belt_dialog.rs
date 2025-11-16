/// 快捷栏对话框（血瓶框）
/// 
/// 6个物品格子的快捷栏，可以放置药水、卷轴等常用物品
/// 支持水平和垂直两种布局，玩家可以通过旋转按钮切换
/// 按数字键1-6可以快速使用对应格子的物品

use egui_macroquad::egui;
use crate::resources::LibraryName;
use crate::scenes::dialogs::Dialog;

/// 快捷栏布局模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BeltLayout {
    /// 水平布局（默认）
    Horizontal,
    /// 垂直布局
    Vertical,
}

/// 快捷栏对话框
pub struct BeltDialog {
    visible: bool,
    layout: BeltLayout,
    position: egui::Pos2,
    
    // 格子数据（实际物品数据应该从 ECS/GameState 获取）
    // 这里只是占位，真实实现需要关联到物品系统
    #[allow(dead_code)]
    cells: [Option<CellItem>; 6],
}

/// 快捷栏格子物品（临时结构，后续应该使用真实的物品系统）
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct CellItem {
    texture_index: u32,
    count: u32,
}

impl BeltDialog {
    /// 创建快捷栏对话框
    pub fn new(main_dialog_x: f32, screen_height: f32) -> Self {
        // 默认水平布局，位于主界面上方居中
        let position = egui::pos2(main_dialog_x + 230.0, screen_height - 150.0);
        
        Self {
            visible: true,
            layout: BeltLayout::Horizontal,
            position,
            cells: Default::default(),
        }
    }
    
    /// 切换布局（水平 ↔ 垂直）
    pub fn flip_layout(&mut self) {
        self.layout = match self.layout {
            BeltLayout::Horizontal => {
                // 切换到垂直布局
                self.position = egui::pos2(0.0, 200.0);
                BeltLayout::Vertical
            }
            BeltLayout::Vertical => {
                // 切换回水平布局
                // 注意：这里的x坐标需要根据主界面位置动态计算
                // 暂时使用固定值，后续需要传入 main_dialog_x
                self.position = egui::pos2(400.0, 500.0);
                BeltLayout::Horizontal
            }
        };
    }
    
    /// 绘制快捷栏
    fn draw_belt(&self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // 获取背景纹理索引
        let bg_index = match self.layout {
            BeltLayout::Horizontal => 1932,
            BeltLayout::Vertical => 1944,
        };
        
        // 绘制主背景
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, bg_index) {
            if let Some(bg_texture) = info.egui_texture {
                let size = bg_texture.size_vec2();
                ui.painter().image(
                    bg_texture.id(),
                    egui::Rect::from_min_size(self.position, size),
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                
                // 绘制半透明覆盖层
                if let Some(info2) = LibraryName::Prguse.get_egui_texture(ctx, bg_index + 1) {
                    if let Some(overlay) = info2.egui_texture {
                        ui.painter().image(
                            overlay.id(),
                            egui::Rect::from_min_size(self.position, size),
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::from_white_alpha(128), // 50% 透明度
                        );
                    }
                }
            }
        }
    }
    
    /// 绘制物品格子
    fn draw_cells(&self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        for i in 0..6 {
            let cell_pos = self.get_cell_position(i);
            let cell_size = egui::vec2(32.0, 32.0);
            
            // 绘制格子背景（深色边框）
            let rect = egui::Rect::from_min_size(cell_pos, cell_size);
            ui.painter().rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 60)),
                egui::epaint::StrokeKind::Middle,
            );
            
            // 绘制数字键提示
            let key_text = format!("{}", i + 1);
            let key_pos = match self.layout {
                BeltLayout::Horizontal => egui::pos2(cell_pos.x + 9.0, cell_pos.y - 12.0),
                BeltLayout::Vertical => egui::pos2(cell_pos.x - 15.0, cell_pos.y + 13.0),
            };
            
            ui.painter().text(
                key_pos,
                egui::Align2::CENTER_CENTER,
                &key_text,
                egui::FontId::proportional(10.0),
                egui::Color32::WHITE,
            );
            
            // TODO: 绘制物品图标和数量
            // if let Some(item) = &self.cells[i] {
            //     // 绘制物品纹理
            //     // 绘制数量文字
            // }
        }
    }
    
    /// 获取格子位置
    fn get_cell_position(&self, index: usize) -> egui::Pos2 {
        match self.layout {
            BeltLayout::Horizontal => {
                // 水平布局：从左到右
                egui::pos2(
                    self.position.x + (index as f32) * 35.0 + 12.0,
                    self.position.y + 3.0
                )
            }
            BeltLayout::Vertical => {
                // 垂直布局：从上到下
                egui::pos2(
                    self.position.x + 3.0,
                    self.position.y + (index as f32) * 35.0 + 12.0
                )
            }
        }
    }
    
    /// 绘制旋转按钮
    fn draw_rotate_button(&self, ui: &mut egui::Ui, ctx: &egui::Context) -> bool {
        let (index, hover_index, pressed_index, position) = match self.layout {
            BeltLayout::Horizontal => (1926_usize, 1927_usize, 1928_usize, egui::pos2(self.position.x + 222.0, self.position.y + 3.0)),
            BeltLayout::Vertical => (1938_usize, 1939_usize, 1940_usize, egui::pos2(self.position.x + 19.0, self.position.y + 222.0)),
        };
        
        self.draw_button(ui, ctx, index, hover_index, pressed_index, position, "旋转")
    }
    
    /// 绘制关闭按钮
    fn draw_close_button(&self, ui: &mut egui::Ui, ctx: &egui::Context) -> bool {
        let (index, hover_index, pressed_index, position) = match self.layout {
            BeltLayout::Horizontal => (1923_usize, 1924_usize, 1925_usize, egui::pos2(self.position.x + 222.0, self.position.y + 19.0)),
            BeltLayout::Vertical => (1935_usize, 1936_usize, 1937_usize, egui::pos2(self.position.x + 3.0, self.position.y + 222.0)),
        };
        
        self.draw_button(ui, ctx, index, hover_index, pressed_index, position, "关闭")
    }
    
    /// 绘制单个按钮（复用逻辑）
    fn draw_button(
        &self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        index: usize,
        hover_index: usize,
        pressed_index: usize,
        position: egui::Pos2,
        hint: &str,
    ) -> bool {
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, index) {
            if let Some(normal_texture) = info.egui_texture {
                let size = normal_texture.size_vec2();
                let rect = egui::Rect::from_min_size(position, size);
                
                // 检测鼠标交互
                let response = ui.allocate_rect(rect, egui::Sense::click());
                let clicked = response.clicked();
                
                // 根据状态选择纹理
                let texture_id = if response.is_pointer_button_down_on() {
                    // 按下状态
                    if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, pressed_index) {
                        info.egui_texture.map(|t| t.id()).unwrap_or(normal_texture.id())
                    } else {
                        normal_texture.id()
                    }
                } else if response.hovered() {
                    // 悬停状态
                    if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, hover_index) {
                        info.egui_texture.map(|t| t.id()).unwrap_or(normal_texture.id())
                    } else {
                        normal_texture.id()
                    }
                } else {
                    // 正常状态
                    normal_texture.id()
                };
                
                // 绘制按钮
                ui.painter().image(
                    texture_id,
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                
                // 悬停提示
                if response.hovered() {
                    response.on_hover_text(hint);
                }
                
                return clicked;
            }
        }
        false
    }
}

impl Dialog for BeltDialog {
    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        if !self.visible {
            *open = false;
            return;
        }
        
        // 使用 Area 创建一个自由浮动的窗口
        egui::Area::new(egui::Id::new("belt_dialog"))
            .fixed_pos(self.position)
            .show(ctx, |ui| {
                // 绘制快捷栏背景
                self.draw_belt(ui, ctx);
                
                // 绘制物品格子
                self.draw_cells(ui, ctx);
                
                // 绘制旋转按钮
                if self.draw_rotate_button(ui, ctx) {
                    // 点击旋转按钮
                    self.flip_layout();
                }
                
                // 绘制关闭按钮
                if self.draw_close_button(ui, ctx) {
                    // 点击关闭按钮
                    self.visible = false;
                    *open = false;
                }
            });
        
        *open = self.visible;
    }
}
