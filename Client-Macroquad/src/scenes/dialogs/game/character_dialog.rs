// ============================================================================
// CharacterDialogHybrid - 角色/装备对话框（混合版本）
// ============================================================================
//
// Native 绘制 + mqui 拖放
// 支持：装备穿戴/卸载、装备与背包交换
//
// ============================================================================

use macroquad::prelude::*;
use macroquad::ui::{hash, root_ui, widgets::Group, Drag, Skin};
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::*;

/// 标签页类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterTabHybrid {
    Character = 0,  // 装备页
    Status = 1,     // 状态I
    State = 2,      // 状态II
    Skills = 3,     // 技能页
}

impl CharacterTabHybrid {
    pub fn from_index(i: usize) -> Self {
        match i {
            0 => Self::Character,
            1 => Self::Status,
            2 => Self::State,
            3 => Self::Skills,
            _ => Self::Character,
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            Self::Character => "角色",
            Self::Status => "状态I",
            Self::State => "状态II",
            Self::Skills => "技能",
        }
    }
}

/// 装备槽位类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipSlot {
    Weapon = 0,
    Armor = 1,
    Helmet = 2,
    Torch = 3,
    Necklace = 4,
    BraceletL = 5,
    BraceletR = 6,
    RingL = 7,
    RingR = 8,
    Amulet = 9,
    Belt = 10,
    Boots = 11,
    Stone = 12,
    Mount = 13,
}

/// 装备数据
#[derive(Debug, Clone)]
pub struct EquipmentItemHybrid {
    pub icon_index: usize,
    pub state_image: usize,  // StateItems库中的外观索引
    pub durability: (u32, u32),
    pub name: String,
    pub unique_id: u64,
}

impl EquipmentItemHybrid {
    pub fn new(icon_index: usize) -> Self {
        Self { icon_index, state_image: 0, durability: (100, 100), name: String::new(), unique_id: 0 }
    }

    pub fn with_state(icon_index: usize, state_image: usize) -> Self {
        Self { icon_index, state_image, durability: (100, 100), name: String::new(), unique_id: 0 }
    }

    pub fn with_details(icon_index: usize, state_image: usize, name: String, unique_id: u64) -> Self {
        Self { icon_index, state_image, durability: (100, 100), name, unique_id }
    }
}

/// 角色属性
#[derive(Debug, Clone)]
pub struct CharacterStatsHybrid {
    pub level: u32,
    pub health: (u32, u32),
    pub mana: (u32, u32),
    pub dc: (u32, u32),
    pub mc: (u32, u32),
    pub sc: (u32, u32),
    pub ac: (u32, u32),
    pub mac: (u32, u32),
}

impl Default for CharacterStatsHybrid {
    fn default() -> Self {
        Self {
            level: 1,
            health: (100, 100),
            mana: (50, 50),
            dc: (1, 5),
            mc: (0, 0),
            sc: (0, 0),
            ac: (0, 3),
            mac: (0, 2),
        }
    }
}

/// 技能数据
#[derive(Debug, Clone)]
pub struct SkillInfo {
    pub spell_id: u8,
    pub name: String,
    pub level: u8,
    pub icon_index: usize,
    pub can_use: bool,
}

impl SkillInfo {
    pub fn new(name: &str, level: u8, icon_index: usize) -> Self {
        Self { spell_id: 0, name: name.to_string(), level, icon_index, can_use: true }
    }
}

/// 角色对话框（混合版本）
pub struct CharacterDialogHybrid {
    // 窗口状态
    position: Vec2,
    size: Vec2,
    visible: bool,
    
    // 窗口拖动
    drag_helper: DragHelper,
    
    // 标签页
    current_tab: CharacterTabHybrid,
    hovered_tab: Option<usize>,
    
    // 装备数据 (14个槽位)
    pub equipment: [Option<EquipmentItemHybrid>; 14],
    
    // 角色信息
    pub name: String,
    pub guild: Option<String>,
    pub stats: CharacterStatsHybrid,
    pub skills: Vec<SkillInfo>,
    
    // mqui 拖放
    item_dragging: bool,
    dragging_from: Option<usize>,
    transparent_skin: Option<Skin>,
    
    // 悬停
    hovered_slot: Option<usize>,

    /// 拖出窗口请求（跨对话框拖拽）：(slot, drop_position)
    pending_drag_out: Option<(usize, Vec2)>,

    // 纹理
    bg_texture: Option<Texture2D>,
    page_textures: [Option<Texture2D>; 4],  // 4个页面背景
    tab_textures: [[Option<Texture2D>; 2]; 4],  // 4个标签，每个2状态
    close_btn: ButtonTextures,
    class_texture: Option<Texture2D>,
    item_cache: ItemTextureCache,
    state_cache: ItemTextureCache,  // StateItems外观缓存
}

impl CharacterDialogHybrid {
    // 装备槽位位置（相对于页面背景）
    const EQUIP_SLOTS: [(f32, f32); 14] = [
        (123.0, 7.0),   // Weapon
        (163.0, 7.0),   // Armor
        (203.0, 7.0),   // Helmet
        (203.0, 134.0), // Torch
        (203.0, 98.0),  // Necklace
        (8.0, 170.0),   // BraceletL
        (203.0, 170.0), // BraceletR
        (8.0, 206.0),   // RingL
        (203.0, 206.0), // RingR
        (8.0, 242.0),   // Amulet
        (88.0, 242.0),  // Belt
        (48.0, 242.0),  // Boots
        (128.0, 242.0), // Stone
        (203.0, 62.0),  // Mount
    ];
    
    const SLOT_SIZE: f32 = 36.0;
    const PAGE_OFFSET: (f32, f32) = (8.0, 90.0);
    
    pub fn new() -> Self {
        // 示例装备 (带外观索引)
        // StateItems库的索引参考character_dialog.rs
        let mut equipment = std::array::from_fn(|_| None);
        equipment[0] = Some(EquipmentItemHybrid::with_state(4, 10));   // 武器 - StateItems[10]
        equipment[1] = Some(EquipmentItemHybrid::with_state(24, 10)); // 衣服 - StateItems[10]  
        equipment[2] = Some(EquipmentItemHybrid::with_state(44, 10)); // 头盔 - StateItems[10]
        equipment[4] = Some(EquipmentItemHybrid::new(84));  // 项链
        equipment[5] = Some(EquipmentItemHybrid::new(104)); // 左手镯
        equipment[6] = Some(EquipmentItemHybrid::new(104)); // 右手镯
        equipment[7] = Some(EquipmentItemHybrid::new(124)); // 左戒指
        equipment[8] = Some(EquipmentItemHybrid::new(124)); // 右戒指
        
        // 示例技能
        let skills = vec![
            SkillInfo::new("基本剑术", 3, 0),
            SkillInfo::new("攻杀剑术", 2, 1),
            SkillInfo::new("刺杀剑术", 1, 2),
            SkillInfo::new("半月弯刀", 2, 3),
            SkillInfo::new("野蛮冲撞", 1, 4),
            SkillInfo::new("烈火剑法", 0, 5),
        ];
        
        Self {
            position: vec2(100.0, 100.0),
            size: vec2(264.0, 380.0),
            visible: false,
            
            drag_helper: DragHelper::new(),
            
            current_tab: CharacterTabHybrid::Character,
            hovered_tab: None,
            
            equipment,
            
            name: "测试角色".to_string(),
            guild: Some("传奇公会".to_string()),
            stats: CharacterStatsHybrid::default(),
            skills,
            
            item_dragging: false,
            dragging_from: None,
            transparent_skin: None,
            
            hovered_slot: None,
            pending_drag_out: None,
            
            bg_texture: None,
            page_textures: [None, None, None, None],
            tab_textures: [[None, None], [None, None], [None, None], [None, None]],
            close_btn: ButtonTextures::new(),
            class_texture: None,
            item_cache: ItemTextureCache::new(),
            state_cache: ItemTextureCache::new(),
        }
    }
    
    pub fn load_textures(&mut self) {
        // 主背景 Title[504]
        if let Some(info) = LibraryName::Title.get_texture(504) {
            self.size = vec2(info.width as f32, info.height as f32);
            self.bg_texture = info.image;
        }
        
        // 页面背景
        // Character: Prguse[340], Status: Title[506], State: Title[507], Skills: Title[508]
        if let Some(info) = LibraryName::Prguse.get_texture(340) {
            self.page_textures[0] = info.image;
        }
        for (i, idx) in [506, 507, 508].iter().enumerate() {
            if let Some(info) = LibraryName::Title.get_texture(*idx) {
                self.page_textures[i + 1] = info.image;
            }
        }
        
        // 标签页纹理 Title[500-503] 普通/选中
        for i in 0..4 {
            if let Some(info) = LibraryName::Title.get_texture(500 + i) {
                self.tab_textures[i][0] = info.image.clone(); // 普通
                self.tab_textures[i][1] = info.image;         // 选中（同一个）
            }
        }
        
        // 关闭按钮 Prguse2[360-362]
        self.close_btn = ButtonTextures::load_from_indices(LibraryName::Prguse2, [360, 361, 362]);
        
        // 职业图标 Prguse[100]
        if let Some(info) = LibraryName::Prguse.get_texture(100) {
            self.class_texture = info.image;
        }
        
        // 物品图标
        self.item_cache.preload(LibraryName::Items, 0, 250);
        
        // 预加载 StateItems（人物外观）
        self.state_cache.preload(LibraryName::StateItems, 0, 50);
        
        // 透明 Skin
        self.transparent_skin = Some(create_transparent_skin());
    }
    
    // === 基本操作 ===
    
    pub fn open(&mut self) {
        if !self.visible {
            self.visible = true;
        }
    }
    
    pub fn close(&mut self) {
        if self.visible {
            self.visible = false;
            self.item_dragging = false;
            self.dragging_from = None;
        }
    }
    
    pub fn toggle(&mut self) {
        if self.visible { self.close(); } else { self.open(); }
    }
    
    pub fn is_visible(&self) -> bool { self.visible }
    
    pub fn set_position(&mut self, pos: Vec2) { self.position = pos; }
    
    /// 检查当前是否在技能标签页
    pub fn is_skills_tab(&self) -> bool {
        self.current_tab == CharacterTabHybrid::Skills
    }

    /// 添加学到的技能
    pub fn learn_skill(&mut self, spell_id: u8, name: String, level: u8, icon: u8) {
        if let Some(existing) = self.skills.iter_mut().find(|s| s.spell_id == spell_id) {
            existing.level = level;
            existing.name = name;
            existing.icon_index = icon as usize;
            return;
        }
        self.skills.push(SkillInfo { spell_id, name, level, icon_index: icon as usize, can_use: true });
    }

    /// 技能升级
    pub fn level_up_skill(&mut self, spell_id: u8, level: u8) {
        if let Some(skill) = self.skills.iter_mut().find(|s| s.spell_id == spell_id) {
            skill.level = level;
        }
    }

    /// 移除技能
    pub fn remove_skill(&mut self, spell_id: u8) {
        self.skills.retain(|s| s.spell_id != spell_id);
    }

    /// 切换技能可用状态
    pub fn toggle_skill(&mut self, spell_id: u8, can_use: bool) {
        if let Some(skill) = self.skills.iter_mut().find(|s| s.spell_id == spell_id) {
            skill.can_use = can_use;
        }
    }
    
    pub fn switch_tab(&mut self, tab: CharacterTabHybrid) {
        if self.current_tab != tab {
            self.current_tab = tab;
        }
    }
    
    // === 辅助方法 ===
    
    fn get_tab_rect(&self, index: usize) -> Rect {
        // 标签位置: (8,70), (70,70), (132,70), (194,70)
        let x = self.position.x + 8.0 + index as f32 * 62.0;
        Rect::new(x, self.position.y + 70.0, 64.0, 20.0)
    }
    
    fn get_equip_slot_rect(&self, index: usize) -> Rect {
        let (ox, oy) = Self::EQUIP_SLOTS[index];
        Rect::new(
            self.position.x + Self::PAGE_OFFSET.0 + ox,
            self.position.y + Self::PAGE_OFFSET.1 + oy - 2.0,
            Self::SLOT_SIZE,
            Self::SLOT_SIZE,
        )
    }
    
    pub fn contains(&self, pos: Vec2) -> bool {
        self.visible && Rect::new(self.position.x, self.position.y, self.size.x, self.size.y).contains(pos)
    }
    
    // === 主更新循环 ===
    
    pub fn take_drag_out_request(&mut self) -> Option<(usize, Vec2)> {
        self.pending_drag_out.take()
    }

    // === 主更新循环 ===

    pub fn update_and_draw(&mut self) {
        if !self.visible { return; }
        
        let mouse = mouse_pos();
        
        // 快捷键
        if is_key_pressed(KeyCode::Key1) { self.switch_tab(CharacterTabHybrid::Character); }
        if is_key_pressed(KeyCode::Key2) { self.switch_tab(CharacterTabHybrid::Status); }
        if is_key_pressed(KeyCode::Key3) { self.switch_tab(CharacterTabHybrid::State); }
        if is_key_pressed(KeyCode::Key4) { self.switch_tab(CharacterTabHybrid::Skills); }
        
        // 更新悬停
        self.hovered_tab = (0..4).find(|&i| self.get_tab_rect(i).contains(mouse));
        self.hovered_slot = if self.current_tab == CharacterTabHybrid::Character {
            (0..14).find(|&i| self.get_equip_slot_rect(i).contains(mouse))
        } else {
            None
        };
        
        // 关闭按钮 (Prguse2[360-362])
        let close_size = if self.close_btn.size.x > 0.0 && self.close_btn.size.y > 0.0 {
            self.close_btn.size
        } else {
            vec2(20.0, 20.0)
        };
        let close_rect = Rect::new(
            self.position.x + 241.0,
            self.position.y + 3.0,
            close_size.x,
            close_size.y,
        );
        let close_hovered = close_rect.contains(mouse);
        
        // 标签页点击
        if is_mouse_button_pressed(MouseButton::Left) {
            if let Some(tab) = self.hovered_tab {
                self.switch_tab(CharacterTabHybrid::from_index(tab));
            } else if close_hovered {
                self.close();
                return;
            }
        }
        
        // 窗口拖动（标题栏）
        let drag_area = Rect::new(self.position.x, self.position.y, self.size.x - 30.0, 30.0);
        if !self.item_dragging && self.hovered_tab.is_none() && !close_hovered {
            self.drag_helper.apply(drag_area, &mut self.position);
        }
        
        // ========== 绘制背景 ==========
        if let Some(ref bg) = self.bg_texture {
            draw_texture(bg, self.position.x, self.position.y, WHITE);
        }
        
        // 角色名
        draw_text_cn(&self.name, self.position.x + 132.0 - self.name.len() as f32 * 4.0, self.position.y + 25.0, 14.0, WHITE);
        if let Some(ref guild) = self.guild {
            draw_text_cn(guild, self.position.x + 132.0 - guild.len() as f32 * 3.0, self.position.y + 45.0, 12.0, GOLD);
        }
        
        // 标签页
        self.draw_tabs(mouse);
        
        // 关闭按钮
        self.close_btn.draw(vec2(close_rect.x, close_rect.y), ButtonState::from_mouse(close_rect, mouse));
        
        // 页面内容
        match self.current_tab {
            CharacterTabHybrid::Character => self.draw_character_page(mouse),
            CharacterTabHybrid::Status => self.draw_status_page(),
            CharacterTabHybrid::State => self.draw_state_page(),
            CharacterTabHybrid::Skills => self.draw_skills_page(),
        }
    }
    
    fn draw_tabs(&self, _mouse: Vec2) {
        for i in 0..4 {
            let rect = self.get_tab_rect(i);
            let is_current = self.current_tab as usize == i;
            let state = if is_current { 1 } else { 0 };
            
            if let Some(ref tex) = self.tab_textures[i][state] {
                let alpha = if is_current { 1.0 } else { 0.7 };
                draw_texture(tex, rect.x, rect.y, Color::new(1.0, 1.0, 1.0, alpha));
            }
        }
    }
    
    fn draw_character_page(&mut self, mouse: Vec2) {
        // 页面背景
        if let Some(ref tex) = self.page_textures[0] {
            draw_texture(tex, self.position.x + Self::PAGE_OFFSET.0, self.position.y + Self::PAGE_OFFSET.1, WHITE);
        }
        
        // 职业图标
        if let Some(ref tex) = self.class_texture {
            draw_texture(tex, self.position.x + 15.0, self.position.y + 33.0, WHITE);
        }
        
        // === 绘制人物模型区域 ===
        self.draw_character_model();
        
        // === mqui 装备拖放 ===
        let item_dragging = self.item_dragging;
        let mut drag_command: Option<DragCommand> = None;
        let mut new_dragging = false;
        let mut new_from: Option<usize> = None;

        if let Some(ref skin) = self.transparent_skin {
            root_ui().push_skin(skin);
        }

        for i in 0..14 {
            let rect = self.get_equip_slot_rect(i);
            let has_item = self.equipment[i].is_some();
            let slot_id = hash!("char_equip_slot", i);
            
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
                    for j in 0..14 {
                        if hash!("char_equip_slot", j) == target_id && j != i {
                            drag_command = Some(DragCommand::SwapEquip { from: i, to: j });
                            break;
                        }
                    }
                }
                Drag::Dropped(pos, None) if has_item => {
                    let window = Rect::new(self.position.x, self.position.y, self.size.x, self.size.y);
                    if !window.contains(pos) {
                        // 拖出窗口 → 跨对话框拖拽
                        self.pending_drag_out = Some((i, pos));
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
        
        // === Native 绘制装备 ===
        for i in 0..14 {
            let rect = self.get_equip_slot_rect(i);
            
            // 高亮
            let highlight = if self.item_dragging && self.dragging_from == Some(i) {
                CellHighlight::Selected
            } else if self.item_dragging && rect.contains(mouse) && self.dragging_from != Some(i) {
                CellHighlight::DragTarget
            } else if rect.contains(mouse) {
                CellHighlight::Hovered
            } else {
                CellHighlight::None
            };
            
            if highlight != CellHighlight::None {
                let color = match highlight {
                    CellHighlight::Hovered => Color::from_rgba(0, 255, 0, 255),
                    CellHighlight::Selected => Color::from_rgba(255, 255, 0, 255),
                    CellHighlight::DragTarget => Color::from_rgba(0, 255, 255, 255),
                    CellHighlight::None => unreachable!(),
                };
                draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, color);
            }
            
            // 装备图标
            if let Some(equip) = &self.equipment[i] {
                let alpha = if self.item_dragging && self.dragging_from == Some(i) { 0.4 } else { 1.0 };
                if let Some(tex) = self.item_cache.get_cached(equip.icon_index) {
                    draw_item_icon(rect, tex, alpha);
                }
            }
        }
        
        // 拖动中的装备
        if self.item_dragging {
            if let Some(from) = self.dragging_from {
                if let Some(equip) = &self.equipment[from] {
                    if let Some(tex) = self.item_cache.get_cached(equip.icon_index) {
                        draw_texture(tex, mouse.x - tex.width() / 2.0, mouse.y - tex.height() / 2.0, WHITE);
                    }
                }
            }
        }
        
        // 执行命令
        match drag_command {
            Some(DragCommand::SwapEquip { from, to }) => {
                self.equipment.swap(from, to);
            }
            _ => {}
        }
    }
    
    fn draw_status_page(&self) {
        if let Some(ref tex) = self.page_textures[1] {
            draw_texture(tex, self.position.x + Self::PAGE_OFFSET.0, self.position.y + Self::PAGE_OFFSET.1, WHITE);
        }
        
        let x = self.position.x + 134.0;
        let y = self.position.y + 110.0;
        let h = 18.0;
        
        draw_text_cn(&format!("{}/{}", self.stats.health.0, self.stats.health.1), x, y, 12.0, WHITE);
        draw_text_cn(&format!("{}/{}", self.stats.mana.0, self.stats.mana.1), x, y + h, 12.0, WHITE);
        draw_text_cn(&format!("{}-{}", self.stats.ac.0, self.stats.ac.1), x, y + h * 2.0, 12.0, WHITE);
        draw_text_cn(&format!("{}-{}", self.stats.mac.0, self.stats.mac.1), x, y + h * 3.0, 12.0, WHITE);
        draw_text_cn(&format!("{}-{}", self.stats.dc.0, self.stats.dc.1), x, y + h * 4.0, 12.0, WHITE);
        draw_text_cn(&format!("{}-{}", self.stats.mc.0, self.stats.mc.1), x, y + h * 5.0, 12.0, WHITE);
        draw_text_cn(&format!("{}-{}", self.stats.sc.0, self.stats.sc.1), x, y + h * 6.0, 12.0, WHITE);
    }
    
    fn draw_state_page(&self) {
        if let Some(ref tex) = self.page_textures[2] {
            draw_texture(tex, self.position.x + Self::PAGE_OFFSET.0, self.position.y + Self::PAGE_OFFSET.1, WHITE);
        }
        // 状态II详细属性（简化）
        draw_text_cn("经验: 0%", self.position.x + 134.0, self.position.y + 110.0, 12.0, WHITE);
    }
    
    fn draw_skills_page(&self) {
        if let Some(ref tex) = self.page_textures[3] {
            draw_texture(tex, self.position.x + Self::PAGE_OFFSET.0, self.position.y + Self::PAGE_OFFSET.1, WHITE);
        }
        
        // 绘制技能列表
        let start_x = self.position.x + 20.0;
        let start_y = self.position.y + 115.0;
        let row_height = 24.0;
        let col_width = 110.0;
        
        for (i, skill) in self.skills.iter().enumerate() {
            let col = i % 2;
            let row = i / 2;
            let x = start_x + col as f32 * col_width;
            let y = start_y + row as f32 * row_height;

            // 技能名称（禁用态：红色灰化）
            let color = if !skill.can_use {
                Color::from_rgba(180, 80, 80, 255)
            } else if skill.level > 0 {
                WHITE
            } else {
                GRAY
            };
            draw_text_cn(&skill.name, x, y, 12.0, color);

            // 技能等级
            let level_text = format!("Lv.{}", skill.level);
            let level_color = if !skill.can_use {
                Color::from_rgba(150, 60, 60, 255)
            } else if skill.level > 0 {
                GOLD
            } else {
                DARKGRAY
            };
            draw_text_cn(&level_text, x + 70.0, y, 11.0, level_color);
        }
    }
    
    /// 绘制人物模型（使用StateItems库的真实纹理）
    fn draw_character_model(&mut self) {
        // 人物锚点位置 (CharacterPage的DisplayLocation)
        let anchor_x = self.position.x + Self::PAGE_OFFSET.0;
        let anchor_y = self.position.y + Self::PAGE_OFFSET.1;
        
        // 绘制顺序：衣服 -> 武器 -> 头盔（从下到上）
        
        // 1. 绘制衣服/盔甲外观
        if let Some(armour) = &self.equipment[1] {
            if armour.state_image > 0 {
                if let Some(info) = LibraryName::StateItems.get_texture(armour.state_image) {
                    if let Some(tex) = &info.image {
                        let x = anchor_x + info.offset_x as f32;
                        let y = anchor_y + info.offset_y as f32;
                        draw_texture(tex, x, y, WHITE);
                    }
                }
            }
        }
        
        // 2. 绘制武器外观
        if let Some(weapon) = &self.equipment[0] {
            if weapon.state_image > 0 {
                if let Some(info) = LibraryName::StateItems.get_texture(weapon.state_image) {
                    if let Some(tex) = &info.image {
                        let x = anchor_x + info.offset_x as f32;
                        let y = anchor_y + info.offset_y as f32;
                        draw_texture(tex, x, y, WHITE);
                    }
                }
            }
        }
        
        // 3. 绘制头盔外观
        if let Some(helmet) = &self.equipment[2] {
            if helmet.state_image > 0 {
                if let Some(info) = LibraryName::StateItems.get_texture(helmet.state_image) {
                    if let Some(tex) = &info.image {
                        let x = anchor_x + info.offset_x as f32;
                        let y = anchor_y + info.offset_y as f32;
                        draw_texture(tex, x, y, WHITE);
                    }
                }
            }
        }
        
        // 如果没有任何装备有外观，显示一个占位符
        let has_any_visual = self.equipment[0].as_ref().is_some_and(|e| e.state_image > 0)
            || self.equipment[1].as_ref().is_some_and(|e| e.state_image > 0)
            || self.equipment[2].as_ref().is_some_and(|e| e.state_image > 0);
        
        if !has_any_visual {
            // 绘制简单的人物轮廓占位符
            let center_x = anchor_x + 120.0;
            let center_y = anchor_y + 120.0;
            
            let body_color = Color::from_rgba(120, 100, 80, 180);
            let outline_color = Color::from_rgba(80, 60, 40, 200);
            
            // 头
            draw_circle(center_x, center_y - 50.0, 15.0, body_color);
            draw_circle_lines(center_x, center_y - 50.0, 15.0, 1.5, outline_color);
            
            // 身体
            draw_rectangle(center_x - 15.0, center_y - 33.0, 30.0, 45.0, body_color);
            draw_rectangle_lines(center_x - 15.0, center_y - 33.0, 30.0, 45.0, 1.5, outline_color);
            
            // 腿
            draw_rectangle(center_x - 12.0, center_y + 12.0, 10.0, 35.0, body_color);
            draw_rectangle(center_x + 2.0, center_y + 12.0, 10.0, 35.0, body_color);
            
            // 提示文字
            draw_text_cn("无外观", center_x - 18.0, center_y + 70.0, 12.0, GRAY);
        }
    }
}

impl Default for CharacterDialogHybrid {
    fn default() -> Self { Self::new() }
}
