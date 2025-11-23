/// 任务追踪对话框 - 屏幕上显示当前进行中的任务
/// 对应原工程 QuestDialogs.cs 中的 QuestTrackingDialog
/// 
/// 功能：
/// - 显示当前进行中的任务列表
/// - 显示任务进度
/// - 点击任务可查看详情
/// - 可拖拽移动位置

use egui_macroquad::egui;
use crate::scenes::dialogs::Dialog;
use super::quest_log_dialog::{QuestInfo, QuestStatus};

/// 任务追踪对话框
pub struct QuestTrackingDialog {
    visible: bool,
    position: egui::Pos2,
    
    /// 正在追踪的任务列表
    tracking_quests: Vec<QuestInfo>,
    
    /// 窗口拖拽状态
    dragging: bool,
    drag_offset: egui::Vec2,
    
    /// 最大显示任务数
    max_quests: usize,
}

impl QuestTrackingDialog {
    pub fn new() -> Self {
        Self {
            visible: true,  // 默认显示
            position: egui::pos2(10.0, 150.0),
            tracking_quests: Vec::new(),
            dragging: false,
            drag_offset: egui::vec2(0.0, 0.0),
            max_quests: 5,
        }
    }
    
    /// 设置追踪的任务列表
    pub fn set_tracking_quests(&mut self, quests: Vec<QuestInfo>) {
        // 只显示进行中的任务
        self.tracking_quests = quests.into_iter()
            .filter(|q| q.status == QuestStatus::Accepted)
            .take(self.max_quests)
            .collect();
    }
    
    /// 添加任务到追踪列表
    pub fn add_quest(&mut self, quest: QuestInfo) {
        if self.tracking_quests.len() < self.max_quests {
            self.tracking_quests.push(quest);
        }
    }
    
    /// 移除任务
    pub fn remove_quest(&mut self, quest_id: u32) {
        self.tracking_quests.retain(|q| q.id != quest_id);
    }
    
    /// 获取可见状态
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    
    /// 切换显示状态
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }
    
    /// 绘制单个追踪任务
    fn draw_tracking_quest(&mut self, ui: &mut egui::Ui, quest: &QuestInfo, pos: egui::Pos2, width: f32) -> f32 {
        let start_y = pos.y;
        let mut y_offset = start_y;
        
        // 任务名称
        let name_color = if quest.progress >= quest.max_progress {
            egui::Color32::GREEN
        } else {
            egui::Color32::YELLOW
        };
        
        ui.painter().text(
            egui::pos2(pos.x + 5.0, y_offset),
            egui::Align2::LEFT_TOP,
            &quest.name,
            egui::FontId::proportional(12.0),
            name_color,
        );
        y_offset += 18.0;
        
        // 任务进度
        if quest.max_progress > 0 {
            // 进度文字
            let progress_text = format!("{} / {}", quest.progress, quest.max_progress);
            ui.painter().text(
                egui::pos2(pos.x + 10.0, y_offset),
                egui::Align2::LEFT_TOP,
                &progress_text,
                egui::FontId::proportional(10.0),
                egui::Color32::LIGHT_GRAY,
            );
            y_offset += 15.0;
            
            // 进度条
            let progress_rect = egui::Rect::from_min_size(
                egui::pos2(pos.x + 10.0, y_offset),
                egui::vec2(width - 20.0, 6.0)
            );
            
            ui.painter().rect_filled(
                progress_rect,
                2.0,
                egui::Color32::from_rgba_premultiplied(40, 40, 40, 200),
            );
            
            let progress_percent = quest.progress as f32 / quest.max_progress as f32;
            let filled_rect = egui::Rect::from_min_size(
                progress_rect.min,
                egui::vec2(progress_rect.width() * progress_percent, progress_rect.height())
            );
            
            let progress_color = if progress_percent >= 1.0 {
                egui::Color32::from_rgb(100, 200, 100)
            } else {
                egui::Color32::from_rgb(200, 150, 50)
            };
            
            ui.painter().rect_filled(
                filled_rect,
                2.0,
                progress_color,
            );
            
            y_offset += 10.0;
        }
        
        // 分隔线
        ui.painter().line_segment(
            [
                egui::pos2(pos.x + 5.0, y_offset),
                egui::pos2(pos.x + width - 5.0, y_offset),
            ],
            egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(80, 80, 80, 150)),
        );
        y_offset += 5.0;
        
        // 返回总高度
        y_offset - start_y
    }
    
    /// 处理窗口拖拽
    fn handle_window_dragging(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, area_rect: &egui::Rect) {
        let drag_response = ui.interact(*area_rect, egui::Id::new("quest_tracking_drag"), egui::Sense::drag());
        
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
}

impl Dialog for QuestTrackingDialog {
    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        if !self.visible || self.tracking_quests.is_empty() {
            *open = false;
            return;
        }
        
        egui::Area::new(egui::Id::new("quest_tracking_dialog"))
            .fixed_pos(self.position)
            .movable(false)
            .show(ctx, |ui| {
                // 计算所需的总高度
                let quest_width = 250.0;
                let title_height = 25.0;
                let padding = 10.0;
                
                // 估算内容高度(每个任务约50像素)
                let content_height = self.tracking_quests.len() as f32 * 55.0;
                let total_height = title_height + content_height + padding * 2.0;
                
                let area_rect = egui::Rect::from_min_size(
                    self.position,
                    egui::vec2(quest_width, total_height)
                );
                
                // 绘制半透明背景
                ui.painter().rect_filled(
                    area_rect,
                    5.0,
                    egui::Color32::from_rgba_premultiplied(25, 25, 30, 220),
                );
                ui.painter().rect_stroke(
                    area_rect,
                    5.0,
                    egui::Stroke::new(1.5, egui::Color32::from_rgba_premultiplied(100, 100, 100, 180)),
                    egui::epaint::StrokeKind::Outside,
                );
                
                // 标题
                ui.painter().text(
                    egui::pos2(area_rect.min.x + quest_width / 2.0, area_rect.min.y + 12.0),
                    egui::Align2::CENTER_CENTER,
                    "任务追踪",
                    egui::FontId::proportional(13.0),
                    egui::Color32::from_rgb(220, 200, 100),
                );
                
                // 标题下划线
                ui.painter().line_segment(
                    [
                        egui::pos2(area_rect.min.x + 10.0, area_rect.min.y + title_height),
                        egui::pos2(area_rect.max.x - 10.0, area_rect.min.y + title_height),
                    ],
                    egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(120, 120, 120, 180)),
                );
                
                // 绘制追踪的任务
                let mut current_y = area_rect.min.y + title_height + padding;
                
                // 克隆任务列表以避免借用冲突
                let quests: Vec<_> = self.tracking_quests.clone();
                for quest in &quests {
                    let quest_height = self.draw_tracking_quest(
                        ui,
                        quest,
                        egui::pos2(area_rect.min.x, current_y),
                        quest_width
                    );
                    current_y += quest_height;
                }
                
                // 处理拖拽
                self.handle_window_dragging(ui, ctx, &area_rect);
            });
        
        *open = self.visible;
    }
}
