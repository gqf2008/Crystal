// ============================================================================
// BeltDialogNative - 快捷栏（macroquad 原生 UI 版本）
// ============================================================================
//
// 6个物品格子的快捷栏，支持水平/垂直布局切换
// 按数字键1-6可以快速使用对应格子的物品
//
// 使用 native_ui_utils 公共组件重构
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use super::native_ui_utils::*;

/// 快捷栏布局模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BeltLayout {
    /// 水平布局（默认）
    Horizontal,
    /// 垂直布局
    Vertical,
}

/// 快捷栏格子物品
#[derive(Debug, Clone, Copy)]
pub struct BeltItem {
    /// 物品图标索引（Items库）
    pub icon_index: usize,
    /// 物品数量
    pub count: u32,
}

impl BeltItem {
    pub fn new(icon_index: usize, count: u32) -> Self {
        Self { icon_index, count }
    }
}

/// 快捷栏对话框（原生UI版本）
pub struct BeltDialogNative {
    /// 是否可见
    visible: bool,
    /// 布局模式
    layout: BeltLayout,
    /// 窗口位置
    position: Vec2,
    /// 水平布局时的位置（用于切换时恢复）
    horizontal_position: Vec2,
    
    // 纹理缓存（使用公共组件）
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
    
    // 拖动
    drag_helper: DragHelper,
    
    // 格子数据
    cells: [Option<BeltItem>; 6],
    
    // 悬停状态
    hovered_cell: Option<usize>,
}

impl BeltDialogNative {
    // 格子尺寸
    const CELL_SIZE: f32 = 32.0;
    const CELL_SPACING: f32 = 35.0;
    const CELL_OFFSET: f32 = 12.0;
    
    pub fn new() -> Self {
        let position = vec2(400.0, 600.0);
        
        // 初始化示例物品
        let cells = [
            Some(BeltItem::new(0, 15)),
            Some(BeltItem::new(1, 8)),
            Some(BeltItem::new(2, 12)),
            Some(BeltItem::new(3, 6)),
            Some(BeltItem::new(5, 3)),
            Some(BeltItem::new(6, 2)),
        ];
        
        Self {
            visible: true,
            layout: BeltLayout::Horizontal,
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
            
            cells,
            hovered_cell: None,
        }
    }
    
    /// 异步加载纹理
    pub async fn load_textures(&mut self) {
        println!("🎒 BeltDialogNative: 加载纹理...");
        
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
        
        println!("  ✅ 快捷栏纹理加载成功");
    }
    
    /// 获取当前背景
    fn current_bg(&self) -> &BackgroundTexture {
        match self.layout {
            BeltLayout::Horizontal => &self.bg_horizontal,
            BeltLayout::Vertical => &self.bg_vertical,
        }
    }
    
    /// 获取当前尺寸
    pub fn get_size(&self) -> Vec2 {
        self.current_bg().size
    }
    
    /// 设置位置
    pub fn set_position(&mut self, pos: Vec2) {
        self.position = pos;
        if self.layout == BeltLayout::Horizontal {
            self.horizontal_position = pos;
        }
    }
    
    /// 获取位置
    pub fn get_position(&self) -> Vec2 {
        self.position
    }
    
    /// 打开
    pub fn open(&mut self) {
        if !self.visible {
            self.visible = true;
            println!("🎒 快捷栏: 打开");
        }
    }
    
    /// 关闭
    pub fn close(&mut self) {
        if self.visible {
            self.visible = false;
            println!("🎒 快捷栏: 关闭");
        }
    }
    
    /// 切换显示
    pub fn toggle(&mut self) {
        if self.visible { self.close(); } else { self.open(); }
    }
    
    /// 是否可见
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    
    /// 切换布局
    pub fn flip_layout(&mut self) {
        self.layout = match self.layout {
            BeltLayout::Horizontal => {
                self.horizontal_position = self.position;
                self.position = vec2(0.0, 200.0);
                println!("🔄 快捷栏: 切换到垂直布局");
                BeltLayout::Vertical
            }
            BeltLayout::Vertical => {
                self.position = self.horizontal_position;
                println!("🔄 快捷栏: 切换到水平布局");
                BeltLayout::Horizontal
            }
        };
    }
    
    /// 获取格子位置
    fn get_cell_position(&self, index: usize) -> Vec2 {
        match self.layout {
            BeltLayout::Horizontal => vec2(
                self.position.x + (index as f32) * Self::CELL_SPACING + Self::CELL_OFFSET,
                self.position.y + 3.0
            ),
            BeltLayout::Vertical => vec2(
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
    
    /// 获取旋转按钮位置
    fn get_rotate_button_rect(&self) -> Rect {
        let btn = self.current_rotate_btn();
        match self.layout {
            BeltLayout::Horizontal => Rect::new(
                self.position.x + 222.0, self.position.y + 3.0,
                btn.size.x, btn.size.y
            ),
            BeltLayout::Vertical => Rect::new(
                self.position.x + 19.0, self.position.y + 222.0,
                btn.size.x, btn.size.y
            ),
        }
    }
    
    /// 获取关闭按钮位置
    fn get_close_button_rect(&self) -> Rect {
        let btn = self.current_close_btn();
        match self.layout {
            BeltLayout::Horizontal => Rect::new(
                self.position.x + 222.0, self.position.y + 19.0,
                btn.size.x, btn.size.y
            ),
            BeltLayout::Vertical => Rect::new(
                self.position.x + 3.0, self.position.y + 222.0,
                btn.size.x, btn.size.y
            ),
        }
    }
    
    fn current_rotate_btn(&self) -> &ButtonTextures {
        match self.layout {
            BeltLayout::Horizontal => &self.rotate_btn_h,
            BeltLayout::Vertical => &self.rotate_btn_v,
        }
    }
    
    fn current_close_btn(&self) -> &ButtonTextures {
        match self.layout {
            BeltLayout::Horizontal => &self.close_btn_h,
            BeltLayout::Vertical => &self.close_btn_v,
        }
    }
    
    /// 检查鼠标是否在窗口内
    pub fn contains(&self, pos: Vec2) -> bool {
        let size = self.get_size();
        Rect::new(self.position.x, self.position.y, size.x, size.y).contains(pos)
    }
    
    /// 更新和绘制
    pub fn update_and_draw(&mut self) -> bool {
        if !self.visible {
            return false;
        }
        
        let mouse = mouse_pos();
        
        // 更新悬停状态
        self.hovered_cell = (0..6).find(|&i| self.get_cell_rect(i).contains(mouse));
        
        let rotate_rect = self.get_rotate_button_rect();
        let close_rect = self.get_close_button_rect();
        let hovered_rotate = rotate_rect.contains(mouse);
        let hovered_close = close_rect.contains(mouse);
        
        // 处理按钮点击
        if is_mouse_button_pressed(MouseButton::Left) {
            if hovered_rotate {
                self.flip_layout();
            } else if hovered_close {
                self.close();
                return false;
            }
        }
        
        // 处理拖动（排除按钮和格子区域）
        let can_drag = !hovered_rotate && !hovered_close && self.hovered_cell.is_none();
        if can_drag {
            let drag_area = Rect::new(
                self.position.x, self.position.y,
                self.get_size().x - 20.0, self.get_size().y
            );
            self.drag_helper.apply(drag_area, &mut self.position);
            if self.layout == BeltLayout::Horizontal && self.drag_helper.dragging {
                self.horizontal_position = self.position;
            }
        } else if is_mouse_button_released(MouseButton::Left) {
            self.drag_helper.dragging = false;
        }
        
        // ========== 绘制 ==========
        self.current_bg().draw(self.position);
        self.draw_cells(mouse);
        self.draw_buttons(mouse);
        
        true
    }
    
    /// 绘制物品格子
    fn draw_cells(&self, mouse: Vec2) {
        let style = CellStyle::belt_style();
        
        for i in 0..6 {
            let rect = self.get_cell_rect(i);
            let highlight = if rect.contains(mouse) { CellHighlight::Hovered } else { CellHighlight::None };
            
            draw_cell_frame(rect, highlight, &style);
            
            if let Some(item) = &self.cells[i] {
                if let Some(tex) = self.item_cache.get_cached(item.icon_index) {
                    draw_item_icon(rect, tex, 1.0);
                }
                draw_item_count(rect, item.count, false);
            }
            
            // 数字键提示
            let key_text = format!("{}", i + 1);
            let (key_x, key_y) = match self.layout {
                BeltLayout::Horizontal => (rect.x + 12.0, rect.y - 2.0),
                BeltLayout::Vertical => (rect.x - 12.0, rect.y + 20.0),
            };
            draw_text(&key_text, key_x, key_y, 14.0, YELLOW);
        }
    }
    
    /// 绘制按钮
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
    
    /// 使用物品（按数字键1-6）
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
    pub fn set_item(&mut self, slot: usize, item: Option<BeltItem>) {
        if slot < 6 { self.cells[slot] = item; }
    }
    
    /// 获取格子物品
    pub fn get_item(&self, slot: usize) -> Option<&BeltItem> {
        if slot < 6 { self.cells[slot].as_ref() } else { None }
    }
}

impl Default for BeltDialogNative {
    fn default() -> Self { Self::new() }
}
