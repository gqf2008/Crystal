// ============================================================================
// 技能学习对话框 - 显示可学习技能列表
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::{Canvas, Color, Rect, DrawParam, Text, TextFragment};
use crate::ecs::components::{SpellType, LearnedMagic};

/// 技能学习对话框动作
#[derive(Debug, Clone, Copy)]
pub enum MagicLearningAction {
    Close,
    SelectMagic(usize),      // 选中某个技能
    StartDragMagic(usize),   // 开始拖拽技能
    LearnMagic(SpellType),   // 学习技能
}

/// 技能学习对话框
pub struct MagicLearningDialog {
    pub visible: bool,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    
    /// 可学习的技能列表 (技能类型, 所需等级)
    pub available_magics: Vec<(SpellType, u16)>,
    
    /// 当前选中的技能索引
    pub selected_index: Option<usize>,
    
    /// 悬停的技能索引
    pub hover_index: Option<usize>,
    
    /// 拖拽状态
    pub dragging_index: Option<usize>,
    pub drag_offset_x: f32,
    pub drag_offset_y: f32,
    
    /// 滚动位置
    pub scroll_offset: f32,
}

impl MagicLearningDialog {
    const ITEM_HEIGHT: f32 = 40.0;
    const PADDING: f32 = 10.0;
    const SCROLL_SPEED: f32 = 20.0;
    
    pub fn new() -> Self {
        Self {
            visible: false,
            x: 300.0,
            y: 100.0,
            width: 300.0,
            height: 400.0,
            available_magics: Vec::new(),
            selected_index: None,
            hover_index: None,
            dragging_index: None,
            drag_offset_x: 0.0,
            drag_offset_y: 0.0,
            scroll_offset: 0.0,
        }
    }
    
    /// 设置可学习技能列表
    pub fn set_available_magics(&mut self, magics: Vec<(SpellType, u16)>) {
        self.available_magics = magics;
        self.scroll_offset = 0.0;
        self.selected_index = None;
    }
    
    /// 显示/隐藏对话框
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if !self.visible {
            self.selected_index = None;
            self.hover_index = None;
            self.dragging_index = None;
        }
    }
    
    /// 设置可见性
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
        if !visible {
            self.selected_index = None;
            self.hover_index = None;
            self.dragging_index = None;
        }
    }
    
    pub fn show(&mut self) {
        self.visible = true;
    }
    
    pub fn hide(&mut self) {
        self.visible = false;
        self.selected_index = None;
        self.hover_index = None;
        self.dragging_index = None;
    }
    
    /// 检查是否打开
    pub fn is_open(&self) -> bool {
        self.visible
    }
    
    /// 检查点是否在对话框内
    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        if !self.visible {
            return false;
        }
        x >= self.x && x <= self.x + self.width &&
        y >= self.y && y <= self.y + self.height
    }
    
    /// 鼠标点击处理
    pub fn on_mouse_down(&mut self, x: f32, y: f32) -> Option<MagicLearningAction> {
        if !self.visible {
            return None;
        }
        
        // 检查关闭按钮 (右上角)
        let close_btn_x = self.x + self.width - 30.0;
        let close_btn_y = self.y + 5.0;
        if x >= close_btn_x && x <= close_btn_x + 25.0 &&
           y >= close_btn_y && y <= close_btn_y + 25.0 {
            return Some(MagicLearningAction::Close);
        }
        
        // 检查技能项点击
        if let Some(index) = self.get_item_at_pos(x, y) {
            self.selected_index = Some(index);
            self.dragging_index = Some(index);
            self.drag_offset_x = x - self.x;
            self.drag_offset_y = y - (self.y + 40.0 + index as f32 * Self::ITEM_HEIGHT - self.scroll_offset);
            return Some(MagicLearningAction::StartDragMagic(index));
        }
        
        None
    }
    
    /// 鼠标释放处理
    pub fn on_mouse_up(&mut self, _x: f32, _y: f32) -> Option<MagicLearningAction> {
        let result = if let Some(index) = self.dragging_index {
            if index < self.available_magics.len() {
                Some(MagicLearningAction::LearnMagic(self.available_magics[index].0))
            } else {
                None
            }
        } else {
            None
        };
        
        self.dragging_index = None;
        result
    }
    
    /// 鼠标移动处理
    pub fn on_mouse_move(&mut self, x: f32, y: f32) {
        if !self.visible {
            return;
        }
        
        // 更新悬停状态
        self.hover_index = self.get_item_at_pos(x, y);
        
        // 更新拖拽位置
        if self.dragging_index.is_some() {
            self.drag_offset_x = x - self.x;
            self.drag_offset_y = y - self.y;
        }
    }
    
    /// 鼠标滚轮处理
    pub fn on_scroll(&mut self, delta: f32) {
        if !self.visible {
            return;
        }
        
        self.scroll_offset -= delta * Self::SCROLL_SPEED;
        
        // 限制滚动范围
        let max_scroll = (self.available_magics.len() as f32 * Self::ITEM_HEIGHT - (self.height - 50.0)).max(0.0);
        self.scroll_offset = self.scroll_offset.clamp(0.0, max_scroll);
    }
    
    /// 获取指定位置的技能索引
    fn get_item_at_pos(&self, x: f32, y: f32) -> Option<usize> {
        if !self.contains_point(x, y) {
            return None;
        }
        
        let list_y = self.y + 40.0;
        let list_height = self.height - 50.0;
        
        if y < list_y || y > list_y + list_height {
            return None;
        }
        
        let relative_y = y - list_y + self.scroll_offset;
        let index = (relative_y / Self::ITEM_HEIGHT) as usize;
        
        if index < self.available_magics.len() {
            Some(index)
        } else {
            None
        }
    }
    
    /// 渲染对话框
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        if !self.visible {
            return Ok(());
        }
        
        // 绘制背景
        let bg_rect = Rect::new(self.x, self.y, self.width, self.height);
        let bg_mesh = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::fill(),
            bg_rect,
            Color::from_rgba(20, 20, 20, 230),
        )?;
        canvas.draw(&bg_mesh, DrawParam::default());
        
        // 绘制边框
        let border_mesh = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::stroke(2.0),
            bg_rect,
            Color::from_rgb(180, 140, 80),
        )?;
        canvas.draw(&border_mesh, DrawParam::default());
        
        // 绘制标题
        let title = Text::new(TextFragment::new("可学习技能").color(Color::from_rgb(255, 220, 150)));
        canvas.draw(
            &title,
            DrawParam::default()
                .dest([self.x + Self::PADDING, self.y + Self::PADDING])
        );
        
        // 绘制关闭按钮
        let close_btn_x = self.x + self.width - 30.0;
        let close_btn_y = self.y + 5.0;
        let close_btn = Rect::new(close_btn_x, close_btn_y, 25.0, 25.0);
        let close_mesh = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::fill(),
            close_btn,
            Color::from_rgb(150, 50, 50),
        )?;
        canvas.draw(&close_mesh, DrawParam::default());
        
        let close_text = Text::new(TextFragment::new("×").color(Color::WHITE));
        canvas.draw(
            &close_text,
            DrawParam::default()
                .dest([close_btn_x + 6.0, close_btn_y + 2.0])
        );
        
        // 设置裁剪区域用于滚动
        let list_y = self.y + 40.0;
        let list_height = self.height - 50.0;
        
        // 绘制技能列表
        for (i, (spell, req_level)) in self.available_magics.iter().enumerate() {
            let item_y = list_y + i as f32 * Self::ITEM_HEIGHT - self.scroll_offset;
            
            // 跳过不可见的项
            if item_y + Self::ITEM_HEIGHT < list_y || item_y > list_y + list_height {
                continue;
            }
            
            // 如果正在拖拽这个技能，跳过（会在最后绘制）
            if self.dragging_index == Some(i) {
                continue;
            }
            
            self.draw_magic_item(ctx, canvas, i, *spell, *req_level, self.x + Self::PADDING, item_y)?;
        }
        
        // 绘制正在拖拽的技能
        if let Some(drag_index) = self.dragging_index {
            if drag_index < self.available_magics.len() {
                let (spell, req_level) = self.available_magics[drag_index];
                let drag_x = self.x + self.drag_offset_x - 50.0;
                let drag_y = self.y + self.drag_offset_y - 15.0;
                self.draw_magic_item(ctx, canvas, drag_index, spell, req_level, drag_x, drag_y)?;
            }
        }
        
        Ok(())
    }
    
    /// 绘制单个技能项
    fn draw_magic_item(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        index: usize,
        spell: SpellType,
        req_level: u16,
        x: f32,
        y: f32,
    ) -> GameResult {
        let item_width = self.width - Self::PADDING * 2.0;
        let item_rect = Rect::new(x, y, item_width, Self::ITEM_HEIGHT - 5.0);
        
        // 背景色（选中/悬停/普通）
        let bg_color = if Some(index) == self.selected_index {
            Color::from_rgba(100, 80, 40, 200)
        } else if Some(index) == self.hover_index {
            Color::from_rgba(60, 50, 30, 180)
        } else {
            Color::from_rgba(40, 40, 40, 150)
        };
        
        let item_bg = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::fill(),
            item_rect,
            bg_color,
        )?;
        canvas.draw(&item_bg, DrawParam::default());
        
        // 边框
        let item_border = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::stroke(1.0),
            item_rect,
            Color::from_rgb(100, 80, 40),
        )?;
        canvas.draw(&item_border, DrawParam::default());
        
        // 技能名称
        let name_text = Text::new(
            TextFragment::new(spell.name())
                .color(Color::from_rgb(255, 255, 200))
        );
        canvas.draw(
            &name_text,
            DrawParam::default()
                .dest([x + 5.0, y + 5.0])
        );
        
        // 所需等级
        let level_text = Text::new(
            TextFragment::new(format!("需要等级: {}", req_level))
                .color(Color::from_rgb(180, 180, 180))
        );
        canvas.draw(
            &level_text,
            DrawParam::default()
                .dest([x + 5.0, y + 20.0])
        );
        
        Ok(())
    }
}
