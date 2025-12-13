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
use crate::ui::text_renderer::draw_text_cn;
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
                group: "新手任务".to_string(),
                rewards: QuestRewards {
                    experience: 500,
                    gold: 100,
                    items: vec![],
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
                group: "新手任务".to_string(),
                rewards: QuestRewards {
                    experience: 800,
                    gold: 200,
                    items: vec![],
                },
            },
        ];

        Self {
            position: vec2(200.0, 60.0),
            visible: false,
            size: vec2(316.0, 466.0), // 默认值，会被纹理覆盖
            quests,
            selected_quest: None,
            scroll_offset: 0.0,
            expanded_groups: Vec::new(),
            bg_texture: None,
            title_texture: None,
            close_button_textures: [None, None, None],
            close_x_textures: [None, None, None],
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
        Rect::new(self.position.x, self.position.y, self.size.x, self.size.y).contains(point)
    }

    /// 异步加载纹理
    pub async fn load_textures(&mut self) {
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

        let mouse_pos = vec2(mouse_position().0, mouse_position().1);

        // 拖拽
        let drag_area = Rect::new(self.position.x, self.position.y, self.size.x, 30.0);
        self.drag_helper.apply(drag_area, &mut self.position);

        // 绘制背景
        self.draw_background();

        // 绘制任务列表
        self.draw_quest_list(mouse_pos);

        // 绘制关闭按钮
        self.draw_close_buttons(mouse_pos);
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
