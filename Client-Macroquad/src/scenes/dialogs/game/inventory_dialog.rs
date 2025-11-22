// ============================================================================
// InventoryDialog - 背包系统
// ============================================================================
//
// 【功能说明】
// 1. 背包窗口（46格基础 + 最多40格扩展 = 86格）
// 2. 任务物品栏（40格，独立页面）
// 3. 物品格子显示、拖拽、使用
// 4. 金币显示和拾取
// 5. 负重显示
// 6. 背包扩展功能
//
// 【布局】
// - 窗口: Title[196]
// - 标签页: ItemButton(197/737), ItemButton2(168/738), QuestButton(198/739)
// - 物品格子: 8列 x 10行 = 80格（前46格默认可见）
// - 任务格子: 8列 x 5行 = 40格（独立页面）
//
// ============================================================================

use crate::resources::{mlibrary::ImageInfo, LibraryName};
use crate::scenes::dialogs::Dialog;
use egui_macroquad::egui;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::inventory_persistence::InventoryData;

/// 背包标签页类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InventoryTab {
    Items,  // 物品页1（前46格）
    Items2, // 物品页2（扩展格子）
    Quest,  // 任务物品
}

/// 选中的物品格子
#[derive(Clone, Debug, PartialEq)]
struct SelectedItem {
    /// 来源容器类型
    container: ItemContainer,
    /// 格子索引
    index: usize,
    /// 物品图标索引
    icon_index: usize,
    /// 物品数量
    count: u32,
}

/// 物品容器类型
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
enum ItemContainer {
    Inventory, // 普通背包
    Quest,     // 任务物品栏
}

/// 物品槽位数据（模拟）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemSlot {
    /// 物品图标索引（Libraries.Items）
    pub icon_index: Option<usize>,
    /// 物品数量
    pub count: u32,
    /// 是否锁定
    locked: bool,
}

impl ItemSlot {
    fn empty() -> Self {
        Self {
            icon_index: None,
            count: 0,
            locked: false,
        }
    }

    fn new(icon_index: usize, count: u32) -> Self {
        Self {
            icon_index: Some(icon_index),
            count,
            locked: false,
        }
    }
}

/// UI布局常量
struct InventoryLayout {
    /// 物品格子宽度
    cell_width: f32,
    /// 物品格子高度
    cell_height: f32,
    /// 网格列数
    grid_cols: usize,
    /// 格子间距
    cell_spacing: f32,
    /// 内容边距
    content_margin: egui::Vec2,
}

impl InventoryLayout {
    /// 布局常量
    const CELL_WIDTH: f32 = 36.0;
    const CELL_HEIGHT: f32 = 32.0;
    const GRID_COLS: usize = 8;
    const CELL_SPACING: f32 = 1.0;
    const GRID_OFFSET_X: f32 = 0.0;
    const GRID_OFFSET_Y: f32 = 3.0;
    const DISPLAY_ROWS: usize = 5;
    
    /// 获取格子位置
    fn get_cell_rect(&self, row: usize, col: usize, grid_min: egui::Pos2) -> egui::Rect {
        let x = grid_min.x + col as f32 * (self.cell_width + self.cell_spacing) + Self::GRID_OFFSET_X;
        let y = grid_min.y + row as f32 * (self.cell_height + self.cell_spacing) + Self::GRID_OFFSET_Y;
        egui::Rect::from_min_size(
            egui::pos2(x, y),
            egui::vec2(self.cell_width, self.cell_height)
        )
    }
}

impl Default for InventoryLayout {
    fn default() -> Self {
        Self {
            cell_width: Self::CELL_WIDTH,
            cell_height: Self::CELL_HEIGHT,
            grid_cols: Self::GRID_COLS,
            cell_spacing: Self::CELL_SPACING,
            content_margin: egui::vec2(9.0, 37.0),
        }
    }
}

/// 金币飞行动画状态
#[derive(Debug, Clone)]
struct GoldFlyAnimation {
    /// 起始位置（屏幕坐标）
    start_pos: egui::Pos2,
    /// 目标位置（背包金币区域）
    target_pos: egui::Pos2,
    /// 当前位置
    current_pos: egui::Pos2,
    /// 动画开始时间
    start_time: std::time::Instant,
    /// 动画持续时间
    duration: std::time::Duration,
    /// 金币数量
    amount: u32,
    /// 动画完成标志
    completed: bool,
}

/// 背包对话框
pub struct InventoryDialog {
    /// 窗口位置
    position: egui::Pos2,
    /// 背景纹理
    bg: ImageInfo,

    layout: InventoryLayout,

    /// 当前选中的物品格子
    selected_item: Option<SelectedItem>,

    /// 滚动偏移量（每个标签页独立）
    scroll_offset_items: f32, // Items I 滚动偏移
    scroll_offset_items2: f32, // Items II 滚动偏移
    scroll_offset_quest: f32,  // Quest 滚动偏移

    /// 当前标签页
    pub active_tab: InventoryTab,

    /// 物品格子（80格，前46格默认，后34格需扩展）
    /// 索引 0-45: 默认格子
    /// 索引 46-79: 扩展格子（需要购买解锁）
    pub item_slots: Vec<ItemSlot>,

    /// 任务物品格子（40格）
    pub quest_slots: Vec<ItemSlot>,

    /// 背包最大容量（46-86）
    pub max_capacity: usize,

    /// 金币数量
    pub gold: u32,

    /// 当前负重 / 最大负重
    pub weight: (u32, u32),

    /// 金币飞行动画列表
    gold_animations: Vec<GoldFlyAnimation>,
}

impl InventoryDialog {
    const BG_INDEX: usize = 196;
    pub fn new() -> Self {
        // 创建物品格子（80格）
        let mut item_slots = Vec::with_capacity(80);
        for i in 0..80 {
            // Items I 页: 索引0-45 使用图标0-45
            // Items II 页: 索引46-85 使用图标46-85
            if i < 46 {
                // Items I 页填满46格
                item_slots.push(ItemSlot::new(i, (i % 10 + 1) as u32));
            } else if i < 86 {
                // Items II 页填满40格 (索引46-85)
                item_slots.push(ItemSlot::new(i, ((i - 46) % 10 + 1) as u32));
            } else {
                item_slots.push(ItemSlot::empty());
            }
        }

        // 创建任务物品格子（40格）- 填满所有格子
        let mut quest_slots = Vec::with_capacity(40);
        for i in 0..40 {
            // Quest 页填满40格,使用图标300-339
            quest_slots.push(ItemSlot::new(300 + i, (i % 10 + 1) as u32));
        }
        let mut bg = None;
        egui_macroquad::cfg(|ctx| {
            bg = LibraryName::Title.get_egui_texture(ctx, Self::BG_INDEX);
        });
        let bg = bg.expect("❌ 背包背景纹理 Title[196] 获取 egui 纹理失败！");
        let mut dialog = Self {
            position: egui::pos2(300.0, 100.0),
            bg,
            layout: InventoryLayout::default(),
            selected_item: None, // 初始化选中状态
            scroll_offset_items: 0.0,
            scroll_offset_items2: 0.0,
            scroll_offset_quest: 0.0,
            active_tab: InventoryTab::Items,
            item_slots,
            quest_slots,
            max_capacity: 80, // 扩展到80格,方便测试 Items II
            gold: 123456,
            weight: (75, 100),

            // 金币动画
            gold_animations: Vec::new(),
        };

        // 尝试从文件加载数据
        if let Err(e) = dialog.load_data() {
            println!("⚠️ 无法加载背包数据: {}", e);
        }

        dialog
    }

    /// 获取当前位置
    pub fn get_position(&self) -> egui::Pos2 {
        self.position
    }

    /// 获取当前金币数量
    pub fn get_gold(&self) -> u32 {
        self.gold
    }

    /// 设置金币数量
    pub fn set_gold(&mut self, amount: u32) {
        self.gold = amount;
    }

    /// 获取槽位数据（统一访问接口）
    fn get_slot(&self, container: ItemContainer, index: usize) -> Option<&ItemSlot> {
        match container {
            ItemContainer::Inventory => self.item_slots.get(index),
            ItemContainer::Quest => self.quest_slots.get(index),
        }
    }

    /// 获取可变槽位数据
    fn get_slot_mut(&mut self, container: ItemContainer, index: usize) -> Option<&mut ItemSlot> {
        match container {
            ItemContainer::Inventory => self.item_slots.get_mut(index),
            ItemContainer::Quest => self.quest_slots.get_mut(index),
        }
    }

    /// 清空槽位
    fn clear_slot(&mut self, container: ItemContainer, index: usize) {
        if let Some(slot) = self.get_slot_mut(container, index) {
            slot.icon_index = None;
            slot.count = 0;
        }
    }

    /// 设置槽位数据
    #[allow(dead_code)]
    fn set_slot(&mut self, container: ItemContainer, index: usize, icon_index: usize, count: u32) {
        if let Some(slot) = self.get_slot_mut(container, index) {
            slot.icon_index = Some(icon_index);
            slot.count = count;
        }
    }

    /// 触发金币拾取动画
    ///
    /// # 参数
    /// * `start_pos` - 金币在屏幕上的起始位置（比如地面上的金币）
    /// * `amount` - 拾取的金币数量
    /// * `target_pos` - 背包金币显示区域的位置
    pub fn trigger_gold_pickup(
        &mut self,
        start_pos: egui::Pos2,
        amount: u32,
        target_pos: egui::Pos2,
    ) {
        let animation = GoldFlyAnimation {
            start_pos,
            target_pos,
            current_pos: start_pos,
            start_time: std::time::Instant::now(),
            duration: std::time::Duration::from_millis(800), // 0.8秒动画
            amount,
            completed: false,
        };

        self.gold_animations.push(animation);

        // 同时更新金币数量
        self.gold += amount;

        println!(
            "💰 触发金币拾取动画: +{} 金币，从 ({:.0},{:.0}) 飞向 ({:.0},{:.0})",
            amount, start_pos.x, start_pos.y, target_pos.x, target_pos.y
        );
    }

    /// 更新金币动画状态
    fn update_gold_animations(&mut self) {
        let now = std::time::Instant::now();

        for animation in &mut self.gold_animations {
            if animation.completed {
                continue;
            }

            let elapsed = now.duration_since(animation.start_time);
            let progress = (elapsed.as_secs_f32() / animation.duration.as_secs_f32()).min(1.0);

            if progress >= 1.0 {
                // 动画完成
                animation.current_pos = animation.target_pos;
                animation.completed = true;
            } else {
                // 使用二次贝塞尔曲线计算飞行轨迹（抛物线效果）
                let t = progress;
                let ease_progress = 1.0 - (1.0 - t) * (1.0 - t); // easeOutQuad

                // 直线插值计算水平位置
                let x = animation.start_pos.x
                    + (animation.target_pos.x - animation.start_pos.x) * ease_progress;

                // 加入抛物线效果（向上的弧度）
                let y_direct = animation.start_pos.y
                    + (animation.target_pos.y - animation.start_pos.y) * ease_progress;
                let arc_height = 50.0; // 抛物线高度
                let arc_offset = arc_height * (4.0 * t * (1.0 - t)); // 抛物线公式
                let y = y_direct - arc_offset;

                animation.current_pos = egui::pos2(x, y);
            }
        }

        // 移除已完成的动画
        self.gold_animations.retain(|anim| !anim.completed);
    }

    /// 渲染飞行中的金币
    fn render_flying_gold(&self, painter: &egui::Painter, ctx: &egui::Context) {
        for animation in &self.gold_animations {
            if animation.completed {
                continue;
            }

            // 使用金币图标（假定使用Items库的索引116）
            if let Some(info) = LibraryName::Items.get_egui_texture(ctx, 116) {
                if let Some(gold_texture) = info.egui_texture {
                    let size = egui::vec2(16.0, 16.0); // 金币小图标
                    let rect = egui::Rect::from_center_size(animation.current_pos, size);

                    // 计算动画透明度（开始和结束时渐变）
                    let now = std::time::Instant::now();
                    let elapsed = now.duration_since(animation.start_time);
                    let progress =
                        (elapsed.as_secs_f32() / animation.duration.as_secs_f32()).min(1.0);

                    let alpha = if progress < 0.1 {
                        // 开始渐入
                        progress / 0.1
                    } else if progress > 0.9 {
                        // 结束渐出
                        (1.0 - progress) / 0.1
                    } else {
                        1.0
                    };

                    let color = egui::Color32::from_rgba_premultiplied(
                        255,
                        255,
                        255,
                        (255.0 * alpha) as u8,
                    );

                    painter.image(
                        gold_texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        color,
                    );

                    // 显示金币数量（在金币图标旁边）
                    if animation.amount > 1 {
                        let amount_text = format!("+{}", animation.amount);
                        let text_pos = egui::pos2(
                            animation.current_pos.x + 10.0,
                            animation.current_pos.y - 5.0,
                        );

                        painter.text(
                            text_pos,
                            egui::Align2::LEFT_CENTER,
                            amount_text,
                            egui::FontId::proportional(10.0),
                            egui::Color32::from_rgba_premultiplied(
                                255,
                                255,
                                0, // 黄色文字
                                (255.0 * alpha) as u8,
                            ),
                        );
                    }
                }
            }
        }
    }

    /// 处理物品交互（点击交换）
    fn handle_item_interaction(
        &mut self,
        response: egui::Response,
        container: ItemContainer,
        index: usize,
        ctx: &egui::Context,
    ) {
        // 处理左键点击
        if response.clicked() {
            // 检查修饰键状态
            let modifiers = ctx.input(|i| i.modifiers);

            if modifiers.shift {
                // Shift+点击：分离一半
                self.handle_item_click(container, index);
            } else if modifiers.ctrl {
                // Ctrl+点击：自定义分离数量
                self.handle_item_click(container, index);
            } else {
                // 普通点击：选择/交换
                self.handle_item_click(container, index);
            }
        }
    }

    /// 处理物品点击（选择/交换）
    fn handle_item_click(&mut self, container: ItemContainer, index: usize) {
        // 获取当前点击格子的物品
        let current_slot = match container {
            ItemContainer::Inventory => self.item_slots.get(index).cloned(),
            ItemContainer::Quest => self.quest_slots.get(index).cloned(),
        };

        if let Some(selected) = &self.selected_item {
            // 已有选中物品，进行交换或移动
            if selected.container == container && selected.index == index {
                // 点击同一格子，取消选择
                self.selected_item = None;
                println!("❌ 取消选择物品");
            } else {
                // 点击不同格子，进行交换或移动
                self.perform_item_exchange(selected.clone(), container, index);
                self.selected_item = None;
            }
        } else {
            // 没有选中物品，选择当前格子的物品
            if let Some(slot) = current_slot {
                if let Some(icon_index) = slot.icon_index {
                    self.selected_item = Some(SelectedItem {
                        container,
                        index,
                        icon_index,
                        count: slot.count,
                    });
                    println!(
                        "✅ 选择物品: 格子{}, 图标{}, 数量{}",
                        index, icon_index, slot.count
                    );
                } else {
                    println!("⚠️ 点击空格子");
                }
            }
        }
    }

    /// 执行物品交换或移动（支持堆叠）
    fn perform_item_exchange(
        &mut self,
        selected: SelectedItem,
        target_container: ItemContainer,
        target_index: usize,
    ) {
        // 获取目标格子的物品
        let target_slot = self.get_slot(target_container, target_index).cloned();

        if let Some(target_slot) = target_slot {
            if target_slot.icon_index.is_none() {
                // 目标格子为空，移动物品
                self.move_item_to_empty_slot(selected, target_container, target_index);
            } else if target_slot.icon_index == Some(selected.icon_index) {
                // 相同物品，尝试堆叠
                self.try_stack_items(selected, target_container, target_index, target_slot);
            } else {
                // 不同物品，交换物品
                self.swap_items(selected, target_container, target_index, target_slot);
            }
        }
    }

    /// 尝试堆叠相同物品
    fn try_stack_items(
        &mut self,
        selected: SelectedItem,
        target_container: ItemContainer,
        target_index: usize,
        target_slot: ItemSlot,
    ) {
        let max_stack = 100; // 默认最大堆叠数为100

        if max_stack <= 1 {
            // 不可堆叠物品，进行交换
            self.swap_items(selected, target_container, target_index, target_slot);
            return;
        }

        let total_count = selected.count + target_slot.count;

        if total_count <= max_stack {
            // 可以完全合并
            self.merge_items_completely(selected, target_container, target_index, total_count);
        } else {
            // 部分合并，目标格子满了
            let remaining = total_count - max_stack;
            self.merge_items_partially(
                selected,
                target_container,
                target_index,
                max_stack,
                remaining,
            );
        }
    }

    /// 完全合并物品
    fn merge_items_completely(
        &mut self,
        selected: SelectedItem,
        target_container: ItemContainer,
        target_index: usize,
        total_count: u32,
    ) {
        // 清空源格子
        self.clear_slot(selected.container, selected.index);

        // 更新目标格子数量
        if let Some(target_slot) = self.get_slot_mut(target_container, target_index) {
            target_slot.count = total_count;
        }

        println!(
            "🔄 物品完全合并: 格子{} -> 格子{}，数量{}",
            selected.index, target_index, total_count
        );
    }

    /// 部分合并物品
    fn merge_items_partially(
        &mut self,
        selected: SelectedItem,
        target_container: ItemContainer,
        target_index: usize,
        max_stack: u32,
        remaining: u32,
    ) {
        // 更新源格子数量
        match selected.container {
            ItemContainer::Inventory => {
                if let Some(source_slot) = self.item_slots.get_mut(selected.index) {
                    source_slot.count = remaining;
                }
            }
            ItemContainer::Quest => {
                if let Some(source_slot) = self.quest_slots.get_mut(selected.index) {
                    source_slot.count = remaining;
                }
            }
        }

        // 更新目标格子数量（填满）
        match target_container {
            ItemContainer::Inventory => {
                if let Some(target_slot) = self.item_slots.get_mut(target_index) {
                    target_slot.count = max_stack;
                }
            }
            ItemContainer::Quest => {
                if let Some(target_slot) = self.quest_slots.get_mut(target_index) {
                    target_slot.count = max_stack;
                }
            }
        }

        println!(
            "🔄 物品部分合并: 格子{}剩余{}, 格子{}填满{}",
            selected.index, remaining, target_index, max_stack
        );
    }

    /// 移动物品到空格子
    fn move_item_to_empty_slot(
        &mut self,
        selected: SelectedItem,
        target_container: ItemContainer,
        target_index: usize,
    ) {
        // 清空源格子
        match selected.container {
            ItemContainer::Inventory => {
                if let Some(source_slot) = self.item_slots.get_mut(selected.index) {
                    source_slot.icon_index = None;
                    source_slot.count = 0;
                }
            }
            ItemContainer::Quest => {
                if let Some(source_slot) = self.quest_slots.get_mut(selected.index) {
                    source_slot.icon_index = None;
                    source_slot.count = 0;
                }
            }
        }

        // 设置目标格子
        match target_container {
            ItemContainer::Inventory => {
                if let Some(target_slot) = self.item_slots.get_mut(target_index) {
                    target_slot.icon_index = Some(selected.icon_index);
                    target_slot.count = selected.count;
                }
            }
            ItemContainer::Quest => {
                if let Some(target_slot) = self.quest_slots.get_mut(target_index) {
                    target_slot.icon_index = Some(selected.icon_index);
                    target_slot.count = selected.count;
                }
            }
        }

        println!(
            "✅ 物品移动成功: 格子{} -> 格子{}",
            selected.index, target_index
        );
    }

    /// 交换两个格子的物品
    fn swap_items(
        &mut self,
        selected: SelectedItem,
        target_container: ItemContainer,
        target_index: usize,
        target_slot: ItemSlot,
    ) {
        // 将选中物品放入目标格子
        match target_container {
            ItemContainer::Inventory => {
                if let Some(slot) = self.item_slots.get_mut(target_index) {
                    slot.icon_index = Some(selected.icon_index);
                    slot.count = selected.count;
                }
            }
            ItemContainer::Quest => {
                if let Some(slot) = self.quest_slots.get_mut(target_index) {
                    slot.icon_index = Some(selected.icon_index);
                    slot.count = selected.count;
                }
            }
        }

        // 将目标物品放入源格子
        match selected.container {
            ItemContainer::Inventory => {
                if let Some(slot) = self.item_slots.get_mut(selected.index) {
                    slot.icon_index = target_slot.icon_index;
                    slot.count = target_slot.count;
                }
            }
            ItemContainer::Quest => {
                if let Some(slot) = self.quest_slots.get_mut(selected.index) {
                    slot.icon_index = target_slot.icon_index;
                    slot.count = target_slot.count;
                }
            }
        }

        println!(
            "🔄 物品交换成功: 格子{} ↔ 格子{}",
            selected.index, target_index
        );
    }

    /// 切换到物品页1
    fn show_items_page1(&mut self) {
        self.active_tab = InventoryTab::Items;
    }

    /// 切换到物品页2（扩展页）
    fn show_items_page2(&mut self) {
        if self.max_capacity == 46 {
            // 提示需要扩展背包
            println!("⚠️ 需要扩展背包才能使用第二页");
            // TODO: 显示扩展背包对话框
        } else {
            self.active_tab = InventoryTab::Items2;
        }
    }

    /// 切换到任务页
    fn show_quest_page(&mut self) {
        self.active_tab = InventoryTab::Quest;
    }

    /// 获取当前标签页的滚动偏移量（可变引用）
    fn get_scroll_offset_mut(&mut self) -> &mut f32 {
        match self.active_tab {
            InventoryTab::Items => &mut self.scroll_offset_items,
            InventoryTab::Items2 => &mut self.scroll_offset_items2,
            InventoryTab::Quest => &mut self.scroll_offset_quest,
        }
    }

    /// 获取当前标签页的滚动偏移量（只读）- 供旧代码使用
    #[allow(dead_code)]
    fn get_scroll_offset(&self) -> f32 {
        match self.active_tab {
            InventoryTab::Items => self.scroll_offset_items,
            InventoryTab::Items2 => self.scroll_offset_items2,
            InventoryTab::Quest => self.scroll_offset_quest,
        }
    }



    /// 绘制标签页按钮
    fn draw_tab_buttons(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // 使用纹理标签页按钮
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.;

            // 标签页配置：(library, normal_idx, selected_idx, tab_type)
            // 注意：普通标签使用 Title 库，选中状态也使用 Title 库
            let tab_configs = [
                (LibraryName::Title, 737usize, 197usize, InventoryTab::Items), // 物品1
                (LibraryName::Title, 738usize, 168usize, InventoryTab::Items2), // 物品2
                (LibraryName::Title, 739usize, 198usize, InventoryTab::Quest), // 任务
            ];

            for (normal_lib, normal_idx, selected_idx, tab_type) in tab_configs.iter() {
                // 物品2特殊处理：未扩展时显示锁定状态
                let is_locked = *tab_type == InventoryTab::Items2 && self.max_capacity <= 46;
                let is_selected = self.active_tab == *tab_type;

                // 确定纹理索引和库
                let (texture_lib, texture_idx) = if is_locked {
                    (LibraryName::Title, 169) // 锁定状态纹理
                } else if is_selected {
                    (LibraryName::Title, *selected_idx)
                } else {
                    (*normal_lib, *normal_idx)
                };

                // 绘制纹理按钮
                if let Some(info) = texture_lib.get_egui_texture(ctx, texture_idx) {
                    if let Some(_texture) = info.egui_texture {
                        let btn_size = egui::vec2(72.0, 23.0);
                        let (rect, response) =
                            ui.allocate_exact_size(btn_size, egui::Sense::click());

                        // 确定显示的纹理（悬停时显示选中状态）
                        let (display_lib, display_idx) =
                            if !is_locked && response.hovered() && !is_selected {
                                (LibraryName::Title, *selected_idx) // 悬停时显示高亮
                            } else {
                                (texture_lib, texture_idx)
                            };

                        // 绘制纹理
                        if let Some(display_info) = display_lib.get_egui_texture(ctx, display_idx) {
                            if let Some(display_texture) = display_info.egui_texture {
                                ui.painter().image(
                                    display_texture.id(),
                                    rect,
                                    egui::Rect::from_min_max(
                                        egui::pos2(0.0, 0.0),
                                        egui::pos2(1.0, 1.0),
                                    ),
                                    egui::Color32::WHITE,
                                );
                            }
                        }

                        // 处理点击事件
                        if !is_locked && response.clicked() {
                            match tab_type {
                                InventoryTab::Items => self.show_items_page1(),
                                InventoryTab::Items2 => self.show_items_page2(),
                                InventoryTab::Quest => self.show_quest_page(),
                            }
                        }

                        // 悬停提示
                        if response.hovered() {
                            let tooltip_text = if is_locked {
                                "🔒 需要扩展背包才能使用"
                            } else {
                                match tab_type {
                                    InventoryTab::Items => "物品栏 1",
                                    InventoryTab::Items2 => "物品栏 2",
                                    InventoryTab::Quest => "任务物品",
                                }
                            };
                            response.on_hover_text(tooltip_text);
                        }
                    }
                }
            }
        });

        ui.add_space(4.0);
    }

    /// 使用egui::Grid布局绘制物品格子（优化版本）
    fn draw_item_grid_optimized(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        _bg_rect: &egui::Rect,
    ) {
        // 根据当前标签页计算需要的行数
        let display_rows = 5.0; // 显示区域限制为5行
        let grid_height = display_rows * (self.layout.cell_height + self.layout.cell_spacing); // 显示区域高度

        // 获取当前页面的数据
        let (slot_count, container, start_index) = match self.active_tab {
            InventoryTab::Items => (46, ItemContainer::Inventory, 0),
            InventoryTab::Items2 => (self.max_capacity.min(80) - 46, ItemContainer::Inventory, 46),
            InventoryTab::Quest => (40, ItemContainer::Quest, 0),
        };

        // 为每个标签页创建独立的滚动区域
        let scroll_id = match self.active_tab {
            InventoryTab::Items => "inventory_scroll_items",
            InventoryTab::Items2 => "inventory_scroll_items2",
            InventoryTab::Quest => "inventory_scroll_quest",
        };

        // 使用ScrollArea + 手动布局（完全控制格子位置，无Grid间距）
        egui::ScrollArea::vertical()
            .id_salt(scroll_id)
            .max_height(grid_height)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                // 计算总高度
                let total_rows = (slot_count + self.layout.grid_cols - 1) / self.layout.grid_cols;
                let total_height = total_rows as f32 * self.layout.cell_height;
                
                // 分配整个绘制区域
                let (grid_rect, _) = ui.allocate_exact_size(
                    egui::vec2(self.layout.grid_cols as f32 * self.layout.cell_width, total_height),
                    egui::Sense::hover()
                );

                // 手动绘制每个格子
                for i in 0..slot_count {
                    let global_index = start_index + i;
                    let row = i / self.layout.grid_cols;
                    let col = i % self.layout.grid_cols;

                    // 使用布局方法计算格子位置
                    let rect = self.layout.get_cell_rect(row, col, grid_rect.min);

                    // 交互检测
                    let response = ui.interact(
                        rect,
                        egui::Id::new(format!("{}_{}", scroll_id, global_index)),
                        egui::Sense::click_and_drag()
                    );

                    // 获取格子数据
                    if let Some(slot) = self.get_slot(container, global_index) {
                        // 绘制物品内容
                        if let Some(icon_index) = slot.icon_index {
                            // 绘制图标 - 获取纹理真实尺寸并居中显示
                            if let Some(info) =
                                LibraryName::Items.get_egui_texture(ctx, icon_index)
                            {
                                if let Some(texture) = info.egui_texture {
                                    let img_size = texture.size_vec2();
                                    let offset_x = (rect.width() - img_size.x) / 2.0;
                                    let offset_y = (rect.height() - img_size.y) / 2.0 + 1.0;
                                    let icon_rect = egui::Rect::from_min_size(
                                        egui::pos2(
                                            rect.min.x + offset_x,
                                            rect.min.y + offset_y,
                                        ),
                                        img_size,
                                    );
                                    ui.painter().image(
                                        texture.id(),
                                        icon_rect,
                                        egui::Rect::from_min_max(
                                            egui::pos2(0.0, 0.0),
                                            egui::pos2(1.0, 1.0),
                                        ),
                                        egui::Color32::WHITE,
                                    );
                                }
                            }

                            // 绘制数量
                            if slot.count > 1 {
                                ui.painter().text(
                                    egui::pos2(rect.max.x - 3.0, rect.max.y - 3.0),
                                    egui::Align2::RIGHT_BOTTOM,
                                    format!("{}", slot.count),
                                    egui::FontId::proportional(10.0),
                                    egui::Color32::WHITE,
                                );
                            }

                            // 处理tooltip
                            if response.hovered() {
                                self.check_and_show_delayed_tooltip(
                                    ui,
                                    ctx,
                                    icon_index,
                                    slot.count,
                                    container,
                                    global_index,
                                );
                            }
                        }

                        // 绘制边框（在纹理之后，确保边框在最上层）
                        // 优先级：选中状态 > 悬停状态
                        let is_selected = if let Some(selected) = &self.selected_item {
                            selected.container == container && selected.index == global_index
                        } else {
                            false
                        };

                        if is_selected {
                            // 选中状态：黄色边框（默认Outside样式，边框在格子外围）
                            ui.painter().rect_stroke(
                                rect,
                                0.0,
                                egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 255, 0)),
                                egui::epaint::StrokeKind::Outside,
                            );
                        } else if response.hovered() {
                            // 悬停状态：绿色边框（默认Outside样式，边框在格子外围）
                            ui.painter().rect_stroke(
                                rect,
                                0.0,
                                egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 255, 0)),
                                egui::epaint::StrokeKind::Outside,
                            );
                        }

                        // 处理交互事件
                        self.handle_item_interaction(
                            response,
                            container,
                            global_index,
                            ctx,
                        );
                    }
                }
            });
    }

    /// 绘制物品格子（已废弃，使用draw_item_grid_optimized）
    #[allow(dead_code)]
    fn draw_item_grid(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 原版布局参数：Location = new Point(x * 36 + 9 + x, y % 5 * 32 + 37 + y % 5)
        // X: x * 37 + 9 (格子32px + 间距1px = 每格占37px，起始位置9px)
        // Y: (y % 5) * 33 + 37 (格子32px + 间距1px = 每格占33px，起始位置37px)
        let grid_start_x = 9.0;
        let grid_start_y = 37.0 - 4.;
        let x_spacing = 37.0; // X方向每格占用(36 + 1间距)
        let y_spacing = 33.0; // Y方向每格占用(32 + 1间距)

        // 定义可见区域(裁剪区):从格子起始位置向下5px开始,高度减少5px
        // 这是窗口坐标系下的固定区域,不随滚动变化
        let visible_area = egui::Rect::from_min_size(
            egui::pos2(
                bg_rect.min.x + grid_start_x,
                bg_rect.min.y + grid_start_y + 5.0,
            ),
            egui::vec2(8.0 * x_spacing, 5.0 * y_spacing - 5.0), // 可见区域高度减少5px
        );

        // 设置裁剪区域,防止格子绘制到可见区域外
        ui.set_clip_rect(visible_area);

        match self.active_tab {
            InventoryTab::Items => {
                // 显示前46格（8列 x 6行，最后一行只有6格）
                // 应用滚动偏移，可以看到所有6行
                let scroll_offset = self.get_scroll_offset();
                for idx in 0..46 {
                    let x = idx % 8;
                    let y = idx / 8;

                    let cell_x = grid_start_x + x as f32 * x_spacing;
                    let cell_y = grid_start_y + y as f32 * y_spacing + scroll_offset;

                    // 只绘制在可见区域内的格子（裁剪优化）
                    let cell_rect = egui::Rect::from_min_size(
                        egui::pos2(bg_rect.min.x + cell_x, bg_rect.min.y + cell_y),
                        egui::vec2(32.0, 32.0),
                    );

                    if visible_area.intersects(cell_rect) {
                        self.draw_item_cell(ui, ctx, bg_rect, idx, cell_x, cell_y);
                    }
                }
            }
            InventoryTab::Items2 => {
                // 显示扩展格子（46-85，8列 x 5行）
                let scroll_offset = self.get_scroll_offset();
                for i in 0..40 {
                    let idx = 46 + i;
                    let x = i % 8;
                    let y = i / 8;
                    let cell_x = grid_start_x + x as f32 * x_spacing;
                    let cell_y = grid_start_y + y as f32 * y_spacing + scroll_offset;

                    // 裁剪检查
                    let cell_rect = egui::Rect::from_min_size(
                        egui::pos2(bg_rect.min.x + cell_x, bg_rect.min.y + cell_y),
                        egui::vec2(32.0, 32.0),
                    );

                    if visible_area.intersects(cell_rect) {
                        if idx >= self.max_capacity {
                            // 绘制锁定图标
                            self.draw_locked_cell(
                                ui,
                                ctx,
                                bg_rect,
                                i,
                                grid_start_x,
                                grid_start_y + scroll_offset,
                                x_spacing,
                                y_spacing,
                            );
                        } else {
                            self.draw_item_cell(ui, ctx, bg_rect, idx, cell_x, cell_y);
                        }
                    }
                }

                // 扩展按钮（如果还能扩展）
                if self.max_capacity < 86 {
                    self.draw_expand_button(ui, ctx, bg_rect);
                }
            }
            InventoryTab::Quest => {
                // 显示任务物品（8列 x 5行 = 40格）
                let scroll_offset = self.get_scroll_offset();
                for idx in 0..40 {
                    let x = idx % 8;
                    let y = idx / 8;

                    let cell_x = grid_start_x + x as f32 * x_spacing;
                    let cell_y = grid_start_y + y as f32 * y_spacing + scroll_offset;

                    // 裁剪检查
                    let cell_rect = egui::Rect::from_min_size(
                        egui::pos2(bg_rect.min.x + cell_x, bg_rect.min.y + cell_y),
                        egui::vec2(32.0, 32.0),
                    );

                    if visible_area.intersects(cell_rect) {
                        self.draw_quest_cell(ui, ctx, bg_rect, idx, cell_x, cell_y);
                    }
                }
            }
        }
    }

    /// 绘制单个物品格子（已废弃）
    #[allow(dead_code)]
    fn draw_item_cell(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        bg_rect: &egui::Rect,
        idx: usize,
        x: f32,
        y: f32,
    ) {
        let cell_rect = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + x, bg_rect.min.y + y),
            egui::vec2(self.layout.cell_width, self.layout.cell_height),
        );

        // 交互检测(支持点击和拖拽)
        let response = ui.interact(
            cell_rect,
            egui::Id::new(format!("inv_cell_{}", idx)),
            egui::Sense::click_and_drag(),
        );

        // 鼠标悬停高亮: 使用绿色边框(原工程使用 Color.Lime = RGB(0, 255, 0))
        if response.hovered() {
            ui.painter().rect_stroke(
                cell_rect,
                0.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 255, 0)),
                egui::epaint::StrokeKind::Outside,
            );
        }

        // 选中状态高亮: 使用黄色边框
        if let Some(selected) = &self.selected_item {
            if selected.container == ItemContainer::Inventory && selected.index == idx {
                ui.painter().rect_stroke(
                    cell_rect,
                    0.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 255, 0)), // 黄色边框
                    egui::epaint::StrokeKind::Outside,
                );
            }
        }

        // 绘制物品图标（如果有）
        if let Some(slot) = self.item_slots.get(idx) {
            if let Some(icon_idx) = slot.icon_index {
                // 从 Libraries.Items 加载物品图标纹理
                if let Some(info) = LibraryName::Items.get_egui_texture(ctx, icon_idx) {
                    if let Some(texture) = info.egui_texture {
                        // Items纹理是32x32，直接填充整个格子
                        ui.painter().image(
                            texture.id(),
                            cell_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }

                // 绘制数量
                if slot.count > 1 {
                    ui.painter().text(
                        egui::pos2(cell_rect.max.x - 5.0, cell_rect.max.y - 5.0),
                        egui::Align2::RIGHT_BOTTOM,
                        format!("{}", slot.count),
                        egui::FontId::proportional(10.0),
                        egui::Color32::WHITE,
                    );
                }
            }
        }

        // 原版传奇2风格的物品预览系统（带延时）
        if response.hovered() {
            if let Some(slot) = self.item_slots.get(idx) {
                if let Some(icon_idx) = slot.icon_index {
                    // 检查并显示tooltip（如果延时已过）
                    self.check_and_show_delayed_tooltip(
                        ui,
                        ctx,
                        icon_idx,
                        slot.count,
                        ItemContainer::Inventory,
                        idx,
                    );
                }
            }
        }

        // 处理物品交互（原版传奇2风格）
        self.handle_item_interaction(response, ItemContainer::Inventory, idx, ctx);
    }

    /// 绘制任务物品格子（已废弃）
    #[allow(dead_code)]
    fn draw_quest_cell(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        bg_rect: &egui::Rect,
        idx: usize,
        x: f32,
        y: f32,
    ) {
        // 与普通格子类似，但使用 quest_slots 数据
        let cell_rect = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + x, bg_rect.min.y + y),
            egui::vec2(self.layout.cell_width, self.layout.cell_height),
        );

        // 交互检测(支持点击和拖拽)
        let response = ui.interact(
            cell_rect,
            egui::Id::new(format!("quest_cell_{}", idx)),
            egui::Sense::click_and_drag(),
        );

        // 鼠标悬停高亮: 使用绿色边框(原工程使用 Color.Lime = RGB(0, 255, 0))
        if response.hovered() {
            ui.painter().rect_stroke(
                cell_rect,
                0.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 255, 0)),
                egui::epaint::StrokeKind::Outside,
            );
        }

        // 选中状态高亮: 使用黄色边框
        if let Some(selected) = &self.selected_item {
            if selected.container == ItemContainer::Quest && selected.index == idx {
                ui.painter().rect_stroke(
                    cell_rect,
                    0.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 255, 0)), // 黄色边框
                    egui::epaint::StrokeKind::Outside,
                );
            }
        }

        // 绘制任务物品（如果有）
        if let Some(slot) = self.quest_slots.get(idx) {
            if let Some(icon_idx) = slot.icon_index {
                // 从 Libraries.Items 加载任务物品图标纹理
                if let Some(info) = LibraryName::Items.get_egui_texture(ctx, icon_idx) {
                    if let Some(texture) = info.egui_texture {
                        // 缩小纹理尺寸: 28x28 居中显示 (留出2px边距)
                        let icon_rect = cell_rect.shrink(2.0);
                        ui.painter().image(
                            texture.id(),
                            icon_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }

                // 绘制数量
                if slot.count > 1 {
                    ui.painter().text(
                        egui::pos2(cell_rect.max.x - 5.0, cell_rect.max.y - 5.0),
                        egui::Align2::RIGHT_BOTTOM,
                        format!("{}", slot.count),
                        egui::FontId::proportional(10.0),
                        egui::Color32::WHITE,
                    );
                }
            }
        }

        // 原版传奇2风格的任务物品预览系统（带延时）
        if response.hovered() {
            if let Some(slot) = self.quest_slots.get(idx) {
                if let Some(icon_idx) = slot.icon_index {
                    // 检查并显示tooltip（如果延时已过）
                    self.check_and_show_delayed_tooltip(
                        ui,
                        ctx,
                        icon_idx,
                        slot.count,
                        ItemContainer::Quest,
                        idx,
                    );
                }
            }
        }

        // 处理任务物品交互
        self.handle_item_interaction(response, ItemContainer::Quest, idx, ctx);
    }

    /// 绘制底部UI元素（金币、重量条）（已废弃）
    #[allow(dead_code)]
    fn draw_bottom_ui(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        content_rect: &egui::Rect,
    ) {
        // 金币显示区域 (40, 212, 111x14) - 原版精确位置
        let gold_rect = egui::Rect::from_min_size(
            egui::pos2(content_rect.min.x + 40.0, content_rect.min.y + 212.0),
            egui::vec2(111.0, 14.0),
        );

        // 金币交互
        let gold_response =
            ui.interact(gold_rect, egui::Id::new("gold_area"), egui::Sense::click());

        // 绘制金币背景
        if gold_response.hovered() {
            ui.painter().rect_filled(
                gold_rect,
                2.0,
                egui::Color32::from_rgba_premultiplied(255, 215, 0, 60), // 淡金色高亮
            );
        }

        // 绘制金币数量
        let gold_text = self.gold.to_string(); // 显示金币数量
        ui.painter().text(
            gold_rect.center(),
            egui::Align2::CENTER_CENTER,
            gold_text,
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(255, 215, 0), // 金色
        );

        if gold_response.clicked() {
            println!("💰 点击金币: {}", self.gold);

            // 演示金币拾取动画 - 从屏幕底部随机位置飞入金币区域
            let screen_rect = ctx.screen_rect();

            // 使用当前时间作为简单的随机源
            let time_nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos();

            let random_offset = ((time_nanos % 1000) as f32 / 1000.0 - 0.5) * 200.0;
            let start_pos = egui::pos2(
                screen_rect.center().x + random_offset,
                screen_rect.max.y - 50.0, // 屏幕底部
            );
            let target_pos = gold_rect.center();
            let pickup_amount = 100 + (time_nanos % 900); // 100-999金币

            self.trigger_gold_pickup(start_pos, pickup_amount, target_pos);
        }

        // 重量条 (182, 217) - 原版精确位置，使用纹理自然大小
        let weight_bar_rect = egui::Rect::from_min_size(
            egui::pos2(content_rect.min.x + 182.0, content_rect.min.y + 217.0),
            egui::vec2(50.0, 14.0), // 匹配原版纹理大小
        );

        // 绘制重量条背景
        ui.painter()
            .rect_filled(weight_bar_rect, 2.0, egui::Color32::from_rgb(60, 60, 60));

        // 计算重量百分比
        let weight_percent = if self.weight.1 > 0 {
            (self.weight.0 as f32 / self.weight.1 as f32).min(1.0)
        } else {
            0.0
        };

        // 绘制重量条填充
        if weight_percent > 0.0 {
            let fill_width = weight_bar_rect.width() * weight_percent;
            let fill_rect = egui::Rect::from_min_size(
                weight_bar_rect.min,
                egui::vec2(fill_width, weight_bar_rect.height()),
            );

            // 根据重量百分比选择颜色
            let fill_color = if weight_percent > 0.8 {
                egui::Color32::from_rgb(220, 50, 50) // 红色（超重）
            } else if weight_percent > 0.6 {
                egui::Color32::from_rgb(255, 140, 0) // 橙色（较重）
            } else {
                egui::Color32::from_rgb(100, 200, 100) // 绿色（正常）
            };

            ui.painter().rect_filled(fill_rect, 2.0, fill_color);
        }

        // 空格数标签 (268, 212, 26x14) - 原版精确位置和大小
        let empty_slots = self
            .item_slots
            .iter()
            .filter(|slot| slot.icon_index.is_none())
            .count();
        let weight_text_rect = egui::Rect::from_min_size(
            egui::pos2(content_rect.min.x + 268.0, content_rect.min.y + 212.0),
            egui::vec2(26.0, 14.0),
        );

        ui.painter().text(
            weight_text_rect.center(),
            egui::Align2::CENTER_CENTER,
            empty_slots.to_string(),
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
    }

    /// 绘制锁定的格子（已废弃）
    #[allow(dead_code)]
    fn draw_locked_cell(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        bg_rect: &egui::Rect,
        idx: usize,
        grid_x: f32,
        grid_y: f32,
        x_spacing: f32,
        y_spacing: f32,
    ) {
        let x = idx % 8;
        let y = idx / 8;
        let cell_x = grid_x + x as f32 * x_spacing;
        let cell_y = grid_y + y as f32 * y_spacing;

        // 绘制锁定图标 Prguse2[307]
        if let Some(info) = LibraryName::Prguse2.get_egui_texture(ctx, 307) {
            if let Some(texture) = info.egui_texture {
                let lock_rect = egui::Rect::from_min_size(
                    egui::pos2(bg_rect.min.x + cell_x, bg_rect.min.y + cell_y),
                    egui::vec2(32.0, 32.0),
                );

                ui.painter().image(
                    texture.id(),
                    lock_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        }
    }

    /// 绘制扩展按钮（仅在需要时显示）（已废弃）
    #[allow(dead_code)]
    fn draw_expand_button(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 只有在背包未满且在Items页面时才显示扩展按钮
        if self.max_capacity >= 80 || self.active_tab != InventoryTab::Items {
            return;
        }

        let x = 235.0;
        let y = 5.0;

        if let Some(info) = LibraryName::Title.get_egui_texture(ctx, 483) {
            if let Some(_texture) = info.egui_texture {
                let size = egui::vec2(72.0, 23.0);
                let btn_rect = egui::Rect::from_min_size(
                    egui::pos2(bg_rect.min.x + x, bg_rect.min.y + y),
                    size,
                );

                let response = ui.interact(
                    btn_rect,
                    egui::Id::new("inv_expand_btn"),
                    egui::Sense::click(),
                );

                let texture_idx = if response.is_pointer_button_down_on() {
                    485
                } else if response.hovered() {
                    484
                } else {
                    483
                };

                if let Some(btn_info) = LibraryName::Title.get_egui_texture(ctx, texture_idx) {
                    if let Some(btn_texture) = btn_info.egui_texture {
                        ui.painter().image(
                            btn_texture.id(),
                            btn_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }

                if response.clicked() {
                    let expand_level = (self.max_capacity - 46) / 4;
                    let expand_cost = (1000000 + expand_level * 1000000) as u32;

                    if self.gold >= expand_cost {
                        self.expand_inventory();
                        println!("🎒 背包扩展成功！消耗 {} 金币", expand_cost);
                    } else {
                        println!(
                            "💰 金币不足！需要 {} 金币，当前只有 {}",
                            expand_cost, self.gold
                        );
                    }
                }
            }
        }
    }

    /// 绘制金币和负重信息（已废弃）
    #[allow(dead_code)]
    fn draw_info_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 金币标签 (40, 212) - 原版精确位置，左对齐
        let gold_text = format!("{}", self.gold);
        ui.painter().text(
            egui::pos2(bg_rect.min.x + 40.0, bg_rect.min.y + 212.0 + 7.0), // 垂直居中在14px高度中
            egui::Align2::LEFT_CENTER,
            &gold_text,
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(255, 215, 0), // 金色
        );

        // 负重条 Prguse[24] 在 (182, 217)
        let weight_percent = self.weight.0 as f32 / self.weight.1 as f32;
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 24) {
            if let Some(texture) = info.egui_texture {
                let bar_width = 50.0 * weight_percent;
                let bar_rect = egui::Rect::from_min_size(
                    egui::pos2(bg_rect.min.x + 182.0, bg_rect.min.y + 217.0),
                    egui::vec2(bar_width, 14.0),
                );

                // 裁剪纹理显示负重条
                let tex_rect =
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(weight_percent, 1.0));

                ui.painter()
                    .image(texture.id(), bar_rect, tex_rect, egui::Color32::WHITE);
            }
        }

        // 空格数量 (268, 212) - 原版精确位置，26x14区域内居中
        let empty_slots = self.item_slots[0..self.max_capacity]
            .iter()
            .filter(|s| s.icon_index.is_none())
            .count();

        ui.painter().text(
            egui::pos2(bg_rect.min.x + 268.0 + 13.0, bg_rect.min.y + 212.0 + 7.0), // 在26x14区域内居中
            egui::Align2::CENTER_CENTER,
            format!("{}", empty_slots),
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
    }

    /// 扩展背包容量（每次扩展 4 个格子）（已废弃）
    #[allow(dead_code)]
    fn expand_inventory(&mut self) {
        if self.max_capacity < 80 {
            let old_capacity = self.max_capacity;
            self.max_capacity = (self.max_capacity + 4).min(80);
            let new_slots = self.max_capacity - old_capacity;

            // 添加新的空格子
            for _ in 0..new_slots {
                self.item_slots.push(ItemSlot {
                    icon_index: None,
                    count: 0,
                    locked: false,
                });
            }

            // 模拟扣除金币（简化处理）
            let expand_level = (old_capacity - 46) / 4;
            let expand_cost = (1000000 + expand_level * 1000000) as u32;
            if self.gold >= expand_cost {
                self.gold -= expand_cost;
                println!(
                    "🎒 背包已扩展到 {} 个格子，消耗 {} 金币",
                    self.max_capacity, expand_cost
                );
            } else {
                println!("💰 金币不足，需要 {} 金币", expand_cost);
            }
        } else {
            println!("⚠️ 背包已达到最大容量 (80 格)");
        }
    }

    /// 检查并显示延迟提示
    fn check_and_show_delayed_tooltip(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        icon_index: usize,
        count: u32,
        _container: ItemContainer,
        _index: usize,
    ) {
        // 简单的Tooltip实现
        egui::show_tooltip(ctx, ui.layer_id(), ui.id().with("tooltip"), |ui| {
            ui.label(format!("Item: {} (Count: {})", icon_index, count));
        });
    }

    /// 处理快捷键使用
    #[allow(dead_code)]
    fn handle_hotkey_use(&mut self, slot_index: usize) {
        println!("Hotkey use: {}", slot_index);
    }

    /// 处理物品丢弃
    #[allow(dead_code)]
    fn handle_item_drop(&mut self, container: ItemContainer, index: usize) {
        println!("Drop item: {:?} {}", container, index);
    }

    /// 处理选中物品使用
    #[allow(dead_code)]
    fn handle_selected_item_use(&mut self) {
        println!("Use selected item");
    }

    /// 处理方向键选择
    #[allow(dead_code)]
    fn handle_arrow_key_selection(&mut self, _i: &egui::InputState) {
        // 简单的方向键处理
    }

    /// 绘制数量输入对话框
    #[allow(dead_code)]
    fn draw_quantity_dialog(&mut self, _ctx: &egui::Context) {
        // TODO
    }

    /// 保存背包数据到文件
    ///
    /// 使用默认路径保存，通常为: %LOCALAPPDATA%/Mir2Client/inventory.json
    pub fn save_data(&self) -> anyhow::Result<()> {
        let data = InventoryData::from_dialog(self);
        let path = InventoryData::get_save_path();
        data.save_to_file(&path)
    }

    /// 保存背包数据到指定路径
    pub fn save_data_to(&self, path: &PathBuf) -> anyhow::Result<()> {
        let data = InventoryData::from_dialog(self);
        data.save_to_file(path)
    }

    /// 从默认路径加载背包数据
    pub fn load_data(&mut self) -> anyhow::Result<()> {
        let path = InventoryData::get_save_path();
        if !path.exists() {
            println!("⚠️ 背包数据文件不存在: {:?}", path);
            return Ok(()); // 不是错误，只是文件不存在
        }

        let data = InventoryData::load_from_file(&path)?;
        data.apply_to_dialog(self);
        Ok(())
    }

    /// 从指定路径加载背包数据
    pub fn load_data_from(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        let data = InventoryData::load_from_file(path)?;
        data.apply_to_dialog(self);
        Ok(())
    }

    /// 自动保存（在关闭背包时调用）
    pub fn auto_save(&self) {
        if let Err(e) = self.save_data() {
            println!("⚠️ 自动保存背包数据失败: {}", e);
        }
    }

    /// 绘制背包窗口
    pub fn draw(&mut self, ctx: &egui::Context) {
        // 使用预加载的窗口大小
        let window_size = (self.bg.width as f32,self.bg.height as f32).into();

        // 使用 Area 来实现可拖动的自定义窗口 (类似消息框的模式)
        egui::Area::new(egui::Id::new("inventory_dialog"))
            .movable(true)
            .interactable(true)
            .default_pos(self.position)
            .show(ctx, |ui| {
                // 分配固定大小的区域
                let (bg_rect, _response) =
                    ui.allocate_exact_size(window_size, egui::Sense::hover());

                // === 第1层：背景纹理（静态） ===
                if let Some(texture) = self.bg.egui_texture.as_ref() {
                        ui.painter().image(
                            texture.id(),
                            bg_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }

                // === 第2层：交互层（使用绝对定位） ===

                // 关闭按钮（右上角）- 原版位置(289, 3)
                let close_btn_size = egui::vec2(20.0, 20.0);
                let close_btn_pos = egui::pos2(bg_rect.min.x + 289.0, bg_rect.min.y + 3.0);
                let close_rect = egui::Rect::from_min_size(close_btn_pos, close_btn_size);
                let close_response = ui.interact(close_rect, egui::Id::new("close_btn"), egui::Sense::click());
                
                // 关闭按钮纹理索引 (Prguse2库: 360普通/361悬停/362按下)
                let close_texture_idx = if close_response.is_pointer_button_down_on() {
                    362 // 按下状态
                } else if close_response.hovered() {
                    361 // 悬停状态
                } else {
                    360 // 普通状态
                };
                
                // 绘制关闭按钮纹理
                if let Some(info) = LibraryName::Prguse2.get_egui_texture(ctx, close_texture_idx) {
                    if let Some(texture) = info.egui_texture {
                        ui.painter().image(
                            texture.id(),
                            close_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
                
                // 处理关闭按钮点击
                if close_response.clicked() {
                    self.auto_save();
                    // 通过返回值通知外部关闭
                    ui.memory_mut(|mem| {
                        mem.data.insert_temp(egui::Id::new("inventory_close_requested"), true);
                    });
                }

                // 标签页按钮区域
                let tab_area = egui::Rect::from_min_size(
                    bg_rect.min + egui::vec2(6.0, 7.0),
                    egui::vec2(220.0, 23.0),
                );
                let mut tab_ui = ui.new_child(egui::UiBuilder::new().max_rect(tab_area));
                self.draw_tab_buttons(&mut tab_ui, ctx);

                // 内容区域（标签页下方）- 严格限制在背景内
                let content_top = bg_rect.min.y + 35.0;
                let content_rect = egui::Rect::from_min_max(
                    egui::pos2(bg_rect.min.x + self.layout.content_margin.x, content_top), // 左边距
                    egui::pos2(bg_rect.max.x - 5.0, bg_rect.max.y - 30.0), // 右边距和底边距,留空间给底部信息栏
                );

                let mut content_ui = ui.new_child(egui::UiBuilder::new().max_rect(content_rect));
                content_ui.vertical(|ui| {
                    // 绘制物品网格
                    self.draw_item_grid_optimized(ui, ctx, &bg_rect);
                });

                // 绘制底部UI元素 - 使用绝对定位
                // 金币显示区域 (40, 212, 111x14) - 原版精确位置
                let gold_rect = egui::Rect::from_min_size(
                    egui::pos2(bg_rect.min.x + 40.0, bg_rect.min.y + 212.0),
                    egui::vec2(111.0, 14.0),
                );
                
                ui.painter().text(
                    gold_rect.left_center(),
                    egui::Align2::LEFT_CENTER,
                    format!("{}", self.gold),
                    egui::FontId::proportional(12.0),
                    egui::Color32::WHITE,
                );

                // 负重条 (182, 217) - 原版精确位置，3像素高度，透明背景
                let (current_weight, max_weight) = self.weight;
                let weight_ratio = if max_weight > 0 {
                    (current_weight as f32 / max_weight as f32).min(1.0)
                } else {
                    0.0
                };
                
                let weight_bar_rect = egui::Rect::from_min_size(
                    egui::pos2(bg_rect.min.x + 182.0, bg_rect.min.y + 217.0),
                    egui::vec2(50.0, 3.0), // 3像素高度
                );

                // 绘制进度条填充（不绘制背景）
                if weight_ratio > 0.0 {
                    let fill_width = weight_bar_rect.width() * weight_ratio;
                    let fill_rect = egui::Rect::from_min_size(
                        weight_bar_rect.min,
                        egui::vec2(fill_width, weight_bar_rect.height()),
                    );

                    // 根据负重比例选择颜色
                    let fill_color = if weight_ratio > 0.8 {
                        egui::Color32::from_rgb(220, 50, 50) // 红色（超重）
                    } else if weight_ratio > 0.6 {
                        egui::Color32::from_rgb(255, 140, 0) // 橙色（较重）
                    } else {
                        egui::Color32::from_rgb(100, 200, 100) // 绿色（正常）
                    };

                    ui.painter().rect_filled(fill_rect, 0.0, fill_color);
                }

                // 绘制负重数值文本（在进度条右侧）
                let weight_text_pos = egui::pos2(
                    weight_bar_rect.max.x + 5.0,
                    weight_bar_rect.center().y,
                );
                ui.painter().text(
                    weight_text_pos,
                    egui::Align2::LEFT_CENTER,
                    format!("{}/{}", current_weight, max_weight),
                    egui::FontId::proportional(10.0),
                    egui::Color32::WHITE,
                );

                // 绘制扩展按钮（如果需要）
                let mut content_ui2 = ui.new_child(egui::UiBuilder::new().max_rect(content_rect));
                content_ui2.vertical(|ui| {
                    ui.add_space(170.0); // 跳过网格区域

                    // 扩展按钮
                    if self.max_capacity < 80 {
                        ui.horizontal(|ui| {
                            let next_capacity = self.max_capacity + 4;
                            let expansion_cost =
                                (1_000_000 + (next_capacity / 4) as u32 * 1_000_000) as u64;

                            if ui
                                .button(format!("📦 扩展背包 (+4格) - {} 金币", expansion_cost))
                                .clicked()
                            {
                                if (self.gold as u64) >= expansion_cost {
                                    self.max_capacity = next_capacity;
                                    self.gold -= expansion_cost as u32;
                                    println!("✅ 背包扩展成功！新容量: {}", self.max_capacity);
                                } else {
                                    println!(
                                        "❌ 金币不足！需要: {}, 当前: {}",
                                        expansion_cost, self.gold
                                    );
                                }
                            }
                        });
                    }
                });
            }); // Area.show 结束

        // 渲染飞行金币 (需要在最上层)
        let foreground_painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("flying_gold_layer"),
        ));
        self.render_flying_gold(&foreground_painter, ctx);

        // 更新金币动画状态
        self.update_gold_animations();

        // 处理数量输入对话框
        self.draw_quantity_dialog(ctx);
    }
}

impl Dialog for InventoryDialog {
    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        if !*open {
            return;
        }

        // 检查关闭按钮是否被点击
        let close_requested = ctx.memory(|mem| {
            mem.data.get_temp::<bool>(egui::Id::new("inventory_close_requested"))
                .unwrap_or(false)
        });
        
        if close_requested {
            *open = false;
            // 清除标记
            ctx.memory_mut(|mem| {
                mem.data.remove::<bool>(egui::Id::new("inventory_close_requested"));
            });
            println!("🖱️ 点击关闭按钮关闭背包对话框");
            return;
        }

        // 原版传奇2风格的键盘快捷键
        ctx.input(|i| {
            // I键或ESC键关闭背包
            if i.key_pressed(egui::Key::I) || i.key_pressed(egui::Key::Escape) {
                *open = false;
                self.auto_save(); // 关闭时自动保存
                println!("⌨️ 键盘关闭背包对话框");
            }

            // Tab键切换标签页
            if i.key_pressed(egui::Key::Tab) {
                self.active_tab = match self.active_tab {
                    InventoryTab::Items => InventoryTab::Items2,
                    InventoryTab::Items2 => InventoryTab::Quest,
                    InventoryTab::Quest => InventoryTab::Items,
                };
                println!("⌨️ Tab键切换标签页: {:?}", self.active_tab);
            }

            // 数字键1-9快速使用物品（前9个格子）
            for num in 1..=9 {
                let key = match num {
                    1 => egui::Key::Num1,
                    2 => egui::Key::Num2,
                    3 => egui::Key::Num3,
                    4 => egui::Key::Num4,
                    5 => egui::Key::Num5,
                    6 => egui::Key::Num6,
                    7 => egui::Key::Num7,
                    8 => egui::Key::Num8,
                    9 => egui::Key::Num9,
                    _ => continue,
                };

                if i.key_pressed(key) {
                    let slot_index = (num - 1) as usize;
                    self.handle_hotkey_use(slot_index);
                }
            }

            // Delete键丢弃选中的物品
            if i.key_pressed(egui::Key::Delete) {
                if let Some(selected) = &self.selected_item {
                    println!(
                        "⌨️ Delete键丢弃物品: 容器{:?}, 格子{}",
                        selected.container, selected.index
                    );
                    self.handle_item_drop(selected.container, selected.index);
                }
            }

            // Enter键使用选中的物品
            if i.key_pressed(egui::Key::Enter) {
                if let Some(selected) = &self.selected_item {
                    println!(
                        "⌨️ Enter键使用物品: 容器{:?}, 格子{}",
                        selected.container, selected.index
                    );
                    self.handle_selected_item_use();
                }
            }

            // 方向键选择物品
            if i.key_pressed(egui::Key::ArrowLeft)
                || i.key_pressed(egui::Key::ArrowRight)
                || i.key_pressed(egui::Key::ArrowUp)
                || i.key_pressed(egui::Key::ArrowDown)
            {
                self.handle_arrow_key_selection(i);
            }
        });

        // 处理鼠标滚轮（在物品格子区域）
        let scroll_delta = ctx.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta != 0.0 {
            if let Some(pointer_pos) = ctx.pointer_latest_pos() {
                // 检查鼠标是否在背包窗口内
                let window_size =self.bg.get_size_vec2();
                let window_rect = egui::Rect::from_min_size(self.position, window_size);
                if window_rect.contains(pointer_pos) {
                    // 滚动物品列表
                    let (min_scroll, max_scroll) = match self.active_tab {
                        InventoryTab::Items => (-33.0, 0.0),
                        InventoryTab::Items2 | InventoryTab::Quest => (0.0, 0.0),
                    };

                    let scroll_offset = self.get_scroll_offset_mut();
                    *scroll_offset += scroll_delta * 0.5;
                    *scroll_offset = scroll_offset.clamp(min_scroll, max_scroll);
                }
            }
        }

        // 使用新的draw方法
        self.draw(ctx);
    }
}
