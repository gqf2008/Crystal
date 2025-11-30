// ============================================================================
// QuestLogDialogHybrid - 任务日志对话框（混合版本）
// ============================================================================
//
// 【实现方式】
// - 使用 macroquad 原生 draw_* 函数绘制
// - 使用 DragHelper 实现拖拽功能
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::DragHelper;

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

/// 任务日志标签页
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestTab {
    InProgress, // 进行中
    Available,  // 可接受
    Completed,  // 已完成
}

/// 任务日志对话框（混合版本）
pub struct QuestLogDialogHybrid {
    /// 窗口位置
    position: Vec2,
    /// 是否可见
    visible: bool,
    /// 对话框尺寸
    size: Vec2,
    /// 任务列表
    quests: Vec<QuestInfo>,
    /// 当前选中的任务索引
    selected_quest: Option<usize>,
    /// 当前标签页
    active_tab: QuestTab,
    /// 滚动偏移
    scroll_offset: f32,
    /// 背景纹理
    bg_texture: Option<Texture2D>,
    /// 拖拽辅助器
    drag_helper: DragHelper,
}

impl QuestLogDialogHybrid {
    pub fn new() -> Self {
        // 创建示例任务
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
                    items: vec![],
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
                    items: vec![],
                },
            },
            QuestInfo {
                id: 4,
                name: "击败骷髅战士".to_string(),
                description: "地牢深处的骷髅战士复活了，需要强大的战士将其消灭。".to_string(),
                npc_name: "队长".to_string(),
                status: QuestStatus::Completed,
                progress: 1,
                max_progress: 1,
                level_required: 20,
                rewards: QuestRewards {
                    experience: 5000,
                    gold: 2000,
                    items: vec![],
                },
            },
        ];

        Self {
            position: vec2(200.0, 150.0),
            visible: false,
            size: vec2(400.0, 500.0),
            quests,
            selected_quest: None,
            active_tab: QuestTab::InProgress,
            scroll_offset: 0.0,
            bg_texture: None,
            drag_helper: DragHelper::new(),
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
        point.x >= self.position.x
            && point.x <= self.position.x + self.size.x
            && point.y >= self.position.y
            && point.y <= self.position.y + self.size.y
    }

    /// 异步加载纹理
    pub async fn load_textures(&mut self) {
        // 预加载任务日志纹理
        if let Some(texture) = LibraryName::Prguse.get_texture(1750) {
            self.size = vec2(texture.width as f32, texture.height as f32);
            if let Some(tex) = texture.image {
                self.bg_texture = Some(tex);
            }
        }
    }

    /// 获取当前标签页的任务列表
    fn get_filtered_quests(&self) -> Vec<&QuestInfo> {
        self.quests
            .iter()
            .filter(|q| match self.active_tab {
                QuestTab::InProgress => q.status == QuestStatus::Accepted,
                QuestTab::Available => q.status == QuestStatus::Available,
                QuestTab::Completed => q.status == QuestStatus::Completed,
            })
            .collect()
    }

    /// 更新和绘制
    pub fn update_and_draw(&mut self) {
        if !self.visible {
            return;
        }

        let mouse_pos = vec2(mouse_position().0, mouse_position().1);

        // 使用 DragHelper 实现拖拽
        let drag_area = Rect::new(self.position.x, self.position.y, self.size.x, 30.0);
        self.drag_helper.apply(drag_area, &mut self.position);

        // 绘制背景
        self.draw_background();

        // 绘制标签页按钮
        self.draw_tab_buttons(mouse_pos);

        // 绘制任务列表
        self.draw_quest_list(mouse_pos);

        // 绘制任务详情
        self.draw_quest_detail();

        // 绘制关闭按钮
        if self.draw_close_button(mouse_pos) {
            self.close();
        }
    }

    /// 绘制背景
    fn draw_background(&self) {
        if let Some(texture) = &self.bg_texture {
            draw_texture_ex(
                texture,
                self.position.x,
                self.position.y,
                WHITE,
                DrawTextureParams::default(),
            );
        } else {
            // 降级
            draw_rectangle(
                self.position.x,
                self.position.y,
                self.size.x,
                self.size.y,
                Color::from_rgba(35, 35, 45, 250),
            );
            draw_rectangle_lines(
                self.position.x,
                self.position.y,
                self.size.x,
                self.size.y,
                2.0,
                Color::from_rgba(100, 100, 100, 255),
            );
        }

        // 标题
        draw_text(
            "任务日志",
            self.position.x + self.size.x / 2.0 - 40.0,
            self.position.y + 25.0,
            20.0,
            Color::from_rgba(255, 215, 0, 255),
        );
    }

    /// 绘制标签页按钮
    fn draw_tab_buttons(&mut self, mouse_pos: Vec2) {
        let tab_y = self.position.y + 45.0;
        let tab_buttons = [
            (QuestTab::InProgress, "进行中", self.position.x + 20.0),
            (QuestTab::Available, "可接受", self.position.x + 100.0),
            (QuestTab::Completed, "已完成", self.position.x + 180.0),
        ];

        for (tab, label, x) in tab_buttons {
            let button_rect = Rect::new(x, tab_y, 70.0, 25.0);
            let is_active = self.active_tab == tab;
            let is_hovered = button_rect.contains(mouse_pos);

            let bg_color = if is_active {
                Color::from_rgba(80, 120, 160, 255)
            } else if is_hovered {
                Color::from_rgba(60, 60, 70, 255)
            } else {
                Color::from_rgba(50, 50, 55, 255)
            };

            draw_rectangle(button_rect.x, button_rect.y, button_rect.w, button_rect.h, bg_color);
            draw_rectangle_lines(
                button_rect.x,
                button_rect.y,
                button_rect.w,
                button_rect.h,
                1.0,
                Color::from_rgba(100, 100, 100, 255),
            );

            draw_text(label, button_rect.x + 10.0, button_rect.y + 17.0, 14.0, WHITE);

            if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
                self.active_tab = tab;
                self.selected_quest = None;
            }
        }
    }

    /// 绘制任务列表
    fn draw_quest_list(&mut self, mouse_pos: Vec2) {
        let list_x = self.position.x + 20.0;
        let list_y = self.position.y + 80.0;
        let list_width = 360.0;
        let list_height = 180.0;

        // 列表背景
        draw_rectangle(
            list_x,
            list_y,
            list_width,
            list_height,
            Color::from_rgba(25, 25, 35, 200),
        );
        draw_rectangle_lines(
            list_x,
            list_y,
            list_width,
            list_height,
            1.0,
            Color::from_rgba(80, 80, 80, 255),
        );

        let filtered_quests = self.get_filtered_quests();
        let item_height = 35.0;
        let mut y = list_y + 5.0;
        let mut clicked_idx: Option<usize> = None;

        for (i, quest) in filtered_quests.iter().enumerate() {
            if y + item_height > list_y + list_height {
                break;
            }

            let item_rect = Rect::new(list_x + 5.0, y, list_width - 10.0, item_height);
            let is_selected = self.selected_quest == Some(i);
            let is_hovered = item_rect.contains(mouse_pos);

            // 项目背景
            let bg_color = if is_selected {
                Color::from_rgba(60, 80, 100, 255)
            } else if is_hovered {
                Color::from_rgba(45, 45, 55, 255)
            } else {
                Color::from_rgba(35, 35, 45, 200)
            };
            draw_rectangle(item_rect.x, item_rect.y, item_rect.w, item_rect.h, bg_color);

            // 任务名称
            let name_color = match quest.status {
                QuestStatus::Available => Color::from_rgba(255, 255, 100, 255),
                QuestStatus::Accepted => WHITE,
                QuestStatus::Completed => Color::from_rgba(100, 255, 100, 255),
                QuestStatus::Failed => Color::from_rgba(255, 100, 100, 255),
            };
            draw_text_cn(&quest.name, item_rect.x + 10.0, y + 15.0, 14.0, name_color);

            // 进度
            if quest.status == QuestStatus::Accepted {
                let progress_text = format!("{}/{}", quest.progress, quest.max_progress);
                draw_text_cn(
                    &progress_text,
                    item_rect.x + item_rect.w - 60.0,
                    y + 15.0,
                    12.0,
                    Color::from_rgba(150, 200, 255, 255),
                );

                // 进度条
                let progress_bar_width = 80.0;
                let progress_bar_height = 4.0;
                let progress_percent = quest.progress as f32 / quest.max_progress as f32;
                draw_rectangle(
                    item_rect.x + 10.0,
                    y + 25.0,
                    progress_bar_width,
                    progress_bar_height,
                    Color::from_rgba(40, 40, 40, 255),
                );
                draw_rectangle(
                    item_rect.x + 10.0,
                    y + 25.0,
                    progress_bar_width * progress_percent,
                    progress_bar_height,
                    Color::from_rgba(100, 200, 100, 255),
                );
            }

            // 记录点击
            if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
                clicked_idx = Some(i);
            }

            y += item_height;
        }

        // 处理点击（在循环外，避免借用冲突）
        if let Some(idx) = clicked_idx {
            self.selected_quest = Some(idx);
        }
    }

    /// 绘制任务详情
    fn draw_quest_detail(&self) {
        let detail_x = self.position.x + 20.0;
        let detail_y = self.position.y + 270.0;
        let detail_width = 360.0;
        let detail_height = 200.0;

        // 详情背景
        draw_rectangle(
            detail_x,
            detail_y,
            detail_width,
            detail_height,
            Color::from_rgba(25, 25, 35, 200),
        );
        draw_rectangle_lines(
            detail_x,
            detail_y,
            detail_width,
            detail_height,
            1.0,
            Color::from_rgba(80, 80, 80, 255),
        );

        let filtered_quests = self.get_filtered_quests();

        if let Some(idx) = self.selected_quest {
            if let Some(quest) = filtered_quests.get(idx) {
                let mut y = detail_y + 15.0;
                let line_height = 18.0;

                // 任务名称
                draw_text(
                    &quest.name,
                    detail_x + 10.0,
                    y,
                    16.0,
                    Color::from_rgba(255, 215, 0, 255),
                );
                y += line_height + 5.0;

                // NPC
                draw_text(
                    &format!("任务发布: {}", quest.npc_name),
                    detail_x + 10.0,
                    y,
                    12.0,
                    Color::from_rgba(150, 150, 150, 255),
                );
                y += line_height;

                // 需求等级
                draw_text(
                    &format!("需求等级: {}", quest.level_required),
                    detail_x + 10.0,
                    y,
                    12.0,
                    Color::from_rgba(150, 150, 150, 255),
                );
                y += line_height + 5.0;

                // 描述
                draw_text_cn("任务描述:", detail_x + 10.0, y, 12.0, Color::from_rgba(200, 200, 200, 255));
                y += line_height;

                // 简单的文字换行（每行约40个字符）
                let desc = &quest.description;
                let chars_per_line = 30;
                let mut start = 0;
                while start < desc.len() {
                    let end = (start + chars_per_line).min(desc.len());
                    let line: String = desc.chars().skip(start).take(end - start).collect();
                    draw_text_cn(&line, detail_x + 10.0, y, 11.0, Color::from_rgba(180, 180, 180, 255));
                    y += line_height - 4.0;
                    start = end;
                    if y > detail_y + detail_height - 50.0 {
                        break;
                    }
                }

                y += 10.0;

                // 奖励
                draw_text("奖励:", detail_x + 10.0, y, 12.0, Color::from_rgba(255, 215, 0, 255));
                y += line_height;

                draw_text(
                    &format!("经验: {} | 金币: {}", quest.rewards.experience, quest.rewards.gold),
                    detail_x + 10.0,
                    y,
                    11.0,
                    Color::from_rgba(150, 200, 255, 255),
                );
            }
        } else {
            draw_text(
                "请选择一个任务",
                detail_x + detail_width / 2.0 - 50.0,
                detail_y + detail_height / 2.0,
                14.0,
                Color::from_rgba(100, 100, 100, 255),
            );
        }
    }

    /// 绘制关闭按钮（返回是否点击）
    fn draw_close_button(&self, mouse_pos: Vec2) -> bool {
        let close_size = 20.0;
        let close_x = self.position.x + self.size.x - 25.0;
        let close_y = self.position.y + 5.0;
        let close_rect = Rect::new(close_x, close_y, close_size, close_size);

        let is_hovered = close_rect.contains(mouse_pos);

        let bg_color = if is_hovered {
            Color::from_rgba(200, 70, 70, 255)
        } else {
            Color::from_rgba(150, 50, 50, 255)
        };
        draw_rectangle(close_x, close_y, close_size, close_size, bg_color);

        draw_text("×", close_x + 4.0, close_y + 16.0, 18.0, WHITE);

        is_hovered && is_mouse_button_pressed(MouseButton::Left)
    }
}
