// ============================================================================
// QuestLogDialogHybrid - 任务日志对话框（混合版本）
// ============================================================================
//
// 【C# 原版参考】
// - 背景: Prguse[961]
// - 标题: Title[15] at (18, 9)
// - 关闭按钮: Title[193/194/195] at (200, 436)
// - 右上关闭按钮: Prguse2[360/361/362] at (289, 3)
// - 无标签页系统，只是简单的任务分组列表
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::{draw_text_cn, measure_text_cn};
use super::native_ui_utils::DragHelper;

const MAX_CONCURRENT_QUESTS: usize = 10;

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
    pub npc_name: String,
    pub status: QuestStatus,
    pub progress: u32,
    pub max_progress: u32,
    pub level_required: u32,
    pub rewards: QuestRewards,
    pub group: String,  // 任务分组
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

/// 任务日志对话框（混合版本）
/// 按照 C# QuestDiaryDialog 实现
pub struct QuestLogDialogHybrid {
    /// 窗口位置
    position: Vec2,
    /// 是否可见
    visible: bool,
    /// 对话框尺寸 (从纹理获取)
    size: Vec2,
    /// 任务列表
    quests: Vec<QuestInfo>,
    /// 当前选中的任务索引
    selected_quest: Option<usize>,
    /// 滚动偏移
    scroll_offset: f32,
    /// 展开的任务分组（为空表示全部展开，行为对齐 C# ExpandedGroups）
    expanded_groups: Vec<String>,
    /// 背景纹理 - Prguse[961]
    bg_texture: Option<Texture2D>,
    /// 标题纹理 - Title[15]
    title_texture: Option<Texture2D>,
    /// 关闭按钮纹理 - Title[193/194/195]
    close_button_textures: [Option<Texture2D>; 3],
    /// 右上关闭按钮纹理 - Prguse2[360/361/362]
    close_x_textures: [Option<Texture2D>; 3],
    /// 拖拽辅助器
    drag_helper: DragHelper,
    /// 是否显示任务详情面板
    show_details: bool,
    /// 任务完成通知队列
    completion_notifications: Vec<(u32, f32)>, // (quest_id, remaining_time)
}

impl Default for QuestLogDialogHybrid {
    fn default() -> Self {
        Self::new()
    }
}

impl QuestLogDialogHybrid {
    pub fn new() -> Self {
        Self {
            position: vec2(200.0, 60.0),
            visible: false,
            size: vec2(316.0, 466.0), // 默认值，会被纹理覆盖
            quests: Vec::new(),
            selected_quest: None,
            scroll_offset: 0.0,
            expanded_groups: Vec::new(),
            bg_texture: None,
            title_texture: None,
            close_button_textures: [None, None, None],
            close_x_textures: [None, None, None],
            drag_helper: DragHelper::new(),
            show_details: false,
            completion_notifications: Vec::new(),
        }
    }

    /// 显示对话框
    pub fn open(&mut self) {
        self.visible = true;
    }

    /// 关闭对话框
    pub fn close(&mut self) {
        self.visible = false;
    }

    /// 切换显示状态
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// 是否可见
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// 设置位置
    pub fn set_position(&mut self, pos: Vec2) {
        self.position = pos;
    }

    /// 获取位置
    pub fn get_position(&self) -> Vec2 {
        self.position
    }

    /// 检查点是否在对话框内
    pub fn contains(&self, point: Vec2) -> bool {
        if !self.visible {
            return false;
        }
        Rect::new(self.position.x, self.position.y, self.size.x, self.size.y).contains(point)
    }

    /// 异步加载纹理
    pub fn load_textures(&mut self) {
        // 背景纹理 - Prguse[961]
        if let Some(texture) = LibraryName::Prguse.get_texture(961) {
            self.size = vec2(texture.width as f32, texture.height as f32);
            if let Some(tex) = texture.image {
                self.bg_texture = Some(tex);
                println!("📋 任务日志背景 Prguse[961]: {}x{}", texture.width, texture.height);
            }
        }
        
        // 标题纹理 - Title[15]
        if let Some(texture) = LibraryName::Title.get_texture(15) {
            if let Some(tex) = texture.image {
                self.title_texture = Some(tex);
                println!("📋 任务日志标题 Title[15] 加载成功");
            }
        }
        
        // 关闭按钮 - Title[193/194/195] at (200, 436)
        for (i, idx) in [193, 194, 195].iter().enumerate() {
            if let Some(texture) = LibraryName::Title.get_texture(*idx) {
                if let Some(tex) = texture.image {
                    self.close_button_textures[i] = Some(tex);
                }
            }
        }
        
        // 右上关闭按钮 - Prguse2[360/361/362] at (289, 3)
        for (i, idx) in [360, 361, 362].iter().enumerate() {
            if let Some(texture) = LibraryName::Prguse2.get_texture(*idx) {
                if let Some(tex) = texture.image {
                    self.close_x_textures[i] = Some(tex);
                }
            }
        }
        
        println!("📋 任务日志对话框纹理加载完成");
    }

    /// 更新和绘制
    pub fn update_and_draw(&mut self) {
        if !self.visible {
            return;
        }

        let dt = get_frame_time();
        let mouse_pos = vec2(mouse_position().0, mouse_position().1);

        // 拖拽
        let drag_area = Rect::new(self.position.x, self.position.y, self.size.x, 30.0);
        self.drag_helper.apply(drag_area, &mut self.position);

        // 更新完成通知计时器
        for (_, timer) in self.completion_notifications.iter_mut() {
            *timer -= dt;
        }
        self.completion_notifications.retain(|(_, t)| *t > 0.0);

        // 绘制背景
        self.draw_background();

        // 绘制任务列表
        self.draw_quest_list(mouse_pos);

        // 绘制任务详情面板
        if self.show_details {
            self.draw_quest_details(mouse_pos);
        }

        // 绘制关闭按钮
        self.draw_close_buttons(mouse_pos);
    }

    /// 切换任务详情面板显示
    pub fn toggle_details(&mut self) {
        self.show_details = !self.show_details;
    }

    /// 添加任务完成通知
    pub fn notify_quest_complete(&mut self, quest_id: u32) {
        self.completion_notifications.push((quest_id, 3.0)); // 显示3秒
    }

    /// 更新任务进度
    pub fn update_quest_progress(&mut self, quest_id: u32, progress: u32) {
        if let Some(quest) = self.quests.iter_mut().find(|q| q.id == quest_id) {
            let old_progress = quest.progress;
            quest.progress = progress;
            if progress >= quest.max_progress && quest.max_progress > 0 {
                quest.status = QuestStatus::Completed;
                self.completion_notifications.push((quest_id, 3.0));
            }
            if progress != old_progress {
                tracing::info!("任务进度更新: {} ({}/{})", quest.name, progress, quest.max_progress);
            }
        }
    }

    /// 从文本更新任务进度（网络消息：progress 是分号分隔的任务列表）
    pub fn update_quest_progress_from_text(&mut self, quest_id: u32, progress_text: &str) {
        if let Some(quest) = self.quests.iter_mut().find(|q| q.id == quest_id) {
            // 计算完成的子任务数量
            let tasks: Vec<&str> = progress_text.split(';').filter(|s| !s.trim().is_empty()).collect();
            quest.max_progress = tasks.len().max(1) as u32;
            quest.progress = tasks.len() as u32;
            quest.description = progress_text.to_string();
            tracing::debug!("任务进度更新: {} - {}", quest.name, progress_text);
        }
    }

    /// 添加新任务
    pub fn add_quest(&mut self, quest: QuestInfo) {
        self.quests.push(quest);
    }

    /// 清空所有任务（用于切换到真实服务器数据前清除模拟数据）
    pub fn clear_quests(&mut self) {
        self.quests.clear();
        self.selected_quest = None;
    }

    /// 移除任务
    pub fn remove_quest(&mut self, quest_id: u32) {
        self.quests.retain(|q| q.id != quest_id);
        if let Some(selected) = self.selected_quest {
            if selected >= self.quests.len() {
                self.selected_quest = self.quests.last().map(|_| self.quests.len() - 1);
            }
        }
    }

    /// 获取可追踪的任务列表（已接受且未完成）
    pub fn get_trackable_quests(&self) -> Vec<&QuestInfo> {
        self.quests.iter()
            .filter(|q| q.status == QuestStatus::Accepted && q.progress < q.max_progress)
            .collect()
    }

    /// 绘制背景
    fn draw_background(&self) {
        // 背景纹理
        if let Some(texture) = &self.bg_texture {
            draw_texture_ex(
                texture,
                self.position.x,
                self.position.y,
                WHITE,
                DrawTextureParams::default(),
            );
        }

        // 标题纹理 - Title[15] at (18, 9)
        if let Some(title_tex) = &self.title_texture {
            draw_texture_ex(
                title_tex,
                self.position.x + 18.0,
                self.position.y + 9.0,
                WHITE,
                DrawTextureParams::default(),
            );
        }

        // _takenQuestsLabel: Location (210,7)
        let label = format!(
            "List: {}/{}",
            self.quests.len(),
            MAX_CONCURRENT_QUESTS
        );
        draw_text_cn(&label, self.position.x + 210.0, self.position.y + 20.0, 12.0, WHITE);
    }

    /// 绘制任务列表 (C# 原版从 Y=40 开始)
    fn draw_quest_list(&mut self, mouse_pos: Vec2) {
        let list_x = self.position.x + 15.0;
        let list_w = self.size.x - 30.0;
        let list_top = self.position.y + 40.0;
        let list_bottom = self.position.y + 430.0;
        let list_h = (list_bottom - list_top).max(0.0);
        let list_rect = Rect::new(list_x, list_top, list_w, list_h);

        // 鼠标滚轮滚动
        if list_rect.contains(mouse_pos) {
            let wheel = mouse_wheel().1;
            if wheel != 0.0 {
                self.scroll_offset = (self.scroll_offset - wheel * 20.0).max(0.0);
            }
        }

        // 按出现顺序分组（对齐 C# GroupBy 的直觉表现）
        let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
        for (idx, quest) in self.quests.iter().enumerate() {
            if let Some((_, indices)) = groups.iter_mut().find(|(g, _)| g == &quest.group) {
                indices.push(idx);
            } else {
                groups.push((quest.group.clone(), vec![idx]));
            }
        }

        let group_header_h = 20.0;
        let quest_item_h = 18.0;

        // 计算最大滚动
        let mut content_h = 0.0;
        for (group_name, indices) in groups.iter() {
            content_h += group_header_h;
            let expanded = self.expanded_groups.is_empty() || self.expanded_groups.iter().any(|g| g == group_name);
            if expanded {
                content_h += indices.len() as f32 * quest_item_h;
            }
        }
        let max_scroll = (content_h - list_h).max(0.0);
        if self.scroll_offset > max_scroll {
            self.scroll_offset = max_scroll;
        }

        let mut clicked_group: Option<String> = None;
        let mut clicked_quest: Option<usize> = None;

        let mut y = list_top - self.scroll_offset;

        for (group_name, indices) in groups.iter() {
            // Group header
            let header_rect = Rect::new(list_x, y, list_w, group_header_h);
            let header_visible = header_rect.y + header_rect.h > list_top && header_rect.y < list_bottom;

            let expanded = self.expanded_groups.is_empty() || self.expanded_groups.iter().any(|g| g == group_name);
            let header_hovered = header_rect.contains(mouse_pos);

            if header_visible {
                if header_hovered {
                    draw_rectangle(header_rect.x, header_rect.y, header_rect.w, header_rect.h, Color::from_rgba(60, 60, 80, 140));
                }

                let prefix = if expanded { "-" } else { "+" };
                draw_text_cn(
                    &format!("{} {}", prefix, group_name),
                    list_x + 5.0,
                    header_rect.y + 14.0,
                    12.0,
                    Color::from_rgba(230, 230, 230, 255),
                );

                if header_hovered && is_mouse_button_pressed(MouseButton::Left) {
                    clicked_group = Some(group_name.clone());
                }
            }

            y += group_header_h;

            if !expanded {
                continue;
            }

            // Quests
            for quest_idx in indices.iter().copied() {
                let quest = &self.quests[quest_idx];
                let item_rect = Rect::new(list_x + 10.0, y, list_w - 10.0, quest_item_h);
                let item_visible = item_rect.y + item_rect.h > list_top && item_rect.y < list_bottom;
                let is_selected = self.selected_quest == Some(quest_idx);
                let is_hovered = item_rect.contains(mouse_pos);

                if item_visible {
                    if is_selected || is_hovered {
                        let color = if is_selected {
                            Color::from_rgba(60, 80, 100, 150)
                        } else {
                            Color::from_rgba(50, 50, 60, 100)
                        };
                        draw_rectangle(item_rect.x, item_rect.y, item_rect.w, item_rect.h, color);
                    }

                    let name_color = match quest.status {
                        QuestStatus::Available => Color::from_rgba(255, 255, 100, 255),
                        QuestStatus::Accepted => WHITE,
                        QuestStatus::Completed => Color::from_rgba(100, 255, 100, 255),
                        QuestStatus::Failed => Color::from_rgba(255, 100, 100, 255),
                    };
                    draw_text_cn(&quest.name, item_rect.x + 5.0, item_rect.y + 13.0, 11.0, name_color);

                    if quest.status == QuestStatus::Accepted && quest.max_progress > 0 {
                        let progress_text = format!("{}/{}", quest.progress, quest.max_progress);
                        draw_text_cn(
                            &progress_text,
                            self.position.x + self.size.x - 60.0,
                            item_rect.y + 13.0,
                            11.0,
                            GRAY,
                        );
                    }

                    if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
                        clicked_quest = Some(quest_idx);
                    }
                }

                y += quest_item_h;
            }
        }

        if let Some(group) = clicked_group {
            // ExpandedGroups 为空表示“全部展开”；一旦用户操作，就进入显式列表模式
            if self.expanded_groups.is_empty() {
                // 复制当前所有组为展开，再 toggle 点击的那组
                self.expanded_groups = groups.iter().map(|(g, _)| g.clone()).collect();
            }

            if let Some(pos) = self.expanded_groups.iter().position(|g| g == &group) {
                self.expanded_groups.remove(pos);
            } else {
                self.expanded_groups.push(group);
            }
        }

        if let Some(idx) = clicked_quest {
            self.selected_quest = Some(idx);
        }
    }

    /// 绘制任务详情面板
    fn draw_quest_details(&mut self, _mouse_pos: Vec2) {
        let Some(selected_idx) = self.selected_quest else { return; };
        let Some(quest) = self.quests.get(selected_idx) else { return; };

        // 详情面板区域（在对话框底部，列表下方）
        let panel_x = self.position.x + 10.0;
        let panel_y = self.position.y + 280.0;
        let panel_w = self.size.x - 20.0;
        let panel_h = 140.0;

        // 半透明背景
        draw_rectangle(panel_x, panel_y, panel_w, panel_h, Color::from_rgba(20, 20, 30, 200));

        // 边框
        draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 1.0, Color::from_rgba(80, 80, 120, 255));

        // 任务名称（大字体）
        let name_color = match quest.status {
            QuestStatus::Available => Color::from_rgba(255, 255, 100, 255),
            QuestStatus::Accepted => WHITE,
            QuestStatus::Completed => Color::from_rgba(100, 255, 100, 255),
            QuestStatus::Failed => Color::from_rgba(255, 100, 100, 255),
        };
        draw_text_cn(&quest.name, panel_x + 10.0, panel_y + 15.0, 14.0, name_color);

        // 任务状态标签
        let status_text = match quest.status {
            QuestStatus::Available => "可接受",
            QuestStatus::Accepted => "进行中",
            QuestStatus::Completed => "已完成",
            QuestStatus::Failed => "已失败",
        };
        draw_text_cn(&format!("[{}]", status_text), panel_x + panel_w - 80.0, panel_y + 15.0, 12.0, name_color);

        // 任务描述
        draw_text_cn(&quest.description, panel_x + 10.0, panel_y + 35.0, 11.0, Color::from_rgba(200, 200, 200, 255));

        // 任务来源 NPC
        if !quest.npc_name.is_empty() {
            draw_text_cn(&format!("来源: {}", quest.npc_name), panel_x + 10.0, panel_y + 70.0, 10.0, Color::from_rgba(180, 180, 255, 255));
        }

        // 等级需求
        if quest.level_required > 0 {
            let lvl_color = if quest.level_required > 0 { Color::from_rgba(255, 200, 100, 255) } else { GRAY };
            draw_text_cn(&format!("需要等级: {}", quest.level_required), panel_x + 10.0, panel_y + 85.0, 10.0, lvl_color);
        }

        // 进度条
        if quest.max_progress > 0 {
            let progress_x = panel_x + 10.0;
            let progress_y = panel_y + 100.0;
            let progress_w = panel_w - 20.0;
            let progress_h = 12.0;

            // 背景
            draw_rectangle(progress_x, progress_y, progress_w, progress_h, Color::from_rgba(40, 40, 50, 255));

            // 进度填充
            let progress_ratio = quest.progress as f32 / quest.max_progress as f32;
            let fill_w = (progress_w * progress_ratio).clamp(0.0, progress_w);
            let fill_color = if quest.status == QuestStatus::Completed {
                Color::from_rgba(80, 200, 80, 200)
            } else {
                Color::from_rgba(100, 150, 255, 200)
            };
            draw_rectangle(progress_x, progress_y, fill_w, progress_h, fill_color);

            // 进度文字
            let progress_text = format!("{}/{}", quest.progress, quest.max_progress);
            let dims = measure_text_cn(&progress_text, 9.0);
            draw_text_cn(&progress_text, progress_x + (progress_w - dims.width) / 2.0, progress_y + 9.0, 9.0, WHITE);
        }

        // 奖励信息
        let reward_y = if quest.max_progress > 0 { panel_y + 118.0 } else { panel_y + 100.0 };
        let mut reward_text = format!("奖励: {} 经验, {} 金币", quest.rewards.experience, quest.rewards.gold);
        if !quest.rewards.items.is_empty() {
            let item_names: Vec<&str> = quest.rewards.items.iter().map(|i| i.name.as_str()).collect();
            reward_text.push_str(&format!(", 物品: {}", item_names.join(", ")));
        }
        draw_text_cn(&reward_text, panel_x + 10.0, reward_y, 10.0, Color::from_rgba(255, 220, 100, 255));
    }

    /// 绘制任务完成通知
    pub fn draw_completion_notifications(&self) {
        if self.completion_notifications.is_empty() {
            return;
        }

        let screen_w = screen_width();
        let start_y = 80.0;
        let spacing = 30.0;

        for (i, (quest_id, remaining)) in self.completion_notifications.iter().enumerate() {
            if let Some(quest) = self.quests.iter().find(|q| q.id == *quest_id) {
                let alpha = (remaining / 3.0 * 255.0) as u8;
                let y = start_y + i as f32 * spacing;

                // 半透明背景
                let text_w = 300.0;
                let text_h = 25.0;
                let x = (screen_w - text_w) / 2.0;
                draw_rectangle(x, y, text_w, text_h, Color::from_rgba(20, 40, 20, (alpha as f32 * 0.8).min(255.0) as u8));

                // 完成文字
                let msg = format!("任务完成: {}", quest.name);
                draw_text_cn(&msg, x + 10.0, y + 15.0, 13.0, Color::from_rgba(100, 255, 100, alpha));
            }
        }
    }

    /// 绘制任务追踪面板（游戏屏幕右侧小面板）
    pub fn draw_quest_tracker(&self, tracker_x: f32, tracker_y: f32) {
        let trackable = self.get_trackable_quests();
        if trackable.is_empty() {
            return;
        }

        let panel_w = 220.0;
        let header_h = 22.0;
        let quest_h = 40.0;
        let panel_h = header_h + trackable.len() as f32 * quest_h + 5.0;

        // 面板背景
        draw_rectangle(tracker_x, tracker_y, panel_w, panel_h, Color::from_rgba(15, 15, 25, 220));
        draw_rectangle_lines(tracker_x, tracker_y, panel_w, header_h, 1.0, Color::from_rgba(80, 80, 120, 200));

        // 标题
        draw_text_cn("任务追踪", tracker_x + 5.0, tracker_y + 15.0, 12.0, Color::from_rgba(255, 220, 100, 255));

        // 任务列表
        for (i, quest) in trackable.iter().enumerate() {
            let qy = tracker_y + header_h + i as f32 * quest_h;

            // 任务名称
            draw_text_cn(&quest.name, tracker_x + 5.0, qy + 12.0, 11.0, Color::from_rgba(255, 255, 150, 255));

            // 进度
            if quest.max_progress > 0 {
                // 进度条背景
                let bar_x = tracker_x + 5.0;
                let bar_y = qy + 25.0;
                let bar_w = panel_w - 15.0;
                let bar_h = 8.0;
                draw_rectangle(bar_x, bar_y, bar_w, bar_h, Color::from_rgba(40, 40, 50, 255));

                // 进度填充
                let progress_ratio = quest.progress as f32 / quest.max_progress as f32;
                let fill_w = (bar_w * progress_ratio).clamp(0.0, bar_w);
                draw_rectangle(bar_x, bar_y, fill_w, bar_h, Color::from_rgba(100, 150, 255, 180));

                // 进度文字
                let progress_text = format!("{}/{}", quest.progress, quest.max_progress);
                let dims = measure_text_cn(&progress_text, 8.0);
                draw_text_cn(&progress_text, bar_x + (bar_w - dims.width) / 2.0, bar_y + 6.0, 8.0, WHITE);
            }
        }
    }

    /// 绘制关闭按钮
    fn draw_close_buttons(&mut self, mouse_pos: Vec2) {
        // 底部关闭按钮 - Title[193/194/195] at (200, 436)
        let btn_x = self.position.x + 200.0;
        let btn_y = self.position.y + 436.0;
        
        if let Some(normal) = &self.close_button_textures[0] {
            let btn_rect = Rect::new(btn_x, btn_y, normal.width(), normal.height());
            let is_hovered = btn_rect.contains(mouse_pos);
            let is_pressed = is_hovered && is_mouse_button_down(MouseButton::Left);
            
            let texture = if is_pressed {
                self.close_button_textures[2].as_ref().unwrap_or(normal)
            } else if is_hovered {
                self.close_button_textures[1].as_ref().unwrap_or(normal)
            } else {
                normal
            };
            
            draw_texture_ex(texture, btn_x, btn_y, WHITE, DrawTextureParams::default());
            
            if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
                self.close();
            }
        }
        
        // 右上关闭按钮 - Prguse2[360/361/362] at (289, 3)
        let close_x = self.position.x + 289.0;
        let close_y = self.position.y + 3.0;
        
        if let Some(normal) = &self.close_x_textures[0] {
            let btn_rect = Rect::new(close_x, close_y, normal.width(), normal.height());
            let is_hovered = btn_rect.contains(mouse_pos);
            let is_pressed = is_hovered && is_mouse_button_down(MouseButton::Left);
            
            let texture = if is_pressed {
                self.close_x_textures[2].as_ref().unwrap_or(normal)
            } else if is_hovered {
                self.close_x_textures[1].as_ref().unwrap_or(normal)
            } else {
                normal
            };
            
            draw_texture_ex(texture, close_x, close_y, WHITE, DrawTextureParams::default());
            
            if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
                self.close();
            }
        }
    }
}
