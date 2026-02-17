// ============================================================================
// CraftDialogHybrid - 合成/精炼/镶嵌对话框（对齐 C# NPCDialogs.cs）
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/NPCDialogs.cs (CraftDialog, RefineDialog, SocketDialog)
// - CraftDialog: 配方列表，材料格子（3×3），合成按钮，结果预览
// - RefineDialog: 目标物品槽 + 精炼材料槽 + 金币消耗，精炼按钮
// - SocketDialog: 武器槽 + 宝石槽（3），镶嵌/拆卸按钮
//
// ============================================================================

use macroquad::prelude::*;
use super::native_ui_utils::*;

// ============================================================================
// 常量
// ============================================================================

const CRAFT_WIDTH: f32 = 300.0;
const CRAFT_HEIGHT: f32 = 360.0;
const REFINE_WIDTH: f32 = 240.0;
const REFINE_HEIGHT: f32 = 220.0;
const SOCKET_WIDTH: f32 = 260.0;
const SOCKET_HEIGHT: f32 = 240.0;
const CELL_SIZE: f32 = 34.0;
const CELL_GAP: f32 = 2.0;
const MATERIAL_COLS: usize = 3;
const MATERIAL_ROWS: usize = 3;
const GEM_SLOTS: usize = 3;
const RECIPE_LIST_ROWS: usize = 8;
const ROW_HEIGHT: f32 = 20.0;

// ============================================================================
// 类型定义
// ============================================================================

/// 合成配方
#[derive(Debug, Clone)]
pub struct CraftRecipe {
    pub id: u32,
    pub name: String,
    pub materials: Vec<(u32, u32)>,
    pub result_name: String,
}

impl CraftRecipe {
    pub fn new(id: u32, name: &str, materials: Vec<(u32, u32)>, result_name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            materials,
            result_name: result_name.to_string(),
        }
    }

    /// 材料种类数
    pub fn material_count(&self) -> usize {
        self.materials.len()
    }
}

/// 合成/精炼/镶嵌操作事件
#[derive(Debug, Clone, PartialEq)]
pub enum CraftAction {
    /// 选择配方
    SelectRecipe(u32),
    /// 合成
    Craft,
    /// 精炼
    Refine,
    /// 镶嵌
    Socket,
    /// 拆卸宝石
    Extract,
    /// 关闭
    Close,
}

// ============================================================================
// CraftDialogHybrid
// ============================================================================

/// 合成对话框
pub struct CraftDialogHybrid {
    pub visible: bool,
    pub recipes: Vec<CraftRecipe>,
    pub selected_recipe: Option<usize>,
    pub material_slots: Vec<Option<usize>>,
    position: Vec2,
    drag_helper: DragHelper,
}

impl CraftDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            recipes: Vec::new(),
            selected_recipe: None,
            material_slots: vec![None; MATERIAL_COLS * MATERIAL_ROWS],
            position: Vec2::new(250.0, 100.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 绘制并处理输入
    pub fn draw(&mut self) -> Option<CraftAction> {
        if !self.visible {
            return None;
        }

        let mouse = mouse_pos();
        let mut action = None;

        let title_rect = Rect::new(self.position.x, self.position.y, CRAFT_WIDTH, 22.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        // 背景
        draw_rectangle(x, y, CRAFT_WIDTH, CRAFT_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.85));
        draw_rectangle_lines(x, y, CRAFT_WIDTH, CRAFT_HEIGHT, 1.0, DARKGRAY);
        draw_text("合成", x + 10.0, y + 16.0, 14.0, GOLD);

        // 配方列表
        draw_text("配方:", x + 10.0, y + 36.0, 12.0, WHITE);
        let list_y = y + 42.0;
        for (i, recipe) in self.recipes.iter().enumerate().take(RECIPE_LIST_ROWS) {
            let row_y = list_y + i as f32 * ROW_HEIGHT;
            let row_rect = Rect::new(x + 10.0, row_y, 130.0, ROW_HEIGHT);

            let bg_color = if self.selected_recipe == Some(i) {
                Color::new(0.3, 0.3, 0.5, 0.6)
            } else if row_rect.contains(mouse) {
                Color::new(0.2, 0.2, 0.3, 0.4)
            } else {
                Color::new(0.0, 0.0, 0.0, 0.0)
            };
            draw_rectangle(row_rect.x, row_rect.y, row_rect.w, row_rect.h, bg_color);
            draw_text(&recipe.name, x + 14.0, row_y + 14.0, 11.0, WHITE);

            if is_mouse_over(row_rect) && is_mouse_button_pressed(MouseButton::Left) {
                self.selected_recipe = Some(i);
                action = Some(CraftAction::SelectRecipe(recipe.id));
            }
        }

        // 材料格子 (3x3)
        let grid_x = x + 160.0;
        let grid_y = y + 42.0;
        draw_text("材料:", grid_x, grid_y - 6.0, 11.0, WHITE);
        for row in 0..MATERIAL_ROWS {
            for col in 0..MATERIAL_COLS {
                let slot = row * MATERIAL_COLS + col;
                let cell_x = grid_x + col as f32 * (CELL_SIZE + CELL_GAP);
                let cell_y = grid_y + row as f32 * (CELL_SIZE + CELL_GAP);
                let cell_rect = Rect::new(cell_x, cell_y, CELL_SIZE, CELL_SIZE);

                let highlight = if cell_rect.contains(mouse) {
                    CellHighlight::Hovered
                } else {
                    CellHighlight::None
                };
                draw_cell_frame(cell_rect, highlight, &CellStyle::default());

                if self.material_slots[slot].is_some() {
                    draw_rectangle(cell_x + 4.0, cell_y + 4.0, CELL_SIZE - 8.0, CELL_SIZE - 8.0, Color::new(0.4, 0.5, 0.3, 0.5));
                }
            }
        }

        // 结果预览
        if let Some(idx) = self.selected_recipe {
            if let Some(recipe) = self.recipes.get(idx) {
                let result_y = grid_y + MATERIAL_ROWS as f32 * (CELL_SIZE + CELL_GAP) + 10.0;
                draw_text("结果:", grid_x, result_y, 11.0, GOLD);
                let result_rect = Rect::new(grid_x, result_y + 14.0, CELL_SIZE, CELL_SIZE);
                draw_cell_frame(result_rect, CellHighlight::None, &CellStyle::default());
                draw_text(&recipe.result_name, grid_x + CELL_SIZE + 6.0, result_y + 28.0, 11.0, WHITE);
            }
        }

        // 合成按钮
        let btn_y = y + CRAFT_HEIGHT - 32.0;
        let craft_rect = Rect::new(x + 100.0, btn_y, 60.0, 20.0);
        draw_rectangle_lines(craft_rect.x, craft_rect.y, craft_rect.w, craft_rect.h, 1.0, GRAY);
        draw_text("合成", craft_rect.x + 16.0, craft_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(craft_rect) && is_mouse_button_pressed(MouseButton::Left) {
            action = Some(CraftAction::Craft);
        }

        // 关闭
        let close_rect = Rect::new(x + CRAFT_WIDTH - 20.0, y + 2.0, 16.0, 16.0);
        draw_text("X", close_rect.x + 3.0, close_rect.y + 12.0, 12.0, GRAY);
        if is_mouse_over(close_rect) && is_mouse_button_pressed(MouseButton::Left) {
            self.visible = false;
            action = Some(CraftAction::Close);
        }

        action
    }
}

// ============================================================================
// RefineDialogHybrid
// ============================================================================

/// 精炼对话框
pub struct RefineDialogHybrid {
    pub visible: bool,
    pub target_item: Option<usize>,
    pub material_item: Option<usize>,
    pub gold_cost: u64,
    position: Vec2,
    drag_helper: DragHelper,
}

impl RefineDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            target_item: None,
            material_item: None,
            gold_cost: 0,
            position: Vec2::new(280.0, 150.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 绘制并处理输入
    pub fn draw(&mut self) -> Option<CraftAction> {
        if !self.visible {
            return None;
        }

        let mouse = mouse_pos();
        let mut action = None;

        let title_rect = Rect::new(self.position.x, self.position.y, REFINE_WIDTH, 22.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        // 背景
        draw_rectangle(x, y, REFINE_WIDTH, REFINE_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.85));
        draw_rectangle_lines(x, y, REFINE_WIDTH, REFINE_HEIGHT, 1.0, DARKGRAY);
        draw_text("精炼", x + 10.0, y + 16.0, 14.0, GOLD);

        // 目标物品槽
        draw_text("目标:", x + 10.0, y + 42.0, 11.0, WHITE);
        let target_rect = Rect::new(x + 60.0, y + 30.0, CELL_SIZE, CELL_SIZE);
        let target_hl = if target_rect.contains(mouse) { CellHighlight::Hovered } else { CellHighlight::None };
        draw_cell_frame(target_rect, target_hl, &CellStyle::default());
        if self.target_item.is_some() {
            draw_rectangle(target_rect.x + 4.0, target_rect.y + 4.0, CELL_SIZE - 8.0, CELL_SIZE - 8.0, Color::new(0.5, 0.4, 0.2, 0.6));
        }

        // 精炼材料槽
        draw_text("材料:", x + 10.0, y + 82.0, 11.0, WHITE);
        let material_rect = Rect::new(x + 60.0, y + 70.0, CELL_SIZE, CELL_SIZE);
        let mat_hl = if material_rect.contains(mouse) { CellHighlight::Hovered } else { CellHighlight::None };
        draw_cell_frame(material_rect, mat_hl, &CellStyle::default());
        if self.material_item.is_some() {
            draw_rectangle(material_rect.x + 4.0, material_rect.y + 4.0, CELL_SIZE - 8.0, CELL_SIZE - 8.0, Color::new(0.3, 0.5, 0.3, 0.6));
        }

        // 金币消耗
        draw_text(&format!("金币: {}", self.gold_cost), x + 10.0, y + 126.0, 11.0, GOLD);

        // 精炼按钮
        let btn_y = y + REFINE_HEIGHT - 32.0;
        let refine_rect = Rect::new(x + 70.0, btn_y, 60.0, 20.0);
        draw_rectangle_lines(refine_rect.x, refine_rect.y, refine_rect.w, refine_rect.h, 1.0, GRAY);
        draw_text("精炼", refine_rect.x + 16.0, refine_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(refine_rect) && is_mouse_button_pressed(MouseButton::Left) {
            action = Some(CraftAction::Refine);
        }

        // 关闭
        let close_rect = Rect::new(x + REFINE_WIDTH - 20.0, y + 2.0, 16.0, 16.0);
        draw_text("X", close_rect.x + 3.0, close_rect.y + 12.0, 12.0, GRAY);
        if is_mouse_over(close_rect) && is_mouse_button_pressed(MouseButton::Left) {
            self.visible = false;
            action = Some(CraftAction::Close);
        }

        action
    }
}

// ============================================================================
// SocketDialogHybrid
// ============================================================================

/// 镶嵌对话框
pub struct SocketDialogHybrid {
    pub visible: bool,
    pub weapon_slot: Option<usize>,
    pub gem_slots: Vec<Option<usize>>,
    position: Vec2,
    drag_helper: DragHelper,
}

impl SocketDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            weapon_slot: None,
            gem_slots: vec![None; GEM_SLOTS],
            position: Vec2::new(290.0, 160.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 绘制并处理输入
    pub fn draw(&mut self) -> Option<CraftAction> {
        if !self.visible {
            return None;
        }

        let mouse = mouse_pos();
        let mut action = None;

        let title_rect = Rect::new(self.position.x, self.position.y, SOCKET_WIDTH, 22.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        // 背景
        draw_rectangle(x, y, SOCKET_WIDTH, SOCKET_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.85));
        draw_rectangle_lines(x, y, SOCKET_WIDTH, SOCKET_HEIGHT, 1.0, DARKGRAY);
        draw_text("镶嵌", x + 10.0, y + 16.0, 14.0, GOLD);

        // 武器槽
        draw_text("武器:", x + 10.0, y + 44.0, 11.0, WHITE);
        let weapon_rect = Rect::new(x + 60.0, y + 32.0, CELL_SIZE, CELL_SIZE);
        let weapon_hl = if weapon_rect.contains(mouse) { CellHighlight::Hovered } else { CellHighlight::None };
        draw_cell_frame(weapon_rect, weapon_hl, &CellStyle::default());
        if self.weapon_slot.is_some() {
            draw_rectangle(weapon_rect.x + 4.0, weapon_rect.y + 4.0, CELL_SIZE - 8.0, CELL_SIZE - 8.0, Color::new(0.5, 0.3, 0.2, 0.6));
        }

        // 宝石槽 (3个)
        draw_text("宝石:", x + 10.0, y + 84.0, 11.0, WHITE);
        for i in 0..GEM_SLOTS {
            let gem_x = x + 60.0 + i as f32 * (CELL_SIZE + CELL_GAP + 4.0);
            let gem_y = y + 72.0;
            let gem_rect = Rect::new(gem_x, gem_y, CELL_SIZE, CELL_SIZE);
            let gem_hl = if gem_rect.contains(mouse) { CellHighlight::Hovered } else { CellHighlight::None };
            draw_cell_frame(gem_rect, gem_hl, &CellStyle::default());

            if self.gem_slots[i].is_some() {
                draw_rectangle(gem_x + 4.0, gem_y + 4.0, CELL_SIZE - 8.0, CELL_SIZE - 8.0, Color::new(0.2, 0.4, 0.6, 0.6));
            }
        }

        // 操作按钮
        let btn_y = y + SOCKET_HEIGHT - 32.0;

        let socket_rect = Rect::new(x + 30.0, btn_y, 60.0, 20.0);
        draw_rectangle_lines(socket_rect.x, socket_rect.y, socket_rect.w, socket_rect.h, 1.0, GRAY);
        draw_text("镶嵌", socket_rect.x + 16.0, socket_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(socket_rect) && is_mouse_button_pressed(MouseButton::Left) {
            action = Some(CraftAction::Socket);
        }

        let extract_rect = Rect::new(x + 110.0, btn_y, 60.0, 20.0);
        draw_rectangle_lines(extract_rect.x, extract_rect.y, extract_rect.w, extract_rect.h, 1.0, GRAY);
        draw_text("拆卸", extract_rect.x + 16.0, extract_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(extract_rect) && is_mouse_button_pressed(MouseButton::Left) {
            action = Some(CraftAction::Extract);
        }

        // 关闭
        let close_rect = Rect::new(x + SOCKET_WIDTH - 20.0, y + 2.0, 16.0, 16.0);
        draw_text("X", close_rect.x + 3.0, close_rect.y + 12.0, 12.0, GRAY);
        if is_mouse_over(close_rect) && is_mouse_button_pressed(MouseButton::Left) {
            self.visible = false;
            action = Some(CraftAction::Close);
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
    fn test_craft_recipe_creation() {
        let recipe = CraftRecipe::new(
            1,
            "Iron Sword",
            vec![(100, 2), (101, 1)],
            "Iron Sword +1",
        );
        assert_eq!(recipe.id, 1);
        assert_eq!(recipe.material_count(), 2);
        assert_eq!(recipe.result_name, "Iron Sword +1");
    }

    #[test]
    fn test_craft_dialog_creation() {
        let dialog = CraftDialogHybrid::new();
        assert!(!dialog.visible);
        assert!(dialog.recipes.is_empty());
        assert_eq!(dialog.material_slots.len(), MATERIAL_COLS * MATERIAL_ROWS);
    }

    #[test]
    fn test_socket_dialog_creation() {
        let dialog = SocketDialogHybrid::new();
        assert!(!dialog.visible);
        assert!(dialog.weapon_slot.is_none());
        assert_eq!(dialog.gem_slots.len(), GEM_SLOTS);
        assert!(dialog.gem_slots.iter().all(|g| g.is_none()));
    }
}
