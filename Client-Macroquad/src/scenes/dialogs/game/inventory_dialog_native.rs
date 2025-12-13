// ============================================================================
// InventoryDialogNative - 背包系统（macroquad 原生 UI 版本）
// ============================================================================
//
// 【说明】
// 这是使用 macroquad 原生 UI 实现的背包对话框
// 主要展示：
// 1. 使用 Skin 系统实现纹理按钮
// 2. 使用 Group::draggable() 实现拖放功能
// 3. 像素级定位控制
//
// 与 egui 版本的区别：
// - 更适合拖放操作
// - 纹理按钮更简洁
// - 手动处理滚动和文本
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use std::collections::HashMap;

/// 物品槽位
#[derive(Debug, Clone)]
pub struct ItemSlotNative {
    /// 物品图标索引
    pub icon_index: Option<usize>,
    /// 物品数量
    pub count: u32,
}

impl ItemSlotNative {
    pub fn empty() -> Self {
        Self {
            icon_index: None,
            count: 0,
        }
    }

    pub fn new(icon_index: usize, count: u32) -> Self {
        Self {
            icon_index: Some(icon_index),
            count,
        }
    }
}

/// 拖放状态
#[derive(Debug, Clone)]
struct DragState {
    /// 正在拖动的物品来源索引
    source_index: usize,
    /// 物品图标
    icon_index: usize,
    /// 物品数量
    count: u32,
}

/// 标签页类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryTab {
    /// 装备页
    Equipment = 0,
    /// 道具页
    Items = 1,
    /// 任务物品页
    Quest = 2,
}

impl InventoryTab {
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => InventoryTab::Equipment,
            1 => InventoryTab::Items,
            2 => InventoryTab::Quest,
            _ => InventoryTab::Equipment,
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            InventoryTab::Equipment => "装备",
            InventoryTab::Items => "道具",
            InventoryTab::Quest => "任务",
        }
    }
}

/// 背包对话框（原生 UI 版本）
pub struct InventoryDialogNative {
    /// 窗口位置
    position: Vec2,
    /// 窗口大小
    size: Vec2,
    /// 是否可见
    visible: bool,
    /// 是否正在拖动窗口
    dragging_window: bool,
    /// 拖动偏移
    drag_offset: Vec2,
    
    /// 当前选中的标签页
    current_tab: InventoryTab,
    /// 各标签页的物品格子（每页46格）
    pub tab_items: [Vec<ItemSlotNative>; 3],
    /// 金币
    pub gold: u32,
    
    /// 当前选中的格子
    selected_slot: Option<usize>,
    /// 拖放状态
    drag_state: Option<DragState>,
    /// 悬停的格子
    hovered_slot: Option<usize>,
    /// 悬停的标签页
    hovered_tab: Option<usize>,
    
    /// 背景纹理
    bg_texture: Option<Texture2D>,
    /// 关闭按钮纹理（普通/悬停/按下）
    close_textures: [Option<Texture2D>; 3],
    /// 标签页按钮纹理（普通/选中）- 每个标签3个状态
    tab_textures: [[Option<Texture2D>; 2]; 3],
    /// 物品图标纹理缓存
    item_textures: HashMap<usize, Texture2D>,
    /// 滚动偏移（每个标签页独立）
    scroll_offsets: [f32; 3],
    /// 可见区域高度
    visible_height: f32,
}

impl InventoryDialogNative {
    // 原版布局参数：Location = new Point(x * 36 + 9 + x, y % 5 * 32 + 37 + y % 5)
    // 格子宽度36，高度32，间距1
    const CELL_WIDTH: f32 = 36.0;
    const CELL_HEIGHT: f32 = 32.0;
    const GRID_COLS: usize = 8;
    #[allow(dead_code)]
    const GRID_ROWS: usize = 6;  // 总行数
    const VISIBLE_ROWS: usize = 5;  // 可见行数
    const GRID_START_X: f32 = 9.0;
    const GRID_START_Y: f32 = 37.0;
    const CELL_SPACING: f32 = 1.0;
    
    // 标签页布局参数（原版: 72x23, 位置从(6,7)开始，间距70像素）
    // ItemButton: Location=(6, 7)
    // ItemButton2: Location=(76, 7)
    // QuestButton: Location=(146, 7)
    const TAB_WIDTH: f32 = 72.0;
    const TAB_HEIGHT: f32 = 23.0;
    const TAB_START_X: f32 = 6.0;   // 原版: 6
    const TAB_START_Y: f32 = 7.0;   // 原版: 7
    const TAB_SPACING: f32 = 70.0;  // 原版间距: 76-6=70, 146-76=70
    
    pub fn new() -> Self {
        // 创建各标签页的示例物品
        let mut tab_items: [Vec<ItemSlotNative>; 3] = [
            Vec::with_capacity(46),
            Vec::with_capacity(46),
            Vec::with_capacity(46),
        ];
        
        // 装备页 - 使用装备类图标（0-19）
        for i in 0..46 {
            tab_items[0].push(ItemSlotNative::new(i % 20, (i % 5 + 1) as u32));
        }
        
        // 道具页 - 使用道具类图标（20-39）
        for i in 0..46 {
            tab_items[1].push(ItemSlotNative::new(20 + i % 20, (i % 10 + 1) as u32));
        }
        
        // 任务页 - 使用任务物品图标（40-49）
        for i in 0..20 {  // 任务物品较少
            tab_items[2].push(ItemSlotNative::new(40 + i % 10, 1));
        }
        
        Self {
            position: vec2(100.0, 100.0),
            size: vec2(312.0, 232.0), // 原版背包窗口大小
            visible: false,
            dragging_window: false,
            drag_offset: Vec2::ZERO,
            current_tab: InventoryTab::Equipment,
            tab_items,
            gold: 999999,
            selected_slot: None,
            drag_state: None,
            hovered_slot: None,
            hovered_tab: None,
            bg_texture: None,
            close_textures: [None, None, None],
            tab_textures: [[None, None], [None, None], [None, None]],
            item_textures: HashMap::new(),
            scroll_offsets: [0.0, 0.0, 0.0],
            visible_height: Self::VISIBLE_ROWS as f32 * (Self::CELL_HEIGHT + Self::CELL_SPACING),
        }
    }
    
    /// 获取当前标签页的物品
    fn current_items(&self) -> &Vec<ItemSlotNative> {
        &self.tab_items[self.current_tab as usize]
    }
    
    /// 获取当前标签页的物品（可变）
    fn current_items_mut(&mut self) -> &mut Vec<ItemSlotNative> {
        let idx = self.current_tab as usize;
        &mut self.tab_items[idx]
    }
    
    /// 获取当前标签页的滚动偏移
    fn current_scroll(&self) -> f32 {
        self.scroll_offsets[self.current_tab as usize]
    }
    
    /// 设置当前标签页的滚动偏移
    fn set_current_scroll(&mut self, offset: f32) {
        let idx = self.current_tab as usize;
        self.scroll_offsets[idx] = offset;
    }
    
    /// 加载纹理
    pub async fn load_textures(&mut self) {
        println!("📦 InventoryDialogNative: 加载纹理...");
        
        // 加载背景纹理 Title[196]
        if let Some(info) = LibraryName::Title.get_texture(196) {
            self.size = vec2(info.width as f32, info.height as f32);
            if let Some(tex) = info.image {
                self.bg_texture = Some(tex);
                println!("  ✅ 背景纹理加载成功: {}x{}", info.width, info.height);
            }
        }
        
        // 加载关闭按钮纹理 Prguse2[360/361/362]
        for i in 0..3 {
            if let Some(info) = LibraryName::Prguse2.get_texture(360 + i) {
                if let Some(tex) = info.image {
                    self.close_textures[i] = Some(tex);
                }
            }
        }
        println!("  ✅ 关闭按钮纹理加载成功");
        
        // 加载标签页纹理
        // 标签配置：[普通状态索引, 选中状态索引]
        // - 物品1: 普通 Title[737], 选中 Title[197]
        // - 物品2: 普通 Title[738], 选中 Title[168]
        // - 任务:  普通 Title[739], 选中 Title[198]
        let tab_indices: [[usize; 2]; 3] = [
            [737, 197],  // 装备/物品1
            [738, 168],  // 道具/物品2
            [739, 198],  // 任务
        ];
        
        for (tab_idx, indices) in tab_indices.iter().enumerate() {
            for (state_idx, tex_idx) in indices.iter().enumerate() {
                if let Some(info) = LibraryName::Title.get_texture(*tex_idx) {
                    if let Some(tex) = info.image {
                        self.tab_textures[tab_idx][state_idx] = Some(tex);
                    }
                }
            }
        }
        println!("  ✅ 标签页纹理加载成功");
        
        // 预加载一些物品图标
        for i in 0..60 {
            if let Some(info) = LibraryName::Items.get_texture(i) {
                if let Some(tex) = info.image {
                    self.item_textures.insert(i, tex);
                }
            }
        }
        println!("  ✅ 物品图标纹理加载成功");
    }
    
    /// 打开对话框
    pub fn open(&mut self) {
        if !self.visible {
            self.visible = true;
            println!("📦 背包对话框: 打开 (标签: {})", self.current_tab.name());
        }
    }
    
    /// 关闭对话框
    pub fn close(&mut self) {
        if self.visible {
            self.visible = false;
            self.drag_state = None;
            self.selected_slot = None;
            println!("📦 背包对话框: 关闭");
        }
    }
    
    /// 切换显示状态
    pub fn toggle(&mut self) {
        if self.visible {
            self.close();
        } else {
            self.open();
        }
    }
    
    /// 是否可见
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    
    /// 设置窗口位置
    pub fn set_position(&mut self, pos: Vec2) {
        self.position = pos;
    }
    
    /// 获取窗口位置
    pub fn get_position(&self) -> Vec2 {
        self.position
    }
    
    /// 切换标签页
    pub fn switch_tab(&mut self, tab: InventoryTab) {
        if self.current_tab != tab {
            self.current_tab = tab;
            self.selected_slot = None;
            self.drag_state = None;
            println!("📑 切换标签页: {}", tab.name());
        }
    }
    
    /// 获取标签页矩形区域
    fn get_tab_rect(&self, tab_index: usize) -> Rect {
        // 原版位置: (6,7), (76,7), (146,7)，间距70
        let (width, height) = if let Some(ref tex) = self.tab_textures[tab_index][0] {
            (tex.width(), tex.height())
        } else {
            (Self::TAB_WIDTH, Self::TAB_HEIGHT)
        };
        
        // 使用固定间距70（原版设计）
        let x = self.position.x + Self::TAB_START_X + tab_index as f32 * Self::TAB_SPACING;
        let y = self.position.y + Self::TAB_START_Y;
        Rect::new(x, y, width, height)
    }

    /// 获取格子在屏幕上的矩形区域（考虑滚动）
    fn get_slot_rect(&self, index: usize) -> Rect {
        let col = index % Self::GRID_COLS;
        let row = index / Self::GRID_COLS;
        
        let x = self.position.x + Self::GRID_START_X + col as f32 * (Self::CELL_WIDTH + Self::CELL_SPACING);
        let y = self.position.y + Self::GRID_START_Y + row as f32 * (Self::CELL_HEIGHT + Self::CELL_SPACING) - self.current_scroll();
        
        Rect::new(x, y, Self::CELL_WIDTH, Self::CELL_HEIGHT)
    }
    
    /// 获取可见区域
    fn get_visible_area(&self) -> Rect {
        Rect::new(
            self.position.x + Self::GRID_START_X,
            self.position.y + Self::GRID_START_Y,
            Self::GRID_COLS as f32 * (Self::CELL_WIDTH + Self::CELL_SPACING),
            self.visible_height
        )
    }
    
    /// 计算最大滚动值
    fn max_scroll(&self) -> f32 {
        let total_rows = (self.current_items().len() + Self::GRID_COLS - 1) / Self::GRID_COLS;
        let total_height = total_rows as f32 * (Self::CELL_HEIGHT + Self::CELL_SPACING);
        (total_height - self.visible_height).max(0.0)
    }
    
    /// 根据屏幕坐标获取格子索引（只返回可见区域内的）
    fn get_slot_at_pos(&self, pos: Vec2) -> Option<usize> {
        let visible_area = self.get_visible_area();
        
        // 首先检查是否在可见区域内
        if !visible_area.contains(pos) {
            return None;
        }
        
        let items = self.current_items();
        for i in 0..items.len() {
            let rect = self.get_slot_rect(i);
            
            // 检查格子是否在可见区域内
            if rect.y + rect.h <= visible_area.y || rect.y >= visible_area.y + visible_area.h {
                continue;
            }
            
            if rect.contains(pos) {
                return Some(i);
            }
        }
        None
    }
    
    /// 根据屏幕坐标获取标签页索引
    fn get_tab_at_pos(&self, pos: Vec2) -> Option<usize> {
        for i in 0..3 {
            let rect = self.get_tab_rect(i);
            if rect.contains(pos) {
                return Some(i);
            }
        }
        None
    }
    
    /// 绘制和更新（主循环中调用）
    pub fn update_and_draw(&mut self) {
        if !self.visible {
            return;
        }
        
        let mouse_pos = mouse_position();
        let mouse_pos = vec2(mouse_pos.0, mouse_pos.1);
        
        // ========== 输入处理 ==========
        
        // ESC 关闭
        if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::I) {
            self.close();
            return;
        }
        
        // 数字键切换标签页
        if is_key_pressed(KeyCode::Key1) {
            self.switch_tab(InventoryTab::Equipment);
        }
        if is_key_pressed(KeyCode::Key2) {
            self.switch_tab(InventoryTab::Items);
        }
        if is_key_pressed(KeyCode::Key3) {
            self.switch_tab(InventoryTab::Quest);
        }
        
        // 窗口拖动 - 标签页右侧到关闭按钮左侧的区域
        // 第3个标签结束位置: 146 + 72 = 218
        // 关闭按钮开始位置: 289
        // 可拖动区域: 218 ~ 289
        let drag_start_x = Self::TAB_START_X + 2.0 * Self::TAB_SPACING + Self::TAB_WIDTH; // 6 + 140 + 72 = 218
        let title_bar = Rect::new(
            self.position.x + drag_start_x, 
            self.position.y, 
            289.0 - drag_start_x,  // 289 - 218 = 71
            35.0
        );
        
        // 更新悬停的标签页
        self.hovered_tab = self.get_tab_at_pos(mouse_pos);
        
        if is_mouse_button_pressed(MouseButton::Left) {
            // 检查是否点击标签页
            if let Some(tab_idx) = self.hovered_tab {
                self.switch_tab(InventoryTab::from_index(tab_idx));
            } else if title_bar.contains(mouse_pos) {
                self.dragging_window = true;
                self.drag_offset = mouse_pos - self.position;
            }
        }
        
        if is_mouse_button_released(MouseButton::Left) {
            self.dragging_window = false;
        }
        
        if self.dragging_window {
            self.position = mouse_pos - self.drag_offset;
        }
        
        // 关闭按钮检测 - 原版: Location=(289, 3), Prguse2[360]
        let close_btn_rect = Rect::new(
            self.position.x + 289.0,
            self.position.y + 3.0,
            20.0, 20.0
        );
        let close_hovered = close_btn_rect.contains(mouse_pos);
        let close_pressed = close_hovered && is_mouse_button_down(MouseButton::Left);
        
        if close_hovered && is_mouse_button_released(MouseButton::Left) {
            self.close();
            return;
        }
        
        // 更新悬停状态
        self.hovered_slot = self.get_slot_at_pos(mouse_pos);
        
        // 物品点击/拖放处理
        if is_mouse_button_pressed(MouseButton::Left) {
            if let Some(slot_idx) = self.hovered_slot {
                // 检查是否有物品
                let items = self.current_items();
                if let Some(ref slot) = items.get(slot_idx) {
                    if let Some(icon_index) = slot.icon_index {
                        // 开始拖动
                        self.drag_state = Some(DragState {
                            source_index: slot_idx,
                            icon_index,
                            count: slot.count,
                        });
                        self.selected_slot = Some(slot_idx);
                        println!("🖱️ 开始拖动物品: 格子{}, 图标{}", slot_idx, icon_index);
                    }
                }
            }
        }
        
        if is_mouse_button_released(MouseButton::Left) {
            if let Some(drag) = self.drag_state.take() {
                // 放下物品
                if let Some(target_idx) = self.hovered_slot {
                    if target_idx != drag.source_index {
                        // 交换物品
                        self.swap_items(drag.source_index, target_idx);
                    }
                }
                self.selected_slot = None;
            }
        }
        
        // 右键取消选择
        if is_mouse_button_pressed(MouseButton::Right) {
            self.drag_state = None;
            self.selected_slot = None;
        }
        
        // 鼠标滚轮滚动
        let visible_area = self.get_visible_area();
        if visible_area.contains(mouse_pos) {
            let wheel = mouse_wheel().1;
            if wheel != 0.0 {
                let current = self.current_scroll();
                let max = self.max_scroll();
                let new_scroll = (current - wheel * 20.0).clamp(0.0, max);
                self.set_current_scroll(new_scroll);
            }
        }
        
        // ========== 绘制 ==========
        
        // 绘制背景
        if let Some(ref bg) = self.bg_texture {
            draw_texture(bg, self.position.x, self.position.y, WHITE);
        } else {
            // 备用：绘制纯色背景
            draw_rectangle(
                self.position.x, self.position.y,
                self.size.x, self.size.y,
                Color::from_rgba(40, 40, 50, 240)
            );
            // 绘制边框
            draw_rectangle_lines(
                self.position.x, self.position.y,
                self.size.x, self.size.y,
                2.0, Color::from_rgba(100, 100, 120, 255)
            );
        }
        
        // 绘制标签页
        self.draw_tabs(mouse_pos);
        
        // 绘制关闭按钮
        let close_tex_idx = if close_pressed { 2 } else if close_hovered { 1 } else { 0 };
        if let Some(ref tex) = self.close_textures[close_tex_idx] {
            draw_texture(tex, close_btn_rect.x, close_btn_rect.y, WHITE);
        } else {
            // 备用：绘制文字X
            let color = if close_pressed {
                RED
            } else if close_hovered {
                YELLOW
            } else {
                WHITE
            };
            draw_text("X", close_btn_rect.x + 5.0, close_btn_rect.y + 15.0, 20.0, color);
        }
        
        // 绘制物品格子（只绘制可见区域内的）
        let visible_area = self.get_visible_area();
        let items = self.current_items();
        for i in 0..items.len() {
            let rect = self.get_slot_rect(i);
            
            // 跳过不在可见区域内的格子
            if rect.y + rect.h <= visible_area.y || rect.y >= visible_area.y + visible_area.h {
                continue;
            }
            
            let slot = &items[i];
            
            // 绘制格子边框
            let is_selected = self.selected_slot == Some(i);
            let is_hovered = self.hovered_slot == Some(i);
            let is_drag_target = self.drag_state.is_some() && is_hovered && !is_selected;
            
            let border_color = if is_selected {
                Color::from_rgba(255, 255, 0, 255) // 黄色 - 选中
            } else if is_drag_target {
                Color::from_rgba(0, 255, 255, 255) // 青色 - 拖放目标
            } else if is_hovered {
                Color::from_rgba(0, 255, 0, 255) // 绿色 - 悬停
            } else {
                Color::from_rgba(80, 80, 80, 128) // 默认
            };
            
            draw_rectangle_lines(
                rect.x, rect.y,
                rect.w, rect.h,
                if is_selected || is_hovered || is_drag_target { 2.0 } else { 1.0 },
                border_color
            );
            
            // 绘制物品图标
            if let Some(icon_index) = slot.icon_index {
                // 如果正在被拖动，绘制半透明
                let alpha = if is_selected && self.drag_state.is_some() {
                    0.5
                } else {
                    1.0
                };
                
                if let Some(tex) = self.item_textures.get(&icon_index) {
                    let icon_size = vec2(tex.width(), tex.height());
                    let offset_x = (rect.w - icon_size.x) / 2.0;
                    let offset_y = (rect.h - icon_size.y) / 2.0;
                    
                    draw_texture(
                        tex, 
                        rect.x + offset_x, 
                        rect.y + offset_y, 
                        Color::from_rgba(255, 255, 255, (255.0 * alpha) as u8)
                    );
                }
                
                // 绘制数量
                if slot.count > 1 {
                    let count_text = format!("{}", slot.count);
                    let text_x = rect.x + rect.w - 5.0;
                    let text_y = rect.y + rect.h - 2.0;
                    
                    // 文字阴影
                    draw_text(&count_text, text_x + 1.0, text_y + 1.0, 14.0, BLACK);
                    draw_text(&count_text, text_x, text_y, 14.0, WHITE);
                }
            }
        }
        
        // 绘制金币 - 原版: Location=(40, 212), Size=(111,14)
        let gold_text = format!("{}", self.gold);
        draw_text(
            &gold_text,
            self.position.x + 40.0,
            self.position.y + 212.0 + 12.0,  // +12 因为 draw_text 从基线绘制
            14.0,
            Color::from_rgba(255, 215, 0, 255) // 金色
        );
        
        // 绘制正在拖动的物品（跟随鼠标）
        if let Some(ref drag) = self.drag_state {
            if let Some(tex) = self.item_textures.get(&drag.icon_index) {
                let icon_size = vec2(tex.width(), tex.height());
                draw_texture(
                    tex,
                    mouse_pos.x - icon_size.x / 2.0,
                    mouse_pos.y - icon_size.y / 2.0,
                    WHITE
                );
                
                // 绘制数量
                if drag.count > 1 {
                    let count_text = format!("{}", drag.count);
                    draw_text(&count_text, mouse_pos.x + 10.0, mouse_pos.y + 15.0, 14.0, WHITE);
                }
            }
        }
        
        // 绘制提示文本
        if let Some(slot_idx) = self.hovered_slot {
            if self.drag_state.is_none() {
                let items = self.current_items();
                if let Some(slot) = items.get(slot_idx) {
                    if let Some(icon_index) = slot.icon_index {
                        let tab_name = self.current_tab.name();
                        let tooltip = format!("[{}] 物品 {} (数量: {})", tab_name, icon_index, slot.count);
                        let tooltip_x = mouse_pos.x + 15.0;
                        let tooltip_y = mouse_pos.y - 5.0;
                        
                        // 背景
                        let text_width = tooltip.len() as f32 * 8.0;
                        draw_rectangle(
                            tooltip_x - 2.0, tooltip_y - 14.0,
                            text_width + 4.0, 18.0,
                            Color::from_rgba(0, 0, 0, 200)
                        );
                        draw_text(&tooltip, tooltip_x, tooltip_y, 14.0, WHITE);
                    }
                }
            }
        }
    }
    
    /// 绘制标签页（使用纹理）
    fn draw_tabs(&self, _mouse_pos: Vec2) {
        for i in 0..3 {
            let rect = self.get_tab_rect(i);
            let tab = InventoryTab::from_index(i);
            let is_current = self.current_tab == tab;
            let is_hovered = self.hovered_tab == Some(i);
            
            // 确定使用哪个纹理状态：0=普通，1=选中（悬停时也显示选中）
            let state_idx = if is_current || is_hovered { 1 } else { 0 };
            
            // 尝试绘制纹理
            if let Some(ref tex) = self.tab_textures[i][state_idx] {
                draw_texture(tex, rect.x, rect.y, WHITE);
            } else {
                // 备用：绘制纯色背景
                let bg_color = if is_current {
                    Color::from_rgba(80, 80, 120, 255)
                } else if is_hovered {
                    Color::from_rgba(60, 60, 90, 255)
                } else {
                    Color::from_rgba(40, 40, 60, 200)
                };
                
                draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg_color);
                
                let border_color = if is_current {
                    Color::from_rgba(200, 200, 255, 255)
                } else if is_hovered {
                    Color::from_rgba(150, 150, 200, 255)
                } else {
                    Color::from_rgba(100, 100, 150, 200)
                };
                draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, border_color);
                
                // 标签页文字
                let text_color = if is_current {
                    WHITE
                } else if is_hovered {
                    Color::from_rgba(220, 220, 255, 255)
                } else {
                    Color::from_rgba(180, 180, 200, 255)
                };
                
                let text = tab.name();
                let text_width = text.len() as f32 * 7.0;
                let text_x = rect.x + (rect.w - text_width) / 2.0;
                let text_y = rect.y + rect.h - 5.0;
                
                draw_text(text, text_x, text_y, 14.0, text_color);
            }
        }
    }
    
    /// 交换两个格子的物品
    fn swap_items(&mut self, idx1: usize, idx2: usize) {
        let items = self.current_items_mut();
        if idx1 >= items.len() || idx2 >= items.len() {
            return;
        }
        
        // 使用 swap 方法交换
        items.swap(idx1, idx2);
        
        println!("🔄 交换物品: 格子{} <-> 格子{}", idx1, idx2);
    }
    
    /// 获取窗口矩形区域（用于遮挡检测）
    pub fn get_rect(&self) -> Rect {
        Rect::new(self.position.x, self.position.y, self.size.x, self.size.y)
    }
    
    /// 检查点是否在对话框内
    pub fn contains(&self, pos: Vec2) -> bool {
        self.visible && self.get_rect().contains(pos)
    }
    
    /// 为兼容性保留 item_slots 访问（返回当前标签页）
    pub fn item_slots(&self) -> &Vec<ItemSlotNative> {
        self.current_items()
    }
}
