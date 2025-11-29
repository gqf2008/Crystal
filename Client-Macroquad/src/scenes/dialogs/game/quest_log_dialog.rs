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
            QuestInfo {
                id: 5,
                name: "护送公主".to_string(),
                description: "公主需要前往邻国参加婚礼，请沿途保护她的安全。".to_string(),
                npc_name: "国王".to_string(),
                status: QuestStatus::Accepted,
                progress: 2,
                max_progress: 5,
                level_required: 25,
                rewards: QuestRewards {
                    experience: 8000,
                    gold: 5000,
                    items: vec![],
                },
            },
            QuestInfo {
                id: 6,
                name: "讨伐山贼".to_string(),
                description: "山贼在官道上劫掠过往商旅，请前去剿灭。".to_string(),
                npc_name: "捕快".to_string(),
                status: QuestStatus::Available,
                progress: 0,
                max_progress: 20,
                level_required: 12,
                rewards: QuestRewards {
                    experience: 3000,
                    gold: 1500,
                    items: vec![],
                },
            },
            QuestInfo {
                id: 7,
                name: "修复古桥".to_string(),
                description: "村口的古桥年久失修，需要收集材料进行修复。".to_string(),
                npc_name: "工匠".to_string(),
                status: QuestStatus::Accepted,
                progress: 50,
                max_progress: 100,
                level_required: 8,
                rewards: QuestRewards {
                    experience: 1200,
                    gold: 300,
                    items: vec![],
                },
            },
            QuestInfo {
                id: 8,
                name: "学习新技能".to_string(),
                description: "武馆师傅愿意教你一招新技能，但需要先通过考验。".to_string(),
                npc_name: "武馆师傅".to_string(),
                status: QuestStatus::Accepted,
                progress: 1,
                max_progress: 3,
                level_required: 20,
                rewards: QuestRewards {
                    experience: 5000,
                    gold: 0,
                    items: vec![],
                },
            },
            QuestInfo {
                id: 9,
                name: "拯救矿工".to_string(),
                description: "矿洞发生塌方，有矿工被困其中，请尽快救援。".to_string(),
                npc_name: "矿主".to_string(),
                status: QuestStatus::Accepted,
                progress: 0,
                max_progress: 5,
                level_required: 22,
                rewards: QuestRewards {
                    experience: 6000,
                    gold: 3000,
                    items: vec![],
                },
            },
            QuestInfo {
                id: 10,
                name: "消灭巨型蜘蛛".to_string(),
                description: "森林深处出现了巨型蜘蛛，威胁到伐木工的安全。".to_string(),
                npc_name: "伐木工头".to_string(),
                status: QuestStatus::Available,
                progress: 0,
                max_progress: 8,
                level_required: 16,
                rewards: QuestRewards {
                    experience: 3500,
                    gold: 1200,
                    items: vec![],
                },
            },
            QuestInfo {
                id: 11,
                name: "寻找传说之剑".to_string(),
                description: "传说中有一把神剑被封印在远古遗迹中，等待有缘人将其唤醒。".to_string(),
                npc_name: "老隐士".to_string(),
                status: QuestStatus::Available,
                progress: 0,
                max_progress: 1,
                level_required: 50,
                rewards: QuestRewards {
                    experience: 50000,
                    gold: 10000,
                    items: vec![],
                },
            },
            QuestInfo {
                id: 12,
                name: "采集草药".to_string(),
                description: "药师需要特殊的草药来制作治疗药水，请到森林中采集。".to_string(),
                npc_name: "药师".to_string(),
                status: QuestStatus::Accepted,
                progress: 8,
                max_progress: 15,
                level_required: 3,
                rewards: QuestRewards {
                    experience: 600,
                    gold: 150,
                    items: vec![],
                },
            },
            QuestInfo {
                id: 13,
                name: "驯服野马".to_string(),
                description: "马厩主人希望你能驯服草原上的野马。".to_string(),
                npc_name: "马倌".to_string(),
                status: QuestStatus::Available,
                progress: 0,
                max_progress: 3,
                level_required: 15,
                rewards: QuestRewards {
                    experience: 2500,
                    gold: 800,
                    items: vec![],
                },
            },
            QuestInfo {
                id: 14,
                name: "调查神秘洞穴".to_string(),
                description: "村民报告在山脚发现了一个发光的洞穴，请前去调查。".to_string(),
                npc_name: "猎人".to_string(),
                status: QuestStatus::Completed,
                progress: 1,
                max_progress: 1,
                level_required: 18,
                rewards: QuestRewards {
                    experience: 4000,
                    gold: 2500,
                    items: vec![],
                },
            },
            QuestInfo {
                id: 15,
                name: "寻找失踪的商人".to_string(),
                description: "一位商人在前往城镇的路上失踪了，请帮忙寻找他的下落。".to_string(),
                npc_name: "公会长".to_string(),
                status: QuestStatus::Available,
                progress: 0,
                max_progress: 3,
                level_required: 10,
                rewards: QuestRewards {
                    experience: 1500,
                    gold: 500,
                    items: vec![],
                },
            },
        ];
        
        Self {
            visible: false,
            // 原工程位置: Settings.ScreenWidth / 2 - 300 - 20, 60
            // 假设屏幕宽度800，则位置为 (800/2 - 300 - 20, 60) = (80, 60)
            position: egui::pos2(80.0, 60.0),
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
        // 任务对话框背景纹理 (Index: 961 from Prguse)
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 961) {
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
        // 列表区域铺满背景宽度，留出右侧滚动按钮空间
        let list_area = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 10.0, bg_rect.min.y + 40.0),
            egui::vec2(bg_rect.width() - 40.0, bg_rect.height() - 70.0)
        );
        
        // 绘制任务项
        let item_height = 60.0;
        let mut y_offset = list_area.min.y + 5.0 - self.scroll_offset;
        
        // 先收集点击事件，再更新选中状态
        let mut clicked_index: Option<usize> = None;
        
        // 绘制任务项 - 内联以避免借用问题
        for (i, quest) in self.quests.iter().enumerate() {
            // 只绘制完全在可视区域内的项目
            let item_bottom = y_offset + item_height;
            let item_top = y_offset;
            
            // 只绘制完整可见的任务项（不绘制部分可见的）
            if item_top >= list_area.min.y && item_bottom <= list_area.max.y {
                // 内联绘制任务项 - 铺满列表区域宽度
                let item_rect = egui::Rect::from_min_size(
                    egui::pos2(list_area.min.x + 5.0, y_offset), 
                    egui::vec2(list_area.width() - 10.0, item_height)
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
                    egui::FontId::proportional(12.0),
                    egui::Color32::YELLOW,
                );
                
                // 任务状态
                let status_text = match quest.status {
                    QuestStatus::Available => "可接受",
                    QuestStatus::Accepted => "进行中",
                    QuestStatus::Completed => "已完成",
                    QuestStatus::Failed => "已失败",
                };
                ui.painter().text(
                    egui::pos2(item_rect.min.x + 10.0, item_rect.min.y + 30.0),
                    egui::Align2::LEFT_TOP,
                    status_text,
                    egui::FontId::proportional(10.0),
                    egui::Color32::GRAY,
                );
                
                // 检查点击
                let response = ui.interact(item_rect, egui::Id::new(format!("quest_log_{}", i)), egui::Sense::click());
                if response.clicked() {
                    clicked_index = Some(i);
                }
            }
            y_offset += item_height + 5.0;
        }
        
        // 更新选中状态
        if let Some(idx) = clicked_index {
            self.selected_quest = Some(idx);
            if let Some(quest) = self.quests.get(idx) {
                println!("📋 选中任务: {}", quest.name);
            }
        }
        
        // 计算最大滚动范围
        let total_content_height = self.quests.len() as f32 * (item_height + 5.0);
        let visible_height = list_area.height() - 10.0;
        let max_scroll = (total_content_height - visible_height).max(0.0);
        
        // 处理鼠标滚轮滚动
        let list_response = ui.interact(list_area, egui::Id::new("quest_list_scroll"), egui::Sense::hover());
        if list_response.hovered() {
            let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
            // 减小滚动幅度，每次滚动一行的高度
            let scroll_step = (item_height + 5.0) * 0.5; // 滚动速度减半
            self.scroll_offset = (self.scroll_offset - scroll_delta * scroll_step / 50.0).clamp(0.0, max_scroll);
        }
        
        // 绘制滚动按钮
        self.draw_scroll_buttons(ui, ctx, bg_rect, max_scroll, item_height);
    }
    
    /// 绘制滚动按钮
    fn draw_scroll_buttons(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect, max_scroll: f32, item_height: f32) {
        let scroll_step = item_height + 5.0;
        let button_size = 16.0;
        
        // 上滚动按钮位置 - 在右侧边缘，按钮的底部在列表区域顶部
        let scroll_up_pos = egui::pos2(bg_rect.max.x - 25.0, bg_rect.min.y + 40.0 - button_size);
        // 下滚动按钮位置 - 在列表区域底部
        let scroll_down_pos = egui::pos2(bg_rect.max.x - 25.0, bg_rect.max.y - 30.0 - button_size);
        
        // 绘制上滚动按钮
        let up_button_rect = egui::Rect::from_min_size(scroll_up_pos, egui::vec2(button_size, button_size));
        let up_response = ui.interact(up_button_rect, egui::Id::new("quest_scroll_up"), egui::Sense::click());
        
        // 尝试使用纹理，否则用简单图形
        if let Some(info) = LibraryName::Prguse2.get_egui_texture(ctx, 197) {
            if let Some(texture) = info.egui_texture {
                let texture_idx = if up_response.is_pointer_button_down_on() {
                    199
                } else if up_response.hovered() {
                    198
                } else {
                    197
                };
                if let Some(info2) = LibraryName::Prguse2.get_egui_texture(ctx, texture_idx) {
                    if let Some(tex) = info2.egui_texture {
                        ui.painter().image(
                            tex.id(),
                            up_button_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
            }
        } else {
            // 降级绘制
            let color = if up_response.hovered() { egui::Color32::YELLOW } else { egui::Color32::GRAY };
            ui.painter().rect_filled(up_button_rect, 2.0, egui::Color32::from_rgb(50, 50, 60));
            ui.painter().text(
                up_button_rect.center(),
                egui::Align2::CENTER_CENTER,
                "▲",
                egui::FontId::proportional(10.0),
                color,
            );
        }
        
        if up_response.clicked() {
            self.scroll_offset = (self.scroll_offset - scroll_step).max(0.0);
        }
        
        // 绘制下滚动按钮
        let down_button_rect = egui::Rect::from_min_size(scroll_down_pos, egui::vec2(button_size, button_size));
        let down_response = ui.interact(down_button_rect, egui::Id::new("quest_scroll_down"), egui::Sense::click());
        
        if let Some(info) = LibraryName::Prguse2.get_egui_texture(ctx, 207) {
            if let Some(texture) = info.egui_texture {
                let texture_idx = if down_response.is_pointer_button_down_on() {
                    209
                } else if down_response.hovered() {
                    208
                } else {
                    207
                };
                if let Some(info2) = LibraryName::Prguse2.get_egui_texture(ctx, texture_idx) {
                    if let Some(tex) = info2.egui_texture {
                        ui.painter().image(
                            tex.id(),
                            down_button_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
            }
        } else {
            // 降级绘制
            let color = if down_response.hovered() { egui::Color32::YELLOW } else { egui::Color32::GRAY };
            ui.painter().rect_filled(down_button_rect, 2.0, egui::Color32::from_rgb(50, 50, 60));
            ui.painter().text(
                down_button_rect.center(),
                egui::Align2::CENTER_CENTER,
                "▼",
                egui::FontId::proportional(10.0),
                color,
            );
        }
        
        if down_response.clicked() {
            self.scroll_offset = (self.scroll_offset + scroll_step).min(max_scroll);
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
    fn draw_quest_details(&self, ui: &mut egui::Ui, _ctx: &egui::Context, bg_rect: &egui::Rect) {
        let detail_area = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 200.0, bg_rect.min.y + 40.0),
            egui::vec2(bg_rect.width() - 220.0, bg_rect.height() - 80.0)
        );
        
        if let Some(quest_index) = self.selected_quest {
            if let Some(quest) = self.quests.get(quest_index) {
                // 使用egui的ScrollArea和Label来实现文字自动换行
                let text_area = detail_area.shrink(10.0);
                
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(text_area), |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("quest_log_detail_scroll")
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                        .show(ui, |ui| {
                            ui.set_width(text_area.width() - 10.0);
                            
                            // 任务名称
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&quest.name)
                                        .size(16.0)
                                        .color(egui::Color32::YELLOW)
                                )
                                .wrap()
                            );
                            ui.add_space(10.0);
                            
                            // 发布者和等级需求
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(format!("发布者: {} | 需求等级: {}", quest.npc_name, quest.level_required))
                                        .size(11.0)
                                        .color(egui::Color32::GRAY)
                                )
                            );
                            ui.add_space(5.0);
                            
                            // 任务状态
                            let status_text = match quest.status {
                                QuestStatus::Available => "可接受",
                                QuestStatus::Accepted => "进行中",
                                QuestStatus::Completed => "已完成",
                                QuestStatus::Failed => "已失败",
                            };
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(format!("状态: {}", status_text))
                                        .size(11.0)
                                        .color(egui::Color32::WHITE)
                                )
                            );
                            ui.add_space(10.0);
                            
                            // 任务描述标题
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new("任务描述:")
                                        .size(12.0)
                                        .color(egui::Color32::YELLOW)
                                )
                            );
                            ui.add_space(5.0);
                            
                            // 任务描述内容（自动换行）
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&quest.description)
                                        .size(11.0)
                                        .color(egui::Color32::WHITE)
                                )
                                .wrap()
                            );
                            ui.add_space(15.0);
                            
                            // 任务奖励
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new("任务奖励:")
                                        .size(12.0)
                                        .color(egui::Color32::YELLOW)
                                )
                            );
                            ui.add_space(5.0);
                            
                            // 经验和金币
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(format!("经验: {} | 金币: {}", quest.rewards.experience, quest.rewards.gold))
                                        .size(11.0)
                                        .color(egui::Color32::WHITE)
                                )
                            );
                            
                            // 物品奖励
                            if !quest.rewards.items.is_empty() {
                                ui.add_space(5.0);
                                for item in &quest.rewards.items {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(format!("  - {} x{}", item.name, item.count))
                                                .size(10.0)
                                                .color(egui::Color32::from_rgb(200, 200, 150))
                                        )
                                    );
                                }
                            }
                        });
                });
            }
        } else {
            // 无选中任务时显示提示
            ui.painter().text(
                detail_area.center(),
                egui::Align2::CENTER_CENTER,
                "选择左侧任务查看详情",
                egui::FontId::proportional(12.0),
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
        // 原工程: Prguse2 的 360/361/362，位置 (289, 3)
        let close_pos = egui::pos2(bg_rect.min.x + 289.0, bg_rect.min.y + 3.0);
        
        if let Some(info) = LibraryName::Prguse2.get_egui_texture(ctx, 360) {
            if let Some(close_texture) = info.egui_texture {
                let close_size = close_texture.size_vec2();
                let close_rect = egui::Rect::from_min_size(close_pos, close_size);
                
                let response = ui.interact(close_rect, egui::Id::new("quest_close"), egui::Sense::click());
                
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
        if !*open {
            self.visible = false;
            return;
        }
        self.visible = true;
        
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