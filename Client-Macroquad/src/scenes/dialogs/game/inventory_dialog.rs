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

/// 背包对话框
pub struct InventoryDialog {
    visible: bool,
    position: egui::Pos2,
    
    /// 是否正在拖动
    dragging: bool,
    /// 拖动时的鼠标偏移
    drag_offset: egui::Vec2,
    
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
            dragging: false,
            drag_offset: egui::vec2(0.0, 0.0),
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
    
    /// 处理窗口拖动
    fn handle_dragging(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 定义可拖动区域（顶部标题栏区域，避免与关闭按钮冲突）
        // 关闭按钮位置是 (289, 3)，大小 20x20，所以拖动区域应该避开 (289-310, 3-23) 区域
        let drag_area_width = 289.0 - 5.0;  // 在关闭按钮左侧留5像素间隙
        let title_area = egui::Rect::from_min_size(
            bg_rect.min,
            egui::vec2(drag_area_width, 30.0),  // 只占用关闭按钮左侧的区域
        );
        
        let title_response = ui.interact(
            title_area,
            egui::Id::new("inv_drag_area"),
            egui::Sense::drag(),
        );
        
        // 开始拖动
        if title_response.drag_started() {
            self.dragging = true;
            if let Some(pointer_pos) = ctx.pointer_interact_pos() {
                self.drag_offset = self.position.to_vec2() - pointer_pos.to_vec2();
            }
        }
        
        // 拖动中
        if self.dragging {
            if let Some(pointer_pos) = ctx.pointer_latest_pos() {
                self.position = (pointer_pos.to_vec2() + self.drag_offset).to_pos2();
            }
            
            // 停止拖动
            if title_response.drag_stopped() || !title_response.dragged() {
                self.dragging = false;
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
    
    /// 绘制物品格子
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
        
        // 交互检测(先检测,用于悬停效果)
        let response = ui.interact(
            cell_rect,
            egui::Id::new(format!("inv_cell_{}", idx)),
            egui::Sense::click(),
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
        
        if response.clicked() {
            println!("🎒 点击背包格子 {}", idx);
            // TODO: 物品拖拽、使用等逻辑
        }
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
        
        // 交互检测(先检测,用于悬停效果)
        let response = ui.interact(
            cell_rect,
            egui::Id::new(format!("quest_cell_{}", idx)),
            egui::Sense::click(),
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
        
        if response.clicked() {
            println!("📜 点击任务物品格子 {}", idx);
            // TODO: 物品拖拽、使用等逻辑
        }
    }
    
    /// 绘制底部UI元素（金币、重量条）
    fn draw_bottom_ui(&mut self, ui: &mut egui::Ui, content_rect: &egui::Rect) {
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
                    println!("💰 扩展背包需要 {} 金币 (当前有 {})", expand_cost, self.gold);
                    
                    if self.gold >= expand_cost {
                        self.expand_inventory();
                    } else {
                        println!("⚠️ 金币不足，无法扩展背包");
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
}

impl Dialog for InventoryDialog {
    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        if !self.visible {
            *open = false;
            return;
        }
        
        // 键盘快捷键：I键或ESC键关闭背包
        ctx.input(|i| {
            if i.key_pressed(egui::Key::I) || i.key_pressed(egui::Key::Escape) {
                self.visible = false;
                println!("⌨️ 键盘关闭背包对话框");
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
                
                // 处理窗口拖动（点击背景区域可拖动）
                self.handle_dragging(ui, ctx, &bg_rect);
                
                // 绘制标签页按钮
                self.draw_tab_buttons(ui, ctx, &bg_rect);
                
                // 绘制物品格子
                self.draw_item_grid(ui, ctx, &bg_rect);
                
                // 绘制金币和负重信息
                self.draw_info_bar(ui, ctx, &bg_rect);
                
                // 绘制底部UI（金币可点击区域等）
                self.draw_bottom_ui(ui, &bg_rect);
                
                // 绘制关闭按钮
                self.draw_close_button(ui, ctx, &bg_rect);
                
                // 绘制扩展按钮（仅在需要时显示）
                self.draw_expand_button(ui, ctx, &bg_rect);
            });
        
        *open = self.visible;
    }
}
