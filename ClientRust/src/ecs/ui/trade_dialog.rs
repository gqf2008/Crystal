// ============================================================================
// 交易窗口 - ECS组件方式
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::{Canvas, Color, Rect, Text, DrawParam, PxScale};
use ggez::mint::Point2;
use crate::ecs::systems::TradeData;
use mir2_shared::data::item::UserItem;

/// 交易窗口UI组件
#[derive(Debug, Clone)]
pub struct TradeDialogComp {
    pub is_open: bool,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub trade_data: Option<TradeData>,
}

impl TradeDialogComp {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            is_open: false,
            x,
            y,
            width: 500.0,
            height: 400.0,
            trade_data: None,
        }
    }
    
    /// 打开交易窗口
    pub fn open(&mut self, trade_data: TradeData) {
        self.trade_data = Some(trade_data);
        self.is_open = true;
    }
    
    /// 关闭交易窗口
    pub fn close(&mut self) {
        self.is_open = false;
        self.trade_data = None;
    }
    
    /// 更新交易数据
    pub fn update_trade_data(&mut self, trade_data: TradeData) {
        self.trade_data = Some(trade_data);
    }
    
    /// 处理鼠标点击
    pub fn handle_click(&mut self, mouse_x: f32, mouse_y: f32) -> Option<TradeAction> {
        if !self.is_open {
            return None;
        }
        
        // 检查关闭按钮
        let close_button_rect = Rect::new(
            self.x + self.width - 30.0,
            self.y + 5.0,
            25.0,
            25.0,
        );
        
        if point_in_rect(mouse_x, mouse_y, close_button_rect) {
            return Some(TradeAction::Cancel);
        }
        
        // 检查我的物品格子 (左侧)
        let my_grid_start_x = self.x + 20.0;
        let my_grid_start_y = self.y + 100.0;
        let cell_size = 40.0;
        let cols = 5;
        let rows = 4;
        
        for row in 0..rows {
            for col in 0..cols {
                let cell_x = my_grid_start_x + col as f32 * (cell_size + 5.0);
                let cell_y = my_grid_start_y + row as f32 * (cell_size + 5.0);
                let cell_rect = Rect::new(cell_x, cell_y, cell_size, cell_size);
                
                if point_in_rect(mouse_x, mouse_y, cell_rect) {
                    let slot = (row * cols + col) as u8;
                    return Some(TradeAction::ClickMySlot(slot));
                }
            }
        }
        
        // 检查对方物品格子 (右侧)
        let partner_grid_start_x = self.x + 260.0;
        let partner_grid_start_y = self.y + 100.0;
        
        for row in 0..rows {
            for col in 0..cols {
                let cell_x = partner_grid_start_x + col as f32 * (cell_size + 5.0);
                let cell_y = partner_grid_start_y + row as f32 * (cell_size + 5.0);
                let cell_rect = Rect::new(cell_x, cell_y, cell_size, cell_size);
                
                if point_in_rect(mouse_x, mouse_y, cell_rect) {
                    let slot = (row * cols + col) as u8;
                    return Some(TradeAction::ClickPartnerSlot(slot));
                }
            }
        }
        
        // 检查金币输入框
        let gold_input_rect = Rect::new(self.x + 20.0, self.y + 300.0, 200.0, 30.0);
        if point_in_rect(mouse_x, mouse_y, gold_input_rect) {
            return Some(TradeAction::ClickGoldInput);
        }
        
        // 检查按钮
        let button_y = self.y + self.height - 50.0;
        
        if mouse_y >= button_y && mouse_y <= button_y + 35.0 {
            // "锁定" 按钮
            if mouse_x >= self.x + 50.0 && mouse_x <= self.x + 150.0 {
                return Some(TradeAction::Lock);
            }
            // "确认" 按钮
            else if mouse_x >= self.x + 200.0 && mouse_x <= self.x + 300.0 {
                return Some(TradeAction::Confirm);
            }
            // "取消" 按钮
            else if mouse_x >= self.x + 350.0 && mouse_x <= self.x + 450.0 {
                return Some(TradeAction::Cancel);
            }
        }
        
        None
    }
    
    /// 渲染交易窗口
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        if !self.is_open {
            return Ok(());
        }
        
        // 绘制背景
        let bg_rect = Rect::new(self.x, self.y, self.width, self.height);
        let bg_mesh = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::fill(),
            bg_rect,
            Color::from_rgba(20, 20, 30, 240),
        )?;
        canvas.draw(&bg_mesh, DrawParam::default());
        
        // 绘制边框
        let border_mesh = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::stroke(2.0),
            bg_rect,
            Color::from_rgb(100, 100, 120),
        )?;
        canvas.draw(&border_mesh, DrawParam::default());
        
        if let Some(ref trade_data) = self.trade_data {
            // 绘制标题
            let title = format!("与 {} 交易", trade_data.partner_name);
            let title_text = Text::new(&title);
            canvas.draw(
                &title_text,
                DrawParam::default()
                    .dest(Point2 { x: self.x + 10.0, y: self.y + 10.0 })
                    .color(Color::from_rgb(220, 220, 255))
                    .scale([18.0f32 / 40.0, 18.0f32 / 40.0]),
            );
            
            // 绘制关闭按钮
            let close_text = Text::new("×");
            canvas.draw(
                &close_text,
                DrawParam::default()
                    .dest(Point2 { x: self.x + self.width - 25.0, y: self.y + 5.0 })
                    .color(Color::from_rgb(255, 100, 100))
                    .scale([24.0f32 / 40.0, 24.0f32 / 40.0]),
            );
            
            // 绘制左右分隔标签
            let my_label = Text::new("我的物品");
            canvas.draw(
                &my_label,
                DrawParam::default()
                    .dest(Point2 { x: self.x + 20.0, y: self.y + 70.0 })
                    .color(Color::from_rgb(200, 255, 200))
                    .scale([14.0f32 / 40.0, 14.0f32 / 40.0]),
            );
            
            let partner_label = Text::new(format!("{} 的物品", trade_data.partner_name));
            canvas.draw(
                &partner_label,
                DrawParam::default()
                    .dest(Point2 { x: self.x + 260.0, y: self.y + 70.0 })
                    .color(Color::from_rgb(255, 200, 200))
                    .scale([14.0f32 / 40.0, 14.0f32 / 40.0]),
            );
            
            // 绘制我的物品格子
            self.draw_item_grid(ctx, canvas, self.x + 20.0, self.y + 100.0, &trade_data.my_items, trade_data.my_locked)?;
            
            // 绘制对方物品格子 (只读)
            self.draw_partner_items(ctx, canvas, self.x + 260.0, self.y + 100.0, &trade_data.partner_items)?;
            
            // 绘制金币信息
            let my_gold_text = format!("我的金币: {}", trade_data.my_gold);
            let my_gold = Text::new(&my_gold_text);
            canvas.draw(
                &my_gold,
                DrawParam::default()
                    .dest(Point2 { x: self.x + 20.0, y: self.y + 305.0 })
                    .color(Color::from_rgb(255, 215, 0))
                    .scale([14.0f32 / 40.0, 14.0f32 / 40.0]),
            );
            
            let partner_gold_text = format!("对方金币: {}", trade_data.partner_gold);
            let partner_gold = Text::new(&partner_gold_text);
            canvas.draw(
                &partner_gold,
                DrawParam::default()
                    .dest(Point2 { x: self.x + 260.0, y: self.y + 305.0 })
                    .color(Color::from_rgb(255, 215, 0))
                    .scale([14.0f32 / 40.0, 14.0f32 / 40.0]),
            );
            
            // 绘制锁定状态
            if trade_data.my_locked {
                let my_lock_text = Text::new("已锁定");
                canvas.draw(
                    &my_lock_text,
                    DrawParam::default()
                        .dest(Point2 { x: self.x + 20.0, y: self.y + 270.0 })
                        .color(Color::from_rgb(100, 255, 100))
                        .scale([12.0f32 / 40.0, 12.0f32 / 40.0]),
                );
            }
            
            if trade_data.partner_locked {
                let partner_lock_text = Text::new("对方已锁定");
                canvas.draw(
                    &partner_lock_text,
                    DrawParam::default()
                        .dest(Point2 { x: self.x + 260.0, y: self.y + 270.0 })
                        .color(Color::from_rgb(100, 255, 100))
                        .scale([12.0f32 / 40.0, 12.0f32 / 40.0]),
                );
            }
            
            // 绘制按钮
            self.draw_buttons(ctx, canvas, trade_data)?;
        }
        
        Ok(())
    }
    
    fn draw_item_grid(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        start_x: f32,
        start_y: f32,
        items: &[(u8, UserItem)],
        is_locked: bool,
    ) -> GameResult {
        let cell_size = 40.0;
        let cols = 5;
        let rows = 4;
        
        for row in 0..rows {
            for col in 0..cols {
                let cell_x = start_x + col as f32 * (cell_size + 5.0);
                let cell_y = start_y + row as f32 * (cell_size + 5.0);
                
                // 绘制格子背景
                let cell_rect = Rect::new(cell_x, cell_y, cell_size, cell_size);
                let cell_color = if is_locked {
                    Color::from_rgba(40, 40, 40, 200)
                } else {
                    Color::from_rgba(30, 30, 40, 200)
                };
                
                let cell_mesh = ggez::graphics::Mesh::new_rectangle(
                    ctx,
                    ggez::graphics::DrawMode::fill(),
                    cell_rect,
                    cell_color,
                )?;
                canvas.draw(&cell_mesh, DrawParam::default());
                
                let border_mesh = ggez::graphics::Mesh::new_rectangle(
                    ctx,
                    ggez::graphics::DrawMode::stroke(1.0),
                    cell_rect,
                    Color::from_rgb(80, 80, 100),
                )?;
                canvas.draw(&border_mesh, DrawParam::default());
                
                // 绘制物品 (如果有)
                let slot = (row * cols + col) as u8;
                if let Some((_, item)) = items.iter().find(|(s, _)| *s == slot) {
                    // TODO: 绘制物品图标
                    let item_text = Text::new("物");
                    canvas.draw(
                        &item_text,
                        DrawParam::default()
                            .dest(Point2 { x: cell_x + 10.0, y: cell_y + 10.0 })
                            .color(Color::from_rgb(255, 255, 100))
                            .scale([16.0f32 / 40.0, 16.0f32 / 40.0]),
                    );
                }
            }
        }
        
        Ok(())
    }
    
    fn draw_partner_items(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        start_x: f32,
        start_y: f32,
        items: &[UserItem],
    ) -> GameResult {
        let cell_size = 40.0;
        let cols = 5;
        let rows = 4;
        
        for row in 0..rows {
            for col in 0..cols {
                let cell_x = start_x + col as f32 * (cell_size + 5.0);
                let cell_y = start_y + row as f32 * (cell_size + 5.0);
                
                let cell_rect = Rect::new(cell_x, cell_y, cell_size, cell_size);
                let cell_mesh = ggez::graphics::Mesh::new_rectangle(
                    ctx,
                    ggez::graphics::DrawMode::fill(),
                    cell_rect,
                    Color::from_rgba(30, 30, 40, 200),
                )?;
                canvas.draw(&cell_mesh, DrawParam::default());
                
                let border_mesh = ggez::graphics::Mesh::new_rectangle(
                    ctx,
                    ggez::graphics::DrawMode::stroke(1.0),
                    cell_rect,
                    Color::from_rgb(80, 80, 100),
                )?;
                canvas.draw(&border_mesh, DrawParam::default());
                
                // 绘制对方的物品
                let index = (row * cols + col) as usize;
                if index < items.len() {
                    // TODO: 绘制物品图标
                    let item_text = Text::new("物");
                    canvas.draw(
                        &item_text,
                        DrawParam::default()
                            .dest(Point2 { x: cell_x + 10.0, y: cell_y + 10.0 })
                            .color(Color::from_rgb(200, 200, 255))
                            .scale([16.0f32 / 40.0, 16.0f32 / 40.0]),
                    );
                }
            }
        }
        
        Ok(())
    }
    
    fn draw_buttons(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        trade_data: &TradeData,
    ) -> GameResult {
        let button_y = self.y + self.height - 50.0;
        
        // "锁定" 按钮
        let lock_color = if trade_data.my_locked {
            Color::from_rgb(100, 100, 100)
        } else {
            Color::from_rgb(50, 100, 200)
        };
        self.draw_button(ctx, canvas, self.x + 50.0, button_y, 100.0, 35.0, "锁定", lock_color)?;
        
        // "确认" 按钮 (双方都锁定后才能确认)
        let confirm_enabled = trade_data.my_locked && trade_data.partner_locked;
        let confirm_color = if confirm_enabled {
            Color::from_rgb(50, 150, 50)
        } else {
            Color::from_rgb(80, 80, 80)
        };
        self.draw_button(ctx, canvas, self.x + 200.0, button_y, 100.0, 35.0, "确认", confirm_color)?;
        
        // "取消" 按钮
        self.draw_button(ctx, canvas, self.x + 350.0, button_y, 100.0, 35.0, "取消", Color::from_rgb(150, 50, 50))?;
        
        Ok(())
    }
    
    fn draw_button(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        text: &str,
        color: Color,
    ) -> GameResult {
        let button_rect = Rect::new(x, y, width, height);
        let button_mesh = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::fill(),
            button_rect,
            color,
        )?;
        canvas.draw(&button_mesh, DrawParam::default());
        
        let border_mesh = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::stroke(1.0),
            button_rect,
            Color::from_rgb(200, 200, 200),
        )?;
        canvas.draw(&border_mesh, DrawParam::default());
        
        let button_text = Text::new(text);
        canvas.draw(
            &button_text,
            DrawParam::default()
                .dest(Point2 { x: x + 25.0, y: y + 8.0 })
                .color(Color::WHITE)
                .scale([14.0f32 / 40.0, 14.0f32 / 40.0]),
        );
        
        Ok(())
    }
}

/// 交易动作
#[derive(Debug, Clone)]
pub enum TradeAction {
    ClickMySlot(u8),
    ClickPartnerSlot(u8),
    ClickGoldInput,
    Lock,
    Confirm,
    Cancel,
}

fn point_in_rect(x: f32, y: f32, rect: Rect) -> bool {
    x >= rect.x && x <= rect.x + rect.w && y >= rect.y && y <= rect.y + rect.h
}

