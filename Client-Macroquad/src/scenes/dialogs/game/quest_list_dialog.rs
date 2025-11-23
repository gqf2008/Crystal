/// 任务列表对话框 - 与NPC对话时显示的任务接受/完成界面
/// 对应原工程 QuestDialogs.cs 中的 QuestListDialog
/// 
/// 功能：
/// - 显示NPC提供的任务列表
/// - 任务详细描述和奖励展示
/// - 接受新任务
/// - 完成已接受的任务
/// - 选择任务奖励物品

use egui_macroquad::egui;
use crate::resources::LibraryName;
use crate::scenes::dialogs::Dialog;
use super::quest_log_dialog::{QuestInfo, QuestStatus, QuestItem};

/// 任务行数据
#[derive(Debug, Clone)]
pub struct QuestRow {
    pub quest: QuestInfo,
    pub selected: bool,
}

/// 任务列表对话框
pub struct QuestListDialog {
    visible: bool,
    position: egui::Pos2,
    
    /// 当前NPC提供的任务列表
    quests: Vec<QuestInfo>,
    
    /// 当前选中的任务索引
    selected_index: Option<usize>,
    
    /// 列表起始索引(用于分页)
    start_index: usize,
    
    /// 每页显示的任务数量
    rows_per_page: usize,
    
    /// 选中的奖励物品索引
    selected_reward_index: Option<usize>,
    
    /// 窗口拖拽状态
    dragging: bool,
    drag_offset: egui::Vec2,
    
    /// 消息区域滚动偏移
    message_scroll: f32,
}

impl QuestListDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: egui::pos2(400.0, 100.0),
            quests: Vec::new(),
            selected_index: None,
            start_index: 0,
            rows_per_page: 5,
            selected_reward_index: None,
            dragging: false,
            drag_offset: egui::vec2(0.0, 0.0),
            message_scroll: 0.0,
        }
    }
    
    /// 设置NPC任务列表
    pub fn set_quests(&mut self, quests: Vec<QuestInfo>) {
        self.quests = quests;
        self.selected_index = None;
        self.start_index = 0;
        self.selected_reward_index = None;
        self.message_scroll = 0.0;
    }
    
    /// 显示对话框
    pub fn show_with_quests(&mut self, quests: Vec<QuestInfo>) {
        self.set_quests(quests);
        self.visible = true;
        println!("📋 任务列表对话框: 显示 {} 个任务", self.quests.len());
    }
    
    /// 获取可见状态
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    
    /// 绘制对话框背景
    fn draw_background(&self, ui: &mut egui::Ui, ctx: &egui::Context) -> egui::Rect {
        // 任务列表对话框背景 (Index: 950)
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 950) {
            if let Some(bg_texture) = info.egui_texture {
                let bg_size = bg_texture.size_vec2();
                let bg_rect = egui::Rect::from_min_size(self.position, bg_size);
                
                println!("✅ 绘制任务列表背景: 位置={:?}, 大小={:?}", self.position, bg_size);
                
                ui.painter().image(
                    bg_texture.id(),
                    bg_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                
                return bg_rect;
            } else {
                println!("❌ 纹理加载失败: egui_texture 为 None");
            }
        } else {
            println!("❌ 获取纹理信息失败: Prguse index 950 返回 None");
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
    
    /// 绘制任务选择区域
    fn draw_quest_selection(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        let list_area = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 9.0, bg_rect.min.y + 36.0),
            egui::vec2(280.0, 95.0)
        );
        
        // 绘制任务行
        let row_height = 19.0;
        let start_index = self.start_index;
        let rows_per_page = self.rows_per_page;
        
        // 收集需要绘制的任务信息（避免借用冲突）
        let visible_quests: Vec<(usize, QuestInfo)> = self.quests
            .iter()
            .skip(start_index)
            .take(rows_per_page)
            .enumerate()
            .map(|(i, q)| (start_index + i, q.clone()))
            .collect();
        
        for (actual_index, quest) in visible_quests {
            let i = actual_index - start_index;
            let y_pos = list_area.min.y + i as f32 * row_height;
            
            self.draw_quest_row(ui, ctx, &quest, actual_index, egui::pos2(list_area.min.x, y_pos));
        }
        
        // 绘制上下滚动按钮
        self.draw_scroll_buttons(ui, ctx, bg_rect);
    }
    
    /// 绘制单个任务行
    fn draw_quest_row(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, quest: &QuestInfo, index: usize, pos: egui::Pos2) {
        let row_rect = egui::Rect::from_min_size(pos, egui::vec2(200.0, 17.0));
        
        let is_selected = self.selected_index == Some(index);
        
        // 绘制选中背景
        if is_selected {
            if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 956) {
                if let Some(sel_texture) = info.egui_texture {
                    let sel_rect = egui::Rect::from_min_size(
                        egui::pos2(pos.x + 25.0, pos.y),
                        sel_texture.size_vec2()
                    );
                    ui.painter().image(
                        sel_texture.id(),
                        sel_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
            }
        }
        
        // 绘制任务图标
        let icon_index = match quest.status {
            QuestStatus::Available => 961,  // 可接受任务图标
            QuestStatus::Accepted => 962,   // 进行中任务图标
            QuestStatus::Completed => 963,  // 已完成任务图标
            QuestStatus::Failed => 964,     // 失败任务图标
        };
        
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, icon_index) {
            if let Some(icon_texture) = info.egui_texture {
                let icon_rect = egui::Rect::from_min_size(
                    egui::pos2(pos.x + 3.0, pos.y),
                    icon_texture.size_vec2()
                );
                ui.painter().image(
                    icon_texture.id(),
                    icon_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        }
        
        // 绘制任务名称
        ui.painter().text(
            egui::pos2(pos.x + 60.0, pos.y + 8.0),
            egui::Align2::LEFT_CENTER,
            &quest.name,
            egui::FontId::proportional(11.0),
            egui::Color32::WHITE,
        );
        
        // 绘制等级要求
        if quest.level_required > 0 {
            ui.painter().text(
                egui::pos2(pos.x + 20.0, pos.y + 8.0),
                egui::Align2::LEFT_CENTER,
                format!("Lv {}", quest.level_required),
                egui::FontId::proportional(9.0),
                egui::Color32::GRAY,
            );
        }
        
        // 处理点击
        let response = ui.interact(row_rect, egui::Id::new(format!("quest_row_{}", quest.id)), egui::Sense::click());
        if response.clicked() {
            self.selected_index = Some(index);
            self.selected_reward_index = None;
            self.message_scroll = 0.0;
            println!("📌 选中任务: {}", quest.name);
        }
    }
    
    /// 绘制滚动按钮
    fn draw_scroll_buttons(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 上滚按钮
        let up_button_pos = egui::pos2(bg_rect.min.x + 291.0, bg_rect.min.y + 35.0);
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 951) {
            if let Some(btn_texture) = info.egui_texture {
                let btn_rect = egui::Rect::from_min_size(up_button_pos, btn_texture.size_vec2());
                let response = ui.interact(btn_rect, egui::Id::new("quest_scroll_up"), egui::Sense::click());
                
                let texture_id = if response.is_pointer_button_down_on() {
                    LibraryName::Prguse.get_egui_texture(ctx, 953)
                        .and_then(|info| info.egui_texture)
                        .map(|t| t.id())
                        .unwrap_or(btn_texture.id())
                } else if response.hovered() {
                    LibraryName::Prguse.get_egui_texture(ctx, 952)
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
                
                if response.clicked() && self.start_index > 0 {
                    self.start_index -= 1;
                }
            }
        }
        
        // 下滚按钮
        let down_button_pos = egui::pos2(bg_rect.min.x + 291.0, bg_rect.min.y + 83.0);
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 957) {
            if let Some(btn_texture) = info.egui_texture {
                let btn_rect = egui::Rect::from_min_size(down_button_pos, btn_texture.size_vec2());
                let response = ui.interact(btn_rect, egui::Id::new("quest_scroll_down"), egui::Sense::click());
                
                let texture_id = if response.is_pointer_button_down_on() {
                    LibraryName::Prguse.get_egui_texture(ctx, 959)
                        .and_then(|info| info.egui_texture)
                        .map(|t| t.id())
                        .unwrap_or(btn_texture.id())
                } else if response.hovered() {
                    LibraryName::Prguse.get_egui_texture(ctx, 958)
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
                
                let max_start = self.quests.len().saturating_sub(self.rows_per_page);
                if response.clicked() && self.start_index < max_start {
                    self.start_index += 1;
                }
            }
        }
    }
    
    /// 绘制任务消息区域
    fn draw_message_area(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context, bg_rect: &egui::Rect) {
        let message_area = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 10.0, bg_rect.min.y + 135.0),
            egui::vec2(280.0, 160.0)
        );
        
        // 绘制消息背景
        ui.painter().rect_filled(
            message_area,
            3.0,
            egui::Color32::from_rgba_premultiplied(20, 20, 25, 200),
        );
        ui.painter().rect_stroke(
            message_area,
            3.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)),
            egui::epaint::StrokeKind::Outside,
        );
        
        if let Some(index) = self.selected_index {
            if let Some(quest) = self.quests.get(index) {
                // 绘制任务描述
                let text_pos = egui::pos2(message_area.min.x + 10.0, message_area.min.y + 10.0 - self.message_scroll);
                
                ui.painter().text(
                    text_pos,
                    egui::Align2::LEFT_TOP,
                    &quest.description,
                    egui::FontId::proportional(11.0),
                    egui::Color32::WHITE,
                );
                
                // 处理滚动
                let response = ui.interact(message_area, egui::Id::new("quest_message"), egui::Sense::hover());
                if response.hovered() {
                    let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
                    self.message_scroll = (self.message_scroll - scroll_delta * 10.0).max(0.0);
                }
            }
        } else {
            // 无选中任务时显示提示
            ui.painter().text(
                message_area.center(),
                egui::Align2::CENTER_CENTER,
                "选择一个任务查看详情",
                egui::FontId::proportional(12.0),
                egui::Color32::GRAY,
            );
        }
    }
    
    /// 绘制奖励区域
    fn draw_rewards(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        let reward_area = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 5.0, bg_rect.min.y + 307.0),
            egui::vec2(313.0, 130.0)
        );
        
        if let Some(index) = self.selected_index {
            if let Some(quest) = self.quests.get(index) {
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
                            
                            // 经验值文字
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
                            
                            // 金币数量文字
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
                
                // 绘制固定物品奖励
                if !quest.rewards.items.is_empty() {
                    ui.painter().text(
                        egui::pos2(reward_area.min.x + 15.0, y_offset),
                        egui::Align2::LEFT_CENTER,
                        "固定奖励:",
                        egui::FontId::proportional(11.0),
                        egui::Color32::YELLOW,
                    );
                    y_offset += 20.0;
                    
                    let item_size = egui::vec2(36.0, 36.0);
                    let mut x_offset = reward_area.min.x + 15.0;
                    
                    // 克隆items以避免借用冲突
                    let items: Vec<_> = quest.rewards.items.iter().take(5).cloned().collect();
                    for (i, item) in items.iter().enumerate() {
                        self.draw_reward_item(
                            ui, 
                            ctx, 
                            item, 
                            egui::pos2(x_offset, y_offset),
                            item_size,
                            false,
                            i
                        );
                        x_offset += 45.0;
                    }
                }
            }
        }
    }
    
    /// 绘制奖励物品
    fn draw_reward_item(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        item: &QuestItem,
        pos: egui::Pos2,
        size: egui::Vec2,
        selectable: bool,
        index: usize,
    ) {
        let item_rect = egui::Rect::from_min_size(pos, size);
        
        let is_selected = selectable && self.selected_reward_index == Some(index);
        
        // 绘制物品框背景
        let bg_color = if is_selected {
            egui::Color32::from_rgb(100, 100, 150)
        } else {
            egui::Color32::from_rgb(40, 40, 40)
        };
        
        ui.painter().rect_filled(item_rect, 2.0, bg_color);
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
        
        // 处理点击(如果可选择)
        if selectable {
            let response = ui.interact(item_rect, egui::Id::new(format!("reward_item_{}", index)), egui::Sense::click());
            if response.clicked() {
                self.selected_reward_index = Some(index);
                println!("💎 选择奖励: {}", item.name);
            }
            if response.hovered() {
                response.on_hover_text(&item.name);
            }
        }
    }
    
    /// 绘制底部按钮
    fn draw_buttons(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        let button_y = bg_rect.min.y + 436.0;
        
        if let Some(index) = self.selected_index {
            if let Some(quest) = self.quests.get(index) {
                match quest.status {
                    QuestStatus::Available => {
                        // 接受任务按钮
                        self.draw_accept_button(ui, ctx, bg_rect, button_y);
                    },
                    QuestStatus::Completed => {
                        // 完成任务按钮
                        self.draw_finish_button(ui, ctx, bg_rect, button_y);
                    },
                    _ => {}
                }
            }
        }
        
        // 离开按钮
        self.draw_leave_button(ui, ctx, bg_rect, button_y);
    }
    
    /// 绘制接受按钮
    fn draw_accept_button(&self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect, y: f32) -> bool {
        let button_pos = egui::pos2(bg_rect.min.x + 40.0, y);
        
        if let Some(info) = LibraryName::Title.get_egui_texture(ctx, 270) {
            if let Some(btn_texture) = info.egui_texture {
                let btn_rect = egui::Rect::from_min_size(button_pos, btn_texture.size_vec2());
                let response = ui.interact(btn_rect, egui::Id::new("quest_accept"), egui::Sense::click());
                
                let texture_id = if response.is_pointer_button_down_on() {
                    LibraryName::Title.get_egui_texture(ctx, 272)
                        .and_then(|info| info.egui_texture)
                        .map(|t| t.id())
                        .unwrap_or(btn_texture.id())
                } else if response.hovered() {
                    LibraryName::Title.get_egui_texture(ctx, 271)
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
                    println!("✅ 接受任务");
                    // TODO: 发送接受任务消息到服务器
                    return true;
                }
            }
        }
        false
    }
    
    /// 绘制完成按钮
    fn draw_finish_button(&self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect, y: f32) -> bool {
        let button_pos = egui::pos2(bg_rect.min.x + 40.0, y);
        
        if let Some(info) = LibraryName::Title.get_egui_texture(ctx, 273) {
            if let Some(btn_texture) = info.egui_texture {
                let btn_rect = egui::Rect::from_min_size(button_pos, btn_texture.size_vec2());
                let response = ui.interact(btn_rect, egui::Id::new("quest_finish"), egui::Sense::click());
                
                let texture_id = if response.is_pointer_button_down_on() {
                    LibraryName::Title.get_egui_texture(ctx, 275)
                        .and_then(|info| info.egui_texture)
                        .map(|t| t.id())
                        .unwrap_or(btn_texture.id())
                } else if response.hovered() {
                    LibraryName::Title.get_egui_texture(ctx, 274)
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
                    println!("🎉 完成任务");
                    // TODO: 发送完成任务消息到服务器
                    return true;
                }
            }
        }
        false
    }
    
    /// 绘制离开按钮
    fn draw_leave_button(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect, y: f32) -> bool {
        let button_pos = egui::pos2(bg_rect.min.x + 205.0, y);
        
        if let Some(info) = LibraryName::Title.get_egui_texture(ctx, 276) {
            if let Some(btn_texture) = info.egui_texture {
                let btn_rect = egui::Rect::from_min_size(button_pos, btn_texture.size_vec2());
                let response = ui.interact(btn_rect, egui::Id::new("quest_leave"), egui::Sense::click());
                
                let texture_id = if response.is_pointer_button_down_on() {
                    LibraryName::Title.get_egui_texture(ctx, 278)
                        .and_then(|info| info.egui_texture)
                        .map(|t| t.id())
                        .unwrap_or(btn_texture.id())
                } else if response.hovered() {
                    LibraryName::Title.get_egui_texture(ctx, 277)
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
                    self.visible = false;
                    println!("👋 关闭任务列表");
                    return true;
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
                let response = ui.interact(close_rect, egui::Id::new("quest_list_close"), egui::Sense::click());
                
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
        
        let drag_response = ui.interact(title_area, egui::Id::new("quest_list_drag"), egui::Sense::drag());
        
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

impl Dialog for QuestListDialog {
    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        if !self.visible {
            *open = false;
            return;
        }
        
        println!("🖼️ QuestListDialog::show 被调用, visible={}", self.visible);
        
        egui::Area::new(egui::Id::new("quest_list_dialog"))
            .fixed_pos(self.position)
            .movable(false)
            .show(ctx, |ui| {
                println!("🎨 开始绘制 QuestListDialog UI");
                
                // 绘制背景
                let bg_rect = self.draw_background(ui, ctx);
                
                // 处理窗口拖拽
                self.handle_window_dragging(ui, ctx, &bg_rect);
                
                // 绘制任务选择区域
                self.draw_quest_selection(ui, ctx, &bg_rect);
                
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
