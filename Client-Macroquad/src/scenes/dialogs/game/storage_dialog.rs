// ============================================================================
// StorageDialogHybrid - NPC 仓库（对齐 C# StorageDialog）
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/NPCDialogs.cs:2239-2480
// - 背景：Prguse[586]
// - 10x8 格子（每页 80 格，共 2 页 = 160 格）
// - 页签按钮：Title[743-744] (页 1)，Title[746] (页 2)
// - 关闭按钮：Prguse2[360-362]
// - 租用扩展按钮：Title[483-485]
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::*;

// ============================================================================
// 常量
// ============================================================================

/// 每页列数
const STORAGE_COLS: usize = 10;
/// 每页行数
const STORAGE_ROWS: usize = 8;
/// 每页格子数
const SLOTS_PER_PAGE: usize = STORAGE_COLS * STORAGE_ROWS;
/// 总格子数（2 页）
const TOTAL_SLOTS: usize = SLOTS_PER_PAGE * 2;
/// 格子尺寸
const CELL_SIZE: f32 = 32.0;
/// 格子 X 间距
const CELL_SPACING_X: f32 = 37.0;
/// 格子 Y 间距
const CELL_SPACING_Y: f32 = 33.0;
/// 格子起始偏移
const CELL_OFFSET_X: f32 = 9.0;
const CELL_OFFSET_Y: f32 = 60.0;

// ============================================================================
// 类型定义
// ============================================================================

/// 仓库物品
#[derive(Debug, Clone)]
pub struct StorageItem {
    pub icon_index: usize,
    pub name: String,
    pub count: u32,
}

/// 仓库操作事件
#[derive(Debug, Clone)]
pub enum StorageAction {
    /// 点击物品槽位（取出物品）
    ClickSlot { slot: usize },
    /// 切换页
    SwitchPage { page: usize },
    /// 关闭
    Close,
}

/// NPC 仓库对话框
pub struct StorageDialogHybrid {
    /// 是否可见
    visible: bool,
    /// 窗口位置
    position: Vec2,
    /// 当前页 (0 或 1)
    current_page: usize,
    /// 是否有扩展仓库
    has_expanded: bool,

    // === 数据 ===
    items: Vec<Option<StorageItem>>,

    // === 纹理 ===
    bg_texture: BackgroundTexture,
    page1_btn: ButtonTextures,
    page2_btn: ButtonTextures,
    close_btn: ButtonTextures,

    // === 拖动 ===
    drag_helper: DragHelper,

    // === 交互 ===
    hovered_slot: Option<usize>,
}

impl StorageDialogHybrid {
    pub fn new() -> Self {
        let mut items = Vec::with_capacity(TOTAL_SLOTS);
        items.resize_with(TOTAL_SLOTS, || None);

        Self {
            visible: false,
            position: vec2(0.0, 0.0),
            current_page: 0,
            has_expanded: false,

            items,

            bg_texture: BackgroundTexture::new(),
            page1_btn: ButtonTextures::new(),
            page2_btn: ButtonTextures::new(),
            close_btn: ButtonTextures::new(),

            drag_helper: DragHelper::new(),

            hovered_slot: None,
        }
    }

    /// 加载纹理
    pub fn load_textures(&mut self) {
        println!("📦 StorageDialog: 加载纹理...");

        self.bg_texture = BackgroundTexture::load(LibraryName::Prguse, 586, None);
        self.page1_btn = ButtonTextures::load_from_indices(LibraryName::Title, [743, 743, 744]);
        self.page2_btn = ButtonTextures::load_from_indices(LibraryName::Title, [746, 746, 746]);
        self.close_btn = ButtonTextures::load_from_indices(LibraryName::Prguse2, [360, 361, 362]);

        println!("  ✅ 仓库对话框纹理加载完成");
    }

    // === 公共 API ===

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn current_page(&self) -> usize {
        self.current_page
    }

    /// 设置物品
    pub fn set_item(&mut self, slot: usize, item: Option<StorageItem>) {
        if slot < TOTAL_SLOTS {
            self.items[slot] = item;
        }
    }

    /// 清空所有物品
    pub fn clear(&mut self) {
        for item in &mut self.items {
            *item = None;
        }
    }

    /// 设置扩展仓库
    pub fn set_expanded(&mut self, expanded: bool) {
        self.has_expanded = expanded;
    }

    // === 绘制 ===

    pub fn draw(&mut self) -> Option<StorageAction> {
        if !self.visible {
            return None;
        }

        let mouse_pos = vec2(mouse_position().0, mouse_position().1);
        let mut action = None;

        // 窗口拖动
        let bg_size = self.bg_texture.size;
        let drag_rect = Rect::new(self.position.x, self.position.y, bg_size.x, bg_size.y);
        self.position = self.drag_helper.update(drag_rect, self.position, mouse_pos);
        let pos = self.position;

        // 背景
        self.bg_texture.draw(pos);

        // 标题
        draw_text_cn("仓库", pos.x + 170.0, pos.y + 22.0, 14.0, WHITE);

        // 页签按钮
        let p1_rect = Rect::new(pos.x + 8.0, pos.y + 36.0, 60.0, 20.0);
        let p2_rect = Rect::new(pos.x + 80.0, pos.y + 36.0, 60.0, 20.0);

        let p1_state = ButtonState::from_mouse(p1_rect, mouse_pos);
        let p2_state = ButtonState::from_mouse(p2_rect, mouse_pos);
        self.page1_btn.draw(vec2(pos.x + 8.0, pos.y + 36.0), p1_state);
        self.page2_btn.draw(vec2(pos.x + 80.0, pos.y + 36.0), p2_state);

        if ButtonState::is_clicked(p1_rect, mouse_pos) {
            self.current_page = 0;
            action = Some(StorageAction::SwitchPage { page: 0 });
        }
        if ButtonState::is_clicked(p2_rect, mouse_pos) && self.has_expanded {
            self.current_page = 1;
            action = Some(StorageAction::SwitchPage { page: 1 });
        }

        // 当前页标识
        let indicator_x = if self.current_page == 0 { pos.x + 38.0 } else { pos.x + 110.0 };
        draw_text_cn("▼", indicator_x, pos.y + 54.0, 8.0, YELLOW);

        // 物品格子
        self.hovered_slot = None;
        let page_offset = self.current_page * SLOTS_PER_PAGE;

        for row in 0..STORAGE_ROWS {
            for col in 0..STORAGE_COLS {
                let idx = page_offset + row * STORAGE_COLS + col;
                let cell_x = pos.x + CELL_OFFSET_X + col as f32 * CELL_SPACING_X;
                let cell_y = pos.y + CELL_OFFSET_Y + row as f32 * CELL_SPACING_Y;
                let cell_rect = Rect::new(cell_x, cell_y, CELL_SIZE, CELL_SIZE);

                // 格子边框
                draw_rectangle_lines(cell_x, cell_y, CELL_SIZE, CELL_SIZE, 1.0, Color::new(0.3, 0.3, 0.3, 0.5));

                if let Some(Some(item)) = self.items.get(idx) {
                    // 物品图标
                    if let Some(info) = LibraryName::Items.get_texture(item.icon_index) {
                        if let Some(tex) = &info.image {
                            draw_texture_ex(
                                tex,
                                cell_x, cell_y,
                                WHITE,
                                DrawTextureParams {
                                    dest_size: Some(vec2(CELL_SIZE, CELL_SIZE)),
                                    ..Default::default()
                                },
                            );
                        }
                    }

                    // 数量
                    if item.count > 1 {
                        let count_text = format!("{}", item.count);
                        draw_text_cn(&count_text, cell_x + 2.0, cell_y + CELL_SIZE - 2.0, 8.0, YELLOW);
                    }
                }

                // 悬停
                if cell_rect.contains(mouse_pos) {
                    self.hovered_slot = Some(idx);
                    draw_rectangle(cell_x, cell_y, CELL_SIZE, CELL_SIZE, Color::new(1.0, 1.0, 1.0, 0.12));

                    if is_mouse_button_pressed(MouseButton::Left) {
                        action = Some(StorageAction::ClickSlot { slot: idx });
                    }
                }
            }
        }

        // 关闭按钮
        let close_x = pos.x + 363.0;
        let close_y = pos.y + 3.0;
        let close_rect = Rect::new(close_x, close_y, 20.0, 20.0);
        let close_state = ButtonState::from_mouse(close_rect, mouse_pos);
        self.close_btn.draw(vec2(close_x, close_y), close_state);
        if ButtonState::is_clicked(close_rect, mouse_pos) {
            action = Some(StorageAction::Close);
        }

        // 工具提示
        if let Some(idx) = self.hovered_slot {
            if let Some(Some(item)) = self.items.get(idx) {
                let tooltip = if item.count > 1 {
                    format!("{}\n数量: {}", item.name, item.count)
                } else {
                    item.name.clone()
                };
                let tip_x = mouse_pos.x + 15.0;
                let tip_y = mouse_pos.y + 15.0;
                let lines: Vec<&str> = tooltip.lines().collect();
                let tip_w = 160.0;
                let tip_h = lines.len() as f32 * 16.0 + 8.0;

                draw_rectangle(tip_x, tip_y, tip_w, tip_h, Color::new(0.0, 0.0, 0.0, 0.85));
                draw_rectangle_lines(tip_x, tip_y, tip_w, tip_h, 1.0, Color::new(0.6, 0.6, 0.6, 0.8));
                for (j, line) in lines.iter().enumerate() {
                    draw_text_cn(line, tip_x + 6.0, tip_y + 14.0 + j as f32 * 16.0, 12.0, WHITE);
                }
            }
        }

        action
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_basic() {
        let mut dialog = StorageDialogHybrid::new();
        assert!(!dialog.visible());
        assert_eq!(dialog.current_page(), 0);
        assert_eq!(dialog.items.len(), TOTAL_SLOTS);
    }

    #[test]
    fn test_storage_set_items() {
        let mut dialog = StorageDialogHybrid::new();
        dialog.set_item(0, Some(StorageItem { icon_index: 1, name: "药品".into(), count: 10 }));
        assert!(dialog.items[0].is_some());

        dialog.set_item(TOTAL_SLOTS - 1, Some(StorageItem { icon_index: 2, name: "武器".into(), count: 1 }));
        assert!(dialog.items[TOTAL_SLOTS - 1].is_some());

        // Out of bounds
        dialog.set_item(TOTAL_SLOTS + 1, Some(StorageItem { icon_index: 0, name: "x".into(), count: 1 }));

        dialog.clear();
        assert!(dialog.items[0].is_none());
    }
}
