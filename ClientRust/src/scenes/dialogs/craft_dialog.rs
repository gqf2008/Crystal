// Craft Dialog - 物品制作对话框
// 使用工具和材料制作物品

use mir2_shared::UserItem;
use std::collections::HashMap;

/// 工具槽数量
pub const TOOL_SLOT_COUNT: usize = 3;
/// 材料槽数量
pub const INGREDIENT_SLOT_COUNT: usize = 6;
/// 总槽位数
pub const TOTAL_SLOT_COUNT: usize = TOOL_SLOT_COUNT + INGREDIENT_SLOT_COUNT;

/// 制作配方信息
#[derive(Debug, Clone)]
pub struct ClientRecipeInfo {
    pub item: UserItem,              // 成品
    pub tools: Vec<UserItem>,        // 所需工具
    pub ingredients: Vec<UserItem>,  // 所需材料
    pub gold: u32,                   // 所需金币
    pub chance: i32,                 // 成功率 (%)
}

/// 物品制作对话框
/// 
/// 功能:
/// - 3个工具槽 + 6个材料槽
/// - 配方选择
/// - 自动填充
/// - 制作确认
#[derive(Debug, Clone)]
pub struct CraftDialog {
    /// 当前选中的配方物品
    pub recipe_item: Option<UserItem>,
    /// 配方信息
    pub recipe: Option<ClientRecipeInfo>,
    /// 槽位 (3工具 + 6材料 = 9)
    pub slots: [Option<UserItem>; TOTAL_SLOT_COUNT],
    /// 阴影物品 (显示需求)
    pub shadow_items: [Option<UserItem>; TOTAL_SLOT_COUNT],
    /// 已选择的物品 (cell_index -> unique_id)
    pub selected: HashMap<usize, u64>,
    /// 是否可见
    pub visible: bool,
    /// 制作按钮是否启用
    pub craft_button_enabled: bool,
}

impl Default for CraftDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl CraftDialog {
    /// 创建新的制作对话框
    pub fn new() -> Self {
        Self {
            recipe_item: None,
            recipe: None,
            slots: Default::default(),
            shadow_items: Default::default(),
            selected: HashMap::new(),
            visible: false,
            craft_button_enabled: false,
        }
    }

    /// 显示对话框
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// 隐藏对话框
    pub fn hide(&mut self) {
        self.visible = false;
        self.reset_cells(true);
    }

    /// 重置所有槽位
    pub fn reset_cells(&mut self, clear_recipe: bool) {
        if clear_recipe {
            self.recipe_item = None;
            self.recipe = None;
        }

        for slot in &mut self.slots {
            *slot = None;
        }
        for shadow in &mut self.shadow_items {
            *shadow = None;
        }

        self.selected.clear();
        self.craft_button_enabled = false;
    }

    /// 刷新配方槽位
    pub fn refresh_craft_cells(&mut self, recipe_item: UserItem, recipe: ClientRecipeInfo) {
        self.recipe_item = Some(recipe_item);
        self.craft_button_enabled = true;

        // 设置阴影物品 (显示需求)
        for (i, tool) in recipe.tools.iter().enumerate() {
            if i < TOOL_SLOT_COUNT {
                self.shadow_items[i] = Some(tool.clone());
                if self.slots[i].is_none() {
                    self.craft_button_enabled = false;
                }
            }
        }

        for (i, ingredient) in recipe.ingredients.iter().enumerate() {
            let slot_index = TOOL_SLOT_COUNT + i;
            if slot_index < TOTAL_SLOT_COUNT {
                self.shadow_items[slot_index] = Some(ingredient.clone());
                if self.slots[slot_index].is_none() {
                    self.craft_button_enabled = false;
                }
            }
        }

        self.recipe = Some(recipe);
    }

    /// 自动填充槽位 (从背包中查找材料)
    pub fn auto_fill(&mut self, inventory: &[Option<UserItem>]) {
        // 简化实现：实际应该从背包中查找匹配的材料
        // 这里只是示例逻辑
    }

    /// 检查是否有足够的材料制作
    pub fn has_craft_items(&self, count: u16) -> bool {
        for i in 0..TOTAL_SLOT_COUNT {
            if let Some(shadow) = &self.shadow_items[i] {
                if let Some(item) = &self.slots[i] {
                    if i >= TOOL_SLOT_COUNT {
                        // 材料槽：检查数量
                        if item.count < shadow.count * count as u32 {
                            return false;
                        }
                    } else {
                        // 工具槽：检查耐久度
                        if item.current_dura < 1000 * count as u32 {
                            return false;
                        }
                    }
                } else {
                    return false;
                }
            }
        }
        true
    }

    /// 获取指定槽位的物品
    pub fn get_slot(&self, slot: usize) -> Option<&UserItem> {
        if slot < TOTAL_SLOT_COUNT {
            self.slots[slot].as_ref()
        } else {
            None
        }
    }

    /// 设置指定槽位的物品
    pub fn set_slot(&mut self, slot: usize, item: Option<UserItem>) -> bool {
        if slot < TOTAL_SLOT_COUNT {
            self.slots[slot] = item;
            true
        } else {
            false
        }
    }

    /// 获取槽位在对话框中的位置
    pub fn get_slot_position(&self, slot: usize) -> Option<(i32, i32)> {
        if slot >= TOTAL_SLOT_COUNT {
            return None;
        }

        if slot >= TOOL_SLOT_COUNT {
            // 材料槽 (6个，横向排列)
            let index = slot - TOOL_SLOT_COUNT;
            Some(((index as i32 * 40) + 52, 86))
        } else {
            // 工具槽 (3个，横向排列)
            Some(((slot as i32 * 44) + 108, 44))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_item(unique_id: u64, name: &str, count: u32) -> UserItem {
        UserItem {
            unique_id,
            item_index: 100,
            name: name.to_string(),
            count,
            ..Default::default()
        }
    }

    #[test]
    fn test_craft_dialog_new() {
        let dialog = CraftDialog::new();
        assert!(!dialog.visible);
        assert!(!dialog.craft_button_enabled);
    }

    #[test]
    fn test_reset_cells() {
        let mut dialog = CraftDialog::new();
        dialog.recipe_item = Some(create_test_item(1, "Sword", 1));
        dialog.set_slot(0, Some(create_test_item(2, "Tool", 1)));

        dialog.reset_cells(true);
        assert!(dialog.recipe_item.is_none());
        assert!(dialog.get_slot(0).is_none());
    }

    #[test]
    fn test_set_and_get_slot() {
        let mut dialog = CraftDialog::new();
        let item = create_test_item(1, "Hammer", 1);

        assert!(dialog.set_slot(0, Some(item.clone())));
        assert!(dialog.get_slot(0).is_some());
        assert_eq!(dialog.get_slot(0).unwrap().name, "Hammer");
    }

    #[test]
    fn test_slot_positions() {
        let dialog = CraftDialog::new();
        
        // 工具槽位置
        let tool_pos = dialog.get_slot_position(0).unwrap();
        assert_eq!(tool_pos.1, 44);

        // 材料槽位置
        let ingredient_pos = dialog.get_slot_position(3).unwrap();
        assert_eq!(ingredient_pos.1, 86);
    }
}
