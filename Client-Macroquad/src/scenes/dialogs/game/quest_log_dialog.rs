/// 任务日志对话框 - 显示任务列表和详情
/// 对应原工程 QuestDialogs.cs 中的 QuestLogDialog
/// 
/// 功能：
/// - 显示可接受的任务列表
/// - 显示已接受的任务进度
/// - 任务详情查看
/// - 任务完成状态跟踪

use egui_macroquad::egui;
use crate::resources::LibraryName;
use crate::scenes::dialogs::Dialog;

/// 任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestStatus {
    Available,   // 可接受
    Accepted,    // 已接受
    Completed,   // 已完成
    Failed,      // 已失败
}

/// 任务数据
#[derive(Debug, Clone)]
pub struct QuestInfo {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub npc_name: String,      // 任务发布者
    pub status: QuestStatus,
    pub progress: u32,         // 当前进度
    pub max_progress: u32,     // 最大进度
    pub level_required: u32,   // 需求等级
    pub rewards: QuestRewards, // 任务奖励
}

/// 任务奖励
#[derive(Debug, Clone)]
pub struct QuestRewards {
    pub experience: u64,
    pub gold: u32,
    pub items: Vec<QuestItem>,
}

/// 任务奖励物品
#[derive(Debug, Clone)]
pub struct QuestItem {
    pub icon_index: usize,
    pub name: String,
    pub count: u32,
}

/// 任务对话框
pub struct QuestLogDialog {
    visible: bool,
    position: egui::Pos2,
    
    /// 任务列表
    quests: Vec<QuestInfo>,
    
    /// 当前选中的任务索引
    selected_quest: Option<usize>,
    
    /// 窗口拖拽状态
    dragging: bool,
    drag_offset: egui::Vec2,
    
    /// 滚动状态
    scroll_offset: f32,
}

impl QuestLogDialog {
    pub fn new() -> Self {
        // 创建一些示例任务
        let quests = vec![
            QuestInfo {
                id: 1,
                name: "消灭稻草人".to_string(),
                description: "新手村外的稻草人威胁着村民的安全，请帮忙消灭10只稻草人。".to_string(),
                npc_name: "村长".to_string(),
                status: QuestStatus::Accepted,
                progress: 3,
                max_progress: 10,
                level_required: 1,
                rewards: QuestRewards {
                    experience: 500,
                    gold: 100,
                    items: vec![
                        QuestItem {
                            icon_index: 0,
                            name: "小血瓶".to_string(),
                            count: 5,
                        }
                    ],
                },
            },
            QuestInfo {
                id: 2,
                name: "收集鸡毛".to_string(),
                description: "铁匠需要20根鸡毛来制作羽毛箭，请到鸡舍收集。".to_string(),
                npc_name: "铁匠".to_string(),
                status: QuestStatus::Accepted,
                progress: 15,
                max_progress: 20,
                level_required: 5,
                rewards: QuestRewards {
                    experience: 800,
                    gold: 200,
                    items: vec![
                        QuestItem {
                            icon_index: 10,
                            name: "铁剑".to_string(),
                            count: 1,
                        }
                    ],
                },
            },
            QuestInfo {
                id: 3,
                name: "探索古墓".to_string(),
                description: "传说中的古墓出现了异常，需要勇敢的冒险者前去调查。".to_string(),
                npc_name: "法师".to_string(),
                status: QuestStatus::Available,
                progress: 0,
                max_progress: 1,
                level_required: 15,
                rewards: QuestRewards {
                    experience: 2000,
                    gold: 1000,
                    items: vec![
                        QuestItem {
                            icon_index: 30,
                            name: "魔法戒指".to_string(),
                            count: 1,
                        }
                    ],
                },
            },
            QuestInfo {
                id: 4,
                name: "击败骷髅战士".to_string(),
                description: "地牢深处的骷髅战士复活了，需要强大的战士将其重新消灭。".to_string(),
                npc_name: "队长".to_string(),
                status: QuestStatus::Completed,
                progress: 1,
                max_progress: 1,
                level_required: 20,
                rewards: QuestRewards {
                    experience: 5000,
                    gold: 2000,
                    items: vec![
                        QuestItem {
                            icon_index: 50,
                            name: "战士头盔".to_string(),
                            count: 1,
                        }
                    ],
                },
            },
        ];
        
        Self {
            visible: false,
            position: egui::pos2(200.0, 150.0),
            quests,
            selected_quest: None,
            dragging: false,
            drag_offset: egui::vec2(0.0, 0.0),
            scroll_offset: 0.0,
        }
    }
    
    /// 显示/隐藏对话框
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        println!("📜 任务对话框: {}", if self.visible { "显示" } else { "隐藏" });
    }
    
    /// 获取可见状态
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    
    /// 绘制对话框背景
    fn draw_background(&self, ui: &mut egui::Ui, ctx: &egui::Context) -> egui::Rect {
        // 任务对话框背景纹理
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 1047) {
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
        
        // 降级：绘制默认背景
        let default_size = egui::vec2(400.0, 500.0);
        let default_rect = egui::Rect::from_min_size(self.position, default_size);
        ui.painter().rect_filled(
            default_rect,
            5.0,
            egui::Color32::from_rgb(35, 35, 40),
        );
        ui.painter().rect_stroke(
            default_rect,
            5.0,
            egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 100, 100)),
            egui::epaint::StrokeKind::Outside,
        );
        
        // 绘制标题
        ui.painter().text(
            egui::pos2(default_rect.center().x, default_rect.min.y + 20.0),
            egui::Align2::CENTER_CENTER,
            "任务日志",
            egui::FontId::proportional(16.0),
            egui::Color32::YELLOW,
        );
        
        default_rect
    }
    
    /// 绘制任务列表
    fn draw_quest_list(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        let list_area = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 10.0, bg_rect.min.y + 40.0),
            egui::vec2(180.0, bg_rect.height() - 80.0)
        );
        
        // 绘制列表背景
        ui.painter().rect_filled(
            list_area,
            3.0,
            egui::Color32::from_rgba_premultiplied(20, 20, 25, 200),
        );
        ui.painter().rect_stroke(
            list_area,
            3.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)),
            egui::epaint::StrokeKind::Outside,
        );
        
        // 绘制任务项
        let item_height = 60.0;
        let mut y_offset = list_area.min.y + 5.0 - self.scroll_offset;
        
        // 绘制任务项 - 内联以避免借用问题
        for (i, quest) in self.quests.iter().enumerate() {
            if y_offset + item_height > list_area.min.y && y_offset < list_area.max.y {
                // 内联绘制任务项
                let item_rect = egui::Rect::from_min_size(
                    egui::pos2(list_area.min.x + 5.0, y_offset), 
                    egui::vec2(170.0, item_height)
                );
                
                // 任务背景
                let bg_color = if Some(i) == self.selected_quest {
                    egui::Color32::from_rgb(80, 80, 120)
                } else {
                    egui::Color32::from_rgb(40, 40, 50)
                };
                
                ui.painter().rect_filled(item_rect, 3.0, bg_color);
                ui.painter().rect_stroke(
                    item_rect,
                    3.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 70, 70)),
                    egui::epaint::StrokeKind::Outside,
                );
                
                // 任务标题
                ui.painter().text(
                    egui::pos2(item_rect.min.x + 10.0, item_rect.min.y + 10.0),
                    egui::Align2::LEFT_TOP,
                    &quest.name,
                    egui::FontId::proportional(14.0),
                    egui::Color32::YELLOW,
                );
                
                // 检查点击
                let response = ui.interact(item_rect, egui::Id::new(format!("quest_{}", i)), egui::Sense::click());
                if response.clicked() {
                    // 注意：这里不能直接修改self.selected_quest，需要在循环外处理
                    println!("选择任务: {}", quest.name);
                }
            }
            y_offset += item_height + 5.0;
        }
        
        // 处理滚动
        let list_response = ui.interact(list_area, egui::Id::new("quest_list"), egui::Sense::click_and_drag());
        if list_response.hovered() {
            let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
            self.scroll_offset = (self.scroll_offset - scroll_delta * 20.0).max(0.0);
        }
    }
    
    /// 绘制单个任务项
    fn draw_quest_item(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context, quest: &QuestInfo, index: usize, pos: egui::Pos2, size: egui::Vec2) {
        let item_rect = egui::Rect::from_min_size(pos, size);
        
        // 检查是否选中
        let is_selected = self.selected_quest == Some(index);
        
        // 绘制项目背景
        let bg_color = if is_selected {
            egui::Color32::from_rgb(60, 100, 140)
        } else {
            egui::Color32::from_rgba_premultiplied(40, 40, 45, 150)
        };
        
        ui.painter().rect_filled(item_rect, 3.0, bg_color);
        ui.painter().rect_stroke(
            item_rect,
            3.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 70, 70)),
            egui::epaint::StrokeKind::Outside,
        );
        
        // 任务状态颜色指示
        let status_color = match quest.status {
            QuestStatus::Available => egui::Color32::GREEN,
            QuestStatus::Accepted => egui::Color32::YELLOW,
            QuestStatus::Completed => egui::Color32::BLUE,
            QuestStatus::Failed => egui::Color32::RED,
        };
        
        // 绘制状态指示点
        ui.painter().circle_filled(
            egui::pos2(pos.x + 10.0, pos.y + 10.0),
            4.0,
            status_color,
        );
        
        // 绘制任务名称
        ui.painter().text(
            egui::pos2(pos.x + 25.0, pos.y + 10.0),
            egui::Align2::LEFT_CENTER,
            &quest.name,
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
        
        // 绘制发布者
        ui.painter().text(
            egui::pos2(pos.x + 25.0, pos.y + 25.0),
            egui::Align2::LEFT_CENTER,
            format!("发布者: {}", quest.npc_name),
            egui::FontId::proportional(10.0),
            egui::Color32::GRAY,
        );
        
        // 绘制进度条（如果任务已接受）
        if quest.status == QuestStatus::Accepted && quest.max_progress > 0 {
            let progress_rect = egui::Rect::from_min_size(
                egui::pos2(pos.x + 25.0, pos.y + 40.0),
                egui::vec2(120.0, 8.0)
            );
            
            // 进度条背景
            ui.painter().rect_filled(
                progress_rect,
                2.0,
                egui::Color32::from_rgb(30, 30, 30),
            );
            
            // 进度条填充
            let progress_percent = quest.progress as f32 / quest.max_progress as f32;
            let filled_rect = egui::Rect::from_min_size(
                progress_rect.min,
                egui::vec2(progress_rect.width() * progress_percent, progress_rect.height())
            );
            
            ui.painter().rect_filled(
                filled_rect,
                2.0,
                egui::Color32::from_rgb(100, 150, 100),
            );
            
            // 进度文字
            ui.painter().text(
                egui::pos2(pos.x + 25.0, pos.y + 52.0),
                egui::Align2::LEFT_CENTER,
                format!("{}/{}", quest.progress, quest.max_progress),
                egui::FontId::proportional(9.0),
                egui::Color32::WHITE,
            );
        }
        
        // 处理点击
        let response = ui.interact(item_rect, egui::Id::new(format!("quest_item_{}", quest.id)), egui::Sense::click());
        if response.clicked() {
            self.selected_quest = Some(index);
            println!("📋 选中任务: {}", quest.name);
        }
    }
    
    /// 绘制任务详情
    fn draw_quest_details(&self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        let detail_area = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 200.0, bg_rect.min.y + 40.0),
            egui::vec2(bg_rect.width() - 220.0, bg_rect.height() - 80.0)
        );
        
        // 绘制详情背景
        ui.painter().rect_filled(
            detail_area,
            3.0,
            egui::Color32::from_rgba_premultiplied(25, 25, 30, 200),
        );
        ui.painter().rect_stroke(
            detail_area,
            3.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)),
            egui::epaint::StrokeKind::Outside,
        );
        
        if let Some(quest_index) = self.selected_quest {
            if let Some(quest) = self.quests.get(quest_index) {
                let mut y_pos = detail_area.min.y + 15.0;
                
                // 任务名称
                ui.painter().text(
                    egui::pos2(detail_area.min.x + 15.0, y_pos),
                    egui::Align2::LEFT_CENTER,
                    &quest.name,
                    egui::FontId::proportional(16.0),
                    egui::Color32::YELLOW,
                );
                y_pos += 30.0;
                
                // 发布者和等级需求
                ui.painter().text(
                    egui::pos2(detail_area.min.x + 15.0, y_pos),
                    egui::Align2::LEFT_CENTER,
                    format!("发布者: {} | 需求等级: {}", quest.npc_name, quest.level_required),
                    egui::FontId::proportional(12.0),
                    egui::Color32::GRAY,
                );
                y_pos += 25.0;
                
                // 任务状态
                let status_text = match quest.status {
                    QuestStatus::Available => "可接受",
                    QuestStatus::Accepted => "进行中",
                    QuestStatus::Completed => "已完成",
                    QuestStatus::Failed => "已失败",
                };
                
                ui.painter().text(
                    egui::pos2(detail_area.min.x + 15.0, y_pos),
                    egui::Align2::LEFT_CENTER,
                    format!("状态: {}", status_text),
                    egui::FontId::proportional(12.0),
                    egui::Color32::WHITE,
                );
                y_pos += 25.0;
                
                // 任务描述
                ui.painter().text(
                    egui::pos2(detail_area.min.x + 15.0, y_pos),
                    egui::Align2::LEFT_CENTER,
                    "任务描述:",
                    egui::FontId::proportional(12.0),
                    egui::Color32::YELLOW,
                );
                y_pos += 20.0;
                
                // 描述文本（可能需要换行）
                ui.painter().text(
                    egui::pos2(detail_area.min.x + 15.0, y_pos),
                    egui::Align2::LEFT_TOP,
                    &quest.description,
                    egui::FontId::proportional(11.0),
                    egui::Color32::WHITE,
                );
                y_pos += 60.0;
                
                // 任务奖励
                ui.painter().text(
                    egui::pos2(detail_area.min.x + 15.0, y_pos),
                    egui::Align2::LEFT_CENTER,
                    "任务奖励:",
                    egui::FontId::proportional(12.0),
                    egui::Color32::YELLOW,
                );
                y_pos += 20.0;
                
                // 经验和金币奖励
                ui.painter().text(
                    egui::pos2(detail_area.min.x + 15.0, y_pos),
                    egui::Align2::LEFT_CENTER,
                    format!("经验: {} | 金币: {}", quest.rewards.experience, quest.rewards.gold),
                    egui::FontId::proportional(11.0),
                    egui::Color32::WHITE,
                );
                y_pos += 20.0;
                
                // 物品奖励
                if !quest.rewards.items.is_empty() {
                    ui.painter().text(
                        egui::pos2(detail_area.min.x + 15.0, y_pos),
                        egui::Align2::LEFT_CENTER,
                        "物品奖励:",
                        egui::FontId::proportional(11.0),
                        egui::Color32::WHITE,
                    );
                    y_pos += 20.0;
                    
                    for item in &quest.rewards.items {
                        self.draw_reward_item(ui, ctx, item, egui::pos2(detail_area.min.x + 25.0, y_pos));
                        y_pos += 35.0;
                    }
                }
            }
        } else {
            // 没有选中任务时显示提示
            ui.painter().text(
                detail_area.center(),
                egui::Align2::CENTER_CENTER,
                "选择一个任务查看详情",
                egui::FontId::proportional(14.0),
                egui::Color32::GRAY,
            );
        }
    }
    
    /// 绘制奖励物品
    fn draw_reward_item(&self, ui: &mut egui::Ui, ctx: &egui::Context, item: &QuestItem, pos: egui::Pos2) {
        let item_size = egui::vec2(24.0, 24.0);
        let item_rect = egui::Rect::from_min_size(pos, item_size);
        
        // 绘制物品图标背景
        ui.painter().rect_filled(
            item_rect,
            2.0,
            egui::Color32::from_rgba_premultiplied(40, 40, 40, 200),
        );
        ui.painter().rect_stroke(
            item_rect,
            2.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 100)),
            egui::epaint::StrokeKind::Outside,
        );
        
        // 绘制物品图标
        if let Some(info) = LibraryName::Items.get_egui_texture(ctx, item.icon_index) {
            if let Some(item_texture) = info.egui_texture {
                ui.painter().image(
                    item_texture.id(),
                    item_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        }
        
        // 绘制物品名称和数量
        ui.painter().text(
            egui::pos2(pos.x + 30.0, pos.y + 12.0),
            egui::Align2::LEFT_CENTER,
            format!("{} x{}", item.name, item.count),
            egui::FontId::proportional(10.0),
            egui::Color32::WHITE,
        );
    }
    
    /// 绘制关闭按钮
    fn draw_close_button(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) -> bool {
        let close_pos = egui::pos2(bg_rect.max.x - 25.0, bg_rect.min.y + 5.0);
        
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 360) {
            if let Some(close_texture) = info.egui_texture {
                let close_size = close_texture.size_vec2();
                let close_rect = egui::Rect::from_min_size(close_pos, close_size);
                
                let response = ui.interact(close_rect, egui::Id::new("quest_close"), egui::Sense::click());
                
                let texture_id = if response.is_pointer_button_down_on() {
                    LibraryName::Prguse.get_egui_texture(ctx, 362)
                        .and_then(|info| info.egui_texture)
                        .map(|t| t.id())
                        .unwrap_or(close_texture.id())
                } else if response.hovered() {
                    LibraryName::Prguse.get_egui_texture(ctx, 361)
                        .and_then(|info| info.egui_texture)
                        .map(|t| t.id())
                        .unwrap_or(close_texture.id())
                } else {
                    close_texture.id()
                };
                
                ui.painter().image(
                    texture_id,
                    close_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                
                let is_clicked = response.clicked();
                if response.hovered() {
                    response.on_hover_text("关闭");
                }
                
                return is_clicked;
            }
        }
        
        false
    }
    
    /// 处理窗口拖拽
    fn handle_window_dragging(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        let title_area = egui::Rect::from_min_size(
            bg_rect.min,
            egui::vec2(bg_rect.width(), 35.0),
        );
        
        let drag_response = ui.interact(title_area, egui::Id::new("quest_drag"), egui::Sense::drag());
        
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

impl Dialog for QuestLogDialog {
    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        if !self.visible {
            *open = false;
            return;
        }
        
        egui::Area::new(egui::Id::new("quest_log_dialog"))
            .fixed_pos(self.position)
            .movable(false)
            .show(ctx, |ui| {
                // 绘制背景
                let bg_rect = self.draw_background(ui, ctx);
                
                // 处理窗口拖拽
                self.handle_window_dragging(ui, ctx, &bg_rect);
                
                // 绘制任务列表
                self.draw_quest_list(ui, ctx, &bg_rect);
                
                // 绘制任务详情
                self.draw_quest_details(ui, ctx, &bg_rect);
                
                // 绘制关闭按钮
                if self.draw_close_button(ui, ctx, &bg_rect) {
                    self.visible = false;
                    *open = false;
                }
            });
        
        *open = self.visible;
    }
}