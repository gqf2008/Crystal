// ============================================================================
// 背包对话框 - InventoryDialog
// ============================================================================
//
// 功能：
// - 显示玩家背包物品（8x5格子，40个格子）
// - 显示金币、负重
// - 支持物品拖拽、使用、丢弃
// - 支持背包扩展
//
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::{Canvas, Color, DrawParam, Rect};

/// 背包对话框
pub struct InventoryDialog {
    /// 是否可见
    visible: bool,
    
    /// 对话框位置
    x: f32,
    y: f32,
    
    /// 对话框尺寸
    width: f32,
    height: f32,
    
    /// 物品格子（8列 x 5行 = 40个）
    grid_cols: usize,
    grid_rows: usize,
    
    /// 格子起始位置
    grid_start_x: f32,
    grid_start_y: f32,
    
    /// 格子尺寸
    cell_width: f32,
    cell_height: f32,
    cell_spacing: f32,
    
    /// 选中的格子索引
    selected_slot: Option<usize>,
    
    /// 悬停的格子索引
    hover_slot: Option<usize>,
    
    /// 当前金币数
    gold: u32,
    
    /// 当前负重/最大负重
    current_weight: u16,
    max_weight: u16,
    
    /// 拖拽状态
    dragging_slot: Option<usize>,
    drag_offset_x: f32,
    drag_offset_y: f32,
}

impl InventoryDialog {
    pub fn new() -> Self {
        // 对话框尺寸和位置
        let width = 320.0;
        let height = 250.0;
        let x = 100.0; // 默认位置
        let y = 100.0;
        
        // 格子配置（参考C#: 8列5行，每格36x32，间距1）
        let grid_cols = 8;
        let grid_rows = 5;
        let grid_start_x = 9.0;
        let grid_start_y = 37.0;
        let cell_width = 36.0;
        let cell_height = 32.0;
        let cell_spacing = 1.0;
        
        Self {
            visible: false,
            x,
            y,
            width,
            height,
            grid_cols,
            grid_rows,
            grid_start_x,
            grid_start_y,
            cell_width,
            cell_height,
            cell_spacing,
            selected_slot: None,
            hover_slot: None,
            gold: 0,
            current_weight: 0,
            max_weight: 100,
            dragging_slot: None,
            drag_offset_x: 0.0,
            drag_offset_y: 0.0,
        }
    }
    
    /// 显示/隐藏背包
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        println!("🎒 背包对话框 {}", if self.visible { "打开" } else { "关闭" });
    }
    
    pub fn show(&mut self) {
        self.visible = true;
    }
    
    pub fn hide(&mut self) {
        self.visible = false;
    }
    
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    
    /// 更新金币数
    pub fn set_gold(&mut self, gold: u32) {
        self.gold = gold;
    }
    
    /// 更新负重
    pub fn set_weight(&mut self, current: u16, max: u16) {
        self.current_weight = current;
        self.max_weight = max;
    }
    
    /// 检查鼠标位置是否在对话框内
    pub fn contains_point(&self, mouse_x: f32, mouse_y: f32) -> bool {
        if !self.visible {
            return false;
        }
        
        mouse_x >= self.x && mouse_x <= self.x + self.width &&
        mouse_y >= self.y && mouse_y <= self.y + self.height
    }
    
    /// 获取鼠标悬停的格子索引
    fn get_slot_at(&self, mouse_x: f32, mouse_y: f32) -> Option<usize> {
        if !self.visible {
            return None;
        }
        
        // 转换为相对坐标
        let rel_x = mouse_x - self.x - self.grid_start_x;
        let rel_y = mouse_y - self.y - self.grid_start_y;
        
        if rel_x < 0.0 || rel_y < 0.0 {
            return None;
        }
        
        // 计算列和行（考虑间距）
        let col = (rel_x / (self.cell_width + self.cell_spacing)) as usize;
        let row = (rel_y / (self.cell_height + self.cell_spacing)) as usize;
        
        if col >= self.grid_cols || row >= self.grid_rows {
            return None;
        }
        
        // 检查是否点击在格子内部（不是间距上）
        let cell_local_x = rel_x % (self.cell_width + self.cell_spacing);
        let cell_local_y = rel_y % (self.cell_height + self.cell_spacing);
        
        if cell_local_x > self.cell_width || cell_local_y > self.cell_height {
            return None;
        }
        
        Some(row * self.grid_cols + col)
    }
    
    /// 开始拖拽
    pub fn start_drag(&mut self, slot: usize, mouse_x: f32, mouse_y: f32) {
        self.dragging_slot = Some(slot);
        
        // 计算拖拽偏移量(鼠标相对于格子中心的偏移)
        let (slot_x, slot_y) = self.get_slot_position(slot);
        self.drag_offset_x = mouse_x - (slot_x + self.cell_width / 2.0);
        self.drag_offset_y = mouse_y - (slot_y + self.cell_height / 2.0);
    }
    
    /// 结束拖拽
    pub fn end_drag(&mut self, mouse_x: f32, mouse_y: f32) -> Option<InventoryAction> {
        if let Some(from_slot) = self.dragging_slot {
            self.dragging_slot = None;
            
            // 检查是否拖拽到另一个格子
            let to_slot = self.get_slot_at(mouse_x, mouse_y);
            
            return Some(InventoryAction::EndDrag {
                from: from_slot,
                to: to_slot,
            });
        }
        None
    }
    
    /// 是否正在拖拽
    pub fn is_dragging(&self) -> bool {
        self.dragging_slot.is_some()
    }
    
    /// 获取格子的屏幕位置
    fn get_slot_position(&self, slot: usize) -> (f32, f32) {
        let row = slot / self.grid_cols;
        let col = slot % self.grid_cols;
        
        let x = self.x + self.grid_start_x + col as f32 * (self.cell_width + self.cell_spacing);
        let y = self.y + self.grid_start_y + row as f32 * (self.cell_height + self.cell_spacing);
        
        (x, y)
    }
    
    /// 更新鼠标悬停状态
    pub fn update_hover(&mut self, mouse_x: f32, mouse_y: f32) {
        self.hover_slot = self.get_slot_at(mouse_x, mouse_y);
    }
    
    /// 处理鼠标点击
    pub fn on_mouse_down(&mut self, mouse_x: f32, mouse_y: f32) -> Option<InventoryAction> {
        if !self.visible {
            return None;
        }
        
        // 检查是否点击关闭按钮区域（右上角）
        let close_button_x = self.x + self.width - 30.0;
        let close_button_y = self.y + 3.0;
        let close_button_size = 20.0;
        
        if mouse_x >= close_button_x && mouse_x <= close_button_x + close_button_size &&
           mouse_y >= close_button_y && mouse_y <= close_button_y + close_button_size {
            self.hide();
            return Some(InventoryAction::Close);
        }
        
        // 检查是否点击物品格子
        if let Some(slot) = self.get_slot_at(mouse_x, mouse_y) {
            self.selected_slot = Some(slot);
            // 开始拖拽
            self.start_drag(slot, mouse_x, mouse_y);
            println!("🖱️ 点击背包格子: {}, 开始拖拽", slot);
            return Some(InventoryAction::StartDrag(slot));
        }
        
        None
    }
    
    /// 处理鼠标释放
    pub fn on_mouse_up(&mut self, mouse_x: f32, mouse_y: f32) -> Option<InventoryAction> {
        if !self.visible {
            return None;
        }
        
        // 如果正在拖拽,结束拖拽
        if self.is_dragging() {
            return self.end_drag(mouse_x, mouse_y);
        }
        
        None
    }
    
    /// 处理鼠标移动(拖拽中)
    pub fn on_mouse_move(&mut self, mouse_x: f32, mouse_y: f32) -> Option<InventoryAction> {
        if let Some(dragging_slot) = self.dragging_slot {
            return Some(InventoryAction::Dragging {
                slot: dragging_slot,
                x: mouse_x - self.drag_offset_x,
                y: mouse_y - self.drag_offset_y,
            });
        }
        None
    }
    
    /// 绘制背包对话框
    /// 
    /// 参数:
    /// - z_index: 绘制层级(越大越靠前), 默认为 50
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        self.draw_with_z(ctx, canvas, 50)
    }
    
    /// 绘制背包对话框(带 z-index 参数)
    pub fn draw_with_z(&self, ctx: &mut Context, canvas: &mut Canvas, z_index: i32) -> GameResult {
        if !self.visible {
            return Ok(());
        }
        
        use ggez::graphics::{Mesh, DrawMode};
        
        // 绘制背景框
        let bg_rect = Rect::new(self.x, self.y, self.width, self.height);
        let bg_mesh = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            bg_rect,
            Color::from_rgba(30, 30, 30, 230),
        )?;
        canvas.draw(&bg_mesh, DrawParam::default().z(z_index));
        
        // 绘制边框 (z+1, 在背景之上)
        let border_mesh = Mesh::new_rectangle(
            ctx,
            DrawMode::stroke(2.0),
            bg_rect,
            Color::from_rgb(100, 100, 100),
        )?;
        canvas.draw(&border_mesh, DrawParam::default().z(z_index + 1));
        
        // 绘制标题 (z+2, 在边框之上)
        use ggez::graphics::Text;
        let title = Text::new("背包 (Inventory)");
        canvas.draw(
            &title,
            DrawParam::default()
                .dest([self.x + 10.0, self.y + 8.0])
                .color(Color::from_rgb(255, 255, 200))
                .z(z_index + 2),
        );
        
        // 绘制关闭按钮 (z+2, 在边框之上)
        let close_button_x = self.x + self.width - 30.0;
        let close_button_y = self.y + 3.0;
        let close_rect = Rect::new(close_button_x, close_button_y, 20.0, 20.0);
        let close_mesh = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            close_rect,
            Color::from_rgb(150, 50, 50),
        )?;
        canvas.draw(&close_mesh, DrawParam::default().z(z_index + 2));
        
        let close_text = Text::new("X");
        canvas.draw(
            &close_text,
            DrawParam::default()
                .dest([close_button_x + 6.0, close_button_y + 2.0])
                .color(Color::WHITE)
                .z(z_index + 3),
        );
        
        // 绘制物品格子
        for row in 0..self.grid_rows {
            for col in 0..self.grid_cols {
                let slot_index = row * self.grid_cols + col;
                let cell_x = self.x + self.grid_start_x + col as f32 * (self.cell_width + self.cell_spacing);
                let cell_y = self.y + self.grid_start_y + row as f32 * (self.cell_height + self.cell_spacing);
                
                // 格子背景色
                let is_selected = self.selected_slot == Some(slot_index);
                let is_hover = self.hover_slot == Some(slot_index);
                
                let cell_color = if is_selected {
                    Color::from_rgba(100, 100, 200, 200) // 选中：蓝色
                } else if is_hover {
                    Color::from_rgba(80, 80, 80, 200) // 悬停：灰色
                } else {
                    Color::from_rgba(50, 50, 50, 180) // 默认：深灰
                };
                
                let cell_rect = Rect::new(cell_x, cell_y, self.cell_width, self.cell_height);
                let cell_mesh = Mesh::new_rectangle(ctx, DrawMode::fill(), cell_rect, cell_color)?;
                canvas.draw(&cell_mesh, DrawParam::default());
                
                // 格子边框
                let border_color = if is_selected {
                    Color::from_rgb(150, 150, 255)
                } else {
                    Color::from_rgb(70, 70, 70)
                };
                
                let cell_border = Mesh::new_rectangle(ctx, DrawMode::stroke(1.0), cell_rect, border_color)?;
                canvas.draw(&cell_border, DrawParam::default());
                
                // TODO: 绘制物品图标
            }
        }
        
        // 绘制金币标签
        let gold_text = Text::new(format!("金币: {}", self.gold));
        canvas.draw(
            &gold_text,
            DrawParam::default()
                .dest([self.x + 40.0, self.y + self.height - 30.0])
                .color(Color::from_rgb(255, 215, 0)), // 金色
        );
        
        // 绘制负重标签
        let weight_color = if self.current_weight as f32 / self.max_weight as f32 > 0.9 {
            Color::from_rgb(255, 0, 0) // 超重：红色
        } else if self.current_weight as f32 / self.max_weight as f32 > 0.7 {
            Color::from_rgb(255, 165, 0) // 接近超重：橙色
        } else {
            Color::from_rgb(200, 200, 200) // 正常：白色
        };
        
        let weight_text = Text::new(format!("负重: {}/{}", self.current_weight, self.max_weight));
        canvas.draw(
            &weight_text,
            DrawParam::default()
                .dest([self.x + self.width - 120.0, self.y + self.height - 30.0])
                .color(weight_color),
        );
        
        // 绘制拖拽中的物品
        if let Some(drag_slot) = self.dragging_slot {
            // TODO: 从ECS世界或ItemComponent获取物品数据并绘制
            // 暂时显示一个简单的拖拽提示
            let drag_text = Text::new(format!("拖拽物品 slot: {}", drag_slot));
            canvas.draw(
                &drag_text,
                DrawParam::default()
                    .dest([self.x + self.width / 2.0 - 50.0, self.y + 40.0])
                    .color(Color::from_rgba(255, 255, 100, 180)),
            );
        }
        
        Ok(())
    }
}

/// 背包操作事件
#[derive(Debug, Clone, Copy)]
pub enum InventoryAction {
    Close,
    SelectSlot(usize),
    UseItem(usize),
    DropItem(usize),
    /// 开始拖拽物品
    StartDrag(usize),
    /// 拖拽中
    Dragging { slot: usize, x: f32, y: f32 },
    /// 结束拖拽 (from_slot, to_slot)
    EndDrag { from: usize, to: Option<usize> },
}
