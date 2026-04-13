// ============================================================================
// CraftDialogHybrid - 合成对话框
// ============================================================================
//
// C# 参考: Client/MIRScenes/Dialogs/NPCDialogs.cs -> CraftDialog (~1700 行)
// - 显示配方名称
// - 材料槽位网格（3 工具 + 6 原料 = 9 格）
// - 一键填充（AutoFill）：从背包自动匹配材料
// - 确认合成按钮
//
// ============================================================================

use macroquad::prelude::*;
use crate::ui::text_renderer::draw_text_cn;
use mir2_shared::data::item::UserItem;

/// 材料槽位
#[derive(Debug, Clone)]
pub struct CraftSlot {
    /// 需求的材料物品（影子物品，显示需要什么）
    pub shadow_item: Option<UserItem>,
    /// 玩家放入的实际物品
    pub filled_item: Option<UserItem>,
    /// 填入的背包槽位索引
    pub inventory_slot: Option<i32>,
}

/// 合成配方
#[derive(Debug, Clone)]
pub struct CraftRecipe {
    pub name: String,
    /// 配方物品的 unique_id（用于发包）
    pub recipe_unique_id: u64,
    /// 材料需求列表（来自服务器）
    pub materials: Vec<UserItem>,
}

/// 合成结果（供上层发包）
pub struct CraftResult {
    pub recipe_unique_id: u64,
    pub count: u16,
    pub slots: Vec<i32>,
}

const TOOL_COUNT: usize = 3;
const INGREDIENT_COUNT: usize = 6;
const TOTAL_SLOTS: usize = TOOL_COUNT + INGREDIENT_COUNT;

/// 合成对话框
pub struct CraftDialogHybrid {
    pub visible: bool,
    pub recipe: Option<CraftRecipe>,
    pub slots: [CraftSlot; TOTAL_SLOTS],
    /// 是否正在从背包选择材料
    selecting_slot: Option<usize>,
    /// 背包快照（用于一键填充）
    inventory_snapshot: Vec<(i32, UserItem)>,
}

impl Default for CraftDialogHybrid {
    fn default() -> Self {
        Self {
            visible: false,
            recipe: None,
            slots: std::array::from_fn(|_| CraftSlot {
                shadow_item: None,
                filled_item: None,
                inventory_slot: None,
            }),
            selecting_slot: None,
            inventory_snapshot: Vec::new(),
        }
    }
}

impl CraftDialogHybrid {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, recipes: Vec<CraftRecipe>) {
        if let Some(recipe) = recipes.into_iter().next() {
            self.recipe = Some(recipe.clone());
            // 设置材料槽位的影子物品
            for (i, slot) in self.slots.iter_mut().enumerate() {
                slot.shadow_item = recipe.materials.get(i).cloned();
                slot.filled_item = None;
                slot.inventory_slot = None;
            }
        }
        self.selecting_slot = None;
        self.visible = true;
    }

    /// 设置背包快照（用于一键填充）
    pub fn set_inventory_snapshot(&mut self, inventory: Vec<(i32, UserItem)>) {
        self.inventory_snapshot = inventory;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.reset_cells();
    }

    fn reset_cells(&mut self) {
        for slot in self.slots.iter_mut() {
            slot.filled_item = None;
            slot.inventory_slot = None;
        }
    }

    /// 一键填充（AutoFill）：模拟从背包匹配材料
    /// 上层需要传入背包物品列表和对应的槽位索引
    pub fn auto_fill(&mut self, inventory: &[(i32, UserItem)]) {
        self.reset_cells();
        let mut used_slots = std::collections::HashSet::new();

        // 遍历所有材料槽位
        for (slot_idx, slot) in self.slots.iter_mut().enumerate() {
            let Some(shadow) = &slot.shadow_item else { continue };
            let Some(shadow_info) = &shadow.info else { continue };

            // 在背包中查找匹配的物品
            for &(inv_slot, ref item) in inventory.iter() {
                if used_slots.contains(&inv_slot) { continue; }
                let Some(item_info) = &item.info else { continue; };

                if item_info.index != shadow_info.index { continue; }

                // 工具类需要耐久度 >= 1000
                if slot_idx < TOOL_COUNT && item.current_dura < 1000 {
                    continue;
                }

                // 原料类需要数量足够
                if slot_idx >= TOOL_COUNT && item.count < shadow.count {
                    continue;
                }

                // 匹配成功
                slot.filled_item = Some(item.clone());
                slot.inventory_slot = Some(inv_slot);
                used_slots.insert(inv_slot);
                break;
            }
        }
    }

    /// 检查是否所有有需求（shadow_item）的槽位都已填充
    fn all_slots_filled(&self) -> bool {
        self.slots.iter().all(|s| s.shadow_item.is_none() || s.filled_item.is_some())
    }

    /// 获取合成结果数据（供发包）
    fn get_craft_data(&self) -> Option<CraftResult> {
        let recipe = self.recipe.as_ref()?;
        if !self.all_slots_filled() {
            return None;
        }

        let slots: Vec<i32> = self.slots.iter()
            .filter_map(|s| s.inventory_slot)
            .collect();

        Some(CraftResult {
            recipe_unique_id: recipe.recipe_unique_id,
            count: 1,
            slots,
        })
    }

    /// 设置材料数据（从服务器配方更新）
    pub fn update_materials(&mut self, materials: Vec<UserItem>) {
        for (i, slot) in self.slots.iter_mut().enumerate() {
            slot.shadow_item = materials.get(i).cloned();
        }
        if let Some(ref mut recipe) = self.recipe {
            recipe.materials = materials;
        }
    }

    /// 点击材料槽位
    pub fn click_slot(&mut self, slot_idx: usize) {
        if self.slots[slot_idx].filled_item.is_some() {
            // 已填充的槽位：清空
            self.slots[slot_idx].filled_item = None;
            self.slots[slot_idx].inventory_slot = None;
        } else {
            // 未填充的槽位：标记为待选择
            self.selecting_slot = Some(slot_idx);
        }
    }

    /// 检查是否正在等待选择材料
    pub fn is_selecting_material(&self) -> bool {
        self.selecting_slot.is_some()
    }

    /// 获取待选择的槽位索引
    pub fn selecting_slot_index(&self) -> Option<usize> {
        self.selecting_slot
    }

    /// 从背包选择物品填入槽位
    pub fn fill_slot_from_inventory(&mut self, inv_slot: i32, item: UserItem) {
        if let Some(slot_idx) = self.selecting_slot.take() {
            let shadow = &self.slots[slot_idx].shadow_item;
            // 验证物品是否匹配
            if let (Some(shadow_item), Some(item_info), Some(shadow_info)) =
                (shadow.as_ref(), item.info.as_ref(), shadow.as_ref().and_then(|s| s.info.as_ref())) {
                if item_info.index == shadow_info.index {
                    // 工具类检查耐久度
                    if slot_idx < TOOL_COUNT && item.current_dura < 1000 {
                        return;
                    }
                    // 原料类检查数量
                    if slot_idx >= TOOL_COUNT && item.count < shadow_item.count {
                        return;
                    }
                    self.slots[slot_idx].filled_item = Some(item);
                    self.slots[slot_idx].inventory_slot = Some(inv_slot);
                }
            }
        }
    }

    /// 绘制并返回合成结果（玩家点击合成按钮时）
    pub fn draw(&mut self, screen_w: f32, screen_h: f32, mouse_pos: Vec2,
                _mouse_wheel: f32, left_clicked: bool) -> Option<CraftResult> {
        if !self.visible {
            return None;
        }

        let padding = 15.0;
        let title_h = 30.0;
        let slot_size = 40.0;
        let slot_gap = 4.0;
        let btn_h = 28.0;
        let btn_w = 80.0;
        let dialog_w = 380.0;

        // 计算高度：标题 + 配方名 + 工具行(3格) + 原料行(6格) + 按钮行
        let tool_row_w = TOOL_COUNT as f32 * (slot_size + slot_gap) - slot_gap;
        let ingredient_row_w = INGREDIENT_COUNT as f32 * (slot_size + slot_gap) - slot_gap;
        let grid_w = tool_row_w.max(ingredient_row_w);
        let grid_h = 2.0 * slot_size + slot_gap; // 两行
        let dialog_h = title_h + 30.0 + 20.0 + grid_h + btn_h + padding * 4.0;

        let dialog_x = (screen_w - dialog_w) / 2.0;
        let dialog_y = (screen_h - dialog_h) / 2.0;

        // 背景
        draw_rectangle(dialog_x, dialog_y, dialog_w, dialog_h, Color::from_rgba(25, 25, 35, 230));

        // 标题
        draw_text_cn("合成", dialog_x + 15.0, dialog_y + 10.0, 16.0,
            Color::from_rgba(255, 220, 100, 255));

        // 配方名称
        let recipe_name = self.recipe.as_ref().map(|r| r.name.as_str()).unwrap_or("未选择配方");
        draw_text_cn(recipe_name, dialog_x + 15.0, dialog_y + title_h + 10.0, 14.0,
            Color::from_rgba(200, 200, 200, 255));

        // 材料槽位网格
        let grid_start_x = dialog_x + (dialog_w - grid_w) / 2.0;
        let grid_start_y = dialog_y + title_h + 40.0;

        // 工具行（前3格）
        let tool_label = "工具:";
        draw_text_cn(tool_label, dialog_x + 10.0, grid_start_y + 5.0, 12.0,
            Color::from_rgba(150, 200, 150, 255));
        for i in 0..TOOL_COUNT {
            let slot_x = grid_start_x + i as f32 * (slot_size + slot_gap);
            let slot_y = grid_start_y;
            self.draw_slot(i, slot_x, slot_y, slot_size, mouse_pos, left_clicked);
        }

        // 原料行（后6格，分两行显示，每行3格）
        let ingredient_label = "材料:";
        let ingredient_start_y = grid_start_y + slot_size + slot_gap + 10.0;
        draw_text_cn(ingredient_label, dialog_x + 10.0, ingredient_start_y + 5.0, 12.0,
            Color::from_rgba(200, 150, 100, 255));
        for i in 0..INGREDIENT_COUNT {
            let row = i / 3;
            let col = i % 3;
            let slot_x = grid_start_x + col as f32 * (slot_size + slot_gap);
            let slot_y = ingredient_start_y + row as f32 * (slot_size + slot_gap);
            self.draw_slot(TOOL_COUNT + i, slot_x, slot_y, slot_size, mouse_pos, left_clicked);
        }

        // 按钮行
        let btn_y = ingredient_start_y + 2.0 * (slot_size + slot_gap) + padding;

        // 一键填充按钮
        let autofill_x = dialog_x + padding;
        let autofill_hover = mouse_pos.x >= autofill_x && mouse_pos.x <= autofill_x + btn_w
            && mouse_pos.y >= btn_y && mouse_pos.y <= btn_y + btn_h;
        draw_rectangle(autofill_x, btn_y, btn_w, btn_h,
            if autofill_hover { Color::from_rgba(80, 80, 40, 255) } else { Color::from_rgba(50, 50, 30, 255) });
        draw_text_cn("一键填充", autofill_x + 10.0, btn_y + 7.0, 14.0, WHITE);

        if left_clicked && autofill_hover {
            let snapshot = self.inventory_snapshot.clone();
            self.auto_fill(&snapshot);
        }

        // 合成按钮
        let craft_x = autofill_x + btn_w + 10.0;
        let can_craft = self.all_slots_filled();
        let craft_hover = mouse_pos.x >= craft_x && mouse_pos.x <= craft_x + btn_w
            && mouse_pos.y >= btn_y && mouse_pos.y <= btn_y + btn_h;
        draw_rectangle(craft_x, btn_y, btn_w, btn_h,
            if can_craft && craft_hover {
                Color::from_rgba(60, 120, 60, 255)
            } else if can_craft {
                Color::from_rgba(40, 100, 40, 255)
            } else {
                Color::from_rgba(40, 40, 40, 255)
            });
        draw_text_cn("合成", craft_x + 20.0, btn_y + 7.0, 14.0, WHITE);

        // 关闭按钮
        let close_x = craft_x + btn_w + 10.0;
        let close_hover = mouse_pos.x >= close_x && mouse_pos.x <= close_x + btn_w
            && mouse_pos.y >= btn_y && mouse_pos.y <= btn_y + btn_h;
        draw_rectangle(close_x, btn_y, btn_w, btn_h,
            if close_hover { Color::from_rgba(150, 50, 50, 255) } else { Color::from_rgba(100, 30, 30, 255) });
        draw_text_cn("关闭", close_x + 20.0, btn_y + 7.0, 14.0, WHITE);

        if left_clicked && close_hover {
            self.close();
        }

        // 合成按钮点击
        let mut craft_result = None;
        if left_clicked && can_craft && craft_hover {
            craft_result = self.get_craft_data();
        }

        craft_result
    }

    fn draw_slot(&mut self, idx: usize, x: f32, y: f32, size: f32,
                 mouse_pos: Vec2, left_clicked: bool) {
        let slot = &self.slots[idx];
        let is_selecting = self.selecting_slot == Some(idx);
        let is_filled = slot.filled_item.is_some();

        // 槽位背景
        let bg_color = if is_selecting {
            Color::from_rgba(80, 60, 20, 200)
        } else if is_filled {
            Color::from_rgba(40, 60, 40, 180)
        } else {
            Color::from_rgba(30, 30, 30, 150)
        };
        draw_rectangle(x, y, size, size, bg_color);

        // 槽位边框
        let border_color = if is_selecting {
            Color::from_rgba(255, 200, 50, 255)
        } else if is_filled {
            Color::from_rgba(100, 200, 100, 200)
        } else {
            Color::from_rgba(100, 100, 100, 150)
        };
        draw_line(x, y, x + size, y, 1.0, border_color);
        draw_line(x + size, y, x + size, y + size, 1.0, border_color);
        draw_line(x + size, y + size, x, y + size, 1.0, border_color);
        draw_line(x, y + size, x, y, 1.0, border_color);

        // 显示影子物品名称（需求）或填充物品名称
        let display_name = if is_filled {
            slot.filled_item.as_ref()
                .and_then(|item| item.info.as_ref())
                .map(|info| info.name.as_str())
                .unwrap_or("已填入")
        } else {
            slot.shadow_item.as_ref()
                .and_then(|item| item.info.as_ref())
                .map(|info| info.name.as_str())
                .unwrap_or("[空]")
        };

        let font_size = if is_filled { 10.0 } else { 9.0 };
        let text_color = if is_filled {
            Color::from_rgba(150, 255, 150, 255)
        } else {
            Color::from_rgba(150, 150, 150, 200)
        };
        draw_text_cn(display_name, x + 3.0, y + 5.0, font_size, text_color);

        // 显示数量
        if let Some(ref item) = slot.shadow_item {
            if !is_filled {
                draw_text_cn(&format!("需要: {}", item.count), x + 3.0, y + size - 10.0, 8.0,
                    Color::from_rgba(255, 150, 100, 200));
            }
        }

        // 点击槽位
        if left_clicked && mouse_pos.x >= x && mouse_pos.x <= x + size
            && mouse_pos.y >= y && mouse_pos.y <= y + size {
            self.click_slot(idx);
        }
    }
}
