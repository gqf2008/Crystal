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

use egui_macroquad::egui;
use crate::resources::LibraryName;
use crate::scenes::dialogs::Dialog;

/// 背包标签页类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InventoryTab {
    Items,      // 物品页1（前46格）
    Items2,     // 物品页2（扩展格子）
    Quest,      // 任务物品
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
#[derive(Clone, Copy, Debug, PartialEq)]
enum ItemContainer {
    Inventory,  // 普通背包
    Quest,      // 任务物品栏
}

/// 物品操作选项（简化版右键菜单）
#[derive(Clone, Copy, Debug, PartialEq)]
enum ItemAction {
    Use,        // 使用
    Drop,       // 丢弃
    Properties, // 查看属性
}

/// 简化的右键菜单状态
#[derive(Clone, Debug)]
struct ContextMenu {
    /// 显示菜单的位置
    position: egui::Pos2,
    /// 目标物品信息
    target_item: SelectedItem,
    /// 显示状态
    visible: bool,
}

/// 物品信息（与原版传奇2完全一致）
#[derive(Clone, Debug)]
struct ItemInfo {
    /// 物品名称
    name: String,
    /// 物品类型
    item_type: String,
    /// 物品属性描述
    description: String,
    /// 物品等级或品质
    level: u32,
    /// 攻击力范围 (min, max)
    attack: Option<(u32, u32)>,
    /// 魔法攻击力范围 (min, max)
    magic_attack: Option<(u32, u32)>,
    /// 道术攻击力范围 (min, max)
    taoist_attack: Option<(u32, u32)>,
    /// 防御力范围 (min, max)
    defence: Option<(u32, u32)>,
    /// 魔法防御力范围 (min, max)
    magic_defence: Option<(u32, u32)>,
    /// 准确度
    accuracy: Option<u32>,
    /// 敏捷度
    agility: Option<u32>,
    /// 幸运值
    luck: Option<u32>,
    /// 重量
    weight: u32,
    /// 耐久度 (current, max)
    durability: Option<(u32, u32)>,
    /// 物品品质等级
    grade: ItemGrade,
    /// 是否经过强化
    refined: bool,
    /// 职业限制
    class_requirement: Option<String>,
    /// 性别限制
    gender_requirement: Option<String>,
    /// 最大堆叠数量
    max_stack: u32,
}

/// 物品品质等级（与原版一致）
#[derive(Clone, Debug, PartialEq)]
enum ItemGrade {
    None,
    Common,      // 普通
    Rare,        // 稀有
    Legendary,   // 传说
    Mythical,    // 神话
    Heroic,      // 英雄
}

/// 物品槽位数据（模拟）
#[derive(Debug, Clone)]
struct ItemSlot {
    /// 物品图标索引（Libraries.Items）
    icon_index: Option<usize>,
    /// 物品数量
    count: u32,
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
    /// 窗口尺寸
    window_size: egui::Vec2,
    /// 物品格子尺寸
    cell_size: f32,
    /// 网格列数
    grid_cols: usize,
    /// 格子间距
    cell_spacing: f32,
    /// 内容边距
    content_margin: egui::Vec2,
}

impl Default for InventoryLayout {
    fn default() -> Self {
        Self {
            window_size: egui::vec2(318.0, 245.0),
            cell_size: 36.0,      // 原工程格子大小：36像素
            grid_cols: 8,
            cell_spacing: 1.0,     // 原工程间距：1像素
            content_margin: egui::vec2(8.0, 8.0),
        }
    }
}

/// UI交互状态
#[derive(Default, Debug)]
struct InteractionState {
    /// 鼠标悬停的格子索引
    hovered_slot: Option<(ItemContainer, usize)>,
    /// 拖拽状态
    dragging_item: Option<SelectedItem>,
    /// 窗口拖拽状态
    window_dragging: bool,
    /// 窗口拖拽偏移
    drag_offset: egui::Vec2,
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
    visible: bool,
    position: egui::Pos2,
    layout: InventoryLayout,
    interaction: InteractionState,
    
    /// 当前选中的物品格子
    selected_item: Option<SelectedItem>,
    
    /// 简化的右键菜单状态（无tooltip干扰）
    context_menu: Option<ContextMenu>,
    
    /// 滚动偏移量（每个标签页独立）
    scroll_offset_items: f32,   // Items I 滚动偏移
    scroll_offset_items2: f32,  // Items II 滚动偏移
    scroll_offset_quest: f32,   // Quest 滚动偏移
    
    /// 当前标签页
    active_tab: InventoryTab,
    
    /// 物品格子（80格，前46格默认，后34格需扩展）
    /// 索引 0-45: 默认格子
    /// 索引 46-79: 扩展格子（需要购买解锁）
    item_slots: Vec<ItemSlot>,
    
    /// 任务物品格子（40格）
    quest_slots: Vec<ItemSlot>,
    
    /// 背包最大容量（46-86）
    max_capacity: usize,
    
    /// 金币数量
    gold: u32,
    
    /// 当前负重 / 最大负重
    weight: (u32, u32),
    
    /// 是否正在拾取金币
    picking_gold: bool,
    
    /// UI状态
    /// 金币区域是否悬停
    gold_hovered: bool,
    /// 关闭按钮是否悬停
    close_hovered: bool,
    
    /// tooltip显示延时控制
    tooltip_start_time: Option<std::time::Instant>,
    tooltip_delay: std::time::Duration,
    /// 当前悬停的物品（用于检测切换）
    current_hovered_item: Option<(ItemContainer, usize)>,
    
    /// 数量选择对话框状态
    quantity_dialog_visible: bool,
    quantity_dialog_item: Option<SelectedItem>,
    quantity_input: String,
    quantity_max: u32,
    
    /// 金币飞行动画列表
    gold_animations: Vec<GoldFlyAnimation>,
}

impl InventoryDialog {
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
        
        Self {
            visible: false,
            position: egui::pos2(300.0, 100.0),  // 默认位置
            layout: InventoryLayout::default(),
            interaction: InteractionState::default(),
            selected_item: None,  // 初始化选中状态
            // 简化右键菜单（无tooltip）
            context_menu: None,
            scroll_offset_items: 0.0,
            scroll_offset_items2: 0.0,
            scroll_offset_quest: 0.0,
            active_tab: InventoryTab::Items,
            item_slots,
            quest_slots,
            max_capacity: 80,  // 扩展到80格,方便测试 Items II
            gold: 123456,
            weight: (75, 100),
            picking_gold: false,
            gold_hovered: false,
            close_hovered: false,
            tooltip_start_time: None,
            tooltip_delay: std::time::Duration::from_millis(800), // 0.8秒延时
            current_hovered_item: None,
            
            // 数量选择对话框
            quantity_dialog_visible: false,
            quantity_dialog_item: None,
            quantity_input: String::new(),
            quantity_max: 0,
            
            // 金币动画
            gold_animations: Vec::new(),
        }
    }
    
    /// 显示/隐藏背包
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        println!("🎒 背包对话框: {}", if self.visible { "显示" } else { "隐藏" });
    }
    
    /// 获取可见状态
    pub fn is_visible(&self) -> bool {
        self.visible
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
    
    /// 触发金币拾取动画
    /// 
    /// # 参数
    /// * `start_pos` - 金币在屏幕上的起始位置（比如地面上的金币）
    /// * `amount` - 拾取的金币数量
    /// * `target_pos` - 背包金币显示区域的位置
    pub fn trigger_gold_pickup(&mut self, start_pos: egui::Pos2, amount: u32, target_pos: egui::Pos2) {
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
        
        println!("💰 触发金币拾取动画: +{} 金币，从 ({:.0},{:.0}) 飞向 ({:.0},{:.0})", 
                 amount, start_pos.x, start_pos.y, target_pos.x, target_pos.y);
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
                let x = animation.start_pos.x + (animation.target_pos.x - animation.start_pos.x) * ease_progress;
                
                // 加入抛物线效果（向上的弧度）
                let y_direct = animation.start_pos.y + (animation.target_pos.y - animation.start_pos.y) * ease_progress;
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
                    let progress = (elapsed.as_secs_f32() / animation.duration.as_secs_f32()).min(1.0);
                    
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
                        let text_pos = egui::pos2(animation.current_pos.x + 10.0, animation.current_pos.y - 5.0);
                        
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
    fn handle_item_interaction(&mut self, response: egui::Response, container: ItemContainer, 
                               index: usize, ctx: &egui::Context) {
        // 处理左键点击
        if response.clicked() {
            // 检查修饰键状态
            let modifiers = ctx.input(|i| i.modifiers);
            
            if modifiers.shift {
                // Shift+点击：分离一半
                self.handle_item_split_half(container, index);
            } else if modifiers.ctrl {
                // Ctrl+点击：自定义分离数量
                self.handle_item_split_custom(container, index);
            } else {
                // 普通点击：选择/交换
                self.handle_item_click(container, index);
            }
        }
        // 处理右键点击 - 显示简化菜单（无tooltip干扰）
        else if response.secondary_clicked() {
            let slot = match container {
                ItemContainer::Inventory => self.item_slots.get(index).cloned(),
                ItemContainer::Quest => self.quest_slots.get(index).cloned(),
            };
            
            if let Some(slot) = slot {
                if let Some(icon_index) = slot.icon_index {
                    if let Some(pointer_pos) = ctx.pointer_latest_pos() {
                        self.context_menu = Some(ContextMenu {
                            position: pointer_pos + egui::vec2(10.0, -10.0),
                            target_item: SelectedItem {
                                container,
                                index,
                                icon_index,
                                count: slot.count,
                            },
                            visible: true,
                        });
                        println!("📋 显示简化右键菜单: 格子{}, 图标{}", index, icon_index);
                    }
                }
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
                    println!("✅ 选择物品: 格子{}, 图标{}, 数量{}", index, icon_index, slot.count);
                } else {
                    println!("⚠️ 点击空格子");
                }
            }
        }
    }
    
    /// 执行物品交换或移动（支持堆叠）
    fn perform_item_exchange(&mut self, selected: SelectedItem, target_container: ItemContainer, target_index: usize) {
        // 获取目标格子的物品
        let target_slot = match target_container {
            ItemContainer::Inventory => self.item_slots.get(target_index).cloned(),
            ItemContainer::Quest => self.quest_slots.get(target_index).cloned(),
        };
        
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
    fn try_stack_items(&mut self, selected: SelectedItem, target_container: ItemContainer, target_index: usize, target_slot: ItemSlot) {
        let item_info = self.get_item_info(selected.icon_index);
        let max_stack = item_info.max_stack;
        
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
            self.merge_items_partially(selected, target_container, target_index, max_stack, remaining);
        }
    }
    
    /// 完全合并物品
    fn merge_items_completely(&mut self, selected: SelectedItem, target_container: ItemContainer, target_index: usize, total_count: u32) {
        // 清空源格子
        match selected.container {
            ItemContainer::Inventory => {
                if let Some(source_slot) = self.item_slots.get_mut(selected.index) {
                    source_slot.icon_index = None;
                    source_slot.count = 0;
                }
            },
            ItemContainer::Quest => {
                if let Some(source_slot) = self.quest_slots.get_mut(selected.index) {
                    source_slot.icon_index = None;
                    source_slot.count = 0;
                }
            },
        }
        
        // 更新目标格子
        match target_container {
            ItemContainer::Inventory => {
                if let Some(target_slot) = self.item_slots.get_mut(target_index) {
                    target_slot.count = total_count;
                }
            },
            ItemContainer::Quest => {
                if let Some(target_slot) = self.quest_slots.get_mut(target_index) {
                    target_slot.count = total_count;
                }
            },
        }
        
        println!("🔄 物品完全合并: 格子{} -> 格子{}，数量{}", selected.index, target_index, total_count);
    }
    
    /// 部分合并物品
    fn merge_items_partially(&mut self, selected: SelectedItem, target_container: ItemContainer, target_index: usize, max_stack: u32, remaining: u32) {
        // 更新源格子数量
        match selected.container {
            ItemContainer::Inventory => {
                if let Some(source_slot) = self.item_slots.get_mut(selected.index) {
                    source_slot.count = remaining;
                }
            },
            ItemContainer::Quest => {
                if let Some(source_slot) = self.quest_slots.get_mut(selected.index) {
                    source_slot.count = remaining;
                }
            },
        }
        
        // 更新目标格子数量（填满）
        match target_container {
            ItemContainer::Inventory => {
                if let Some(target_slot) = self.item_slots.get_mut(target_index) {
                    target_slot.count = max_stack;
                }
            },
            ItemContainer::Quest => {
                if let Some(target_slot) = self.quest_slots.get_mut(target_index) {
                    target_slot.count = max_stack;
                }
            },
        }
        
        println!("🔄 物品部分合并: 格子{}剩余{}, 格子{}填满{}", selected.index, remaining, target_index, max_stack);
    }
    
    /// 移动物品到空格子
    fn move_item_to_empty_slot(&mut self, selected: SelectedItem, target_container: ItemContainer, target_index: usize) {
        // 清空源格子
        match selected.container {
            ItemContainer::Inventory => {
                if let Some(source_slot) = self.item_slots.get_mut(selected.index) {
                    source_slot.icon_index = None;
                    source_slot.count = 0;
                }
            },
            ItemContainer::Quest => {
                if let Some(source_slot) = self.quest_slots.get_mut(selected.index) {
                    source_slot.icon_index = None;
                    source_slot.count = 0;
                }
            },
        }
        
        // 设置目标格子
        match target_container {
            ItemContainer::Inventory => {
                if let Some(target_slot) = self.item_slots.get_mut(target_index) {
                    target_slot.icon_index = Some(selected.icon_index);
                    target_slot.count = selected.count;
                }
            },
            ItemContainer::Quest => {
                if let Some(target_slot) = self.quest_slots.get_mut(target_index) {
                    target_slot.icon_index = Some(selected.icon_index);
                    target_slot.count = selected.count;
                }
            },
        }
        
        println!("✅ 物品移动成功: 格子{} -> 格子{}", selected.index, target_index);
    }
    
    /// 交换两个格子的物品
    fn swap_items(&mut self, selected: SelectedItem, target_container: ItemContainer, target_index: usize, target_slot: ItemSlot) {
        // 将选中物品放入目标格子
        match target_container {
            ItemContainer::Inventory => {
                if let Some(slot) = self.item_slots.get_mut(target_index) {
                    slot.icon_index = Some(selected.icon_index);
                    slot.count = selected.count;
                }
            },
            ItemContainer::Quest => {
                if let Some(slot) = self.quest_slots.get_mut(target_index) {
                    slot.icon_index = Some(selected.icon_index);
                    slot.count = selected.count;
                }
            },
        }
        
        // 将目标物品放入源格子
        match selected.container {
            ItemContainer::Inventory => {
                if let Some(slot) = self.item_slots.get_mut(selected.index) {
                    slot.icon_index = target_slot.icon_index;
                    slot.count = target_slot.count;
                }
            },
            ItemContainer::Quest => {
                if let Some(slot) = self.quest_slots.get_mut(selected.index) {
                    slot.icon_index = target_slot.icon_index;
                    slot.count = target_slot.count;
                }
            },
        }
        
        println!("🔄 物品交换成功: 格子{} ↔ 格子{}", selected.index, target_index);
    }
    
    // 注意：移除了show_context_menu函数，采用原版传奇2风格
    
    /// 处理物品操作（简化版右键菜单）
    fn handle_item_action(&mut self, action: ItemAction, target_item: &SelectedItem) {
        match action {
            ItemAction::Use => {
                // 根据物品类型决定使用行为
                let item_info = self.get_item_info(target_item.icon_index);
                
                match item_info.item_type.as_str() {
                    "消耗品" => {
                        // 消耗品：使用1个，如血瓶、蓝瓶等
                        println!("🍎 使用消耗品: 格子{}, 图标{}, 剩余{}", 
                                target_item.index, target_item.icon_index, target_item.count - 1);
                        self.remove_item_from_slot(target_item.container, target_item.index, 1);
                    },
                    "武器" | "防具" => {
                        // 装备类：装备到身上（在实际游戏中会移动到装备栏）
                        println!("⚔️ 装备物品: 格子{}, 图标{}", target_item.index, target_item.icon_index);
                        // TODO: 实际应该移动到装备栏，这里暂时移除表示"已装备"
                        self.remove_item_from_slot(target_item.container, target_item.index, target_item.count);
                    },
                    _ => {
                        // 其他物品：默认使用1个
                        println!("🎯 使用物品: 格子{}, 图标{}", target_item.index, target_item.icon_index);
                        self.remove_item_from_slot(target_item.container, target_item.index, 1);
                    }
                }
            },
            ItemAction::Drop => {
                println!("🗑️ 丢弃物品: 格子{}, 图标{}", target_item.index, target_item.icon_index);
                self.remove_item_from_slot(target_item.container, target_item.index, target_item.count);
            },
            ItemAction::Properties => {
                println!("📄 查看属性: 格子{}, 图标{}", target_item.index, target_item.icon_index);
                // TODO: 显示物品属性对话框
            },
        }
        
        // 关闭菜单
        self.context_menu = None;
        // 清除tooltip状态，避免tooltip和菜单冲突
        self.tooltip_start_time = None;
        self.current_hovered_item = None;
    }
    
    /// 从指定格子移除物品
    fn remove_item_from_slot(&mut self, container: ItemContainer, index: usize, amount: u32) {
        match container {
            ItemContainer::Inventory => {
                if let Some(slot) = self.item_slots.get_mut(index) {
                    if slot.count > amount {
                        slot.count -= amount;
                    } else {
                        slot.icon_index = None;
                        slot.count = 0;
                    }
                }
            },
            ItemContainer::Quest => {
                if let Some(slot) = self.quest_slots.get_mut(index) {
                    if slot.count > amount {
                        slot.count -= amount;
                    } else {
                        slot.icon_index = None;
                        slot.count = 0;
                    }
                }
            },
        }
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
    
    /// 获取当前标签页的滚动偏移量（只读）
    fn get_scroll_offset(&self) -> f32 {
        match self.active_tab {
            InventoryTab::Items => self.scroll_offset_items,
            InventoryTab::Items2 => self.scroll_offset_items2,
            InventoryTab::Quest => self.scroll_offset_quest,
        }
    }
    
    /// 处理窗口拖动（使用优化的状态管理）
    fn handle_window_dragging(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 定义可拖动区域（顶部标题栏区域，避免与关闭按钮冲突）
        let drag_area_width = 289.0 - 5.0;  // 在关闭按钮左侧留5像素间隙
        let title_area = egui::Rect::from_min_size(
            bg_rect.min,
            egui::vec2(drag_area_width, 30.0),
        );
        
        let title_response = ui.interact(
            title_area,
            egui::Id::new("inv_drag_area"),
            egui::Sense::drag(),
        );
        
        // 使用优化的状态管理
        if title_response.drag_started() {
            self.interaction.window_dragging = true;
            if let Some(pointer_pos) = ctx.pointer_interact_pos() {
                self.interaction.drag_offset = self.position.to_vec2() - pointer_pos.to_vec2();
            }
        }
        
        if self.interaction.window_dragging {
            if let Some(pointer_pos) = ctx.pointer_latest_pos() {
                self.position = (pointer_pos.to_vec2() + self.interaction.drag_offset).to_pos2();
            }
            
            if title_response.drag_stopped() || !title_response.dragged() {
                self.interaction.window_dragging = false;
            }
        }
    }
    
    /// 绘制背包窗口
    fn draw_window(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) -> egui::Rect {
        // 获取背景纹理 Title[196]
        if let Some(info) = LibraryName::Title.get_egui_texture(ctx, 196) {
            if let Some(bg_texture) = info.egui_texture {
                let bg_size = bg_texture.size_vec2();
                let bg_rect = egui::Rect::from_min_size(self.position, bg_size);
                
                // 绘制背景
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
        let default_size = egui::vec2(318.0, 245.0);
        let bg_rect = egui::Rect::from_min_size(self.position, default_size);
        ui.painter().rect_filled(bg_rect, 4.0, egui::Color32::from_rgb(40, 40, 50));
        bg_rect
    }
    
    /// 绘制标签页按钮
    fn draw_tab_buttons(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 标签页按钮配置：(x, y, normal_idx, selected_idx, tab_type)
        let tab_configs = [
            (6.0, 7.0, 737usize, 197usize, InventoryTab::Items),   // 物品1
            (76.0, 7.0, 738usize, 168usize, InventoryTab::Items2), // 物品2
            (146.0, 7.0, 739usize, 198usize, InventoryTab::Quest), // 任务
        ];
        
        for (x, y, normal_idx, selected_idx, tab_type) in tab_configs.iter() {
            // 根据是否选中决定纹理索引
            let texture_idx = if self.active_tab == *tab_type {
                *selected_idx
            } else {
                *normal_idx
            };
            
            // 特殊处理：如果背包容量=46，物品2按钮显示锁定状态(169)
            let texture_idx = if *tab_type == InventoryTab::Items2 && self.max_capacity == 46 {
                169
            } else {
                texture_idx
            };
            
            if let Some(info) = LibraryName::Title.get_egui_texture(ctx, texture_idx) {
                if let Some(texture) = info.egui_texture {
                    let size = egui::vec2(72.0, 23.0);
                    let btn_rect = egui::Rect::from_min_size(
                        egui::pos2(bg_rect.min.x + x, bg_rect.min.y + y),
                        size,
                    );
                    
                    let response = ui.interact(
                        btn_rect,
                        egui::Id::new(format!("inv_tab_{:?}", tab_type)),
                        egui::Sense::click(),
                    );
                    
                    // 绘制按钮纹理
                    ui.painter().image(
                        texture.id(),
                        btn_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                    
                    // 处理点击
                    if response.clicked() {
                        match tab_type {
                            InventoryTab::Items => self.show_items_page1(),
                            InventoryTab::Items2 => self.show_items_page2(),
                            InventoryTab::Quest => self.show_quest_page(),
                        }
                    }
                }
            }
        }
        
        // 关闭按钮 Prguse2[360-362]
        self.draw_close_button(ui, ctx, bg_rect);
    }
    
    /// 绘制关闭按钮
    fn draw_close_button(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        let x = 289.0;
        let y = 3.0;
        
        // 计算按钮的绝对位置
        let abs_pos = egui::pos2(bg_rect.min.x + x, bg_rect.min.y + y);
        let btn_size = egui::vec2(20.0, 20.0);
        let btn_rect = egui::Rect::from_min_size(abs_pos, btn_size);
        
        // 尝试加载正常状态纹理以获取尺寸
        if let Some(normal_info) = LibraryName::Prguse2.get_egui_texture(ctx, 360) {
            if let Some(normal_texture) = normal_info.egui_texture {
                // 创建ImageButton
                let image_button = egui::ImageButton::new(
                    egui::Image::from_texture(egui::load::SizedTexture::new(
                        normal_texture.id(), 
                        normal_texture.size_vec2()
                    )).fit_to_exact_size(btn_size)
                );
                
                // 将ImageButton放在指定位置
                let response = ui.put(btn_rect, image_button);
                
                // 更新悬停状态
                self.close_hovered = response.hovered();
                
                // 处理点击事件
                if response.clicked() {
                    self.visible = false;
                }
                
                // 如果悬停，在按钮上方叠加悬停纹理
                if self.close_hovered {
                    if let Some(hover_info) = LibraryName::Prguse2.get_egui_texture(ctx, 361) {
                        if let Some(hover_texture) = hover_info.egui_texture {
                            ui.painter().image(
                                hover_texture.id(),
                                btn_rect,
                                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                egui::Color32::WHITE,
                            );
                        }
                    }
                }
                
                return;
            }
        }
        
        // 如果纹理加载失败，使用备用的文本按钮
        let fallback_button = egui::Button::new("×")
            .fill(egui::Color32::from_rgb(150, 80, 80));
        
        let response = ui.put(btn_rect, fallback_button);
        
        if response.clicked() {
            self.visible = false;
        }
    }
    
    /// 使用egui::Grid布局绘制物品格子（优化版本）
    fn draw_item_grid_optimized(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        let grid_start_x = 9.0;
        let grid_start_y = 37.0;  // 原工程起始Y位置：37像素
        
        // 创建网格区域 - 根据原工程计算：8列完整显示，避免右侧多出半格
        let grid_width = 8.0 * 36.0 + 7.0;  // 8列×36像素 + 7像素间隔 = 295像素
        
        // 根据当前标签页计算需要的行数，支持滚动显示更多物品
        let required_rows = match self.active_tab {
            InventoryTab::Items => ((46 + 7) / 8) as f32,    // 46个物品需要6行
            InventoryTab::Items2 => {
                let items2_count = self.max_capacity.min(80) - 46;
                ((items2_count + 7) / 8) as f32  // 根据实际物品数量计算行数
            }
            InventoryTab::Quest => ((40 + 7) / 8) as f32,    // 40个物品需要5行
        };
        let display_rows = 5.0;  // 显示区域限制为5行
        let grid_height = display_rows * 33.0; // 显示区域高度
        let content_height = required_rows * 33.0; // 实际内容高度（可能超过显示区域）
        let grid_area = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + grid_start_x, bg_rect.min.y + grid_start_y),
            egui::vec2(grid_width, grid_height),
        );
        
        // 设置裁剪区域，防止物品绘制到对话框外面
        // 精确控制裁剪区域，避免超出对话框边界
        let clip_rect = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + grid_start_x - 1.0, bg_rect.min.y + grid_start_y - 1.0),
            egui::vec2(grid_width + 2.0, grid_height + 2.0),
        );
        ui.set_clip_rect(clip_rect);
        
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
        
        ui.allocate_ui_at_rect(grid_area, |ui| {
            egui::ScrollArea::vertical()
                .id_salt(scroll_id)
                .max_height(grid_height)
                .show(ui, |ui| {
                    egui::Grid::new(format!("inventory_grid_{}", scroll_id))
                        .num_columns(self.layout.grid_cols)
                        .spacing([self.layout.cell_spacing, self.layout.cell_spacing])
                        .show(ui, |ui| {
                    // 直接绘制格子，避免复杂的借用
                    for i in 0..slot_count {
                        let global_index = start_index + i;
                        
                        // 获取格子数据
                        let slot = match container {
                            ItemContainer::Inventory => self.item_slots.get(global_index),
                            ItemContainer::Quest => self.quest_slots.get(global_index),
                        };
                        
                        if let Some(slot) = slot {
                            // 直接在这里绘制格子，避免方法调用的借用问题
                            let cell_size = egui::vec2(self.layout.cell_size, self.layout.cell_size);
                            let (rect, response) = ui.allocate_exact_size(cell_size, egui::Sense::click_and_drag());
                            
                            // 绘制基础边框
                            ui.painter().rect_stroke(
                                rect,
                                2.0,
                                egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)),
                                egui::epaint::StrokeKind::Outside,
                            );
                            
                            // 绘制悬停和选中状态
                            if response.hovered() {
                                ui.painter().rect_stroke(
                                    rect,
                                    2.0,
                                    egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 255, 0)),
                                    egui::epaint::StrokeKind::Outside,
                                );
                            }
                            
                            if let Some(selected) = &self.selected_item {
                                if selected.container == container && selected.index == global_index {
                                    ui.painter().rect_stroke(
                                        rect,
                                        2.0,
                                        egui::Stroke::new(3.0, egui::Color32::from_rgb(255, 255, 0)),
                                        egui::epaint::StrokeKind::Outside,
                                    );
                                }
                            }
                            
                            // 绘制物品内容
                            if let Some(icon_index) = slot.icon_index {
                                // 绘制图标
                                if let Some(info) = LibraryName::Items.get_egui_texture(ctx, icon_index) {
                                    if let Some(texture) = info.egui_texture {
                                        let icon_rect = rect.shrink(2.0);
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
                                        egui::pos2(rect.max.x - 5.0, rect.max.y - 5.0),
                                        egui::Align2::RIGHT_BOTTOM,
                                        format!("{}", slot.count),
                                        egui::FontId::proportional(10.0),
                                        egui::Color32::WHITE,
                                    );
                                }
                                
                                // 处理tooltip
                                if response.hovered() {
                                    self.check_and_show_delayed_tooltip(ui, ctx, icon_index, slot.count, container, global_index);
                                }
                            }
                            
                            // 处理交互事件
                            self.handle_item_interaction(response, container, global_index, ctx);
                        }
                        
                        // 每8个格子换行
                        if (i + 1) % self.layout.grid_cols == 0 {
                            ui.end_row();
                        }
                    }
                        });
                });
        });
    }

    
    /// 绘制物品格子（保持向后兼容）
    fn draw_item_grid(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 原版布局参数：Location = new Point(x * 36 + 9 + x, y % 5 * 32 + 37 + y % 5)
        // X: x * 37 + 9 (格子32px + 间距1px = 每格占37px，起始位置9px)
        // Y: (y % 5) * 33 + 37 (格子32px + 间距1px = 每格占33px，起始位置37px)
        let grid_start_x = 9.0;
        let grid_start_y = 37.0-4.;
        let x_spacing = 37.0;    // X方向每格占用(36 + 1间距)
        let y_spacing = 33.0;    // Y方向每格占用(32 + 1间距)
        
        // 定义可见区域(裁剪区):从格子起始位置向下5px开始,高度减少5px
        // 这是窗口坐标系下的固定区域,不随滚动变化
        let visible_area = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + grid_start_x, bg_rect.min.y + grid_start_y + 5.0),
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
                            self.draw_locked_cell(ui, ctx, bg_rect, i, grid_start_x, grid_start_y + scroll_offset, x_spacing, y_spacing);
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
    
    /// 绘制单个物品格子
    fn draw_item_cell(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect, 
                      idx: usize, x: f32, y: f32) {
        let cell_size = 32.0;
        let cell_rect = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + x, bg_rect.min.y + y),
            egui::vec2(cell_size, cell_size),
        );
        
        // 交互检测(支持点击和拖拽)
        let response = ui.interact(
            cell_rect,
            egui::Id::new(format!("inv_cell_{}", idx)),
            egui::Sense::click_and_drag(),
        );
        
        // 绘制格子背景（深色边框）
        ui.painter().rect_stroke(
            cell_rect,
            2,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)),
            egui::epaint::StrokeKind::Outside,
        );
        
        // 鼠标悬停高亮: 使用绿色边框(原工程使用 Color.Lime = RGB(0, 255, 0))
        if response.hovered() {
            ui.painter().rect_stroke(
                cell_rect,
                2.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 255, 0)),
                egui::epaint::StrokeKind::Outside,
            );
        }
        
        // 选中状态高亮: 使用黄色边框
        if let Some(selected) = &self.selected_item {
            if selected.container == ItemContainer::Inventory && selected.index == idx {
                ui.painter().rect_stroke(
                    cell_rect,
                    2.0,
                    egui::Stroke::new(3.0, egui::Color32::from_rgb(255, 255, 0)), // 黄色边框
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
        
        // 原版传奇2风格的物品预览系统（带延时）
        if response.hovered() {
            if let Some(slot) = self.item_slots.get(idx) {
                if let Some(icon_idx) = slot.icon_index {
                    // 检查并显示tooltip（如果延时已过）
                    self.check_and_show_delayed_tooltip(ui, ctx, icon_idx, slot.count, ItemContainer::Inventory, idx);
                }
            }
        }
        
        // 处理物品交互（原版传奇2风格）
        self.handle_item_interaction(response, ItemContainer::Inventory, idx, ctx);
    }
    
    /// 绘制任务物品格子
    fn draw_quest_cell(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect, 
                       idx: usize, x: f32, y: f32) {
        // 与普通格子类似，但使用 quest_slots 数据
        let cell_size = 32.0;
        let cell_rect = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + x, bg_rect.min.y + y),
            egui::vec2(cell_size, cell_size),
        );
        
        // 交互检测(支持点击和拖拽)
        let response = ui.interact(
            cell_rect,
            egui::Id::new(format!("quest_cell_{}", idx)),
            egui::Sense::click_and_drag(),
        );
        
        ui.painter().rect_stroke(
            cell_rect,
            2,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)),
            egui::epaint::StrokeKind::Outside,
        );
        
        // 鼠标悬停高亮: 使用绿色边框(原工程使用 Color.Lime = RGB(0, 255, 0))
        if response.hovered() {
            ui.painter().rect_stroke(
                cell_rect,
                2.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 255, 0)),
                egui::epaint::StrokeKind::Outside,
            );
        }
        
        // 选中状态高亮: 使用黄色边框
        if let Some(selected) = &self.selected_item {
            if selected.container == ItemContainer::Quest && selected.index == idx {
                ui.painter().rect_stroke(
                    cell_rect,
                    2.0,
                    egui::Stroke::new(3.0, egui::Color32::from_rgb(255, 255, 0)), // 黄色边框
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
                    self.check_and_show_delayed_tooltip(ui, ctx, icon_idx, slot.count, ItemContainer::Quest, idx);
                }
            }
        }
        
        // 处理任务物品交互
        self.handle_item_interaction(response, ItemContainer::Quest, idx, ctx);
    }
    
    /// 绘制底部UI元素（金币、重量条）
    fn draw_bottom_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, content_rect: &egui::Rect) {
        // 金币显示区域 (40, 212, 111x14) - 原版精确位置
        let gold_rect = egui::Rect::from_min_size(
            egui::pos2(content_rect.min.x + 40.0, content_rect.min.y + 212.0),
            egui::vec2(111.0, 14.0)
        );
        
        // 金币交互
        let gold_response = ui.interact(
            gold_rect,
            egui::Id::new("gold_area"),
            egui::Sense::click()
        );
        
        self.gold_hovered = gold_response.hovered();
        
        // 绘制金币背景
        if self.gold_hovered {
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
            self.picking_gold = !self.picking_gold;
            println!("💰 点击金币: {} (拾取: {})", self.gold, self.picking_gold);
            
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
            egui::vec2(50.0, 14.0) // 匹配原版纹理大小
        );
        
        // 绘制重量条背景
        ui.painter().rect_filled(
            weight_bar_rect,
            2.0,
            egui::Color32::from_rgb(60, 60, 60),
        );
        
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
                egui::vec2(fill_width, weight_bar_rect.height())
            );
            
            // 根据重量百分比选择颜色
            let fill_color = if weight_percent > 0.8 {
                egui::Color32::from_rgb(220, 50, 50)  // 红色（超重）
            } else if weight_percent > 0.6 {
                egui::Color32::from_rgb(255, 140, 0)  // 橙色（较重）
            } else {
                egui::Color32::from_rgb(100, 200, 100) // 绿色（正常）
            };
            
            ui.painter().rect_filled(
                fill_rect,
                2.0,
                fill_color,
            );
        }
        
        // 空格数标签 (268, 212, 26x14) - 原版精确位置和大小
        let empty_slots = self.item_slots.iter().filter(|slot| slot.icon_index.is_none()).count();
        let weight_text_rect = egui::Rect::from_min_size(
            egui::pos2(content_rect.min.x + 268.0, content_rect.min.y + 212.0),
            egui::vec2(26.0, 14.0)
        );
        
        ui.painter().text(
            weight_text_rect.center(),
            egui::Align2::CENTER_CENTER,
            empty_slots.to_string(),
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
    }
    
    /// 绘制锁定的格子
    fn draw_locked_cell(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect,
                        idx: usize, grid_x: f32, grid_y: f32, x_spacing: f32, y_spacing: f32) {
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
    
    /// 绘制扩展按钮（仅在需要时显示）
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
                        println!("💰 金币不足！需要 {} 金币，当前只有 {}", expand_cost, self.gold);
                    }
                }
            }
        }
    }
    
    /// 绘制金币和负重信息
    fn draw_info_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 金币标签 (40, 212) - 原版精确位置，左对齐
        let gold_text = format!("{}", self.gold);
        ui.painter().text(
            egui::pos2(bg_rect.min.x + 40.0, bg_rect.min.y + 212.0 + 7.0), // 垂直居中在14px高度中
            egui::Align2::LEFT_CENTER,
            &gold_text,
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(255, 215, 0),  // 金色
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
                let tex_rect = egui::Rect::from_min_max(
                    egui::pos2(0.0, 0.0),
                    egui::pos2(weight_percent, 1.0),
                );
                
                ui.painter().image(
                    texture.id(),
                    bar_rect,
                    tex_rect,
                    egui::Color32::WHITE,
                );
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
    
    /// 扩展背包容量（每次扩展 4 个格子）
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
                println!("🎒 背包已扩展到 {} 个格子，消耗 {} 金币", self.max_capacity, expand_cost);
            } else {
                println!("💰 金币不足，需要 {} 金币", expand_cost);
            }
        } else {
            println!("⚠️ 背包已达到最大容量 (80 格)");
        }
    }
    
    // 注意：移除了draw_context_menu函数，采用原版传奇2的简单右键使用方式
    

    
    /// 检查并显示带延时的tooltip（可修改self）
    fn check_and_show_delayed_tooltip(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, icon_index: usize, count: u32, container: ItemContainer, slot_index: usize) {
        // 如果右键菜单正在显示，不显示tooltip
        if let Some(menu) = &self.context_menu {
            if menu.visible {
                return;
            }
        }
        
        let current_item = (container, slot_index);
        
        // 检查是否是新的物品，如果是则重置计时器
        if self.current_hovered_item != Some(current_item) {
            self.current_hovered_item = Some(current_item);
            self.tooltip_start_time = Some(std::time::Instant::now());
            return; // 第一帧不显示tooltip，等待延时
        }
        
        // 检查延时是否已过
        if let Some(start_time) = self.tooltip_start_time {
            if start_time.elapsed() >= self.tooltip_delay {
                self.show_item_tooltip(ui, ctx, icon_index, count, container, slot_index);
            }
        }
    }
    
    /// 清除tooltip计时器（当鼠标不在任何物品上时）
    fn clear_tooltip_if_not_hovering(&mut self, ctx: &egui::Context) {
        // 检查鼠标是否在背包窗口外，或者没有悬停任何UI元素
        if let Some(pointer_pos) = ctx.pointer_latest_pos() {
            let window_rect = egui::Rect::from_min_size(self.position, egui::vec2(318.0, 245.0));
            if !window_rect.contains(pointer_pos) {
                // 鼠标在窗口外，清除tooltip计时器和悬停物品记录
                self.tooltip_start_time = None;
                self.current_hovered_item = None;
            }
        } else {
            // 没有鼠标位置信息，清除tooltip计时器和悬停物品记录
            self.tooltip_start_time = None;
            self.current_hovered_item = None;
        }
    }    /// 显示物品tooltip（带纹理背景）
    fn show_item_tooltip(&self, _ui: &mut egui::Ui, ctx: &egui::Context, icon_index: usize, count: u32, container: ItemContainer, slot_index: usize) {
        let item_info = self.get_item_info(icon_index);
        
        // 使用自定义tooltip，而不是默认的show_tooltip_at_pointer
        if let Some(pointer_pos) = ctx.pointer_latest_pos() {
            let tooltip_pos = pointer_pos + egui::vec2(15.0, -10.0);
            
            egui::Area::new(egui::Id::new(format!("tooltip_{}_{}", 
                match container { ItemContainer::Inventory => "inv", ItemContainer::Quest => "quest" }, 
                slot_index)))
                .fixed_pos(tooltip_pos)
                .order(egui::Order::Tooltip)
                .show(ctx, |ui| {
                    // 使用透明frame，让我们自己绘制背景
                    let frame = egui::Frame::new()
                        .fill(egui::Color32::TRANSPARENT)
                        .stroke(egui::Stroke::NONE);
                    
                    frame.show(ui, |ui| {
                        ui.set_max_width(250.0);
                        
                        // 获取内容区域大小
                        let available_rect = ui.available_rect_before_wrap();
                        
                        // 绘制tooltip背景纹理
                        self.draw_tooltip_background(ui, available_rect);
                        
                        // 在背景上绘制内容，添加边距
                        ui.vertical(|ui| {
                            ui.add_space(8.0);
                            
                            ui.horizontal(|ui| {
                                ui.add_space(8.0);
                                ui.vertical(|ui| {
                                    // 物品名称（加粗显示）
                                    ui.label(egui::RichText::new(&item_info.name)
                                        .strong()
                                        .size(14.0)
                                        .color(egui::Color32::from_rgb(255, 255, 128))); // 淡黄色
                                    
                                    ui.add_space(2.0);
                                    
                                    // 物品类型
                                    ui.label(egui::RichText::new(&item_info.item_type)
                                        .size(12.0)
                                        .color(egui::Color32::from_rgb(192, 192, 192))); // 浅灰色
                                    
                                    ui.add_space(4.0);
                                    
                                    // 数量信息
                                    if count > 1 {
                                        ui.label(egui::RichText::new(format!("数量: {}", count))
                                            .size(11.0)
                                            .color(egui::Color32::WHITE));
                                    }
                                    
                                    // 等级信息
                                    if item_info.level > 0 {
                                        ui.label(egui::RichText::new(format!("等级: {}", item_info.level))
                                            .size(11.0)
                                            .color(egui::Color32::from_rgb(176, 196, 222))); // 浅蓝色
                                    }
                                    
                                    // 属性信息（删除旧的power字段引用）
                                    
                                    ui.add_space(4.0);
                                    
                                    // 物品描述
                                    if !item_info.description.is_empty() {
                                        ui.label(egui::RichText::new(&item_info.description)
                                            .size(10.0)
                                            .color(egui::Color32::from_rgb(218, 165, 32))); // 金色
                                    }
                                    
                                    // 调试信息（开发时使用）
                                    #[cfg(debug_assertions)]
                                    {
                                        ui.add_space(2.0);
                                        ui.label(egui::RichText::new(format!("图标ID: {}", icon_index))
                                            .small()
                                            .color(egui::Color32::DARK_GRAY));
                                    }
                                });
                                ui.add_space(8.0);
                            });
                            
                            ui.add_space(8.0);
                        });
                    });
                });
        }
    }
    
    /// 绘制tooltip背景纹理
    fn draw_tooltip_background(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        let painter = ui.painter();
        
        // 使用原版GameScene.cs中的tooltip背景样式
        // BackColour = Color.FromArgb(255, 50, 50, 50)
        // BorderColour = Color.Gray
        // Opacity = 0.4F
        
        // 绘制原版深色背景
        painter.rect_filled(
            rect,
            2.0,
            egui::Color32::from_rgba_unmultiplied(50, 50, 50, 255),
        );
        
        // 绘制灰色边框
        painter.rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(1.0, egui::Color32::GRAY),
            egui::epaint::StrokeKind::Outside,
        );
    }

    /// 绘制简化版右键菜单（无tooltip干扰）
    fn draw_simple_context_menu(&mut self, ctx: &egui::Context) {
        let mut action_to_execute: Option<ItemAction> = None;
        let mut menu_should_close = false;
        
        if let Some(menu) = &self.context_menu {
            if menu.visible {
                let target_item = menu.target_item.clone();
                
                egui::Area::new(egui::Id::new("simple_context_menu"))
                    .fixed_pos(menu.position)
                    .order(egui::Order::Tooltip)  // 使用最高层级确保不被遮挡
                    .show(ctx, |ui| {
                        // 传奇2风格的简洁菜单
                        let frame = egui::Frame::new()
                            .fill(egui::Color32::from_rgb(32, 24, 16))
                            .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(128, 96, 64)))
                            .inner_margin(egui::Margin::same(8))
                            .outer_margin(egui::Margin::ZERO);
                        
                        frame.show(ui, |ui| {
                            ui.set_min_width(90.0);
                            
                            ui.vertical(|ui| {
                                ui.add_space(2.0);
                                
                                // 使用按钮
                                let use_button = egui::Button::new(
                                    egui::RichText::new("使用")
                                        .color(egui::Color32::from_rgb(255, 215, 0))
                                        .size(12.0)
                                ).fill(egui::Color32::from_rgb(64, 48, 32))
                                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(128, 96, 64)));
                                
                                if ui.add_sized([80.0, 22.0], use_button).clicked() {
                                    action_to_execute = Some(ItemAction::Use);
                                }
                                
                                ui.add_space(2.0);
                                
                                // 取消按钮（不做任何操作）
                                let cancel_button = egui::Button::new(
                                    egui::RichText::new("取消")
                                        .color(egui::Color32::from_rgb(192, 192, 192))
                                        .size(12.0)
                                ).fill(egui::Color32::from_rgb(64, 48, 32))
                                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(128, 96, 64)));
                                
                                if ui.add_sized([80.0, 22.0], cancel_button).clicked() {
                                    menu_should_close = true;
                                }
                                
                                ui.add_space(2.0);
                            });
                        });
                    });
                
                // 点击菜单外关闭
                if ctx.input(|i| i.pointer.primary_pressed()) {
                    if let Some(pos) = ctx.pointer_latest_pos() {
                        let menu_area = egui::Rect::from_min_size(menu.position, egui::vec2(90.0, 60.0));
                        if !menu_area.contains(pos) {
                            menu_should_close = true;
                        }
                    }
                }
                
                // 执行动作
                if let Some(action) = action_to_execute {
                    self.handle_item_action(action, &target_item);
                    menu_should_close = true;
                }
                
                if menu_should_close {
                    self.context_menu = None;
                    // 清除tooltip状态，避免tooltip和菜单冲突
                    self.tooltip_start_time = None;
                    self.current_hovered_item = None;
                }
            }
        }
    }

    /// 绘制数量选择对话框
    fn draw_quantity_dialog(&mut self, ctx: &egui::Context) {
        if !self.quantity_dialog_visible {
            return;
        }
        
        egui::Window::new("选择分离数量")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .fixed_size(egui::vec2(250.0, 120.0))
            .frame(egui::Frame {
                fill: egui::Color32::from_rgb(40, 40, 40),
                stroke: egui::Stroke::new(2.0, egui::Color32::GOLD),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    
                    // 标题
                    ui.colored_label(egui::Color32::GOLD, "分离物品数量");
                    ui.add_space(10.0);
                    
                    // 显示最大数量
                    ui.label(format!("最多可分离: {}", self.quantity_max));
                    ui.add_space(5.0);
                    
                    // 数量输入框
                    ui.horizontal(|ui| {
                        ui.label("数量:");
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.quantity_input)
                                .desired_width(60.0)
                        );
                        
                        // 自动选中文本便于输入
                        if !response.has_focus() {
                            response.request_focus();
                        }
                    });
                    
                    ui.add_space(10.0);
                    
                    // 按钮
                    ui.horizontal(|ui| {
                        ui.add_space(30.0);
                        
                        // 确认按钮
                        if ui.add(
                            egui::Button::new("确认")
                                .fill(egui::Color32::from_rgb(0, 150, 0))
                                .min_size(egui::vec2(60.0, 25.0))
                        ).clicked() {
                            if let Ok(quantity) = self.quantity_input.parse::<u32>() {
                                if quantity > 0 && quantity <= self.quantity_max {
                                    self.execute_custom_split(quantity);
                                } else {
                                    println!("⚠️ 无效数量: {} (范围: 1-{})", quantity, self.quantity_max);
                                }
                            } else {
                                println!("⚠️ 输入的数量格式不正确");
                            }
                            self.quantity_dialog_visible = false;
                        }
                        
                        ui.add_space(10.0);
                        
                        // 取消按钮
                        if ui.add(
                            egui::Button::new("取消")
                                .fill(egui::Color32::from_rgb(150, 0, 0))
                                .min_size(egui::vec2(60.0, 25.0))
                        ).clicked() {
                            self.quantity_dialog_visible = false;
                        }
                    });
                });
            });
    }
    
    /// 执行自定义数量分离
    fn execute_custom_split(&mut self, split_quantity: u32) {
        if let Some(item) = &self.quantity_dialog_item {
            if let Some(empty_index) = self.find_empty_slot(item.container) {
                let remaining_quantity = item.count - split_quantity;
                
                // 更新原格子数量
                match item.container {
                    ItemContainer::Inventory => {
                        if let Some(source_slot) = self.item_slots.get_mut(item.index) {
                            source_slot.count = remaining_quantity;
                        }
                        // 在空格子放入分离的部分
                        if let Some(empty_slot) = self.item_slots.get_mut(empty_index) {
                            empty_slot.icon_index = Some(item.icon_index);
                            empty_slot.count = split_quantity;
                        }
                    },
                    ItemContainer::Quest => {
                        if let Some(source_slot) = self.quest_slots.get_mut(item.index) {
                            source_slot.count = remaining_quantity;
                        }
                        if let Some(empty_slot) = self.quest_slots.get_mut(empty_index) {
                            empty_slot.icon_index = Some(item.icon_index);
                            empty_slot.count = split_quantity;
                        }
                    },
                }
                println!("🔢 自定义分离: 格子{} 剩余{}, 格子{} 分离{}", item.index, remaining_quantity, empty_index, split_quantity);
            } else {
                println!("⚠️ 没有空格子进行分离");
            }
        }
        
        // 清理对话框状态
        self.quantity_dialog_item = None;
        self.quantity_input.clear();
        self.quantity_max = 0;
    }
    
    /// 原版传奇2风格：直接使用物品（无菜单）
    fn use_item_directly(&mut self, container: ItemContainer, index: usize, icon_index: usize, count: u32) {
        // 根据物品类型直接使用
        let item_name = match icon_index {
            0..=49 => format!("传奇武器 #{}", icon_index),
            50..=99 => format!("神秘防具 #{}", icon_index),
            100..=199 => format!("神奇药水 #{}", icon_index),
            200..=299 => format!("魔法卷轴 #{}", icon_index),
            _ => format!("物品 #{}", icon_index),
        };
        
        if icon_index >= 100 && icon_index < 200 {
            // 消耗品：使用一个
            self.remove_item_from_slot(container, index, 1);
            println!("🍶 使用消耗品: {} (x1)", item_name);
        } else if icon_index < 100 {
            // 装备：装备所有
            self.remove_item_from_slot(container, index, count);
            println!("⚔️ 装备物品: {} (x{})", item_name, count);
        } else {
            // 其他物品：使用一个
            self.remove_item_from_slot(container, index, 1);
            println!("🔧 使用物品: {} (x1)", item_name);
        }
    }
    
    /// 根据图标索引获取物品信息（原版传奇2风格）
    fn get_item_info(&self, icon_index: usize) -> ItemInfo {
        // 根据图标索引生成与原版传奇2一致的物品信息
        match icon_index {
            0..=49 => ItemInfo {
                name: format!("裁决之杖 +{}", icon_index % 5),
                item_type: "武器".to_string(),
                description: "一把神秘的武器，拥有强大的魔法力量。".to_string(),
                level: (icon_index as u32 % 50) + 1,
                attack: Some((15 + icon_index as u32 % 10, 25 + icon_index as u32 % 15)),
                magic_attack: None,
                taoist_attack: None,
                defence: None,
                magic_defence: None,
                accuracy: Some(5 + icon_index as u32 % 3),
                agility: None,
                luck: Some(2 + icon_index as u32 % 2),
                weight: 8 + icon_index as u32 % 5,
                durability: Some((18000 - (icon_index as u32 % 5) * 1000, 20000)),
                grade: if icon_index % 10 == 0 { ItemGrade::Legendary } else { ItemGrade::Common },
                refined: icon_index % 7 == 0,
                class_requirement: Some("战士".to_string()),
                gender_requirement: None,
                max_stack: 1, // 武器不可堆叠
            },
            50..=99 => ItemInfo {
                name: format!("龙纹盔甲 +{}", (icon_index - 50) % 5),
                item_type: "防具".to_string(),
                description: "龙纹装饰的盔甲，拥有强大的防护能力。".to_string(),
                level: ((icon_index - 50) as u32 % 40) + 10,
                attack: None,
                magic_attack: None,
                taoist_attack: None,
                defence: Some((8 + (icon_index - 50) as u32 % 8, 15 + (icon_index - 50) as u32 % 10)),
                magic_defence: Some((5 + (icon_index - 50) as u32 % 5, 10 + (icon_index - 50) as u32 % 8)),
                accuracy: None,
                agility: None,
                luck: None,
                weight: 12 + (icon_index - 50) as u32 % 8,
                durability: Some((25000 - ((icon_index - 50) as u32 % 8) * 1000, 28000)),
                grade: if (icon_index - 50) % 15 == 0 { ItemGrade::Rare } else { ItemGrade::Common },
                refined: (icon_index - 50) % 9 == 0,
                class_requirement: Some("战士".to_string()),
                gender_requirement: None,
                max_stack: 1, // 防具不可堆叠
            },
            100..=199 => ItemInfo {
                name: format!("超级魔法药"),
                item_type: "药水".to_string(),
                description: "高级的魔法药水，能够快速恢复魔法值。".to_string(),
                level: 1,
                attack: None,
                magic_attack: None,
                taoist_attack: None,
                defence: None,
                magic_defence: None,
                accuracy: None,
                agility: None,
                luck: None,
                weight: 1,
                durability: None,
                grade: ItemGrade::Common,
                refined: false,
                class_requirement: None,
                gender_requirement: None,
                max_stack: 250, // 药水可堆叠250个
            },
            300..=349 => ItemInfo {
                name: format!("任务令牌"),
                item_type: "任务物品".to_string(),
                description: "重要的任务物品，完成特定任务需要用到。".to_string(),
                level: 1,
                attack: None,
                magic_attack: None,
                taoist_attack: None,
                defence: None,
                magic_defence: None,
                accuracy: None,
                agility: None,
                luck: None,
                weight: 1,
                durability: None,
                grade: ItemGrade::Common,
                refined: false,
                class_requirement: None,
                gender_requirement: None,
                max_stack: 100, // 任务物品可堆叠100个
            },
            _ => ItemInfo {
                name: format!("神秘物品"),
                item_type: "特殊物品".to_string(),
                description: "一个神秘的物品，它的用途还未被发现。".to_string(),
                level: 1,
                attack: None,
                magic_attack: None,
                taoist_attack: None,
                defence: None,
                magic_defence: None,
                accuracy: None,
                agility: None,
                luck: None,
                weight: 1,
                durability: None,
                grade: ItemGrade::Mythical,
                refined: false,
                class_requirement: None,
                gender_requirement: None,
                max_stack: 50, // 特殊物品可堆叠50个
            },
        }
    }
    
    /// 处理数字键快速使用物品
    fn handle_hotkey_use(&mut self, slot_index: usize) {
        if slot_index >= self.item_slots.len() {
            return;
        }
        
        if let Some(slot) = self.item_slots.get(slot_index) {
            if let Some(icon_idx) = slot.icon_index {
                println!("⌨️ 数字键{}使用物品: 格子{}, 图标{}", slot_index + 1, slot_index, icon_idx);
                self.use_item_directly(ItemContainer::Inventory, slot_index, icon_idx, slot.count);
            } else {
                println!("⌨️ 数字键{}: 格子{}为空", slot_index + 1, slot_index);
            }
        }
    }
    
    /// 处理Delete键丢弃物品
    fn handle_item_drop(&mut self, container: ItemContainer, index: usize) {
        let success = match container {
            ItemContainer::Inventory => {
                if let Some(slot) = self.item_slots.get_mut(index) {
                    if slot.icon_index.is_some() {
                        let item_name = format!("物品#{}", slot.icon_index.unwrap_or(0));
                        *slot = ItemSlot::empty();
                        println!("🗑️ 丢弃物品: {}", item_name);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            ItemContainer::Quest => {
                if let Some(slot) = self.quest_slots.get_mut(index) {
                    if slot.icon_index.is_some() {
                        let item_name = format!("任务物品#{}", slot.icon_index.unwrap_or(0));
                        *slot = ItemSlot::empty();
                        println!("🗑️ 丢弃任务物品: {}", item_name);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        };
        
        if !success {
            println!("⚠️ 无法丢弃: 格子为空或无效");
        } else {
            // 清除选中状态
            self.selected_item = None;
        }
    }
    
    /// 处理Enter键使用选中的物品
    fn handle_selected_item_use(&mut self) {
        if let Some(selected) = &self.selected_item.clone() {
            let slot = match selected.container {
                ItemContainer::Inventory => self.item_slots.get(selected.index),
                ItemContainer::Quest => self.quest_slots.get(selected.index),
            };
            
            if let Some(slot) = slot {
                if let Some(icon_idx) = slot.icon_index {
                    self.use_item_directly(selected.container, selected.index, icon_idx, slot.count);
                } else {
                    println!("⚠️ 选中的格子为空");
                }
            }
        } else {
            println!("⚠️ 没有选中任何物品");
        }
    }
    
    /// 处理Shift+点击分离一半物品
    fn handle_item_split_half(&mut self, container: ItemContainer, index: usize) {
        let slot = match container {
            ItemContainer::Inventory => self.item_slots.get(index).cloned(),
            ItemContainer::Quest => self.quest_slots.get(index).cloned(),
        };
        
        if let Some(slot) = slot {
            if let Some(icon_index) = slot.icon_index {
                if slot.count > 1 {
                    // 找到空格子进行分离
                    let split_count = slot.count / 2;
                    let remaining_count = slot.count - split_count;
                    
                    if let Some(empty_index) = self.find_empty_slot(container) {
                        // 更新原格子数量
                        match container {
                            ItemContainer::Inventory => {
                                if let Some(source_slot) = self.item_slots.get_mut(index) {
                                    source_slot.count = remaining_count;
                                }
                                // 在空格子放入分离的部分
                                if let Some(empty_slot) = self.item_slots.get_mut(empty_index) {
                                    empty_slot.icon_index = Some(icon_index);
                                    empty_slot.count = split_count;
                                }
                            },
                            ItemContainer::Quest => {
                                if let Some(source_slot) = self.quest_slots.get_mut(index) {
                                    source_slot.count = remaining_count;
                                }
                                if let Some(empty_slot) = self.quest_slots.get_mut(empty_index) {
                                    empty_slot.icon_index = Some(icon_index);
                                    empty_slot.count = split_count;
                                }
                            },
                        }
                        println!("✂️ Shift+点击分离: 格子{} 剩余{}，格子{} 分离{}", index, remaining_count, empty_index, split_count);
                    } else {
                        println!("⚠️ 没有空格子进行分离");
                    }
                } else {
                    println!("⚠️ 物品数量为1，无法分离");
                }
            }
        }
    }
    
    /// 处理Ctrl+点击自定义分离数量
    fn handle_item_split_custom(&mut self, container: ItemContainer, index: usize) {
        let slot = match container {
            ItemContainer::Inventory => self.item_slots.get(index).cloned(),
            ItemContainer::Quest => self.quest_slots.get(index).cloned(),
        };
        
        if let Some(slot) = slot {
            if let Some(icon_index) = slot.icon_index {
                if slot.count > 1 {
                    // 显示数量选择对话框
                    self.quantity_dialog_visible = true;
                    self.quantity_dialog_item = Some(SelectedItem {
                        container,
                        index,
                        icon_index,
                        count: slot.count,
                    });
                    self.quantity_max = slot.count - 1; // 最多可以分离 count-1 个
                    self.quantity_input = "1".to_string(); // 默认分离1个
                    println!("🔢 Ctrl+点击显示数量选择对话框: 最多可分离{}", self.quantity_max);
                } else {
                    println!("⚠️ 物品数量为1，无法分离");
                }
            }
        }
    }
    
    /// 查找空格子
    fn find_empty_slot(&self, container: ItemContainer) -> Option<usize> {
        match container {
            ItemContainer::Inventory => {
                for (i, slot) in self.item_slots.iter().enumerate() {
                    if i < self.max_capacity && slot.icon_index.is_none() {
                        return Some(i);
                    }
                }
            },
            ItemContainer::Quest => {
                for (i, slot) in self.quest_slots.iter().enumerate() {
                    if slot.icon_index.is_none() {
                        return Some(i);
                    }
                }
            },
        }
        None
    }
    
    /// 处理方向键选择物品
    fn handle_arrow_key_selection(&mut self, input: &egui::InputState) {
        let current_selection = self.selected_item.clone();
        
        // 计算当前网格的行列数
        let (cols, rows) = match self.active_tab {
            InventoryTab::Items => (8, 6),      // Items页：8列6行
            InventoryTab::Items2 => (8, 5),     // Items2页：8列5行
            InventoryTab::Quest => (8, 5),      // Quest页：8列5行
        };
        
        let new_selection = if let Some(selected) = current_selection {
            // 如果当前有选中的物品，根据方向键移动
            if selected.container == ItemContainer::Inventory && self.active_tab == InventoryTab::Items ||
               selected.container == ItemContainer::Inventory && self.active_tab == InventoryTab::Items2 ||
               selected.container == ItemContainer::Quest && self.active_tab == InventoryTab::Quest {
                
                let current_row = selected.index / cols;
                let current_col = selected.index % cols;
                
                let (new_row, new_col) = if input.key_pressed(egui::Key::ArrowLeft) {
                    (current_row, if current_col > 0 { current_col - 1 } else { cols - 1 })
                } else if input.key_pressed(egui::Key::ArrowRight) {
                    (current_row, if current_col < cols - 1 { current_col + 1 } else { 0 })
                } else if input.key_pressed(egui::Key::ArrowUp) {
                    (if current_row > 0 { current_row - 1 } else { rows - 1 }, current_col)
                } else if input.key_pressed(egui::Key::ArrowDown) {
                    (if current_row < rows - 1 { current_row + 1 } else { 0 }, current_col)
                } else {
                    (current_row, current_col)
                };
                
                let new_index = new_row * cols + new_col;
                
                // 确保新索引在有效范围内
                let max_index = match self.active_tab {
                    InventoryTab::Items => self.item_slots.len().min(46),
                    InventoryTab::Items2 => self.item_slots.len(),
                    InventoryTab::Quest => self.quest_slots.len(),
                };
                
                if new_index < max_index {
                    // 获取新位置的物品信息
                    let (icon_idx, count) = match selected.container {
                        ItemContainer::Inventory => {
                            if let Some(slot) = self.item_slots.get(new_index) {
                                (slot.icon_index.unwrap_or(0), slot.count)
                            } else {
                                (0, 0)
                            }
                        }
                        ItemContainer::Quest => {
                            if let Some(slot) = self.quest_slots.get(new_index) {
                                (slot.icon_index.unwrap_or(0), slot.count)
                            } else {
                                (0, 0)
                            }
                        }
                    };
                    
                    Some(SelectedItem {
                        container: selected.container,
                        index: new_index,
                        icon_index: icon_idx,
                        count,
                    })
                } else {
                    Some(selected)
                }
            } else {
                Some(selected)
            }
        } else {
            // 如果没有选中的物品，选择第一个格子
            let container = match self.active_tab {
                InventoryTab::Items | InventoryTab::Items2 => ItemContainer::Inventory,
                InventoryTab::Quest => ItemContainer::Quest,
            };
            
            // 获取第一个格子的物品信息
            let (icon_idx, count) = match container {
                ItemContainer::Inventory => {
                    if let Some(slot) = self.item_slots.get(0) {
                        (slot.icon_index.unwrap_or(0), slot.count)
                    } else {
                        (0, 0)
                    }
                }
                ItemContainer::Quest => {
                    if let Some(slot) = self.quest_slots.get(0) {
                        (slot.icon_index.unwrap_or(0), slot.count)
                    } else {
                        (0, 0)
                    }
                }
            };
            
            Some(SelectedItem {
                container,
                index: 0,
                icon_index: icon_idx,
                count,
            })
        };
        
        if let Some(new_sel) = new_selection {
            if self.selected_item.as_ref() != Some(&new_sel) {
                self.selected_item = Some(new_sel.clone());
                println!("⌨️ 方向键选择: 容器{:?}, 格子{}", new_sel.container, new_sel.index);
            }
        }
    }
}

impl Dialog for InventoryDialog {
    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        if !self.visible {
            *open = false;
            return;
        }
        
        // 更新金币动画状态
        self.update_gold_animations();
        
        // 原版传奇2风格的键盘快捷键
        ctx.input(|i| {
            // I键或ESC键关闭背包
            if i.key_pressed(egui::Key::I) || i.key_pressed(egui::Key::Escape) {
                self.visible = false;
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
                    println!("⌨️ Delete键丢弃物品: 容器{:?}, 格子{}", selected.container, selected.index);
                    self.handle_item_drop(selected.container, selected.index);
                }
            }
            
            // Enter键使用选中的物品
            if i.key_pressed(egui::Key::Enter) {
                if let Some(selected) = &self.selected_item {
                    println!("⌨️ Enter键使用物品: 容器{:?}, 格子{}", selected.container, selected.index);
                    self.handle_selected_item_use();
                }
            }
            
            // 方向键选择物品
            if i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::ArrowRight) ||
               i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::ArrowDown) {
                self.handle_arrow_key_selection(i);
            }
        });
        
        // 处理鼠标滚轮（在物品格子区域）
        let scroll_delta = ctx.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta != 0.0 {
            println!("🖱️ 检测到滚轮: {:.1}", scroll_delta);
            if let Some(pointer_pos) = ctx.pointer_latest_pos() {
                // 检查鼠标是否在背包窗口内
                let window_rect = egui::Rect::from_min_size(self.position, egui::vec2(318.0, 245.0));
                println!("   窗口区域: {:?}, 鼠标位置: {:?}", window_rect, pointer_pos);
                if window_rect.contains(pointer_pos) {
                    println!("   ✅ 鼠标在窗口内");
                    // 滚动物品列表
                    // 先计算滚动范围限制（避免借用冲突）
                    let (min_scroll, max_scroll) = match self.active_tab {
                        InventoryTab::Items => {
                            // Items页：6行，可见5行，可以向上滚动1行的距离
                            (-33.0, 0.0)
                        },
                        InventoryTab::Items2 | InventoryTab::Quest => {
                            // Items2/Quest页：5行，刚好填满，不需要滚动
                            (0.0, 0.0)
                        },
                    };
                    
                    // 再获取可变引用更新滚动偏移
                    let scroll_offset = self.get_scroll_offset_mut();
                    let old_offset = *scroll_offset;
                    *scroll_offset += scroll_delta * 0.5;
                    *scroll_offset = scroll_offset.clamp(min_scroll, max_scroll);
                    println!("🖱️ 背包滚动: {:.1} -> {:.1} (范围: {:.1} ~ {:.1})", old_offset, *scroll_offset, min_scroll, max_scroll);
                } else {
                    println!("   ❌ 鼠标不在窗口内");
                }
            }
        }
        
        egui::Window::new("Inventory")
            .title_bar(false)
            .resizable(false)
            .fixed_pos(self.position)
            .movable(false)  // 禁用 egui 默认拖动，使用自定义拖动
            .frame(egui::Frame::NONE)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                // 绘制窗口背景
                let bg_rect = self.draw_window(ui, ctx);
                
                // 处理窗口拖动（使用优化的状态管理）
                self.handle_window_dragging(ui, ctx, &bg_rect);
                
                // 绘制标签页按钮
                self.draw_tab_buttons(ui, ctx, &bg_rect);
                
                // 使用优化的Grid布局绘制物品格子
                self.draw_item_grid_optimized(ui, ctx, &bg_rect);
                
                // 同时保持原有的绘制方法作为备用
                // self.draw_item_grid(ui, ctx, &bg_rect);
                
                // 检查是否需要清除tooltip计时器（当鼠标不在任何物品上时）
                self.clear_tooltip_if_not_hovering(ctx);
                
                // 绘制金币和负重信息
                self.draw_info_bar(ui, ctx, &bg_rect);
                
                // 绘制底部UI（金币可点击区域等）
                self.draw_bottom_ui(ui, ctx, &bg_rect);
                
                // 绘制关闭按钮
                self.draw_close_button(ui, ctx, &bg_rect);
                
                // 绘制扩展按钮（仅在需要时显示）
                self.draw_expand_button(ui, ctx, &bg_rect);
            });
        
        // 绘制简化版右键菜单（无tooltip干扰）
        self.draw_simple_context_menu(ctx);
        
        // 绘制数量选择对话框
        self.draw_quantity_dialog(ctx);
        
        // 渲染飞行中的金币（在最上层，不受窗口裁剪影响）
        let foreground_painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("flying_gold_layer"),
        ));
        self.render_flying_gold(&foreground_painter, ctx);
        
        *open = self.visible;
    }
}
