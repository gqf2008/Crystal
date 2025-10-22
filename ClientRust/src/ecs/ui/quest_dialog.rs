// ============================================================================
// 任务对话框 - ECS组件方式
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::{Canvas, Color, Rect, Text, DrawParam, PxScale};
use ggez::mint::Point2;
use crate::ecs::systems::{Quest, QuestState, QuestObjective};

/// 任务对话框组件
#[derive(Debug, Clone)]
pub struct QuestDialogComp {
    pub is_open: bool,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub active_quests: Vec<Quest>,      // 进行中的任务
    pub available_quests: Vec<Quest>,   // 可接取的任务
    pub selected_index: Option<usize>,   // 当前选中的任务索引
    pub view_mode: QuestViewMode,        // 查看模式
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuestViewMode {
    ActiveQuests,    // 进行中的任务
    AvailableQuests, // 可接取的任务
    CompletedQuests, // 已完成的任务
}

impl QuestDialogComp {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            is_open: false,
            x,
            y,
            width: 400.0,
            height: 500.0,
            active_quests: Vec::new(),
            available_quests: Vec::new(),
            selected_index: None,
            view_mode: QuestViewMode::ActiveQuests,
        }
    }
    
    /// 打开对话框
    pub fn open(&mut self) {
        self.is_open = true;
    }
    
    /// 关闭对话框
    pub fn close(&mut self) {
        self.is_open = false;
    }
    
    /// 切换查看模式
    pub fn switch_mode(&mut self, mode: QuestViewMode) {
        self.view_mode = mode;
        self.selected_index = None;
    }
    
    /// 更新进行中的任务列表
    pub fn update_active_quests(&mut self, quests: Vec<Quest>) {
        self.active_quests = quests;
    }
    
    /// 更新可接取的任务列表
    pub fn update_available_quests(&mut self, quests: Vec<Quest>) {
        self.available_quests = quests;
    }
    
    /// 选择任务
    pub fn select_quest(&mut self, index: usize) {
        let max_index = match self.view_mode {
            QuestViewMode::ActiveQuests => self.active_quests.len(),
            QuestViewMode::AvailableQuests => self.available_quests.len(),
            QuestViewMode::CompletedQuests => 0,
        };
        
        if index < max_index {
            self.selected_index = Some(index);
        }
    }
    
    /// 获取当前选中的任务
    pub fn get_selected_quest(&self) -> Option<&Quest> {
        if let Some(index) = self.selected_index {
            match self.view_mode {
                QuestViewMode::ActiveQuests => self.active_quests.get(index),
                QuestViewMode::AvailableQuests => self.available_quests.get(index),
                QuestViewMode::CompletedQuests => None,
            }
        } else {
            None
        }
    }
    
    /// 检查鼠标点击
    pub fn handle_click(&mut self, mouse_x: f32, mouse_y: f32) -> Option<QuestAction> {
        if !self.is_open {
            return None;
        }
        
        // 检查是否点击关闭按钮
        let close_button_rect = Rect::new(
            self.x + self.width - 30.0,
            self.y + 5.0,
            25.0,
            25.0,
        );
        
        if point_in_rect(mouse_x, mouse_y, close_button_rect) {
            return Some(QuestAction::Close);
        }
        
        // 检查标签页切换
        let tab_y = self.y + 40.0;
        let tab_height = 30.0;
        
        if mouse_y >= tab_y && mouse_y <= tab_y + tab_height {
            if mouse_x >= self.x && mouse_x < self.x + 133.0 {
                return Some(QuestAction::SwitchTab(QuestViewMode::ActiveQuests));
            } else if mouse_x >= self.x + 133.0 && mouse_x < self.x + 266.0 {
                return Some(QuestAction::SwitchTab(QuestViewMode::AvailableQuests));
            } else if mouse_x >= self.x + 266.0 && mouse_x < self.x + 400.0 {
                return Some(QuestAction::SwitchTab(QuestViewMode::CompletedQuests));
            }
        }
        
        // 检查任务列表点击
        let list_y = self.y + 80.0;
        let item_height = 40.0;
        
        let quest_list = match self.view_mode {
            QuestViewMode::ActiveQuests => &self.active_quests,
            QuestViewMode::AvailableQuests => &self.available_quests,
            QuestViewMode::CompletedQuests => return None,
        };
        
        for (i, _) in quest_list.iter().enumerate() {
            let item_y = list_y + i as f32 * item_height;
            let item_rect = Rect::new(self.x + 10.0, item_y, self.width - 20.0, item_height - 5.0);
            
            if point_in_rect(mouse_x, mouse_y, item_rect) {
                return Some(QuestAction::SelectQuest(i));
            }
        }
        
        // 检查动作按钮
        let button_y = self.y + self.height - 50.0;
        
        if mouse_y >= button_y && mouse_y <= button_y + 35.0 {
            match self.view_mode {
                QuestViewMode::ActiveQuests => {
                    // "放弃任务" 和 "交付任务" 按钮
                    if mouse_x >= self.x + 20.0 && mouse_x <= self.x + 120.0 {
                        if let Some(index) = self.selected_index {
                            return Some(QuestAction::AbandonQuest(index));
                        }
                    } else if mouse_x >= self.x + 280.0 && mouse_x <= self.x + 380.0 {
                        if let Some(index) = self.selected_index {
                            if let Some(quest) = self.active_quests.get(index) {
                                if quest.completed {  // ✅ 使用completed字段
                                    return Some(QuestAction::SubmitQuest(quest.id));
                                }
                            }
                        }
                    }
                }
                QuestViewMode::AvailableQuests => {
                    // "接受任务" 按钮
                    if mouse_x >= self.x + 150.0 && mouse_x <= self.x + 250.0 {
                        if let Some(index) = self.selected_index {
                            if let Some(quest) = self.available_quests.get(index) {
                                return Some(QuestAction::AcceptQuest(quest.clone()));
                            }
                        }
                    }
                }
                QuestViewMode::CompletedQuests => {}
            }
        }
        
        None
    }
    
    /// 渲染对话框
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
            Color::from_rgba(20, 20, 30, 230),
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
        
        // 绘制标题
        let title = Text::new("任务日志");
        canvas.draw(
            &title,
            DrawParam::default()
                .dest(Point2 { x: self.x + 10.0, y: self.y + 10.0 })
                .color(Color::from_rgb(220, 220, 255))
                .scale([20.0f32 / 40.0, 20.0f32 / 40.0]),
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
        
        // 绘制标签页
        self.draw_tabs(ctx, canvas)?;
        
        // 绘制任务列表
        self.draw_quest_list(ctx, canvas)?;
        
        // 绘制任务详情
        self.draw_quest_details(ctx, canvas)?;
        
        // 绘制动作按钮
        self.draw_action_buttons(ctx, canvas)?;
        
        Ok(())
    }
    
    fn draw_tabs(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        let tab_y = self.y + 40.0;
        let tab_width = self.width / 3.0;
        let tab_height = 30.0;
        
        let tabs = [
            ("进行中", QuestViewMode::ActiveQuests),
            ("可接取", QuestViewMode::AvailableQuests),
            ("已完成", QuestViewMode::CompletedQuests),
        ];
        
        for (i, (name, mode)) in tabs.iter().enumerate() {
            let tab_x = self.x + i as f32 * tab_width;
            let is_active = *mode == self.view_mode;
            
            let tab_rect = Rect::new(tab_x, tab_y, tab_width, tab_height);
            let tab_color = if is_active {
                Color::from_rgba(60, 60, 80, 255)
            } else {
                Color::from_rgba(40, 40, 50, 200)
            };
            
            let tab_mesh = ggez::graphics::Mesh::new_rectangle(
                ctx,
                ggez::graphics::DrawMode::fill(),
                tab_rect,
                tab_color,
            )?;
            canvas.draw(&tab_mesh, DrawParam::default());
            
            let text = Text::new(*name);
            canvas.draw(
                &text,
                DrawParam::default()
                    .dest(Point2 { x: tab_x + 20.0, y: tab_y + 5.0 })
                    .color(if is_active { Color::WHITE } else { Color::from_rgb(150, 150, 150) })
                    .scale([16.0f32 / 40.0, 16.0f32 / 40.0]),
            );
        }
        
        Ok(())
    }
    
    fn draw_quest_list(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        let list_y = self.y + 80.0;
        let item_height = 40.0;
        
        let quest_list = match self.view_mode {
            QuestViewMode::ActiveQuests => &self.active_quests,
            QuestViewMode::AvailableQuests => &self.available_quests,
            QuestViewMode::CompletedQuests => return Ok(()),
        };
        
        for (i, quest) in quest_list.iter().enumerate() {
            let item_y = list_y + i as f32 * item_height;
            let is_selected = Some(i) == self.selected_index;
            
            // 绘制选中背景
            if is_selected {
                let select_rect = Rect::new(
                    self.x + 10.0,
                    item_y,
                    self.width - 20.0,
                    item_height - 5.0,
                );
                let select_mesh = ggez::graphics::Mesh::new_rectangle(
                    ctx,
                    ggez::graphics::DrawMode::fill(),
                    select_rect,
                    Color::from_rgba(80, 80, 120, 100),
                )?;
                canvas.draw(&select_mesh, DrawParam::default());
            }
            
            // 绘制任务名称，根据状态显示颜色
            let name_color = if quest.completed {
                Color::from_rgb(100, 255, 100)  // 已完成 - 绿色
            } else if quest.taken {
                Color::from_rgb(200, 200, 200)  // 进行中 - 灰白色
            } else {
                Color::from_rgb(255, 255, 100)  // 可接取 - 黄色
            };
            
            let name_text = Text::new(&quest.name);
            canvas.draw(
                &name_text,
                DrawParam::default()
                    .dest(Point2 { x: self.x + 15.0, y: item_y + 5.0 })
                    .color(name_color)
                    .scale([14.0f32 / 40.0, 14.0f32 / 40.0]),
            );
            
            // 绘制状态标签
            let status = if quest.completed {
                "可交付"
            } else if quest.taken {
                "进行中"
            } else {
                "可接取"
            };
            
            let status_text = Text::new(status);
            canvas.draw(
                &status_text,
                DrawParam::default()
                    .dest(Point2 { x: self.x + self.width - 80.0, y: item_y + 5.0 })
                    .color(name_color)
                    .scale([12.0f32 / 40.0, 12.0f32 / 40.0]),
            );
        }
        
        Ok(())
    }
    
    fn draw_quest_details(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        if let Some(quest) = self.get_selected_quest() {
            let details_y = self.y + 300.0;
            
            // 绘制分隔线
            let line_mesh = ggez::graphics::Mesh::new_rectangle(
                ctx,
                ggez::graphics::DrawMode::fill(),
                Rect::new(self.x + 10.0, details_y - 5.0, self.width - 20.0, 2.0),
                Color::from_rgb(100, 100, 120),
            )?;
            canvas.draw(&line_mesh, DrawParam::default());
            
            // 绘制任务描述
            let desc_text = Text::new(&quest.description);
            canvas.draw(
                &desc_text,
                DrawParam::default()
                    .dest(Point2 { x: self.x + 15.0, y: details_y + 5.0 })
                    .color(Color::from_rgb(200, 200, 200))
                    .scale([12.0f32 / 40.0, 12.0f32 / 40.0]),
            );
            
            // 绘制目标进度
            let mut obj_y = details_y + 50.0;
            for objective in &quest.objectives {
                let progress_text = Text::new(objective.get_progress_text());
                let color = if objective.is_complete() {
                    Color::from_rgb(100, 255, 100)
                } else {
                    Color::from_rgb(200, 200, 200)
                };
                
                canvas.draw(
                    &progress_text,
                    DrawParam::default()
                        .dest(Point2 { x: self.x + 20.0, y: obj_y })
                        .color(color)
                        .scale([12.0f32 / 40.0, 12.0f32 / 40.0]),
                );
                
                obj_y += 20.0;
            }
            
            // 绘制奖励信息
            let reward_y = self.y + self.height - 120.0;
            let reward_title = Text::new("奖励:");
            canvas.draw(
                &reward_title,
                DrawParam::default()
                    .dest(Point2 { x: self.x + 15.0, y: reward_y })
                    .color(Color::from_rgb(255, 215, 0))
                    .scale([14.0f32 / 40.0, 14.0f32 / 40.0]),
            );
            
            let reward_info = format!(
                "金币: {}  经验: {}  物品: {}个",
                quest.reward.gold,
                quest.reward.experience,
                quest.reward.items.len()
            );
            let reward_text = Text::new(reward_info);
            canvas.draw(
                &reward_text,
                DrawParam::default()
                    .dest(Point2 { x: self.x + 15.0, y: reward_y + 20.0 })
                    .color(Color::from_rgb(200, 200, 200))
                    .scale([12.0f32 / 40.0, 12.0f32 / 40.0]),
            );
        }
        
        Ok(())
    }
    
    fn draw_action_buttons(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        let button_y = self.y + self.height - 50.0;
        
        match self.view_mode {
            QuestViewMode::ActiveQuests => {
                // "放弃任务" 按钮
                self.draw_button(ctx, canvas, self.x + 20.0, button_y, 100.0, 35.0, "放弃任务", Color::from_rgb(150, 50, 50))?;
                
                // "交付任务" 按钮 (只有完成状态才显示)
                if let Some(quest) = self.get_selected_quest() {
                    if quest.completed {  // ✅ 使用completed字段
                        self.draw_button(ctx, canvas, self.x + 280.0, button_y, 100.0, 35.0, "交付任务", Color::from_rgb(50, 150, 50))?;
                    }
                }
            }
            QuestViewMode::AvailableQuests => {
                // "接受任务" 按钮
                if self.selected_index.is_some() {
                    self.draw_button(ctx, canvas, self.x + 150.0, button_y, 100.0, 35.0, "接受任务", Color::from_rgb(50, 100, 200))?;
                }
            }
            QuestViewMode::CompletedQuests => {}
        }
        
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
                .dest(Point2 { x: x + 10.0, y: y + 8.0 })
                .color(Color::WHITE)
                .scale([14.0f32 / 40.0, 14.0f32 / 40.0]),
        );
        
        Ok(())
    }
}

/// 任务对话框动作
#[derive(Debug, Clone)]
pub enum QuestAction {
    Close,
    SwitchTab(QuestViewMode),
    SelectQuest(usize),
    AcceptQuest(Quest),
    AbandonQuest(usize),
    SubmitQuest(i32),  // ✅ 修正为i32以匹配Quest.id类型
}

/// 检查点是否在矩形内
fn point_in_rect(x: f32, y: f32, rect: Rect) -> bool {
    x >= rect.x && x <= rect.x + rect.w && y >= rect.y && y <= rect.y + rect.h
}

