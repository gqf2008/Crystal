/// 任务详情对话框 - 显示已接受任务的详细信息
/// 对应原工程 QuestDialogs.cs 中的 QuestDetailDialog
/// 
/// 功能：
/// - 显示任务详细描述
/// - 显示任务进度
/// - 显示任务奖励
/// - 分享任务
/// - 放弃任务

use egui_macroquad::egui;
use crate::resources::LibraryName;
use crate::scenes::dialogs::Dialog;
use super::quest_log_dialog::{QuestInfo, QuestItem};

/// 任务详情对话框
pub struct QuestDetailDialog {
    visible: bool,
    position: egui::Pos2,
    
    /// 当前显示的任务
    quest: Option<QuestInfo>,
    
    /// 窗口拖拽状态
    dragging: bool,
    drag_offset: egui::Vec2,
    
    /// 消息区域滚动偏移
    message_scroll: f32,
}

impl QuestDetailDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: egui::pos2(450.0, 50.0),
            quest: None,
            dragging: false,
            drag_offset: egui::vec2(0.0, 0.0),
            message_scroll: 0.0,
        }
    }
    
    /// 显示任务详情
    pub fn display_quest(&mut self, quest: QuestInfo) {
        self.quest = Some(quest);
        self.visible = true;
        self.message_scroll = 0.0;
        println!("📖 显示任务详情");
    }
    
    /// 获取可见状态
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    
    /// 绘制对话框背景
    fn draw_background(&self, ui: &mut egui::Ui, ctx: &egui::Context) -> egui::Rect {
        // 任务详情对话框背景 (Index: 960)
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 960) {
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
        let default_size = egui::vec2(320.0, 470.0);
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
        
        default_rect
    }
    
    /// 绘制消息区域
    fn draw_message_area(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context, bg_rect: &egui::Rect) {
        let message_area = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 10.0, bg_rect.min.y + 35.0),
            egui::vec2(280.0, 260.0)
        );
        
        if let Some(quest) = &self.quest.clone() {
            // 使用egui的ScrollArea和Label来实现文字自动换行
            let text_area = message_area.shrink(8.0);
            
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(text_area), |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("quest_detail_message_scroll")
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .show(ui, |ui| {
                        ui.set_width(text_area.width() - 10.0);
                        
                        // 任务名称
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&quest.name)
                                    .size(14.0)
                                    .color(egui::Color32::YELLOW)
                            )
                            .wrap()
                        );
                        ui.add_space(10.0);
                        
                        // 任务描述标题
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new("任务描述:")
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(200, 200, 100))
                            )
                        );
                        ui.add_space(5.0);
                        
                        // 任务描述内容
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&quest.description)
                                    .size(11.0)
                                    .color(egui::Color32::WHITE)
                            )
                            .wrap()
                        );
                        ui.add_space(15.0);
                        
                        // 任务进度
                        if quest.max_progress > 0 {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new("进度:")
                                        .size(12.0)
                                        .color(egui::Color32::from_rgb(200, 200, 100))
                                )
                            );
                            ui.add_space(5.0);
                            
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(format!("{} / {}", quest.progress, quest.max_progress))
                                        .size(11.0)
                                        .color(egui::Color32::WHITE)
                                )
                            );
                            ui.add_space(10.0);
                            
                            // 进度条
                            let progress_percent = quest.progress as f32 / quest.max_progress as f32;
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(250.0, 10.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 3.0, egui::Color32::from_rgb(40, 40, 50));
                            let filled_width = rect.width() * progress_percent;
                            let filled_rect = egui::Rect::from_min_size(rect.min, egui::vec2(filled_width, rect.height()));
                            ui.painter().rect_filled(filled_rect, 3.0, egui::Color32::from_rgb(100, 200, 100));
                            ui.painter().rect_stroke(rect, 3.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)), egui::epaint::StrokeKind::Outside);
                        }
                    });
            });
        }
    }
    
    /// 绘制奖励区域
    fn draw_rewards(&self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        let reward_area = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 5.0, bg_rect.min.y + 307.0),
            egui::vec2(315.0, 120.0)
        );
        
        if let Some(quest) = &self.quest {
            let mut y_offset = reward_area.min.y + 5.0;
            
            // 绘制经验奖励图标
            if quest.rewards.experience > 0 {
                if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 966) {
                    if let Some(exp_texture) = info.egui_texture {
                        let exp_rect = egui::Rect::from_min_size(
                            egui::pos2(reward_area.min.x + 10.0, y_offset),
                            exp_texture.size_vec2()
                        );
                        ui.painter().image(
                            exp_texture.id(),
                            exp_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                        
                        ui.painter().text(
                            egui::pos2(reward_area.min.x + 40.0, y_offset + 10.0),
                            egui::Align2::LEFT_CENTER,
                            format!("{}", quest.rewards.experience),
                            egui::FontId::proportional(11.0),
                            egui::Color32::YELLOW,
                        );
                    }
                }
            }
            
            // 绘制金币奖励图标
            if quest.rewards.gold > 0 {
                if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 965) {
                    if let Some(gold_texture) = info.egui_texture {
                        let gold_rect = egui::Rect::from_min_size(
                            egui::pos2(reward_area.min.x + 100.0, y_offset),
                            gold_texture.size_vec2()
                        );
                        ui.painter().image(
                            gold_texture.id(),
                            gold_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                        
                        ui.painter().text(
                            egui::pos2(reward_area.min.x + 130.0, y_offset + 10.0),
                            egui::Align2::LEFT_CENTER,
                            format!("{}", quest.rewards.gold),
                            egui::FontId::proportional(11.0),
                            egui::Color32::YELLOW,
                        );
                    }
                }
            }
            
            y_offset += 30.0;
            
            // 绘制物品奖励
            if !quest.rewards.items.is_empty() {
                ui.painter().text(
                    egui::pos2(reward_area.min.x + 15.0, y_offset),
                    egui::Align2::LEFT_CENTER,
                    "物品奖励:",
                    egui::FontId::proportional(11.0),
                    egui::Color32::YELLOW,
                );
                y_offset += 20.0;
                
                let item_size = egui::vec2(36.0, 36.0);
                let mut x_offset = reward_area.min.x + 15.0;
                
                for item in quest.rewards.items.iter().take(5) {
                    self.draw_reward_item(ui, ctx, item, egui::pos2(x_offset, y_offset), item_size);
                    x_offset += 45.0;
                }
            }
        }
    }
    
    /// 绘制奖励物品
    fn draw_reward_item(&self, ui: &mut egui::Ui, ctx: &egui::Context, item: &QuestItem, pos: egui::Pos2, size: egui::Vec2) {
        let item_rect = egui::Rect::from_min_size(pos, size);
        
        // 绘制物品框背景
        ui.painter().rect_filled(item_rect, 2.0, egui::Color32::from_rgb(40, 40, 40));
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
        
        // 绘制数量
        if item.count > 1 {
            ui.painter().text(
                egui::pos2(item_rect.max.x - 5.0, item_rect.max.y - 5.0),
                egui::Align2::RIGHT_BOTTOM,
                format!("{}", item.count),
                egui::FontId::proportional(9.0),
                egui::Color32::WHITE,
            );
        }
        
        // 处理悬停
        let response = ui.interact(item_rect, egui::Id::new(format!("detail_reward_{}", item.name)), egui::Sense::hover());
        if response.hovered() {
            response.on_hover_text(&item.name);
        }
    }
    
    /// 绘制底部按钮
    fn draw_buttons(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        let button_y = bg_rect.min.y + 436.0;
        
        // 分享按钮
        self.draw_share_button(ui, ctx, bg_rect, button_y);
        
        // 放弃按钮
        self.draw_abandon_button(ui, ctx, bg_rect, button_y);
    }
    
    /// 绘制分享按钮
    fn draw_share_button(&self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect, y: f32) -> bool {
        let button_pos = egui::pos2(bg_rect.min.x + 40.0, y);
        
        if let Some(info) = LibraryName::Title.get_egui_texture(ctx, 616) {
            if let Some(btn_texture) = info.egui_texture {
                let btn_rect = egui::Rect::from_min_size(button_pos, btn_texture.size_vec2());
                let response = ui.interact(btn_rect, egui::Id::new("quest_share"), egui::Sense::click());
                
                let texture_id = if response.is_pointer_button_down_on() {
                    LibraryName::Title.get_egui_texture(ctx, 618)
                        .and_then(|info| info.egui_texture)
                        .map(|t| t.id())
                        .unwrap_or(btn_texture.id())
                } else if response.hovered() {
                    LibraryName::Title.get_egui_texture(ctx, 617)
                        .and_then(|info| info.egui_texture)
                        .map(|t| t.id())
                        .unwrap_or(btn_texture.id())
                } else {
                    btn_texture.id()
                };
                
                ui.painter().image(
                    texture_id,
                    btn_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                
                if response.clicked() {
                    println!("🔗 分享任务");
                    // TODO: 发送分享任务消息
                    return true;
                }
                
                if response.hovered() {
                    response.on_hover_text("分享任务");
                }
            }
        }
        false
    }
    
    /// 绘制放弃按钮
    fn draw_abandon_button(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect, y: f32) -> bool {
        let button_pos = egui::pos2(bg_rect.min.x + 200.0, y);
        
        if let Some(info) = LibraryName::Title.get_egui_texture(ctx, 203) {
            if let Some(btn_texture) = info.egui_texture {
                let btn_rect = egui::Rect::from_min_size(button_pos, btn_texture.size_vec2());
                let response = ui.interact(btn_rect, egui::Id::new("quest_abandon"), egui::Sense::click());
                
                let texture_id = if response.is_pointer_button_down_on() {
                    LibraryName::Title.get_egui_texture(ctx, 205)
                        .and_then(|info| info.egui_texture)
                        .map(|t| t.id())
                        .unwrap_or(btn_texture.id())
                } else if response.hovered() {
                    LibraryName::Title.get_egui_texture(ctx, 204)
                        .and_then(|info| info.egui_texture)
                        .map(|t| t.id())
                        .unwrap_or(btn_texture.id())
                } else {
                    btn_texture.id()
                };
                
                ui.painter().image(
                    texture_id,
                    btn_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                
                if response.clicked() {
                    println!("⚠️ 放弃任务");
                    // TODO: 显示确认对话框，然后发送放弃任务消息
                    self.visible = false;
                    return true;
                }
                
                if response.hovered() {
                    response.on_hover_text("放弃任务");
                }
            }
        }
        false
    }
    
    /// 绘制关闭按钮
    fn draw_close_button(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) -> bool {
        let close_pos = egui::pos2(bg_rect.min.x + 289.0, bg_rect.min.y + 3.0);
        
        if let Some(info) = LibraryName::Prguse2.get_egui_texture(ctx, 360) {
            if let Some(close_texture) = info.egui_texture {
                let close_rect = egui::Rect::from_min_size(close_pos, close_texture.size_vec2());
                let response = ui.interact(close_rect, egui::Id::new("quest_detail_close"), egui::Sense::click());
                
                let texture_id = if response.is_pointer_button_down_on() {
                    LibraryName::Prguse2.get_egui_texture(ctx, 362)
                        .and_then(|info| info.egui_texture)
                        .map(|t| t.id())
                        .unwrap_or(close_texture.id())
                } else if response.hovered() {
                    LibraryName::Prguse2.get_egui_texture(ctx, 361)
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
                
                if response.clicked() {
                    self.visible = false;
                    return true;
                }
                
                if response.hovered() {
                    response.on_hover_text("关闭");
                }
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
        
        let drag_response = ui.interact(title_area, egui::Id::new("quest_detail_drag"), egui::Sense::drag());
        
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

impl Dialog for QuestDetailDialog {
    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        if !*open {
            self.visible = false;
            return;
        }
        self.visible = true;
        
        egui::Area::new(egui::Id::new("quest_detail_dialog"))
            .fixed_pos(self.position)
            .movable(false)
            .show(ctx, |ui| {
                // 绘制背景
                let bg_rect = self.draw_background(ui, ctx);
                
                // 处理窗口拖拽
                self.handle_window_dragging(ui, ctx, &bg_rect);
                
                // 绘制消息区域
                self.draw_message_area(ui, ctx, &bg_rect);
                
                // 绘制奖励区域
                self.draw_rewards(ui, ctx, &bg_rect);
                
                // 绘制按钮
                self.draw_buttons(ui, ctx, &bg_rect);
                
                // 绘制关闭按钮
                if self.draw_close_button(ui, ctx, &bg_rect) {
                    self.visible = false;
                    *open = false;
                }
            });
        
        *open = self.visible;
    }
}
