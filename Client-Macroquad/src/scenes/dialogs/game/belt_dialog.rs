// ============================================================================
// BeltDialogHybrid - 快捷栏（混合版本）
// ============================================================================
//
// 结合两种实现方式的优点：
// - native 绘制：背景、物品图标、按钮、数字提示（精确像素控制）
// - mqui Group：物品拖放交换（利用内置 draggable API）
//
// 优势：
// 1. 背景和按钮使用原版纹理，像素级对齐
// 2. 物品拖放使用 macroquad::ui 的 Group::draggable()
// 3. 拖动时物品跟随鼠标，可视反馈好
// 4. 支持物品交换、丢弃
//
// ============================================================================

use macroquad::prelude::*;
use macroquad::ui::{hash, root_ui, widgets::Group, Drag, Skin};
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::*;

/// 快捷栏布局模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BeltLayoutHybrid {
    Horizontal,
    Vertical,
}

/// 快捷栏格子物品
#[derive(Debug, Clone, Copy)]
pub struct BeltItemHybrid {
    pub icon_index: usize,
    pub count: u32,
}

impl BeltItemHybrid {
    pub fn new(icon_index: usize, count: u32) -> Self {
        Self { icon_index, count }
    }
}

/// 拖放命令（延迟执行，避免借用问题）
#[derive(Debug)]
enum DragCommand {
    /// 使用物品（双击）
    Use { slot: usize },
    /// 交换物品
    Swap { from: usize, to: usize },
    /// 丢弃物品（拖出快捷栏）
    Drop { slot: usize },
}

/// 快捷栏对话框（混合版本）
pub struct BeltDialogHybrid {
    /// 是否可见
    visible: bool,
    /// 布局模式
    layout: BeltLayoutHybrid,
    /// 窗口位置
    position: Vec2,
    /// 水平布局时的位置（用于切换时恢复）
    horizontal_position: Vec2,
    
    // === Native 绘制资源 ===
    /// 水平背景
    bg_horizontal: BackgroundTexture,
    /// 垂直背景
    bg_vertical: BackgroundTexture,
    /// 水平旋转按钮
    rotate_btn_h: ButtonTextures,
    /// 水平关闭按钮
    close_btn_h: ButtonTextures,
    /// 垂直旋转按钮
    rotate_btn_v: ButtonTextures,
    /// 垂直关闭按钮
    close_btn_v: ButtonTextures,
    /// 物品图标缓存
    item_cache: ItemTextureCache,
    
    // === 窗口拖动 ===
    drag_helper: DragHelper,
    
    // === mqui 拖放状态 ===
    /// 是否有物品正在被拖动
    item_dragging: bool,
    /// 正在拖动的物品来源格子
    dragging_from: Option<usize>,
    /// 透明 Skin（用于 Group，不绘制任何东西）
    transparent_skin: Option<Skin>,
    
    // === 格子数据 ===
    cells: [Option<BeltItemHybrid>; 6],
    
    // === 交互状态 ===
    hovered_cell: Option<usize>,
    pending_to_inventory: Option<usize>,
    /// 上次点击时间（用于检测双击）
    last_click_time: f64,
    last_click_slot: Option<usize>,
}

impl BeltDialogHybrid {
    // 格子尺寸（与原版一致）
    const CELL_SIZE: f32 = 32.0;
    const CELL_SPACING: f32 = 35.0;
    const CELL_OFFSET: f32 = 12.0;
    const DOUBLE_CLICK_TIME: f64 = 0.3;
    
    pub fn new() -> Self {
        let position = vec2(400.0, 600.0);
        
        // 初始化示例物品
        let cells = [
            Some(BeltItemHybrid::new(0, 15)),
            Some(BeltItemHybrid::new(1, 8)),
            Some(BeltItemHybrid::new(2, 12)),
            Some(BeltItemHybrid::new(3, 6)),
            Some(BeltItemHybrid::new(5, 3)),
            Some(BeltItemHybrid::new(6, 2)),
        ];
        
        Self {
            visible: true,
            layout: BeltLayoutHybrid::Horizontal,
            position,
            horizontal_position: position,
            
            bg_horizontal: BackgroundTexture::new(),
            bg_vertical: BackgroundTexture::new(),
            rotate_btn_h: ButtonTextures::new(),
            close_btn_h: ButtonTextures::new(),
            rotate_btn_v: ButtonTextures::new(),
            close_btn_v: ButtonTextures::new(),
            item_cache: ItemTextureCache::new(),
            
            drag_helper: DragHelper::new(),
            
            item_dragging: false,
            dragging_from: None,
            transparent_skin: None,
            
            cells,
            hovered_cell: None,
            pending_to_inventory: None,
            last_click_time: 0.0,
            last_click_slot: None,
        }
    }
    
    /// 异步加载纹理
    pub  fn load_textures(&mut self) {
        println!("🎒 BeltDialogHybrid: 加载纹理...");
        
        // 水平背景（主 + 覆盖层）
        self.bg_horizontal = BackgroundTexture::load(LibraryName::Prguse, 1932, Some(1933));
        
        // 垂直背景
        self.bg_vertical = BackgroundTexture::load(LibraryName::Prguse, 1944, Some(1945));
        
        // 水平按钮
        self.rotate_btn_h = ButtonTextures::load_from_indices(LibraryName::Prguse, [1926, 1927, 1928]);
        self.close_btn_h = ButtonTextures::load_from_indices(LibraryName::Prguse, [1923, 1924, 1925]);
        
        // 垂直按钮
        self.rotate_btn_v = ButtonTextures::load_from_indices(LibraryName::Prguse, [1938, 1939, 1940]);
        self.close_btn_v = ButtonTextures::load_from_indices(LibraryName::Prguse, [1935, 1936, 1937]);
        
        // 预加载物品图标
        self.item_cache.preload(LibraryName::Items, 0, 20);
        
        // 创建透明 Skin（用于 Group 拖放，不显示任何背景）
        self.create_transparent_skin();
        
        println!("  ✅ 混合版快捷栏纹理加载成功");
    }
    
    /// 创建透明 Skin
    fn create_transparent_skin(&mut self) {
        // 创建 1x1 透明像素
        let transparent_pixel = Image {
            bytes: vec![0, 0, 0, 0],
            width: 1,
            height: 1,
        };
        
        // 完全透明的样式，包括边框
        let transparent_style = root_ui()
            .style_builder()
            .background(transparent_pixel.clone())
            .background_hovered(transparent_pixel.clone())
            .background_clicked(transparent_pixel.clone())
            .color(Color::new(0.0, 0.0, 0.0, 0.0))
            .color_hovered(Color::new(0.0, 0.0, 0.0, 0.0))
            .color_clicked(Color::new(0.0, 0.0, 0.0, 0.0))
            .build();
        
        self.transparent_skin = Some(Skin {
            group_style: transparent_style.clone(),
            button_style: transparent_style.clone(),
            label_style: transparent_style,
            ..root_ui().default_skin()
        });
    }
    
    /// 获取当前背景
    fn current_bg(&self) -> &BackgroundTexture {
        match self.layout {
            BeltLayoutHybrid::Horizontal => &self.bg_horizontal,
            BeltLayoutHybrid::Vertical => &self.bg_vertical,
        }
    }
    
    /// 获取当前尺寸
    pub fn get_size(&self) -> Vec2 {
        self.current_bg().size
    }
    
    /// 设置位置
    pub fn set_position(&mut self, pos: Vec2) {
        self.position = pos;
        if self.layout == BeltLayoutHybrid::Horizontal {
            self.horizontal_position = pos;
        }
    }
    
    /// 获取位置
    pub fn get_position(&self) -> Vec2 {
        self.position
    }

    pub fn is_horizontal_layout(&self) -> bool {
        self.layout == BeltLayoutHybrid::Horizontal
    }
    
    pub fn open(&mut self) {
        if !self.visible {
            self.visible = true;
            println!("🎒 快捷栏: 打开");
        }
    }
    
    pub fn close(&mut self) {
        if self.visible {
            self.visible = false;
            self.item_dragging = false;
            self.dragging_from = None;
            println!("🎒 快捷栏: 关闭");
        }
    }
    
    pub fn toggle(&mut self) {
        if self.visible { self.close(); } else { self.open(); }
    }
    
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    
    /// 切换布局
    pub fn flip_layout(&mut self) {
        self.layout = match self.layout {
            BeltLayoutHybrid::Horizontal => {
                self.horizontal_position = self.position;
                self.position = vec2(0.0, 200.0);
                println!("🔄 快捷栏: 切换到垂直布局");
                BeltLayoutHybrid::Vertical
            }
            BeltLayoutHybrid::Vertical => {
                self.position = self.horizontal_position;
                println!("🔄 快捷栏: 切换到水平布局");
                BeltLayoutHybrid::Horizontal
            }
        };
    }
    
    /// 获取格子屏幕位置
    fn get_cell_position(&self, index: usize) -> Vec2 {
        match self.layout {
            BeltLayoutHybrid::Horizontal => vec2(
                self.position.x + (index as f32) * Self::CELL_SPACING + Self::CELL_OFFSET,
                self.position.y + 3.0
            ),
            BeltLayoutHybrid::Vertical => vec2(
                self.position.x + 3.0,
                self.position.y + (index as f32) * Self::CELL_SPACING + Self::CELL_OFFSET
            ),
        }
    }
    
    /// 获取格子矩形
    fn get_cell_rect(&self, index: usize) -> Rect {
        let pos = self.get_cell_position(index);
        Rect::new(pos.x, pos.y, Self::CELL_SIZE, Self::CELL_SIZE)
    }
    
    /// 获取旋转按钮矩形
    fn get_rotate_button_rect(&self) -> Rect {
        let btn = self.current_rotate_btn();
        match self.layout {
            BeltLayoutHybrid::Horizontal => Rect::new(
                self.position.x + 222.0, self.position.y + 3.0,
                btn.size.x, btn.size.y
            ),
            BeltLayoutHybrid::Vertical => Rect::new(
                self.position.x + 19.0, self.position.y + 222.0,
                btn.size.x, btn.size.y
            ),
        }
    }
    
    /// 获取关闭按钮矩形
    fn get_close_button_rect(&self) -> Rect {
        let btn = self.current_close_btn();
        match self.layout {
            BeltLayoutHybrid::Horizontal => Rect::new(
                self.position.x + 222.0, self.position.y + 19.0,
                btn.size.x, btn.size.y
            ),
            BeltLayoutHybrid::Vertical => Rect::new(
                self.position.x + 3.0, self.position.y + 222.0,
                btn.size.x, btn.size.y
            ),
        }
    }
    
    fn current_rotate_btn(&self) -> &ButtonTextures {
        match self.layout {
            BeltLayoutHybrid::Horizontal => &self.rotate_btn_h,
            BeltLayoutHybrid::Vertical => &self.rotate_btn_v,
        }
    }
    
    fn current_close_btn(&self) -> &ButtonTextures {
        match self.layout {
            BeltLayoutHybrid::Horizontal => &self.close_btn_h,
            BeltLayoutHybrid::Vertical => &self.close_btn_v,
        }
    }
    
    /// 检查点是否在窗口内
    pub fn contains(&self, pos: Vec2) -> bool {
        let size = self.get_size();
        Rect::new(self.position.x, self.position.y, size.x, size.y).contains(pos)
    }

    pub fn take_transfer_to_inventory_request(&mut self) -> Option<usize> {
        self.pending_to_inventory.take()
    }

    pub fn take_item_from_slot(&mut self, slot: usize) -> Option<BeltItemHybrid> {
        if slot >= 6 {
            return None;
        }
        self.cells[slot].take()
    }

    pub fn restore_item_to_slot(&mut self, slot: usize, item: BeltItemHybrid) -> bool {
        if slot >= 6 {
            return false;
        }
        self.cells[slot] = Some(item);
        true
    }

    pub fn try_insert_item(&mut self, item: BeltItemHybrid) -> Result<(), BeltItemHybrid> {
        if let Some(stack_slot) = self
            .cells
            .iter_mut()
            .find(|s| s.as_ref().is_some_and(|existing_item| existing_item.icon_index == item.icon_index))
        {
            if let Some(existing) = stack_slot.as_mut() {
                existing.count = existing.count.saturating_add(item.count);
                return Ok(());
            }
        }

        if let Some(empty_slot) = self.cells.iter_mut().find(|s| s.is_none()) {
            *empty_slot = Some(item);
            return Ok(());
        }

        Err(item)
    }
    
    /// 更新和绘制（主入口）
    pub fn update_and_draw(&mut self) -> bool {
        if !self.visible {
            return false;
        }
        
        let mouse = mouse_pos();
        let current_time = get_time();
        
        // ========== 1. 更新悬停状态 ==========
        self.hovered_cell = (0..6).find(|&i| self.get_cell_rect(i).contains(mouse));
        
        let rotate_rect = self.get_rotate_button_rect();
        let close_rect = self.get_close_button_rect();
        let hovered_rotate = rotate_rect.contains(mouse);
        let hovered_close = close_rect.contains(mouse);
        
        // ========== 2. 处理按钮点击 ==========
        if is_mouse_button_pressed(MouseButton::Left) {
            if hovered_rotate {
                self.flip_layout();
            } else if hovered_close {
                self.close();
                return false;
            }
        }
        if is_mouse_button_pressed(MouseButton::Right) && !self.item_dragging {
            if let Some(slot_idx) = self.hovered_cell {
                if self.cells[slot_idx].is_some() {
                    self.pending_to_inventory = Some(slot_idx);
                }
            }
        }
        
        // ========== 3. 处理窗口拖动（排除格子和按钮区域）==========
        let can_drag_window = !hovered_rotate && !hovered_close && self.hovered_cell.is_none() && !self.item_dragging;
        if can_drag_window {
            let drag_area = Rect::new(
                self.position.x, self.position.y,
                self.get_size().x - 20.0, self.get_size().y
            );
            self.drag_helper.apply(drag_area, &mut self.position);
            if self.layout == BeltLayoutHybrid::Horizontal && self.drag_helper.dragging {
                self.horizontal_position = self.position;
            }
        } else if is_mouse_button_released(MouseButton::Left) && !self.item_dragging {
            self.drag_helper.dragging = false;
        }
        
        // ========== 4. Native 绘制背景 ==========
        self.current_bg().draw(self.position);
        
        // ========== 5. 收集数据用于 mqui 拖放 ==========
        let cells_snapshot: [Option<BeltItemHybrid>; 6] = self.cells;
        let item_dragging = self.item_dragging;
        let _layout = self.layout;
        
        // ========== 6. mqui Group 拖放处理 ==========
        let mut drag_command: Option<DragCommand> = None;
        let mut new_item_dragging = false;
        let mut new_dragging_from: Option<usize> = None;
        
        // 应用透明 Skin
        if let Some(ref skin) = self.transparent_skin {
            root_ui().push_skin(skin);
        }
        
        for i in 0..6 {
            let rect = self.get_cell_rect(i);
            let has_item = cells_snapshot[i].is_some();
            let slot_id = hash!("belt_hybrid_slot", i);
            
                    // 使用 Group 实现拖放
                    let drag = Group::new(slot_id, vec2(Self::CELL_SIZE, Self::CELL_SIZE))
                        .position(vec2(rect.x, rect.y))
                        .draggable(has_item)           // 有物品才能拖
                        .hoverable(item_dragging)      // 拖动时可作为放置目标
                        .ui(&mut root_ui(), |_ui| {
                            // 不在这里绘制任何东西，全部用 native 绘制
                        });            // 处理拖放事件
            match drag {
                Drag::Dragging(_, _) => {
                    new_item_dragging = true;
                    if self.dragging_from.is_none() {
                        new_dragging_from = Some(i);
                    } else {
                        new_dragging_from = self.dragging_from;
                    }
                }
                Drag::Dropped(_, Some(target_id)) if has_item => {
                    // 拖到了另一个格子
                    for j in 0..6 {
                        if hash!("belt_hybrid_slot", j) == target_id && j != i {
                            drag_command = Some(DragCommand::Swap { from: i, to: j });
                            break;
                        }
                    }
                }
                Drag::Dropped(drop_pos, None) if has_item => {
                    // 检查是否拖出了快捷栏区域
                    let window_rect = Rect::new(
                        self.position.x, self.position.y,
                        self.get_size().x, self.get_size().y
                    );
                    if !window_rect.contains(drop_pos) {
                        drag_command = Some(DragCommand::Drop { slot: i });
                    }
                }
                _ => {}
            }
        }
        
        // 恢复默认 Skin
        if self.transparent_skin.is_some() {
            root_ui().pop_skin();
        }
        
        // 更新拖动状态
        self.item_dragging = new_item_dragging;
        self.dragging_from = new_dragging_from;
        
        // ========== 7. Native 绘制格子和物品 ==========
        self.draw_cells(mouse);
        
        // ========== 8. Native 绘制按钮 ==========
        self.draw_buttons(mouse);
        
        // ========== 9. 绘制拖动中的物品（跟随鼠标）==========
        if self.item_dragging {
            if let Some(from) = self.dragging_from {
                if let Some(item) = &self.cells[from] {
                    if let Some(tex) = self.item_cache.get_cached(item.icon_index) {
                        let icon_size = vec2(tex.width(), tex.height());
                        // 物品图标跟随鼠标，居中
                        draw_texture(
                            tex,
                            mouse.x - icon_size.x / 2.0,
                            mouse.y - icon_size.y / 2.0,
                            WHITE
                        );
                        // 绘制数量
                        if item.count > 1 {
                            let count_text = format!("{}", item.count);
                            draw_text_cn(&count_text, mouse.x + 10.0, mouse.y + 10.0, 14.0, WHITE);
                        }
                    }
                }
            }
        }
        
        // ========== 10. 处理双击使用物品 ==========
        if is_mouse_button_pressed(MouseButton::Left) && !self.item_dragging {
            if let Some(slot) = self.hovered_cell {
                if self.last_click_slot == Some(slot) && 
                   current_time - self.last_click_time < Self::DOUBLE_CLICK_TIME 
                {
                    drag_command = Some(DragCommand::Use { slot });
                    self.last_click_slot = None;
                } else {
                    self.last_click_slot = Some(slot);
                    self.last_click_time = current_time;
                }
            }
        }
        
        // ========== 11. 执行拖放命令 ==========
        match drag_command {
            Some(DragCommand::Use { slot }) => {
                self.use_item(slot);
            }
            Some(DragCommand::Swap { from, to }) => {
                println!("🔄 交换物品: 格子{} <-> 格子{}", from + 1, to + 1);
                self.cells.swap(from, to);
            }
            Some(DragCommand::Drop { slot }) => {
                println!("🗑️ 丢弃物品: 格子{}", slot + 1);
                self.cells[slot] = None;
            }
            None => {}
        }
        
        true
    }
    
    /// Native 绘制物品格子
    fn draw_cells(&self, mouse: Vec2) {
        for i in 0..6 {
            let rect = self.get_cell_rect(i);
            
            // 确定高亮状态
            let highlight = if self.item_dragging && self.dragging_from == Some(i) {
                CellHighlight::Selected  // 源格子显示选中状态
            } else if self.item_dragging && rect.contains(mouse) && self.dragging_from != Some(i) {
                CellHighlight::DragTarget  // 目标格子
            } else if rect.contains(mouse) {
                CellHighlight::Hovered
            } else {
                CellHighlight::None
            };
            
            // 只有高亮时才绘制边框（背景纹理已有网格）
            if highlight != CellHighlight::None {
                let color = match highlight {
                    CellHighlight::Hovered => Color::from_rgba(0, 255, 0, 255),
                    CellHighlight::Selected => Color::from_rgba(255, 255, 0, 255),
                    CellHighlight::DragTarget => Color::from_rgba(0, 255, 255, 255),
                    CellHighlight::None => unreachable!(),
                };
                draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, color);
            }
            
            // 绘制物品图标（拖动源格子显示半透明）
            if let Some(item) = &self.cells[i] {
                let alpha = if self.item_dragging && self.dragging_from == Some(i) {
                    0.4  // 源格子半透明
                } else {
                    1.0
                };
                
                if let Some(tex) = self.item_cache.get_cached(item.icon_index) {
                    draw_item_icon(rect, tex, alpha);
                }
                
                // 数量（源格子拖动时不显示数量）
                if !(self.item_dragging && self.dragging_from == Some(i)) {
                    draw_item_count(rect, item.count, false);
                }
            }
            
            // 数字键提示
            let key_text = format!("{}", i + 1);
            let (key_x, key_y) = match self.layout {
                BeltLayoutHybrid::Horizontal => (rect.x + 12.0, rect.y - 2.0),
                BeltLayoutHybrid::Vertical => (rect.x - 12.0, rect.y + 20.0),
            };
            draw_text_cn(&key_text, key_x, key_y, 14.0, YELLOW);
        }
    }
    
    /// Native 绘制按钮
    fn draw_buttons(&self, mouse: Vec2) {
        let rotate_rect = self.get_rotate_button_rect();
        self.current_rotate_btn().draw(
            vec2(rotate_rect.x, rotate_rect.y),
            ButtonState::from_mouse(rotate_rect, mouse)
        );
        
        let close_rect = self.get_close_button_rect();
        self.current_close_btn().draw(
            vec2(close_rect.x, close_rect.y),
            ButtonState::from_mouse(close_rect, mouse)
        );
    }
    
    /// 使用物品
    pub fn use_item(&mut self, slot: usize) {
        if slot < 6 {
            if let Some(item) = &mut self.cells[slot] {
                if item.count > 0 {
                    item.count -= 1;
                    println!("🧪 使用物品: 格子{}, 剩余{}", slot + 1, item.count);
                    if item.count == 0 {
                        self.cells[slot] = None;
                    }
                }
            }
        }
    }
    
    /// 设置格子物品
    pub fn set_item(&mut self, slot: usize, item: Option<BeltItemHybrid>) {
        if slot < 6 { self.cells[slot] = item; }
    }
    
    /// 获取格子物品
    pub fn get_item(&self, slot: usize) -> Option<&BeltItemHybrid> {
        if slot < 6 { self.cells[slot].as_ref() } else { None }
    }
}

impl Default for BeltDialogHybrid {
    fn default() -> Self { Self::new() }
}
