// ============================================================================
// NativeUIUtils - 原生 UI 公共组件工具库
// ============================================================================
//
// 提供可复用的 UI 组件：
// 1. ButtonTextures - 按钮纹理组（支持 normal/hover/pressed 三态）
// 2. ItemCell - 物品格子绘制
// 3. Tooltip - 工具提示
// 4. DragHelper - 窗口拖动辅助
// 5. TextureCache - 纹理缓存管理
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use std::collections::HashMap;

// ============================================================================
// 按钮纹理组
// ============================================================================

/// 按钮状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ButtonState {
    Normal = 0,
    Hover = 1,
    Pressed = 2,
}

impl ButtonState {
    /// 根据鼠标状态获取按钮状态
    pub fn from_mouse(rect: Rect, mouse_pos: Vec2) -> Self {
        if rect.contains(mouse_pos) {
            if is_mouse_button_down(MouseButton::Left) {
                ButtonState::Pressed
            } else {
                ButtonState::Hover
            }
        } else {
            ButtonState::Normal
        }
    }
    
    /// 检查按钮是否被点击（释放时）
    pub fn is_clicked(rect: Rect, mouse_pos: Vec2) -> bool {
        rect.contains(mouse_pos) && is_mouse_button_released(MouseButton::Left)
    }
    
    /// 检查按钮是否被按下（按下时）
    pub fn is_just_pressed(rect: Rect, mouse_pos: Vec2) -> bool {
        rect.contains(mouse_pos) && is_mouse_button_pressed(MouseButton::Left)
    }
}

/// 三态按钮纹理
#[derive(Debug, Clone, Default)]
pub struct ButtonTextures {
    /// [normal, hover, pressed]
    pub textures: [Option<Texture2D>; 3],
    /// 按钮尺寸（从纹理获取或默认值）
    pub size: Vec2,
}

impl ButtonTextures {
    pub fn new() -> Self {
        Self {
            textures: [None, None, None],
            size: vec2(16.0, 16.0),
        }
    }
    
    /// 从资源库加载纹理（连续三个索引：normal, hover, pressed）
    pub fn load_from_library(library: LibraryName, start_index: usize) -> Self {
        let mut btn = Self::new();
        for i in 0..3 {
            if let Some(info) = library.get_texture(start_index + i) {
                if i == 0 {
                    btn.size = vec2(info.width as f32, info.height as f32);
                }
                btn.textures[i] = info.image;
            }
        }
        btn
    }
    
    /// 从资源库加载纹理（自定义索引数组）
    pub fn load_from_indices(library: LibraryName, indices: [usize; 3]) -> Self {
        let mut btn = Self::new();
        for (i, idx) in indices.iter().enumerate() {
            if let Some(info) = library.get_texture(*idx) {
                if i == 0 {
                    btn.size = vec2(info.width as f32, info.height as f32);
                }
                btn.textures[i] = info.image;
            }
        }
        btn
    }
    
    /// 获取当前状态的纹理
    pub fn get_texture(&self, state: ButtonState) -> Option<&Texture2D> {
        self.textures[state as usize].as_ref()
    }
    
    /// 绘制按钮
    pub fn draw(&self, pos: Vec2, state: ButtonState) {
        if let Some(tex) = self.get_texture(state) {
            draw_texture(tex, pos.x, pos.y, WHITE);
        }
    }
    
    /// 绘制按钮并返回是否被点击
    pub fn draw_button(&self, rect: Rect, mouse_pos: Vec2) -> bool {
        let state = ButtonState::from_mouse(rect, mouse_pos);
        self.draw(vec2(rect.x, rect.y), state);
        ButtonState::is_clicked(rect, mouse_pos)
    }
}

// ============================================================================
// 物品格子
// ============================================================================

/// 格子高亮类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CellHighlight {
    /// 无高亮
    None,
    /// 悬停
    Hovered,
    /// 选中
    Selected,
    /// 拖放目标
    DragTarget,
}

/// 物品格子绘制配置
#[derive(Debug, Clone)]
pub struct CellStyle {
    /// 格子背景色
    pub bg_color: Color,
    /// 默认边框色
    pub border_color: Color,
    /// 悬停边框色
    pub hover_color: Color,
    /// 选中边框色
    pub selected_color: Color,
    /// 拖放目标边框色
    pub drag_target_color: Color,
    /// 边框宽度
    pub border_width: f32,
    /// 高亮边框宽度
    pub highlight_border_width: f32,
}

impl Default for CellStyle {
    fn default() -> Self {
        Self {
            bg_color: Color::new(0.16, 0.16, 0.16, 0.8),
            border_color: Color::new(0.4, 0.4, 0.4, 1.0),
            hover_color: Color::new(0.0, 1.0, 0.0, 1.0),        // 绿色
            selected_color: Color::new(1.0, 1.0, 0.0, 1.0),     // 黄色
            drag_target_color: Color::new(0.0, 1.0, 1.0, 1.0),  // 青色
            border_width: 1.0,
            highlight_border_width: 2.0,
        }
    }
}

impl CellStyle {
    /// 快捷栏风格
    pub fn belt_style() -> Self {
        Self {
            bg_color: Color::new(0.16, 0.16, 0.16, 0.8),
            border_color: Color::new(0.4, 0.4, 0.4, 1.0),
            hover_color: Color::new(0.8, 0.8, 0.2, 1.0),   // 黄色
            selected_color: Color::new(1.0, 0.8, 0.2, 1.0),
            drag_target_color: Color::new(0.2, 0.8, 0.8, 1.0),
            border_width: 1.5,
            highlight_border_width: 2.0,
        }
    }
    
    /// 背包风格
    pub fn inventory_style() -> Self {
        Self {
            bg_color: Color::new(0.12, 0.12, 0.15, 0.6),
            border_color: Color::new(0.3, 0.3, 0.35, 0.8),
            hover_color: Color::new(0.0, 1.0, 0.0, 1.0),     // 绿色悬停
            selected_color: Color::new(1.0, 1.0, 0.0, 1.0),  // 黄色选中
            drag_target_color: Color::new(0.0, 1.0, 1.0, 1.0), // 青色目标
            border_width: 1.0,
            highlight_border_width: 2.0,
        }
    }
}

/// 绘制物品格子框架
pub fn draw_cell_frame(rect: Rect, highlight: CellHighlight, style: &CellStyle) {
    // 背景
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, style.bg_color);
    
    // 边框
    let (color, width) = match highlight {
        CellHighlight::None => (style.border_color, style.border_width),
        CellHighlight::Hovered => (style.hover_color, style.highlight_border_width),
        CellHighlight::Selected => (style.selected_color, style.highlight_border_width),
        CellHighlight::DragTarget => (style.drag_target_color, style.highlight_border_width),
    };
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, width, color);
}

/// 绘制物品图标（居中）
pub fn draw_item_icon(rect: Rect, texture: &Texture2D, alpha: f32) {
    let icon_w = texture.width();
    let icon_h = texture.height();
    let offset_x = (rect.w - icon_w) / 2.0;
    let offset_y = (rect.h - icon_h) / 2.0;
    
    let color = Color::new(1.0, 1.0, 1.0, alpha);
    draw_texture(texture, rect.x + offset_x, rect.y + offset_y, color);
}

/// 绘制物品数量（右下角）
pub fn draw_item_count(rect: Rect, count: u32, with_shadow: bool) {
    if count <= 1 {
        return;
    }
    
    let count_text = format!("{}", count);
    let text_x = rect.x + rect.w - 12.0;
    let text_y = rect.y + rect.h - 2.0;
    
    if with_shadow {
        draw_text(&count_text, text_x + 1.0, text_y + 1.0, 16.0, BLACK);
    }
    draw_text(&count_text, text_x, text_y, 16.0, WHITE);
}

// ============================================================================
// 工具提示
// ============================================================================

/// 绘制工具提示
pub fn draw_tooltip(pos: Vec2, text: &str) {
    let text_width = text.chars().count() as f32 * 8.0;
    let padding = 4.0;
    
    // 背景
    draw_rectangle(
        pos.x - padding,
        pos.y - 16.0,
        text_width + padding * 2.0,
        20.0,
        Color::new(0.0, 0.0, 0.0, 0.85)
    );
    
    // 边框
    draw_rectangle_lines(
        pos.x - padding,
        pos.y - 16.0,
        text_width + padding * 2.0,
        20.0,
        1.0,
        Color::new(0.5, 0.5, 0.5, 0.8)
    );
    
    // 文字
    draw_text(text, pos.x, pos.y, 14.0, WHITE);
}

/// 在鼠标位置绘制工具提示
pub fn draw_tooltip_at_mouse(text: &str, offset: Vec2) {
    let mouse = mouse_position();
    draw_tooltip(vec2(mouse.0 + offset.x, mouse.1 + offset.y), text);
}

// ============================================================================
// 窗口拖动辅助
// ============================================================================

/// 窗口拖动状态
#[derive(Debug, Clone)]
pub struct DragHelper {
    /// 是否正在拖动
    pub dragging: bool,
    /// 拖动偏移
    pub offset: Vec2,
}

impl Default for DragHelper {
    fn default() -> Self {
        Self::new()
    }
}

impl DragHelper {
    pub fn new() -> Self {
        Self {
            dragging: false,
            offset: Vec2::ZERO,
        }
    }
    
    /// 更新拖动状态，返回新位置（如果正在拖动）
    pub fn update(&mut self, drag_area: Rect, position: Vec2, mouse_pos: Vec2) -> Vec2 {
        // 开始拖动
        if is_mouse_button_pressed(MouseButton::Left) && drag_area.contains(mouse_pos) {
            self.dragging = true;
            self.offset = mouse_pos - position;
        }
        
        // 停止拖动
        if is_mouse_button_released(MouseButton::Left) {
            self.dragging = false;
        }
        
        // 计算新位置
        if self.dragging {
            mouse_pos - self.offset
        } else {
            position
        }
    }
    
    /// 简化版：更新并直接修改位置
    pub fn apply(&mut self, drag_area: Rect, position: &mut Vec2) {
        let mouse = mouse_position();
        let mouse_pos = vec2(mouse.0, mouse.1);
        *position = self.update(drag_area, *position, mouse_pos);
    }
}

// ============================================================================
// 纹理缓存
// ============================================================================

/// 物品纹理缓存
#[derive(Debug, Default)]
pub struct ItemTextureCache {
    textures: HashMap<usize, Texture2D>,
}

impl ItemTextureCache {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
        }
    }
    
    /// 预加载指定范围的物品图标
    pub fn preload(&mut self, library: LibraryName, start: usize, count: usize) {
        for i in start..(start + count) {
            if let Some(info) = library.get_texture(i) {
                if let Some(tex) = info.image {
                    self.textures.insert(i, tex);
                }
            }
        }
    }
    
    /// 获取物品纹理（按需加载）
    pub fn get(&mut self, library: LibraryName, index: usize) -> Option<&Texture2D> {
        if !self.textures.contains_key(&index) {
            if let Some(info) = library.get_texture(index) {
                if let Some(tex) = info.image {
                    self.textures.insert(index, tex);
                }
            }
        }
        self.textures.get(&index)
    }
    
    /// 获取已缓存的纹理（不加载）
    pub fn get_cached(&self, index: usize) -> Option<&Texture2D> {
        self.textures.get(&index)
    }
    
    /// 是否已缓存
    pub fn contains(&self, index: usize) -> bool {
        self.textures.contains_key(&index)
    }
    
    /// 缓存数量
    pub fn len(&self) -> usize {
        self.textures.len()
    }
    
    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }
}

// ============================================================================
// 背景纹理
// ============================================================================

/// 背景纹理（带覆盖层）
#[derive(Debug, Clone, Default)]
pub struct BackgroundTexture {
    /// 主背景
    pub main: Option<Texture2D>,
    /// 覆盖层（半透明）
    pub overlay: Option<Texture2D>,
    /// 尺寸
    pub size: Vec2,
}

impl BackgroundTexture {
    pub fn new() -> Self {
        Self {
            main: None,
            overlay: None,
            size: Vec2::ZERO,
        }
    }
    
    /// 从资源库加载（main_index, overlay_index）
    pub fn load(library: LibraryName, main_index: usize, overlay_index: Option<usize>) -> Self {
        let mut bg = Self::new();
        
        if let Some(info) = library.get_texture(main_index) {
            bg.size = vec2(info.width as f32, info.height as f32);
            bg.main = info.image;
        }
        
        if let Some(idx) = overlay_index {
            if let Some(info) = library.get_texture(idx) {
                bg.overlay = info.image;
            }
        }
        
        bg
    }
    
    /// 绘制背景
    pub fn draw(&self, pos: Vec2) {
        if let Some(ref tex) = self.main {
            draw_texture(tex, pos.x, pos.y, WHITE);
        }
        
        if let Some(ref tex) = self.overlay {
            draw_texture(tex, pos.x, pos.y, Color::new(1.0, 1.0, 1.0, 0.5));
        }
    }
    
    /// 绘制背景（自定义覆盖层透明度）
    pub fn draw_with_alpha(&self, pos: Vec2, overlay_alpha: f32) {
        if let Some(ref tex) = self.main {
            draw_texture(tex, pos.x, pos.y, WHITE);
        }
        
        if let Some(ref tex) = self.overlay {
            draw_texture(tex, pos.x, pos.y, Color::new(1.0, 1.0, 1.0, overlay_alpha));
        }
    }
}

// ============================================================================
// 鼠标位置辅助
// ============================================================================

/// 获取鼠标位置作为 Vec2
#[inline]
pub fn mouse_pos() -> Vec2 {
    let (x, y) = mouse_position();
    vec2(x, y)
}

/// 检查鼠标是否在矩形内
#[inline]
pub fn is_mouse_over(rect: Rect) -> bool {
    rect.contains(mouse_pos())
}

// ============================================================================
// 拖放状态
// ============================================================================

/// 通用物品拖放状态
#[derive(Debug, Clone)]
pub struct ItemDragState {
    /// 源索引
    pub source_index: usize,
    /// 物品图标索引
    pub icon_index: usize,
    /// 物品数量
    pub count: u32,
}

impl ItemDragState {
    pub fn new(source_index: usize, icon_index: usize, count: u32) -> Self {
        Self {
            source_index,
            icon_index,
            count,
        }
    }
}

// ============================================================================
// 关闭按钮
// ============================================================================

/// 关闭按钮（通用）
#[derive(Debug, Clone, Default)]
pub struct CloseButton {
    pub textures: ButtonTextures,
    /// 相对于窗口右上角的偏移
    pub offset: Vec2,
}

impl CloseButton {
    pub fn new() -> Self {
        Self {
            textures: ButtonTextures::new(),
            offset: vec2(-25.0, 3.0),
        }
    }
    
    /// 从 Prguse2 库加载（360/361/362）
    pub fn load_prguse2() -> Self {
        Self {
            textures: ButtonTextures::load_from_library(LibraryName::Prguse2, 360),
            offset: vec2(-25.0, 3.0),
        }
    }
    
    /// 获取按钮矩形
    pub fn get_rect(&self, window_pos: Vec2, window_size: Vec2) -> Rect {
        Rect::new(
            window_pos.x + window_size.x + self.offset.x,
            window_pos.y + self.offset.y,
            self.textures.size.x.max(20.0),
            self.textures.size.y.max(20.0),
        )
    }
    
    /// 绘制并返回是否被点击
    pub fn draw(&self, window_pos: Vec2, window_size: Vec2, mouse_pos: Vec2) -> bool {
        let rect = self.get_rect(window_pos, window_size);
        self.textures.draw_button(rect, mouse_pos)
    }
}
