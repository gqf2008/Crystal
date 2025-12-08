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
    }

    /// 绘制任务列表 (C# 原版从 Y=40 开始)
    fn draw_quest_list(&mut self, mouse_pos: Vec2) {
        let list_x = self.position.x + 15.0;
        let mut y = self.position.y + 40.0;
        let item_height = 20.0;
        let mut clicked_idx: Option<usize> = None;

        for (i, quest) in self.quests.iter().enumerate() {
            let item_rect = Rect::new(list_x, y, self.size.x - 30.0, item_height);
            let is_selected = self.selected_quest == Some(i);
            let is_hovered = item_rect.contains(mouse_pos);

            // 高亮背景
            if is_selected || is_hovered {
                let color = if is_selected {
                    Color::from_rgba(60, 80, 100, 150)
                } else {
                    Color::from_rgba(50, 50, 60, 100)
                };
                draw_rectangle(item_rect.x, item_rect.y, item_rect.w, item_rect.h, color);
            }

            // 任务名称
            let name_color = match quest.status {
                QuestStatus::Available => Color::from_rgba(255, 255, 100, 255),
                QuestStatus::Accepted => WHITE,
                QuestStatus::Completed => Color::from_rgba(100, 255, 100, 255),
                QuestStatus::Failed => Color::from_rgba(255, 100, 100, 255),
            };
            draw_text_cn(&quest.name, list_x + 5.0, y + 14.0, 12.0, name_color);

            // 进度
            if quest.status == QuestStatus::Accepted && quest.max_progress > 0 {
                let progress_text = format!("{}/{}", quest.progress, quest.max_progress);
                draw_text_cn(&progress_text, self.position.x + self.size.x - 60.0, y + 14.0, 11.0, GRAY);
            }

            if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
                clicked_idx = Some(i);
            }

            y += item_height;
            
            // 防止超出对话框底部（留空间给关闭按钮）
            if y > self.position.y + self.size.y - 50.0 {
                break;
            }
        }

        if let Some(idx) = clicked_idx {
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
