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
    
    // 保存初始水平布局位置，用于从垂直布局切换回来时恢复
    horizontal_position: egui::Pos2,
    
    // 拖动相关
    dragging: bool,
    drag_offset: egui::Vec2,
    
    // 格子数据（实际物品数据应该从 ECS/GameState 获取）
    // 这里只是占位，真实实现需要关联到物品系统
    #[allow(dead_code)]
    cells: [Option<CellItem>; 6],
}

/// 快捷栏格子物品（临时结构，后续应该使用真实的物品系统）
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct CellItem {
    texture_index: u32,
    count: u32,
}

impl BeltDialog {
    /// 创建快捷栏对话框
    pub fn new(main_dialog_x: f32, screen_height: f32) -> Self {
        // 默认水平布局,位于主界面上方,与 ChatDialog 同一水平线
        // 原工程：MainDialog.X + 230, ScreenHeight - 150
        let position = egui::pos2(main_dialog_x + 230.0, screen_height - 150.0);
        
        // 初始化一些药水物品作为示例
        let mut cells = [None; 6];
        
        // 索引 0: 小血瓶 (Items 索引 0)
        cells[0] = Some(CellItem {
            texture_index: 0,  // 小血瓶图标
            count: 15,         // 15个
        });
        
        // 索引 1: 大血瓶 (Items 索引 1)
        cells[1] = Some(CellItem {
            texture_index: 1,  // 大血瓶图标
            count: 8,          // 8个
        });
        
        // 索引 2: 小蓝瓶 (Items 索引 2)
        cells[2] = Some(CellItem {
            texture_index: 2,  // 小蓝瓶图标
            count: 12,         // 12个
        });
        
        // 索引 3: 大蓝瓶 (Items 索引 3)
        cells[3] = Some(CellItem {
            texture_index: 3,  // 大蓝瓶图标
            count: 6,          // 6个
        });
        
        // 索引 4: 金创药 (Items 索引 5)
        cells[4] = Some(CellItem {
            texture_index: 5,  // 金创药图标
            count: 3,          // 3个
        });
        
        // 索引 5: 万能药 (Items 索引 6)
        cells[5] = Some(CellItem {
            texture_index: 6,  // 万能药图标
            count: 2,          // 2个
        });
        
        Self {
            visible: true,
            layout: BeltLayout::Horizontal,
            position,
            horizontal_position: position,  // 保存初始位置
            dragging: false,
            drag_offset: egui::vec2(0.0, 0.0),
            cells,
        }
    }
    
    /// 切换布局（水平 ↔ 垂直）
    pub fn flip_layout(&mut self) {
        self.layout = match self.layout {
            BeltLayout::Horizontal => {
                // 切换到垂直布局前,先保存当前水平位置
                self.horizontal_position = self.position;
                // 切换到垂直布局
                self.position = egui::pos2(0.0, 200.0);
                BeltLayout::Vertical
            }
            BeltLayout::Vertical => {
                // 切换回水平布局,恢复之前保存的位置
                self.position = self.horizontal_position;
                BeltLayout::Horizontal
            }
        };
    }
    
    /// 设置位置（当 ChatDialog 改变大小时需要同步更新）
    pub fn set_position(&mut self, pos: egui::Pos2) {
        self.position = pos;
        // 如果当前是水平布局,同时更新保存的水平位置
        if self.layout == BeltLayout::Horizontal {
            self.horizontal_position = pos;
        }
    }
    
    /// 处理窗口拖动
    fn handle_dragging(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 定义可拖动区域（整个背景区域，但排除按钮区域）
        let drag_area = egui::Rect::from_min_size(
            bg_rect.min,
            egui::vec2(bg_rect.width(), bg_rect.height() - 30.0),  // 排除底部按钮区
        );
        
        let drag_response = ui.interact(
            drag_area,
            egui::Id::new("belt_drag_area"),
            egui::Sense::drag(),
        );
        
        if drag_response.drag_started() {
            self.dragging = true;
            if let Some(pointer_pos) = ctx.pointer_interact_pos() {
                self.drag_offset = self.position.to_vec2() - pointer_pos.to_vec2();
            }
        }
        
        if self.dragging {
            if let Some(pointer_pos) = ctx.pointer_latest_pos() {
                self.position = (pointer_pos.to_vec2() + self.drag_offset).to_pos2();
                // 如果是水平布局，同时更新保存的位置
                if self.layout == BeltLayout::Horizontal {
                    self.horizontal_position = self.position;
                }
            }
            
            if drag_response.drag_stopped() || !drag_response.dragged() {
                self.dragging = false;
            }
        }
    }

    
    /// 绘制快捷栏
    fn draw_belt(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) -> egui::Rect {
        // 获取背景纹理索引
        let bg_index = match self.layout {
            BeltLayout::Horizontal => 1932,
            BeltLayout::Vertical => 1944,
        };
        
        // 绘制主背景
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, bg_index) {
            if let Some(bg_texture) = info.egui_texture {
                let size = bg_texture.size_vec2();
                let bg_rect = egui::Rect::from_min_size(self.position, size);
                
                ui.painter().image(
                    bg_texture.id(),
                    bg_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                
                // 绘制半透明覆盖层
                if let Some(info2) = LibraryName::Prguse.get_egui_texture(ctx, bg_index + 1) {
                    if let Some(overlay) = info2.egui_texture {
                        ui.painter().image(
                            overlay.id(),
                            bg_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::from_white_alpha(128), // 50% 透明度
                        );
                    }
                }
                
                return bg_rect;
            }
        }
        
        // 降级：返回默认矩形
        egui::Rect::from_min_size(self.position, egui::vec2(100.0, 100.0))
    }
    
    /// 绘制物品格子
    fn draw_cells(&self, ui: &mut egui::Ui, ctx: &egui::Context) {
        for i in 0..6 {
            let cell_pos = self.get_cell_position(i);
            let cell_size = egui::vec2(32.0, 32.0);
            
            // 绘制格子纹理背景（使用 Items 库的空格子纹理）
            // 原工程中空格子会显示一个物品槽的背景
            // 这里我们绘制一个带边框的半透明背景来模拟
            let rect = egui::Rect::from_min_size(cell_pos, cell_size);
            
            // 绘制格子背景（深色填充）
            ui.painter().rect_filled(
                rect,
                2.0,
                egui::Color32::from_rgba_premultiplied(40, 40, 40, 200),
            );
            
            // 绘制格子边框（亮色边框）
            ui.painter().rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(1.5, egui::Color32::from_rgb(100, 100, 100)),
                egui::epaint::StrokeKind::Middle,
            );
            
            // 绘制内部高光（模拟3D效果）
            ui.painter().line_segment(
                [
                    egui::pos2(cell_pos.x + 1.0, cell_pos.y + cell_size.y - 1.0),
                    egui::pos2(cell_pos.x + 1.0, cell_pos.y + 1.0),
                ],
                egui::Stroke::new(1.0, egui::Color32::from_rgb(150, 150, 150)),
            );
            ui.painter().line_segment(
                [
                    egui::pos2(cell_pos.x + 1.0, cell_pos.y + 1.0),
                    egui::pos2(cell_pos.x + cell_size.x - 1.0, cell_pos.y + 1.0),
                ],
                egui::Stroke::new(1.0, egui::Color32::from_rgb(150, 150, 150)),
            );
            
            // 绘制阴影（右下角）
            ui.painter().line_segment(
                [
                    egui::pos2(cell_pos.x + cell_size.x - 1.0, cell_pos.y + 1.0),
                    egui::pos2(cell_pos.x + cell_size.x - 1.0, cell_pos.y + cell_size.y - 1.0),
                ],
                egui::Stroke::new(1.0, egui::Color32::from_rgb(30, 30, 30)),
            );
            ui.painter().line_segment(
                [
                    egui::pos2(cell_pos.x + 1.0, cell_pos.y + cell_size.y - 1.0),
                    egui::pos2(cell_pos.x + cell_size.x - 1.0, cell_pos.y + cell_size.y - 1.0),
                ],
                egui::Stroke::new(1.0, egui::Color32::from_rgb(30, 30, 30)),
            );
            
            // 绘制物品图标和数量
            if let Some(item) = &self.cells[i] {
                if let Some(info) = LibraryName::Items.get_egui_texture(ctx, item.texture_index as usize) {
                    if let Some(item_texture) = info.egui_texture {
                        // 根据原始C#代码逻辑：计算居中偏移
                        // Point offSet = new Point((Size.Width - imgSize.Width) / 2, (Size.Height - imgSize.Height) / 2);
                        let cell_inner_size = 30.0; // 32 - 2像素边距
                        let img_width = info.width as f32;
                        let img_height = info.height as f32;
                        
                        // 计算居中偏移
                        let center_offset_x = (cell_inner_size - img_width) / 2.0;
                        let center_offset_y = (cell_inner_size - img_height) / 2.0;
                        
                        // 计算最终绘制位置（格子位置 + 边距 + 居中偏移）
                        let draw_pos = egui::pos2(
                            cell_pos.x + 1.0 + center_offset_x,
                            cell_pos.y + 1.0 + center_offset_y
                        );
                        
                        // 使用纹理的实际尺寸
                        let texture_size = egui::vec2(img_width, img_height);
                        let item_rect = egui::Rect::from_min_size(draw_pos, texture_size);
                        
                        ui.painter().image(
                            item_texture.id(),
                            item_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
                
                // 绘制物品数量（在右下角）
                if item.count > 1 {
                    let count_text = format!("{}", item.count);
                    ui.painter().text(
                        egui::pos2(cell_pos.x + cell_size.x - 3.0, cell_pos.y + cell_size.y - 3.0),
                        egui::Align2::RIGHT_BOTTOM,
                        &count_text,
                        egui::FontId::proportional(9.0),
                        egui::Color32::WHITE,
                    );
                }
            }
            
            // 绘制数字键提示（在格子外面）
            let key_text = format!("{}", i + 1);
            let key_pos = match self.layout {
                BeltLayout::Horizontal => egui::pos2(cell_pos.x + 16.0, cell_pos.y - 12.0),
                BeltLayout::Vertical => egui::pos2(cell_pos.x - 15.0, cell_pos.y + 16.0),
            };
            
            ui.painter().text(
                key_pos,
                egui::Align2::CENTER_CENTER,
                &key_text,
                egui::FontId::proportional(9.0),
                egui::Color32::from_rgb(255, 255, 0), // 黄色数字键提示
            );
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
            .movable(false)  // 禁用默认拖动，使用自定义拖动
            .order(egui::Order::Foreground)  // 确保快捷栏在最前层，不被遮挡
            .show(ctx, |ui| {
                // 绘制快捷栏背景
                let bg_rect = self.draw_belt(ui, ctx);
                
                // 处理拖动
                self.handle_dragging(ui, ctx, &bg_rect);
                
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
