// ============================================================================
// InventoryDialogHybrid - 背包系统（混合版本）
// ============================================================================
//
// 结合两种实现方式的优点：
// - Native 绘制：背景、物品图标、标签页、按钮（精确像素控制）
// - mqui Group：物品拖放（利用内置 draggable API）
//
// 与 Native 版本对比：
// - 拖放逻辑更简洁（不需要手动管理 DragState）
// - 自动处理拖放目标检测
// - 支持物品丢弃（拖出窗口）
// - 支持双击使用物品
//
// ============================================================================

use macroquad::prelude::*;
use macroquad::ui::{hash, root_ui, widgets::Group, Drag, Skin};
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::*;

/// 物品槽位
#[derive(Debug, Clone)]
pub struct ItemSlotHybrid {
    pub icon_index: Option<usize>,
    pub name: String,
    pub count: u32,
}

impl ItemSlotHybrid {
    pub fn empty() -> Self {
        Self { icon_index: None, name: String::new(), count: 0 }
    }

    pub fn new(icon_index: usize, name: String, count: u32) -> Self {
        Self { icon_index: Some(icon_index), name, count }
    }
}

/// 标签页类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryTabHybrid {
    Equipment = 0,
    Items = 1,
    Quest = 2,
}

impl InventoryTabHybrid {
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Equipment,
            1 => Self::Items,
            2 => Self::Quest,
            _ => Self::Equipment,
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            Self::Equipment => "装备",
            Self::Items => "道具",
            Self::Quest => "任务",
        }
    }
}

/// 拖放命令
#[derive(Debug)]
enum DragCommand {
    Use { slot: usize },
    Swap { from: usize, to: usize },
    Drop { slot: usize },
}

/// 丢弃确认对话框状态
#[derive(Debug, Clone)]
struct DropConfirm {
    slot: usize,
    item_name: String,
    count: u32,
}

/// 背包对话框（混合版本）
pub struct InventoryDialogHybrid {
    // 窗口状态
    position: Vec2,
    size: Vec2,
    visible: bool,
    
    // 窗口拖动
    drag_helper: DragHelper,
    
    // 标签页
    current_tab: InventoryTabHybrid,
    tab_items: [Vec<ItemSlotHybrid>; 3],
    hovered_tab: Option<usize>,
    
    // 金币
    pub gold: u32,
    
    // mqui 拖放状态
    item_dragging: bool,
    dragging_from: Option<usize>,
    transparent_skin: Option<Skin>,
    
    // 滚动
    scroll_offsets: [f32; 3],
    
    // 悬停
    hovered_slot: Option<usize>,
    pending_to_belt: Option<(InventoryTabHybrid, usize)>,
    
    // 双击检测
    last_click_time: f64,
    last_click_slot: Option<usize>,

    // 物品拆分模式：右键点击可堆叠物品后进入，点击空格放置一半
    splitting: bool,
    splitting_from: Option<usize>,

    // 丢弃确认对话框
    drop_confirm: Option<DropConfirm>,

    // 纹理
    bg_texture: Option<Texture2D>,
    close_btn: ButtonTextures,
    tab_textures: [[Option<Texture2D>; 2]; 3],
    tab_items2_disabled_texture: Option<Texture2D>,
    item_cache: ItemTextureCache,
}

impl InventoryDialogHybrid {
    // 布局参数（与原版一致）
    const CELL_WIDTH: f32 = 36.0;
    const CELL_HEIGHT: f32 = 32.0;
    const GRID_COLS: usize = 8;
    const VISIBLE_ROWS: usize = 5;
    const GRID_START_X: f32 = 9.0;
    const GRID_START_Y: f32 = 37.0;
    const CELL_SPACING: f32 = 1.0;
    
    const TAB_START_X: f32 = 6.0;
    const TAB_START_Y: f32 = 7.0;
    const TAB_SPACING: f32 = 70.0;
    const TAB_WIDTH: f32 = 72.0;
    const TAB_HEIGHT: f32 = 23.0;
    
    const DOUBLE_CLICK_TIME: f64 = 0.3;
    
    pub fn new() -> Self {
        // 创建示例物品
        let mut tab_items: [Vec<ItemSlotHybrid>; 3] = [
            Vec::with_capacity(46),
            Vec::with_capacity(46),
            Vec::with_capacity(46),
        ];
        
        for i in 0..46 {
            tab_items[0].push(ItemSlotHybrid::new(i % 20, format!("装备{}", i % 10), (i % 5 + 1) as u32));
        }
        for i in 0..46 {
            tab_items[1].push(ItemSlotHybrid::new(20 + i % 20, format!("道具{}", i % 10), (i % 10 + 1) as u32));
        }
        for i in 0..20 {
            tab_items[2].push(ItemSlotHybrid::new(40 + i % 10, format!("任务{}", i % 5), 1));
        }
        
        Self {
            position: vec2(100.0, 100.0),
            size: vec2(312.0, 232.0),
            visible: false,
            
            drag_helper: DragHelper::new(),
            
            current_tab: InventoryTabHybrid::Equipment,
            tab_items,
            hovered_tab: None,
            
            gold: 999999,
            
            item_dragging: false,
            dragging_from: None,
            transparent_skin: None,
            
            scroll_offsets: [0.0, 0.0, 0.0],
            hovered_slot: None,
            pending_to_belt: None,
            
            last_click_time: 0.0,
            last_click_slot: None,

            splitting: false,
            splitting_from: None,

            drop_confirm: None,

            bg_texture: None,
            close_btn: ButtonTextures::new(),
            tab_textures: [[None, None], [None, None], [None, None]],
            tab_items2_disabled_texture: None,
            item_cache: ItemTextureCache::new(),
        }
    }
    
    pub  fn load_textures(&mut self) {
        println!("📦 InventoryDialogHybrid: 加载纹理...");
        
        // 背景
        if let Some(info) = LibraryName::Title.get_texture(196) {
            self.size = vec2(info.width as f32, info.height as f32);
            self.bg_texture = info.image;
        }
        
        // 关闭按钮
        self.close_btn = ButtonTextures::load_from_indices(LibraryName::Prguse2, [360, 361, 362]);
        
        // 标签页纹理
        let tab_indices: [[usize; 2]; 3] = [
            [737, 197], [738, 168], [739, 198],
        ];
        for (tab_idx, indices) in tab_indices.iter().enumerate() {
            for (state_idx, tex_idx) in indices.iter().enumerate() {
                if let Some(info) = LibraryName::Title.get_texture(*tex_idx) {
                    self.tab_textures[tab_idx][state_idx] = info.image;
                }
            }
        }

        // ItemButton2 在 C# Inventory.Length == 46 时的禁用样式: Title[169]
        if let Some(info) = LibraryName::Title.get_texture(169) {
            self.tab_items2_disabled_texture = info.image;
        }
        
        // 物品图标：按需加载（不再预加载，image 值范围通常 1000+）
        // item_cache 使用 lazy get 模式，见下方渲染处
        
        // 透明 Skin
        self.create_transparent_skin();
        
        println!("  ✅ 混合版背包纹理加载成功");
    }
    
    fn create_transparent_skin(&mut self) {
        let transparent_pixel = Image {
            bytes: vec![0, 0, 0, 0],
            width: 1,
            height: 1,
        };
        // 完全透明的样式，包括边框
        let style = root_ui()
            .style_builder()
            .background(transparent_pixel.clone())
            .background_hovered(transparent_pixel.clone())
            .background_clicked(transparent_pixel.clone())
            .color(Color::new(0.0, 0.0, 0.0, 0.0))           // 文字透明
            .color_hovered(Color::new(0.0, 0.0, 0.0, 0.0))   // 悬停时文字透明
            .color_clicked(Color::new(0.0, 0.0, 0.0, 0.0))   // 点击时文字透明
            .build();
        self.transparent_skin = Some(Skin {
            group_style: style.clone(),
            button_style: style.clone(),
            label_style: style,
            ..root_ui().default_skin()
        });
    }
    
    // === 基本操作 ===
    
    pub fn open(&mut self) {
        if !self.visible {
            self.visible = true;
            println!("📦 背包: 打开");
        }
    }
    
    pub fn close(&mut self) {
        if self.visible {
            self.visible = false;
            self.item_dragging = false;
            self.dragging_from = None;
            self.splitting = false;
            self.splitting_from = None;
            println!("📦 背包: 关闭");
        }
    }
    
    pub fn toggle(&mut self) {
        if self.visible { self.close(); } else { self.open(); }
    }

    pub fn is_visible(&self) -> bool { self.visible }

    /// 从 ECS Inventory 组件同步物品数据
    pub fn sync_from_ecs_inventory(&mut self, inv: &crate::components::Inventory) {
        self.gold = inv.gold;

        let mut equip_slots: Vec<ItemSlotHybrid> = Vec::with_capacity(46);
        let mut item_slots: Vec<ItemSlotHybrid> = Vec::with_capacity(46);
        let mut quest_slots: Vec<ItemSlotHybrid> = Vec::with_capacity(20);

        for (i, slot) in inv.items.iter().enumerate() {
            if let Some(item) = slot {
                // 根据物品类型分配到不同标签页
                let is_equip = matches!(item.info.as_ref().map(|x| x.item_type),
                    Some(mir2_shared::enums::ItemType::Weapon) |
                    Some(mir2_shared::enums::ItemType::Armour) |
                    Some(mir2_shared::enums::ItemType::Helmet) |
                    Some(mir2_shared::enums::ItemType::Necklace) |
                    Some(mir2_shared::enums::ItemType::Bracelet) |
                    Some(mir2_shared::enums::ItemType::Ring) |
                    Some(mir2_shared::enums::ItemType::Boots) |
                    Some(mir2_shared::enums::ItemType::Belt) |
                    Some(mir2_shared::enums::ItemType::Amulet) |
                    Some(mir2_shared::enums::ItemType::Torch) |
                    Some(mir2_shared::enums::ItemType::Mount) |
                    Some(mir2_shared::enums::ItemType::Stone)
                );

                // icon_index 使用 ItemInfo.image（C# UserItem.Image 对齐）
                // name 使用 ItemInfo.friendly_name()
                let icon_idx = item.info.as_ref().map(|x| x.image as usize).unwrap_or(0);
                let name = item.info.as_ref().map(|x| x.friendly_name()).unwrap_or_default();
                let slot_item = ItemSlotHybrid {
                    icon_index: Some(icon_idx),
                    name,
                    count: item.count as u32,
                };

                if is_equip {
                    equip_slots.push(slot_item);
                } else {
                    // 任务物品 vs 普通物品简单按索引判断
                    if i < 20 {
                        quest_slots.push(slot_item);
                    } else {
                        item_slots.push(slot_item);
                    }
                }
            }
        }

        // 填充空格到目标大小
        while equip_slots.len() < 46 { equip_slots.push(ItemSlotHybrid::empty()); }
        while item_slots.len() < 46 { item_slots.push(ItemSlotHybrid::empty()); }
        while quest_slots.len() < 20 { quest_slots.push(ItemSlotHybrid::empty()); }

        self.tab_items = [equip_slots, item_slots, quest_slots];
    }
    
    pub fn set_position(&mut self, pos: Vec2) { self.position = pos; }
    
    pub fn get_position(&self) -> Vec2 { self.position }
    
    pub fn switch_tab(&mut self, tab: InventoryTabHybrid) {
        if self.current_tab != tab {
            self.current_tab = tab;
            self.item_dragging = false;
            self.dragging_from = None;
            self.splitting = false;
            self.splitting_from = None;
            println!("📑 切换标签: {}", tab.name());
        }
    }
    
    // === 辅助方法 ===
    
    fn current_items(&self) -> &Vec<ItemSlotHybrid> {
        &self.tab_items[self.current_tab as usize]
    }
    
    fn current_items_mut(&mut self) -> &mut Vec<ItemSlotHybrid> {
        &mut self.tab_items[self.current_tab as usize]
    }
    
    fn current_scroll(&self) -> f32 {
        self.scroll_offsets[self.current_tab as usize]
    }
    
    fn set_current_scroll(&mut self, offset: f32) {
        self.scroll_offsets[self.current_tab as usize] = offset;
    }
    
    fn max_scroll(&self) -> f32 {
        let rows = (self.current_items().len() + Self::GRID_COLS - 1) / Self::GRID_COLS;
        let total = rows as f32 * (Self::CELL_HEIGHT + Self::CELL_SPACING);
        let visible = Self::VISIBLE_ROWS as f32 * (Self::CELL_HEIGHT + Self::CELL_SPACING);
        (total - visible).max(0.0)
    }
    
    fn get_slot_rect(&self, index: usize) -> Rect {
        let col = index % Self::GRID_COLS;
        let row = index / Self::GRID_COLS;
        Rect::new(
            self.position.x + Self::GRID_START_X + col as f32 * (Self::CELL_WIDTH + Self::CELL_SPACING),
            self.position.y + Self::GRID_START_Y + row as f32 * (Self::CELL_HEIGHT + Self::CELL_SPACING) - self.current_scroll(),
            Self::CELL_WIDTH,
            Self::CELL_HEIGHT,
        )
    }
    
    fn get_visible_area(&self) -> Rect {
        Rect::new(
            self.position.x + Self::GRID_START_X,
            self.position.y + Self::GRID_START_Y,
            Self::GRID_COLS as f32 * (Self::CELL_WIDTH + Self::CELL_SPACING),
            Self::VISIBLE_ROWS as f32 * (Self::CELL_HEIGHT + Self::CELL_SPACING),
        )
    }
    
    fn is_slot_visible(&self, index: usize) -> bool {
        let rect = self.get_slot_rect(index);
        let area = self.get_visible_area();
        rect.y + rect.h > area.y && rect.y < area.y + area.h
    }
    
    fn get_tab_rect(&self, index: usize) -> Rect {
        // C# 明确设置 Size = 72x23
        Rect::new(
            self.position.x + Self::TAB_START_X + index as f32 * Self::TAB_SPACING,
            self.position.y + Self::TAB_START_Y,
            Self::TAB_WIDTH,
            Self::TAB_HEIGHT,
        )
    }

    fn is_base_inventory(&self) -> bool {
        // 对齐 C#: GameScene.User.Inventory.Length == 46
        self.tab_items[0].len() == 46
    }
    
    pub fn contains(&self, pos: Vec2) -> bool {
        self.visible && Rect::new(self.position.x, self.position.y, self.size.x, self.size.y).contains(pos)
    }

    pub fn take_transfer_to_belt_request(&mut self) -> Option<(InventoryTabHybrid, usize)> {
        self.pending_to_belt.take()
    }

    pub fn take_item_from_slot(&mut self, tab: InventoryTabHybrid, slot: usize) -> Option<ItemSlotHybrid> {
        let items = &mut self.tab_items[tab as usize];
        if slot >= items.len() {
            return None;
        }
        let item = items[slot].clone();
        if item.icon_index.is_none() || item.count == 0 {
            return None;
        }
        items[slot] = ItemSlotHybrid::empty();
        Some(item)
    }

    pub fn restore_item_to_slot(&mut self, tab: InventoryTabHybrid, slot: usize, item: ItemSlotHybrid) -> bool {
        let items = &mut self.tab_items[tab as usize];
        if slot >= items.len() {
            return false;
        }
        items[slot] = item;
        true
    }

    pub fn try_insert_item(&mut self, item: ItemSlotHybrid) -> Result<(), ItemSlotHybrid> {
        let Some(icon_index) = item.icon_index else {
            return Ok(());
        };
        if item.count == 0 {
            return Ok(());
        }

        let items = self.current_items_mut();

        if let Some(existing) = items.iter_mut().find(|s| s.icon_index == Some(icon_index)) {
            existing.count = existing.count.saturating_add(item.count);
            return Ok(());
        }

        if let Some(empty_slot) = items.iter_mut().find(|s| s.icon_index.is_none() || s.count == 0) {
            *empty_slot = ItemSlotHybrid::new(icon_index, item.name.clone(), item.count);
            return Ok(());
        }

        Err(item)
    }
    
    // === 主更新循环 ===
    
    pub fn update_and_draw(&mut self) {
        if !self.visible { return; }
        
        let mouse = mouse_pos();
        let time = get_time();
        
        // 快捷键
        if is_key_pressed(KeyCode::Key1) { self.switch_tab(InventoryTabHybrid::Equipment); }
        if is_key_pressed(KeyCode::Key2) { self.switch_tab(InventoryTabHybrid::Items); }
        if is_key_pressed(KeyCode::Key3) { self.switch_tab(InventoryTabHybrid::Quest); }
        
        // 更新悬停
        self.hovered_tab = (0..3).find(|&i| self.get_tab_rect(i).contains(mouse));
        self.hovered_slot = self.current_items().iter().enumerate()
            .filter(|(i, _)| self.is_slot_visible(*i))
            .find(|(i, _)| self.get_slot_rect(*i).contains(mouse))
            .map(|(i, _)| i);
        
        // 关闭按钮 (Prguse2[360-362])
        let close_size = if self.close_btn.size.x > 0.0 && self.close_btn.size.y > 0.0 {
            self.close_btn.size
        } else {
            vec2(20.0, 20.0)
        };
        let close_rect = Rect::new(
            self.position.x + 289.0,
            self.position.y + 3.0,
            close_size.x,
            close_size.y,
        );
        let close_hovered = close_rect.contains(mouse);
        
        // 标签页点击
        if is_mouse_button_pressed(MouseButton::Left) {
            if let Some(tab) = self.hovered_tab {
                self.switch_tab(InventoryTabHybrid::from_index(tab));
            } else if close_hovered {
                self.close();
                return;
            }
        }
        if is_mouse_button_pressed(MouseButton::Right) && !self.item_dragging {
            if self.splitting {
                // 拆分模式下，右键点击空格子放置一半
                if let Some(slot_idx) = self.hovered_slot {
                    if let Some(slot) = self.current_items().get(slot_idx) {
                        if slot.icon_index.is_none() || slot.count == 0 {
                            // 放置一半到目标格子
                            if let Some(from_idx) = self.splitting_from {
                                if let Some(src) = self.current_items().get(from_idx) {
                                    let half = (src.count + 1) / 2; // 向上取整
                                    if src.count > 1 {
                                        let items = self.current_items_mut();
                                        let remain = items[from_idx].count - half;
                                        items[slot_idx] = ItemSlotHybrid {
                                            icon_index: items[from_idx].icon_index,
                                            name: items[from_idx].name.clone(),
                                            count: half,
                                        };
                                        items[from_idx].count = remain;
                                    }
                                }
                            }
                        }
                    }
                }
                // 右键也用于退出拆分模式
                self.splitting = false;
                self.splitting_from = None;
            } else if let Some(slot_idx) = self.hovered_slot {
                if self
                    .current_items()
                    .get(slot_idx)
                    .is_some_and(|slot| slot.icon_index.is_some() && slot.count > 1)
                {
                    // 右键点击可堆叠物品 -> 进入拆分模式
                    self.splitting = true;
                    self.splitting_from = Some(slot_idx);
                    println!("✂️ 进入拆分模式，点击空格子放置一半");
                } else if self
                    .current_items()
                    .get(slot_idx)
                    .is_some_and(|slot| slot.icon_index.is_some() && slot.count > 0)
                {
                    // 不可堆叠的物品 -> 发送到快捷栏
                    self.pending_to_belt = Some((self.current_tab, slot_idx));
                }
            }
        }
        
        // 窗口拖动（标签页右侧到关闭按钮左侧）
        let drag_area = Rect::new(
            self.position.x + 218.0, self.position.y,
            71.0, 35.0,
        );
        if !self.item_dragging && self.hovered_tab.is_none() && !close_hovered {
            self.drag_helper.apply(drag_area, &mut self.position);
        }
        
        // 滚动
        let visible_area = self.get_visible_area();
        if visible_area.contains(mouse) {
            let wheel = mouse_wheel().1;
            if wheel != 0.0 {
                let new = (self.current_scroll() - wheel * 20.0).clamp(0.0, self.max_scroll());
                self.set_current_scroll(new);
            }
        }
        
        // ========== 绘制背景 ==========
        if let Some(ref bg) = self.bg_texture {
            draw_texture(bg, self.position.x, self.position.y, WHITE);
        }
        
        // ========== mqui 拖放处理 ==========
        let items_snapshot: Vec<_> = self.current_items().iter().cloned().collect();
        let item_dragging = self.item_dragging;
        let mut drag_command: Option<DragCommand> = None;
        let mut new_dragging = false;
        let mut new_from: Option<usize> = None;
        
        if let Some(ref skin) = self.transparent_skin {
            root_ui().push_skin(skin);
        }
        
        for (i, slot) in items_snapshot.iter().enumerate() {
            if !self.is_slot_visible(i) { continue; }
            
            let rect = self.get_slot_rect(i);
            let has_item = slot.icon_index.is_some();
            let slot_id = hash!("inv_hybrid_slot", self.current_tab as usize, i);
            
            let drag = Group::new(slot_id, vec2(rect.w, rect.h))
                .position(vec2(rect.x, rect.y))
                .draggable(has_item)
                .hoverable(item_dragging)
                .ui(&mut root_ui(), |_| {});
            
            match drag {
                Drag::Dragging(_, _) => {
                    new_dragging = true;
                    new_from = self.dragging_from.or(Some(i));
                }
                Drag::Dropped(_, Some(target_id)) if has_item => {
                    // 查找目标格子
                    for j in 0..items_snapshot.len() {
                        if hash!("inv_hybrid_slot", self.current_tab as usize, j) == target_id && j != i {
                            drag_command = Some(DragCommand::Swap { from: i, to: j });
                            break;
                        }
                    }
                }
                Drag::Dropped(pos, None) if has_item => {
                    // 拖出窗口 = 丢弃（弹出确认）
                    let window = Rect::new(self.position.x, self.position.y, self.size.x, self.size.y);
                    if !window.contains(pos) {
                        drag_command = Some(DragCommand::Drop { slot: i });
                    }
                }
                _ => {}
            }
        }
        
        if self.transparent_skin.is_some() {
            root_ui().pop_skin();
        }
        
        self.item_dragging = new_dragging;
        self.dragging_from = new_from;
        
        // ========== Native 绘制 ==========
        
        // 标签页
        self.draw_tabs();
        
        // 关闭按钮
        self.close_btn.draw(
            vec2(close_rect.x, close_rect.y),
            ButtonState::from_mouse(close_rect, mouse),
        );
        
        // 物品格子
        self.draw_slots(mouse);
        
        // 金币
        draw_text(
            &format!("{}", self.gold),
            self.position.x + 40.0,
            self.position.y + 224.0,
            14.0,
            Color::from_rgba(255, 215, 0, 255),
        );
        
        // 拖动中的物品
        if self.item_dragging {
            if let Some(from) = self.dragging_from {
                let icon_and_count = self.current_items().get(from).map(|slot| (slot.icon_index, slot.count));
                if let (Some(icon_idx), count) = icon_and_count.unwrap_or((None, 0)) {
                    if let Some(tex) = self.item_cache.get(LibraryName::Items, icon_idx) {
                        draw_texture(tex, mouse.x - tex.width() / 2.0, mouse.y - tex.height() / 2.0, WHITE);
                        if count > 1 {
                            draw_text_cn(&format!("{}", count), mouse.x + 10.0, mouse.y + 10.0, 14.0, WHITE);
                        }
                    }
                }
            }
        }
        
        // 双击检测
        if is_mouse_button_pressed(MouseButton::Left) && !self.item_dragging {
            if let Some(slot) = self.hovered_slot {
                if self.last_click_slot == Some(slot) && time - self.last_click_time < Self::DOUBLE_CLICK_TIME {
                    drag_command = Some(DragCommand::Use { slot });
                    self.last_click_slot = None;
                } else {
                    self.last_click_slot = Some(slot);
                    self.last_click_time = time;
                }
            }
        }
        
        // Tooltip
        if !self.item_dragging {
            if let Some(slot_idx) = self.hovered_slot {
                if let Some(slot) = self.current_items().get(slot_idx) {
                    if slot.icon_index.is_some() && !slot.name.is_empty() {
                        let tip = if slot.count > 1 {
                            format!("{} x{}", slot.name, slot.count)
                        } else {
                            slot.name.clone()
                        };
                        draw_tooltip(mouse, &tip);
                    }
                }
            }
        }

        // 拆分模式提示
        if self.splitting {
            if let Some(from) = self.splitting_from {
                if let Some(item) = self.current_items().get(from) {
                    let half = (item.count + 1) / 2;
                    let tip = format!("✂️ 拆分: {} (放{}个到空格子，右键取消)", item.name, half);
                    draw_tooltip(mouse, &tip);
                }
            }
        }

        // 丢弃确认对话框
        if let Some(confirm) = self.drop_confirm.clone() {
            self.draw_drop_confirm(mouse, &confirm);
        }
        
        // ========== 执行命令 ==========
        match drag_command {
            Some(DragCommand::Use { slot }) => {
                println!("🧪 使用物品: 格子{}", slot);
                if let Some(item) = self.current_items_mut().get_mut(slot) {
                    if item.count > 1 {
                        item.count -= 1;
                    } else {
                        *item = ItemSlotHybrid::empty();
                    }
                }
            }
            Some(DragCommand::Swap { from, to }) => {
                println!("🔄 交换: 格子{} <-> 格子{}", from, to);
                self.current_items_mut().swap(from, to);
            }
            Some(DragCommand::Drop { slot }) => {
                // 弹出丢弃确认对话框
                if let Some(item) = self.current_items().get(slot) {
                    self.drop_confirm = Some(DropConfirm {
                        slot,
                        item_name: item.name.clone(),
                        count: item.count,
                    });
                }
            }
            None => {}
        }
    }
    
    fn draw_tabs(&self) {
        for i in 0..3 {
            let rect = self.get_tab_rect(i);
            let is_current = self.current_tab as usize == i;
            let state = if is_current { 1 } else { 0 };

            // C# 逻辑: Inventory.Length==46 时 ItemButton2 显示 Title[169] (禁用)
            if i == 1 && state == 0 && self.is_base_inventory() {
                if let Some(ref tex) = self.tab_items2_disabled_texture {
                    draw_texture(tex, rect.x, rect.y, WHITE);
                    continue;
                }
            }
            
            if let Some(ref tex) = self.tab_textures[i][state] {
                draw_texture(tex, rect.x, rect.y, WHITE);
            }
        }
    }
    
    fn draw_slots(&mut self, mouse: Vec2) {
        // Clone items to avoid borrow conflict with item_cache.get()
        let items_snapshot: Vec<ItemSlotHybrid> = self.current_items().iter().cloned().collect();

        for (i, slot) in items_snapshot.iter().enumerate() {
            if !self.is_slot_visible(i) { continue; }

            let rect = self.get_slot_rect(i);

            // 只在高亮时绘制边框（背景纹理已有网格）
            let highlight = if self.splitting && self.splitting_from == Some(i) {
                CellHighlight::Selected
            } else if self.splitting && rect.contains(mouse) && (slot.icon_index.is_none() || slot.count == 0) {
                CellHighlight::DragTarget
            } else if self.item_dragging && self.dragging_from == Some(i) {
                CellHighlight::Selected
            } else if self.item_dragging && rect.contains(mouse) && self.dragging_from != Some(i) {
                CellHighlight::DragTarget
            } else if rect.contains(mouse) {
                CellHighlight::Hovered
            } else {
                CellHighlight::None
            };

            // 只有高亮时才绘制边框
            if highlight != CellHighlight::None {
                let color = match highlight {
                    CellHighlight::Hovered => Color::from_rgba(0, 255, 0, 255),
                    CellHighlight::Selected => Color::from_rgba(255, 255, 0, 255),
                    CellHighlight::DragTarget => Color::from_rgba(0, 255, 255, 255),
                    CellHighlight::None => unreachable!(),
                };
                draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, color);
            }

            // 物品图标（按需加载）
            if let Some(icon_idx) = slot.icon_index {
                let alpha = if self.item_dragging && self.dragging_from == Some(i) { 0.4 } else { 1.0 };
                if let Some(tex) = self.item_cache.get(LibraryName::Items, icon_idx) {
                    draw_item_icon(rect, tex, alpha);
                }
                if !(self.item_dragging && self.dragging_from == Some(i)) {
                    draw_item_count(rect, slot.count, false);
                }
            }
        }
    }
}

impl Default for InventoryDialogHybrid {
    fn default() -> Self { Self::new() }
}

impl InventoryDialogHybrid {
    /// 绘制丢弃确认对话框
    fn draw_drop_confirm(&mut self, mouse: Vec2, confirm: &DropConfirm) {
        let win_w = 260.0;
        let win_h = 100.0;
        let win_x = self.position.x + (self.size.x - win_w) / 2.0;
        let win_y = self.position.y + (self.size.y - win_h) / 2.0;

        // 背景
        draw_rectangle(win_x, win_y, win_w, win_h, Color::from_rgba(40, 40, 50, 240));
        draw_rectangle_lines(win_x, win_y, win_w, win_h, 2.0, Color::from_rgba(255, 100, 100, 255));

        // 提示文字
        let msg = if confirm.count > 1 {
            format!("确定要丢弃 {} x{} 吗？", confirm.item_name, confirm.count)
        } else {
            format!("确定要丢弃 {} 吗？", confirm.item_name)
        };
        draw_text_cn(&msg, win_x + 10.0, win_y + 15.0, 14.0, WHITE);

        // 确认按钮
        let btn_w = 70.0;
        let btn_h = 28.0;
        let btn_y = win_y + win_h - btn_h - 10.0;
        let confirm_btn = Rect::new(win_x + 30.0, btn_y, btn_w, btn_h);
        let cancel_btn = Rect::new(win_x + win_w - 30.0 - btn_w, btn_y, btn_w, btn_h);

        let confirm_hovered = confirm_btn.contains(mouse);
        let cancel_hovered = cancel_btn.contains(mouse);

        // 确认按钮
        let confirm_color = if confirm_hovered {
            Color::from_rgba(100, 200, 100, 255)
        } else {
            Color::from_rgba(60, 140, 60, 255)
        };
        draw_rectangle(confirm_btn.x, confirm_btn.y, confirm_btn.w, confirm_btn.h, confirm_color);
        draw_rectangle_lines(confirm_btn.x, confirm_btn.y, confirm_btn.w, confirm_btn.h, 1.0, Color::from_rgba(255, 255, 255, 150));
        draw_text_cn("确认", confirm_btn.x + 17.0, confirm_btn.y + 8.0, 14.0, WHITE);

        // 取消按钮
        let cancel_color = if cancel_hovered {
            Color::from_rgba(200, 100, 100, 255)
        } else {
            Color::from_rgba(140, 60, 60, 255)
        };
        draw_rectangle(cancel_btn.x, cancel_btn.y, cancel_btn.w, cancel_btn.h, cancel_color);
        draw_rectangle_lines(cancel_btn.x, cancel_btn.y, cancel_btn.w, cancel_btn.h, 1.0, Color::from_rgba(255, 255, 255, 150));
        draw_text_cn("取消", cancel_btn.x + 17.0, cancel_btn.y + 8.0, 14.0, WHITE);

        // 按钮点击检测
        if is_mouse_button_pressed(MouseButton::Left) {
            if confirm_hovered {
                if let Some(confirm_copy) = self.drop_confirm.clone() {
                    let slot = confirm_copy.slot;
                    if let Some(item) = self.current_items_mut().get_mut(slot) {
                        *item = ItemSlotHybrid::empty();
                    }
                    self.drop_confirm = None;
                    println!("🗑️ 确认丢弃: {}", confirm_copy.item_name);
                }
            } else if cancel_hovered {
                self.drop_confirm = None;
            }
        }
    }
}

// === 辅助函数 ===

fn draw_tooltip(mouse: Vec2, text: &str) {
    let w = text.len() as f32 * 7.0 + 8.0;
    draw_rectangle(mouse.x + 12.0, mouse.y - 18.0, w, 20.0, Color::from_rgba(0, 0, 0, 200));
    draw_text(text, mouse.x + 16.0, mouse.y - 2.0, 14.0, WHITE);
}
