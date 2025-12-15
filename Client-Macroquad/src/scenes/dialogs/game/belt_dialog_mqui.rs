// ============================================================================
// BeltDialogMqui - 快捷栏（使用 macroquad::ui 组件）
// ============================================================================
//
// 使用 macroquad 内置 UI 系统实现
// - 使用 Group::draggable() 实现物品拖放
// - 使用 Skin 自定义外观
// - 参考 macroquad/examples/ui.rs
//
// ============================================================================

use macroquad::prelude::*;
use macroquad::ui::{hash, root_ui, widgets::{self, Group}, Drag, Skin};
use crate::resources::LibraryName;

/// 快捷栏布局模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BeltLayout {
    Horizontal,
    Vertical,
}

/// 快捷栏格子物品
#[derive(Debug, Clone, Copy)]
pub struct BeltItem {
    pub icon_index: usize,
    pub count: u32,
}

impl BeltItem {
    pub fn new(icon_index: usize, count: u32) -> Self {
        Self { icon_index, count }
    }
}

/// 拖放命令
#[derive(Debug)]
#[allow(dead_code)]
enum DragCommand {
    /// 使用物品
    Use { slot: usize },
    /// 交换物品
    Swap { from: usize, to: usize },
    /// 丢弃物品（拖出快捷栏）
    Drop { slot: usize },
}

/// 快捷栏对话框（macroquad UI 版本）
pub struct BeltDialogMqui {
    visible: bool,
    layout: BeltLayout,
    position: Vec2,
    horizontal_position: Vec2,
    
    // 拖动状态
    item_dragging: bool,
    
    // macroquad UI Skin
    skin_horizontal: Option<Skin>,
    skin_vertical: Option<Skin>,
    
    // 物品纹理
    item_textures: Vec<Option<Texture2D>>,
    
    // 背景纹理（用于 window 样式）
    bg_horizontal: Option<Image>,
    bg_vertical: Option<Image>,
    
    // 按钮纹理
    rotate_btn_images: [Option<Image>; 3],  // normal, hover, pressed
    close_btn_images: [Option<Image>; 3],
    
    // 格子数据
    cells: [Option<BeltItem>; 6],
    
    // 尺寸
    size_horizontal: Vec2,
    size_vertical: Vec2,
}

impl BeltDialogMqui {
    const CELL_SIZE: f32 = 32.0;
    const CELL_SPACING: f32 = 35.0;
    
    pub fn new() -> Self {
        let position = vec2(400.0, 600.0);
        
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
            
            item_dragging: false,
            
            skin_horizontal: None,
            skin_vertical: None,
            item_textures: Vec::new(),
            bg_horizontal: None,
            bg_vertical: None,
            rotate_btn_images: [None, None, None],
            close_btn_images: [None, None, None],
            
            cells,
            size_horizontal: vec2(240.0, 38.0),
            size_vertical: vec2(38.0, 240.0),
        }
    }
    
    /// 加载纹理并创建 Skin
    pub  fn load_textures(&mut self) {
        println!("🎒 BeltDialogMqui: 加载纹理并创建 Skin...");
        
        // 加载背景纹理为 Image（用于 Skin）
        if let Some(info) = LibraryName::Prguse.get_texture(1932) {
            self.size_horizontal = vec2(info.width as f32, info.height as f32);
            self.bg_horizontal = info.image.map(|t| t.get_texture_data());
        }
        
        if let Some(info) = LibraryName::Prguse.get_texture(1944) {
            self.size_vertical = vec2(info.width as f32, info.height as f32);
            self.bg_vertical = info.image.map(|t| t.get_texture_data());
        }
        
        // 加载按钮图片
        let rotate_indices = [1926, 1927, 1928];
        for (i, idx) in rotate_indices.iter().enumerate() {
            if let Some(info) = LibraryName::Prguse.get_texture(*idx) {
                self.rotate_btn_images[i] = info.image.map(|t| t.get_texture_data());
            }
        }
        
        let close_indices = [1923, 1924, 1925];
        for (i, idx) in close_indices.iter().enumerate() {
            if let Some(info) = LibraryName::Prguse.get_texture(*idx) {
                self.close_btn_images[i] = info.image.map(|t| t.get_texture_data());
            }
        }
        
        // 预加载物品图标
        for i in 0..20 {
            if let Some(info) = LibraryName::Items.get_texture(i) {
                self.item_textures.push(info.image);
            } else {
                self.item_textures.push(None);
            }
        }
        
        // 创建 Skin
        self.create_skins();
        
        println!("  ✅ Skin 创建成功");
    }
    
    /// 创建 UI Skin
    fn create_skins(&mut self) {
        // 水平布局 Skin
        if let Some(ref bg) = self.bg_horizontal {
            let window_style = root_ui()
                .style_builder()
                .background(bg.clone())
                .background_margin(RectOffset::new(0.0, 0.0, 0.0, 0.0))
                .build();
            
            let button_style = if let (Some(ref normal), Some(ref hover), Some(ref pressed)) = 
                (&self.rotate_btn_images[0], &self.rotate_btn_images[1], &self.rotate_btn_images[2]) 
            {
                root_ui()
                    .style_builder()
                    .background(normal.clone())
                    .background_hovered(hover.clone())
                    .background_clicked(pressed.clone())
                    .build()
            } else {
                root_ui().default_skin().button_style.clone()
            };
            
            self.skin_horizontal = Some(Skin {
                window_style,
                button_style,
                ..root_ui().default_skin()
            });
        }
        
        // 垂直布局 Skin（类似）
        if let Some(ref bg) = self.bg_vertical {
            let window_style = root_ui()
                .style_builder()
                .background(bg.clone())
                .background_margin(RectOffset::new(0.0, 0.0, 0.0, 0.0))
                .build();
            
            self.skin_vertical = Some(Skin {
                window_style,
                ..root_ui().default_skin()
            });
        }
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
            println!("🎒 快捷栏: 关闭");
        }
    }
    
    pub fn toggle(&mut self) {
        if self.visible { self.close(); } else { self.open(); }
    }
    
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    
    pub fn flip_layout(&mut self) {
        self.layout = match self.layout {
            BeltLayout::Horizontal => {
                self.horizontal_position = self.position;
                self.position = vec2(0.0, 200.0);
                BeltLayout::Vertical
            }
            BeltLayout::Vertical => {
                self.position = self.horizontal_position;
                BeltLayout::Horizontal
            }
        };
    }
    
    fn get_size(&self) -> Vec2 {
        match self.layout {
            BeltLayout::Horizontal => self.size_horizontal,
            BeltLayout::Vertical => self.size_vertical,
        }
    }
    
    /// 更新和绘制（使用 macroquad UI + Group 拖放）
    pub fn update_and_draw(&mut self) -> bool {
        if !self.visible {
            return false;
        }
        
        let size = self.get_size();
        let layout = self.layout;
        let position = self.position;
        
        // Clone skin 来避免借用问题
        let skin = match layout {
            BeltLayout::Horizontal => self.skin_horizontal.clone(),
            BeltLayout::Vertical => self.skin_vertical.clone(),
        };
        
        // 应用 Skin
        if let Some(ref s) = skin {
            root_ui().push_skin(s);
        }
        
        // 收集需要的数据避免借用问题
        let cells_data: Vec<_> = self.cells.iter().cloned().collect();
        let item_dragging = self.item_dragging;
        
        let mut rotate_clicked = false;
        let mut close_clicked = false;
        let mut drag_command: Option<DragCommand> = None;
        let mut new_item_dragging = false;
        
        // 使用 Window widget
        let window_id = hash!("belt_dialog");
        
        widgets::Window::new(window_id, position, size)
            .label("")
            .titlebar(false)
            .movable(true)
            .ui(&mut root_ui(), |ui| {
                // 绘制物品格子 - 使用 Group::draggable()
                for i in 0..6 {
                    let (x, y) = match layout {
                        BeltLayout::Horizontal => (12.0 + i as f32 * Self::CELL_SPACING, 3.0),
                        BeltLayout::Vertical => (3.0, 12.0 + i as f32 * Self::CELL_SPACING),
                    };
                    
                    let has_item = cells_data[i].is_some();
                    let slot_id = hash!("belt_slot", i);
                    
                    // 使用 Group 实现可拖放的格子
                    let drag = Group::new(slot_id, vec2(Self::CELL_SIZE, Self::CELL_SIZE))
                        .position(vec2(x, y))
                        .draggable(has_item)           // 有物品才能拖
                        .hoverable(item_dragging)      // 拖动时可作为目标
                        .highlight(item_dragging)      // 拖动时高亮
                        .ui(ui, |ui| {
                            // 绘制数量
                            if let Some(ref item) = cells_data[i] {
                                if item.count > 1 {
                                    ui.label(vec2(20.0, 20.0), &format!("{}", item.count));
                                }
                            }
                        });
                    
                    // 处理拖放结果
                    match drag {
                        Drag::Dropped(_, Some(target_id)) if has_item => {
                            // 拖到了另一个格子 - 交换
                            // 从 target_id 找到目标格子索引
                            for j in 0..6 {
                                if hash!("belt_slot", j) == target_id {
                                    drag_command = Some(DragCommand::Swap { from: i, to: j });
                                    break;
                                }
                            }
                        }
                        Drag::Dropped(_, None) if has_item => {
                            // 拖到了空白处 - 丢弃
                            drag_command = Some(DragCommand::Drop { slot: i });
                        }
                        Drag::Dragging(_, _) => {
                            new_item_dragging = true;
                        }
                        _ => {}
                    }
                    
                    // 数字键提示
                    let (key_x, key_y) = match layout {
                        BeltLayout::Horizontal => (x + 12.0, y - 12.0),
                        BeltLayout::Vertical => (x - 12.0, y + 12.0),
                    };
                    widgets::Label::new(&format!("{}", i + 1))
                        .position(vec2(key_x, key_y))
                        .ui(ui);
                }
                
                // 旋转按钮
                let (rx, ry) = match layout {
                    BeltLayout::Horizontal => (222.0, 3.0),
                    BeltLayout::Vertical => (19.0, 222.0),
                };
                if widgets::Button::new("⟳")
                    .position(vec2(rx, ry))
                    .size(vec2(16.0, 16.0))
                    .ui(ui)
                {
                    rotate_clicked = true;
                }
                
                // 关闭按钮
                let (cx, cy) = match layout {
                    BeltLayout::Horizontal => (222.0, 19.0),
                    BeltLayout::Vertical => (3.0, 222.0),
                };
                if widgets::Button::new("×")
                    .position(vec2(cx, cy))
                    .size(vec2(16.0, 16.0))
                    .ui(ui)
                {
                    close_clicked = true;
                }
            });
        
        // 恢复默认 Skin
        if skin.is_some() {
            root_ui().pop_skin();
        }
        
        // 更新拖动状态
        self.item_dragging = new_item_dragging;
        
        // 在 UI 之后绘制物品图标
        for i in 0..6 {
            let (x, y) = match layout {
                BeltLayout::Horizontal => (12.0 + i as f32 * Self::CELL_SPACING, 3.0),
                BeltLayout::Vertical => (3.0, 12.0 + i as f32 * Self::CELL_SPACING),
            };
            
            if let Some(ref item) = self.cells[i] {
                if item.icon_index < self.item_textures.len() {
                    if let Some(ref tex) = self.item_textures[item.icon_index] {
                        draw_texture(tex, position.x + x, position.y + y, WHITE);
                    }
                }
            }
        }
        
        // 处理事件（在闭包外）
        if rotate_clicked {
            self.flip_layout();
        }
        if close_clicked {
            self.close();
            return false;
        }
        
        // 处理拖放命令
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
    
    pub fn set_item(&mut self, slot: usize, item: Option<BeltItem>) {
        if slot < 6 { self.cells[slot] = item; }
    }
    
    pub fn get_item(&self, slot: usize) -> Option<&BeltItem> {
        if slot < 6 { self.cells[slot].as_ref() } else { None }
    }
}

impl Default for BeltDialogMqui {
    fn default() -> Self { Self::new() }
}
